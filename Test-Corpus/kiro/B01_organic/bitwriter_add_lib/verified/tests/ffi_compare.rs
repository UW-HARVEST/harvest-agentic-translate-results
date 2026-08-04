use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
struct TflacBitwriter {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

impl TflacBitwriter {
    fn new() -> Self {
        Self { val: 0, bits: 0, pos: 0, len: 0, tot: 0, buffer: std::ptr::null_mut() }
    }
    fn with(val: u64, bits: u32) -> Self {
        Self { val, bits, pos: 0, len: 0, tot: 0, buffer: std::ptr::null_mut() }
    }
}

type BitwriterAddFn = unsafe extern "C" fn(*mut TflacBitwriter, u32, u64) -> i32;

fn rust_so_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    dir.join("target").join(profile).join("libbitwriter_add_lib.so")
}

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn compare(bits: u32, val: u64, init: TflacBitwriter) {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };
    let c_fn: Symbol<BitwriterAddFn> = unsafe { c_lib.get(b"bitwriter_add").unwrap() };
    let r_fn: Symbol<BitwriterAddFn> = unsafe { r_lib.get(b"bitwriter_add").unwrap() };

    let mut c_bw = init.clone();
    let mut r_bw = init;
    let c_ret = unsafe { c_fn(&mut c_bw, bits, val) };
    let r_ret = unsafe { r_fn(&mut r_bw, bits, val) };

    assert_eq!(c_ret, r_ret, "return mismatch for bits={bits} val={val}");
    assert_eq!(c_bw.val, r_bw.val, "val mismatch for bits={bits} val={val}");
    assert_eq!(c_bw.bits, r_bw.bits, "bits mismatch for bits={bits} val={val}");
    assert_eq!(c_bw.tot, r_bw.tot, "tot mismatch for bits={bits} val={val}");
}

#[test]
fn test_small_bits() {
    for bits in [1, 2, 4, 8, 16, 31, 32] {
        for val in [0u64, 1, 0xFF, 0xDEAD] {
            compare(bits, val, TflacBitwriter::new());
        }
    }
}

#[test]
fn test_loop_trigger() {
    // Pre-fill bw.bits so that bw.bits + bits >= 64, triggering the while loop
    for pre_bits in [32, 48, 60, 63] {
        for bits in [4, 8, 16, 32] {
            for val in [0u64, 1, 0xABCD, u64::MAX] {
                compare(bits, val, TflacBitwriter::with(0x123456789ABCDEF0, pre_bits));
            }
        }
    }
}

#[test]
fn test_zero_bits() {
    compare(0, 0, TflacBitwriter::new());
    compare(0, 0xFF, TflacBitwriter::new());
}

#[test]
fn test_full_64_bits() {
    compare(64, u64::MAX, TflacBitwriter::new());
    compare(64, 0, TflacBitwriter::new());
    compare(64, 0xDEADBEEFCAFEBABE, TflacBitwriter::with(0, 1));
}

#[test]
fn test_sequential_adds() {
    // Call bitwriter_add multiple times in sequence on the same struct
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };
    let c_fn: Symbol<BitwriterAddFn> = unsafe { c_lib.get(b"bitwriter_add").unwrap() };
    let r_fn: Symbol<BitwriterAddFn> = unsafe { r_lib.get(b"bitwriter_add").unwrap() };

    let mut c_bw = TflacBitwriter::new();
    let mut r_bw = TflacBitwriter::new();

    let ops: &[(u32, u64)] = &[(8, 0xAB), (16, 0x1234), (32, 0xDEADBEEF), (8, 0xFF)];
    for &(bits, val) in ops {
        unsafe { c_fn(&mut c_bw, bits, val) };
        unsafe { r_fn(&mut r_bw, bits, val) };
        assert_eq!(c_bw.val, r_bw.val, "seq val mismatch after bits={bits}");
        assert_eq!(c_bw.bits, r_bw.bits, "seq bits mismatch after bits={bits}");
        assert_eq!(c_bw.tot, r_bw.tot, "seq tot mismatch after bits={bits}");
    }
}
