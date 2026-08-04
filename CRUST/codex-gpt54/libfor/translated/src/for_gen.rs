use crate::forLib;

fn pack_fixed(base: u32, input: &[u32], output: &mut [u8], length: u32, bits: u32) -> u32 {
    forLib::for_compress_bits(input, output, length, base, bits)
}

fn linsearch_fixed(
    base: u32,
    input: &[u8],
    length: u32,
    value: u32,
    bits: u32,
    found: &mut i32,
) -> u32 {
    let index = forLib::for_linear_search_bits(input, length, base, bits, value);
    if index < length {
        *found = index as i32;
    }
    forLib::for_compressed_size_bits(length, bits)
}

fn u32_words_to_le_bytes(input: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(input.len() * 4);
    for &word in input {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn write_u32s_as_le_bytes(values: &[u32], output: &mut [u8]) {
    for (chunk, value) in output.chunks_exact_mut(4).zip(values.iter()) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
}

fn unpack_from_u32_words(
    base: u32,
    input: &[u32],
    output: &mut [u8],
    length: u32,
    bits: u32,
) -> u32 {
    let raw = u32_words_to_le_bytes(input);
    let mut values = vec![0u32; length as usize];
    let consumed = forLib::for_uncompress_bits(&raw, &mut values, length, base, bits);
    write_u32s_as_le_bytes(&values, output);
    consumed
}

macro_rules! define_pack_fixed {
    ($(($name:ident, $bits:literal, $length:literal)),* $(,)?) => {
        $(
            pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
                pack_fixed(base, input, output, $length, $bits)
            }
        )*
    };
}

macro_rules! define_pack_variable {
    ($(($name:ident, $bits:literal)),* $(,)?) => {
        $(
            pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
                pack_fixed(base, input, output, length, $bits)
            }
        )*
    };
}

macro_rules! define_unpack_variable_u8 {
    ($(($name:ident, $bits:literal, $length:literal)),* $(,)?) => {
        $(
            pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
                unpack_from_u32_words(base, input, output, $length, $bits)
            }
        )*
    };
}

macro_rules! define_unpack_variable_u8_len {
    ($(($name:ident, $bits:literal)),* $(,)?) => {
        $(
            pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
                unpack_from_u32_words(base, input, output, length, $bits)
            }
        )*
    };
}

macro_rules! define_linsearch_fixed {
    ($(($name:ident, $bits:literal, $length:literal)),* $(,)?) => {
        $(
            pub fn $name(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
                linsearch_fixed(base, input, $length, value, $bits, found)
            }
        )*
    };
}

macro_rules! define_linsearch_variable {
    ($(($name:ident, $bits:literal)),* $(,)?) => {
        $(
            pub fn $name(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
                linsearch_fixed(base, input, length, value, $bits, found)
            }
        )*
    };
}

pub fn pack0_32(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 {
    0
}

pub fn pack0_16(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 {
    0
}

pub fn pack0_8(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 {
    0
}

pub fn unpack0_32(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    output[..32].fill(base);
    0
}

pub fn unpack0_16(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    output[..16].fill(base);
    0
}

pub fn unpack0_8(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    output[..8].fill(base);
    0
}

pub fn pack0_x(_base: u32, _input: &[u32], _output: &mut [u8], _length: u32) -> u32 {
    0
}

pub fn unpack0_x(base: u32, _input: &[u8], output: &mut [u32], length: u32) -> u32 {
    output[..length as usize].fill(base);
    0
}

pub fn linsearch0_32(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}

pub fn linsearch0_16(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}

pub fn linsearch0_8(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}

pub fn linsearch0_x(base: u32, _input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length > 0 && base == value {
        *found = 0;
    }
    0
}

define_pack_fixed!(
    (pack1_32, 1, 32),
    (pack2_32, 2, 32),
    (pack3_32, 3, 32),
    (pack4_32, 4, 32),
    (pack5_32, 5, 32),
    (pack6_32, 6, 32),
    (pack7_32, 7, 32),
    (pack8_32, 8, 32),
    (pack9_32, 9, 32),
    (pack10_32, 10, 32),
    (pack11_32, 11, 32),
    (pack12_32, 12, 32),
    (pack13_32, 13, 32),
    (pack14_32, 14, 32),
    (pack15_32, 15, 32),
    (pack16_32, 16, 32),
    (pack17_32, 17, 32),
    (pack18_32, 18, 32),
    (pack19_32, 19, 32),
    (pack20_32, 20, 32),
    (pack21_32, 21, 32),
    (pack22_32, 22, 32),
    (pack23_32, 23, 32),
    (pack24_32, 24, 32),
    (pack25_32, 25, 32),
    (pack26_32, 26, 32),
    (pack27_32, 27, 32),
    (pack28_32, 28, 32),
    (pack29_32, 29, 32),
    (pack30_32, 30, 32),
    (pack31_32, 31, 32),
    (pack32_32, 32, 32),
    (pack1_16, 1, 16),
    (pack2_16, 2, 16),
    (pack3_16, 3, 16),
    (pack4_16, 4, 16),
    (pack5_16, 5, 16),
    (pack6_16, 6, 16),
    (pack7_16, 7, 16),
    (pack8_16, 8, 16),
    (pack9_16, 9, 16),
    (pack10_16, 10, 16),
    (pack11_16, 11, 16),
    (pack12_16, 12, 16),
    (pack13_16, 13, 16),
    (pack14_16, 14, 16),
    (pack15_16, 15, 16),
    (pack16_16, 16, 16),
    (pack17_16, 17, 16),
    (pack18_16, 18, 16),
    (pack19_16, 19, 16),
    (pack20_16, 20, 16),
    (pack21_16, 21, 16),
    (pack22_16, 22, 16),
    (pack23_16, 23, 16),
    (pack24_16, 24, 16),
    (pack25_16, 25, 16),
    (pack26_16, 26, 16),
    (pack27_16, 27, 16),
    (pack28_16, 28, 16),
    (pack29_16, 29, 16),
    (pack30_16, 30, 16),
    (pack31_16, 31, 16),
    (pack32_16, 32, 16),
    (pack1_8, 1, 8),
    (pack2_8, 2, 8),
    (pack3_8, 3, 8),
    (pack4_8, 4, 8),
    (pack5_8, 5, 8),
    (pack6_8, 6, 8),
    (pack7_8, 7, 8),
    (pack8_8, 8, 8),
    (pack9_8, 9, 8),
    (pack10_8, 10, 8),
    (pack11_8, 11, 8),
    (pack12_8, 12, 8),
    (pack13_8, 13, 8),
    (pack14_8, 14, 8),
    (pack15_8, 15, 8),
    (pack16_8, 16, 8),
    (pack17_8, 17, 8),
    (pack18_8, 18, 8),
    (pack19_8, 19, 8),
    (pack20_8, 20, 8),
    (pack21_8, 21, 8),
    (pack22_8, 22, 8),
    (pack23_8, 23, 8),
    (pack24_8, 24, 8),
    (pack25_8, 25, 8),
    (pack26_8, 26, 8),
    (pack27_8, 27, 8),
    (pack28_8, 28, 8),
    (pack29_8, 29, 8),
    (pack30_8, 30, 8),
    (pack31_8, 31, 8),
    (pack32_8, 32, 8),
);

define_pack_variable!(
    (pack1_x, 1),
    (pack2_x, 2),
    (pack3_x, 3),
    (pack4_x, 4),
    (pack5_x, 5),
    (pack6_x, 6),
    (pack7_x, 7),
    (pack8_x, 8),
    (pack9_x, 9),
    (pack10_x, 10),
    (pack11_x, 11),
    (pack12_x, 12),
    (pack13_x, 13),
    (pack14_x, 14),
    (pack15_x, 15),
    (pack16_x, 16),
    (pack17_x, 17),
    (pack18_x, 18),
    (pack19_x, 19),
    (pack20_x, 20),
    (pack21_x, 21),
    (pack22_x, 22),
    (pack23_x, 23),
    (pack24_x, 24),
    (pack25_x, 25),
    (pack26_x, 26),
    (pack27_x, 27),
    (pack28_x, 28),
    (pack29_x, 29),
    (pack30_x, 30),
    (pack31_x, 31),
    (pack32_x, 32),
);

define_unpack_variable_u8!(
    (unpack1_8, 1, 8),
    (unpack2_8, 2, 8),
    (unpack3_8, 3, 8),
    (unpack4_8, 4, 8),
    (unpack5_8, 5, 8),
    (unpack6_8, 6, 8),
    (unpack7_8, 7, 8),
    (unpack8_8, 8, 8),
    (unpack9_8, 9, 8),
    (unpack10_8, 10, 8),
    (unpack11_8, 11, 8),
    (unpack12_8, 12, 8),
    (unpack13_8, 13, 8),
    (unpack14_8, 14, 8),
    (unpack15_8, 15, 8),
    (unpack16_8, 16, 8),
    (unpack17_8, 17, 8),
    (unpack18_8, 18, 8),
    (unpack19_8, 19, 8),
    (unpack20_8, 20, 8),
    (unpack21_8, 21, 8),
    (unpack22_8, 22, 8),
    (unpack23_8, 23, 8),
    (unpack24_8, 24, 8),
    (unpack25_8, 25, 8),
    (unpack26_8, 26, 8),
    (unpack27_8, 27, 8),
    (unpack28_8, 28, 8),
    (unpack29_8, 29, 8),
    (unpack30_8, 30, 8),
    (unpack31_8, 31, 8),
    (unpack32_8, 32, 8),
    (unpack1_16, 1, 16),
    (unpack2_16, 2, 16),
    (unpack3_16, 3, 16),
    (unpack4_16, 4, 16),
    (unpack5_16, 5, 16),
    (unpack6_16, 6, 16),
    (unpack7_16, 7, 16),
    (unpack8_16, 8, 16),
    (unpack9_16, 9, 16),
    (unpack10_16, 10, 16),
    (unpack11_16, 11, 16),
    (unpack12_16, 12, 16),
    (unpack13_16, 13, 16),
    (unpack14_16, 14, 16),
    (unpack15_16, 15, 16),
    (unpack16_16, 16, 16),
    (unpack17_16, 17, 16),
    (unpack18_16, 18, 16),
    (unpack19_16, 19, 16),
    (unpack20_16, 20, 16),
    (unpack21_16, 21, 16),
    (unpack22_16, 22, 16),
    (unpack23_16, 23, 16),
    (unpack24_16, 24, 16),
    (unpack25_16, 25, 16),
    (unpack26_16, 26, 16),
    (unpack27_16, 27, 16),
    (unpack28_16, 28, 16),
    (unpack29_16, 29, 16),
    (unpack30_16, 30, 16),
    (unpack31_16, 31, 16),
    (unpack32_16, 32, 16),
    (unpack1_32, 1, 32),
    (unpack2_32, 2, 32),
    (unpack3_32, 3, 32),
    (unpack4_32, 4, 32),
    (unpack5_32, 5, 32),
    (unpack6_32, 6, 32),
    (unpack7_32, 7, 32),
    (unpack8_32, 8, 32),
    (unpack9_32, 9, 32),
    (unpack10_32, 10, 32),
    (unpack11_32, 11, 32),
    (unpack12_32, 12, 32),
    (unpack13_32, 13, 32),
    (unpack14_32, 14, 32),
    (unpack15_32, 15, 32),
    (unpack16_32, 16, 32),
    (unpack17_32, 17, 32),
    (unpack18_32, 18, 32),
    (unpack19_32, 19, 32),
    (unpack20_32, 20, 32),
    (unpack21_32, 21, 32),
    (unpack22_32, 22, 32),
    (unpack23_32, 23, 32),
    (unpack24_32, 24, 32),
    (unpack25_32, 25, 32),
    (unpack26_32, 26, 32),
    (unpack27_32, 27, 32),
    (unpack28_32, 28, 32),
    (unpack29_32, 29, 32),
    (unpack30_32, 30, 32),
    (unpack31_32, 31, 32),
    (unpack32_32, 32, 32),
);

define_unpack_variable_u8_len!(
    (unpack1_x, 1),
    (unpack2_x, 2),
    (unpack3_x, 3),
    (unpack4_x, 4),
    (unpack5_x, 5),
    (unpack6_x, 6),
    (unpack7_x, 7),
    (unpack8_x, 8),
    (unpack9_x, 9),
    (unpack10_x, 10),
    (unpack11_x, 11),
    (unpack12_x, 12),
    (unpack13_x, 13),
    (unpack14_x, 14),
    (unpack15_x, 15),
    (unpack16_x, 16),
    (unpack17_x, 17),
    (unpack18_x, 18),
    (unpack19_x, 19),
    (unpack20_x, 20),
    (unpack21_x, 21),
    (unpack22_x, 22),
    (unpack23_x, 23),
    (unpack24_x, 24),
    (unpack25_x, 25),
    (unpack26_x, 26),
    (unpack27_x, 27),
    (unpack28_x, 28),
    (unpack29_x, 29),
    (unpack30_x, 30),
    (unpack31_x, 31),
    (unpack32_x, 32),
);

define_linsearch_fixed!(
    (linsearch1_32, 1, 32),
    (linsearch2_32, 2, 32),
    (linsearch3_32, 3, 32),
    (linsearch4_32, 4, 32),
    (linsearch5_32, 5, 32),
    (linsearch6_32, 6, 32),
    (linsearch7_32, 7, 32),
    (linsearch8_32, 8, 32),
    (linsearch9_32, 9, 32),
    (linsearch10_32, 10, 32),
    (linsearch11_32, 11, 32),
    (linsearch12_32, 12, 32),
    (linsearch13_32, 13, 32),
    (linsearch14_32, 14, 32),
    (linsearch15_32, 15, 32),
    (linsearch16_32, 16, 32),
    (linsearch17_32, 17, 32),
    (linsearch18_32, 18, 32),
    (linsearch19_32, 19, 32),
    (linsearch20_32, 20, 32),
    (linsearch21_32, 21, 32),
    (linsearch22_32, 22, 32),
    (linsearch23_32, 23, 32),
    (linsearch24_32, 24, 32),
    (linsearch25_32, 25, 32),
    (linsearch26_32, 26, 32),
    (linsearch27_32, 27, 32),
    (linsearch28_32, 28, 32),
    (linsearch29_32, 29, 32),
    (linsearch30_32, 30, 32),
    (linsearch31_32, 31, 32),
    (linsearch32_32, 32, 32),
    (linsearch1_16, 1, 16),
    (linsearch2_16, 2, 16),
    (linsearch3_16, 3, 16),
    (linsearch4_16, 4, 16),
    (linsearch5_16, 5, 16),
    (linsearch6_16, 6, 16),
    (linsearch7_16, 7, 16),
    (linsearch8_16, 8, 16),
    (linsearch9_16, 9, 16),
    (linsearch10_16, 10, 16),
    (linsearch11_16, 11, 16),
    (linsearch12_16, 12, 16),
    (linsearch13_16, 13, 16),
    (linsearch14_16, 14, 16),
    (linsearch15_16, 15, 16),
    (linsearch16_16, 16, 16),
    (linsearch17_16, 17, 16),
    (linsearch18_16, 18, 16),
    (linsearch19_16, 19, 16),
    (linsearch20_16, 20, 16),
    (linsearch21_16, 21, 16),
    (linsearch22_16, 22, 16),
    (linsearch23_16, 23, 16),
    (linsearch24_16, 24, 16),
    (linsearch25_16, 25, 16),
    (linsearch26_16, 26, 16),
    (linsearch27_16, 27, 16),
    (linsearch28_16, 28, 16),
    (linsearch29_16, 29, 16),
    (linsearch30_16, 30, 16),
    (linsearch31_16, 31, 16),
    (linsearch32_16, 32, 16),
    (linsearch1_8, 1, 8),
    (linsearch2_8, 2, 8),
    (linsearch3_8, 3, 8),
    (linsearch4_8, 4, 8),
    (linsearch5_8, 5, 8),
    (linsearch6_8, 6, 8),
    (linsearch7_8, 7, 8),
    (linsearch8_8, 8, 8),
    (linsearch9_8, 9, 8),
    (linsearch10_8, 10, 8),
    (linsearch11_8, 11, 8),
    (linsearch12_8, 12, 8),
    (linsearch13_8, 13, 8),
    (linsearch14_8, 14, 8),
    (linsearch15_8, 15, 8),
    (linsearch16_8, 16, 8),
    (linsearch17_8, 17, 8),
    (linsearch18_8, 18, 8),
    (linsearch19_8, 19, 8),
    (linsearch20_8, 20, 8),
    (linsearch21_8, 21, 8),
    (linsearch22_8, 22, 8),
    (linsearch23_8, 23, 8),
    (linsearch24_8, 24, 8),
    (linsearch25_8, 25, 8),
    (linsearch26_8, 26, 8),
    (linsearch27_8, 27, 8),
    (linsearch28_8, 28, 8),
    (linsearch29_8, 29, 8),
    (linsearch30_8, 30, 8),
    (linsearch31_8, 31, 8),
    (linsearch32_8, 32, 8),
);

define_linsearch_variable!(
    (linsearch1_x, 1),
    (linsearch2_x, 2),
    (linsearch3_x, 3),
    (linsearch4_x, 4),
    (linsearch5_x, 5),
    (linsearch6_x, 6),
    (linsearch7_x, 7),
    (linsearch8_x, 8),
    (linsearch9_x, 9),
    (linsearch10_x, 10),
    (linsearch11_x, 11),
    (linsearch12_x, 12),
    (linsearch13_x, 13),
    (linsearch14_x, 14),
    (linsearch15_x, 15),
    (linsearch16_x, 16),
    (linsearch17_x, 17),
    (linsearch18_x, 18),
    (linsearch19_x, 19),
    (linsearch20_x, 20),
    (linsearch21_x, 21),
    (linsearch22_x, 22),
    (linsearch23_x, 23),
    (linsearch24_x, 24),
    (linsearch25_x, 25),
    (linsearch26_x, 26),
    (linsearch27_x, 27),
    (linsearch28_x, 28),
    (linsearch29_x, 29),
    (linsearch30_x, 30),
    (linsearch31_x, 31),
    (linsearch32_x, 32),
);
