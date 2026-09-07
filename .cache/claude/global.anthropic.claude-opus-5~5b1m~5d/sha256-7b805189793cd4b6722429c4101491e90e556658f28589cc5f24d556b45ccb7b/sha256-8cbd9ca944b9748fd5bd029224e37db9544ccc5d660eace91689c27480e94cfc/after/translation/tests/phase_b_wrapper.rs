//! Phase B, Group 5 — `CONFIGS.md` rows 71-80.
//!
//! The `gjk` top-level wrapper (AABB vs capsule), which is this library's
//! driver-equivalent entry point. Both `*a` and `*b` are compared bit-for-bit.

mod common;
use common::*;

fn box_cap(rng: &mut Rng, mode: u8) -> [f32; 9] {
    let cx = rng.uniform(-6.0, 6.0);
    let cy = rng.uniform(-6.0, 6.0);
    let hx = rng.uniform(0.05, 2.0);
    let hy = rng.uniform(0.05, 2.0);
    let gap = match mode {
        0 => rng.uniform(4.0, 25.0),  // separated
        1 => rng.uniform(-1.5, -0.1), // overlapping
        2 => rng.uniform(-0.3, 0.3),  // touching-ish
        _ => 0.0,                     // coincident
    };
    let t = rng.uniform(-3.15, 3.15);
    let (dx, dy) = (t.cos() * gap, t.sin() * gap);
    let r = rng.uniform(0.0, 1.5);
    let lx = rng.uniform(-2.0, 2.0);
    let ly = rng.uniform(-2.0, 2.0);
    [
        cx - hx,
        cy - hy,
        cx + hx,
        cy + hy,
        cx + dx - lx,
        cy + dy - ly,
        cx + dx + lx,
        cy + dy + ly,
        r,
    ]
}

const REVERSES: [i8; 6] = [0, 1, 2, 0x7f, -1, -128];

#[test]
fn row71_72_forward_and_reverse_separated() {
    let mut rng = Rng::new(0x71);
    for i in 0..8000 {
        let f = box_cap(&mut rng, 0);
        diff_gjk_wrap(&format!("r71 i={i}"), 0, true, true, f);
        diff_gjk_wrap(&format!("r72 i={i}"), 1, true, true, f);
    }
}

/// Row 73: every nonzero `char` must behave like `1` (the C tests `if (reverse)`).
#[test]
fn row73_reverse_char_values() {
    let mut rng = Rng::new(0x73);
    let p = api();
    for i in 0..4000 {
        let f = box_cap(&mut rng, (i % 4) as u8);
        for &rv in &REVERSES {
            diff_gjk_wrap(&format!("r73 i={i}"), rv, true, true, f);
        }
        // Additionally: all the nonzero values must agree with each other
        // *within* the C library, and the Rust must reproduce that grouping.
        let c1 = call_gjk_wrap(&p.c, 1, true, true, f);
        let r1 = call_gjk_wrap(&p.r, 1, true, true, f);
        for &rv in &REVERSES[1..] {
            let cn = call_gjk_wrap(&p.c, rv, true, true, f);
            let rn = call_gjk_wrap(&p.r, rv, true, true, f);
            eq_v("r73 C nonzero-group a", &format!("rv={rv}"), c1.a, cn.a);
            eq_v("r73 C nonzero-group b", &format!("rv={rv}"), c1.b, cn.b);
            eq_v("r73 R nonzero-group a", &format!("rv={rv}"), r1.a, rn.a);
            eq_v("r73 R nonzero-group b", &format!("rv={rv}"), r1.b, rn.b);
        }
    }
}

#[test]
fn row74_overlapping() {
    let mut rng = Rng::new(0x74);
    for i in 0..8000 {
        let f = box_cap(&mut rng, 1);
        diff_gjk_wrap(&format!("r74 i={i}"), 0, true, true, f);
        diff_gjk_wrap(&format!("r74 i={i}"), 1, true, true, f);
    }
}

#[test]
fn row75_touching() {
    let mut rng = Rng::new(0x75);
    for i in 0..8000 {
        let f = box_cap(&mut rng, 2);
        diff_gjk_wrap(&format!("r75 i={i}"), 0, true, true, f);
        diff_gjk_wrap(&format!("r75 i={i}"), 1, true, true, f);
        // Coincident too.
        let g = box_cap(&mut rng, 3);
        diff_gjk_wrap(&format!("r75-coincident i={i}"), 0, true, true, g);
        diff_gjk_wrap(&format!("r75-coincident i={i}"), 1, true, true, g);
    }
}

#[test]
fn row76_degenerate() {
    let mut rng = Rng::new(0x76);
    for i in 0..4000 {
        let mut f = box_cap(&mut rng, (i % 4) as u8);
        match i % 5 {
            // zero-extent box (a point)
            0 => {
                f[2] = f[0];
                f[3] = f[1];
            }
            // zero-length capsule (a circle)
            1 => {
                f[6] = f[4];
                f[7] = f[5];
            }
            // zero radius
            2 => f[8] = 0.0,
            // negative radius
            3 => f[8] = -rng.uniform(0.0, 5.0),
            // everything degenerate at the origin
            _ => f = [0.0; 9],
        }
        for &rv in &[0i8, 1] {
            diff_gjk_wrap(&format!("r76 i={i} case={}", i % 5), rv, true, true, f);
        }
    }
}

#[test]
fn row77_inverted_box() {
    let mut rng = Rng::new(0x77);
    for i in 0..6000 {
        let f0 = box_cap(&mut rng, (i % 4) as u8);
        // Swap min/max in x, in y, and in both.
        let variants = [
            [f0[2], f0[1], f0[0], f0[3], f0[4], f0[5], f0[6], f0[7], f0[8]],
            [f0[0], f0[3], f0[2], f0[1], f0[4], f0[5], f0[6], f0[7], f0[8]],
            [f0[2], f0[3], f0[0], f0[1], f0[4], f0[5], f0[6], f0[7], f0[8]],
        ];
        for (vi, f) in variants.iter().enumerate() {
            for &rv in &[0i8, 1] {
                diff_gjk_wrap(&format!("r77 i={i} v={vi}"), rv, true, true, *f);
            }
        }
    }
}

/// Row 78: non-finite arguments, including one NaN/Inf at a time in each of the
/// 9 float positions (so a swapped-parameter bug can't hide).
#[test]
fn row78_non_finite_args() {
    let mut rng = Rng::new(0x78);
    let poisons = [
        f32::NAN,
        f32::from_bits(0xFFC0_4321),
        f32::from_bits(0x7F80_0009),
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        FLT_MAX,
        -FLT_MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ];
    // One poisoned position at a time.
    for i in 0..600 {
        let base = box_cap(&mut rng, (i % 4) as u8);
        for pos in 0..9 {
            for &pv in &poisons {
                let mut f = base;
                f[pos] = pv;
                for &rv in &[0i8, 1] {
                    diff_gjk_wrap(&format!("r78 i={i} pos={pos}"), rv, true, true, f);
                }
            }
        }
    }
    // Fully wild argument tuples.
    for i in 0..8000 {
        let mut f = [0.0f32; 9];
        for k in 0..9 {
            f[k] = rng.wild_f32();
        }
        for &rv in &REVERSES {
            diff_gjk_wrap(&format!("r78-wild i={i}"), rv, true, true, f);
        }
    }
}

/// Row 79: `a` / `b` NULL — the C forwards them to `c2GJK`, which null-checks.
#[test]
fn row79_null_outputs() {
    let mut rng = Rng::new(0x79);
    for i in 0..4000 {
        let f = box_cap(&mut rng, (i % 4) as u8);
        for &rv in &REVERSES {
            for (wa, wb) in [(false, true), (true, false), (false, false)] {
                diff_gjk_wrap(&format!("r79 i={i} out=({wa},{wb})"), rv, wa, wb, f);
            }
        }
    }
}

/// Row 80: large randomized sweep over the raw 9-float argument space.
#[test]
fn row80_large_random_sweep() {
    let mut rng = Rng::new(0x80);
    for i in 0..20000 {
        let f: [f32; 9] = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        diff_gjk_wrap(&format!("r80 i={i}"), 0, true, true, f);
        diff_gjk_wrap(&format!("r80 i={i}"), 1, true, true, f);
    }
    // A second sweep at a much larger / smaller dynamic range.
    for i in 0..8000 {
        let scale = [1.0e-30f32, 1.0e-6, 1.0, 1.0e6, 1.0e30, 1.0e38][i % 6];
        let mut f = [0.0f32; 9];
        for k in 0..8 {
            f[k] = rng.uniform(-1.0, 1.0) * scale;
        }
        f[8] = rng.uniform(0.0, 1.0) * scale;
        diff_gjk_wrap(&format!("r80-scale i={i} s={scale:e}"), 0, true, true, f);
        diff_gjk_wrap(&format!("r80-scale i={i} s={scale:e}"), 1, true, true, f);
    }
}
