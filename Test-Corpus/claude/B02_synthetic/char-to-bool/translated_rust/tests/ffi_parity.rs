// FFI parity test between the C shared library (built from c_src) and the
// Rust shared library (built from this crate). Both libraries expose
// `process_decisions` as their sole public symbol; we load each via
// `libloading`, drive identical inputs through both, and assert byte-for-byte
// equal return values.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

type ProcessDecisionsFn = unsafe extern "C" fn(*mut c_char, usize, c_int, c_int) -> c_int;

struct LibPair {
    _c_lib: Library,
    _rust_lib: Library,
    c_proc: extern "C" fn(*mut c_char, usize, c_int, c_int) -> c_int,
    rust_proc: extern "C" fn(*mut c_char, usize, c_int, c_int) -> c_int,
}

fn find_c_lib() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libdriver_c.so");
    assert!(
        p.exists(),
        "C shared library not found at {:?}; build it first via gcc -shared -fPIC",
        p
    );
    p
}

fn find_rust_lib() -> PathBuf {
    // Prefer the same build profile we are running under.
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push(profile);
    p.push("libdriver.so");
    assert!(
        p.exists(),
        "Rust shared library not found at {:?}; run `cargo build` first",
        p
    );
    p
}

fn load_libs() -> LibPair {
    unsafe {
        let c_lib = Library::new(find_c_lib()).expect("load C lib");
        let rust_lib = Library::new(find_rust_lib()).expect("load Rust lib");
        let c_sym: Symbol<ProcessDecisionsFn> = c_lib
            .get(b"process_decisions\0")
            .expect("C process_decisions");
        let r_sym: Symbol<ProcessDecisionsFn> = rust_lib
            .get(b"process_decisions\0")
            .expect("Rust process_decisions");
        // Promote to function pointers detached from Symbol lifetimes.
        let c_proc: extern "C" fn(*mut c_char, usize, c_int, c_int) -> c_int =
            std::mem::transmute(*c_sym.into_raw());
        let rust_proc: extern "C" fn(*mut c_char, usize, c_int, c_int) -> c_int =
            std::mem::transmute(*r_sym.into_raw());
        LibPair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_proc,
            rust_proc,
        }
    }
}

/// Drive both libraries with identical inputs. The C `validate_sequence`
/// path mutates the input buffer (it reuses the buffer to store bool values),
/// so we provide each library with its own private copy.
fn call_both(libs: &LibPair, input: &[u8], op: c_int, param: c_int) -> (c_int, c_int) {
    let mut c_buf: Vec<u8> = input.to_vec();
    let mut r_buf: Vec<u8> = input.to_vec();
    let c_ptr = if c_buf.is_empty() {
        std::ptr::null_mut()
    } else {
        c_buf.as_mut_ptr() as *mut c_char
    };
    let r_ptr = if r_buf.is_empty() {
        std::ptr::null_mut()
    } else {
        r_buf.as_mut_ptr() as *mut c_char
    };
    let c_ret = (libs.c_proc)(c_ptr, input.len(), op, param);
    let r_ret = (libs.rust_proc)(r_ptr, input.len(), op, param);
    (c_ret, r_ret)
}

fn check(libs: &LibPair, input: &[u8], op: c_int, param: c_int) {
    let (c, r) = call_both(libs, input, op, param);
    assert_eq!(
        c, r,
        "mismatch for input={:?} op={} param={}: C={} Rust={}",
        std::str::from_utf8(input).unwrap_or("<bin>"),
        op,
        param,
        c,
        r
    );
}

#[test]
fn null_or_empty_input_returns_minus_one() {
    let libs = load_libs();
    // Pass a NULL pointer with zero length: both should return -1.
    let c = (libs.c_proc)(std::ptr::null_mut(), 0, 0, 0);
    let r = (libs.rust_proc)(std::ptr::null_mut(), 0, 0, 0);
    assert_eq!(c, -1);
    assert_eq!(r, -1);

    // Length zero with a non-null pointer: both should still return -1.
    let mut buf = [b'y'];
    let c = (libs.c_proc)(buf.as_mut_ptr() as *mut c_char, 0, 1, 1);
    let r = (libs.rust_proc)(buf.as_mut_ptr() as *mut c_char, 0, 1, 1);
    assert_eq!(c, -1);
    assert_eq!(r, -1);
}

#[test]
fn unknown_operation_returns_minus_three() {
    let libs = load_libs();
    for op in [4, 5, 99, -1, i32::MIN, i32::MAX] {
        check(&libs, b"yyy", op, 0);
    }
}

#[test]
fn op0_apply_permissions_all_combos() {
    let libs = load_libs();
    // operation 0 reads the first 3 chars; require length >= 3.
    let chars = [b'y', b'Y', b'n', b'N', b'x', b'0', b' '];
    for &a in &chars {
        for &b in &chars {
            for &c in &chars {
                let buf = [a, b, c];
                check(&libs, &buf, 0, 0);
            }
        }
    }
    // length < 3 -> -2
    check(&libs, b"y", 0, 0);
    check(&libs, b"yn", 0, 0);
}

#[test]
fn op1_evaluate_conditions_all_combos() {
    let libs = load_libs();
    let chars = [b'y', b'Y', b'n', b'N', b'?'];
    for op_param in [0, 1, 2, 3, 4, -1, 99] {
        for &a in &chars {
            for &b in &chars {
                for &c in &chars {
                    let buf = [a, b, c];
                    check(&libs, &buf, 1, op_param);
                }
            }
        }
    }
    check(&libs, b"yn", 1, 0); // length < 3
}

#[test]
fn op2_configure_flags_various() {
    let libs = load_libs();

    // Single character (count == 1)
    let singles: &[&[u8]] = &[b"y", b"n", b"Y", b"N", b"x"];
    for s in singles {
        check(&libs, s, 2, 0);
    }

    // Variety of patterns
    let cases: &[&[u8]] = &[
        b"yy", b"nn", b"yn", b"ny",
        b"yyy", b"nnn", b"yny", b"nyn", b"yyn", b"ynn",
        b"yyyy", b"nnnn", b"ynyn", b"nyny", b"yyyn", b"ynnn",
        b"yyyyy", b"yynnn", b"ynyyn", b"nynyn",
        b"yyyyyy", b"nnnnnn",
        b"yyyyyyy", b"yynyynn",
        // Long: alternating
        b"ynynynynynynynyn",
        // Long: lots of true
        b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy", // 32 chars all y
        b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy", // 33 chars (truncated to 32 internally)
        b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyn", // 32 chars, last is n
        b"nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy", // 32 chars, first is n
        // Consecutive runs
        b"yyynnny",
        b"yynyyy",
        b"yyyy",
        b"nynyyy",
    ];
    for c in cases {
        check(&libs, c, 2, 0);
    }
}

#[test]
fn op3_validate_sequence_various() {
    let libs = load_libs();

    let cases: &[&[u8]] = &[
        b"y",
        b"n", // doesn't start with y
        b"yn",
        b"yy", // doesn't end with n
        b"yny",
        b"ynn",
        b"yyy", // 3 consecutive same? actually 3 is allowed (>3 fails)
        b"yyyy", // 4 consecutive same -> -12
        b"nyn", // doesn't start with y
        b"yyyn",
        b"ynyn",
        b"ynynyn",
        b"yyynnn",
        b"ynynynyn",
        b"ynynynynyn",       // 10
        b"ynynynynynyn",     // 12 long
        b"yynnyynnyyn",      // 11
        b"yynyyn",
        b"yyyyy", // many consecutive
        b"ynnnn", // 4 consecutive n -> -12
        b"yynnyn",
        b"ynnyynn",
        b"yyyynnnn", // 4 consecutive y
        b"ynnynnynn",
        b"ynynnynyn",
        b"y",
        b"Y", // capital
        b"YN",
        // very long
        b"ynynynynynynynynynyn", // 21
        b"yynnyynnyynnyynnyynnyynn", // 24
        b"yyyynnnnynyn", // hits -12
    ];
    for c in cases {
        check(&libs, c, 3, 0);
    }
}

#[test]
fn op2_count_zero_and_count_eq_count_minus_1_edge() {
    let libs = load_libs();
    // count == 1 and decisions[0] == false hits the wrap-around branch
    // (special_count == 0 -> early return 0).
    check(&libs, b"n", 2, 0);
    // count == 1 and decisions[0] == true hits special_count == count branch
    check(&libs, b"y", 2, 0);
}

#[test]
fn op3_empty_sequence_returns_zero() {
    // Empty length already handled by the top-level NULL/length==0 guard
    // returning -1; can't test op==3 with len==0 in isolation.
    // Skip.
}
