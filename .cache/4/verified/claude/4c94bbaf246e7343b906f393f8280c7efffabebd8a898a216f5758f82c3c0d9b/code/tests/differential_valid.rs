//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every row drives BOTH `libdriver.so`
//! builds (C and Rust) through their exported C symbols and compares the raw
//! stdout bytes.  Rows use many randomized inputs from the relevant class
//! (fixed seed, see `common::SEED`) rather than one hand-picked value.
//!
//! The lowest-level entry points (`printLine`, `printIntLine`) are exercised
//! directly, then `bad` / `good`, then the composed `driver` wrapper, then a
//! mixed interleaved sequence.

#[macro_use]
mod common;

use common::*;
use std::os::raw::c_int;

fn main() {
    common::run(cases![
            // printLine — C1..C8
            c01_print_line_empty,
            c02_print_line_every_single_byte,
            c03_print_line_random_ascii,
            c04_print_line_random_bytes,
            c05_print_line_format_directives,
            c06_print_line_control_and_interior_nul,
            c07_print_line_long_strings,
            c08_print_line_many_calls_one_window,
            // printIntLine — C9..C12
            c09_print_int_line_small_boundaries,
            c10_print_int_line_extremes,
            c11_print_int_line_random_bit_patterns,
            c12_print_int_line_powers,
            // bad — C13..C22
            c13_bad_exact_quotients,
            c14_bad_truncating_positive,
            c15_bad_truncating_negative,
            c16_bad_one_and_two,
            c17_bad_division_special_cases,
            c18_bad_subnormals,
            c19_bad_extremes,
            c20_bad_cvt_range_boundary,
            c21_bad_random_bit_patterns,
            c22_bad_random_log_uniform,
            // good — C23..C29
            c23_good_guard_accepts,
            c24_good_guard_rejects,
            c25_good_nan,
            c26_good_guard_boundary,
            c27_good_accept_path_never_overflows,
            c28_good_infinities,
            c29_good_random_bit_patterns,
            // driver — C30..C35
            c30_driver_nominal,
            c31_driver_good_ok_bad_degenerate,
            c32_driver_good_rejected_bad_normal,
            c33_driver_both_degenerate,
            c34_driver_edge_cross_product,
            c35_driver_random_pairs,
            // composed — C36
            c36_interleaved_mixed_calls,
            // wide sweeps — C38, C39
            c38_bad_exhaustive_stride_sweep,
            c39_good_exhaustive_stride_sweep,
            // exhaustive sweep — C40 (env gated, see the function)
            c40_full_f32_sweep,
    ]);
}

// ===========================================================================
// printLine  (c_src/src/driver.c:30)
// ===========================================================================

/// CONFIGS.md row C1 — non-NULL empty string.
fn c01_print_line_empty() {
    compare_print_line("C1 empty string", &[Vec::new()]);
}

/// CONFIGS.md row C2 — every single-byte string 0x01..=0xFF, including bytes
/// that are not valid UTF-8.
fn c02_print_line_every_single_byte() {
    let v: Vec<Vec<u8>> = (1u8..=255).map(|b| vec![b]).collect();
    compare_print_line("C2 every single byte 0x01..0xFF", &v);
}

/// CONFIGS.md row C3 — random printable ASCII, length 1..=64.
fn c03_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 0x03);
    let mut v = Vec::new();
    for _ in 0..2048 {
        let len = 1 + rng.below(64) as usize;
        v.push(
            (0..len)
                .map(|_| 0x20u8 + rng.below(0x5f) as u8)
                .collect::<Vec<u8>>(),
        );
    }
    compare_print_line("C3 random printable ASCII", &v);
}

/// CONFIGS.md row C4 — random bytes over the full 0x01..=0xFF alphabet
/// (deliberately not valid UTF-8), length 1..=256.
fn c04_print_line_random_bytes() {
    let mut rng = Rng::new(SEED ^ 0x04);
    let mut v = Vec::new();
    for _ in 0..2048 {
        let len = 1 + rng.below(256) as usize;
        v.push(
            (0..len)
                .map(|_| 1u8 + rng.below(255) as u8)
                .collect::<Vec<u8>>(),
        );
    }
    compare_print_line("C4 random non-UTF-8 byte strings", &v);
}

/// CONFIGS.md row C5 — the string content looks like `printf` directives.  The
/// C compiles `printLine` down to `puts(line)` while the Rust calls
/// `printf("%s\n", line)`; either way the bytes must be emitted verbatim and
/// never interpreted.
fn c05_print_line_format_directives() {
    let v: Vec<Vec<u8>> = [
        "%s", "%d", "%n", "%%", "%p", "%x", "%c", "%f", "%1000000d", "%.*s", "%%s%%d", "%s%s%s%s",
        "100%", "%", "%%%%%%", "a%sb%dc", "%hn", "%lln", "\u{1}%n%n%n",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    compare_print_line("C5 printf directives inside the payload", &v);
}

/// CONFIGS.md row C6 — control characters, and a NUL in the middle of a larger
/// buffer so the truncation point becomes observable.
fn c06_print_line_control_and_interior_nul() {
    let plain: Vec<Vec<u8>> = [
        &b"a\nb"[..],
        b"\n",
        b"\n\n\n",
        b"a\tb\tc",
        b"a\rb",
        b"\x07\x08\x0b\x0c\x1b",
        b"trailing newline\n",
        b"\x1b[31mred\x1b[0m",
    ]
    .iter()
    .map(|s| s.to_vec())
    .collect();
    compare_print_line("C6 control characters", &plain);

    // NUL-terminated raw buffers with data *after* the terminator.
    let raw: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"ab\0cd\0".to_vec(),
        b"\0ignored-entirely\0".to_vec(),
        {
            let mut b = vec![b'x'; 40];
            b[10] = 0;
            b.push(0);
            b
        },
        {
            let mut b: Vec<u8> = (1u8..=255).collect();
            b[128] = 0;
            b.push(0);
            b
        },
    ];
    compare_print_line_raw("C6 interior NUL truncation", &raw);
}

/// CONFIGS.md row C7 — lengths straddling stdio's internal buffer size.
fn c07_print_line_long_strings() {
    let mut rng = Rng::new(SEED ^ 0x07);
    let mut v = Vec::new();
    for len in [
        1usize, 2, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1023, 1024, 1025, 2047,
        2048, 2049, 4095, 4096, 4097, 8191, 8192, 8193, 16384, 65536,
    ] {
        v.push((0..len).map(|_| b'a' + rng.below(26) as u8).collect());
    }
    compare_print_line("C7 lengths around BUFSIZ", &v);
}

/// CONFIGS.md row C8 — many calls inside a single capture window, checking that
/// repeated output accumulates identically (buffering / interleaving).
fn c08_print_line_many_calls_one_window() {
    let mut rng = Rng::new(SEED ^ 0x08);
    let strings: Vec<std::ffi::CString> = (0..5000)
        .map(|i| {
            let len = rng.below(40) as usize;
            let mut s: Vec<u8> = (0..len).map(|_| b'A' + rng.below(26) as u8).collect();
            s.extend_from_slice(format!("#{i}").as_bytes());
            std::ffi::CString::new(s).unwrap()
        })
        .collect();
    compare_one("C8 5000 printLine calls in one window", |api| unsafe {
        for s in &strings {
            (api.print_line)(s.as_ptr());
        }
    });
}

// ===========================================================================
// printIntLine  (c_src/src/driver.c:38, "%d\n")
// ===========================================================================

/// CONFIGS.md row C9 — digit-count and sign boundaries.
fn c09_print_int_line_small_boundaries() {
    let v: Vec<c_int> = vec![0, 1, -1, 9, -9, 10, -10, 99, -99, 100, -100, 5, -5];
    compare_print_int_line("C9 small values / sign boundaries", &v);
}

/// CONFIGS.md row C10 — the extremes of `int`.
fn c10_print_int_line_extremes() {
    let v: Vec<c_int> = vec![i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -2147483647];
    compare_print_int_line("C10 INT_MIN / INT_MAX", &v);
}

/// CONFIGS.md row C11 — 4096 uniformly random 32-bit patterns.
fn c11_print_int_line_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0x11);
    let v: Vec<c_int> = (0..4096).map(|_| rng.next_u32() as i32).collect();
    compare_print_int_line("C11 random int bit patterns", &v);
}

/// CONFIGS.md row C12 — powers of ten and of two, ±1, all digit counts.
fn c12_print_int_line_powers() {
    compare_print_int_line("C12 powers of ten / two", &edge_ints());
}

// ===========================================================================
// bad  (c_src/src/driver.c:43) — unguarded 100.0/data then (int) narrowing
// ===========================================================================

/// CONFIGS.md row C13 — quotient exactly representable, well inside `int`.
fn c13_bad_exact_quotients() {
    compare_bad(
        "C13 exact quotients",
        &[2.0, 4.0, 5.0, 8.0, 10.0, 20.0, 25.0, 50.0, 100.0, 0.5, 0.25, 0.125, 1.0],
    );
}

/// CONFIGS.md row C14 — quotient needs truncation toward zero, positive.
fn c14_bad_truncating_positive() {
    let mut rng = Rng::new(SEED ^ 0x14);
    let mut v = vec![3.0f32, 7.0, 9.0, 11.0, 0.3, 0.7, 1.5, 2.5, 33.0, 66.0, 99.0, 101.0];
    for _ in 0..2048 {
        v.push(rng.f32_log_uniform(-6.0, 6.0).abs());
    }
    compare_bad("C14 truncation toward zero, positive", &v);
}

/// CONFIGS.md row C15 — same but negative, where truncation toward zero differs
/// from flooring.
fn c15_bad_truncating_negative() {
    let mut rng = Rng::new(SEED ^ 0x15);
    let mut v = vec![-3.0f32, -7.0, -9.0, -11.0, -0.3, -0.7, -1.5, -2.5, -33.0, -99.0, -101.0];
    for _ in 0..2048 {
        v.push(-rng.f32_log_uniform(-6.0, 6.0).abs());
    }
    compare_bad("C15 truncation toward zero, negative", &v);
}

/// CONFIGS.md row C16 — ±1 and ±2 (the constant `goodG2B` hard-codes).
fn c16_bad_one_and_two() {
    compare_bad("C16 +/-1 and +/-2", &[1.0, -1.0, 2.0, -2.0]);
}

/// CONFIGS.md row C17 — the IEEE-754 division special cases.
fn c17_bad_division_special_cases() {
    compare_bad(
        "C17 zeros / infinities / NaNs",
        &[
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            SNAN,
            NEG_SNAN,
            f32::from_bits(0x7fc0_0000),
            f32::from_bits(0x7fff_ffff),
            f32::from_bits(0xffff_ffff),
            f32::from_bits(0x7f80_0001),
            f32::from_bits(0x7fbf_ffff),
        ],
    );
}

/// CONFIGS.md row C18 — subnormal divisors (quotient overflows `int`).
fn c18_bad_subnormals() {
    let mut rng = Rng::new(SEED ^ 0x18);
    let mut v = vec![
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::from_bits(0x007f_ffff),
        -f32::from_bits(0x007f_ffff),
        f32::from_bits(0x0040_0000),
    ];
    for _ in 0..2048 {
        // Random subnormal: exponent field zero, non-zero mantissa.
        let m = 1 + rng.below(0x007f_ffff);
        let s = (rng.next_u32() & 1) << 31;
        v.push(f32::from_bits(s | m));
    }
    compare_bad("C18 subnormal divisors", &v);
}

/// CONFIGS.md row C19 — ±FLT_MAX and ±FLT_MIN.
fn c19_bad_extremes() {
    compare_bad(
        "C19 FLT_MAX / FLT_MIN",
        &[
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            next_down(f32::MAX),
            next_up(f32::MIN),
        ],
    );
}

/// CONFIGS.md row C20 — divisors straddling the `cvttsd2si` valid-range
/// boundary, where the C result flips between a real number and `INT_MIN`.
fn c20_bad_cvt_range_boundary() {
    let mut v = Vec::new();
    for base in [
        100.0f64 / 2147483648.0,
        100.0f64 / 2147483647.0,
        100.0f64 / 2147483649.0,
        100.0f64 / 2147483646.0,
        -100.0f64 / 2147483648.0,
        -100.0f64 / 2147483647.0,
        -100.0f64 / 2147483649.0,
    ] {
        let b = base as f32;
        v.push(b);
        // Walk several ULPs either side of the boundary.
        let mut up = b;
        let mut down = b;
        for _ in 0..8 {
            up = next_up(up);
            down = next_down(down);
            v.push(up);
            v.push(down);
        }
    }
    compare_bad("C20 cvttsd2si range boundary", &v);
}

/// CONFIGS.md row C21 — 8192 uniformly random 32-bit patterns reinterpreted as
/// `float`: hits every class (normals, subnormals, zeros, infinities, all NaN
/// payloads) in their natural proportions.
fn c21_bad_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0x21);
    let v: Vec<f32> = (0..8192).map(|_| rng.f32_bits()).collect();
    compare_bad("C21 random f32 bit patterns", &v);
}

/// CONFIGS.md row C22 — 8192 random finite magnitudes, log-uniform over 40
/// decades, both signs; concentrates on the interesting boundary between
/// "quotient fits an int" and "quotient does not".
fn c22_bad_random_log_uniform() {
    let mut rng = Rng::new(SEED ^ 0x22);
    let v: Vec<f32> = (0..8192).map(|_| rng.f32_log_uniform(-20.0, 20.0)).collect();
    compare_bad("C22 random log-uniform finite floats", &v);
}

// ===========================================================================
// good  (c_src/src/driver.c:72) -> goodG2B + goodB2G (both static)
// ===========================================================================

/// CONFIGS.md row C23 — the `fabs(data) > 0.000001` guard accepts.
fn c23_good_guard_accepts() {
    let mut rng = Rng::new(SEED ^ 0x23);
    let mut v = vec![2.0f32, -2.0, 1.0, -1.0, 3.0, -3.0, 100.0, -100.0, 1e-5, -1e-5, 1e5, -1e5];
    for _ in 0..4096 {
        let mut x = rng.f32_log_uniform(-5.9, 6.0);
        if x.abs() <= 1e-6 {
            x = 1.0;
        }
        v.push(x);
    }
    compare_good("C23 guard accepts", &v);
}

/// CONFIGS.md row C24 — the guard rejects (|data| <= 1e-6), including zeros and
/// subnormals, so `good` never divides by them.
fn c24_good_guard_rejects() {
    let mut rng = Rng::new(SEED ^ 0x24);
    let mut v = vec![
        0.0f32,
        -0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-8,
        -1e-8,
        1e-20,
        -1e-20,
        1e-7,
        -1e-7,
    ];
    for _ in 0..4096 {
        let x = rng.f32_log_uniform(-45.0, -6.1);
        v.push(x);
    }
    compare_good("C24 guard rejects", &v);
}

/// CONFIGS.md row C25 — NaN makes `comisd` unordered so `jbe` is taken; C
/// semantics: `NaN > x` is false, so the reject branch runs.
fn c25_good_nan() {
    let mut rng = Rng::new(SEED ^ 0x25);
    let mut v = vec![f32::NAN, -f32::NAN, SNAN, NEG_SNAN, f32::from_bits(0x7fc0_0000)];
    for _ in 0..1024 {
        // Random NaN payload, both signs, quiet and signalling.
        let payload = 1 + rng.below(0x007f_ffff);
        let s = (rng.next_u32() & 1) << 31;
        v.push(f32::from_bits(s | 0x7f80_0000 | payload));
    }
    compare_good("C25 NaN payloads (unordered guard)", &v);
}

/// CONFIGS.md row C26 — the exact `0.000001` guard boundary and its neighbours.
fn c26_good_guard_boundary() {
    let g = 1e-6f32;
    let mut v = Vec::new();
    let mut up = g;
    let mut down = g;
    v.push(g);
    v.push(-g);
    for _ in 0..16 {
        up = next_up(up);
        down = next_down(down);
        v.push(up);
        v.push(-up);
        v.push(down);
        v.push(-down);
    }
    // Also the double-precision literal rounded to f32 from both directions.
    v.push(0.000001f64 as f32);
    v.push(-(0.000001f64 as f32));
    compare_good("C26 0.000001 guard boundary", &v);
}

/// CONFIGS.md row C27 — on the accept path the quotient magnitude is always
/// < 1e8 < 2^31, so `INT_MIN` must never appear.  Asserted differentially and,
/// additionally, by checking the C output itself over random accepted inputs.
fn c27_good_accept_path_never_overflows() {
    let mut rng = Rng::new(SEED ^ 0x27);
    let v: Vec<f32> = (0..4096)
        .map(|_| {
            let x = rng.f32_log_uniform(-5.99, 38.0);
            if x.abs() <= 1e-6 {
                2.0
            } else {
                x
            }
        })
        .collect();
    compare_good("C27 accept path", &v);

    let l = libs();
    let (out, ()) = capture(|| unsafe {
        for x in &v {
            (l.c.api.good)(*x);
        }
    });
    assert!(
        !String::from_utf8_lossy(&out).contains("-2147483648"),
        "C accept path unexpectedly produced INT_MIN"
    );
}

/// CONFIGS.md row C28 — ±INF passes the guard, `100.0/±INF` is `±0.0`, and
/// `(int)±0.0` is `0`.
fn c28_good_infinities() {
    compare_good("C28 infinities", &[f32::INFINITY, f32::NEG_INFINITY]);
}

/// CONFIGS.md row C29 — 8192 random bit patterns, both guard branches mixed.
fn c29_good_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0x29);
    let v: Vec<f32> = (0..8192).map(|_| rng.f32_bits()).collect();
    compare_good("C29 random f32 bit patterns", &v);
}

// ===========================================================================
// driver  (c_src/src/driver.c:78) — the composed pipeline
// ===========================================================================

/// CONFIGS.md row C30 — nominal end-to-end transcript, random accepted
/// `goodData` and random normal `badData`.
fn c30_driver_nominal() {
    let mut rng = Rng::new(SEED ^ 0x30);
    let mut v = vec![FF(2.0, 4.0), FF(1.0, 1.0), FF(-3.0, 7.0)];
    for _ in 0..2048 {
        let g = {
            let x = rng.f32_log_uniform(-5.0, 5.0);
            if x.abs() <= 1e-6 {
                2.0
            } else {
                x
            }
        };
        let b = {
            let x = rng.f32_log_uniform(-5.0, 5.0);
            if x == 0.0 {
                3.0
            } else {
                x
            }
        };
        v.push(FF(g, b));
    }
    compare_driver("C30 driver nominal", &v);
}

/// CONFIGS.md row C31 — good path fine, bad path degenerate.
fn c31_driver_good_ok_bad_degenerate() {
    let mut rng = Rng::new(SEED ^ 0x31);
    let degenerate = [
        0.0f32,
        -0.0,
        f32::NAN,
        SNAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-8,
        -1e-8,
    ];
    let mut v = Vec::new();
    for b in degenerate {
        for _ in 0..64 {
            let g = {
                let x = rng.f32_log_uniform(-5.0, 5.0);
                if x.abs() <= 1e-6 {
                    2.0
                } else {
                    x
                }
            };
            v.push(FF(g, b));
        }
    }
    compare_driver("C31 good accepted x bad degenerate", &v);
}

/// CONFIGS.md row C32 — good path rejected, bad path normal.
fn c32_driver_good_rejected_bad_normal() {
    let mut rng = Rng::new(SEED ^ 0x32);
    let rejected = [
        0.0f32,
        -0.0,
        f32::NAN,
        NEG_SNAN,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-8,
        -1e-8,
        1e-6,
    ];
    let mut v = Vec::new();
    for g in rejected {
        for _ in 0..64 {
            let b = {
                let x = rng.f32_log_uniform(-5.0, 5.0);
                if x == 0.0 {
                    3.0
                } else {
                    x
                }
            };
            v.push(FF(g, b));
        }
    }
    compare_driver("C32 good rejected x bad normal", &v);
}

/// CONFIGS.md row C33 — both anomalies in the same call.
fn c33_driver_both_degenerate() {
    let odd = [
        0.0f32,
        -0.0,
        f32::NAN,
        SNAN,
        NEG_SNAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1),
        -f32::from_bits(1),
        1e-8,
        -1e-8,
    ];
    let mut v = Vec::new();
    for g in odd {
        for b in odd {
            v.push(FF(g, b));
        }
    }
    compare_driver("C33 both degenerate", &v);
}

/// CONFIGS.md row C34 — full cross product of the 24-value edge corpus.
fn c34_driver_edge_cross_product() {
    let corpus = cross_floats();
    let mut v = Vec::with_capacity(corpus.len() * corpus.len());
    for g in &corpus {
        for b in &corpus {
            v.push(FF(*g, *b));
        }
    }
    assert_eq!(v.len(), 24 * 24);
    compare_driver("C34 24x24 edge cross product", &v);
}

/// CONFIGS.md row C35 — 4096 random `(f32, f32)` bit-pattern pairs.
fn c35_driver_random_pairs() {
    let mut rng = Rng::new(SEED ^ 0x35);
    let v: Vec<FF> = (0..4096)
        .map(|_| FF(rng.f32_bits(), rng.f32_bits()))
        .collect();
    compare_driver("C35 random f32 pairs", &v);
}

// ===========================================================================
// Composed / interleaved use  (C36)
// ===========================================================================

/// CONFIGS.md row C36 — a randomly ordered, interleaved sequence of calls to
/// all five exported entry points inside one capture window.  This is the only
/// row that can catch a divergence in how the calls compose (ordering, stdio
/// buffering) rather than in one function in isolation.
fn c36_interleaved_mixed_calls() {
    let mut rng = Rng::new(SEED ^ 0x36);
    let floats = edge_floats();

    #[derive(Debug)]
    enum Op {
        Line(std::ffi::CString),
        Int(c_int),
        Bad(f32),
        Good(f32),
        Driver(f32, f32),
        Null,
    }

    let mut ops = Vec::new();
    for i in 0..6000 {
        ops.push(match rng.below(6) {
            0 => {
                let len = rng.below(24) as usize;
                let mut s: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
                s.extend_from_slice(format!("|{i}").as_bytes());
                Op::Line(std::ffi::CString::new(s).unwrap())
            }
            1 => Op::Int(rng.next_u32() as i32),
            2 => Op::Bad(floats[rng.below(floats.len() as u32) as usize]),
            3 => Op::Good(floats[rng.below(floats.len() as u32) as usize]),
            4 => Op::Driver(
                floats[rng.below(floats.len() as u32) as usize],
                floats[rng.below(floats.len() as u32) as usize],
            ),
            _ => Op::Null,
        });
    }

    compare_one("C36 interleaved mixed sequence", |api| unsafe {
        for op in &ops {
            match op {
                Op::Line(s) => (api.print_line)(s.as_ptr()),
                Op::Int(i) => (api.print_int_line)(*i),
                Op::Bad(x) => (api.bad)(*x),
                Op::Good(x) => (api.good)(*x),
                Op::Driver(a, b) => (api.driver)(*a, *b),
                Op::Null => (api.print_line)(std::ptr::null()),
            }
        }
    });
}

// ===========================================================================
// Wide sweeps over the f32 encoding space (C38, C39)
// ===========================================================================

/// Number of `f32` bit patterns visited by the stride sweeps.  Overridable with
/// `DRIVER_SWEEP_LOG2` (default 2^18 = 262144 values per sweep, ~1 s).
fn sweep_count() -> u32 {
    let log2: u32 = std::env::var("DRIVER_SWEEP_LOG2")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(22);
    1u32 << log2.min(31)
}

/// Walks the whole 32-bit `f32` space with a constant stride, so every exponent
/// value and a large, evenly spread set of mantissas is visited (odd stride =>
/// the low mantissa bits vary too).  Compares the concatenated transcripts.
fn stride_sweep(what: &str, call: impl Fn(&Api, f32) + Copy) {
    let n = sweep_count();
    let stride: u32 = 0xFFFF_FFFFu32 / n | 1; // odd => visits all residue classes
    let l = libs();
    let run = move |api: &Api| {
        let mut bits: u32 = 0;
        for _ in 0..n {
            call(api, f32::from_bits(bits));
            bits = bits.wrapping_add(stride);
        }
    };
    let (c_out, ()) = capture(|| run(&l.c.api));
    let (r_out, ()) = capture(|| run(&l.rs.api));
    if c_out == r_out {
        return;
    }
    // Report the first differing line together with the input that produced it.
    let c_lines: Vec<&[u8]> = c_out.split(|b| *b == b'\n').collect();
    let r_lines: Vec<&[u8]> = r_out.split(|b| *b == b'\n').collect();
    let idx = c_lines
        .iter()
        .zip(r_lines.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c_lines.len().min(r_lines.len()));
    panic!(
        "\nDIVERGENCE in {what} at output line {idx}\n  C   : \"{}\"\n  Rust: \"{}\"\n\
         (stride 0x{stride:08x}, {n} inputs; re-run with DRIVER_SWEEP_LOG2 to widen)\n",
        esc(c_lines.get(idx).copied().unwrap_or(b"<eof>")),
        esc(r_lines.get(idx).copied().unwrap_or(b"<eof>")),
    );
}

/// CONFIGS.md row C38 — `bad` over a strided sweep of the entire `f32` space.
fn c38_bad_exhaustive_stride_sweep() {
    stride_sweep("C38 bad() stride sweep", |api, x| unsafe { (api.bad)(x) });
}

/// CONFIGS.md row C39 — `good` over a strided sweep of the entire `f32` space
/// (mixes both guard branches in their natural proportions).
fn c39_good_exhaustive_stride_sweep() {
    stride_sweep("C39 good() stride sweep", |api, x| unsafe { (api.good)(x) });
}

/// CONFIGS.md row C40 — exhaustive sweep over **all 2^32 `f32` bit patterns**
/// for the two single-argument entry points, in chunks so that memory and disk
/// stay bounded.  Skipped unless `DRIVER_SWEEP_FULL=1`; the range can be split
/// across runs with `DRIVER_SWEEP_FROM` / `DRIVER_SWEEP_TO` (hex or decimal).
/// `tests/feature_matrix.sh` does not run it (it takes tens of minutes); it is
/// run explicitly, once, to prove there is no `f32` input at all on which the
/// two implementations disagree.
fn c40_full_f32_sweep() {
    if std::env::var("DRIVER_SWEEP_FULL").as_deref() != Ok("1") {
        println!("(skipped: set DRIVER_SWEEP_FULL=1 to run) ");
        return;
    }
    let parse = |k: &str, d: u64| -> u64 {
        std::env::var(k)
            .ok()
            .map(|s| {
                let s = s.trim().to_string();
                if let Some(h) = s.strip_prefix("0x") {
                    u64::from_str_radix(h, 16).expect("hex")
                } else {
                    s.parse().expect("dec")
                }
            })
            .unwrap_or(d)
    };
    let from = parse("DRIVER_SWEEP_FROM", 0);
    let to = parse("DRIVER_SWEEP_TO", 1u64 << 32);
    let chunk = parse("DRIVER_SWEEP_CHUNK", 1 << 20);
    let l = libs();

    let mut base = from;
    while base < to {
        let end = (base + chunk).min(to);
        for (label, call) in [
            ("bad", 0usize),
            ("good", 1usize),
        ] {
            let run = |api: &Api| unsafe {
                for b in base..end {
                    let x = f32::from_bits(b as u32);
                    if call == 0 {
                        (api.bad)(x)
                    } else {
                        (api.good)(x)
                    }
                }
            };
            let (c_out, ()) = capture(|| run(&l.c.api));
            let (r_out, ()) = capture(|| run(&l.rs.api));
            if c_out != r_out {
                // Binary-search the chunk down to the exact offending value.
                for b in base..end {
                    let x = f32::from_bits(b as u32);
                    let one = |api: &Api| unsafe {
                        if call == 0 {
                            (api.bad)(x)
                        } else {
                            (api.good)(x)
                        }
                    };
                    let (c1, ()) = capture(|| one(&l.c.api));
                    let (r1, ()) = capture(|| one(&l.rs.api));
                    assert_eq!(
                        c1,
                        r1,
                        "\nDIVERGENCE C40 {label}(0x{:08x} = {:e})\n  C   : \"{}\"\n  Rust: \"{}\"\n",
                        x.to_bits(),
                        x,
                        esc(&c1),
                        esc(&r1)
                    );
                }
                panic!("C40 {label}: chunk 0x{base:08x}..0x{end:08x} differs but no single value does");
            }
        }
        base = end;
        if base.is_multiple_of(1u64 << 26) {
            eprint!("[C40 {:.1}%] ", 100.0 * (base - from) as f64 / (to - from) as f64);
        }
    }
    print!("(0x{from:08x}..0x{to:08x} exhaustive) ");
}
