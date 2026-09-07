//! Phase B — valid-path differential tests for the float-heavy entry points.
//!
//! Covers `CONFIGS.md` rows 42–87: `f9` (barycentric), `f10` (half-float),
//! `f11`/`f12`/`f13` (colour spaces) and the composed pipeline. Everything is
//! compared BITWISE (`to_bits()`), so NaN payloads and signed zeros count.

#![allow(non_snake_case)]

mod common;
use common::*;

// ===========================================================================
// Rows 42–47: f9 — barycentric coordinates
// ===========================================================================

fn lm(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

#[track_caller]
fn diff_f9(tag: &str, p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) {
    let a = apis();
    let ctx = (
        Bits(p1.x),
        Bits(p1.y),
        Bits(p2.x),
        Bits(p2.y),
        Bits(p3.x),
        Bits(p3.y),
        Bits(p.x),
        Bits(p.y),
    );
    unsafe {
        eq_lm(tag, &ctx, (a.c.f9)(p1, p2, p3, p), (a.r.f9)(p1, p2, p3, p));
    }
}

/// Rows 42–43 — well-formed triangles, `p` inside / outside / on edge / vertex.
#[test]
fn cfg42_43_f9_nondegenerate() {
    let mut rng = Rng::new();
    for _ in 0..40_000 {
        let p1 = lm(rng.finite_f32(100.0), rng.finite_f32(100.0));
        let p2 = lm(rng.finite_f32(100.0), rng.finite_f32(100.0));
        let p3 = lm(rng.finite_f32(100.0), rng.finite_f32(100.0));
        let p = lm(rng.finite_f32(100.0), rng.finite_f32(100.0));
        diff_f9("f9/random-tri", p1, p2, p3, p);
    }
    // Unit triangle, p at vertices / edge midpoints / centroid / far outside.
    let p1 = lm(0.0, 0.0);
    let p2 = lm(1.0, 0.0);
    let p3 = lm(0.0, 1.0);
    let probes = [
        lm(0.0, 0.0),
        lm(1.0, 0.0),
        lm(0.0, 1.0),
        lm(0.5, 0.0),
        lm(0.0, 0.5),
        lm(0.5, 0.5),
        lm(1.0 / 3.0, 1.0 / 3.0),
        lm(-1.0, -1.0),
        lm(100.0, 100.0),
        lm(-0.0, -0.0),
    ];
    for &p in &probes {
        diff_f9("f9/unit-tri", p1, p2, p3, p);
    }
    // Very small and very large triangles (subnormal / overflow-prone dots).
    for scale in [1e-20f32, 1e-6, 1.0, 1e6, 1e20, f32::MAX] {
        let a = lm(0.0, 0.0);
        let b = lm(scale, 0.0);
        let c = lm(0.0, scale);
        for &p in &probes {
            diff_f9("f9/scaled", a, b, c, p);
            diff_f9(
                "f9/scaled2",
                a,
                b,
                c,
                lm(p.x * scale * 0.5, p.y * scale * 0.5),
            );
        }
    }
}

/// Rows 44–45 — degenerate denominators.
#[test]
fn cfg44_45_f9_degenerate() {
    // All four points identical -> every dot is 0 -> 1/0 = inf, 0*inf = NaN.
    let mut rng = Rng::new();
    for _ in 0..2_000 {
        let q = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        diff_f9("f9/all-equal", q, q, q, q);
    }
    for &v in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN] {
        let q = lm(v, v);
        diff_f9("f9/all-equal-special", q, q, q, q);
    }
    // Collinear p1,p2,p3 -> dot00*dot11 == dot01*dot01 -> denominator 0.
    for _ in 0..20_000 {
        let o = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        let d = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        let (s, t) = (rng.finite_f32(5.0), rng.finite_f32(5.0));
        let p2 = lm(o.x + d.x * s, o.y + d.y * s);
        let p3 = lm(o.x + d.x * t, o.y + d.y * t);
        let p = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        diff_f9("f9/collinear", o, p2, p3, p);
    }
    // Exactly collinear along an axis.
    for k in 1..40 {
        let f = k as f32;
        diff_f9(
            "f9/axis-collinear",
            lm(0.0, 0.0),
            lm(f, 0.0),
            lm(2.0 * f, 0.0),
            lm(1.0, 1.0),
        );
        diff_f9(
            "f9/axis-collinear-y",
            lm(0.0, 0.0),
            lm(0.0, f),
            lm(0.0, 2.0 * f),
            lm(1.0, 1.0),
        );
    }
    // p1 == p2 (v1 == 0) and p1 == p3 (v0 == 0).
    for _ in 0..5_000 {
        let q = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        let o = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        let p = lm(rng.finite_f32(10.0), rng.finite_f32(10.0));
        diff_f9("f9/p1eqp2", q, q, o, p);
        diff_f9("f9/p1eqp3", q, o, q, p);
    }
}

/// Rows 46–47 — every float class, including NaN payload propagation.
#[test]
fn cfg46_47_f9_float_zoo() {
    let zoo = zoo_f32();
    let mut rng = Rng::new();
    // Randomized zoo picks in every one of the 8 coordinate slots.
    for _ in 0..120_000 {
        let p1 = lm(rng.pick(&zoo), rng.pick(&zoo));
        let p2 = lm(rng.pick(&zoo), rng.pick(&zoo));
        let p3 = lm(rng.pick(&zoo), rng.pick(&zoo));
        let p = lm(rng.pick(&zoo), rng.pick(&zoo));
        diff_f9("f9/zoo", p1, p2, p3, p);
    }
    // Fully random bit patterns (dense NaN payload coverage).
    for _ in 0..120_000 {
        let p1 = lm(rng.any_f32(), rng.any_f32());
        let p2 = lm(rng.any_f32(), rng.any_f32());
        let p3 = lm(rng.any_f32(), rng.any_f32());
        let p = lm(rng.any_f32(), rng.any_f32());
        diff_f9("f9/bits", p1, p2, p3, p);
    }
    // One NaN at a time in each of the 8 slots, with distinct payloads.
    for &nan_bits in NAN_ZOO {
        let n = f32::from_bits(nan_bits);
        for slot in 0..8 {
            let mut c = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
            c[slot] = n;
            diff_f9(
                "f9/one-nan",
                lm(c[0], c[1]),
                lm(c[2], c[3]),
                lm(c[4], c[5]),
                lm(c[6], c[7]),
            );
        }
        // Two distinct NaNs simultaneously (payload race between products).
        for &nb2 in NAN_ZOO {
            let n2 = f32::from_bits(nb2);
            diff_f9(
                "f9/two-nan",
                lm(n, 1.0),
                lm(2.0, n2),
                lm(n2, 3.0),
                lm(4.0, n),
            );
        }
    }
}

// ===========================================================================
// Row 48: f10 — exhaustive over all 65536 uint16_t inputs
// ===========================================================================

#[test]
fn cfg48_f10_exhaustive() {
    let a = apis();
    for h in 0u16..=u16::MAX {
        unsafe {
            eq_f32("f10", &h, (a.c.f10)(h), (a.r.f10)(h));
        }
        if h == u16::MAX {
            break;
        }
    }
}

// ===========================================================================
// Rows 49–63: f11 — HSL -> RGB
// ===========================================================================

fn hue_in(rng: &mut Rng, sector: u32) -> f32 {
    // 0:[0,60) 1:[60,120) 2:[120,180) 3:[180,240) 4:[240,300) 5:[300,360)
    // 6: negative      7: >= 360
    let frac = (rng.next_u32() as f64) / (u32::MAX as f64 + 1.0);
    match sector {
        0..=5 => (sector as f64 * 60.0 + frac * 60.0) as f32,
        6 => -((frac * 1000.0) as f32) - f32::MIN_POSITIVE,
        _ => (360.0 + frac * 1000.0) as f32,
    }
}

/// Rows 49–50 — `s == 0` / `s == -0.0` early-out.
#[test]
fn cfg49_50_f11_s_zero() {
    let mut rng = Rng::new();
    let zoo = zoo_f32();
    for &s in &[0.0f32, -0.0] {
        for &l in &zoo {
            for &h in &zoo {
                diff_tri("f11/s0", |a| a.f11, [h, s, l]);
            }
            let _ = l;
        }
        for _ in 0..20_000 {
            diff_tri("f11/s0-rand", |a| a.f11, [rng.any_f32(), s, rng.any_f32()]);
        }
    }
}

/// Rows 51–58 — every hue sector, including the C-bug branch and `h >= 360`.
#[test]
fn cfg51_58_f11_hue_sectors() {
    let mut rng = Rng::new();
    for sector in 0..8u32 {
        for i in 0..40_000u32 {
            let h = hue_in(&mut rng, sector);
            let s = match i % 4 {
                0 => (rng.next_u32() as f64 / (u32::MAX as f64)) as f32,
                1 => rng.finite_f32(3.0),
                2 => rng.small_f32(),
                _ => rng.finite_f32(1e6),
            };
            // s == 0 would take the early-out; keep it out of these rows.
            let s = if s == 0.0 { 0.25 } else { s };
            let l = match i % 5 {
                0 => (rng.next_u32() as f64 / (u32::MAX as f64)) as f32,
                1 => rng.finite_f32(1.0),
                2 => rng.small_f32(),
                3 => rng.finite_f32(1e6),
                _ => rng.finite_f32(1e-20),
            };
            diff_tri("f11/sector", |a| a.f11, [h, s, l]);
        }
        // Exact sector boundaries.
        for &h in &[
            0.0f32, -0.0, 59.999996, 60.0, 60.000004, 119.99999, 120.0, 120.00001, 179.99998,
            180.0, 180.00002, 239.99998, 240.0, 240.00003, 299.99997, 300.0, 300.00003, 359.99997,
            360.0, 360.00003,
        ] {
            for &s in &[1.0f32, 0.5, 0.25, -1.0, 2.0] {
                for &l in &[0.0f32, 0.25, 0.5, 0.75, 1.0, -1.0, 2.0] {
                    diff_tri("f11/boundary", |a| a.f11, [h, s, l]);
                }
            }
        }
    }
}

/// Rows 59–61 — `±inf`, NaN (all payloads), subnormals, fully random bits.
#[test]
fn cfg59_61_f11_float_zoo() {
    let zoo = zoo_f32();
    let mut rng = Rng::new();
    for &h in &zoo {
        for &s in &zoo {
            for &l in &zoo {
                diff_tri("f11/zoo", |a| a.f11, [h, s, l]);
            }
        }
    }
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        for &other in &[0.0f32, 1.0, 0.5, -1.0, f32::INFINITY] {
            diff_tri("f11/nan-h", |a| a.f11, [n, other, other]);
            diff_tri("f11/nan-s", |a| a.f11, [other, n, other]);
            diff_tri("f11/nan-l", |a| a.f11, [other, other, n]);
        }
        for &nb2 in NAN_ZOO {
            let n2 = f32::from_bits(nb2);
            diff_tri("f11/nan-pair", |a| a.f11, [n, n2, 0.5]);
            diff_tri("f11/nan-pair2", |a| a.f11, [30.0, n, n2]);
        }
    }
    for _ in 0..200_000 {
        diff_tri(
            "f11/bits",
            |a| a.f11,
            [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        );
    }
}

/// Rows 62–63 — aliasing: `dest == src`, and ±1-element overlap.
#[test]
fn cfg62_63_f11_aliasing() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..40_000u32 {
        let src: [f32; 3] = if i % 2 == 0 {
            [
                hue_in(&mut rng, i % 8),
                rng.finite_f32(2.0),
                rng.finite_f32(2.0),
            ]
        } else {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
        };
        // dest == src
        let mut bc = src;
        let mut br = src;
        unsafe {
            (a.c.f11)(bc.as_mut_ptr(), bc.as_ptr());
            (a.r.f11)(br.as_mut_ptr(), br.as_ptr());
        }
        eq_tri("f11/alias-same", &BitsTri(src), &bc, &br);

        // 5-element buffer, dest at [0], src at [1] (overlap by 2)
        for off in [1usize, 2] {
            let mut wc = [0.0f32; 6];
            let mut wr = [0.0f32; 6];
            for k in 0..3 {
                wc[off + k] = src[k];
                wr[off + k] = src[k];
            }
            unsafe {
                (a.c.f11)(wc.as_mut_ptr(), wc.as_ptr().add(off));
                (a.r.f11)(wr.as_mut_ptr(), wr.as_ptr().add(off));
            }
            assert_eq!(
                wc.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                wr.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "f11 overlap off={off} src={:?}",
                BitsTri(src)
            );
            // and the mirror: src at [0], dest at [off]
            let mut xc = [0.0f32; 6];
            let mut xr = [0.0f32; 6];
            for k in 0..3 {
                xc[k] = src[k];
                xr[k] = src[k];
            }
            unsafe {
                (a.c.f11)(xc.as_mut_ptr().add(off), xc.as_ptr());
                (a.r.f11)(xr.as_mut_ptr().add(off), xr.as_ptr());
            }
            assert_eq!(
                xc.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                xr.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "f11 mirror overlap off={off} src={:?}",
                BitsTri(src)
            );
        }
    }
}

// ===========================================================================
// Rows 64–75: f12 — HSV -> RGB
// ===========================================================================

/// Row 64 — `s == 0` early-out.
#[test]
fn cfg64_f12_s_zero() {
    let mut rng = Rng::new();
    let zoo = zoo_f32();
    for &s in &[0.0f32, -0.0] {
        for &h in &zoo {
            for &v in &zoo {
                diff_tri("f12/s0", |a| a.f12, [h, s, v]);
            }
        }
        for _ in 0..20_000 {
            diff_tri("f12/s0-rand", |a| a.f12, [rng.any_f32(), s, rng.any_f32()]);
        }
    }
}

/// Rows 65–72 — every `switch (i)` arm, including `default:` via negative /
/// out-of-int-range / NaN hues.
#[test]
fn cfg65_72_f12_switch_arms() {
    let mut rng = Rng::new();
    // i == 0..5 come from h in [0,360); default also from h<0 and h>=360.
    for sector in 0..8u32 {
        for i in 0..40_000u32 {
            let h = hue_in(&mut rng, sector);
            let mut s = match i % 4 {
                0 => (rng.next_u32() as f64 / (u32::MAX as f64)) as f32,
                1 => rng.finite_f32(3.0),
                2 => rng.small_f32(),
                _ => rng.finite_f32(1e6),
            };
            if s == 0.0 {
                s = 0.25;
            }
            let v = match i % 5 {
                0 => (rng.next_u32() as f64 / (u32::MAX as f64)) as f32,
                1 => rng.finite_f32(1.0),
                2 => rng.small_f32(),
                3 => rng.finite_f32(1e6),
                _ => rng.finite_f32(1e-20),
            };
            diff_tri("f12/sector", |a| a.f12, [h, s, v]);
        }
    }
    // Exact i boundaries: h/60 exactly integral, and just below/above.
    for k in -8i32..=10 {
        for d in [-1.0e-5f32, 0.0, 1.0e-5] {
            let h = (k as f32) * 60.0 + d;
            for &s in &[1.0f32, 0.5, -1.0, 2.0] {
                for &v in &[0.0f32, 0.5, 1.0, -1.0, 2.0] {
                    diff_tri("f12/i-boundary", |a| a.f12, [h, s, v]);
                }
            }
        }
    }
    // Row 72: h/60 outside int range, ±inf, NaN -> cvttss2si == INT_MIN.
    let huge = [
        f32::MAX,
        f32::MIN,
        1e30f32,
        -1e30,
        2147483648.0 * 60.0,
        -2147483648.0 * 60.0,
        2147483520.0 * 60.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
    ];
    for &h in &huge {
        for &s in &[1.0f32, 0.5, -1.0, 2.0, f32::NAN, f32::INFINITY] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0, f32::NAN, f32::INFINITY] {
                diff_tri("f12/huge-h", |a| a.f12, [h, s, v]);
            }
        }
    }
}

/// Rows 73–74 — extreme `s`/`v` and fully random bit patterns.
#[test]
fn cfg73_74_f12_float_zoo() {
    let zoo = zoo_f32();
    let mut rng = Rng::new();
    for &h in &zoo {
        for &s in &zoo {
            for &v in &zoo {
                diff_tri("f12/zoo", |a| a.f12, [h, s, v]);
            }
        }
    }
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        for &nb2 in NAN_ZOO {
            let n2 = f32::from_bits(nb2);
            diff_tri("f12/nan-pair", |a| a.f12, [n, n2, 0.5]);
            diff_tri("f12/nan-pair2", |a| a.f12, [90.0, n, n2]);
            diff_tri("f12/nan-pair3", |a| a.f12, [n, 0.5, n2]);
        }
    }
    for _ in 0..200_000 {
        diff_tri(
            "f12/bits",
            |a| a.f12,
            [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        );
    }
}

/// Row 75 — `f12` aliasing.
#[test]
fn cfg75_f12_aliasing() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..40_000u32 {
        let src: [f32; 3] = if i % 2 == 0 {
            [
                hue_in(&mut rng, i % 8),
                rng.finite_f32(2.0),
                rng.finite_f32(2.0),
            ]
        } else {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
        };
        let mut bc = src;
        let mut br = src;
        unsafe {
            (a.c.f12)(bc.as_mut_ptr(), bc.as_ptr());
            (a.r.f12)(br.as_mut_ptr(), br.as_ptr());
        }
        eq_tri("f12/alias-same", &BitsTri(src), &bc, &br);
        for off in [1usize, 2] {
            let mut wc = [0.0f32; 6];
            let mut wr = [0.0f32; 6];
            for k in 0..3 {
                wc[off + k] = src[k];
                wr[off + k] = src[k];
            }
            unsafe {
                (a.c.f12)(wc.as_mut_ptr(), wc.as_ptr().add(off));
                (a.r.f12)(wr.as_mut_ptr(), wr.as_ptr().add(off));
            }
            assert_eq!(
                wc.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                wr.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "f12 overlap off={off} src={:?}",
                BitsTri(src)
            );
        }
    }
}

// ===========================================================================
// Rows 76–86: f13 — RGB -> HSV
// ===========================================================================

/// Rows 76–77 — the `delta == 0 || max == 0` early-out.
#[test]
fn cfg76_77_f13_early_out() {
    let mut rng = Rng::new();
    // delta == 0: r == g == b
    for _ in 0..40_000 {
        let v = match rng.below(4) {
            0 => rng.finite_f32(1.0),
            1 => rng.small_f32(),
            2 => rng.finite_f32(1e20),
            _ => rng.any_f32(),
        };
        diff_tri("f13/delta0", |a| a.f13, [v, v, v]);
    }
    for &v in &zoo_f32() {
        diff_tri("f13/delta0-zoo", |a| a.f13, [v, v, v]);
    }
    // max == 0 with differing components (all <= 0)
    for _ in 0..40_000 {
        let x = -rng.finite_f32(10.0).abs();
        let y = -rng.finite_f32(10.0).abs();
        diff_tri("f13/max0", |a| a.f13, [0.0, x, y]);
        diff_tri("f13/max0b", |a| a.f13, [x, 0.0, y]);
        diff_tri("f13/max0c", |a| a.f13, [x, y, 0.0]);
        diff_tri("f13/max0d", |a| a.f13, [-0.0, x, y]);
    }
    diff_tri("f13/black", |a| a.f13, [0.0, 0.0, 0.0]);
    diff_tri("f13/negzero", |a| a.f13, [-0.0, -0.0, -0.0]);
    diff_tri("f13/mixzero", |a| a.f13, [0.0, -0.0, 0.0]);
}

/// Rows 78–82 — each `max` channel, the negative-hue wrap, and exact ties.
#[test]
fn cfg78_82_f13_channels_and_ties() {
    let mut rng = Rng::new();
    for i in 0..120_000u32 {
        let mk = |rng: &mut Rng| match i % 4 {
            0 => (rng.next_u32() as f64 / (u32::MAX as f64)) as f32,
            1 => rng.finite_f32(1.0),
            2 => rng.small_f32(),
            _ => rng.finite_f32(1e6),
        };
        let (mut r, mut g, mut b) = (mk(&mut rng), mk(&mut rng), mk(&mut rng));
        // Force each channel to be the max in turn, so all three hue formulas
        // and the h<0 wrap are hit densely.
        match i % 3 {
            0 => r = r.abs() + g.abs() + b.abs() + 1.0,
            1 => g = r.abs() + g.abs() + b.abs() + 1.0,
            _ => b = r.abs() + g.abs() + b.abs() + 1.0,
        }
        diff_tri("f13/max-forced", |a| a.f13, [r, g, b]);
        // r == max with g < b -> h < 0 -> += 360
        diff_tri("f13/wrap", |a| a.f13, [10.0, g.abs() * 0.1, g.abs() * 0.5]);
    }
    // Exact ties.
    for _ in 0..20_000 {
        let hi = rng.finite_f32(10.0).abs() + 1.0;
        let lo = rng.finite_f32(10.0).abs() * 0.1;
        diff_tri("f13/tie-rg", |a| a.f13, [hi, hi, lo]);
        diff_tri("f13/tie-gb", |a| a.f13, [lo, hi, hi]);
        diff_tri("f13/tie-rb", |a| a.f13, [hi, lo, hi]);
        diff_tri("f13/tie-neg", |a| a.f13, [-hi, -hi, -lo]);
    }
    // Deterministic grid over a small integer cube (dense exact-equality hits).
    for r in -3i32..=3 {
        for g in -3i32..=3 {
            for b in -3i32..=3 {
                diff_tri(
                    "f13/grid",
                    |a| a.f13,
                    [r as f32 * 0.5, g as f32 * 0.5, b as f32 * 0.5],
                );
            }
        }
    }
}

/// Rows 83–85 — NaN/inf components (min/max ternary asymmetry) and random bits.
#[test]
fn cfg83_85_f13_float_zoo() {
    let zoo = zoo_f32();
    let mut rng = Rng::new();
    for &r in &zoo {
        for &g in &zoo {
            for &b in &zoo {
                diff_tri("f13/zoo", |a| a.f13, [r, g, b]);
            }
        }
    }
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        for &o1 in &[0.0f32, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
            for &o2 in &[0.0f32, 1.0, -1.0, f32::INFINITY] {
                diff_tri("f13/nan0", |a| a.f13, [n, o1, o2]);
                diff_tri("f13/nan1", |a| a.f13, [o1, n, o2]);
                diff_tri("f13/nan2", |a| a.f13, [o1, o2, n]);
            }
        }
        for &nb2 in NAN_ZOO {
            let n2 = f32::from_bits(nb2);
            diff_tri("f13/nan-pair", |a| a.f13, [n, n2, 1.0]);
            diff_tri("f13/nan-pair2", |a| a.f13, [1.0, n, n2]);
            diff_tri("f13/nan-pair3", |a| a.f13, [n, 1.0, n2]);
        }
    }
    for _ in 0..200_000 {
        diff_tri(
            "f13/bits",
            |a| a.f13,
            [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        );
    }
}

/// Row 86 — `f13` aliasing.
#[test]
fn cfg86_f13_aliasing() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..40_000u32 {
        let src: [f32; 3] = if i % 2 == 0 {
            [
                rng.finite_f32(2.0),
                rng.finite_f32(2.0),
                rng.finite_f32(2.0),
            ]
        } else {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
        };
        let mut bc = src;
        let mut br = src;
        unsafe {
            (a.c.f13)(bc.as_mut_ptr(), bc.as_ptr());
            (a.r.f13)(br.as_mut_ptr(), br.as_ptr());
        }
        eq_tri("f13/alias-same", &BitsTri(src), &bc, &br);
        for off in [1usize, 2] {
            let mut wc = [0.0f32; 6];
            let mut wr = [0.0f32; 6];
            for k in 0..3 {
                wc[off + k] = src[k];
                wr[off + k] = src[k];
            }
            unsafe {
                (a.c.f13)(wc.as_mut_ptr(), wc.as_ptr().add(off));
                (a.r.f13)(wr.as_mut_ptr(), wr.as_ptr().add(off));
            }
            assert_eq!(
                wc.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                wr.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "f13 overlap off={off} src={:?}",
                BitsTri(src)
            );
        }
    }
}

// ===========================================================================
// Row 87: composed pipeline f11 -> f13 -> f12 -> f13 ... on the same buffers
// ===========================================================================

#[test]
fn cfg87_composed_colour_pipeline() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..40_000u32 {
        let start: [f32; 3] = match i % 3 {
            0 => [
                hue_in(&mut rng, i % 8),
                (rng.next_u32() as f64 / u32::MAX as f64) as f32,
                (rng.next_u32() as f64 / u32::MAX as f64) as f32,
            ],
            1 => [
                rng.finite_f32(400.0),
                rng.finite_f32(2.0),
                rng.finite_f32(2.0),
            ],
            _ => [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        };

        // HSL -> RGB -> HSV -> RGB -> HSV, chained through both libraries in
        // lockstep. Any divergence anywhere in the chain shows up here.
        let mut c = start;
        let mut r = start;
        for step in 0..4 {
            let mut c2 = [0.0f32; 3];
            let mut r2 = [0.0f32; 3];
            unsafe {
                match step {
                    0 => {
                        (a.c.f11)(c2.as_mut_ptr(), c.as_ptr());
                        (a.r.f11)(r2.as_mut_ptr(), r.as_ptr());
                    }
                    1 => {
                        (a.c.f13)(c2.as_mut_ptr(), c.as_ptr());
                        (a.r.f13)(r2.as_mut_ptr(), r.as_ptr());
                    }
                    2 => {
                        (a.c.f12)(c2.as_mut_ptr(), c.as_ptr());
                        (a.r.f12)(r2.as_mut_ptr(), r.as_ptr());
                    }
                    _ => {
                        (a.c.f13)(c2.as_mut_ptr(), c.as_ptr());
                        (a.r.f13)(r2.as_mut_ptr(), r.as_ptr());
                    }
                }
            }
            eq_tri(
                &format!("pipeline/step{step}"),
                &BitsTri(start),
                &c2,
                &r2,
            );
            c = c2;
            r = r2;
        }
    }
}
