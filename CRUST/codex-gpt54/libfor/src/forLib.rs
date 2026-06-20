pub const METADATA: i32 = 5;

fn metadata_size() -> usize {
    METADATA as usize
}

fn read_u32_le(input: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&input[..4]);
    u32::from_le_bytes(bytes)
}

fn write_u32_le(output: &mut [u8], value: u32) {
    output[..4].copy_from_slice(&value.to_le_bytes());
}

fn read_packed_value(input: &[u8], bits: u32, index: usize) -> u32 {
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        let start = index * 4;
        return read_u32_le(&input[start..start + 4]);
    }

    let bit_offset = index * bits as usize;
    let byte_offset = bit_offset / 8;
    let bit_in_byte = bit_offset % 8;
    let bytes_to_read = (((bit_in_byte + bits as usize).saturating_sub(1)) / 8) + 1;

    let mut word = 0u64;
    for (shift, byte) in input[byte_offset..byte_offset + bytes_to_read]
        .iter()
        .enumerate()
    {
        word |= (*byte as u64) << (shift * 8);
    }

    let mask = (1u64 << bits) - 1;
    ((word >> bit_in_byte) & mask) as u32
}

fn write_packed_value(output: &mut [u8], bits: u32, index: usize, value: u32) {
    if bits == 0 {
        return;
    }
    if bits == 32 {
        let start = index * 4;
        write_u32_le(&mut output[start..start + 4], value);
        return;
    }

    let bit_offset = index * bits as usize;
    let byte_offset = bit_offset / 8;
    let bit_in_byte = bit_offset % 8;
    let bytes_to_write = (((bit_in_byte + bits as usize).saturating_sub(1)) / 8) + 1;

    let mut word = 0u64;
    for (shift, byte) in output[byte_offset..byte_offset + bytes_to_write]
        .iter()
        .enumerate()
    {
        word |= (*byte as u64) << (shift * 8);
    }

    let value_mask = (1u64 << bits) - 1;
    let mask = value_mask << bit_in_byte;
    word = (word & !mask) | (((value as u64) & value_mask) << bit_in_byte);

    for (shift, byte) in output[byte_offset..byte_offset + bytes_to_write]
        .iter_mut()
        .enumerate()
    {
        *byte = ((word >> (shift * 8)) & 0xff) as u8;
    }
}

fn pack_values(input: &[u32], output: &mut [u8], length: usize, base: u32, bits: u32) -> u32 {
    if bits > 32 {
        return 0;
    }
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        for (slot, value) in output[..length * 4]
            .chunks_exact_mut(4)
            .zip(input.iter().take(length))
        {
            write_u32_le(slot, value.wrapping_sub(base));
        }
        return (length * 4) as u32;
    }

    let bytes = ((length * bits as usize) + 7) / 8;
    output[..bytes].fill(0);
    for (index, value) in input.iter().take(length).enumerate() {
        write_packed_value(output, bits, index, value.wrapping_sub(base));
    }
    bytes as u32
}

fn unpack_values(input: &[u8], output: &mut [u32], length: usize, base: u32, bits: u32) -> u32 {
    if bits > 32 {
        return 0;
    }
    if bits == 0 {
        output[..length].fill(base);
        return 0;
    }
    if bits == 32 {
        for (index, slot) in output.iter_mut().take(length).enumerate() {
            let start = index * 4;
            *slot = base.wrapping_add(read_u32_le(&input[start..start + 4]));
        }
        return (length * 4) as u32;
    }

    for (index, slot) in output.iter_mut().take(length).enumerate() {
        *slot = base.wrapping_add(read_packed_value(input, bits, index));
    }
    (((length * bits as usize) + 7) / 8) as u32
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    if bits > 32 {
        return 0;
    }

    let mut c = 0;
    let mut remaining = length;

    if remaining >= 32 {
        let blocks = remaining / 32;
        c += ((blocks * 32 * bits) + 7) / 8;
        remaining %= 32;
    }

    if remaining >= 16 {
        let blocks = remaining / 16;
        c += ((blocks * 16 * bits) + 7) / 8;
        remaining %= 16;
    }

    if remaining >= 8 {
        let blocks = remaining / 8;
        c += ((blocks * 8 * bits) + 7) / 8;
        remaining %= 8;
    }

    c + ((remaining * bits) + 7) / 8
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let values = &input[..length as usize];
    let mut min_value = values[0];
    let mut max_value = values[0];
    for &value in &values[1..] {
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    let bits = required_bits(max_value - min_value);
    METADATA as u32 + for_compressed_size_bits(length, bits)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let values = &input[..length as usize];
    let min_value = values[0];
    let max_value = values[values.len() - 1];
    let bits = required_bits(max_value - min_value);

    METADATA as u32 + for_compressed_size_bits(length, bits)
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    pack_values(input, output, length as usize, base, bits)
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let values = &input[..length as usize];
    let mut min_value = values[0];
    let mut max_value = values[0];
    for &value in &values[1..] {
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    let bits = required_bits(max_value - min_value);
    write_u32_le(&mut output[..4], min_value);
    output[4] = bits as u8;
    METADATA as u32
        + for_compress_bits(
            values,
            &mut output[metadata_size()..],
            length,
            min_value,
            bits,
        )
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let values = &input[..length as usize];
    let min_value = values[0];
    let max_value = values[values.len() - 1];
    let bits = required_bits(max_value - min_value);

    write_u32_le(&mut output[..4], min_value);
    output[4] = bits as u8;
    METADATA as u32
        + for_compress_bits(
            values,
            &mut output[metadata_size()..],
            length,
            min_value,
            bits,
        )
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    unpack_values(input, output, length as usize, base, bits)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }

    let base = read_u32_le(&input[..4]);
    let bits = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[metadata_size()..], output, length, base, bits)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits > 32 || value < base {
        return 0;
    }

    let delta = value - base;
    if bits != 32 && required_bits(delta) > bits {
        return 0;
    }

    if bits == 32 {
        let start = length as usize * 4;
        write_u32_le(&mut input[start..start + 4], delta);
        return ((length + 1) * 4) as u32;
    }

    write_packed_value(input, bits, length as usize, delta);
    (((length as usize * bits as usize) + bits as usize + 7) / 8) as u32
}

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    if bits > 32 {
        return base;
    }
    base.wrapping_add(read_packed_value(input, bits, index as usize))
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let base = read_u32_le(&input[..4]);
    let bits = input[4] as u32;
    for_select_bits(&input[metadata_size()..], base, bits, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let base = read_u32_le(&input[..4]);
    let bits = input[4] as u32;
    for_linear_search_bits(&input[metadata_size()..], length, base, bits, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits > 32 {
        return length;
    }
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
    let base = read_u32_le(&input[..4]);
    let bits = input[4] as u32;
    for_lower_bound_search_bits(&input[metadata_size()..], length, base, bits, value, actual)
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

    let mut imin = 0;
    let mut imax = length - 1;

    while imin + 1 < imax {
        let imid = imin + ((imax - imin) / 2);
        let current = for_select_bits(input, base, bits, imid);
        if current >= value {
            imax = imid;
        } else {
            imin = imid;
        }
    }

    let lower = for_select_bits(input, base, bits, imin);
    if lower >= value {
        *actual = lower;
        return imin;
    }

    let upper = for_select_bits(input, base, bits, imax);
    *actual = upper;
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
        return appendImpl(&[value], input, 1);
    }

    let base = read_u32_le(&input[..4]);
    let bits = input[4] as u32;
    let new_bits = if value >= base {
        required_bits(value - base)
    } else {
        33
    };

    if value < base || new_bits > bits {
        let mut values = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut values[..length as usize], length);
        values[length as usize] = value;
        return appendImpl(&values, input, length + 1);
    }

    METADATA as u32 + for_append_bits(&mut input[metadata_size()..], length, base, bits, value)
}
