// Phase C — error-path differential tests.
// One test per row of ERRORS.md, plus the generic boundary cases.

mod common;

use common::*;

const ERR: &str = "An error occurred\n";

/// Assert both libraries reject `input` in exactly the same way: identical
/// stdout, and that stdout is precisely the C error sentinel.
fn diff_reject(input: &[u8], row: &str) {
    let l = libs();
    let cs = std::ffi::CString::new(input).expect("interior NUL");
    let c_out = {
        let f = l.c.driver();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    };
    let r_out = {
        let f = l.rust.driver();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    };
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "[{row}] C/Rust divergence for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(c_out, r_out, "[{row}] raw byte divergence");
    assert_eq!(
        show(&c_out),
        ERR,
        "[{row}] expected the C rejection sentinel for input {:?}",
        String::from_utf8_lossy(input)
    );
}

/// Assert both libraries ACCEPT `input` identically (used for the
/// one-step-inside-the-range boundaries).
fn diff_accept(input: &[u8], row: &str) {
    let l = libs();
    let cs = std::ffi::CString::new(input).expect("interior NUL");
    let c_out = {
        let f = l.c.driver();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    };
    let r_out = {
        let f = l.rust.driver();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    };
    assert_eq!(show(&c_out), show(&r_out), "[{row}] divergence");
    assert_eq!(c_out, r_out, "[{row}] raw byte divergence");
    assert_ne!(
        show(&c_out),
        ERR,
        "[{row}] C was expected to ACCEPT {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        show(&c_out).lines().count(),
        8,
        "[{row}] accepted path must print 8 lines (2 runs x 4)"
    );
}

// --- Row 1: empty string, endp == str --------------------------------------
#[test]
fn row1_empty_string() {
    diff_reject(b"", "ERRORS row 1");
}

// --- Row 2: whitespace only ------------------------------------------------
#[test]
fn row2_whitespace_only() {
    for s in [
        " ", "  ", "\t", "\n", "\r", "\x0b", "\x0c", "\t\n\x0b\x0c\r ", "     ",
    ] {
        diff_reject(s.as_bytes(), "ERRORS row 2");
    }
}

// --- Row 3: leading non-numeric --------------------------------------------
#[test]
fn row3_leading_non_numeric() {
    for s in [
        "abc", "x12", "!5", "+", "-", "++1", "--1", "+-1", ".", ".5", " -", " +", "e", "E", "nan",
        "inf", "NULL", "_1", "(1)", "\"42\"", "'4'", "\\1",
    ] {
        diff_reject(s.as_bytes(), "ERRORS row 3");
    }
}

// --- Row 4: chars adjacent to digits in ASCII ------------------------------
#[test]
fn row4_ascii_neighbors_of_digits() {
    // '/' == 0x2f (just below '0'), ':' == 0x3a (just above '9')
    for s in ["/", ":", "#", "e5", "/1", ":9", ";", "<", "=", ">", "?", "@", "*", "%"] {
        diff_reject(s.as_bytes(), "ERRORS row 4");
    }
}

// --- Row 5: ERANGE above LONG_MAX -----------------------------------------
#[test]
fn row5_erange_above_long_max() {
    for s in [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999999",
        "18446744073709551616",
        "+9223372036854775808",
    ] {
        diff_reject(s.as_bytes(), "ERRORS row 5");
    }
    // oversized: 4096-digit numeral (B7)
    let big = "9".repeat(4096);
    diff_reject(big.as_bytes(), "ERRORS row 5 / B7");
}

// --- Row 6: ERANGE below LONG_MIN -----------------------------------------
#[test]
fn row6_erange_below_long_min() {
    for s in [
        "-9223372036854775809",
        "-9223372036854775810",
        "-99999999999999999999999",
        "-18446744073709551616",
    ] {
        diff_reject(s.as_bytes(), "ERRORS row 6");
    }
    let big = format!("-{}", "9".repeat(4096));
    diff_reject(big.as_bytes(), "ERRORS row 6 / B7");
}

// --- Row 7: > INT_MAX but within long -------------------------------------
#[test]
fn row7_above_int_max() {
    for s in [
        "2147483648",
        "2147483649",
        "2147483700",
        "4294967295",
        "4294967296",
        "9223372036854775806",
        "9223372036854775807", // exactly LONG_MAX: no ERANGE, rejected by range check
        "+2147483648",
        "0002147483648",
        " 2147483648abc",
    ] {
        diff_reject(s.as_bytes(), "ERRORS row 7");
    }
}

// --- Row 8: < INT_MIN but within long -------------------------------------
#[test]
fn row8_below_int_min() {
    for s in [
        "-2147483649",
        "-2147483650",
        "-4294967296",
        "-9223372036854775807",
        "-9223372036854775808", // exactly LONG_MIN: no ERANGE, rejected by range check
        "-0002147483649",
        " -2147483649xyz",
    ] {
        diff_reject(s.as_bytes(), "ERRORS row 8");
    }
}

// --- Row 9: stale errno must not cause rejection --------------------------
#[test]
fn row9_stale_errno_does_not_reject() {
    let l = libs();
    for &e in &[34 /*ERANGE*/, 22, 2, 1, i32::MAX, -1] {
        let cs = std::ffi::CString::new("123").unwrap();
        let c_out = {
            let f = l.c.driver();
            capture_stdout(|| {
                set_errno(e);
                unsafe { f(cs.as_ptr()) }
            })
        };
        let r_out = {
            let f = l.rust.driver();
            capture_stdout(|| {
                set_errno(e);
                unsafe { f(cs.as_ptr()) }
            })
        };
        assert_eq!(show(&c_out), show(&r_out), "[row 9] divergence errno={e}");
        assert_ne!(show(&c_out), ERR, "[row 9] C must accept with errno={e}");
    }
    // And the converse: a rejected input must be rejected regardless of errno.
    for &e in &[0, 34, 22] {
        let cs = std::ffi::CString::new("abc").unwrap();
        let c_out = {
            let f = l.c.driver();
            capture_stdout(|| {
                set_errno(e);
                unsafe { f(cs.as_ptr()) }
            })
        };
        let r_out = {
            let f = l.rust.driver();
            capture_stdout(|| {
                set_errno(e);
                unsafe { f(cs.as_ptr()) }
            })
        };
        assert_eq!(show(&c_out), show(&r_out), "[row 9b] divergence errno={e}");
        assert_eq!(show(&c_out), ERR, "[row 9b] expected rejection");
    }
    set_errno(0);
}

// --- errno as an observable FFI side effect --------------------------------
// `errno` is part of the C ABI contract: the caller can read it after the call.
// The C code sets `errno = 0` and then lets `strtol` set it, so the value left
// behind is observable and must match. This also pins the *order* of the
// `errno == 0` conjunct relative to the range checks.
#[test]
fn errno_left_behind_matches() {
    let l = libs();
    let inputs: [&str; 12] = [
        "42",
        "",
        "abc",
        " ",
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "9223372036854775807",
        "9223372036854775808",  // ERANGE
        "-9223372036854775809", // ERANGE
        "99999999999999999999999999",
    ];
    for s in inputs {
        let cs = std::ffi::CString::new(s).unwrap();
        let (c_out, c_errno) = {
            let f = l.c.driver();
            let mut e = 0;
            let out = capture_stdout(|| {
                set_errno(12345);
                unsafe { f(cs.as_ptr()) };
                e = get_errno();
            });
            (out, e)
        };
        let (r_out, r_errno) = {
            let f = l.rust.driver();
            let mut e = 0;
            let out = capture_stdout(|| {
                set_errno(12345);
                unsafe { f(cs.as_ptr()) };
                e = get_errno();
            });
            (out, e)
        };
        assert_eq!(show(&c_out), show(&r_out), "[errno] stdout diverged for {s:?}");
        assert_eq!(
            c_errno, r_errno,
            "[errno] errno left behind diverged for {s:?}: C={c_errno} Rust={r_errno}"
        );
    }
    set_errno(0);
}

// --- Row 10: driver(NULL) — no null check in C, UB ------------------------
#[test]
fn row10_driver_null_pointer() {
    let l = libs();
    let c_outcome = {
        let f = l.c.driver();
        fork_outcome(|| unsafe { f(std::ptr::null()) })
    };
    let r_outcome = {
        let f = l.rust.driver();
        fork_outcome(|| unsafe { f(std::ptr::null()) })
    };
    assert_eq!(
        c_outcome, r_outcome,
        "[row 10] driver(NULL): C {c_outcome:?} vs Rust {r_outcome:?}"
    );
}

// --- Row 11: run(NULL) — no null check in C, UB ---------------------------
#[test]
fn row11_run_null_pointer() {
    let l = libs();
    for extra in [0i32, 7, i32::MAX, i32::MIN] {
        let c_outcome = {
            let f = l.c.run();
            fork_outcome(|| unsafe { f(std::ptr::null_mut(), extra) })
        };
        let r_outcome = {
            let f = l.rust.run();
            fork_outcome(|| unsafe { f(std::ptr::null_mut(), extra) })
        };
        assert_eq!(
            c_outcome, r_outcome,
            "[row 11] run(NULL, {extra}): C {c_outcome:?} vs Rust {r_outcome:?}"
        );
    }
}

// --- Boundaries B1-B5: last-valid / first-invalid pairs -------------------
#[test]
fn boundaries_int_min_max_one_step() {
    diff_accept(b"2147483647", "B1 INT_MAX accepted");
    diff_reject(b"2147483648", "B2 INT_MAX+1 rejected");
    diff_accept(b"-2147483648", "B3 INT_MIN accepted");
    diff_reject(b"-2147483649", "B4 INT_MIN-1 rejected");
    diff_accept(b"0", "B5 zero");
    diff_accept(b"-0", "B5 minus zero");
    diff_accept(b"+0", "B5 plus zero");
}

// --- B9: non-UTF-8 / high-bit bytes --------------------------------------
#[test]
fn boundary_non_utf8_input() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0xff, 0xfe],
        vec![0x80, b'4', b'2'],
        vec![0xc3],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xff; 32],
    ];
    for (i, c) in cases.iter().enumerate() {
        diff_reject(c, &format!("B9#{i}"));
    }
    // High-bit bytes AFTER a valid numeral are trailing garbage -> accepted.
    let mut ok = b"42".to_vec();
    ok.extend_from_slice(&[0xff, 0xfe]);
    diff_accept(&ok, "B9 trailing high-bit bytes");
}

// --- Exhaustive single-byte input sweep (every possible 1-char input) -----
#[test]
fn every_single_byte_input() {
    for b in 1u8..=255 {
        let input = [b];
        let l = libs();
        let cs = std::ffi::CString::new(&input[..]).unwrap();
        let c_out = {
            let f = l.c.driver();
            capture_stdout(|| unsafe { f(cs.as_ptr()) })
        };
        let r_out = {
            let f = l.rust.driver();
            capture_stdout(|| unsafe { f(cs.as_ptr()) })
        };
        assert_eq!(
            show(&c_out),
            show(&r_out),
            "single-byte input {b:#04x} ({:?}) diverged",
            b as char
        );
        assert_eq!(c_out, r_out, "single-byte input {b:#04x} raw bytes diverged");
    }
}

// --- Randomized fuzz over arbitrary short byte strings -------------------
#[test]
fn fuzz_random_byte_strings() {
    let mut rng = Rng::new(0xF0FF);
    let alphabet: &[u8] = b"0123456789+- \t\nxabcdefE.,;/:9999";
    for i in 0..3000 {
        let len = (rng.next_u64() % 12) as usize;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            let c = alphabet[(rng.next_u64() as usize) % alphabet.len()];
            s.push(c);
        }
        let l = libs();
        let cs = std::ffi::CString::new(s.clone()).unwrap();
        let c_out = {
            let f = l.c.driver();
            capture_stdout(|| unsafe { f(cs.as_ptr()) })
        };
        let r_out = {
            let f = l.rust.driver();
            capture_stdout(|| unsafe { f(cs.as_ptr()) })
        };
        assert_eq!(
            show(&c_out),
            show(&r_out),
            "fuzz#{i} input {:?} diverged",
            String::from_utf8_lossy(&s)
        );
        assert_eq!(c_out, r_out, "fuzz#{i} raw byte divergence");
    }
}

// --- Randomized fuzz over big-integer decimal strings around boundaries ---
#[test]
fn fuzz_numeric_boundaries() {
    let mut rng = Rng::new(0xB0DE);
    let anchors: [i128; 8] = [
        0,
        i32::MAX as i128,
        i32::MIN as i128,
        u32::MAX as i128,
        i64::MAX as i128,
        i64::MIN as i128,
        u64::MAX as i128,
        i128::from(i32::MAX) * 4,
    ];
    for i in 0..800 {
        let a = anchors[(rng.next_u64() as usize) % anchors.len()];
        let delta = (rng.next_u64() % 9) as i128 - 4;
        let v = a.saturating_add(delta);
        let s = v.to_string();
        let l = libs();
        let cs = std::ffi::CString::new(s.clone()).unwrap();
        let c_out = {
            let f = l.c.driver();
            capture_stdout(|| unsafe { f(cs.as_ptr()) })
        };
        let r_out = {
            let f = l.rust.driver();
            capture_stdout(|| unsafe { f(cs.as_ptr()) })
        };
        assert_eq!(show(&c_out), show(&r_out), "numfuzz#{i} input {s:?} diverged");
        assert_eq!(c_out, r_out, "numfuzz#{i} raw byte divergence");
    }
}
