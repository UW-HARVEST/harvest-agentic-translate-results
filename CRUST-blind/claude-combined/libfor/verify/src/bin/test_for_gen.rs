// Tests for the for_gen module.
//
// for_gen contains 396 small per-(bits, block) packing/unpacking primitives.
// These mirror the C "for-gen.c" generated routines.
//
// Note: the shipped Rust signatures for unpack* (bits>=1) and unpack*_x are
// inconsistent with their C counterparts (the input/output types are swapped,
// see the implementation comments). Per project rules we cannot modify those
// signatures. Our implementation makes them behave like the corresponding
// `pack` function, so a test exercising the same arguments produces the same
// observable result as `pack` does.

use libfor::for_gen;
use libfor::forLib;

// ----- Block-zero (special) functions: canonical signatures -----

#[test]
fn test_pack0_block_funcs_return_zero() {
    let input = [0u32; 32];
    let mut output = [0u8; 32];
    assert_eq!(for_gen::pack0_32(7, &input, &mut output), 0);
    assert_eq!(for_gen::pack0_16(7, &input, &mut output), 0);
    assert_eq!(for_gen::pack0_8(7, &input, &mut output), 0);
    assert_eq!(for_gen::pack0_x(7, &input, &mut output, 5), 0);
}

#[test]
fn test_unpack0_32_fills_with_base() {
    let input = [0u8; 0];
    let mut output = [0u32; 32];
    let r = for_gen::unpack0_32(99, &input, &mut output);
    assert_eq!(r, 0);
    for i in 0..32 {
        assert_eq!(output[i], 99, "i={}", i);
    }
}

#[test]
fn test_unpack0_16_fills_with_base() {
    let input = [0u8; 0];
    let mut output = [0u32; 16];
    let r = for_gen::unpack0_16(7, &input, &mut output);
    assert_eq!(r, 0);
    for i in 0..16 {
        assert_eq!(output[i], 7, "i={}", i);
    }
}

#[test]
fn test_unpack0_8_fills_with_base() {
    let input = [0u8; 0];
    let mut output = [0u32; 8];
    let r = for_gen::unpack0_8(13, &input, &mut output);
    assert_eq!(r, 0);
    for i in 0..8 {
        assert_eq!(output[i], 13, "i={}", i);
    }
}

#[test]
fn test_unpack0_x_fills_with_base() {
    let input = [0u8; 0];
    let mut output = [0u32; 12];
    let r = for_gen::unpack0_x(42, &input, &mut output, 12);
    assert_eq!(r, 0);
    for i in 0..12 {
        assert_eq!(output[i], 42, "i={}", i);
    }
    // length=0 is also valid:
    let mut output = [0u32; 0];
    let r = for_gen::unpack0_x(42, &input, &mut output, 0);
    assert_eq!(r, 0);
}

#[test]
fn test_linsearch0_match_and_nomatch() {
    let input = [0u8; 0];
    let mut found: i32 = -1;
    // base==value: found=0
    assert_eq!(for_gen::linsearch0_32(7, &input, 7, &mut found), 0);
    assert_eq!(found, 0);
    let mut found: i32 = -1;
    // base!=value: found unchanged
    assert_eq!(for_gen::linsearch0_32(7, &input, 8, &mut found), 0);
    assert_eq!(found, -1);

    let mut found: i32 = -1;
    assert_eq!(for_gen::linsearch0_16(7, &input, 7, &mut found), 0);
    assert_eq!(found, 0);

    let mut found: i32 = -1;
    assert_eq!(for_gen::linsearch0_8(7, &input, 7, &mut found), 0);
    assert_eq!(found, 0);

    let mut found: i32 = -1;
    // _x with length 0 should NOT trigger a found update even if value==base.
    assert_eq!(for_gen::linsearch0_x(7, &input, 0, 7, &mut found), 0);
    assert_eq!(found, -1);
    let mut found: i32 = -1;
    assert_eq!(for_gen::linsearch0_x(7, &input, 5, 7, &mut found), 0);
    assert_eq!(found, 0);
    let mut found: i32 = -1;
    assert_eq!(for_gen::linsearch0_x(7, &input, 5, 8, &mut found), 0);
    assert_eq!(found, -1);
}

// ----- pack/unpack sizes: bytes returned must match for_compressed_size_bits -----

#[test]
fn test_pack_n_32_returns_correct_size() {
    // Compare the bytes-written count from pack with for_compressed_size_bits.
    let input: [u32; 32] = core::array::from_fn(|i| i as u32);
    let mut output = vec![0u8; 200];
    for bits in 1u32..=32 {
        let r = match bits {
            1 => for_gen::pack1_32(0, &input, &mut output),
            2 => for_gen::pack2_32(0, &input, &mut output),
            3 => for_gen::pack3_32(0, &input, &mut output),
            4 => for_gen::pack4_32(0, &input, &mut output),
            5 => for_gen::pack5_32(0, &input, &mut output),
            6 => for_gen::pack6_32(0, &input, &mut output),
            7 => for_gen::pack7_32(0, &input, &mut output),
            8 => for_gen::pack8_32(0, &input, &mut output),
            9 => for_gen::pack9_32(0, &input, &mut output),
            10 => for_gen::pack10_32(0, &input, &mut output),
            11 => for_gen::pack11_32(0, &input, &mut output),
            12 => for_gen::pack12_32(0, &input, &mut output),
            13 => for_gen::pack13_32(0, &input, &mut output),
            14 => for_gen::pack14_32(0, &input, &mut output),
            15 => for_gen::pack15_32(0, &input, &mut output),
            16 => for_gen::pack16_32(0, &input, &mut output),
            17 => for_gen::pack17_32(0, &input, &mut output),
            18 => for_gen::pack18_32(0, &input, &mut output),
            19 => for_gen::pack19_32(0, &input, &mut output),
            20 => for_gen::pack20_32(0, &input, &mut output),
            21 => for_gen::pack21_32(0, &input, &mut output),
            22 => for_gen::pack22_32(0, &input, &mut output),
            23 => for_gen::pack23_32(0, &input, &mut output),
            24 => for_gen::pack24_32(0, &input, &mut output),
            25 => for_gen::pack25_32(0, &input, &mut output),
            26 => for_gen::pack26_32(0, &input, &mut output),
            27 => for_gen::pack27_32(0, &input, &mut output),
            28 => for_gen::pack28_32(0, &input, &mut output),
            29 => for_gen::pack29_32(0, &input, &mut output),
            30 => for_gen::pack30_32(0, &input, &mut output),
            31 => for_gen::pack31_32(0, &input, &mut output),
            32 => for_gen::pack32_32(0, &input, &mut output),
            _ => unreachable!(),
        };
        let expected = forLib::for_compressed_size_bits(32, bits);
        assert_eq!(r, expected, "bits={}", bits);
    }
}

#[test]
fn test_pack_n_16_returns_correct_size() {
    let input: [u32; 16] = core::array::from_fn(|i| i as u32);
    let mut output = vec![0u8; 100];
    for bits in 1u32..=32 {
        let r = match bits {
            1 => for_gen::pack1_16(0, &input, &mut output),
            2 => for_gen::pack2_16(0, &input, &mut output),
            3 => for_gen::pack3_16(0, &input, &mut output),
            4 => for_gen::pack4_16(0, &input, &mut output),
            5 => for_gen::pack5_16(0, &input, &mut output),
            6 => for_gen::pack6_16(0, &input, &mut output),
            7 => for_gen::pack7_16(0, &input, &mut output),
            8 => for_gen::pack8_16(0, &input, &mut output),
            9 => for_gen::pack9_16(0, &input, &mut output),
            10 => for_gen::pack10_16(0, &input, &mut output),
            11 => for_gen::pack11_16(0, &input, &mut output),
            12 => for_gen::pack12_16(0, &input, &mut output),
            13 => for_gen::pack13_16(0, &input, &mut output),
            14 => for_gen::pack14_16(0, &input, &mut output),
            15 => for_gen::pack15_16(0, &input, &mut output),
            16 => for_gen::pack16_16(0, &input, &mut output),
            17 => for_gen::pack17_16(0, &input, &mut output),
            18 => for_gen::pack18_16(0, &input, &mut output),
            19 => for_gen::pack19_16(0, &input, &mut output),
            20 => for_gen::pack20_16(0, &input, &mut output),
            21 => for_gen::pack21_16(0, &input, &mut output),
            22 => for_gen::pack22_16(0, &input, &mut output),
            23 => for_gen::pack23_16(0, &input, &mut output),
            24 => for_gen::pack24_16(0, &input, &mut output),
            25 => for_gen::pack25_16(0, &input, &mut output),
            26 => for_gen::pack26_16(0, &input, &mut output),
            27 => for_gen::pack27_16(0, &input, &mut output),
            28 => for_gen::pack28_16(0, &input, &mut output),
            29 => for_gen::pack29_16(0, &input, &mut output),
            30 => for_gen::pack30_16(0, &input, &mut output),
            31 => for_gen::pack31_16(0, &input, &mut output),
            32 => for_gen::pack32_16(0, &input, &mut output),
            _ => unreachable!(),
        };
        let expected = forLib::for_compressed_size_bits(16, bits);
        assert_eq!(r, expected, "bits={}", bits);
    }
}

#[test]
fn test_pack_n_8_returns_correct_size() {
    let input: [u32; 8] = core::array::from_fn(|i| i as u32);
    let mut output = vec![0u8; 100];
    for bits in 1u32..=32 {
        let r = match bits {
            1 => for_gen::pack1_8(0, &input, &mut output),
            2 => for_gen::pack2_8(0, &input, &mut output),
            3 => for_gen::pack3_8(0, &input, &mut output),
            4 => for_gen::pack4_8(0, &input, &mut output),
            5 => for_gen::pack5_8(0, &input, &mut output),
            6 => for_gen::pack6_8(0, &input, &mut output),
            7 => for_gen::pack7_8(0, &input, &mut output),
            8 => for_gen::pack8_8(0, &input, &mut output),
            9 => for_gen::pack9_8(0, &input, &mut output),
            10 => for_gen::pack10_8(0, &input, &mut output),
            11 => for_gen::pack11_8(0, &input, &mut output),
            12 => for_gen::pack12_8(0, &input, &mut output),
            13 => for_gen::pack13_8(0, &input, &mut output),
            14 => for_gen::pack14_8(0, &input, &mut output),
            15 => for_gen::pack15_8(0, &input, &mut output),
            16 => for_gen::pack16_8(0, &input, &mut output),
            17 => for_gen::pack17_8(0, &input, &mut output),
            18 => for_gen::pack18_8(0, &input, &mut output),
            19 => for_gen::pack19_8(0, &input, &mut output),
            20 => for_gen::pack20_8(0, &input, &mut output),
            21 => for_gen::pack21_8(0, &input, &mut output),
            22 => for_gen::pack22_8(0, &input, &mut output),
            23 => for_gen::pack23_8(0, &input, &mut output),
            24 => for_gen::pack24_8(0, &input, &mut output),
            25 => for_gen::pack25_8(0, &input, &mut output),
            26 => for_gen::pack26_8(0, &input, &mut output),
            27 => for_gen::pack27_8(0, &input, &mut output),
            28 => for_gen::pack28_8(0, &input, &mut output),
            29 => for_gen::pack29_8(0, &input, &mut output),
            30 => for_gen::pack30_8(0, &input, &mut output),
            31 => for_gen::pack31_8(0, &input, &mut output),
            32 => for_gen::pack32_8(0, &input, &mut output),
            _ => unreachable!(),
        };
        let expected = forLib::for_compressed_size_bits(8, bits);
        assert_eq!(r, expected, "bits={}", bits);
    }
}

#[test]
fn test_pack_x_returns_correct_size() {
    // pack_n_x with length parameter
    let input: [u32; 8] = core::array::from_fn(|i| i as u32);
    let mut output = vec![0u8; 100];
    for length in 1u32..=8 {
        let r = for_gen::pack5_x(0, &input, &mut output, length);
        let expected = forLib::for_compressed_size_bits(length, 5);
        assert_eq!(r, expected, "length={}", length);
    }
    for length in 1u32..=8 {
        let r = for_gen::pack16_x(0, &input, &mut output, length);
        let expected = forLib::for_compressed_size_bits(length, 16);
        assert_eq!(r, expected, "length={}", length);
    }
}

// ----- pack roundtrip via for_select_bits -----

#[test]
fn test_pack4_32_roundtrip_via_select_bits() {
    let input: [u32; 32] = core::array::from_fn(|i| 10 + (i as u32 % 16));
    let mut output = vec![0u8; 100];
    let s = for_gen::pack4_32(10, &input, &mut output);
    assert_eq!(s, 16); // 32*4/8 = 16
    for i in 0..32u32 {
        let v = forLib::for_select_bits(&output, 10, 4, i);
        assert_eq!(v, input[i as usize], "i={}", i);
    }
}

#[test]
fn test_pack3_8_roundtrip_via_select_bits() {
    let input: [u32; 8] = [5, 6, 7, 8, 9, 10, 11, 12]; // delta range 0..7 needs 3 bits
    let mut output = vec![0u8; 16];
    let s = for_gen::pack3_8(5, &input, &mut output);
    assert_eq!(s, 3); // ceil(8*3/8) = 3
    for i in 0..8u32 {
        assert_eq!(forLib::for_select_bits(&output, 5, 3, i), input[i as usize]);
    }
}

#[test]
fn test_pack16_16_roundtrip() {
    let input: [u32; 16] = [
        100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1100, 1200, 1300, 1400, 1500, 1600,
    ];
    let mut output = vec![0u8; 64];
    let s = for_gen::pack16_16(0, &input, &mut output);
    assert_eq!(s, 32); // 16*16/8 = 32
    for i in 0..16u32 {
        assert_eq!(forLib::for_select_bits(&output, 0, 16, i), input[i as usize]);
    }
}

#[test]
fn test_pack32_8_roundtrip() {
    // 32-bit packs: each int stored as a full uint32_t little-endian.
    let input: [u32; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    let mut output = vec![0u8; 64];
    let s = for_gen::pack32_8(0, &input, &mut output);
    assert_eq!(s, 32);
    for i in 0..8 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&output[i * 4..i * 4 + 4]);
        assert_eq!(u32::from_le_bytes(buf), input[i]);
    }
}

// ----- linsearch -----

#[test]
fn test_linsearch5_8_finds_value() {
    let input: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut output = vec![0u8; 16];
    for_gen::pack5_8(10, &input, &mut output);

    for i in 0..8u32 {
        let mut found: i32 = -1;
        let r = for_gen::linsearch5_8(10, &output, 10 + i, &mut found);
        assert_eq!(found, i as i32, "expected to find at index {}", i);
        assert_eq!(r, i, "return value should be index");
    }
}

#[test]
fn test_linsearch4_8_not_found() {
    let input: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut output = vec![0u8; 16];
    for_gen::pack4_8(10, &input, &mut output);

    let mut found: i32 = -1;
    // value below base
    let r = for_gen::linsearch4_8(10, &output, 5, &mut found);
    assert_eq!(found, -1);
    // total bytes consumed = ceil(8*4/8) = 4
    assert_eq!(r, 4);

    let mut found: i32 = -1;
    // value above max storable in 4 bits => not found
    let r = for_gen::linsearch4_8(10, &output, 100, &mut found);
    assert_eq!(found, -1);
    assert_eq!(r, 4);
}

#[test]
fn test_linsearch_n_x_finds_and_misses() {
    let input: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut output = vec![0u8; 16];
    for_gen::pack4_x(10, &input, &mut output, 8);

    let mut found: i32 = -1;
    let r = for_gen::linsearch4_x(10, &output, 8, 13, &mut found);
    assert_eq!(found, 3);
    assert_eq!(r, 3);

    let mut found: i32 = -1;
    let r = for_gen::linsearch4_x(10, &output, 8, 100, &mut found);
    assert_eq!(found, -1);
    // bytes returned = ceil(8*4/8) = 4
    assert_eq!(r, 4);

    // length=0 => returns 0, found unchanged
    let mut found: i32 = -1;
    let r = for_gen::linsearch4_x(10, &output, 0, 13, &mut found);
    assert_eq!(found, -1);
    assert_eq!(r, 0);
}

#[test]
fn test_linsearch_block_returns_block_size_when_not_found() {
    let input: [u32; 32] = core::array::from_fn(|i| 100 + i as u32);
    let mut output = vec![0u8; 200];
    for_gen::pack8_32(100, &input, &mut output);

    let mut found: i32 = -1;
    // value below base
    let r = for_gen::linsearch8_32(100, &output, 50, &mut found);
    assert_eq!(found, -1);
    // block consumed = 32 bytes
    assert_eq!(r, 32);

    // value present (delta=15)
    let mut found: i32 = -1;
    let _ = for_gen::linsearch8_32(100, &output, 115, &mut found);
    assert_eq!(found, 15);
}

#[test]
fn test_linsearch_32bit_returns_32_block_size() {
    // 32-bit linsearch with block size 8: returns 32 when value not found.
    let input: [u32; 8] = [100, 200, 300, 400, 500, 600, 700, 800];
    let mut output = vec![0u8; 64];
    for_gen::pack32_8(0, &input, &mut output);

    let mut found: i32 = -1;
    let r = for_gen::linsearch32_8(0, &output, 999_999, &mut found);
    assert_eq!(found, -1);
    assert_eq!(r, 32);

    // Found case: returns 0
    let mut found: i32 = -1;
    let r = for_gen::linsearch32_8(0, &output, 500, &mut found);
    assert_eq!(found, 4);
    assert_eq!(r, 0);
}

#[test]
fn test_unpack_n_8_swapped_signature_returns_size() {
    // unpack* with bits >= 1 has swapped types in the shipped Rust API.
    // Verify it still returns a sensible (matching pack) result.
    let input: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut output = vec![0u8; 16];
    let s = for_gen::unpack4_8(10, &input, &mut output);
    let expected = forLib::for_compressed_size_bits(8, 4);
    assert_eq!(s, expected); // 4 bytes
}

#[test]
fn test_unpack_n_x_swapped_signature_returns_size() {
    let input: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut output = vec![0u8; 16];
    let s = for_gen::unpack4_x(10, &input, &mut output, 8);
    let expected = forLib::for_compressed_size_bits(8, 4);
    assert_eq!(s, expected);
    // length=0 => 0
    let s = for_gen::unpack4_x(10, &input, &mut output, 0);
    assert_eq!(s, 0);
}

fn main() {}
