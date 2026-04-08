use crate::for_gen;

pub const METADATA: i32 = 5;

pub fn required_bits(v: u32) -> u32 {
    if v == 0 { 0 } else { 32 - v.leading_zeros() }
}

pub fn for_compressed_size_bits(mut length: u32, bits: u32) -> u32 {
    let mut c = 0u32;
    if length >= 32 {
        let b = length / 32;
        c += (b * 32 * bits + 7) / 8;
        length %= 32;
    }
    if length >= 16 {
        let b = length / 16;
        c += (b * 16 * bits + 7) / 8;
        length %= 16;
    }
    if length >= 8 {
        let b = length / 8;
        c += (b * 8 * bits + 7) / 8;
        length %= 8;
    }
    c + (length * bits + 7) / 8
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut m = input[0];
    let mut big_m = m;
    for i in 1..length as usize {
        if input[i] < m { m = input[i]; }
        if input[i] > big_m { big_m = input[i]; }
    }
    let b = required_bits(big_m - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = input[0];
    let big_m = input[length as usize - 1];
    let b = required_bits(big_m - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compress_bits(input: &[u32], output: &mut [u8], length: u32, base: u32, bits: u32) -> u32 {
    let mut i = 0u32;
    let mut written = 0u32;
    while i + 32 <= length {
        written += for_gen::dispatch_pack32(bits, base, &input[i as usize..], &mut output[written as usize..]);
        i += 32;
    }
    while i + 16 <= length {
        written += for_gen::dispatch_pack16(bits, base, &input[i as usize..], &mut output[written as usize..]);
        i += 16;
    }
    while i + 8 <= length {
        written += for_gen::dispatch_pack8(bits, base, &input[i as usize..], &mut output[written as usize..]);
        i += 8;
    }
    written + for_gen::dispatch_packx(bits, base, &input[i as usize..], &mut output[written as usize..], length - i)
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut m = input[0];
    let mut big_m = m;
    for i in 1..length as usize {
        if input[i] < m { m = input[i]; }
        if input[i] > big_m { big_m = input[i]; }
    }
    let b = required_bits(big_m - m);
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = input[0];
    let big_m = input[length as usize - 1];
    let b = required_bits(big_m - m);
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_uncompress_bits(input: &[u8], output: &mut [u32], length: u32, base: u32, bits: u32) -> u32 {
    let mut i = 0u32;
    let mut consumed = 0u32;
    while i + 32 <= length {
        consumed += for_gen::dispatch_unpack32(bits, base, &input[consumed as usize..], &mut output[i as usize..]);
        i += 32;
    }
    while i + 16 <= length {
        consumed += for_gen::dispatch_unpack16(bits, base, &input[consumed as usize..], &mut output[i as usize..]);
        i += 16;
    }
    while i + 8 <= length {
        consumed += for_gen::dispatch_unpack8(bits, base, &input[consumed as usize..], &mut output[i as usize..]);
        i += 8;
    }
    consumed + for_gen::dispatch_unpackx(bits, base, &input[consumed as usize..], &mut output[i as usize..], length - i)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA as usize..], output, length, m, b)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits == 32 {
        let off = length as usize * 4;
        let v = value.wrapping_sub(base);
        input[off..off + 4].copy_from_slice(&v.to_le_bytes());
        return (length + 1) * 4;
    }

    let mut byte_offset = 0usize;
    let mut rem = length;

    if rem > 32 {
        let b = rem / 32;
        byte_offset += (b * 32 * bits / 8) as usize;
        rem %= 32;
    }
    if rem > 16 {
        let b = rem / 16;
        byte_offset += (b * 16 * bits / 8) as usize;
        rem %= 16;
    }
    if rem > 8 {
        let b = rem / 8;
        byte_offset += (b * 8 * bits / 8) as usize;
        rem %= 8;
    }

    let start_bit = (rem * bits) as usize;
    byte_offset += start_bit / 8;
    let start = (start_bit % 8) as u32;

    let v = value.wrapping_sub(base);

    // Read the u32 word at byte_offset (treating input as little-endian u32 stream)
    if start + bits < 32 {
        let mask = (1u32 << bits) - 1;
        let mut word = u32::from_le_bytes([
            input[byte_offset],
            if byte_offset + 1 < input.len() { input[byte_offset + 1] } else { 0 },
            if byte_offset + 2 < input.len() { input[byte_offset + 2] } else { 0 },
            if byte_offset + 3 < input.len() { input[byte_offset + 3] } else { 0 },
        ]);
        word &= !(mask << start);
        word |= v << start;
        let bytes = word.to_le_bytes();
        for k in 0..4 {
            if byte_offset + k < input.len() {
                input[byte_offset + k] = bytes[k];
            }
        }
    } else {
        let mask1 = (1u32 << bits) - 1;
        let d = bits - (32 - start);
        let mask2 = (1u32 << d) - 1;
        // First word
        let mut word1 = u32::from_le_bytes([
            input[byte_offset],
            if byte_offset + 1 < input.len() { input[byte_offset + 1] } else { 0 },
            if byte_offset + 2 < input.len() { input[byte_offset + 2] } else { 0 },
            if byte_offset + 3 < input.len() { input[byte_offset + 3] } else { 0 },
        ]);
        word1 &= !(mask1 << start);
        word1 |= (v & mask1) << start;
        let bytes1 = word1.to_le_bytes();
        for k in 0..4 {
            if byte_offset + k < input.len() {
                input[byte_offset + k] = bytes1[k];
            }
        }
        // Second word
        let off2 = byte_offset + 4;
        let mut word2 = u32::from_le_bytes([
            if off2 < input.len() { input[off2] } else { 0 },
            if off2 + 1 < input.len() { input[off2 + 1] } else { 0 },
            if off2 + 2 < input.len() { input[off2 + 2] } else { 0 },
            if off2 + 3 < input.len() { input[off2 + 3] } else { 0 },
        ]);
        word2 &= !mask2;
        word2 |= v >> (32 - start);
        let bytes2 = word2.to_le_bytes();
        for k in 0..4 {
            if off2 + k < input.len() {
                input[off2 + k] = bytes2[k];
            }
        }
    }

    byte_offset as u32 + (start + bits + 7) / 8
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(input: &mut [u8], length: u32, value: u32, append_impl_fn: AppendImpl) -> u32 {
    if length == 0 {
        return append_impl_fn(&[value], input, 1);
    }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    let bnew = required_bits(value.wrapping_sub(m));
    if m > value || bnew > b {
        let mut tmp = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp, length);
        tmp[length as usize] = value;
        let s = append_impl_fn(&tmp, input, length + 1);
        return s;
    }
    METADATA as u32 + for_append_bits(&mut input[METADATA as usize..], length, m, b, value)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    if bits == 32 {
        let off = index as usize * 4;
        let v = u32::from_le_bytes([input[off], input[off+1], input[off+2], input[off+3]]);
        return base.wrapping_add(v);
    }

    let mut byte_offset = 0usize;
    let mut rem = index;

    if rem > 32 {
        let b = rem / 32;
        byte_offset += (b * 32 * bits / 8) as usize;
        rem %= 32;
    }
    if rem > 16 {
        let b = rem / 16;
        byte_offset += (b * 16 * bits / 8) as usize;
        rem %= 16;
    }
    if rem > 8 {
        let b = rem / 8;
        byte_offset += (b * 8 * bits / 8) as usize;
        rem %= 8;
    }

    let start_bit = (rem * bits) as usize;
    byte_offset += start_bit / 8;
    let start = (start_bit % 8) as u32;

    if start + bits < 32 {
        let mask = (1u32 << bits) - 1;
        let word = u32::from_le_bytes([
            input[byte_offset],
            if byte_offset + 1 < input.len() { input[byte_offset + 1] } else { 0 },
            if byte_offset + 2 < input.len() { input[byte_offset + 2] } else { 0 },
            if byte_offset + 3 < input.len() { input[byte_offset + 3] } else { 0 },
        ]);
        base.wrapping_add((word >> start) & mask)
    } else {
        let mask1 = (1u32 << bits) - 1;
        let d = bits - (32 - start);
        let mask2 = (1u32 << d) - 1;
        let word1 = u32::from_le_bytes([
            input[byte_offset],
            if byte_offset + 1 < input.len() { input[byte_offset + 1] } else { 0 },
            if byte_offset + 2 < input.len() { input[byte_offset + 2] } else { 0 },
            if byte_offset + 3 < input.len() { input[byte_offset + 3] } else { 0 },
        ]);
        let off2 = byte_offset + 4;
        let word2 = u32::from_le_bytes([
            if off2 < input.len() { input[off2] } else { 0 },
            if off2 + 1 < input.len() { input[off2 + 1] } else { 0 },
            if off2 + 2 < input.len() { input[off2 + 2] } else { 0 },
            if off2 + 3 < input.len() { input[off2 + 3] } else { 0 },
        ]);
        let v1 = (word1 >> start) & mask1;
        let v2 = word2 & mask2;
        base.wrapping_add((v2 << (32 - start)) | v1)
    }
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA as usize..], length, m, b, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    let mut i = 0u32;
    let mut found: i32 = -1;
    let mut offset = 0usize;

    if bits == 0 {
        return if value == base { 0 } else { length };
    }

    while i + 32 <= length {
        offset += for_gen::dispatch_linsearch32(bits, base, &input[offset..], value, &mut found) as usize;
        if found >= 0 { return i + found as u32; }
        i += 32;
    }
    while i + 16 <= length {
        offset += for_gen::dispatch_linsearch16(bits, base, &input[offset..], value, &mut found) as usize;
        if found >= 0 { return i + found as u32; }
        i += 16;
    }
    while i + 8 <= length {
        offset += for_gen::dispatch_linsearch8(bits, base, &input[offset..], value, &mut found) as usize;
        if found >= 0 { return i + found as u32; }
        i += 8;
    }
    for_gen::dispatch_linsearchx(bits, base, &input[offset..], length - i, value, &mut found);
    if found >= 0 { return i + found as u32; }
    length
}

pub fn for_lower_bound_search(input: &[u8], length: u32, value: u32, actual: &mut u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_lower_bound_search_bits(&input[METADATA as usize..], length, m, b, value, actual)
}

pub fn for_lower_bound_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32, actual: &mut u32) -> u32 {
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
