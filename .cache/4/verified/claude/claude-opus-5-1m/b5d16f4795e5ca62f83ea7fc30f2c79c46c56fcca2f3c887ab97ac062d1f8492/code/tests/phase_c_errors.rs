// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md, plus the generic FFI-boundary cases (null
// pointers, zero/oversized lengths, one-step-past-range and out-of-range
// "enum" ints).
//
// Every one of the five exported functions returns `void`, so there is no error
// code or sentinel to compare. The observable rejection signal is therefore
// (a) the exact bytes written to stdout — for a rejected input, ZERO bytes —
// and (b) returning normally instead of trapping. Each test asserts the C and
// Rust agree on both, and additionally pins the absolute expected bytes so a
// "both sides silently print nothing" regression cannot pass vacuously.

mod common;

use common::{assert_same, capture, cstr, libs};
use std::ffi::{c_char, c_int};

/// Asserts C and Rust agree AND that both match the exact expected byte string.
fn assert_same_and_eq<F>(what: &str, expected: &[u8], mut run: F)
where
    F: FnMut(&common::Lib),
{
    assert_same(what, &mut run);

    let l = libs();
    let c_out = capture(|| run(&l.c));
    let r_out = capture(|| run(&l.rust));
    assert_eq!(
        c_out,
        expected,
        "{what}: C produced {:?}, expected {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(expected)
    );
    assert_eq!(
        r_out,
        expected,
        "{what}: Rust produced {:?}, expected {:?}",
        String::from_utf8_lossy(&r_out),
        String::from_utf8_lossy(expected)
    );
}

// ---------------------------------------------------------------------------
// Row 1 — the library's ONLY input-rejection branch
// ---------------------------------------------------------------------------

/// `src/driver.c:32` — `if (line != NULL)`. A NULL pointer is rejected by
/// skipping the `printf` entirely: zero bytes, normal return, no crash.
#[test]
fn err_01_print_line_null() {
    assert_same_and_eq("printLine(NULL)", b"", |l| unsafe {
        (l.print_line)(std::ptr::null())
    });
}

/// The rejection must not be sticky: a NULL call followed by a valid call still
/// prints, and a NULL in the middle of a run drops only its own output.
#[test]
fn err_01b_print_line_null_interleaved() {
    let s = cstr(b"after");
    assert_same_and_eq(
        "printLine(NULL) interleaved with valid calls",
        b"before\nafter\n",
        |l| unsafe {
            let before = cstr(b"before");
            (l.print_line)(before.as_ptr().cast());
            (l.print_line)(std::ptr::null());
            (l.print_line)(std::ptr::null());
            (l.print_line)(s.as_ptr().cast());
        },
    );
}

// ---------------------------------------------------------------------------
// Rows 2, 3, 11 — length boundaries around the guard
// ---------------------------------------------------------------------------

/// Row 2: zero length. Non-NULL, so it PASSES the guard and emits exactly "\n".
/// (The distinction between NULL and "" is the classic place a translation
/// conflates "empty" with "absent".)
#[test]
fn err_02_print_line_empty() {
    let s = cstr(b"");
    assert_same_and_eq("printLine(\"\")", b"\n", |l| unsafe {
        (l.print_line)(s.as_ptr().cast())
    });
}

/// Row 3: oversized input — 1 MiB of non-NUL bytes then the terminator. C has
/// no length cap, so the whole payload plus one '\n' must come out.
#[test]
fn err_03_print_line_oversized() {
    const N: usize = 1 << 20;
    let payload = vec![b'Z'; N];
    let s = cstr(&payload);

    let mut expected = payload.clone();
    expected.push(b'\n');

    assert_same_and_eq("printLine(1 MiB)", &expected, |l| unsafe {
        (l.print_line)(s.as_ptr().cast())
    });
}

/// Row 11: a NUL at index 0 of a longer buffer. The C string ends immediately,
/// so only "\n" is emitted and the trailing bytes are ignored — even though the
/// allocation is much larger.
#[test]
fn err_11_print_line_embedded_nul() {
    let mut buf = vec![0u8; 1];
    buf.extend_from_slice(b"THIS MUST NOT BE PRINTED");
    buf.push(0);

    assert_same_and_eq("printLine(leading NUL)", b"\n", |l| unsafe {
        (l.print_line)(buf.as_ptr().cast())
    });

    // NUL in the middle: only the prefix is printed.
    let mut mid = Vec::from(&b"visible"[..]);
    mid.push(0);
    mid.extend_from_slice(b"hidden");
    mid.push(0);

    assert_same_and_eq("printLine(interior NUL)", b"visible\n", |l| unsafe {
        (l.print_line)(mid.as_ptr().cast())
    });
}

// ---------------------------------------------------------------------------
// Rows 4-7 — `useGood` is an int flag that accepts any of 2^32 values
// ---------------------------------------------------------------------------

/// Row 4: the false branch.
#[test]
fn err_04_driver_zero() {
    assert_same_and_eq("driver(0)", b"0\n", |l| unsafe { (l.driver)(0) });
}

/// Row 5: INT_MIN — out of range for a 0/1 flag, negative, and still truthy in C.
#[test]
fn err_05_driver_int_min() {
    assert_same_and_eq("driver(INT_MIN)", b"0\n", |l| unsafe {
        (l.driver)(c_int::MIN)
    });
}

/// Row 6: INT_MAX.
#[test]
fn err_06_driver_int_max() {
    assert_same_and_eq("driver(INT_MAX)", b"0\n", |l| unsafe {
        (l.driver)(c_int::MAX)
    });
}

/// Row 7: out-of-range "enum" values crossing the FFI boundary. A C enum/flag
/// accepts any int, so values with no valid variant are real inputs. Every
/// nonzero value must take the `good()` branch; only exact 0 takes `bad()`.
#[test]
fn err_07_driver_enum_range() {
    let out_of_range: [c_int; 14] = [
        -1,
        2,
        3,
        42,
        0x100,
        0xFFFF,
        0x7FFF_FFFE,
        -2,
        -42,
        1 << 16,
        1 << 30,
        i32::MIN + 1,
        i32::MAX - 1,
        0x0100_0000,
    ];
    for v in out_of_range {
        // Every one of these is nonzero, hence truthy, hence `good()`.
        assert_same_and_eq(&format!("driver({v}) [out-of-range flag]"), b"0\n", |l| unsafe {
            (l.driver)(v)
        });
    }

    // One step on either side of the only special value.
    assert_same_and_eq("driver(0) [only falsey value]", b"0\n", |l| unsafe {
        (l.driver)(0)
    });
    assert_same_and_eq("driver(1)", b"0\n", |l| unsafe { (l.driver)(1) });
    assert_same_and_eq("driver(-1)", b"0\n", |l| unsafe { (l.driver)(-1) });
}

/// `driver` prints the same bytes on both branches, so byte equality alone
/// cannot prove the flag was routed correctly. This pins the routing by
/// comparing each branch against a DIRECT call to the function it must select,
/// on both libraries independently.
#[test]
fn err_07b_driver_routes_to_correct_branch() {
    let l = libs();
    for lib in [&l.c, &l.rust] {
        let via_driver_false = capture(|| unsafe { (lib.driver)(0) });
        let direct_bad = capture(|| unsafe { (lib.bad)() });
        assert_eq!(
            via_driver_false, direct_bad,
            "{}: driver(0) must behave like bad()",
            lib.name
        );

        for truthy in [1, -1, 2, i32::MIN, i32::MAX] {
            let via_driver_true = capture(|| unsafe { (lib.driver)(truthy) });
            let direct_good = capture(|| unsafe { (lib.good)() });
            assert_eq!(
                via_driver_true, direct_good,
                "{}: driver({truthy}) must behave like good()",
                lib.name
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 8, 9 — printIntLine value boundaries
// ---------------------------------------------------------------------------

/// Row 8: INT_MIN has no positive counterpart; a naive `abs()` in a translation
/// would overflow here.
#[test]
fn err_08_print_int_line_int_min() {
    assert_same_and_eq("printIntLine(INT_MIN)", b"-2147483648\n", |l| unsafe {
        (l.print_int_line)(c_int::MIN)
    });
}

/// Row 9: INT_MAX.
#[test]
fn err_09_print_int_line_int_max() {
    assert_same_and_eq("printIntLine(INT_MAX)", b"2147483647\n", |l| unsafe {
        (l.print_int_line)(c_int::MAX)
    });
}

/// One step past each documented boundary, and the sign-transition neighbours.
#[test]
fn err_09b_print_int_line_one_past_boundaries() {
    let cases: [(c_int, &[u8]); 8] = [
        (c_int::MIN + 1, b"-2147483647\n"),
        (c_int::MAX - 1, b"2147483646\n"),
        (-1, b"-1\n"),
        (0, b"0\n"),
        (1, b"1\n"),
        (-10, b"-10\n"),
        (10, b"10\n"),
        (-2147483647, b"-2147483647\n"),
    ];
    for (v, expected) in cases {
        assert_same_and_eq(&format!("printIntLine({v})"), expected, |l| unsafe {
            (l.print_int_line)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 10 — format-string injection
// ---------------------------------------------------------------------------

/// Row 10: the C is `printf("%s\n", line)`, so specifiers in `line` are literal
/// data. A translation that passed `line` as the FORMAT would consume garbage
/// varargs (or, with `%n`, be exploitable / crash). Asserting the exact literal
/// bytes catches that.
#[test]
fn err_10_print_line_format_injection() {
    let cases: [(&[u8], &[u8]); 7] = [
        (b"%d", b"%d\n"),
        (b"%s", b"%s\n"),
        (b"%n", b"%n\n"),
        (b"%%", b"%%\n"),
        (b"%p %p %p %p", b"%p %p %p %p\n"),
        (b"%99999999d", b"%99999999d\n"),
        (b"%s%n%d%p%%", b"%s%n%d%p%%\n"),
    ];
    for (input, expected) in cases {
        let s = cstr(input);
        assert_same_and_eq(
            &format!("printLine({:?}) [format injection]", String::from_utf8_lossy(input)),
            expected,
            |l| unsafe { (l.print_line)(s.as_ptr().cast()) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 — the deliberate CWE-131 undersized allocation
// ---------------------------------------------------------------------------

/// Row 12: `bad()` writes 40 bytes through a 10-byte `alloca`. The C ground
/// truth still prints "0\n" and returns normally; the Rust must do the same.
/// Repeated calls check that neither side degrades after the overflow.
#[test]
fn err_12_bad_undersized_alloc() {
    assert_same_and_eq("bad() [CWE-131]", b"0\n", |l| unsafe { (l.bad)() });

    // Must stay stable across many overflowing calls, and must not corrupt a
    // subsequent good() call in the same frame chain.
    let expected: Vec<u8> = b"0\n".repeat(64);
    assert_same_and_eq("bad() x32 + good() x32 [CWE-131 stability]", &expected, |l| {
        for _ in 0..32 {
            unsafe { (l.bad)() }
        }
        for _ in 0..32 {
            unsafe { (l.good)() }
        }
    });
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary cases required by Phase C beyond the table
// ---------------------------------------------------------------------------

/// Misaligned / non-canonical but non-NULL pointer values are NOT dereferenced
/// by the guard itself, so we only test the pointers C can legally receive:
/// NULL (rejected) and a valid string at an odd alignment (accepted).
#[test]
fn err_generic_unaligned_string_pointer() {
    let mut buf = Vec::from(&b"_odd-aligned payload"[..]);
    buf.push(0);
    // Start one byte in, so the pointer is very unlikely to be word-aligned.
    let p = unsafe { buf.as_ptr().add(1) } as *const c_char;

    assert_same_and_eq(
        "printLine(unaligned interior pointer)",
        b"odd-aligned payload\n",
        |l| unsafe { (l.print_line)(p) },
    );
}

/// NULL passed repeatedly, and as the very first call into a freshly-used
/// library, must be inert on both sides.
#[test]
fn err_generic_null_storm() {
    assert_same_and_eq("printLine(NULL) x1000", b"", |l| {
        for _ in 0..1000 {
            unsafe { (l.print_line)(std::ptr::null()) }
        }
    });
}

/// The whole error surface exercised back-to-back in one capture window, mixing
/// rejected and accepted inputs, to be sure a rejection never corrupts the
/// stream for the calls around it.
#[test]
fn err_generic_mixed_valid_and_invalid_stream() {
    let ok = cstr(b"ok");
    let empty = cstr(b"");
    let fmt = cstr(b"%d%n");

    let expected: &[u8] = b"ok\n\n%d%n\n0\n-2147483648\n2147483647\n0\n0\n";

    assert_same_and_eq("mixed valid/invalid stream", expected, |l| unsafe {
        (l.print_line)(ok.as_ptr().cast());
        (l.print_line)(std::ptr::null()); // rejected, no bytes
        (l.print_line)(empty.as_ptr().cast());
        (l.print_line)(fmt.as_ptr().cast());
        (l.print_int_line)(0);
        (l.print_int_line)(c_int::MIN);
        (l.print_int_line)(c_int::MAX);
        (l.driver)(0);
        (l.driver)(-1);
        (l.print_line)(std::ptr::null()); // rejected, no bytes
    });
}
