/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let insize = input.len();

    // Overflow check: if insize > (usize::MAX >> 4) * 15
    if insize > (usize::MAX >> 4).saturating_mul(15) {
        return Vec::new();
    }

    let reqsize = (insize + 14) / 15 + insize;
    let mut output = Vec::with_capacity(reqsize);
    let mut pos = 0;

    while pos + 15 <= insize {
        let block: &[u8; 15] = input[pos..pos + 15].try_into().unwrap();
        hecc_encode_impl(block, &mut output);
        pos += 15;
    }

    if pos < insize {
        let mut buf = [0u8; 15];
        let remaining = insize - pos;
        buf[..remaining].copy_from_slice(&input[pos..]);
        // Encode using remaining actual size
        let mut p1: u8 = 0;
        let mut p2: u8 = 0;
        let mut power2: usize = 4;
        let mut data: u8 = 0;
        let total_bits = remaining * 8;
        let mut src_idx = 0;
        let mut j: usize = 0;
        let mut i: usize = 3;
        while j < total_bits {
            if power2 == i {
                power2 *= 2;
                i += 1;
                continue;
            }
            if j % 8 == 0 {
                data = buf[src_idx];
                output.push(data);
                src_idx += 1;
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
        output.push(p1.wrapping_add(p2 << 7));
    }

    output
}

fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let total_bits = 15 * 8;
    let mut src_idx = 0;
    let mut j: usize = 0;
    let mut i: usize = 3;

    while j < total_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[src_idx];
            output.push(data);
            src_idx += 1;
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
    output.push(p1.wrapping_add(p2 << 7));
}

/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();

    if insize % 16 == 1 {
        return None;
    }

    let reqsize = insize - (insize / 16 + if insize % 16 != 0 { 1 } else { 0 });
    let mut output = Vec::with_capacity(reqsize);
    let mut pos = 0;

    while pos + 16 <= insize {
        let block: &[u8; 16] = input[pos..pos + 16].try_into().unwrap();
        if !hecc_decode_impl(block, &mut output) {
            return None;
        }
        pos += 16;
    }

    if pos < insize {
        let remaining = insize - pos;
        let mut buf = [0u8; 16];
        buf[..remaining].copy_from_slice(&input[pos..]);

        if remaining < 2 {
            return None;
        }

        let data_len = remaining - 1;
        let check = buf[data_len];
        let mut outbuf = [0u8; 15];
        outbuf[..data_len].copy_from_slice(&buf[..data_len]);

        let mut p1: u8 = 0;
        let mut p2: u8 = 0;
        let mut power2: usize = 4;
        let mut data: u8 = 0;
        let total_bits = data_len * 8;
        let mut src_idx = 0;
        let mut j: usize = 0;
        let mut i: usize = 3;

        while j < total_bits {
            if power2 == i {
                power2 *= 2;
                i += 1;
                continue;
            }
            if j % 8 == 0 {
                data = buf[src_idx];
                src_idx += 1;
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
            p2 ^= parity(check);

            if p2 == 0 {
                return None;
            }

            if (p1 & (p1 - 1)) != 0 {
                p2 = p1 - log2uint8(p1) - 2;
                let byte_idx = (p2 / 8) as usize;
                let bit_idx = p2 % 8;
                outbuf[byte_idx] ^= 1 << bit_idx;
            }
        }

        for idx in 0..data_len {
            output.push(outbuf[idx]);
        }
    }

    Some(output)
}

fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let data_len = 15; // 16 - 1
    let check = input[15];
    let mut outbuf = [0u8; 15];
    outbuf.copy_from_slice(&input[..15]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let total_bits = data_len * 8;
    let mut src_idx = 0;
    let mut j: usize = 0;
    let mut i: usize = 3;

    while j < total_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[src_idx];
            src_idx += 1;
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
        let pm = check;
        p2 ^= parity(pm);

        if p2 == 0 {
            return false;
        }

        if (p1 & (p1 - 1)) != 0 {
            p2 = p1 - log2uint8(p1) - 2;
            let byte_idx = (p2 / 8) as usize;
            let bit_idx = p2 % 8;
            outbuf[byte_idx] ^= 1 << bit_idx;
        }
    }

    output.extend_from_slice(&outbuf);
    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to 31 - __builtin_clz(v) for u8 values
    // __builtin_clz operates on unsigned int (32-bit), so for a u8 value v,
    // 31 - clz32(v) = 7 - clz8(v) = 7 - (v as u8).leading_zeros() as u8
    7 - (x.leading_zeros() as u8)
}

fn parity(v: u8) -> u8 {
    (v.count_ones() & 1) as u8
}
