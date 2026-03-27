use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

type ProcessDecisionsFn = unsafe extern "C" fn(*mut i8, usize, i32, i32) -> i32;

fn call_c(lib: &Library, input: &[u8], operation: i32, param: i32) -> i32 {
    let mut buf = input.to_vec();
    let len = buf.len();
    unsafe {
        let func: Symbol<ProcessDecisionsFn> = lib.get(b"process_decisions").unwrap();
        func(buf.as_mut_ptr() as *mut i8, len, operation, param)
    }
}

fn call_rust(input: &[u8], operation: i32, param: i32) -> i32 {
    let mut buf = input.to_vec();
    let len = buf.len();
    driver::process_decisions(&mut buf, len, operation, param)
}

fn check(lib: &Library, input: &[u8], operation: i32, param: i32) {
    let c_result = call_c(lib, input, operation, param);
    let r_result = call_rust(input, operation, param);
    assert_eq!(
        c_result, r_result,
        "Mismatch for input={:?} op={} param={}: C={} Rust={}",
        std::str::from_utf8(input).unwrap_or("<non-utf8>"),
        operation, param, c_result, r_result
    );
}

#[test]
fn test_process_decisions_edge_cases() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };

    // Empty / short inputs
    check(&lib, b"", 0, 0);       // length 0 -> -1
    check(&lib, b"y", 0, 0);      // length < 3 -> -2
    check(&lib, b"yn", 0, 0);     // length < 3 -> -2
    check(&lib, b"y", 1, 0);      // length < 3 -> -2
    check(&lib, b"y", 5, 0);      // invalid op -> -3
}

#[test]
fn test_op0_apply_permissions() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };

    // All 8 combinations of y/n for 3 bools
    let combos: &[&[u8]] = &[
        b"nnn", b"nny", b"nyn", b"nyy",
        b"ynn", b"yny", b"yyn", b"yyy",
    ];
    for combo in combos {
        check(&lib, *combo, 0, 0);
    }
    // With uppercase
    check(&lib, b"YYY", 0, 0);
    check(&lib, b"YNy", 0, 0);
    // Invalid chars (treated as false)
    check(&lib, b"xyz", 0, 0);
    check(&lib, b"y x", 0, 0);
}

#[test]
fn test_op1_evaluate_conditions() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };

    let combos: &[&[u8]] = &[
        b"nnn", b"nny", b"nyn", b"nyy",
        b"ynn", b"yny", b"yyn", b"yyy",
    ];
    // Test all 4 logic ops with all 8 bool combos
    for logic_op in 0..=3 {
        for combo in combos {
            check(&lib, *combo, 1, logic_op);
        }
    }
    // Invalid logic op
    check(&lib, b"yyy", 1, 4);
    check(&lib, b"yyy", 1, -1);
}

#[test]
fn test_op2_configure_flags() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };

    // All false
    check(&lib, b"nnnn", 2, 0);
    // All true
    check(&lib, b"yyyy", 2, 0);
    // Single true at various positions
    check(&lib, b"ynnn", 2, 0);
    check(&lib, b"nynn", 2, 0);
    check(&lib, b"nnyn", 2, 0);
    check(&lib, b"nnny", 2, 0);
    // Single false at various positions
    check(&lib, b"nyyy", 2, 0);
    check(&lib, b"ynyy", 2, 0);
    check(&lib, b"yyny", 2, 0);
    check(&lib, b"yyyn", 2, 0);
    // Alternating
    check(&lib, b"ynyn", 2, 0);
    check(&lib, b"nyny", 2, 0);
    check(&lib, b"ynynyn", 2, 0);
    // Consecutive trues >= 3
    check(&lib, b"nyyyn", 2, 0);
    check(&lib, b"yyyyn", 2, 0);
    check(&lib, b"nyyyyyy", 2, 0);
    // Mixed patterns
    check(&lib, b"yynny", 2, 0);
    check(&lib, b"yynyn", 2, 0);
    // Single element
    check(&lib, b"y", 2, 0);
    check(&lib, b"n", 2, 0);
    // Two elements
    check(&lib, b"yn", 2, 0);
    check(&lib, b"ny", 2, 0);
    check(&lib, b"yy", 2, 0);
    check(&lib, b"nn", 2, 0);
}

#[test]
fn test_op3_validate_sequence() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };

    // Single char
    check(&lib, b"y", 3, 0);
    check(&lib, b"n", 3, 0);
    // Length 2
    check(&lib, b"yn", 3, 0);
    check(&lib, b"yy", 3, 0);
    check(&lib, b"nn", 3, 0);
    check(&lib, b"ny", 3, 0);
    // Length 3
    check(&lib, b"yyn", 3, 0);
    check(&lib, b"yny", 3, 0);
    check(&lib, b"ynn", 3, 0);
    check(&lib, b"nyn", 3, 0);
    check(&lib, b"yyy", 3, 0);
    check(&lib, b"nnn", 3, 0);
    // Medium length (4-10)
    check(&lib, b"ynyn", 3, 0);
    check(&lib, b"ynynynynyy", 3, 0);  // ends with y -> -11 after rule 1 passes
    check(&lib, b"ynynynynyn", 3, 0);
    check(&lib, b"yyyyn", 3, 0);       // 4 consecutive y -> -12
    check(&lib, b"ynnnn", 3, 0);       // 4 consecutive n -> -12
    check(&lib, b"yyyyyyyyyyn", 3, 0); // many consecutive
    check(&lib, b"ynnynnynnyn", 3, 0); // long sequence
    // Long sequences (>10)
    check(&lib, b"ynynynynynyn", 3, 0);
    check(&lib, b"yyynyyynyyynyyynyyynyyynyyynyyn", 3, 0);
    // Starts with n
    check(&lib, b"nyyy", 3, 0);
    // Various patterns
    check(&lib, b"yyn", 3, 0);
    check(&lib, b"yyyn", 3, 0);
    check(&lib, b"yynyn", 3, 0);
    check(&lib, b"ynnynnynnynnyn", 3, 0);
}
