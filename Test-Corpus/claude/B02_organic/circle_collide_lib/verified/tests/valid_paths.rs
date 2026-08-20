//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are reached only through `dlopen`/`dlsym`, so the
//! `extern "C"` wrappers, the `#[repr(C)]` layouts and the SysV struct-passing
//! classes are all part of what is compared.

mod common;

use common::*;
use std::os::raw::c_void;

const N: usize = 20_000;

// ===========================================================================
// C1 — c2V
// ===========================================================================

#[test]
fn c1_c2v_passthrough() {
    let (c, r) = libs();
    let mut rng = Rng::new(1);

    // Hand-picked value classes first.
    for &x in SPECIALS.iter().chain(NANS.iter()) {
        for &y in SPECIALS.iter().chain(NANS.iter()) {
            assert_v_bits(
                (c.c2V)(x, y),
                (r.c2V)(x, y),
                &format!("c2V({}, {})", fmt_f32(x), fmt_f32(y)),
            );
        }
    }
    // Then fully random 32-bit patterns.
    for _ in 0..N {
        let x = rng.bit_f32();
        let y = rng.bit_f32();
        assert_v_bits(
            (c.c2V)(x, y),
            (r.c2V)(x, y),
            &format!("c2V({}, {})", fmt_f32(x), fmt_f32(y)),
        );
    }
}

// ===========================================================================
// C2 / C3 — c2Mulvs
// ===========================================================================

#[test]
fn c2_mulvs_value_classes() {
    let (c, r) = libs();
    let mut rng = Rng::new(2);
    let scales: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        0.5,
        f32::MIN_POSITIVE,
        1.0e-45,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &s in scales {
        for _ in 0..500 {
            let a = C2v::new(rng.any_f32(), rng.any_f32());
            assert_v_bits(
                (c.c2Mulvs)(a, s),
                (r.c2Mulvs)(a, s),
                &format!("c2Mulvs({}, {})", fmt_v(a), fmt_f32(s)),
            );
        }
    }
    for _ in 0..N {
        let a = C2v::new(rng.tame_f32(200.0), rng.tame_f32(200.0));
        let s = rng.tame_f32(200.0);
        assert_v_bits(
            (c.c2Mulvs)(a, s),
            (r.c2Mulvs)(a, s),
            &format!("c2Mulvs({}, {})", fmt_v(a), fmt_f32(s)),
        );
    }
}

#[test]
fn c3_mulvs_nan_matrix() {
    let (c, r) = libs();
    let mut rng = Rng::new(3);

    // NaN in a only / in b only / in both — exhaustive over the NaN table.
    for &nx in NANS.iter() {
        for &ny in NANS.iter() {
            for &s in NANS.iter().chain(SPECIALS.iter()) {
                let a = C2v::new(nx, ny);
                assert_v_bits(
                    (c.c2Mulvs)(a, s),
                    (r.c2Mulvs)(a, s),
                    &format!("c2Mulvs({}, {}) [both-NaN]", fmt_v(a), fmt_f32(s)),
                );
            }
        }
    }
    for &s in NANS.iter() {
        for &x in SPECIALS.iter() {
            for &y in SPECIALS.iter() {
                let a = C2v::new(x, y);
                assert_v_bits(
                    (c.c2Mulvs)(a, s),
                    (r.c2Mulvs)(a, s),
                    &format!("c2Mulvs({}, {}) [NaN scale]", fmt_v(a), fmt_f32(s)),
                );
            }
        }
    }
    // Random NaN payloads on both sides.
    for _ in 0..N {
        let a = C2v::new(rng.bit_f32(), rng.bit_f32());
        let s = rng.bit_f32();
        assert_v_bits(
            (c.c2Mulvs)(a, s),
            (r.c2Mulvs)(a, s),
            &format!("c2Mulvs({}, {})", fmt_v(a), fmt_f32(s)),
        );
    }
}

// ===========================================================================
// C4 / C5 — c2Maxv / c2Minv
// ===========================================================================

fn minmax_matrix(
    name: &str,
    f: extern "C" fn(C2v, C2v) -> C2v,
    g: extern "C" fn(C2v, C2v) -> C2v,
    stream: u64,
) {
    let mut rng = Rng::new(stream);
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();
    // Exhaustive over the value pool on the x lane (y lane driven independently).
    for &ax in pool.iter() {
        for &bx in pool.iter() {
            let ay = pool[rng.below(pool.len() as u32) as usize];
            let by = pool[rng.below(pool.len() as u32) as usize];
            let a = C2v::new(ax, ay);
            let b = C2v::new(bx, by);
            assert_v_bits(
                f(a, b),
                g(a, b),
                &format!("{name}({}, {})", fmt_v(a), fmt_v(b)),
            );
        }
    }
    // Orderings a<b, a>b, a==b via tame values plus shared values.
    for _ in 0..N {
        let a = rng.tame_v(200.0);
        let b = if rng.below(4) == 0 {
            a // exact ties, including -0.0 vs -0.0
        } else if rng.below(4) == 0 {
            C2v::new(-a.x, -a.y)
        } else {
            rng.tame_v(200.0)
        };
        assert_v_bits(
            f(a, b),
            g(a, b),
            &format!("{name}({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    // +0.0 / -0.0 cross product (the `?:` picks the second operand on a tie).
    for &ax in &[0.0f32, -0.0f32] {
        for &ay in &[0.0f32, -0.0f32] {
            for &bx in &[0.0f32, -0.0f32] {
                for &by in &[0.0f32, -0.0f32] {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_v_bits(
                        f(a, b),
                        g(a, b),
                        &format!("{name}({}, {}) [signed zeros]", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
    // Fully random bit patterns.
    for _ in 0..N {
        let a = rng.bit_v();
        let b = rng.bit_v();
        assert_v_bits(
            f(a, b),
            g(a, b),
            &format!("{name}({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
}

#[test]
fn c4_maxv_matrix() {
    let (c, r) = libs();
    minmax_matrix("c2Maxv", c.c2Maxv, r.c2Maxv, 4);
}

#[test]
fn c5_minv_matrix() {
    let (c, r) = libs();
    minmax_matrix("c2Minv", c.c2Minv, r.c2Minv, 5);
}

// ===========================================================================
// C6 / C7 — c2Clampv
// ===========================================================================

fn region(v: f32, lo: f32, hi: f32) -> &'static str {
    if v < lo {
        "below"
    } else if v > hi {
        "above"
    } else {
        "inside"
    }
}

const REGIONS: [&str; 9] = [
    "below/below",
    "below/inside",
    "below/above",
    "inside/below",
    "inside/inside",
    "inside/above",
    "above/below",
    "above/inside",
    "above/above",
];

#[test]
fn c6_clampv_all_regions() {
    let (c, r) = libs();
    let mut rng = Rng::new(6);
    let mut cov = Cover::new("c2Clampv regions", &REGIONS);

    for _ in 0..N {
        // Well-formed bounds: lo <= hi on both axes.
        let l = rng.tame_v(100.0);
        let h = C2v::new(l.x + rng.unit() * 100.0, l.y + rng.unit() * 100.0);
        let a = rng.tame_v(200.0);
        let key = format!("{}/{}", region(a.x, l.x, h.x), region(a.y, l.y, h.y));
        cov.hit(&key);
        assert_v_bits(
            (c.c2Clampv)(a, l, h),
            (r.c2Clampv)(a, l, h),
            &format!("c2Clampv({}, {}, {}) [{key}]", fmt_v(a), fmt_v(l), fmt_v(h)),
        );
    }
    cov.require_all(10);
}

#[test]
fn c7_clampv_degenerate_bounds() {
    let (c, r) = libs();
    let mut rng = Rng::new(7);

    // Inverted bounds (lo > hi) on x only, y only, and both.
    for mode in 0..4u32 {
        for _ in 0..2_000 {
            let mut l = rng.tame_v(100.0);
            let mut h = C2v::new(l.x + rng.unit() * 50.0, l.y + rng.unit() * 50.0);
            if mode & 1 != 0 {
                std::mem::swap(&mut l.x, &mut h.x);
            }
            if mode & 2 != 0 {
                std::mem::swap(&mut l.y, &mut h.y);
            }
            let a = rng.tame_v(200.0);
            assert_v_bits(
                (c.c2Clampv)(a, l, h),
                (r.c2Clampv)(a, l, h),
                &format!(
                    "c2Clampv({}, {}, {}) [inverted mode {mode}]",
                    fmt_v(a),
                    fmt_v(l),
                    fmt_v(h)
                ),
            );
        }
    }
    // Zero-width bounds (lo == hi).
    for _ in 0..2_000 {
        let l = rng.tame_v(100.0);
        let a = rng.tame_v(200.0);
        assert_v_bits(
            (c.c2Clampv)(a, l, l),
            (r.c2Clampv)(a, l, l),
            &format!("c2Clampv({}, {0}, {1}) [zero width]", fmt_v(a), fmt_v(l)),
        );
    }
    // NaN / inf bounds and values, plus fully random patterns.
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();
    for _ in 0..N {
        let pick = |rng: &mut Rng| -> f32 {
            if rng.below(2) == 0 {
                pool[rng.below(pool.len() as u32) as usize]
            } else {
                rng.bit_f32()
            }
        };
        let a = C2v::new(pick(&mut rng), pick(&mut rng));
        let l = C2v::new(pick(&mut rng), pick(&mut rng));
        let h = C2v::new(pick(&mut rng), pick(&mut rng));
        assert_v_bits(
            (c.c2Clampv)(a, l, h),
            (r.c2Clampv)(a, l, h),
            &format!(
                "c2Clampv({}, {}, {}) [extremes]",
                fmt_v(a),
                fmt_v(l),
                fmt_v(h)
            ),
        );
    }
}

// ===========================================================================
// C8 — c2Sub
// ===========================================================================

#[test]
fn c8_sub_value_classes() {
    let (c, r) = libs();
    let mut rng = Rng::new(8);
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();

    // Exhaustive x-lane cross product (covers inf-inf, ±0-±0, NaN-NaN).
    for &ax in pool.iter() {
        for &bx in pool.iter() {
            for &ay in pool.iter() {
                let by = pool[rng.below(pool.len() as u32) as usize];
                let a = C2v::new(ax, ay);
                let b = C2v::new(bx, by);
                assert_v_bits(
                    (c.c2Sub)(a, b),
                    (r.c2Sub)(a, b),
                    &format!("c2Sub({}, {})", fmt_v(a), fmt_v(b)),
                );
            }
        }
    }
    for _ in 0..N {
        let a = rng.tame_v(200.0);
        let b = if rng.below(4) == 0 { a } else { rng.tame_v(200.0) };
        assert_v_bits(
            (c.c2Sub)(a, b),
            (r.c2Sub)(a, b),
            &format!("c2Sub({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    for _ in 0..N {
        let a = rng.bit_v();
        let b = rng.bit_v();
        assert_v_bits(
            (c.c2Sub)(a, b),
            (r.c2Sub)(a, b),
            &format!("c2Sub({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
}

// ===========================================================================
// C9 / C10 — c2Dot
// ===========================================================================

#[test]
fn c9_dot_value_classes() {
    let (c, r) = libs();
    let mut rng = Rng::new(9);

    for _ in 0..N {
        let a = rng.tame_v(200.0);
        let b = rng.tame_v(200.0);
        assert_f32_bits(
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
            &format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    // Mixed magnitudes: overflow, underflow and catastrophic cancellation
    // (a.x*b.x == -(a.y*b.y) exactly).
    for _ in 0..N {
        let m = [1.0e-38f32, 1.0e-20, 1.0, 1.0e20, 1.0e38][rng.below(5) as usize];
        let n = [1.0e-38f32, 1.0e-20, 1.0, 1.0e20, 1.0e38][rng.below(5) as usize];
        let a = C2v::new(rng.sym(1.0) * m, rng.sym(1.0) * n);
        let b = C2v::new(rng.sym(1.0) * n, rng.sym(1.0) * m);
        assert_f32_bits(
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
            &format!("c2Dot({}, {}) [mixed magnitudes]", fmt_v(a), fmt_v(b)),
        );
    }
    for _ in 0..N {
        let t = rng.tame_f32(1.0e19);
        let u = rng.tame_f32(1.0e19);
        let a = C2v::new(t, u);
        let b = C2v::new(u, -t); // exact cancellation to +/-0
        assert_f32_bits(
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
            &format!("c2Dot({}, {}) [cancellation]", fmt_v(a), fmt_v(b)),
        );
    }
    for _ in 0..N {
        let a = C2v::new(rng.any_f32(), rng.any_f32());
        let b = C2v::new(rng.any_f32(), rng.any_f32());
        assert_f32_bits(
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
            &format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
}

#[test]
fn c10_dot_nan_matrix() {
    let (c, r) = libs();
    let mut rng = Rng::new(10);
    let pool: Vec<f32> = NANS.iter().chain(SPECIALS.iter()).copied().collect();

    // Exhaustive over (a.x, b.x) with the y lane swept too — this is where the
    // per-term SSE first-operand payload priority shows up.
    for &ax in pool.iter() {
        for &bx in pool.iter() {
            for &ay in pool.iter() {
                for &by in pool.iter() {
                    let a = C2v::new(ax, ay);
                    let b = C2v::new(bx, by);
                    assert_f32_bits(
                        (c.c2Dot)(a, b),
                        (r.c2Dot)(a, b),
                        &format!("c2Dot({}, {}) [nan matrix]", fmt_v(a), fmt_v(b)),
                    );
                }
            }
        }
    }
    // Random NaN payloads.
    for _ in 0..N {
        let a = rng.bit_v();
        let b = rng.bit_v();
        assert_f32_bits(
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
            &format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
}

// ===========================================================================
// C11 / C12 / C13 — c2CircletoCircle
// ===========================================================================

#[test]
fn c11_circle_circle_random() {
    let (c, r) = libs();
    let mut rng = Rng::new(11);
    let mut cov = Cover::new("circle/circle outcome", &["hit", "miss"]);

    for _ in 0..N {
        let a = C2Circle {
            p: rng.tame_v(200.0),
            r: rng.unit() * 60.0,
        };
        let b = C2Circle {
            p: rng.tame_v(200.0),
            r: rng.unit() * 60.0,
        };
        let cr = (c.c2CircletoCircle)(a, b);
        let rr = (r.c2CircletoCircle)(a, b);
        assert_int(cr, rr, &format!("c2CircletoCircle({a:?}, {b:?})"));
        cov.hit(if cr != 0 { "hit" } else { "miss" });
    }
    // Near-boundary sampling: place B at (sum of radii) * (1 +/- epsilon).
    for _ in 0..N {
        let ar = rng.unit() * 50.0 + 0.5;
        let br = rng.unit() * 50.0 + 0.5;
        let ang = rng.unit() * std::f32::consts::TAU;
        let scale = 1.0 + (rng.unit() - 0.5) * 1.0e-3;
        let d = (ar + br) * scale;
        let ap = rng.tame_v(50.0);
        let a = C2Circle { p: ap, r: ar };
        let b = C2Circle {
            p: C2v::new(ap.x + d * ang.cos(), ap.y + d * ang.sin()),
            r: br,
        };
        let cr = (c.c2CircletoCircle)(a, b);
        assert_int(
            cr,
            (r.c2CircletoCircle)(a, b),
            &format!("c2CircletoCircle boundary({a:?}, {b:?})"),
        );
        cov.hit(if cr != 0 { "hit" } else { "miss" });
    }
    cov.require_all(100);
}

#[test]
fn c12_circle_circle_degenerate() {
    let (c, r) = libs();
    let mut rng = Rng::new(12);

    // Exact tangency (integers, exactly representable): d2 == r2 => 0.
    for k in 1..64u32 {
        let ar = k as f32;
        let br = (k * 3) as f32;
        let d = ar + br;
        let a = C2Circle {
            p: C2v::new(0.0, 0.0),
            r: ar,
        };
        let b = C2Circle {
            p: C2v::new(d, 0.0),
            r: br,
        };
        assert_int(
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
            &format!("tangent k={k}"),
        );
    }
    // Zero / negative / opposite radii, coincident centres, nesting.
    let radii: &[f32] = &[0.0, -0.0, 1.0, -1.0, 20.0, -20.0, 1.0e30, -1.0e30];
    for &ar in radii {
        for &br in radii {
            for _ in 0..200 {
                let p = rng.tame_v(60.0);
                let q = match rng.below(3) {
                    0 => p,                                            // coincident
                    1 => C2v::new(p.x + rng.sym(0.5), p.y + rng.sym(0.5)), // nested
                    _ => rng.tame_v(60.0),
                };
                let a = C2Circle { p, r: ar };
                let b = C2Circle { p: q, r: br };
                assert_int(
                    (c.c2CircletoCircle)(a, b),
                    (r.c2CircletoCircle)(a, b),
                    &format!("c2CircletoCircle degenerate({a:?}, {b:?})"),
                );
            }
        }
    }
}

#[test]
fn c13_circle_circle_extremes() {
    let (c, r) = libs();
    let mut rng = Rng::new(13);
    for _ in 0..N {
        let a = C2Circle {
            p: C2v::new(rng.any_f32(), rng.any_f32()),
            r: rng.any_f32(),
        };
        let b = C2Circle {
            p: C2v::new(rng.any_f32(), rng.any_f32()),
            r: rng.any_f32(),
        };
        assert_int(
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
            &format!("c2CircletoCircle extremes({a:?}, {b:?})"),
        );
    }
    for _ in 0..N {
        let a = C2Circle {
            p: rng.bit_v(),
            r: rng.bit_f32(),
        };
        let b = C2Circle {
            p: rng.bit_v(),
            r: rng.bit_f32(),
        };
        assert_int(
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
            &format!("c2CircletoCircle bits({a:?}, {b:?})"),
        );
    }
}

// ===========================================================================
// C14 / C15 / C16 — c2CircletoAABB
// ===========================================================================

#[test]
fn c14_circle_aabb_all_regions() {
    let (c, r) = libs();
    let mut rng = Rng::new(14);
    let mut cov = Cover::new("aabb regions", &REGIONS);
    let mut outcome = Cover::new("aabb outcome", &["hit", "miss"]);

    for _ in 0..N {
        let min = rng.tame_v(100.0);
        let max = C2v::new(min.x + rng.unit() * 80.0, min.y + rng.unit() * 80.0);
        let bb = C2Aabb { min, max };
        let p = rng.tame_v(200.0);
        // Radius spread so both hit and miss occur in every region.
        let rad = match rng.below(4) {
            0 => rng.unit() * 2.0,
            1 => rng.unit() * 40.0,
            2 => rng.unit() * 300.0,
            _ => rng.unit() * 100.0,
        };
        let a = C2Circle { p, r: rad };
        let key = format!(
            "{}/{}",
            region(p.x, min.x, max.x),
            region(p.y, min.y, max.y)
        );
        cov.hit(&key);
        let cr = (c.c2CircletoAABB)(a, bb);
        assert_int(
            cr,
            (r.c2CircletoAABB)(a, bb),
            &format!("c2CircletoAABB({a:?}, {bb:?}) [{key}]"),
        );
        outcome.hit(if cr != 0 { "hit" } else { "miss" });
    }
    cov.require_all(10);
    outcome.require_all(100);

    // Near-boundary: distance to the clamped point exactly ~= r.
    for _ in 0..N {
        let min = C2v::new(-20.0, -20.0);
        let max = C2v::new(20.0, 20.0);
        let bb = C2Aabb { min, max };
        let rad = rng.unit() * 30.0 + 0.25;
        // Put the centre straight out from a face at distance rad*(1 +/- eps).
        let scale = 1.0 + (rng.unit() - 0.5) * 1.0e-3;
        let p = match rng.below(4) {
            0 => C2v::new(20.0 + rad * scale, rng.sym(20.0)),
            1 => C2v::new(-20.0 - rad * scale, rng.sym(20.0)),
            2 => C2v::new(rng.sym(20.0), 20.0 + rad * scale),
            _ => C2v::new(rng.sym(20.0), -20.0 - rad * scale),
        };
        let a = C2Circle { p, r: rad };
        assert_int(
            (c.c2CircletoAABB)(a, bb),
            (r.c2CircletoAABB)(a, bb),
            &format!("c2CircletoAABB boundary({a:?}, {bb:?})"),
        );
    }
}

#[test]
fn c15_circle_aabb_degenerate() {
    let (c, r) = libs();
    let mut rng = Rng::new(15);

    // mode bit0: invert x, bit1: invert y, plus point boxes (min == max).
    for mode in 0..5u32 {
        for _ in 0..4_000 {
            let mut min = rng.tame_v(80.0);
            let mut max = if mode == 4 {
                min
            } else {
                C2v::new(min.x + rng.unit() * 60.0, min.y + rng.unit() * 60.0)
            };
            if mode & 1 != 0 {
                std::mem::swap(&mut min.x, &mut max.x);
            }
            if mode & 2 != 0 {
                std::mem::swap(&mut min.y, &mut max.y);
            }
            let bb = C2Aabb { min, max };
            let rad = [0.0f32, -0.0, -5.0, 5.0, 1.0e-30, 1.0e30][rng.below(6) as usize];
            let a = C2Circle {
                p: rng.tame_v(120.0),
                r: rad,
            };
            assert_int(
                (c.c2CircletoAABB)(a, bb),
                (r.c2CircletoAABB)(a, bb),
                &format!("c2CircletoAABB degenerate mode={mode} ({a:?}, {bb:?})"),
            );
        }
    }
}

#[test]
fn c16_circle_aabb_extremes() {
    let (c, r) = libs();
    let mut rng = Rng::new(16);
    for _ in 0..N {
        let a = C2Circle {
            p: C2v::new(rng.any_f32(), rng.any_f32()),
            r: rng.any_f32(),
        };
        let bb = C2Aabb {
            min: C2v::new(rng.any_f32(), rng.any_f32()),
            max: C2v::new(rng.any_f32(), rng.any_f32()),
        };
        assert_int(
            (c.c2CircletoAABB)(a, bb),
            (r.c2CircletoAABB)(a, bb),
            &format!("c2CircletoAABB extremes({a:?}, {bb:?})"),
        );
    }
    for _ in 0..N {
        let a = C2Circle {
            p: rng.bit_v(),
            r: rng.bit_f32(),
        };
        let bb = C2Aabb {
            min: rng.bit_v(),
            max: rng.bit_v(),
        };
        assert_int(
            (c.c2CircletoAABB)(a, bb),
            (r.c2CircletoAABB)(a, bb),
            &format!("c2CircletoAABB bits({a:?}, {bb:?})"),
        );
    }
}

// ===========================================================================
// C17 / C18 / C19 / C20 / C21 — c2CircletoCapsule
// ===========================================================================

/// Which arm of `c2CircletoCapsule` a given input takes, computed with the C
/// library's *own* `c2Sub`/`c2Dot` so the classification is exact.
fn capsule_arm(api: &Api, a: C2Circle, b: C2Capsule) -> u8 {
    let n = (api.c2Sub)(b.b, b.a);
    let ap = (api.c2Sub)(a.p, b.a);
    let da = (api.c2Dot)(ap, n);
    if da < 0.0 {
        return 1;
    }
    let db = (api.c2Dot)((api.c2Sub)(a.p, b.b), n);
    if db < 0.0 {
        2
    } else {
        3
    }
}

/// Random capsule + circle, biased so that all three arms occur often.
fn random_capsule_case(rng: &mut Rng) -> (C2Circle, C2Capsule) {
    let a0 = rng.tame_v(80.0);
    let len = rng.unit() * 120.0 + 0.001;
    let ang = rng.unit() * std::f32::consts::TAU;
    let b0 = match rng.below(6) {
        0 => C2v::new(a0.x + len, a0.y),        // horizontal
        1 => C2v::new(a0.x, a0.y + len),        // vertical
        2 => C2v::new(a0.x + len, a0.y + len),  // diagonal
        3 => a0,                                // zero length (degenerate)
        _ => C2v::new(a0.x + len * ang.cos(), a0.y + len * ang.sin()),
    };
    // Query point along/around the segment so every arm is reachable.
    let t = rng.unit() * 2.4 - 0.7;
    let px = a0.x + (b0.x - a0.x) * t + rng.sym(30.0);
    let py = a0.y + (b0.y - a0.y) * t + rng.sym(30.0);
    let cap = C2Capsule {
        a: a0,
        b: b0,
        r: rng.unit() * 30.0,
    };
    let cir = C2Circle {
        p: C2v::new(px, py),
        r: rng.unit() * 30.0,
    };
    (cir, cap)
}

fn capsule_arm_test(arm: u8, stream: u64, label: &str) {
    let (c, r) = libs();
    let mut rng = Rng::new(stream);
    let mut outcome = Cover::new("capsule outcome", &["hit", "miss"]);
    let mut accepted = 0usize;

    for _ in 0..(N * 8) {
        let (cir, cap) = random_capsule_case(&mut rng);
        if capsule_arm(c, cir, cap) != arm {
            continue;
        }
        // Same input must select the same arm in the Rust build, too.
        assert_eq!(
            capsule_arm(r, cir, cap),
            arm,
            "{label}: arm classification diverged for ({cir:?}, {cap:?})"
        );
        let cr = (c.c2CircletoCapsule)(cir, cap);
        assert_int(
            cr,
            (r.c2CircletoCapsule)(cir, cap),
            &format!("{label} c2CircletoCapsule({cir:?}, {cap:?})"),
        );
        outcome.hit(if cr != 0 { "hit" } else { "miss" });
        accepted += 1;
        if accepted >= N {
            break;
        }
    }
    assert!(
        accepted >= 2_000,
        "{label}: only {accepted} samples reached arm {arm}"
    );
    outcome.require_all(50);
}

#[test]
fn c17_capsule_arm_before_a() {
    capsule_arm_test(1, 17, "arm1 (da<0)");
}

#[test]
fn c18_capsule_arm_middle() {
    capsule_arm_test(2, 18, "arm2 (da>=0, db<0)");
}

#[test]
fn c19_capsule_arm_beyond_b() {
    capsule_arm_test(3, 19, "arm3 (da>=0, db>=0)");
}

#[test]
fn c20_capsule_random_mixed() {
    let (c, r) = libs();
    let mut rng = Rng::new(20);
    let mut arms = Cover::new("capsule arms", &["1", "2", "3"]);
    let mut outcome = Cover::new("capsule outcome", &["hit", "miss"]);

    for _ in 0..N {
        let (cir, cap) = random_capsule_case(&mut rng);
        let arm = capsule_arm(c, cir, cap);
        arms.hit(match arm {
            1 => "1",
            2 => "2",
            _ => "3",
        });
        let cr = (c.c2CircletoCapsule)(cir, cap);
        assert_int(
            cr,
            (r.c2CircletoCapsule)(cir, cap),
            &format!("c2CircletoCapsule({cir:?}, {cap:?})"),
        );
        outcome.hit(if cr != 0 { "hit" } else { "miss" });
    }
    arms.require_all(100);
    outcome.require_all(100);

    // Near-boundary sampling around the middle arm (perpendicular offset ~= r).
    for _ in 0..N {
        let a0 = C2v::new(-30.0, 10.0);
        let b0 = C2v::new(40.0, 25.0);
        let cr_ = rng.unit() * 20.0 + 0.25;
        let ar = rng.unit() * 10.0;
        let total = ar + cr_;
        let dx = b0.x - a0.x;
        let dy = b0.y - a0.y;
        let ilen = 1.0 / (dx * dx + dy * dy).sqrt();
        let (nx, ny) = (-dy * ilen, dx * ilen);
        let t = rng.unit();
        let off = total * (1.0 + (rng.unit() - 0.5) * 1.0e-3) * if rng.below(2) == 0 { 1.0 } else { -1.0 };
        let p = C2v::new(a0.x + dx * t + nx * off, a0.y + dy * t + ny * off);
        let cir = C2Circle { p, r: ar };
        let cap = C2Capsule {
            a: a0,
            b: b0,
            r: cr_,
        };
        assert_int(
            (c.c2CircletoCapsule)(cir, cap),
            (r.c2CircletoCapsule)(cir, cap),
            &format!("c2CircletoCapsule boundary({cir:?}, {cap:?})"),
        );
    }
}

#[test]
fn c21_capsule_degenerate_and_extremes() {
    let (c, r) = libs();
    let mut rng = Rng::new(21);

    // Zero-length capsule => c2Dot(n,n) == 0 => division by zero in arm 2.
    for _ in 0..N {
        let a0 = rng.tame_v(80.0);
        let cap = C2Capsule {
            a: a0,
            b: a0,
            r: [0.0f32, -0.0, 5.0, -5.0, 1.0e30][rng.below(5) as usize],
        };
        let cir = C2Circle {
            p: if rng.below(3) == 0 { a0 } else { rng.tame_v(120.0) },
            r: [0.0f32, -0.0, 3.0, -3.0, 1.0e-30][rng.below(5) as usize],
        };
        assert_int(
            (c.c2CircletoCapsule)(cir, cap),
            (r.c2CircletoCapsule)(cir, cap),
            &format!("zero-length capsule({cir:?}, {cap:?})"),
        );
    }
    // Extremes / fully random bit patterns.
    for _ in 0..N {
        let cir = C2Circle {
            p: C2v::new(rng.any_f32(), rng.any_f32()),
            r: rng.any_f32(),
        };
        let cap = C2Capsule {
            a: C2v::new(rng.any_f32(), rng.any_f32()),
            b: C2v::new(rng.any_f32(), rng.any_f32()),
            r: rng.any_f32(),
        };
        assert_int(
            (c.c2CircletoCapsule)(cir, cap),
            (r.c2CircletoCapsule)(cir, cap),
            &format!("c2CircletoCapsule extremes({cir:?}, {cap:?})"),
        );
    }
    for _ in 0..N {
        let cir = C2Circle {
            p: rng.bit_v(),
            r: rng.bit_f32(),
        };
        let cap = C2Capsule {
            a: rng.bit_v(),
            b: rng.bit_v(),
            r: rng.bit_f32(),
        };
        assert_int(
            (c.c2CircletoCapsule)(cir, cap),
            (r.c2CircletoCapsule)(cir, cap),
            &format!("c2CircletoCapsule bits({cir:?}, {cap:?})"),
        );
    }
}

// ===========================================================================
// C22..C26 — c2Collided dispatch
// ===========================================================================

#[test]
fn c22_collided_circle() {
    let (c, r) = libs();
    let mut rng = Rng::new(22);
    for _ in 0..N {
        let a = C2Circle {
            p: if rng.below(4) == 0 {
                rng.bit_v()
            } else {
                rng.tame_v(200.0)
            },
            r: if rng.below(4) == 0 {
                rng.bit_f32()
            } else {
                rng.unit() * 60.0
            },
        };
        let b = C2Circle {
            p: rng.tame_v(200.0),
            r: rng.unit() * 60.0,
        };
        let (pa, pb) = (
            &a as *const C2Circle as *const c_void,
            &b as *const C2Circle as *const c_void,
        );
        let (cr, rr) = unsafe {
            (
                (c.c2Collided)(pa, pb, C2_TYPE_CIRCLE),
                (r.c2Collided)(pa, pb, C2_TYPE_CIRCLE),
            )
        };
        assert_int(cr, rr, &format!("c2Collided CIRCLE({a:?}, {b:?})"));
        // Must agree with the direct call, too.
        assert_int(
            cr,
            (c.c2CircletoCircle)(a, b),
            "c2Collided CIRCLE vs direct (C)",
        );
    }
}

#[test]
fn c23_collided_aabb() {
    let (c, r) = libs();
    let mut rng = Rng::new(23);
    for _ in 0..N {
        let a = C2Circle {
            p: rng.tame_v(200.0),
            r: if rng.below(4) == 0 {
                rng.bit_f32()
            } else {
                rng.unit() * 60.0
            },
        };
        let min = rng.tame_v(100.0);
        let bb = C2Aabb {
            min,
            max: C2v::new(min.x + rng.unit() * 80.0, min.y + rng.unit() * 80.0),
        };
        let (pa, pb) = (
            &a as *const C2Circle as *const c_void,
            &bb as *const C2Aabb as *const c_void,
        );
        let (cr, rr) = unsafe {
            (
                (c.c2Collided)(pa, pb, C2_TYPE_AABB),
                (r.c2Collided)(pa, pb, C2_TYPE_AABB),
            )
        };
        assert_int(cr, rr, &format!("c2Collided AABB({a:?}, {bb:?})"));
        assert_int(cr, (c.c2CircletoAABB)(a, bb), "c2Collided AABB vs direct");
    }
}

#[test]
fn c24_collided_capsule() {
    let (c, r) = libs();
    let mut rng = Rng::new(24);
    for _ in 0..N {
        let (cir, cap) = random_capsule_case(&mut rng);
        let (pa, pb) = (
            &cir as *const C2Circle as *const c_void,
            &cap as *const C2Capsule as *const c_void,
        );
        let (cr, rr) = unsafe {
            (
                (c.c2Collided)(pa, pb, C2_TYPE_CAPSULE),
                (r.c2Collided)(pa, pb, C2_TYPE_CAPSULE),
            )
        };
        assert_int(cr, rr, &format!("c2Collided CAPSULE({cir:?}, {cap:?})"));
        assert_int(
            cr,
            (c.c2CircletoCapsule)(cir, cap),
            "c2Collided CAPSULE vs direct",
        );
    }
}

#[test]
fn c25_collided_aliased_pointers() {
    let (c, r) = libs();
    let mut rng = Rng::new(25);
    for _ in 0..N {
        let a = C2Circle {
            p: if rng.below(4) == 0 {
                rng.bit_v()
            } else {
                rng.tame_v(200.0)
            },
            r: if rng.below(4) == 0 {
                rng.bit_f32()
            } else {
                rng.tame_f32(60.0)
            },
        };
        let p = &a as *const C2Circle as *const c_void;
        let (cr, rr) = unsafe {
            (
                (c.c2Collided)(p, p, C2_TYPE_CIRCLE),
                (r.c2Collided)(p, p, C2_TYPE_CIRCLE),
            )
        };
        assert_int(cr, rr, &format!("c2Collided aliased({a:?})"));
    }
}

#[test]
fn c26_collided_raw_byte_buffers() {
    let (c, r) = libs();
    let mut rng = Rng::new(26);
    // 4-byte aligned scratch buffers, larger than any of the three B types.
    for _ in 0..N {
        let mut abuf = [0u32; 8];
        let mut bbuf = [0u32; 8];
        for w in abuf.iter_mut() {
            *w = rng.next_u32();
        }
        for w in bbuf.iter_mut() {
            *w = rng.next_u32();
        }
        let pa = abuf.as_ptr() as *const c_void;
        let pb = bbuf.as_ptr() as *const c_void;
        for tag in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let (cr, rr) = unsafe { ((c.c2Collided)(pa, pb, tag), (r.c2Collided)(pa, pb, tag)) };
            assert_int(
                cr,
                rr,
                &format!("c2Collided raw tag={tag} A={abuf:08x?} B={bbuf:08x?}"),
            );
        }
    }
}

// ===========================================================================
// C27 / C28 / C29 — circle_collide
// ===========================================================================

#[test]
fn c27_circle_collide_grid() {
    let (c, r) = libs();
    let mut seen = [0u32; 8];
    let mut ix = 0;
    while ix <= 120 {
        let x = -130.0 + ix as f32 * 2.0;
        let mut iy = 0;
        while iy <= 120 {
            let y = -130.0 + iy as f32 * 2.0;
            let mut ir = 0;
            while ir <= 35 {
                let rad = ir as f32 * 2.0;
                let cr = (c.circle_collide)(x, y, rad);
                let rr = (r.circle_collide)(x, y, rad);
                assert_int(cr, rr, &format!("circle_collide({x}, {y}, {rad})"));
                assert!(
                    (0..8).contains(&cr),
                    "circle_collide returned {cr} outside 0..8"
                );
                seen[cr as usize] += 1;
                ir += 1;
            }
            iy += 1;
        }
        ix += 1;
    }
    let missing: Vec<usize> = (0..8).filter(|i| seen[*i] == 0).collect();
    assert!(
        missing.is_empty(),
        "grid never produced result bitmasks {missing:?}; histogram {seen:?}"
    );
}

#[test]
fn c28_circle_collide_random() {
    let (c, r) = libs();
    let mut rng = Rng::new(28);
    for _ in 0..(N * 5) {
        let x = rng.tame_f32(200.0);
        let y = rng.tame_f32(200.0);
        let rad = rng.tame_f32(200.0);
        assert_int(
            (c.circle_collide)(x, y, rad),
            (r.circle_collide)(x, y, rad),
            &format!(
                "circle_collide({}, {}, {})",
                fmt_f32(x),
                fmt_f32(y),
                fmt_f32(rad)
            ),
        );
    }
    // Boundary-hugging radii against each of the three fixed shapes.
    let anchors: &[(f32, f32)] = &[
        (-70.0, 0.0),
        (-40.0, -40.0),
        (-15.0, -15.0),
        (-27.5, -27.5),
        (-40.0, 40.0),
        (-20.0, 100.0),
        (-30.0, 70.0),
    ];
    for &(ax, ay) in anchors {
        for _ in 0..5_000 {
            let x = ax + rng.sym(60.0);
            let y = ay + rng.sym(60.0);
            let d = ((x - ax) * (x - ax) + (y - ay) * (y - ay)).sqrt();
            for rad in [
                d,
                d - 1.0e-4,
                d + 1.0e-4,
                d - 20.0,
                d + 20.0,
                d - 10.0,
                d + 10.0,
            ] {
                assert_int(
                    (c.circle_collide)(x, y, rad),
                    (r.circle_collide)(x, y, rad),
                    &format!("circle_collide near({x}, {y}, {rad})"),
                );
            }
        }
    }
}

#[test]
fn c29_circle_collide_bitpatterns() {
    let (c, r) = libs();
    let mut rng = Rng::new(29);
    let pool: Vec<f32> = SPECIALS.iter().chain(NANS.iter()).copied().collect();
    for &x in pool.iter() {
        for &y in pool.iter() {
            for &rad in pool.iter() {
                assert_int(
                    (c.circle_collide)(x, y, rad),
                    (r.circle_collide)(x, y, rad),
                    &format!(
                        "circle_collide({}, {}, {})",
                        fmt_f32(x),
                        fmt_f32(y),
                        fmt_f32(rad)
                    ),
                );
            }
        }
    }
    for _ in 0..(N * 5) {
        let x = rng.bit_f32();
        let y = rng.bit_f32();
        let rad = rng.bit_f32();
        assert_int(
            (c.circle_collide)(x, y, rad),
            (r.circle_collide)(x, y, rad),
            &format!(
                "circle_collide bits({}, {}, {})",
                fmt_f32(x),
                fmt_f32(y),
                fmt_f32(rad)
            ),
        );
    }
    for _ in 0..(N * 5) {
        let x = rng.any_f32();
        let y = rng.any_f32();
        let rad = rng.any_f32();
        assert_int(
            (c.circle_collide)(x, y, rad),
            (r.circle_collide)(x, y, rad),
            &format!(
                "circle_collide any({}, {}, {})",
                fmt_f32(x),
                fmt_f32(y),
                fmt_f32(rad)
            ),
        );
    }
}
