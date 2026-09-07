//! Phase C — error-path / rejection differential tests, one per `ERRORS.md`
//! row, plus the generic C-API boundary cases.

mod common;

use common::*;
use std::ffi::{CString, c_int};

// ---------------------------------------------------------------------------
// Row 1 — the NULL sentinel from strchr: needle absent -> loop exits, 0
// ---------------------------------------------------------------------------

#[test]
fn err_row1_no_occurrence_returns_zero() {
    for input in [
        &b"abc"[..],
        &b"zzzzzzzzzzzzzzzz"[..],
        &b"\x01\x02\x03\xfe\xff"[..],
        &b"the quick brown fox"[..],
    ] {
        let v = assert_foo_eq("err1", input, b'Q');
        assert_eq!(v, 0, "needle absent must return the 0 sentinel-count");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — empty string, zero-length input
// ---------------------------------------------------------------------------

#[test]
fn err_row2_empty_string_returns_zero() {
    for needle in [1u8, b'A', b'x', 0x7f, 0x80, 0xff] {
        let v = assert_foo_eq("err2", b"", needle);
        assert_eq!(v, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — needle '\0'. `strchr(s, 0)` always finds a terminator, so it never
// returns the NULL sentinel; the loop is unbounded and `s++` walks off the end
// of the object until the process faults. Both libraries must fault the same
// way. Run out-of-process, with an alarm so a divergence cannot hang the suite.
// ---------------------------------------------------------------------------

#[test]
fn err_row3_nul_needle_matches_terminator() {
    let p = pair();
    // Zero-padded arena: the bytes immediately after the string are known, so
    // the start of the out-of-bounds walk is identical for both libraries.
    let mut arena = vec![0u8; 4096];
    arena[..5].copy_from_slice(b"hello");

    let mut outs = Vec::new();
    for imp in [&p.c, &p.rust] {
        let f = imp.foo();
        let ptr = arena.as_ptr() as *const std::ffi::c_char;
        outs.push(run_in_child(
            || {
                let _ = unsafe { f(ptr, 0) };
            },
            30,
            None,
        ));
    }
    assert_eq!(
        outs[0], outs[1],
        "[err3] divergence on NUL needle: C {:?} vs Rust {:?}",
        outs[0], outs[1]
    );
    assert!(
        matches!(outs[0], Outcome::Signalled(SIGSEGV) | Outcome::Signalled(SIGBUS)),
        "[err3] expected a fatal memory fault from the unbounded walk, got {:?}",
        outs[0]
    );
}

// ---------------------------------------------------------------------------
// Row 4 — NULL `in` passed to `foo`
// ---------------------------------------------------------------------------

#[test]
fn err_row4_null_in_foo_segv() {
    let p = pair();
    let mut outs = Vec::new();
    for imp in [&p.c, &p.rust] {
        let f = imp.foo();
        outs.push(run_in_child(
            || {
                let _ = unsafe { f(std::ptr::null(), b'A' as i8) };
            },
            20,
            None,
        ));
    }
    assert_eq!(
        outs[0], outs[1],
        "[err4] divergence on NULL input to foo: C {:?} vs Rust {:?}",
        outs[0], outs[1]
    );
    assert_eq!(
        outs[0],
        Outcome::Signalled(SIGSEGV),
        "[err4] expected SIGSEGV, got {:?}",
        outs[0]
    );
}

// ---------------------------------------------------------------------------
// Row 5 — NULL `in` passed to `driver`: faults inside the first foo call, so
// nothing is printed. Compare both the signal and the (empty) stdout.
// ---------------------------------------------------------------------------

#[test]
fn err_row5_null_in_driver_segv() {
    let p = pair();
    let mut results = Vec::new();
    for imp in [&p.c, &p.rust] {
        let d = imp.driver();
        let path = std::env::temp_dir().join(format!(
            "driver-err5-{}-{}",
            std::process::id(),
            imp.name
        ));
        let out = run_in_child(
            || {
                unsafe { d(std::ptr::null()) };
            },
            20,
            Some(&path),
        );
        let stdout = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        results.push((out, stdout));
    }
    assert_eq!(
        results[0], results[1],
        "[err5] divergence on NULL input to driver: C {:?} vs Rust {:?}",
        results[0], results[1]
    );
    assert_eq!(
        results[0].0,
        Outcome::Signalled(SIGSEGV),
        "[err5] expected SIGSEGV, got {:?}",
        results[0].0
    );
    assert!(
        results[0].1.is_empty(),
        "[err5] expected no output before the fault, got {:?}",
        String::from_utf8_lossy(&results[0].1)
    );
}

// ---------------------------------------------------------------------------
// Row 6 — high-bit needle: the "one past the valid ASCII range" value class,
// negative when interpreted as a signed char. Exhaustive over 0x80..=0xFF.
// ---------------------------------------------------------------------------

#[test]
fn err_row6_high_bit_needle() {
    let mut input: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    input.extend((0x80u16..=0xFF).map(|b| b as u8)); // duplicates of the high half
    for needle in 0x80u16..=0xFF {
        let v = assert_foo_eq("err6", &input, needle as u8);
        let expect = input.iter().filter(|&&b| b == needle as u8).count() as i32;
        assert_eq!(v, expect, "needle {needle:#04x}");
    }
    // Also the 0x7f/0x80 boundary pair against an input holding just one of them.
    for needle in [0x7fu8, 0x80u8] {
        assert_foo_eq("err6", &[0x7f], needle);
        assert_foo_eq("err6", &[0x80], needle);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — a full-width `int` needle with garbage above bit 7 pushed across the
// FFI boundary. This is the analogue of an out-of-range enum value: the ABI
// lets any int through, and the `char` parameter must observe only the low 8
// bits. Both libraries must agree, including on the truncation.
// ---------------------------------------------------------------------------

#[test]
fn err_row7_wide_int_needle_truncates() {
    let input: &[u8] = b"AAxxA-zZ\x01\xfe\xff\x80\x7f";
    let wide: &[c_int] = &[
        0x0000_0041,        // plain 'A'
        0x1122_3341,        // 'A' with garbage above bit 7
        -0x7fff_ffbf,       // 0x80000041 as i32, low byte 0x41
        0x0000_0141,        // low byte 0x41, bit 8 set
        0x0000_1278,        // low byte 0x78 == 'x'
        0x0000_00ff,        // 0xff
        0x7fff_ffff,        // low byte 0xff
        -1,                 // all bits set, low byte 0xff
        0x0000_0100,        // low byte 0x00 -> NUL needle: skipped below
        c_int::MIN,         // low byte 0x00 -> skipped below
        0x0000_0080,        // 0x80 boundary
        0x0000_007f,        // 0x7f boundary
    ];
    for &w in wide {
        // A low byte of zero means the NUL needle, whose behaviour is the
        // unbounded walk covered by row 3; exclude it from the in-process test.
        if (w & 0xff) == 0 {
            continue;
        }
        let v = assert_foo_wide_eq("err7", input, w);
        let expect = input.iter().filter(|&&b| b == (w & 0xff) as u8).count() as i32;
        assert_eq!(
            v, expect,
            "wide needle {w:#010x} must behave as the low byte {:#04x}",
            (w & 0xff) as u8
        );
    }

    // And the low-byte-zero cases really do behave like the NUL needle in BOTH
    // libraries (unbounded walk -> fatal fault), not like "no match".
    let p = pair();
    let s = CString::new(input).unwrap();
    for &w in &[0x0000_0100 as c_int, c_int::MIN] {
        let mut outs = Vec::new();
        for imp in [&p.c, &p.rust] {
            let f = imp.foo_wide();
            let ptr = s.as_ptr();
            outs.push(run_in_child(
                || {
                    let _ = unsafe { f(ptr, w) };
                },
                30,
                None,
            ));
        }
        assert_eq!(
            outs[0], outs[1],
            "[err7] divergence for wide needle {w:#010x}: C {:?} vs Rust {:?}",
            outs[0], outs[1]
        );
        assert!(
            matches!(outs[0], Outcome::Signalled(SIGSEGV) | Outcome::Signalled(SIGBUS)),
            "[err7] wide needle {w:#010x} should truncate to NUL and fault, got {:?}",
            outs[0]
        );
    }
}

// ---------------------------------------------------------------------------
// Row 8 — oversized input
// ---------------------------------------------------------------------------

#[test]
fn err_row8_oversized_input() {
    const N: usize = 1 << 20;
    let needle = b'A';
    // Every byte a match: maximum number of loop iterations for this size.
    let all = vec![needle; N];
    let v = assert_foo_eq("err8", &all, needle);
    assert_eq!(v, N as i32);

    // No match at all: single strchr scan over the whole buffer.
    let none = vec![b'.'; N];
    let v = assert_foo_eq("err8", &none, needle);
    assert_eq!(v, 0);

    // Match only at the very last byte of a large buffer.
    let mut last = vec![b'.'; N - 1];
    last.push(needle);
    let v = assert_foo_eq("err8", &last, needle);
    assert_eq!(v, 1);
}

// ---------------------------------------------------------------------------
// Row 9 — match on the final byte, so `s++` lands exactly on the terminator
// and the next strchr must stop there rather than run past it.
// ---------------------------------------------------------------------------

#[test]
fn err_row9_match_at_last_byte() {
    for needle in [1u8, b'A', b'x', 0x7f, 0x80, 0xfe, 0xff] {
        // needle as the only byte
        assert_eq!(assert_foo_eq("err9", &[needle], needle), 1);
        // needle as the final byte after filler
        let mut buf = vec![b'.'; 17];
        buf.push(needle);
        assert_eq!(assert_foo_eq("err9", &buf, needle), 1);
        // a run of the needle ending at the final byte
        let mut buf = vec![b'.'; 5];
        buf.extend(std::iter::repeat_n(needle, 9));
        assert_eq!(assert_foo_eq("err9", &buf, needle), 9);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary sweep, beyond the table: every needle value 1..=255 against
// a set of adversarial shapes, cross-checked between the two libraries.
// ---------------------------------------------------------------------------

#[test]
fn err_generic_boundary_sweep() {
    let shapes: Vec<Vec<u8>> = vec![
        vec![],
        vec![0x01],
        vec![0xff],
        vec![0x7f, 0x80],
        (1u16..=255).map(|b| b as u8).collect(),
        (1u16..=255).rev().map(|b| b as u8).collect(),
        vec![0xff; 64],
        b"\x01\x01\x01\xff\xff\xff\x80\x80\x80".to_vec(),
    ];
    for shape in &shapes {
        for needle in 1u16..=255 {
            let v = assert_foo_eq("err-generic", shape, needle as u8);
            let expect = shape.iter().filter(|&&b| b == needle as u8).count() as i32;
            assert_eq!(v, expect);
        }
    }
}
