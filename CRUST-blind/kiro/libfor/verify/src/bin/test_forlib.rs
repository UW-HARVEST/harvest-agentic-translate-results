use libfor::forLib;

// Port of the C test's rnd() function (static state)
struct Rnd { a: u32 }
impl Rnd {
    fn new() -> Self { Rnd { a: 3 } }
    fn next(&mut self) -> u32 {
        self.a = ((self.a.wrapping_mul(214013).wrapping_add(2531011)) >> 16) & 32767;
        self.a
    }
}

fn generate_input(base: u32, length: u32, bits: u32) -> Vec<u32> {
    let max = if bits == 0 { 1 } else if bits == 32 { 0 } else { (1u32 << bits) - 1 };
    (0..length).map(|i| {
        if bits == 0 { base }
        else if bits == 32 { base + i }
        else { base + (i % max) }
    }).collect()
}

// ===== required_bits =====
#[test]
fn test_required_bits() {
    assert_eq!(forLib::required_bits(0), 0);
    assert_eq!(forLib::required_bits(1), 1);
    assert_eq!(forLib::required_bits(2), 2);
    assert_eq!(forLib::required_bits(3), 2);
    assert_eq!(forLib::required_bits(4), 3);
    assert_eq!(forLib::required_bits(255), 8);
    assert_eq!(forLib::required_bits(256), 9);
    assert_eq!(forLib::required_bits(u32::MAX), 32);
}

// ===== for_compressed_size_bits =====
#[test]
fn test_compressed_size_bits() {
    assert_eq!(forLib::for_compressed_size_bits(0, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(0, 5), 0);
    assert_eq!(forLib::for_compressed_size_bits(32, 0), 0);
    assert_eq!(forLib::for_compressed_size_bits(32, 1), 4);  // 32*1=32 bits = 4 bytes
    assert_eq!(forLib::for_compressed_size_bits(32, 8), 32); // 32*8=256 bits = 32 bytes
    assert_eq!(forLib::for_compressed_size_bits(32, 32), 128);
    assert_eq!(forLib::for_compressed_size_bits(1, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(1, 32), 4);
    assert_eq!(forLib::for_compressed_size_bits(33, 1), 4 + 1); // 32 block + 1 remainder
    assert_eq!(forLib::for_compressed_size_bits(48, 1), 6); // 32+16 = 4+2
    assert_eq!(forLib::for_compressed_size_bits(56, 1), 7); // 32+16+8 = 4+2+1
    assert_eq!(forLib::for_compressed_size_bits(57, 1), 8); // 32+16+8+1 = 4+2+1+1
}

// ===== for_compressed_size_sorted =====
#[test]
fn test_compressed_size_sorted() {
    assert_eq!(forLib::for_compressed_size_sorted(&[], 0), 0);
    let data: Vec<u32> = (33..33 + 16).collect();
    let s = forLib::for_compressed_size_sorted(&data, 16);
    // range = 15, bits = 4, size_bits(16,4) = (16*4+7)/8 = 8, + 5 metadata = 13
    assert_eq!(s, 13);
}

// ===== for_compressed_size_unsorted =====
#[test]
fn test_compressed_size_unsorted() {
    assert_eq!(forLib::for_compressed_size_unsorted(&[], 0), 0);
    let data = vec![10u32, 5, 15, 8];
    let s = forLib::for_compressed_size_unsorted(&data, 4);
    // min=5, max=15, range=10, bits=4, size_bits(4,4) = (4*4+7)/8 = 2+1=3 -> wait: (16+7)/8=2
    // + 5 metadata = 7
    assert_eq!(s, 7);
}

// ===== compress/uncompress roundtrip sorted =====
#[test]
fn test_compress_uncompress_sorted() {
    for &length in &[0u32, 1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257, 1024, 1025] {
        if length == 0 {
            let s = forLib::for_compress_sorted(&[], &mut [], 0);
            assert_eq!(s, 0);
            continue;
        }
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        let s1 = forLib::for_compress_sorted(&input, &mut out, length);
        let s3 = forLib::for_compressed_size_sorted(&input, length);
        assert_eq!(s1, s3, "compress size mismatch for length={}", length);
        let mut tmp = vec![0u32; length as usize];
        let s2 = forLib::for_uncompress(&out, &mut tmp, length);
        assert_eq!(s1, s2, "uncompress size mismatch for length={}", length);
        assert_eq!(input, tmp, "data mismatch for length={}", length);
    }
}

// ===== compress/uncompress roundtrip unsorted =====
#[test]
fn test_compress_uncompress_unsorted() {
    let mut rng = Rnd::new();
    for &length in &[0u32, 1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257, 1024, 1025] {
        if length == 0 {
            let s = forLib::for_compress_unsorted(&[], &mut [], 0);
            assert_eq!(s, 0);
            continue;
        }
        let input: Vec<u32> = (0..length).map(|_| 7u32.wrapping_add(rng.next()).wrapping_sub(7)).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        let s1 = forLib::for_compress_unsorted(&input, &mut out, length);
        let s3 = forLib::for_compressed_size_unsorted(&input, length);
        assert_eq!(s1, s3, "compress size mismatch for length={}", length);
        let mut tmp = vec![0u32; length as usize];
        let s2 = forLib::for_uncompress(&out, &mut tmp, length);
        assert_eq!(s1, s2, "uncompress size mismatch for length={}", length);
        assert_eq!(input, tmp, "data mismatch for length={}", length);
    }
}

// ===== for_select on sorted data =====
#[test]
fn test_select_sorted() {
    for &length in &[1u32, 16, 33, 64, 129, 256] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        forLib::for_compress_sorted(&input, &mut out, length);
        for i in 0..length {
            assert_eq!(forLib::for_select(&out, i), input[i as usize],
                "select mismatch at index {} for length={}", i, length);
        }
    }
}

// ===== for_select on unsorted data =====
#[test]
fn test_select_unsorted() {
    let mut rng = Rnd::new();
    for &length in &[1u32, 16, 33, 64] {
        let input: Vec<u32> = (0..length).map(|_| rng.next()).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        forLib::for_compress_unsorted(&input, &mut out, length);
        for i in 0..length {
            assert_eq!(forLib::for_select(&out, i), input[i as usize]);
        }
    }
}

// ===== for_linear_search on sorted data =====
#[test]
fn test_linear_search_sorted() {
    for &length in &[1u32, 16, 33, 64, 129] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        forLib::for_compress_sorted(&input, &mut out, length);
        for i in 0..length {
            let idx = forLib::for_linear_search(&out, length, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize]);
        }
        // search for value not present
        let idx = forLib::for_linear_search(&out, length, 9999);
        assert_eq!(idx, length);
    }
}

// ===== for_linear_search on unsorted data =====
#[test]
fn test_linear_search_unsorted() {
    let mut rng = Rnd::new();
    for &length in &[1u32, 16, 33, 64] {
        let input: Vec<u32> = (0..length).map(|_| rng.next()).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        forLib::for_compress_unsorted(&input, &mut out, length);
        for i in 0..length {
            let idx = forLib::for_linear_search(&out, length, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize]);
        }
    }
}

// ===== for_lower_bound_search =====
#[test]
fn test_lower_bound_search() {
    for &length in &[2u32, 16, 33, 64, 129] {
        let input: Vec<u32> = (0..length).map(|i| 33 + i).collect();
        let mut out = vec![0u8; (length as usize) * 8 + 64];
        forLib::for_compress_sorted(&input, &mut out, length);
        for i in 0..length {
            let mut actual = 0u32;
            let idx = forLib::for_lower_bound_search(&out, length, input[i as usize], &mut actual);
            assert_eq!(input[idx as usize], input[i as usize]);
            assert_eq!(actual, input[i as usize]);
        }
    }
}

// ===== for_select_bits / for_linear_search_bits low-level =====
#[test]
fn test_lowlevel_pack_unpack_roundtrip() {
    for bits in 0..=32u32 {
        let input = generate_input(10, 32, bits);
        let mut out = vec![0u8; 1024];
        let mut tmp = vec![0u32; 1024];
        let s1 = forLib::for_compress_bits(&input, &mut out, 32, 10, bits);
        let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 32, 10, bits);
        assert_eq!(s1, s2, "bits={}", bits);
        assert_eq!(&input[..32], &tmp[..32], "bits={}", bits);

        // select_bits
        for i in 0..32u32 {
            assert_eq!(forLib::for_select_bits(&out, 10, bits, i), input[i as usize], "bits={} i={}", bits, i);
        }

        // linear_search_bits
        for i in 0..32u32 {
            let idx = forLib::for_linear_search_bits(&out, 32, 10, bits, input[i as usize]);
            assert_eq!(input[idx as usize], input[i as usize], "bits={} i={}", bits, i);
        }
    }
}

#[test]
fn test_lowlevel_pack16() {
    for bits in 0..=32u32 {
        let input = generate_input(10, 16, bits);
        let mut out = vec![0u8; 1024];
        let mut tmp = vec![0u32; 1024];
        let s1 = forLib::for_compress_bits(&input, &mut out, 16, 10, bits);
        let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 16, 10, bits);
        assert_eq!(s1, s2, "bits={}", bits);
        assert_eq!(&input[..16], &tmp[..16], "bits={}", bits);
    }
}

#[test]
fn test_lowlevel_pack8() {
    for bits in 0..=32u32 {
        let input = generate_input(10, 8, bits);
        let mut out = vec![0u8; 1024];
        let mut tmp = vec![0u32; 1024];
        let s1 = forLib::for_compress_bits(&input, &mut out, 8, 10, bits);
        let s2 = forLib::for_uncompress_bits(&out, &mut tmp, 8, 10, bits);
        assert_eq!(s1, s2, "bits={}", bits);
        assert_eq!(&input[..8], &tmp[..8], "bits={}", bits);
    }
}

#[test]
fn test_lowlevel_packx_variable_lengths() {
    for bits in 0..32u32 {
        for len in 0..8u32 {
            let input = generate_input(10, 8, bits);
            let mut out = vec![0u8; 1024];
            let mut tmp = vec![0u32; 1024];
            let s1 = forLib::for_compress_bits(&input[..len as usize], &mut out, len, 10, bits);
            let s2 = forLib::for_uncompress_bits(&out, &mut tmp, len, 10, bits);
            assert_eq!(s1, s2, "bits={} len={}", bits, len);
            for i in 0..len as usize {
                assert_eq!(input[i], tmp[i], "bits={} len={} i={}", bits, len, i);
            }
        }
    }
}

// ===== for_append_sorted =====
#[test]
fn test_append_sorted() {
    let max = 500;
    let mut out1 = vec![0u8; max * 8];
    let mut out2 = vec![0u8; max * 8];
    let mut input = Vec::new();
    for i in 0..max {
        input.push(i as u32);
        let s1 = forLib::for_append_sorted(&mut out1, i as u32, i as u32);
        let s2 = forLib::for_compress_sorted(&input, &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "i={}", i);
    }
}

// ===== for_append_unsorted =====
#[test]
fn test_append_unsorted() {
    let max = 500;
    let mut rng = Rnd::new();
    let mut out1 = vec![0u8; max * 8];
    let mut out2 = vec![0u8; max * 8];
    let mut input = Vec::new();
    for i in 0..max {
        let v = 7u32.wrapping_add(rng.next()).wrapping_sub(7);
        input.push(v);
        let s1 = forLib::for_append_unsorted(&mut out1, i as u32, v);
        let s2 = forLib::for_compress_unsorted(&input, &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "i={}", i);
    }
}

// ===== for_append_sorted with big numbers (re-encoding path) =====
#[test]
fn test_append_sorted_bignum() {
    let max = 10;
    let mut out1 = vec![0u8; max * 8];
    let mut out2 = vec![0u8; max * 8];
    let mut input = Vec::new();
    for i in 0..max {
        let v = 1u32 << (17 + i);
        input.push(v);
        let s1 = forLib::for_append_sorted(&mut out1, i as u32, v);
        let s2 = forLib::for_compress_sorted(&input, &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "i={}", i);
        assert_eq!(&out1[..s1 as usize], &out2[..s2 as usize], "i={}", i);
    }
}

// ===== for_append_bits =====
#[test]
fn test_append_bits_basic() {
    // Compress a sequence, then append one more using for_append_bits
    let input = vec![10u32, 11, 12, 13];
    let mut out = vec![0u8; 256];
    let s1 = forLib::for_compress_bits(&input, &mut out, 4, 10, 4);
    // Now append value 14 (base=10, bits=4, index=4)
    let s2 = forLib::for_append_bits(&mut out, 4, 10, 4, 14);
    // Verify by uncompressing 5 values
    let mut tmp = vec![0u32; 5];
    forLib::for_uncompress_bits(&out, &mut tmp, 5, 10, 4);
    assert_eq!(tmp, vec![10, 11, 12, 13, 14]);
    // s2 should equal the compressed size of 5 values at 4 bits
    assert_eq!(s2, forLib::for_compressed_size_bits(5, 4));
}

// ===== for_linear_search_bits with 0 bits =====
#[test]
fn test_linear_search_bits_zero_bits() {
    let out = vec![0u8; 64];
    // All values are base when bits=0
    assert_eq!(forLib::for_linear_search_bits(&out, 10, 42, 0, 42), 0);
    assert_eq!(forLib::for_linear_search_bits(&out, 10, 42, 0, 99), 10);
}

// ===== for_lower_bound_search_bits =====
#[test]
fn test_lower_bound_search_bits() {
    // Sorted sequence: 100, 101, 102, ..., 109
    let input: Vec<u32> = (100..110).collect();
    let mut out = vec![0u8; 256];
    forLib::for_compress_bits(&input, &mut out, 10, 100, 4);
    // Search for 105
    let mut actual = 0u32;
    let idx = forLib::for_lower_bound_search_bits(&out, 10, 100, 4, 105, &mut actual);
    assert_eq!(actual, 105);
    assert_eq!(idx, 5);
    // Search for value between elements (e.g. 103 should find 103)
    let idx2 = forLib::for_lower_bound_search_bits(&out, 10, 100, 4, 103, &mut actual);
    assert_eq!(actual, 103);
    assert_eq!(idx2, 3);
}

// ===== for_select_bits with 32 bits =====
#[test]
fn test_select_bits_32() {
    let input: Vec<u32> = (0..10).map(|i| 100 + i).collect();
    let mut out = vec![0u8; 256];
    forLib::for_compress_bits(&input, &mut out, 10, 100, 32);
    for i in 0..10u32 {
        assert_eq!(forLib::for_select_bits(&out, 100, 32, i), 100 + i);
    }
}

// ===== all-same values (0 bits) =====
#[test]
fn test_all_same_values() {
    let input = vec![42u32; 64];
    let mut out = vec![0u8; 256];
    let s1 = forLib::for_compress_sorted(&input, &mut out, 64);
    // bits should be 0, so compressed size = METADATA + 0
    assert_eq!(s1, 5);
    let mut tmp = vec![0u32; 64];
    let s2 = forLib::for_uncompress(&out, &mut tmp, 64);
    assert_eq!(s1, s2);
    assert_eq!(input, tmp);
    for i in 0..64 {
        assert_eq!(forLib::for_select(&out, i), 42);
    }
    assert_eq!(forLib::for_linear_search(&out, 64, 42), 0);
    assert_eq!(forLib::for_linear_search(&out, 64, 99), 64);
}

// ===== single element =====
#[test]
fn test_single_element() {
    let input = vec![777u32];
    let mut out = vec![0u8; 64];
    let s1 = forLib::for_compress_sorted(&input, &mut out, 1);
    assert_eq!(s1, 5); // METADATA only, 0 bits needed
    assert_eq!(forLib::for_select(&out, 0), 777);
    let mut tmp = vec![0u32; 1];
    forLib::for_uncompress(&out, &mut tmp, 1);
    assert_eq!(tmp[0], 777);
}

fn main() {}
