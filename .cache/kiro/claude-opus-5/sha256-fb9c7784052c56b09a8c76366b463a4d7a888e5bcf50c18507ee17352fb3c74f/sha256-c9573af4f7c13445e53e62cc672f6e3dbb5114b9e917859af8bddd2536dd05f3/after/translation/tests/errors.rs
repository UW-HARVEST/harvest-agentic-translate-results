//! Phase C — error/rejection-path differential tests, one per `ERRORS.md` row.
//!
//! `driver` has no return value and no explicit error surface, so "same error"
//! here means: for the exact invalid input or edge condition of the row, both
//! `.so`s must reject (or, as is the case throughout, *not* reject) in the same
//! way and emit the same bytes. Where a row's ground truth is a specific
//! sentinel — glibc's `EOF` table slot, the `%c` rendering of a negative int,
//! the raw ctype mask rather than a normalised 0/1 — the test pins that exact
//! value against the C, not merely "both did something".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Pull one `label: value` field out of a capture, as an integer.
fn field(bytes: &[u8], label: &str) -> i64 {
    let text = String::from_utf8_lossy(bytes).to_string();
    let prefix = format!("{label}: ");
    let line = text
        .lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no {label:?} line in {:?}", render(bytes)));
    line[prefix.len()..]
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("bad {label:?} value in {line:?}: {e}"))
}

/// The single byte `printf("%c")` produced for the given label.
fn char_field(bytes: &[u8], label: &str) -> u8 {
    let prefix = format!("{label}: ").into_bytes();
    let at = bytes
        .windows(prefix.len())
        .position(|w| w == prefix.as_slice())
        .unwrap_or_else(|| panic!("no {label:?} line in {:?}", render(bytes)));
    bytes[at + prefix.len()]
}

fn c_out(c: c_char) -> Vec<u8> {
    let cd = c_driver();
    capture(|| unsafe { cd(c) })
}

// ---------------------------------------------------------------------------
// Row 1 — c == 0 (NUL / empty-string sentinel)
// ---------------------------------------------------------------------------

#[test]
fn err_01_nul_byte() {
    reset_global_locale();
    diff_char(0, "ERRORS row 1: NUL");

    // Pin the ground truth read off the C, so a later "normalising" change to
    // the Rust cannot pass by matching a wrong expectation.
    let out = c_out(0);
    assert_eq!(field(&out, "control"), 2, "row 1: _IScntrl raw mask");
    for label in [
        "alphanumeric",
        "alphabetic",
        "lowercase",
        "uppercase",
        "digit",
        "hexadecimal",
        "graphical",
        "space",
        "blank",
        "printing",
        "punctuation",
    ] {
        assert_eq!(field(&out, label), 0, "row 1: {label}");
    }
    assert_eq!(char_field(&out, "to lower"), 0, "row 1: %c of 0 is a NUL byte");
    assert_eq!(char_field(&out, "to upper"), 0, "row 1: %c of 0 is a NUL byte");
}

// ---------------------------------------------------------------------------
// Row 2 — c == -1, which is also EOF and has its own table slot
// ---------------------------------------------------------------------------

#[test]
fn err_02_minus_one_eof_slot() {
    reset_global_locale();
    diff_char(-1, "ERRORS row 2: (char)0xFF == -1 == EOF slot");

    let out = c_out(-1);
    for label in [
        "alphanumeric",
        "alphabetic",
        "lowercase",
        "uppercase",
        "digit",
        "hexadecimal",
        "control",
        "graphical",
        "space",
        "blank",
        "printing",
        "punctuation",
    ] {
        assert_eq!(field(&out, label), 0, "row 2: {label} must be 0 for EOF");
    }
    // glibc's tolower/toupper return -1 for the EOF slot; printf("%c", -1)
    // narrows to the byte 0xFF.
    assert_eq!(char_field(&out, "to lower"), 0xFF, "row 2: %c of -1");
    assert_eq!(char_field(&out, "to upper"), 0xFF, "row 2: %c of -1");
}

// ---------------------------------------------------------------------------
// Row 3 — c == -128, the lowest legal ctype-table index
// ---------------------------------------------------------------------------

#[test]
fn err_03_most_negative_char() {
    reset_global_locale();
    diff_char(-128, "ERRORS row 3: (char)0x80 == -128");

    let out = c_out(-128);
    assert_eq!(char_field(&out, "to lower"), 0x80, "row 3: %c of 128");
    assert_eq!(char_field(&out, "to upper"), 0x80, "row 3: %c of 128");
    assert_eq!(field(&out, "control"), 0, "row 3: not cntrl in the C locale");
}

// ---------------------------------------------------------------------------
// Row 4 — every negative char index, exhaustively
// ---------------------------------------------------------------------------

#[test]
fn err_04_all_negative_chars() {
    reset_global_locale();
    for v in 0x80u16..=0xFF {
        let c = v as u8 as c_char;
        assert!(c < 0, "0x{v:02x} must be a negative char on this target");
        diff_char(c, &format!("ERRORS row 4: negative index 0x{v:02x}"));
    }
    // All twelve classifiers are 0 across the whole negative range in "C".
    for v in 0x80u16..=0xFF {
        let out = c_out(v as u8 as c_char);
        for label in [
            "alphanumeric",
            "alphabetic",
            "lowercase",
            "uppercase",
            "digit",
            "hexadecimal",
            "control",
            "graphical",
            "space",
            "blank",
            "printing",
            "punctuation",
        ] {
            assert_eq!(
                field(&out, label),
                0,
                "row 4: {label} for 0x{v:02x} in the C locale"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — c == 127 (DEL)
// ---------------------------------------------------------------------------

#[test]
fn err_05_del_127() {
    reset_global_locale();
    diff_char(127, "ERRORS row 5: DEL");

    let out = c_out(127);
    assert_eq!(field(&out, "control"), 2, "row 5: DEL is _IScntrl");
    assert_eq!(field(&out, "printing"), 0, "row 5: DEL is not printable");
    assert_eq!(char_field(&out, "to lower"), 0x7F);
    assert_eq!(char_field(&out, "to upper"), 0x7F);
}

// ---------------------------------------------------------------------------
// Rows 6, 7, 9 — wider-than-char values must narrow identically
// ---------------------------------------------------------------------------

#[test]
fn err_06_128_narrowing() {
    reset_global_locale();
    diff_int(128, "ERRORS row 6: int 128 is not representable in a char");
    // The C narrows 128 to -128, so it must agree with the char call.
    let cd = c_driver();
    let cdi = c_driver_int();
    let via_char = capture(|| unsafe { cd(-128) });
    let via_int = capture(|| unsafe { cdi(128) });
    assert_eq!(via_char, via_int, "row 6: C must narrow 128 to -128");
    let rd = rust_driver();
    let rdi = rust_driver_int();
    let r_char = capture(|| unsafe { rd(-128) });
    let r_int = capture(|| unsafe { rdi(128) });
    assert_eq!(r_char, r_int, "row 6: Rust must narrow 128 to -128");
    assert_eq!(via_int, r_int, "row 6: C/Rust divergence");
}

#[test]
fn err_07_256_one_past_uchar() {
    reset_global_locale();
    diff_int(256, "ERRORS row 7: int 256, one past the unsigned char range");
    let cdi = c_driver_int();
    let rdi = rust_driver_int();
    let c_256 = capture(|| unsafe { cdi(256) });
    let c_0 = capture(|| unsafe { cdi(0) });
    assert_eq!(c_256, c_0, "row 7: C must narrow 256 to 0");
    let r_256 = capture(|| unsafe { rdi(256) });
    let r_0 = capture(|| unsafe { rdi(0) });
    assert_eq!(r_256, r_0, "row 7: Rust must narrow 256 to 0");
    assert_eq!(c_256, r_256, "row 7: C/Rust divergence");
}

#[test]
fn err_09_negative_wide_int() {
    reset_global_locale();
    // 0xFFFF_0100: negative as an int, low byte 0x00.
    let v: c_int = -65280;
    assert_eq!((v as u32) & 0xFF, 0x00);
    diff_int(v, "ERRORS row 9: negative wide int with a positive low byte");
    let cdi = c_driver_int();
    let c_v = capture(|| unsafe { cdi(v) });
    let c_0 = capture(|| unsafe { cdi(0) });
    assert_eq!(c_v, c_0, "row 9: C must keep only the low byte");
}

// ---------------------------------------------------------------------------
// Row 8 — the FFI analogue of an out-of-range enum value
// ---------------------------------------------------------------------------

#[test]
fn err_08_garbage_high_bits() {
    reset_global_locale();
    // A C `enum` accepts any int, and the ABI leaves bits 8..31 of a `char`
    // argument unspecified; a caller with no prototype (or a hand-written FFI
    // binding) really can deliver these. The C keeps only the low byte.
    let cases: [(c_int, u8); 6] = [
        (0xDEAD_BE41u32 as c_int, 0x41),
        (0x7FFF_FF00u32 as c_int, 0x00),
        (0xFFFF_FF7Fu32 as c_int, 0x7F),
        (0x0BAD_C0DEu32 as c_int, 0xDE),
        (c_int::MIN, 0x00),
        (c_int::MAX, 0xFF),
    ];
    let cd = c_driver();
    let cdi = c_driver_int();
    let rd = rust_driver();
    let rdi = rust_driver_int();
    for (wide, low) in cases {
        diff_int(wide, &format!("ERRORS row 8: garbage high bits {wide:#010x}"));

        let want = capture(|| unsafe { cd(low as c_char) });
        let c_wide = capture(|| unsafe { cdi(wide) });
        assert_eq!(
            c_wide,
            want,
            "row 8: C({wide:#010x}) must equal C(char {low:#04x})\n  {}\n  {}",
            render(&c_wide),
            render(&want)
        );
        let r_want = capture(|| unsafe { rd(low as c_char) });
        let r_wide = capture(|| unsafe { rdi(wide) });
        assert_eq!(
            r_wide, r_want,
            "row 8: Rust({wide:#010x}) must equal Rust(char {low:#04x})"
        );
        assert_eq!(c_wide, r_wide, "row 8: C/Rust divergence at {wide:#010x}");
    }

    // Property-style: for 512 seeded-random 32-bit arguments, the wide call must
    // equal the char call on the low byte, for both libraries.
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..512 {
        let wide = rng.next_u32() as c_int;
        let low = (wide as u32 & 0xFF) as u8;
        let c_wide = capture(|| unsafe { cdi(wide) });
        let c_low = capture(|| unsafe { cd(low as c_char) });
        assert_eq!(c_wide, c_low, "row 8: C draw {i} ({wide:#010x})");
        let r_wide = capture(|| unsafe { rdi(wide) });
        assert_eq!(c_wide, r_wide, "row 8: divergence draw {i} ({wide:#010x})");
    }
}

// ---------------------------------------------------------------------------
// Rows 10-12 — multi-bit classes, where a normalised 0/1 would diverge
// ---------------------------------------------------------------------------

#[test]
fn err_10_multi_bit_classes_raw_mask() {
    reset_global_locale();
    for c in b'0'..=b'9' {
        diff_char(c as c_char, &format!("ERRORS row 10: digit {}", c as char));
        let out = c_out(c as c_char);
        assert_eq!(field(&out, "alphanumeric"), 8, "_ISalnum");
        assert_eq!(field(&out, "digit"), 2048, "_ISdigit");
        assert_eq!(field(&out, "hexadecimal"), 4096, "_ISxdigit");
        assert_eq!(field(&out, "graphical"), 32768, "_ISgraph");
        assert_eq!(field(&out, "printing"), 16384, "_ISprint");
        assert_eq!(field(&out, "alphabetic"), 0);
        assert_eq!(field(&out, "punctuation"), 0);
    }
    // Letters carry a different multi-bit combination.
    let out = c_out(b'A' as c_char);
    assert_eq!(field(&out, "uppercase"), 256, "_ISupper");
    assert_eq!(field(&out, "alphabetic"), 1024, "_ISalpha");
    assert_eq!(field(&out, "hexadecimal"), 4096, "_ISxdigit for 'A'");
    assert_eq!(field(&out, "alphanumeric"), 8, "_ISalnum");
    let out = c_out(b'g' as c_char);
    assert_eq!(field(&out, "lowercase"), 512, "_ISlower");
    assert_eq!(field(&out, "hexadecimal"), 0, "'g' is not a hex digit");
}

#[test]
fn err_11_space_is_blank_not_graph() {
    reset_global_locale();
    diff_char(b' ' as c_char, "ERRORS row 11: SPACE");
    let out = c_out(b' ' as c_char);
    assert_eq!(field(&out, "space"), 8192, "_ISspace");
    assert_eq!(field(&out, "blank"), 1, "_ISblank");
    assert_eq!(field(&out, "printing"), 16384, "_ISprint");
    assert_eq!(field(&out, "graphical"), 0, "SPACE is not _ISgraph");
    assert_eq!(field(&out, "punctuation"), 0);
}

#[test]
fn err_12_tab_is_blank_and_cntrl() {
    reset_global_locale();
    diff_char(b'\t' as c_char, "ERRORS row 12: TAB");
    let out = c_out(b'\t' as c_char);
    assert_eq!(field(&out, "control"), 2, "_IScntrl");
    assert_eq!(field(&out, "space"), 8192, "_ISspace");
    assert_eq!(field(&out, "blank"), 1, "_ISblank");
    assert_eq!(field(&out, "printing"), 0, "TAB is not printable");

    // The other whitespace controls are space but NOT blank.
    for c in [b'\n', 0x0B, 0x0C, b'\r'] {
        diff_char(c as c_char, &format!("ERRORS row 12: ws control {c:#04x}"));
        let out = c_out(c as c_char);
        assert_eq!(field(&out, "space"), 8192, "{c:#04x} _ISspace");
        assert_eq!(field(&out, "blank"), 0, "{c:#04x} is not _ISblank");
        assert_eq!(field(&out, "control"), 2, "{c:#04x} _IScntrl");
    }
}

// ---------------------------------------------------------------------------
// Row 13 — setlocale's return value is discarded; calls are idempotent
// ---------------------------------------------------------------------------

#[test]
fn err_13_repeated_calls_idempotent() {
    reset_global_locale();
    let cd = c_driver();
    let rd = rust_driver();
    // Start from a deliberately foreign global locale so the first call is the
    // one that has to reset it. The locale is re-established before each side,
    // because the C's own setlocale would otherwise normalise the state for the
    // Rust side and hide a missing setlocale there.
    for name in ["de_DE.iso88591", "C.utf8", "en_US.utf8"] {
        if !set_global_locale(name) {
            eprintln!("skip: {name} unavailable");
            continue;
        }
        for &c in &[0i8, b'A' as i8, b'z' as i8, -1, -128, 0x7F] {
            assert!(set_global_locale(name));
            let c0 = capture(|| unsafe { cd(c) });
            assert!(set_global_locale(name));
            let r0 = capture(|| unsafe { rd(c) });
            assert_eq!(c0, r0, "row 13: first call from locale {name}, c={c}");
            for n in 1..5 {
                assert!(set_global_locale(name));
                let cn = capture(|| unsafe { cd(c) });
                assert!(set_global_locale(name));
                let rn = capture(|| unsafe { rd(c) });
                assert_eq!(cn, c0, "row 13: C not idempotent at call {n}");
                assert_eq!(rn, r0, "row 13: Rust not idempotent at call {n}");
            }
        }
        reset_global_locale();
    }
    reset_global_locale();
}

// ---------------------------------------------------------------------------
// Row 14 — the callee never flushes; both must share one stdout FILE buffer
// ---------------------------------------------------------------------------

#[test]
fn err_14_shared_stdout_buffer() {
    reset_global_locale();
    set_stdout_buffering(IOFBF);
    let cd = c_driver();
    let rd = rust_driver();
    for &c in &[b'A' as i8, 0, -1, -128] {
        let (c_before, c_after) = capture_two_stage(|| unsafe { cd(c) });
        let (r_before, r_after) = capture_two_stage(|| unsafe { rd(c) });
        assert_eq!(
            c_before,
            r_before,
            "row 14: unflushed bytes differ, c={c}\n  C   : {}\n  Rust: {}",
            render(&c_before),
            render(&r_before)
        );
        assert_eq!(c_after, r_after, "row 14: flushed bytes differ, c={c}");
    }
    set_stdout_buffering(IOFBF);
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries the task calls out, beyond the table
// ---------------------------------------------------------------------------

/// `driver` takes no pointer and no length, so there is no null-pointer or
/// length parameter to abuse. This test records that mechanically (from the
/// header) rather than leaving the boundary silently unaddressed, and checks the
/// one thing that *is* checkable: the symbol's arity and that a well-formed call
/// with the extreme values of the only parameter is total (no crash, no
/// divergence) for the entire domain.
#[test]
fn generic_boundaries_no_pointer_or_length_params() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("driver(") && !l.trim_start().starts_with("//"))
        .collect();
    assert_eq!(decls.len(), 1, "driver.h declares exactly one function");
    assert!(
        decls[0].contains("void driver(char c)"),
        "unexpected prototype: {:?}",
        decls[0]
    );
    assert!(!decls[0].contains('*'), "no pointer parameters exist");

    // Totality over the whole parameter domain, both directions of the boundary.
    reset_global_locale();
    diff_all_chars("generic boundary: total over all 256 char values");
    for v in [c_int::MIN, -1, 0, 1, 255, 256, 257, c_int::MAX] {
        diff_int(v, "generic boundary: extreme int");
    }
}

/// One step past each documented range boundary, in both directions.
#[test]
fn generic_boundaries_one_step_past_ranges() {
    reset_global_locale();
    // ASCII class edges: the byte just below and just above each class.
    let edges: [u8; 22] = [
        0x00, 0x08, 0x09, 0x0A, 0x0D, 0x0E, 0x1F, 0x20, 0x21, 0x2F, 0x30, 0x39, 0x3A, 0x40, 0x41,
        0x46, 0x47, 0x5A, 0x60, 0x61, 0x7A, 0x7B,
    ];
    for e in edges {
        for d in [-1i32, 0, 1] {
            let v = (e as i32 + d) & 0xFF;
            diff_char(
                v as u8 as c_char,
                &format!("generic boundary: {e:#04x} step {d}"),
            );
        }
    }
    // The signed/unsigned char frontier and the EOF slot neighbours.
    for v in [0x7E, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
        diff_char(v as u8 as c_char, "generic boundary: char sign frontier");
    }
}
