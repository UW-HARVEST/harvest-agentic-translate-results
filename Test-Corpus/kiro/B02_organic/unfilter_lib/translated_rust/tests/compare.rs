use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libunfilter_lib.so")
}

fn load_c_lib() -> Library {
    unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") }
}

// ============ unfilter tests ============

/// Call C unfilter via libloading
fn c_unfilter(lib: &Library, w: i32, h: i32, bpp: i32, data: &mut [u8]) -> i32 {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(i32, i32, i32, *mut u8) -> i32> =
            lib.get(b"unfilter").unwrap();
        func(w, h, bpp, data.as_mut_ptr())
    }
}

/// Call Rust unfilter
fn rust_unfilter(w: i32, h: i32, bpp: i32, data: &mut [u8]) -> i32 {
    unsafe { unfilter_lib::unfilter(w, h, bpp, data.as_mut_ptr()) }
}

fn run_unfilter_test(w: i32, h: i32, bpp: i32, input: &[u8]) {
    let lib = load_c_lib();
    let mut c_data = input.to_vec();
    let mut r_data = input.to_vec();
    let c_ret = c_unfilter(&lib, w, h, bpp, &mut c_data);
    let r_ret = rust_unfilter(w, h, bpp, &mut r_data);
    assert_eq!(c_ret, r_ret, "Return values differ");
    assert_eq!(c_data, r_data, "Output data differs");
}

// Filter type 0 (None) - single row
#[test]
fn test_unfilter_type0_single_row() {
    // format: [filter_byte, pixel_data...]
    // w=4, h=1, bpp=1 => len=4, total = 1 (filter) + 4 (data) = 5
    let input = vec![0, 10, 20, 30, 40];
    run_unfilter_test(4, 1, 1, &input);
}

// Filter type 1 (Sub) - single row
#[test]
fn test_unfilter_type1_single_row() {
    let input = vec![1, 10, 5, 7, 3];
    run_unfilter_test(4, 1, 1, &input);
}

// Filter type 1 (Sub) - single row, bpp=3
#[test]
fn test_unfilter_type1_bpp3() {
    // w=2, bpp=3 => len=6
    let input = vec![1, 10, 20, 30, 5, 7, 3];
    run_unfilter_test(2, 1, 3, &input);
}

// Filter type 2 (Up) - single row (no-op for first row)
#[test]
fn test_unfilter_type2_single_row() {
    let input = vec![2, 10, 20, 30, 40];
    run_unfilter_test(4, 1, 1, &input);
}

// Filter type 3 (Average) - single row
#[test]
fn test_unfilter_type3_single_row() {
    let input = vec![3, 10, 20, 30, 40];
    run_unfilter_test(4, 1, 1, &input);
}

// Filter type 4 (Paeth) - single row
#[test]
fn test_unfilter_type4_single_row() {
    let input = vec![4, 10, 20, 30, 40];
    run_unfilter_test(4, 1, 1, &input);
}

// Multi-row tests with various filter combinations
#[test]
fn test_unfilter_multirow_type0_type0() {
    // w=3, h=2, bpp=1 => len=3, each row = 1+3=4, total=8
    let input = vec![0, 10, 20, 30, 0, 40, 50, 60];
    run_unfilter_test(3, 2, 1, &input);
}

#[test]
fn test_unfilter_multirow_type1_type1() {
    let input = vec![1, 10, 5, 7, 1, 3, 8, 2];
    run_unfilter_test(3, 2, 1, &input);
}

#[test]
fn test_unfilter_multirow_type0_type2() {
    let input = vec![0, 10, 20, 30, 2, 5, 10, 15];
    run_unfilter_test(3, 2, 1, &input);
}

#[test]
fn test_unfilter_multirow_type0_type3() {
    // Average filter on second row
    let input = vec![0, 100, 150, 200, 3, 10, 20, 30];
    run_unfilter_test(3, 2, 1, &input);
}

#[test]
fn test_unfilter_multirow_type0_type4() {
    // Paeth filter on second row
    let input = vec![0, 100, 150, 200, 4, 10, 20, 30];
    run_unfilter_test(3, 2, 1, &input);
}

#[test]
fn test_unfilter_multirow_type1_type3() {
    let input = vec![1, 50, 30, 20, 3, 10, 15, 25];
    run_unfilter_test(3, 2, 1, &input);
}

#[test]
fn test_unfilter_multirow_type1_type4() {
    let input = vec![1, 50, 30, 20, 4, 10, 15, 25];
    run_unfilter_test(3, 2, 1, &input);
}

// Multi-row with bpp > 1
#[test]
fn test_unfilter_multirow_bpp3_type3() {
    // w=2, bpp=3 => len=6, each row = 1+6=7, total=14
    let input = vec![
        1, 10, 20, 30, 5, 7, 3,
        3, 15, 25, 35, 8, 12, 6,
    ];
    run_unfilter_test(2, 2, 3, &input);
}

#[test]
fn test_unfilter_multirow_bpp4_type4() {
    // w=2, bpp=4 => len=8, each row = 1+8=9, total=18
    let input = vec![
        0, 10, 20, 30, 40, 50, 60, 70, 80,
        4, 5, 10, 15, 20, 25, 30, 35, 40,
    ];
    run_unfilter_test(2, 2, 4, &input);
}

// Three rows
#[test]
fn test_unfilter_three_rows() {
    let input = vec![
        1, 10, 5, 7, 3,
        3, 20, 15, 10, 5,
        4, 8, 12, 6, 9,
    ];
    run_unfilter_test(4, 3, 1, &input);
}

// Edge case: wrapping arithmetic
#[test]
fn test_unfilter_wrapping() {
    let input = vec![1, 200, 200, 200, 200];
    run_unfilter_test(4, 1, 1, &input);
}

// Edge case: all 255s
#[test]
fn test_unfilter_all_255() {
    let input = vec![
        1, 255, 255, 255, 255,
        3, 255, 255, 255, 255,
    ];
    run_unfilter_test(4, 2, 1, &input);
}

// Invalid filter type
#[test]
fn test_unfilter_invalid_filter() {
    let input = vec![5, 10, 20, 30, 40];
    run_unfilter_test(4, 1, 1, &input);
}

// Large random-ish data
#[test]
fn test_unfilter_large() {
    let w = 16;
    let h = 8;
    let bpp = 4;
    let len = w * bpp;
    let mut input = Vec::new();
    let filters = [0u8, 1, 2, 3, 4, 1, 3, 4];
    for row in 0..h {
        input.push(filters[row]);
        for i in 0..len {
            input.push(((row * 37 + i * 13 + 7) & 0xFF) as u8);
        }
    }
    run_unfilter_test(w as i32, h as i32, bpp as i32, &input);
}

// ============ cp_inflate tests ============

/// Call C cp_inflate via libloading
fn c_cp_inflate(lib: &Library, input: &[u8], out_bytes: usize) -> (i32, Vec<u8>) {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*const u8, i32, *mut u8, i32) -> i32> =
            lib.get(b"cp_inflate").unwrap();
        let mut out = vec![0u8; out_bytes];
        let ret = func(input.as_ptr(), input.len() as i32, out.as_mut_ptr(), out_bytes as i32);
        (ret, out)
    }
}

// Test cp_inflate with a known deflate stream (stored block)
#[test]
fn test_cp_inflate_stored_block() {
    // A stored (uncompressed) deflate block:
    // bfinal=1, btype=00 (stored)
    // Then LEN=5, NLEN=~5, then 5 bytes of data "hello"
    let data = b"hello";
    let len = data.len() as u16;
    let nlen = !len;
    let mut input = Vec::new();
    input.push(0x01); // bfinal=1, btype=00
    input.push((len & 0xFF) as u8);
    input.push((len >> 8) as u8);
    input.push((nlen & 0xFF) as u8);
    input.push((nlen >> 8) as u8);
    input.extend_from_slice(data);

    let lib = load_c_lib();
    let (c_ret, c_out) = c_cp_inflate(&lib, &input, 5);
    assert_eq!(c_ret, 1, "C cp_inflate should succeed");
    assert_eq!(&c_out, data, "C cp_inflate output mismatch");
}

// Test cp_inflate with a fixed Huffman block
#[test]
fn test_cp_inflate_fixed_block() {
    // Use flate2 to create a known deflate stream, or use a pre-computed one.
    // Let's use a minimal fixed Huffman block that encodes a few bytes.
    // We'll compress with C and decompress with C to verify, then compare.
    // Actually, let's just use a known compressed stream.
    // Simplest: compress empty data -> just end-of-block symbol 256
    // Fixed Huffman: bfinal=1, btype=01, then symbol 256 = 7-bit code 0000000
    // Bit layout: 1 (bfinal) | 01 (btype) | 0000000 (end of block, reversed)
    // = 011 | 0000000 = 0b0000000_011 = 0x03 0x00
    let input = vec![0x03, 0x00];
    let lib = load_c_lib();
    let (c_ret, c_out) = c_cp_inflate(&lib, &input, 0);
    assert_eq!(c_ret, 1, "C cp_inflate empty fixed block should succeed");
    assert_eq!(c_out.len(), 0);
}
