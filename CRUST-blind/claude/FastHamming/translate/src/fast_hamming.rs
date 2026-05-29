use std::cmp;
/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();
    let len = input.len();
    let mut idx: usize = 0;

    // Process full 15-byte blocks.
    while idx + 15 <= len {
        let block: &[u8; 15] = (&input[idx..idx + 15]).try_into().unwrap();
        hecc_encode_impl(block, &mut output);
        idx += 15;
    }

    // Process trailing partial block (if any).
    if idx < len {
        let remaining = len - idx;
        let mut buf = [0u8; 15];
        for i in 0..remaining {
            buf[i] = input[idx + i];
        }
        encode_block_internal(&buf, remaining, &mut output);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    encode_block_internal(input, 15, output);
}

/// Hamming ECC encoder for one block. Reads `data_bytes` bytes from `buf`
/// (must be at most 15) and appends `data_bytes + 1` bytes to `output`:
/// the original data bytes followed by a parity byte.
fn encode_block_internal(buf: &[u8; 15], data_bytes: usize, output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits = data_bytes * 8;

    let mut inbuf_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 = power2.wrapping_mul(2);
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = buf[inbuf_idx];
            output.push(data);
            inbuf_idx += 1;
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

    output.push(p1.wrapping_add(p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();
    if insize % 16 == 1 {
        // truncated message: a partial block must contain at least 2 bytes
        return None;
    }

    let mut output: Vec<u8> = Vec::new();
    let mut idx: usize = 0;

    // Process full 16-byte blocks.
    while idx + 16 <= insize {
        let block: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }

    // Process trailing partial block (if any).
    if idx < insize {
        let remaining = insize - idx;
        if !decode_block_internal(&input[idx..idx + remaining], &mut output) {
            return None;
        }
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    decode_block_internal(&input[..], output)
}

/// Hamming ECC decoder for one block. The last byte of `input` is the parity
/// byte; the rest are data. On success, appends `input.len() - 1` (possibly
/// corrected) data bytes to `output` and returns true. Returns false if the
/// block has too few bytes or if an unrecoverable error is detected.
fn decode_block_internal(input: &[u8], output: &mut Vec<u8>) -> bool {
    let insize = input.len();
    if insize < 2 {
        return false;
    }

    let data_bytes = insize - 1;
    let check = input[data_bytes];

    // Copy data bytes into a working buffer (zero-padded to 15 bytes).
    let mut buf = [0u8; 15];
    for i in 0..data_bytes {
        buf[i] = input[i];
    }

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let insize_bits = data_bytes * 8;

    let mut inbuf_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < insize_bits {
        if power2 == i {
            power2 = power2.wrapping_mul(2);
            i += 1;
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
        i += 1;
    }

    p1 ^= check & 0x7f;

    if p1 != 0 {
        // Mix in the overall message parity.
        p2 ^= (check.count_ones() & 1) as u8;

        if p2 == 0 {
            // At least two errors – cannot correct.
            return false;
        }

        // A power of two in p1 indicates one of the parity bits is wrong;
        // any other value indicates a single data-bit error we can fix.
        if (p1 & p1.wrapping_sub(1)) != 0 {
            // Calculate the error location (bit offset within the data area).
            let err_pos = p1.wrapping_sub(log2uint8(p1)).wrapping_sub(2);
            let pm_byte = (err_pos / 8) as usize;
            let p2_bit = err_pos % 8;

            // Bound the write to the buffer for safety. A correctly computed
            // syndrome from a single bit-flip in a properly-sized block will
            // always fall within the data area; the bounds check protects
            // against pathological inputs.
            if pm_byte < buf.len() {
                buf[pm_byte] ^= 1u8 << p2_bit;
            }
        }
    }

    // Emit the (possibly corrected) data bytes.
    for k in 0..data_bytes {
        output.push(buf[k]);
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Returns floor(log2(x)) for x > 0. For x == 0, behavior in C is
    // undefined; return 0 here to keep the function total and panic-free.
    if x == 0 {
        return 0;
    }
    // Mirror the C: 31 - __builtin_clz(v) where v is promoted to int.
    // For a non-zero u8, this equals 7 - leading_zeros(v as u8).
    let _ = cmp::max(0u8, 0u8); // keep `cmp` import live
    7u8 - x.leading_zeros() as u8
}
