use std::cmp;
/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let block_count = input.len().div_ceil(15);
    let capacity = input.len().checked_add(block_count).unwrap_or(0);
    let mut output = Vec::with_capacity(capacity);

    let mut offset = 0;
    while offset < input.len() {
        let chunk_len = cmp::min(15, input.len() - offset);
        let chunk = &input[offset..offset + chunk_len];

        if chunk_len == 15 {
            let block: &[u8; 15] = chunk.try_into().expect("full blocks are 15 bytes");
            hecc_encode_impl(block, &mut output);
        } else {
            hecc_encode_partial(chunk, &mut output);
        }

        offset += chunk_len;
    }

    output
}
fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    hecc_encode_partial(input, output);
}
/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() % 16 == 1 {
        return None;
    }

    let block_count = input.len() / 16 + usize::from(input.len() % 16 != 0);
    let capacity = input.len().checked_sub(block_count)?;
    let mut output = Vec::with_capacity(capacity);

    let mut offset = 0;
    while offset + 16 <= input.len() {
        let block: &[u8; 16] = input[offset..offset + 16].try_into().expect("full blocks are 16 bytes");
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        offset += 16;
    }

    if offset < input.len() && !hecc_decode_partial(&input[offset..], &mut output) {
        return None;
    }

    Some(output)
}
fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    hecc_decode_partial(input, output)
}
pub fn log2uint8(x: u8) -> u8 {
    if x == 0 {
        0
    } else {
        (u8::BITS as u8 - 1) - x.leading_zeros() as u8
    }
}

fn hecc_encode_partial(input: &[u8], output: &mut Vec<u8>) {
    debug_assert!(input.len() <= 15);

    let mut p1 = 0u8;
    let mut p2 = 0u8;
    let mut power2 = 4usize;
    let mut bit_position = 3usize;

    for &byte in input {
        output.push(byte);
        p2 ^= parity(byte);

        let mut data = byte;
        for _ in 0..8 {
            while power2 == bit_position {
                power2 *= 2;
                bit_position += 1;
            }

            if data & 1 != 0 {
                p1 ^= bit_position as u8;
            }

            data >>= 1;
            bit_position += 1;
        }
    }

    p2 ^= parity(p1);
    output.push(p1 | (p2 << 7));
}

fn hecc_decode_partial(input: &[u8], output: &mut Vec<u8>) -> bool {
    if input.len() < 2 {
        return false;
    }

    let data_len = input.len() - 1;
    let check = input[data_len];

    let mut p1 = 0u8;
    let mut p2 = 0u8;
    let mut power2 = 4usize;
    let mut bit_position = 3usize;

    let start = output.len();
    output.extend_from_slice(&input[..data_len]);

    for &byte in &input[..data_len] {
        p2 ^= parity(byte);

        let mut data = byte;
        for _ in 0..8 {
            while power2 == bit_position {
                power2 *= 2;
                bit_position += 1;
            }

            if data & 1 != 0 {
                p1 ^= bit_position as u8;
            }

            data >>= 1;
            bit_position += 1;
        }
    }

    p1 ^= check & 0x7f;

    if p1 != 0 {
        p2 ^= parity(check);
        if p2 == 0 {
            output.truncate(start);
            return false;
        }

        if p1 & (p1 - 1) != 0 {
            let bit_index = p1 - log2uint8(p1) - 2;
            let byte_index = usize::from(bit_index / 8);
            let bit_offset = bit_index % 8;
            output[start + byte_index] ^= 1 << bit_offset;
        }
    }

    true
}

fn parity(byte: u8) -> u8 {
    (byte.count_ones() as u8) & 1
}
