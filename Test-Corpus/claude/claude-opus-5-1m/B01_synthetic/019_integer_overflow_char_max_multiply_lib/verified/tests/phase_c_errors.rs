//! Phase C — error/rejection-path differential tests, one test per row of
//! ERRORS.md, plus the generic FFI-boundary cases.
//!
//! Every function in this library returns `void`, so the "same error result"
//! being asserted is the exact byte stream written to stdout — including the
//! *absence* of output, which is how `printLine`'s NULL rejection manifests.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// E1 — printLine(NULL): the only explicit null check in the library.
// ---------------------------------------------------------------------------

#[test]
fn e1_print_line_null_writes_nothing() {
    assert_same_and_eq("E1 printLine(NULL)", b"", |api| unsafe {
        (api.print_line)(ptr::null())
    });
}

#[test]
fn e1b_print_line_null_repeated_and_mixed_with_valid_calls() {
    // The rejection must be a silent no-op even when surrounded by real
    // output: no stray newline, no partial write, no stream corruption.
    assert_same_and_eq(
        "E1b printLine(NULL) interleaved",
        b"before\nafter\n",
        |api| unsafe {
            let a = CString::new("before").unwrap();
            let b = CString::new("after").unwrap();
            (api.print_line)(a.as_ptr());
            (api.print_line)(ptr::null());
            (api.print_line)(ptr::null());
            (api.print_line)(ptr::null());
            (api.print_line)(b.as_ptr());
        },
    );

    assert_same_and_eq("E1c printLine(NULL) x1000", b"", |api| unsafe {
        for _ in 0..1000 {
            (api.print_line)(ptr::null())
        }
    });
}

// ---------------------------------------------------------------------------
// E2 — printLine(""): passes the null check, degenerate content.
// ---------------------------------------------------------------------------

#[test]
fn e2_print_line_empty_string_writes_only_newline() {
    assert_same_and_eq("E2 printLine(\"\")", b"\n", |api| {
        call_print_line(api, b"")
    });
    assert_same_and_eq("E2b printLine(\"\") x5", b"\n\n\n\n\n", |api| {
        for _ in 0..5 {
            call_print_line(api, b"")
        }
    });
}

#[test]
fn e2c_print_line_null_and_empty_are_distinguishable() {
    // A Rust translation that conflated "NULL" with "empty" would pass E1 and
    // E2 individually but fail this.
    let p = pair();
    for lib in [&p.c, &p.rust] {
        let null_out = capture(|| unsafe { (lib.print_line)(ptr::null()) });
        let empty_out = capture(|| call_print_line(lib, b""));
        assert_eq!(null_out, b"", "[{}] NULL must emit nothing", lib.name);
        assert_eq!(empty_out, b"\n", "[{}] \"\" must emit \"\\n\"", lib.name);
        assert_ne!(
            null_out, empty_out,
            "[{}] NULL and \"\" must NOT be treated alike",
            lib.name
        );
    }
}

// ---------------------------------------------------------------------------
// E3 — printLine with printf conversion specifiers in the payload.
// ---------------------------------------------------------------------------

#[test]
fn e3_print_line_format_specifiers_are_literal() {
    // `printf("%s\n", line)` -> the payload is data, never a format string.
    let cases: &[&[u8]] = &[
        b"%s",
        b"%n",
        b"%d",
        b"%%",
        b"%p",
        b"%x",
        b"%hhn",
        b"%1000000d",
        b"%.*s",
        b"%s%n%d",
        b"%n%n%n%n%n%n%n%n",
        b"AAAA%08x.%08x.%08x.%08x",
        b"%s\n%s",
        b"50%% done",
    ];
    for payload in cases {
        let mut expected = payload.to_vec();
        expected.push(b'\n');
        assert_same_and_eq(
            &format!("E3 printLine({:?})", String::from_utf8_lossy(payload)),
            &expected,
            |api| call_print_line(api, payload),
        );
    }
}

// ---------------------------------------------------------------------------
// E4 — printLine with high-bit / invalid-UTF-8 bytes.
// ---------------------------------------------------------------------------

#[test]
fn e4_print_line_non_utf8_bytes_pass_through() {
    let cases: Vec<Vec<u8>> = vec![
        b"\x80".to_vec(),
        b"\xff".to_vec(),
        b"\xff\xfe\x80".to_vec(),
        b"\xc0\x80".to_vec(),     // overlong NUL encoding
        b"\xf5\x80\x80\x80".to_vec(), // beyond U+10FFFF
        b"\xed\xa0\xbd".to_vec(), // lone surrogate
        b"a\xffb\xfec".to_vec(),
        (0x80u8..=0xff).collect(),
    ];
    for payload in &cases {
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_same_and_eq("E4 printLine(non-UTF-8)", &expected, |api| {
            call_print_line(api, payload)
        });
    }

    // Randomized high-bit-heavy payloads.
    let mut rng = Rng::new(SEED ^ 0xE004);
    for i in 0..256 {
        let len = 1 + rng.below(64) as usize;
        let payload: Vec<u8> = (0..len).map(|_| 0x80 | (rng.below(128) as u8)).collect();
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("E4b #{i} printLine(high-bit)"), &expected, |api| {
            call_print_line(api, &payload)
        });
    }
}

// ---------------------------------------------------------------------------
// E5 — printLine with an embedded newline.
// ---------------------------------------------------------------------------

#[test]
fn e5_print_line_embedded_newlines_are_not_deduplicated() {
    let cases: &[&[u8]] = &[
        b"\n",
        b"\n\n",
        b"a\nb",
        b"a\n",
        b"\na",
        b"a\n\n\nb\n\n",
        b"\r\n",
        b"line\r\n",
    ];
    for payload in cases {
        let mut expected = payload.to_vec();
        expected.push(b'\n'); // printf appends unconditionally
        assert_same_and_eq("E5 printLine(embedded \\n)", &expected, |api| {
            call_print_line(api, payload)
        });
    }
}

// ---------------------------------------------------------------------------
// E6 — printLine with a very long payload (> BUFSIZ).
// ---------------------------------------------------------------------------

#[test]
fn e6_print_line_very_long_payload() {
    let mut rng = Rng::new(SEED ^ 0xE006);
    for &len in &[8192usize, 65536, 65537, 100_000] {
        let payload = rng.next_ascii(len);
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("E6 printLine(len={len})"), &expected, |api| {
            call_print_line(api, &payload)
        });
    }
    // A 1 MiB single byte value, to be sure nothing chunks differently.
    let payload = vec![b'Z'; 1 << 20];
    let mut expected = payload.clone();
    expected.push(b'\n');
    assert_same_and_eq("E6b printLine(1MiB)", &expected, |api| {
        call_print_line(api, &payload)
    });
}

// ---------------------------------------------------------------------------
// E7 — printHexCharLine with negative values (out of %02x's unsigned domain).
// ---------------------------------------------------------------------------

#[test]
fn e7_print_hex_char_line_negative_sign_extends() {
    // char -> int promotion sign-extends, then %02x reinterprets as unsigned:
    // 8 hex digits, and the `02` minimum width never truncates.
    let cases: &[(i32, &[u8])] = &[
        (-1, b"ffffffff\n"),
        (-2, b"fffffffe\n"),
        (-15, b"fffffff1\n"),
        (-16, b"fffffff0\n"),
        (-127, b"ffffff81\n"),
        (-128, b"ffffff80\n"), // CHAR_MIN
    ];
    for &(v, expected) in cases {
        let cv = v as i8 as c_char;
        assert_same_and_eq(
            &format!("E7 printHexCharLine({v})"),
            expected,
            |api| unsafe { (api.print_hex_char_line)(cv) },
        );
    }

    // Every negative char value.
    for v in -128i32..0 {
        let cv = v as i8 as c_char;
        let out = assert_same(&format!("E7b printHexCharLine({v})"), |api| unsafe {
            (api.print_hex_char_line)(cv)
        });
        assert_eq!(out.len(), 9, "expected 8 hex digits + newline for {v}");
    }
}

// ---------------------------------------------------------------------------
// E8 — printHexCharLine(0): zero padded to width 2.
// ---------------------------------------------------------------------------

#[test]
fn e8_print_hex_char_line_zero_is_zero_padded() {
    assert_same_and_eq("E8 printHexCharLine(0)", b"00\n", |api| unsafe {
        (api.print_hex_char_line)(0)
    });
    // The whole 1-digit range must be padded, never emitted bare.
    for v in 1i32..16 {
        let cv = v as i8 as c_char;
        let out = assert_same(&format!("E8b printHexCharLine({v})"), |api| unsafe {
            (api.print_hex_char_line)(cv)
        });
        assert_eq!(out.len(), 3, "expected 2 padded hex digits + newline for {v}");
        assert_eq!(out[0], b'0', "expected a leading zero for {v}");
    }
}

// ---------------------------------------------------------------------------
// E9 — out-of-`char`-range int handed across the FFI boundary.
// ---------------------------------------------------------------------------

#[test]
fn e9_print_hex_char_line_out_of_range_int_across_ffi() {
    // Calling the `char`-taking symbol through a widened `extern "C" fn(int)`
    // prototype: the SysV AMD64 ABI leaves the upper 24 bits of %edi
    // unspecified for a `char` parameter, so both callees must narrow to the
    // low byte and sign-extend it. Verified identical in both disassemblies.
    let vals: &[i32] = &[
        0x1234_5678,
        0x0000_0100, // 256: low byte 0
        0x0000_01ff, // low byte 0xff -> must print as -1
        0x7fff_ff80,
        -1000,
        -1,
        0,
        i32::MIN,
        i32::MAX,
        0xdead_beefu32 as i32,
        0xffff_ff7fu32 as i32,
    ];
    for &v in vals {
        // Both must agree with each other ...
        let out = assert_same(
            &format!("E9 printHexCharLine(widened {v:#x})"),
            |api| unsafe { (api.print_hex_char_line_widened)(v) },
        );
        // ... and with the properly-typed call using the truncated low byte.
        let narrowed = (v as u8) as i8 as c_char;
        let reference = assert_same(&format!("E9 reference({narrowed})"), |api| unsafe {
            (api.print_hex_char_line)(narrowed)
        });
        assert_eq!(
            out, reference,
            "printHexCharLine({v:#x}) must behave as printHexCharLine((char){narrowed})"
        );
    }

    let mut rng = Rng::new(SEED ^ 0xE009);
    for i in 0..512 {
        let v = rng.next_i32();
        assert_same(&format!("E9b #{i} printHexCharLine(widened {v:#x})"), |api| unsafe {
            (api.print_hex_char_line_widened)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// E10/E11 — bad(): the unreachable guard and the CWE-190 overflow itself.
// ---------------------------------------------------------------------------

#[test]
fn e10_bad_guard_is_always_taken() {
    // `data` is hard-coded to CHAR_MAX, so `if (data > 0)` can never fail:
    // bad() must NEVER produce empty output.
    for i in 0..200 {
        let out = assert_same(&format!("E10 bad() #{i}"), |api| unsafe { (api.bad)() });
        assert!(!out.is_empty(), "bad() must always emit a line");
        assert_eq!(out, b"fffffffe\n");
    }
}

#[test]
fn e11_bad_signed_char_overflow_wraps_and_does_not_trap() {
    // (char)(CHAR_MAX * 2) == (char)254 == -2. The Rust must wrap, not panic,
    // not saturate to CHAR_MAX ("7f"), and not stay unsigned ("fe").
    let out = assert_same("E11 bad() overflow", |api| unsafe { (api.bad)() });
    assert_eq!(out, b"fffffffe\n");
    assert_ne!(out, b"7f\n", "must not saturate");
    assert_ne!(out, b"fe\n", "must not treat the result as unsigned char");
    assert_ne!(out, b"00fe\n");
}

// ---------------------------------------------------------------------------
// E12/E13/E14/E15 — the good() paths, incl. the range-check rejection.
// ---------------------------------------------------------------------------

#[test]
fn e12_good_g2b_guard_is_always_taken() {
    // goodG2B: data = 2, `if (data > 0)` always true, prints (char)(2*2).
    let out = assert_same("E12 good() prefix", |api| unsafe { (api.good)() });
    assert!(
        out.starts_with(b"04\n"),
        "goodG2B must always emit \"04\\n\" first, got {out:?}"
    );
}

#[test]
fn e13_good_b2g_outer_guard_is_always_taken() {
    // goodB2G: data = CHAR_MAX, `if (data > 0)` always true, so the
    // diagnostic (or a hex line) must always follow goodG2B's output.
    let out = assert_same("E13 good() suffix", |api| unsafe { (api.good)() });
    assert!(
        out.len() > b"04\n".len(),
        "goodB2G's outer guard must always be entered, got {out:?}"
    );
}

#[test]
fn e14_good_b2g_range_check_rejects_the_arithmetic() {
    // The CWE-190 fix: `if (data < (CHAR_MAX/2))` is 127 < 63 -> FALSE, so the
    // else branch rejects the multiplication and emits the diagnostic.
    let expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    assert_same_and_eq("E14 good() rejection", expected, |api| unsafe {
        (api.good)()
    });

    let out = assert_same("E14b good()", |api| unsafe { (api.good)() });
    assert!(
        out.ends_with(b"data value is too large to perform arithmetic safely.\n"),
        "the rejection diagnostic must be emitted verbatim"
    );
    // Exactly ONE hex line (goodG2B's); goodB2G must NOT emit one.
    assert_eq!(
        out.iter().filter(|&&c| c == b'\n').count(),
        2,
        "expected exactly 2 lines: one hex line and the diagnostic"
    );
    assert!(
        !out.ends_with(b"fffffffe\n"),
        "goodB2G must not perform the unsafe arithmetic"
    );
}

#[test]
fn e15_good_b2g_dead_store_has_no_effect() {
    // `data = ' '` is immediately overwritten by `data = CHAR_MAX`, so the
    // 0x20 value must never be observable. (' ' * 2 == 0x40 -> "40\n".)
    let out = assert_same("E15 good() dead store", |api| unsafe { (api.good)() });
    let s = String::from_utf8_lossy(&out);
    assert!(!s.contains("40\n"), "the dead ' ' store leaked: {s:?}");
    assert!(!s.contains("20\n"), "the dead ' ' store leaked: {s:?}");
    assert_eq!(out, b"04\ndata value is too large to perform arithmetic safely.\n");
}

// ---------------------------------------------------------------------------
// E16 — driver(0).
// ---------------------------------------------------------------------------

#[test]
fn e16_driver_zero_takes_the_bad_branch() {
    let p = pair();
    for lib in [&p.c, &p.rust] {
        let d0 = capture(|| unsafe { (lib.driver)(0) });
        let b = capture(|| unsafe { (lib.bad)() });
        assert_eq!(d0, b, "[{}] driver(0) must be bad()", lib.name);
        assert_eq!(d0, b"fffffffe\n", "[{}]", lib.name);
    }
}

// ---------------------------------------------------------------------------
// E17 — nonzero useGood with a zero low byte.
// ---------------------------------------------------------------------------

#[test]
fn e17_driver_nonzero_with_zero_low_byte() {
    // `if (useGood)` tests the full int. A Rust bug narrowing to a byte would
    // wrongly route these to bad().
    let good_expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    let vals: &[i32] = &[
        0x0000_0100,
        0x0000_0200,
        0x0000_ff00,
        0x0001_0000,
        0x0100_0000,
        0x7f00_0000,
        0xffff_ff00u32 as i32,
        -256,
        -65536,
        -16_777_216,
        i32::MIN, // 0x8000_0000: every byte but the top one is zero
    ];
    for &v in vals {
        assert_same_and_eq(
            &format!("E17 driver({v:#x})"),
            good_expected,
            |api| unsafe { (api.driver)(v) },
        );
    }
    // Every value whose low 16 bits are zero but which is still nonzero.
    for hi in 1u32..=0xff {
        let v = (hi << 16) as i32;
        assert_same_and_eq(
            &format!("E17b driver({v:#x})"),
            good_expected,
            |api| unsafe { (api.driver)(v) },
        );
    }
}

// ---------------------------------------------------------------------------
// E18 — driver at the extremes of int.
// ---------------------------------------------------------------------------

#[test]
fn e18_driver_int_extremes() {
    let good_expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    for &v in &[i32::MIN, i32::MIN + 1, -2, -1, 1, 2, i32::MAX - 1, i32::MAX] {
        assert_same_and_eq(
            &format!("E18 driver({v})"),
            good_expected,
            |api| unsafe { (api.driver)(v) },
        );
    }
    // 0 is the only value that must NOT reach good().
    assert_same_and_eq("E18 driver(0)", b"fffffffe\n", |api| unsafe {
        (api.driver)(0)
    });
}

// ---------------------------------------------------------------------------
// E19 — out-of-range "enum-like" ints: no validation, no rejection.
// ---------------------------------------------------------------------------

#[test]
fn e19_driver_out_of_range_enum_like_values_are_not_rejected() {
    // `useGood` is a C `int`, so it accepts any bit pattern; there is no
    // variant validation and nothing is rejected. Values with no meaningful
    // "variant" must still be accepted and treated as true.
    let good_expected: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
    let vals: &[i32] = &[
        2, 3, 4, 5, -7, -42, 0x7f, 0xff, 1000, 123_456_789, -123_456_789, 0x5555_5555,
        0xaaaa_aaaau32 as i32, 0x8000_0001u32 as i32,
    ];
    for &v in vals {
        assert_same_and_eq(
            &format!("E19 driver({v}) [no valid variant]"),
            good_expected,
            |api| unsafe { (api.driver)(v) },
        );
    }

    // Exhaustive over a contiguous window straddling zero, so the single
    // rejecting value (0) is found by search rather than assumed.
    let p = pair();
    for v in -300i32..=300 {
        let out = assert_same(&format!("E19b driver({v})"), |api| unsafe {
            (api.driver)(v)
        });
        let want: &[u8] = if v == 0 { b"fffffffe\n" } else { good_expected };
        assert_eq!(out, want, "driver({v}) misrouted");
    }
    let _ = p;

    let mut rng = Rng::new(SEED ^ 0xE019);
    for i in 0..1024 {
        let v = rng.next_i32();
        assert_same(&format!("E19c #{i} driver({v})"), |api| unsafe {
            (api.driver)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// E20 — statelessness / idempotence across repeated and interleaved calls.
// ---------------------------------------------------------------------------

#[test]
fn e20_all_exports_are_stateless_and_idempotent() {
    let p = pair();
    let msg = CString::new("x").unwrap();

    // Each entry point called many times must yield an exact repetition, and
    // calling the others in between must not perturb it.
    for lib in [&p.c, &p.rust] {
        let one_bad = capture(|| unsafe { (lib.bad)() });
        let one_good = capture(|| unsafe { (lib.good)() });

        let ten_bad = capture(|| {
            for _ in 0..10 {
                unsafe { (lib.bad)() }
            }
        });
        assert_eq!(ten_bad, one_bad.repeat(10), "[{}] bad() not idempotent", lib.name);

        let ten_good = capture(|| {
            for _ in 0..10 {
                unsafe { (lib.good)() }
            }
        });
        assert_eq!(
            ten_good,
            one_good.repeat(10),
            "[{}] good() not idempotent",
            lib.name
        );

        // bad() after good() after printLine(NULL) etc. must be unchanged.
        let after_others = capture(|| unsafe {
            (lib.good)();
            (lib.print_line)(ptr::null());
            (lib.print_line)(msg.as_ptr());
            (lib.print_hex_char_line)(-128i32 as i8 as c_char);
            (lib.driver)(0);
            (lib.driver)(1);
            (lib.bad)();
        });
        assert!(
            after_others.ends_with(&one_bad),
            "[{}] bad() perturbed by preceding calls",
            lib.name
        );
    }

    // And the whole mixed sequence must match between the libraries.
    assert_same("E20 mixed sequence", |api| unsafe {
        (api.good)();
        (api.print_line)(ptr::null());
        (api.print_line)(msg.as_ptr());
        (api.print_hex_char_line)(-128i32 as i8 as c_char);
        (api.driver)(0);
        (api.driver)(1);
        (api.bad)();
    });
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary cases beyond the table.
// ---------------------------------------------------------------------------

#[test]
fn generic_print_line_pointer_edge_cases() {
    // A pointer to a NUL byte that is the LAST byte of an allocation: the C
    // must stop at the NUL and not read past it (checked under the same
    // allocation for both libraries).
    let buf: Vec<u8> = vec![0u8];
    let ptr0 = buf.as_ptr() as *const c_char;
    assert_same_and_eq("generic printLine(&\"\\0\")", b"\n", |api| unsafe {
        (api.print_line)(ptr0)
    });

    // Interior NUL: `%s` stops at the first NUL, the tail is never emitted.
    let with_interior: Vec<u8> = b"head\0tail\0".to_vec();
    let p = with_interior.as_ptr() as *const c_char;
    assert_same_and_eq("generic printLine(interior NUL)", b"head\n", |api| unsafe {
        (api.print_line)(p)
    });

    // Not the start of the allocation (offset pointer).
    let off = unsafe { with_interior.as_ptr().add(5) } as *const c_char;
    assert_same_and_eq("generic printLine(offset ptr)", b"tail\n", |api| unsafe {
        (api.print_line)(off)
    });
}

#[test]
fn generic_zero_and_oversized_lengths() {
    // There is no length parameter anywhere in this API (all strings are
    // NUL-terminated and all other parameters are scalars), so "zero length"
    // is the empty string and "oversized" is a multi-megabyte payload.
    assert_same_and_eq("generic zero length", b"\n", |api| {
        call_print_line(api, b"")
    });

    let big = vec![b'q'; 4 << 20]; // 4 MiB
    let mut expected = big.clone();
    expected.push(b'\n');
    assert_same_and_eq("generic oversized length (4MiB)", &expected, |api| {
        call_print_line(api, &big)
    });
}

#[test]
fn generic_one_step_past_every_documented_range() {
    // char domain: CHAR_MIN-1 and CHAR_MAX+1 do not exist as `char` values, so
    // the boundary is probed through the widened prototype (E9's mechanism).
    for &v in &[-129i32, -128, -127, 126, 127, 128, 129, 255, 256, 257] {
        assert_same(
            &format!("generic printHexCharLine(widened {v})"),
            |api| unsafe { (api.print_hex_char_line_widened)(v as c_int) },
        );
    }
    // int domain: the true extremes, plus one step inside each end.
    for &v in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        assert_same(&format!("generic driver({v})"), |api| unsafe {
            (api.driver)(v)
        });
    }
}

#[test]
fn generic_no_export_crashes_on_any_scalar_input() {
    // Sweep every export with adversarial scalars and require that neither
    // library aborts and that they always agree. (A Rust panic inside an
    // `extern "C"` function aborts the process, so surviving this test is
    // itself the assertion.)
    // NOTE: the values are precomputed, never drawn inside the closure — the
    // closure runs twice (once per library) and both runs must see the *same*
    // inputs for the comparison to mean anything.
    let mut rng = Rng::new(SEED ^ 0xFFFF);
    let chars: Vec<c_char> = (0..64).map(|_| rng.next_c_char()).collect();
    assert_same("generic adversarial sweep", |api| unsafe {
        (api.print_line)(ptr::null());
        (api.bad)();
        (api.good)();
        for &c in &chars {
            (api.print_hex_char_line)(c);
        }
    });
    let mut rng = Rng::new(SEED ^ 0xEEEE);
    let vals: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    assert_same("generic adversarial driver sweep", |api| {
        for &v in &vals {
            unsafe { (api.driver)(v) }
        }
    });
}
