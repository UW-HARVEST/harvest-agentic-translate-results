// Phase C -- error-path differential tests.
//
// One test per row of ERRORS.md. Each constructs the exact invalid input /
// boundary condition, calls BOTH implementations through their exported
// symbols, and asserts they reject/behave identically.
//
// The C library returns `void` from all four functions and has exactly one
// rejection branch (`if (line != NULL)`), so "same error" here means "same
// observable rejection": no output, no crash, normal return. The tests assert
// the *specific* expected sentinel behaviour, not merely "both did something".

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// Row 1 -- the ONLY input-rejection branch in the library
// ---------------------------------------------------------------------------

#[test]
fn row1_printline_null_prints_nothing() {
    // Differential: both must produce identical output for NULL.
    diff("ERRORS row1 printLine(NULL)", |imp| imp.print_line(std::ptr::null()));

    // Specific expected result (not just "both failed somehow"): the guard is
    // false, so printLine returns having emitted ZERO bytes.
    for imp in [c_default(), rust()] {
        let out = capture_stdout(|| imp.print_line(std::ptr::null()));
        assert!(
            out.is_empty(),
            "{}: printLine(NULL) must emit nothing, got {}",
            imp.name,
            show(&out)
        );
    }

    // And it must return normally (reaching here proves no crash), repeatedly.
    diff("ERRORS row1 printLine(NULL) x100", |imp| {
        for _ in 0..100 {
            imp.print_line(std::ptr::null());
        }
    });
}

// ---------------------------------------------------------------------------
// Row 2 -- empty string is NOT the same as NULL
// ---------------------------------------------------------------------------

#[test]
fn row2_printline_empty_string_prints_newline() {
    let buf = cstr(b"");
    diff("ERRORS row2 printLine(\"\")", |imp| {
        imp.print_line(buf.as_ptr() as *const c_char)
    });

    // The sharp distinction: NULL -> 0 bytes, "" -> exactly 1 byte ('\n').
    for imp in [c_default(), rust()] {
        let empty = capture_stdout(|| imp.print_line(buf.as_ptr() as *const c_char));
        let null = capture_stdout(|| imp.print_line(std::ptr::null()));
        assert_eq!(empty, b"\n", "{}: printLine(\"\") must emit one newline", imp.name);
        assert!(null.is_empty(), "{}: printLine(NULL) must emit nothing", imp.name);
        assert_ne!(
            empty, null,
            "{}: empty string and NULL must NOT behave the same",
            imp.name
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 -- minimal non-empty length
// ---------------------------------------------------------------------------

#[test]
fn row3_printline_single_byte() {
    for b in 1u8..=255 {
        let buf = cstr(&[b]);
        diff(&format!("ERRORS row3 printLine(0x{b:02x})"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
        let out = capture_stdout(|| rust().print_line(buf.as_ptr() as *const c_char));
        assert_eq!(out, vec![b, b'\n'], "row3 byte 0x{b:02x}");
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- format specifiers must never be interpreted
// ---------------------------------------------------------------------------

#[test]
fn row4_printline_format_specifiers_are_literal() {
    // `%n` is the dangerous one: if the Rust side ever passed `line` as the
    // format string, glibc would try to write through a bogus pointer.
    const PAYLOADS: &[&[u8]] = &[
        b"%n",
        b"%n%n%n%n%n%n%n%n",
        b"%s",
        b"%99999999s",
        b"%p%p%p%p",
        b"%%n",
        b"%",
        b"%%",
        b"AAAA%08x.%08x.%08x.%08x.%n",
    ];
    for (i, p) in PAYLOADS.iter().enumerate() {
        let buf = cstr(p);
        diff(&format!("ERRORS row4[{i}]"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
        // Expected result is the literal bytes plus '\n'.
        let out = capture_stdout(|| rust().print_line(buf.as_ptr() as *const c_char));
        assert_eq!(
            out,
            expected_print_line(p),
            "row4[{i}]: payload must be emitted literally"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- control bytes / NUL termination semantics
// ---------------------------------------------------------------------------

#[test]
fn row5_printline_control_bytes() {
    for &c in &[b'\n', b'\r', b'\t', 0x00u8.wrapping_add(1), 0x1b, 0x7f] {
        let buf = cstr(&[b'x', c, b'y']);
        diff(&format!("ERRORS row5 ctrl=0x{c:02x}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }

    // Interior NUL: output must stop at the first NUL in BOTH implementations.
    // (`cstr` forbids NUL, so build this buffer by hand.)
    let buf: Vec<u8> = b"visible\0hidden\0".to_vec();
    diff("ERRORS row5 interior NUL truncates", |imp| {
        imp.print_line(buf.as_ptr() as *const c_char)
    });
    for imp in [c_default(), rust()] {
        let out = capture_stdout(|| imp.print_line(buf.as_ptr() as *const c_char));
        assert_eq!(
            out, b"visible\n",
            "{}: output must stop at the first interior NUL",
            imp.name
        );
    }
}

// ---------------------------------------------------------------------------
// Row 6 -- non-UTF-8 bytes must pass through untouched
// ---------------------------------------------------------------------------

#[test]
fn row6_printline_high_bytes() {
    const PAYLOADS: &[&[u8]] = &[
        &[0xff, 0xfe, 0xfd],
        &[0x80, 0x81, 0x82],
        &[0xc0, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xf5, 0x90, 0x80, 0x80],
        &[0xc3],
    ];
    for (i, p) in PAYLOADS.iter().enumerate() {
        let buf = cstr(p);
        diff(&format!("ERRORS row6[{i}]"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
        let out = capture_stdout(|| rust().print_line(buf.as_ptr() as *const c_char));
        assert_eq!(
            out,
            expected_print_line(p),
            "row6[{i}]: invalid UTF-8 must pass through byte-for-byte"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 7 -- oversized lengths
// ---------------------------------------------------------------------------

#[test]
fn row7_printline_oversized() {
    for &len in &[4096usize, 262_144] {
        let payload: Vec<u8> = std::iter::repeat(b'Z').take(len).collect();
        let buf = cstr(&payload);
        diff(&format!("ERRORS row7 len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
        for imp in [c_default(), rust()] {
            let out = capture_stdout(|| imp.print_line(buf.as_ptr() as *const c_char));
            assert_eq!(
                out.len(),
                len + 1,
                "{}: len={len} must not be truncated",
                imp.name
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- out-of-range "enum-like" ints across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn row9_driver_nonzero_int_boundaries() {
    // The C API has no `enum` at all, so the analogous class of bug is an
    // arbitrary `int` in `useGood`. C's `if (useGood)` is true for every
    // non-zero value; a translation using `useGood > 0`, `useGood == 1`, or a
    // narrowing cast to `i8`/`u8` would fail some of these.
    let vals: &[i32] = &[
        1,
        -1,
        2,
        -2,
        7,
        255,
        256,        // would break an `as u8` cast
        0x0000_0100,
        0x0001_0000, // would break an `as u16` cast
        0x0100_0000,
        i32::MAX,
        i32::MIN, // would break `useGood > 0`
        i32::MIN + 1,
        i32::MAX - 1,
        0x8000_0000u32 as i32,
        0xffff_ff00u32 as i32,
        0x0000_ff00,
    ];
    for &v in vals {
        assert_ne!(v, 0);
        diff(&format!("ERRORS row9 driver({v})"), |imp| imp.driver(v));
        // Every non-zero value must take the good() branch => "string\n".
        for imp in [c_default(), rust()] {
            let out = capture_stdout(|| imp.driver(v));
            assert_eq!(
                out, b"string\n",
                "{}: driver({v}) must take the good() branch",
                imp.name
            );
        }
    }

    // Values whose low byte / low half-word is zero are the ones a narrowing
    // translation bug would silently turn into `false`.
    for shift in 8..32u32 {
        let v = 1i32.wrapping_shl(shift);
        if v == 0 {
            continue;
        }
        diff(&format!("ERRORS row9 driver(1<<{shift})"), |imp| imp.driver(v));
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- no initialization requirement / idempotence
// ---------------------------------------------------------------------------

#[test]
fn row10_repeated_invocation_is_idempotent() {
    // No init/teardown exists in the C API; each call must stand alone and
    // re-emit its output. Also verifies calling the low-level entry point
    // first, with no prior setup, behaves identically.
    let buf = cstr(b"idempotent");

    diff("ERRORS row10 printLine first, no setup", |imp| {
        imp.print_line(buf.as_ptr() as *const c_char)
    });
    diff("ERRORS row10 good() x25", |imp| {
        for _ in 0..25 {
            imp.good();
        }
    });
    diff("ERRORS row10 driver(1) x25", |imp| {
        for _ in 0..25 {
            imp.driver(1);
        }
    });
    diff("ERRORS row10 printLine x25", |imp| {
        for _ in 0..25 {
            imp.print_line(buf.as_ptr() as *const c_char);
        }
    });

    // Output must scale exactly linearly with the call count (no latching).
    for n in [1usize, 2, 3, 10] {
        for imp in [c_default(), rust()] {
            let out = capture_stdout(|| {
                for _ in 0..n {
                    imp.good();
                }
            });
            assert_eq!(
                out.len(),
                n * b"string\n".len(),
                "{}: good() x{n} output length",
                imp.name
            );
        }
    }
}
