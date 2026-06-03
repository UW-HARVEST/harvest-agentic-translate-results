use std::cmp;

#[inline]
fn parity_u8(x: u8) -> u8 {
    let mut p = x;
    p ^= p >> 4;
    p ^= p >> 2;
    p ^= p >> 1;
    p & 1
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let block_count = (input.len() + 14) / 15;
    let mut output = Vec::with_capacity(input.len() + block_count);

    let mut idx = 0;
    while idx + 15 <= input.len() {
        let arr: &[u8; 15] = input[idx..idx + 15].try_into().unwrap();
        hecc_encode_impl(arr, &mut output);
        idx += 15;
    }

    if idx < input.len() {
        let remainder = input.len() - idx;
        let mut buf = [0u8; 15];
        buf[..remainder].copy_from_slice(&input[idx..]);

        let prev_len = output.len();
        hecc_encode_impl(&buf, &mut output);

        // For partial blocks: keep only `remainder` data bytes plus parity byte.
        let parity_byte = output[prev_len + 15];
        output.truncate(prev_len + remainder);
        output.push(parity_byte);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;
    let mut in_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[in_idx];
            in_idx += 1;
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
    // A length with `% 16 == 1` indicates a truncated message (just a parity byte).
    if input.len() % 16 == 1 {
        return None;
    }

    let full_blocks = input.len() / 16;
    let partial = input.len() % 16;
    let expected_data_len = full_blocks * 15 + if partial > 0 { partial - 1 } else { 0 };
    let mut output = Vec::with_capacity(expected_data_len);

    let mut idx = 0;
    while idx + 16 <= input.len() {
        let arr: &[u8; 16] = input[idx..idx + 16].try_into().unwrap();
        if !hecc_decode_impl(arr, &mut output) {
            return None;
        }
        idx += 16;
    }

    if idx < input.len() {
        let remainder = input.len() - idx; // includes the trailing parity byte
        let mut buf = [0u8; 16];
        // Data bytes in this partial block are the first `remainder - 1` bytes.
        buf[..remainder - 1].copy_from_slice(&input[idx..idx + remainder - 1]);
        // Parity byte must sit at position 15 so the impl picks it up correctly.
        buf[15] = input[idx + remainder - 1];

        let prev_len = output.len();
        if !hecc_decode_impl(&buf, &mut output) {
            return None;
        }
        output.truncate(prev_len + remainder - 1);
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let check = input[15];

    let prev_len = output.len();
    output.extend_from_slice(&input[..15]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;
    let mut in_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[in_idx];
            in_idx += 1;
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
        // Check overall message parity (including the parity byte).
        p2 ^= parity_u8(check);

        if p2 == 0 {
            // At least two errors detected, unrecoverable.
            return false;
        }

        // If p1 is not a power of two, the error is in a data bit (not a parity bit).
        if p1 & p1.wrapping_sub(1) != 0 {
            // Calculate the data bit position.
            let pos = p1 - log2uint8(p1) - 2;
            let byte_idx = (pos / 8) as usize;
            let bit_idx = pos % 8;

            // Use cmp::min to defensively bound the index against the block size.
            let target = prev_len + cmp::min(byte_idx, 14);
            output[target] ^= 1u8 << bit_idx;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to: 31 - __builtin_clz(x) for nonzero u8.
    // For x == 0, the C version's behavior is undefined; we return 0 here.
    if x == 0 {
        return 0;
    }
    7 - x.leading_zeros() as u8
}
