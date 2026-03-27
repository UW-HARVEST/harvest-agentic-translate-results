use libloading::{Library, Symbol};
use std::path::PathBuf;

type ProcessBufferFn = unsafe extern "C" fn(*mut u8, usize, u32, i32, i32) -> usize;

fn c_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src/build/libdriver.so")
}

fn call_c(buf: &[u8], flags: u32, param1: i32, param2: i32) -> (usize, Vec<u8>) {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let func: Symbol<ProcessBufferFn> =
        unsafe { lib.get(b"process_buffer").expect("find process_buffer") };
    let mut cbuf = [0u8; 256];
    let len = buf.len();
    cbuf[..len].copy_from_slice(buf);
    let new_len = unsafe { func(cbuf.as_mut_ptr(), len, flags, param1, param2) };
    (new_len, cbuf[..new_len].to_vec())
}

fn call_rust(buf: &[u8], flags: u32, param1: i32, param2: i32) -> (usize, Vec<u8>) {
    let mut rbuf = [0u8; 256];
    let len = buf.len();
    rbuf[..len].copy_from_slice(buf);
    let new_len = driver::process_buffer(&mut rbuf, len, flags, param1, param2);
    (new_len, rbuf[..new_len].to_vec())
}

fn check(label: &str, buf: &[u8], flags: u32, param1: i32, param2: i32) {
    let (c_len, c_out) = call_c(buf, flags, param1, param2);
    let (r_len, r_out) = call_rust(buf, flags, param1, param2);
    assert_eq!(c_len, r_len, "{label}: length mismatch (C={c_len}, Rust={r_len})");
    assert_eq!(c_out, r_out, "{label}: data mismatch\n  C:    {c_out:?}\n  Rust: {r_out:?}");
}

// === Flag 0x01: rotate_buffer ===

#[test]
fn test_rotate_right_small() {
    check("rotate right 2", &[1, 2, 3, 4, 5, 6, 7, 8], 0x01, 2, 0);
}

#[test]
fn test_rotate_right_large() {
    check("rotate right 6", &[1, 2, 3, 4, 5, 6, 7, 8], 0x01, 6, 0);
}

#[test]
fn test_rotate_left() {
    check("rotate left", &[1, 2, 3, 4, 5, 6, 7, 8], 0x01, -3, 0);
}

#[test]
fn test_rotate_single() {
    check("rotate single", &[42], 0x01, 1, 0);
}

#[test]
fn test_rotate_zero() {
    check("rotate zero", &[1, 2, 3], 0x01, 0, 0);
}

// === Flag 0x02: compact_runs ===

#[test]
fn test_compact_basic() {
    check("compact basic", &[1, 1, 1, 2, 3, 3, 3, 3, 4], 0x02, 3, 0);
}

#[test]
fn test_compact_no_runs() {
    check("compact no runs", &[1, 2, 3, 4, 5], 0x02, 3, 0);
}

#[test]
fn test_compact_all_same() {
    check("compact all same", &[7, 7, 7, 7, 7, 7, 7, 7], 0x02, 3, 0);
}

#[test]
fn test_compact_threshold_2() {
    check("compact threshold 2", &[1, 1, 2, 2, 2, 3], 0x02, 2, 0);
}

// === Flag 0x04: remove_duplicates ===

#[test]
fn test_dedup_preserve_order() {
    check("dedup preserve", &[3, 1, 2, 1, 3, 4, 2], 0x04, 0, 1);
}

#[test]
fn test_dedup_no_preserve() {
    check("dedup no preserve", &[3, 1, 2, 1, 3, 4, 2], 0x04, 0, 0);
}

#[test]
fn test_dedup_all_unique() {
    check("dedup all unique", &[1, 2, 3, 4, 5], 0x04, 0, 1);
}

#[test]
fn test_dedup_all_same() {
    check("dedup all same", &[5, 5, 5, 5], 0x04, 0, 1);
}

// === Flag 0x08: interleave_halves ===

#[test]
fn test_interleave_even() {
    check("interleave even", &[1, 2, 3, 4, 5, 6, 7, 8], 0x08, 0, 0);
}

#[test]
fn test_interleave_odd() {
    check("interleave odd", &[1, 2, 3, 4, 5, 6, 7], 0x08, 0, 0);
}

#[test]
fn test_interleave_two() {
    check("interleave two", &[1, 2], 0x08, 0, 0);
}

#[test]
fn test_interleave_three() {
    check("interleave three", &[1, 2, 3], 0x08, 0, 0);
}

// === Flag 0x10: reverse_segments ===

#[test]
fn test_reverse_seg4() {
    check("reverse seg 4", &[1, 2, 3, 4, 5, 6, 7, 8], 0x10, 4, 0);
}

#[test]
fn test_reverse_seg2() {
    check("reverse seg 2", &[1, 2, 3, 4, 5, 6, 7, 8, 9], 0x10, 2, 0);
}

#[test]
fn test_reverse_seg_default() {
    // param1 <= 0 means default seg_size=4
    check("reverse seg default", &[1, 2, 3, 4, 5, 6, 7, 8], 0x10, 0, 0);
}

// === Combined flags ===

#[test]
fn test_rotate_then_compact() {
    check("rotate+compact", &[1, 1, 1, 2, 3, 3, 3, 4], 0x03, 3, 0);
}

#[test]
fn test_compact_then_dedup() {
    check("compact+dedup", &[1, 1, 1, 2, 2, 2, 1, 3], 0x06, 3, 1);
}

#[test]
fn test_all_flags() {
    check("all flags", &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 0x1F, 4, 1);
}

#[test]
fn test_interleave_then_reverse() {
    check("interleave+reverse", &[1, 2, 3, 4, 5, 6, 7, 8], 0x18, 4, 0);
}

// === Edge cases ===

#[test]
fn test_empty_buffer() {
    check("empty", &[], 0x1F, 3, 1);
}

#[test]
fn test_null_flags() {
    check("no flags", &[1, 2, 3, 4], 0x00, 0, 0);
}

#[test]
fn test_large_buffer() {
    let buf: Vec<u8> = (0..=255).collect();
    check("large 256", &buf, 0x01, 7, 0);
}

#[test]
fn test_large_rotate_and_dedup() {
    let mut buf: Vec<u8> = (0..200).map(|i| (i % 50) as u8).collect();
    check("large rotate+dedup", &buf, 0x05, 13, 1);
}
