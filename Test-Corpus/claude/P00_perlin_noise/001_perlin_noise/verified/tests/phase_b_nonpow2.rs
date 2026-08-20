//! Phase B rows C30..C37: `stb_perlin_noise3_wrap_nonpow2`.
//!
//! This is the only function that can index the permutation table out of
//! bounds (whenever a wrap argument leaves `1..=256`).  Inputs whose indices
//! stay inside the 1024-byte window that both C builds lay out identically
//! (`randtab` followed by `randtab_grad_idx`) are compared bit-for-bit;
//! `common::classify_nonpow2` filters out the genuinely undefined ones, which
//! `phase_c_errors.rs` handles instead.

mod common;

use common::{classify_nonpow2, Diff, Nonpow2Class, Rng};

#[allow(clippy::too_many_arguments)]
fn compare(
    d: &mut Diff,
    skipped: &mut usize,
    x: f32,
    y: f32,
    z: f32,
    xw: i32,
    yw: i32,
    zw: i32,
    seed: u8,
) {
    if classify_nonpow2(x, y, z, xw, yw, zw, seed) != Nonpow2Class::Reproducible {
        *skipped += 1;
        return;
    }
    let (c, r) = (common::c_api(), common::rust_api());
    d.check(
        format_args!(
            "wraps=({xw},{yw},{zw}) seed={seed} x={:#010x} y={:#010x} z={:#010x} ({x:e},{y:e},{z:e})",
            x.to_bits(),
            y.to_bits(),
            z.to_bits()
        ),
        unsafe { (c.wrap_nonpow2)(x, y, z, xw, yw, zw, seed) },
        unsafe { (r.wrap_nonpow2)(x, y, z, xw, yw, zw, seed) },
    );
}

/// C30: all wraps 0 (meaning 256).
#[test]
fn c30_nonpow2_zero_wraps() {
    let mut d = Diff::new("C30 nonpow2 wraps=(0,0,0)");
    let mut skipped = 0;
    let mut rng = Rng::new(0x30);
    for _ in 0..4000 {
        let (x, y, z) = (rng.coord(300), rng.coord(300), rng.coord(300));
        let s = rng.seed_u8();
        compare(&mut d, &mut skipped, x, y, z, 0, 0, 0, s);
    }
    assert_eq!(skipped, 0, "wraps of 0 must never leave the table");
    d.finish();
}

/// C31: uniform wraps in 1..=256.
#[test]
fn c31_nonpow2_uniform_wraps() {
    let mut d = Diff::new("C31 nonpow2 uniform wraps 1..=256");
    let mut skipped = 0;
    let mut rng = Rng::new(0x31);
    for _ in 0..8000 {
        let w = rng.range(1, 256);
        let (x, y, z) = (rng.coord(300), rng.coord(300), rng.coord(300));
        let s = rng.seed_u8();
        compare(&mut d, &mut skipped, x, y, z, w, w, w, s);
    }
    assert_eq!(skipped, 0, "wraps in 1..=256 must never leave the table");
    d.finish();
}

/// C32: per-axis different wraps in 1..=256, large coordinates.
#[test]
fn c32_nonpow2_mixed_wraps() {
    let mut d = Diff::new("C32 nonpow2 mixed wraps, large coords");
    let mut skipped = 0;
    let mut rng = Rng::new(0x32);
    for _ in 0..8000 {
        let (xw, yw, zw) = (rng.range(1, 256), rng.range(1, 256), rng.range(1, 256));
        let (x, y, z) = (
            rng.coord(1 << 20),
            rng.coord(1 << 20),
            rng.coord(1 << 20),
        );
        let s = rng.seed_u8();
        compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
    }
    assert_eq!(skipped, 0);
    d.finish();
}

/// C33: prime / non-power-of-two wraps.
#[test]
fn c33_nonpow2_prime_wraps() {
    let mut d = Diff::new("C33 nonpow2 prime wraps");
    let mut skipped = 0;
    let primes = [3, 5, 7, 11, 13, 97, 251, 17, 31, 127, 255];
    let mut rng = Rng::new(0x33);
    for &xw in &primes {
        for &yw in &primes {
            for &zw in &primes {
                let (x, y, z) = (rng.coord(1000), rng.coord(1000), rng.coord(1000));
                let s = rng.seed_u8();
                compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
            }
        }
    }
    assert_eq!(skipped, 0);
    d.finish();
}

/// C34: wrap == 1 on one/two/three axes.
#[test]
fn c34_nonpow2_wrap_one() {
    let mut d = Diff::new("C34 nonpow2 wrap==1");
    let mut skipped = 0;
    let others = [1, 2, 3, 5, 64, 256, 0];
    let mut rng = Rng::new(0x34);
    for mask in 1..8u32 {
        for _ in 0..300 {
            let pick = |rng: &mut Rng, on: bool| if on { 1 } else { *rng.pick(&others) };
            let xw = pick(&mut rng, mask & 1 != 0);
            let yw = pick(&mut rng, mask & 2 != 0);
            let zw = pick(&mut rng, mask & 4 != 0);
            let (x, y, z) = (rng.coord(64), rng.coord(64), rng.coord(64));
            let s = rng.seed_u8();
            compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
        }
    }
    assert_eq!(skipped, 0);
    d.finish();
}

/// C35: wraps above 256 whose indices still land inside the modelled window
/// (this is where the C code reads the *gradient* table through `randtab`).
#[test]
fn c35_nonpow2_wrap_above_256_in_window() {
    let mut d = Diff::new("C35 nonpow2 wraps > 256 inside the modelled window");
    let mut skipped = 0;
    let mut rng = Rng::new(0x35);
    for _ in 0..40000 {
        let (xw, yw, zw) = (
            rng.range(257, 1100),
            rng.range(257, 1100),
            rng.range(257, 1100),
        );
        let (x, y, z) = (rng.coord(1200), rng.coord(1200), rng.coord(1200));
        let s = rng.seed_u8();
        compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
    }
    assert!(
        d.cases() > 200,
        "expected a decent number of reproducible cases, got {} (skipped {skipped})",
        d.cases()
    );
    d.finish();
}

/// C36: negative wraps with a non-negative `px` (`px % -w >= 0`).
#[test]
fn c36_nonpow2_negative_wrap_positive_px() {
    let mut d = Diff::new("C36 nonpow2 negative wraps, px >= 0");
    let mut skipped = 0;
    let mut rng = Rng::new(0x36);
    for _ in 0..20000 {
        let (xw, yw, zw) = (
            -rng.range(1, 256),
            -rng.range(1, 256),
            -rng.range(1, 256),
        );
        // Positive coordinates keep `px % wrap` non-negative.
        let (x, y, z) = (
            (rng.range(0, 4000) as f32) + 0.25,
            (rng.range(0, 4000) as f32) + 0.5,
            (rng.range(0, 4000) as f32) + 0.75,
        );
        let s = rng.seed_u8();
        compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
    }
    assert!(d.cases() > 1000, "only {} in-window cases", d.cases());
    d.finish();
}

/// C37: exhaustive seed sweep.
#[test]
fn c37_nonpow2_all_seeds() {
    let mut d = Diff::new("C37 nonpow2 seed 0..=255");
    let mut skipped = 0;
    let coords = [
        (1.5f32, 2.5f32, 3.5f32),
        (0.0, 0.0, 0.0),
        (-1.25, 255.5, 17.0),
        (100.5, -100.5, 0.5),
    ];
    let wraps = [
        (0, 0, 0),
        (3, 5, 7),
        (256, 256, 256),
        (1, 251, 64),
        (97, 13, 11),
    ];
    for &(x, y, z) in &coords {
        for &(xw, yw, zw) in &wraps {
            for s in 0..=255u8 {
                compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
            }
        }
    }
    assert_eq!(skipped, 0);
    d.finish();
}

/// Extra: fully randomised inputs; every input that the classifier calls
/// reproducible must match, and the undefined ones must at least not make the
/// Rust library crash (it is called here, and the process survives).
#[test]
fn nonpow2_fully_random() {
    let mut d = Diff::new("nonpow2 fully randomised (in-window subset)");
    let mut skipped = 0;
    let mut rng = Rng::new(0x37);
    let rust = common::rust_api();
    for _ in 0..30000 {
        let w = |rng: &mut Rng| match rng.below(4) {
            0 => rng.range(1, 256),
            1 => rng.range(-2000, 2000),
            2 => *rng.pick(common::SPECIAL_WRAPS),
            _ => rng.next_i32(),
        };
        let (xw, yw, zw) = (w(&mut rng), w(&mut rng), w(&mut rng));
        let f = |rng: &mut Rng| match rng.below(3) {
            0 => rng.coord(300),
            1 => *rng.pick(common::SPECIAL_F32),
            _ => rng.any_f32(),
        };
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let s = rng.seed_u8();
        if classify_nonpow2(x, y, z, xw, yw, zw, s) == Nonpow2Class::Reproducible {
            compare(&mut d, &mut skipped, x, y, z, xw, yw, zw, s);
        } else {
            skipped += 1;
            // The Rust translation must stay memory-safe for these.
            let v = unsafe { (rust.wrap_nonpow2)(x, y, z, xw, yw, zw, s) };
            let _ = v.to_bits();
        }
    }
    assert!(d.cases() > 1000, "only {} in-window cases", d.cases());
    println!("skipped {skipped} undefined-behaviour inputs");
    d.finish();
}
