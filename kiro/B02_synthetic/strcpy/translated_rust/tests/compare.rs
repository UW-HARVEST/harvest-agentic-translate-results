use libloading::{Library, Symbol};
use std::path::PathBuf;

type ProcessStringsFn = unsafe extern "C" fn(
    *mut u8,   // input
    usize,     // input_len
    *const u8, // reference
    usize,     // ref_len
    i32,       // operation
    u32,       // flags
) -> i32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libstrcpy_fun.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the cdylib in target/debug or target/release
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug = base.join("target/debug/libstrcpy_fun.so");
    if debug.exists() {
        return debug;
    }
    base.join("target/release/libstrcpy_fun.so")
}

fn call_both(
    input: &[u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> (i32, i32) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<ProcessStringsFn> =
        unsafe { c_lib.get(b"process_strings").expect("C process_strings") };
    let r_fn: Symbol<ProcessStringsFn> =
        unsafe { r_lib.get(b"process_strings").expect("Rust process_strings") };

    // Make mutable copies for each call (C may modify input)
    let mut c_input = [0u8; 1024];
    let mut r_input = [0u8; 1024];
    c_input[..input.len()].copy_from_slice(input);
    r_input[..input.len()].copy_from_slice(input);

    let mut c_ref = [0u8; 1024];
    let mut r_ref = [0u8; 1024];
    c_ref[..reference.len()].copy_from_slice(reference);
    r_ref[..reference.len()].copy_from_slice(reference);

    let c_result = unsafe {
        c_fn(
            c_input.as_mut_ptr(),
            input_len,
            c_ref.as_ptr(),
            ref_len,
            operation,
            flags,
        )
    };
    let r_result = unsafe {
        r_fn(
            r_input.as_mut_ptr(),
            input_len,
            r_ref.as_ptr(),
            ref_len,
            operation,
            flags,
        )
    };

    (c_result, r_result)
}

macro_rules! assert_match {
    ($input:expr, $ilen:expr, $refer:expr, $rlen:expr, $op:expr, $flags:expr, $label:expr) => {
        let (c, r) = call_both($input, $ilen, $refer, $rlen, $op, $flags);
        assert_eq!(c, r, "MISMATCH [{}]: C={} Rust={}", $label, c, r);
    };
}

// ===== Operation 0: validate_token =====

#[test]
fn op0_exact_match() {
    assert_match!(b"hello\0", 6, b"hello\0", 6, 0, 0, "op0 exact match");
}

#[test]
fn op0_no_match() {
    assert_match!(b"hello\0", 6, b"world\0", 6, 0, 0, "op0 no match");
}

#[test]
fn op0_valid_token() {
    assert_match!(b"VALID\0", 6, b"other\0", 6, 0, 0, "op0 VALID");
}

#[test]
fn op0_ok_token() {
    assert_match!(b"OK\0", 3, b"other\0", 6, 0, 0, "op0 OK");
}

// ===== Operation 1: parse_command =====

#[test]
fn op1_start() {
    assert_match!(b"START\0", 6, b"\0", 1, 1, 0, "op1 START");
}

#[test]
fn op1_stop() {
    assert_match!(b"STOP\0", 5, b"\0", 1, 1, 0, "op1 STOP");
}

#[test]
fn op1_pause() {
    assert_match!(b"PAUSE\0", 6, b"\0", 1, 1, 0, "op1 PAUSE");
}

#[test]
fn op1_resume() {
    assert_match!(b"RESUME\0", 7, b"\0", 1, 1, 0, "op1 RESUME");
}

#[test]
fn op1_reset() {
    assert_match!(b"RESET\0", 6, b"\0", 1, 1, 0, "op1 RESET");
}

#[test]
fn op1_admin() {
    assert_match!(b"ADMIN\0", 6, b"\0", 1, 1, 0, "op1 ADMIN");
}

#[test]
fn op1_unknown() {
    assert_match!(b"UNKNOWN\0", 8, b"\0", 1, 1, 0, "op1 UNKNOWN");
}

#[test]
fn op1_start_with_space() {
    assert_match!(b"START args\0", 11, b"\0", 1, 1, 0, "op1 START+space");
}

// ===== Operation 2: compare_prefix =====

#[test]
fn op2_prefix_match() {
    assert_match!(b"hello_world\0", 12, b"hello\0", 6, 2, 0, "op2 prefix");
}

#[test]
fn op2_exact_match() {
    assert_match!(b"hello\0", 6, b"hello\0", 6, 2, 1, "op2 exact");
}

#[test]
fn op2_exact_no_match() {
    assert_match!(b"hello\0", 6, b"world\0", 6, 2, 1, "op2 exact no match");
}

#[test]
fn op2_exact_v1() {
    assert_match!(b"test_v1\0", 8, b"test\0", 5, 2, 1, "op2 exact _v1");
}

#[test]
fn op2_exact_v2() {
    assert_match!(b"test_v2\0", 8, b"test\0", 5, 2, 1, "op2 exact _v2");
}

#[test]
fn op2_exact_old() {
    assert_match!(b"test_old\0", 9, b"test\0", 5, 2, 1, "op2 exact _old");
}

#[test]
fn op2_exact_new() {
    assert_match!(b"test_new\0", 9, b"test\0", 5, 2, 1, "op2 exact _new");
}

#[test]
fn op2_exact_tmp() {
    assert_match!(b"test_tmp\0", 9, b"test\0", 5, 2, 1, "op2 exact _tmp");
}

#[test]
fn op2_no_prefix() {
    assert_match!(b"xyz\0", 4, b"hello\0", 6, 2, 0, "op2 no prefix");
}

// ===== Operation 3: find_delimiter =====

#[test]
fn op3_colon_default() {
    assert_match!(b"key:value\0", 10, b"\0", 0, 3, 0, "op3 colon default");
}

#[test]
fn op3_custom_delim() {
    assert_match!(b"a|b|c\0", 6, b"|\0", 2, 3, 0, "op3 pipe delim");
}

#[test]
fn op3_no_delim() {
    assert_match!(b"nope\0", 5, b";\0", 2, 3, 0, "op3 no delim");
}

#[test]
fn op3_none_special() {
    assert_match!(b"NONE\0", 5, b"|\0", 2, 3, 0, "op3 NONE special");
}

#[test]
fn op3_empty_special() {
    assert_match!(b"EMPTY\0", 6, b":\0", 2, 3, 0, "op3 EMPTY special");
}

#[test]
fn op3_zero_len() {
    assert_match!(b"data\0", 0, b":\0", 2, 3, 0, "op3 zero len");
}

// ===== Operation 4: match_pattern (case sensitive) =====

#[test]
fn op4_cs_exact() {
    assert_match!(b"hello\0", 6, b"hello\0", 6, 4, 2, "op4 cs exact");
}

#[test]
fn op4_cs_no_match() {
    assert_match!(b"hello\0", 6, b"world\0", 6, 4, 2, "op4 cs no match");
}

#[test]
fn op4_cs_contains() {
    assert_match!(b"say hello world\0", 16, b"hello\0", 6, 4, 2, "op4 cs contains");
}

#[test]
fn op4_cs_wildcard_both() {
    assert_match!(b"*hello*\0", 8, b"hello\0", 6, 4, 2, "op4 cs *X*");
}

#[test]
fn op4_cs_wildcard_suffix() {
    assert_match!(b"hello*\0", 7, b"hello\0", 6, 4, 2, "op4 cs X*");
}

#[test]
fn op4_cs_wildcard_prefix() {
    assert_match!(b"*hello\0", 7, b"hello\0", 6, 4, 2, "op4 cs *X");
}

// ===== Operation 4: match_pattern (case insensitive) =====

#[test]
fn op4_ci_exact() {
    assert_match!(b"hello\0", 6, b"hello\0", 6, 4, 0, "op4 ci exact");
}

#[test]
fn op4_ci_case_diff() {
    assert_match!(b"HELLO\0", 6, b"hello\0", 6, 4, 0, "op4 ci case diff");
}

#[test]
fn op4_ci_prefix() {
    assert_match!(b"hello_world\0", 12, b"hello\0", 6, 4, 0, "op4 ci prefix");
}

#[test]
fn op4_ci_no_match() {
    assert_match!(b"xyz\0", 4, b"hello\0", 6, 4, 0, "op4 ci no match");
}

// ===== Default operation =====

#[test]
fn op_default() {
    assert_match!(b"test\0", 5, b"ref\0", 4, 99, 0, "default op");
}
