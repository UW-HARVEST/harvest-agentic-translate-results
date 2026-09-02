//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error return, no
//! assert, no pointer and no enum, so "same error" here means "same
//! observable rejection behaviour": the same emitted bytes and the same
//! termination outcome (normal return vs signal), asserted for both `.so`s.

mod common;

use common::*;

/// Row 1 — `val = 0`, the smallest non-negative input.
#[test]
fn err01_zero() {
    let (c, r) = both();
    assert_same(&c, &r, 0, Buffering::Default);
    let out = run_capture(&c, 0, Buffering::Default);
    assert_eq!(out, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

/// Row 2 — `INT_MIN`, one step past the low end of the representable range.
/// Not a rejection: it counts up to +9. Compared as a bounded prefix.
#[test]
fn err02_int_min() {
    let (c, r) = both();
    assert_same_prefix(&c, &r, i32::MIN, 8192);
    assert_eq!(i32::MIN % 10, -8, "C truncating remainder assumption");
    assert_eq!(line_count(i32::MIN), Some(2_147_483_658));
}

/// Row 3 — `INT_MAX`: `val++` signed-overflows (UB in C). The C as compiled
/// wraps; the Rust must produce the identical wrapped sequence, and must not
/// panic or abort.
#[test]
fn err03_int_max_overflow() {
    let (c, r) = both();
    assert_same_prefix(&c, &r, i32::MAX, 8192);

    let pc = run_prefix(&c, i32::MAX, 40);
    let pr = run_prefix(&r, i32::MAX, 40);
    assert_eq!(pc, pr);
    assert!(
        pc.starts_with(b"2147483647\n-2147483648\n"),
        "C prefix at INT_MAX: {:?}",
        String::from_utf8_lossy(&pc)
    );

    // And it must not die: the Rust must not trip a debug overflow panic.
    let oc = run_child_outcome(&c, i32::MAX, StdoutState::DevNull);
    let or = run_child_outcome(&r, i32::MAX, StdoutState::DevNull);
    // Both are killed by SIGALRM (the run is ~2e9 lines) — never by SIGILL,
    // SIGABRT or SIGSEGV, which is what a panic/trap would look like.
    for (name, o) in [("C", &oc), ("Rust", &or)] {
        match o {
            ChildOutcome::Signaled(sig) => assert_eq!(
                *sig,
                libc::SIGALRM,
                "{name} died from signal {sig} instead of the timeout"
            ),
            ChildOutcome::Exited(code) => assert_eq!(*code, 0, "{name} exited {code}"),
        }
    }
    assert_eq!(
        std::mem::discriminant(&oc),
        std::mem::discriminant(&or),
        "C and Rust terminated differently at INT_MAX: {oc:?} vs {or:?}"
    );
}

/// Row 4 — one step before the overflow point.
#[test]
fn err04_just_below_int_max() {
    let (c, r) = both();
    for val in [i32::MAX - 1, i32::MAX - 2, i32::MAX - 5] {
        assert_eq!(line_count(val), None, "sieve({val}) should overflow");
        assert_same_prefix(&c, &r, val, 8192);
    }
    // `INT_MAX - 8` ends in 9, so it terminates immediately — the boundary
    // between "stops now" and "overflows".
    assert_eq!(i32::MAX - 8, 2_147_483_639);
    assert_same(&c, &r, i32::MAX - 8, Buffering::Default);
    assert_eq!(
        run_capture(&c, i32::MAX - 8, Buffering::Default),
        b"2147483639\n"
    );
}

/// Row 5 — `-9`: the trap for a Euclidean-remainder "fix". Must NOT stop early.
#[test]
fn err05_negative_nine_no_early_stop() {
    let (c, r) = both();
    assert_same(&c, &r, -9, Buffering::Default);
    let out_c = run_capture(&c, -9, Buffering::Default);
    let out_r = run_capture(&r, -9, Buffering::Default);
    assert_eq!(out_c.iter().filter(|&&b| b == b'\n').count(), 19);
    assert_eq!(out_c, out_r);
    assert_ne!(out_c, b"-9\n", "C must not treat -9 as terminating");
}

/// Row 6 — `-1`, the classic error sentinel, passed as data.
#[test]
fn err06_minus_one_sentinel() {
    let (c, r) = both();
    assert_same(&c, &r, -1, Buffering::Default);
    assert_eq!(
        run_capture(&c, -1, Buffering::Default),
        b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n"
    );
}

/// Row 7 — `9`: the loop body runs once and the terminating value is still
/// printed, because the print precedes the check.
#[test]
fn err07_immediate_termination_still_prints() {
    let (c, r) = both();
    assert_same(&c, &r, 9, Buffering::Default);
    assert_eq!(run_capture(&c, 9, Buffering::Default), b"9\n");
    assert_eq!(run_capture(&r, 9, Buffering::Default), b"9\n");
}

/// Row 8 — one step either side of the terminating residue.
#[test]
fn err08_around_terminating_residue() {
    let (c, r) = both();
    for val in [8, 9, 10, 18, 19, 20] {
        assert_same(&c, &r, val, Buffering::Default);
    }
    assert_eq!(run_capture(&c, 8, Buffering::Default), b"8\n9\n");
    assert_eq!(run_capture(&c, 19, Buffering::Default), b"19\n");
    assert_eq!(
        run_capture(&c, 10, Buffering::Default),
        b"10\n11\n12\n13\n14\n15\n16\n17\n18\n19\n"
    );
}

/// Row 9 — "out-of-range enum" degenerates to "any 32-bit word", because the
/// ABI parameter is a bare `int` with no variant set. Hostile bit patterns are
/// pushed across the FFI boundary and must be handled identically.
#[test]
fn err09_arbitrary_bit_patterns() {
    let (c, r) = both();
    let fixed: [i32; 10] = [
        0x7FFF_FFFF,
        0x8000_0000u32 as i32,
        -2_147_483_647,
        0x5555_5555,
        0xAAAA_AAAAu32 as i32,
        12_345_678,
        -12_345_678,
        0x0000_FFFF,
        0xFFFF_0000u32 as i32,
        -0x7FFF_FFFF,
    ];
    for val in fixed {
        assert_same_auto(&c, &r, val);
    }
    let mut rng = Rng::with_seed(0xB1AB_B1AB_B1AB_B1AB);
    for _ in 0..120 {
        assert_same_auto(&c, &r, rng.next_i32());
    }
}

/// Row 10 — the 64-bit-register / 32-bit-parameter width boundary.
#[test]
fn err10_upper_bits_ignored() {
    let (c, r) = both();
    for raw in [
        0xFFFF_FFFF_0000_0009u64 as i64,
        0x0000_0001_0000_0000u64 as i64,
        i64::MIN,
        i64::MAX,
        -1i64,
    ] {
        let low = raw as i32;
        if !is_cheap(low) {
            continue;
        }
        let out_c = run_capture_i64(&c, raw);
        let out_r = run_capture_i64(&r, raw);
        assert_eq!(out_c, out_r, "diverged for raw={raw:#x}");
        assert_eq!(out_c, run_capture(&c, low, Buffering::Default));
    }
}

/// Row 11 — `printf` fails on every line (stdout closed / read-only /
/// /dev/full). The C ignores `printf`'s return value, so the loop still
/// terminates on the `% 10 == 9` rule and the function returns normally. The
/// Rust must not abort, retry, or loop forever either.
#[test]
fn err11_printf_failure_is_ignored() {
    let (c, r) = both();
    for state in [
        StdoutState::Closed,
        StdoutState::ReadOnly,
        StdoutState::DevFull,
        StdoutState::DevNull,
    ] {
        for val in [9, 0, -9, 95] {
            let oc = run_child_outcome(&c, val, state);
            let or = run_child_outcome(&r, val, state);
            assert_eq!(
                oc, or,
                "termination outcome diverged for sieve({val}) with stdout={state:?}: \
                 C={oc:?} Rust={or:?}"
            );
            assert_eq!(
                oc,
                ChildOutcome::Exited(0),
                "sieve({val}) with stdout={state:?} did not return normally in C: {oc:?}"
            );
        }
    }
}

/// Generic boundary sweep: every input one step past each interesting edge,
/// plus a dense sweep of the whole low-magnitude neighbourhood, where nearly
/// all behavioural edges of this function live.
#[test]
fn err12_dense_boundary_sweep() {
    let (c, r) = both();
    for val in -60..=60 {
        assert_same(&c, &r, val, Buffering::Default);
    }
    for val in [
        i32::MIN + 9,
        i32::MIN + 10,
        -1_000_000_009,
        -10,
        -11,
        1,
        i32::MAX - 8,
        i32::MAX - 9,
        i32::MAX - 18,
        999_999_999,
        1_000_000_000,
    ] {
        assert_same_auto(&c, &r, val);
    }
}

/// The API takes no pointer, no length and no enum — record that as a checked
/// fact rather than a silent omission, by asserting the exported symbol really
/// is the single-`int` function the header declares.
#[test]
fn err13_surface_has_no_pointer_or_length_parameters() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/sieve.h"),
    )
    .expect("read sieve.h");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(");") && !l.starts_with("//"))
        .collect();
    assert_eq!(
        decls,
        vec!["void sieve(int start);"],
        "the public API changed; ERRORS.md/CONFIGS.md must be re-derived"
    );
    assert!(
        !decls[0].contains('*'),
        "a pointer parameter appeared — null-pointer rows are now required"
    );

    // And both .so files really do export exactly that symbol and accept the
    // call through it.
    let (c, r) = both();
    assert_same(&c, &r, 9, Buffering::Default);
}
