//! Phase C — error-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. Every test asserts the *same*
//! rejection, not merely "both failed": for `driver` the rejection is the exact
//! byte string `An error occurred\n` with no house lines, so the tests assert
//! both implementations produce that exact sentinel AND agree byte-for-byte.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const REJECT: &[u8] = b"An error occurred\n";

/// Assert both implementations reject `input` with the identical sentinel.
fn assert_both_reject(label: &str, input: &[u8]) {
    let mut s = input.to_vec();
    s.push(0);
    let p = pair();
    let c_out = capture(|| unsafe { (p.c.driver)(s.as_ptr() as *const c_char) });
    let r_out = capture(|| unsafe { (p.rs.driver)(s.as_ptr() as *const c_char) });

    assert_eq!(
        c_out,
        r_out,
        "[{label}] driver({:?}): C and Rust disagree\n C   : {}\n Rust: {}",
        show(input),
        show(&c_out),
        show(&r_out)
    );
    assert_eq!(
        c_out,
        REJECT,
        "[{label}] driver({:?}): expected the rejection sentinel, C produced {}",
        show(input),
        show(&c_out)
    );
    assert_eq!(
        r_out,
        REJECT,
        "[{label}] driver({:?}): Rust did not produce the rejection sentinel: {}",
        show(input),
        show(&r_out)
    );
}

/// Assert both implementations ACCEPT `input` identically (8 house lines).
fn assert_both_accept(label: &str, input: &[u8], expect_value: c_int) {
    let mut s = input.to_vec();
    s.push(0);
    let p = pair();
    let c_out = capture(|| unsafe { (p.c.driver)(s.as_ptr() as *const c_char) });
    let r_out = capture(|| unsafe { (p.rs.driver)(s.as_ptr() as *const c_char) });

    assert_eq!(
        c_out,
        r_out,
        "[{label}] driver({:?}): C and Rust disagree\n C   : {}\n Rust: {}",
        show(input),
        show(&c_out),
        show(&r_out)
    );
    assert_ne!(
        c_out,
        REJECT,
        "[{label}] driver({:?}) was expected to be ACCEPTED but C rejected it",
        show(input)
    );
    assert_eq!(
        c_out.iter().filter(|b| **b == b'\n').count(),
        8,
        "[{label}] driver({:?}): expected 8 house lines, got {}",
        show(input),
        show(&c_out)
    );
    // Confirm the parsed value really was `expect_value`: the C builds
    // house_t{2,5,2.5} and calls run twice, so bedrooms ends at 5 + 2*value.
    let expected_last = format!(
        "The house has 4 floors, {} bedrooms, and 4.5 bathrooms\n",
        5i32.wrapping_add(expect_value).wrapping_add(expect_value)
    );
    assert!(
        c_out.ends_with(expected_last.as_bytes()),
        "[{label}] driver({:?}): parsed value != {expect_value}; tail was {}",
        show(input),
        show(&c_out)
    );
}

// ===========================================================================
// E1 — empty string: strtol performs no conversion, endp == str
// ===========================================================================
#[test]
fn e1_empty_string() {
    common::isolated(|| {
    assert_both_reject("E1", b"");
    });
}

// ===========================================================================
// E2 — whitespace only
// ===========================================================================
#[test]
fn e2_whitespace_only() {
    common::isolated(|| {
    const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    for &w in WS.iter() {
        assert_both_reject(&format!("E2 single {w:#04x}"), &[w]);
        assert_both_reject(&format!("E2 double {w:#04x}"), &[w, w]);
    }
    assert_both_reject("E2 mix", b" \t\n\x0b\x0c\r \t");
    let mut rng = Rng::new(0xE002);
    for i in 0..200 {
        let n = rng.below(12) + 1;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(&WS)).collect();
        assert_both_reject(&format!("E2 rand#{i}"), &s);
    }
    });
}

// ===========================================================================
// E3 — sign only / repeated signs
// ===========================================================================
#[test]
fn e3_sign_only() {
    common::isolated(|| {
    for s in [
        &b"+"[..], b"-", b"++", b"--", b"+-", b"-+", b"  +", b"\t-", b"--1", b"+-1", b"-+1",
        b"++1", b"+ 1", b"- 1", b"+ ", b"- ",
    ] {
        assert_both_reject(&format!("E3 {}", show(s)), s);
    }
    });
}

// ===========================================================================
// E4 — first non-space char is neither digit nor sign
// ===========================================================================
#[test]
fn e4_non_numeric_lead() {
    common::isolated(|| {
    for s in [
        &b"abc"[..], b"x12", b".", b".5", b"e5", b"E5", b"#", b"/", b":", b"\xff", b"\x80",
        b"\x01", b"_1", b"(1)", b"$5",
    ] {
        if s.is_empty() {
            continue;
        }
        assert_both_reject(&format!("E4 {}", show(s)), s);
    }
    // Exhaustive single-byte sweep: every byte 0x01..=0xff. Digits and signs
    // that begin a conversion are accepted; everything else is rejected. The
    // point is that C and Rust must agree on the *whole* byte alphabet.
    let p = pair();
    for b in 1u16..=255 {
        let buf = [b as u8, 0u8];
        let c_out = capture(|| unsafe { (p.c.driver)(buf.as_ptr() as *const c_char) });
        let r_out = capture(|| unsafe { (p.rs.driver)(buf.as_ptr() as *const c_char) });
        assert_eq!(
            c_out, r_out,
            "E4 byte-sweep {b:#04x}: C={} Rust={}",
            show(&c_out),
            show(&r_out)
        );
        let is_digit = (b as u8).is_ascii_digit();
        if !is_digit {
            assert_eq!(
                c_out, REJECT,
                "E4 byte-sweep {b:#04x}: expected rejection, got {}",
                show(&c_out)
            );
        }
    }
    // Two-byte sweep over sign/space prefixes.
    for pre in [b'+', b'-', b' ', b'\t'] {
        for b in 1u16..=255 {
            let buf = [pre, b as u8, 0u8];
            let c_out = capture(|| unsafe { (p.c.driver)(buf.as_ptr() as *const c_char) });
            let r_out = capture(|| unsafe { (p.rs.driver)(buf.as_ptr() as *const c_char) });
            assert_eq!(
                c_out, r_out,
                "E4 pair-sweep {:?}+{b:#04x}: C={} Rust={}",
                pre as char,
                show(&c_out),
                show(&r_out)
            );
        }
    }
    });
}

// ===========================================================================
// E5 — base-10 vs hex/octal prefixes
// ===========================================================================
#[test]
fn e5_prefixes() {
    common::isolated(|| {
    // "0x" and friends START with a digit, so strtol(base 10) converts 0 and
    // stops at 'x' -> endp != str -> ACCEPTED with value 0.
    assert_both_accept("E5 0x", b"0x", 0);
    assert_both_accept("E5 0x10", b"0x10", 0);
    assert_both_accept("E5 0X1F", b"0X1F", 0);
    assert_both_accept("E5 0b101", b"0b101", 0);
    assert_both_accept("E5 0o17", b"0o17", 0);
    assert_both_accept("E5 017", b"017", 17); // NOT octal: base 10
    assert_both_accept("E5 -0x10", b"-0x10", 0);
    // No leading digit -> no conversion -> rejected.
    for s in [&b"x0"[..], b"X", b"xyz", b"x", b"X0", b"xFF"] {
        if s.is_empty() {
            continue;
        }
        assert_both_reject(&format!("E5 {}", show(s)), s);
    }
    });
}

// ===========================================================================
// E6 — ERANGE, magnitude above LONG_MAX
// ===========================================================================
#[test]
fn e6_erange_above_long_max() {
    common::isolated(|| {
    assert_both_reject("E6 LONG_MAX+1", b"9223372036854775808");
    assert_both_reject("E6 LONG_MAX+2", b"9223372036854775809");
    assert_both_reject("E6 20 nines", b"99999999999999999999");
    assert_both_reject("E6 +LONG_MAX+1", b"+9223372036854775808");
    let big = "9".repeat(400);
    assert_both_reject("E6 400 nines", big.as_bytes());
    let mut rng = Rng::new(0xE006);
    for i in 0..200 {
        let extra_digits = rng.below(30) + 1;
        let mut s = String::from("9223372036854775808");
        for _ in 0..extra_digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        assert_both_reject(&format!("E6 rand#{i}"), s.as_bytes());
    }
    // With trailing junk too (ERANGE still dominates).
    assert_both_reject("E6 junk", b"99999999999999999999abc");
    });
}

// ===========================================================================
// E7 — ERANGE, magnitude below LONG_MIN
// ===========================================================================
#[test]
fn e7_erange_below_long_min() {
    common::isolated(|| {
    assert_both_reject("E7 LONG_MIN-1", b"-9223372036854775809");
    assert_both_reject("E7 LONG_MIN-2", b"-9223372036854775810");
    assert_both_reject("E7 -20 nines", b"-99999999999999999999");
    let big = format!("-{}", "9".repeat(400));
    assert_both_reject("E7 -400 nines", big.as_bytes());
    let mut rng = Rng::new(0xE007);
    for i in 0..200 {
        let extra_digits = rng.below(30) + 1;
        let mut s = String::from("-9223372036854775809");
        for _ in 0..extra_digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        assert_both_reject(&format!("E7 rand#{i}"), s.as_bytes());
    }
    });
}

// ===========================================================================
// E8 — tmp < INT_MIN (in long range, no errno)
// ===========================================================================
#[test]
fn e8_below_int_min() {
    common::isolated(|| {
    assert_both_reject("E8 INT_MIN-1", b"-2147483649");
    assert_both_reject("E8 INT_MIN-2", b"-2147483650");
    assert_both_reject("E8 -3e9", b"-3000000000");
    assert_both_reject("E8 LONG_MIN", b"-9223372036854775808");
    assert_both_reject("E8 LONG_MIN pad", b"-0009223372036854775808");
    let mut rng = Rng::new(0xE008);
    for i in 0..300 {
        let v = rng.range_i64(i64::MIN + 1, (i32::MIN as i64) - 1);
        assert_both_reject(&format!("E8 rand#{i}"), v.to_string().as_bytes());
    }
    });
}

// ===========================================================================
// E9 — tmp > INT_MAX (in long range, no errno)
// ===========================================================================
#[test]
fn e9_above_int_max() {
    common::isolated(|| {
    assert_both_reject("E9 INT_MAX+1", b"2147483648");
    assert_both_reject("E9 INT_MAX+2", b"2147483649");
    assert_both_reject("E9 3e9", b"3000000000");
    assert_both_reject("E9 LONG_MAX", b"9223372036854775807");
    assert_both_reject("E9 +LONG_MAX", b"+9223372036854775807");
    assert_both_reject("E9 LONG_MAX pad", b"00009223372036854775807");
    let mut rng = Rng::new(0xE009);
    for i in 0..300 {
        let v = rng.range_i64((i32::MAX as i64) + 1, i64::MAX);
        assert_both_reject(&format!("E9 rand#{i}"), v.to_string().as_bytes());
    }
    });
}

// ===========================================================================
// E10 — inclusive boundary: INT_MIN and INT_MAX MUST be accepted
// ===========================================================================
#[test]
fn e10_inclusive_boundaries() {
    common::isolated(|| {
    assert_both_accept("E10 INT_MAX", b"2147483647", i32::MAX);
    assert_both_accept("E10 INT_MIN", b"-2147483648", i32::MIN);
    assert_both_accept("E10 INT_MAX-1", b"2147483646", i32::MAX - 1);
    assert_both_accept("E10 INT_MIN+1", b"-2147483647", i32::MIN + 1);
    assert_both_accept("E10 +INT_MAX", b"+2147483647", i32::MAX);
    // one step past on each side is rejected (pairs with E8/E9)
    assert_both_reject("E10 INT_MAX+1", b"2147483648");
    assert_both_reject("E10 INT_MIN-1", b"-2147483649");
    });
}

// ===========================================================================
// E11 — trailing garbage is NOT a rejection in this C
// ===========================================================================
#[test]
fn e11_trailing_garbage_accepted() {
    common::isolated(|| {
    assert_both_accept("E11 12abc", b"12abc", 12);
    assert_both_accept("E11 '7 8'", b"7 8", 7);
    assert_both_accept("E11 5-", b"5-", 5);
    assert_both_accept("E11 1.9", b"1.9", 1);
    assert_both_accept("E11 -13xyz", b"-13xyz", -13);
    assert_both_accept("E11 1e5", b"1e5", 1);
    assert_both_accept("E11 nul-ish", b"3\x7f", 3);
    assert_both_accept("E11 highbyte", b"3\xff", 3);
    });
}

// ===========================================================================
// E12 — whitespace + sign + leading zeros + junk, accepted
// ===========================================================================
#[test]
fn e12_permissive_prefix_accepted() {
    common::isolated(|| {
    assert_both_accept("E12", b"  \t\n+0000042xyz", 42);
    assert_both_accept("E12 b", b"\r\x0b\x0c-0000007!!", -7);
    assert_both_accept("E12 c", b"        00000000000000000000001", 1);
    });
}

// ===========================================================================
// E13 — pre-existing errno is cleared by `errno = 0`
// ===========================================================================
#[test]
fn e13_stale_errno_cleared() {
    common::isolated(|| {
    const ERANGE: c_int = 34;
    const EINVAL: c_int = 22;
    const ENOMEM: c_int = 12;
    let p = pair();
    for &err in &[ERANGE, EINVAL, ENOMEM, 1, 4095] {
        let s = b"123\0";
        set_errno(err);
        let c_out = capture(|| unsafe { (p.c.driver)(s.as_ptr() as *const c_char) });
        set_errno(err);
        let r_out = capture(|| unsafe { (p.rs.driver)(s.as_ptr() as *const c_char) });
        assert_eq!(
            c_out, r_out,
            "E13 errno={err}: C={} Rust={}",
            show(&c_out),
            show(&r_out)
        );
        assert_ne!(
            c_out, REJECT,
            "E13 errno={err}: stale errno must not cause rejection"
        );
    }
    set_errno(0);
    });
}

// ===========================================================================
// E14 — no errno contamination across calls
// ===========================================================================
#[test]
fn e14_no_errno_leak_between_calls() {
    common::isolated(|| {
    let p = pair();
    let bad = b"99999999999999999999\0"; // sets ERANGE inside
    let good = b"77\0";
    let c_out = capture(|| unsafe {
        (p.c.driver)(bad.as_ptr() as *const c_char);
        (p.c.driver)(good.as_ptr() as *const c_char);
    });
    let r_out = capture(|| unsafe {
        (p.rs.driver)(bad.as_ptr() as *const c_char);
        (p.rs.driver)(good.as_ptr() as *const c_char);
    });
    assert_eq!(c_out, r_out, "E14: C={} Rust={}", show(&c_out), show(&r_out));
    assert!(
        c_out.starts_with(REJECT) && c_out.len() > REJECT.len(),
        "E14: expected reject then accept, got {}",
        show(&c_out)
    );
    // Cross-library contamination: C then Rust then C.
    let mixed_c = capture(|| unsafe {
        (p.c.driver)(bad.as_ptr() as *const c_char);
        (p.rs.driver)(good.as_ptr() as *const c_char);
    });
    let mixed_r = capture(|| unsafe {
        (p.rs.driver)(bad.as_ptr() as *const c_char);
        (p.c.driver)(good.as_ptr() as *const c_char);
    });
    assert_eq!(
        mixed_c, mixed_r,
        "E14 cross: {} vs {}",
        show(&mixed_c),
        show(&mixed_r)
    );
    });
}

// ===========================================================================
// E15 — driver(NULL): unchecked null, passed to strtol
// ===========================================================================
#[test]
fn e15_driver_null_pointer() {
    common::isolated(|| {
    let p = pair();
    let c_status = child_status(|| unsafe { (p.c.driver)(std::ptr::null()) });
    let r_status = child_status(|| unsafe { (p.rs.driver)(std::ptr::null()) });
    assert_eq!(
        describe_status(c_status),
        describe_status(r_status),
        "E15 driver(NULL): C {} vs Rust {}",
        describe_status(c_status),
        describe_status(r_status)
    );
    });
}

// ===========================================================================
// E16 — run(NULL): unchecked null deref
// ===========================================================================
#[test]
fn e16_run_null_pointer() {
    common::isolated(|| {
    let p = pair();
    for extra in [0i32, 1, -1, i32::MAX, i32::MIN] {
        let c_status = child_status(|| unsafe { (p.c.run)(std::ptr::null_mut(), extra) });
        let r_status = child_status(|| unsafe { (p.rs.run)(std::ptr::null_mut(), extra) });
        assert_eq!(
            describe_status(c_status),
            describe_status(r_status),
            "E16 run(NULL,{extra}): C {} vs Rust {}",
            describe_status(c_status),
            describe_status(r_status)
        );
    }
    // Also a couple of other invalid, non-null pointers (misaligned / unmapped).
    for bogus in [1usize, 8, 0xdead_beef, usize::MAX & !7] {
        let c_status = child_status(|| unsafe { (p.c.run)(bogus as *mut House, 3) });
        let r_status = child_status(|| unsafe { (p.rs.run)(bogus as *mut House, 3) });
        assert_eq!(
            describe_status(c_status),
            describe_status(r_status),
            "E16 run({bogus:#x}): C {} vs Rust {}",
            describe_status(c_status),
            describe_status(r_status)
        );
    }
    });
}

// ===========================================================================
// E17 — floors++ overflow (no INT_MAX guard)
// ===========================================================================
#[test]
fn e17_floors_overflow() {
    common::isolated(|| {
    let p = pair();
    let start = House {
        floors: i32::MAX,
        bedrooms: 5,
        bathrooms: 2.5,
    };
    let mut hc = start;
    let c_out = capture(|| unsafe { (p.c.run)(&mut hc, 1) });
    let mut hr = start;
    let r_out = capture(|| unsafe { (p.rs.run)(&mut hr, 1) });
    assert_eq!(c_out, r_out, "E17: C={} Rust={}", show(&c_out), show(&r_out));
    assert_eq!(hc.raw(), hr.raw(), "E17 struct");
    // The C (gcc -O0) wraps: INT_MAX -> INT_MIN.
    assert!(
        String::from_utf8_lossy(&c_out).contains("-2147483648 floors"),
        "E17: expected wrap to INT_MIN, got {}",
        show(&c_out)
    );
    // Randomized around the boundary, incl. multi-call sequences.
    let mut rng = Rng::new(0xE017);
    for i in 0..200 {
        let h = House {
            floors: i32::MAX - rng.range_i32(0, 3),
            bedrooms: rng.next_i32(),
            bathrooms: rng.range_i32(-50, 50) as f64 / 4.0,
        };
        let extras: Vec<c_int> = (0..6).map(|_| rng.next_i32()).collect();
        diff_run_seq(&format!("E17 rand#{i}"), h, &extras);
    }
    });
}

// ===========================================================================
// E18 — bedrooms += overflow, both directions
// ===========================================================================
#[test]
fn e18_bedrooms_overflow() {
    common::isolated(|| {
    diff_run(
        "E18 max+1",
        House { floors: 2, bedrooms: i32::MAX, bathrooms: 2.5 },
        1,
    );
    diff_run(
        "E18 min-1",
        House { floors: 2, bedrooms: i32::MIN, bathrooms: 2.5 },
        -1,
    );
    diff_run(
        "E18 max+max",
        House { floors: 2, bedrooms: i32::MAX, bathrooms: 2.5 },
        i32::MAX,
    );
    diff_run(
        "E18 min+min",
        House { floors: 2, bedrooms: i32::MIN, bathrooms: 2.5 },
        i32::MIN,
    );
    let mut rng = Rng::new(0xE018);
    for i in 0..300 {
        let (bedrooms, e) = if i % 2 == 0 {
            (i32::MAX - rng.range_i32(0, 3), rng.range_i32(1, i32::MAX))
        } else {
            (i32::MIN + rng.range_i32(0, 3), rng.range_i32(i32::MIN, -1))
        };
        diff_run(
            &format!("E18 rand#{i}"),
            House { floors: rng.next_i32(), bedrooms, bathrooms: 2.5 },
            e,
        );
    }
    });
}

// ===========================================================================
// E19 — non-finite / extreme bathrooms
// ===========================================================================
#[test]
fn e19_extreme_bathrooms() {
    common::isolated(|| {
    let vals: [f64; 14] = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_dead_beef),
        f64::from_bits(0xfff0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        0.0,
        1e308,
        -1e308,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        5e-324,
    ];
    for (i, &b) in vals.iter().enumerate() {
        for &e in &[0i32, 1, -1, i32::MAX, i32::MIN] {
            diff_run(
                &format!("E19#{i} bits={:#018x} e={e}", b.to_bits()),
                House { floors: 2, bedrooms: 5, bathrooms: b },
                e,
            );
            // Repeated calls: inf/nan must stay put, MAX must overflow to inf.
            diff_run_seq(
                &format!("E19#{i} seq e={e}"),
                House { floors: 2, bedrooms: 5, bathrooms: b },
                &[e, e, e, e],
            );
        }
    }
    });
}

// ===========================================================================
// E20 — extreme extra_bedrooms
// ===========================================================================
#[test]
fn e20_extreme_extra_bedrooms() {
    common::isolated(|| {
    let mut rng = Rng::new(0xE020);
    for &e in &[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        for i in 0..60 {
            diff_run(
                &format!("E20 e={e} #{i}"),
                House {
                    floors: rng.next_i32(),
                    bedrooms: rng.next_i32(),
                    bathrooms: rng.range_i32(-100, 100) as f64 / 8.0,
                },
                e,
            );
        }
    }
    });
}

// ===========================================================================
// E21 — oversized input (1 MiB of digits)
// ===========================================================================
#[test]
fn e21_oversized_input() {
    common::isolated(|| {
    let big = "1".repeat(1 << 20);
    assert_both_reject("E21 1MiB ones", big.as_bytes());
    let big_neg = format!("-{}", "9".repeat(1 << 20));
    assert_both_reject("E21 1MiB nines neg", big_neg.as_bytes());
    // 1 MiB of leading zeros followed by a valid value -> ACCEPTED (no length
    // check anywhere), value 42.
    let padded = format!("{}42", "0".repeat(1 << 20));
    assert_both_accept("E21 1MiB zeros then 42", padded.as_bytes(), 42);
    // 1 MiB of whitespace -> rejected.
    let ws = " ".repeat(1 << 20);
    assert_both_reject("E21 1MiB spaces", ws.as_bytes());
    });
}

// ===========================================================================
// E22 — embedded NUL truncates (no explicit length taken)
// ===========================================================================
#[test]
fn e22_embedded_nul() {
    common::isolated(|| {
    let p = pair();
    // Leading NUL -> same as empty string.
    let buf = b"\0123\0";
    let c_out = capture(|| unsafe { (p.c.driver)(buf.as_ptr() as *const c_char) });
    let r_out = capture(|| unsafe { (p.rs.driver)(buf.as_ptr() as *const c_char) });
    assert_eq!(c_out, r_out, "E22 leading NUL");
    assert_eq!(c_out, REJECT, "E22 leading NUL should reject: {}", show(&c_out));

    // NUL after digits -> the trailing bytes are invisible.
    let buf2 = b"42\0xxxx\0";
    let c2 = capture(|| unsafe { (p.c.driver)(buf2.as_ptr() as *const c_char) });
    let r2 = capture(|| unsafe { (p.rs.driver)(buf2.as_ptr() as *const c_char) });
    assert_eq!(c2, r2, "E22 mid NUL");
    assert!(
        String::from_utf8_lossy(&c2).contains("89 bedrooms"),
        "E22: expected value 42 to be parsed, got {}",
        show(&c2)
    );

    // NUL right after a whitespace run.
    let buf3 = b"   \0123\0";
    let c3 = capture(|| unsafe { (p.c.driver)(buf3.as_ptr() as *const c_char) });
    let r3 = capture(|| unsafe { (p.rs.driver)(buf3.as_ptr() as *const c_char) });
    assert_eq!(c3, r3, "E22 ws then NUL");
    assert_eq!(c3, REJECT, "E22 ws then NUL: {}", show(&c3));
    });
}

// ===========================================================================
// Generic FFI-boundary boundaries required by Phase C, beyond the table:
// out-of-range "enum"-like ints, zero/oversized lengths, one-past-range.
// ===========================================================================

/// The library has no enum parameter, but `int extra_bedrooms` is the only int
/// crossing the FFI boundary, so every representable `int` — including values
/// that would be "no valid variant" for an enum — is exercised, plus the
/// pattern of values a C enum ABI would deliver (negatives, 0, huge).
#[test]
fn generic_int_boundary_sweep() {
    common::isolated(|| {
    let mut rng = Rng::new(0xF00D);
    let mut extras: Vec<c_int> = vec![
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -65536,
        -32769,
        -32768,
        -257,
        -256,
        -129,
        -128,
        -2,
        -1,
        0,
        1,
        2,
        127,
        128,
        255,
        256,
        32767,
        32768,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..100 {
        extras.push(rng.next_i32());
    }
    for (i, &e) in extras.iter().enumerate() {
        diff_run(
            &format!("GEN int#{i} e={e}"),
            House { floors: 2, bedrooms: 5, bathrooms: 2.5 },
            e,
        );
        diff_run(
            &format!("GEN int#{i} e={e} extreme"),
            House { floors: i32::MAX, bedrooms: i32::MIN, bathrooms: -0.0 },
            e,
        );
    }
    });
}

/// Zero-length and oversized string lengths for `driver`.
#[test]
fn generic_length_boundaries() {
    common::isolated(|| {
    assert_both_reject("GEN len=0", b"");
    for len in 1..=4usize {
        // Shortest possible accepted inputs.
        let s = "1".repeat(len);
        let v: i64 = s.parse().unwrap();
        assert_both_accept(&format!("GEN len={len}"), s.as_bytes(), v as c_int);
    }
    // Growing lengths across the accept/reject boundary (10 vs 11 digits).
    for len in 1..=25usize {
        let s = "1".repeat(len);
        let mut buf = s.clone().into_bytes();
        buf.push(0);
        let p = pair();
        let c_out = capture(|| unsafe { (p.c.driver)(buf.as_ptr() as *const c_char) });
        let r_out = capture(|| unsafe { (p.rs.driver)(buf.as_ptr() as *const c_char) });
        assert_eq!(
            c_out, r_out,
            "GEN ones len={len}: C={} Rust={}",
            show(&c_out),
            show(&r_out)
        );
    }
    // Oversized: 4 MiB.
    let huge = "5".repeat(4 << 20);
    assert_both_reject("GEN 4MiB", huge.as_bytes());
    });
}
