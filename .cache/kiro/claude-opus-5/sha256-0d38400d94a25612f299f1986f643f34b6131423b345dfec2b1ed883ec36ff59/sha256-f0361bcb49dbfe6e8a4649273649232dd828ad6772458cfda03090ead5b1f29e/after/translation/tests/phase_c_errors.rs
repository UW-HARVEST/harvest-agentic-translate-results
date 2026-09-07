// Phase C -- error-path differential tests.
//
// One test per row of ERRORS.md. Each asserts that the C and the Rust return
// the SAME sentinel (1) *and* emit the SAME error message on stdout -- not
// merely "both failed".

mod common;

use common::{assert_same, assert_same_and, diff_call, libs, ChildExit, Rng};
use std::ffi::{c_char, c_int};

const E_START: &[u8] = b"Error: start is off the end of the string!\n";
const E_STOP_END: &[u8] = b"Error: stop is off the end of the string!\n";
const E_STOP_ORDER: &[u8] = b"Error: stop must come after start!\n";

// --- E1 -------------------------------------------------------------------
#[test]
fn e1_start_off_end_positive() {
    let mut rng = Rng::new(0xE1);
    for _ in 0..400 {
        let len = rng.range(0, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        let over = rng.range(len + 1, len + 500) as c_int;
        assert_same_and("E1", &s, Some(over), None, 1, E_START);
        // and with a stop pointer present: the start check still fires first
        assert_same_and("E1/stop", &s, Some(over), Some(0), 1, E_START);
    }
}

// --- E2 -------------------------------------------------------------------
#[test]
fn e2_start_negative() {
    // A negative int promotes to a huge size_t in `start > len`, so it is
    // rejected as "off the end", NOT interpreted as a Python-style index.
    let mut rng = Rng::new(0xE2);
    for _ in 0..400 {
        let len = rng.range(0, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        let neg = -(rng.range(1, 100000) as c_int);
        assert_same_and("E2", &s, Some(neg), None, 1, E_START);
        assert_same_and("E2/stop", &s, Some(neg), Some(len as c_int), 1, E_START);
    }
    assert_same_and("E2/-1", b"hello", Some(-1), None, 1, E_START);
}

// --- E3 -------------------------------------------------------------------
#[test]
fn e3_start_int_extremes() {
    for s in [b"".as_ref(), b"a".as_ref(), b"hello world".as_ref()] {
        for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            assert_same_and("E3", s, Some(v), None, 1, E_START);
            assert_same_and("E3/stop", s, Some(v), Some(1), 1, E_START);
        }
    }
}

// --- E4 -------------------------------------------------------------------
#[test]
fn e4_stop_off_end_positive() {
    let mut rng = Rng::new(0xE4);
    for _ in 0..400 {
        let len = rng.range(0, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        let over = rng.range(len + 1, len + 500) as c_int;
        // start omitted
        assert_same_and("E4/nostart", &s, None, Some(over), 1, E_STOP_END);
        // start present and valid
        let start = rng.range(0, len) as c_int;
        assert_same_and("E4", &s, Some(start), Some(over), 1, E_STOP_END);
    }
}

// --- E5 -------------------------------------------------------------------
#[test]
fn e5_stop_negative_precedence() {
    // Key ordering property: `stop > len` (unsigned) is checked BEFORE
    // `stop <= start` (signed). A negative stop is therefore reported as
    // "off the end", never as "must come after start".
    let mut rng = Rng::new(0xE5);
    for _ in 0..400 {
        let len = rng.range(1, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        let neg = -(rng.range(1, 100000) as c_int);
        let start = rng.range(0, len) as c_int;
        assert_same_and("E5", &s, Some(start), Some(neg), 1, E_STOP_END);
        assert_same_and("E5/nostart", &s, None, Some(neg), 1, E_STOP_END);
    }
    // Explicit: stop=-1 with start=0 would satisfy `stop <= start` too, but the
    // "off the end" branch wins.
    assert_same_and("E5/pin", b"hello", Some(0), Some(-1), 1, E_STOP_END);
}

// --- E6 -------------------------------------------------------------------
#[test]
fn e6_stop_int_extremes() {
    for s in [b"".as_ref(), b"a".as_ref(), b"hello world".as_ref()] {
        for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            assert_same_and("E6/nostart", s, None, Some(v), 1, E_STOP_END);
            assert_same_and("E6", s, Some(0), Some(v), 1, E_STOP_END);
        }
    }
}

// --- E7 -------------------------------------------------------------------
#[test]
fn e7_stop_before_start() {
    let mut rng = Rng::new(0xE7);
    for _ in 0..400 {
        let len = rng.range(2, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        let start = rng.range(1, len);
        let stop = rng.range(0, start - 1);
        assert_same_and("E7", &s, Some(start as c_int), Some(stop as c_int), 1, E_STOP_ORDER);
    }
}

// --- E8 -------------------------------------------------------------------
#[test]
fn e8_stop_equals_start() {
    // `stop <= start` uses <=, so a zero-width explicit slice is an ERROR
    // (unlike the implicit zero-width slice from start == len with stop NULL,
    // which is accepted -- see C7).
    let mut rng = Rng::new(0xE8);
    for _ in 0..400 {
        let len = rng.range(0, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        let v = rng.range(0, len) as c_int;
        assert_same_and("E8", &s, Some(v), Some(v), 1, E_STOP_ORDER);
    }
}

// --- E9 -------------------------------------------------------------------
#[test]
fn e9_null_start_zero_stop() {
    let mut rng = Rng::new(0xE9);
    for _ in 0..200 {
        let len = rng.range(0, 64);
        let s = rng.bytes(len, 0x01, 0xff);
        // start_ptr NULL => start = 0; stop = 0 => 0 <= 0 => order error
        assert_same_and("E9", &s, None, Some(0), 1, E_STOP_ORDER);
    }
}

// --- E10 ------------------------------------------------------------------
#[test]
fn e10_empty_string_zero_stop() {
    // len == 0: `0 > 0` is false so the range check passes, then `0 <= 0`
    // trips the order check.
    assert_same_and("E10", b"", Some(0), Some(0), 1, E_STOP_ORDER);
    assert_same_and("E10/nostart", b"", None, Some(0), 1, E_STOP_ORDER);
    // any non-zero stop on an empty string is off the end instead
    for v in [1, 2, 100, i32::MAX] {
        assert_same_and("E10/over", b"", Some(0), Some(v), 1, E_STOP_END);
    }
}

// --- E11 ------------------------------------------------------------------
#[test]
fn e11_start_error_wins() {
    // Both indices invalid: only the start message is printed, and `stop_ptr`
    // is never dereferenced.
    let mut rng = Rng::new(0xEB);
    for _ in 0..200 {
        let len = rng.range(0, 40);
        let s = rng.bytes(len, 0x01, 0xff);
        let bad_start = rng.range(len + 1, len + 200) as c_int;
        let bad_stop = rng.range(len + 1, len + 200) as c_int;
        assert_same_and("E11", &s, Some(bad_start), Some(bad_stop), 1, E_START);
        assert_same_and("E11/neg", &s, Some(-1), Some(-1), 1, E_START);
    }

    // stop_ptr must not be read at all when start already failed: pass a
    // deliberately invalid (non-null but unwritable) stop pointer is UB, so
    // instead assert via a sentinel value that never appears in the output.
    let f = libs();
    for fun in [f.c_slice, f.rust_slice] {
        let mut buf = b"abc\0".to_vec();
        let mut start: c_int = 99;
        let mut stop: c_int = 1;
        let (ret, out) = common::capture_stdout(|| unsafe {
            fun(buf.as_mut_ptr() as *mut c_char, &mut start, &mut stop)
        });
        assert_eq!(ret, 1);
        assert_eq!(out, E_START);
    }
}

// --- E12 ------------------------------------------------------------------
#[test]
fn e13_one_past_boundary() {
    // `len` itself is a valid index for start; `len + 1` is the first invalid
    // one. For stop, `len` is valid (if > start) and `len + 1` is invalid.
    let mut rng = Rng::new(0xEC);
    for _ in 0..200 {
        let len = rng.range(0, 64);
        let s = rng.bytes(len, 0x01, 0xff);

        // start: len valid, len+1 invalid
        assert_same("E12/start-at-len", &s, Some(len as c_int), None);
        assert_same_and("E12/start-past", &s, Some(len as c_int + 1), None, 1, E_START);

        // stop: len valid when len > 0 (start 0), len+1 invalid
        if len > 0 {
            assert_same("E12/stop-at-len", &s, Some(0), Some(len as c_int));
        }
        assert_same_and("E12/stop-past", &s, Some(0), Some(len as c_int + 1), 1, E_STOP_END);
    }
}

// --- generic FFI boundary sweep -------------------------------------------
#[test]
fn generic_boundary_sweep() {
    // Every combination of {NULL, INT_MIN, -1, 0, 1, len-1, len, len+1, INT_MAX}
    // for both index pointers, over several string shapes. This subsumes the
    // "out-of-range value with no valid variant crosses the FFI boundary"
    // class -- the API has no enums, so the analogue is arbitrary ints.
    let shapes: [&[u8]; 5] = [b"", b"x", b"ab", b"hello", b"\xff\x00\x01ok"];
    for s in shapes {
        // note: the 4th shape contains an interior NUL, so its C length is 1.
        let len = s.iter().position(|&b| b == 0).unwrap_or(s.len()) as i64;
        let mut vals: Vec<Option<c_int>> = vec![None];
        for v in [
            i32::MIN as i64,
            -2,
            -1,
            0,
            1,
            len - 1,
            len,
            len + 1,
            len + 2,
            i32::MAX as i64,
        ] {
            vals.push(Some(v.clamp(i32::MIN as i64, i32::MAX as i64) as c_int));
        }
        for a in &vals {
            for b in &vals {
                assert_same("generic", s, *a, *b);
            }
        }
    }
}

// --- U1: NULL string is UB in the C -- compare crash behaviour ------------
#[test]
fn u1_null_string_both_crash_identically() {
    // The C does no null check: `strlen(NULL)` faults. Verify the Rust faults
    // the same way rather than, say, returning an error code. Run each side in
    // a forked child so the test process survives.
    let f = libs();
    let c_exit = common::run_in_child(|| unsafe {
        (f.c_slice)(core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut());
    });
    let r_exit = common::run_in_child(|| unsafe {
        (f.rust_slice)(core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut());
    });
    assert_eq!(
        c_exit, r_exit,
        "NULL mystr: C terminated as {c_exit:?} but Rust as {r_exit:?}"
    );
    assert!(
        matches!(c_exit, ChildExit::Signalled(11)),
        "expected SIGSEGV from the C reference, got {c_exit:?}"
    );

    // Same check with non-null index pointers, in case argument handling differs.
    let c2 = common::run_in_child(|| unsafe {
        let mut a: c_int = 0;
        let mut b: c_int = 1;
        (f.c_slice)(core::ptr::null_mut(), &mut a, &mut b);
    });
    let r2 = common::run_in_child(|| unsafe {
        let mut a: c_int = 0;
        let mut b: c_int = 1;
        (f.rust_slice)(core::ptr::null_mut(), &mut a, &mut b);
    });
    assert_eq!(c2, r2, "NULL mystr with indices: C {c2:?} vs Rust {r2:?}");
}

// --- return value is exactly the C sentinel set ---------------------------
#[test]
fn return_values_are_only_0_or_1() {
    let mut rng = Rng::new(0xF0);
    for _ in 0..2000 {
        let len = rng.range(0, 32);
        let s = rng.bytes(len, 0x01, 0xff);
        let a = if rng.bool() { Some(rng.i32()) } else { None };
        let b = if rng.bool() { Some(rng.i32()) } else { None };
        let (c, r) = diff_call(&s, a, b);
        assert_eq!(c, r, "divergence: s.len={len} a={a:?} b={b:?}");
        assert!(c.ret == 0 || c.ret == 1, "C returned unexpected {}", c.ret);
    }
}
