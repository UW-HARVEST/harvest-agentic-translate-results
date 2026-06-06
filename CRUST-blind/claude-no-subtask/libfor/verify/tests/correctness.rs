use libfor::forLib::*;

fn rnd_iter() -> impl FnMut() -> u32 {
    let mut a: u32 = 3;
    move || {
        a = a.wrapping_mul(214013).wrapping_add(2531011);
        (a >> 16) & 32767
    }
}

#[test]
fn highlevel_sorted() {
    for &length in &[0u32, 1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257, 1024, 1025, 1333] {
        let mut input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut output = vec![0u8; (length as usize + 16) * 8];
        let mut tmp = vec![0u32; length as usize + 16];

        let s3 = for_compressed_size_sorted(&input, length);
        let s1 = for_compress_sorted(&input, &mut output, length);
        let s2 = for_uncompress(&output, &mut tmp, length);
        assert_eq!(s1, s2, "size mismatch len={}", length);
        assert_eq!(s2, s3, "compressed_size mismatch len={}", length);
        for i in 0..length as usize {
            assert_eq!(tmp[i], input[i], "value mismatch at {} len={}", i, length);
        }

        for i in 0..length {
            let v = for_select(&output, i);
            assert_eq!(v, input[i as usize], "select mismatch at {} len={}", i, length);
        }

        for i in 0..length {
            let idx = for_linear_search(&output, length, input[i as usize]);
            assert_eq!(idx, i, "linear_search mismatch len={}, val={}", length, input[i as usize]);
        }

        for i in 0..length {
            let mut actual: u32 = 0;
            let idx = for_lower_bound_search(&output, length, input[i as usize], &mut actual);
            assert_eq!(input[i as usize], input[idx as usize]);
            assert_eq!(actual, input[i as usize]);
        }

        // Mark unused arrays to silence warnings.
        let _ = (&mut input, &mut output, &mut tmp);
    }
}

#[test]
fn highlevel_unsorted() {
    let mut rnd = rnd_iter();
    for &length in &[0u32, 1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257, 1024, 1025, 1333] {
        let input: Vec<u32> = (0..length).map(|_| 7 + rnd().wrapping_sub(7)).collect();
        let mut output = vec![0u8; (length as usize + 16) * 8];
        let mut tmp = vec![0u32; length as usize + 16];

        let s3 = for_compressed_size_unsorted(&input, length);
        let s1 = for_compress_unsorted(&input, &mut output, length);
        let s2 = for_uncompress(&output, &mut tmp, length);
        assert_eq!(s1, s2, "size mismatch len={}", length);
        assert_eq!(s2, s3, "compressed_size mismatch len={}", length);
        for i in 0..length as usize {
            assert_eq!(tmp[i], input[i], "value mismatch at {}, len={}", i, length);
        }

        for i in 0..length {
            let v = for_select(&output, i);
            assert_eq!(v, input[i as usize], "select mismatch at {} len={}", i, length);
        }

        for i in 0..length {
            let idx = for_linear_search(&output, length, input[i as usize]);
            assert_eq!(input[i as usize], input[idx as usize]);
        }
    }
}

#[test]
fn append_sorted_test() {
    const N: usize = 1000;
    let mut out1 = vec![0u8; N * 8];
    let mut out2 = vec![0u8; N * 8];
    let mut input = vec![0u32; N];

    for i in 0..N {
        input[i] = i as u32;
        let s1 = for_append_sorted(&mut out1, i as u32, input[i]);
        let s2 = for_compress_sorted(&input[..i + 1], &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "size mismatch at i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "data mismatch at i={}", i);
    }
}

#[test]
fn append_unsorted_test() {
    let mut rnd = rnd_iter();
    const N: usize = 1000;
    let mut out1 = vec![0u8; N * 8];
    let mut out2 = vec![0u8; N * 8];
    let mut input = vec![0u32; N];

    for i in 0..N {
        input[i] = 7 + rnd().wrapping_sub(7);
        let s1 = for_append_unsorted(&mut out1, i as u32, input[i]);
        let s2 = for_compress_unsorted(&input[..i + 1], &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "size mismatch at i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "data mismatch at i={}", i);
    }
}

#[test]
fn append_sorted_bignum() {
    const N: usize = 10;
    let mut out1 = vec![0u8; N * 8];
    let mut out2 = vec![0u8; N * 8];
    let mut input = vec![0u32; N];

    for i in 0..N {
        input[i] = 1u32 << (17 + i);
        let s1 = for_append_sorted(&mut out1, i as u32, input[i]);
        let s2 = for_compress_sorted(&input[..i + 1], &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "size mismatch at i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "data mismatch at i={}", i);
    }
}
