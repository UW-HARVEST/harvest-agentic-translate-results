use crate::for_gen;

pub const METADATA: i32 = 5;

#[inline]
fn read_u32_le(input: &[u8]) -> u32 {
    u32::from_le_bytes([input[0], input[1], input[2], input[3]])
}

#[inline]
fn write_u32_le(output: &mut [u8], value: u32) {
    output[0..4].copy_from_slice(&value.to_le_bytes());
}

pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    debug_assert!(bits <= 32);
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
    let b = required_bits(max_v - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[length as usize - 1];
    let b = required_bits(max_v - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    debug_assert!(bits <= 32);
    let mut i: u32 = 0;
    let mut written: u32 = 0;
    let mut in_off: usize = 0;

    while i + 32 <= length {
        let w = pack_dispatch_block(
            base,
            &input[in_off..],
            &mut output[written as usize..],
            bits,
            32,
        );
        written += w;
        i += 32;
        in_off += 32;
    }
    while i + 16 <= length {
        let w = pack_dispatch_block(
            base,
            &input[in_off..],
            &mut output[written as usize..],
            bits,
            16,
        );
        written += w;
        i += 16;
        in_off += 16;
    }
    while i + 8 <= length {
        let w = pack_dispatch_block(
            base,
            &input[in_off..],
            &mut output[written as usize..],
            bits,
            8,
        );
        written += w;
        i += 8;
        in_off += 8;
    }
    let remaining = length - i;
    written
        + pack_dispatch_x(
            base,
            &input[in_off..],
            &mut output[written as usize..],
            bits,
            remaining,
        )
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
    let b = required_bits(max_v - m);
    write_u32_le(&mut output[0..4], m);
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[length as usize - 1];
    let b = required_bits(max_v - m);
    write_u32_le(&mut output[0..4], m);
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    debug_assert!(bits <= 32);
    let mut i: u32 = 0;
    let mut in_pos: usize = 0;
    let mut out_pos: usize = 0;

    while i + 32 <= length {
        let r = unpack_dispatch_block(
            base,
            &input[in_pos..],
            &mut output[out_pos..],
            bits,
            32,
        );
        in_pos += r as usize;
        i += 32;
        out_pos += 32;
    }
    while i + 16 <= length {
        let r = unpack_dispatch_block(
            base,
            &input[in_pos..],
            &mut output[out_pos..],
            bits,
            16,
        );
        in_pos += r as usize;
        i += 16;
        out_pos += 16;
    }
    while i + 8 <= length {
        let r = unpack_dispatch_block(
            base,
            &input[in_pos..],
            &mut output[out_pos..],
            bits,
            8,
        );
        in_pos += r as usize;
        i += 8;
        out_pos += 8;
    }
    let remaining = length - i;
    let r = unpack_dispatch_x(
        base,
        &input[in_pos..],
        &mut output[out_pos..],
        bits,
        remaining,
    );
    in_pos as u32 + r
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = read_u32_le(&input[0..4]);
    let b = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA as usize..], output, length, m, b)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_append_bits(
    input: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    debug_assert!(bits <= 32);
    debug_assert!(required_bits(value - base) <= bits);
    debug_assert!(value >= base);

    let mut len = length;
    let mut in_pos: usize = 0;
    let initin_pos: usize = 0;

    if bits == 32 {
        // Treat input as u32 array; write at index `length`
        let off = (length as usize) * 4;
        write_u32_le(&mut input[off..off + 4], value - base);
        return ((length + 1) * 4) as u32;
    }

    if len > 32 {
        let b = len / 32;
        in_pos += ((b * 32 * bits) / 8) as usize;
        len %= 32;
    }
    if len > 16 {
        let b = len / 16;
        in_pos += ((b * 16 * bits) / 8) as usize;
        len %= 16;
    }
    if len > 8 {
        let b = len / 8;
        in_pos += ((b * 8 * bits) / 8) as usize;
        len %= 8;
    }

    let mut start = len * bits;
    in_pos += (start / 8) as usize;
    start %= 8;

    let v = value - base;

    // We need to read/write 4 or 8 bytes at in_pos
    if start + bits < 32 {
        let mask = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };
        let mut word = read_u32_le(&input[in_pos..in_pos + 4]);
        word &= !(mask << start);
        word |= v << start;
        write_u32_le(&mut input[in_pos..in_pos + 4], word);
    } else {
        let mask1 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };
        let d = bits + start - 32;
        let mask2 = if d >= 32 { u32::MAX } else { (1u32 << d) - 1 };
        let mut w0 = read_u32_le(&input[in_pos..in_pos + 4]);
        let mut w1 = read_u32_le(&input[in_pos + 4..in_pos + 8]);
        w0 &= !(mask1.wrapping_shl(start));
        w0 |= (v & mask1).wrapping_shl(start);
        w1 &= !mask2;
        w1 |= v.wrapping_shr(32 - start);
        write_u32_le(&mut input[in_pos..in_pos + 4], w0);
        write_u32_le(&mut input[in_pos + 4..in_pos + 8], w1);
    }

    (in_pos - initin_pos) as u32 + (start + bits + 7) / 8
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    debug_assert!(bits <= 32);
    let mut idx = index;
    let mut in_pos: usize = 0;

    if bits == 32 {
        let off = (idx as usize) * 4;
        return base + read_u32_le(&input[off..off + 4]);
    }
    if bits == 0 {
        return base;
    }

    if idx > 32 {
        let b = idx / 32;
        in_pos += ((b * 32 * bits) / 8) as usize;
        idx %= 32;
    }
    if idx > 16 {
        let b = idx / 16;
        in_pos += ((b * 16 * bits) / 8) as usize;
        idx %= 16;
    }
    if idx > 8 {
        let b = idx / 8;
        in_pos += ((b * 8 * bits) / 8) as usize;
        idx %= 8;
    }

    let mut start = idx * bits;
    in_pos += (start / 8) as usize;
    start %= 8;

    if start + bits < 32 {
        let mask = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };
        let word = read_u32_le(&input[in_pos..in_pos + 4]);
        return base + ((word >> start) & mask);
    } else {
        let mask1 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };
        let d = bits + start - 32;
        let mask2 = if d >= 32 { u32::MAX } else { (1u32 << d) - 1 };
        let w0 = read_u32_le(&input[in_pos..in_pos + 4]);
        let w1 = read_u32_le(&input[in_pos + 4..in_pos + 8]);
        let v1 = (w0 >> start) & mask1;
        let v2 = w1 & mask2;
        return base + ((v2.wrapping_shl(32 - start)) | v1);
    }
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = read_u32_le(&input[0..4]);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = read_u32_le(&input[0..4]);
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
    debug_assert!(bits <= 32);
    if bits == 0 {
        return if value == base { 0 } else { length };
    }
    let mut i: u32 = 0;
    let mut in_pos: usize = 0;
    let mut found: i32 = -1;

    while i + 32 <= length {
        let r = linsearch_dispatch_block(
            base,
            &input[in_pos..],
            value,
            &mut found,
            bits,
            32,
        );
        in_pos += r as usize;
        if found >= 0 {
            return i + found as u32;
        }
        i += 32;
    }
    while i + 16 <= length {
        let r = linsearch_dispatch_block(
            base,
            &input[in_pos..],
            value,
            &mut found,
            bits,
            16,
        );
        in_pos += r as usize;
        if found >= 0 {
            return i + found as u32;
        }
        i += 16;
    }
    while i + 8 <= length {
        let r = linsearch_dispatch_block(
            base,
            &input[in_pos..],
            value,
            &mut found,
            bits,
            8,
        );
        in_pos += r as usize;
        if found >= 0 {
            return i + found as u32;
        }
        i += 8;
    }
    let remaining = length - i;
    linsearch_dispatch_x(
        base,
        &input[in_pos..],
        remaining,
        value,
        &mut found,
        bits,
    );
    if found >= 0 {
        return i + found as u32;
    }
    length
}

pub fn for_lower_bound_search(
    input: &[u8],
    length: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    let m = read_u32_le(&input[0..4]);
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
    let m = read_u32_le(&input[0..4]);
    let b = input[4] as u32;

    let bnew = if value >= m {
        required_bits(value - m)
    } else {
        // m > value will cause re-encode anyway
        0
    };
    if m > value || bnew > b {
        let mut tmp: Vec<u32> = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp, length);
        tmp[length as usize] = value;
        return appendImpl(&tmp, input, length + 1);
    }
    METADATA as u32
        + for_append_bits(&mut input[METADATA as usize..], length, m, b, value)
}

// === Internal dispatch helpers (call into for_gen functions or internal helpers) ===

fn pack_dispatch_block(
    base: u32,
    input: &[u32],
    output: &mut [u8],
    bits: u32,
    block: usize,
) -> u32 {
    // Use for_gen pack_block via the named functions (reduces redundancy by calling helper)
    // Just call the generic helper directly. We expose them via for_gen module.
    for_gen::pack_block_pub(base, input, output, bits, block)
}

fn unpack_dispatch_block(
    base: u32,
    input: &[u8],
    output: &mut [u32],
    bits: u32,
    block: usize,
) -> u32 {
    for_gen::unpack_block_pub(base, input, output, bits, block)
}

fn linsearch_dispatch_block(
    base: u32,
    input: &[u8],
    value: u32,
    found: &mut i32,
    bits: u32,
    block: usize,
) -> u32 {
    for_gen::linsearch_block_pub(base, input, value, found, bits, block)
}

fn pack_dispatch_x(
    base: u32,
    input: &[u32],
    output: &mut [u8],
    bits: u32,
    length: u32,
) -> u32 {
    for_gen::pack_x_pub(base, input, output, bits, length)
}

fn unpack_dispatch_x(
    base: u32,
    input: &[u8],
    output: &mut [u32],
    bits: u32,
    length: u32,
) -> u32 {
    for_gen::unpack_x_pub(base, input, output, bits, length)
}

fn linsearch_dispatch_x(
    base: u32,
    input: &[u8],
    length: u32,
    value: u32,
    found: &mut i32,
    bits: u32,
) -> u32 {
    for_gen::linsearch_x_pub(base, input, length, value, found, bits)
}
