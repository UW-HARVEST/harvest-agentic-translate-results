use crate::for_gen;

pub const METADATA: i32 = 5;

/// Number of bits required to represent v (0 -> 0).
pub fn required_bits(v: u32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.leading_zeros()
    }
}

// ===== Dispatch helpers =====

fn call_pack(bits: u32, block: usize, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    macro_rules! disp {
        ($block:expr, $b:expr) => {{
            match $b {
                0 => match $block {
                    32 => for_gen::pack0_32(base, input, output),
                    16 => for_gen::pack0_16(base, input, output),
                    8 => for_gen::pack0_8(base, input, output),
                    _ => unreachable!(),
                },
                _ => match $block {
                    32 => match $b {
                        1 => for_gen::pack1_32(base, input, output),
                        2 => for_gen::pack2_32(base, input, output),
                        3 => for_gen::pack3_32(base, input, output),
                        4 => for_gen::pack4_32(base, input, output),
                        5 => for_gen::pack5_32(base, input, output),
                        6 => for_gen::pack6_32(base, input, output),
                        7 => for_gen::pack7_32(base, input, output),
                        8 => for_gen::pack8_32(base, input, output),
                        9 => for_gen::pack9_32(base, input, output),
                        10 => for_gen::pack10_32(base, input, output),
                        11 => for_gen::pack11_32(base, input, output),
                        12 => for_gen::pack12_32(base, input, output),
                        13 => for_gen::pack13_32(base, input, output),
                        14 => for_gen::pack14_32(base, input, output),
                        15 => for_gen::pack15_32(base, input, output),
                        16 => for_gen::pack16_32(base, input, output),
                        17 => for_gen::pack17_32(base, input, output),
                        18 => for_gen::pack18_32(base, input, output),
                        19 => for_gen::pack19_32(base, input, output),
                        20 => for_gen::pack20_32(base, input, output),
                        21 => for_gen::pack21_32(base, input, output),
                        22 => for_gen::pack22_32(base, input, output),
                        23 => for_gen::pack23_32(base, input, output),
                        24 => for_gen::pack24_32(base, input, output),
                        25 => for_gen::pack25_32(base, input, output),
                        26 => for_gen::pack26_32(base, input, output),
                        27 => for_gen::pack27_32(base, input, output),
                        28 => for_gen::pack28_32(base, input, output),
                        29 => for_gen::pack29_32(base, input, output),
                        30 => for_gen::pack30_32(base, input, output),
                        31 => for_gen::pack31_32(base, input, output),
                        32 => for_gen::pack32_32(base, input, output),
                        _ => unreachable!(),
                    },
                    16 => match $b {
                        1 => for_gen::pack1_16(base, input, output),
                        2 => for_gen::pack2_16(base, input, output),
                        3 => for_gen::pack3_16(base, input, output),
                        4 => for_gen::pack4_16(base, input, output),
                        5 => for_gen::pack5_16(base, input, output),
                        6 => for_gen::pack6_16(base, input, output),
                        7 => for_gen::pack7_16(base, input, output),
                        8 => for_gen::pack8_16(base, input, output),
                        9 => for_gen::pack9_16(base, input, output),
                        10 => for_gen::pack10_16(base, input, output),
                        11 => for_gen::pack11_16(base, input, output),
                        12 => for_gen::pack12_16(base, input, output),
                        13 => for_gen::pack13_16(base, input, output),
                        14 => for_gen::pack14_16(base, input, output),
                        15 => for_gen::pack15_16(base, input, output),
                        16 => for_gen::pack16_16(base, input, output),
                        17 => for_gen::pack17_16(base, input, output),
                        18 => for_gen::pack18_16(base, input, output),
                        19 => for_gen::pack19_16(base, input, output),
                        20 => for_gen::pack20_16(base, input, output),
                        21 => for_gen::pack21_16(base, input, output),
                        22 => for_gen::pack22_16(base, input, output),
                        23 => for_gen::pack23_16(base, input, output),
                        24 => for_gen::pack24_16(base, input, output),
                        25 => for_gen::pack25_16(base, input, output),
                        26 => for_gen::pack26_16(base, input, output),
                        27 => for_gen::pack27_16(base, input, output),
                        28 => for_gen::pack28_16(base, input, output),
                        29 => for_gen::pack29_16(base, input, output),
                        30 => for_gen::pack30_16(base, input, output),
                        31 => for_gen::pack31_16(base, input, output),
                        32 => for_gen::pack32_16(base, input, output),
                        _ => unreachable!(),
                    },
                    8 => match $b {
                        1 => for_gen::pack1_8(base, input, output),
                        2 => for_gen::pack2_8(base, input, output),
                        3 => for_gen::pack3_8(base, input, output),
                        4 => for_gen::pack4_8(base, input, output),
                        5 => for_gen::pack5_8(base, input, output),
                        6 => for_gen::pack6_8(base, input, output),
                        7 => for_gen::pack7_8(base, input, output),
                        8 => for_gen::pack8_8(base, input, output),
                        9 => for_gen::pack9_8(base, input, output),
                        10 => for_gen::pack10_8(base, input, output),
                        11 => for_gen::pack11_8(base, input, output),
                        12 => for_gen::pack12_8(base, input, output),
                        13 => for_gen::pack13_8(base, input, output),
                        14 => for_gen::pack14_8(base, input, output),
                        15 => for_gen::pack15_8(base, input, output),
                        16 => for_gen::pack16_8(base, input, output),
                        17 => for_gen::pack17_8(base, input, output),
                        18 => for_gen::pack18_8(base, input, output),
                        19 => for_gen::pack19_8(base, input, output),
                        20 => for_gen::pack20_8(base, input, output),
                        21 => for_gen::pack21_8(base, input, output),
                        22 => for_gen::pack22_8(base, input, output),
                        23 => for_gen::pack23_8(base, input, output),
                        24 => for_gen::pack24_8(base, input, output),
                        25 => for_gen::pack25_8(base, input, output),
                        26 => for_gen::pack26_8(base, input, output),
                        27 => for_gen::pack27_8(base, input, output),
                        28 => for_gen::pack28_8(base, input, output),
                        29 => for_gen::pack29_8(base, input, output),
                        30 => for_gen::pack30_8(base, input, output),
                        31 => for_gen::pack31_8(base, input, output),
                        32 => for_gen::pack32_8(base, input, output),
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                },
            }
        }};
    }
    disp!(block, bits)
}

fn call_unpack(bits: u32, block: usize, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    match (block, bits) {
        (32, 0) => for_gen::unpack0_32(base, input, output),
        (16, 0) => for_gen::unpack0_16(base, input, output),
        (8, 0) => for_gen::unpack0_8(base, input, output),
        (32, 1) => for_gen::unpack1_32(base, input, output),
        (32, 2) => for_gen::unpack2_32(base, input, output),
        (32, 3) => for_gen::unpack3_32(base, input, output),
        (32, 4) => for_gen::unpack4_32(base, input, output),
        (32, 5) => for_gen::unpack5_32(base, input, output),
        (32, 6) => for_gen::unpack6_32(base, input, output),
        (32, 7) => for_gen::unpack7_32(base, input, output),
        (32, 8) => for_gen::unpack8_32(base, input, output),
        (32, 9) => for_gen::unpack9_32(base, input, output),
        (32, 10) => for_gen::unpack10_32(base, input, output),
        (32, 11) => for_gen::unpack11_32(base, input, output),
        (32, 12) => for_gen::unpack12_32(base, input, output),
        (32, 13) => for_gen::unpack13_32(base, input, output),
        (32, 14) => for_gen::unpack14_32(base, input, output),
        (32, 15) => for_gen::unpack15_32(base, input, output),
        (32, 16) => for_gen::unpack16_32(base, input, output),
        (32, 17) => for_gen::unpack17_32(base, input, output),
        (32, 18) => for_gen::unpack18_32(base, input, output),
        (32, 19) => for_gen::unpack19_32(base, input, output),
        (32, 20) => for_gen::unpack20_32(base, input, output),
        (32, 21) => for_gen::unpack21_32(base, input, output),
        (32, 22) => for_gen::unpack22_32(base, input, output),
        (32, 23) => for_gen::unpack23_32(base, input, output),
        (32, 24) => for_gen::unpack24_32(base, input, output),
        (32, 25) => for_gen::unpack25_32(base, input, output),
        (32, 26) => for_gen::unpack26_32(base, input, output),
        (32, 27) => for_gen::unpack27_32(base, input, output),
        (32, 28) => for_gen::unpack28_32(base, input, output),
        (32, 29) => for_gen::unpack29_32(base, input, output),
        (32, 30) => for_gen::unpack30_32(base, input, output),
        (32, 31) => for_gen::unpack31_32(base, input, output),
        (32, 32) => for_gen::unpack32_32(base, input, output),
        (16, 1) => for_gen::unpack1_16(base, input, output),
        (16, 2) => for_gen::unpack2_16(base, input, output),
        (16, 3) => for_gen::unpack3_16(base, input, output),
        (16, 4) => for_gen::unpack4_16(base, input, output),
        (16, 5) => for_gen::unpack5_16(base, input, output),
        (16, 6) => for_gen::unpack6_16(base, input, output),
        (16, 7) => for_gen::unpack7_16(base, input, output),
        (16, 8) => for_gen::unpack8_16(base, input, output),
        (16, 9) => for_gen::unpack9_16(base, input, output),
        (16, 10) => for_gen::unpack10_16(base, input, output),
        (16, 11) => for_gen::unpack11_16(base, input, output),
        (16, 12) => for_gen::unpack12_16(base, input, output),
        (16, 13) => for_gen::unpack13_16(base, input, output),
        (16, 14) => for_gen::unpack14_16(base, input, output),
        (16, 15) => for_gen::unpack15_16(base, input, output),
        (16, 16) => for_gen::unpack16_16(base, input, output),
        (16, 17) => for_gen::unpack17_16(base, input, output),
        (16, 18) => for_gen::unpack18_16(base, input, output),
        (16, 19) => for_gen::unpack19_16(base, input, output),
        (16, 20) => for_gen::unpack20_16(base, input, output),
        (16, 21) => for_gen::unpack21_16(base, input, output),
        (16, 22) => for_gen::unpack22_16(base, input, output),
        (16, 23) => for_gen::unpack23_16(base, input, output),
        (16, 24) => for_gen::unpack24_16(base, input, output),
        (16, 25) => for_gen::unpack25_16(base, input, output),
        (16, 26) => for_gen::unpack26_16(base, input, output),
        (16, 27) => for_gen::unpack27_16(base, input, output),
        (16, 28) => for_gen::unpack28_16(base, input, output),
        (16, 29) => for_gen::unpack29_16(base, input, output),
        (16, 30) => for_gen::unpack30_16(base, input, output),
        (16, 31) => for_gen::unpack31_16(base, input, output),
        (16, 32) => for_gen::unpack32_16(base, input, output),
        (8, 1) => for_gen::unpack1_8(base, input, output),
        (8, 2) => for_gen::unpack2_8(base, input, output),
        (8, 3) => for_gen::unpack3_8(base, input, output),
        (8, 4) => for_gen::unpack4_8(base, input, output),
        (8, 5) => for_gen::unpack5_8(base, input, output),
        (8, 6) => for_gen::unpack6_8(base, input, output),
        (8, 7) => for_gen::unpack7_8(base, input, output),
        (8, 8) => for_gen::unpack8_8(base, input, output),
        (8, 9) => for_gen::unpack9_8(base, input, output),
        (8, 10) => for_gen::unpack10_8(base, input, output),
        (8, 11) => for_gen::unpack11_8(base, input, output),
        (8, 12) => for_gen::unpack12_8(base, input, output),
        (8, 13) => for_gen::unpack13_8(base, input, output),
        (8, 14) => for_gen::unpack14_8(base, input, output),
        (8, 15) => for_gen::unpack15_8(base, input, output),
        (8, 16) => for_gen::unpack16_8(base, input, output),
        (8, 17) => for_gen::unpack17_8(base, input, output),
        (8, 18) => for_gen::unpack18_8(base, input, output),
        (8, 19) => for_gen::unpack19_8(base, input, output),
        (8, 20) => for_gen::unpack20_8(base, input, output),
        (8, 21) => for_gen::unpack21_8(base, input, output),
        (8, 22) => for_gen::unpack22_8(base, input, output),
        (8, 23) => for_gen::unpack23_8(base, input, output),
        (8, 24) => for_gen::unpack24_8(base, input, output),
        (8, 25) => for_gen::unpack25_8(base, input, output),
        (8, 26) => for_gen::unpack26_8(base, input, output),
        (8, 27) => for_gen::unpack27_8(base, input, output),
        (8, 28) => for_gen::unpack28_8(base, input, output),
        (8, 29) => for_gen::unpack29_8(base, input, output),
        (8, 30) => for_gen::unpack30_8(base, input, output),
        (8, 31) => for_gen::unpack31_8(base, input, output),
        (8, 32) => for_gen::unpack32_8(base, input, output),
        _ => unreachable!(),
    }
}

fn call_pack_x(bits: u32, base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    match bits {
        0 => for_gen::pack0_x(base, input, output, length),
        1 => for_gen::pack1_x(base, input, output, length),
        2 => for_gen::pack2_x(base, input, output, length),
        3 => for_gen::pack3_x(base, input, output, length),
        4 => for_gen::pack4_x(base, input, output, length),
        5 => for_gen::pack5_x(base, input, output, length),
        6 => for_gen::pack6_x(base, input, output, length),
        7 => for_gen::pack7_x(base, input, output, length),
        8 => for_gen::pack8_x(base, input, output, length),
        9 => for_gen::pack9_x(base, input, output, length),
        10 => for_gen::pack10_x(base, input, output, length),
        11 => for_gen::pack11_x(base, input, output, length),
        12 => for_gen::pack12_x(base, input, output, length),
        13 => for_gen::pack13_x(base, input, output, length),
        14 => for_gen::pack14_x(base, input, output, length),
        15 => for_gen::pack15_x(base, input, output, length),
        16 => for_gen::pack16_x(base, input, output, length),
        17 => for_gen::pack17_x(base, input, output, length),
        18 => for_gen::pack18_x(base, input, output, length),
        19 => for_gen::pack19_x(base, input, output, length),
        20 => for_gen::pack20_x(base, input, output, length),
        21 => for_gen::pack21_x(base, input, output, length),
        22 => for_gen::pack22_x(base, input, output, length),
        23 => for_gen::pack23_x(base, input, output, length),
        24 => for_gen::pack24_x(base, input, output, length),
        25 => for_gen::pack25_x(base, input, output, length),
        26 => for_gen::pack26_x(base, input, output, length),
        27 => for_gen::pack27_x(base, input, output, length),
        28 => for_gen::pack28_x(base, input, output, length),
        29 => for_gen::pack29_x(base, input, output, length),
        30 => for_gen::pack30_x(base, input, output, length),
        31 => for_gen::pack31_x(base, input, output, length),
        32 => for_gen::pack32_x(base, input, output, length),
        _ => unreachable!(),
    }
}

fn call_unpack_x(bits: u32, base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    match bits {
        0 => for_gen::unpack0_x(base, input, output, length),
        1 => for_gen::unpack1_x(base, input, output, length),
        2 => for_gen::unpack2_x(base, input, output, length),
        3 => for_gen::unpack3_x(base, input, output, length),
        4 => for_gen::unpack4_x(base, input, output, length),
        5 => for_gen::unpack5_x(base, input, output, length),
        6 => for_gen::unpack6_x(base, input, output, length),
        7 => for_gen::unpack7_x(base, input, output, length),
        8 => for_gen::unpack8_x(base, input, output, length),
        9 => for_gen::unpack9_x(base, input, output, length),
        10 => for_gen::unpack10_x(base, input, output, length),
        11 => for_gen::unpack11_x(base, input, output, length),
        12 => for_gen::unpack12_x(base, input, output, length),
        13 => for_gen::unpack13_x(base, input, output, length),
        14 => for_gen::unpack14_x(base, input, output, length),
        15 => for_gen::unpack15_x(base, input, output, length),
        16 => for_gen::unpack16_x(base, input, output, length),
        17 => for_gen::unpack17_x(base, input, output, length),
        18 => for_gen::unpack18_x(base, input, output, length),
        19 => for_gen::unpack19_x(base, input, output, length),
        20 => for_gen::unpack20_x(base, input, output, length),
        21 => for_gen::unpack21_x(base, input, output, length),
        22 => for_gen::unpack22_x(base, input, output, length),
        23 => for_gen::unpack23_x(base, input, output, length),
        24 => for_gen::unpack24_x(base, input, output, length),
        25 => for_gen::unpack25_x(base, input, output, length),
        26 => for_gen::unpack26_x(base, input, output, length),
        27 => for_gen::unpack27_x(base, input, output, length),
        28 => for_gen::unpack28_x(base, input, output, length),
        29 => for_gen::unpack29_x(base, input, output, length),
        30 => for_gen::unpack30_x(base, input, output, length),
        31 => for_gen::unpack31_x(base, input, output, length),
        32 => for_gen::unpack32_x(base, input, output, length),
        _ => unreachable!(),
    }
}

fn call_linsearch(
    bits: u32,
    block: usize,
    base: u32,
    input: &[u8],
    value: u32,
    found: &mut i32,
) -> u32 {
    match (block, bits) {
        (32, 0) => for_gen::linsearch0_32(base, input, value, found),
        (16, 0) => for_gen::linsearch0_16(base, input, value, found),
        (8, 0) => for_gen::linsearch0_8(base, input, value, found),
        (32, 1) => for_gen::linsearch1_32(base, input, value, found),
        (32, 2) => for_gen::linsearch2_32(base, input, value, found),
        (32, 3) => for_gen::linsearch3_32(base, input, value, found),
        (32, 4) => for_gen::linsearch4_32(base, input, value, found),
        (32, 5) => for_gen::linsearch5_32(base, input, value, found),
        (32, 6) => for_gen::linsearch6_32(base, input, value, found),
        (32, 7) => for_gen::linsearch7_32(base, input, value, found),
        (32, 8) => for_gen::linsearch8_32(base, input, value, found),
        (32, 9) => for_gen::linsearch9_32(base, input, value, found),
        (32, 10) => for_gen::linsearch10_32(base, input, value, found),
        (32, 11) => for_gen::linsearch11_32(base, input, value, found),
        (32, 12) => for_gen::linsearch12_32(base, input, value, found),
        (32, 13) => for_gen::linsearch13_32(base, input, value, found),
        (32, 14) => for_gen::linsearch14_32(base, input, value, found),
        (32, 15) => for_gen::linsearch15_32(base, input, value, found),
        (32, 16) => for_gen::linsearch16_32(base, input, value, found),
        (32, 17) => for_gen::linsearch17_32(base, input, value, found),
        (32, 18) => for_gen::linsearch18_32(base, input, value, found),
        (32, 19) => for_gen::linsearch19_32(base, input, value, found),
        (32, 20) => for_gen::linsearch20_32(base, input, value, found),
        (32, 21) => for_gen::linsearch21_32(base, input, value, found),
        (32, 22) => for_gen::linsearch22_32(base, input, value, found),
        (32, 23) => for_gen::linsearch23_32(base, input, value, found),
        (32, 24) => for_gen::linsearch24_32(base, input, value, found),
        (32, 25) => for_gen::linsearch25_32(base, input, value, found),
        (32, 26) => for_gen::linsearch26_32(base, input, value, found),
        (32, 27) => for_gen::linsearch27_32(base, input, value, found),
        (32, 28) => for_gen::linsearch28_32(base, input, value, found),
        (32, 29) => for_gen::linsearch29_32(base, input, value, found),
        (32, 30) => for_gen::linsearch30_32(base, input, value, found),
        (32, 31) => for_gen::linsearch31_32(base, input, value, found),
        (32, 32) => for_gen::linsearch32_32(base, input, value, found),
        (16, 1) => for_gen::linsearch1_16(base, input, value, found),
        (16, 2) => for_gen::linsearch2_16(base, input, value, found),
        (16, 3) => for_gen::linsearch3_16(base, input, value, found),
        (16, 4) => for_gen::linsearch4_16(base, input, value, found),
        (16, 5) => for_gen::linsearch5_16(base, input, value, found),
        (16, 6) => for_gen::linsearch6_16(base, input, value, found),
        (16, 7) => for_gen::linsearch7_16(base, input, value, found),
        (16, 8) => for_gen::linsearch8_16(base, input, value, found),
        (16, 9) => for_gen::linsearch9_16(base, input, value, found),
        (16, 10) => for_gen::linsearch10_16(base, input, value, found),
        (16, 11) => for_gen::linsearch11_16(base, input, value, found),
        (16, 12) => for_gen::linsearch12_16(base, input, value, found),
        (16, 13) => for_gen::linsearch13_16(base, input, value, found),
        (16, 14) => for_gen::linsearch14_16(base, input, value, found),
        (16, 15) => for_gen::linsearch15_16(base, input, value, found),
        (16, 16) => for_gen::linsearch16_16(base, input, value, found),
        (16, 17) => for_gen::linsearch17_16(base, input, value, found),
        (16, 18) => for_gen::linsearch18_16(base, input, value, found),
        (16, 19) => for_gen::linsearch19_16(base, input, value, found),
        (16, 20) => for_gen::linsearch20_16(base, input, value, found),
        (16, 21) => for_gen::linsearch21_16(base, input, value, found),
        (16, 22) => for_gen::linsearch22_16(base, input, value, found),
        (16, 23) => for_gen::linsearch23_16(base, input, value, found),
        (16, 24) => for_gen::linsearch24_16(base, input, value, found),
        (16, 25) => for_gen::linsearch25_16(base, input, value, found),
        (16, 26) => for_gen::linsearch26_16(base, input, value, found),
        (16, 27) => for_gen::linsearch27_16(base, input, value, found),
        (16, 28) => for_gen::linsearch28_16(base, input, value, found),
        (16, 29) => for_gen::linsearch29_16(base, input, value, found),
        (16, 30) => for_gen::linsearch30_16(base, input, value, found),
        (16, 31) => for_gen::linsearch31_16(base, input, value, found),
        (16, 32) => for_gen::linsearch32_16(base, input, value, found),
        (8, 1) => for_gen::linsearch1_8(base, input, value, found),
        (8, 2) => for_gen::linsearch2_8(base, input, value, found),
        (8, 3) => for_gen::linsearch3_8(base, input, value, found),
        (8, 4) => for_gen::linsearch4_8(base, input, value, found),
        (8, 5) => for_gen::linsearch5_8(base, input, value, found),
        (8, 6) => for_gen::linsearch6_8(base, input, value, found),
        (8, 7) => for_gen::linsearch7_8(base, input, value, found),
        (8, 8) => for_gen::linsearch8_8(base, input, value, found),
        (8, 9) => for_gen::linsearch9_8(base, input, value, found),
        (8, 10) => for_gen::linsearch10_8(base, input, value, found),
        (8, 11) => for_gen::linsearch11_8(base, input, value, found),
        (8, 12) => for_gen::linsearch12_8(base, input, value, found),
        (8, 13) => for_gen::linsearch13_8(base, input, value, found),
        (8, 14) => for_gen::linsearch14_8(base, input, value, found),
        (8, 15) => for_gen::linsearch15_8(base, input, value, found),
        (8, 16) => for_gen::linsearch16_8(base, input, value, found),
        (8, 17) => for_gen::linsearch17_8(base, input, value, found),
        (8, 18) => for_gen::linsearch18_8(base, input, value, found),
        (8, 19) => for_gen::linsearch19_8(base, input, value, found),
        (8, 20) => for_gen::linsearch20_8(base, input, value, found),
        (8, 21) => for_gen::linsearch21_8(base, input, value, found),
        (8, 22) => for_gen::linsearch22_8(base, input, value, found),
        (8, 23) => for_gen::linsearch23_8(base, input, value, found),
        (8, 24) => for_gen::linsearch24_8(base, input, value, found),
        (8, 25) => for_gen::linsearch25_8(base, input, value, found),
        (8, 26) => for_gen::linsearch26_8(base, input, value, found),
        (8, 27) => for_gen::linsearch27_8(base, input, value, found),
        (8, 28) => for_gen::linsearch28_8(base, input, value, found),
        (8, 29) => for_gen::linsearch29_8(base, input, value, found),
        (8, 30) => for_gen::linsearch30_8(base, input, value, found),
        (8, 31) => for_gen::linsearch31_8(base, input, value, found),
        (8, 32) => for_gen::linsearch32_8(base, input, value, found),
        _ => unreachable!(),
    }
}

fn call_linsearch_x(
    bits: u32,
    base: u32,
    input: &[u8],
    length: u32,
    value: u32,
    found: &mut i32,
) -> u32 {
    match bits {
        0 => for_gen::linsearch0_x(base, input, length, value, found),
        1 => for_gen::linsearch1_x(base, input, length, value, found),
        2 => for_gen::linsearch2_x(base, input, length, value, found),
        3 => for_gen::linsearch3_x(base, input, length, value, found),
        4 => for_gen::linsearch4_x(base, input, length, value, found),
        5 => for_gen::linsearch5_x(base, input, length, value, found),
        6 => for_gen::linsearch6_x(base, input, length, value, found),
        7 => for_gen::linsearch7_x(base, input, length, value, found),
        8 => for_gen::linsearch8_x(base, input, length, value, found),
        9 => for_gen::linsearch9_x(base, input, length, value, found),
        10 => for_gen::linsearch10_x(base, input, length, value, found),
        11 => for_gen::linsearch11_x(base, input, length, value, found),
        12 => for_gen::linsearch12_x(base, input, length, value, found),
        13 => for_gen::linsearch13_x(base, input, length, value, found),
        14 => for_gen::linsearch14_x(base, input, length, value, found),
        15 => for_gen::linsearch15_x(base, input, length, value, found),
        16 => for_gen::linsearch16_x(base, input, length, value, found),
        17 => for_gen::linsearch17_x(base, input, length, value, found),
        18 => for_gen::linsearch18_x(base, input, length, value, found),
        19 => for_gen::linsearch19_x(base, input, length, value, found),
        20 => for_gen::linsearch20_x(base, input, length, value, found),
        21 => for_gen::linsearch21_x(base, input, length, value, found),
        22 => for_gen::linsearch22_x(base, input, length, value, found),
        23 => for_gen::linsearch23_x(base, input, length, value, found),
        24 => for_gen::linsearch24_x(base, input, length, value, found),
        25 => for_gen::linsearch25_x(base, input, length, value, found),
        26 => for_gen::linsearch26_x(base, input, length, value, found),
        27 => for_gen::linsearch27_x(base, input, length, value, found),
        28 => for_gen::linsearch28_x(base, input, length, value, found),
        29 => for_gen::linsearch29_x(base, input, length, value, found),
        30 => for_gen::linsearch30_x(base, input, length, value, found),
        31 => for_gen::linsearch31_x(base, input, length, value, found),
        32 => for_gen::linsearch32_x(base, input, length, value, found),
        _ => unreachable!(),
    }
}

// ===== Public API =====

pub fn for_compressed_size_bits(length: u32, bits: u32) -> u32 {
    assert!(bits <= 32);
    let mut c: u32 = 0;
    let mut length = length;
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
    if length == 0 {
        return 0;
    }
    let n = length as usize;
    let mut m = input[0];
    let mut max_v = m;
    for i in 1..n {
        let v = input[i];
        if v < m {
            m = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    let b = required_bits(max_v - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compressed_size_sorted(input: &[u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[(length - 1) as usize];
    let b = required_bits(max_v - m);
    METADATA as u32 + for_compressed_size_bits(length, b)
}

pub fn for_compress_bits(
    input: &[u32],
    output: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    assert!(bits <= 32);
    let length = length as usize;
    let mut i: usize = 0;
    let mut written: usize = 0;

    while i + 32 <= length {
        let w = call_pack(bits, 32, base, &input[i..], &mut output[written..]);
        written += w as usize;
        i += 32;
    }
    while i + 16 <= length {
        let w = call_pack(bits, 16, base, &input[i..], &mut output[written..]);
        written += w as usize;
        i += 16;
    }
    while i + 8 <= length {
        let w = call_pack(bits, 8, base, &input[i..], &mut output[written..]);
        written += w as usize;
        i += 8;
    }
    let remaining = (length - i) as u32;
    let w = call_pack_x(bits, base, &input[i..], &mut output[written..], remaining);
    written += w as usize;
    written as u32
}

pub fn for_compress_unsorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let n = length as usize;
    let mut m = input[0];
    let mut max_v = m;
    for i in 1..n {
        let v = input[i];
        if v < m {
            m = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    let b = required_bits(max_v - m);
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_compress_sorted(input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = input[0];
    let max_v = input[(length - 1) as usize];
    let b = required_bits(max_v - m);
    output[0..4].copy_from_slice(&m.to_le_bytes());
    output[4] = b as u8;
    METADATA as u32 + for_compress_bits(input, &mut output[METADATA as usize..], length, m, b)
}

pub fn for_uncompress_bits(
    input: &[u8],
    output: &mut [u32],
    length: u32,
    base: u32,
    bits: u32,
) -> u32 {
    assert!(bits <= 32);
    let length = length as usize;
    let mut i: usize = 0;
    let mut consumed: usize = 0;
    let mut o: usize = 0;

    while i + 32 <= length {
        let r = call_unpack(bits, 32, base, &input[consumed..], &mut output[o..]);
        consumed += r as usize;
        i += 32;
        o += 32;
    }
    while i + 16 <= length {
        let r = call_unpack(bits, 16, base, &input[consumed..], &mut output[o..]);
        consumed += r as usize;
        i += 16;
        o += 16;
    }
    while i + 8 <= length {
        let r = call_unpack(bits, 8, base, &input[consumed..], &mut output[o..]);
        consumed += r as usize;
        i += 8;
        o += 8;
    }
    let remaining = (length - i) as u32;
    let r = call_unpack_x(bits, base, &input[consumed..], &mut output[o..], remaining);
    (consumed + r as usize) as u32
}

pub fn for_uncompress(input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    METADATA as u32 + for_uncompress_bits(&input[METADATA as usize..], output, length, m, b)
}

/// Returns the value at the given `index` from a compressed sequence
/// (without metadata header).
pub fn for_select_bits(input: &[u8], base: u32, bits: u32, index: u32) -> u32 {
    assert!(bits <= 32);
    if bits == 0 {
        return base;
    }
    if bits == 32 {
        let off = (index as usize) * 4;
        let v = u32::from_le_bytes([input[off], input[off + 1], input[off + 2], input[off + 3]]);
        return base.wrapping_add(v);
    }

    // The compressed sequence is a packed bit-stream with values laid out
    // sequentially within block-aligned segments (32, 16, 8 then remainder).
    // The encoding is byte-aligned at every block boundary; within a block
    // values are tightly packed LSB-first as a little-endian byte stream.
    // Compute global bit position by walking blocks just like the C code.

    let mut input_off: usize = 0;
    let mut index = index;

    if index >= 32 {
        let b = index / 32;
        input_off += (b as usize * 32 * bits as usize) / 8;
        index %= 32;
    }
    if index >= 16 {
        let b = index / 16;
        input_off += (b as usize * 16 * bits as usize) / 8;
        index %= 16;
    }
    if index >= 8 {
        let b = index / 8;
        input_off += (b as usize * 8 * bits as usize) / 8;
        index %= 8;
    }

    let bit_pos = index as usize * bits as usize;
    let byte_off = input_off + bit_pos / 8;
    let bit_off = bit_pos % 8;
    let n_bytes = (bit_off + bits as usize + 7) / 8;
    let mut v64: u64 = 0;
    for k in 0..n_bytes {
        v64 |= (input[byte_off + k] as u64) << (k * 8);
    }
    let mask: u64 = (1u64 << bits) - 1;
    let v = ((v64 >> bit_off) & mask) as u32;
    base.wrapping_add(v)
}

pub fn for_select(input: &[u8], index: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_select_bits(&input[METADATA as usize..], m, b, index)
}

pub fn for_linear_search_bits(
    input: &[u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    assert!(bits <= 32);
    if bits == 0 {
        return if value == base { 0 } else { length };
    }
    let length = length as usize;
    let mut i: usize = 0;
    let mut consumed: usize = 0;
    let mut found: i32 = -1;

    while i + 32 <= length {
        let r = call_linsearch(bits, 32, base, &input[consumed..], value, &mut found);
        consumed += r as usize;
        if found >= 0 {
            return (i + found as usize) as u32;
        }
        i += 32;
    }
    while i + 16 <= length {
        let r = call_linsearch(bits, 16, base, &input[consumed..], value, &mut found);
        consumed += r as usize;
        if found >= 0 {
            return (i + found as usize) as u32;
        }
        i += 16;
    }
    while i + 8 <= length {
        let r = call_linsearch(bits, 8, base, &input[consumed..], value, &mut found);
        consumed += r as usize;
        if found >= 0 {
            return (i + found as usize) as u32;
        }
        i += 8;
    }
    let remaining = (length - i) as u32;
    call_linsearch_x(bits, base, &input[consumed..], remaining, value, &mut found);
    if found >= 0 {
        return (i + found as usize) as u32;
    }
    length as u32
}

pub fn for_linear_search(input: &[u8], length: u32, value: u32) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_linear_search_bits(&input[METADATA as usize..], length, m, b, value)
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

pub fn for_lower_bound_search(
    input: &[u8],
    length: u32,
    value: u32,
    actual: &mut u32,
) -> u32 {
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;
    for_lower_bound_search_bits(&input[METADATA as usize..], length, m, b, value, actual)
}

// ===== Append =====

pub fn for_append_bits(
    input: &mut [u8],
    length: u32,
    base: u32,
    bits: u32,
    value: u32,
) -> u32 {
    assert!(bits <= 32);
    assert!(required_bits(value.wrapping_sub(base)) <= bits);
    assert!(value >= base);

    if bits == 32 {
        // 32-bit: each value is a u32 little-endian word
        let off = (length as usize) * 4;
        let v = value.wrapping_sub(base);
        input[off..off + 4].copy_from_slice(&v.to_le_bytes());
        return ((length + 1) * 4) as u32;
    }

    // Walk through complete blocks (matches C's `length > 32`/`> 16`/`> 8` checks).
    let mut length = length;
    let mut in_off: usize = 0;
    if length > 32 {
        let b = length / 32;
        in_off += (b as usize * 32 * bits as usize) / 8;
        length %= 32;
    }
    if length > 16 {
        let b = length / 16;
        in_off += (b as usize * 16 * bits as usize) / 8;
        length %= 16;
    }
    if length > 8 {
        let b = length / 8;
        in_off += (b as usize * 8 * bits as usize) / 8;
        length %= 8;
    }

    let start_bits = (length * bits) as usize;
    let in_off_bytes = in_off + start_bits / 8;
    let bit_off = start_bits % 8;

    // Deposit `value - base` (in `bits` bits) starting at byte in_off_bytes,
    // bit offset `bit_off`. We treat the input as a little-endian byte stream
    // and OR our value into it (after clearing the target bit range).
    let v = value.wrapping_sub(base) as u64;
    let mask: u64 = (1u64 << bits) - 1;
    let v_shifted = (v & mask) << bit_off;

    let n_bytes = (bit_off + bits as usize + 7) / 8;
    for k in 0..n_bytes {
        let byte_idx = in_off_bytes + k;
        let mask_byte = ((mask << bit_off) >> (k * 8)) & 0xff;
        input[byte_idx] &= !(mask_byte as u8);
        input[byte_idx] |= ((v_shifted >> (k * 8)) & 0xff) as u8;
    }

    (in_off_bytes - in_off) as u32 + ((bit_off + bits as usize + 7) / 8) as u32
        + in_off as u32
}

pub type AppendImpl = fn(&[u32], &mut [u8], u32) -> u32;

pub fn for_append_impl(
    input: &mut [u8],
    length: u32,
    value: u32,
    appendImpl: AppendImpl,
) -> u32 {
    if length == 0 {
        let tmp_in = [value];
        return appendImpl(&tmp_in, input, 1);
    }
    // Load m and bits from metadata header
    let m = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let b = input[4] as u32;

    let bnew = required_bits(value.wrapping_sub(m));
    if m > value || bnew > b {
        // Re-encode the whole sequence
        let n = length as usize + 1;
        let mut tmp: Vec<u32> = vec![0; n];
        for_uncompress(input, &mut tmp[..n - 1], length);
        tmp[n - 1] = value;
        return appendImpl(&tmp, input, length + 1);
    }
    METADATA as u32
        + for_append_bits(&mut input[METADATA as usize..], length, m, b, value)
}

pub fn for_append_unsorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_unsorted)
}

pub fn for_append_sorted(input: &mut [u8], length: u32, value: u32) -> u32 {
    for_append_impl(input, length, value, for_compress_sorted)
}
