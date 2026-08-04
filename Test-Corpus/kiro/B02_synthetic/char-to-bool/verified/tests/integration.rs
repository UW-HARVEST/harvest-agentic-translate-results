use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;

type ProcessDecisionsFn = unsafe extern "C" fn(*const c_char, usize, i32, i32) -> i32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

fn call_both(input: &[u8], operation: i32, param: i32) -> (i32, i32) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
        let c_fn: Symbol<ProcessDecisionsFn> = c_lib.get(b"process_decisions").unwrap();
        let r_fn: Symbol<ProcessDecisionsFn> = r_lib.get(b"process_decisions").unwrap();
        // C validate_sequence writes bools in-place over the input buffer,
        // so we must pass mutable heap copies to avoid SIGSEGV on read-only memory.
        let mut c_buf = input.to_vec();
        let mut r_buf = input.to_vec();
        let c_res = c_fn(c_buf.as_mut_ptr() as *const c_char, c_buf.len(), operation, param);
        let r_res = r_fn(r_buf.as_mut_ptr() as *const c_char, r_buf.len(), operation, param);
        (c_res, r_res)
    }
}

fn assert_match(input: &[u8], operation: i32, param: i32) {
    let (c, r) = call_both(input, operation, param);
    assert_eq!(c, r, "mismatch op={operation} param={param} input={:?}: C={c} Rust={r}",
        std::str::from_utf8(input).unwrap_or("<non-utf8>"));
}

// --- NULL / empty edge cases ---
#[test]
fn test_null_and_empty() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
        let c_fn: Symbol<ProcessDecisionsFn> = c_lib.get(b"process_decisions").unwrap();
        let r_fn: Symbol<ProcessDecisionsFn> = r_lib.get(b"process_decisions").unwrap();
        // NULL pointer
        assert_eq!(c_fn(std::ptr::null(), 0, 0, 0), r_fn(std::ptr::null(), 0, 0, 0));
        // Non-null but length 0
        let b = b"x";
        assert_eq!(
            c_fn(b.as_ptr() as *const c_char, 0, 0, 0),
            r_fn(b.as_ptr() as *const c_char, 0, 0, 0)
        );
    }
}

// --- Operation 0: apply_permissions ---
#[test]
fn test_op0_permissions() {
    // All 8 combinations of y/n for 3 chars
    for &a in &[b'y', b'n'] {
        for &b in &[b'y', b'n'] {
            for &c in &[b'y', b'n'] {
                assert_match(&[a, b, c], 0, 0);
            }
        }
    }
    // Short inputs
    assert_match(b"y", 0, 0);
    assert_match(b"yn", 0, 0);
    // Non-standard chars
    assert_match(b"YYY", 0, 0);
    assert_match(b"YnN", 0, 0);
    assert_match(b"abc", 0, 0);
    assert_match(b"yyy", 0, 0);
}

// --- Operation 1: evaluate_conditions ---
#[test]
fn test_op1_conditions() {
    for logic_op in 0..=4 {
        for &a in &[b'y', b'n'] {
            for &b in &[b'y', b'n'] {
                for &c in &[b'y', b'n'] {
                    assert_match(&[a, b, c], 1, logic_op);
                }
            }
        }
    }
    // Short input
    assert_match(b"y", 1, 0);
    assert_match(b"yn", 1, 0);
    // Invalid logic_op
    assert_match(b"yyy", 1, 99);
}

// --- Operation 2: configure_flags ---
#[test]
fn test_op2_flags() {
    // All false
    assert_match(b"nnnn", 2, 0);
    // All true
    assert_match(b"yyyy", 2, 0);
    // Single true at each position
    for i in 0..5 {
        let mut v = vec![b'n'; 5];
        v[i] = b'y';
        assert_match(&v, 2, 0);
    }
    // Single false at each position
    for i in 0..5 {
        let mut v = vec![b'y'; 5];
        v[i] = b'n';
        assert_match(&v, 2, 0);
    }
    // Alternating
    assert_match(b"ynyn", 2, 0);
    assert_match(b"nyny", 2, 0);
    assert_match(b"ynyny", 2, 0);
    // Consecutive trues
    assert_match(b"nyyyn", 2, 0);
    assert_match(b"yyyyn", 2, 0);
    assert_match(b"nyyyy", 2, 0);
    // Mixed
    assert_match(b"yynny", 2, 0);
    assert_match(b"yynyn", 2, 0);
    // Long input (32+)
    let long_y: Vec<u8> = vec![b'y'; 35];
    assert_match(&long_y, 2, 0);
    // Single char
    assert_match(b"y", 2, 0);
    assert_match(b"n", 2, 0);
}

// --- Operation 3: validate_sequence ---
#[test]
fn test_op3_validate_short() {
    // Length 1
    assert_match(b"y", 3, 0);
    assert_match(b"n", 3, 0);
    // Length 2
    assert_match(b"yn", 3, 0);
    assert_match(b"yy", 3, 0);
    assert_match(b"ny", 3, 0);
    assert_match(b"nn", 3, 0);
    // Length 3
    assert_match(b"yyn", 3, 0);
    assert_match(b"yny", 3, 0);
    assert_match(b"ynn", 3, 0);
    assert_match(b"yyy", 3, 0);
    assert_match(b"nyn", 3, 0);
    assert_match(b"nny", 3, 0);
    assert_match(b"nnn", 3, 0);
}

#[test]
fn test_op3_validate_medium() {
    assert_match(b"ynyn", 3, 0);
    assert_match(b"ynnyn", 3, 0);
    assert_match(b"yyynn", 3, 0);
    assert_match(b"yyyyyyyy", 3, 0);
    assert_match(b"ynnnnnn", 3, 0);
    assert_match(b"ynynynyn", 3, 0);
    assert_match(b"ynnnynnyn", 3, 0);
    assert_match(b"ynnnnnnnnn", 3, 0);
}

#[test]
fn test_op3_validate_long() {
    // >10 chars
    assert_match(b"ynynynynynyn", 3, 0);
    assert_match(b"ynnnnnnnnnnnn", 3, 0);
    assert_match(b"ynynynynynynynyn", 3, 0);
    // 4+ consecutive
    assert_match(b"ynnnnnyyyyyn", 3, 0);
    assert_match(b"yyyyyyyyyyyyn", 3, 0);
}

// --- Invalid operation ---
#[test]
fn test_invalid_op() {
    assert_match(b"yyy", 4, 0);
    assert_match(b"yyy", -1, 0);
    assert_match(b"yyy", 99, 0);
}
