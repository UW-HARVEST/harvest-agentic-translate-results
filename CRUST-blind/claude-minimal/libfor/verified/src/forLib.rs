use crate::for_gen::{
    for_linsearch16, for_linsearch32, for_linsearch8, for_linsearchx, for_pack16, for_pack32,
    for_pack8, for_packx, for_unpack16, for_unpack32, for_unpack8, for_unpackx,
};

pub const METADATA: i32 = 5;

const METADATA_USIZE: usize = 5;

#[inline]
fn read_u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

#[inline]
fn write_u32_le(buf: &mut [u8], off: usize, val: u32) {
    let b = val.to_le_bytes();
    buf[off] = b[0];
    buf[off + 1] = b[1];
    buf[off + 2] = b[2];
    buf[off + 3] = b[3];
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    let mut c: u32 = 0;
    let mut length = length;
    let mut b: u32;

    if length >= 32 {
        b = length / 32;
        c += ((b * 32 * bits) + 7) / 8;
        length %= 32;
    }

    if length >= 16 {
        b = length / 16;
        c += ((b * 16 * bits) + 7) / 8;
        length %= 16;
    }

    if length >= 8 {
        b = length / 8;
        c += ((b * 8 * bits) + 7) / 8;
        length %= 8;
    }

    c + ((length * bits) + 7) / 8
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let mut m = input[0];
    let mut max_v = m;

    for i in 1..(length as usize) {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_v {
            max_v = input[i];
        }
    }

    let b = required_bits(max_v.wrapping_sub(m));

    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let m = input[0];
    let max_v = input[(length - 1) as usize];

    let b = required_bits(max_v.wrapping_sub(m));

    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    let mut i: u32 = 0;
    let mut written: u32 = 0;
    let mut in_off: usize = 0;

    let bits_idx = bits as usize;

    while i + 32 <= length {
        let w = for_pack32[bits_idx](
            base,
            &input[in_off..in_off + 32],
            &mut output[written as usize..],
        );
        written += w;
        i += 32;
        in_off += 32;
    }

    while i + 16 <= length {
        let w = for_pack16[bits_idx](
            base,
            &input[in_off..in_off + 16],
            &mut output[written as usize..],
        );
        written += w;
        i += 16;
        in_off += 16;
    }

    while i + 8 <= length {
        let w = for_pack8[bits_idx](
            base,
            &input[in_off..in_off + 8],
            &mut output[written as usize..],
        );
        written += w;
        i += 8;
        in_off += 8;
    }

    written
        + for_packx[bits_idx](
            base,
            &input[in_off..],
            &mut output[written as usize..],
            length - i,
        )
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let mut m = input[0];
    let mut max_v = m;

    for i in 1..(length as usize) {
        if input[i] < m {
            m = input[i];
        }
        if input[i] > max_v {
            max_v = input[i];
        }
    }

    let b = required_bits(max_v.wrapping_sub(m));

    write_u32_le(output, 0, m);
    output[4] = b as u8;

    METADATA as u32 + for_compress_bits(input, &mut output[METADATA_USIZE..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let m = input[0];
    let max_v = input[(length - 1) as usize];

    let b = required_bits(max_v.wrapping_sub(m));

    write_u32_le(output, 0, m);
    output[4] = b as u8;

    METADATA as u32 + for_compress_bits(input, &mut output[METADATA_USIZE..], length, m, b)
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    let mut i: u32 = 0;
    let mut in_consumed: usize = 0;
    let mut out_off: usize = 0;

    let bits_idx = bits as usize;

    while i + 32 <= length {
        let r = for_unpack32[bits_idx](
            base,
            &input[in_consumed..],
            &mut output[out_off..out_off + 32],
        );
        in_consumed += r as usize;
        i += 32;
        out_off += 32;
    }

    while i + 16 <= length {
        let r = for_unpack16[bits_idx](
            base,
            &input[in_consumed..],
            &mut output[out_off..out_off + 16],
        );
        in_consumed += r as usize;
        i += 16;
        out_off += 16;
    }

    while i + 8 <= length {
        let r = for_unpack8[bits_idx](
            base,
            &input[in_consumed..],
            &mut output[out_off..out_off + 8],
        );
        in_consumed += r as usize;
        i += 8;
        out_off += 8;
    }

    in_consumed as u32
        + for_unpackx[bits_idx](
            base,
            &input[in_consumed..],
            &mut output[out_off..],
            length - i,
        )
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let m = read_u32_le(input, 0);
    let b = input[4] as u32;

    METADATA as u32 + for_uncompress_bits(&input[METADATA_USIZE..], output, length, m, b)
}

pub fn for_append_bits(
    input: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    let mut length = length;

    if bits == 32 {
        // store at index `length`
        write_u32_le(input, (length as usize) * 4, value.wrapping_sub(base));
        return (length + 1) * 4;
    }

    let mut in_off: usize = 0;
    let mut b: u32;

    if length > 32 {
        b = length / 32;
        in_off += ((b * 32 * bits) / 8) as usize;
        length %= 32;
    }

    if length > 16 {
        b = length / 16;
        in_off += ((b * 16 * bits) / 8) as usize;
        length %= 16;
    }

    if length > 8 {
        b = length / 8;
        in_off += ((b * 8 * bits) / 8) as usize;
        length %= 8;
    }

    let mut start: u32 = length * bits;

    in_off += (start / 8) as usize;
    start %= 8;

    let value = value.wrapping_sub(base);

    if start + bits < 32 {
        let mask: u32 = (1u32 << bits) - 1;
        let cur = read_u32_le(input, in_off);
        let new_val = (cur & !(mask << start)) | (value << start);
        write_u32_le(input, in_off, new_val);
    } else {
        let mask1: u32 = (1u32 << bits) - 1;
        let mask2: u32 = (1u32 << (bits - (32 - start))) - 1;
        let cur0 = read_u32_le(input, in_off);
        let new0 = (cur0 & !(mask1 << start)) | ((value & mask1) << start);
        write_u32_le(input, in_off, new0);
        let cur1 = read_u32_le(input, in_off + 4);
        let new1 = (cur1 & !mask2) | (value >> (32 - start));
        write_u32_le(input, in_off + 4, new1);
    }

    in_off as u32 + ((start + bits) + 7) / 8
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    let mut index = index;
    let mut in_off: usize = 0;
    let mut b: u32;

    if bits == 32 {
        return base.wrapping_add(read_u32_le(input, (index as usize) * 4));
    }

    if index > 32 {
        b = index / 32;
        in_off += ((b * 32 * bits) / 8) as usize;
        index %= 32;
    }

    if index > 16 {
        b = index / 16;
        in_off += ((b * 16 * bits) / 8) as usize;
        index %= 16;
    }

    if index > 8 {
        b = index / 8;
        in_off += ((b * 8 * bits) / 8) as usize;
        index %= 8;
    }

    let mut start: u32 = index * bits;
    in_off += (start / 8) as usize;
    start %= 8;

    if start + bits < 32 {
        let mask: u32 = (1u32 << bits) - 1;
        base.wrapping_add((read_u32_le(input, in_off) >> start) & mask)
    } else {
        let mask1: u32 = (1u32 << bits) - 1;
        let mask2: u32 = (1u32 << (bits - (32 - start))) - 1;
        let v1 = (read_u32_le(input, in_off) >> start) & mask1;
        let v2 = read_u32_le(input, in_off + 4) & mask2;
        base.wrapping_add((v2 << (32 - start)) | v1)
    }
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;

    for_select_bits(&input[METADATA_USIZE..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;

    for_linear_search_bits(&input[METADATA_USIZE..], length, m, b, value)
}

pub fn for_linear_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    let mut i: u32 = 0;
    let mut found: i32 = -1;
    let mut in_off: usize = 0;

    if bits == 0 {
        return if value == base { 0 } else { length };
    }

    let bits_idx = bits as usize;

    while i + 32 <= length {
        let r = for_linsearch32[bits_idx](base, &input[in_off..], value, &mut found);
        in_off += r as usize;
        if found >= 0 {
            return i + found as u32;
        }
        i += 32;
    }

    while i + 16 <= length {
        let r = for_linsearch16[bits_idx](base, &input[in_off..], value, &mut found);
        in_off += r as usize;
        if found >= 0 {
            return i + found as u32;
        }
        i += 16;
    }

    while i + 8 <= length {
        let r = for_linsearch8[bits_idx](base, &input[in_off..], value, &mut found);
        in_off += r as usize;
        if found >= 0 {
            return i + found as u32;
        }
        i += 8;
    }

    for_linsearchx[bits_idx](base, &input[in_off..], length - i, value, &mut found);
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
    let m = read_u32_le(input, 0);
    let b = input[4] as u32;

    for_lower_bound_search_bits(&input[METADATA_USIZE..], length, m, b, value, actual)
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
    let mut imid: u32;
    let mut v: u32;

    while imin + 1 < imax {
        imid = imin + ((imax - imin) / 2);

        v = for_select_bits(input, base, bits, imid);
        if v >= value {
            imax = imid;
        } else {
            imin = imid;
        }
    }

    v = for_select_bits(input, base, bits, imin);
    if v >= value {
        *actual = v;
        return imin;
    }

    v = for_select_bits(input, base, bits, imax);
    *actual = v;
    imax
}

// Helper function
pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(
    input: &mut [u8],
    length: u32,
    value: u32,
    appendImpl: AppendImpl,
) -> u32 {
    if length == 0 {
        let value_arr = [value];
        return appendImpl(&value_arr, input, 1);
    }

    let m = read_u32_le(input, 0);
    let b = input[4] as u32;

    let bnew = required_bits(value.wrapping_sub(m));
    if m > value || bnew > b {
        let mut tmp: Vec<u32> = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp, length);
        tmp[length as usize] = value;
        return appendImpl(&tmp, input, length + 1);
    }

    METADATA as u32 + for_append_bits(&mut input[METADATA_USIZE..], length, m, b, value)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}
