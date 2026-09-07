//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`. Every defined-behaviour row asserts exact
//! equality of the value or of the printed bytes; the three rows that are
//! undefined behaviour in C are explicitly marked and only assert that neither
//! library faults where the other does not.

mod common;

use std::ffi::{c_char, c_int, CString};
use std::ptr;

mod fork_util {
    use std::ffi::c_int;

    extern "C" {
        fn fork() -> c_int;
        fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum Outcome {
        Exited(c_int),
        Signalled(c_int),
    }

    /// Runs `f` in a forked child and reports how the child terminated.
    /// Used for the undefined-behaviour rows that may legitimately crash.
    pub fn run_isolated<F: FnOnce()>(f: F) -> Outcome {
        unsafe {
            let pid = fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                f();
                // Bypass atexit/stdio teardown so the parent sees a clean code.
                std::process::exit(7);
            }
            let mut status: c_int = 0;
            let r = waitpid(pid, &mut status, 0);
            assert_eq!(r, pid, "waitpid failed");
            if status & 0x7f != 0 {
                Outcome::Signalled(status & 0x7f)
            } else {
                Outcome::Exited((status >> 8) & 0xff)
            }
        }
    }
}

use common::{c, diff_call_fma, diff_driver, diff_fma_array, rs, run_driver_raw, Rng, EXTREMES, SEED};
use fork_util::{run_isolated, Outcome};

const SENTINEL_CAP: usize = 16;

fn sentinel(cap: usize) -> Vec<c_int> {
    (0..cap).map(|i| 0x5A5A_0000u32 as i32 ^ i as i32).collect()
}

// ---------------------------------------------------------------------------
// Row 1 — call_fma: len == 0 (the single explicit guard in the library)
// ---------------------------------------------------------------------------

#[test]
fn err01_call_fma_len_zero() {
    let mut rng = Rng::new(SEED ^ 0x101);
    for rep in 0..200 {
        let n = rng.usize_range(1, 32);
        let data: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let v = diff_call_fma(&data, 0, &format!("err01 rep={rep}"));
        assert_eq!(v, 0, "err01: call_fma(_, 0) must return 0");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — call_fma: len == 0 with a NULL data pointer (guard precedes deref)
// ---------------------------------------------------------------------------

#[test]
fn err02_call_fma_len_zero_null() {
    let cv = unsafe { (c().call_fma)(ptr::null(), 0) };
    let rv = unsafe { (rs().call_fma)(ptr::null(), 0) };
    assert_eq!(cv, 0, "err02: C must return 0");
    assert_eq!(cv, rv, "err02: call_fma(NULL, 0) mismatch");
}

// ---------------------------------------------------------------------------
// Row 3 — fma_array: len == 0 writes nothing
// ---------------------------------------------------------------------------

#[test]
fn err03_fma_len_zero() {
    let mut rng = Rng::new(SEED ^ 0x103);
    for rep in 0..200 {
        let n = rng.usize_range(1, 16);
        let a: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let b: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let k: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let out = diff_fma_array(&a, &b, &k, 0, SENTINEL_CAP, &format!("err03 rep={rep}"));
        assert_eq!(out, sentinel(SENTINEL_CAP), "err03: nothing may be written");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — fma_array: negative len writes nothing
// ---------------------------------------------------------------------------

#[test]
fn err04_fma_negative_len() {
    let a = [1i32, 2, 3, 4];
    let b = [5i32, 6, 7, 8];
    let k = [9i32, 10, 11, 12];
    for &len in &[-1i32, -2, -7, -100, -1000, i32::MIN + 1, i32::MIN] {
        let out = diff_fma_array(&a, &b, &k, len, SENTINEL_CAP, &format!("err04 len={len}"));
        assert_eq!(
            out,
            sentinel(SENTINEL_CAP),
            "err04: negative len={len} must write nothing"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 — fma_array: all-NULL pointers with len == 0 must not dereference
// ---------------------------------------------------------------------------

#[test]
fn err05_fma_null_ptrs_len_zero() {
    for &len in &[0i32, -1, -5, i32::MIN] {
        unsafe {
            (c().fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len);
            (rs().fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len);
        }
    }
    // Reaching here means neither library dereferenced anything.
}

// ---------------------------------------------------------------------------
// Row 6 — fma_array: signed overflow (UB in C; both must wrap identically)
// ---------------------------------------------------------------------------

#[test]
fn err06_fma_overflow_wrap() {
    let cases: &[(i32, i32, i32)] = &[
        (i32::MAX, 2, 0),
        (i32::MAX, i32::MAX, 0),
        (i32::MIN, -1, 0),
        (i32::MIN, i32::MIN, 0),
        (1, 1, i32::MAX),
        (1, -1, i32::MIN),
        (i32::MAX, 1, 1),
        (i32::MIN, 1, -1),
        (65536, 65536, 0),
        (-65536, 65536, i32::MAX),
    ];
    for (i, &(a, b, k)) in cases.iter().enumerate() {
        let out = diff_fma_array(&[a], &[b], &[k], 1, 4, &format!("err06 case={i}"));
        assert_eq!(
            out[0],
            a.wrapping_mul(b).wrapping_add(k),
            "err06: case {i} ({a}*{b}+{k}) is not two's-complement wrapping"
        );
    }
    // Randomized overflow sweep.
    let mut rng = Rng::new(SEED ^ 0x106);
    for rep in 0..500 {
        let n = rng.usize_range(1, 8);
        let a: Vec<c_int> = (0..n).map(|_| *rng.pick(&EXTREMES)).collect();
        let b: Vec<c_int> = (0..n).map(|_| *rng.pick(&EXTREMES)).collect();
        let k: Vec<c_int> = (0..n).map(|_| *rng.pick(&EXTREMES)).collect();
        diff_fma_array(&a, &b, &k, n as i32, n + 4, &format!("err06 rand rep={rep}"));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — driver: empty string, sscanf returns EOF
// ---------------------------------------------------------------------------

#[test]
fn err07_driver_empty() {
    let out = diff_driver("", "err07 empty");
    assert_eq!(out, b"0\n", "err07: empty input must print 0");
}

// ---------------------------------------------------------------------------
// Row 8 — driver: non-convertible leading char, sscanf returns 0
// ---------------------------------------------------------------------------

#[test]
fn err08_driver_no_leading_int() {
    for s in [
        "abc", ",", "x1", ".5", "e10", "#", "]", "/", "*", "\\", "'", "\"", "abc 1 2 3", ",,,,",
        "x", "X", "null", "NaN", "inf", "0x", ":", ";", "@", "~",
    ] {
        let out = diff_driver(s, &format!("err08 {s:?}"));
        assert_eq!(out, b"0\n", "err08: {s:?} must print 0");
    }
    // Randomized: any non-digit, non-sign, non-space first byte rejects.
    let mut rng = Rng::new(SEED ^ 0x108);
    for rep in 0..300 {
        let first = loop {
            let b = rng.range(0x21, 0x7e) as u8;
            if !b.is_ascii_digit() && b != b'+' && b != b'-' {
                break b;
            }
        };
        let mut s = String::new();
        s.push(first as char);
        let n = rng.usize_range(0, 8);
        for _ in 0..n {
            s.push(*rng.pick(b"0123456789 -+") as char);
        }
        let out = diff_driver(&s, &format!("err08 rand rep={rep} {s:?}"));
        assert_eq!(out, b"0\n", "err08 rand: {s:?} must print 0");
    }
}

// ---------------------------------------------------------------------------
// Row 9 — driver: whitespace only
// ---------------------------------------------------------------------------

#[test]
fn err09_driver_whitespace_only() {
    for s in [
        " ",
        "  ",
        "\t",
        "\n",
        "\r",
        "\u{b}",
        "\u{c}",
        " \t\n\r\u{b}\u{c} ",
        "                    ",
    ] {
        let out = diff_driver(s, &format!("err09 {s:?}"));
        assert_eq!(out, b"0\n", "err09: {s:?} must print 0");
    }
}

// ---------------------------------------------------------------------------
// Row 10 — driver: sign with no digits
// ---------------------------------------------------------------------------

#[test]
fn err10_driver_lone_sign() {
    for s in ["-", "+", "- 5", "+ 5", "--3", "++3", "-+3", "+-3", "  -  ", "-\n5", "-a"] {
        let out = diff_driver(s, &format!("err10 {s:?}"));
        assert_eq!(out, b"0\n", "err10: {s:?} must print 0");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — driver: valid prefix then garbage
// ---------------------------------------------------------------------------

#[test]
fn err11_driver_garbage_tail() {
    let cases: &[(&str, i32)] = &[
        ("1 2 x", 2),
        ("5,6", 5),
        ("7 8 9 abc 10", 9),
        ("42;99", 42),
        ("1 2 3 .4", 3),
        ("-8 e", -8),
        // "100 0x20": %d reads 100, then reads 0 and stops at 'x' -> last value is 0
        ("100 0x20", 0),
    ];
    for &(s, expect) in cases {
        let out = diff_driver(s, &format!("err11 {s:?}"));
        assert_eq!(out, format!("{expect}\n").into_bytes(), "err11: {s:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — driver: more than 100 integers
// ---------------------------------------------------------------------------

#[test]
fn err12_driver_over_100() {
    let mut rng = Rng::new(SEED ^ 0x112);
    for &n in &[101usize, 102, 150, 250, 1000] {
        let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let s = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        let out = diff_driver(&s, &format!("err12 n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[99]).into_bytes(),
            "err12: with n={n} only the first 100 are parsed"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — driver: literals outside the int range
// ---------------------------------------------------------------------------

#[test]
fn err13_driver_out_of_range_int() {
    for s in [
        "99999999999",
        "-99999999999",
        "2147483648",
        "-2147483649",
        "4294967296",
        "9223372036854775808",
        "-9223372036854775809",
        "123456789012345678901234567890",
        "2147483648 5",
        "1 99999999999",
        "00000000002147483648",
    ] {
        diff_driver(s, &format!("err13 {s:?}"));
    }
    // Randomized: long digit runs well past 2^31.
    let mut rng = Rng::new(SEED ^ 0x113);
    for rep in 0..300 {
        let digits = rng.usize_range(11, 30);
        let mut s = String::new();
        if rng.bool() {
            s.push('-');
        }
        s.push(*rng.pick(b"123456789") as char);
        for _ in 1..digits {
            s.push(*rng.pick(b"0123456789") as char);
        }
        diff_driver(&s, &format!("err13 rand rep={rep} {s:?}"));
    }
}

// ---------------------------------------------------------------------------
// Row 14 — driver: hex-looking input ("%d" is decimal only)
// ---------------------------------------------------------------------------

#[test]
fn err14_driver_hex_like() {
    for (s, expect) in [
        ("0x10", 0),
        ("0X10", 0),
        ("0xff", 0),
        ("-0x1", 0),
        // "1 0x10": second conversion yields 0 (stops at 'x'), so 0 is printed
        ("1 0x10", 0),
        ("0x", 0),
        ("0b101", 0),
        ("0o17", 0),
    ] {
        let out = diff_driver(s, &format!("err14 {s:?}"));
        assert_eq!(out, format!("{expect}\n").into_bytes(), "err14: {s:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 15 — driver: float-looking input
// ---------------------------------------------------------------------------

#[test]
fn err15_driver_float_like() {
    for (s, expect) in [
        ("1e5", 1),
        ("1.5", 1),
        ("-2.75", -2),
        ("3E4", 3),
        ("1.2 3.4", 1),
        ("0.0", 0),
        ("5.", 5),
        (".5", 0),
    ] {
        let out = diff_driver(s, &format!("err15 {s:?}"));
        assert_eq!(out, format!("{expect}\n").into_bytes(), "err15: {s:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 16 — driver: exactly 100 integers (loop-bound boundary)
// ---------------------------------------------------------------------------

#[test]
fn err16_driver_exactly_100() {
    let mut rng = Rng::new(SEED ^ 0x116);
    for rep in 0..20 {
        let vals: Vec<i32> = (0..100).map(|_| rng.next_i32()).collect();
        let s = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        let out = diff_driver(&s, &format!("err16 rep={rep}"));
        assert_eq!(out, format!("{}\n", vals[99]).into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 17 — driver: exactly 101 integers (one past the bound)
// ---------------------------------------------------------------------------

#[test]
fn err17_driver_101() {
    let mut rng = Rng::new(SEED ^ 0x117);
    for rep in 0..20 {
        let vals: Vec<i32> = (0..101).map(|_| rng.next_i32()).collect();
        let s = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        let out = diff_driver(&s, &format!("err17 rep={rep}"));
        assert_eq!(
            out,
            format!("{}\n", vals[99]).into_bytes(),
            "err17: the 101st integer must be ignored"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 18 — driver: very long input, and interior NUL terminates parsing
// ---------------------------------------------------------------------------

#[test]
fn err18_driver_long_input() {
    // ~10 KB of integers.
    let mut rng = Rng::new(SEED ^ 0x118);
    for rep in 0..8 {
        let n = rng.usize_range(1500, 2500);
        let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let s = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
        let out = diff_driver(&s, &format!("err18 rep={rep} bytes={}", s.len()));
        assert_eq!(out, format!("{}\n", vals[99]).into_bytes());
    }

    // A single enormous digit run.
    let huge: String = std::iter::repeat('9').take(5000).collect();
    diff_driver(&huge, "err18 huge digit run");

    // Interior NUL: parsing must stop at the terminator.
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"1 2 3\0 4 5 6\0");
    let c_out = run_driver_raw(c(), &buf);
    let r_out = run_driver_raw(rs(), &buf);
    assert_eq!(c_out, r_out, "err18: interior-NUL mismatch");
    assert_eq!(c_out, b"3\n", "err18: parsing must stop at the first NUL");

    // Just a NUL.
    let c_out = run_driver_raw(c(), b"\0");
    let r_out = run_driver_raw(rs(), b"\0");
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, b"0\n");
}

// ---------------------------------------------------------------------------
// Row 19 — call_fma: len == 1 (one step past the len == 0 guard)
// ---------------------------------------------------------------------------

#[test]
fn err19_call_fma_len_one() {
    let mut rng = Rng::new(SEED ^ 0x119);
    for rep in 0..200 {
        let v = rng.next_i32();
        let got = diff_call_fma(&[v], 1, &format!("err19 rep={rep}"));
        assert_eq!(got, v, "err19: call_fma(&[v], 1) must return v");
    }
    for &v in EXTREMES.iter() {
        assert_eq!(diff_call_fma(&[v], 1, "err19 extreme"), v);
    }
}

// ---------------------------------------------------------------------------
// Row 20 — no enum exists in the public API; documented in ERRORS.md.
// The equivalent "any int is accepted" boundary is covered by rows 1/3/4/19/21.
// ---------------------------------------------------------------------------

#[test]
fn err20_no_enum_in_public_api() {
    // Static assertion about the surface: driver.h declares one function taking
    // a `const char *`; there is no enum/flags parameter anywhere, so there is
    // no invalid-variant case to differentiate. Assert the header really is
    // that small so this row cannot silently rot.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");
    assert!(
        !header.contains("enum"),
        "err20: driver.h grew an enum — ERRORS.md row 20 must be revisited"
    );
    // The unconstrained `int len` boundary, swept densely instead.
    let data: Vec<c_int> = (0..8).map(|i| 1000 + i).collect();
    for len in -4i32..=8 {
        if len < 0 {
            continue; // UB, see err21
        }
        let cv = unsafe { (c().call_fma)(data.as_ptr(), len) };
        let rv = unsafe { (rs().call_fma)(data.as_ptr(), len) };
        assert_eq!(cv, rv, "err20: call_fma len={len} mismatch");
    }
    for len in -4i32..=8 {
        diff_fma_array(&data, &data, &data, len, 12, &format!("err20 fma len={len}"));
    }
}

// ---------------------------------------------------------------------------
// Row 21 — call_fma with negative len: negative-size VLA, UNDEFINED BEHAVIOUR.
// The C result is uninitialised stack memory (observed to differ between two
// identical calls), so no byte-identical comparison is possible. The test
// pins down what IS observable: neither library aborts the process.
// ---------------------------------------------------------------------------

#[test]
fn err21_call_fma_negative_len_ub() {
    let data = [11i32, 22, 33, 44];
    for &len in &[-1i32, -2, -3, -8] {
        let c_out = run_isolated(|| {
            let _ = unsafe { (c().call_fma)(data.as_ptr(), len) };
        });
        let r_out = run_isolated(|| {
            let _ = unsafe { (rs().call_fma)(data.as_ptr(), len) };
        });
        assert_eq!(
            c_out,
            Outcome::Exited(7),
            "err21: C build unexpectedly died for len={len}"
        );
        assert_eq!(
            r_out, c_out,
            "err21: len={len} — Rust must survive exactly where C survives \
             (the returned value itself is uninitialised stack memory in C and \
             is documented in ERRORS.md as not byte-comparable)"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 22 — driver(NULL): glibc dereferences the pointer. UNDEFINED BEHAVIOUR;
// asserted out-of-process so both libraries must fail the SAME way.
// ---------------------------------------------------------------------------

#[test]
fn err22_driver_null_ub() {
    let c_out = run_isolated(|| unsafe { (c().driver)(ptr::null()) });
    let r_out = run_isolated(|| unsafe { (rs().driver)(ptr::null()) });
    assert_eq!(
        c_out, r_out,
        "err22: driver(NULL) must terminate the same way in both builds \
         (C={c_out:?}, Rust={r_out:?})"
    );
}

// ---------------------------------------------------------------------------
// Extra generic FFI boundaries (not distinct ERRORS.md rows, but required).
// ---------------------------------------------------------------------------

#[test]
fn extra_boundaries_zero_and_oversized_lengths() {
    // fma_array with a valid buffer but len exactly equal to the capacity, and
    // len == capacity - 1 / + 0 boundaries.
    let mut rng = Rng::new(SEED ^ 0x1FF);
    for rep in 0..100 {
        let cap = rng.usize_range(1, 32);
        let a: Vec<c_int> = (0..cap).map(|_| rng.next_i32()).collect();
        let b: Vec<c_int> = (0..cap).map(|_| rng.next_i32()).collect();
        let k: Vec<c_int> = (0..cap).map(|_| rng.next_i32()).collect();
        for len in [0usize, cap.saturating_sub(1), cap] {
            diff_fma_array(&a, &b, &k, len as i32, cap, &format!("extra rep={rep} len={len}"));
        }
    }
}

#[test]
fn extra_driver_single_byte_inputs() {
    // Every single ASCII byte as the whole input.
    for b in 1u8..=127 {
        let s = (b as char).to_string();
        diff_driver(&s, &format!("extra single byte {b:#04x}"));
    }
}

#[test]
fn extra_driver_high_bytes() {
    // Bytes >= 0x80 are not whitespace/digits for the C locale; make sure both
    // reject identically. Passed as a raw NUL-terminated buffer because they
    // are not valid UTF-8.
    for b in 128u8..=255 {
        let buf = [b, 0u8];
        let c_out = run_driver_raw(c(), &buf);
        let r_out = run_driver_raw(rs(), &buf);
        assert_eq!(c_out, r_out, "extra high byte {b:#04x} mismatch");
        let buf = [b'4', b'2', b, 0u8];
        let c_out = run_driver_raw(c(), &buf);
        let r_out = run_driver_raw(rs(), &buf);
        assert_eq!(c_out, r_out, "extra '42'+{b:#04x} mismatch");
    }
}

#[test]
fn extra_call_fma_and_driver_agree_on_empty() {
    // The composed empty case: no integers parsed -> call_fma(_, 0) -> 0.
    let cs = CString::new("").unwrap();
    let c_out = common::capture_stdout(|| unsafe { (c().driver)(cs.as_ptr() as *const c_char) });
    let r_out = common::capture_stdout(|| unsafe { (rs().driver)(cs.as_ptr() as *const c_char) });
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, b"0\n");
}
