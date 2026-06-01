#[inline]
fn parity(x: u8) -> u8 {
    (x.count_ones() & 1) as u8
}

fn encode_block(input: &[u8], output: &mut Vec<u8>) {
    let insize = input.len();
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let bits = insize * 8;
    let mut idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[idx];
            idx += 1;
            output.push(data);
            p2 ^= parity(data);
        }
        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;
        j += 1;
        i += 1;
    }

    p2 ^= parity(p1);
    output.push(p1.wrapping_add(p2 << 7));
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chunks = input.chunks_exact(15);
    while let Some(chunk) = chunks.next() {
        let block: &[u8; 15] = chunk.try_into().unwrap();
        hecc_encode_impl(block, &mut output);
    }
    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        encode_block(remainder, &mut output);
    }
    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    encode_block(input, output);
}

fn decode_block(input: &[u8], output: &mut Vec<u8>) -> bool {
    if input.len() < 2 {
        return false;
    }
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize = input.len() - 1;
    let check = input[insize];

    let start = output.len();
    for k in 0..insize {
        output.push(input[k]);
    }

    let bits = insize * 8;
    let mut idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[idx];
            idx += 1;
            p2 ^= parity(data);
        }
        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;
        j += 1;
        i += 1;
    }

    p1 ^= check & 0x7f;

    if p1 != 0 {
        let pm = check;
        p2 ^= parity(pm);

        if p2 == 0 {
            // At least two errors
            return false;
        }

        // Check if p1 is power of 2 (i.e. parity bit error)
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate error location
            let p2_loc = p1 - log2uint8(p1) - 2;
            let pm_byte = (p2_loc / 8) as usize;
            let p2_bit = p2_loc % 8;
            output[start + pm_byte] ^= 1u8 << p2_bit;
        }
    }

    true
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();
    if insize % 16 == 1 {
        return None;
    }
    let mut output = Vec::new();
    let mut idx: usize = 0;
    while idx + 16 <= insize {
        let block: &[u8; 16] = input[idx..idx + 16].try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }
    if idx < insize {
        let partial = &input[idx..];
        if !decode_block(partial, &mut output) {
            return None;
        }
    }
    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    decode_block(input, output)
}

pub fn log2uint8(x: u8) -> u8 {
    7 - x.leading_zeros() as u8
}
