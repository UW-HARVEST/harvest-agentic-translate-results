use libloading::{Library, Symbol};
use std::path::PathBuf;

type ProcessBufferFn = unsafe extern "C" fn(*mut u8, usize, u32, i32, i32) -> usize;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

fn load_fn(lib: &Library) -> Symbol<ProcessBufferFn> {
    unsafe { lib.get(b"process_buffer").expect("symbol not found") }
}

fn call_both(input: &[u8], flags: u32, p1: i32, p2: i32) -> (Vec<u8>, usize, Vec<u8>, usize) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = load_fn(&c_lib);
    let r_fn = load_fn(&r_lib);

    let mut c_buf = [0u8; 256];
    let mut r_buf = [0u8; 256];
    c_buf[..input.len()].copy_from_slice(input);
    r_buf[..input.len()].copy_from_slice(input);

    let c_len = unsafe { c_fn(c_buf.as_mut_ptr(), input.len(), flags, p1, p2) };
    let r_len = unsafe { r_fn(r_buf.as_mut_ptr(), input.len(), flags, p1, p2) };

    (c_buf[..c_len].to_vec(), c_len, r_buf[..r_len].to_vec(), r_len)
}

macro_rules! assert_match {
    ($input:expr, $flags:expr, $p1:expr, $p2:expr) => {{
        let (c_out, c_len, r_out, r_len) = call_both($input, $flags, $p1, $p2);
        assert_eq!(c_len, r_len, "length mismatch: flags={:#x} p1={} p2={} input={:?}", $flags, $p1, $p2, $input);
        assert_eq!(c_out, r_out, "data mismatch: flags={:#x} p1={} p2={} input={:?}", $flags, $p1, $p2, $input);
    }};
}

// --- Null / empty edge cases ---
#[test]
fn test_null_and_empty() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = load_fn(&c_lib);
    let r_fn = load_fn(&r_lib);

    // null pointer
    assert_eq!(unsafe { c_fn(std::ptr::null_mut(), 10, 0xFF, 0, 0) }, 0);
    assert_eq!(unsafe { r_fn(std::ptr::null_mut(), 10, 0xFF, 0, 0) }, 0);

    // zero length
    let mut buf = [0u8; 1];
    assert_eq!(unsafe { c_fn(buf.as_mut_ptr(), 0, 0xFF, 0, 0) }, 0);
    assert_eq!(unsafe { r_fn(buf.as_mut_ptr(), 0, 0xFF, 0, 0) }, 0);
}

// --- Rotate (flag 0x01) ---
#[test]
fn test_rotate_right() {
    assert_match!(&[1, 2, 3, 4, 5], 0x01, 2, 0);
}

#[test]
fn test_rotate_left() {
    assert_match!(&[1, 2, 3, 4, 5], 0x01, -2, 0);
}

#[test]
fn test_rotate_zero() {
    assert_match!(&[1, 2, 3], 0x01, 0, 0);
}

#[test]
fn test_rotate_full() {
    assert_match!(&[1, 2, 3], 0x01, 3, 0);
}

#[test]
fn test_rotate_single() {
    assert_match!(&[42], 0x01, 5, 0);
}

#[test]
fn test_rotate_large_offset() {
    assert_match!(&[10, 20, 30, 40, 50, 60], 0x01, 4, 0);
}

// --- Compact runs (flag 0x02) ---
#[test]
fn test_compact_basic() {
    assert_match!(&[1, 1, 1, 2, 3, 3, 3], 0x02, 3, 0);
}

#[test]
fn test_compact_no_runs() {
    assert_match!(&[1, 2, 3, 4], 0x02, 3, 0);
}

#[test]
fn test_compact_all_same() {
    assert_match!(&[5, 5, 5, 5, 5], 0x02, 2, 0);
}

#[test]
fn test_compact_threshold_default() {
    // param1 <= 0 => threshold defaults to 3
    assert_match!(&[7, 7, 7, 7, 8, 8], 0x02, 0, 0);
}

// --- Remove duplicates (flag 0x04) ---
#[test]
fn test_dedup_preserve_order() {
    assert_match!(&[3, 1, 2, 1, 3, 4, 2], 0x04, 0, 1);
}

#[test]
fn test_dedup_no_preserve() {
    assert_match!(&[3, 1, 2, 1, 3, 4, 2], 0x04, 0, 0);
}

#[test]
fn test_dedup_all_same() {
    assert_match!(&[9, 9, 9, 9], 0x04, 0, 1);
}

#[test]
fn test_dedup_single() {
    assert_match!(&[42], 0x04, 0, 0);
}

// --- Interleave halves (flag 0x08) ---
#[test]
fn test_interleave_even() {
    assert_match!(&[1, 2, 3, 4, 5, 6], 0x08, 0, 0);
}

#[test]
fn test_interleave_odd() {
    assert_match!(&[1, 2, 3, 4, 5], 0x08, 0, 0);
}

#[test]
fn test_interleave_two() {
    assert_match!(&[10, 20], 0x08, 0, 0);
}

// --- Reverse segments (flag 0x10) ---
#[test]
fn test_reverse_seg_4() {
    assert_match!(&[1, 2, 3, 4, 5, 6, 7, 8, 9], 0x10, 4, 0);
}

#[test]
fn test_reverse_seg_2() {
    assert_match!(&[1, 2, 3, 4, 5, 6], 0x10, 2, 0);
}

#[test]
fn test_reverse_seg_default() {
    // param1 <= 0 => seg_size defaults to 4
    assert_match!(&[1, 2, 3, 4, 5, 6, 7, 8], 0x10, 0, 0);
}

// --- Combined flags ---
#[test]
fn test_rotate_then_compact() {
    assert_match!(&[1, 1, 1, 2, 3, 3, 3, 4], 0x03, 2, 0);
}

#[test]
fn test_compact_then_dedup() {
    assert_match!(&[5, 5, 5, 3, 5, 5, 5], 0x06, 3, 1);
}

#[test]
fn test_all_flags() {
    assert_match!(&[1, 2, 2, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0x1F, 3, 1);
}

#[test]
fn test_all_flags_no_preserve() {
    assert_match!(&[10, 20, 20, 20, 30, 40, 50, 60, 70, 80], 0x1F, 3, 0);
}

// --- Stress: larger buffer ---
#[test]
fn test_large_buffer_rotate() {
    let input: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
    assert_match!(&input, 0x01, 73, 0);
}

#[test]
fn test_large_buffer_all_flags() {
    let input: Vec<u8> = (0..128).map(|i| (i % 10) as u8).collect();
    assert_match!(&input, 0x1F, 4, 1);
}

// --- Negative rotation edge cases ---
#[test]
fn test_rotate_negative_large() {
    assert_match!(&[1, 2, 3, 4, 5, 6, 7, 8], 0x01, -13, 0);
}
