//! Phase B — CONFIGS.md rows 67..71: `gjk_cache`, the one function declared in
//! `c_src/include/lib.h`.
//!
//! Note what the C actually does: it never dereferences `a9` / `b9` and it
//! returns `void`, so its ONLY observable effects are (a) not crashing and
//! (b) leaving `*a9` / `*b9` untouched.  Both are asserted here against
//! pre-poisoned buffers, plus a canary region either side of them to catch a
//! stray write of the wrong width.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_char;

const N: u32 = 4000;

/// A `gjk_cache`-shaped no-op, used to snapshot the pristine buffer contents.
unsafe extern "C" fn noop_gjk_cache(
    _: c_char,
    _: *mut c2v,
    _: *mut c2v,
    _: f32,
    _: f32,
    _: f32,
    _: f32,
    _: f32,
    _: f32,
    _: f32,
    _: f32,
    _: f32,
) {
}

fn drive(f: FnGjkCache, rev: c_char, a9_null: bool, b9_null: bool, p: [f32; 9]) -> Vec<u32> {
    // guard | a9 | guard | b9 | guard
    let mut buf = [c2v { x: 0.0, y: 0.0 }; 5];
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = c2v {
            x: f32::from_bits(0x5AA5_0000 | i as u32),
            y: f32::from_bits(0xA55A_0000 | i as u32),
        };
    }
    let a9 = if a9_null { std::ptr::null_mut() } else { &mut buf[1] as *mut c2v };
    let b9 = if b9_null { std::ptr::null_mut() } else { &mut buf[3] as *mut c2v };
    unsafe {
        f(rev, a9, b9, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
    }
    let mut out = Vec::new();
    for slot in buf.iter() {
        out.push(slot.x.to_bits());
        out.push(slot.y.to_bits());
    }
    out
}

fn check_gjk_cache(ctx: &str, rev: c_char, a9_null: bool, b9_null: bool, p: [f32; 9]) {
    let (c, r) = apis();
    let oc = drive(c.gjk_cache, rev, a9_null, b9_null, p);
    let or = drive(r.gjk_cache, rev, a9_null, b9_null, p);
    if oc != or {
        panic!("DIVERGENCE [{ctx}]\n  rev={rev} p={p:?}\n  C    = {oc:?}\n  RUST = {or:?}");
    }
    // The C never writes through a9/b9: the whole buffer must be pristine.
    let pristine = drive(noop_gjk_cache, rev, a9_null, b9_null, p);
    assert_eq!(oc, pristine, "gjk_cache wrote through a9/b9 [{ctx}]");
}

fn rand_params(rng: &mut Rng, mag: f32) -> [f32; 9] {
    let mut p = [0.0f32; 9];
    for slot in p.iter_mut() {
        *slot = rng.sym(mag);
    }
    p
}

fn spicy_params(rng: &mut Rng, mag: f32) -> [f32; 9] {
    let mut p = [0.0f32; 9];
    for slot in p.iter_mut() {
        *slot = rng.spicy(mag);
    }
    p
}

fn bit_params(rng: &mut Rng) -> [f32; 9] {
    let mut p = [0.0f32; 9];
    for slot in p.iter_mut() {
        *slot = rng.any_bits();
    }
    p
}

// --------------------------------------------------------------- rows 67, 68
#[test]
fn row67_68_reverse_both_ways() {
    let mut rng = Rng::new(0x6067);
    for i in 0..N {
        for rev in [0i8, 1i8] {
            for mag in [1.0f32, 100.0, 1e4] {
                let p = rand_params(&mut rng, mag);
                check_gjk_cache(&format!("row67-68 rev={rev} mag={mag} i={i}"), rev, false, false, p);
            }
            // an AABB + capsule arrangement resembling the C's own demo data
            let p = [-10.0f32, -10.0, 10.0, 10.0, 100.0, -25.0, 75.0, 100.0, 10.0];
            check_gjk_cache(&format!("row67-68 demo rev={rev} i={i}"), rev, false, false, p);
            // degenerate AABB (min == max) and degenerate capsule (a == b)
            let p = [3.0f32, 4.0, 3.0, 4.0, 1.0, 1.0, 1.0, 1.0, 0.0];
            check_gjk_cache(&format!("row67-68 degen rev={rev} i={i}"), rev, false, false, p);
            // inverted AABB
            let p = [10.0f32, 10.0, -10.0, -10.0, 0.0, 0.0, 5.0, 5.0, 2.0];
            check_gjk_cache(&format!("row67-68 inverted rev={rev} i={i}"), rev, false, false, p);
        }
    }
}

// -------------------------------------------------------------------- row 69
#[test]
fn row69_reverse_char_truthiness() {
    let mut rng = Rng::new(0x6069);
    // `char` is signed on x86-64 Linux; cover the whole sign range.
    let revs: [i8; 9] = [0, 1, -1, 2, 127, -128, 0x40, -0x40, 65 /* 'A' */];
    for i in 0..N / 4 {
        for &rev in revs.iter() {
            let p = rand_params(&mut rng, 100.0);
            check_gjk_cache(&format!("row69 rev={rev} i={i}"), rev, false, false, p);
        }
    }
}

// -------------------------------------------------------------------- row 70
#[test]
fn row70_null_out_pointers() {
    let mut rng = Rng::new(0x6070);
    for i in 0..N {
        for rev in [0i8, 1i8] {
            for (an, bn) in [(true, true), (true, false), (false, true)] {
                let p = rand_params(&mut rng, 100.0);
                check_gjk_cache(
                    &format!("row70 rev={rev} a9null={an} b9null={bn} i={i}"),
                    rev,
                    an,
                    bn,
                    p,
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 71
#[test]
fn row71_extreme_params() {
    let mut rng = Rng::new(0x6071);
    for i in 0..N {
        for rev in [0i8, 1i8, -1i8] {
            for mag in [1e-20f32, 1e-6, 1e18, 1e30] {
                let p = rand_params(&mut rng, mag);
                check_gjk_cache(&format!("row71 rev={rev} mag={mag} i={i}"), rev, false, false, p);
            }
            let p = spicy_params(&mut rng, 1e3);
            check_gjk_cache(&format!("row71 spicy rev={rev} i={i}"), rev, false, false, p);
            let p = bit_params(&mut rng);
            check_gjk_cache(&format!("row71 bits rev={rev} i={i}"), rev, false, false, p);
            // all-NaN and all-inf
            check_gjk_cache(&format!("row71 nan rev={rev} i={i}"), rev, false, false, [f32::NAN; 9]);
            check_gjk_cache(
                &format!("row71 inf rev={rev} i={i}"),
                rev,
                false,
                false,
                [f32::INFINITY; 9],
            );
            check_gjk_cache(
                &format!("row71 zero rev={rev} i={i}"),
                rev,
                false,
                false,
                [0.0f32; 9],
            );
        }
    }
}
