/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() + input.len().div_ceil(15));

    let mut chunks = input.chunks_exact(15);
    for chunk in &mut chunks {
        let block: &[u8; 15] = chunk.try_into().expect("chunk length is fixed");
        hecc_encode_impl(block, &mut output);
    }

    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let mut block = [0u8; 15];
        block[..remainder.len()].copy_from_slice(remainder);
        let start_len = output.len();
        hecc_encode_impl(&block, &mut output);
        output.truncate(start_len + remainder.len() + 1);
    }

    output
}
fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1 = 0u8;
    let mut p2 = 0u8;
    let mut power2 = 4usize;

    for (j, &byte) in input.iter().enumerate() {
        output.push(byte);
        p2 ^= (byte.count_ones() as u8) & 1;

        let mut data = byte;
        for bit in 0..8 {
            let j_bit = j * 8 + bit;
            let mut i = j_bit + 3;
            while i >= power2 {
                power2 *= 2;
                i += 1;
            }

            if data & 1 != 0 {
                p1 ^= i as u8;
            }
            data >>= 1;
        }
    }

    p2 ^= (p1.count_ones() as u8) & 1;
    output.push(p1.wrapping_add(p2 << 7));
}
/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() % 16 == 1 {
        return None;
    }

    let required_size = input.len() - (input.len() / 16 + usize::from(input.len() % 16 != 0));
    let mut output = Vec::with_capacity(required_size);

    let mut chunks = input.chunks_exact(16);
    for chunk in &mut chunks {
        let block: &[u8; 16] = chunk.try_into().expect("chunk length is fixed");
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
    }

    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let mut block = [0u8; 16];
        block[..remainder.len()].copy_from_slice(remainder);
        let start_len = output.len();
        if !hecc_decode_impl(&block, &mut output) {
            return None;
        }
        output.truncate(start_len + remainder.len() - 1);
    }

    Some(output)
}
fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let insize = input.len();
    if insize < 2 {
        return false;
    }

    let mut p1 = 0u8;
    let mut p2 = 0u8;
    let mut power2 = 4usize;
    let check = input[insize - 1];

    let start_len = output.len();
    output.extend_from_slice(&input[..insize - 1]);

    for (j, &byte) in input[..insize - 1].iter().enumerate() {
        p2 ^= (byte.count_ones() as u8) & 1;

        let mut data = byte;
        for bit in 0..8 {
            let j_bit = j * 8 + bit;
            let mut i = j_bit + 3;
            while i >= power2 {
                power2 *= 2;
                i += 1;
            }

            if data & 1 != 0 {
                p1 ^= i as u8;
            }
            data >>= 1;
        }
    }

    p1 ^= check & 0x7f;

    if p1 != 0 {
        p2 ^= (check.count_ones() as u8) & 1;
        if p2 == 0 {
            output.truncate(start_len);
            return false;
        }

        if p1 & (p1 - 1) != 0 {
            let bit_index = usize::from(p1) - usize::from(log2uint8(p1)) - 2;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            output[start_len + byte_index] ^= 1u8 << bit_offset;
        }
    }

    true
}
pub fn log2uint8(x: u8) -> u8 {
    debug_assert!(x != 0);
    (u8::BITS - 1 - x.leading_zeros()) as u8
}
