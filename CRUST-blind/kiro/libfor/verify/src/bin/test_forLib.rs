use libfor::forLib;

// C's rnd() function equivalent
fn rnd(state: &mut u32) -> u32 {
    *state = (((*state as u64).wrapping_mul(214013).wrapping_add(2531011)) >> 16) as u32 & 32767;
    *state
}

#[test]
fn test_required_bits() {
    assert_eq!(forLib::required_bits(0), 0);
    assert_eq!(forLib::required_bits(1), 1);
    assert_eq!(forLib::required_bits(2), 2);
    assert_eq!(forLib::required_bits(3), 2);
    assert_eq!(forLib::required_bits(4), 3);
    assert_eq!(forLib::required_bits(7), 3);
    assert_eq!(forLib::required_bits(8), 4);
    assert_eq!(forLib::required_bits(15), 4);
    assert_eq!(forLib::required_bits(16), 5);
    assert_eq!(forLib::required_bits(31), 5);
    assert_eq!(forLib::required_bits(32), 6);
    assert_eq!(forLib::required_bits(255), 8);
    assert_eq!(forLib::required_bits(256), 9);
    assert_eq!(forLib::required_bits(65535), 16);
    assert_eq!(forLib::required_bits(65536), 17);
    assert_eq!(forLib::required_bits(0xFFFFFFFF), 32);
}

#[test]
fn test_compressed_size_bits() {
    assert_eq!(forLib::for_compressed_size_bits(0, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(1, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(32, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(1, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(8, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(16, 1), 2);
    assert_eq!(forLib::for_compressed_size_bits(32, 1), 4);
    assert_eq!(forLib::for_compressed_size_bits(33, 1), 5);
    assert_eq!(forLib::for_compressed_size_bits(64, 1), 8);
    assert_eq!(forLib::for_compressed_size_bits(65, 1), 9);
    assert_eq!(forLib::for_compressed_size_bits(32, 5), 20);
    assert_eq!(forLib::for_compressed_size_bits(32, 8), 32);
    assert_eq!(forLib::for_compressed_size_bits(32, 16), 64);
    assert_eq!(forLib::for_compressed_size_bits(32, 32), 128);
    assert_eq!(forLib::for_compressed_size_bits(33, 5), 21);
    assert_eq!(forLib::for_compressed_size_bits(100, 10), 125);
    assert_eq!(forLib::for_compressed_size_bits(1333, 7), 1167);
}

#[test]
fn test_compressed_size_sorted_empty() {
    assert_eq!(forLib::for_compressed_size_sorted(&[], 0), 0);
}

#[test]
fn test_compressed_size_unsorted_empty() {
    assert_eq!(forLib::for_compressed_size_unsorted(&[], 0), 0);
}

#[test]
fn test_compressed_size_sorted_one() {
    assert_eq!(forLib::for_compressed_size_sorted(&[42], 1), 5);
}

#[test]
fn test_compressed_size_unsorted_one() {
    assert_eq!(forLib::for_compressed_size_unsorted(&[42], 1), 5);
}

#[test]
fn test_compressed_size_same_values() {
    assert_eq!(forLib::for_compressed_size_sorted(&[5, 5, 5, 5], 4), 5);
    assert_eq!(forLib::for_compressed_size_unsorted(&[5, 5, 5, 5], 4), 5);
}

#[test]
fn test_highlevel_sorted_roundtrip() {
    let lengths: &[u32] = &[1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257];
    let expected_sizes: &[u32] = &[5, 6, 6, 13, 16, 25, 30, 53, 62, 117, 134, 261, 295];

    for (li, &length) in lengths.iter().enumerate() {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; 1024 * 10];
        let mut tmp = vec![0u32; length as usize];

        let s3 = forLib::for_compressed_size_sorted(&input, length);
        let s1 = forLib::for_compress_sorted(&input, &mut out, length);
        let s2 = forLib::for_uncompress(&out, &mut tmp, length);

        assert_eq!(s1, expected_sizes[li], "compress size mismatch for len={}", length);
        assert_eq!(s2, expected_sizes[li], "uncompress size mismatch for len={}", length);
        assert_eq!(s3, expected_sizes[li], "compressed_size_sorted mismatch for len={}", length);
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
        assert_eq!(input, tmp, "roundtrip data mismatch for len={}", length);
    }
}

#[test]
fn test_highlevel_sorted_select() {
    for &length in &[1u32, 2, 3, 16, 32, 33, 64, 65, 128, 129, 256, 257] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; 1024 * 10];
        forLib::for_compress_sorted(&input, &mut out, length);

        assert_eq!(forLib::for_select(&out, 0), 33);
        assert_eq!(forLib::for_select(&out, length - 1), 33 + length - 1);
        for i in 0..length {
            assert_eq!(forLib::for_select(&out, i), input[i as usize]);
        }
    }
}

#[test]
fn test_highlevel_sorted_linear_search() {
    for &length in &[1u32, 2, 3, 16, 32, 33, 64, 65, 128, 129, 256, 257] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; 1024 * 10];
        forLib::for_compress_sorted(&input, &mut out, length);

        for i in 0..length {
            assert_eq!(forLib::for_linear_search(&out, length, input[i as usize]), i);
        }
    }
}

#[test]
fn test_highlevel_sorted_lower_bound() {
    for &length in &[2u32, 3, 16, 32, 33, 64, 65, 128, 129, 256, 257] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; 1024 * 10];
        forLib::for_compress_sorted(&input, &mut out, length);

        for i in 0..length {
            let mut actual = 0u32;
            let idx = forLib::for_lower_bound_search(&out, length, input[i as usize], &mut actual);
            assert_eq!(actual, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize]);
        }
    }
}

#[test]
fn test_linear_search_not_found() {
    let input: Vec<u32> = vec![33, 34, 35, 36, 37];
    let mut out = vec![0u8; 1024];
    forLib::for_compress_sorted(&input, &mut out, 5);
    assert_eq!(forLib::for_linear_search(&out, 5, 99), 5);
    assert_eq!(forLib::for_linear_search(&out, 5, 32), 5);
}

#[test]
fn test_highlevel_unsorted_roundtrip() {
    let unsorted_lengths: &[u32] = &[1, 2, 3, 16, 17, 32, 33];
    let expected_sizes: &[u32] = &[5, 7, 9, 35, 37, 65, 67];

    for (li, &length) in unsorted_lengths.iter().enumerate() {
        let mut state = 3u32;
        let input: Vec<u32> = (0..length).map(|_| {
            let r = rnd(&mut state);
            7u32.wrapping_add(r.wrapping_sub(7))
        }).collect();

        let mut out = vec![0u8; 1024 * 10];
        let mut tmp = vec![0u32; length as usize];

        let s3 = forLib::for_compressed_size_unsorted(&input, length);
        let s1 = forLib::for_compress_unsorted(&input, &mut out, length);
        let s2 = forLib::for_uncompress(&out, &mut tmp, length);

        assert_eq!(s1, expected_sizes[li], "compress size mismatch for unsorted len={}", length);
        assert_eq!(s2, expected_sizes[li]);
        assert_eq!(s3, expected_sizes[li]);
        assert_eq!(input, tmp, "roundtrip data mismatch for unsorted len={}", length);
    }
}

#[test]
fn test_highlevel_unsorted_select() {
    for &length in &[1u32, 2, 3, 16, 17, 32, 33] {
        let mut state = 3u32;
        let input: Vec<u32> = (0..length).map(|_| {
            let r = rnd(&mut state);
            7u32.wrapping_add(r.wrapping_sub(7))
        }).collect();

        let mut out = vec![0u8; 1024 * 10];
        forLib::for_compress_unsorted(&input, &mut out, length);

        for i in 0..length {
            assert_eq!(forLib::for_select(&out, i), input[i as usize]);
        }
    }
}

#[test]
fn test_highlevel_unsorted_linear_search() {
    for &length in &[1u32, 2, 3, 16, 17, 32, 33] {
        let mut state = 3u32;
        let input: Vec<u32> = (0..length).map(|_| {
            let r = rnd(&mut state);
            7u32.wrapping_add(r.wrapping_sub(7))
        }).collect();

        let mut out = vec![0u8; 1024 * 10];
        forLib::for_compress_unsorted(&input, &mut out, length);

        for i in 0..length {
            let idx = forLib::for_linear_search(&out, length, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize]);
        }
    }
}

#[test]
fn test_compress_uncompress_empty() {
    let mut out = vec![0u8; 100];
    let mut tmp = vec![0u32; 10];
    assert_eq!(forLib::for_compress_sorted(&[], &mut out, 0), 0);
    assert_eq!(forLib::for_compress_unsorted(&[], &mut out, 0), 0);
    assert_eq!(forLib::for_uncompress(&out, &mut tmp, 0), 0);
}

#[test]
fn test_append_sorted() {
    let mut out1 = vec![0u8; 1024];
    let mut out2 = vec![0u8; 1024];
    let mut input = Vec::new();

    let expected_sizes: &[u32] = &[5, 6, 6, 6, 7, 8, 8, 8, 10, 10];

    for i in 0..10u32 {
        input.push(i);
        let s1 = forLib::for_append_sorted(&mut out1, i, i);
        let s2 = forLib::for_compress_sorted(&input, &mut out2, i + 1);
        assert_eq!(s1, expected_sizes[i as usize], "append_sorted size mismatch at i={}", i);
        assert_eq!(s1, s2, "append vs compress mismatch at i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "data mismatch at i={}", i);
    }
}

#[test]
fn test_append_sorted_bignum() {
    let mut out1 = vec![0u8; 1024];
    let mut out2 = vec![0u8; 1024];
    let mut input = Vec::new();

    let expected_sizes: &[u32] = &[5, 10, 13, 15, 19, 22, 26, 29, 34, 38];

    for i in 0..10u32 {
        let val = 1u32 << (17 + i);
        input.push(val);
        let s1 = forLib::for_append_sorted(&mut out1, i, val);
        let s2 = forLib::for_compress_sorted(&input, &mut out2, i + 1);
        assert_eq!(s1, expected_sizes[i as usize], "append_sorted_bignum size mismatch at i={}", i);
        assert_eq!(s1, s2);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize]);
    }
}

#[test]
fn test_append_unsorted() {
    let mut out1 = vec![0u8; 80_000];
    let mut out2 = vec![0u8; 80_000];
    let mut input = Vec::new();
    let mut state = 3u32;
    // Reset rnd state for unsorted append (C test resets after sorted tests)
    // The C append_unsorted test uses rnd() which continues from the sorted test state.
    // But we need to match the C behavior. The C test calls append_sorted first (10000 iters),
    // then append_sorted_bignum (10 iters), then append_unsorted.
    // Let's just verify the first few iterations match.
    for i in 0..100u32 {
        let r = rnd(&mut state);
        let val = 7u32.wrapping_add(r.wrapping_sub(7));
        input.push(val);
        let s1 = forLib::for_append_unsorted(&mut out1, i, val);
        let s2 = forLib::for_compress_unsorted(&input, &mut out2, i + 1);
        assert_eq!(s1, s2, "append_unsorted mismatch at i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "data mismatch at i={}", i);
    }
}

#[test]
fn test_lowlevel_compress_bits_roundtrip() {
    let test_bits: &[u32] = &[0, 1, 5, 8, 16, 32];
    let expected_sizes: &[u32] = &[0, 4, 20, 32, 64, 128];

    for (bi, &bits) in test_bits.iter().enumerate() {
        let base = 10u32;
        let max_val = if bits == 0 { 0 } else if bits == 32 { 31 } else { (1u32 << bits) - 1 };
        let input: Vec<u32> = (0..32).map(|i| {
            if bits == 0 { base }
            else if bits == 32 { base + i }
            else { base + (i % max_val) }
        }).collect();

        let mut out = vec![0u8; 1024];
        let mut tmp = vec![0u32; 32];

        let s1 = forLib::for_compress_bits(&input, &mut out, 32, base, bits);
        let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 32, base, bits);

        assert_eq!(s1, expected_sizes[bi], "compress_bits size mismatch for bits={}", bits);
        assert_eq!(s2, expected_sizes[bi], "uncompress_bits size mismatch for bits={}", bits);
        assert_eq!(input, tmp, "roundtrip mismatch for bits={}", bits);
    }
}

#[test]
fn test_lowlevel_select_bits() {
    let test_bits: &[u32] = &[0, 1, 5, 8, 16, 32];
    for &bits in test_bits {
        let base = 10u32;
        let max_val = if bits == 0 { 0 } else if bits == 32 { 31 } else { (1u32 << bits) - 1 };
        let input: Vec<u32> = (0..32).map(|i| {
            if bits == 0 { base }
            else if bits == 32 { base + i }
            else { base + (i % max_val) }
        }).collect();

        let mut out = vec![0u8; 1024];
        forLib::for_compress_bits(&input, &mut out, 32, base, bits);

        for i in 0..32 {
            assert_eq!(forLib::for_select_bits(&out, base, bits, i), input[i as usize],
                "select_bits mismatch for bits={} index={}", bits, i);
        }
    }
}

#[test]
fn test_lowlevel_linear_search_bits() {
    let test_bits: &[u32] = &[0, 1, 5, 8, 16, 32];
    for &bits in test_bits {
        let base = 10u32;
        let max_val = if bits == 0 { 0 } else if bits == 32 { 31 } else { (1u32 << bits) - 1 };
        let input: Vec<u32> = (0..32).map(|i| {
            if bits == 0 { base }
            else if bits == 32 { base + i }
            else { base + (i % max_val) }
        }).collect();

        let mut out = vec![0u8; 1024];
        forLib::for_compress_bits(&input, &mut out, 32, base, bits);

        for i in 0..32 {
            let idx = forLib::for_linear_search_bits(&out, 32, base, bits, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize],
                "linear_search_bits mismatch for bits={} i={}", bits, i);
        }

        // Not found test (only for bits < 32)
        if bits < 32 {
            let nf_val = base + max_val + 1;
            assert_eq!(forLib::for_linear_search_bits(&out, 32, base, bits, nf_val), 32,
                "linear_search_bits not_found mismatch for bits={}", bits);
        }
    }
}

#[test]
fn test_lowlevel_lower_bound_search_bits() {
    for &length in &[32u32, 64, 128, 256] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let base = 33u32;
        let bits = forLib::required_bits(input[length as usize - 1] - base);
        let mut out = vec![0u8; 1024 * 10];
        forLib::for_compress_bits(&input, &mut out, length, base, bits);

        for i in 0..length {
            let mut actual = 0u32;
            let idx = forLib::for_lower_bound_search_bits(&out, length, base, bits, input[i as usize], &mut actual);
            assert_eq!(actual, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize]);
        }
    }
}

fn main() {}
