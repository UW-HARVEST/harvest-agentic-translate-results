#[allow(unused_imports)]
use std::cmp;

/// Computes the parity (1 if odd number of set bits, else 0) of a byte.
fn parity_u8(x: u8) -> u8 {
    let mut x = x;
    x ^= x >> 4;
    x ^= x >> 2;
    x ^= x >> 1;
    x & 1
}

/// Encodes a chunk of `input.len()` bytes (1..=15) into the output, appending
/// `input.len() + 1` bytes (the data followed by a parity byte).
fn encode_chunk(input: &[u8], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let bits = input.len() * 8;
    let mut byte_idx: usize = 0;
    let mut j: usize = 0;
    let mut i: usize = 3;

    while j < bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[byte_idx];
            output.push(data);
            byte_idx += 1;
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
    output.push(p1 | (p2 << 7));
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let len = input.len();
    let n_full = len / 15;
    let rem = len % 15;
    let total = n_full * 16 + if rem == 0 { 0 } else { rem + 1 };
    let mut output = Vec::with_capacity(total);

    let mut idx = 0;
    while idx + 15 <= len {
        let arr: &[u8; 15] = (&input[idx..idx + 15]).try_into().unwrap();
        hecc_encode_impl(arr, &mut output);
        idx += 15;
    }

    if idx < len {
        encode_chunk(&input[idx..len], &mut output);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    encode_chunk(input.as_slice(), output);
}

/// Decodes a chunk of `input.len()` bytes (2..=16). The last byte is the parity
/// byte and the first `input.len() - 1` bytes are the data bytes (which are
/// appended to `output`, possibly after applying single-bit error correction).
/// Returns `false` if a non-correctable double-bit error is detected; in that
/// case any partial output for this chunk is discarded.
fn decode_chunk(input: &[u8], output: &mut Vec<u8>) -> bool {
    let n = input.len();
    if n < 2 {
        return false;
    }

    let data_len = n - 1;
    let check = input[data_len];

    let start = output.len();
    output.extend_from_slice(&input[..data_len]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let bits = data_len * 8;
    let mut byte_idx: usize = 0;
    let mut j: usize = 0;
    let mut i: usize = 3;

    while j < bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[byte_idx];
            byte_idx += 1;
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
        // Check overall message parity.
        p2 ^= parity_u8(check);

        if p2 == 0 {
            // At least two errors: not correctable.
            output.truncate(start);
            return false;
        }

        // If p1 is not a power of two, the error is in the data bits.
        if p1 & p1.wrapping_sub(1) != 0 {
            let bit_index = p1 - log2uint8(p1) - 2;
            let byte_pos = (bit_index / 8) as usize;
            let bit_pos = bit_index % 8;
            output[start + byte_pos] ^= 1 << bit_pos;
        }
        // Otherwise (p1 is a power of two), the error is in a parity bit
        // and the data is unaffected.
    }

    true
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let len = input.len();

    // A trailing single byte indicates a truncated message.
    if len % 16 == 1 {
        return None;
    }

    let n_full = len / 16;
    let rem = len % 16;
    let total = n_full * 15 + if rem == 0 { 0 } else { rem - 1 };
    let mut output = Vec::with_capacity(total);

    let mut idx = 0;
    while idx + 16 <= len {
        let arr: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(arr, &mut output) {
            return None;
        }
        idx += 16;
    }

    if idx < len {
        if !decode_chunk(&input[idx..len], &mut output) {
            return None;
        }
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    decode_chunk(input.as_slice(), output)
}

pub fn log2uint8(x: u8) -> u8 {
    if x == 0 {
        // Mirrors C's undefined-behavior-on-zero; return 0 for safety.
        return 0;
    }
    (u8::BITS - 1 - x.leading_zeros()) as u8
}
