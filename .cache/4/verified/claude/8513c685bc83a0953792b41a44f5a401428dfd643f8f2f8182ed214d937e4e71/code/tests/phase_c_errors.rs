// PHASE C — error-path differential tests, one test per row of ERRORS.md.
//
// Both public functions return `void`, so the library's only rejection signal is
// the exact byte string "An error occurred\n" on stdout (plus the absence of any
// run() output). Every test below therefore asserts BOTH of:
//
//   * C and Rust transcripts are byte-identical, AND
//   * the transcript is EXACTLY the rejection signal
//
// i.e. it is never satisfied by "both failed somehow". `assert_rejected` also
// proves no `run()` side effects leaked out, and `assert_accepted` guards the
// boundary from the other side so an over-eager rejection cannot pass.

mod common;
use common::*;

/// The exact bytes the C code emits on rejection (gcc lowers the `printf` to
/// `puts`, which re-appends the newline).
const ERR: &str = "An error occurred\n";

/// Assert the input is REJECTED identically by both implementations, with the
/// exact expected error bytes and no `run()` output.
fn assert_rejected(label: &str, op_of: impl Fn() -> Op) {
    // 1. C and Rust agree byte-for-byte.
    assert_same(label, &[op_of()]);
    // 2. and the agreed-upon output is precisely the rejection signal.
    for im in [&pair().c, &pair().rust] {
        let out = capture_one(im, &[op_of()]);
        assert_eq!(
            out, ERR,
            "[{}] {} implementation did not emit exactly the rejection signal.\n\
             expected {:?}\n     got {:?}",
            label, im.name, ERR, out
        );
        assert!(
            !out.contains("The house has"),
            "[{}] {} ran the house mutation despite rejecting the input:\n{}",
            label,
            im.name,
            out
        );
    }
}

/// Assert the input is ACCEPTED identically by both (guards against off-by-one
/// rejection at range boundaries).
fn assert_accepted(label: &str, op_of: impl Fn() -> Op) {
    assert_same(label, &[op_of()]);
    for im in [&pair().c, &pair().rust] {
        let out = capture_one(im, &[op_of()]);
        assert!(
            !out.contains("An error occurred"),
            "[{}] {} rejected an input the C guard accepts:\n{}",
            label,
            im.name,
            out
        );
        // driver() calls run() twice => 8 "The house has" lines.
        assert_eq!(
            out.lines().filter(|l| l.starts_with("The house has")).count(),
            8,
            "[{}] {} produced the wrong number of run() lines:\n{}",
            label,
            im.name,
            out
        );
    }
}

// ===========================================================================
// Row 1 — conjunct `endp != str` fails: empty string
// ===========================================================================
#[test]
fn row01_empty_string() {
    assert_rejected("err01_empty", || Op::driver(""));
}

// ===========================================================================
// Row 2 — purely non-numeric input
// ===========================================================================
#[test]
fn row02_non_numeric() {
    for s in [
        "abc", "!!", "@", "hello", "NaN", "inf", "null", "#", "~", "\u{7f}", "/", ":",
    ] {
        assert_rejected(&format!("err02_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
    // Randomized non-numeric strings: bytes drawn from the non-digit,
    // non-whitespace, non-sign range so `strtol` can never convert.
    let mut rng = Rng::new(0xC002);
    for i in 0..128 {
        let n = rng.below(8) as usize + 1;
        let bytes: Vec<u8> = (0..n)
            .map(|_| loop {
                let b = (rng.below(94) + 33) as u8; // printable, no space
                if !b.is_ascii_digit() && b != b'+' && b != b'-' {
                    return b;
                }
            })
            .collect();
        assert_rejected(&format!("err02_rand{}", i), || Op::driver_bytes(&bytes));
    }
}

// ===========================================================================
// Row 3 — whitespace only (strtol skips it, then finds no digits)
// ===========================================================================
#[test]
fn row03_whitespace_only() {
    for s in [
        " ", "  ", "\t", "\n", "\u{b}", "\u{c}", "\r", "\t\n ", " \r\n\t\u{b}\u{c} ",
    ] {
        assert_rejected(&format!("err03_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
}

// ===========================================================================
// Row 4 — sign with no digits
// ===========================================================================
#[test]
fn row04_sign_without_digits() {
    for s in [
        "+", "-", "  -", "+ 1", "- 1", "++1", "--1", "+-1", "-+", "\t+", "+ ", "-abc",
    ] {
        assert_rejected(&format!("err04_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
}

// ===========================================================================
// Row 5 — base-10 rejects non-decimal forms with no leading digit
// ===========================================================================
#[test]
fn row05_no_leading_digit_forms() {
    // None of these has a leading decimal digit, so `strtol` converts nothing.
    for s in ["x10", "e5", ".", "-.", ".5", "-.5", "o17", "$5", "'9'", "b1"] {
        assert_rejected(&format!("err05_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
    // Contrast: these DO start with a decimal digit, so base 10 converts the
    // leading `0` and the call SUCCEEDS (partial consumption is enough for the C
    // guard). Documented in ERRORS.md row 5 / row 17.
    assert_accepted("err05_0x_accepted", || Op::driver("0x"));
    assert_accepted("err05_0x1F_accepted", || Op::driver("0x1F"));
    assert_accepted("err05_0b1_accepted", || Op::driver("0b1"));
}

// ===========================================================================
// Row 6 — ERANGE: value > LONG_MAX
// ===========================================================================
#[test]
fn row06_erange_above_long_max() {
    for s in [
        "9223372036854775808",             // LONG_MAX + 1
        "9223372036854775809",
        "99999999999999999999",
        "18446744073709551616",            // 2^64
        "+9223372036854775808",
        "  9223372036854775808",
    ] {
        assert_rejected(&format!("err06_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
    let mut rng = Rng::new(0xC006);
    for i in 0..64 {
        // 20-38 digit positive numbers are all > LONG_MAX.
        let n = rng.below(19) as usize + 20;
        let mut s = String::from("9");
        for _ in 1..n {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        assert_rejected(&format!("err06_rand{}", i), || Op::driver(&s));
    }
}

// ===========================================================================
// Row 7 — ERANGE: value < LONG_MIN
// ===========================================================================
#[test]
fn row07_erange_below_long_min() {
    for s in [
        "-9223372036854775809",            // LONG_MIN - 1
        "-9223372036854775810",
        "-99999999999999999999",
        "-18446744073709551616",
        "  -9223372036854775809",
    ] {
        assert_rejected(&format!("err07_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
    let mut rng = Rng::new(0xC007);
    for i in 0..64 {
        let n = rng.below(19) as usize + 20;
        let mut s = String::from("-9");
        for _ in 1..n {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        assert_rejected(&format!("err07_rand{}", i), || Op::driver(&s));
    }
}

// ===========================================================================
// Row 8 — converts cleanly (errno == 0) but > INT_MAX
// ===========================================================================
#[test]
fn row08_above_int_max_errno_clean() {
    for s in [
        "2147483648",   // INT_MAX + 1
        "2147483649",
        "4294967295",   // UINT32_MAX
        "4294967296",
        "+2147483648",
        "  0002147483648",
        "10000000000",
    ] {
        assert_rejected(&format!("err08_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
    let mut rng = Rng::new(0xC008);
    for i in 0..128 {
        // Strictly between INT_MAX and LONG_MAX => no ERANGE, but out of int range.
        let v = i32::MAX as i64 + 1 + (rng.next_u64() % (i64::MAX as u64 - i32::MAX as u64 - 2)) as i64;
        assert_rejected(&format!("err08_rand{}", i), || Op::driver(&format!("{}", v)));
    }
}

// ===========================================================================
// Row 9 — converts cleanly (errno == 0) but < INT_MIN
// ===========================================================================
#[test]
fn row09_below_int_min_errno_clean() {
    for s in [
        "-2147483649",  // INT_MIN - 1
        "-2147483650",
        "-4294967296",
        "-10000000000",
        "  -0002147483649",
    ] {
        assert_rejected(&format!("err09_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
    let mut rng = Rng::new(0xC009);
    for i in 0..128 {
        let v = i32::MIN as i64 - 1 - (rng.next_u64() % (i64::MAX as u64 - i32::MAX as u64 - 2)) as i64;
        assert_rejected(&format!("err09_rand{}", i), || Op::driver(&format!("{}", v)));
    }
}

// ===========================================================================
// Row 10 — LONG_MAX exactly: converts with errno == 0, fails `<= INT_MAX`
// ===========================================================================
#[test]
fn row10_long_max_exactly() {
    assert_rejected("err10_LONG_MAX", || Op::driver("9223372036854775807"));
    assert_rejected("err10_LONG_MAX_plus", || Op::driver("+9223372036854775807"));
    assert_rejected("err10_LONG_MAX_zeros", || {
        Op::driver("0000009223372036854775807")
    });
}

// ===========================================================================
// Row 11 — LONG_MIN exactly: converts with errno == 0, fails `>= INT_MIN`
// ===========================================================================
#[test]
fn row11_long_min_exactly() {
    assert_rejected("err11_LONG_MIN", || Op::driver("-9223372036854775808"));
    assert_rejected("err11_LONG_MIN_zeros", || {
        Op::driver("-0000009223372036854775808")
    });
}

// ===========================================================================
// Row 12 — NULL pointer: no guard exists, both must fault the SAME way
// ===========================================================================
#[test]
fn row12_null_pointer_faults_identically() {
    // `driver(NULL)` reaches `strtol(NULL, ...)`, which dereferences. The C code
    // has no null check, so the process dies. The Rust translation must die with
    // the SAME signal rather than, say, panicking or silently succeeding.
    let c_out = outcome_of(&pair().c, &Op::DriverNull);
    let r_out = outcome_of(&pair().rust, &Op::DriverNull);
    assert_eq!(
        c_out, r_out,
        "driver(NULL): C terminated as {:?} but Rust terminated as {:?} — \
         the two implementations must fault identically",
        c_out, r_out
    );
    // And pin down what that shared outcome actually is: SIGSEGV (11).
    assert_eq!(
        c_out,
        Outcome::Signaled(11),
        "expected driver(NULL) to die on SIGSEGV; got {:?}",
        c_out
    );
}

// ===========================================================================
// Row 13 — zero length / string that is only a NUL terminator
// ===========================================================================
#[test]
fn row13_zero_length_and_embedded_nul() {
    assert_rejected("err13_empty", || Op::driver(""));
    // A buffer whose first byte is NUL is an empty C string.
    assert_rejected("err13_raw_nul", || Op::driver_raw(b""));
    // Interior NUL truncates: the library only ever sees "12".
    assert_accepted("err13_interior_nul", || Op::driver_raw(b"12\0 34"));
    // Interior NUL right after garbage still rejects.
    assert_rejected("err13_interior_nul_bad", || Op::driver_raw(b"ab\0 12"));
}

// ===========================================================================
// Row 14 — oversized inputs
// ===========================================================================
#[test]
fn row14_oversized_inputs() {
    // 4096- and 100000-digit numbers overflow long => ERANGE.
    let d4096 = "7".repeat(4096);
    assert_rejected("err14_4096_digits", || Op::driver(&d4096));
    let d100k = "9".repeat(100_000);
    assert_rejected("err14_100k_digits", || Op::driver(&d100k));
    let neg100k = format!("-{}", "9".repeat(100_000));
    assert_rejected("err14_neg_100k_digits", || Op::driver(&neg100k));
    // 65536-byte non-numeric string.
    let junk = "z".repeat(65_536);
    assert_rejected("err14_64k_junk", || Op::driver(&junk));
    // Huge leading-zero run is NOT an overflow: it converts to a small value.
    let zeros = format!("{}5", "0".repeat(100_000));
    assert_accepted("err14_100k_zeros_then_5", || Op::driver(&zeros));
    // Huge whitespace run followed by a valid number also converts.
    let ws = format!("{}42", " ".repeat(65_536));
    assert_accepted("err14_64k_ws_then_42", || Op::driver(&ws));
}

// ===========================================================================
// Row 15 — one step INSIDE the valid range must SUCCEED
// ===========================================================================
#[test]
fn row15_boundary_inside_succeeds() {
    assert_accepted("err15_INT_MAX", || Op::driver("2147483647"));
    assert_accepted("err15_INT_MIN", || Op::driver("-2147483648"));
    assert_accepted("err15_INT_MAX_minus1", || Op::driver("2147483646"));
    assert_accepted("err15_INT_MIN_plus1", || Op::driver("-2147483647"));
}

// ===========================================================================
// Row 16 — one step PAST each documented range must FAIL
// ===========================================================================
#[test]
fn row16_boundary_one_past_fails() {
    // INT_MAX + 1 / INT_MIN - 1 (int range), LONG_MAX + 1 / LONG_MIN - 1 (ERANGE),
    // plus LONG_MAX / LONG_MIN exactly (clean convert, out of int range).
    for (name, s) in [
        ("INT_MAX_plus_1", "2147483648"),
        ("INT_MIN_minus_1", "-2147483649"),
        ("LONG_MAX", "9223372036854775807"),
        ("LONG_MAX_plus_1", "9223372036854775808"),
        ("LONG_MIN", "-9223372036854775808"),
        ("LONG_MIN_minus_1", "-9223372036854775809"),
    ] {
        assert_rejected(&format!("err16_{}", name), || Op::driver(s));
    }
}

// ===========================================================================
// Row 17 — trailing garbage is ACCEPTED (partial consumption is enough)
// ===========================================================================
#[test]
fn row17_trailing_garbage_accepted_not_an_error() {
    for s in ["42abc", "7 8", "1,000", "5-", "12.75", "3e4", "0x1F", "99]"] {
        assert_accepted(&format!("err17_{:x}", fnv(s.as_bytes())), || Op::driver(s));
    }
}

// ===========================================================================
// Row 18 — `run` accepts the FULL int domain (no enum in this API)
// ===========================================================================
#[test]
fn row18_run_has_no_error_path() {
    // The public API declares no enum, so the analogous "invalid variant across
    // FFI" case is the unconstrained int domain reaching `run` directly. Every
    // bit pattern is valid input and must never produce a rejection.
    let fixed = [0, 1, -1, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for v in fixed {
        let label = format!("err18_run_{}", v);
        assert_same(&label, &[Op::Run(v)]);
        for im in [&pair().c, &pair().rust] {
            let out = capture_one(im, &[Op::Run(v)]);
            assert!(
                !out.contains("An error occurred"),
                "[{}] {} rejected run({}), but run() has no error path:\n{}",
                label,
                im.name,
                v,
                out
            );
            assert_eq!(
                out.lines().filter(|l| l.starts_with("The house has")).count(),
                4,
                "[{}] {} did not emit run()'s 4 lines:\n{}",
                label,
                im.name,
                out
            );
        }
    }
    // Randomized full-domain sweep, differential only (cheaper).
    let mut rng = Rng::new(0xC018);
    for i in 0..256 {
        assert_same(&format!("err18_rand{}", i), &[Op::Run(rng.next_i32())]);
    }
}

// ===========================================================================
// Extra: rejection must be side-effect free even against evolving state
// ===========================================================================
#[test]
fn rejection_leaves_state_untouched_across_a_sequence() {
    // Interleave a known-good call with every rejection trigger. If a rejection
    // mutated `the_house` in only one implementation, the following good call's
    // output would diverge.
    let bad = [
        "", "abc", "   ", "+", "-", "x10", ".", "2147483648", "-2147483649",
        "9223372036854775807", "-9223372036854775808", "9223372036854775808",
        "-9223372036854775809", "99999999999999999999",
    ];
    let mut ops: Vec<Op> = Vec::new();
    for s in bad {
        ops.push(Op::driver(s));
        ops.push(Op::Run(1)); // canary: reveals any state drift immediately
        ops.push(Op::driver("10"));
    }
    assert_same("errX_reject_no_side_effects", &ops);
}

/// Small FNV-1a hash, used only to build unique temp-file labels from inputs.
fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}
