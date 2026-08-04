/// Encodes the input buffer using Hamming ECC encoding.
pub fn hecc_encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + (input.len() + 14) / 15);
    let mut remaining = input;

    while remaining.len() >= 15 {
        let mut block = [0u8; 15];
        block.copy_from_slice(&remaining[..15]);
        hecc_encode_impl(&block, &mut out);
        remaining = &remaining[15..];
    }

    if !remaining.is_empty() {
        let mut block = [0u8; 15];
        block[..remaining.len()].copy_from_slice(remaining);
        // Encode with full block but only push insize bytes + 1 parity
        let mut p1: u8 = 0;
        let mut p2: u8 = 0;
        let mut power2: usize = 4;
        let mut data: u8 = 0;
        let insize_bits = remaining.len() * 8;
        let mut src_idx = 0usize;
        let mut j = 0usize;
        let mut i = 3usize;
        while j < insize_bits {
            if power2 == i {
                power2 *= 2;
                i += 1;
                continue;
            }
            if j % 8 == 0 {
                data = block[src_idx];
                out.push(data);
                p2 ^= (data.count_ones() & 1) as u8;
                src_idx += 1;
            }
            if data & 1 != 0 {
                p1 ^= i as u8;
            }
            data >>= 1;
            j += 1;
            i += 1;
        }
        p2 ^= (p1.count_ones() & 1) as u8;
        out.push(p1 + (p2 << 7));
    }

    out
}
fn hecc_encode_impl(input: &[u8; 15], output: &mut Vec<u8>) {
    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let insize_bits = 15 * 8;
    let mut src_idx = 0usize;
    let mut j = 0usize;
    let mut i = 3usize;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[src_idx];
            output.push(data);
            p2 ^= (data.count_ones() & 1) as u8;
            src_idx += 1;
        }
        if data & 1 != 0 {
            p1 ^= i as u8;
        }
        data >>= 1;
        j += 1;
        i += 1;
    }
    p2 ^= (p1.count_ones() & 1) as u8;
    output.push(p1 + (p2 << 7));
}
/// Decodes the input buffer using Hamming ECC decoding.
pub fn hecc_decode(input: &[u8]) -> Option<Vec<u8>> {
    let insize = input.len();
    if insize % 16 == 1 {
        return None;
    }

    let reqsize = {
        let full_blocks = insize / 16;
        let partial = if insize % 16 != 0 { 1 } else { 0 };
        insize - full_blocks - partial
    };

    let mut out = Vec::with_capacity(reqsize);
    let mut remaining = input;

    while remaining.len() >= 16 {
        let mut block = [0u8; 16];
        block.copy_from_slice(&remaining[..16]);
        if !hecc_decode_impl(&block, &mut out) {
            return None;
        }
        remaining = &remaining[16..];
    }

    if !remaining.is_empty() {
        let mut block = [0u8; 16];
        block[..remaining.len()].copy_from_slice(remaining);
        let mut temp = Vec::new();
        if !hecc_decode_impl_partial(&block, &mut temp, remaining.len()) {
            return None;
        }
        out.extend_from_slice(&temp[..remaining.len() - 1]);
    }

    Some(out)
}
fn hecc_decode_impl(input: &[u8; 16], output: &mut Vec<u8>) -> bool {
    let insize: usize = 15; // data bytes (16 - 1 for check byte)
    let start = output.len();
    output.extend_from_slice(&input[..insize]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let check = input[insize];
    let insize_bits = insize * 8;
    let mut src_idx = 0usize;
    let mut j = 0usize;
    let mut i = 3usize;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[src_idx];
            p2 ^= (data.count_ones() & 1) as u8;
            src_idx += 1;
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
        p2 ^= (pm.count_ones() & 1) as u8;

        if p2 == 0 {
            // At least two errors
            output.truncate(start);
            return false;
        }

        // Check for non-power of two in p1
        if (p1 & (p1 - 1)) != 0 {
            let bit_pos = p1 - log2uint8(p1) - 2;
            let byte_idx = (bit_pos / 8) as usize;
            let bit_idx = bit_pos % 8;
            output[start + byte_idx] ^= 1 << bit_idx;
        }
    }

    true
}

fn hecc_decode_impl_partial(input: &[u8; 16], output: &mut Vec<u8>, block_len: usize) -> bool {
    if block_len < 2 {
        return false;
    }
    let insize = block_len - 1; // data bytes
    let start = output.len();
    output.extend_from_slice(&input[..insize]);

    let mut p1: u8 = 0;
    let mut p2: u8 = 0;
    let mut power2: usize = 4;
    let mut data: u8 = 0;
    let check = input[insize];
    let insize_bits = insize * 8;
    let mut src_idx = 0usize;
    let mut j = 0usize;
    let mut i = 3usize;
    while j < insize_bits {
        if power2 == i {
            power2 *= 2;
            i += 1;
            continue;
        }
        if j % 8 == 0 {
            data = input[src_idx];
            p2 ^= (data.count_ones() & 1) as u8;
            src_idx += 1;
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
        p2 ^= (pm.count_ones() & 1) as u8;

        if p2 == 0 {
            output.truncate(start);
            return false;
        }

        if (p1 & (p1 - 1)) != 0 {
            let bit_pos = p1 - log2uint8(p1) - 2;
            let byte_idx = (bit_pos / 8) as usize;
            let bit_idx = bit_pos % 8;
            output[start + byte_idx] ^= 1 << bit_idx;
        }
    }

    true
}

pub fn log2uint8(x: u8) -> u8 {
    // Equivalent to 31 - __builtin_clz(v) for a u8 value
    // __builtin_clz operates on unsigned int (32-bit), so for u8 v:
    // 31 - clz32(v) = 7 - clz8(v)
    7 - x.leading_zeros() as u8
}
