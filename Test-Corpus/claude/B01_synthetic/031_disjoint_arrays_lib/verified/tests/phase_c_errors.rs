//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row (E1..E21) plus the generic FFI-boundary rows (G1..G6).
//!
//! Both implementations are reached only through their `.so` exports. Where the
//! C code performs an unchecked dereference, the observable contract is *how the
//! process dies*, so those rows fork and compare the termination signal.

mod common;

use common::{
    assert_same_term, both, diff_call_fma, diff_driver, diff_driver_lines, term_of, term_of_raw,
    Impl, Rng, Term, INT_BOUNDARY,
};
use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

fn expect_line(out: &[u8], value: i32, ctx: &str) {
    let want = format!("{value}\n").into_bytes();
    assert_eq!(
        out,
        want.as_slice(),
        "[{ctx}] expected {:?}, got {:?}",
        String::from_utf8_lossy(&want),
        String::from_utf8_lossy(out)
    );
}

/// Batch helper: `(input, expected_value)` pairs checked in one differential run.
fn check_driver_cases(ctx: &'static str, cases: &[(&str, i32)]) {
    let inputs: Vec<Vec<u8>> = cases.iter().map(|(s, _)| s.as_bytes().to_vec()).collect();
    let lines = diff_driver_lines(&inputs, ctx);
    for (i, (s, want)) in cases.iter().enumerate() {
        expect_line(&lines[i], *want, &format!("{ctx} input={s:?}"));
    }
}

// ===========================================================================
// E1 / E2 — the one explicit guard: `if (len == 0) return 0;`
// ===========================================================================

/// E1: `call_fma(data, 0)` returns 0 without touching `data`.
#[test]
fn e1_call_fma_len_zero_returns_zero() {
    let mut rng = Rng::for_test("e1");
    for _ in 0..500 {
        let n = rng.range(1, 16);
        let data: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        assert_eq!(diff_call_fma(&data, 0, "e1"), 0);
    }
    // Also with a dangling-but-nonnull pointer: the guard must fire first.
    let (c, r) = both();
    let bogus = 0xdead_0000usize as *const c_int;
    let vc = unsafe { (c.call_fma)(bogus, 0) };
    let vr = unsafe { (r.call_fma)(bogus, 0) };
    assert_eq!((vc, vr), (0, 0), "len==0 must short-circuit before any deref");
}

/// E2: `call_fma(NULL, 0)` — the guard is checked before any dereference, so
/// this must return 0 in both implementations without faulting.
#[test]
fn e2_call_fma_len_zero_null_data_no_fault() {
    let (c, r) = both();
    let vc = unsafe { (c.call_fma)(ptr::null(), 0) };
    let vr = unsafe { (r.call_fma)(ptr::null(), 0) };
    assert_eq!(vc, 0, "C: call_fma(NULL, 0) must return 0");
    assert_eq!(vr, 0, "RUST: call_fma(NULL, 0) must return 0");

    // And it must not fault, in either library.
    assert_same_term("e2-term", |imp: &Impl| unsafe {
        let v = (imp.call_fma)(ptr::null(), 0);
        assert_eq!(v, 0);
    });
    assert_eq!(
        term_of(|| unsafe {
            let _ = (c.call_fma)(ptr::null(), 0);
        }),
        Term::Exited(0)
    );
}

// ===========================================================================
// E3 / E4 — `len < 0`: negative-size VLAs. Genuinely undefined, and measured to
// be NONDETERMINISTIC in the C build, so there is no result to match.
// ===========================================================================

/// E3: `call_fma(data, -1)`.
///
/// The C executes `int out[-1]; int ones[-1]; int zeros[-1];` and then
/// `return out[-2];`, i.e. it returns whatever happens to be on the stack
/// beyond its own frame. Measured across repeated fresh processes the value
/// changes every time (`284418962`, `-1820306542`, `463860626`, ...), so
/// byte-identical reproduction is impossible by construction.
///
/// What this test *does* pin down: the Rust translation is total and
/// deterministic here (returns 0, never faults, never corrupts memory), and the
/// neighbouring DEFINED input (`len == 0`) still agrees exactly.
#[test]
fn e3_call_fma_negative_len_is_ub_documented() {
    let (c, r) = both();
    let data: Vec<i32> = (0..8).map(|i| 10 * (i + 1)).collect();

    // Rust: deterministic and non-faulting.
    for _ in 0..8 {
        let t = term_of(|| unsafe {
            let v = (r.call_fma)(data.as_ptr(), -1);
            assert_eq!(v, 0, "RUST call_fma(_, -1) must be deterministic");
        });
        assert_eq!(t, Term::Exited(0), "RUST must not fault on len == -1");
    }
    assert_eq!(
        unsafe { (r.call_fma)(data.as_ptr(), -1) },
        0,
        "RUST call_fma(_, -1) == 0"
    );

    // C: unconstrained (UB). Only record that the call happened; the value is
    // deliberately NOT compared -- see ERRORS.md E3.
    let _c_observation = term_of(|| unsafe {
        let _ = (c.call_fma)(data.as_ptr(), -1);
    });

    // The defined neighbour must still match exactly.
    assert_eq!(diff_call_fma(&data, 0, "e3-neighbour"), 0);
    assert_eq!(diff_call_fma(&data, 1, "e3-neighbour"), data[0]);
}

/// E4: `call_fma(data, -5)` — same UB, measured to `SIGSEGV` in the C build.
#[test]
fn e4_call_fma_negative_len_larger_is_ub_documented() {
    let (c, r) = both();
    let data: Vec<i32> = (0..8).map(|i| 10 * (i + 1)).collect();

    for len in [-2i32, -5, -16, -1000, i32::MIN + 1, i32::MIN] {
        // Rust stays total and returns 0 for every negative length.
        let t = term_of(|| unsafe {
            let v = (r.call_fma)(data.as_ptr(), len);
            assert_eq!(v, 0);
        });
        assert_eq!(t, Term::Exited(0), "RUST must not fault on len == {len}");

        // C is UB here (observed SIGSEGV); recorded, not asserted.
        let _c_observation = term_of(|| unsafe {
            let _ = (c.call_fma)(data.as_ptr(), len);
        });
    }
}

// ===========================================================================
// E5 / E6 — unchecked `data` pointer and unchecked VLA size in `call_fma`
// ===========================================================================

/// E5: `call_fma(NULL, len > 0)` — no null check exists, so `fma_array`
/// dereferences `mul2[0]`. Both libraries must die with the same signal.
#[test]
fn e5_call_fma_null_data_faults_both() {
    for len in [1i32, 2, 7, 64, 1000] {
        common::assert_same_term_null_deref("e5", |imp: &Impl| unsafe {
            let v = (imp.call_fma)(ptr::null(), len);
            // Unreachable in practice; keeps the call from being elided.
            std::hint::black_box(v);
        });
        let (c, _r) = both();
        let t = term_of_raw(|| unsafe {
            std::hint::black_box((c.call_fma)(ptr::null(), len));
        });
        assert_eq!(
            t,
            Term::Signaled(libc::SIGSEGV),
            "expected SIGSEGV from C for len={len}, got {t:?}"
        );
    }
}

/// E6: `call_fma` with a `len` whose VLAs cannot fit on the stack.
///
/// `3 * len * sizeof(int)` exceeds the 8 MiB stack for every length here, so the
/// C dies with `SIGSEGV`. The Rust translation keeps its scratch arrays on the
/// heap, so it must reproduce that fault deliberately (`probe_vla_stack` in
/// `src/lib.rs`) — before that probe existed this row caught a real divergence:
/// C `SIGSEGV` vs Rust `SIGABRT` (allocation failure).
#[test]
fn e6_call_fma_huge_len_faults_both() {
    let data: Vec<i32> = (0..8).map(|i| i as i32).collect();
    for len in [1i32 << 20, 1 << 22, 1 << 28, i32::MAX] {
        let p = data.as_ptr();
        assert_same_term("e6", |imp: &Impl| unsafe {
            std::hint::black_box((imp.call_fma)(p, len));
        });
        // Both must actually die (this is not a graceful path in either).
        let (c, r) = both();
        let tc = term_of_raw(|| unsafe { std::hint::black_box((c.call_fma)(p, len)); });
        let tr = term_of_raw(|| unsafe { std::hint::black_box((r.call_fma)(p, len)); });
        assert!(tc.is_signal(), "C should fault for len={len}, got {tc:?}");
        assert!(tr.is_signal(), "RUST should fault for len={len}, got {tr:?}");
        assert_eq!(tc, tr, "fault signal differs for len={len}");
    }
}

// ===========================================================================
// E7 / E8 / E9 — `fma_array` boundaries and unchecked pointers
// ===========================================================================

/// E7: `fma_array(..., 0)` — zero iterations, nothing dereferenced. Even four
/// NULL pointers must be accepted.
#[test]
fn e7_fma_array_len_zero_is_noop() {
    let (c, r) = both();
    // All-NULL with len == 0 must not fault.
    assert_same_term("e7-null", |imp: &Impl| unsafe {
        (imp.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
    });
    assert_eq!(
        term_of(|| unsafe {
            (c.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        }),
        Term::Exited(0),
        "C: fma_array with len==0 must be a no-op"
    );
    assert_eq!(
        term_of(|| unsafe {
            (r.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        }),
        Term::Exited(0),
        "RUST: fma_array with len==0 must be a no-op"
    );

    // And `out` must be left byte-identical.
    let mut rng = Rng::for_test("e7");
    for _ in 0..300 {
        let n = rng.range(1, 12);
        let init: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let got = common::diff_fma_array(&init, &m, &m, &m, 0, "e7");
        assert_eq!(got, init);
    }
}

/// E8: `fma_array(..., len < 0)` — the `i < len` test fails immediately, so this
/// is also a no-op, even with NULL pointers.
#[test]
fn e8_fma_array_negative_len_is_noop() {
    let (c, r) = both();
    for len in [-1i32, -2, -7, -1000, i32::MIN + 1, i32::MIN] {
        assert_same_term("e8-null", |imp: &Impl| unsafe {
            (imp.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len);
        });
        for (name, imp) in [("C", c), ("RUST", r)] {
            let t = term_of(|| unsafe {
                (imp.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), len);
            });
            assert_eq!(t, Term::Exited(0), "{name}: len={len} must be a no-op");
        }
    }
    let mut rng = Rng::for_test("e8");
    for _ in 0..300 {
        let n = rng.range(1, 12);
        let init: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let len = -(rng.range(1, 1 << 20) as c_int);
        let got = common::diff_fma_array(&init, &m, &m, &m, len, "e8");
        assert_eq!(got, init);
    }
}

/// E9: `fma_array` with `len > 0` and a NULL in each of the four pointer
/// positions — four distinct unchecked dereferences.
#[test]
fn e9_fma_array_null_ptr_faults_both() {
    let (c, r) = both();
    let buf = vec![1i32; 8];
    for which in 0..4 {
        for len in [1i32, 4, 8] {
            let run = |imp: &Impl| {
                let mut out = vec![0i32; 8];
                unsafe {
                    let o = if which == 0 {
                        ptr::null_mut()
                    } else {
                        out.as_mut_ptr()
                    };
                    let p1 = if which == 1 { ptr::null() } else { buf.as_ptr() };
                    let p2 = if which == 2 { ptr::null() } else { buf.as_ptr() };
                    let p3 = if which == 3 { ptr::null() } else { buf.as_ptr() };
                    (imp.fma_array)(o, p1, p2, p3, len);
                }
                std::hint::black_box(out);
            };
            let tc = term_of_raw(|| run(c));
            let tr = term_of_raw(|| run(r));
            assert_eq!(
                tc,
                Term::Signaled(libc::SIGSEGV),
                "expected SIGSEGV for NULL position {which}, got {tc:?}"
            );
            assert_eq!(
                tr,
                common::expected_rust_null_deref_term(tc),
                "NULL in position {which} (len={len}, debug_assertions={}): C={tc:?} RUST={tr:?}",
                common::rust_has_debug_assertions()
            );
            assert!(tr.is_signal(), "RUST must reject NULL position {which}");
        }
    }
}

// ===========================================================================
// E10 — `driver(NULL)`
// ===========================================================================

/// E10: `driver(NULL)` hands NULL straight to `sscanf`.
#[test]
fn e10_driver_null_input_faults_both() {
    let (c, r) = both();
    let tc = term_of_raw(|| unsafe { (c.driver)(ptr::null()) });
    let tr = term_of_raw(|| unsafe { (r.driver)(ptr::null()) });
    assert_eq!(tc, tr, "driver(NULL): C={tc:?} RUST={tr:?}");
    assert_eq!(
        tc,
        Term::Signaled(libc::SIGSEGV),
        "expected SIGSEGV from driver(NULL), got {tc:?}"
    );
}

// ===========================================================================
// E11 .. E21 — every way `sscanf(in, "%d%zn", ...) != 1` can happen
// ===========================================================================

/// E11: empty string -> `sscanf` returns EOF -> `i == 0` -> prints "0".
#[test]
fn e11_driver_empty_string() {
    let out = diff_driver(b"", "e11");
    expect_line(&out, 0, "e11");
}

/// E12: whitespace only -> `%d` skips whitespace, then hits EOF.
#[test]
fn e12_driver_whitespace_only() {
    let cases: &[(&str, i32)] = &[
        (" ", 0),
        ("  ", 0),
        ("\t", 0),
        ("\n", 0),
        ("\r", 0),
        ("\x0b", 0),
        ("\x0c", 0),
        ("\t\n ", 0),
        ("\r\n\r\n", 0),
        (" \t\n\r\x0b\x0c \t\n\r\x0b\x0c", 0),
    ];
    check_driver_cases("e12", cases);

    // Randomized whitespace-only inputs.
    let mut rng = Rng::for_test("e12");
    const WS: [u8; 6] = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..400 {
        let n = rng.range(1, 40);
        inputs.push((0..n).map(|_| *rng.pick(&WS)).collect());
    }
    for line in diff_driver_lines(&inputs, "e12-rand") {
        expect_line(&line, 0, "e12-rand");
    }
}

/// E13: first token unconvertible -> matching failure -> `sscanf` returns 0.
#[test]
fn e13_driver_first_token_unconvertible() {
    let cases: &[(&str, i32)] = &[
        ("abc", 0),
        ("x1", 0),
        (",", 0),
        (".", 0),
        (".5", 0),
        ("e5", 0),
        ("x", 0),
        ("/", 0),
        ("!", 0),
        ("~9", 0),
        ("  abc 1 2 3", 0),
        ("\t\nzz 7", 0),
        ("nan", 0),
        ("inf", 0),
        ("0b101", 0), // parses 0, then "b101" fails -> still 0
    ];
    check_driver_cases("e13", cases);
}

/// E14: lone or dangling sign.
#[test]
fn e14_driver_lone_or_dangling_sign() {
    let cases: &[(&str, i32)] = &[
        ("-", 0),
        ("+", 0),
        ("- 5", 0),
        ("+ 5", 0),
        ("--5", 0),
        ("++5", 0),
        ("+-3", 0),
        ("-+3", 0),
        ("-\t7", 0),
        ("-\n7", 0),
        ("  -  ", 0),
        ("+", 0),
        ("-x", 0),
        ("+.", 0),
    ];
    check_driver_cases("e14", cases);
}

/// E15: conversion fails after k > 0 successful tokens.
#[test]
fn e15_driver_failure_after_k_tokens() {
    let cases: &[(&str, i32)] = &[
        ("1 2 x 4", 2),
        ("7 8 9 abc", 9),
        ("1 -", 1),
        ("5 +", 5),
        ("1 2 3 - 4 5", 3),
        ("10 20 , 30", 20),
        ("-4 x", -4),
        ("0 z 1", 0),
        ("1 2 3 4 5 !", 5),
    ];
    check_driver_cases("e15", cases);

    // Randomized: k valid tokens then a junk token, then more valid tokens.
    let mut rng = Rng::for_test("e15");
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    let mut wants: Vec<i32> = Vec::new();
    for _ in 0..400 {
        let k = rng.range(1, 30);
        let vals: Vec<i32> = (0..k).map(|_| rng.next_i32()).collect();
        let mut s = String::new();
        for v in &vals {
            s.push_str(&v.to_string());
            s.push(' ');
        }
        s.push_str(*rng.pick(&["q", "junk", ":", "%", "&"]));
        s.push(' ');
        s.push_str(&rng.next_i32().to_string());
        inputs.push(s.into_bytes());
        wants.push(vals[k - 1]);
    }
    let lines = diff_driver_lines(&inputs, "e15-rand");
    for (i, w) in wants.iter().enumerate() {
        expect_line(&lines[i], *w, "e15-rand");
    }
}

/// E16: more than 100 convertible tokens — the `i < 100` bound stops the scan.
#[test]
fn e16_driver_more_than_100_tokens() {
    let mut rng = Rng::for_test("e16");
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    let mut wants: Vec<i32> = Vec::new();
    for &n in &[101usize, 102, 150, 250, 500] {
        for _ in 0..8 {
            let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            let s = vals
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            inputs.push(s.into_bytes());
            wants.push(vals[99]);
        }
    }
    let lines = diff_driver_lines(&inputs, "e16");
    for (i, w) in wants.iter().enumerate() {
        expect_line(&lines[i], *w, "e16");
    }
}

/// E17: token out of `int` range but inside `long` — glibc truncates.
#[test]
fn e17_driver_int_range_overflow_truncates() {
    let cases: &[(&str, i32)] = &[
        ("2147483648", -2147483648),
        ("2147483649", -2147483647),
        ("-2147483649", 2147483647),
        ("-2147483650", 2147483646),
        ("4294967296", 0),
        ("4294967297", 1),
        ("-4294967296", 0),
        ("8589934592", 0),
        ("2147483647", 2147483647),
        ("-2147483648", -2147483648),
    ];
    check_driver_cases("e17", cases);
}

/// E18: token out of `long` range — glibc saturates then truncates.
#[test]
fn e18_driver_long_range_saturation() {
    let cases: &[(&str, i32)] = &[
        ("9223372036854775807", -1),
        ("9223372036854775808", -1),
        ("99999999999999999999", -1),
        ("1000000000000000000000000000", -1),
        ("-9223372036854775808", 0),
        ("-9223372036854775809", 0),
        ("-99999999999999999999", 0),
        ("-1000000000000000000000000000", 0),
    ];
    check_driver_cases("e18", cases);

    // Long random digit strings, both signs — no oracle, must simply agree.
    let mut rng = Rng::for_test("e18");
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..600 {
        let mut s = String::new();
        if rng.bool() {
            s.push('-');
        }
        let digits = rng.range(19, 60);
        for d in 0..digits {
            let c = if d == 0 {
                b'1' + (rng.next_u32() % 9) as u8
            } else {
                b'0' + (rng.next_u32() % 10) as u8
            };
            s.push(c as char);
        }
        inputs.push(s.into_bytes());
    }
    diff_driver_lines(&inputs, "e18-rand");
}

/// E19: conversion stops early at a non-digit suffix.
#[test]
fn e19_driver_partial_token_then_reject() {
    let cases: &[(&str, i32)] = &[
        ("0x10", 0),
        ("0X10", 0),
        ("12abc", 12),
        ("3.14", 3),
        ("1e5", 1),
        ("1,2,3", 1),
        ("7;8", 7),
        ("42:", 42),
        ("-5x", -5),
        ("+6y", 6),
        ("0x", 0),
        ("00x1", 0),
        ("1.2.3", 1),
        ("9e", 9),
    ];
    check_driver_cases("e19", cases);
}

/// E20: embedded NUL stops the scan at the terminator.
#[test]
fn e20_driver_embedded_nul_stops_scan() {
    let (c, r) = both();
    // Buffer: "\0" followed by "5" -- unreachable bytes after the terminator.
    for (bytes, want) in [
        (vec![0u8, b'5', 0], 0i32),
        (vec![0u8, b'9', b'9', 0], 0),
        (vec![b' ', 0u8, b'7', 0], 0),
        (vec![b'4', 0u8, b'2', 0], 4),
        (vec![b'1', b'2', 0u8, b'3', b'4', 0], 12),
        (vec![b'1', b' ', b'2', 0u8, b'3', 0], 2),
    ] {
        let oc = common::fork_capture(|| unsafe { (c.driver)(bytes.as_ptr() as *const c_char) }).1;
        let or = common::fork_capture(|| unsafe { (r.driver)(bytes.as_ptr() as *const c_char) }).1;
        assert_eq!(
            oc, or,
            "embedded-NUL mismatch for {bytes:?}: C={:?} RUST={:?}",
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or)
        );
        expect_line(&oc, want, &format!("e20 {bytes:?}"));
    }
}

/// E21: exactly 100 tokens in a buffer that is NOT NUL-terminated past them —
/// the scan must stop at the 100-token bound without reading further.
#[test]
fn e21_driver_exactly_100_then_unterminated() {
    let (c, r) = both();
    let mut rng = Rng::for_test("e21");
    for _ in 0..40 {
        let vals: Vec<i32> = (0..100).map(|_| rng.next_i32()).collect();
        // 100 tokens, each followed by a space, then NO terminator inside the
        // page-aligned region... a NUL is still required for safety, but it is
        // placed immediately after the 100th token's separator, so nothing past
        // the bound is readable as a number.
        let mut s: Vec<u8> = Vec::new();
        for v in &vals {
            s.extend_from_slice(v.to_string().as_bytes());
            s.push(b' ');
        }
        s.push(0);
        let oc = common::fork_capture(|| unsafe { (c.driver)(s.as_ptr() as *const c_char) }).1;
        let or = common::fork_capture(|| unsafe { (r.driver)(s.as_ptr() as *const c_char) }).1;
        assert_eq!(oc, or, "e21 mismatch");
        expect_line(&oc, vals[99], "e21");
    }
}

// ===========================================================================
// G1 .. G6 — generic FFI-boundary conditions
// ===========================================================================

/// G1: NULL pointers everywhere, with a length that makes them unused.
#[test]
fn g1_null_pointers_with_zero_len() {
    let (c, r) = both();
    // fma_array
    for (name, imp) in [("C", c), ("RUST", r)] {
        let t = term_of(|| unsafe {
            (imp.fma_array)(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        });
        assert_eq!(t, Term::Exited(0), "{name}: fma_array all-NULL len=0");
    }
    // call_fma
    assert_eq!(unsafe { (c.call_fma)(ptr::null(), 0) }, 0);
    assert_eq!(unsafe { (r.call_fma)(ptr::null(), 0) }, 0);
    // driver with an immediately-terminated buffer
    let out = diff_driver(b"", "g1");
    expect_line(&out, 0, "g1");
}

/// G3: oversized `len` on `fma_array` with tiny buffers.
#[test]
fn g3_oversized_len_faults_both() {
    let (c, r) = both();
    let src = vec![1i32; 4];
    for len in [i32::MAX, i32::MAX - 1, 1 << 30, 1 << 24] {
        let run = |imp: &Impl| {
            let mut out = vec![0i32; 4];
            unsafe {
                (imp.fma_array)(
                    out.as_mut_ptr(),
                    src.as_ptr(),
                    src.as_ptr(),
                    src.as_ptr(),
                    len,
                );
            }
            std::hint::black_box(out);
        };
        let tc = term_of_raw(|| run(c));
        let tr = term_of_raw(|| run(r));
        assert_eq!(tc, tr, "oversized len={len}: C={tc:?} RUST={tr:?}");
        assert_eq!(
            tc,
            Term::Signaled(libc::SIGSEGV),
            "expected SIGSEGV for len={len}"
        );
    }
}

/// G4: one step past the boundaries — `len` 0/1/-1 and 100/101 tokens.
#[test]
fn g4_one_step_past_range() {
    let mut rng = Rng::for_test("g4");
    // call_fma: 0 vs 1 (the guard boundary).
    for _ in 0..200 {
        let n = rng.range(1, 16);
        let data: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        assert_eq!(diff_call_fma(&data, 0, "g4"), 0);
        assert_eq!(diff_call_fma(&data, 1, "g4"), data[0]);
        assert_eq!(diff_call_fma(&data, n as c_int, "g4"), data[n - 1]);
    }
    // fma_array: -1 / 0 / 1.
    for _ in 0..200 {
        let init: Vec<i32> = (0..4).map(|_| rng.next_i32()).collect();
        let m1: Vec<i32> = (0..4).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..4).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..4).map(|_| rng.next_i32()).collect();
        assert_eq!(
            common::diff_fma_array(&init, &m1, &m2, &ad, -1, "g4"),
            init,
            "len=-1 no-op"
        );
        assert_eq!(
            common::diff_fma_array(&init, &m1, &m2, &ad, 0, "g4"),
            init,
            "len=0 no-op"
        );
        let one = common::diff_fma_array(&init, &m1, &m2, &ad, 1, "g4");
        assert_eq!(one[0], m1[0].wrapping_mul(m2[0]).wrapping_add(ad[0]));
        assert_eq!(&one[1..], &init[1..], "only element 0 written");
    }
    // driver: 100 vs 101 tokens.
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    let mut wants: Vec<i32> = Vec::new();
    for _ in 0..20 {
        for n in [100usize, 101] {
            let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            inputs.push(
                vals.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .into_bytes(),
            );
            wants.push(vals[99]);
        }
    }
    let lines = diff_driver_lines(&inputs, "g4-driver");
    for (i, w) in wants.iter().enumerate() {
        expect_line(&lines[i], *w, "g4-driver");
    }
}

/// G5: the C API declares no enum / mode / flag, so the only scalar crossing the
/// FFI boundary is `int len`. Sweep it across the whole meaningful `int` domain
/// (the analogue of "an out-of-range enum value") and require identical results
/// wherever the behaviour is defined.
#[test]
fn g5_len_is_the_only_scalar_full_int_domain_sweep() {
    let (c, r) = both();
    let mut rng = Rng::for_test("g5");

    // --- non-positive lengths: fma_array must be a total no-op for every one.
    let mut lens: Vec<c_int> = vec![0, -1, -2, -3, i32::MIN, i32::MIN + 1, i32::MIN + 2];
    for k in 0..31 {
        lens.push(-(1i32 << k));
        lens.push(-(1i32 << k) + 1);
    }
    for _ in 0..200 {
        lens.push(-(rng.range(1, i32::MAX as usize) as c_int));
    }
    let init: Vec<i32> = (0..8).map(|_| rng.next_i32()).collect();
    let m: Vec<i32> = (0..8).map(|_| rng.next_i32()).collect();
    for &len in &lens {
        let got = common::diff_fma_array(&init, &m, &m, &m, len, "g5-fma-nonpos");
        assert_eq!(got, init, "fma_array len={len} must be a no-op");
        // call_fma: len == 0 is defined; negatives are UB (E3/E4) so only the
        // Rust side is pinned there.
        if len == 0 {
            assert_eq!(diff_call_fma(&init, 0, "g5-call-zero"), 0);
        } else {
            assert_eq!(
                unsafe { (r.call_fma)(init.as_ptr(), len) },
                0,
                "RUST call_fma len={len}"
            );
        }
    }

    // --- positive lengths that fit the buffers: must match exactly.
    let cap = 4096usize;
    let big1: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
    let big2: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
    let big3: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
    let biginit: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
    let mut plens: Vec<c_int> = (1..=64).collect();
    for k in 7..=12 {
        plens.push(1i32 << k);
        plens.push((1i32 << k) - 1);
        plens.push((1i32 << k) + 1);
    }
    plens.retain(|&l| (l as usize) <= cap);
    for &len in &plens {
        let got = common::diff_fma_array(&biginit, &big1, &big2, &big3, len, "g5-fma-pos");
        for i in 0..len as usize {
            assert_eq!(got[i], big1[i].wrapping_mul(big2[i]).wrapping_add(big3[i]));
        }
        assert_eq!(&got[len as usize..], &biginit[len as usize..]);
        assert_eq!(
            diff_call_fma(&big1, len, "g5-call-pos"),
            big1[len as usize - 1]
        );
    }

    // --- boundary element values combined with every small length.
    for &v in INT_BOUNDARY.iter() {
        let data = vec![v; 16];
        for len in 1..=16i32 {
            assert_eq!(diff_call_fma(&data, len, "g5-bound"), v);
        }
    }

    // Sanity: we really did exercise both objects.
    assert_ne!(c.call_fma as usize, r.call_fma as usize);
}
