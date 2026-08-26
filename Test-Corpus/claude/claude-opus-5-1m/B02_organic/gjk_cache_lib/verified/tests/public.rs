//! Phase B — CONFIGS.md rows 51..56: `gjk_cache`, the only entry point declared
//! in `c_src/include/lib.h`.
//!
//! Note what the C actually does: `gjk_cache` computes four `c2GJK` results into
//! locals and then **discards all of them**. `a9` and `b9` are parameters that
//! are never dereferenced and never written. The function therefore has NO
//! observable output at all, so the differential assertions are:
//!   1. neither library writes through `a9` / `b9` (poison must survive intact),
//!   2. both accept NULL `a9` / `b9` without dereferencing them,
//!   3. neither traps, for any float arguments including NaN/inf.

mod common;
use common::*;
use std::ffi::c_char;

/// A distinctive bit pattern that must survive the call untouched.
const POISON_A: c2v = c2v { x: -1.2345678e-11, y: 9.8765432e21 };
const POISON_B: c2v = c2v { x: 4.2424242e13, y: -8.5858585e-19 };

#[allow(clippy::too_many_arguments)]
fn diff_gjk_cache(ctx: &str, p: &Pair, reverse: c_char, a: [f32; 4], b: [f32; 5]) {
    // Wide buffers so an out-of-bounds write past a9[0] / b9[0] is detected too.
    let mut ac = [POISON_A; 4];
    let mut bc = [POISON_B; 4];
    let mut ar = [POISON_A; 4];
    let mut br = [POISON_B; 4];
    unsafe {
        (p.c.gjk_cache)(reverse, ac.as_mut_ptr(), bc.as_mut_ptr(), a[0], a[1], a[2], a[3],
                        b[0], b[1], b[2], b[3], b[4]);
        (p.r.gjk_cache)(reverse, ar.as_mut_ptr(), br.as_mut_ptr(), a[0], a[1], a[2], a[3],
                        b[0], b[1], b[2], b[3], b[4]);
    }
    // (1) C and Rust agree on the (non-)effect.
    eq_bytes(&format!("{ctx}: a9 buffer"), &ac, &ar);
    eq_bytes(&format!("{ctx}: b9 buffer"), &bc, &br);
    // (2) and the effect is specifically "nothing was written".
    for k in 0..4 {
        eq_v(&format!("{ctx}: C a9[{k}] untouched"), ac[k], POISON_A);
        eq_v(&format!("{ctx}: Rust a9[{k}] untouched"), ar[k], POISON_A);
        eq_v(&format!("{ctx}: C b9[{k}] untouched"), bc[k], POISON_B);
        eq_v(&format!("{ctx}: Rust b9[{k}] untouched"), br[k], POISON_B);
    }
}

// ---------------------------------------------------------------------------
// Rows 51/52: reverse == 0 and reverse != 0
// ---------------------------------------------------------------------------

#[test]
fn row51_reverse_zero() {
    let p = pair();
    let mut rng = Rng::new(0x5151);
    for i in 0..4096 {
        let s = rng.scale_choice();
        let a = [rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s)];
        let b = [rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s).abs()];
        diff_gjk_cache(&format!("row51[{i}]"), p, 0, a, b);
    }
}

#[test]
fn row52_reverse_nonzero() {
    let p = pair();
    let mut rng = Rng::new(0x5252);
    for i in 0..4096 {
        let s = rng.scale_choice();
        let a = [rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s)];
        let b = [rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s).abs()];
        diff_gjk_cache(&format!("row52[{i}]"), p, 1, a, b);
    }
}

// ---------------------------------------------------------------------------
// Row 53: poisoned out-buffers stay poisoned (covered by the helper above, but
// pinned explicitly here with a properly ordered AABB + capsule).
// ---------------------------------------------------------------------------

#[test]
fn row53_out_buffers_never_written() {
    let p = pair();
    // The exact scenario the C's own `main`-style usage implies.
    diff_gjk_cache("row53 canonical", p, 0, [-10.0, -10.0, 10.0, 10.0],
                   [100.0, -25.0, 75.0, 100.0, 10.0]);
    diff_gjk_cache("row53 canonical-rev", p, 1, [-10.0, -10.0, 10.0, 10.0],
                   [100.0, -25.0, 75.0, 100.0, 10.0]);
    // overlapping
    diff_gjk_cache("row53 overlap", p, 0, [-50.0, -50.0, 50.0, 50.0],
                   [0.0, 0.0, 10.0, 10.0, 5.0]);
    diff_gjk_cache("row53 overlap-rev", p, 1, [-50.0, -50.0, 50.0, 50.0],
                   [0.0, 0.0, 10.0, 10.0, 5.0]);
}

// ---------------------------------------------------------------------------
// Row 54: NULL a9 / b9 (never dereferenced by the C)
// ---------------------------------------------------------------------------

#[test]
fn row54_null_out_pointers() {
    let p = pair();
    let mut rng = Rng::new(0x5454);
    let nul = std::ptr::null_mut::<c2v>();
    let mut one = [POISON_A; 2];
    for i in 0..2048 {
        let s = rng.scale_choice();
        let (a1, a2, a3, a4) = (rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s));
        let (b1, b2, b3, b4, b5) =
            (rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s), rng.scaled(s));
        let rev = (i % 2) as c_char;
        unsafe {
            // both NULL
            (p.c.gjk_cache)(rev, nul, nul, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            (p.r.gjk_cache)(rev, nul, nul, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            // a9 NULL only
            (p.c.gjk_cache)(rev, nul, one.as_mut_ptr(), a1, a2, a3, a4, b1, b2, b3, b4, b5);
            (p.r.gjk_cache)(rev, nul, one.as_mut_ptr(), a1, a2, a3, a4, b1, b2, b3, b4, b5);
            // b9 NULL only
            (p.c.gjk_cache)(rev, one.as_mut_ptr(), nul, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            (p.r.gjk_cache)(rev, one.as_mut_ptr(), nul, a1, a2, a3, a4, b1, b2, b3, b4, b5);
        }
        for k in 0..2 {
            eq_v(&format!("row54[{i}] buf[{k}] untouched"), one[k], POISON_A);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 55: degenerate / extreme arguments
// ---------------------------------------------------------------------------

#[test]
fn row55_extreme_arguments() {
    let p = pair();
    let inf = f32::INFINITY;
    let nan = f32::NAN;
    let sub = f32::from_bits(1);
    let cases: Vec<([f32; 4], [f32; 5])> = vec![
        // zero-size box, zero-length capsule, zero radius
        ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0]),
        // inverted box (min > max)
        ([10.0, 10.0, -10.0, -10.0], [0.0, 0.0, 5.0, 5.0, 1.0]),
        // partially inverted
        ([10.0, -10.0, -10.0, 10.0], [0.0, 0.0, 5.0, 5.0, 1.0]),
        // a == b capsule
        ([-1.0, -1.0, 1.0, 1.0], [3.0, 3.0, 3.0, 3.0, 2.0]),
        // negative radius
        ([-1.0, -1.0, 1.0, 1.0], [3.0, 3.0, 5.0, 5.0, -2.0]),
        // huge
        ([-f32::MAX, -f32::MAX, f32::MAX, f32::MAX], [f32::MAX, f32::MAX, -f32::MAX, -f32::MAX, f32::MAX]),
        // infinities
        ([-inf, -inf, inf, inf], [inf, -inf, -inf, inf, inf]),
        ([0.0, 0.0, inf, inf], [1.0, 1.0, 2.0, 2.0, inf]),
        // NaNs in every position
        ([nan, 0.0, 1.0, 1.0], [1.0, 1.0, 2.0, 2.0, 1.0]),
        ([0.0, nan, 1.0, 1.0], [1.0, 1.0, 2.0, 2.0, 1.0]),
        ([0.0, 0.0, nan, 1.0], [1.0, 1.0, 2.0, 2.0, 1.0]),
        ([0.0, 0.0, 1.0, nan], [1.0, 1.0, 2.0, 2.0, 1.0]),
        ([0.0, 0.0, 1.0, 1.0], [nan, 1.0, 2.0, 2.0, 1.0]),
        ([0.0, 0.0, 1.0, 1.0], [1.0, nan, 2.0, 2.0, 1.0]),
        ([0.0, 0.0, 1.0, 1.0], [1.0, 1.0, nan, 2.0, 1.0]),
        ([0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 2.0, nan, 1.0]),
        ([0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 2.0, 2.0, nan]),
        ([nan, nan, nan, nan], [nan, nan, nan, nan, nan]),
        // subnormals and signed zeros
        ([sub, -sub, sub, -sub], [sub, sub, -sub, -sub, sub]),
        ([-0.0, -0.0, 0.0, 0.0], [-0.0, 0.0, 0.0, -0.0, -0.0]),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        for rev in [0i8, 1, -1, 127, -128] {
            diff_gjk_cache(&format!("row55[{i}] rev={rev}"), p, rev as c_char, *a, *b);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 56: randomised sweep, incl. `reverse` truthiness (ERRORS.md row 60)
// ---------------------------------------------------------------------------

#[test]
fn row56_random_sweep() {
    let p = pair();
    let mut rng = Rng::new(0x5656);
    for i in 0..4096 {
        // fully nasty arguments (may contain NaN/inf) — gjk_cache produces no
        // output, so this stays a STRICT "nothing was written" comparison.
        let a = [rng.nasty(), rng.nasty(), rng.nasty(), rng.nasty()];
        let b = [rng.nasty(), rng.nasty(), rng.nasty(), rng.nasty(), rng.nasty()];
        let rev = (rng.next_u32() & 0xff) as u8 as i8;
        diff_gjk_cache(&format!("row56 nasty[{i}] rev={rev}"), p, rev as c_char, a, b);
    }
    // every distinct `char` value for `reverse`
    for rev in i8::MIN..=i8::MAX {
        diff_gjk_cache(
            &format!("row56 rev={rev}"),
            p,
            rev as c_char,
            [-3.0, -4.0, 5.0, 6.0],
            [7.0, -8.0, 9.0, 10.0, 2.0],
        );
    }
}
