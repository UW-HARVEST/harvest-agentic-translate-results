use libfor::forLib;

#[test]
fn test_metadata_constant() {
    assert_eq!(forLib::METADATA, 5);
}

#[test]
fn test_required_bits() {
    // Values verified against C: required_bits is 0 for v=0, otherwise 32-clz(v).
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
    assert_eq!(forLib::required_bits(0xFFFFFFFF), 32);
    assert_eq!(forLib::required_bits(0x80000000), 32);
    assert_eq!(forLib::required_bits(0x7FFFFFFF), 31);
}

#[test]
fn test_compressed_size_bits_zero_length() {
    // Verified against C: l=0 -> 0
    for b in 0u32..=32 {
        assert_eq!(forLib::for_compressed_size_bits(0, b), 0, "bits={}", b);
    }
}

#[test]
fn test_compressed_size_bits_zero_bits() {
    // l=*, b=0 -> 0
    for l in [0u32, 1, 7, 8, 32, 100, 1000] {
        assert_eq!(forLib::for_compressed_size_bits(l, 0), 0, "len={}", l);
    }
}

#[test]
fn test_compressed_size_bits_known_values() {
    // Captured from running the C oracle.
    assert_eq!(forLib::for_compressed_size_bits(1, 1), 1);
    assert_eq!(forLib::for_compressed_size_bits(1, 13), 2);
    assert_eq!(forLib::for_compressed_size_bits(1, 32), 4);
    assert_eq!(forLib::for_compressed_size_bits(7, 3), 3);
    assert_eq!(forLib::for_compressed_size_bits(8, 7), 7);
    assert_eq!(forLib::for_compressed_size_bits(8, 8), 8);
    assert_eq!(forLib::for_compressed_size_bits(15, 3), 6);
    assert_eq!(forLib::for_compressed_size_bits(15, 13), 25);
    assert_eq!(forLib::for_compressed_size_bits(16, 13), 26);
    assert_eq!(forLib::for_compressed_size_bits(31, 3), 12);
    assert_eq!(forLib::for_compressed_size_bits(32, 32), 128);
    assert_eq!(forLib::for_compressed_size_bits(33, 5), 21);
    assert_eq!(forLib::for_compressed_size_bits(33, 13), 54);
    assert_eq!(forLib::for_compressed_size_bits(47, 8), 47);
    assert_eq!(forLib::for_compressed_size_bits(48, 13), 78);
    assert_eq!(forLib::for_compressed_size_bits(63, 16), 126);
    assert_eq!(forLib::for_compressed_size_bits(64, 16), 128);
    assert_eq!(forLib::for_compressed_size_bits(100, 13), 163);
    assert_eq!(forLib::for_compressed_size_bits(1000, 24), 3000);
    assert_eq!(forLib::for_compressed_size_bits(1000, 32), 4000);
}

#[test]
fn test_compressed_size_sorted_unsorted() {
    let zsorted: [u32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(forLib::for_compressed_size_sorted(&zsorted, 10), 10);
    assert_eq!(forLib::for_compressed_size_unsorted(&zsorted, 10), 10);
    let zsame = [42u32; 5];
    assert_eq!(forLib::for_compressed_size_sorted(&zsame, 5), 5);
    assert_eq!(forLib::for_compressed_size_unsorted(&zsame, 5), 5);
    let empty: [u32; 0] = [];
    assert_eq!(forLib::for_compressed_size_sorted(&empty, 0), 0);
    assert_eq!(forLib::for_compressed_size_unsorted(&empty, 0), 0);
}

#[test]
fn test_compress_uncompress_sorted_roundtrip_small() {
    // L=1 -> 5 (just metadata + 0 bytes for bits=0 since min==max)
    let mut input = [33u32; 1];
    input[0] = 100;
    let mut out = vec![0u8; 8000];
    let mut tmp = vec![0u32; 1];
    let s1 = forLib::for_compress_sorted(&input, &mut out, 1);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 1);
    let s3 = forLib::for_compressed_size_sorted(&input, 1);
    assert_eq!(s1, 5);
    assert_eq!(s2, 5);
    assert_eq!(s3, 5);
    assert_eq!(tmp[0], 100);
}

#[test]
fn test_compress_uncompress_sorted_roundtrip_various() {
    for &l in &[1u32, 2, 3, 8, 16, 32, 33, 64, 100, 257] {
        let input: Vec<u32> = (0..l).map(|i| 100 + i * 3).collect();
        let mut out = vec![0u8; 8000];
        let mut tmp = vec![0u32; l as usize];
        let s1 = forLib::for_compress_sorted(&input, &mut out, l);
        let s2 = forLib::for_uncompress(&out, &mut tmp, l);
        let s3 = forLib::for_compressed_size_sorted(&input, l);
        assert_eq!(s1, s2, "L={}", l);
        assert_eq!(s2, s3, "L={}", l);
        for i in 0..l as usize {
            assert_eq!(tmp[i], input[i], "L={} i={}", l, i);
        }
    }
}

#[test]
fn test_compress_uncompress_unsorted_roundtrip() {
    let in2: [u32; 10] = [7, 3, 9, 2, 10, 5, 8, 1, 6, 4];
    let mut out = vec![0u8; 8000];
    let mut tmp = vec![0u32; 10];
    let s = forLib::for_compress_unsorted(&in2, &mut out, 10);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 10);
    assert_eq!(s, s2);
    assert_eq!(s, 10);
    for i in 0..10 {
        assert_eq!(tmp[i], in2[i]);
    }
}

#[test]
fn test_for_select_sorted() {
    let input: Vec<u32> = (0..16).map(|i| 50 + i).collect();
    let mut out = vec![0u8; 200];
    forLib::for_compress_sorted(&input, &mut out, 16);
    for i in 0..16u32 {
        assert_eq!(forLib::for_select(&out, i), 50 + i, "idx={}", i);
    }
}

#[test]
fn test_for_linear_search() {
    let input: Vec<u32> = (0..16).map(|i| 50 + i).collect();
    let mut out = vec![0u8; 200];
    forLib::for_compress_sorted(&input, &mut out, 16);
    // present:
    assert_eq!(forLib::for_linear_search(&out, 16, 50), 0);
    assert_eq!(forLib::for_linear_search(&out, 16, 55), 5);
    assert_eq!(forLib::for_linear_search(&out, 16, 65), 15);
    // not present, returns length:
    assert_eq!(forLib::for_linear_search(&out, 16, 100), 16);
    assert_eq!(forLib::for_linear_search(&out, 16, 49), 16);
}

#[test]
fn test_for_lower_bound_search() {
    let input: Vec<u32> = (0..16).map(|i| 50 + i).collect();
    let mut out = vec![0u8; 200];
    forLib::for_compress_sorted(&input, &mut out, 16);
    let mut actual: u32 = 0;
    let idx = forLib::for_lower_bound_search(&out, 16, 55, &mut actual);
    assert_eq!(idx, 5);
    assert_eq!(actual, 55);
    let idx = forLib::for_lower_bound_search(&out, 16, 50, &mut actual);
    assert_eq!(idx, 0);
    assert_eq!(actual, 50);
    let idx = forLib::for_lower_bound_search(&out, 16, 64, &mut actual);
    assert_eq!(idx, 14);
    assert_eq!(actual, 64);
    let idx = forLib::for_lower_bound_search(&out, 16, 65, &mut actual);
    assert_eq!(idx, 15);
    assert_eq!(actual, 65);
    let idx = forLib::for_lower_bound_search(&out, 16, 100, &mut actual);
    assert_eq!(idx, 15);
    assert_eq!(actual, 65);
    let idx = forLib::for_lower_bound_search(&out, 16, 49, &mut actual);
    assert_eq!(idx, 0);
    assert_eq!(actual, 50);
}

#[test]
fn test_compress_bits_uncompress_bits_roundtrip() {
    let inb: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    for bits in 3u32..=8 {
        let mut outb = vec![0u8; 100];
        let mut tmpb = vec![0u32; 8];
        let s = forLib::for_compress_bits(&inb, &mut outb, 8, 0, bits);
        let u = forLib::for_uncompress_bits(&outb, &mut tmpb, 8, 0, bits);
        assert_eq!(s, u, "bits={}", bits);
        for j in 0..8 {
            assert_eq!(tmpb[j], inb[j], "bits={} j={}", bits, j);
        }
    }
}

#[test]
fn test_for_select_bits() {
    let inc: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut outb = vec![0u8; 100];
    forLib::for_compress_bits(&inc, &mut outb, 8, 10, 4);
    for i in 0..8u32 {
        assert_eq!(forLib::for_select_bits(&outb, 10, 4, i), 10 + i, "idx={}", i);
    }
}

#[test]
fn test_for_linear_search_bits() {
    let inc: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut outb = vec![0u8; 100];
    forLib::for_compress_bits(&inc, &mut outb, 8, 10, 4);
    for i in 0..8u32 {
        assert_eq!(
            forLib::for_linear_search_bits(&outb, 8, 10, 4, 10 + i),
            i,
            "search {} should be at {}",
            10 + i,
            i
        );
    }
    // Below base: returns length
    assert_eq!(forLib::for_linear_search_bits(&outb, 8, 10, 4, 5), 8);
    // Above max storable: returns length
    assert_eq!(forLib::for_linear_search_bits(&outb, 8, 10, 4, 100), 8);
}

#[test]
fn test_for_linear_search_bits_zero_bits() {
    // bits=0: only base is encodable; if value matches, returns 0.
    let outb = vec![0u8; 16];
    assert_eq!(forLib::for_linear_search_bits(&outb, 8, 42, 0, 42), 0);
    assert_eq!(forLib::for_linear_search_bits(&outb, 8, 42, 0, 0), 8);
    assert_eq!(forLib::for_linear_search_bits(&outb, 8, 42, 0, 99), 8);
}

#[test]
fn test_for_select_bits_zero_bits() {
    // bits=0 always returns base
    let outb = vec![0u8; 16];
    assert_eq!(forLib::for_select_bits(&outb, 42, 0, 0), 42);
    assert_eq!(forLib::for_select_bits(&outb, 42, 0, 5), 42);
}

#[test]
fn test_for_append_sorted_increments_size() {
    let mut out = vec![0u8; 200];
    let expected_sizes = [5u32, 6, 6, 6, 7, 8, 8, 8, 10, 10, 11, 11, 12, 12, 13, 13];
    for i in 0..16u32 {
        let s = forLib::for_append_sorted(&mut out, i, i);
        assert_eq!(s, expected_sizes[i as usize], "i={}", i);
    }
    // Read back
    let mut tmp = vec![0u32; 16];
    forLib::for_uncompress(&out, &mut tmp, 16);
    for i in 0..16 {
        assert_eq!(tmp[i], i as u32);
    }
}

#[test]
fn test_for_append_unsorted() {
    let mut out = vec![0u8; 8000];
    let values = [7u32, 3, 9, 2, 10, 5, 8, 1, 6, 4];
    for (i, v) in values.iter().enumerate() {
        forLib::for_append_unsorted(&mut out, i as u32, *v);
    }
    // Read back
    let mut tmp = vec![0u32; 10];
    forLib::for_uncompress(&out, &mut tmp, 10);
    for i in 0..10 {
        assert_eq!(tmp[i], values[i], "i={}", i);
    }
}

#[test]
fn test_for_append_bits_basic() {
    // Append a single value at index 0 with bits=4, base=10.
    let mut out = vec![0u8; 16];
    let s = forLib::for_append_bits(&mut out, 0, 10, 4, 13);
    // total bits 4 -> 1 byte
    assert_eq!(s, 1);
    // Read back
    assert_eq!(forLib::for_select_bits(&out, 10, 4, 0), 13);
}

#[test]
fn test_uncompress_zero_length() {
    let out = vec![0u8; 16];
    let mut tmp = vec![0u32; 8];
    let r = forLib::for_uncompress(&out, &mut tmp, 0);
    assert_eq!(r, 0);
}

#[test]
fn test_compress_zero_length() {
    let input: [u32; 0] = [];
    let mut out = vec![0u8; 8];
    assert_eq!(forLib::for_compress_sorted(&input, &mut out, 0), 0);
    assert_eq!(forLib::for_compress_unsorted(&input, &mut out, 0), 0);
}

#[test]
fn test_compress_bits_zero_length() {
    let input: [u32; 0] = [];
    let mut out = vec![0u8; 8];
    let mut tmp = vec![0u32; 0];
    assert_eq!(forLib::for_compress_bits(&input, &mut out, 0, 0, 8), 0);
    assert_eq!(forLib::for_uncompress_bits(&out, &mut tmp, 0, 0, 8), 0);
}

#[test]
fn test_uncompress_bits_zero_bits_fills_base() {
    // bits=0 should fill output with `base` for `length` entries.
    let out = vec![0u8; 16];
    let mut tmp = vec![0u32; 8];
    let r = forLib::for_uncompress_bits(&out, &mut tmp, 8, 42, 0);
    assert_eq!(r, 0);
    for i in 0..8 {
        assert_eq!(tmp[i], 42, "i={}", i);
    }
}

#[test]
fn test_compress_sorted_bigger_block() {
    // Sequence with len=33 to exercise the 32-block + 1-extra path.
    // Verified against C oracle: with values 100..=132 (delta=32 -> 6 bits),
    // total = METADATA(5) + ceil(33*6/8) = 5 + 25 = 30.
    let input: Vec<u32> = (0..33).map(|i| 100 + i).collect();
    let mut out = vec![0u8; 200];
    let mut tmp = vec![0u32; 33];
    let s1 = forLib::for_compress_sorted(&input, &mut out, 33);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 33);
    assert_eq!(s1, 30);
    assert_eq!(s2, 30);
    for i in 0..33 {
        assert_eq!(tmp[i], input[i]);
    }
}

#[test]
fn test_compress_unsorted_random() {
    // Random-ish inputs with min/max known.
    let input: Vec<u32> = vec![100, 50, 70, 200, 150, 80, 90, 110];
    let mut out = vec![0u8; 200];
    let mut tmp = vec![0u32; 8];
    let s1 = forLib::for_compress_unsorted(&input, &mut out, 8);
    let s2 = forLib::for_uncompress(&out, &mut tmp, 8);
    assert_eq!(s1, s2);
    for i in 0..8 {
        assert_eq!(tmp[i], input[i]);
    }
    // Check stored min and bits in metadata
    let stored_min = u32::from_le_bytes([out[0], out[1], out[2], out[3]]);
    let stored_bits = out[4];
    assert_eq!(stored_min, 50);
    // delta range = 200 - 50 = 150 needs 8 bits
    assert_eq!(stored_bits, 8);
}

fn main() {}
