#[inline]
fn parity(x: u8) -> u8 {
    (x.count_ones() & 1) as u8
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();
    let mut idx = 0usize;
    let total = input.len();

    while idx + 15 <= total {
        let mut arr = [0u8; 15];
        arr.copy_from_slice(&input[idx..idx + 15]);
        hecc_encode_impl(&arr, &mut output);
        idx += 15;
    }

    let remaining = total - idx;
    if remaining > 0 {
        let mut arr = [0u8; 15];
        arr[..remaining].copy_from_slice(&input[idx..idx + remaining]);
        let prev_len = output.len();
        hecc_encode_impl(&arr, &mut output);
        // Impl appended 16 bytes: 15 data bytes (last 15-remaining are zero) + 1 parity byte.
        // We want to keep only `remaining` data bytes + the parity byte.
        let parity_byte = output[prev_len + 15];
        output.truncate(prev_len + remaining);
        output.push(parity_byte);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let bits: usize = 15 * 8;

    let mut input_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;

    while j < bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[input_idx];
            input_idx += 1;
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

    output.push(p1 | (p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let total = input.len();
    if total % 16 == 1 {
        return None;
    }

    let mut output: Vec<u8> = Vec::new();
    let mut idx: usize = 0;

    while idx + 16 <= total {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&input[idx..idx + 16]);
        if !hecc_decode_impl(&arr, &mut output) {
            return None;
        }
        idx += 16;
    }

    let remaining = total - idx;
    if remaining > 0 {
        // Partial block of `remaining` bytes (2..=15).
        // Layout: data bytes at positions 0..remaining-1, zero padding,
        // check byte at position 15.
        let mut arr = [0u8; 16];
        arr[..remaining - 1].copy_from_slice(&input[idx..idx + remaining - 1]);
        arr[15] = input[idx + remaining - 1];
        let prev_len = output.len();
        if !hecc_decode_impl(&arr, &mut output) {
            return None;
        }
        // Impl appended 15 bytes: first remaining-1 bytes are real data,
        // the rest are zero padding. Keep only the real data bytes.
        output.truncate(prev_len + remaining - 1);
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let check = input[15];

    let prev_len = output.len();
    for k in 0..15 {
        output.push(input[k]);
    }

    let bits: usize = 15 * 8;
    let mut input_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;

    while j < bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[input_idx];
            input_idx += 1;
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
        // Check overall message parity
        let pm = check;
        p2 ^= parity(pm);

        if p2 == 0 {
            // At least two errors - cannot correct
            output.truncate(prev_len);
            return false;
        }

        // Check for non-power of two in p1:
        // A power of two means one of the parity bits is wrong (no data correction needed)
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate error location
            let pos = p1.wrapping_sub(log2uint8(p1)).wrapping_sub(2);
            let pm_idx = (pos / 8) as usize;
            let bit_pos = (pos % 8) as u32;

            // Actually fix that error
            output[prev_len + pm_idx] ^= 1u8 << bit_pos;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to 31 - __builtin_clz(v) for u8 (promoted to int).
    // For u8 input, leading_zeros() returns 0..=8.
    // For x >= 1, 7 - x.leading_zeros() gives floor(log2(x)).
    7u8.wrapping_sub(x.leading_zeros() as u8)
}
