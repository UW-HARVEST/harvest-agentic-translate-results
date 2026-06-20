fn block_size(length: u32, bits: u32) -> u32 {
    if bits == 32 {
        length * 4
    } else {
        ((length * bits) + 7) / 8
    }
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

pub(crate) fn pack_block(base: u32, input: &[u32], output: &mut [u8], bits: u32) -> u32 {
    assert!(bits <= 32);

    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        for (index, &value) in input.iter().enumerate() {
            output[index * 4..index * 4 + 4].copy_from_slice(&(value - base).to_le_bytes());
        }
        return input.len() as u32 * 4;
    }

    let written = block_size(input.len() as u32, bits) as usize;
    output[..written].fill(0);
    for (index, &value) in input.iter().enumerate() {
        let delta = value - base;
        for shift in 0..bits as usize {
            set_bit(output, index * bits as usize + shift, ((delta >> shift) & 1) != 0);
        }
    }
    written as u32
}

pub(crate) fn unpack_block(base: u32, input: &[u8], output: &mut [u32], bits: u32) -> u32 {
    assert!(bits <= 32);

    if bits == 0 {
        output.fill(base);
        return 0;
    }
    if bits == 32 {
        for (index, slot) in output.iter_mut().enumerate() {
            let start = index * 4;
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&input[start..start + 4]);
            *slot = base + u32::from_le_bytes(bytes);
        }
        return output.len() as u32 * 4;
    }

    for (index, slot) in output.iter_mut().enumerate() {
        *slot = base + read_bits(input, index * bits as usize, bits);
    }
    block_size(output.len() as u32, bits)
}

pub(crate) fn linsearch_block(
    base: u32,
    input: &[u8],
    length: u32,
    value: u32,
    found: &mut i32,
    bits: u32,
) -> u32 {
    *found = -1;

    if bits == 0 {
        if length > 0 && value == base {
            *found = 0;
        }
        return 0;
    }

    if bits == 32 {
        for index in 0..length as usize {
            let start = index * 4;
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&input[start..start + 4]);
            if base + u32::from_le_bytes(bytes) == value {
                *found = index as i32;
                break;
            }
        }
        return length * 4;
    }

    for index in 0..length as usize {
        if base + read_bits(input, index * bits as usize, bits) == value {
            *found = index as i32;
            break;
        }
    }
    block_size(length, bits)
}

macro_rules! define_fixed_wrappers {
    ($len:literal, [$(($bits:literal, $pack:ident, $unpack:ident, $search:ident)),+ $(,)?]) => {
        $(
            pub fn $pack(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
                pack_block(base, &input[..$len], output, $bits)
            }

            pub fn $unpack(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
                unpack_block(base, input, &mut output[..$len], $bits)
            }

            pub fn $search(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
                linsearch_block(base, input, $len as u32, value, found, $bits)
            }
        )+
    };
}

macro_rules! define_var_wrappers {
    ($(($bits:literal, $pack:ident, $unpack:ident, $search:ident)),+ $(,)?) => {
        $(
            pub fn $pack(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
                pack_block(base, &input[..length as usize], output, $bits)
            }

            pub fn $unpack(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
                unpack_block(base, input, &mut output[..length as usize], $bits)
            }

            pub fn $search(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
                linsearch_block(base, input, length, value, found, $bits)
            }
        )+
    };
}

define_fixed_wrappers!(32, [
    (0, pack0_32, unpack0_32, linsearch0_32),
    (1, pack1_32, unpack1_32, linsearch1_32),
    (2, pack2_32, unpack2_32, linsearch2_32),
    (3, pack3_32, unpack3_32, linsearch3_32),
    (4, pack4_32, unpack4_32, linsearch4_32),
    (5, pack5_32, unpack5_32, linsearch5_32),
    (6, pack6_32, unpack6_32, linsearch6_32),
    (7, pack7_32, unpack7_32, linsearch7_32),
    (8, pack8_32, unpack8_32, linsearch8_32),
    (9, pack9_32, unpack9_32, linsearch9_32),
    (10, pack10_32, unpack10_32, linsearch10_32),
    (11, pack11_32, unpack11_32, linsearch11_32),
    (12, pack12_32, unpack12_32, linsearch12_32),
    (13, pack13_32, unpack13_32, linsearch13_32),
    (14, pack14_32, unpack14_32, linsearch14_32),
    (15, pack15_32, unpack15_32, linsearch15_32),
    (16, pack16_32, unpack16_32, linsearch16_32),
    (17, pack17_32, unpack17_32, linsearch17_32),
    (18, pack18_32, unpack18_32, linsearch18_32),
    (19, pack19_32, unpack19_32, linsearch19_32),
    (20, pack20_32, unpack20_32, linsearch20_32),
    (21, pack21_32, unpack21_32, linsearch21_32),
    (22, pack22_32, unpack22_32, linsearch22_32),
    (23, pack23_32, unpack23_32, linsearch23_32),
    (24, pack24_32, unpack24_32, linsearch24_32),
    (25, pack25_32, unpack25_32, linsearch25_32),
    (26, pack26_32, unpack26_32, linsearch26_32),
    (27, pack27_32, unpack27_32, linsearch27_32),
    (28, pack28_32, unpack28_32, linsearch28_32),
    (29, pack29_32, unpack29_32, linsearch29_32),
    (30, pack30_32, unpack30_32, linsearch30_32),
    (31, pack31_32, unpack31_32, linsearch31_32),
    (32, pack32_32, unpack32_32, linsearch32_32),
]);

define_fixed_wrappers!(16, [
    (0, pack0_16, unpack0_16, linsearch0_16),
    (1, pack1_16, unpack1_16, linsearch1_16),
    (2, pack2_16, unpack2_16, linsearch2_16),
    (3, pack3_16, unpack3_16, linsearch3_16),
    (4, pack4_16, unpack4_16, linsearch4_16),
    (5, pack5_16, unpack5_16, linsearch5_16),
    (6, pack6_16, unpack6_16, linsearch6_16),
    (7, pack7_16, unpack7_16, linsearch7_16),
    (8, pack8_16, unpack8_16, linsearch8_16),
    (9, pack9_16, unpack9_16, linsearch9_16),
    (10, pack10_16, unpack10_16, linsearch10_16),
    (11, pack11_16, unpack11_16, linsearch11_16),
    (12, pack12_16, unpack12_16, linsearch12_16),
    (13, pack13_16, unpack13_16, linsearch13_16),
    (14, pack14_16, unpack14_16, linsearch14_16),
    (15, pack15_16, unpack15_16, linsearch15_16),
    (16, pack16_16, unpack16_16, linsearch16_16),
    (17, pack17_16, unpack17_16, linsearch17_16),
    (18, pack18_16, unpack18_16, linsearch18_16),
    (19, pack19_16, unpack19_16, linsearch19_16),
    (20, pack20_16, unpack20_16, linsearch20_16),
    (21, pack21_16, unpack21_16, linsearch21_16),
    (22, pack22_16, unpack22_16, linsearch22_16),
    (23, pack23_16, unpack23_16, linsearch23_16),
    (24, pack24_16, unpack24_16, linsearch24_16),
    (25, pack25_16, unpack25_16, linsearch25_16),
    (26, pack26_16, unpack26_16, linsearch26_16),
    (27, pack27_16, unpack27_16, linsearch27_16),
    (28, pack28_16, unpack28_16, linsearch28_16),
    (29, pack29_16, unpack29_16, linsearch29_16),
    (30, pack30_16, unpack30_16, linsearch30_16),
    (31, pack31_16, unpack31_16, linsearch31_16),
    (32, pack32_16, unpack32_16, linsearch32_16),
]);

define_fixed_wrappers!(8, [
    (0, pack0_8, unpack0_8, linsearch0_8),
    (1, pack1_8, unpack1_8, linsearch1_8),
    (2, pack2_8, unpack2_8, linsearch2_8),
    (3, pack3_8, unpack3_8, linsearch3_8),
    (4, pack4_8, unpack4_8, linsearch4_8),
    (5, pack5_8, unpack5_8, linsearch5_8),
    (6, pack6_8, unpack6_8, linsearch6_8),
    (7, pack7_8, unpack7_8, linsearch7_8),
    (8, pack8_8, unpack8_8, linsearch8_8),
    (9, pack9_8, unpack9_8, linsearch9_8),
    (10, pack10_8, unpack10_8, linsearch10_8),
    (11, pack11_8, unpack11_8, linsearch11_8),
    (12, pack12_8, unpack12_8, linsearch12_8),
    (13, pack13_8, unpack13_8, linsearch13_8),
    (14, pack14_8, unpack14_8, linsearch14_8),
    (15, pack15_8, unpack15_8, linsearch15_8),
    (16, pack16_8, unpack16_8, linsearch16_8),
    (17, pack17_8, unpack17_8, linsearch17_8),
    (18, pack18_8, unpack18_8, linsearch18_8),
    (19, pack19_8, unpack19_8, linsearch19_8),
    (20, pack20_8, unpack20_8, linsearch20_8),
    (21, pack21_8, unpack21_8, linsearch21_8),
    (22, pack22_8, unpack22_8, linsearch22_8),
    (23, pack23_8, unpack23_8, linsearch23_8),
    (24, pack24_8, unpack24_8, linsearch24_8),
    (25, pack25_8, unpack25_8, linsearch25_8),
    (26, pack26_8, unpack26_8, linsearch26_8),
    (27, pack27_8, unpack27_8, linsearch27_8),
    (28, pack28_8, unpack28_8, linsearch28_8),
    (29, pack29_8, unpack29_8, linsearch29_8),
    (30, pack30_8, unpack30_8, linsearch30_8),
    (31, pack31_8, unpack31_8, linsearch31_8),
    (32, pack32_8, unpack32_8, linsearch32_8),
]);

define_var_wrappers!(
    (0, pack0_x, unpack0_x, linsearch0_x),
    (1, pack1_x, unpack1_x, linsearch1_x),
    (2, pack2_x, unpack2_x, linsearch2_x),
    (3, pack3_x, unpack3_x, linsearch3_x),
    (4, pack4_x, unpack4_x, linsearch4_x),
    (5, pack5_x, unpack5_x, linsearch5_x),
    (6, pack6_x, unpack6_x, linsearch6_x),
    (7, pack7_x, unpack7_x, linsearch7_x),
    (8, pack8_x, unpack8_x, linsearch8_x),
    (9, pack9_x, unpack9_x, linsearch9_x),
    (10, pack10_x, unpack10_x, linsearch10_x),
    (11, pack11_x, unpack11_x, linsearch11_x),
    (12, pack12_x, unpack12_x, linsearch12_x),
    (13, pack13_x, unpack13_x, linsearch13_x),
    (14, pack14_x, unpack14_x, linsearch14_x),
    (15, pack15_x, unpack15_x, linsearch15_x),
    (16, pack16_x, unpack16_x, linsearch16_x),
    (17, pack17_x, unpack17_x, linsearch17_x),
    (18, pack18_x, unpack18_x, linsearch18_x),
    (19, pack19_x, unpack19_x, linsearch19_x),
    (20, pack20_x, unpack20_x, linsearch20_x),
    (21, pack21_x, unpack21_x, linsearch21_x),
    (22, pack22_x, unpack22_x, linsearch22_x),
    (23, pack23_x, unpack23_x, linsearch23_x),
    (24, pack24_x, unpack24_x, linsearch24_x),
    (25, pack25_x, unpack25_x, linsearch25_x),
    (26, pack26_x, unpack26_x, linsearch26_x),
    (27, pack27_x, unpack27_x, linsearch27_x),
    (28, pack28_x, unpack28_x, linsearch28_x),
    (29, pack29_x, unpack29_x, linsearch29_x),
    (30, pack30_x, unpack30_x, linsearch30_x),
    (31, pack31_x, unpack31_x, linsearch31_x),
    (32, pack32_x, unpack32_x, linsearch32_x),
);
