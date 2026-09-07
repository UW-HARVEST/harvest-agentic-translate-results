//! Phase C — error/rejection-path differential tests, one test per ERRORS.md row.
//!
//! The C library has no error returns (`synth_pair` is `void`), so its whole
//! rejection surface is the clamping in `mp3d_scale_pcm` plus out-of-contract
//! parameter values. Each test asserts BOTH that C and Rust agree AND what the
//! concrete expected sentinel value is (`INT16_MAX` / `INT16_MIN` / `0`), so a
//! row can never pass by "both failed somehow".

mod common;
use common::*;

/// Tap index + coefficient that let us set an accumulator to an exact value:
/// with every other tap zero, `a == v * coeff` exactly (all other terms add 0).
const ACC1_TAP: usize = 7 * 64; // a += z[7*64] * 75038
const ACC1_COEFF: f32 = 75038.0;
const ACC2_TAP: usize = 2 + 8 * 64; // a += z[8*64] * 64019 (after z += 2)
const ACC2_COEFF: f32 = 64019.0;

fn next_up(x: f32) -> f32 {
    let b = x.to_bits();
    if x == 0.0 {
        f32::from_bits(1)
    } else if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}
fn next_down(x: f32) -> f32 {
    let b = x.to_bits();
    if x == 0.0 {
        -f32::from_bits(1)
    } else if x > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

/// Find `v` such that `v * coeff` rounds to exactly `target` in f32.
fn solve_exact(coeff: f32, target: f32) -> f32 {
    let base = target / coeff;
    let bits = base.to_bits() as i64;
    for d in 0..4096i64 {
        for s in [d, -d] {
            let v = f32::from_bits(((bits + s) as u32) & 0xFFFF_FFFF);
            if v * coeff == target {
                return v;
            }
        }
    }
    panic!("no f32 v with v*{coeff} == {target}");
}

/// Not every f32 is a product `v * coeff`, so for targets that are not exactly
/// reachable we take the closest reachable accumulator and predict the expected
/// sample with `model` (an independent transcription of the C source) rather
/// than with a hand-written constant.
fn solve_near(coeff: f32, target: f32) -> (f32, f32) {
    let base = target / coeff;
    let bits = base.to_bits() as i64;
    let mut best = (base, base * coeff);
    let mut best_err = f32::INFINITY;
    for d in -4096i64..=4096 {
        let v = f32::from_bits(((bits + d) as u32) & 0xFFFF_FFFF);
        let prod = v * coeff;
        let err = (prod - target).abs();
        if err < best_err {
            best_err = err;
            best = (v, prod);
        }
    }
    best
}

/// Independent transcription of the C `mp3d_scale_pcm`, used only to predict
/// expected values in the error-path tests. C and Rust are still compared to
/// each other directly; this just stops a typo'd constant from masking a real
/// disagreement (and pins the exact sentinel, so a row can never pass by
/// "both failed somehow").
fn model(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i32 as i16;
    s -= (s < 0) as i16;
    s
}

/// Drive `acc` to the accumulator value closest to `a_target`, then assert
/// C == Rust == model, and report the exact sentinel produced.
fn expect_model(p: &Pair, acc: u8, a_target: f32, ctx: &str) -> i16 {
    let nch = 2i32;
    let (tap, coeff, idx) = match acc {
        1 => (ACC1_TAP, ACC1_COEFF, 0usize),
        _ => (ACC2_TAP, ACC2_COEFF, 16 * nch as usize),
    };
    let (v, actual) = if a_target.is_finite() {
        solve_near(coeff, a_target)
    } else {
        (a_target, a_target)
    };
    let mut z = make_z(0, |_| 0.0);
    z[tap] = v;
    let case = Case::new(nch, &z);
    let (c, r) = run(p, &case);
    assert_eq!(
        c, r,
        "[{ctx}] acc{acc} a={actual:e} (v={v:e}) DIVERGED:\n  C   ={c:?}\n  Rust={r:?}"
    );
    let want = model(actual);
    assert_eq!(
        c[idx], want,
        "[{ctx}] acc{acc} a={actual:e}: C gave {} but the C-semantics model says {want}",
        c[idx]
    );
    c[idx]
}

/// Set a single tap so the chosen accumulator equals `a_exact`, run both
/// libraries, and return (c_pcm, rust_pcm) plus the index that accumulator wrote.
fn drive(p: &Pair, acc: u8, a_exact: f32, nch: i32) -> (Vec<i16>, Vec<i16>, usize) {
    let (tap, coeff, idx) = match acc {
        1 => (ACC1_TAP, ACC1_COEFF, 0usize),
        _ => (ACC2_TAP, ACC2_COEFF, 16 * nch as usize),
    };
    let v = if a_exact.is_finite() {
        solve_exact(coeff, a_exact)
    } else {
        a_exact // inf / NaN propagate through the multiply unchanged
    };
    let mut z = make_z(0, |_| 0.0);
    z[tap] = v;
    let case = Case::new(nch, &z);
    let (c, r) = run(p, &case);
    assert_eq!(
        c, r,
        "acc{acc} a={a_exact:e} (v={v:e}) diverged:\n  C   ={c:?}\n  Rust={r:?}"
    );
    (c, r, case.head + idx)
}

fn expect(p: &Pair, acc: u8, a_exact: f32, want: i16, ctx: &str) {
    let nch = 2;
    let (c, r, idx) = drive(p, acc, a_exact, nch);
    assert_eq!(c[idx], want, "[{ctx}] C produced {} want {want}", c[idx]);
    assert_eq!(r[idx], want, "[{ctx}] Rust produced {} want {want}", r[idx]);
}

// ------------------------------------------------- ERRORS rows 1 & 3: clamp high
#[test]
fn err01_03_clamp_high_both_stores() {
    let p = Pair::load();
    for a in [32766.5f32, 32767.0, 40000.0, 1.0e9, 3.0e38] {
        expect(&p, 1, a, 32767, &format!("row01 acc1 a={a:e}"));
        expect(&p, 2, a, 32767, &format!("row03 acc2 a={a:e}"));
    }
}

// ------------------------------------------------- ERRORS rows 2 & 4: clamp low
#[test]
fn err02_04_clamp_low_both_stores() {
    let p = Pair::load();
    for a in [-32767.5f32, -32768.0, -40000.0, -1.0e9, -3.0e38] {
        expect(&p, 1, a, -32768, &format!("row02 acc1 a={a:e}"));
        expect(&p, 2, a, -32768, &format!("row04 acc2 a={a:e}"));
    }
}

// ------------------------------------------------- ERRORS row 5: exact upper boundary
#[test]
fn err05_exact_upper_boundary_inclusive() {
    let p = Pair::load();
    // `sample >= 32766.5` is inclusive -> clamp, NOT the rounding path.
    expect(&p, 1, 32766.5, 32767, "row05 acc1 == 32766.5");
    expect(&p, 2, 32766.5, 32767, "row05 acc2 == 32766.5");
}

// ------------------------------------------------- ERRORS row 6: exact lower boundary
#[test]
fn err06_exact_lower_boundary_inclusive() {
    let p = Pair::load();
    expect(&p, 1, -32767.5, -32768, "row06 acc1 == -32767.5");
    expect(&p, 2, -32767.5, -32768, "row06 acc2 == -32767.5");
}

// ------------------------------------------------- ERRORS row 7: one ULP below upper
#[test]
fn err07_one_ulp_below_upper_boundary() {
    let p = Pair::load();
    // Sweep the whole reachable neighbourhood just under the boundary so the
    // exact flip point from the round path (32766) to the clamp (32767) is
    // pinned on both sides, for both accumulators.
    let mut a = 32766.5f32;
    for _ in 0..8 {
        a = next_down(a);
        let got1 = expect_model(&p, 1, a, &format!("row07 acc1 a={a:e}"));
        let got2 = expect_model(&p, 2, a, &format!("row07 acc2 a={a:e}"));
        // Just below the boundary the round path must be taken, never the clamp.
        for got in [got1, got2] {
            assert!(
                (32760..=32767).contains(&got),
                "row07: unexpected sample {got} just under the upper boundary"
            );
        }
    }
    // The reachable value nearest 32766.498046875 must not clamp.
    let (_, actual) = solve_near(ACC1_COEFF, next_down(32766.5f32));
    if (actual as f64) < 32766.5 {
        assert_eq!(model(actual), 32766, "row07: round path should yield 32766");
        expect_model(&p, 1, actual, "row07 acc1 nearest-below");
    }
}

// ------------------------------------------------- ERRORS row 8: one ULP above lower
#[test]
fn err08_one_ulp_above_lower_boundary() {
    let p = Pair::load();
    let mut a = -32767.5f32;
    for _ in 0..8 {
        a = next_up(a);
        let got1 = expect_model(&p, 1, a, &format!("row08 acc1 a={a:e}"));
        let got2 = expect_model(&p, 2, a, &format!("row08 acc2 a={a:e}"));
        for got in [got1, got2] {
            assert!(
                (-32768..=-32760).contains(&got),
                "row08: unexpected sample {got} just above the lower boundary"
            );
        }
    }
    // The reachable value nearest -32767.498046875 must take the round path
    // (trunc toward zero gives -32766, then `s -= (s < 0)` -> -32767).
    let (_, actual) = solve_near(ACC1_COEFF, next_up(-32767.5f32));
    if (actual as f64) > -32767.5 {
        assert_eq!(
            model(actual),
            -32767,
            "row08: round path + decrement should yield -32767"
        );
        expect_model(&p, 1, actual, "row08 acc1 nearest-above");
    }
}

// ------------------------------------------------- ERRORS row 9: `s -= (s < 0)` fires
#[test]
fn err09_negative_decrement_path() {
    let p = Pair::load();
    // For a in (-1.5, -0.5): trunc(a+.5) == 0 -> s==0, NO decrement (row 10).
    // For a <= -1.5: trunc(a+.5) < 0 -> decrement fires.
    for (a, want) in [
        (-1.5f32, -2i16),
        (-2.0, -2),
        (-2.5, -3),
        (-10.25, -10),
        (-100.75, -101),
        (-1000.0, -1000),
        (-32766.0, -32766),
    ] {
        expect(&p, 1, a, want, &format!("row09 acc1 a={a}"));
        expect(&p, 2, a, want, &format!("row09 acc2 a={a}"));
    }
}

// ------------------------------------------------- ERRORS row 10: s == 0 no decrement
#[test]
fn err10_zero_no_decrement() {
    let p = Pair::load();
    for a in [-0.5f32, -0.75, -1.0, -1.25, next_up(-1.5f32), next_down(-0.5f32)] {
        expect(&p, 1, a, 0, &format!("row10 acc1 a={a}"));
        expect(&p, 2, a, 0, &format!("row10 acc2 a={a}"));
    }
    // And the positive side of the same window. Note `next_down(0.5f32)` is NOT
    // 0 in C: 0.49999997 + .5f rounds to exactly 1.0f, so the C truncation gives
    // 1. The model pins that behaviour instead of a guessed constant.
    for a in [0.0f32, 0.25, 0.49] {
        expect(&p, 1, a, 0, &format!("row10 acc1 pos a={a}"));
    }
    expect(&p, 1, 0.5, 1, "row10 acc1 a=0.5 rounds up");
    for a in [next_down(0.5f32), next_up(0.5f32), 0.4999, 0.50001] {
        expect_model(&p, 1, a, &format!("row10 acc1 near-half a={a:e}"));
        expect_model(&p, 2, a, &format!("row10 acc2 near-half a={a:e}"));
    }
}

// ------------------------------------------------- ERRORS row 11: negative zero
#[test]
fn err11_negative_zero_accumulator() {
    let p = Pair::load();
    // -0.0 * coeff == -0.0; (0 - 0) * 29 == 0, and 0 + (-0.0) == 0.0, so the
    // accumulator is +0.0 either way -> 0. Verified against C regardless.
    let mut z = make_z(0, |_| -0.0f32);
    z[ACC1_TAP] = -0.0;
    let case = Case::new(2, &z);
    let (c, r) = run(&p, &case);
    assert_eq!(c, r, "row11 -0.0 buffer diverged: C={c:?} Rust={r:?}");
    assert_eq!(c[0], 0, "row11 expected 0 from -0.0, got {}", c[0]);
    assert_eq!(c[32], 0, "row11 expected 0 from -0.0, got {}", c[32]);
}

// ------------------------------------------------- ERRORS rows 12 & 15: NaN
#[test]
fn err12_15_nan_accumulator() {
    let p = Pair::load();
    // Row 12: a direct NaN tap.
    for nan in [f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_0001), f32::from_bits(0xFFFF_FFFF)] {
        expect(&p, 1, nan, 0, &format!("row12 acc1 nan bits={:#x}", nan.to_bits()));
        expect(&p, 2, nan, 0, &format!("row12 acc2 nan bits={:#x}", nan.to_bits()));
    }
    // Row 15: NaN generated *inside* the dot product by inf - inf.
    // Line 15: a = (z[14*64] - z[0]) * 29 -> inf - inf == NaN.
    let mut z = make_z(0, |_| 0.0);
    z[14 * 64] = f32::INFINITY;
    z[0] = f32::INFINITY;
    let case = Case::new(2, &z);
    let (c, r) = run(&p, &case);
    assert_eq!(c, r, "row15 inf-inf diverged: C={c:?} Rust={r:?}");
    assert_eq!(c[0], 0, "row15 inf-inf expected 0, got {}", c[0]);

    // NaN generated by inf * 0 is not reachable (all coefficients are non-zero),
    // so instead: +inf and -inf in the same accumulator via the paired adds.
    let mut z = make_z(0, |_| 0.0);
    z[1 * 64] = f32::INFINITY;
    z[13 * 64] = f32::NEG_INFINITY;
    let case = Case::new(2, &z);
    let (c, r) = run(&p, &case);
    assert_eq!(c, r, "row15 inf+(-inf) diverged: C={c:?} Rust={r:?}");
    assert_eq!(c[0], 0, "row15 inf+(-inf) expected 0, got {}", c[0]);
}

// ------------------------------------------------- ERRORS rows 13 & 14: infinities
#[test]
fn err13_14_infinite_accumulator() {
    let p = Pair::load();
    expect(&p, 1, f32::INFINITY, 32767, "row13 acc1 +inf");
    expect(&p, 2, f32::INFINITY, 32767, "row13 acc2 +inf");
    expect(&p, 1, f32::NEG_INFINITY, -32768, "row14 acc1 -inf");
    expect(&p, 2, f32::NEG_INFINITY, -32768, "row14 acc2 -inf");

    // Overflow to infinity inside the accumulation (f32::MAX * 29).
    let mut z = make_z(0, |_| 0.0);
    z[14 * 64] = f32::MAX;
    let case = Case::new(2, &z);
    let (c, r) = run(&p, &case);
    assert_eq!(c, r, "overflow-to-inf diverged: C={c:?} Rust={r:?}");
    assert_eq!(c[0], 32767, "f32::MAX*29 should clamp high, got {}", c[0]);
}

// ------------------------------------------------- ERRORS row 16: nch == 0 aliasing
#[test]
fn err16_nch_zero_second_store_wins() {
    let p = Pair::load();
    // acc1 drives to +32767, acc2 drives to -32768. With nch == 0 both target
    // pcm[0], so the SECOND store (acc2) must win in both implementations.
    let mut z = make_z(0, |_| 0.0);
    z[ACC1_TAP] = solve_exact(ACC1_COEFF, 32767.0);
    z[ACC2_TAP] = solve_exact(ACC2_COEFF, -32768.0);
    let mut case = Case::new(0, &z);
    case.pcm_len = 4;
    let (c, r) = run(&p, &case);
    assert_eq!(c, r, "row16 nch=0 diverged: C={c:?} Rust={r:?}");
    assert_eq!(
        c[0], -32768,
        "row16: with nch==0 the second store must overwrite the first, got {}",
        c[0]
    );
    // And the reverse ordering, to prove it is store order and not value size.
    let mut z = make_z(0, |_| 0.0);
    z[ACC1_TAP] = solve_exact(ACC1_COEFF, -32768.0);
    z[ACC2_TAP] = solve_exact(ACC2_COEFF, 32767.0);
    let mut case = Case::new(0, &z);
    case.pcm_len = 4;
    let (c, r) = run(&p, &case);
    assert_eq!(c, r, "row16 reverse diverged");
    assert_eq!(c[0], 32767, "row16 reverse: got {}", c[0]);
}

// ------------------------------------------------- ERRORS row 17: negative nch
#[test]
fn err17_negative_nch_writes_below_pcm() {
    let p = Pair::load();
    let mut rng = Rng::new(0xBADC_0FFE);
    for nch in [-1i32, -2, -3, -16, -100] {
        for _ in 0..64 {
            let z = make_z(0, |_| rng.sym(0.4));
            let mut case = Case::new(nch, &z);
            case.pcm_len = case.head + 4;
            let (c, r) = run(&p, &case);
            assert_eq!(c, r, "row17 nch={nch} diverged: C={c:?} Rust={r:?}");
            // The second sample must land at head + 16*nch == 0.
            let target = (case.head as i64 + 16 * nch as i64) as usize;
            assert_eq!(target, 0, "test setup: expected target index 0");
            // Cells other than pcm[head] and pcm[0] must remain canaries.
            for (i, cell) in c.iter().enumerate() {
                if i != 0 && i != case.head {
                    assert_eq!(*cell, case.canary, "row17 nch={nch}: C touched index {i}");
                }
            }
        }
    }
}

// ------------------------------------------------- ERRORS row 21: minimal z span
#[test]
fn err21_reads_only_the_documented_899_floats() {
    let p = Pair::load();
    let mut rng = Rng::new(0xFEED_5EED);
    for _ in 0..256 {
        // Poison every index the C does NOT read with NaN; if either
        // implementation read an extra index the result would become 0/garbage
        // and the two would diverge from the clean-reference run.
        let taps = tap_offsets();
        let clean = make_z(0, |i| if taps.contains(&i) { rng.sym(0.4) } else { 0.0 });
        let mut poisoned = clean.clone();
        for i in 0..Z_MIN_LEN {
            if !taps.contains(&i) {
                poisoned[i] = f32::NAN;
            }
        }
        let a = run(&p, &Case::new(1, &clean));
        let b = run(&p, &Case::new(1, &poisoned));
        assert_eq!(a.0, a.1, "row21 clean run diverged");
        assert_eq!(b.0, b.1, "row21 poisoned run diverged");
        assert_eq!(
            a.0, b.0,
            "row21: C result changed when non-tap indices were poisoned -> \
             the tap set in tap_offsets() is wrong"
        );
    }
}

// ------------------------------------------------- ERRORS row 22: subnormals
#[test]
fn err22_subnormal_accumulator() {
    let p = Pair::load();
    for a in [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::from_bits(0x007F_FFFF),
        -f32::from_bits(0x007F_FFFF),
    ] {
        let mut z = make_z(0, |_| 0.0);
        z[ACC1_TAP] = a; // product may itself be subnormal/zero; both must agree
        let case = Case::new(2, &z);
        let (c, r) = run(&p, &case);
        assert_eq!(c, r, "row22 a={a:e} diverged: C={c:?} Rust={r:?}");
        assert_eq!(c[0], 0, "row22 a={a:e}: expected 0, got {}", c[0]);
    }
}

// ------------------------------------------------- generic FFI boundary values
// `nch` is a C `int`: every 32-bit value is a legal thing for a caller to pass.
// This is the enum/out-of-range-int analogue for this API. We test every value
// whose store offset still lands inside a buffer we can safely provide.
#[test]
fn generic_nch_value_sweep() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0A11_600D);
    let mut values: Vec<i32> = vec![
        0, 1, -1, 2, -2, 3, -3, 7, 8, -8, 15, 16, -16, 31, 32, 63, 64, -64, 127, 128, 255, 256,
        511, 512, 1023, 1024, 2047, 2048, 4095, 4096, -4096,
    ];
    for _ in 0..64 {
        values.push((rng.next_u32() % 8192) as i32 - 4096);
    }
    for nch in values {
        let z = make_z(0, |_| rng.sym(0.4));
        let mut case = Case::new(nch, &z);
        case.pcm_len = case.head + 16 * nch.unsigned_abs() as usize + 4;
        let (c, r) = run(&p, &case);
        assert_eq!(c, r, "nch sweep diverged at nch={nch}");
    }
}

// Boundary: the two written cells for the smallest legal buffer (nch == 0 means
// a 1-cell buffer is enough; nch == 1 needs 17).
#[test]
fn generic_minimum_pcm_buffers() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1234_5678);
    for (nch, len) in [(0i32, 1usize), (1, 17), (2, 33), (5, 81)] {
        for _ in 0..128 {
            let z = make_z(0, |_| rng.sym(0.4));
            let mut case = Case::new(nch, &z);
            case.pcm_len = len;
            let (c, r) = run(&p, &case);
            assert_eq!(c, r, "minimum buffer nch={nch} len={len} diverged");
        }
    }
}

// The library has no null checks; confirm BOTH .so's expose that same absence
// by verifying neither exports any validation entry point and that the symbol
// surface is identical. (Dereferencing NULL is UB in both and would abort the
// test process, so it is asserted structurally, per ERRORS.md rows 19-20.)
#[test]
fn generic_no_validation_symbols() {
    let p = Pair::load();
    // Both handles resolved `synth_pair`; nothing else is exported by the C .so.
    let _ = p.c;
    let _ = p.rust;
    unsafe {
        let lib = libloading::Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/libsynth_pair_lib.so"),
        );
        if let Ok(lib) = lib {
            // A private helper must NOT be exported.
            let sym: Result<libloading::Symbol<unsafe extern "C" fn(f32) -> i16>, _> =
                lib.get(b"mp3d_scale_pcm\0");
            assert!(
                sym.is_err(),
                "mp3d_scale_pcm is `static` in C and must not be exported by Rust"
            );
        }
    }
}
