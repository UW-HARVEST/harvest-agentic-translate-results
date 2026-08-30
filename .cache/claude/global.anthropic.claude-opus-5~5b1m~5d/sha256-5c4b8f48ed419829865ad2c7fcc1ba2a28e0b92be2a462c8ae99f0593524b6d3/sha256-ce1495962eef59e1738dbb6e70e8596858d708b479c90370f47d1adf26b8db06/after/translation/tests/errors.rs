//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each asserts the two implementations
//! produce the *same* rejection sentinel (the exact 18 bytes
//! `"An error occurred\n"`), not merely "both failed somehow".

mod harness;
use harness::*;

use std::ffi::{c_char, c_int};

// ===========================================================================
// E1 — endp == str : empty string
// ===========================================================================
#[test]
fn err_e1_empty_string() {
    assert_rejected(b"", "E1 empty string");
}

// ===========================================================================
// E2 — endp == str : no leading digits at all
// ===========================================================================
#[test]
fn err_e2_no_leading_digits() {
    let cases: &[&[u8]] = &[
        b"abc",
        b"!",
        b"?",
        b"/",           // char just below '0'
        b":",           // char just above '9'
        b"++1",
        b"--1",
        b"+-1",
        b"-+1",
        b".",
        b".5",
        b"+",
        b"-",
        b"e5",
        b"x10",
        b"NaN",
        b"inf",
        b"null",
        b"\x80\xff",    // non-ASCII bytes
        b"\x7f",
        b"'1'",
        b"(1)",
        b"[1]",
        b"#1",
        b"one",
        b"\\1",
    ];
    for c in cases {
        assert_rejected(c, &format!("E2 no digits {:?}", show(c)));
    }

    // Randomized: strings built only from bytes strtol can never start a
    // number with (excludes whitespace, digits and signs).
    let mut rng = Rng::new();
    let bad: Vec<u8> = (1u8..=255)
        .filter(|b| !b.is_ascii_digit())
        .filter(|&b| !matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'))
        .filter(|&b| b != b'+' && b != b'-')
        .collect();
    for i in 0..500 {
        let len = 1 + rng.below(8) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| bad[rng.below(bad.len() as u64) as usize])
            .collect();
        assert_rejected(&s, &format!("E2 random#{i} {:?}", show(&s)));
    }
}

// ===========================================================================
// E3 — endp == str : whitespace-only
// ===========================================================================
#[test]
fn err_e3_whitespace_only() {
    let ws: &[u8] = b" \t\n\x0b\x0c\r";
    for &w in ws {
        assert_rejected(&[w], &format!("E3 single ws {w:#04x}"));
        assert_rejected(&[w, w, w], &format!("E3 triple ws {w:#04x}"));
    }
    assert_rejected(b" \t\n\x0b\x0c\r", "E3 all ws");
    assert_rejected(b"   ", "E3 spaces");

    let mut rng = Rng::new();
    for i in 0..300 {
        let len = 1 + rng.below(12) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| ws[rng.below(ws.len() as u64) as usize])
            .collect();
        assert_rejected(&s, &format!("E3 random#{i}"));
    }
}

// ===========================================================================
// E4 — endp == str : sign not immediately followed by a digit
// ===========================================================================
#[test]
fn err_e4_sign_without_digits() {
    let cases: &[&[u8]] = &[
        b"+ 1", b"- 1", b"+\t1", b"-\n1", b"+abc", b"-abc", b"+.5", b"-.5", b"+ ", b"- ",
        b"  +  9", b"  -  9", b"+-", b"-+", b"++", b"--", b"+x10", b"-x10",
    ];
    for c in cases {
        assert_rejected(c, &format!("E4 {:?}", show(c)));
    }

    // Randomized: leading whitespace, a sign, more whitespace, then digits.
    let ws: &[u8] = b" \t\n\x0b\x0c\r";
    let mut rng = Rng::new();
    for i in 0..300 {
        let mut s = Vec::new();
        for _ in 0..rng.below(3) {
            s.push(ws[rng.below(6) as usize]);
        }
        s.push(if rng.next_u64() & 1 == 0 { b'+' } else { b'-' });
        for _ in 0..=rng.below(3) {
            s.push(ws[rng.below(6) as usize]);
        }
        s.extend_from_slice(rng.below(1_000_000).to_string().as_bytes());
        assert_rejected(&s, &format!("E4 random#{i} {:?}", show(&s)));
    }
}

// ===========================================================================
// E5 — errno == ERANGE : positive long overflow
// ===========================================================================
#[test]
fn err_e5_erange_positive() {
    let cases: &[&[u8]] = &[
        b"9223372036854775808",                 // LONG_MAX + 1
        b"9223372036854775809",
        b"+9223372036854775808",
        b"18446744073709551615",                // UINT64_MAX
        b"18446744073709551616",
        b"99999999999999999999999999",
        b"340282366920938463463374607431768211456",
        b"9223372036854775808abc",              // overflow *and* trailing junk
        b"  9223372036854775808",
        b"000000009223372036854775808",         // leading zeros do not help
    ];
    for c in cases {
        assert_rejected(c, &format!("E5 {:?}", show(c)));
    }

    // Randomized: 20..=80 digit numbers, first digit non-zero -> always > LONG_MAX.
    let mut rng = Rng::new();
    for i in 0..300 {
        let len = 20 + rng.below(61) as usize;
        let mut s = Vec::with_capacity(len + 1);
        if rng.next_u64() & 1 == 0 {
            s.push(b'+');
        }
        s.push(b'1' + rng.below(9) as u8);
        for _ in 1..len {
            s.push(b'0' + rng.below(10) as u8);
        }
        assert_rejected(&s, &format!("E5 random#{i}"));
    }

    // A 4096-digit monster.
    let mut big = vec![b'7'; 4096];
    big[0] = b'9';
    assert_rejected(&big, "E5 4096 digits");
}

// ===========================================================================
// E6 — errno == ERANGE : negative long overflow
// ===========================================================================
#[test]
fn err_e6_erange_negative() {
    let cases: &[&[u8]] = &[
        b"-9223372036854775809",                // LONG_MIN - 1
        b"-9223372036854775810",
        b"-18446744073709551616",
        b"-99999999999999999999999999",
        b"-340282366920938463463374607431768211456",
        b"-9223372036854775809xyz",
        b"   -9223372036854775809",
        b"-000000009223372036854775809",
    ];
    for c in cases {
        assert_rejected(c, &format!("E6 {:?}", show(c)));
    }

    let mut rng = Rng::new();
    for i in 0..300 {
        let len = 20 + rng.below(61) as usize;
        let mut s = vec![b'-'];
        s.push(b'1' + rng.below(9) as u8);
        for _ in 1..len {
            s.push(b'0' + rng.below(10) as u8);
        }
        assert_rejected(&s, &format!("E6 random#{i}"));
    }

    let mut big = vec![b'7'; 4097];
    big[0] = b'-';
    big[1] = b'9';
    assert_rejected(&big, "E6 4096 digits negative");
}

// ===========================================================================
// E7 — tmp < INT_MIN (parses cleanly as long, errno == 0)
// ===========================================================================
#[test]
fn err_e7_below_int_min() {
    let cases: &[&[u8]] = &[
        b"-2147483649",             // INT_MIN - 1
        b"-2147483650",
        b"-3000000000",
        b"-4294967296",
        b"-9223372036854775807",
        b"-9223372036854775808",    // LONG_MIN exactly: errno stays 0
        b"-0002147483649",
        b"  -2147483649",
        b"-2147483649trailing",
    ];
    for c in cases {
        assert_rejected(c, &format!("E7 {:?}", show(c)));
    }

    let mut rng = Rng::new();
    for i in 0..500 {
        let v = rng.range_i64(i64::MIN, i32::MIN as i64 - 1);
        let s = v.to_string();
        assert_rejected(s.as_bytes(), &format!("E7 random#{i} {s}"));
    }
}

// ===========================================================================
// E8 — tmp > INT_MAX (parses cleanly as long, errno == 0)
// ===========================================================================
#[test]
fn err_e8_above_int_max() {
    let cases: &[&[u8]] = &[
        b"2147483648",              // INT_MAX + 1
        b"2147483649",
        b"3000000000",
        b"4294967295",
        b"4294967296",
        b"9223372036854775806",
        b"9223372036854775807",     // LONG_MAX exactly: errno stays 0
        b"+2147483648",
        b"0002147483648",
        b"  +2147483648",
        b"2147483648trailing",
    ];
    for c in cases {
        assert_rejected(c, &format!("E8 {:?}", show(c)));
    }

    let mut rng = Rng::new();
    for i in 0..500 {
        let v = rng.range_i64(i32::MAX as i64 + 1, i64::MAX);
        let s = v.to_string();
        assert_rejected(s.as_bytes(), &format!("E8 random#{i} {s}"));
        let s2 = format!("+{v}");
        assert_rejected(s2.as_bytes(), &format!("E8 random+#{i} {s2}"));
    }
}

// ===========================================================================
// E9 — one step past the valid range, paired against the last valid values
// ===========================================================================
#[test]
fn err_e9_one_step_past_range() {
    // Last valid values are accepted...
    assert_accepted(b"2147483647", i32::MAX, "E9 INT_MAX accepted");
    assert_accepted(b"-2147483648", i32::MIN, "E9 INT_MIN accepted");
    assert_accepted(b"+2147483647", i32::MAX, "E9 +INT_MAX accepted");
    // ...and one step past is rejected.
    assert_rejected(b"2147483648", "E9 INT_MAX+1 rejected");
    assert_rejected(b"-2147483649", "E9 INT_MIN-1 rejected");
    assert_rejected(b"+2147483648", "E9 +INT_MAX+1 rejected");

    // Same story at the long boundary (ERANGE side).
    assert_rejected(b"9223372036854775807", "E9 LONG_MAX (range-rejected)");
    assert_rejected(b"9223372036854775808", "E9 LONG_MAX+1 (ERANGE)");
    assert_rejected(b"-9223372036854775808", "E9 LONG_MIN (range-rejected)");
    assert_rejected(b"-9223372036854775809", "E9 LONG_MIN-1 (ERANGE)");

    // Sweep +-4 around each boundary.
    for delta in -4i64..=4 {
        let v = i32::MAX as i64 + delta;
        let s = v.to_string();
        if v <= i32::MAX as i64 {
            assert_accepted(s.as_bytes(), v as i32, &format!("E9 sweep {s}"));
        } else {
            assert_rejected(s.as_bytes(), &format!("E9 sweep {s}"));
        }
        let v = i32::MIN as i64 + delta;
        let s = v.to_string();
        if v >= i32::MIN as i64 {
            assert_accepted(s.as_bytes(), v as i32, &format!("E9 sweep {s}"));
        } else {
            assert_rejected(s.as_bytes(), &format!("E9 sweep {s}"));
        }
    }
}

// ===========================================================================
// E10 / E11 — NULL pointers.
//
// The C has no null guard, so `driver(NULL)` / `run(NULL, x)` are UB and fault.
// We still verify the two implementations behave *identically* by running each
// call in a forked child and comparing the wait status (same terminating
// signal, or the same exit code if it somehow returns).
// ===========================================================================

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(c_int),
    Signaled(c_int),
    Other(c_int),
}

fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut st: c_int = 0;
        let r = waitpid(pid, &mut st as *mut c_int, 0);
        assert!(r > 0, "waitpid failed");
        // WIFEXITED / WIFSIGNALED, glibc encoding.
        if st & 0x7f == 0x7f {
            Outcome::Other(st)
        } else if st & 0x7f == 0 {
            Outcome::Exited((st >> 8) & 0xff)
        } else {
            Outcome::Signaled(st & 0x7f)
        }
    }
}

#[test]
fn err_e10_driver_null_pointer() {
    let l = libs();
    let c = outcome_of(|| unsafe { (l.c.driver)(std::ptr::null()) });
    let r = outcome_of(|| unsafe { (l.rs.driver)(std::ptr::null()) });
    assert_eq!(c, r, "driver(NULL): C {c:?} vs Rust {r:?}");
    assert_eq!(
        c,
        Outcome::Signaled(11),
        "expected both to fault with SIGSEGV (no null guard exists in the C), got {c:?}"
    );
}

#[test]
fn err_e11_run_null_pointer() {
    let l = libs();
    for extra in [0i32, 1, -1, i32::MAX, i32::MIN] {
        let c = outcome_of(|| unsafe { (l.c.run)(std::ptr::null_mut(), extra as c_int) });
        let r = outcome_of(|| unsafe { (l.rs.run)(std::ptr::null_mut(), extra as c_int) });
        assert_eq!(c, r, "run(NULL, {extra}): C {c:?} vs Rust {r:?}");
        assert_eq!(c, Outcome::Signaled(11), "expected SIGSEGV, got {c:?}");
    }
}

// ===========================================================================
// E12 — signed overflow wraparound (the C is built at -O0, so it wraps).
// Also covered by cfg_c15/c16/c17; kept here so the ERRORS.md row has a test.
// ===========================================================================
#[test]
fn err_e12_signed_overflow_wraps_identically() {
    // floors++ at INT_MAX
    diff_run(House::new(i32::MAX, 0, 1.0), 0, "E12 floors INT_MAX");
    // bedrooms += extra, overflowing both ways
    diff_run(House::new(0, i32::MAX, 1.0), 1, "E12 bedrooms up");
    diff_run(House::new(0, i32::MAX, 1.0), i32::MAX, "E12 bedrooms up max");
    diff_run(House::new(0, i32::MIN, 1.0), -1, "E12 bedrooms down");
    diff_run(House::new(0, i32::MIN, 1.0), i32::MIN, "E12 bedrooms down min");
    // via driver: 5 + x + (x) across the two internal run() calls
    for s in [
        "2147483647",
        "-2147483648",
        "2147483640",
        "1073741824",
        "-1073741824",
    ] {
        let x: i32 = s.parse().unwrap();
        assert_accepted(s.as_bytes(), x, &format!("E12 driver {s}"));
    }
}

// ===========================================================================
// Generic FFI boundary hardening (required even though not an ERRORS.md row):
// zero/oversized lengths and "enum-like" out-of-range ints.
// ===========================================================================
#[test]
fn err_generic_zero_and_oversized_inputs() {
    // Zero-length input.
    assert_rejected(b"", "generic zero length");

    // Oversized inputs: 1 MiB of digits (ERANGE) and 1 MiB of junk (no digits).
    let huge_digits = vec![b'9'; 1 << 20];
    assert_rejected(&huge_digits, "generic 1MiB digits");
    let huge_junk = vec![b'q'; 1 << 20];
    assert_rejected(&huge_junk, "generic 1MiB junk");
    let huge_ws = vec![b' '; 1 << 20];
    assert_rejected(&huge_ws, "generic 1MiB whitespace");

    // Oversized but *valid*: 1 MiB of leading zeros then "7".
    let mut padded = vec![b'0'; (1 << 20) - 1];
    padded.push(b'7');
    let out = diff_driver_raw(&padded, "generic 1MiB padded valid");
    assert_ne!(out, ERR_MSG);
    assert_eq!(out, model_driver(7));
}

#[test]
fn err_generic_out_of_range_int_across_ffi() {
    // `extra_bedrooms` is an `int` with no notion of a "valid variant"; the C
    // accepts every bit pattern. Same for the enum-sized extremes an
    // out-of-range C enum value would produce.
    let sentinels = [
        0i32,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7fff_ffff,
        -0x8000_0000,
        0x0000_ffff,
        0xffff_u32 as i32,
        0x0001_0000,
        1234567890,
        -1234567890,
        99999,
        -99999,
    ];
    for &e in &sentinels {
        diff_run(House::driver_default(), e, &format!("generic extra={e}"));
        diff_run(House::new(0, 0, 0.0), e, &format!("generic zero-house extra={e}"));
        diff_run(
            House::new(i32::MIN, i32::MIN, -0.0),
            e,
            &format!("generic min-house extra={e}"),
        );
    }
}

#[test]
fn err_generic_unterminated_only_via_embedded_nul() {
    // A NUL immediately at position 0 is the empty string (rejected); a NUL
    // after digits truncates the parse (accepted).
    let mut buf = b"5".to_vec();
    buf.push(0);
    buf.extend_from_slice(b"999");
    let l = libs();
    let z = {
        let mut v = buf.clone();
        v.push(0);
        v
    };
    let p = z.as_ptr() as *const c_char;
    let c_out = capture_stdout(|| unsafe { (l.c.driver)(p) });
    let rs_out = capture_stdout(|| unsafe { (l.rs.driver)(p) });
    assert_eq!(c_out, rs_out, "embedded NUL mismatch");
    assert_eq!(c_out, model_driver(5));

    let z2 = vec![0u8, b'9', b'9', 0];
    let p2 = z2.as_ptr() as *const c_char;
    let c_out = capture_stdout(|| unsafe { (l.c.driver)(p2) });
    let rs_out = capture_stdout(|| unsafe { (l.rs.driver)(p2) });
    assert_eq!(c_out, rs_out, "leading NUL mismatch");
    assert_eq!(c_out, ERR_MSG);
}

// ===========================================================================
// errno hygiene: a pre-existing errno must not cause a spurious rejection, and
// the two implementations must agree after a rejection has set errno.
// ===========================================================================
#[test]
fn err_errno_is_reset_and_does_not_leak() {
    // Force ERANGE first, then a perfectly valid parse must still succeed.
    for _ in 0..3 {
        assert_rejected(b"99999999999999999999999999", "errno prime ERANGE");
        assert_accepted(b"11", 11, "errno reset after ERANGE");
        assert_rejected(b"zzz", "errno no-conversion");
        assert_accepted(b"-11", -11, "errno reset after no-conversion");
    }
}
