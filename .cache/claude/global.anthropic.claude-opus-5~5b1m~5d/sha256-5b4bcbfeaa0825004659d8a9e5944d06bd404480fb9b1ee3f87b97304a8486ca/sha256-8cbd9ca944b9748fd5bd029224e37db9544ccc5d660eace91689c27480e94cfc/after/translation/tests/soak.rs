//! Property-style soak: a large, fixed-seed random walk over the entire input
//! space of both exported entry points, mixing every value class and every
//! shape. This is what catches value-dependent divergences that a per-row test
//! with a handful of samples can miss.
//!
//! Case count is `SOAK` (default 60_000 per entry point); the seed is fixed, so
//! a failure is always reproducible and prints the exact offending input.

mod common;
use common::*;
use std::ffi::c_int;

fn cases() -> usize {
    std::env::var("SOAK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60_000)
}

/// Pick an element from a randomly chosen value class.
fn wild_f32(rng: &mut Rng) -> f32 {
    match rng.below(12) {
        0 => 0.0,
        1 => -0.0,
        2 => f32::INFINITY,
        3 => f32::NEG_INFINITY,
        4 => f32::from_bits(0x7FC0_0000 | (rng.next_u32() & 0x003F_FFFF)), // qNaN
        5 => f32::from_bits(0x7F80_0000 | ((rng.next_u32() & 0x003F_FFFF) | 1)), // sNaN
        6 => f32::from_bits((rng.next_u32() & 0x807F_FFFF) | 1),           // subnormal
        7 => rng.raw_f32(),
        8 => (rng.range(1e30, 3.4e38)) as f32,
        9 => (rng.range(-3.4e38, -1e30)) as f32,
        10 => rng.range(-1.0, 1.0) as f32,
        _ => rng.range(-1000.0, 1000.0) as f32,
    }
}

fn wild_f64(rng: &mut Rng) -> f64 {
    match rng.below(14) {
        0 => 0.0,
        1 => -0.0,
        2 => f64::INFINITY,
        3 => f64::NEG_INFINITY,
        4 => f64::from_bits(0x7FF8_0000_0000_0000 | (rng.next_u64() & 0x0007_FFFF_FFFF_FFFF)),
        5 => f64::from_bits(0x7FF0_0000_0000_0000 | ((rng.next_u64() & 0x0007_FFFF_FFFF_FFFF) | 1)),
        6 => f64::from_bits((rng.next_u64() & 0x800F_FFFF_FFFF_FFFF) | 1),
        7 => rng.raw_f64(),
        8 => rng.range(1e290, 1e308),
        9 => rng.range(-1e308, -1e290),
        10 => rng.range(1e-308, 1e-290),
        11 => rng.range(0.0, 1.0),
        12 => rng.range(-1.0, 1.0),
        _ => rng.range(-1e6, 1e6),
    }
}

fn wild_len(rng: &mut Rng) -> c_int {
    match rng.below(10) {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => (1 + rng.below(16)) as c_int,
        4 => 15,
        5 => 16,
        6 => 17,
        7 => (1 + rng.below(40)) as c_int,
        8 => (1 + rng.below(200)) as c_int,
        _ => (1 + rng.below(600)) as c_int,
    }
}

#[test]
fn soak_spectral_contrast() {
    let p = load();
    let mut rng = Rng::with_seed(0xB5AD_4ECE_DA10_1234);
    let n = cases();
    for k in 0..n {
        let len = wild_len(&mut rng);
        let ul = len as usize;
        // Correlate the two vectors sometimes, so realistic near-1 contrasts
        // and near-0 contrasts both occur.
        let a: Vec<f32> = (0..ul).map(|_| wild_f32(&mut rng)).collect();
        let b: Vec<f32> = if rng.below(4) == 0 {
            a.iter()
                .map(|&x| x * (rng.range(0.5, 2.0) as f32))
                .collect()
        } else {
            (0..ul).map(|_| wild_f32(&mut rng)).collect()
        };
        let alias = match rng.below(8) {
            0 => Alias::Same,
            1 => Alias::OffsetB(1),
            2 => Alias::OffsetA(1),
            _ => Alias::Disjoint,
        };
        let (a, b) = match alias {
            Alias::OffsetB(_) | Alias::OffsetA(_) => {
                let mut aa = a.clone();
                aa.push(wild_f32(&mut rng));
                (aa.clone(), aa)
            }
            Alias::Same => (a.clone(), a),
            Alias::Disjoint => (a, b),
        };
        diff_spectral(&p, &format!("soak_spectral #{k}"), &a, &b, len, alias);
    }
    println!("soak_spectral_contrast: {n} randomized cases, all byte-identical");
}

#[test]
fn soak_match() {
    let p = load();
    let mut rng = Rng::with_seed(0x1D8E_4B2A_77C0_FFEE);
    let n = cases();
    for k in 0..n {
        let bins = wild_len(&mut rng);
        let ub = bins as usize;
        let t: Vec<f64> = (0..ub).map(|_| wild_f64(&mut rng)).collect();
        let r: Vec<f64> = if rng.below(4) == 0 {
            t.iter().map(|&x| x * rng.range(0.5, 2.0)).collect()
        } else {
            (0..ub).map(|_| wild_f64(&mut rng)).collect()
        };
        let th = match rng.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => f64::NAN,
            6 => rng.raw_f64(),
            _ => rng.range(-2.0, 2.0),
        };
        let same_ptr = rng.below(8) == 0;
        diff_match(&p, &format!("soak_match #{k}"), &t, &r, bins, th, same_ptr);
    }
    println!("soak_match: {n} randomized cases, all byte-identical");
}
