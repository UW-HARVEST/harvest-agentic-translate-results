//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! No function in this library returns a status code, so "same error" is
//! asserted as "same emitted byte stream (possibly empty) and normal return
//! from the FFI call" — for both shared objects.

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------- E1
/// The library's only rejection: `printLine(NULL)` prints nothing at all.
#[test]
fn err_e1_print_line_null() {
    let c = capture_stdout(|| c_api().print_line(std::ptr::null::<c_char>()));
    let r = capture_stdout(|| rust_api().print_line(std::ptr::null::<c_char>()));
    assert_eq!(c, r, "printLine(NULL) diverges");
    assert!(c.is_empty(), "C printLine(NULL) must print nothing, got {c:?}");
    assert!(r.is_empty(), "Rust printLine(NULL) must print nothing, got {r:?}");

    // Repeated NULL calls must also stay silent and must not corrupt the stream.
    assert_same_and_eq("E1 NULL x100", b"", |api| {
        for _ in 0..100 {
            api.print_line(std::ptr::null());
        }
    });

    // NULL surrounded by valid calls: only the valid ones produce output.
    let ok = CBuf::new(b"ok");
    assert_same_and_eq("E1 NULL sandwiched", b"ok\nok\n", |api| {
        api.print_line(ok.as_ptr());
        api.print_line(std::ptr::null());
        api.print_line(ok.as_ptr());
    });
}

// ---------------------------------------------------------------------- E2
/// Zero-length string is NOT an error: the non-NULL check passes.
#[test]
fn err_e2_print_line_empty_is_not_an_error() {
    let empty = CBuf::new(b"");
    assert_same_and_eq("E2 empty string", b"\n", |api| api.print_line(empty.as_ptr()));
    assert_same_and_eq("E2 empty string x5", b"\n\n\n\n\n", |api| {
        for _ in 0..5 {
            api.print_line(empty.as_ptr());
        }
    });
}

// ---------------------------------------------------------------------- E3
/// Conversion specifiers in the *argument* must be emitted literally.
#[test]
fn err_e3_print_line_format_specifiers() {
    // `%n` is the dangerous one: if a translation ever passed the argument as a
    // format string, this would write through a garbage pointer / abort.
    let cases: &[&[u8]] = &[b"%n", b"%n%n%n%n%n%n%n%n", b"%s", b"%99999999d", b"%*d", b"%%n"];
    for (i, case) in cases.iter().enumerate() {
        let buf = CBuf::new(case);
        let mut expected = case.to_vec();
        expected.push(b'\n');
        assert_same_and_eq(&format!("E3 specifier #{i}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- E4
/// Arbitrary non-UTF-8 bytes must pass through verbatim (byte-oriented stdio).
#[test]
fn err_e4_print_line_non_utf8_bytes() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff],
        vec![0xc3],                   // truncated 2-byte sequence
        vec![0xe2, 0x82],             // truncated 3-byte sequence
        vec![0xf0, 0x9f, 0x92],       // truncated 4-byte sequence
        vec![0xed, 0xa0, 0x80],       // UTF-16 surrogate encoded as UTF-8
        vec![0xfe, 0xff],             // never valid UTF-8
        (0x80u8..=0xff).collect(),    // every high byte
        vec![0xc0, 0x80],             // overlong encoding of NUL
    ];
    for (i, case) in cases.iter().enumerate() {
        let buf = CBuf::new(case);
        let mut expected = case.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("E4 non-utf8 #{i}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- E5
/// No length limit exists; oversized inputs are not rejected.
#[test]
fn err_e5_print_line_oversized() {
    for &len in &[1usize, 4095, 4096, 65537, 1 << 18] {
        let bytes = vec![b'Z'; len];
        let buf = CBuf::new(&bytes);
        let mut expected = bytes.clone();
        expected.push(b'\n');
        assert_same_and_eq(&format!("E5 oversized {len}"), &expected, |api| {
            api.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------- E6
#[test]
fn err_e6_print_int_line_int_min() {
    assert_same_and_eq("E6 INT_MIN", b"-2147483648\n", |api| {
        api.print_int_line(i32::MIN)
    });
    assert_same_and_eq("E6 INT_MIN+1", b"-2147483647\n", |api| {
        api.print_int_line(i32::MIN + 1)
    });
}

// ---------------------------------------------------------------------- E7
#[test]
fn err_e7_print_int_line_int_max_and_past_range() {
    assert_same_and_eq("E7 INT_MAX", b"2147483647\n", |api| api.print_int_line(i32::MAX));
    assert_same_and_eq("E7 INT_MAX-1", b"2147483646\n", |api| {
        api.print_int_line(i32::MAX - 1)
    });
    // One step past INT_MAX wraps to INT_MIN when narrowed to a 32-bit `int`,
    // exactly as the C ABI requires on both sides.
    let wrapped = (i32::MAX as i64 + 1) as u32 as i32;
    let expected = format!("{wrapped}\n").into_bytes();
    assert_same_and_eq("E7 INT_MAX+1 narrowed", &expected, |api| {
        api.print_int_line(wrapped)
    });
    let wrapped2 = (i32::MIN as i64 - 1) as u32 as i32;
    let expected2 = format!("{wrapped2}\n").into_bytes();
    assert_same_and_eq("E7 INT_MIN-1 narrowed", &expected2, |api| {
        api.print_int_line(wrapped2)
    });
}

// ---------------------------------------------------------------------- E8
/// The C has no enum, so every `int` is in range — including values that would
/// be invalid discriminants for any enum a caller might imagine.
#[test]
fn err_e8_print_int_line_out_of_range_enum_values() {
    let values: &[i32] = &[-1, 0, 1, 2, 3, 42, 255, 256, 999_999, -999_999, i32::MIN, i32::MAX];
    for &v in values {
        let expected = format!("{v}\n").into_bytes();
        assert_same_and_eq(&format!("E8 pseudo-enum {v}"), &expected, |api| {
            api.print_int_line(v)
        });
    }
}

// ---------------------------------------------------------------------- E9
/// The CWE-482 defect in `bad()` is behaviour, not a bug to fix.
#[test]
fn err_e9_bad_defect_is_preserved() {
    let c = capture_stdout(|| c_api().bad());
    let r = capture_stdout(|| rust_api().bad());
    assert_eq!(c, r, "bad() diverges");
    assert_eq!(&c, b"0\n0\n", "C bad() must print the defective 0/0");
    assert_eq!(&r, b"0\n0\n", "Rust bad() must reproduce the defective 0/0");
    // And good() must NOT be defective.
    let cg = capture_stdout(|| c_api().good());
    let rg = capture_stdout(|| rust_api().good());
    assert_eq!(cg, rg, "good() diverges");
    assert_eq!(&cg, b"0\n2\n");
}

// ---------------------------------------------------------------------- E10
/// All five functions are stateless; output depends only on the arguments.
#[test]
fn err_e10_stateless_across_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 0xE10);
    // Same call sequence executed three times must yield three identical blocks.
    let bytes: Vec<u8> = (0..17).map(|_| 1 + rng.below(255) as u8).collect();
    let buf = CBuf::new(&bytes);
    let v = rng.next_i32();

    let block = capture_stdout(|| {
        c_api().print_line(buf.as_ptr());
        c_api().print_int_line(v);
        c_api().good();
        c_api().bad();
        c_api().driver();
    });
    for i in 0..3 {
        let again = capture_stdout(|| {
            c_api().print_line(buf.as_ptr());
            c_api().print_int_line(v);
            c_api().good();
            c_api().bad();
            c_api().driver();
        });
        assert_eq!(block, again, "C is not stateless (iteration {i})");
        let rblock = capture_stdout(|| {
            rust_api().print_line(buf.as_ptr());
            rust_api().print_int_line(v);
            rust_api().good();
            rust_api().bad();
            rust_api().driver();
        });
        assert_eq!(block, rblock, "Rust diverges on iteration {i}");
    }
}
