use crate::for_gen::*;

pub const METADATA: i32 = 5;

// Helper function: number of bits required to represent v.
pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    let mut length = length;
    let mut c: u32 = 0;
    let mut b: u32;

    if length >= 32 {
        b = length / 32;
        c += (b * 32 * bits + 7) / 8;
        length %= 32;
    }

    if length >= 16 {
        b = length / 16;
        c += (b * 16 * bits + 7) / 8;
        length %= 16;
    }

    if length >= 8 {
        b = length / 8;
        c += (b * 8 * bits + 7) / 8;
        length %= 8;
    }

    c + (length * bits + 7) / 8
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

// Dispatch helpers that mirror the C function-pointer tables.
fn pack_block_32(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    match bits {
        0 => pack0_32(base, input, output),
        1 => pack1_32(base, input, output),
        2 => pack2_32(base, input, output),
        3 => pack3_32(base, input, output),
        4 => pack4_32(base, input, output),
        5 => pack5_32(base, input, output),
        6 => pack6_32(base, input, output),
        7 => pack7_32(base, input, output),
        8 => pack8_32(base, input, output),
        9 => pack9_32(base, input, output),
        10 => pack10_32(base, input, output),
        11 => pack11_32(base, input, output),
        12 => pack12_32(base, input, output),
        13 => pack13_32(base, input, output),
        14 => pack14_32(base, input, output),
        15 => pack15_32(base, input, output),
        16 => pack16_32(base, input, output),
        17 => pack17_32(base, input, output),
        18 => pack18_32(base, input, output),
        19 => pack19_32(base, input, output),
        20 => pack20_32(base, input, output),
        21 => pack21_32(base, input, output),
        22 => pack22_32(base, input, output),
        23 => pack23_32(base, input, output),
        24 => pack24_32(base, input, output),
        25 => pack25_32(base, input, output),
        26 => pack26_32(base, input, output),
        27 => pack27_32(base, input, output),
        28 => pack28_32(base, input, output),
        29 => pack29_32(base, input, output),
        30 => pack30_32(base, input, output),
        31 => pack31_32(base, input, output),
        32 => pack32_32(base, input, output),
        _ => unreachable!(),
    }
}

fn pack_block_16(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    match bits {
        0 => pack0_16(base, input, output),
        1 => pack1_16(base, input, output),
        2 => pack2_16(base, input, output),
        3 => pack3_16(base, input, output),
        4 => pack4_16(base, input, output),
        5 => pack5_16(base, input, output),
        6 => pack6_16(base, input, output),
        7 => pack7_16(base, input, output),
        8 => pack8_16(base, input, output),
        9 => pack9_16(base, input, output),
        10 => pack10_16(base, input, output),
        11 => pack11_16(base, input, output),
        12 => pack12_16(base, input, output),
        13 => pack13_16(base, input, output),
        14 => pack14_16(base, input, output),
        15 => pack15_16(base, input, output),
        16 => pack16_16(base, input, output),
        17 => pack17_16(base, input, output),
        18 => pack18_16(base, input, output),
        19 => pack19_16(base, input, output),
        20 => pack20_16(base, input, output),
        21 => pack21_16(base, input, output),
        22 => pack22_16(base, input, output),
        23 => pack23_16(base, input, output),
        24 => pack24_16(base, input, output),
        25 => pack25_16(base, input, output),
        26 => pack26_16(base, input, output),
        27 => pack27_16(base, input, output),
        28 => pack28_16(base, input, output),
        29 => pack29_16(base, input, output),
        30 => pack30_16(base, input, output),
        31 => pack31_16(base, input, output),
        32 => pack32_16(base, input, output),
        _ => unreachable!(),
    }
}

fn pack_block_8(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    match bits {
        0 => pack0_8(base, input, output),
        1 => pack1_8(base, input, output),
        2 => pack2_8(base, input, output),
        3 => pack3_8(base, input, output),
        4 => pack4_8(base, input, output),
        5 => pack5_8(base, input, output),
        6 => pack6_8(base, input, output),
        7 => pack7_8(base, input, output),
        8 => pack8_8(base, input, output),
        9 => pack9_8(base, input, output),
        10 => pack10_8(base, input, output),
        11 => pack11_8(base, input, output),
        12 => pack12_8(base, input, output),
        13 => pack13_8(base, input, output),
        14 => pack14_8(base, input, output),
        15 => pack15_8(base, input, output),
        16 => pack16_8(base, input, output),
        17 => pack17_8(base, input, output),
        18 => pack18_8(base, input, output),
        19 => pack19_8(base, input, output),
        20 => pack20_8(base, input, output),
        21 => pack21_8(base, input, output),
        22 => pack22_8(base, input, output),
        23 => pack23_8(base, input, output),
        24 => pack24_8(base, input, output),
        25 => pack25_8(base, input, output),
        26 => pack26_8(base, input, output),
        27 => pack27_8(base, input, output),
        28 => pack28_8(base, input, output),
        29 => pack29_8(base, input, output),
        30 => pack30_8(base, input, output),
        31 => pack31_8(base, input, output),
        32 => pack32_8(base, input, output),
        _ => unreachable!(),
    }
}

fn pack_block_x(bits: u32, base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    match bits {
        0 => pack0_x(base, input, output, length),
        1 => pack1_x(base, input, output, length),
        2 => pack2_x(base, input, output, length),
        3 => pack3_x(base, input, output, length),
        4 => pack4_x(base, input, output, length),
        5 => pack5_x(base, input, output, length),
        6 => pack6_x(base, input, output, length),
        7 => pack7_x(base, input, output, length),
        8 => pack8_x(base, input, output, length),
        9 => pack9_x(base, input, output, length),
        10 => pack10_x(base, input, output, length),
        11 => pack11_x(base, input, output, length),
        12 => pack12_x(base, input, output, length),
        13 => pack13_x(base, input, output, length),
        14 => pack14_x(base, input, output, length),
        15 => pack15_x(base, input, output, length),
        16 => pack16_x(base, input, output, length),
        17 => pack17_x(base, input, output, length),
        18 => pack18_x(base, input, output, length),
        19 => pack19_x(base, input, output, length),
        20 => pack20_x(base, input, output, length),
        21 => pack21_x(base, input, output, length),
        22 => pack22_x(base, input, output, length),
        23 => pack23_x(base, input, output, length),
        24 => pack24_x(base, input, output, length),
        25 => pack25_x(base, input, output, length),
        26 => pack26_x(base, input, output, length),
        27 => pack27_x(base, input, output, length),
        28 => pack28_x(base, input, output, length),
        29 => pack29_x(base, input, output, length),
        30 => pack30_x(base, input, output, length),
        31 => pack31_x(base, input, output, length),
        32 => pack32_x(base, input, output, length),
        _ => unreachable!(),
    }
}

fn unpack_block_32(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    match bits {
        0 => unpack0_32(base, input, output),
        _ => crate::for_gen::unpack_dispatch::un32(bits, base, input, output),
    }
}

fn unpack_block_16(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    match bits {
        0 => unpack0_16(base, input, output),
        1 => crate::for_gen::unpack_dispatch::un16(1, base, input, output),
        2 => crate::for_gen::unpack_dispatch::un16(2, base, input, output),
        3 => crate::for_gen::unpack_dispatch::un16(3, base, input, output),
        4 => crate::for_gen::unpack_dispatch::un16(4, base, input, output),
        5 => crate::for_gen::unpack_dispatch::un16(5, base, input, output),
        6 => crate::for_gen::unpack_dispatch::un16(6, base, input, output),
        7 => crate::for_gen::unpack_dispatch::un16(7, base, input, output),
        8 => crate::for_gen::unpack_dispatch::un16(8, base, input, output),
        9 => crate::for_gen::unpack_dispatch::un16(9, base, input, output),
        10 => crate::for_gen::unpack_dispatch::un16(10, base, input, output),
        11 => crate::for_gen::unpack_dispatch::un16(11, base, input, output),
        12 => crate::for_gen::unpack_dispatch::un16(12, base, input, output),
        13 => crate::for_gen::unpack_dispatch::un16(13, base, input, output),
        14 => crate::for_gen::unpack_dispatch::un16(14, base, input, output),
        15 => crate::for_gen::unpack_dispatch::un16(15, base, input, output),
        16 => crate::for_gen::unpack_dispatch::un16(16, base, input, output),
        17 => crate::for_gen::unpack_dispatch::un16(17, base, input, output),
        18 => crate::for_gen::unpack_dispatch::un16(18, base, input, output),
        19 => crate::for_gen::unpack_dispatch::un16(19, base, input, output),
        20 => crate::for_gen::unpack_dispatch::un16(20, base, input, output),
        21 => crate::for_gen::unpack_dispatch::un16(21, base, input, output),
        22 => crate::for_gen::unpack_dispatch::un16(22, base, input, output),
        23 => crate::for_gen::unpack_dispatch::un16(23, base, input, output),
        24 => crate::for_gen::unpack_dispatch::un16(24, base, input, output),
        25 => crate::for_gen::unpack_dispatch::un16(25, base, input, output),
        26 => crate::for_gen::unpack_dispatch::un16(26, base, input, output),
        27 => crate::for_gen::unpack_dispatch::un16(27, base, input, output),
        28 => crate::for_gen::unpack_dispatch::un16(28, base, input, output),
        29 => crate::for_gen::unpack_dispatch::un16(29, base, input, output),
        30 => crate::for_gen::unpack_dispatch::un16(30, base, input, output),
        31 => crate::for_gen::unpack_dispatch::un16(31, base, input, output),
        32 => crate::for_gen::unpack_dispatch::un16(32, base, input, output),
        _ => unreachable!(),
    }
}

fn unpack_block_8(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    match bits {
        0 => unpack0_8(base, input, output),
        1 => crate::for_gen::unpack_dispatch::un8(1, base, input, output),
        2 => crate::for_gen::unpack_dispatch::un8(2, base, input, output),
        3 => crate::for_gen::unpack_dispatch::un8(3, base, input, output),
        4 => crate::for_gen::unpack_dispatch::un8(4, base, input, output),
        5 => crate::for_gen::unpack_dispatch::un8(5, base, input, output),
        6 => crate::for_gen::unpack_dispatch::un8(6, base, input, output),
        7 => crate::for_gen::unpack_dispatch::un8(7, base, input, output),
        8 => crate::for_gen::unpack_dispatch::un8(8, base, input, output),
        9 => crate::for_gen::unpack_dispatch::un8(9, base, input, output),
        10 => crate::for_gen::unpack_dispatch::un8(10, base, input, output),
        11 => crate::for_gen::unpack_dispatch::un8(11, base, input, output),
        12 => crate::for_gen::unpack_dispatch::un8(12, base, input, output),
        13 => crate::for_gen::unpack_dispatch::un8(13, base, input, output),
        14 => crate::for_gen::unpack_dispatch::un8(14, base, input, output),
        15 => crate::for_gen::unpack_dispatch::un8(15, base, input, output),
        16 => crate::for_gen::unpack_dispatch::un8(16, base, input, output),
        17 => crate::for_gen::unpack_dispatch::un8(17, base, input, output),
        18 => crate::for_gen::unpack_dispatch::un8(18, base, input, output),
        19 => crate::for_gen::unpack_dispatch::un8(19, base, input, output),
        20 => crate::for_gen::unpack_dispatch::un8(20, base, input, output),
        21 => crate::for_gen::unpack_dispatch::un8(21, base, input, output),
        22 => crate::for_gen::unpack_dispatch::un8(22, base, input, output),
        23 => crate::for_gen::unpack_dispatch::un8(23, base, input, output),
        24 => crate::for_gen::unpack_dispatch::un8(24, base, input, output),
        25 => crate::for_gen::unpack_dispatch::un8(25, base, input, output),
        26 => crate::for_gen::unpack_dispatch::un8(26, base, input, output),
        27 => crate::for_gen::unpack_dispatch::un8(27, base, input, output),
        28 => crate::for_gen::unpack_dispatch::un8(28, base, input, output),
        29 => crate::for_gen::unpack_dispatch::un8(29, base, input, output),
        30 => crate::for_gen::unpack_dispatch::un8(30, base, input, output),
        31 => crate::for_gen::unpack_dispatch::un8(31, base, input, output),
        32 => crate::for_gen::unpack_dispatch::un8(32, base, input, output),
        _ => unreachable!(),
    }
}

fn unpack_block_x(bits: u32, base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    match bits {
        0 => unpack0_x(base, input, output, length),
        _ => crate::for_gen::unpack_dispatch::unx(bits, base, input, output, length),
    }
}

fn linsearch_block_32(bits: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    match bits {
        0 => linsearch0_32(base, input, value, found),
        _ => crate::for_gen::linsearch_dispatch::ls_n(bits, 32, base, input, value, found),
    }
}

fn linsearch_block_16(bits: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    match bits {
        0 => linsearch0_16(base, input, value, found),
        _ => crate::for_gen::linsearch_dispatch::ls_n(bits, 16, base, input, value, found),
    }
}

fn linsearch_block_8(bits: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    match bits {
        0 => linsearch0_8(base, input, value, found),
        _ => crate::for_gen::linsearch_dispatch::ls_n(bits, 8, base, input, value, found),
    }
}

fn linsearch_block_x(bits: u32, base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    match bits {
        0 => linsearch0_x(base, input, length, value, found),
        _ => crate::for_gen::linsearch_dispatch::ls_x(bits, base, input, length, value, found),
    }
}

pub fn for_compress_bits(input: &[u32], output: &mut [u8], length: u32, base: u32, bits: u32) -> u32 {
    let mut i: u32 = 0;
    let mut written: u32 = 0;
    let mut in_off: usize = 0;

    while i + 32 <= length {
        let w = pack_block_32(bits, base, &input[in_off..], &mut output[written as usize..]);
        written += w;
        i += 32;
        in_off += 32;
    }

    while i + 16 <= length {
        let w = pack_block_16(bits, base, &input[in_off..], &mut output[written as usize..]);
        written += w;
        i += 16;
        in_off += 16;
    }

    while i + 8 <= length {
        let w = pack_block_8(bits, base, &input[in_off..], &mut output[written as usize..]);
        written += w;
        i += 8;
        in_off += 8;
    }

    written + pack_block_x(bits, base, &input[in_off..], &mut output[written as usize..], length - i)
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
    let b = required_bits(max_v.wrapping_sub(m));
    let m_bytes = m.to_le_bytes();
    output[0..4].copy_from_slice(&m_bytes);
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[(length - 1) as usize];
    let b = required_bits(max_v.wrapping_sub(m));
    let m_bytes = m.to_le_bytes();
    output[0..4].copy_from_slice(&m_bytes);
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_uncompress_bits(input: &[u8], output: &mut [u32], length: u32, base: u32, bits: u32) -> u32 {
    let mut i: u32 = 0;
    let mut consumed: u32 = 0;
    let mut out_off: usize = 0;

    while i + 32 <= length {
        let c = unpack_block_32(bits, base, &input[consumed as usize..], &mut output[out_off..]);
        consumed += c;
        i += 32;
        out_off += 32;
    }
    while i + 16 <= length {
        let c = unpack_block_16(bits, base, &input[consumed as usize..], &mut output[out_off..]);
        consumed += c;
        i += 16;
        out_off += 16;
    }
    while i + 8 <= length {
        let c = unpack_block_8(bits, base, &input[consumed as usize..], &mut output[out_off..]);
        consumed += c;
        i += 8;
        out_off += 8;
    }
    consumed + unpack_block_x(bits, base, &input[consumed as usize..], &mut output[out_off..], length - i)
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let mut m_buf = [0u8; 4];
    m_buf.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(m_buf);
    let b = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA as usize..], output, length, m, b)
}

pub fn for_append_bits(input: &mut [u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    let mut length = length;
    let initin_len = input.len();
    let mut offset: usize = 0;

    if bits == 32 {
        // Treat input as u32-aligned; just write at index `length`.
        let v = value.wrapping_sub(base);
        let bytes = v.to_le_bytes();
        let pos = (length as usize) * 4;
        input[pos..pos + 4].copy_from_slice(&bytes);
        return ((length + 1) * 4) as u32;
    }

    let mut b: u32;
    if length > 32 {
        b = length / 32;
        offset += ((b * 32 * bits) / 8) as usize;
        length %= 32;
    }
    if length > 16 {
        b = length / 16;
        offset += ((b * 16 * bits) / 8) as usize;
        length %= 16;
    }
    if length > 8 {
        b = length / 8;
        offset += ((b * 8 * bits) / 8) as usize;
        length %= 8;
    }

    let mut start = length * bits;
    offset += (start / 8) as usize;
    start %= 8;

    // |offset| now points to the byte where the new value will be stored
    // |start| is the bit position where the compressed value starts
    let v = value.wrapping_sub(base);

    if start + bits < 32 {
        // Single word
        let mut buf = [0u8; 4];
        let avail = (initin_len - offset).min(4);
        buf[..avail].copy_from_slice(&input[offset..offset + avail]);
        let mut word = u32::from_le_bytes(buf);
        let mask = (1u32 << bits) - 1;
        word &= !(mask << start);
        word |= v << start;
        let bytes = word.to_le_bytes();
        // Only write up to avail bytes
        let avail_after = (initin_len - offset).min(4);
        input[offset..offset + avail_after].copy_from_slice(&bytes[..avail_after]);
    } else {
        // Two words
        let mut buf0 = [0u8; 4];
        let avail0 = (initin_len - offset).min(4);
        buf0[..avail0].copy_from_slice(&input[offset..offset + avail0]);
        let mut word0 = u32::from_le_bytes(buf0);

        let mut buf1 = [0u8; 4];
        let off1 = offset + 4;
        let avail1 = if off1 < initin_len {
            (initin_len - off1).min(4)
        } else {
            0
        };
        if avail1 > 0 {
            buf1[..avail1].copy_from_slice(&input[off1..off1 + avail1]);
        }
        let mut word1 = u32::from_le_bytes(buf1);

        let mask1 = if bits == 32 { 0xFFFFFFFFu32 } else { (1u32 << bits) - 1 };
        let mask2_shift = bits - (32 - start);
        let mask2 = if mask2_shift >= 32 {
            0xFFFFFFFFu32
        } else {
            (1u32 << mask2_shift) - 1
        };

        word0 &= !(mask1.wrapping_shl(start));
        word0 |= (v & mask1).wrapping_shl(start);
        word1 &= !mask2;
        word1 |= v.wrapping_shr(32 - start);

        let bytes0 = word0.to_le_bytes();
        input[offset..offset + avail0].copy_from_slice(&bytes0[..avail0]);
        if avail1 > 0 {
            let bytes1 = word1.to_le_bytes();
            input[off1..off1 + avail1].copy_from_slice(&bytes1[..avail1]);
        }
    }

    (offset as u32) + ((start + bits) + 7) / 8
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(input: &mut [u8], length: u32, value: u32, appendImpl: AppendImpl) -> u32 {
    if length == 0 {
        let val_arr = [value];
        return appendImpl(&val_arr, input, 1);
    }

    let mut m_buf = [0u8; 4];
    m_buf.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(m_buf);
    let b = input[4] as u32;

    let bnew = required_bits(value.wrapping_sub(m));
    if m > value || bnew > b {
        // Re-encode the whole sequence using a heap allocation.
        let mut tmp = vec![0u32; (length + 1) as usize];
        for_uncompress(input, &mut tmp, length);
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
    let mut index = index;
    let mut offset: usize = 0;

    if bits == 32 {
        let pos = (index as usize) * 4;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&input[pos..pos + 4]);
        return base.wrapping_add(u32::from_le_bytes(buf));
    }

    let mut b: u32;
    if index > 32 {
        b = index / 32;
        offset += ((b * 32 * bits) / 8) as usize;
        index %= 32;
    }
    if index > 16 {
        b = index / 16;
        offset += ((b * 16 * bits) / 8) as usize;
        index %= 16;
    }
    if index > 8 {
        b = index / 8;
        offset += ((b * 8 * bits) / 8) as usize;
        index %= 8;
    }

    let mut start = index * bits;
    offset += (start / 8) as usize;
    start %= 8;

    let avail0 = (input.len() - offset).min(4);
    let mut buf0 = [0u8; 4];
    buf0[..avail0].copy_from_slice(&input[offset..offset + avail0]);
    let word0 = u32::from_le_bytes(buf0);

    if start + bits < 32 {
        if bits == 0 {
            return base;
        }
        let mask = (1u32 << bits) - 1;
        return base.wrapping_add((word0 >> start) & mask);
    }

    // Two-word case
    let off1 = offset + 4;
    let avail1 = if off1 < input.len() {
        (input.len() - off1).min(4)
    } else {
        0
    };
    let mut buf1 = [0u8; 4];
    if avail1 > 0 {
        buf1[..avail1].copy_from_slice(&input[off1..off1 + avail1]);
    }
    let word1 = u32::from_le_bytes(buf1);

    let mask1 = if bits == 32 { 0xFFFFFFFFu32 } else { (1u32 << bits) - 1 };
    let mask2_shift = bits - (32 - start);
    let mask2 = if mask2_shift >= 32 {
        0xFFFFFFFFu32
    } else {
        (1u32 << mask2_shift) - 1
    };

    let v1 = (word0 >> start) & mask1;
    let v2 = word1 & mask2;
    base.wrapping_add((v2 << (32 - start)) | v1)
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let mut m_buf = [0u8; 4];
    m_buf.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(m_buf);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let mut m_buf = [0u8; 4];
    m_buf.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(m_buf);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA as usize..], length, m, b, value)
}

pub fn for_linear_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32) -> u32 {
    let mut i: u32 = 0;
    let mut found: i32 = -1;
    let mut consumed: u32 = 0;

    if bits == 0 {
        return if value == base { 0 } else { length };
    }

    while i + 32 <= length {
        let c = linsearch_block_32(bits, base, &input[consumed as usize..], value, &mut found);
        if found >= 0 {
            return i + found as u32;
        }
        consumed += c;
        i += 32;
    }
    while i + 16 <= length {
        let c = linsearch_block_16(bits, base, &input[consumed as usize..], value, &mut found);
        if found >= 0 {
            return i + found as u32;
        }
        consumed += c;
        i += 16;
    }
    while i + 8 <= length {
        let c = linsearch_block_8(bits, base, &input[consumed as usize..], value, &mut found);
        if found >= 0 {
            return i + found as u32;
        }
        consumed += c;
        i += 8;
    }
    let _ = linsearch_block_x(bits, base, &input[consumed as usize..], length - i, value, &mut found);
    if found >= 0 {
        return i + found as u32;
    }
    length
}

pub fn for_lower_bound_search(input: &[u8], length: u32, value: u32, actual: &mut u32) -> u32 {
    let mut m_buf = [0u8; 4];
    m_buf.copy_from_slice(&input[0..4]);
    let m = u32::from_le_bytes(m_buf);
    let b = input[4] as u32;
    for_lower_bound_search_bits(&input[METADATA as usize..], length, m, b, value, actual)
}

pub fn for_lower_bound_search_bits(input: &[u8], length: u32, base: u32, bits: u32, value: u32, actual: &mut u32) -> u32 {
    let mut imin: u32 = 0;
    let mut imax: u32 = length - 1;

    while imin + 1 < imax {
        let imid = imin + ((imax - imin) / 2);
        let v = for_select_bits(input, base, bits, imid);
        if v >= value {
            imax = imid;
        } else if v < value {
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
