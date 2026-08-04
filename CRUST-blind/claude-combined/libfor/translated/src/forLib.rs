pub const METADATA: i32 = 5;

// Helper: number of bits required to encode `v`.
pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    assert!(bits <= 32);
    let mut c: u32 = 0;
    let mut length = length;

    if length >= 32 {
        let b = length / 32;
        c = c.wrapping_add(((b.wrapping_mul(32).wrapping_mul(bits)).wrapping_add(7)) / 8);
        length %= 32;
    }
    if length >= 16 {
        let b = length / 16;
        c = c.wrapping_add(((b.wrapping_mul(16).wrapping_mul(bits)).wrapping_add(7)) / 8);
        length %= 16;
    }
    if length >= 8 {
        let b = length / 8;
        c = c.wrapping_add(((b.wrapping_mul(8).wrapping_mul(bits)).wrapping_add(7)) / 8);
        length %= 8;
    }
    c.wrapping_add((length.wrapping_mul(bits).wrapping_add(7)) / 8)
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let len = length as usize;
    let mut m = input[0];
    let mut max_val = m;
    for i in 1..len {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_val {
            max_val = input[i];
        }
    }
    let b = required_bits(max_val.wrapping_sub(m));
    (METADATA as u32) + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let len = length as usize;
    let m = input[0];
    let max_val = input[len - 1];
    let b = required_bits(max_val.wrapping_sub(m));
    (METADATA as u32) + for_compressed_size_bits(length, b)
}

// Internal helper: write a value into the output at a specified bit position.
// Bits beyond the value within the same byte may be cleared.
fn write_bits_clearing(output: &mut [u8], bit_pos: usize, bits: u32, value: u32) {
    if bits == 0 {
        return;
    }
    let mut bit_pos = bit_pos;
    let mut bits_remaining = bits as i32;
    let mut value = value;
    while bits_remaining > 0 {
        let byte_idx = bit_pos / 8;
        let bit_idx = bit_pos % 8;
        let avail = 8 - bit_idx;
        let take = (avail as i32).min(bits_remaining) as u32;
        let mask: u8 = if take == 8 { 0xFF } else { ((1u32 << take) - 1) as u8 };
        let chunk = (value as u8) & mask;
        output[byte_idx] = (output[byte_idx] & !(mask << bit_idx)) | (chunk << bit_idx);
        value >>= take;
        bits_remaining -= take as i32;
        bit_pos += take as usize;
    }
}

// Internal helper: read `bits` bits from input starting at bit_pos.
fn read_bits(input: &[u8], bit_pos: usize, bits: u32) -> u32 {
    if bits == 0 {
        return 0;
    }
    let mut bit_pos = bit_pos;
    let mut bits_remaining = bits as i32;
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    while bits_remaining > 0 {
        let byte_idx = bit_pos / 8;
        let bit_idx = bit_pos % 8;
        let avail = 8 - bit_idx;
        let take = (avail as i32).min(bits_remaining) as u32;
        let mask: u8 = if take == 8 { 0xFF } else { ((1u32 << take) - 1) as u8 };
        let chunk = (input[byte_idx] >> bit_idx) & mask;
        result |= (chunk as u64) << shift;
        shift += take;
        bits_remaining -= take as i32;
        bit_pos += take as usize;
    }
    result as u32
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    assert!(bits <= 32);
    if length == 0 {
        return 0;
    }
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..length as usize {
            let v = input[i].wrapping_sub(base);
            output[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        return length * 4;
    }
    let total_bytes = ((length * bits) + 7) / 8;
    // Zero the target region (caller may have other contents in mind)
    for i in 0..total_bytes as usize {
        output[i] = 0;
    }
    let mask: u64 = (1u64 << bits) - 1;
    let mut buf: u64 = 0;
    let mut buf_bits: u32 = 0;
    let mut out_pos: usize = 0;
    for i in 0..length as usize {
        let v = (input[i].wrapping_sub(base)) as u64 & mask;
        buf |= v << buf_bits;
        buf_bits += bits;
        while buf_bits >= 8 {
            output[out_pos] = (buf & 0xFF) as u8;
            out_pos += 1;
            buf >>= 8;
            buf_bits -= 8;
        }
    }
    if buf_bits > 0 {
        output[out_pos] = (buf & 0xFF) as u8;
        out_pos += 1;
    }
    out_pos as u32
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let len = length as usize;
    let mut m = input[0];
    let mut max_val = m;
    for i in 1..len {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_val {
            max_val = input[i];
        }
    }
    let b = required_bits(max_val.wrapping_sub(m));
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    (METADATA as u32) + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let len = length as usize;
    let m = input[0];
    let max_val = input[len - 1];
    let b = required_bits(max_val.wrapping_sub(m));
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    (METADATA as u32) + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    assert!(bits <= 32);
    if length == 0 {
        return 0;
    }
    if bits == 0 {
        for i in 0..length as usize {
            output[i] = base;
        }
        return 0;
    }
    if bits == 32 {
        for i in 0..length as usize {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&input[i * 4..i * 4 + 4]);
            output[i] = base.wrapping_add(u32::from_le_bytes(buf));
        }
        return length * 4;
    }
    let total_bytes = ((length * bits) + 7) / 8;
    let mask: u64 = (1u64 << bits) - 1;
    let mut buf: u64 = 0;
    let mut buf_bits: u32 = 0;
    let mut in_pos: usize = 0;
    for i in 0..length as usize {
        while buf_bits < bits {
            buf |= (input[in_pos] as u64) << buf_bits;
            in_pos += 1;
            buf_bits += 8;
        }
        let v = (buf & mask) as u32;
        output[i] = base.wrapping_add(v);
        buf >>= bits;
        buf_bits -= bits;
    }
    total_bytes
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let mut mb = [0u8; 4];
    mb.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(mb);
    let b = input[4] as u32;
    (METADATA as u32) + for_uncompress_bits(&input[METADATA as usize..], output, length, m, b)
}

pub fn for_append_bits(
    input: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    assert!(bits <= 32);
    assert!(value >= base);
    let delta = value.wrapping_sub(base);
    assert!(required_bits(delta) <= bits);

    if bits == 32 {
        let off = (length as usize) * 4;
        input[off..off + 4].copy_from_slice(&delta.to_le_bytes());
        return (length + 1) * 4;
    }

    if bits == 0 {
        // value must equal base; nothing to write.
        return 0;
    }

    let bit_pos = (length as usize) * (bits as usize);
    write_bits_clearing(input, bit_pos, bits, delta);
    let total_bits = bit_pos + bits as usize;
    ((total_bits + 7) / 8) as u32
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(
    input: &mut [u8],
    length: u32,
    value: u32,
    appendImpl: AppendImpl,
) -> u32 {
    if length == 0 {
        let v = [value];
        return appendImpl(&v, input, 1);
    }
    let mut mb = [0u8; 4];
    mb.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(mb);
    let b = input[4] as u32;

    let bnew = if value >= m {
        required_bits(value.wrapping_sub(m))
    } else {
        // Force re-encode by using a very large value.
        u32::MAX
    };
    if m > value || bnew > b {
        let len = length as usize;
        let mut tmp: Vec<u32> = vec![0u32; len + 1];
        for_uncompress(input, &mut tmp, length);
        tmp[len] = value;
        return appendImpl(&tmp, input, length + 1);
    }
    (METADATA as u32)
        + for_append_bits(&mut input[METADATA as usize..], length, m, b, value)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    assert!(bits <= 32);
    if bits == 0 {
        return base;
    }
    if bits == 32 {
        let i = index as usize;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&input[i * 4..i * 4 + 4]);
        return base.wrapping_add(u32::from_le_bytes(buf));
    }
    let bit_pos = (index as usize) * (bits as usize);
    let v = read_bits(input, bit_pos, bits);
    base.wrapping_add(v)
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let mut mb = [0u8; 4];
    mb.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(mb);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let mut mb = [0u8; 4];
    mb.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(mb);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA as usize..], length, m, b, value)
}

pub fn for_linear_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    assert!(bits <= 32);
    if bits == 0 {
        return if value == base { 0 } else { length };
    }
    if value < base {
        return length;
    }
    let delta = value.wrapping_sub(base);
    if bits < 32 {
        let max_delta = (1u64 << bits) - 1;
        if (delta as u64) > max_delta {
            return length;
        }
    }
    for i in 0..length {
        let v = read_bits(input, (i as usize) * (bits as usize), bits);
        if v == delta {
            return i;
        }
    }
    length
}

pub fn for_lower_bound_search(
    input: &[u8],
    length: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    let mut mb = [0u8; 4];
    mb.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(mb);
    let b = input[4] as u32;
    for_lower_bound_search_bits(
        &input[METADATA as usize..],
        length,
        m,
        b,
        value,
        actual,
    )
}

pub fn for_lower_bound_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    assert!(bits <= 32);
    let mut imin: u32 = 0;
    let mut imax: u32 = length.wrapping_sub(1);
    while imin + 1 < imax {
        let imid = imin + ((imax - imin) / 2);
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
