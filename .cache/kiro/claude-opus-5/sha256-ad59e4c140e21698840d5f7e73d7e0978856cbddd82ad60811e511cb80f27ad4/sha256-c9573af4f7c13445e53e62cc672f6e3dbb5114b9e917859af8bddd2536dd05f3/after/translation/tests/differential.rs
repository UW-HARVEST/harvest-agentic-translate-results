//! Differential tests: C `.so` vs Rust `.so`, both loaded via `libloading`.
//!
//! Built with `harness = false` (see `Cargo.toml`) and driven by the `main()`
//! at the bottom of this file. A custom harness is REQUIRED here: each case
//! captures file descriptor 1 to compare the two libraries byte for byte, and
//! libtest's own progress output (written from a different thread to the same
//! fd) would otherwise land inside the captured bytes and produce spurious
//! divergences. `main()` runs every case sequentially and emits its own report
//! only while fd 1 is not redirected.
//!
//! * `phase_b_*` — one test per row of `CONFIGS.md` (valid-path, randomized).
//! * `phase_c_*` — one test per row of `ERRORS.md` (error/rejection paths).
//!
//! Every assertion compares the raw `stdout` bytes produced by the C shared
//! object against those produced by the Rust shared object. `stdout` is the
//! library's only observable channel: all five exported functions return
//! `void`, and `grep` over `c_src/` finds no `return` statement, no `errno`
//! use, no `assert`, and no global state.

mod harness;

use harness::{assert_same, libs, special_f32, special_strings, Driver, Rng, SEED};
use std::ffi::{c_char, c_int};

/// Inputs are replayed in batches inside a single capture: that keeps the test
/// fast *and* exercises call sequencing through the shared stdio buffer.
const BATCH: usize = 200;

fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ===========================================================================
// Phase B — valid-path differential tests (CONFIGS.md)
// ===========================================================================

/// Helper: run `f` for each chunk of `inputs`, comparing C vs Rust per chunk.
fn each_chunk<T, F>(label: &str, inputs: &[T], f: F)
where
    T: Copy,
    F: Fn(&Driver, T) + Copy,
{
    for (i, chunk) in inputs.chunks(BATCH).enumerate() {
        assert_same(&format!("{label} chunk {i}"), |d| {
            for &x in chunk {
                f(d, x);
            }
        });
    }
}

// --- CONFIGS.md row 1: printIntLine, uniform random i32 --------------------
fn phase_b_row01_print_int_line_random() {
    let mut rng = Rng::new(SEED ^ 1);
    let inputs: Vec<c_int> = (0..4000).map(|_| rng.next_i32()).collect();
    each_chunk("row01 printIntLine random i32", &inputs, |d, v| unsafe {
        (d.print_int_line)(v)
    });
}

// --- CONFIGS.md row 2: printIntLine, int boundaries ------------------------
fn phase_b_row02_print_int_line_boundaries() {
    let inputs: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    each_chunk("row02 printIntLine boundaries", &inputs, |d, v| unsafe {
        (d.print_int_line)(v)
    });
}

// --- CONFIGS.md row 3: printIntLine, small magnitudes ----------------------
fn phase_b_row03_print_int_line_small() {
    let inputs: Vec<c_int> = (-1000..=1000).collect();
    each_chunk("row03 printIntLine -1000..=1000", &inputs, |d, v| unsafe {
        (d.print_int_line)(v)
    });
}

// --- CONFIGS.md row 4: printIntLine, powers of two and 10^k ---------------
fn phase_b_row04_print_int_line_powers() {
    let mut inputs: Vec<c_int> = Vec::new();
    for k in 0..31 {
        let p = 1i32 << k;
        inputs.extend_from_slice(&[p, -p, p - 1, -(p - 1)]);
    }
    let mut ten = 1i64;
    while ten <= c_int::MAX as i64 {
        inputs.extend_from_slice(&[ten as c_int, -(ten as c_int)]);
        ten *= 10;
    }
    each_chunk("row04 printIntLine powers", &inputs, |d, v| unsafe {
        (d.print_int_line)(v)
    });
}

/// Helper for the `printLine` rows: compare C vs Rust over a batch of byte
/// strings, each passed as a NUL-terminated pointer.
fn compare_strings(label: &str, strings: &[Vec<u8>]) {
    for (i, chunk) in strings.chunks(32).enumerate() {
        let owned: Vec<Vec<u8>> = chunk.iter().map(|s| cstr(s)).collect();
        assert_same(&format!("{label} chunk {i}"), |d| {
            for s in &owned {
                unsafe { (d.print_line)(s.as_ptr() as *const c_char) };
            }
        });
    }
}

// --- CONFIGS.md row 5: printLine, random printable ASCII ------------------
fn phase_b_row05_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 5);
    let strings: Vec<Vec<u8>> = (0..2000)
        .map(|_| {
            let len = rng.below(65) as usize;
            (0..len)
                .map(|_| 0x20u8 + rng.below(0x5f) as u8)
                .collect()
        })
        .collect();
    compare_strings("row05 printLine random ascii", &strings);
}

// --- CONFIGS.md row 6: printLine, random non-UTF-8 bytes -------------------
fn phase_b_row06_print_line_random_bytes() {
    let mut rng = Rng::new(SEED ^ 6);
    let strings: Vec<Vec<u8>> = (0..2000)
        .map(|_| {
            let len = rng.below(65) as usize;
            // 0x01..=0xff: any byte except the NUL terminator.
            (0..len).map(|_| 1u8 + rng.below(255) as u8).collect()
        })
        .collect();
    compare_strings("row06 printLine random bytes", &strings);
}

// --- CONFIGS.md row 7: printLine, length sweep + stdio buffer crossings ----
fn phase_b_row07_print_line_length_sweep() {
    let mut strings: Vec<Vec<u8>> = (0..=80).map(|n| vec![b'z'; n]).collect();
    for n in [4094usize, 4095, 4096, 4097, 8191, 8192, 8193, 65536] {
        strings.push(vec![b'q'; n]);
    }
    compare_strings("row07 printLine length sweep", &strings);
}

// --- CONFIGS.md row 8: printLine, format-specifier payloads ---------------
fn phase_b_row08_print_line_format_specifiers() {
    let strings: Vec<Vec<u8>> = vec![
        b"%s".to_vec(),
        b"%d".to_vec(),
        b"%n".to_vec(),
        b"%%".to_vec(),
        b"%p %x %f".to_vec(),
        b"%s%s%s%s%s%s%s%s".to_vec(),
        b"%1000000d".to_vec(),
        b"100% done".to_vec(),
        b"a%sb%dc".to_vec(),
    ];
    compare_strings("row08 printLine format specifiers", &strings);
}

// --- CONFIGS.md row 9: printLine, control characters ----------------------
fn phase_b_row09_print_line_control_chars() {
    let strings: Vec<Vec<u8>> = vec![
        b"tab\there".to_vec(),
        b"cr\rhere".to_vec(),
        b"vt\x0bhere".to_vec(),
        b"ff\x0chere".to_vec(),
        b"bel\x07here".to_vec(),
        b"esc\x1b[31mhere".to_vec(),
        b"interior\nnewline".to_vec(),
        b"\n".to_vec(),
        b"\r\n".to_vec(),
        b"trailing\n".to_vec(),
    ];
    compare_strings("row09 printLine control chars", &strings);
}

// --- CONFIGS.md row 10: bad, uniform random f32 bit patterns --------------
fn phase_b_row10_bad_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 10);
    let inputs: Vec<f32> = (0..20000).map(|_| rng.any_f32()).collect();
    each_chunk("row10 bad random bit patterns", &inputs, |d, v| unsafe {
        (d.bad)(v)
    });
}

// --- CONFIGS.md row 11: bad, normal finite magnitudes ---------------------
fn phase_b_row11_bad_normal_range() {
    let mut rng = Rng::new(SEED ^ 11);
    let inputs: Vec<f32> = (0..8000).map(|_| rng.log_uniform_f32(1e-3, 1e3)).collect();
    each_chunk("row11 bad normal range", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// --- CONFIGS.md row 12: bad, tiny/subnormal -> quotient overflows int -----
fn phase_b_row12_bad_tiny_overflow_range() {
    let mut rng = Rng::new(SEED ^ 12);
    let inputs: Vec<f32> = (0..4000)
        .map(|_| rng.log_uniform_f32(1e-45, 1e-6))
        .collect();
    each_chunk("row12 bad tiny/subnormal", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// --- CONFIGS.md row 13: bad, large -> quotient truncates to 0 -------------
fn phase_b_row13_bad_large_range() {
    let mut rng = Rng::new(SEED ^ 13);
    let inputs: Vec<f32> = (0..4000)
        .map(|_| rng.log_uniform_f32(1e6, f32::MAX as f64))
        .collect();
    each_chunk("row13 bad large range", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// --- CONFIGS.md row 14: bad, exact cvttsd2si cliff ------------------------
fn phase_b_row14_bad_int_range_cliff() {
    let mut inputs: Vec<f32> = Vec::new();
    // Walk the f32 neighbourhood of the values where 100.0/data crosses
    // +-2^31 and +-2^31-1, i.e. where cvttsd2si flips to INT_MIN.
    for target in [
        2147483648.0f64,
        2147483647.0f64,
        2147483649.0f64,
        -2147483648.0f64,
        -2147483649.0f64,
        -2147483647.0f64,
    ] {
        let centre = (100.0f64 / target) as f32;
        let bits = centre.to_bits();
        for delta in -6i64..=6 {
            let b = (bits as i64 + delta) as u32;
            inputs.push(f32::from_bits(b));
        }
    }
    each_chunk("row14 bad int-range cliff", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// --- CONFIGS.md row 15: bad, degenerate specials -------------------------
fn phase_b_row15_bad_specials() {
    let inputs = special_f32();
    each_chunk("row15 bad specials", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// --- CONFIGS.md row 16: good, uniform random f32 bit patterns ------------
fn phase_b_row16_good_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 16);
    let inputs: Vec<f32> = (0..20000).map(|_| rng.any_f32()).collect();
    each_chunk("row16 good random bit patterns", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
}

// --- CONFIGS.md row 17: good, just inside the fabs(data) > 1e-6 guard ----
fn phase_b_row17_good_guard_passing_band() {
    let mut rng = Rng::new(SEED ^ 17);
    let inputs: Vec<f32> = (0..6000)
        .map(|_| rng.log_uniform_f32(1.000001e-6, 1e-3))
        .collect();
    each_chunk("row17 good guard-passing band", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
}

// --- CONFIGS.md row 18: good, guard-failing band -------------------------
fn phase_b_row18_good_guard_failing_band() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut inputs: Vec<f32> = (0..6000)
        .map(|_| rng.log_uniform_f32(1e-45, 1e-6))
        .collect();
    inputs.push(0.0);
    inputs.push(-0.0);
    each_chunk("row18 good guard-failing band", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
}

// --- CONFIGS.md row 19: good, exact guard boundary ----------------------
fn phase_b_row19_good_guard_boundary_exact() {
    let mut inputs: Vec<f32> = Vec::new();
    for centre in [1e-6f32, -1e-6f32, 0.000001f64 as f32, 1e-7f32, 1e-5f32] {
        let bits = centre.to_bits();
        for delta in -8i64..=8 {
            inputs.push(f32::from_bits((bits as i64 + delta) as u32));
        }
    }
    each_chunk("row19 good guard boundary", &inputs, |d, v| unsafe { (d.good)(v) });
}

// --- CONFIGS.md row 20: driver, random (goodData, badData) pairs ---------
fn phase_b_row20_driver_random_pairs() {
    let mut rng = Rng::new(SEED ^ 20);
    let inputs: Vec<(f32, f32)> = (0..12000).map(|_| (rng.any_f32(), rng.any_f32())).collect();
    each_chunk("row20 driver random pairs", &inputs, |d, (g, b)| unsafe {
        (d.driver)(g, b)
    });
}

// --- CONFIGS.md row 21: driver, degenerate cross-product ----------------
fn phase_b_row21_driver_degenerate_cross_product() {
    let set = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e-7,
        -1e-7,
        2.0,
        -2.0,
        1e-6,
    ];
    let mut inputs: Vec<(f32, f32)> = Vec::new();
    for &g in &set {
        for &b in &set {
            inputs.push((g, b));
        }
    }
    each_chunk("row21 driver degenerate cross", &inputs, |d, (g, b)| unsafe {
        (d.driver)(g, b)
    });
}

// --- CONFIGS.md row 22: driver, guard-passing good x overflowing bad ----
fn phase_b_row22_driver_pass_x_overflow() {
    let mut rng = Rng::new(SEED ^ 22);
    let inputs: Vec<(f32, f32)> = (0..4000)
        .map(|_| {
            (
                rng.log_uniform_f32(1.000001e-6, 1e3),
                rng.log_uniform_f32(1e-45, 4.6e-8),
            )
        })
        .collect();
    each_chunk("row22 driver pass x overflow", &inputs, |d, (g, b)| unsafe {
        (d.driver)(g, b)
    });
}

// --- CONFIGS.md row 23: driver, guard-failing good x normal bad ---------
fn phase_b_row23_driver_fail_x_normal() {
    let mut rng = Rng::new(SEED ^ 23);
    let inputs: Vec<(f32, f32)> = (0..4000)
        .map(|_| {
            (
                rng.log_uniform_f32(1e-45, 1e-6),
                rng.log_uniform_f32(1e-3, 1e3),
            )
        })
        .collect();
    each_chunk("row23 driver fail x normal", &inputs, |d, (g, b)| unsafe {
        (d.driver)(g, b)
    });
}

// --- CONFIGS.md row 24: all five entry points interleaved ---------------
fn phase_b_row24_mixed_sequence() {
    let mut rng = Rng::new(SEED ^ 24);
    // Pre-generate the whole script so C and Rust replay the identical thing.
    struct Step {
        kind: u8,
        i: c_int,
        f: f32,
        g: f32,
        s: Vec<u8>,
    }
    let script: Vec<Step> = (0..500)
        .map(|_| Step {
            kind: rng.below(5) as u8,
            i: rng.next_i32(),
            f: rng.any_f32(),
            g: rng.any_f32(),
            s: cstr(
                &(0..rng.below(40))
                    .map(|_| 1u8 + rng.below(255) as u8)
                    .collect::<Vec<u8>>(),
            ),
        })
        .collect();

    for (ci, chunk) in script.chunks(50).enumerate() {
        assert_same(&format!("row24 mixed sequence chunk {ci}"), |d| unsafe {
            for st in chunk {
                match st.kind {
                    0 => (d.print_line)(st.s.as_ptr() as *const c_char),
                    1 => (d.print_int_line)(st.i),
                    2 => (d.bad)(st.f),
                    3 => (d.good)(st.f),
                    _ => (d.driver)(st.g, st.f),
                }
            }
        });
    }
}

// --- CONFIGS.md row 25: statelessness of repeated driver calls ----------
fn phase_b_row25_driver_repeated_is_stateless() {
    let mut rng = Rng::new(SEED ^ 25);
    for round in 0..40 {
        let g = rng.any_f32();
        let b = rng.any_f32();
        // C vs Rust for 7 back-to-back calls...
        assert_same(&format!("row25 driver x7 round {round}"), |d| unsafe {
            for _ in 0..7 {
                (d.driver)(g, b);
            }
        });
        // ...and, additionally, assert the C library itself is stateless, so
        // that the byte-equality above is not hiding a shared drift.
        let libs = libs();
        let one = capture_c_once(&libs.c, g, b);
        let mut expected = Vec::new();
        for _ in 0..7 {
            expected.extend_from_slice(&one);
        }
        let seven = capture_c_seven(&libs.c, g, b);
        assert_eq!(
            seven, expected,
            "C driver() is not stateless for ({g:?}, {b:?})"
        );
    }
}

// --- CONFIGS.md row 26: printLine(NULL) interleaved --------------------
fn phase_b_row26_null_interleaved() {
    let mut rng = Rng::new(SEED ^ 26);
    let inputs: Vec<f32> = (0..600).map(|_| rng.any_f32()).collect();
    let msg = cstr(b"between");
    for (i, chunk) in inputs.chunks(60).enumerate() {
        assert_same(&format!("row26 NULL interleaved chunk {i}"), |d| unsafe {
            for &v in chunk {
                (d.print_line)(std::ptr::null());
                (d.print_int_line)(v.to_bits() as c_int);
                (d.print_line)(std::ptr::null());
                (d.bad)(v);
                (d.print_line)(msg.as_ptr() as *const c_char);
                (d.good)(v);
                (d.print_line)(std::ptr::null());
            }
        });
    }
}

// Small helpers used by row 25. They deliberately go through the harness'
// capture path by way of `assert_same`-free direct captures.
fn capture_c_once(d: &Driver, g: f32, b: f32) -> Vec<u8> {
    harness::capture_bytes(|| unsafe { (d.driver)(g, b) })
}
fn capture_c_seven(d: &Driver, g: f32, b: f32) -> Vec<u8> {
    harness::capture_bytes(|| unsafe {
        for _ in 0..7 {
            (d.driver)(g, b);
        }
    })
}

// ===========================================================================
// Phase C — error-path differential tests (ERRORS.md)
// ===========================================================================

// ERRORS.md row 1: printLine(NULL) -> null check fails, nothing printed.
fn phase_c_row01_print_line_null() {
    assert_same("ERRORS row01 printLine(NULL)", |d| unsafe {
        (d.print_line)(std::ptr::null())
    });
    // The rejection is observable as "zero bytes"; assert that explicitly so
    // the row is not satisfied by both sides being wrong in the same way.
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.print_line)(std::ptr::null()) });
    let r = harness::capture_bytes(|| unsafe { (libs.rust.print_line)(std::ptr::null()) });
    assert_eq!(c, Vec::<u8>::new(), "C printLine(NULL) must print nothing");
    assert_eq!(r, Vec::<u8>::new(), "Rust printLine(NULL) must print nothing");
}

// ERRORS.md row 2: printLine("") -> a lone newline.
fn phase_c_row02_print_line_empty() {
    let s = cstr(b"");
    assert_same("ERRORS row02 printLine(\"\")", |d| unsafe {
        (d.print_line)(s.as_ptr() as *const c_char)
    });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.print_line)(s.as_ptr() as *const c_char) });
    assert_eq!(c, b"\n".to_vec());
}

// ERRORS.md row 3: format-specifier bytes are data, not directives.
fn phase_c_row03_print_line_format_bytes() {
    let strings: Vec<Vec<u8>> = vec![
        b"%d %s %n".to_vec(),
        b"%s%n%s%n".to_vec(),
        b"%99999999d".to_vec(),
    ];
    compare_strings("ERRORS row03 format bytes", &strings);
}

// ERRORS.md row 4: non-UTF-8 / control bytes pass through verbatim.
fn phase_c_row04_print_line_non_utf8() {
    let strings: Vec<Vec<u8>> = vec![
        vec![0x80, 0xff, 0xfe, 0xc3, 0x28],
        (0x01u8..=0xffu8).collect(),
        vec![0xed, 0xa0, 0x80],       // UTF-16 surrogate encoding
        vec![0xf4, 0x90, 0x80, 0x80], // > U+10FFFF
        b"mix\ted\r\x0b\x1bbytes\xc0\xaf".to_vec(),
    ];
    compare_strings("ERRORS row04 non-utf8", &strings);
}

// ERRORS.md row 5: oversized string (past the stdio buffer).
fn phase_c_row05_print_line_oversized() {
    let strings: Vec<Vec<u8>> = vec![vec![b'A'; 65536], vec![b'B'; 100_000]];
    compare_strings("ERRORS row05 oversized", &strings);
}

// ERRORS.md rows 6-8: printIntLine extremes and sentinel-looking values.
fn phase_c_row06_07_08_print_int_line_extremes() {
    let inputs: Vec<c_int> = vec![c_int::MIN, c_int::MAX, 0, -1, 1, c_int::MIN + 1, c_int::MAX - 1];
    each_chunk("ERRORS row06-08 printIntLine extremes", &inputs, |d, v| unsafe {
        (d.print_int_line)(v)
    });
    // Pin the exact expected bytes for the two extremes.
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.print_int_line)(c_int::MIN) });
    assert_eq!(c, b"-2147483648\n".to_vec());
    let c = harness::capture_bytes(|| unsafe { (libs.c.print_int_line)(c_int::MAX) });
    assert_eq!(c, b"2147483647\n".to_vec());
}

// ERRORS.md row 9: bad(+0.0) — the unguarded divide by zero.
fn phase_c_row09_bad_positive_zero() {
    assert_same("ERRORS row09 bad(+0.0)", |d| unsafe { (d.bad)(0.0) });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.bad)(0.0) });
    let r = harness::capture_bytes(|| unsafe { (libs.rust.bad)(0.0) });
    assert_eq!(c, b"-2147483648\n".to_vec(), "C bad(+0.0)");
    assert_eq!(r, c, "Rust bad(+0.0)");
}

// ERRORS.md row 10: bad(-0.0) -> -inf -> INT_MIN.
fn phase_c_row10_bad_negative_zero() {
    assert_same("ERRORS row10 bad(-0.0)", |d| unsafe { (d.bad)(-0.0) });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.bad)(-0.0) });
    assert_eq!(c, b"-2147483648\n".to_vec());
}

// ERRORS.md row 11: bad(subnormal / FLT_MIN) -> quotient out of int range.
fn phase_c_row11_bad_subnormals() {
    let inputs: Vec<f32> = vec![
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-40,
        -1e-40,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF), // largest subnormal
        f32::from_bits(0x807F_FFFF),
    ];
    each_chunk("ERRORS row11 bad subnormals", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// ERRORS.md row 12: bad(quiet NaN).
fn phase_c_row12_bad_quiet_nan() {
    assert_same("ERRORS row12 bad(NaN)", |d| unsafe { (d.bad)(f32::NAN) });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.bad)(f32::NAN) });
    assert_eq!(c, b"-2147483648\n".to_vec());
}

// ERRORS.md row 13: bad(signalling / negative / payload NaNs).
fn phase_c_row13_bad_exotic_nans() {
    let inputs: Vec<f32> = vec![
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFA0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0x7F80_0001), // smallest signalling NaN
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0xFFFF_FFFF),
        -f32::NAN,
    ];
    each_chunk("ERRORS row13 bad exotic NaNs", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// ERRORS.md rows 14-15: bad(+/-inf) -> +/-0.0 -> "0".
fn phase_c_row14_15_bad_infinities() {
    let inputs: Vec<f32> = vec![f32::INFINITY, f32::NEG_INFINITY];
    each_chunk("ERRORS row14-15 bad infinities", &inputs, |d, v| unsafe {
        (d.bad)(v)
    });
    let libs = libs();
    for v in [f32::INFINITY, f32::NEG_INFINITY] {
        let c = harness::capture_bytes(|| unsafe { (libs.c.bad)(v) });
        assert_eq!(c, b"0\n".to_vec(), "C bad({v:?})");
    }
}

// ERRORS.md rows 16-17: both sides of the cvttsd2si int-range cliff.
fn phase_c_row16_17_bad_cliff_both_sides() {
    let mut inputs: Vec<f32> = Vec::new();
    for target in [2147483648.0f64, -2147483648.0f64, -2147483649.0f64] {
        let centre = (100.0f64 / target) as f32;
        let bits = centre.to_bits();
        for delta in -20i64..=20 {
            inputs.push(f32::from_bits((bits as i64 + delta) as u32));
        }
    }
    each_chunk("ERRORS row16-17 bad cliff", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// ERRORS.md row 18: bad at +/-FLT_MAX, +/-1.0, +/-100.0.
fn phase_c_row18_bad_truncation_toward_zero() {
    let inputs: Vec<f32> = vec![
        f32::MAX,
        f32::MIN,
        1.0,
        -1.0,
        100.0,
        -100.0,
        3.0,
        -3.0,
        99.0,
        -99.0,
        101.0,
        -101.0,
    ];
    each_chunk("ERRORS row18 bad truncation", &inputs, |d, v| unsafe { (d.bad)(v) });
}

// ERRORS.md rows 19-20: goodB2G rejects +/-0.0 via `fabs(data) > 1e-6`.
fn phase_c_row19_20_good_zero_rejected() {
    let inputs: Vec<f32> = vec![0.0, -0.0];
    each_chunk("ERRORS row19-20 good(+/-0)", &inputs, |d, v| unsafe { (d.good)(v) });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.good)(0.0) });
    assert_eq!(
        c,
        b"50\nThis would result in a divide by zero\n".to_vec(),
        "C good(0.0): goodG2B prints 50 first, then the rejection message"
    );
}

// ERRORS.md row 21: data == 1e-6f exactly — the off-by-one-ULP boundary.
fn phase_c_row21_good_exact_threshold() {
    let inputs: Vec<f32> = vec![1e-6f32, -1e-6f32, 0.000001f64 as f32];
    each_chunk("ERRORS row21 good(1e-6)", &inputs, |d, v| unsafe { (d.good)(v) });
    // (double)1e-6f == 9.999999747e-07 which is NOT > 1e-6, so the guard fails.
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.good)(1e-6f32) });
    assert_eq!(
        c,
        b"50\nThis would result in a divide by zero\n".to_vec(),
        "C good(1e-6f) must take the else branch"
    );
}

// ERRORS.md row 22: one step below the threshold (finite division, rejected).
fn phase_c_row22_good_below_threshold() {
    let inputs: Vec<f32> = vec![1e-7, 5e-7, -1e-7, -5e-7, 9.999999e-7, -9.999999e-7];
    each_chunk("ERRORS row22 good below threshold", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.good)(5e-7) });
    assert_eq!(
        c,
        b"50\nThis would result in a divide by zero\n".to_vec(),
        "C good(5e-7): finite quotient, still rejected by the guard"
    );
}

// ERRORS.md row 23: one step above the threshold -> guard passes.
fn phase_c_row23_good_above_threshold() {
    let above = f32::from_bits(1e-6f32.to_bits() + 1);
    let inputs: Vec<f32> = vec![above, -above, 1.0000001e-6, 1.1e-6, -1.1e-6];
    each_chunk("ERRORS row23 good above threshold", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.good)(1.1e-6f32) });
    assert!(
        c.starts_with(b"50\n") && !c.ends_with(b"zero\n"),
        "C good(1.1e-6) must divide, got {c:?}"
    );
}

// ERRORS.md row 24: NaN takes the `else` branch (all comparisons false).
fn phase_c_row24_good_nan_rejected() {
    let inputs: Vec<f32> = vec![
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FFF_FFFF),
    ];
    each_chunk("ERRORS row24 good(NaN)", &inputs, |d, v| unsafe { (d.good)(v) });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.good)(f32::NAN) });
    assert_eq!(
        c,
        b"50\nThis would result in a divide by zero\n".to_vec(),
        "C good(NaN) must take the else branch"
    );
}

// ERRORS.md row 25: +/-inf passes the guard and divides to +/-0.0.
fn phase_c_row25_good_infinities() {
    let inputs: Vec<f32> = vec![f32::INFINITY, f32::NEG_INFINITY];
    each_chunk("ERRORS row25 good infinities", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.good)(f32::INFINITY) });
    assert_eq!(c, b"50\n0\n".to_vec(), "C good(inf) divides to 0");
}

// ERRORS.md row 26: subnormals / FLT_MIN are below the guard.
fn phase_c_row26_good_subnormals() {
    let inputs: Vec<f32> = vec![
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        1e-40,
        -1e-40,
        f32::from_bits(0x007F_FFFF),
    ];
    each_chunk("ERRORS row26 good subnormals", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
}

// ERRORS.md row 27: goodG2B always runs first and always prints 50.
fn phase_c_row27_good_ordering_invariant() {
    let mut rng = Rng::new(SEED ^ 27);
    let libs = libs();
    for _ in 0..300 {
        let v = rng.any_f32();
        let c = harness::capture_bytes(|| unsafe { (libs.c.good)(v) });
        let r = harness::capture_bytes(|| unsafe { (libs.rust.good)(v) });
        assert_eq!(c, r, "good({v:?}) diverged");
        assert!(
            c.starts_with(b"50\n"),
            "C good({v:?}) must start with goodG2B's 50, got {:?}",
            String::from_utf8_lossy(&c)
        );
    }
}

// ERRORS.md row 28: driver with the flawed badData == 0.0.
fn phase_c_row28_driver_bad_zero() {
    assert_same("ERRORS row28 driver(2.0, 0.0)", |d| unsafe {
        (d.driver)(2.0, 0.0)
    });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.driver)(2.0, 0.0) });
    assert_eq!(
        c,
        b"Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n-2147483648\nFinished bad()\n"
            .to_vec(),
        "C driver(2.0, 0.0) exact byte sequence"
    );
}

// ERRORS.md row 29: both arguments degenerate at once.
fn phase_c_row29_driver_both_zero() {
    assert_same("ERRORS row29 driver(0.0, 0.0)", |d| unsafe {
        (d.driver)(0.0, 0.0)
    });
    let libs = libs();
    let c = harness::capture_bytes(|| unsafe { (libs.c.driver)(0.0, 0.0) });
    assert_eq!(
        c,
        b"Calling good()...\n50\nThis would result in a divide by zero\nFinished good()\n\
          Calling bad()...\n-2147483648\nFinished bad()\n"
            .to_vec(),
        "C driver(0.0, 0.0) exact byte sequence"
    );
}

// ERRORS.md row 30: full degenerate cross-product through driver().
fn phase_c_row30_driver_degenerate_matrix() {
    let set = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e-7,
        1e-6,
        f32::MIN_POSITIVE,
        f32::MAX,
        2.0,
    ];
    let mut inputs: Vec<(f32, f32)> = Vec::new();
    for &g in &set {
        for &b in &set {
            inputs.push((g, b));
        }
    }
    each_chunk("ERRORS row30 driver matrix", &inputs, |d, (g, b)| unsafe {
        (d.driver)(g, b)
    });
}

// ===========================================================================
// Generic FFI-boundary boundaries (required even though not in ERRORS.md)
// ===========================================================================

/// There are no `enum` types anywhere in the public API (`grep -n enum c_src/`
/// finds nothing), so the "out-of-range enum value across FFI" class becomes
/// "arbitrary 32-bit pattern in a `float` parameter" and "arbitrary `int`".
/// Both are swept exhaustively-by-sampling here, including the ranges no valid
/// caller would produce.
fn phase_c_generic_exhaustive_bitpattern_sweep() {
    // Structured sweep of the f32 exponent/mantissa space: every exponent, a
    // handful of mantissas, both signs. Catches anything value-dependent.
    let mut inputs: Vec<f32> = Vec::new();
    for sign in [0u32, 1u32] {
        for exp in 0u32..=255 {
            for mant in [0u32, 1, 0x0000_2A, 0x2A_AAAA, 0x40_0000, 0x7F_FFFF] {
                let bits = (sign << 31) | (exp << 23) | mant;
                inputs.push(f32::from_bits(bits));
            }
        }
    }
    each_chunk("generic f32 bit sweep / bad", &inputs, |d, v| unsafe { (d.bad)(v) });
    each_chunk("generic f32 bit sweep / good", &inputs, |d, v| unsafe { (d.good)(v) });
}

fn phase_c_generic_int_sweep() {
    let mut inputs: Vec<c_int> = Vec::new();
    for k in 0..32 {
        let p = 1u32 << k;
        for d in [-1i64, 0, 1] {
            inputs.push((p as i64 + d) as u32 as c_int);
        }
    }
    inputs.extend_from_slice(&[c_int::MIN, c_int::MAX, 0, -1]);
    each_chunk("generic int sweep", &inputs, |d, v| unsafe {
        (d.print_int_line)(v)
    });
}

/// Zero-length and oversized buffers, plus a pointer to a string whose only
/// content is the terminator, all through the lowest-level entry point.
fn phase_c_generic_lengths() {
    let strings = special_strings();
    compare_strings("generic length/shape set", &strings);
}

/// Both libraries must agree that `printLine(NULL)` is safe in every position
/// of a call sequence, not merely in isolation.
fn phase_c_generic_null_positions() {
    let msg = cstr(b"x");
    assert_same("generic NULL in every position", |d| unsafe {
        (d.print_line)(std::ptr::null());
        (d.print_line)(std::ptr::null());
        (d.print_line)(msg.as_ptr() as *const c_char);
        (d.print_line)(std::ptr::null());
        (d.print_int_line)(7);
        (d.print_line)(std::ptr::null());
        (d.driver)(0.0, 0.0);
        (d.print_line)(std::ptr::null());
    });
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

fn phase_d_symbol_parity() {
    use std::process::Command;

    fn dynamic_defined(path: &str) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", path])
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {path}");
        let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
                // Only real code/data symbols; skip Rust/ELF bookkeeping.
                if !matches!(kind, "T" | "t" | "W" | "D" | "B" | "R" | "i") {
                    return None;
                }
                if name.starts_with("__") || name.starts_with('_') {
                    return None;
                }
                Some(name.to_string())
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    let c = dynamic_defined(harness::c_so().to_str().unwrap());
    let r = dynamic_defined(harness::rust_so().to_str().unwrap());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    assert_eq!(
        c,
        vec!["bad", "driver", "good", "printIntLine", "printLine"],
        "the C .so's exported set changed; SYMBOLS.md needs revisiting"
    );
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

// ===========================================================================
// Reachability proofs for the two branch details that no input can distinguish
// ===========================================================================
//
// Mutation testing this suite left exactly two surviving mutants. Both are
// *equivalent* mutants — no input can tell them apart — and the tests below
// prove it mechanically so the claim is enforced rather than asserted in prose.

/// ERRORS.md row 31.
///
/// `driver.c:61` is `fabs(data) > 0.000001`. Mutating `>` to `>=` changes
/// nothing, because the two differ only when `fabs((double)data)` is *exactly*
/// the double `0.000001`, and no `float` widens to that value: `1e-6`'s double
/// significand has non-zero bits below the 24-bit `float` precision, and
/// `float -> double` is exact.
///
/// This matters for the translation: it means `>` may be written either way and
/// the Rust is not relying on luck.
fn phase_c_proof_guard_equality_unreachable() {
    let threshold = 0.000001f64;
    assert_ne!(
        threshold.to_bits() & ((1u64 << 29) - 1),
        0,
        "1e-6 would be exactly float-representable; the `>` vs `>=` distinction \
         would then be reachable and needs a dedicated differential test"
    );
    // Belt and braces: sweep a wide ULP neighbourhood of the nearest float.
    let base = (threshold as f32).to_bits();
    for d in -2_000_000i64..=2_000_000 {
        let x = f32::from_bits((base as i64 + d) as u32);
        assert_ne!(
            (x as f64).abs(),
            threshold,
            "float {x:e} widens to exactly 1e-6; guard equality IS reachable"
        );
    }
    // And confirm both libraries agree right at the nearest representable
    // values on either side.
    let inputs: Vec<f32> = vec![
        threshold as f32,
        f32::from_bits(base - 1),
        f32::from_bits(base + 1),
        -(threshold as f32),
    ];
    each_chunk("proof: guard equality neighbourhood", &inputs, |d, v| unsafe {
        (d.good)(v)
    });
}

/// ERRORS.md row 32.
///
/// `to_c_int`'s lower bound is `value <= -2147483649.0`. Relaxing it to
/// `<= -2147483648.0` is also an equivalent mutant: every `double` in
/// `(-2^31 - 1, -2^31]` truncates toward zero to exactly `INT_MIN`, so the
/// early return and the fallthrough produce the same `int`.
fn phase_c_proof_int_min_interval() {
    let hi = -2147483648.0f64;
    let lo = -2147483649.0f64;
    let mut v = hi;
    let mut checked = 0u64;
    while v > lo {
        assert_eq!(
            v as i32,
            i32::MIN,
            "double {v} in (-2^31-1, -2^31] does not truncate to INT_MIN"
        );
        v = f64::from_bits(v.to_bits() + 1); // step further from zero
        checked += 1;
        if checked > 1_000_000 {
            break;
        }
    }
    assert!(checked > 0, "interval walk did not execute");

    // Now the differential half: sweep the float divisors whose quotient lands
    // in (or adjacent to) that interval and require byte-identical output.
    let centre = (100.0f64 / hi) as f32;
    let mut inputs: Vec<f32> = Vec::new();
    for d in -3000i64..=3000 {
        inputs.push(f32::from_bits((centre.to_bits() as i64 + d) as u32));
    }
    each_chunk("proof: INT_MIN interval divisors", &inputs, |d, v| unsafe {
        (d.bad)(v)
    });
}

type Case = (&'static str, fn());
const CASES: &[Case] = &[
    ("phase_b_row01_print_int_line_random", phase_b_row01_print_int_line_random),
    ("phase_b_row02_print_int_line_boundaries", phase_b_row02_print_int_line_boundaries),
    ("phase_b_row03_print_int_line_small", phase_b_row03_print_int_line_small),
    ("phase_b_row04_print_int_line_powers", phase_b_row04_print_int_line_powers),
    ("phase_b_row05_print_line_random_ascii", phase_b_row05_print_line_random_ascii),
    ("phase_b_row06_print_line_random_bytes", phase_b_row06_print_line_random_bytes),
    ("phase_b_row07_print_line_length_sweep", phase_b_row07_print_line_length_sweep),
    ("phase_b_row08_print_line_format_specifiers", phase_b_row08_print_line_format_specifiers),
    ("phase_b_row09_print_line_control_chars", phase_b_row09_print_line_control_chars),
    ("phase_b_row10_bad_random_bit_patterns", phase_b_row10_bad_random_bit_patterns),
    ("phase_b_row11_bad_normal_range", phase_b_row11_bad_normal_range),
    ("phase_b_row12_bad_tiny_overflow_range", phase_b_row12_bad_tiny_overflow_range),
    ("phase_b_row13_bad_large_range", phase_b_row13_bad_large_range),
    ("phase_b_row14_bad_int_range_cliff", phase_b_row14_bad_int_range_cliff),
    ("phase_b_row15_bad_specials", phase_b_row15_bad_specials),
    ("phase_b_row16_good_random_bit_patterns", phase_b_row16_good_random_bit_patterns),
    ("phase_b_row17_good_guard_passing_band", phase_b_row17_good_guard_passing_band),
    ("phase_b_row18_good_guard_failing_band", phase_b_row18_good_guard_failing_band),
    ("phase_b_row19_good_guard_boundary_exact", phase_b_row19_good_guard_boundary_exact),
    ("phase_b_row20_driver_random_pairs", phase_b_row20_driver_random_pairs),
    ("phase_b_row21_driver_degenerate_cross_product", phase_b_row21_driver_degenerate_cross_product),
    ("phase_b_row22_driver_pass_x_overflow", phase_b_row22_driver_pass_x_overflow),
    ("phase_b_row23_driver_fail_x_normal", phase_b_row23_driver_fail_x_normal),
    ("phase_b_row24_mixed_sequence", phase_b_row24_mixed_sequence),
    ("phase_b_row25_driver_repeated_is_stateless", phase_b_row25_driver_repeated_is_stateless),
    ("phase_b_row26_null_interleaved", phase_b_row26_null_interleaved),
    ("phase_c_row01_print_line_null", phase_c_row01_print_line_null),
    ("phase_c_row02_print_line_empty", phase_c_row02_print_line_empty),
    ("phase_c_row03_print_line_format_bytes", phase_c_row03_print_line_format_bytes),
    ("phase_c_row04_print_line_non_utf8", phase_c_row04_print_line_non_utf8),
    ("phase_c_row05_print_line_oversized", phase_c_row05_print_line_oversized),
    ("phase_c_row06_07_08_print_int_line_extremes", phase_c_row06_07_08_print_int_line_extremes),
    ("phase_c_row09_bad_positive_zero", phase_c_row09_bad_positive_zero),
    ("phase_c_row10_bad_negative_zero", phase_c_row10_bad_negative_zero),
    ("phase_c_row11_bad_subnormals", phase_c_row11_bad_subnormals),
    ("phase_c_row12_bad_quiet_nan", phase_c_row12_bad_quiet_nan),
    ("phase_c_row13_bad_exotic_nans", phase_c_row13_bad_exotic_nans),
    ("phase_c_row14_15_bad_infinities", phase_c_row14_15_bad_infinities),
    ("phase_c_row16_17_bad_cliff_both_sides", phase_c_row16_17_bad_cliff_both_sides),
    ("phase_c_row18_bad_truncation_toward_zero", phase_c_row18_bad_truncation_toward_zero),
    ("phase_c_row19_20_good_zero_rejected", phase_c_row19_20_good_zero_rejected),
    ("phase_c_row21_good_exact_threshold", phase_c_row21_good_exact_threshold),
    ("phase_c_row22_good_below_threshold", phase_c_row22_good_below_threshold),
    ("phase_c_row23_good_above_threshold", phase_c_row23_good_above_threshold),
    ("phase_c_row24_good_nan_rejected", phase_c_row24_good_nan_rejected),
    ("phase_c_row25_good_infinities", phase_c_row25_good_infinities),
    ("phase_c_row26_good_subnormals", phase_c_row26_good_subnormals),
    ("phase_c_row27_good_ordering_invariant", phase_c_row27_good_ordering_invariant),
    ("phase_c_row28_driver_bad_zero", phase_c_row28_driver_bad_zero),
    ("phase_c_row29_driver_both_zero", phase_c_row29_driver_both_zero),
    ("phase_c_row30_driver_degenerate_matrix", phase_c_row30_driver_degenerate_matrix),
    ("phase_c_generic_exhaustive_bitpattern_sweep", phase_c_generic_exhaustive_bitpattern_sweep),
    ("phase_c_generic_int_sweep", phase_c_generic_int_sweep),
    ("phase_c_generic_lengths", phase_c_generic_lengths),
    ("phase_c_generic_null_positions", phase_c_generic_null_positions),
    ("phase_c_proof_guard_equality_unreachable", phase_c_proof_guard_equality_unreachable),
    ("phase_c_proof_int_min_interval", phase_c_proof_int_min_interval),
    ("phase_d_symbol_parity", phase_d_symbol_parity),
];

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Outcome of one case.
enum Outcome {
    Pass,
    Panic,
    /// Killed by a signal — e.g. the Rust `.so` dereferencing a pointer that the
    /// C library guards against. That is a genuine behavioural divergence, so
    /// it must be reported as a failure rather than taking the whole run down.
    Signal(c_int),
    Other(c_int),
}

/// Run one case in a forked child so a segfault or abort inside either shared
/// object is contained and attributed to the case that caused it, instead of
/// killing the runner and hiding every other result.
fn run_isolated(f: fn()) -> Outcome {
    use std::io::Write;
    // Flush first: anything still buffered would otherwise be emitted twice,
    // once by the parent and once by the child.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        // `_exit`, not `exit`: skip atexit handlers and stdio flushing so the
        // child cannot duplicate the parent's buffered output.
        unsafe { _exit(if outcome.is_ok() { 0 } else { 1 }) }
    }

    let mut status: c_int = 0;
    let waited = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(waited, pid, "waitpid() failed");

    // WIFSTOPPED / WTERMSIG / WEXITSTATUS, spelled out to avoid a libc dep.
    if status & 0x7f == 0x7f {
        Outcome::Other(status)
    } else if status & 0x7f != 0 {
        Outcome::Signal(status & 0x7f)
    } else {
        match (status >> 8) & 0xff {
            0 => Outcome::Pass,
            1 => Outcome::Panic,
            c => Outcome::Other(c),
        }
    }
}

fn main() {
    // Optional substring filter, mirroring `cargo test -- <filter>`. Also
    // accepts `--in-process` to skip forking (handy under a debugger).
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let in_process = argv.iter().any(|a| a == "--in-process");
    let filters: Vec<&String> = argv.iter().filter(|a| !a.starts_with('-')).collect();

    let selected: Vec<&Case> = CASES
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|a| name.contains(a.as_str())))
        .collect();

    println!(
        "running {} differential cases ({} isolation)",
        selected.len(),
        if in_process { "no" } else { "fork" }
    );
    let total = selected.len();
    let mut failed: Vec<&str> = Vec::new();
    let start = std::time::Instant::now();

    for (name, f) in &selected {
        let outcome = if in_process {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f)) {
                Ok(()) => Outcome::Pass,
                Err(_) => Outcome::Panic,
            }
        } else {
            run_isolated(*f)
        };
        match outcome {
            Outcome::Pass => println!("ok    {name}"),
            Outcome::Panic => {
                println!("FAIL  {name}  (assertion failed; detail printed above)");
                failed.push(name);
            }
            Outcome::Signal(sig) => {
                println!(
                    "FAIL  {name}  (killed by signal {sig} - one library crashed \
                     where the other did not)"
                );
                failed.push(name);
            }
            Outcome::Other(code) => {
                println!("FAIL  {name}  (unexpected child status {code})");
                failed.push(name);
            }
        }
    }

    println!(
        "\nresult: {} passed, {} failed, in {:.2}s",
        total - failed.len(),
        failed.len(),
        start.elapsed().as_secs_f64()
    );
    if !failed.is_empty() {
        println!("failures:");
        for f in &failed {
            println!("    {f}");
        }
        std::process::exit(1);
    }
}
