//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md` (E1–E9) plus the generic FFI-boundary rows
//! (G1–G7). Both implementations are loaded from their `.so` and driven only
//! through their exported C symbols.
//!
//! Every function here returns `void`, so "same error" means: the same
//! rejection *behaviour* — identical bytes emitted (typically none) and a
//! normal return instead of a crash/abort/panic. A Rust panic across the FFI
//! boundary would abort the test process, so surviving the call is itself part
//! of the assertion.

mod common;

use common::{assert_same, capture, cstr, pair};
use std::ffi::c_char;

// ---------------------------------------------------------------- E1 / G1
fn e1_print_line_null_pointer_writes_nothing() {
    let p = pair();
    let c_out = capture(|| unsafe { p.c.print_line_raw(std::ptr::null()) });
    let rs_out = capture(|| unsafe { p.rust.print_line_raw(std::ptr::null()) });

    // The C guard `if (line != NULL)` suppresses all output.
    assert_eq!(c_out, b"", "C: printLine(NULL) must emit nothing");
    assert_eq!(rs_out, b"", "Rust: printLine(NULL) must emit nothing");
    assert_eq!(c_out, rs_out);

    // Repeated NULL calls stay silent, and a valid call afterwards still works,
    // proving the rejection has no lingering effect in either implementation.
    assert_same("printLine(NULL) x100 then a valid line", |imp| unsafe {
        for _ in 0..100 {
            imp.print_line_raw(std::ptr::null());
        }
        let s = cstr(b"after nulls");
        imp.print_line_raw(s.as_ptr());
    });
}

// ---------------------------------------------------------------- E2 / G2
fn e2_print_line_empty_string_is_not_rejected() {
    let p = pair();
    let s = cstr(b"");
    let c_out = capture(|| unsafe { p.c.print_line_raw(s.as_ptr()) });
    let rs_out = capture(|| unsafe { p.rust.print_line_raw(s.as_ptr()) });
    // Non-NULL, so the guard passes: exactly one newline (NOT zero bytes).
    assert_eq!(c_out, b"\n");
    assert_eq!(rs_out, b"\n");
    assert_ne!(
        c_out,
        Vec::<u8>::new(),
        "empty string must not be treated like NULL"
    );
}

// ---------------------------------------------------------------- E3
fn e3_format_directives_are_not_interpreted() {
    let p = pair();
    // `%n` is the dangerous one: if either implementation ever passed `line` as
    // the *format*, this would write through a bogus pointer / abort.
    for payload in [
        &b"%n"[..],
        b"%s%s%s%s%s%s%s%s",
        b"%d %d %d %d",
        b"%.2000d",
        b"%99999999s",
        b"%p%p%p",
        b"%%",
        b"%1$n",
        b"%hhn%hn%n%lln",
        b"AAAA%08x.%08x.%08x.%08x.%08x.%n",
    ] {
        let s = cstr(payload);
        let c_out = capture(|| unsafe { p.c.print_line_raw(s.as_ptr()) });
        let rs_out = capture(|| unsafe { p.rust.print_line_raw(s.as_ptr()) });
        let mut expected = payload.to_vec();
        expected.push(b'\n');
        assert_eq!(
            c_out,
            expected,
            "C must print {} literally",
            common::escape(payload)
        );
        assert_eq!(
            rs_out, expected,
            "Rust must print {} literally",
            common::escape(payload)
        );
    }
}

// ---------------------------------------------------------------- E4 / G3
fn e4_no_length_limit_oversized_inputs() {
    let p = pair();
    for len in [64 * 1024usize, 256 * 1024, 1024 * 1024] {
        let payload = vec![b'Z'; len];
        let s = cstr(&payload);
        let c_out = capture(|| unsafe { p.c.print_line_raw(s.as_ptr()) });
        let rs_out = capture(|| unsafe { p.rust.print_line_raw(s.as_ptr()) });
        assert_eq!(c_out.len(), len + 1, "C must not truncate at len {len}");
        assert_eq!(c_out, rs_out, "mismatch at len {len}");
    }
}

// ---------------------------------------------------------------- E5 / G4
fn e5_g4_non_ascii_and_control_bytes_pass_through() {
    let p = pair();

    // Exhaustive single-byte sweep over the whole expressible alphabet
    // (0x00 terminates a C string, so 0x01..=0xFF is "one step past" on both
    // ends of what the API can receive).
    for b in 1u8..=255 {
        let s = cstr(&[b]);
        let c_out = capture(|| unsafe { p.c.print_line_raw(s.as_ptr()) });
        let rs_out = capture(|| unsafe { p.rust.print_line_raw(s.as_ptr()) });
        assert_eq!(c_out, vec![b, b'\n'], "C passthrough of byte 0x{b:02x}");
        assert_eq!(rs_out, vec![b, b'\n'], "Rust passthrough of byte 0x{b:02x}");
    }

    // Deliberately invalid UTF-8 sequences (truncated multi-byte, lone
    // continuation bytes, surrogate/overlong encodings) — no validation in C,
    // so Rust must not validate either.
    for payload in [
        &b"\xff\xfe\xfd"[..],
        b"\x80\x81\x82",
        b"\xc3",              // truncated 2-byte
        b"\xe2\x82",          // truncated 3-byte
        b"\xf0\x9f\x92",      // truncated 4-byte
        b"\xed\xa0\x80",      // UTF-16 surrogate, invalid UTF-8
        b"\xc0\xaf",          // overlong '/'
        b"\xf5\x80\x80\x80",  // > U+10FFFF
        b"caf\xe9",           // Latin-1 'é'
    ] {
        let s = cstr(payload);
        let mut expected = payload.to_vec();
        expected.push(b'\n');
        let c_out = capture(|| unsafe { p.c.print_line_raw(s.as_ptr()) });
        let rs_out = capture(|| unsafe { p.rust.print_line_raw(s.as_ptr()) });
        assert_eq!(c_out, expected, "C: invalid UTF-8 must pass through");
        assert_eq!(rs_out, expected, "Rust: invalid UTF-8 must pass through");
    }
}

// ---------------------------------------------------------------- E6 / G7
fn e6_g7_buffered_stdout_ordering_under_rejections() {
    // Mix of accepted and rejected inputs; ordering and flushing must match.
    let payloads: Vec<Option<Vec<c_char>>> = vec![
        None,
        Some(cstr(b"one")),
        None,
        None,
        Some(cstr(b"")),
        Some(cstr(b"two")),
        None,
        Some(cstr(b"three")),
    ];
    assert_same("accepted/rejected interleaving", |imp| unsafe {
        for pl in &payloads {
            match pl {
                None => imp.print_line_raw(std::ptr::null()),
                Some(s) => imp.print_line_raw(s.as_ptr()),
            }
        }
    });
}

// ---------------------------------------------------------------- E7
fn e7_bad_never_reaches_dead_static_helper() {
    let p = pair();
    for imp in [&p.c, &p.rust] {
        let out = capture(|| unsafe { imp.bad() });
        assert_eq!(out, b"bad()\n", "{}: bad() output", imp.name);
        assert!(
            !out.windows(11).any(|w| w == b"helperBad()"),
            "{}: helperBad() must remain dead code",
            imp.name
        );
    }
}

// ---------------------------------------------------------------- E8
fn e8_good_exact_output() {
    let p = pair();
    for imp in [&p.c, &p.rust] {
        let out = capture(|| unsafe { imp.good() });
        assert_eq!(out, b"good()\nhelperGood()\n", "{}: good() output", imp.name);
    }
}

// ---------------------------------------------------------------- E9
fn e9_driver_exact_output_via_void_prototype() {
    let p = pair();
    let expected: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\n\
Calling bad()...\nbad()\nFinished bad()\n";
    for imp in [&p.c, &p.rust] {
        let out = capture(|| unsafe { imp.driver() });
        assert_eq!(out, expected, "{}: driver() output", imp.name);
    }
}

// ---------------------------------------------------------------- G5
fn g5_no_enum_or_integer_parameters_exist() {
    // Documented non-applicability, asserted mechanically so it cannot silently
    // become wrong: the only parameter anywhere in the public header is the
    // `const char *` of printLine, so there is no enum/int to pass an
    // out-of-range value for.
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/include/driver.h"
    ))
    .expect("read driver.h");
    assert!(
        !header.contains("enum"),
        "the public header grew an enum — Phase C needs out-of-range enum tests"
    );
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/src/driver.c"))
        .expect("read driver.c");
    assert!(
        !src.contains("enum"),
        "driver.c grew an enum — Phase C needs out-of-range enum tests"
    );
    // Exported prototypes take either no args or a single `const char *`.
    assert!(src.contains("void printLine(const char *line)"));
    assert!(src.contains("void bad()"));
    assert!(src.contains("void good()"));
    assert!(src.contains("void driver()"));
}

// ---------------------------------------------------------------- G6
fn g6_unaligned_and_interior_pointers() {
    let backing = cstr(b"0123456789abcdefghijklmnopqrstuvwxyz");
    for off in 0..36usize {
        let ptr = unsafe { backing.as_ptr().add(off) };
        assert_same(&format!("printLine(offset {off})"), |imp| unsafe {
            imp.print_line_raw(ptr)
        });
    }
    // Pointer exactly at the NUL terminator: valid, empty payload.
    let end = unsafe { backing.as_ptr().add(36) };
    let p = pair();
    let c_out = capture(|| unsafe { p.c.print_line_raw(end) });
    assert_eq!(c_out, b"\n");
    assert_eq!(c_out, capture(|| unsafe { p.rust.print_line_raw(end) }));
}

// ---------------------------------------------------------------- extra
fn extra_null_then_every_entry_point_still_works() {
    // A rejected call must not poison the rest of the API surface.
    assert_same("NULL then all entry points", |imp| unsafe {
        imp.print_line_raw(std::ptr::null());
        imp.good();
        imp.print_line_raw(std::ptr::null());
        imp.bad();
        imp.print_line_raw(std::ptr::null());
        imp.driver();
    });
}

fn main() {
    let mut r = common::Runner::new("error_paths (Phase C / ERRORS.md)");
    r.case("e1_print_line_null_pointer_writes_nothing", e1_print_line_null_pointer_writes_nothing);
    r.case("e2_print_line_empty_string_is_not_rejected", e2_print_line_empty_string_is_not_rejected);
    r.case("e3_format_directives_are_not_interpreted", e3_format_directives_are_not_interpreted);
    r.case("e4_no_length_limit_oversized_inputs", e4_no_length_limit_oversized_inputs);
    r.case("e5_g4_non_ascii_and_control_bytes_pass_through", e5_g4_non_ascii_and_control_bytes_pass_through);
    r.case("e6_g7_buffered_stdout_ordering_under_rejections", e6_g7_buffered_stdout_ordering_under_rejections);
    r.case("e7_bad_never_reaches_dead_static_helper", e7_bad_never_reaches_dead_static_helper);
    r.case("e8_good_exact_output", e8_good_exact_output);
    r.case("e9_driver_exact_output_via_void_prototype", e9_driver_exact_output_via_void_prototype);
    r.case("g5_no_enum_or_integer_parameters_exist", g5_no_enum_or_integer_parameters_exist);
    r.case("g6_unaligned_and_interior_pointers", g6_unaligned_and_interior_pointers);
    r.case("extra_null_then_every_entry_point_still_works", extra_null_then_every_entry_point_still_works);
    r.finish();
}
