#[allow(unused_imports)]
use std::cmp;

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let insize = input.len();
    let reqsize = insize + (insize + 14) / 15;
    let mut output: Vec<u8> = Vec::with_capacity(reqsize);

    let full_blocks = insize / 15;
    let remainder = insize % 15;

    for b in 0..full_blocks {
        let start = b * 15;
        let block: &[u8; 15] = (&input[start..start + 15]).try_into().unwrap();
        hecc_encode_impl(block, &mut output);
    }

    if remainder > 0 {
        let mut padded = [0u8; 15];
        let off = full_blocks * 15;
        padded[..remainder].copy_from_slice(&input[off..off + remainder]);

        let start = output.len();
        hecc_encode_impl(&padded, &mut output);
        // impl appended 16 bytes: 15 data + 1 parity
        // Keep only first `remainder` data bytes, then the parity byte
        let parity = output[start + 15];
        output.truncate(start + remainder);
        output.push(parity);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;
    let mut input_iter = input.iter();

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = *input_iter.next().unwrap();
            output.push(data);
            p2 ^= parity_u8(data);
        }

        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i += 1;
    }

    p2 ^= parity_u8(p1);

    output.push(p1.wrapping_add(p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();

    // A trailing single byte cannot be a valid (data + parity) tail
    if insize % 16 == 1 {
        return None;
    }

    let ecc_count = insize / 16 + if insize % 16 != 0 { 1 } else { 0 };
    let reqsize = insize - ecc_count;
    let mut output: Vec<u8> = Vec::with_capacity(reqsize);

    let full_blocks = insize / 16;
    let remainder = insize % 16;

    for b in 0..full_blocks {
        let start = b * 16;
        let block: &[u8; 16] = (&input[start..start + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
    }

    if remainder > 0 {
        // remainder is in 2..=15 here
        let off = full_blocks * 16;
        let mut padded = [0u8; 16];
        // First (remainder - 1) bytes are data
        padded[..remainder - 1].copy_from_slice(&input[off..off + remainder - 1]);
        // The last byte of the partial block is the parity/check byte
        padded[15] = input[off + remainder - 1];

        let start = output.len();
        if !hecc_decode_impl(&padded, &mut output) {
            return None;
        }
        // impl appended 15 bytes; keep only the first (remainder - 1) data bytes
        output.truncate(start + remainder - 1);
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let check = input[15];
    let start = output.len();

    // Copy data bytes (15 of them) to output
    output.extend_from_slice(&input[..15]);

    let insize_bits: usize = 15 * 8;
    let mut input_iter = input[..15].iter();

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = *input_iter.next().unwrap();
            p2 ^= parity_u8(data);
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
        // Check overall message parity
        p2 ^= parity_u8(check);

        if p2 == 0 {
            // At least two errors
            return false;
        }

        // If p1 is not a power of two, the error is in a data bit (not a parity bit).
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate error location: pos = p1 - log2(p1) - 2
            let pos = p1.wrapping_sub(log2uint8(p1)).wrapping_sub(2);
            let pm = (pos / 8) as usize;
            let bit = pos % 8;

            // Fix the error within the just-copied data region
            output[start + pm] ^= 1u8 << bit;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Mirrors C: 31 - __builtin_clz(v)
    // Uses u32 leading zeros to match the C semantics where v is promoted to int.
    // For v == 0 in C this is undefined; here we mimic the wrap to 255 (matches
    // C's `31 - 32 = -1` cast to uint8_t).
    let lz = (x as u32).leading_zeros() as i32;
    (31i32 - lz) as u8
}

fn parity_u8(mut x: u8) -> u8 {
    x ^= x >> 4;
    x ^= x >> 2;
    x ^= x >> 1;
    x & 1
}
