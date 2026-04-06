use bitwriter_add_lib::tflac_bitwriter;
use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libbitwriter_add_lib.so")
}

#[repr(C)]
#[derive(Clone)]
struct CBitwriter {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

fn make_bw(val: u64, bits: u32, pos: u32, len: u32, tot: u32) -> CBitwriter {
    CBitwriter { val, bits, pos, len, tot, buffer: std::ptr::null_mut() }
}

fn run_c(lib: &Library, bw: &mut CBitwriter, bits: u32, val: u64) -> i32 {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*mut CBitwriter, u32, u64) -> i32> =
            lib.get(b"bitwriter_add").unwrap();
        func(bw as *mut CBitwriter, bits, val)
    }
}

fn run_rust(bw: &mut tflac_bitwriter, bits: u32, val: u64) -> i32 {
    unsafe { bitwriter_add_lib::bitwriter_add(bw as *mut tflac_bitwriter, bits, val) }
}

fn compare(label: &str, init: CBitwriter, bits: u32, val: u64) {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };

    let mut c_bw = init.clone();
    let mut r_bw = tflac_bitwriter {
        val: init.val, bits: init.bits, pos: init.pos,
        len: init.len, tot: init.tot, buffer: init.buffer,
    };

    let c_ret = run_c(&lib, &mut c_bw, bits, val);
    let r_ret = run_rust(&mut r_bw, bits, val);

    assert_eq!(c_ret, r_ret, "{label}: return value mismatch");
    assert_eq!(c_bw.val, r_bw.val, "{label}: val mismatch (C={:#x}, Rust={:#x})", c_bw.val, r_bw.val);
    assert_eq!(c_bw.bits, r_bw.bits, "{label}: bits mismatch (C={}, Rust={})", c_bw.bits, r_bw.bits);
    assert_eq!(c_bw.tot, r_bw.tot, "{label}: tot mismatch (C={}, Rust={})", c_bw.tot, r_bw.tot);
    assert_eq!(c_bw.pos, r_bw.pos, "{label}: pos mismatch");
    assert_eq!(c_bw.len, r_bw.len, "{label}: len mismatch");
}

#[test]
fn test_zero_bits() {
    compare("zero_bits", make_bw(0, 0, 0, 0, 0), 0, 0);
}

#[test]
fn test_small_add() {
    compare("small_add", make_bw(0, 0, 0, 100, 0), 8, 0xAB);
}

#[test]
fn test_partial_fill() {
    compare("partial_fill", make_bw(0, 0, 0, 100, 0), 32, 0xDEADBEEF);
}

#[test]
fn test_near_full() {
    compare("near_full", make_bw(0, 60, 0, 100, 0), 8, 0xFF);
}

#[test]
fn test_exact_64() {
    compare("exact_64", make_bw(0, 0, 0, 100, 0), 64, 0x123456789ABCDEF0);
}

#[test]
fn test_overflow_loop() {
    compare("overflow_loop", make_bw(0, 32, 0, 100, 0), 64, 0xFFFFFFFFFFFFFFFF);
}

#[test]
fn test_preexisting_val() {
    compare("preexisting_val", make_bw(0xAAAAAAAAAAAAAAAA, 16, 5, 200, 100), 16, 0x5555);
}

#[test]
fn test_one_bit() {
    compare("one_bit", make_bw(0, 0, 0, 100, 0), 1, 1);
}

#[test]
fn test_63_bits_then_add_2() {
    compare("63_then_2", make_bw(0, 63, 0, 100, 0), 2, 0x3);
}

#[test]
fn test_sequential_adds() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let mut c_bw = make_bw(0, 0, 0, 200, 0);
    let mut r_bw = tflac_bitwriter {
        val: 0, bits: 0, pos: 0, len: 200, tot: 0, buffer: std::ptr::null_mut(),
    };

    let inputs: &[(u32, u64)] = &[
        (8, 0xFF), (16, 0x1234), (32, 0xDEADBEEF), (4, 0xA), (1, 1),
    ];

    for (i, &(bits, val)) in inputs.iter().enumerate() {
        let c_ret = run_c(&lib, &mut c_bw, bits, val);
        let r_ret = run_rust(&mut r_bw, bits, val);
        assert_eq!(c_ret, r_ret, "seq[{i}]: return mismatch");
        assert_eq!(c_bw.val, r_bw.val, "seq[{i}]: val mismatch (C={:#x}, Rust={:#x})", c_bw.val, r_bw.val);
        assert_eq!(c_bw.bits, r_bw.bits, "seq[{i}]: bits mismatch (C={}, Rust={})", c_bw.bits, r_bw.bits);
        assert_eq!(c_bw.tot, r_bw.tot, "seq[{i}]: tot mismatch");
    }
}
