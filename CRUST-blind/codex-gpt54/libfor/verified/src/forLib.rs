use crate::for_gen::{pack_block, unpack_block};

pub const METADATA: i32 = 5;

fn metadata_size() -> usize {
    METADATA as usize
}

fn read_u32_le(input: &[u8], offset: usize) -> u32 {
    let mut bytes = [0u8; 4];
    let available = input.len().saturating_sub(offset).min(4);
    bytes[..available].copy_from_slice(&input[offset..offset + available]);
    u32::from_le_bytes(bytes)
}

fn write_u32_le(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn set_bit(output: &mut [u8], bit_index: usize, value: bool) {
    let byte_index = bit_index / 8;
    let mask = 1u8 << (bit_index % 8);
    if value {
        output[byte_index] |= mask;
    } else {
        output[byte_index] &= !mask;
    }
}

fn read_bits(input: &[u8], bit_index: usize, bits: u32) -> u32 {
    let mut value = 0u32;
    for shift in 0..bits as usize {
        let absolute = bit_index + shift;
        let byte = input.get(absolute / 8).copied().unwrap_or(0);
        let bit = (byte >> (absolute % 8)) & 1;
        value |= (bit as u32) << shift;
    }
    value
}

fn locate_entry_position(index: u32, bits: u32) -> (usize, usize) {
    let mut remaining = index;
    let mut byte_offset = 0usize;

    if remaining > 32 {
        let blocks = remaining / 32;
        byte_offset += ((blocks * 32 * bits) / 8) as usize;
        remaining %= 32;
    }
    if remaining > 16 {
        let blocks = remaining / 16;
        byte_offset += ((blocks * 16 * bits) / 8) as usize;
        remaining %= 16;
    }
    if remaining > 8 {
        let blocks = remaining / 8;
        byte_offset += ((blocks * 8 * bits) / 8) as usize;
        remaining %= 8;
    }

    let start = remaining * bits;
    byte_offset += (start / 8) as usize;
    (byte_offset, (start % 8) as usize)
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    assert!(bits <= 32);

    let mut compressed = 0u32;
    let mut remaining = length;

    if remaining >= 32 {
        let blocks = remaining / 32;
        compressed += ((blocks * 32 * bits) + 7) / 8;
        remaining %= 32;
    }
    if remaining >= 16 {
        let blocks = remaining / 16;
        compressed += ((blocks * 16 * bits) + 7) / 8;
        remaining %= 16;
    }
    if remaining >= 8 {
        let blocks = remaining / 8;
        compressed += ((blocks * 8 * bits) + 7) / 8;
        remaining %= 8;
    }

    compressed + ((remaining * bits) + 7) / 8
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let slice = &input[..length as usize];
    let mut min_value = slice[0];
    let mut max_value = min_value;
    for &value in &slice[1..] {
        if value < min_value {
            min_value = value;
        }
        if value > max_value {
            max_value = value;
        }
    }

    METADATA as u32 + for_compressed_size_bits(length, required_bits(max_value - min_value))
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let slice = &input[..length as usize];
    let min_value = slice[0];
    let max_value = slice[slice.len() - 1];

    METADATA as u32 + for_compressed_size_bits(length, required_bits(max_value - min_value))
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    assert!(bits <= 32);

    let mut index = 0usize;
    let mut written = 0usize;
    let length = length as usize;

    while index + 32 <= length {
        written += pack_block(base, &input[index..index + 32], &mut output[written..], bits) as usize;
        index += 32;
    }
    while index + 16 <= length {
        written += pack_block(base, &input[index..index + 16], &mut output[written..], bits) as usize;
        index += 16;
    }
    while index + 8 <= length {
        written += pack_block(base, &input[index..index + 8], &mut output[written..], bits) as usize;
        index += 8;
    }

    written += pack_block(base, &input[index..length], &mut output[written..], bits) as usize;
    written as u32
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let slice = &input[..length as usize];
    let mut min_value = slice[0];
    let mut max_value = min_value;
    for &value in &slice[1..] {
        if value < min_value {
            min_value = value;
        }
        if value > max_value {
            max_value = value;
        }
    }

    let bits = required_bits(max_value - min_value);
    write_u32_le(output, 0, min_value);
    output[4] = bits as u8;

    METADATA as u32 + for_compress_bits(slice, &mut output[metadata_size()..], length, min_value, bits)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let slice = &input[..length as usize];
    let min_value = slice[0];
    let max_value = slice[slice.len() - 1];
    let bits = required_bits(max_value - min_value);

    write_u32_le(output, 0, min_value);
    output[4] = bits as u8;

    METADATA as u32 + for_compress_bits(slice, &mut output[metadata_size()..], length, min_value, bits)
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    assert!(bits <= 32);

    let mut index = 0usize;
    let mut consumed = 0usize;
    let length = length as usize;

    while index + 32 <= length {
        consumed +=
            unpack_block(base, &input[consumed..], &mut output[index..index + 32], bits) as usize;
        index += 32;
    }
    while index + 16 <= length {
        consumed +=
            unpack_block(base, &input[consumed..], &mut output[index..index + 16], bits) as usize;
        index += 16;
    }
    while index + 8 <= length {
        consumed +=
            unpack_block(base, &input[consumed..], &mut output[index..index + 8], bits) as usize;
        index += 8;
    }

    consumed += unpack_block(base, &input[consumed..], &mut output[index..length], bits) as usize;
    consumed as u32
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let min_value = read_u32_le(input, 0);
    let bits = input[4] as u32;

    METADATA as u32 + for_uncompress_bits(&input[metadata_size()..], output, length, min_value, bits)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    assert!(bits <= 32);
    assert!(value >= base);

    let delta = value - base;
    assert!(required_bits(delta) <= bits);

    if bits == 32 {
        let offset = length as usize * 4;
        write_u32_le(input, offset, delta);
        return (length + 1) * 4;
    }

    let (byte_offset, start) = locate_entry_position(length, bits);
    let first_bit = byte_offset * 8 + start;
    for shift in 0..bits as usize {
        set_bit(input, first_bit + shift, ((delta >> shift) & 1) != 0);
    }

    (byte_offset + ((start + bits as usize) + 7) / 8) as u32
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    assert!(bits <= 32);

    if bits == 32 {
        return base + read_u32_le(input, index as usize * 4);
    }

    let (byte_offset, start) = locate_entry_position(index, bits);
    base + read_bits(input, byte_offset * 8 + start, bits)
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let min_value = read_u32_le(input, 0);
    let bits = input[4] as u32;
    for_select_bits(&input[metadata_size()..], min_value, bits, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let min_value = read_u32_le(input, 0);
    let bits = input[4] as u32;
    for_linear_search_bits(&input[metadata_size()..], length, min_value, bits, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    assert!(bits <= 32);

    if bits == 0 {
        return if value == base { 0 } else { length };
    }

    for index in 0..length {
        if for_select_bits(input, base, bits, index) == value {
            return index;
        }
    }

    length
}

pub fn for_lower_bound_search(input: &[u8], length: u32, value: u32, actual: &mut u32) -> u32 {
    let min_value = read_u32_le(input, 0);
    let bits = input[4] as u32;
    for_lower_bound_search_bits(&input[metadata_size()..], length, min_value, bits, value, actual)
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

    let mut imin = 0u32;
    let mut imax = length - 1;

    while imin + 1 < imax {
        let imid = imin + ((imax - imin) / 2);
        let selected = for_select_bits(input, base, bits, imid);
        if selected >= value {
            imax = imid;
        } else {
            imin = imid;
        }
    }

    let left = for_select_bits(input, base, bits, imin);
    if left >= value {
        *actual = left;
        return imin;
    }

    let right = for_select_bits(input, base, bits, imax);
    *actual = right;
    imax
}

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
        return appendImpl(std::slice::from_ref(&value), input, 1);
    }

    let min_value = read_u32_le(input, 0);
    let bits = input[4] as u32;
    let new_bits = required_bits(value.wrapping_sub(min_value));

    if min_value > value || new_bits > bits {
        let mut decoded = vec![0u32; length as usize + 1];
        for_uncompress(input, &mut decoded, length);
        decoded[length as usize] = value;
        return appendImpl(&decoded, input, length + 1);
    }

    METADATA as u32 + for_append_bits(&mut input[metadata_size()..], length, min_value, bits, value)
}
