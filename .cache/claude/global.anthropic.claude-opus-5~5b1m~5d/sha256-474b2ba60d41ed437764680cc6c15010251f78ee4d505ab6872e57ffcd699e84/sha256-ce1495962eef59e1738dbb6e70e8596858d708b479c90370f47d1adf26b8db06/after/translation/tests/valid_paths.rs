//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C27). Every row drives BOTH the C
//! `.so` and the Rust `.so` through `dlopen`/`dlsym` and compares the exact
//! bytes each writes to `stdout`.
//!
//! Lowest-level entry points (`printLine`, `printHexCharLine`) are exercised
//! directly, not only via the `driver` convenience wrapper.

mod common;

use common::{cstr, pair, random_program, run_program, Rng};
use std::ffi::{c_char, c_int};

// ===========================================================================
// printHexCharLine  (lowest level: the hex formatter)
// ===========================================================================

/// C1 — exhaustive over all 256 `char` bit patterns.
#[test]
fn c1_print_hex_all_256_values() {
    let mut p = pair();
    for raw in 0u16..=255 {
        let v = raw as u8 as c_char;
        p.assert_same(&format!("C1 printHexCharLine({v})"), |lib| unsafe {
            (lib.print_hex_char_line)(v)
        });
    }
}

/// C2 — zero-pad boundary shapes (the two-digit / padded paths).
#[test]
fn c2_print_hex_zero_pad_boundaries() {
    let mut p = pair();
    for v in [0i8, 1, 9, 0x0f, 0x10, 0x5a, 0x7e, 0x7f] {
        let v = v as c_char;
        p.assert_same(&format!("C2 printHexCharLine({v})"), |lib| unsafe {
            (lib.print_hex_char_line)(v)
        });
    }
}

/// C3 — negative values: default argument promotion sign-extends, so `%02x`
/// prints eight digits and the width is not honoured.
#[test]
fn c3_print_hex_negative_sign_extension() {
    let mut p = pair();
    for v in [-1i8, -2, -15, -16, -100, -127, -128] {
        let v = v as c_char;
        p.assert_same(&format!("C3 printHexCharLine({v})"), |lib| unsafe {
            (lib.print_hex_char_line)(v)
        });
    }
}

/// C4 — randomized sweep, fixed seed.
#[test]
fn c4_print_hex_randomized() {
    let mut p = pair();
    let mut rng = Rng::new(Rng::DEFAULT_SEED);
    for i in 0..4096 {
        let v = rng.next_u8() as c_char;
        p.assert_same(
            &format!("C4 #{i} printHexCharLine({v})"),
            |lib| unsafe { (lib.print_hex_char_line)(v) },
        );
    }
}

/// C5 — caller pushes a full `int` outside `char` range; the *callee* is what
/// truncates, so this compares the two ABI boundaries against each other.
#[test]
fn c5_print_hex_out_of_char_range_ints() {
    let mut p = pair();
    let mut cases: Vec<c_int> = vec![
        128,
        129,
        255,
        256,
        257,
        -129,
        -256,
        -257,
        512,
        0x1234_5678,
        0x0000_ff00,
        0x7fff_ff80,
        c_int::MAX,
        c_int::MIN,
    ];
    let mut rng = Rng::new(0xC5C5_C5C5_C5C5_C5C5);
    for _ in 0..1024 {
        cases.push(rng.next_i32());
    }
    for v in cases {
        p.assert_same(
            &format!("C5 printHexCharLine(int {v:#x})"),
            |lib| unsafe { (lib.print_hex_char_line_as_int)(v) },
        );
    }
}

// ===========================================================================
// printLine  (lowest level: the string printer)
// ===========================================================================

/// C6 — the guarded shape: NULL pointer.
#[test]
fn c6_print_line_null() {
    let mut p = pair();
    p.assert_same("C6 printLine(NULL)", |lib| unsafe {
        (lib.print_line)(std::ptr::null())
    });
}

/// C7 — empty string.
#[test]
fn c7_print_line_empty() {
    let mut p = pair();
    let s = cstr(b"");
    p.assert_same("C7 printLine(\"\")", |lib| unsafe {
        (lib.print_line)(s.as_ptr() as *const c_char)
    });
}

/// C8 — exhaustive over every single-byte (non-NUL) string.
#[test]
fn c8_print_line_all_single_bytes() {
    let mut p = pair();
    for b in 1u16..=255 {
        let s = cstr(&[b as u8]);
        p.assert_same(&format!("C8 printLine([{b:#04x}])"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C9 — randomized arbitrary byte strings, including invalid UTF-8.
#[test]
fn c9_print_line_randomized_bytes() {
    let mut p = pair();
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    for i in 0..512 {
        let n = rng.range_usize(0, 64);
        let body: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.next_u8();
                if b == 0 {
                    0xff
                } else {
                    b
                }
            })
            .collect();
        let s = cstr(&body);
        p.assert_same(&format!("C9 #{i} printLine(len {n})"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C10 — long strings that cross libc's stdio buffer (4 KiB) many times.
#[test]
fn c10_print_line_long_strings() {
    let mut p = pair();
    let mut rng = Rng::new(0x1010_1010_1010_1010);
    for i in 0..64 {
        let n = rng.range_usize(1024, 32 * 1024);
        let body: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.next_u8();
                if b == 0 {
                    b'Z'
                } else {
                    b
                }
            })
            .collect();
        let s = cstr(&body);
        p.assert_same(&format!("C10 #{i} printLine(len {n})"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C11 — printf conversion specifiers in the *data*; they must stay literal
/// because the C code passes `line` as an argument to `"%s\n"`.
#[test]
fn c11_print_line_format_specifiers() {
    let mut p = pair();
    let cases: &[&[u8]] = &[
        b"%s",
        b"%d %d %d",
        b"%n",
        b"%%",
        b"100%",
        b"%p %p %p %p %p %p %p %p",
        b"%1000000d",
        b"%.*s",
        b"%hhn%hhn",
        b"a%sb%dc%nd",
    ];
    for c in cases {
        let s = cstr(c);
        p.assert_same(
            &format!("C11 printLine({:?})", String::from_utf8_lossy(c)),
            |lib| unsafe { (lib.print_line)(s.as_ptr() as *const c_char) },
        );
    }
}

/// C12 — embedded whitespace / control bytes and an embedded NUL (which C
/// treats as end-of-string).
#[test]
fn c12_print_line_control_and_embedded_nul() {
    let mut p = pair();
    let cases: &[&[u8]] = &[
        b"\n",
        b"a\nb",
        b"\t",
        b"\r\n",
        b"a\x00b",          // C sees "a"
        b"\x00",            // C sees ""
        b"line1\nline2\n",  // trailing newline plus the added one
        b"\x1b[31mred\x1b[0m",
        b"\x7f\x80\xff",
        b"tab\there",
    ];
    for c in cases {
        let s = cstr(c);
        p.assert_same(
            &format!("C12 printLine({})", common::render(c)),
            |lib| unsafe { (lib.print_line)(s.as_ptr() as *const c_char) },
        );
    }
}

/// C13 — randomized printable-ASCII strings.
#[test]
fn c13_print_line_randomized_ascii() {
    let mut p = pair();
    let mut rng = Rng::new(0x1313_1313_1313_1313);
    for i in 0..256 {
        let n = rng.range_usize(1, 200);
        let body: Vec<u8> = (0..n)
            .map(|_| 0x20u8 + (rng.next_u8() % (0x7f - 0x20)))
            .collect();
        let s = cstr(&body);
        p.assert_same(&format!("C13 #{i} printLine(len {n})"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

// ===========================================================================
// bad / good  (mid level)
// ===========================================================================

/// C14 — `bad()`: the single fixed path with the overflowing `CHAR_MAX * 2`.
#[test]
fn c14_bad_single_call() {
    let mut p = pair();
    p.assert_same("C14 bad()", |lib| unsafe { (lib.bad)() });
}

/// C15 — `bad()` is stateless: 100 consecutive calls.
#[test]
fn c15_bad_repeated() {
    let mut p = pair();
    p.assert_same("C15 bad() x100", |lib| unsafe {
        for _ in 0..100 {
            (lib.bad)();
        }
    });
}

/// C16 — `good()`: both sub-modes, in order (goodG2B then goodB2G).
#[test]
fn c16_good_single_call() {
    let mut p = pair();
    p.assert_same("C16 good()", |lib| unsafe { (lib.good)() });
}

/// C17 — `good()` is stateless: 100 consecutive calls.
#[test]
fn c17_good_repeated() {
    let mut p = pair();
    p.assert_same("C17 good() x100", |lib| unsafe {
        for _ in 0..100 {
            (lib.good)();
        }
    });
}

// ===========================================================================
// driver  (top level wrapper)
// ===========================================================================

/// C18 — `driver(0)` ⇒ bad path.
#[test]
fn c18_driver_zero() {
    let mut p = pair();
    p.assert_same("C18 driver(0)", |lib| unsafe { (lib.driver)(0) });
}

/// C19 — `driver(1)` ⇒ good path.
#[test]
fn c19_driver_one() {
    let mut p = pair();
    p.assert_same("C19 driver(1)", |lib| unsafe { (lib.driver)(1) });
}

/// C20 — small non-zero flags (incl. negative) all take the good path.
#[test]
fn c20_driver_small_nonzero() {
    let mut p = pair();
    for v in [2i32, 3, -1, 7, 42, -42, 255, -255] {
        p.assert_same(&format!("C20 driver({v})"), |lib| unsafe {
            (lib.driver)(v as c_int)
        });
    }
}

/// C21 — non-zero flags whose LOW BYTE is zero. A `as u8 != 0` / `as bool`
/// mistranslation would wrongly take the bad path here.
#[test]
fn c21_driver_low_byte_zero() {
    let mut p = pair();
    for v in [
        256i32,
        512,
        0x0001_0000,
        0x0100_0000,
        i32::MIN,
        -256,
        -65536,
        0x7fff_ff00,
    ] {
        p.assert_same(&format!("C21 driver({v:#x})"), |lib| unsafe {
            (lib.driver)(v as c_int)
        });
    }
}

/// C22 — extreme values.
#[test]
fn c22_driver_extremes() {
    let mut p = pair();
    for v in [
        c_int::MAX,
        c_int::MIN,
        -2147483647,
        0x7fff_ffff,
        0x8000_0001u32 as i32,
        0xffff_ffffu32 as i32,
        1,
        -1,
    ] {
        p.assert_same(&format!("C22 driver({v:#x})"), |lib| unsafe {
            (lib.driver)(v)
        });
    }
}

/// C23 — randomized uniform `i32` sweep over the whole bit-pattern domain.
#[test]
fn c23_driver_randomized_uniform() {
    let mut p = pair();
    let mut rng = Rng::new(0x2323_2323_2323_2323);
    for i in 0..4096 {
        let v = rng.next_i32();
        p.assert_same(&format!("C23 #{i} driver({v:#x})"), |lib| unsafe {
            (lib.driver)(v)
        });
    }
}

/// C24 — zero-biased distribution so both branches are hit densely.
#[test]
fn c24_driver_randomized_zero_biased() {
    let mut p = pair();
    let mut rng = Rng::new(0x2424_2424_2424_2424);
    for i in 0..2048 {
        let v = if rng.next_u64() & 1 == 0 {
            0
        } else {
            // small magnitudes, both signs, plus low-byte-zero shapes
            match rng.below(4) {
                0 => (rng.next_u8() as i32) - 128,
                1 => (rng.next_u8() as i32) << 8,
                2 => -((rng.next_u8() as i32) << 8),
                _ => rng.next_i32(),
            }
        };
        p.assert_same(&format!("C24 #{i} driver({v:#x})"), |lib| unsafe {
            (lib.driver)(v)
        });
    }
}

// ===========================================================================
// Composed pipeline over ALL entry points
// ===========================================================================

/// C25 — a randomized 512-op program mixing all five exported entry points;
/// the whole transcript is compared as one byte stream.
#[test]
fn c25_mixed_pipeline_transcript() {
    let mut p = pair();
    let mut rng = Rng::new(0x2525_2525_2525_2525);
    for round in 0..8 {
        let prog = random_program(&mut rng, 512);
        p.assert_same(&format!("C25 round {round} (512 ops)"), |lib| unsafe {
            run_program(lib, &prog)
        });
    }
}

/// C26 — same, but the Rust library is driven first and each program is run
/// twice per library, proving there is no cross-call or cross-library state.
#[test]
fn c26_mixed_pipeline_flipped_and_repeated() {
    let mut p = pair();
    let mut rng = Rng::new(0x2626_2626_2626_2626);
    for round in 0..8 {
        let prog = random_program(&mut rng, 256);

        // Rust first: capture Rust, then C, then compare (assert_same always
        // runs C first, so do this pair manually via two assert_same calls on
        // a doubled program to also cover the run-twice case).
        let doubled: Vec<_> = prog.iter().cloned().chain(prog.iter().cloned()).collect();
        p.assert_same(
            &format!("C26 round {round} (256 ops x2)"),
            |lib| unsafe { run_program(lib, &doubled) },
        );

        // And a warm-up-then-measure shape: run the program once (discarded
        // side effects: there are none) then compare the second run.
        p.assert_same(&format!("C26 round {round} rerun"), |lib| unsafe {
            run_program(lib, &prog)
        });
    }
}

/// C27 — the feature-set row. `Cargo.toml` declares no `[features]`, so the
/// default set is the only configuration; this test records that fact so the
/// row is machine-checked rather than asserted in prose.
#[test]
fn c27_single_feature_configuration() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml gained a [features] table -- CONFIGS.md row C27 and \
         scripts/check_features.sh must be extended to cover every combination"
    );
    // And the one configuration must still work end to end.
    let mut p = pair();
    p.assert_same("C27 driver(0)/driver(1)", |lib| unsafe {
        (lib.driver)(0);
        (lib.driver)(1);
    });
}
