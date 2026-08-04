use libfor::forLib;

#[test]
fn test_required_bits() {
    // 0 -> 0, otherwise floor(log2(v)) + 1
    assert_eq!(forLib::required_bits(0), 0);
    assert_eq!(forLib::required_bits(1), 1);
    assert_eq!(forLib::required_bits(2), 2);
    assert_eq!(forLib::required_bits(3), 2);
    assert_eq!(forLib::required_bits(4), 3);
    assert_eq!(forLib::required_bits(7), 3);
    assert_eq!(forLib::required_bits(8), 4);
    assert_eq!(forLib::required_bits(15), 4);
    assert_eq!(forLib::required_bits(16), 5);
    assert_eq!(forLib::required_bits(255), 8);
    assert_eq!(forLib::required_bits(256), 9);
    assert_eq!(forLib::required_bits(0xFFFF), 16);
    assert_eq!(forLib::required_bits(0xFFFFFF), 24);
    assert_eq!(forLib::required_bits(0xFFFFFFFF), 32);
}

#[test]
fn test_compressed_size_bits_small() {
    // values from running the C code
    assert_eq!(forLib::for_compressed_size_bits(0, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(0, 32), 0);
    assert_eq!(forLib::for_compressed_size_bits(1, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(1, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(1, 8), 1);
    assert_eq!(forLib::for_compressed_size_bits(1, 16), 2);
    assert_eq!(forLib::for_compressed_size_bits(1, 24), 3);
    assert_eq!(forLib::for_compressed_size_bits(1, 32), 4);
    assert_eq!(forLib::for_compressed_size_bits(7, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(7, 8), 7);
    assert_eq!(forLib::for_compressed_size_bits(7, 32), 28);
    assert_eq!(forLib::for_compressed_size_bits(8, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(8, 8), 8);
    assert_eq!(forLib::for_compressed_size_bits(8, 32), 32);
}

#[test]
fn test_compressed_size_bits_blocks() {
    // From running the C ground truth.
    assert_eq!(forLib::for_compressed_size_bits(15, 4), 8);
    assert_eq!(forLib::for_compressed_size_bits(15, 7), 14);
    assert_eq!(forLib::for_compressed_size_bits(16, 1), 2);
    assert_eq!(forLib::for_compressed_size_bits(16, 8), 16);
    assert_eq!(forLib::for_compressed_size_bits(16, 16), 32);
    assert_eq!(forLib::for_compressed_size_bits(31, 5), 20);
    assert_eq!(forLib::for_compressed_size_bits(32, 1), 4);
    assert_eq!(forLib::for_compressed_size_bits(32, 8), 32);
    assert_eq!(forLib::for_compressed_size_bits(32, 32), 128);
    assert_eq!(forLib::for_compressed_size_bits(33, 1), 5);
    assert_eq!(forLib::for_compressed_size_bits(33, 7), 29);
    assert_eq!(forLib::for_compressed_size_bits(33, 32), 132);
    assert_eq!(forLib::for_compressed_size_bits(64, 7), 56);
    assert_eq!(forLib::for_compressed_size_bits(128, 5), 80);
    assert_eq!(forLib::for_compressed_size_bits(1000, 1), 125);
    assert_eq!(forLib::for_compressed_size_bits(1000, 32), 4000);
}

#[test]
fn test_compressed_size_unsorted_basic() {
    // Single value: METADATA(5) + 0 (0 bits)
    let a = [5u32];
    assert_eq!(forLib::for_compressed_size_unsorted(&a, 1), 5);

    // [1,2,3]: range=2, bits=2, size = 5 + ((3*2 + 7)/8) = 5 + 1 = 6
    let a = [1u32, 2, 3];
    assert_eq!(forLib::for_compressed_size_unsorted(&a, 3), 6);

    // [100,5,50,1,200]: range=199, bits=8, size = 5 + 5 = 10
    let a = [100u32, 5, 50, 1, 200];
    assert_eq!(forLib::for_compressed_size_unsorted(&a, 5), 10);

    // All same -> 0 bits, size = METADATA = 5
    let a = [0u32, 0, 0, 0];
    assert_eq!(forLib::for_compressed_size_unsorted(&a, 4), 5);

    // length = 0
    let a: [u32; 0] = [];
    assert_eq!(forLib::for_compressed_size_unsorted(&a, 0), 0);
}

#[test]
fn test_compressed_size_sorted_basic() {
    // length = 0
    let a: [u32; 0] = [];
    assert_eq!(forLib::for_compressed_size_sorted(&a, 0), 0);

    let a = [33u32, 34, 35, 36, 37];
    assert_eq!(forLib::for_compressed_size_sorted(&a, 5), 7);

    let a: Vec<u32> = (0..17).map(|i| 33 + i).collect();
    assert_eq!(forLib::for_compressed_size_sorted(&a, 17), 16);

    let a: Vec<u32> = (0..33).map(|i| 100 + i).collect();
    assert_eq!(forLib::for_compressed_size_sorted(&a, 33), 30);
}

#[test]
fn test_compress_uncompress_sorted_5() {
    // Round-trip test for a sorted sequence of 5 values.
    let input: [u32; 5] = [33, 34, 35, 36, 37];
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 5];

    let s1 = forLib::for_compress_sorted(&input, &mut out, 5);
    assert_eq!(s1, 7);

    let s2 = forLib::for_uncompress(&out, &mut tmp, 5);
    assert_eq!(s2, 7);

    let s3 = forLib::for_compressed_size_sorted(&input, 5);
    assert_eq!(s3, 7);

    for i in 0..5 {
        assert_eq!(tmp[i], input[i]);
    }

    // From C ground truth: bytes = 21 00 00 00 03 88 46
    // metadata: 0x21 0x00 0x00 0x00 (33 LE), 0x03 (bits = 3, since max-min=4 needs 3 bits)
    assert_eq!(out[0], 0x21);
    assert_eq!(out[1], 0x00);
    assert_eq!(out[2], 0x00);
    assert_eq!(out[3], 0x00);
    assert_eq!(out[4], 0x03);
    assert_eq!(out[5], 0x88);
    assert_eq!(out[6], 0x46);
}

#[test]
fn test_compress_uncompress_sorted_17() {
    let input: Vec<u32> = (0..17).map(|i| 33 + i).collect();
    let mut out = vec![0u8; 256];
    let mut tmp = vec![0u32; 17];

    let s1 = forLib::for_compress_sorted(&input, &mut out, 17);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 17);
    let s3 = forLib::for_compressed_size_sorted(&input, 17);

    assert_eq!(s1, 16);
    assert_eq!(s2, 16);
    assert_eq!(s3, 16);

    for i in 0..17 {
        assert_eq!(tmp[i], input[i]);
    }

    // verify select
    for i in 0..17 {
        assert_eq!(forLib::for_select(&out, i as u32), input[i]);
    }

    // verify linear_search
    for i in 0..17 {
        assert_eq!(forLib::for_linear_search(&out, 17, input[i]), i as u32);
    }

    // verify lower_bound
    for i in 0..17 {
        let mut actual = 999u32;
        let idx = forLib::for_lower_bound_search(&out, 17, input[i], &mut actual);
        assert_eq!(idx, i as u32);
        assert_eq!(actual, input[i]);
    }
}

#[test]
fn test_compress_uncompress_various_lengths() {
    for &len in &[1u32, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257] {
        let input: Vec<u32> = (0..len).map(|i| 33 + i).collect();
        let mut out = vec![0u8; (len as usize) * 8 + 32];
        let mut tmp = vec![0u32; len as usize];

        let s1 = forLib::for_compress_sorted(&input, &mut out, len);
        let s2 = forLib::for_uncompress(&out, &mut tmp, len);
        let s3 = forLib::for_compressed_size_sorted(&input, len);

        assert_eq!(s1, s2, "compress != uncompress for len={}", len);
        assert_eq!(s2, s3, "uncompress != size for len={}", len);

        for i in 0..len as usize {
            assert_eq!(tmp[i], input[i], "mismatch at i={} len={}", i, len);
        }
        for i in 0..len {
            assert_eq!(
                forLib::for_select(&out, i),
                input[i as usize],
                "select mismatch at i={} len={}",
                i,
                len
            );
        }
    }
}

#[test]
fn test_compress_zero_length() {
    let input: [u32; 0] = [];
    let mut out = vec![0u8; 32];
    let s = forLib::for_compress_sorted(&input, &mut out, 0);
    assert_eq!(s, 0);

    let s = forLib::for_compress_unsorted(&input, &mut out, 0);
    assert_eq!(s, 0);

    let mut tmp = vec![0u32; 4];
    let s = forLib::for_uncompress(&out, &mut tmp, 0);
    assert_eq!(s, 0);
}

#[test]
fn test_compress_unsorted_round_trip() {
    let input: [u32; 5] = [100, 5, 50, 1, 200];
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 5];

    let s1 = forLib::for_compress_unsorted(&input, &mut out, 5);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 5);
    let s3 = forLib::for_compressed_size_unsorted(&input, 5);

    assert_eq!(s1, 10);
    assert_eq!(s2, 10);
    assert_eq!(s3, 10);

    for i in 0..5 {
        assert_eq!(tmp[i], input[i]);
    }

    // min=1, bits=8
    assert_eq!(out[0], 0x01);
    assert_eq!(out[1], 0x00);
    assert_eq!(out[2], 0x00);
    assert_eq!(out[3], 0x00);
    assert_eq!(out[4], 0x08);
}

#[test]
fn test_compress_bits_uncompress_bits_4() {
    let input: Vec<u32> = (0..32).map(|i| 10 + (i % 15)).collect();
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 32];

    let s1 = forLib::for_compress_bits(&input, &mut out, 32, 10, 4);
    assert_eq!(s1, 16);

    let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 32, 10, 4);
    assert_eq!(s2, 16);

    for i in 0..32 {
        assert_eq!(tmp[i], input[i]);
    }
    for i in 0..32 {
        assert_eq!(forLib::for_select_bits(&out, 10, 4, i as u32), input[i]);
    }
}

#[test]
fn test_compress_bits_8_bits() {
    let input: Vec<u32> = (0..16).map(|i| 5 + i * 3).collect();
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 16];

    let s1 = forLib::for_compress_bits(&input, &mut out, 16, 5, 8);
    assert_eq!(s1, 16);

    let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 16, 5, 8);
    assert_eq!(s2, 16);

    for i in 0..16 {
        assert_eq!(tmp[i], input[i]);
    }
}

#[test]
fn test_compress_bits_3_bits() {
    let input: Vec<u32> = (0..8).map(|i| 100 + (i % 7)).collect();
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 8];

    let s1 = forLib::for_compress_bits(&input, &mut out, 8, 100, 3);
    assert_eq!(s1, 3);

    let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 8, 100, 3);
    assert_eq!(s2, 3);

    for i in 0..8 {
        assert_eq!(tmp[i], input[i]);
    }
}

#[test]
fn test_compress_bits_32_bits() {
    let input: Vec<u32> = (0..8).map(|i| 1_000_000 + i * 100_000).collect();
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 8];

    let s1 = forLib::for_compress_bits(&input, &mut out, 8, 1_000_000, 32);
    assert_eq!(s1, 32);

    let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 8, 1_000_000, 32);
    assert_eq!(s2, 32);

    for i in 0..8 {
        assert_eq!(tmp[i], input[i]);
    }
}

#[test]
fn test_compress_bits_1_bit() {
    let input: Vec<u32> = (0..16).map(|i| (i % 2) as u32).collect();
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 16];

    let s1 = forLib::for_compress_bits(&input, &mut out, 16, 0, 1);
    assert_eq!(s1, 2);

    let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 16, 0, 1);
    assert_eq!(s2, 2);

    for i in 0..16 {
        assert_eq!(tmp[i], input[i]);
    }
    for i in 0..16 {
        assert_eq!(forLib::for_select_bits(&out, 0, 1, i as u32), input[i]);
    }
}

#[test]
fn test_compress_bits_17_bits() {
    let input: Vec<u32> = (0..32).map(|i| 1000 + i * 1000).collect();
    let mut out = vec![0u8; 256];
    let mut tmp = vec![0u32; 32];

    let s1 = forLib::for_compress_bits(&input, &mut out, 32, 1000, 17);
    assert_eq!(s1, 68);

    let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 32, 1000, 17);
    assert_eq!(s2, 68);

    for i in 0..32 {
        assert_eq!(tmp[i], input[i]);
    }
}

#[test]
fn test_select_bits_zero_bits() {
    // bits=0 means all values equal base
    let dummy = [0u8; 4];
    assert_eq!(forLib::for_select_bits(&dummy, 42, 0, 0), 42);
    assert_eq!(forLib::for_select_bits(&dummy, 42, 0, 5), 42);
    assert_eq!(forLib::for_select_bits(&dummy, 42, 0, 100), 42);
}

#[test]
fn test_compress_unsorted_max_range() {
    // [0, 0xFFFFFFFF] -> bits=32, size=METADATA + 8 = 13
    let input = [0u32, 0xFFFFFFFFu32];
    let mut out = vec![0u8; 32];
    let mut tmp = vec![0u32; 2];

    let s1 = forLib::for_compress_unsorted(&input, &mut out, 2);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 2);
    assert_eq!(s1, 13);
    assert_eq!(s2, 13);
    assert_eq!(out[4], 32);
    assert_eq!(tmp[0], 0);
    assert_eq!(tmp[1], 0xFFFFFFFFu32);
}

#[test]
fn test_linear_search_not_found() {
    let input: [u32; 5] = [10, 20, 30, 40, 50];
    let mut out = vec![0u8; 64];
    let _ = forLib::for_compress_sorted(&input, &mut out, 5);

    assert_eq!(forLib::for_linear_search(&out, 5, 25), 5);
    assert_eq!(forLib::for_linear_search(&out, 5, 30), 2);
    assert_eq!(forLib::for_linear_search(&out, 5, 99), 5);
    assert_eq!(forLib::for_linear_search(&out, 5, 10), 0);
    assert_eq!(forLib::for_linear_search(&out, 5, 50), 4);
}

#[test]
fn test_bits_zero_all_same() {
    let input: [u32; 4] = [42, 42, 42, 42];
    let mut out = vec![0u8; 64];
    let mut tmp = vec![0u32; 4];

    let s1 = forLib::for_compress_sorted(&input, &mut out, 4);
    assert_eq!(s1, 5);
    assert_eq!(out[4], 0);

    let s2 = forLib::for_uncompress(&out, &mut tmp, 4);
    assert_eq!(s2, 5);
    for i in 0..4 {
        assert_eq!(tmp[i], 42);
    }

    // Linear search: when bits=0, returns 0 if value matches base, else length
    assert_eq!(forLib::for_linear_search(&out, 4, 42), 0);
    assert_eq!(forLib::for_linear_search(&out, 4, 99), 4);

    // lower_bound
    let mut actual = 0u32;
    let idx = forLib::for_lower_bound_search(&out, 4, 42, &mut actual);
    assert_eq!(idx, 0);
    assert_eq!(actual, 42);
}

#[test]
fn test_lower_bound_edge_cases() {
    let input: [u32; 5] = [10, 20, 30, 40, 50];
    let mut out = vec![0u8; 64];
    let _ = forLib::for_compress_sorted(&input, &mut out, 5);

    // From running C: lb(5)=0,actual=10; lb(15)=1,actual=20;
    // lb(25)=2,actual=30; lb(35)=3,actual=40; lb(45)=4,actual=50;
    // lb(55)=4,actual=50
    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 5, &mut actual), 0);
    assert_eq!(actual, 10);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 10, &mut actual), 0);
    assert_eq!(actual, 10);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 15, &mut actual), 1);
    assert_eq!(actual, 20);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 20, &mut actual), 1);
    assert_eq!(actual, 20);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 25, &mut actual), 2);
    assert_eq!(actual, 30);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 50, &mut actual), 4);
    assert_eq!(actual, 50);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 5, 55, &mut actual), 4);
    assert_eq!(actual, 50);
}

#[test]
fn test_lower_bound_length_1() {
    let input = [42u32];
    let mut out = vec![0u8; 64];
    let _ = forLib::for_compress_sorted(&input, &mut out, 1);

    // From C: lb(42)=0 actual=42; lb(30)=0 actual=42; lb(99)=0 actual=42
    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 1, 42, &mut actual), 0);
    assert_eq!(actual, 42);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 1, 30, &mut actual), 0);
    assert_eq!(actual, 42);

    let mut actual = 0;
    assert_eq!(forLib::for_lower_bound_search(&out, 1, 99, &mut actual), 0);
    assert_eq!(actual, 42);
}

#[test]
fn test_append_sorted_size_progression() {
    // From C: append_sorted with values 0,2,4,6,...,98 (i*2)
    let expected_sizes: [u32; 50] = [
        5, 6, 7, 7, 8, 8, 9, 9, 11, 12, 12, 13, 14, 14, 15, 15, 18, 19, 20, 20, 21, 22, 23, 23,
        24, 25, 26, 26, 27, 28, 29, 29, 34, 35, 36, 37, 38, 39, 40, 40, 41, 42, 43, 44, 45, 46,
        47, 47, 48, 49,
    ];
    let mut out = vec![0u8; 2048];

    for i in 0..50u32 {
        let s = forLib::for_append_sorted(&mut out, i, i * 2);
        assert_eq!(
            s, expected_sizes[i as usize],
            "size mismatch at append i={} val={}",
            i,
            i * 2
        );
    }

    // verify final select values
    for i in 0..50u32 {
        assert_eq!(forLib::for_select(&out, i), i * 2);
    }
}

#[test]
fn test_append_unsorted_with_reencode() {
    // From C ground truth:
    // values: 10, 12, 11, 13, 100
    // expected sizes: 5, 6, 6, 6, 10
    let vals: [u32; 5] = [10, 12, 11, 13, 100];
    let expected: [u32; 5] = [5, 6, 6, 6, 10];
    let mut out = vec![0u8; 2048];

    for i in 0..5u32 {
        let s = forLib::for_append_unsorted(&mut out, i, vals[i as usize]);
        assert_eq!(s, expected[i as usize], "size mismatch i={}", i);
    }

    for i in 0..5 {
        assert_eq!(forLib::for_select(&out, i as u32), vals[i]);
    }
}

#[test]
fn test_append_sorted_small() {
    // Append values 100..107 sequentially.
    // Expected sizes from C: 5, 6, 6, 6, 7, 8, 8, 8
    let expected: [u32; 8] = [5, 6, 6, 6, 7, 8, 8, 8];
    let mut out = vec![0u8; 256];

    for i in 0..8u32 {
        let s = forLib::for_append_sorted(&mut out, i, 100 + i);
        assert_eq!(s, expected[i as usize], "size mismatch at i={}", i);
    }

    for i in 0..8 {
        assert_eq!(forLib::for_select(&out, i), 100 + i);
    }
}

#[test]
fn test_append_bits_low_level() {
    // Build 5 values with 4 bits each, base=10.
    // From C: after compress 5 ints (4 bits) size = 3.
    // Then append 15: size=3, append 16: size=4, append 17: size=4
    let in_arr: [u32; 5] = [10, 11, 12, 13, 14];
    let mut out = vec![0u8; 32];

    let s1 = forLib::for_compress_bits(&in_arr, &mut out, 5, 10, 4);
    assert_eq!(s1, 3);

    let new_sizes: [u32; 3] = [3, 4, 4];
    for (idx, val) in [15u32, 16, 17].iter().enumerate() {
        let s = forLib::for_append_bits(&mut out, 5 + idx as u32, 10, 4, *val);
        assert_eq!(s, new_sizes[idx], "append at idx={}, val={}", idx, val);
    }

    // verify by selecting
    for i in 0..8u32 {
        assert_eq!(forLib::for_select_bits(&out, 10, 4, i), 10 + i);
    }
}

#[test]
fn test_append_impl_direct() {
    // Test for_append_impl directly using for_compress_sorted as the impl.
    // length=0 -> compresses [value] alone.
    let mut out = vec![0u8; 256];
    let s = forLib::for_append_impl(&mut out, 0, 33, forLib::for_compress_sorted);
    // size = METADATA + size_for_1 = 5 + 0 = 5 (bits=0 since only one value)
    assert_eq!(s, 5);
    assert_eq!(forLib::for_select(&out, 0), 33);

    // Now append 34
    let s = forLib::for_append_impl(&mut out, 1, 34, forLib::for_compress_sorted);
    // result is at most a couple of bytes greater
    assert_eq!(s, 6);
    assert_eq!(forLib::for_select(&out, 0), 33);
    assert_eq!(forLib::for_select(&out, 1), 34);
}

#[test]
fn test_select_consistency_with_uncompress() {
    let input: Vec<u32> = (0..100).map(|i| 1000 + i * 7).collect();
    let mut out = vec![0u8; 1024];
    let mut tmp = vec![0u32; 100];

    forLib::for_compress_sorted(&input, &mut out, 100);
    forLib::for_uncompress(&out, &mut tmp, 100);

    for i in 0..100 {
        assert_eq!(tmp[i], input[i]);
        assert_eq!(forLib::for_select(&out, i as u32), input[i]);
    }
}

#[test]
fn test_linear_search_bits_explicit() {
    // 32 values, 4 bits, base=10
    let input: Vec<u32> = (0..32).map(|i| 10 + (i % 15)).collect();
    let mut out = vec![0u8; 64];
    forLib::for_compress_bits(&input, &mut out, 32, 10, 4);

    // Searching for 10..24 returns the first index of that value (0..14)
    for v in 10..=24u32 {
        assert_eq!(
            forLib::for_linear_search_bits(&out, 32, 10, 4, v),
            (v - 10),
            "search for {}",
            v
        );
    }
    // Not found
    assert_eq!(forLib::for_linear_search_bits(&out, 32, 10, 4, 99), 32);
}

#[test]
fn test_linear_search_bits_zero_bits() {
    // bits == 0: only base is stored; if value == base and length > 0 -> 0,
    // else returns length.
    let dummy = [0u8; 0];
    assert_eq!(forLib::for_linear_search_bits(&dummy, 4, 42, 0, 42), 0);
    assert_eq!(forLib::for_linear_search_bits(&dummy, 4, 42, 0, 100), 4);
    assert_eq!(forLib::for_linear_search_bits(&dummy, 0, 42, 0, 42), 0);
}

#[test]
fn test_lower_bound_search_bits_explicit() {
    let input: [u32; 5] = [10, 20, 30, 40, 50];
    let mut out = vec![0u8; 64];
    forLib::for_compress_bits(&input, &mut out, 5, 10, 6);

    let mut actual = 0;
    assert_eq!(
        forLib::for_lower_bound_search_bits(&out, 5, 10, 6, 25, &mut actual),
        2
    );
    assert_eq!(actual, 30);

    let mut actual = 0;
    assert_eq!(
        forLib::for_lower_bound_search_bits(&out, 5, 10, 6, 50, &mut actual),
        4
    );
    assert_eq!(actual, 50);
}

#[test]
fn test_metadata_constant() {
    assert_eq!(forLib::METADATA, 5);
}

fn main() {}
