/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chunks = input.chunks_exact(15);
    for chunk in chunks.by_ref() {
        let arr: &[u8; 15] = chunk.try_into().unwrap();
        hecc_encode_impl(arr, &mut output);
    }
    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let mut padded = [0u8; 15];
        padded[..remainder.len()].copy_from_slice(remainder);
        let len_before = output.len();
        hecc_encode_impl(&padded, &mut output);
        // The impl appended 16 bytes: 15 data bytes + 1 ECC byte.
        // For a partial block we want only `remainder.len()` data bytes
        // followed by the ECC byte (computed over the same bits, since
        // the padded zeros do not contribute to p1 or p2).
        let ecc_byte = output[len_before + 15];
        output.truncate(len_before + remainder.len());
        output.push(ecc_byte);
    }
    output
}
fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;
    let mut input_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
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

    p2 ^= (p1.count_ones() & 1) as u8;

    output.push(p1.wrapping_add(p2 << 7));
}
/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    // A trailing single byte indicates a truncated message.
    if input.len() % 16 == 1 {
        return None;
    }

    let mut output = Vec::new();
    let mut idx = 0;
    while idx + 16 <= input.len() {
        let block: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }

    if idx < input.len() {
        let remainder = &input[idx..];
        // remainder.len() is between 2 and 15 inclusive (already filtered out len%16 == 1)
        let data_len = remainder.len() - 1;

        // Pad the partial block to 16 bytes by placing the data at the start,
        // zero-padding, and putting the ECC byte at position 15. The encode
        // side guarantees that padding with zeros yields an equivalent ECC
        // value, so the impl can recover the original bytes correctly.
        let mut padded = [0u8; 16];
        padded[..data_len].copy_from_slice(&remainder[..data_len]);
        padded[15] = remainder[data_len];

        let len_before = output.len();
        if !hecc_decode_impl(&padded, &mut output) {
            return None;
        }
        // The impl wrote 15 bytes; keep only the first `data_len`.
        output.truncate(len_before + data_len);
    }

    Some(output)
}
fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let check = input[15];

    let len_before = output.len();
    output.extend_from_slice(&input[..15]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;

    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits: usize = 15 * 8;
    let mut input_idx: usize = 0;

    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
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
        // Check overall message parity to distinguish single vs. double errors.
        let pm = check;
        p2 ^= (pm.count_ones() & 1) as u8;

        if p2 == 0 {
            // At least two errors were detected; cannot correct.
            output.truncate(len_before);
            return false;
        }

        // A power-of-two p1 means the error landed on one of the parity bits,
        // so the data is intact and no correction is required.
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate the data bit position that flipped.
            let pos = p1.wrapping_sub(log2uint8(p1)).wrapping_sub(2) as usize;
            let byte_idx = pos / 8;
            let bit_idx = pos % 8;

            output[len_before + byte_idx] ^= 1u8 << bit_idx;
        }
    }

    true
}
pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to `31 - __builtin_clz((unsigned)x)` for x > 0.
    // For x == 0 the C builtin is undefined; mirror that by returning 0.
    if x == 0 {
        return 0;
    }
    7 - x.leading_zeros() as u8
}
