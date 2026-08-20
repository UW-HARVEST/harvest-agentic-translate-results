//! Phase B — CONFIGS.md rows 72-79: `gjk()`, the only function in `include/lib.h`.
//!
//! `gjk(reverse, a, b, a1..a4, b1..b5)` builds an AABB from `a1..a4` and a
//! capsule from `b1..b5` and calls `c2GJK` with `use_radius=1`, no transforms,
//! no cache — with the shape ORDER swapped when `reverse` is truthy.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_char;

const POISON_F: f32 = 1.0e-3;

struct W<'a> {
    c: libloading::Symbol<'a, FnGjkWrapper>,
    r: libloading::Symbol<'a, FnGjkWrapper>,
}

impl<'a> W<'a> {
    fn load(l: &'a Pair) -> Self {
        let (c, r) = l.get::<FnGjkWrapper>("gjk");
        W { c, r }
    }

    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    fn check(&self, ctx: &str, rev: c_char, p: [f32; 9]) {
        let poison = f32::from_bits(0xA5A5_A5A5);
        let (mut ca, mut cb) = (V::new(poison, poison), V::new(poison, poison));
        let (mut ra, mut rb) = (V::new(poison, poison), V::new(poison, poison));
        unsafe {
            (self.c)(
                rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
            );
            (self.r)(
                rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
            );
        }
        ck_v!("gjk outA", ca, ra, "{ctx} rev={rev} p={p:?}");
        ck_v!("gjk outB", cb, rb, "{ctx} rev={rev} p={p:?}");
    }

    /// Same, but with NULL out-pointers (row 64 / ERRORS row 64).
    #[track_caller]
    fn check_null_outs(&self, ctx: &str, rev: c_char, p: [f32; 9], null_a: bool, null_b: bool) {
        let poison = f32::from_bits(0xA5A5_A5A5);
        let (mut ca, mut cb) = (V::new(poison, poison), V::new(poison, poison));
        let (mut ra, mut rb) = (V::new(poison, poison), V::new(poison, poison));
        let (cap, cbp) = (
            if null_a { std::ptr::null_mut() } else { &mut ca as *mut V },
            if null_b { std::ptr::null_mut() } else { &mut cb as *mut V },
        );
        let (rap, rbp) = (
            if null_a { std::ptr::null_mut() } else { &mut ra as *mut V },
            if null_b { std::ptr::null_mut() } else { &mut rb as *mut V },
        );
        unsafe {
            (self.c)(rev, cap, cbp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            (self.r)(rev, rap, rbp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        }
        ck_v!("gjk outA (null test)", ca, ra, "{ctx} rev={rev} na={null_a} nb={null_b} p={p:?}");
        ck_v!("gjk outB (null test)", cb, rb, "{ctx} rev={rev} na={null_a} nb={null_b} p={p:?}");
        if null_a {
            assert_eq!(ca.x.to_bits(), poison.to_bits(), "outA must stay untouched");
            assert_eq!(ra.x.to_bits(), poison.to_bits(), "outA must stay untouched");
        }
        if null_b {
            assert_eq!(cb.x.to_bits(), poison.to_bits(), "outB must stay untouched");
            assert_eq!(rb.x.to_bits(), poison.to_bits(), "outB must stay untouched");
        }
    }
}

/// Random parameter tuple for `gjk`: a1..a4 = AABB, b1..b5 = capsule.
fn params(g: &mut Rng, mode: u32) -> [f32; 9] {
    match mode {
        // normalised AABB, ordinary capsule
        0 => {
            let (x0, y0) = (g.range(-50.0, 50.0), g.range(-50.0, 50.0));
            let (w, h) = (g.range(0.1, 30.0), g.range(0.1, 30.0));
            [
                x0,
                y0,
                x0 + w,
                y0 + h,
                g.range(-60.0, 60.0),
                g.range(-60.0, 60.0),
                g.range(-60.0, 60.0),
                g.range(-60.0, 60.0),
                g.range(0.0, 10.0),
            ]
        }
        // grid-snapped: exact touching + support ties
        1 => [
            g.grid(), g.grid(), g.grid(), g.grid(),
            g.grid(), g.grid(), g.grid(), g.grid(),
            g.grid().abs(),
        ],
        // fully unconstrained (inverted boxes, negative radii, NaN, Inf)
        2 => [
            g.mixed(), g.mixed(), g.mixed(), g.mixed(),
            g.mixed(), g.mixed(), g.mixed(), g.mixed(),
            g.mixed(),
        ],
        // overlapping: capsule through the middle of the box
        3 => {
            let (x0, y0) = (g.range(-20.0, 20.0), g.range(-20.0, 20.0));
            let (w, h) = (g.range(1.0, 20.0), g.range(1.0, 20.0));
            let (cx, cy) = (x0 + w * 0.5, y0 + h * 0.5);
            [
                x0, y0, x0 + w, y0 + h,
                cx - g.range(0.0, w), cy - g.range(0.0, h),
                cx + g.range(0.0, w), cy + g.range(0.0, h),
                g.range(0.0, 5.0),
            ]
        }
        // separated: capsule far from the box (radius-correction path)
        4 => {
            let (x0, y0) = (g.range(-10.0, 10.0), g.range(-10.0, 10.0));
            let (w, h) = (g.range(1.0, 5.0), g.range(1.0, 5.0));
            let d = g.range(30.0, 300.0);
            [
                x0, y0, x0 + w, y0 + h,
                x0 + d, y0 + d, x0 + d + g.range(0.0, 5.0), y0 + d + g.range(0.0, 5.0),
                g.range(0.0, 3.0),
            ]
        }
        // degenerate: zero-extent box and/or zero-length capsule
        _ => {
            let (x0, y0) = (g.grid(), g.grid());
            let same_pt = g.below(2) == 0;
            let (cx, cy) = (g.grid(), g.grid());
            [
                x0, y0,
                if g.below(2) == 0 { x0 } else { x0 + g.range(0.0, 5.0) },
                if g.below(2) == 0 { y0 } else { y0 + g.range(0.0, 5.0) },
                cx, cy,
                if same_pt { cx } else { cx + g.range(-5.0, 5.0) },
                if same_pt { cy } else { cy + g.range(-5.0, 5.0) },
                if g.below(3) == 0 { 0.0 } else { g.range(0.0, 5.0) },
            ]
        }
    }
}

/// Row 72 — `reverse = 0`
#[test]
fn row72_forward() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7201);
    for i in 0..100_000 {
        let p = params(&mut g, (i % 6) as u32);
        w.check(&format!("fwd i={i}"), 0, p);
    }
}

/// Row 73 — `reverse = 1`
#[test]
fn row73_reverse() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7301);
    for i in 0..100_000 {
        let p = params(&mut g, (i % 6) as u32);
        w.check(&format!("rev i={i}"), 1, p);
    }
}

/// Row 74 — every `reverse` byte value must be handled identically (C tests
/// truthiness of a `char`, so all 256 values are legal input).
#[test]
fn row74_reverse_all_byte_values() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7401);
    for rev in i8::MIN..=i8::MAX {
        for i in 0..40 {
            let p = params(&mut g, (i % 6) as u32);
            w.check(&format!("rev-byte i={i}"), rev as c_char, p);
        }
    }
}

/// Row 75 — grid-snapped coordinates (exact touching, maximal ties).
#[test]
fn row75_grid_snapped() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7501);
    for i in 0..100_000 {
        let p = params(&mut g, 1);
        w.check(&format!("grid i={i}"), (i % 2) as c_char, p);
    }
    // Exhaustive-ish integer sweep: box [0,2]x[0,2], capsule endpoints over a
    // small integer lattice, radii 0..3.
    for ax in 0..3i32 {
        for ay in 0..3i32 {
            for bx in -1..4i32 {
                for by in -1..4i32 {
                    for r in 0..4i32 {
                        let p = [
                            0.0, 0.0, 2.0, 2.0,
                            ax as f32, ay as f32, bx as f32, by as f32,
                            r as f32,
                        ];
                        w.check("lattice", 0, p);
                        w.check("lattice", 1, p);
                    }
                }
            }
        }
    }
}

/// Row 76 — overlapping (hit path, dist forced to 0).
#[test]
fn row76_overlapping() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7601);
    for i in 0..100_000 {
        let p = params(&mut g, 3);
        w.check(&format!("overlap i={i}"), (i % 2) as c_char, p);
    }
}

/// Row 77 — separated (radius-correction path).
#[test]
fn row77_separated() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7701);
    for i in 0..100_000 {
        let p = params(&mut g, 4);
        w.check(&format!("separated i={i}"), (i % 2) as c_char, p);
    }
}

/// Row 78 — capsule radius 0 / huge / negative.
#[test]
fn row78_radius_classes() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7801);
    let radii: &[f32] = &[
        0.0,
        -0.0,
        POISON_F,
        1.0,
        -1.0,
        -1e6,
        1e6,
        1e30,
        -1e30,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        1.1920929e-7,
        1e-40,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &r in radii {
        for i in 0..3000 {
            let mut p = params(&mut g, (i % 6) as u32);
            p[8] = r;
            w.check(&format!("radius={r:?} i={i}"), (i % 2) as c_char, p);
        }
    }
}

/// Row 79 — degenerate AABB (zero extent, inverted) and degenerate capsule.
#[test]
fn row79_degenerate_shapes() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x7901);

    for i in 0..100_000 {
        let p = params(&mut g, 5);
        w.check(&format!("degenerate i={i}"), (i % 2) as c_char, p);
    }

    // explicit degeneracies
    for i in 0..20_000 {
        let (x, y) = (g.grid(), g.grid());
        let cases: [[f32; 9]; 6] = [
            // zero-extent AABB, zero-length capsule, zero radius
            [x, y, x, y, x, y, x, y, 0.0],
            // zero-extent AABB, zero-length capsule elsewhere
            [x, y, x, y, x + 3.0, y + 4.0, x + 3.0, y + 4.0, 1.0],
            // inverted AABB
            [x + 5.0, y + 5.0, x, y, x, y, x + 1.0, y + 1.0, 1.0],
            // AABB inverted on one axis only
            [x, y + 5.0, x + 5.0, y, x, y, x + 1.0, y + 1.0, 1.0],
            // capsule endpoints identical, huge radius
            [x, y, x + 2.0, y + 2.0, x, y, x, y, 1e6],
            // everything at the origin
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        for (k, p) in cases.iter().enumerate() {
            w.check(&format!("degen-case {k} i={i}"), 0, *p);
            w.check(&format!("degen-case {k} i={i}"), 1, *p);
        }
    }
}

/// Row 64 (ERRORS) — NULL out-pointers in every combination.
#[test]
fn row64_null_out_pointers() {
    let l = libs();
    let w = W::load(l);
    let mut g = Rng::new(0x6401);
    for i in 0..20_000 {
        let p = params(&mut g, (i % 6) as u32);
        for rev in [0 as c_char, 1] {
            w.check_null_outs("null both", rev, p, true, true);
            w.check_null_outs("null a", rev, p, true, false);
            w.check_null_outs("null b", rev, p, false, true);
            w.check_null_outs("null none", rev, p, false, false);
        }
    }
}
