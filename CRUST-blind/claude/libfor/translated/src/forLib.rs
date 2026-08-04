pub const METADATA: i32 = 5;

// ---------- Internal bit-packing helpers (shared across functions) ----------

/// Compute the number of bits required to store `v`. 0 returns 0; otherwise
/// floor(log2(v)) + 1.
fn req_bits(v: u32) -> u32 {
    if v == 0 { 0 } else { 32 - v.leading_zeros() }
}

/// Read a 32-bit little-endian word from `input` at byte offset `off`.
/// Bytes past the end are treated as 0 (defensive).
fn read_u32_le(input: &[u8], off: usize) -> u32 {
    let b0 = *input.get(off).unwrap_or(&0);
    let b1 = *input.get(off + 1).unwrap_or(&0);
    let b2 = *input.get(off + 2).unwrap_or(&0);
    let b3 = *input.get(off + 3).unwrap_or(&0);
    u32::from_le_bytes([b0, b1, b2, b3])
}

/// Pack `length` values, each `bits` bits, into `output` starting at byte 0.
/// Returns total bytes written.
fn pack_bitstream(input: &[u32], output: &mut [u8], length: u32, base: u32, bits: u32) -> u32 {
    if bits == 0 || length == 0 {
        return 0;
    }
    let length_us = length as usize;

    if bits == 32 {
        for i in 0..length_us {
            let v = input[i].wrapping_sub(base);
            let off = i * 4;
            output[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
        return (length * 4) as u32;
    }

    let total_bits = length_us * bits as usize;
    let total_bytes = (total_bits + 7) / 8;

    // Clear the bytes we will write so OR-in works correctly.
    for k in 0..total_bytes {
        if k < output.len() {
            output[k] = 0;
        }
    }

    let mask: u64 = (1u64 << bits) - 1;

    for i in 0..length_us {
        let v = (input[i].wrapping_sub(base) as u64) & mask;
        let bit_pos = i * bits as usize;
        let byte_off = bit_pos / 8;
        let bit_off = bit_pos % 8;
        let total = bits as usize + bit_off;
        let bytes_to_write = (total + 7) / 8;
        let shifted: u64 = v << bit_off;

        for k in 0..bytes_to_write {
            let idx = byte_off + k;
            if idx < output.len() {
                output[idx] |= ((shifted >> (k * 8)) & 0xFF) as u8;
            }
        }
    }

    total_bytes as u32
}

/// Unpack `length` values from a bit stream in `input` starting at byte 0.
/// Returns total bytes read.
fn unpack_bitstream(input: &[u8], output: &mut [u32], length: u32, base: u32, bits: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let length_us = length as usize;

    if bits == 0 {
        for i in 0..length_us {
            output[i] = base;
        }
        return 0;
    }

    if bits == 32 {
        for i in 0..length_us {
            let off = i * 4;
            let v = read_u32_le(input, off);
            output[i] = base.wrapping_add(v);
        }
        return (length * 4) as u32;
    }

    let total_bits = length_us * bits as usize;
    let total_bytes = (total_bits + 7) / 8;
    let mask: u64 = (1u64 << bits) - 1;

    for i in 0..length_us {
        let bit_pos = i * bits as usize;
        let byte_off = bit_pos / 8;
        let bit_off = bit_pos % 8;
        let total = bits as usize + bit_off;
        let bytes_to_read = (total + 7) / 8;
        let mut acc: u64 = 0;
        for k in 0..bytes_to_read {
            let idx = byte_off + k;
            if idx < input.len() {
                acc |= (input[idx] as u64) << (k * 8);
            }
        }
        let v = ((acc >> bit_off) & mask) as u32;
        output[i] = base.wrapping_add(v);
    }

    total_bytes as u32
}

// ---------- Public API ----------

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    let mut c: u32 = 0;
    let mut len = length;

    if len >= 32 {
        let b = len / 32;
        c += (b * 32 * bits + 7) / 8;
        len %= 32;
    }
    if len >= 16 {
        let b = len / 16;
        c += (b * 16 * bits + 7) / 8;
        len %= 16;
    }
    if len >= 8 {
        let b = len / 8;
        c += (b * 8 * bits + 7) / 8;
        len %= 8;
    }
    c + (len * bits + 7) / 8
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let mut m = input[0];
    let mut max_v = m;
    for i in 1..length as usize {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_v {
            max_v = input[i];
        }
    }
    let b = req_bits(max_v - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[length as usize - 1];
    let b = req_bits(max_v - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compress_bits(input: &[u32], output: &mut [u8], length: u32, base: u32, bits: u32) -> u32 {
    pack_bitstream(input, output, length, base, bits)
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let mut m = input[0];
    let mut max_v = m;
    for i in 1..length as usize {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_v {
            max_v = input[i];
        }
    }
    let b = req_bits(max_v - m);

    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;

    METADATA as u32 + pack_bitstream(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[length as usize - 1];
    let b = req_bits(max_v - m);

    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;

    METADATA as u32 + pack_bitstream(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_uncompress_bits(input: &[u8], output: &mut [u32], length: u32, base: u32, bits: u32) -> u32 {
    unpack_bitstream(input, output, length, base, bits)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;
    METADATA as u32 + unpack_bitstream(&input[METADATA as usize..], output, length, m, b)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits == 32 {
        let offset = length as usize * 4;
        let v = value.wrapping_sub(base);
        input[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
        return (length + 1) * 4;
    }

    if bits == 0 {
        // value must equal base; nothing to write.
        return 0;
    }

    let bit_pos = length as usize * bits as usize;
    let byte_off = bit_pos / 8;
    let bit_off = bit_pos % 8;

    let v = value.wrapping_sub(base);
    let mask: u64 = (1u64 << bits) - 1;
    let v_masked = (v as u64) & mask;

    let total = bits as usize + bit_off;
    let bytes_to_write = (total + 7) / 8;
    let shifted: u64 = v_masked << bit_off;
    let clear_mask: u64 = mask << bit_off;

    for k in 0..bytes_to_write {
        let idx = byte_off + k;
        if idx < input.len() {
            let byte_clear = ((clear_mask >> (k * 8)) & 0xFF) as u8;
            let byte_val = ((shifted >> (k * 8)) & 0xFF) as u8;
            input[idx] = (input[idx] & !byte_clear) | byte_val;
        }
    }

    let new_bit_pos = bit_pos + bits as usize;
    ((new_bit_pos + 7) / 8) as u32
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    if bits == 0 {
        return base;
    }
    if bits == 32 {
        let off = index as usize * 4;
        let v = read_u32_le(input, off);
        return base.wrapping_add(v);
    }

    let bit_pos = index as usize * bits as usize;
    let byte_off = bit_pos / 8;
    let bit_off = bit_pos % 8;
    let total = bits as usize + bit_off;
    let bytes_to_read = (total + 7) / 8;
    let mut acc: u64 = 0;
    for k in 0..bytes_to_read {
        let idx = byte_off + k;
        if idx < input.len() {
            acc |= (input[idx] as u64) << (k * 8);
        }
    }
    let mask: u64 = (1u64 << bits) - 1;
    let v = ((acc >> bit_off) & mask) as u32;
    base.wrapping_add(v)
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA as usize..], length, m, b, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits == 0 {
        return if value == base && length > 0 { 0 } else { length };
    }
    for i in 0..length {
        let v = for_select_bits(input, base, bits, i);
        if v == value {
            return i;
        }
    }
    length
}

pub fn for_lower_bound_search(input: &[u8], length: u32, value: u32, actual: &mut u32) -> u32 {
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;
    for_lower_bound_search_bits(&input[METADATA as usize..], length, m, b, value, actual)
}

pub fn for_lower_bound_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    if length == 0 {
        *actual = 0;
        return 0;
    }

    let mut imin: u32 = 0;
    let mut imax: u32 = length - 1;

    while imin + 1 < imax {
        let imid = imin + (imax - imin) / 2;
        let v = for_select_bits(input, base, bits, imid);
        if v >= value {
            imax = imid;
        } else {
            imin = imid;
        }
    }

    let v = for_select_bits(input, base, bits, imin);
    if v >= value {
        *actual = v;
        return imin;
    }

    let v = for_select_bits(input, base, bits, imax);
    *actual = v;
    imax
}

// Helper function
pub fn required_bits(v: u32) -> u32 {
    req_bits(v)
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(input: &mut [u8], length: u32, value: u32, appendImpl: AppendImpl) -> u32 {
    if length == 0 {
        return appendImpl(&[value], input, 1);
    }

    // Load min and the bits.
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;

    let bnew = if value < m { 0 } else { req_bits(value - m) };
    if m > value || bnew > b {
        // Need to re-encode the whole sequence.
        let mut tmp = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp, length);
        tmp[length as usize] = value;
        return appendImpl(&tmp, input, length + 1);
    }

    METADATA as u32 + for_append_bits(&mut input[METADATA as usize..], length, m, b, value)
}
