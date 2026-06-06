pub const METADATA: i32 = 5;

// Internal helper: read a u32 from `buf` starting at index `i` (little-endian).
fn read_u32(buf: &[u8], i: usize) -> u32 {
    (buf[i] as u32)
        | ((buf[i + 1] as u32) << 8)
        | ((buf[i + 2] as u32) << 16)
        | ((buf[i + 3] as u32) << 24)
}

// Internal helper: write a u32 to `buf` starting at index `i` (little-endian).
fn write_u32(buf: &mut [u8], i: usize, v: u32) {
    buf[i] = (v & 0xff) as u8;
    buf[i + 1] = ((v >> 8) & 0xff) as u8;
    buf[i + 2] = ((v >> 16) & 0xff) as u8;
    buf[i + 3] = ((v >> 24) & 0xff) as u8;
}

pub fn for_compressed_size_bits(length: u32, _bits: u32) -> u32 {
    // Uniform format: each integer is stored as a 32-bit delta. The `bits`
    // parameter is therefore ignored (we always use 32 bits per value).
    length * 4
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    METADATA as u32 + for_compressed_size_bits(length, 32)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    METADATA as u32 + for_compressed_size_bits(length, 32)
}

pub fn for_compress_bits(input: &[u32], output: &mut [u8], length: u32, base: u32, _bits: u32) -> u32 {
    for i in 0..length as usize {
        write_u32(output, i * 4, input[i].wrapping_sub(base));
    }
    length * 4
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let mut m = input[0];
    let mut max = m;
    for i in 1..length as usize {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max {
            max = input[i];
        }
    }
    let bits = required_bits(max - m);
    write_u32(output, 0, m);
    output[4] = bits as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, bits)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max = input[length as usize - 1];
    let bits = required_bits(max - m);
    write_u32(output, 0, m);
    output[4] = bits as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, bits)
}

pub fn for_uncompress_bits(input: &[u8], output: &mut [u32], length: u32, base: u32, _bits: u32) -> u32 {
    for i in 0..length as usize {
        output[i] = base.wrapping_add(read_u32(input, i * 4));
    }
    length * 4
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = read_u32(input, 0);
    let bits = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA as usize..], output, length, m, bits)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, _bits: u32, value: u32) -> u32 {
    write_u32(input, length as usize * 4, value.wrapping_sub(base));
    (length + 1) * 4
}

pub fn for_select_bits(input: &[u8], base: u32, _bits: u32, index: u32) -> u32 {
    base.wrapping_add(read_u32(input, index as usize * 4))
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = read_u32(input, 0);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = read_u32(input, 0);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA as usize..], length, m, b, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, _bits: u32, value: u32) -> u32 {
    let target = value.wrapping_sub(base);
    for i in 0..length as usize {
        if read_u32(input, i * 4) == target {
            return i as u32;
        }
    }
    length
}

pub fn for_lower_bound_search(input: &[u8], length: u32, value: u32, actual: &mut u32) -> u32 {
    let m = read_u32(input, 0);
    let b = input[4] as u32;
    for_lower_bound_search_bits(&input[METADATA as usize..], length, m, b, value, actual)
}

pub fn for_lower_bound_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32, actual: &mut u32) -> u32 {
    // Adapted from the C implementation in for.c.
    let mut imin: u32 = 0;
    let mut imax: u32 = length - 1;

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

// Helper function: returns the number of bits required to represent `v`.
pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(input: &mut [u8], length: u32, value: u32, appendImpl: AppendImpl) -> u32 {
    if length == 0 {
        return appendImpl(&[value], input, 1);
    }

    let m = read_u32(input, 0);
    let b = input[4] as u32;

    // If the new value cannot be stored relative to the existing base, or if
    // it is smaller than the base, then re-encode the whole sequence.
    let bnew = if value >= m { required_bits(value - m) } else { u32::MAX };
    if m > value || bnew > b {
        let mut tmp: Vec<u32> = vec![0u32; length as usize + 1];
        for_uncompress(input, &mut tmp, length);
        tmp[length as usize] = value;
        return appendImpl(&tmp, input, length + 1);
    }

    METADATA as u32 + for_append_bits(&mut input[METADATA as usize..], length, m, b, value)
}
