// Integration tests that compare the C shared library (ground truth) against
// the Rust shared library through their C ABI by loading both with libloading.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

// Mirror struct layout from C: int (4) + padding (4) + time_t i64 (8) + StatusCode i32 (4) + padding (4) = 24 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct ComputationResult {
    value: c_int,
    _pad1: u32,
    timestamp: i64,
    status: c_int,
    _pad2: u32,
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    // CARGO_MANIFEST_DIR/target/{debug,release}/libmathop_lib.so
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Tests run with the same profile as build; default is `debug`.
    // Try debug first, then release.
    let debug = {
        let mut q = p.clone();
        q.push("debug");
        q.push("libmathop_lib.so");
        q
    };
    if debug.exists() {
        return debug;
    }
    let mut q = p.clone();
    q.push("release");
    q.push("libmathop_lib.so");
    q
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_so_path()).expect("load C lib");
        let r = Library::new(rust_so_path()).expect("load Rust lib");
        (c, r)
    }
}

#[test]
fn test_is_valid_operation() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_char) -> bool> =
            c.get(b"is_valid_operation").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_char) -> bool> =
            r.get(b"is_valid_operation").unwrap();
        for i in -128i32..=127 {
            let arg = i as c_char;
            let cv = cf(arg);
            let rv = rf(arg);
            assert_eq!(cv, rv, "mismatch for char {}", i);
        }
    }
}

#[test]
fn test_get_operation_priority() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c.get(b"get_operation_priority").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r.get(b"get_operation_priority").unwrap();
        for op in 1..=5 {
            assert_eq!(cf(op), rf(op), "op={}", op);
        }
    }
}

#[test]
fn test_arith_operations() {
    let (c, r) = load_libs();
    unsafe {
        let names: &[&[u8]] = &[
            b"add_operation",
            b"multiply_operation",
            b"subtract_operation",
            b"divide_operation",
            b"modulo_operation",
        ];
        for name in names {
            let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
                c.get(name).unwrap();
            let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
                r.get(name).unwrap();
            // skip values that would overflow on add/sub/mul/divide-by-zero
            let cases: &[(c_int, c_int, c_int)] = &[
                (0, 0, 0),
                (1, 2, 3),
                (10, 5, 0),
                (-7, 3, 0),
                (100, 0, 0),
                (-100, -5, 99),
                (12345, 67, 0),
            ];
            for (a, b, u) in cases {
                let cv = cf(*a, *b, *u);
                let rv = rf(*a, *b, *u);
                assert_eq!(
                    cv,
                    rv,
                    "mismatch in {} for ({}, {}, {})",
                    std::str::from_utf8(name).unwrap(),
                    a,
                    b,
                    u
                );
            }
        }
    }
}

#[test]
fn test_select_operation() {
    let (c, r) = load_libs();
    unsafe {
        // select_operation returns a function pointer; we cannot directly
        // compare pointers across libs (they live in different address spaces),
        // but we can call the returned function pointer and verify behavior.
        type Selector =
            unsafe extern "C" fn(c_int) -> unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
        let cf: Symbol<Selector> = c.get(b"select_operation").unwrap();
        let rf: Symbol<Selector> = r.get(b"select_operation").unwrap();
        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            // (op, a, b, unused)
            (1, 2, 3, 0),
            (2, 4, 5, 0),
            (3, 10, 4, 0),
            (4, 20, 5, 0),
            (4, 20, 0, 0),
            (5, 13, 5, 0),
            (5, 13, 0, 0),
        ];
        for (op, a, b, u) in cases {
            let cfp = cf(*op);
            let rfp = rf(*op);
            let cv = cfp(*a, *b, *u);
            let rv = rfp(*a, *b, *u);
            assert_eq!(cv, rv, "mismatch select_operation op={}", op);
        }
    }
}

#[test]
fn test_get_computation_timestamp() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn() -> i64> =
            c.get(b"get_computation_timestamp").unwrap();
        let rf: Symbol<unsafe extern "C" fn() -> i64> =
            r.get(b"get_computation_timestamp").unwrap();
        // The C function calls time() and shifts >> 29. The exact value
        // depends on the wall clock at moment-of-call, so we cannot expect
        // exact byte-equality across two separate invocations. Instead, take
        // a few samples and assert |c - r| <= 1.
        for _ in 0..3 {
            let cv = cf();
            let rv = rf();
            let diff = (cv - rv).abs();
            assert!(diff <= 1, "timestamp drift too large: c={} r={}", cv, rv);
        }
    }
}

#[test]
fn test_allocate_results() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int) -> *mut ComputationResult> =
            c.get(b"allocate_results").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int) -> *mut ComputationResult> =
            r.get(b"allocate_results").unwrap();
        let cp = cf(10);
        let rp = rf(10);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        // calloc zero-fills; verify all bytes are zero.
        let n = 10usize * std::mem::size_of::<ComputationResult>();
        let cb = std::slice::from_raw_parts(cp as *const u8, n);
        let rb = std::slice::from_raw_parts(rp as *const u8, n);
        assert!(cb.iter().all(|&x| x == 0));
        assert!(rb.iter().all(|&x| x == 0));
        libc::free(cp as *mut _);
        libc::free(rp as *mut _);
    }
}

#[test]
fn test_perform_computation_with_history() {
    let (c, r) = load_libs();
    unsafe {
        type Fn = unsafe extern "C" fn(
            c_int,
            c_int,
            c_int,
            *mut *mut ComputationResult,
            *mut c_int,
        ) -> c_int;
        let cf: Symbol<Fn> = c.get(b"perform_computation_with_history").unwrap();
        let rf: Symbol<Fn> = r.get(b"perform_computation_with_history").unwrap();

        let cases: &[(c_int, c_int, c_int)] = &[
            (1, 2, 1),
            (3, 4, 2),
            (10, 4, 3),
            (20, 5, 4),
            (13, 5, 5),
        ];
        let mut c_hist: *mut ComputationResult = std::ptr::null_mut();
        let mut c_count: c_int = 0;
        let mut r_hist: *mut ComputationResult = std::ptr::null_mut();
        let mut r_count: c_int = 0;
        for (a, b, op) in cases {
            let cv = cf(*a, *b, *op, &mut c_hist, &mut c_count);
            let rv = rf(*a, *b, *op, &mut r_hist, &mut r_count);
            assert_eq!(cv, rv, "mismatch perform_computation_with_history return");
            assert_eq!(c_count, r_count, "history_count mismatch");
            // Compare values written into history (skip timestamp because it
            // depends on real-time clock); compare value & status fields.
            for i in 0..(c_count as isize) {
                let ce = &*c_hist.offset(i);
                let re = &*r_hist.offset(i);
                assert_eq!(ce.value, re.value, "history value mismatch idx={}", i);
                assert_eq!(ce.status, re.status, "history status mismatch idx={}", i);
            }
        }
        libc::free(c_hist as *mut _);
        libc::free(r_hist as *mut _);
    }
}

#[test]
fn test_mathop() {
    let (c, r) = load_libs();
    unsafe {
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let cf: Symbol<Fn> = c.get(b"mathop").unwrap();
        let rf: Symbol<Fn> = r.get(b"mathop").unwrap();

        // mathop's return depends on the wall clock (time_modifier from
        // get_computation_timestamp() % 100). The shift >> 29 means the
        // raw timestamp changes only every ~17 minutes; the modulo 100 of
        // that quotient is therefore stable for many minutes. We compare
        // C and Rust calls back-to-back to catch the rare boundary case.
        //
        // Each library has its own static state (history_count, etc), so
        // calling mathop multiple times will diverge after history_count
        // exceeds 10 (which it doesn't, since each call adds 2 entries
        // until clamped). We allow up to 1 small numeric discrepancy if
        // a clock tick happens between the two calls.
        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (49, 5, 0, 1), // op 1 (add), then op 3
            (50, 7, 1, 2), // op 3 (subtract), then op 4
            (51, 3, 2, 0), // op 4 (divide), then op 2
            (52, 4, 3, 4), // op 5 (modulo), then op 1
            (53, 9, 4, 7), // op 1 (add), then op 4
        ];
        for (i, (a, b, p3, p4)) in cases.iter().enumerate() {
            let cv = cf(*a, *b, *p3, *p4);
            let rv = rf(*a, *b, *p3, *p4);
            // Allow up to 1 difference (boundary tick); typically 0.
            let diff = (cv - rv).abs();
            assert!(
                diff <= 1,
                "mathop case {} mismatch: c={} rust={} diff={}",
                i,
                cv,
                rv,
                diff
            );
        }
    }
}
