/// Returns 1 if `x` has odd number of set bits, else 0 (mirrors GCC `__builtin_parity`).
fn parity_u8(x: u8) -> u8 {
    (x.count_ones() & 1) as u8
}

/// Generic Hamming-ECC block encode helper.
///
/// Reads `input.len()` data bytes and pushes `input.len() + 1` bytes
/// (the data followed by the parity byte) to `output`.
fn encode_block(input: &[u8], output: &mut Vec<u8>) {
    let insize = input.len();
    let total_bits = insize * 8;

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let mut inbuf_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < total_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[inbuf_idx];
            output.push(data);
            inbuf_idx += 1;
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

/// Generic Hamming-ECC block decode helper.
///
/// Reads `input.len()` bytes (last byte is the ECC/parity byte) and pushes
/// `input.len() - 1` decoded data bytes to `output`. Performs single-bit
/// error correction. Returns `false` and leaves `output` unchanged (rolled
/// back to its original length) on uncorrectable errors.
fn decode_block(input: &[u8], output: &mut Vec<u8>) -> bool {
    let insize = input.len();
    if insize < 2 {
        return false;
    }

    let data_size = insize - 1;
    let check = input[data_size];

    let start_pos = output.len();
    output.extend_from_slice(&input[..data_size]);

    let total_bits = data_size * 8;

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;

    let mut inbuf_idx: usize = 0;
    let mut i: usize = 3;
    let mut j: usize = 0;
    while j < total_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }

        if j % 8 == 0 {
            data = input[inbuf_idx];
            inbuf_idx += 1;
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
        // Check overall parity
        let pm = check;
        p2 ^= parity_u8(pm);

        if p2 == 0 {
            // At least two errors -> uncorrectable
            output.truncate(start_pos);
            return false;
        }

        // Non-power-of-two p1 indicates a data-bit error to correct.
        if (p1 & p1.wrapping_sub(1)) != 0 {
            let loc = p1.wrapping_sub(log2uint8(p1)).wrapping_sub(2);
            let byte_idx = (loc / 8) as usize;
            let bit_idx = loc % 8;

            if byte_idx < data_size {
                output[start_pos + byte_idx] ^= 1 << bit_idx;
            }
        }
        // If p1 is a power of two, the error is in a parity bit; data is fine.
    }

    true
}

/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() + (input.len() + 14) / 15);

    let mut idx = 0;
    while input.len() - idx >= 15 {
        let block: &[u8; 15] = (&input[idx..idx + 15]).try_into().unwrap();
        hecc_encode_impl(block, &mut output);
        idx += 15;
    }

    let remaining = input.len() - idx;
    if remaining > 0 {
        // Encode the partial block; only `remaining + 1` bytes are emitted.
        encode_block(&input[idx..], &mut output);
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    encode_block(input.as_slice(), output);
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    // A trailing single byte cannot constitute a valid block (need data + parity).
    if input.len() % 16 == 1 {
        return None;
    }

    let mut output = Vec::with_capacity(input.len());

    let mut idx = 0;
    while input.len() - idx >= 16 {
        let block: &[u8; 16] = (&input[idx..idx + 16]).try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        idx += 16;
    }

    let remaining = input.len() - idx;
    if remaining > 0 {
        if !decode_block(&input[idx..], &mut output) {
            return None;
        }
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    decode_block(input.as_slice(), output)
}

pub fn log2uint8(x: u8) -> u8 {
    // Mirrors `31 - __builtin_clz(v)` from the C reference.
    // Undefined for x == 0 (matches the C version).
    7u8.wrapping_sub(x.leading_zeros() as u8)
}
