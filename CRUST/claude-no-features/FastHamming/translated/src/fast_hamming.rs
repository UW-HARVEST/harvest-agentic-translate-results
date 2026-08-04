use std::cmp;

/// Returns 1 if the byte has an odd number of set bits, 0 otherwise.
fn parity_u8(x: u8) -> u8 {
    (x.count_ones() & 1) as u8
}

/// Variable-length encoding helper. `input.len()` gives the data byte count
/// (1..=15). Pushes `input.len() + 1` bytes into `output`: the original data
/// bytes followed by a parity/check byte.
fn encode_block(input: &[u8], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let bit_count = input.len() * 8;
    let mut input_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < bit_count {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[input_idx];
            input_idx += 1;
            output.push(data);
            p2 ^= parity_u8(data);
        }

        if data & 1 == 1 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i += 1;
    }

    p2 ^= parity_u8(p1);
    output.push(p1 + (p2 << 7));
}

/// Variable-length decoding helper. Decodes a block consisting of
/// `input.len() - 1` data bytes followed by 1 parity byte. Pushes the decoded
/// data bytes (with at most one bit corrected) onto `output`. Returns false
/// if uncorrectable (two-bit) errors are detected.
fn decode_block(input: &[u8], output: &mut Vec<u8>) -> bool {
    let insize = input.len();
    if insize < 2 {
        return false;
    }

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let data_len = insize - 1;
    let check = input[data_len];

    let start_idx = output.len();
    for i in 0..data_len {
        output.push(input[i]);
    }

    let bit_count = data_len * 8;
    let mut input_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < bit_count {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[input_idx];
            input_idx += 1;
            p2 ^= parity_u8(data);
        }

        if data & 1 == 1 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i += 1;
    }

    p1 ^= check & 0x7f;

    if p1 != 0 {
        // Check overall message parity (including the check byte itself)
        let pm = check;
        p2 ^= parity_u8(pm);

        if p2 == 0 {
            // At least two errors
            return false;
        }

        // Check for non-power-of-two in p1: a power of two means one of the
        // parity bits is wrong, which the receiver doesn't need to fix.
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate error location
            let loc = p1 - log2uint8(p1) - 2;
            let byte_off = (loc / 8) as usize;
            let bit_off = loc % 8;

            output[start_idx + byte_off] ^= 1 << bit_off;
        }
    }

    true
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let full_blocks = input.len() / 15;
    let remainder = input.len() % 15;
    let total_blocks = full_blocks + if remainder != 0 { 1 } else { 0 };
    let req_size = input.len() + total_blocks;

    let mut output = Vec::with_capacity(req_size);

    let mut idx: usize = 0;
    for _ in 0..full_blocks {
        let block: &[u8; 15] = (&input[idx..idx + 15]).try_into().unwrap();
        hecc_encode_impl(block, &mut output);
        idx += 15;
    }

    if remainder > 0 {
        // Partial block: encode using the variable-length helper directly so
        // that parity is computed only over the actual data bits.
        encode_block(&input[idx..], &mut output);
    }

    // Silence unused-import warning if cmp ever becomes unused.
    let _ = cmp::min(0usize, 0usize);

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    encode_block(input, output);
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    // A trailing single byte cannot be a valid block (need at least 1 data
    // byte + 1 check byte).
    if input.len() % 16 == 1 {
        return None;
    }

    let full_blocks = input.len() / 16;
    let has_partial = input.len() % 16 != 0;
    let total_blocks = full_blocks + if has_partial { 1 } else { 0 };
    let req_size = input.len() - total_blocks;

    let mut output = Vec::with_capacity(req_size);

    let mut idx: usize = 0;
    for _ in 0..full_blocks {
        let block: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }

    if has_partial {
        if !decode_block(&input[idx..], &mut output) {
            return None;
        }
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    decode_block(input, output)
}

pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to `31 - __builtin_clz(v)` for v > 0. For v == 0 the C version
    // is undefined; we mirror that by returning 0xff (saturating) which is
    // never reached by callers in practice.
    if x == 0 {
        return 0xff;
    }
    7 - x.leading_zeros() as u8
}
