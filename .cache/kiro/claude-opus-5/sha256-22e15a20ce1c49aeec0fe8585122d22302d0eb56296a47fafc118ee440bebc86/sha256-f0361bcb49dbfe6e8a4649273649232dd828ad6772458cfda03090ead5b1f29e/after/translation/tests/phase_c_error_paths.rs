//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! The C library has **zero explicit rejection paths** (no returns, asserts,
//! error codes, range checks or null checks — see `ERRORS.md` for the grep that
//! establishes this). Its error surface is therefore entirely the set of
//! boundaries it silently does not check: signed-`char` sign extension, the
//! overflowing truncating store in `driver`, the `%02x` minimum-width flag, and
//! arguments outside the parameter's range presented across the FFI boundary.
//!
//! Each test constructs one such condition, calls BOTH `.so`s, and asserts they
//! agree on the exact outcome — the exact emitted bytes AND the fact that
//! neither rejects, traps, or aborts. Because both functions return `void`,
//! "same error/sentinel" means "same emitted line and same non-rejection", and
//! each test additionally pins the exact expected C bytes so a test cannot pass
//! by both sides being equally broken.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Asserts C and Rust agree, *and* that the shared result is the exact byte
/// string the C semantics demand. Pinning the literal is what stops a row from
/// passing vacuously if both implementations were wrong in the same way.
#[track_caller]
fn assert_both(row: &str, c_out: &[u8], r_out: &[u8], expected: &str) {
    assert_same(row, c_out, r_out);
    assert_eq!(
        String::from_utf8_lossy(c_out),
        expected,
        "{row}: C output did not match the value derived from the C source"
    );
}

// ---------------------------------------------------------------------------
// E1 — printHexCharLine(-1): sign extension, NOT truncated to two digits
// ---------------------------------------------------------------------------

fn e1_print_hex_negative_one_sign_extends() {
    let v = -1i8 as c_char;
    assert_both(
        "E1: printHexCharLine(-1)",
        &cap_print_hex(c(), v),
        &cap_print_hex(rs(), v),
        "ffffffff\n",
    );
}

// ---------------------------------------------------------------------------
// E2 — printHexCharLine(CHAR_MIN)
// ---------------------------------------------------------------------------

fn e2_print_hex_char_min() {
    let v = i8::MIN as c_char; // -128 == 0x80
    assert_both(
        "E2: printHexCharLine(CHAR_MIN = -128)",
        &cap_print_hex(c(), v),
        &cap_print_hex(rs(), v),
        "ffffff80\n",
    );
}

// ---------------------------------------------------------------------------
// E3 — printHexCharLine(CHAR_MAX): one step below the sign boundary
// ---------------------------------------------------------------------------

fn e3_print_hex_char_max_not_sign_extended() {
    let v = i8::MAX as c_char; // 127 == 0x7F
    assert_both(
        "E3: printHexCharLine(CHAR_MAX = 127)",
        &cap_print_hex(c(), v),
        &cap_print_hex(rs(), v),
        "7f\n",
    );
}

// ---------------------------------------------------------------------------
// E4 — printHexCharLine(0): the zero-pad case
// ---------------------------------------------------------------------------

fn e4_print_hex_zero_pads_to_two_digits() {
    assert_both(
        "E4: printHexCharLine(0)",
        &cap_print_hex(c(), 0),
        &cap_print_hex(rs(), 0),
        "00\n",
    );
}

// ---------------------------------------------------------------------------
// E5 — the `02` minimum-width boundary: 0x0F vs 0x10
// ---------------------------------------------------------------------------

fn e5_print_hex_width_boundary_0f_10() {
    assert_both(
        "E5: printHexCharLine(0x0F)",
        &cap_print_hex(c(), 0x0F),
        &cap_print_hex(rs(), 0x0F),
        "0f\n",
    );
    assert_both(
        "E5: printHexCharLine(0x10)",
        &cap_print_hex(c(), 0x10),
        &cap_print_hex(rs(), 0x10),
        "10\n",
    );
}

// ---------------------------------------------------------------------------
// E6 — out-of-range argument across the FFI boundary, printHexCharLine
//
// The nearest analogue of "enum value with no valid variant": an `int` with no
// `char` representation. C does not check it; the callee consumes only the low
// byte. Named boundaries plus a randomized sweep of the full 32-bit space.
// ---------------------------------------------------------------------------

fn e6_print_hex_out_of_range_int_argument() {
    // Named cases with the value the C semantics pin them to.
    let named: &[(c_int, &str)] = &[
        (0x1FF, "ffffffff"),                       // low byte 0xFF -> -1
        (0x100, "00"),                             // low byte 0x00
        (0x17F, "7f"),                             // low byte 0x7F
        (0x180, "ffffff80"),                       // low byte 0x80
        (0xFFFF_FF00u32 as c_int, "00"),           // low byte 0x00
        (0xFFFF_FF01u32 as c_int, "01"),           // low byte 0x01
        (c_int::MIN, "00"),                        // 0x80000000, low byte 0x00
        (c_int::MAX, "ffffffff"),                  // 0x7FFFFFFF, low byte 0xFF
    ];
    for (v, expected) in named {
        assert_both(
            &format!("E6: printHexCharLine as void(*)(int) with 0x{:08x}", *v as u32),
            &cap_print_hex_int(c(), *v),
            &cap_print_hex_int(rs(), *v),
            &format!("{expected}\n"),
        );
    }

    // Randomized sweep of the full 32-bit argument space.
    let mut rng = Rng::new(SEED ^ 0xE6);
    for _ in 0..SAMPLES {
        let v = rng.next_u32() as c_int;
        assert_same(
            &format!("E6 (random): printHexCharLine as void(*)(int) with 0x{:08x}", v as u32),
            &cap_print_hex_int(c(), v),
            &cap_print_hex_int(rs(), v),
        );
    }
}

// ---------------------------------------------------------------------------
// E7 — driver(CHAR_MAX): overflowing truncating store, unchecked by C
// ---------------------------------------------------------------------------

fn e7_driver_char_max_wraps_to_char_min() {
    let v = i8::MAX as c_char; // 127; 127 + 1 truncates to -128
    assert_both(
        "E7: driver(CHAR_MAX = 127)",
        &cap_driver(c(), v),
        &cap_driver(rs(), v),
        "ffffff80\n",
    );
}

// ---------------------------------------------------------------------------
// E8 — driver(-1): the unique input whose result is 0
// ---------------------------------------------------------------------------

fn e8_driver_negative_one_yields_zero() {
    let v = -1i8 as c_char;
    assert_both(
        "E8: driver(-1)",
        &cap_driver(c(), v),
        &cap_driver(rs(), v),
        "00\n",
    );
}

// ---------------------------------------------------------------------------
// E9 — driver(CHAR_MIN)
// ---------------------------------------------------------------------------

fn e9_driver_char_min() {
    let v = i8::MIN as c_char; // -128; -128 + 1 == -127 == 0x81
    assert_both(
        "E9: driver(CHAR_MIN = -128)",
        &cap_driver(c(), v),
        &cap_driver(rs(), v),
        "ffffff81\n",
    );
}

// ---------------------------------------------------------------------------
// E10 — driver(126): one step below the overflow boundary; must NOT wrap
// ---------------------------------------------------------------------------

fn e10_driver_one_below_overflow_does_not_wrap() {
    assert_both(
        "E10: driver(126)",
        &cap_driver(c(), 126),
        &cap_driver(rs(), 126),
        "7f\n",
    );
}

// ---------------------------------------------------------------------------
// E11 — driver across the %02x width boundary
// ---------------------------------------------------------------------------

fn e11_driver_width_boundary() {
    assert_both(
        "E11: driver(0x0E)",
        &cap_driver(c(), 0x0E),
        &cap_driver(rs(), 0x0E),
        "0f\n",
    );
    assert_both(
        "E11: driver(0x0F)",
        &cap_driver(c(), 0x0F),
        &cap_driver(rs(), 0x0F),
        "10\n",
    );
}

// ---------------------------------------------------------------------------
// E12 — out-of-range argument across the FFI boundary, driver
// ---------------------------------------------------------------------------

fn e12_driver_out_of_range_int_argument() {
    let named: &[(c_int, &str)] = &[
        (0x1FF, "00"),                    // low byte 0xFF -> -1 -> 0
        (0x100, "01"),                    // low byte 0x00 -> 1
        (0x17F, "ffffff80"),              // low byte 0x7F -> wraps
        (0x180, "ffffff81"),              // low byte 0x80 -> -127
        (0xFFFF_FF7Fu32 as c_int, "ffffff80"),
        (0xFFFF_FF00u32 as c_int, "01"),
        (c_int::MIN, "01"),               // 0x80000000, low byte 0x00 -> 1
        (c_int::MAX, "00"),               // 0x7FFFFFFF, low byte 0xFF -> 0
    ];
    for (v, expected) in named {
        assert_both(
            &format!("E12: driver as void(*)(int) with 0x{:08x}", *v as u32),
            &cap_driver_int(c(), *v),
            &cap_driver_int(rs(), *v),
            &format!("{expected}\n"),
        );
    }

    let mut rng = Rng::new(SEED ^ 0xE12);
    for _ in 0..SAMPLES {
        let v = rng.next_u32() as c_int;
        assert_same(
            &format!("E12 (random): driver as void(*)(int) with 0x{:08x}", v as u32),
            &cap_driver_int(c(), v),
            &cap_driver_int(rs(), v),
        );
    }
}

// ---------------------------------------------------------------------------
// E13 — repeated / interleaved invocation is stateless and never fails twice
// ---------------------------------------------------------------------------

fn e13_repeated_and_interleaved_calls_are_stateless() {
    for v in [0u8, 1, 0x0F, 0x7F, 0x80, 0xFE, 0xFF] {
        let arg = v as c_char;

        // Same call repeated 8x must give 8 identical lines from both sides.
        let c_rep = capture(|| {
            for _ in 0..8 {
                unsafe { (c().driver)(arg) }
            }
        });
        let r_rep = capture(|| {
            for _ in 0..8 {
                unsafe { (rs().driver)(arg) }
            }
        });
        assert_same(&format!("E13: driver(0x{v:02x}) x8 repeated"), &c_rep, &r_rep);

        let single = cap_driver(c(), arg);
        assert_eq!(
            c_rep,
            single.repeat(8),
            "E13: driver(0x{v:02x}) is not stateless in C"
        );

        // C -> Rust -> C -> Rust on the same value, in one capture.
        let mixed = capture(|| unsafe {
            (c().driver)(arg);
            (rs().driver)(arg);
            (c().print_hex_char_line)(arg);
            (rs().print_hex_char_line)(arg);
        });
        let expected = {
            let d = cap_driver(c(), arg);
            let p = cap_print_hex(c(), arg);
            let mut e = Vec::new();
            e.extend_from_slice(&d);
            e.extend_from_slice(&d);
            e.extend_from_slice(&p);
            e.extend_from_slice(&p);
            e
        };
        assert_same(
            &format!("E13: C/Rust interleaved on 0x{v:02x}"),
            &expected,
            &mixed,
        );
    }
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries recorded N/A in ERRORS.md — asserted mechanically
// so the "N/A" claim cannot silently rot if the API ever grows a pointer or
// length parameter.
// ---------------------------------------------------------------------------

fn generic_boundaries_na_surface_is_still_pointer_and_length_free() {
    let header = include_str!("../../c_src/include/driver.h");
    // Strip comments so the licence text does not produce false positives.
    let code: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains('*'),
        "public header grew a pointer parameter; ERRORS.md's null-pointer N/A row is now wrong:\n{code}"
    );
    assert!(
        !code.contains("size_t") && !code.contains("len"),
        "public header grew a length parameter; ERRORS.md's length N/A row is now wrong:\n{code}"
    );
    assert!(
        !code.contains("enum"),
        "public header grew an enum; ERRORS.md's enum N/A row is now wrong:\n{code}"
    );
    assert!(
        code.contains("void driver(char data);"),
        "public header no longer declares `void driver(char data);`:\n{code}"
    );
}


// -------------------------------------------------------------------------
// Sequential entry point (`harness = false`; see tests/common/mod.rs for why).
// -------------------------------------------------------------------------

fn main() {
    common::install_quiet_panic_hook();
    let mut r = Runner::new();
    r.row("e1_print_hex_negative_one_sign_extends", e1_print_hex_negative_one_sign_extends);
    r.row("e2_print_hex_char_min", e2_print_hex_char_min);
    r.row("e3_print_hex_char_max_not_sign_extended", e3_print_hex_char_max_not_sign_extended);
    r.row("e4_print_hex_zero_pads_to_two_digits", e4_print_hex_zero_pads_to_two_digits);
    r.row("e5_print_hex_width_boundary_0f_10", e5_print_hex_width_boundary_0f_10);
    r.row("e6_print_hex_out_of_range_int_argument", e6_print_hex_out_of_range_int_argument);
    r.row("e7_driver_char_max_wraps_to_char_min", e7_driver_char_max_wraps_to_char_min);
    r.row("e8_driver_negative_one_yields_zero", e8_driver_negative_one_yields_zero);
    r.row("e9_driver_char_min", e9_driver_char_min);
    r.row("e10_driver_one_below_overflow_does_not_wrap", e10_driver_one_below_overflow_does_not_wrap);
    r.row("e11_driver_width_boundary", e11_driver_width_boundary);
    r.row("e12_driver_out_of_range_int_argument", e12_driver_out_of_range_int_argument);
    r.row("e13_repeated_and_interleaved_calls_are_stateless", e13_repeated_and_interleaved_calls_are_stateless);
    r.row("generic_boundaries_na_surface_is_still_pointer_and_length_free", generic_boundaries_na_surface_is_still_pointer_and_length_free);
    r.finish("Phase C");
}
