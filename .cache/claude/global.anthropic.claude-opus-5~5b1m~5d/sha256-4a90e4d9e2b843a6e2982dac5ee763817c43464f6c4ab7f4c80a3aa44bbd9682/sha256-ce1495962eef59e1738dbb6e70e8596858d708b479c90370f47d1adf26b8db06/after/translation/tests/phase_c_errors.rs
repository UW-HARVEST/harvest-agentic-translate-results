//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`.  The C library contains **no** explicit
//! rejection path (no `return -1`, no `NULL` check, no `assert`, no range
//! check — see the grep evidence in `ERRORS.md`), so each row asserts the
//! *specific* non-rejection behaviour and that Rust matches it exactly, rather
//! than settling for "both failed somehow".
//!
//! The generic C-API boundaries are covered too: out-of-range integers passed
//! across the FFI boundary (the C-enum-accepts-any-int analogue for this API),
//! values one step past the `char` range, and a broken output stream.  Null
//! pointers and zero/oversized lengths do not exist in this API's signatures
//! (both functions are `void f(char)`), which is asserted structurally below.

#![allow(dead_code)]

include!("common/harness.rs");

/// Row 1: `printHexCharLine` validates nothing — every one of the 256 bit
/// patterns is accepted, never rejected, and prints the documented form.
#[test]
fn err_row1_print_all_256_bit_patterns_accepted() {
    let cf = sym_char(c_lib(), PRINT_HEX);
    let rf = sym_char(rust_lib(), PRINT_HEX);
    for v in all_256() {
        let c = capture(|| unsafe { cf(v) });
        let r = capture(|| unsafe { rf(v) });
        assert_eq!(c, r, "[err-1] divergence at 0x{:02x}", v as u8);
        // Not rejected: real output was produced, terminated by '\n'.
        assert!(!c.is_empty() && *c.last().unwrap() == b'\n', "[err-1] no line emitted");
        // And it is exactly what the C semantics dictate.
        let expected = if v < 0 {
            format!("{:08x}\n", v as i32 as u32)
        } else {
            format!("{:02x}\n", v)
        };
        assert_eq!(
            String::from_utf8_lossy(&c),
            expected,
            "[err-1] C output for 0x{:02x} not the expected non-rejection form",
            v as u8
        );
    }
}

/// Row 2: `driver` validates nothing either.
#[test]
fn err_row2_driver_all_256_bit_patterns_accepted() {
    let cf = sym_char(c_lib(), DRIVER);
    let rf = sym_char(rust_lib(), DRIVER);
    for v in all_256() {
        let c = capture(|| unsafe { cf(v) });
        let r = capture(|| unsafe { rf(v) });
        assert_eq!(c, r, "[err-2] divergence at 0x{:02x}", v as u8);
        let result = v.wrapping_add(1);
        let expected = if result < 0 {
            format!("{:08x}\n", result as i32 as u32)
        } else {
            format!("{:02x}\n", result)
        };
        assert_eq!(
            String::from_utf8_lossy(&c),
            expected,
            "[err-2] C output for 0x{:02x} not the expected non-rejection form",
            v as u8
        );
    }
}

/// Row 3: `driver(0x7f)` — `data + 1` overflows the signed `char` range.  This
/// is implementation-defined narrowing, NOT an error: gcc truncates to `-128`
/// and the value prints sign-extended as `ffffff80`.
#[test]
fn err_row3_driver_signed_overflow_boundary_0x7f() {
    let cf = sym_char(c_lib(), DRIVER);
    let rf = sym_char(rust_lib(), DRIVER);
    let c = capture(|| unsafe { cf(0x7f) });
    let r = capture(|| unsafe { rf(0x7f) });
    assert_eq!(c, r, "[err-3] C={:?} Rust={:?}", show(&c), show(&r));
    assert_eq!(
        String::from_utf8_lossy(&c),
        "ffffff80\n",
        "[err-3] C did not truncate 128 to -128 as expected"
    );
    // One step either side, to pin the boundary rather than a single point.
    diff_char_each(DRIVER, "err-3-neighbours", &[0x7e, 0x7f, 0x80u8 as i8]);
}

/// Row 4: `driver(0xff)` — `-1 + 1 == 0`, the only case where both `%02x`
/// digits are padding.  Still no error.
#[test]
fn err_row4_driver_wrap_to_zero_0xff() {
    let cf = sym_char(c_lib(), DRIVER);
    let rf = sym_char(rust_lib(), DRIVER);
    let c = capture(|| unsafe { cf(0xffu8 as i8) });
    let r = capture(|| unsafe { rf(0xffu8 as i8) });
    assert_eq!(c, r, "[err-4] C={:?} Rust={:?}", show(&c), show(&r));
    assert_eq!(String::from_utf8_lossy(&c), "00\n", "[err-4] unexpected C output");
    diff_char_each(DRIVER, "err-4-neighbours", &[0xfeu8 as i8, 0xffu8 as i8, 0x00]);
}

/// Row 5: `printHexCharLine` over `0x00..=0x0f` — the zero-padding path.
#[test]
fn err_row5_print_single_hex_digit_padding() {
    let inputs: Vec<i8> = (0x00..=0x0f).collect();
    diff_char_each(PRINT_HEX, "err-5", &inputs);
    let cf = sym_char(c_lib(), PRINT_HEX);
    for v in inputs {
        let c = capture(|| unsafe { cf(v) });
        assert_eq!(c.len(), 3, "[err-5] expected 2 digits + newline for 0x{:02x}", v);
        assert_eq!(c[0], b'0', "[err-5] missing zero pad for 0x{:02x}", v);
    }
}

/// Row 6: out-of-range values across the FFI boundary.  The C prototype takes
/// `char`, but nothing stops a caller from declaring `void f(int)` / `f(long)`
/// and passing a value with no `char` representation — the direct analogue of
/// passing an int with no valid enum variant.  The C callee does not reject it;
/// it silently truncates to the low byte.  Rust must do the identical thing,
/// producing the SAME bytes as the equivalent in-range call.
#[test]
fn err_row6_out_of_range_int_across_ffi_truncates() {
    let c_int_fns = [sym_int(c_lib(), PRINT_HEX), sym_int(c_lib(), DRIVER)];
    let r_int_fns = [sym_int(rust_lib(), PRINT_HEX), sym_int(rust_lib(), DRIVER)];
    let c_char_fns = [sym_char(c_lib(), PRINT_HEX), sym_char(c_lib(), DRIVER)];
    let names = ["printHexCharLine", "driver"];

    let mut rng = Rng::new(SEED ^ 0x06);
    let mut values: Vec<i32> = vec![
        -1,
        256,
        257,
        -256,
        -257,
        128,
        0x100,
        0x1_0000,
        0x1234_5678,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    values.extend((0..300).map(|_| rng.i32()));

    for i in 0..2 {
        for &v in &values {
            let c = capture(|| unsafe { c_int_fns[i](v) });
            let r = capture(|| unsafe { r_int_fns[i](v) });
            assert_eq!(
                c,
                r,
                "[err-6] {} as fn(int) with {v} (0x{:08x}): C={:?} Rust={:?}",
                names[i],
                v as u32,
                show(&c),
                show(&r)
            );
            // And confirm the "truncate, don't reject" semantics explicitly:
            // it must equal the in-range call with the low byte.
            let low = v as u8 as i8;
            let c_low = capture(|| unsafe { c_char_fns[i](low) });
            assert_eq!(
                c, c_low,
                "[err-6] {} did not behave as low-byte truncation for 0x{:08x}",
                names[i], v as u32
            );
        }
    }

    // 64-bit register garbage in the upper half as well.
    let mut longs: Vec<i64> = vec![
        i64::MIN,
        i64::MAX,
        -1,
        0x1234_5678_9abc_def0,
        0xffff_ffff_ffff_ff00u64 as i64,
    ];
    longs.extend((0..200).map(|_| rng.i64()));
    diff_long_each(PRINT_HEX, "err-6-long", &longs);
    diff_long_each(DRIVER, "err-6-long", &longs);
}

/// Row 7: `printf` fails (fd 1 closed, stdout unbuffered) and the library
/// ignores the return value.  Run in a forked child so a crash/abort in either
/// library is observable as a wait status; both must exit cleanly with 0.
#[test]
fn err_row7_printf_failure_is_ignored() {
    for (name, sym) in [("printHexCharLine", PRINT_HEX), ("driver", DRIVER)] {
        let cf = sym_char(c_lib(), sym);
        let rf = sym_char(rust_lib(), sym);
        for v in [0i8, 0x7f, -1, -128, 0x41] {
            let cs = status_in_child(|| unsafe { cf(v) });
            let rs = status_in_child(|| unsafe { rf(v) });
            assert_eq!(
                cs, rs,
                "[err-7] {name}(0x{:02x}): wait status C={cs} Rust={rs}",
                v as u8
            );
            assert_eq!(
                cs, 0,
                "[err-7] {name}(0x{:02x}): C did not exit cleanly after a failing printf",
                v as u8
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries that Phase C mandates even when absent from the
// table.
// ---------------------------------------------------------------------------

/// Null pointers / zero-length / oversized-length are structurally impossible
/// here: neither exported function takes a pointer or a length.  Assert that
/// the ABI surface really is `void f(char)` for both, so this exemption stays
/// honest if the C ever changes.
#[test]
fn err_generic_no_pointer_or_length_parameters_exist() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/driver.c"),
    )
    .expect("read driver.c");

    assert!(header.contains("void driver(char data);"), "driver.h prototype changed");
    assert!(
        src.contains("void printHexCharLine (char charHex)"),
        "printHexCharLine signature changed"
    );
    assert!(src.contains("void driver(char data)"), "driver signature changed");

    // No pointer parameters anywhere in the two public signatures.
    for line in src.lines().chain(header.lines()) {
        let l = line.trim_start();
        if l.starts_with("void driver") || l.starts_with("void printHexCharLine") {
            assert!(!l.contains('*'), "a pointer parameter appeared: {l}");
        }
    }

    // No error-return / validation constructs in the C at all.
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["return ", "assert", "NULL", "errno", "exit(", "if (", "if(", "switch"] {
        assert!(
            !code.contains(forbidden),
            "ERRORS.md claims the C has no `{forbidden}`, but it does now — retable the error surface"
        );
    }
}

/// One step past the valid `char` range on both ends, through the true
/// `char` prototype (`0x7f`→`0x80`, `0xff`→`0x00` wrap) and through the `int`
/// prototype (`127`→`128`, `255`→`256`, `0`→`-1`).
#[test]
fn err_generic_one_step_past_range() {
    diff_char_each(PRINT_HEX, "err-step", &[0x7f, 0x80u8 as i8, 0xff_u8 as i8, 0x00]);
    diff_char_each(DRIVER, "err-step", &[0x7f, 0x80u8 as i8, 0xff_u8 as i8, 0x00]);
    diff_int_each(PRINT_HEX, "err-step-int", &[127, 128, 255, 256, 0, -1, -128, -129]);
    diff_int_each(DRIVER, "err-step-int", &[127, 128, 255, 256, 0, -1, -128, -129]);
}

/// Repeated / idempotent invocation: calling either function many times must
/// not accumulate hidden state in one library but not the other.
#[test]
fn err_generic_repeated_calls_are_stateless() {
    for sym in [PRINT_HEX, DRIVER] {
        let inputs: Vec<i8> = std::iter::repeat(0x41i8).take(500).collect();
        diff_char_batch(sym, "err-stateless", &inputs);
    }
}
