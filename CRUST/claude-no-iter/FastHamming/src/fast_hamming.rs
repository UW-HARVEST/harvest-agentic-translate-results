#[allow(unused_imports)]
use std::cmp;

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();

    // Process full 15-byte blocks first.
    let mut idx = 0;
    while idx + 15 <= input.len() {
        let block: &[u8; 15] = (&input[idx..idx + 15]).try_into().unwrap();
        hecc_encode_impl(block, &mut output);
        idx += 15;
    }

    // Handle remaining partial block (if any).
    let remainder = &input[idx..];
    if !remainder.is_empty() {
        let mut block = [0u8; 15];
        block[..remainder.len()].copy_from_slice(remainder);

        // Run the impl on the zero-padded block; tmp will receive 16 bytes:
        // 15 data bytes followed by 1 parity byte. Padding bytes are zero so
        // they do not affect the computed parity.
        let mut tmp: Vec<u8> = Vec::new();
        hecc_encode_impl(&block, &mut tmp);

        // Append only the actual data bytes plus the parity byte.
        output.extend_from_slice(&tmp[..remainder.len()]);
        output.push(tmp[15]);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;

    let mut inbuf_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;

    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i = i.wrapping_add(1);
            continue;
        }

        if j % 8 == 0 {
            data = input[inbuf_idx];
            output.push(data);
            inbuf_idx += 1;
            p2 ^= (data.count_ones() & 1) as u8;
        }

        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i = i.wrapping_add(1);
    }

    // Fold p1's parity into p2 (single-bit overall parity).
    p2 ^= (p1.count_ones() & 1) as u8;

    output.push(p1.wrapping_add(p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    // A length of insize % 16 == 1 indicates a truncated message.
    if input.len() % 16 == 1 {
        return None;
    }

    let mut output: Vec<u8> = Vec::new();
    let mut idx = 0;

    // Process full 16-byte blocks.
    while idx + 16 <= input.len() {
        let block: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }

    // Handle remaining partial block (insize bytes, with check at the end).
    let rem = input.len() - idx;
    if rem > 0 {
        // Build a zero-padded 16-byte block: data bytes at [0..rem-1],
        // padding zeros at [rem-1..15], and the parity check byte at [15].
        // Padding zeros do not affect parity computations, and the impl
        // uses input[15] as the parity check, matching the partial-block
        // semantics of the original C implementation.
        let mut block = [0u8; 16];
        let r = rem - 1;
        block[..r].copy_from_slice(&input[idx..idx + r]);
        block[15] = input[idx + r];

        let mut tmp: Vec<u8> = Vec::new();
        if !hecc_decode_impl(&block, &mut tmp) {
            return None;
        }

        // The impl writes 15 bytes to tmp; only the first `r` are real data.
        output.extend_from_slice(&tmp[..r]);
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    // Track where this block starts in `output` so we can revert any
    // tentative writes if a double-bit error is detected.
    let start = output.len();

    let check = input[15];

    // Pre-populate output with the (possibly corrupted) data bytes.
    output.extend_from_slice(&input[..15]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;

    let mut inbuf_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;

    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i = i.wrapping_add(1);
            continue;
        }

        if j % 8 == 0 {
            data = input[inbuf_idx];
            inbuf_idx += 1;
            p2 ^= (data.count_ones() & 1) as u8;
        }

        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i = i.wrapping_add(1);
    }

    // Compare against the parity bits stored in the lower 7 bits of `check`.
    p1 ^= check & 0x7f;

    if p1 != 0 {
        // Check overall message parity bit (top bit of `check`).
        let pm = check;
        p2 ^= (pm.count_ones() & 1) as u8;

        if p2 == 0 {
            // At least two errors -- cannot correct.
            output.truncate(start);
            return false;
        }

        // If p1 is not a power of two, the error is in a data bit (not a
        // parity bit) and we can locate and flip it. Powers of two indicate
        // an error in one of the parity bits themselves, which we can
        // simply ignore (data is already correct).
        if p1 & p1.wrapping_sub(1) != 0 {
            // Calculate error location in the data bit-stream.
            let pos: u8 = p1.wrapping_sub(log2uint8(p1)).wrapping_sub(2);
            let byte_idx = (pos / 8) as usize;
            let bit_idx = pos % 8;

            output[start + byte_idx] ^= 1u8 << bit_idx;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Matches the C implementation: 31 - __builtin_clz(v) on uint8_t
    // promoted to int. Behavior for x == 0 is undefined in the C version;
    // we return 0 to avoid panicking.
    if x == 0 {
        return 0;
    }
    7u8 - x.leading_zeros() as u8
}
