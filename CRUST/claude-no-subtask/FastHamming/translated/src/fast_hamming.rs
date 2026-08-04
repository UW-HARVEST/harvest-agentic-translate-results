use std::cmp;

/// Compute the parity (number of 1-bits mod 2) of a byte.
fn parity8(x: u8) -> u8 {
    (x.count_ones() & 1) as u8
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let _ = cmp::min(0usize, 0usize); // silence unused import
    let n = input.len();
    let full_blocks = n / 15;
    let partial = n % 15;

    let mut output: Vec<u8> = Vec::with_capacity(full_blocks * 16 + if partial > 0 { partial + 1 } else { 0 });

    for block in 0..full_blocks {
        let start = block * 15;
        let chunk: &[u8; 15] = (&input[start..start + 15]).try_into().unwrap();
        hecc_encode_impl(chunk, &mut output);
    }

    if partial > 0 {
        let rem = &input[full_blocks * 15..];
        // Pad with zeros to a full 15-byte block. Zero padding does not affect
        // parity calculations because:
        //   - For p1: extra bytes are zero, so no bits trigger XOR with i.
        //   - For p2: parity(0) = 0, no contribution.
        let mut padded = [0u8; 15];
        padded[..rem.len()].copy_from_slice(rem);

        let mut tmp: Vec<u8> = Vec::with_capacity(16);
        hecc_encode_impl(&padded, &mut tmp);

        // tmp = [15 data bytes (last few are zero padding), 1 parity byte]
        // We want only the actual data bytes followed by the parity byte.
        output.extend_from_slice(&tmp[..rem.len()]);
        output.push(tmp[15]);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let total_bits: usize = 15 * 8; // 120

    let mut byte_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;

    while j < total_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[byte_idx];
            byte_idx += 1;
            output.push(data);
            p2 ^= parity8(data);
        }

        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i += 1;
    }

    p2 ^= parity8(p1);

    output.push(p1 | (p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let n = input.len();

    // A trailing single byte cannot represent a valid (data + parity) block.
    if n % 16 == 1 {
        return None;
    }

    let full_blocks = n / 16;
    let partial = n % 16;

    let mut output: Vec<u8> = Vec::with_capacity(full_blocks * 15 + partial.saturating_sub(1));

    for block in 0..full_blocks {
        let start = block * 16;
        let chunk: &[u8; 16] = (&input[start..start + 16]).try_into().unwrap();
        if !hecc_decode_impl(chunk, &mut output) {
            return None;
        }
    }

    if partial > 0 {
        let rem = &input[full_blocks * 16..];
        if !decode_partial(rem, &mut output) {
            return None;
        }
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    decode_block(&input[..], output)
}

/// Helper used by `hecc_decode` for trailing partial blocks
/// (anything between 2 and 15 input bytes inclusive).
fn decode_partial(input: &[u8], output: &mut Vec<u8>) -> bool {
    decode_block(input, output)
}

/// Shared decoding logic for both full and partial blocks.
///
/// `input` has length `N`, where the first `N-1` bytes are data and the last
/// byte is the combined parity byte (`p1 | (p2 << 7)`).
fn decode_block(input: &[u8], output: &mut Vec<u8>) -> bool {
    if input.len() < 2 {
        return false;
    }

    let data_len = input.len() - 1;
    let check = input[data_len];

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let start_idx = output.len();
    output.extend_from_slice(&input[..data_len]);

    let total_bits = data_len * 8;
    let mut byte_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;

    while j < total_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[byte_idx];
            byte_idx += 1;
            p2 ^= parity8(data);
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
        let pm = check;
        p2 ^= parity8(pm);

        if p2 == 0 {
            // At least two errors: not correctable
            output.truncate(start_idx);
            return false;
        }

        // If p1 is not a power of two, the error is in the data; correct it.
        // (If p1 is a power of two, the error is in one of the parity bits
        // themselves, and the data is already fine.)
        if (p1 & p1.wrapping_sub(1)) != 0 {
            let err_bit_pos: u8 = p1 - log2uint8(p1) - 2;
            let byte_pos = (err_bit_pos / 8) as usize;
            let bit_in_byte = err_bit_pos % 8;
            output[start_idx + byte_pos] ^= 1u8 << bit_in_byte;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to the C version's `31 - __builtin_clz(v)` for nonzero v.
    // For a byte, floor(log2(v)) = 7 - (number of leading zeros in 8 bits).
    7 - x.leading_zeros() as u8
}
