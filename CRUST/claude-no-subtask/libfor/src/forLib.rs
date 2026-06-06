use crate::for_gen;

pub const METADATA: i32 = 5;
const METADATA_USIZE: usize = 5;

pub fn required_bits(v: u32) -> u32 {
    if v == 0 { 0 } else { 32 - v.leading_zeros() }
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    let mut c: u32 = 0;
    let mut length = length;

    if length >= 32 {
        let b = length / 32;
        c += ((b * 32 * bits) + 7) / 8;
        length %= 32;
    }
    if length >= 16 {
        let b = length / 16;
        c += ((b * 16 * bits) + 7) / 8;
        length %= 16;
    }
    if length >= 8 {
        let b = length / 8;
        c += ((b * 8 * bits) + 7) / 8;
        length %= 8;
    }
    c + ((length * bits) + 7) / 8
}

pub fn for_compressed_size_unsorted(input: &[u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let len = length as usize;
    let mut m = input[0];
    let mut max = m;
    for i in 1..len {
        if input[i] < m { m = input[i]; }
        if input[i] > max { max = input[i]; }
    }
    let b = required_bits(max.wrapping_sub(m));
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = input[0];
    let max = input[length as usize - 1];
    let b = required_bits(max.wrapping_sub(m));
    METADATA as u32 + for_compressed_size_bits(length, b)
}

// Pack/unpack dispatch tables – mirrors the C `for_pack32[bits]` arrays.

fn pack_block_32(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    match bits {
        0 => for_gen::pack0_32(base, input, output),
        b if b <= 32 => {
            // Use the generic implementation directly.
            generic_pack(base, input, output, 32, b)
        }
        _ => 0,
    }
}

fn pack_block_16(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    if bits == 0 { return 0; }
    generic_pack(base, input, output, 16, bits)
}

fn pack_block_8(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    if bits == 0 { return 0; }
    generic_pack(base, input, output, 8, bits)
}

fn pack_block_x(bits: u32, base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if bits == 0 { return 0; }
    if length == 0 { return 0; }
    generic_pack(base, input, output, length as usize, bits)
}

fn unpack_block_32(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    if bits == 0 {
        for k in 0..32 { output[k] = base; }
        return 0;
    }
    generic_unpack(base, input, output, 32, bits)
}

fn unpack_block_16(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    if bits == 0 {
        for k in 0..16 { output[k] = base; }
        return 0;
    }
    generic_unpack(base, input, output, 16, bits)
}

fn unpack_block_8(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    if bits == 0 {
        for k in 0..8 { output[k] = base; }
        return 0;
    }
    generic_unpack(base, input, output, 8, bits)
}

fn unpack_block_x(bits: u32, base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    if bits == 0 {
        for k in 0..length as usize { output[k] = base; }
        return 0;
    }
    generic_unpack(base, input, output, length as usize, bits)
}

#[inline]
fn read_u32_le(input: &[u8], byte_offset: usize) -> u32 {
    let mut buf = [0u8; 4];
    for j in 0..4 {
        if byte_offset + j < input.len() {
            buf[j] = input[byte_offset + j];
        }
    }
    u32::from_le_bytes(buf)
}

#[inline]
fn write_u32_le_partial(output: &mut [u8], byte_offset: usize, value: u32, bytes: usize) {
    let buf = value.to_le_bytes();
    for j in 0..bytes {
        if byte_offset + j < output.len() {
            output[byte_offset + j] = buf[j];
        }
    }
}

fn generic_pack(base: u32, input: &[u32], output: &mut [u8], length: usize, bits: u32) -> u32 {
    if length == 0 { return 0; }
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;

    if bits == 32 {
        for i in 0..length {
            let v = input[i].wrapping_sub(base);
            output[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        return total_bytes as u32;
    }

    let num_words = (total_bits + 31) / 32;
    let mut words = vec![0u32; num_words];
    let mask: u32 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };

    for i in 0..length {
        let v = input[i].wrapping_sub(base) & mask;
        let bit_pos = i * bits as usize;
        let word_idx = bit_pos / 32;
        let bit_offset = (bit_pos % 32) as u32;

        if bit_offset + bits <= 32 {
            words[word_idx] |= v.wrapping_shl(bit_offset);
        } else {
            words[word_idx] |= v.wrapping_shl(bit_offset);
            let consumed = 32 - bit_offset;
            words[word_idx + 1] |= v >> consumed;
        }
    }

    for i in 0..num_words {
        let start = i * 4;
        let bytes_in_word = if start + 4 <= total_bytes { 4 } else { total_bytes - start };
        write_u32_le_partial(output, start, words[i], bytes_in_word);
    }

    total_bytes as u32
}

fn generic_unpack(base: u32, input: &[u8], output: &mut [u32], length: usize, bits: u32) -> u32 {
    if length == 0 { return 0; }
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;

    if bits == 32 {
        for i in 0..length {
            let v = read_u32_le(input, i * 4);
            output[i] = base.wrapping_add(v);
        }
        return total_bytes as u32;
    }

    let mask: u32 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };

    for i in 0..length {
        let bit_pos = i * bits as usize;
        let word_idx = bit_pos / 32;
        let bit_offset = (bit_pos % 32) as u32;
        let v = if bit_offset + bits <= 32 {
            (read_u32_le(input, word_idx * 4) >> bit_offset) & mask
        } else {
            let consumed = 32 - bit_offset;
            let low = read_u32_le(input, word_idx * 4) >> bit_offset;
            let high_mask = (1u32 << (bits - consumed)) - 1;
            let high = read_u32_le(input, (word_idx + 1) * 4) & high_mask;
            (low | (high << consumed)) & mask
        };
        output[i] = base.wrapping_add(v);
    }
    total_bytes as u32
}

pub fn for_compress_bits(input: &[u32], output: &mut [u8], length: u32, base: u32, bits: u32) -> u32 {
    let mut i: u32 = 0;
    let mut written: u32 = 0;
    let mut input_offset: usize = 0;

    while i + 32 <= length {
        let w = pack_block_32(bits, base, &input[input_offset..], &mut output[written as usize..]);
        written += w;
        i += 32;
        input_offset += 32;
    }

    while i + 16 <= length {
        let w = pack_block_16(bits, base, &input[input_offset..], &mut output[written as usize..]);
        written += w;
        i += 16;
        input_offset += 16;
    }

    while i + 8 <= length {
        let w = pack_block_8(bits, base, &input[input_offset..], &mut output[written as usize..]);
        written += w;
        i += 8;
        input_offset += 8;
    }

    let remaining = length - i;
    written + pack_block_x(bits, base, &input[input_offset..], &mut output[written as usize..], remaining)
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let len = length as usize;
    let mut m = input[0];
    let mut max = m;
    for i in 1..len {
        if input[i] < m { m = input[i]; }
        if input[i] > max { max = input[i]; }
    }
    let b = required_bits(max.wrapping_sub(m));
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA_USIZE..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = input[0];
    let max = input[length as usize - 1];
    let b = required_bits(max.wrapping_sub(m));
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA_USIZE..], length, m, b)
}

pub fn for_uncompress_bits(input: &[u8], output: &mut [u32], length: u32, base: u32, bits: u32) -> u32 {
    let mut i: u32 = 0;
    let mut consumed: u32 = 0;
    let mut output_offset: usize = 0;

    while i + 32 <= length {
        let c = unpack_block_32(bits, base, &input[consumed as usize..], &mut output[output_offset..]);
        consumed += c;
        i += 32;
        output_offset += 32;
    }
    while i + 16 <= length {
        let c = unpack_block_16(bits, base, &input[consumed as usize..], &mut output[output_offset..]);
        consumed += c;
        i += 16;
        output_offset += 16;
    }
    while i + 8 <= length {
        let c = unpack_block_8(bits, base, &input[consumed as usize..], &mut output[output_offset..]);
        consumed += c;
        i += 8;
        output_offset += 8;
    }
    let remaining = length - i;
    consumed + unpack_block_x(bits, base, &input[consumed as usize..], &mut output[output_offset..], remaining)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA_USIZE..], output, length, m, b)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    // bits == 32 special case
    if bits == 32 {
        let v = value.wrapping_sub(base);
        let off = length as usize * 4;
        input[off..off + 4].copy_from_slice(&v.to_le_bytes());
        return (length + 1) * 4;
    }

    if bits == 0 {
        // value must equal base; nothing to write
        return 0;
    }

    let mut length = length;
    let mut byte_offset: usize = 0;

    if length > 32 {
        let b = length / 32;
        byte_offset += ((b * 32 * bits) / 8) as usize;
        length %= 32;
    }
    if length > 16 {
        let b = length / 16;
        byte_offset += ((b * 16 * bits) / 8) as usize;
        length %= 16;
    }
    if length > 8 {
        let b = length / 8;
        byte_offset += ((b * 8 * bits) / 8) as usize;
        length %= 8;
    }

    let mut start = length * bits;
    byte_offset += (start / 8) as usize;
    start %= 8;

    let v = value.wrapping_sub(base);
    let mask: u32 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };

    if start + bits < 32 {
        let cur = read_u32_le(input, byte_offset);
        let cleared = cur & !(mask << start);
        let new_val = cleared | ((v & mask) << start);
        // write 4 bytes (or fewer if at end of buffer)
        write_u32_le_partial(input, byte_offset, new_val, 4.min(input.len() - byte_offset));
    } else {
        let cur1 = read_u32_le(input, byte_offset);
        let cleared1 = cur1 & !(mask << start);
        let new_val1 = cleared1 | ((v & mask) << start);
        write_u32_le_partial(input, byte_offset, new_val1, 4.min(input.len() - byte_offset));

        let bits_in_high = bits - (32 - start);
        let mask2 = if bits_in_high >= 32 { u32::MAX } else { (1u32 << bits_in_high) - 1 };
        let cur2 = read_u32_le(input, byte_offset + 4);
        let cleared2 = cur2 & !mask2;
        let new_val2 = cleared2 | (v >> (32 - start));
        let avail = if input.len() > byte_offset + 4 { input.len() - byte_offset - 4 } else { 0 };
        write_u32_le_partial(input, byte_offset + 4, new_val2, 4.min(avail));
    }

    byte_offset as u32 + ((start + bits) + 7) / 8
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(input: &mut [u8], length: u32, value: u32, appendImpl: AppendImpl) -> u32 {
    if length == 0 {
        return appendImpl(&[value], input, 1);
    }

    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;

    let bnew = if value >= m { required_bits(value - m) } else { 0 };
    if m > value || bnew > b {
        let mut tmp = vec![0u32; (length + 1) as usize];
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

pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    if bits == 0 { return base; }
    if bits == 32 {
        let v = read_u32_le(input, index as usize * 4);
        return base.wrapping_add(v);
    }

    let mut index = index;
    let mut byte_offset: usize = 0;

    if index > 32 {
        let b = index / 32;
        byte_offset += ((b * 32 * bits) / 8) as usize;
        index %= 32;
    }
    if index > 16 {
        let b = index / 16;
        byte_offset += ((b * 16 * bits) / 8) as usize;
        index %= 16;
    }
    if index > 8 {
        let b = index / 8;
        byte_offset += ((b * 8 * bits) / 8) as usize;
        index %= 8;
    }

    let mut start = index * bits;
    byte_offset += (start / 8) as usize;
    start %= 8;

    let mask: u32 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };

    if start + bits < 32 {
        let v = read_u32_le(input, byte_offset);
        base.wrapping_add((v >> start) & mask)
    } else {
        let v1 = read_u32_le(input, byte_offset);
        let v2 = read_u32_le(input, byte_offset + 4);
        let bits_in_high = bits - (32 - start);
        let mask2 = if bits_in_high >= 32 { u32::MAX } else { (1u32 << bits_in_high) - 1 };
        let low = (v1 >> start) & mask;
        let high = v2 & mask2;
        base.wrapping_add((high << (32 - start)) | low)
    }
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA_USIZE..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA_USIZE..], length, m, b, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    if bits == 0 {
        return if value == base { 0 } else { length };
    }

    let mut i: u32 = 0;
    let mut consumed: u32 = 0;
    let mut found: i32 = -1;

    while i + 32 <= length {
        let c = linsearch_block(bits, 32, base, &input[consumed as usize..], value, &mut found);
        consumed += c;
        if found >= 0 {
            return i + found as u32;
        }
        i += 32;
    }
    while i + 16 <= length {
        let c = linsearch_block(bits, 16, base, &input[consumed as usize..], value, &mut found);
        consumed += c;
        if found >= 0 {
            return i + found as u32;
        }
        i += 16;
    }
    while i + 8 <= length {
        let c = linsearch_block(bits, 8, base, &input[consumed as usize..], value, &mut found);
        consumed += c;
        if found >= 0 {
            return i + found as u32;
        }
        i += 8;
    }
    let remaining = length - i;
    linsearch_block_x(bits, base, &input[consumed as usize..], remaining, value, &mut found);
    if found >= 0 {
        return i + found as u32;
    }
    length
}

fn linsearch_block(bits: u32, block: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if bits == 0 {
        if base == value { *found = 0; }
        return 0;
    }
    let _ = block;
    // Use the same scan as the unpack block, but search for value
    let length = block as usize;
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;
    let v_target = value.wrapping_sub(base);

    if bits == 32 {
        for i in 0..length {
            let v = read_u32_le(input, i * 4);
            if v == v_target {
                *found = i as i32;
                return 0;
            }
        }
        return total_bytes as u32;
    }

    let mask: u32 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };

    for i in 0..length {
        let bit_pos = i * bits as usize;
        let word_idx = bit_pos / 32;
        let bit_offset = (bit_pos % 32) as u32;
        let v = if bit_offset + bits <= 32 {
            (read_u32_le(input, word_idx * 4) >> bit_offset) & mask
        } else {
            let consumed = 32 - bit_offset;
            let low = read_u32_le(input, word_idx * 4) >> bit_offset;
            let bits_in_high = bits - consumed;
            let high_mask = if bits_in_high >= 32 { u32::MAX } else { (1u32 << bits_in_high) - 1 };
            let high = read_u32_le(input, (word_idx + 1) * 4) & high_mask;
            (low | (high << consumed)) & mask
        };
        if v == v_target {
            *found = i as i32;
            return i as u32;
        }
    }
    total_bytes as u32
}

fn linsearch_block_x(bits: u32, base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    if bits == 0 {
        if base == value && length > 0 { *found = 0; }
        return 0;
    }
    let length = length as usize;
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;
    let v_target = value.wrapping_sub(base);

    if bits == 32 {
        for i in 0..length {
            let v = read_u32_le(input, i * 4);
            if v == v_target {
                *found = i as i32;
                return i as u32;
            }
        }
        return total_bytes as u32;
    }

    let mask: u32 = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };

    for i in 0..length {
        let bit_pos = i * bits as usize;
        let word_idx = bit_pos / 32;
        let bit_offset = (bit_pos % 32) as u32;
        let v = if bit_offset + bits <= 32 {
            (read_u32_le(input, word_idx * 4) >> bit_offset) & mask
        } else {
            let consumed = 32 - bit_offset;
            let low = read_u32_le(input, word_idx * 4) >> bit_offset;
            let bits_in_high = bits - consumed;
            let high_mask = if bits_in_high >= 32 { u32::MAX } else { (1u32 << bits_in_high) - 1 };
            let high = read_u32_le(input, (word_idx + 1) * 4) & high_mask;
            (low | (high << consumed)) & mask
        };
        if v == v_target {
            *found = i as i32;
            return i as u32;
        }
    }
    total_bytes as u32
}

pub fn for_lower_bound_search(input: &[u8], length: u32, value: u32, actual: &mut u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_lower_bound_search_bits(&input[METADATA_USIZE..], length, m, b, value, actual)
}

pub fn for_lower_bound_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32, actual: &mut u32) -> u32 {
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
