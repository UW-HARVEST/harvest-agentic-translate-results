//! Phase B rows 51-54: the header-declared entry point `gjk_cache`.
//!
//! `gjk_cache`'s C body never dereferences `a9` or `b9` and returns `void`, so
//! its entire observable surface is: (1) it must not write through the two
//! pointers, (2) it must not crash for any input, (3) it must not corrupt
//! adjacent memory. All three are asserted for both implementations, using
//! guard buffers around the out-parameters.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_char;

fn pair() -> Pair {
    load_pair()
}

/// A `c2v` out-parameter surrounded by canary words so any stray write is caught.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct Guarded {
    front: [u32; 4],
    v: c2v,
    back: [u32; 4],
}

fn guarded() -> Guarded {
    Guarded {
        front: [0xDEAD_BEEF; 4],
        v: c2v { x: 1.25, y: -8.5 },
        back: [0xFEED_FACE; 4],
    }
}

#[allow(clippy::too_many_arguments)]
fn gjk_cache_diff(
    p: &Pair,
    reverse: c_char,
    pass_ptrs: bool,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    ctx: &str,
) {
    let mut ac = guarded();
    let mut bc = guarded();
    let mut ar = guarded();
    let mut br = guarded();
    unsafe {
        let (pac, pbc) = if pass_ptrs {
            (&mut ac.v as *mut c2v, &mut bc.v as *mut c2v)
        } else {
            (std::ptr::null_mut(), std::ptr::null_mut())
        };
        (p.c.gjk_cache)(reverse, pac, pbc, a1, a2, a3, a4, b1, b2, b3, b4, b5);
        let (par, pbr) = if pass_ptrs {
            (&mut ar.v as *mut c2v, &mut br.v as *mut c2v)
        } else {
            (std::ptr::null_mut(), std::ptr::null_mut())
        };
        (p.r.gjk_cache)(reverse, par, pbr, a1, a2, a3, a4, b1, b2, b3, b4, b5);
    }
    let full = format!(
        "{ctx} reverse={reverse} ptrs={pass_ptrs} a=({a1},{a2},{a3},{a4}) b=({b1},{b2},{b3},{b4},{b5})"
    );
    // Both sides must agree byte-for-byte...
    ck_b(&ac, &ar, &format!("{full} a9 buffer"));
    ck_b(&bc, &br, &format!("{full} b9 buffer"));
    // ...and both must have left the buffers exactly as handed in.
    assert_eq!(ac, guarded(), "C mutated the a9 buffer :: {full}");
    assert_eq!(bc, guarded(), "C mutated the b9 buffer :: {full}");
    assert_eq!(ar, guarded(), "Rust mutated the a9 buffer :: {full}");
    assert_eq!(br, guarded(), "Rust mutated the b9 buffer :: {full}");
}

const N: usize = 3000;

#[test]
fn row51_gjk_cache_reverse0() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 51);
    for i in 0..N {
        let mag = [1.0f32, 50.0, 1.0e4, 1.0e-4][(i % 4) as usize];
        gjk_cache_diff(
            &p, 0, true,
            rng.sym(mag), rng.sym(mag), rng.sym(mag), rng.sym(mag),
            rng.sym(mag), rng.sym(mag), rng.sym(mag), rng.sym(mag), rng.unit() * mag,
            &format!("row51 i={i}"),
        );
    }
}

#[test]
fn row52_gjk_cache_reverse1() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 52);
    for i in 0..N {
        let mag = [1.0f32, 50.0, 1.0e4, 1.0e-4][(i % 4) as usize];
        gjk_cache_diff(
            &p, 1, true,
            rng.sym(mag), rng.sym(mag), rng.sym(mag), rng.sym(mag),
            rng.sym(mag), rng.sym(mag), rng.sym(mag), rng.sym(mag), rng.unit() * mag,
            &format!("row52 i={i}"),
        );
    }
}

#[test]
fn row53_gjk_cache_other_reverse_values_and_null_ptrs() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 53);
    for &reverse in &[-1i8, 2, 127, -128, 3, -5] {
        for pass_ptrs in [true, false] {
            for i in 0..300 {
                gjk_cache_diff(
                    &p, reverse, pass_ptrs,
                    rng.sym(100.0), rng.sym(100.0), rng.sym(100.0), rng.sym(100.0),
                    rng.sym(100.0), rng.sym(100.0), rng.sym(100.0), rng.sym(100.0),
                    rng.unit() * 20.0,
                    &format!("row53 i={i}"),
                );
            }
        }
    }
}

#[test]
fn row54_gjk_cache_degenerate_and_nonfinite_inputs() {
    let p = pair();
    // Degenerate AABB / capsule geometry.
    let degenerate: [[f32; 9]; 10] = [
        // inverted AABB
        [10.0, 10.0, -10.0, -10.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        // zero-area AABB
        [5.0, 5.0, 5.0, 5.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        // zero-width AABB
        [5.0, -5.0, 5.0, 5.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        // zero-height AABB
        [-5.0, 5.0, 5.0, 5.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        // point capsule
        [-1.0, -1.0, 1.0, 1.0, 3.0, 3.0, 3.0, 3.0, 0.5],
        // zero-radius capsule
        [-1.0, -1.0, 1.0, 1.0, 3.0, 3.0, 4.0, 4.0, 0.0],
        // negative-radius capsule (C does not validate)
        [-1.0, -1.0, 1.0, 1.0, 3.0, 3.0, 4.0, 4.0, -2.0],
        // huge radius
        [-1.0, -1.0, 1.0, 1.0, 3.0, 3.0, 4.0, 4.0, 1.0e30],
        // everything zero
        [0.0; 9],
        // capsule fully inside the AABB
        [-10.0, -10.0, 10.0, 10.0, -1.0, -1.0, 1.0, 1.0, 0.5],
    ];
    for (k, a) in degenerate.iter().enumerate() {
        for reverse in [0i8, 1] {
            for pass_ptrs in [true, false] {
                gjk_cache_diff(&p, reverse, pass_ptrs, a[0], a[1], a[2], a[3], a[4], a[5],
                    a[6], a[7], a[8], &format!("row54 degenerate k={k}"));
            }
        }
    }
    // Non-finite inputs.
    let mut rng = Rng::new(SEED ^ 54);
    for i in 0..N {
        let vals: Vec<f32> = (0..9).map(|_| rng.wild_f32()).collect();
        for reverse in [0i8, 1] {
            gjk_cache_diff(&p, reverse, true, vals[0], vals[1], vals[2], vals[3], vals[4],
                vals[5], vals[6], vals[7], vals[8], &format!("row54 wild i={i}"));
        }
    }
    // Exhaustive placement of one special value in each slot.
    let specials = [
        0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, f32::MAX, f32::MIN,
        f32::MIN_POSITIVE, f32::from_bits(1),
    ];
    for slot in 0..9 {
        for (si, &sp) in specials.iter().enumerate() {
            let mut v = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
            v[slot] = sp;
            for reverse in [0i8, 1] {
                gjk_cache_diff(&p, reverse, true, v[0], v[1], v[2], v[3], v[4], v[5], v[6],
                    v[7], v[8], &format!("row54 slot={slot} special={si}"));
            }
        }
    }
}
