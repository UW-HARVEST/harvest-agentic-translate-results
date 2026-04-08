use crate::for_gen;

pub const METADATA: i32 = 5;

pub fn required_bits(v: u32) -> u32 {
    if v == 0 { 0 } else { 32 - v.leading_zeros() }
}

fn dispatch_pack(bits: u32, count: usize, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    macro_rules! d {
        ($b:expr, $c:expr, $base:expr, $inp:expr, $out:expr, $($bit:expr => ($f32:path, $f16:path, $f8:path)),+ $(,)?) => {
            match $b {
                $($bit => match $c {
                    32 => $f32($base, $inp, $out),
                    16 => $f16($base, $inp, $out),
                    _ => $f8($base, $inp, $out),
                },)+
                _ => 0,
            }
        };
    }
    d!(bits, count, base, input, output,
        0 => (for_gen::pack0_32, for_gen::pack0_16, for_gen::pack0_8),
        1 => (for_gen::pack1_32, for_gen::pack1_16, for_gen::pack1_8),
        2 => (for_gen::pack2_32, for_gen::pack2_16, for_gen::pack2_8),
        3 => (for_gen::pack3_32, for_gen::pack3_16, for_gen::pack3_8),
        4 => (for_gen::pack4_32, for_gen::pack4_16, for_gen::pack4_8),
        5 => (for_gen::pack5_32, for_gen::pack5_16, for_gen::pack5_8),
        6 => (for_gen::pack6_32, for_gen::pack6_16, for_gen::pack6_8),
        7 => (for_gen::pack7_32, for_gen::pack7_16, for_gen::pack7_8),
        8 => (for_gen::pack8_32, for_gen::pack8_16, for_gen::pack8_8),
        9 => (for_gen::pack9_32, for_gen::pack9_16, for_gen::pack9_8),
        10 => (for_gen::pack10_32, for_gen::pack10_16, for_gen::pack10_8),
        11 => (for_gen::pack11_32, for_gen::pack11_16, for_gen::pack11_8),
        12 => (for_gen::pack12_32, for_gen::pack12_16, for_gen::pack12_8),
        13 => (for_gen::pack13_32, for_gen::pack13_16, for_gen::pack13_8),
        14 => (for_gen::pack14_32, for_gen::pack14_16, for_gen::pack14_8),
        15 => (for_gen::pack15_32, for_gen::pack15_16, for_gen::pack15_8),
        16 => (for_gen::pack16_32, for_gen::pack16_16, for_gen::pack16_8),
        17 => (for_gen::pack17_32, for_gen::pack17_16, for_gen::pack17_8),
        18 => (for_gen::pack18_32, for_gen::pack18_16, for_gen::pack18_8),
        19 => (for_gen::pack19_32, for_gen::pack19_16, for_gen::pack19_8),
        20 => (for_gen::pack20_32, for_gen::pack20_16, for_gen::pack20_8),
        21 => (for_gen::pack21_32, for_gen::pack21_16, for_gen::pack21_8),
        22 => (for_gen::pack22_32, for_gen::pack22_16, for_gen::pack22_8),
        23 => (for_gen::pack23_32, for_gen::pack23_16, for_gen::pack23_8),
        24 => (for_gen::pack24_32, for_gen::pack24_16, for_gen::pack24_8),
        25 => (for_gen::pack25_32, for_gen::pack25_16, for_gen::pack25_8),
        26 => (for_gen::pack26_32, for_gen::pack26_16, for_gen::pack26_8),
        27 => (for_gen::pack27_32, for_gen::pack27_16, for_gen::pack27_8),
        28 => (for_gen::pack28_32, for_gen::pack28_16, for_gen::pack28_8),
        29 => (for_gen::pack29_32, for_gen::pack29_16, for_gen::pack29_8),
        30 => (for_gen::pack30_32, for_gen::pack30_16, for_gen::pack30_8),
        31 => (for_gen::pack31_32, for_gen::pack31_16, for_gen::pack31_8),
        32 => (for_gen::pack32_32, for_gen::pack32_16, for_gen::pack32_8)
    )
}

fn dispatch_packx(bits: u32, base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    macro_rules! d {
        ($b:expr, $($bit:expr => $f:path),+ $(,)?) => {
            match $b { $($bit => $f(base, input, output, length),)+ _ => 0 }
        };
    }
    d!(bits,
        0=>for_gen::pack0_x, 1=>for_gen::pack1_x, 2=>for_gen::pack2_x, 3=>for_gen::pack3_x,
        4=>for_gen::pack4_x, 5=>for_gen::pack5_x, 6=>for_gen::pack6_x, 7=>for_gen::pack7_x,
        8=>for_gen::pack8_x, 9=>for_gen::pack9_x, 10=>for_gen::pack10_x, 11=>for_gen::pack11_x,
        12=>for_gen::pack12_x, 13=>for_gen::pack13_x, 14=>for_gen::pack14_x, 15=>for_gen::pack15_x,
        16=>for_gen::pack16_x, 17=>for_gen::pack17_x, 18=>for_gen::pack18_x, 19=>for_gen::pack19_x,
        20=>for_gen::pack20_x, 21=>for_gen::pack21_x, 22=>for_gen::pack22_x, 23=>for_gen::pack23_x,
        24=>for_gen::pack24_x, 25=>for_gen::pack25_x, 26=>for_gen::pack26_x, 27=>for_gen::pack27_x,
        28=>for_gen::pack28_x, 29=>for_gen::pack29_x, 30=>for_gen::pack30_x, 31=>for_gen::pack31_x,
        32=>for_gen::pack32_x
    )
}

// For unpack, the for_gen functions have swapped types. We need to call them
// by reinterpreting our &[u8] input as &[u32] and &mut [u32] output as &mut [u8].
// But for_uncompress_bits receives input: &[u8] and output: &mut [u32].
// We'll use the generic functions directly instead.

fn dispatch_unpack(bits: u32, count: usize, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for_gen::generic_unpack_pub(base, input, output, count, bits)
}

fn dispatch_unpackx(bits: u32, base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    for_gen::generic_unpack_x_pub(base, input, output, length, bits)
}

fn dispatch_linsearch(bits: u32, count: usize, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    for_gen::generic_linsearch_pub(base, input, count, bits, value, found)
}

fn dispatch_linsearchx(bits: u32, base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    for_gen::generic_linsearch_x_pub(base, input, length, bits, value, found)
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    let mut length = length;
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
    let inp = input;
    while i + 32 <= length {
        written += dispatch_pack(bits, 32, base, &inp[i as usize..], &mut output[written as usize..]);
        i += 32;
    }
    while i + 16 <= length {
        written += dispatch_pack(bits, 16, base, &inp[i as usize..], &mut output[written as usize..]);
        i += 16;
    }
    while i + 8 <= length {
        written += dispatch_pack(bits, 8, base, &inp[i as usize..], &mut output[written as usize..]);
        i += 8;
    }
    written + dispatch_packx(bits, base, &inp[i as usize..], &mut output[written as usize..], length - i)
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
        consumed += dispatch_unpack(bits, 32, base, &input[consumed as usize..], &mut output[i as usize..]);
        i += 32;
    }
    while i + 16 <= length {
        consumed += dispatch_unpack(bits, 16, base, &input[consumed as usize..], &mut output[i as usize..]);
        i += 16;
    }
    while i + 8 <= length {
        consumed += dispatch_unpack(bits, 8, base, &input[consumed as usize..], &mut output[i as usize..]);
        i += 8;
    }
    consumed + dispatch_unpackx(bits, base, &input[consumed as usize..], &mut output[i as usize..], length - i)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA as usize..], output, length, m, b)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits == 32 {
        let offset = length as usize * 4;
        let val = value.wrapping_sub(base);
        input[offset..offset+4].copy_from_slice(&val.to_le_bytes());
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
    let start_bit = rem * bits;
    byte_offset += (start_bit / 8) as usize;
    let start = start_bit % 8;
    let val = value.wrapping_sub(base);
    // Read u32 at byte_offset
    let mut w = read_u32_le(input, byte_offset);
    if start + bits < 32 {
        let mask = (1u32 << bits) - 1;
        w &= !(mask << start);
        w |= val << start;
        write_u32_le(input, byte_offset, w);
    } else {
        let mask1 = (1u32 << bits) - 1;
        let overflow = bits - (32 - start);
        let mask2 = if overflow >= 32 { u32::MAX } else { (1u32 << overflow) - 1 };
        w &= !(mask1 << start);
        w |= (val & mask1) << start;
        write_u32_le(input, byte_offset, w);
        let mut w2 = read_u32_le(input, byte_offset + 4);
        w2 &= !mask2;
        w2 |= val >> (32 - start);
        write_u32_le(input, byte_offset + 4, w2);
    }
    (byte_offset as u32) + (start + bits + 7) / 8
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    let mut buf = [0u8; 4];
    let end = (offset + 4).min(bytes.len());
    let len = end.saturating_sub(offset);
    buf[..len].copy_from_slice(&bytes[offset..offset + len]);
    u32::from_le_bytes(buf)
}

fn write_u32_le(bytes: &mut [u8], offset: usize, val: u32) {
    let le = val.to_le_bytes();
    let end = (offset + 4).min(bytes.len());
    let len = end.saturating_sub(offset);
    bytes[offset..offset + len].copy_from_slice(&le[..len]);
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(input: &mut [u8], length: u32, value: u32, appendImpl: AppendImpl) -> u32 {
    if length == 0 {
        return appendImpl(&[value], input, 1);
    }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    let bnew = required_bits(value.wrapping_sub(m));
    if m > value || bnew > b {
        let mut tmp = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp[..length as usize], length);
        tmp[length as usize] = value;
        return appendImpl(&tmp, input, length + 1);
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
        return base.wrapping_add(read_u32_le(input, index as usize * 4));
    }
    let mut byte_offset = 0usize;
    let mut idx = index;
    if idx > 32 {
        let b = idx / 32;
        byte_offset += (b * 32 * bits / 8) as usize;
        idx %= 32;
    }
    if idx > 16 {
        let b = idx / 16;
        byte_offset += (b * 16 * bits / 8) as usize;
        idx %= 16;
    }
    if idx > 8 {
        let b = idx / 8;
        byte_offset += (b * 8 * bits / 8) as usize;
        idx %= 8;
    }
    let start_bit = idx * bits;
    byte_offset += (start_bit / 8) as usize;
    let start = start_bit % 8;
    let w = read_u32_le(input, byte_offset);
    if start + bits < 32 {
        let mask = (1u32 << bits) - 1;
        base.wrapping_add((w >> start) & mask)
    } else {
        let mask1 = (1u32 << bits) - 1;
        let overflow = bits - (32 - start);
        let mask2 = if overflow >= 32 { u32::MAX } else { (1u32 << overflow) - 1 };
        let v1 = (w >> start) & mask1;
        let w2 = read_u32_le(input, byte_offset + 4);
        let v2 = w2 & mask2;
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
    if bits == 0 {
        return if value == base { 0 } else { length };
    }
    let mut offset = 0usize;
    while i + 32 <= length {
        offset += dispatch_linsearch(bits, 32, base, &input[offset..], value, &mut found) as usize;
        if found >= 0 { return i + found as u32; }
        i += 32;
    }
    while i + 16 <= length {
        offset += dispatch_linsearch(bits, 16, base, &input[offset..], value, &mut found) as usize;
        if found >= 0 { return i + found as u32; }
        i += 16;
    }
    while i + 8 <= length {
        offset += dispatch_linsearch(bits, 8, base, &input[offset..], value, &mut found) as usize;
        if found >= 0 { return i + found as u32; }
        i += 8;
    }
    dispatch_linsearchx(bits, base, &input[offset..], length - i, value, &mut found);
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
        if v >= value { imax = imid; } else { imin = imid; }
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
