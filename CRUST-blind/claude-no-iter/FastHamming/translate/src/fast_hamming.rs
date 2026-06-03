use std::cmp;

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let _ = cmp::min(0, 0); // touch import
    let insize = input.len();

    // Compute reqsize: insize + ceil(insize / 15) ECC bytes
    let reqsize = insize + (insize + 14) / 15;
    let mut output: Vec<u8> = Vec::with_capacity(reqsize);

    let mut idx = 0usize;
    // Full blocks
    while idx + 15 <= insize {
        let block: &[u8; 15] = (&input[idx..idx + 15]).try_into().unwrap();
        hecc_encode_impl(block, &mut output);
        idx += 15;
    }

    // Partial block
    if idx < insize {
        let n = insize - idx;
        let mut buf = [0u8; 15];
        buf[..n].copy_from_slice(&input[idx..]);

        let start = output.len();
        hecc_encode_impl(&buf, &mut output);
        // After impl: output now has 16 new bytes appended:
        //   buf[0..15] (which is input[idx..idx+n] followed by zeros) plus parity
        // We want only n data bytes followed by parity.
        let parity = output[start + 15];
        output.truncate(start + n);
        output.push(parity);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8; // 120

    let mut input_idx: usize = 0;
    let parity_pos = output.len() + 15; // position where parity will go

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 = power2.wrapping_mul(2);
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[input_idx];
            output.push(data);
            input_idx += 1;
            p2 ^= (data.count_ones() & 1) as u8;
        }

        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;

        j += 1;
        i += 1;
    }

    // p2 ^= parity(p1)
    p2 ^= (p1.count_ones() & 1) as u8;

    // Push parity byte: p1 + (p2 << 7). Since p1 < 128, this is also p1 | (p2<<7).
    let parity = p1.wrapping_add(p2 << 7);
    debug_assert_eq!(output.len(), parity_pos);
    output.push(parity);
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();

    // A trailing single byte indicates a truncated message
    if insize % 16 == 1 {
        return None;
    }

    // reqsize = insize - (number of ECC bytes)
    // number of ECC bytes = full_blocks + (1 if partial else 0)
    let full_blocks = insize / 16;
    let has_partial = insize % 16 != 0;
    let ecc_count = full_blocks + if has_partial { 1 } else { 0 };
    let reqsize = insize - ecc_count;

    let mut output: Vec<u8> = Vec::with_capacity(reqsize);

    let mut idx = 0usize;

    // Full blocks
    while idx + 16 <= insize {
        let block: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }

    // Partial block
    if idx < insize {
        let m = insize - idx; // m is in [2, 15] since we rejected m == 1
        let mut buf = [0u8; 16];
        // m-1 data bytes from partial input
        buf[..m - 1].copy_from_slice(&input[idx..idx + m - 1]);
        // buf[m-1..15] remains 0 (zero padding for parity calculations)
        // check byte goes at position 15
        buf[15] = input[idx + m - 1];

        let start = output.len();
        if !hecc_decode_impl(&buf, &mut output) {
            return None;
        }
        // hecc_decode_impl appended 15 bytes (the data part) to output.
        // We want only m-1 of them (the actual data bytes).
        output.truncate(start + m - 1);
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let check = input[15];

    // Append the 15 data bytes to output
    let start = output.len();
    output.extend_from_slice(&input[0..15]);

    let insize_bits: usize = 15 * 8;

    let mut input_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 = power2.wrapping_mul(2);
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[input_idx];
            input_idx += 1;
            p2 ^= (data.count_ones() & 1) as u8;
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
        // Overall parity check
        let pm = check;
        p2 ^= (pm.count_ones() & 1) as u8;

        if p2 == 0 {
            // At least two errors
            return false;
        }

        // If p1 is not a power of two, the error is in a data bit
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate error location
            let loc = p1 - log2uint8(p1) - 2;
            let pm_byte = (loc / 8) as usize;
            let p2_bit = loc % 8;

            // Fix the bit error in the appended data
            output[start + pm_byte] ^= 1u8 << p2_bit;
        }
        // If p1 is a power of two, the error is in one of the parity bits
        // within the check byte; data is fine, nothing to correct.
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // 31 - __builtin_clz(v) for an unsigned int v == position of highest set bit.
    // For u8, that's 7 - leading_zeros(u8).
    // __builtin_clz(0) is undefined behaviour in C; we return 0 to be safe.
    if x == 0 {
        return 0;
    }
    7 - x.leading_zeros() as u8
}
