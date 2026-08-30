//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every test drives BOTH shared objects
//! through `dlsym` and compares the bytes they write byte-for-byte.  Inputs
//! are randomized from a fixed seed (and, where the domain is only 256 wide,
//! exhaustive).
//!
//! Both public entry points are exercised directly, including the low-level
//! `printHexCharLine` (which is not even declared in `driver.h` but *is*
//! exported by the `.so`) -- not just the `driver` convenience wrapper.

#![allow(dead_code)]

include!("common/harness.rs");

// ---------------------------------------------------------------------------
// printHexCharLine -- the LOW-LEVEL entry point, called directly.
// ---------------------------------------------------------------------------

/// Row 1: `charHex == 0`, both `%02x` digits come from padding.
#[test]
fn cfg_row01_print_zero() {
    diff_char_each(PRINT_HEX, "cfg-01", &[0]);
}

/// Row 2: `0x01..=0x0f` — one significant digit, one padded.
#[test]
fn cfg_row02_print_single_digit_range() {
    let mut inputs = random_chars(200, 0x01, 0x0f, SEED ^ 0x02);
    inputs.extend([0x01i8, 0x0f]); // boundaries of the class
    diff_char_each(PRINT_HEX, "cfg-02", &inputs);
}

/// Row 3: `0x10..=0x7f` — exactly two digits, no padding.
#[test]
fn cfg_row03_print_two_digit_range() {
    let mut inputs = random_chars(300, 0x10, 0x7f, SEED ^ 0x03);
    inputs.extend([0x10i8, 0x7f]);
    diff_char_each(PRINT_HEX, "cfg-03", &inputs);
}

/// Row 4: `0x80..=0xff` — negative `char`, sign-extended, EIGHT hex digits.
#[test]
fn cfg_row04_print_negative_range() {
    let mut inputs = random_chars(300, 0x80, 0xff, SEED ^ 0x04);
    inputs.extend([0x80u8 as i8, 0xffu8 as i8]);
    diff_char_each(PRINT_HEX, "cfg-04", &inputs);
}

/// Row 5: the sign boundary pair `0x7f` then `0x80`.
#[test]
fn cfg_row05_print_sign_boundary_pair() {
    diff_char_each(PRINT_HEX, "cfg-05", &[0x7f, 0x80u8 as i8]);
    diff_char_batch(PRINT_HEX, "cfg-05-batch", &[0x7f, 0x80u8 as i8]);
}

/// Row 6: exhaustive sweep, one capture per value.
#[test]
fn cfg_row06_print_exhaustive_each() {
    diff_char_each(PRINT_HEX, "cfg-06", &all_256());
}

/// Row 7: wrong-width prototype — full 32-bit / 64-bit register values.
#[test]
fn cfg_row07_print_full_width_register() {
    let mut rng = Rng::new(SEED ^ 0x07);
    let mut ints: Vec<i32> = vec![
        0,
        1,
        -1,
        255,
        256,
        257,
        0x1234_5678,
        i32::MIN,
        i32::MAX,
        0x0000_0100,
        0x7fff_ff80u32 as i32,
    ];
    ints.extend((0..200).map(|_| rng.i32()));
    diff_int_each(PRINT_HEX, "cfg-07-int", &ints);

    let mut longs: Vec<i64> = vec![0, -1, i64::MIN, i64::MAX, 0x1234_5678_9abc_def0];
    longs.extend((0..200).map(|_| rng.i64()));
    diff_long_each(PRINT_HEX, "cfg-07-long", &longs);
}

/// Row 8: all 256 values in ONE capture — ordering and stdio buffering.
#[test]
fn cfg_row08_print_exhaustive_batch() {
    diff_char_batch(PRINT_HEX, "cfg-08", &all_256());
}

/// Row 9: 1000 randomized calls in one capture.
#[test]
fn cfg_row09_print_bulk_random_batch() {
    let inputs = random_chars(1000, 0x00, 0xff, SEED ^ 0x09);
    diff_char_batch(PRINT_HEX, "cfg-09", &inputs);
}

// ---------------------------------------------------------------------------
// driver -- the convenience wrapper (`data + 1` then the low-level call).
// ---------------------------------------------------------------------------

/// Row 10: `data == 0` → `result == 1` → `01`.
#[test]
fn cfg_row10_driver_zero() {
    diff_char_each(DRIVER, "cfg-10", &[0]);
}

/// Row 11: `data` in `0x00..=0x0e` — result stays in the single-digit class.
#[test]
fn cfg_row11_driver_single_digit_result() {
    let mut inputs = random_chars(200, 0x00, 0x0e, SEED ^ 0x11);
    inputs.extend([0x00i8, 0x0e]);
    diff_char_each(DRIVER, "cfg-11", &inputs);
}

/// Row 12: `data` in `0x0f..=0x7e` — two-digit result, no overflow.
#[test]
fn cfg_row12_driver_two_digit_result() {
    let mut inputs = random_chars(300, 0x0f, 0x7e, SEED ^ 0x12);
    inputs.extend([0x0fi8, 0x7e]);
    diff_char_each(DRIVER, "cfg-12", &inputs);
}

/// Row 13: `data` in `0x80..=0xfe` — negative, result still negative.
#[test]
fn cfg_row13_driver_negative_result() {
    let mut inputs = random_chars(300, 0x80, 0xfe, SEED ^ 0x13);
    inputs.extend([0x80u8 as i8, 0xfeu8 as i8]);
    diff_char_each(DRIVER, "cfg-13", &inputs);
}

/// Row 14: `data == 0x7f` — signed-overflow narrowing to `-128`.
#[test]
fn cfg_row14_driver_overflow_boundary() {
    diff_char_each(DRIVER, "cfg-14", &[0x7f]);
}

/// Row 15: `data == 0xff` (`-1`) — wraps to `0`, prints `00`.
#[test]
fn cfg_row15_driver_wrap_to_zero() {
    diff_char_each(DRIVER, "cfg-15", &[0xffu8 as i8]);
}

/// Row 16: exhaustive sweep, one capture per value.
#[test]
fn cfg_row16_driver_exhaustive_each() {
    diff_char_each(DRIVER, "cfg-16", &all_256());
}

/// Row 17: wrong-width prototype — full 32-bit / 64-bit register values.
#[test]
fn cfg_row17_driver_full_width_register() {
    let mut rng = Rng::new(SEED ^ 0x17);
    let mut ints: Vec<i32> = vec![
        0,
        1,
        -1,
        127,
        128,
        255,
        256,
        257,
        0x1234_5678,
        0x1234_567f,
        0x1234_56ff,
        i32::MIN,
        i32::MAX,
    ];
    ints.extend((0..200).map(|_| rng.i32()));
    diff_int_each(DRIVER, "cfg-17-int", &ints);

    let mut longs: Vec<i64> = vec![0, -1, i64::MIN, i64::MAX, 0x1234_5678_9abc_def0];
    longs.extend((0..200).map(|_| rng.i64()));
    diff_long_each(DRIVER, "cfg-17-long", &longs);
}

/// Row 18: all 256 values in ONE capture.
#[test]
fn cfg_row18_driver_exhaustive_batch() {
    diff_char_batch(DRIVER, "cfg-18", &all_256());
}

/// Row 19: 1000 randomized calls in one capture.
#[test]
fn cfg_row19_driver_bulk_random_batch() {
    let inputs = random_chars(1000, 0x00, 0xff, SEED ^ 0x19);
    diff_char_batch(DRIVER, "cfg-19", &inputs);
}

// ---------------------------------------------------------------------------
// Composed / cross-entry-point rows.
// ---------------------------------------------------------------------------

/// Row 20: interleave BOTH entry points inside a single capture, so the
/// wrapper and the low-level function share one output stream — the composed
/// pipeline that per-function tests cannot see.
#[test]
fn cfg_row20_interleaved_both_entry_points() {
    let c_drv = sym_char(c_lib(), DRIVER);
    let c_phx = sym_char(c_lib(), PRINT_HEX);
    let r_drv = sym_char(rust_lib(), DRIVER);
    let r_phx = sym_char(rust_lib(), PRINT_HEX);

    let mut rng = Rng::new(SEED ^ 0x20);
    // (use_driver, value) script, replayed identically against both libraries
    let script: Vec<(bool, i8)> = (0..1000)
        .map(|_| {
            let w = rng.next_u64();
            ((w & 1) == 0, (w >> 8) as u8 as i8)
        })
        .collect();

    let c = capture(|| {
        for &(use_driver, v) in &script {
            unsafe {
                if use_driver {
                    c_drv(v)
                } else {
                    c_phx(v)
                }
            }
        }
    });
    let r = capture(|| {
        for &(use_driver, v) in &script {
            unsafe {
                if use_driver {
                    r_drv(v)
                } else {
                    r_phx(v)
                }
            }
        }
    });
    assert_eq!(
        c,
        r,
        "[cfg-20] interleaved driver/printHexCharLine pipeline diverged\nC   ={:?}\nRust={:?}",
        show(&c),
        show(&r)
    );
}

/// Row 21: the internal composition the C performs — for every `d`,
/// `driver(d)` must equal `printHexCharLine(d + 1)`.  Checked on BOTH
/// libraries, so a Rust `driver` that got the `+ 1` narrowing wrong is caught
/// even if its own output happened to look plausible.
#[test]
fn cfg_row21_driver_equals_print_of_data_plus_one() {
    let c_drv = sym_char(c_lib(), DRIVER);
    let c_phx = sym_char(c_lib(), PRINT_HEX);
    let r_drv = sym_char(rust_lib(), DRIVER);
    let r_phx = sym_char(rust_lib(), PRINT_HEX);

    for d in all_256() {
        let succ = d.wrapping_add(1);

        let c_via_driver = capture(|| unsafe { c_drv(d) });
        let c_via_print = capture(|| unsafe { c_phx(succ) });
        assert_eq!(
            c_via_driver,
            c_via_print,
            "[cfg-21] C self-consistency broke at 0x{:02x}",
            d as u8
        );

        let r_via_driver = capture(|| unsafe { r_drv(d) });
        let r_via_print = capture(|| unsafe { r_phx(succ) });
        assert_eq!(
            r_via_driver,
            r_via_print,
            "[cfg-21] Rust driver(0x{:02x}) != Rust printHexCharLine(0x{:02x}): {:?} vs {:?}",
            d as u8,
            succ as u8,
            show(&r_via_driver),
            show(&r_via_print)
        );

        assert_eq!(
            c_via_driver,
            r_via_driver,
            "[cfg-21] C vs Rust driver diverged at 0x{:02x}",
            d as u8
        );
    }
}

/// Row 22: single-byte datum, so there is no endianness/width axis.  Confirm
/// over the exhaustive sweep that the output alphabet really is just lowercase
/// hex digits plus `\n` — for BOTH libraries, and identical.
#[test]
fn cfg_row22_output_alphabet_is_lowercase_hex_and_newline() {
    for sym in [PRINT_HEX, DRIVER] {
        let cf = sym_char(c_lib(), sym);
        let rf = sym_char(rust_lib(), sym);
        let all = all_256();
        let c = capture(|| {
            for &v in &all {
                unsafe { cf(v) }
            }
        });
        let r = capture(|| {
            for &v in &all {
                unsafe { rf(v) }
            }
        });
        assert_eq!(c, r, "[cfg-22] divergence for {:?}", String::from_utf8_lossy(sym));
        for &b in &c {
            assert!(
                b == b'\n' || b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
                "[cfg-22] unexpected output byte {b:#04x} from C"
            );
        }
        // Exactly one line per call.
        assert_eq!(
            c.iter().filter(|&&b| b == b'\n').count(),
            256,
            "[cfg-22] expected exactly 256 lines"
        );
    }
}
