//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E15). Because every function in this
//! library returns `void`, the observable "error result" is (a) the exact bytes
//! written to stdout and (b) returning normally instead of aborting/panicking.
//! Each test therefore ALSO pins the exact expected C bytes, so a test cannot
//! pass by both sides "failing somehow" in the same way.

mod common;

use common::{cstr, pair, render, Rng};
use std::ffi::{c_char, c_int};

/// Assert C and Rust agree *and* that C produced exactly the documented bytes.
macro_rules! diff_and_pin {
    ($p:expr, $what:expr, $expected:expr, $run:expr) => {{
        let p: &mut common::Pair = $p;
        let expected: &[u8] = $expected;
        p.assert_same($what, $run);
        // Re-capture C on its own to pin the absolute expectation.
        let got = p.capture_c($run);
        assert_eq!(
            got,
            expected,
            "\n[{}] C produced {} but ERRORS.md documents {}",
            $what,
            render(&got),
            render(expected)
        );
    }};
}

// ---------------------------------------------------------------------------
// E1 / E2 / E3 / E4 -- printLine's NULL guard and accepted degenerate shapes
// ---------------------------------------------------------------------------

/// E1 — `line == NULL`: the `if(line != NULL)` guard fails ⇒ nothing printed.
#[test]
fn err_e1_print_line_null() {
    let mut p = pair();
    diff_and_pin!(&mut p, "E1 printLine(NULL)", b"", |lib| unsafe {
        (lib.print_line)(std::ptr::null())
    });
}

/// E1b — NULL repeatedly, and interleaved with a valid call, to be sure the
/// rejected call contributes nothing at all to the stream.
#[test]
fn err_e1b_print_line_null_interleaved() {
    let mut p = pair();
    let ok = cstr(b"x");
    diff_and_pin!(
        &mut p,
        "E1b printLine(NULL) interleaved",
        b"x\nx\n",
        |lib| unsafe {
            (lib.print_line)(std::ptr::null());
            (lib.print_line)(ok.as_ptr() as *const c_char);
            (lib.print_line)(std::ptr::null());
            (lib.print_line)(std::ptr::null());
            (lib.print_line)(ok.as_ptr() as *const c_char);
            (lib.print_line)(std::ptr::null());
        }
    );
}

/// E2 — zero-length string is *accepted* (only NULL is rejected).
#[test]
fn err_e2_print_line_empty() {
    let mut p = pair();
    let s = cstr(b"");
    diff_and_pin!(&mut p, "E2 printLine(\"\")", b"\n", |lib| unsafe {
        (lib.print_line)(s.as_ptr() as *const c_char)
    });
}

/// E3 — invalid UTF-8 must be emitted verbatim (no replacement chars, no panic).
#[test]
fn err_e3_print_line_invalid_utf8() {
    let mut p = pair();
    for body in [
        &b"\x80\xff\xfe"[..],
        &b"\xc3"[..],             // truncated 2-byte sequence
        &b"\xed\xa0\x80"[..],     // encoded surrogate
        &b"\xf5\x80\x80\x80"[..], // out of Unicode range
        &b"\xfe\xff"[..],
        &b"ok\xffbad\xfe"[..],
    ] {
        let s = cstr(body);
        let mut expected = body.to_vec();
        expected.push(b'\n');
        diff_and_pin!(
            &mut p,
            &format!("E3 printLine({})", render(body)),
            &expected,
            |lib| unsafe { (lib.print_line)(s.as_ptr() as *const c_char) }
        );
    }
}

/// E4 — 32 KiB payload full of `%` conversion specifiers: the data is an
/// argument to `"%s\n"`, never a format string, so it must appear literally.
#[test]
fn err_e4_print_line_percent_and_long() {
    let mut p = pair();
    let unit = b"%n%s%d%p%%";
    let mut body = Vec::new();
    while body.len() < 32 * 1024 {
        body.extend_from_slice(unit);
    }
    let s = cstr(&body);
    let mut expected = body.clone();
    expected.push(b'\n');
    diff_and_pin!(
        &mut p,
        "E4 printLine(32KiB of % specifiers)",
        &expected,
        |lib| unsafe { (lib.print_line)(s.as_ptr() as *const c_char) }
    );
}

// ---------------------------------------------------------------------------
// E5 / E6 / E7 -- printHexCharLine has NO guard at all
// ---------------------------------------------------------------------------

/// E5 — negative `char`: promotion sign-extends, `%02x` reinterprets as
/// `unsigned int` ⇒ eight hex digits and the `02` width is not honoured.
#[test]
fn err_e5_print_hex_negative() {
    let mut p = pair();
    for (v, expected) in [
        (-1i8, &b"ffffffff\n"[..]),
        (-2, &b"fffffffe\n"[..]),
        (-16, &b"fffffff0\n"[..]),
        (-127, &b"ffffff81\n"[..]),
        (-128, &b"ffffff80\n"[..]),
    ] {
        let v = v as c_char;
        diff_and_pin!(
            &mut p,
            &format!("E5 printHexCharLine({v})"),
            expected,
            |lib| unsafe { (lib.print_hex_char_line)(v) }
        );
    }
}

/// E6 — the falsy boundary value 0 takes the zero-pad path.
#[test]
fn err_e6_print_hex_zero() {
    let mut p = pair();
    diff_and_pin!(
        &mut p,
        "E6 printHexCharLine(0)",
        b"00\n",
        |lib| unsafe { (lib.print_hex_char_line)(0) }
    );
}

/// E7 — caller passes an `int` one (or many) steps past the `char` range; the
/// callee truncates mod 256 and sign-extends.
#[test]
fn err_e7_print_hex_out_of_char_range() {
    let mut p = pair();
    for (v, expected) in [
        (128i32, &b"ffffff80\n"[..]),
        (255, &b"ffffffff\n"[..]),
        (256, &b"00\n"[..]),
        (257, &b"01\n"[..]),
        (-129, &b"7f\n"[..]),
        (-256, &b"00\n"[..]),
        (c_int::MAX, &b"ffffffff\n"[..]),
        (c_int::MIN, &b"00\n"[..]),
    ] {
        diff_and_pin!(
            &mut p,
            &format!("E7 printHexCharLine(int {v:#x})"),
            expected,
            |lib| unsafe { (lib.print_hex_char_line_as_int)(v) }
        );
    }
}

// ---------------------------------------------------------------------------
// E8 -- bad(): the unguarded signed overflow (the CWE under test)
// ---------------------------------------------------------------------------

/// E8 — `CHAR_MAX * 2` truncated back to `char`. The Rust translation must not
/// panic on this (a naive `i8 * 2` would panic in a debug build) and must
/// reproduce the sign-extended print.
#[test]
fn err_e8_bad_overflow_no_panic() {
    let mut p = pair();
    diff_and_pin!(&mut p, "E8 bad()", b"fffffffe\n", |lib| unsafe {
        (lib.bad)()
    });
}

// ---------------------------------------------------------------------------
// E9 / E10 -- goodB2G's range check and its dead store
// ---------------------------------------------------------------------------

const TOO_LARGE: &[u8] = b"data value is too large to perform arithmetic safely.\n";

/// E9 — `if (data < (CHAR_MAX/2))` fails (127 < 63 is false) ⇒ the rejection
/// branch runs and the multiplication is never performed.
#[test]
fn err_e9_good_b2g_rejects() {
    let mut p = pair();
    // good() == goodG2B() ("04\n") followed by goodB2G()'s rejection message.
    let mut expected = b"04\n".to_vec();
    expected.extend_from_slice(TOO_LARGE);
    diff_and_pin!(&mut p, "E9 good()", &expected, |lib| unsafe {
        (lib.good)()
    });
    assert_eq!(TOO_LARGE.len(), 54, "rejection message length");
}

/// E10 — the dead store `data = ' '` (32) at driver.c:68 is immediately
/// overwritten. If it were honoured, `32 < 63` would hold and the accept
/// branch would print "40\n" instead of the rejection message.
#[test]
fn err_e10_good_b2g_dead_store_ignored() {
    let mut p = pair();
    let out = p.capture_c(|lib| unsafe { (lib.good)() });
    assert!(
        out.ends_with(TOO_LARGE),
        "C's good() must end with the rejection message, got {}",
        render(&out)
    );
    assert!(
        !out.contains(&b'4') || !out.windows(3).any(|w| w == b"40\n"),
        "the dead store must not be observable (no \"40\\n\" line): {}",
        render(&out)
    );
    let rust_out = p.capture_rust(|lib| unsafe { (lib.good)() });
    assert_eq!(
        out,
        rust_out,
        "E10 divergence: C {} vs Rust {}",
        render(&out),
        render(&rust_out)
    );
}

// ---------------------------------------------------------------------------
// E11..E14 -- driver's mode dispatch treated as a raw `int`
// ---------------------------------------------------------------------------

/// E11 — `useGood == 0` ⇒ bad().
#[test]
fn err_e11_driver_zero() {
    let mut p = pair();
    diff_and_pin!(&mut p, "E11 driver(0)", b"fffffffe\n", |lib| unsafe {
        (lib.driver)(0)
    });
}

/// E12 — negative flags are truthy in C.
#[test]
fn err_e12_driver_negative() {
    let mut p = pair();
    let mut expected = b"04\n".to_vec();
    expected.extend_from_slice(TOO_LARGE);
    for v in [-1i32, -2, -42, i32::MIN, -2147483647] {
        diff_and_pin!(
            &mut p,
            &format!("E12 driver({v})"),
            &expected,
            |lib| unsafe { (lib.driver)(v as c_int) }
        );
    }
}

/// E13 — non-zero flags whose low byte is zero are still truthy.
#[test]
fn err_e13_driver_low_byte_zero() {
    let mut p = pair();
    let mut expected = b"04\n".to_vec();
    expected.extend_from_slice(TOO_LARGE);
    for v in [
        256i32,
        512,
        0x0001_0000,
        0x0100_0000,
        i32::MIN,
        -256,
        -65536,
        0x7fff_ff00,
        0x0000_ff00,
    ] {
        assert_eq!(v as u8, 0, "test case {v:#x} must have a zero low byte");
        assert_ne!(v, 0);
        diff_and_pin!(
            &mut p,
            &format!("E13 driver({v:#x})"),
            &expected,
            |lib| unsafe { (lib.driver)(v as c_int) }
        );
    }
}

/// E14 — `useGood` is a plain `int`: there is no "invalid enum variant" to
/// reject, so every one of the 2^32 bit patterns is a legal input and the
/// behaviour must be partitioned purely by `== 0`. Sweep hand-picked extremes
/// plus a large randomized sample, and check the exact expected bytes for both
/// partitions (this is the out-of-range-enum-across-FFI class of bug).
#[test]
fn err_e14_driver_full_int_sweep() {
    let mut p = pair();
    let bad_out: &[u8] = b"fffffffe\n";
    let mut good_out = b"04\n".to_vec();
    good_out.extend_from_slice(TOO_LARGE);

    let mut cases: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        c_int::MAX,
        c_int::MIN,
        0x8000_0000u32 as i32,
        0x7fff_ffff,
        0xffff_ffffu32 as i32,
        0x0000_0100,
        0xdead_beefu32 as i32,
        0xcafe_babeu32 as i32,
    ];
    for bit in 0..32 {
        cases.push(1i32 << bit);
        cases.push(!(1i32 << bit));
    }
    let mut rng = Rng::new(0xE14E_14E1_4E14_E14E);
    for _ in 0..2048 {
        cases.push(rng.next_i32());
    }

    for v in cases {
        let expected: &[u8] = if v == 0 { bad_out } else { &good_out };
        diff_and_pin!(
            &mut p,
            &format!("E14 driver({v:#x})"),
            expected,
            |lib| unsafe { (lib.driver)(v) }
        );
    }
}

// ---------------------------------------------------------------------------
// E15 -- statelessness across repeated / interleaved calls on a shared stdout
// ---------------------------------------------------------------------------

/// E15 — no hidden per-library state: the Nth call equals the 1st, and driving
/// the C and Rust libraries alternately through the *same* process stdout does
/// not perturb either.
#[test]
fn err_e15_statelessness() {
    let mut p = pair();

    // (a) N calls == N repetitions of 1 call, for each entry point.
    let one_bad = p.capture_c(|lib| unsafe { (lib.bad)() });
    let many_bad = p.capture_c(|lib| unsafe {
        for _ in 0..50 {
            (lib.bad)()
        }
    });
    assert_eq!(many_bad, one_bad.repeat(50), "C bad() is not stateless");
    let many_bad_rust = p.capture_rust(|lib| unsafe {
        for _ in 0..50 {
            (lib.bad)()
        }
    });
    assert_eq!(many_bad, many_bad_rust);

    let one_good = p.capture_c(|lib| unsafe { (lib.good)() });
    let many_good = p.capture_c(|lib| unsafe {
        for _ in 0..50 {
            (lib.good)()
        }
    });
    assert_eq!(many_good, one_good.repeat(50), "C good() is not stateless");
    let many_good_rust = p.capture_rust(|lib| unsafe {
        for _ in 0..50 {
            (lib.good)()
        }
    });
    assert_eq!(many_good, many_good_rust);

    // (b) Alternating C/Rust calls in one capture must produce a stream where
    // the C-contributed and Rust-contributed halves are identical.
    let alternating = {
        let c: *const common::Lib = &p.c;
        let r: *const common::Lib = &p.rust;
        p.capture_raw(&mut || unsafe {
            for i in 0..25 {
                let lib = if i % 2 == 0 { &*c } else { &*r };
                (lib.driver)(i as c_int % 2);
                (lib.print_hex_char_line)((i as i8).wrapping_mul(7) as c_char);
                (lib.print_line)(std::ptr::null());
            }
        })
    };
    let all_c = {
        let c: *const common::Lib = &p.c;
        p.capture_raw(&mut || unsafe {
            for i in 0..25 {
                let lib = &*c;
                (lib.driver)(i as c_int % 2);
                (lib.print_hex_char_line)((i as i8).wrapping_mul(7) as c_char);
                (lib.print_line)(std::ptr::null());
            }
        })
    };
    assert_eq!(
        alternating,
        all_c,
        "interleaving the two libraries changed the transcript:\n  mixed: {}\n  all-C: {}",
        render(&alternating),
        render(&all_c)
    );
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks required by Phase C beyond the table.
// ---------------------------------------------------------------------------

/// Every pointer parameter in the API (there is exactly one) with NULL, plus
/// zero-length and oversized buffers.
#[test]
fn generic_null_and_size_boundaries() {
    let mut p = pair();
    // NULL
    p.assert_same("generic printLine(NULL)", |lib| unsafe {
        (lib.print_line)(std::ptr::null())
    });
    // zero length
    let z = cstr(b"");
    p.assert_same("generic printLine(zero-length)", |lib| unsafe {
        (lib.print_line)(z.as_ptr() as *const c_char)
    });
    // 1 MiB (oversized) -- well past every stdio buffer boundary
    let big = cstr(&vec![b'A'; 1024 * 1024]);
    p.assert_same("generic printLine(1MiB)", |lib| unsafe {
        (lib.print_line)(big.as_ptr() as *const c_char)
    });
    // exactly at common buffer boundaries
    for n in [
        1usize, 2, 3, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537,
    ] {
        let s = cstr(&vec![b'q'; n]);
        p.assert_same(&format!("generic printLine(len {n})"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// One step past the documented valid range for every scalar parameter.
#[test]
fn generic_one_past_range_boundaries() {
    let mut p = pair();
    // printHexCharLine: char domain and one step past on both ends.
    for v in [
        c_int::from(i8::MIN) - 1,
        c_int::from(i8::MIN),
        c_int::from(i8::MAX),
        c_int::from(i8::MAX) + 1,
        c_int::from(u8::MAX),
        c_int::from(u8::MAX) + 1,
    ] {
        p.assert_same(
            &format!("generic printHexCharLine(int {v})"),
            |lib| unsafe { (lib.print_hex_char_line_as_int)(v) },
        );
    }
    // driver: the "enum-like" flag one step past its only two documented
    // values (0 and 1).
    for v in [-1i32, 0, 1, 2, 3, c_int::MIN, c_int::MAX] {
        p.assert_same(&format!("generic driver({v})"), |lib| unsafe {
            (lib.driver)(v)
        });
    }
}
