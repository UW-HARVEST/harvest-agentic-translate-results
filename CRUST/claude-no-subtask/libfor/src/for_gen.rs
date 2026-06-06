// Generic helper functions for pack/unpack/linsearch operations.
// All specialized functions delegate to these.

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

fn pack_generic(base: u32, input: &[u32], output: &mut [u8], length: usize, bits: u32) -> u32 {
    if length == 0 { return 0; }
    if bits == 0 { return 0; }
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;

    if bits == 32 {
        for i in 0..length {
            let v = input[i].wrapping_sub(base);
            let bytes = v.to_le_bytes();
            output[i * 4..i * 4 + 4].copy_from_slice(&bytes);
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
            // entire value fits within one word
            if bit_offset == 32 {
                // unreachable as bits >= 1 means bit_offset < 32
            } else {
                words[word_idx] |= v << bit_offset;
            }
        } else {
            words[word_idx] |= v << bit_offset;
            let consumed = 32 - bit_offset;
            words[word_idx + 1] |= v >> consumed;
        }
    }

    // Write out all complete words, then the partial last word
    for i in 0..num_words {
        let start = i * 4;
        let bytes_in_word = if start + 4 <= total_bytes {
            4
        } else {
            total_bytes - start
        };
        write_u32_le_partial(output, start, words[i], bytes_in_word);
    }

    total_bytes as u32
}

fn unpack_generic(base: u32, input: &[u8], output: &mut [u32], length: usize, bits: u32) -> u32 {
    if length == 0 { return 0; }
    if bits == 0 {
        for i in 0..length {
            output[i] = base;
        }
        return 0;
    }
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;

    if bits == 32 {
        for i in 0..length {
            let v = u32::from_le_bytes([
                input[i * 4],
                input[i * 4 + 1],
                input[i * 4 + 2],
                input[i * 4 + 3],
            ]);
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

fn linsearch_generic(base: u32, input: &[u8], length: usize, bits: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    if bits == 0 {
        if base == value && length > 0 {
            *found = 0;
        }
        return 0;
    }
    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;
    let v_target = value.wrapping_sub(base);

    if bits == 32 {
        for i in 0..length {
            let v = u32::from_le_bytes([
                input[i * 4],
                input[i * 4 + 1],
                input[i * 4 + 2],
                input[i * 4 + 3],
            ]);
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
            let high_mask = (1u32 << (bits - consumed)) - 1;
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

// pack0 functions for blocks
pub fn pack0_32(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn pack0_16(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn pack0_8(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn unpack0_32(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..32 { output[k] = base; }
    0
}
pub fn unpack0_16(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..16 { output[k] = base; }
    0
}
pub fn unpack0_8(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..8 { output[k] = base; }
    0
}
pub fn pack0_x(_base: u32, _input: &[u32], _output: &mut [u8], _length: u32) -> u32 { 0 }
pub fn unpack0_x(base: u32, _input: &[u8], output: &mut [u32], length: u32) -> u32 {
    for k in 0..length as usize { output[k] = base; }
    0
}
pub fn linsearch0_32(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; }
    0
}
pub fn linsearch0_16(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; }
    0
}
pub fn linsearch0_8(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; }
    0
}
pub fn linsearch0_x(base: u32, _input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if base == value && length > 0 { *found = 0; }
    0
}
pub fn pack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 1) }
pub fn unpack1_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 1) }
pub fn pack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 2) }
pub fn unpack2_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 2) }
pub fn pack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 3) }
pub fn unpack3_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 3) }
pub fn pack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 4) }
pub fn unpack4_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 4) }
pub fn pack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 5) }
pub fn unpack5_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 5) }
pub fn pack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 6) }
pub fn unpack6_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 6) }
pub fn pack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 7) }
pub fn unpack7_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 7) }
pub fn pack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 8) }
pub fn unpack8_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 8) }
pub fn pack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 9) }
pub fn unpack9_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 9) }
pub fn pack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 10) }
pub fn unpack10_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 10) }
pub fn pack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 11) }
pub fn unpack11_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 11) }
pub fn pack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 12) }
pub fn unpack12_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 12) }
pub fn pack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 13) }
pub fn unpack13_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 13) }
pub fn pack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 14) }
pub fn unpack14_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 14) }
pub fn pack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 15) }
pub fn unpack15_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 15) }
pub fn pack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 16) }
pub fn unpack16_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 16) }
pub fn pack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 17) }
pub fn unpack17_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 17) }
pub fn pack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 18) }
pub fn unpack18_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 18) }
pub fn pack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 19) }
pub fn unpack19_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 19) }
pub fn pack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 20) }
pub fn unpack20_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 20) }
pub fn pack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 21) }
pub fn unpack21_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 21) }
pub fn pack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 22) }
pub fn unpack22_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 22) }
pub fn pack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 23) }
pub fn unpack23_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 23) }
pub fn pack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 24) }
pub fn unpack24_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 24) }
pub fn pack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 25) }
pub fn unpack25_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 25) }
pub fn pack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 26) }
pub fn unpack26_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 26) }
pub fn pack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 27) }
pub fn unpack27_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 27) }
pub fn pack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 28) }
pub fn unpack28_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 28) }
pub fn pack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 29) }
pub fn unpack29_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 29) }
pub fn pack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 30) }
pub fn unpack30_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 30) }
pub fn pack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 31) }
pub fn unpack31_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 31) }
pub fn pack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 32, 32) }
pub fn unpack32_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 32, 32) }
pub fn pack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 1) }
pub fn unpack1_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 1) }
pub fn pack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 2) }
pub fn unpack2_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 2) }
pub fn pack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 3) }
pub fn unpack3_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 3) }
pub fn pack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 4) }
pub fn unpack4_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 4) }
pub fn pack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 5) }
pub fn unpack5_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 5) }
pub fn pack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 6) }
pub fn unpack6_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 6) }
pub fn pack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 7) }
pub fn unpack7_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 7) }
pub fn pack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 8) }
pub fn unpack8_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 8) }
pub fn pack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 9) }
pub fn unpack9_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 9) }
pub fn pack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 10) }
pub fn unpack10_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 10) }
pub fn pack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 11) }
pub fn unpack11_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 11) }
pub fn pack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 12) }
pub fn unpack12_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 12) }
pub fn pack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 13) }
pub fn unpack13_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 13) }
pub fn pack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 14) }
pub fn unpack14_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 14) }
pub fn pack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 15) }
pub fn unpack15_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 15) }
pub fn pack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 16) }
pub fn unpack16_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 16) }
pub fn pack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 17) }
pub fn unpack17_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 17) }
pub fn pack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 18) }
pub fn unpack18_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 18) }
pub fn pack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 19) }
pub fn unpack19_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 19) }
pub fn pack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 20) }
pub fn unpack20_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 20) }
pub fn pack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 21) }
pub fn unpack21_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 21) }
pub fn pack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 22) }
pub fn unpack22_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 22) }
pub fn pack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 23) }
pub fn unpack23_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 23) }
pub fn pack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 24) }
pub fn unpack24_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 24) }
pub fn pack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 25) }
pub fn unpack25_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 25) }
pub fn pack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 26) }
pub fn unpack26_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 26) }
pub fn pack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 27) }
pub fn unpack27_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 27) }
pub fn pack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 28) }
pub fn unpack28_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 28) }
pub fn pack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 29) }
pub fn unpack29_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 29) }
pub fn pack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 30) }
pub fn unpack30_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 30) }
pub fn pack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 31) }
pub fn unpack31_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 31) }
pub fn pack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 16, 32) }
pub fn unpack32_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 16, 32) }
pub fn pack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 1) }
pub fn unpack1_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 1) }
pub fn pack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 2) }
pub fn unpack2_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 2) }
pub fn pack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 3) }
pub fn unpack3_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 3) }
pub fn pack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 4) }
pub fn unpack4_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 4) }
pub fn pack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 5) }
pub fn unpack5_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 5) }
pub fn pack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 6) }
pub fn unpack6_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 6) }
pub fn pack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 7) }
pub fn unpack7_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 7) }
pub fn pack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 8) }
pub fn unpack8_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 8) }
pub fn pack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 9) }
pub fn unpack9_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 9) }
pub fn pack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 10) }
pub fn unpack10_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 10) }
pub fn pack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 11) }
pub fn unpack11_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 11) }
pub fn pack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 12) }
pub fn unpack12_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 12) }
pub fn pack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 13) }
pub fn unpack13_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 13) }
pub fn pack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 14) }
pub fn unpack14_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 14) }
pub fn pack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 15) }
pub fn unpack15_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 15) }
pub fn pack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 16) }
pub fn unpack16_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 16) }
pub fn pack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 17) }
pub fn unpack17_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 17) }
pub fn pack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 18) }
pub fn unpack18_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 18) }
pub fn pack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 19) }
pub fn unpack19_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 19) }
pub fn pack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 20) }
pub fn unpack20_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 20) }
pub fn pack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 21) }
pub fn unpack21_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 21) }
pub fn pack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 22) }
pub fn unpack22_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 22) }
pub fn pack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 23) }
pub fn unpack23_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 23) }
pub fn pack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 24) }
pub fn unpack24_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 24) }
pub fn pack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 25) }
pub fn unpack25_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 25) }
pub fn pack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 26) }
pub fn unpack26_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 26) }
pub fn pack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 27) }
pub fn unpack27_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 27) }
pub fn pack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 28) }
pub fn unpack28_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 28) }
pub fn pack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 29) }
pub fn unpack29_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 29) }
pub fn pack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 30) }
pub fn unpack30_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 30) }
pub fn pack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 31) }
pub fn unpack31_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 31) }
pub fn pack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_generic(base, input, output, 8, 32) }
pub fn unpack32_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_generic(base, input, output, 8, 32) }
pub fn pack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 1) }
pub fn unpack1_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 1) }
pub fn pack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 2) }
pub fn unpack2_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 2) }
pub fn pack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 3) }
pub fn unpack3_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 3) }
pub fn pack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 4) }
pub fn unpack4_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 4) }
pub fn pack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 5) }
pub fn unpack5_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 5) }
pub fn pack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 6) }
pub fn unpack6_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 6) }
pub fn pack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 7) }
pub fn unpack7_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 7) }
pub fn pack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 8) }
pub fn unpack8_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 8) }
pub fn pack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 9) }
pub fn unpack9_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 9) }
pub fn pack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 10) }
pub fn unpack10_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 10) }
pub fn pack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 11) }
pub fn unpack11_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 11) }
pub fn pack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 12) }
pub fn unpack12_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 12) }
pub fn pack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 13) }
pub fn unpack13_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 13) }
pub fn pack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 14) }
pub fn unpack14_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 14) }
pub fn pack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 15) }
pub fn unpack15_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 15) }
pub fn pack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 16) }
pub fn unpack16_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 16) }
pub fn pack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 17) }
pub fn unpack17_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 17) }
pub fn pack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 18) }
pub fn unpack18_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 18) }
pub fn pack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 19) }
pub fn unpack19_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 19) }
pub fn pack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 20) }
pub fn unpack20_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 20) }
pub fn pack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 21) }
pub fn unpack21_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 21) }
pub fn pack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 22) }
pub fn unpack22_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 22) }
pub fn pack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 23) }
pub fn unpack23_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 23) }
pub fn pack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 24) }
pub fn unpack24_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 24) }
pub fn pack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 25) }
pub fn unpack25_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 25) }
pub fn pack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 26) }
pub fn unpack26_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 26) }
pub fn pack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 27) }
pub fn unpack27_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 27) }
pub fn pack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 28) }
pub fn unpack28_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 28) }
pub fn pack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 29) }
pub fn unpack29_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 29) }
pub fn pack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 30) }
pub fn unpack30_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 30) }
pub fn pack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 31) }
pub fn unpack31_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 31) }
pub fn pack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_generic(base, input, output, length as usize, 32) }
pub fn unpack32_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 { unpack_generic(base, input, output, length as usize, 32) }
pub fn linsearch1_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 1, value, found) }
pub fn linsearch2_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 2, value, found) }
pub fn linsearch3_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 3, value, found) }
pub fn linsearch4_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 4, value, found) }
pub fn linsearch5_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 5, value, found) }
pub fn linsearch6_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 6, value, found) }
pub fn linsearch7_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 7, value, found) }
pub fn linsearch8_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 8, value, found) }
pub fn linsearch9_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 9, value, found) }
pub fn linsearch10_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 10, value, found) }
pub fn linsearch11_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 11, value, found) }
pub fn linsearch12_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 12, value, found) }
pub fn linsearch13_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 13, value, found) }
pub fn linsearch14_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 14, value, found) }
pub fn linsearch15_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 15, value, found) }
pub fn linsearch16_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 16, value, found) }
pub fn linsearch17_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 17, value, found) }
pub fn linsearch18_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 18, value, found) }
pub fn linsearch19_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 19, value, found) }
pub fn linsearch20_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 20, value, found) }
pub fn linsearch21_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 21, value, found) }
pub fn linsearch22_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 22, value, found) }
pub fn linsearch23_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 23, value, found) }
pub fn linsearch24_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 24, value, found) }
pub fn linsearch25_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 25, value, found) }
pub fn linsearch26_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 26, value, found) }
pub fn linsearch27_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 27, value, found) }
pub fn linsearch28_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 28, value, found) }
pub fn linsearch29_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 29, value, found) }
pub fn linsearch30_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 30, value, found) }
pub fn linsearch31_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 31, value, found) }
pub fn linsearch32_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 32, 32, value, found) }
pub fn linsearch1_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 1, value, found) }
pub fn linsearch2_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 2, value, found) }
pub fn linsearch3_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 3, value, found) }
pub fn linsearch4_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 4, value, found) }
pub fn linsearch5_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 5, value, found) }
pub fn linsearch6_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 6, value, found) }
pub fn linsearch7_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 7, value, found) }
pub fn linsearch8_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 8, value, found) }
pub fn linsearch9_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 9, value, found) }
pub fn linsearch10_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 10, value, found) }
pub fn linsearch11_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 11, value, found) }
pub fn linsearch12_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 12, value, found) }
pub fn linsearch13_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 13, value, found) }
pub fn linsearch14_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 14, value, found) }
pub fn linsearch15_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 15, value, found) }
pub fn linsearch16_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 16, value, found) }
pub fn linsearch17_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 17, value, found) }
pub fn linsearch18_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 18, value, found) }
pub fn linsearch19_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 19, value, found) }
pub fn linsearch20_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 20, value, found) }
pub fn linsearch21_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 21, value, found) }
pub fn linsearch22_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 22, value, found) }
pub fn linsearch23_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 23, value, found) }
pub fn linsearch24_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 24, value, found) }
pub fn linsearch25_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 25, value, found) }
pub fn linsearch26_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 26, value, found) }
pub fn linsearch27_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 27, value, found) }
pub fn linsearch28_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 28, value, found) }
pub fn linsearch29_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 29, value, found) }
pub fn linsearch30_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 30, value, found) }
pub fn linsearch31_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 31, value, found) }
pub fn linsearch32_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 16, 32, value, found) }
pub fn linsearch1_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 1, value, found) }
pub fn linsearch2_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 2, value, found) }
pub fn linsearch3_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 3, value, found) }
pub fn linsearch4_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 4, value, found) }
pub fn linsearch5_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 5, value, found) }
pub fn linsearch6_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 6, value, found) }
pub fn linsearch7_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 7, value, found) }
pub fn linsearch8_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 8, value, found) }
pub fn linsearch9_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 9, value, found) }
pub fn linsearch10_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 10, value, found) }
pub fn linsearch11_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 11, value, found) }
pub fn linsearch12_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 12, value, found) }
pub fn linsearch13_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 13, value, found) }
pub fn linsearch14_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 14, value, found) }
pub fn linsearch15_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 15, value, found) }
pub fn linsearch16_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 16, value, found) }
pub fn linsearch17_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 17, value, found) }
pub fn linsearch18_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 18, value, found) }
pub fn linsearch19_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 19, value, found) }
pub fn linsearch20_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 20, value, found) }
pub fn linsearch21_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 21, value, found) }
pub fn linsearch22_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 22, value, found) }
pub fn linsearch23_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 23, value, found) }
pub fn linsearch24_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 24, value, found) }
pub fn linsearch25_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 25, value, found) }
pub fn linsearch26_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 26, value, found) }
pub fn linsearch27_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 27, value, found) }
pub fn linsearch28_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 28, value, found) }
pub fn linsearch29_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 29, value, found) }
pub fn linsearch30_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 30, value, found) }
pub fn linsearch31_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 31, value, found) }
pub fn linsearch32_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, 8, 32, value, found) }
pub fn linsearch1_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 1, value, found) }
pub fn linsearch2_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 2, value, found) }
pub fn linsearch3_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 3, value, found) }
pub fn linsearch4_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 4, value, found) }
pub fn linsearch5_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 5, value, found) }
pub fn linsearch6_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 6, value, found) }
pub fn linsearch7_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 7, value, found) }
pub fn linsearch8_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 8, value, found) }
pub fn linsearch9_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 9, value, found) }
pub fn linsearch10_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 10, value, found) }
pub fn linsearch11_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 11, value, found) }
pub fn linsearch12_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 12, value, found) }
pub fn linsearch13_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 13, value, found) }
pub fn linsearch14_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 14, value, found) }
pub fn linsearch15_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 15, value, found) }
pub fn linsearch16_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 16, value, found) }
pub fn linsearch17_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 17, value, found) }
pub fn linsearch18_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 18, value, found) }
pub fn linsearch19_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 19, value, found) }
pub fn linsearch20_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 20, value, found) }
pub fn linsearch21_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 21, value, found) }
pub fn linsearch22_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 22, value, found) }
pub fn linsearch23_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 23, value, found) }
pub fn linsearch24_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 24, value, found) }
pub fn linsearch25_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 25, value, found) }
pub fn linsearch26_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 26, value, found) }
pub fn linsearch27_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 27, value, found) }
pub fn linsearch28_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 28, value, found) }
pub fn linsearch29_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 29, value, found) }
pub fn linsearch30_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 30, value, found) }
pub fn linsearch31_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 31, value, found) }
pub fn linsearch32_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_generic(base, input, length as usize, 32, value, found) }
