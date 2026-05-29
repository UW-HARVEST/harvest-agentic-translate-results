use libfor::for_gen;

// The for_gen module is a Rust translation of the auto-generated for-gen.c
// pack/unpack/linsearch helpers. As documented in src/for_gen.rs the unpack
// functions have non-standard signatures and only return the correct byte
// count rather than performing real unpacking. We therefore test:
//   * pack functions actually produce the same byte stream as the bit-packing
//     algorithm in C (verified by selecting values back via forLib).
//   * unpack stubs return correct byte counts.
//   * linsearch functions find/miss values properly.
//   * pack0/unpack0/linsearch0 special cases.

use libfor::forLib;

#[test]
fn test_pack0_returns_zero() {
    let mut out = [0u8; 8];
    let in_arr = [0u32; 32];
    assert_eq!(for_gen::pack0_32(0, &in_arr, &mut out), 0);
    assert_eq!(for_gen::pack0_16(0, &in_arr, &mut out), 0);
    assert_eq!(for_gen::pack0_8(0, &in_arr, &mut out), 0);
    assert_eq!(for_gen::pack0_x(0, &in_arr, &mut out, 5), 0);
}

#[test]
fn test_unpack0_fills_with_base() {
    let in_arr = [0u8; 4];

    let mut out = [0u32; 32];
    let n = for_gen::unpack0_32(99, &in_arr, &mut out);
    assert_eq!(n, 0);
    for i in 0..32 {
        assert_eq!(out[i], 99, "unpack0_32 idx {}", i);
    }

    let mut out = [0u32; 16];
    let n = for_gen::unpack0_16(7, &in_arr, &mut out);
    assert_eq!(n, 0);
    for i in 0..16 {
        assert_eq!(out[i], 7, "unpack0_16 idx {}", i);
    }

    let mut out = [0u32; 8];
    let n = for_gen::unpack0_8(123, &in_arr, &mut out);
    assert_eq!(n, 0);
    for i in 0..8 {
        assert_eq!(out[i], 123, "unpack0_8 idx {}", i);
    }

    let mut out = [0u32; 5];
    let n = for_gen::unpack0_x(55, &in_arr, &mut out, 5);
    assert_eq!(n, 0);
    for i in 0..5 {
        assert_eq!(out[i], 55, "unpack0_x idx {}", i);
    }
}

#[test]
fn test_linsearch0_finds_match() {
    let mut found = -1i32;
    let in_arr = [0u8; 4];

    // Found
    for_gen::linsearch0_32(42, &in_arr, 42, &mut found);
    assert_eq!(found, 0);

    let mut found = -1i32;
    for_gen::linsearch0_16(42, &in_arr, 42, &mut found);
    assert_eq!(found, 0);

    let mut found = -1i32;
    for_gen::linsearch0_8(42, &in_arr, 42, &mut found);
    assert_eq!(found, 0);

    let mut found = -1i32;
    for_gen::linsearch0_x(42, &in_arr, 5, 42, &mut found);
    assert_eq!(found, 0);

    // Not found - found stays -1
    let mut found = -1i32;
    for_gen::linsearch0_32(42, &in_arr, 99, &mut found);
    assert_eq!(found, -1);

    let mut found = -1i32;
    for_gen::linsearch0_x(42, &in_arr, 0, 42, &mut found);
    // length = 0 means "not found"
    assert_eq!(found, -1);
}

#[test]
fn test_pack4_32_produces_expected_bytes() {
    // pack4 with 32 ints, base=10, values 10..24 cycling
    let in_arr: Vec<u32> = (0..32).map(|i| 10 + (i % 15)).collect();
    let mut out = [0u8; 32];

    let n = for_gen::pack4_32(10, &in_arr, &mut out);
    // 32 ints * 4 bits = 128 bits = 16 bytes
    assert_eq!(n, 16);

    // Verify the bytes by selecting through forLib::for_select_bits.
    for i in 0..32u32 {
        let v = forLib::for_select_bits(&out, 10, 4, i);
        assert_eq!(v, in_arr[i as usize]);
    }
}

#[test]
fn test_pack8_16_produces_expected_bytes() {
    // pack 16 ints with 8 bits each
    let in_arr: Vec<u32> = (0..16).map(|i| 5 + i * 3).collect();
    let mut out = [0u8; 32];

    let n = for_gen::pack8_16(5, &in_arr, &mut out);
    assert_eq!(n, 16); // 16 * 8 = 128 bits = 16 bytes

    for i in 0..16u32 {
        let v = forLib::for_select_bits(&out, 5, 8, i);
        assert_eq!(v, in_arr[i as usize]);
    }
}

#[test]
fn test_pack3_8_produces_expected_bytes() {
    let in_arr: Vec<u32> = (0..8).map(|i| 100 + (i % 7)).collect();
    let mut out = [0u8; 16];

    let n = for_gen::pack3_8(100, &in_arr, &mut out);
    assert_eq!(n, 3); // 8 * 3 = 24 bits = 3 bytes

    for i in 0..8u32 {
        let v = forLib::for_select_bits(&out, 100, 3, i);
        assert_eq!(v, in_arr[i as usize]);
    }
}

#[test]
fn test_pack32_32_full_word_packing() {
    let in_arr: Vec<u32> = (0..32).map(|i| 1000 + i * 100).collect();
    let mut out = [0u8; 256];
    let n = for_gen::pack32_32(1000, &in_arr, &mut out);
    assert_eq!(n, 32 * 4);
    for i in 0..32u32 {
        let v = forLib::for_select_bits(&out, 1000, 32, i);
        assert_eq!(v, in_arr[i as usize]);
    }
}

#[test]
fn test_pack17_32_works() {
    // 32 ints, 17 bits, base = 1000
    let in_arr: Vec<u32> = (0..32).map(|i| 1000 + i * 1000).collect();
    let mut out = [0u8; 256];
    let n = for_gen::pack17_32(1000, &in_arr, &mut out);
    // 32 * 17 = 544 bits = 68 bytes
    assert_eq!(n, 68);
    for i in 0..32u32 {
        let v = forLib::for_select_bits(&out, 1000, 17, i);
        assert_eq!(v, in_arr[i as usize]);
    }
}

#[test]
fn test_pack_x_variable_length() {
    // pack 5 values with 4 bits each
    let in_arr: Vec<u32> = vec![10, 11, 12, 13, 14];
    let mut out = [0u8; 16];

    let n = for_gen::pack4_x(10, &in_arr, &mut out, 5);
    // 5 * 4 = 20 bits => 3 bytes
    assert_eq!(n, 3);
    for i in 0..5u32 {
        let v = forLib::for_select_bits(&out, 10, 4, i);
        assert_eq!(v, in_arr[i as usize]);
    }
}

#[test]
fn test_pack_x_zero_length() {
    let in_arr: Vec<u32> = vec![];
    let mut out = [0u8; 8];
    assert_eq!(for_gen::pack4_x(0, &in_arr, &mut out, 0), 0);
    assert_eq!(for_gen::pack8_x(0, &in_arr, &mut out, 0), 0);
}

#[test]
fn test_unpack_block_returns_byte_count() {
    // unpack stubs only return the correct byte count (signature is wrong).
    // 32 ints * 4 bits = 128 bits = 16 bytes
    let in_arr = [0u32; 8];
    let mut out = [0u8; 32];
    assert_eq!(for_gen::unpack4_32(0, &in_arr, &mut out), 16);
    assert_eq!(for_gen::unpack8_16(0, &in_arr, &mut out), 16);
    assert_eq!(for_gen::unpack3_8(0, &in_arr, &mut out), 3);
    assert_eq!(for_gen::unpack17_32(0, &in_arr, &mut out), 68);
    assert_eq!(for_gen::unpack32_32(0, &in_arr, &mut out), 128);
    assert_eq!(for_gen::unpack32_16(0, &in_arr, &mut out), 64);
    assert_eq!(for_gen::unpack32_8(0, &in_arr, &mut out), 32);
}

#[test]
fn test_unpack_x_returns_byte_count() {
    let in_arr = [0u32; 8];
    let mut out = [0u8; 32];
    assert_eq!(for_gen::unpack4_x(0, &in_arr, &mut out, 5), 3);
    assert_eq!(for_gen::unpack8_x(0, &in_arr, &mut out, 4), 4);
    assert_eq!(for_gen::unpack17_x(0, &in_arr, &mut out, 8), 17);
    assert_eq!(for_gen::unpack32_x(0, &in_arr, &mut out, 8), 32);
    assert_eq!(for_gen::unpack1_x(0, &in_arr, &mut out, 8), 1);
}

#[test]
fn test_linsearch_block_finds_value() {
    // Pack values 10..24 cycling, then linear search.
    let in_arr: Vec<u32> = (0..32).map(|i| 10 + (i % 15)).collect();
    let mut buf = [0u8; 32];
    for_gen::pack4_32(10, &in_arr, &mut buf);

    // Find first occurrence of 14 (idx 4)
    let mut found = -1i32;
    let n = for_gen::linsearch4_32(10, &buf, 14, &mut found);
    assert_eq!(found, 4);
    // Returned size = bytes consumed up to and including index 4 (5 elements)
    // 5 * 4 = 20 bits = 3 bytes
    assert_eq!(n, 3);

    // Searching for value not present returns full block size with found unchanged
    let mut found = -1i32;
    let n = for_gen::linsearch4_32(10, &buf, 99, &mut found);
    assert_eq!(found, -1);
    assert_eq!(n, 16);
}

#[test]
fn test_linsearch_block_value_below_base() {
    // value < base is interpreted as "not found" and returns full block.
    let buf = [0u8; 32];
    let mut found = -1i32;
    let n = for_gen::linsearch4_32(10, &buf, 5, &mut found);
    assert_eq!(found, -1);
    assert_eq!(n, 16);
}

#[test]
fn test_linsearch_x_finds_value() {
    let in_arr: Vec<u32> = vec![10, 11, 12, 13, 14];
    let mut buf = [0u8; 16];
    for_gen::pack4_x(10, &in_arr, &mut buf, 5);

    let mut found = -1i32;
    let n = for_gen::linsearch4_x(10, &buf, 5, 13, &mut found);
    assert_eq!(found, 3);
    // 4 elements consumed = 16 bits = 2 bytes
    assert_eq!(n, 2);

    let mut found = -1i32;
    let n = for_gen::linsearch4_x(10, &buf, 5, 99, &mut found);
    assert_eq!(found, -1);
    // 5 * 4 = 20 bits => 3 bytes
    assert_eq!(n, 3);
}

#[test]
fn test_linsearch_x_zero_length() {
    let buf = [0u8; 4];
    let mut found = -1i32;
    let n = for_gen::linsearch4_x(10, &buf, 0, 99, &mut found);
    assert_eq!(found, -1);
    assert_eq!(n, 0);
}

#[test]
fn test_linsearch_x_value_below_base() {
    let buf = [0u8; 4];
    let mut found = -1i32;
    let n = for_gen::linsearch4_x(10, &buf, 5, 5, &mut found);
    assert_eq!(found, -1);
    // 5 * 4 = 20 bits => 3 bytes
    assert_eq!(n, 3);
}

#[test]
fn test_pack1_32_packs_bits() {
    // 32 ints of 1 bit each = 4 bytes
    let in_arr: Vec<u32> = (0..32).map(|i| (i % 2) as u32).collect();
    let mut out = [0u8; 8];
    let n = for_gen::pack1_32(0, &in_arr, &mut out);
    assert_eq!(n, 4);
    for i in 0..32u32 {
        assert_eq!(forLib::for_select_bits(&out, 0, 1, i), in_arr[i as usize]);
    }
}

#[test]
fn test_pack16_8_packs_bits() {
    // 8 ints of 16 bits each = 16 bytes
    let in_arr: Vec<u32> = (0..8).map(|i| 1000 + i * 100).collect();
    let mut out = [0u8; 32];
    let n = for_gen::pack16_8(1000, &in_arr, &mut out);
    assert_eq!(n, 16);
    for i in 0..8u32 {
        assert_eq!(
            forLib::for_select_bits(&out, 1000, 16, i),
            in_arr[i as usize]
        );
    }
}

#[test]
fn test_pack24_16_packs_bits() {
    // 16 ints of 24 bits each = 48 bytes
    let in_arr: Vec<u32> = (0..16).map(|i| 100_000 + i * 1000).collect();
    let mut out = [0u8; 64];
    let n = for_gen::pack24_16(100_000, &in_arr, &mut out);
    assert_eq!(n, 48);
    for i in 0..16u32 {
        assert_eq!(
            forLib::for_select_bits(&out, 100_000, 24, i),
            in_arr[i as usize]
        );
    }
}

fn main() {}
