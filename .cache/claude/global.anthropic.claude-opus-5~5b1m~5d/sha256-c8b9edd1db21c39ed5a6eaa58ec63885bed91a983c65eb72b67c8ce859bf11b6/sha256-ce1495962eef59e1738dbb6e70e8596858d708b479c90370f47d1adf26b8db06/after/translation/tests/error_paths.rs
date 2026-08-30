//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of ERRORS.md (E1..E11).
//!
//! The C library validates nothing, so several rows are inputs on which the C
//! code has undefined behaviour and dies. For those the test forks a child per
//! implementation and asserts BOTH children terminate with the SAME outcome
//! (same signal, or same exit code) — not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::CString;

const SEED: u64 = 0x0BAD_C0DE;

// ---------------------------------------------------------------------------
// E1 — foo(NULL, c): no null check in C -> fault
// ---------------------------------------------------------------------------
#[test]
fn err_e1_foo_null_pointer() {
    let l = libs();
    for needle in [b'A' as i8, b'x' as i8, 1i8, -1i8, 127i8] {
        let c_out = child_outcome(5, || unsafe {
            (l.c.foo)(std::ptr::null(), needle);
            0
        });
        let rs_out = child_outcome(5, || unsafe {
            (l.rs.foo)(std::ptr::null(), needle);
            0
        });
        assert_eq!(
            c_out, rs_out,
            "E1: foo(NULL, {needle}) must terminate identically (C={c_out:?} Rust={rs_out:?})"
        );
        assert_eq!(
            c_out,
            Outcome::Signaled(SIGSEGV),
            "E1: the C code is expected to fault on a null haystack"
        );
    }
}

// ---------------------------------------------------------------------------
// E2 — driver(NULL): faults inside the first foo() call, prints nothing
// ---------------------------------------------------------------------------
#[test]
fn err_e2_driver_null_pointer() {
    let l = libs();
    let c_out = child_outcome(5, || unsafe {
        (l.c.driver)(std::ptr::null());
        0
    });
    let rs_out = child_outcome(5, || unsafe {
        (l.rs.driver)(std::ptr::null());
        0
    });
    assert_eq!(
        c_out, rs_out,
        "E2: driver(NULL) must terminate identically (C={c_out:?} Rust={rs_out:?})"
    );
    assert_eq!(c_out, Outcome::Signaled(SIGSEGV), "E2: expected a fault");
}

// ---------------------------------------------------------------------------
// E3 — foo(s, 0): strchr matches the terminator, then s++ runs off the end;
//      strchr can never return NULL for c == 0, so the loop never terminates
//      normally. Must behave identically in both libraries.
// ---------------------------------------------------------------------------
#[test]
fn err_e3_foo_nul_needle() {
    let l = libs();
    // Keep the haystack identical for both children by allocating before fork.
    let cs = CString::new("hello Ax world").unwrap();
    let p = cs.as_ptr();

    let c_out = child_outcome(5, || unsafe {
        let n = (l.c.foo)(p, 0);
        // If it ever returns, report the count so a divergence is visible.
        1 + (n & 0x3f)
    });
    let rs_out = child_outcome(5, || unsafe {
        let n = (l.rs.foo)(p, 0);
        1 + (n & 0x3f)
    });
    eprintln!("E3 outcome: C={c_out:?} Rust={rs_out:?}");
    assert_eq!(
        c_out, rs_out,
        "E3: foo(s, 0) must behave identically (C={c_out:?} Rust={rs_out:?})"
    );
    // Whatever it is, it must be a hard termination, not a quiet return.
    assert!(
        matches!(
            c_out,
            Outcome::Signaled(SIGSEGV) | Outcome::Signaled(SIGBUS) | Outcome::Signaled(SIGALRM)
        ),
        "E3: expected the C code to run off the end of the object, got {c_out:?}"
    );
}

// ---------------------------------------------------------------------------
// E4 — unterminated buffer: reads past the end; both must agree.
//      Both children are forked from the same parent, so the memory after the
//      buffer is byte-identical for each; the counts are compared via the
//      child exit status.
// ---------------------------------------------------------------------------
#[test]
fn err_e4_unterminated_buffer() {
    let l = libs();
    // 4 KiB with NO terminating NUL anywhere.
    let mut rng = Rng::new(SEED ^ 4);
    let buf: Vec<u8> = (0..4096).map(|_| loop {
        let b = rng.nonzero_byte();
        if b != b'A' {
            return b;
        }
    }).collect();
    let mut buf = buf;
    for i in (0..buf.len()).step_by(7) {
        buf[i] = b'A';
    }
    let p = buf.as_ptr() as *const std::os::raw::c_char;

    let c_out = child_outcome(5, || unsafe { (l.c.foo)(p, b'A' as i8) & 0x7f });
    let rs_out = child_outcome(5, || unsafe { (l.rs.foo)(p, b'A' as i8) & 0x7f });
    eprintln!("E4 outcome: C={c_out:?} Rust={rs_out:?}");
    assert_eq!(
        c_out, rs_out,
        "E4: unterminated-buffer scan must agree (C={c_out:?} Rust={rs_out:?})"
    );
    // Neither implementation may add a bounds check that the C code lacks:
    // whatever the C does (return a count, or fault), Rust must do the same.
}

// ---------------------------------------------------------------------------
// E5 — high-bit (negative) needle must NOT be treated as "not found"
// ---------------------------------------------------------------------------
#[test]
fn err_e5_negative_needle() {
    let mut rng = Rng::new(SEED ^ 5);
    for needle_b in [0x80u8, 0x81, 0xA5, 0xFE, 0xFF] {
        let needle = needle_b as i8;
        assert!(needle < 0);
        for i in 0..40 {
            let len = rng.range(1, 128);
            let hay: Vec<u8> = (0..len)
                .map(|_| if rng.below(3) == 0 { needle_b } else { rng.nonzero_byte() })
                .collect();
            let r = diff_foo(&hay, needle, &format!("E5 0x{needle_b:02x} iter {i}"));
            assert_eq!(
                r,
                expected_count(&hay, needle),
                "E5: byte 0x{needle_b:02x} must be matched, not rejected"
            );
        }
        // A haystack that is entirely the needle: must be the full length.
        let hay = vec![needle_b; 17];
        assert_eq!(diff_foo(&hay, needle, "E5 all"), 17);
    }
}

// ---------------------------------------------------------------------------
// E6 — control bytes just outside the printable ASCII range
// ---------------------------------------------------------------------------
#[test]
fn err_e6_control_byte_needles() {
    let mut rng = Rng::new(SEED ^ 6);
    for needle_b in [0x01u8, 0x02, 0x09, 0x0a, 0x0d, 0x1f, 0x7f] {
        let needle = needle_b as i8;
        for i in 0..40 {
            let len = rng.range(1, 96);
            let hay: Vec<u8> = (0..len)
                .map(|_| if rng.below(4) == 0 { needle_b } else { rng.ascii_byte() })
                .collect();
            let r = diff_foo(&hay, needle, &format!("E6 0x{needle_b:02x} iter {i}"));
            assert_eq!(r, expected_count(&hay, needle));
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — empty string / zero length
// ---------------------------------------------------------------------------
#[test]
fn err_e7_empty_string() {
    for n in 1u16..=255 {
        let r = diff_foo(b"", n as u8 as i8, "E7");
        assert_eq!(r, 0, "E7: empty haystack, needle 0x{n:02x}");
    }
    let out = diff_driver(b"", "E7 driver");
    assert_eq!(out, b"A: 0\nx: 0\n");
}

// ---------------------------------------------------------------------------
// E8 — "oversized" input: far larger than any plausible internal buffer
// ---------------------------------------------------------------------------
#[test]
fn err_e8_oversized_input() {
    let len = 256 * 1024usize;
    let hay = vec![b'A'; len];
    let r = diff_foo(&hay, b'A' as i8, "E8 all-match");
    assert_eq!(r, len as i32, "E8: no truncation for a {len}-byte input");

    let mut rng = Rng::new(SEED ^ 8);
    let hay: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
    for needle_b in [b'A', b'x', 0xffu8] {
        let needle = needle_b as i8;
        assert_eq!(
            diff_foo(&hay, needle, "E8 random"),
            expected_count(&hay, needle)
        );
    }

    let out = diff_driver(&hay, "E8 driver");
    let na = hay.iter().filter(|&&b| b == b'A').count();
    let nx = hay.iter().filter(|&&b| b == b'x').count();
    assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
}

// ---------------------------------------------------------------------------
// E9 — driver with non-UTF-8 bytes (a `to_str()`-based port would error here)
// ---------------------------------------------------------------------------
#[test]
fn err_e9_driver_non_utf8() {
    for hay in [
        &b"\xff"[..],
        &b"\x80\x80\x80"[..],
        &b"A\xc3x"[..],          // truncated UTF-8 sequence
        &b"\xed\xa0\x80"[..],    // encoded surrogate
        &b"\xf4\x90\x80\x80"[..],// above U+10FFFF
        &b"Ax\xfe\xff\xfe\xffAx"[..],
    ] {
        assert!(std::str::from_utf8(hay).is_err(), "sanity: {:?} is invalid UTF-8", preview(hay));
        let out = diff_driver(hay, "E9");
        let na = hay.iter().filter(|&&b| b == b'A').count();
        let nx = hay.iter().filter(|&&b| b == b'x').count();
        assert_eq!(
            out,
            format!("A: {na}\nx: {nx}\n").into_bytes(),
            "E9: invalid UTF-8 must be processed byte-wise, not rejected"
        );
    }
}

// ---------------------------------------------------------------------------
// E10 — printf format specifiers in the *data* must never be interpreted
// ---------------------------------------------------------------------------
#[test]
fn err_e10_driver_format_specifiers() {
    for hay in [
        &b"%s"[..],
        &b"%d %d %d %d"[..],
        &b"%n"[..],
        &b"%p%p%p%p%p%p%p%p"[..],
        &b"A%sx%nA"[..],
        &b"%%%%"[..],
        &b"100%"[..],
        &b"%1000000d"[..],
    ] {
        let out = diff_driver(hay, "E10");
        let na = hay.iter().filter(|&&b| b == b'A').count();
        let nx = hay.iter().filter(|&&b| b == b'x').count();
        assert_eq!(
            out,
            format!("A: {na}\nx: {nx}\n").into_bytes(),
            "E10: input {:?} must not be used as a format string",
            preview(hay)
        );
    }
}

// ---------------------------------------------------------------------------
// E11 — every representable value of the `char` parameter crossing FFI
//       (the "no valid variant" / out-of-range-enum analogue), except 0 (E3).
// ---------------------------------------------------------------------------
#[test]
fn err_e11_full_needle_domain() {
    let mut rng = Rng::new(SEED ^ 11);
    let haystacks: Vec<Vec<u8>> = (0..5)
        .map(|_| {
            let len = rng.range(1, 256);
            (0..len).map(|_| rng.nonzero_byte()).collect()
        })
        .chain(std::iter::once(Vec::new()))
        .chain(std::iter::once((1u8..=255).collect::<Vec<u8>>()))
        .collect();

    for v in i16::from(i8::MIN)..=i16::from(i8::MAX) {
        let needle = v as i8;
        if needle == 0 {
            continue; // UB, covered by E3
        }
        for hay in &haystacks {
            let r = diff_foo(hay, needle, &format!("E11 needle {needle}"));
            assert_eq!(
                r,
                expected_count(hay, needle),
                "E11: needle {needle} (0x{:02x})",
                needle as u8
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundary: repeated / interleaved calls must not carry state.
// ---------------------------------------------------------------------------
#[test]
fn err_e12_no_hidden_state_between_calls() {
    let l = libs();
    let a = CString::new("AAAxxx").unwrap();
    let b = CString::new("no needles here").unwrap();
    for _ in 0..500 {
        unsafe {
            assert_eq!((l.c.foo)(a.as_ptr(), b'A' as i8), (l.rs.foo)(a.as_ptr(), b'A' as i8));
            assert_eq!((l.c.foo)(b.as_ptr(), b'A' as i8), (l.rs.foo)(b.as_ptr(), b'A' as i8));
            assert_eq!((l.c.foo)(a.as_ptr(), b'x' as i8), (l.rs.foo)(a.as_ptr(), b'x' as i8));
        }
    }
    // driver called twice in a row: output must be identical each time.
    let first = diff_driver(b"AAAxxx", "E12 first");
    let second = diff_driver(b"AAAxxx", "E12 second");
    assert_eq!(first, second);
    assert_eq!(first, b"A: 3\nx: 3\n");
}
