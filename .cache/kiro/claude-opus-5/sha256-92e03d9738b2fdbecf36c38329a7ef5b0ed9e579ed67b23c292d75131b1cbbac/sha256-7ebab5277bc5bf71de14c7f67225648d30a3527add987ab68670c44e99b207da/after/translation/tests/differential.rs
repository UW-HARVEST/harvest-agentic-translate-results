//! Differential tests: every exported symbol of the Rust cdylib is called
//! through `libloading` and its stdout compared byte-for-byte against the same
//! symbol in the C shared library.
//!
//! Ordered lowest-level first (`printLine`, `printHexCharLine`), then the
//! callers (`bad`, `good`), then the public entry point (`driver`).

mod common;

use std::ffi::{CString, c_char, c_int};

use common::{CharFn, IntFn, StrFn, VoidFn, assert_same, compare};

// ---------------------------------------------------------------------------
// Level 0: printLine(const char *)
// ---------------------------------------------------------------------------

#[test]
fn print_line_null_pointer() {
    let (c_out, rust_out) = compare::<StrFn, _>(b"printLine", |f| unsafe {
        f(std::ptr::null());
    });
    assert_same("printLine(NULL)", &c_out, &rust_out);
    assert!(c_out.is_empty(), "C printLine(NULL) should print nothing");
}

#[test]
fn print_line_strings() {
    // Includes printf format specifiers to confirm the string is passed as an
    // argument to "%s\n" rather than used as the format itself.
    let cases: &[&[u8]] = &[
        b"",
        b" ",
        b"a",
        b"hello world",
        b"data value is too large to perform arithmetic safely.",
        b"%s",
        b"%d %d %d",
        b"%n",
        b"100%",
        b"tab\there",
        b"line\nbreak",
        b"trailing space ",
        b"\x01\x02\x7f",
        b"\xc3\xa9\xe2\x82\xac", // UTF-8 bytes
        b"\xff\xfe\xfd",         // invalid UTF-8
        &[b'x'; 4096],
    ];

    for case in cases {
        let cstr = CString::new(*case).expect("no interior NUL in test case");
        let (c_out, rust_out) = compare::<StrFn, _>(b"printLine", |f| unsafe {
            f(cstr.as_ptr());
        });
        assert_same(
            &format!("printLine({:?})", String::from_utf8_lossy(case)),
            &c_out,
            &rust_out,
        );

        let mut expected = case.to_vec();
        expected.push(b'\n');
        assert_eq!(
            c_out, expected,
            "sanity check on C behaviour for {:?}",
            String::from_utf8_lossy(case)
        );
    }
}

#[test]
fn print_line_repeated_calls_share_buffering() {
    // Several calls in one capture window: verifies ordering and that both
    // libraries flush through the same libc stdout in the same way.
    let a = CString::new("first").unwrap();
    let b = CString::new("second").unwrap();
    let (c_out, rust_out) = compare::<StrFn, _>(b"printLine", |f| unsafe {
        f(a.as_ptr());
        f(std::ptr::null());
        f(b.as_ptr());
    });
    assert_same("printLine x3", &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// Level 0: printHexCharLine(char)
// ---------------------------------------------------------------------------

#[test]
fn print_hex_char_line_exhaustive() {
    // Every representable `char` value on this platform. `c_char` mirrors the
    // platform signedness, so this covers the full domain of the parameter.
    let lo = c_char::MIN as i32;
    let hi = c_char::MAX as i32;

    for v in lo..=hi {
        let value = v as c_char;
        let (c_out, rust_out) = compare::<CharFn, _>(b"printHexCharLine", |f| unsafe {
            f(value);
        });
        assert_same(&format!("printHexCharLine({v})"), &c_out, &rust_out);
    }
}

#[test]
fn print_hex_char_line_negative_promotion() {
    // The interesting case from `bad()`: 127 * 2 truncated to char is -2, which
    // printf("%02x") widens to the 32-bit pattern fffffffe on a signed-char
    // platform. Pin the C output down explicitly.
    let overflowed = ((c_char::MAX as c_int) * 2) as c_char;
    let (c_out, rust_out) = compare::<CharFn, _>(b"printHexCharLine", |f| unsafe {
        f(overflowed);
    });
    assert_same("printHexCharLine(CHAR_MAX*2)", &c_out, &rust_out);

    if c_char::MIN < 0 {
        assert_eq!(c_out, b"fffffffe\n", "C signed-char overflow rendering");
    }
}

// ---------------------------------------------------------------------------
// Level 1: bad(), good()
// ---------------------------------------------------------------------------

#[test]
fn bad_matches() {
    let (c_out, rust_out) = compare::<VoidFn, _>(b"bad", |f| unsafe { f() });
    assert_same("bad()", &c_out, &rust_out);
    assert!(!c_out.is_empty(), "bad() is expected to print something");
}

#[test]
fn good_matches() {
    let (c_out, rust_out) = compare::<VoidFn, _>(b"good", |f| unsafe { f() });
    assert_same("good()", &c_out, &rust_out);
    assert!(!c_out.is_empty(), "good() is expected to print something");
}

#[test]
fn bad_and_good_are_idempotent_across_calls() {
    // No hidden state: repeated invocations must keep matching.
    let (c_out, rust_out) = compare::<VoidFn, _>(b"bad", |f| unsafe {
        f();
        f();
        f();
    });
    assert_same("bad() x3", &c_out, &rust_out);

    let (c_out, rust_out) = compare::<VoidFn, _>(b"good", |f| unsafe {
        f();
        f();
    });
    assert_same("good() x2", &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// Level 2: driver(int)
// ---------------------------------------------------------------------------

#[test]
fn driver_all_truthiness_cases() {
    let values: &[c_int] = &[
        0,
        1,
        -1,
        2,
        -2,
        42,
        0x100,
        0xffff,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
    ];

    for &v in values {
        let (c_out, rust_out) = compare::<IntFn, _>(b"driver", |f| unsafe {
            f(v);
        });
        assert_same(&format!("driver({v})"), &c_out, &rust_out);
    }
}

#[test]
fn driver_dispatches_to_bad_and_good() {
    // driver(0) must reproduce bad(); any non-zero must reproduce good().
    let (c_bad, rust_bad) = compare::<VoidFn, _>(b"bad", |f| unsafe { f() });
    assert_same("bad() baseline", &c_bad, &rust_bad);

    let (c_good, rust_good) = compare::<VoidFn, _>(b"good", |f| unsafe { f() });
    assert_same("good() baseline", &c_good, &rust_good);

    let (c_zero, rust_zero) = compare::<IntFn, _>(b"driver", |f| unsafe { f(0) });
    assert_same("driver(0)", &c_zero, &rust_zero);
    assert_eq!(c_zero, c_bad, "C driver(0) should equal C bad()");
    assert_eq!(rust_zero, rust_bad, "Rust driver(0) should equal Rust bad()");

    let (c_one, rust_one) = compare::<IntFn, _>(b"driver", |f| unsafe { f(1) });
    assert_same("driver(1)", &c_one, &rust_one);
    assert_eq!(c_one, c_good, "C driver(1) should equal C good()");
    assert_eq!(rust_one, rust_good, "Rust driver(1) should equal Rust good()");
}

#[test]
fn driver_mixed_sequence() {
    let (c_out, rust_out) = compare::<IntFn, _>(b"driver", |f| unsafe {
        f(0);
        f(1);
        f(0);
        f(-7);
    });
    assert_same("driver sequence", &c_out, &rust_out);
}
