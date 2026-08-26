//! Phase B — valid-path differential tests, rows 1..17 of `CONFIGS.md`
//! (the scalar / vector / predicate layer: the lowest-level exported entry
//! points).  Every call goes through `dlopen`ed function pointers of both the
//! C `.so` and the Rust `.so`; results are compared bit-for-bit.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4000;

/* ---------------------------------------------------------------- row 1 --- */

#[test]
fn row01_c2V_all_value_classes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1001);
    let mut d = Diff::new("1: c2V, all value classes");
    for i in 0..N {
        let (x, y) = match i % 4 {
            0 => (rng.mixed(), rng.mixed()),
            1 => (rng.special(), rng.special()),
            2 => (rng.any_bits(), rng.any_bits()),
            _ => (rng.ordinary(), rng.ordinary()),
        };
        let rc = unsafe { (c.c2V)(x, y) };
        let rr = unsafe { (r.c2V)(x, y) };
        d.check(v_eq_bits(rc, rr), || {
            format!(
                "c2V({}, {}): C {} vs RUST {}",
                fshow(x),
                fshow(y),
                vshow(rc),
                vshow(rr)
            )
        });
    }
    d.finish();
}

/* --------------------------------------------------------------- rows 2-3 - */

fn dot_row(row: &str, seed: u64, mk: fn(&mut Rng) -> (C2v, C2v)) {
    let (c, r) = apis();
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N {
        let (a, b) = mk(&mut rng);
        let rc = unsafe { (c.c2Dot)(a, b) };
        let rr = unsafe { (r.c2Dot)(a, b) };
        d.check(f_eq_bits(rc, rr), || {
            format!(
                "c2Dot({}, {}): C {} vs RUST {}",
                vshow(a),
                vshow(b),
                fshow(rc),
                fshow(rr)
            )
        });
    }
    d.finish();
}

#[test]
fn row02_c2Dot_finite_and_overflow() {
    dot_row("2: c2Dot ordinary/overflow", 0x2002, |rng| match rng.below(4) {
        0 => (
            v(rng.range(-1e19, 1e19), rng.range(-1e19, 1e19)),
            v(rng.range(-1e19, 1e19), rng.range(-1e19, 1e19)),
        ),
        1 => {
            // near-total cancellation: a.x*b.x == -(a.y*b.y)
            let ax = rng.ordinary();
            let bx = rng.ordinary();
            let ay = rng.ordinary();
            let by = -(ax * bx) / ay;
            (v(ax, ay), v(bx, by))
        }
        2 => (
            v(f32::MAX, f32::MAX),
            v(rng.range(0.5, 2.0), rng.range(0.5, 2.0)),
        ),
        _ => (rng.v_ordinary(), rng.v_ordinary()),
    });
}

#[test]
fn row03_c2Dot_zero_inf_nan() {
    dot_row("3: c2Dot zero/inf/nan", 0x3003, |rng| match rng.below(3) {
        0 => (rng.v_special(), rng.v_special()),
        1 => (rng.v_any_bits(), rng.v_special()),
        _ => (rng.v_special(), rng.v_any_bits()),
    });
}

/* --------------------------------------------------------------- rows 4-5 - */

fn len_row(row: &str, seed: u64, mk: fn(&mut Rng) -> C2v) {
    let (c, r) = apis();
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N {
        let a = mk(&mut rng);
        let rc = unsafe { (c.c2Len)(a) };
        let rr = unsafe { (r.c2Len)(a) };
        d.check(f_eq_bits(rc, rr), || {
            format!("c2Len({}): C {} vs RUST {}", vshow(a), fshow(rc), fshow(rr))
        });
    }
    d.finish();
}

#[test]
fn row04_c2Len_ordinary_and_denormal() {
    len_row("4: c2Len ordinary/denormal", 0x4004, |rng| match rng.below(3) {
        0 => rng.v_ordinary(),
        1 => v(rng.range(-1e-20, 1e-20), rng.range(-1e-20, 1e-20)),
        _ => v(
            f32::from_bits(rng.below(0x0080_0000)),
            f32::from_bits(rng.below(0x0080_0000)),
        ),
    });
}

#[test]
fn row05_c2Len_huge_inf_nan() {
    len_row("5: c2Len huge/inf/nan (sqrtf edge)", 0x5005, |rng| match rng.below(4) {
        0 => v(rng.range(1e30, 3e38), rng.range(1e30, 3e38)),
        1 => rng.v_special(),
        2 => rng.v_any_bits(),
        _ => v(rng.special(), rng.ordinary()),
    });
}

/* ----------------------------------------------------------------- row 6 -- */

#[test]
fn row06_c2Add_c2Sub() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x6006);
    let mut d = Diff::new("6: c2Add/c2Sub");
    for i in 0..N {
        let (a, b) = match i % 4 {
            0 => (rng.v_ordinary(), rng.v_ordinary()),
            1 => (rng.v_special(), rng.v_special()),
            2 => (rng.v_any_bits(), rng.v_any_bits()),
            _ => (rng.v_mixed(), rng.v_mixed()),
        };
        let ac = unsafe { (c.c2Add)(a, b) };
        let ar = unsafe { (r.c2Add)(a, b) };
        d.check(v_eq_bits(ac, ar), || {
            format!(
                "c2Add({}, {}): C {} vs RUST {}",
                vshow(a),
                vshow(b),
                vshow(ac),
                vshow(ar)
            )
        });
        let sc = unsafe { (c.c2Sub)(a, b) };
        let sr = unsafe { (r.c2Sub)(a, b) };
        d.check(v_eq_bits(sc, sr), || {
            format!(
                "c2Sub({}, {}): C {} vs RUST {}",
                vshow(a),
                vshow(b),
                vshow(sc),
                vshow(sr)
            )
        });
    }
    d.finish();
}

/* ----------------------------------------------------------------- row 7 -- */

#[test]
fn row07_c2Mulvs() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x7007);
    let mut d = Diff::new("7: c2Mulvs");
    for i in 0..N {
        let a = match i % 3 {
            0 => rng.v_ordinary(),
            1 => rng.v_special(),
            _ => rng.v_any_bits(),
        };
        let s = match i % 4 {
            0 => rng.ordinary(),
            1 => rng.special(),
            2 => rng.any_bits(),
            _ => rng.range(1e30, 3e38),
        };
        let rc = unsafe { (c.c2Mulvs)(a, s) };
        let rr = unsafe { (r.c2Mulvs)(a, s) };
        d.check(v_eq_bits(rc, rr), || {
            format!(
                "c2Mulvs({}, {}): C {} vs RUST {}",
                vshow(a),
                fshow(s),
                vshow(rc),
                vshow(rr)
            )
        });
    }
    d.finish();
}

/* ----------------------------------------------------------------- row 8 -- */

#[test]
fn row08_c2Div_reciprocal_semantics() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x8008);
    let mut d = Diff::new("8: c2Div (reciprocal multiply)");
    for i in 0..N {
        let a = match i % 3 {
            0 => rng.v_ordinary(),
            1 => rng.v_special(),
            _ => rng.v_any_bits(),
        };
        let b = match i % 6 {
            0 => rng.ordinary(),
            1 => rng.special(),
            2 => rng.any_bits(),
            3 => 0.0,
            4 => -0.0,
            _ => rng.range(-3.0, 3.0),
        };
        let rc = unsafe { (c.c2Div)(a, b) };
        let rr = unsafe { (r.c2Div)(a, b) };
        d.check(v_eq_bits(rc, rr), || {
            format!(
                "c2Div({}, {}): C {} vs RUST {}",
                vshow(a),
                fshow(b),
                vshow(rc),
                vshow(rr)
            )
        });
    }
    // sanity: the reciprocal form must differ from plain division somewhere,
    // otherwise this row would not be testing anything.
    let mut differs = 0;
    let mut rng2 = Rng::new(0x8009);
    for _ in 0..20000 {
        let a = v(rng2.ordinary(), rng2.ordinary());
        let b = rng2.range(-100.0, 100.0);
        let got = unsafe { (c.c2Div)(a, b) };
        if !f_eq_bits(got.x, a.x / b) {
            differs += 1;
        }
    }
    assert!(
        differs > 0,
        "reciprocal-multiply vs divide never differed — generator too weak"
    );
    println!("c2Div: reciprocal-multiply differs from a/b in {differs}/20000 samples");
    d.finish();
}

/* ----------------------------------------------------------------- row 9 -- */

#[test]
fn row09_c2Norm() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x9009);
    let mut d = Diff::new("9: c2Norm");
    for i in 0..N {
        let a = match i % 6 {
            0 => rng.v_ordinary(),
            1 => v(0.0, 0.0),
            2 => v(-0.0, 0.0),
            3 => v(rng.range(-1e-30, 1e-30), rng.range(-1e-30, 1e-30)),
            4 => v(rng.range(1e30, 3e38), rng.range(1e30, 3e38)),
            _ => rng.v_mixed(),
        };
        let rc = unsafe { (c.c2Norm)(a) };
        let rr = unsafe { (r.c2Norm)(a) };
        d.check(v_eq_bits(rc, rr), || {
            format!("c2Norm({}): C {} vs RUST {}", vshow(a), vshow(rc), vshow(rr))
        });
    }
    d.finish();
}

/* ---------------------------------------------------------------- row 10 -- */

#[test]
fn row10_c2Minv_c2Maxv_tie_and_nan_semantics() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xA00A);
    let mut d = Diff::new("10: c2Minv/c2Maxv ternary semantics");
    // hand-picked ±0 / NaN orderings, then randomized
    let picks = [
        (v(0.0, -0.0), v(-0.0, 0.0)),
        (v(-0.0, 0.0), v(0.0, -0.0)),
        (v(f32::NAN, 1.0), v(1.0, f32::NAN)),
        (v(1.0, f32::NAN), v(f32::NAN, 1.0)),
        (v(f32::from_bits(0xFFC0_0000), 2.0), v(2.0, f32::from_bits(0x7FA0_0000))),
        (v(f32::INFINITY, f32::NEG_INFINITY), v(f32::NEG_INFINITY, f32::INFINITY)),
    ];
    for i in 0..N {
        let (a, b) = if i < picks.len() {
            picks[i]
        } else {
            match i % 5 {
                0 => (rng.v_special(), rng.v_special()),
                1 => (rng.v_any_bits(), rng.v_any_bits()),
                2 => {
                    let x = rng.ordinary();
                    (v(x, rng.ordinary()), v(x, rng.ordinary()))
                }
                _ => (rng.v_mixed(), rng.v_mixed()),
            }
        };
        let mc = unsafe { (c.c2Minv)(a, b) };
        let mr = unsafe { (r.c2Minv)(a, b) };
        d.check(v_eq_bits(mc, mr), || {
            format!(
                "c2Minv({}, {}): C {} vs RUST {}",
                vshow(a),
                vshow(b),
                vshow(mc),
                vshow(mr)
            )
        });
        let xc = unsafe { (c.c2Maxv)(a, b) };
        let xr = unsafe { (r.c2Maxv)(a, b) };
        d.check(v_eq_bits(xc, xr), || {
            format!(
                "c2Maxv({}, {}): C {} vs RUST {}",
                vshow(a),
                vshow(b),
                vshow(xc),
                vshow(xr)
            )
        });
    }
    d.finish();
}

/* ---------------------------------------------------------------- row 11 -- */

#[test]
fn row11_c2Skew_c2CCW90() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB00B);
    let mut d = Diff::new("11: c2Skew/c2CCW90");
    for i in 0..N {
        let a = match i % 4 {
            0 => rng.v_ordinary(),
            1 => rng.v_special(),
            2 => rng.v_any_bits(),
            _ => v(0.0, -0.0),
        };
        let sc = unsafe { (c.c2Skew)(a) };
        let sr = unsafe { (r.c2Skew)(a) };
        d.check(v_eq_bits(sc, sr), || {
            format!("c2Skew({}): C {} vs RUST {}", vshow(a), vshow(sc), vshow(sr))
        });
        let cc = unsafe { (c.c2CCW90)(a) };
        let cr = unsafe { (r.c2CCW90)(a) };
        d.check(v_eq_bits(cc, cr), || {
            format!("c2CCW90({}): C {} vs RUST {}", vshow(a), vshow(cc), vshow(cr))
        });
    }
    d.finish();
}

/* ---------------------------------------------------------------- row 12 -- */

#[test]
fn row12_c2Absv_ternary_not_fabsf() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC00C);
    let mut d = Diff::new("12: c2Absv ternary semantics");
    let picks = [
        v(-0.0, 0.0),
        v(f32::from_bits(0xFFC0_0000), f32::from_bits(0x7FC0_0000)),
        v(f32::from_bits(0xFFA0_0000), f32::from_bits(0x7FA0_0000)),
        v(f32::NEG_INFINITY, f32::INFINITY),
        v(f32::from_bits(0x8000_0001), f32::from_bits(0x0000_0001)),
    ];
    // `-0.0` and `-NaN` must come back UNCHANGED (fabsf would clear the sign).
    for (i, a) in picks.iter().enumerate() {
        let rc = unsafe { (c.c2Absv)(*a) };
        let rr = unsafe { (r.c2Absv)(*a) };
        d.check(v_eq_bits(rc, rr), || {
            format!(
                "c2Absv(pick {i} {}): C {} vs RUST {}",
                vshow(*a),
                vshow(rc),
                vshow(rr)
            )
        });
    }
    for i in 0..N {
        let a = match i % 4 {
            0 => rng.v_ordinary(),
            1 => rng.v_special(),
            2 => rng.v_any_bits(),
            _ => rng.v_mixed(),
        };
        let rc = unsafe { (c.c2Absv)(a) };
        let rr = unsafe { (r.c2Absv)(a) };
        d.check(v_eq_bits(rc, rr), || {
            format!("c2Absv({}): C {} vs RUST {}", vshow(a), vshow(rc), vshow(rr))
        });
    }
    // the C keeps -0.0 / -NaN: assert the reference really behaves that way, so
    // the row is meaningful.
    let z = unsafe { (c.c2Absv)(v(-0.0, f32::from_bits(0xFFC0_0000))) };
    assert_eq!(z.x.to_bits(), 0x8000_0000, "C c2Absv(-0.0) should stay -0.0");
    assert_eq!(z.y.to_bits(), 0xFFC0_0000, "C c2Absv(-NaN) should stay -NaN");
    d.finish();
}

/* ---------------------------------------------------------------- row 13 -- */

#[test]
fn row13_c2MulmvT() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xD00D);
    let mut d = Diff::new("13: c2MulmvT");
    for i in 0..N {
        let m = match i % 3 {
            0 => C2m {
                x: rng.v_ordinary(),
                y: rng.v_ordinary(),
            },
            1 => {
                // a real rotation frame, built the way c2RaytoCapsule builds M
                let y = unsafe { (c.c2Norm)(rng.v_ordinary()) };
                let x = unsafe { (c.c2CCW90)(y) };
                C2m { x, y }
            }
            _ => C2m {
                x: rng.v_mixed(),
                y: rng.v_any_bits(),
            },
        };
        let b = match i % 4 {
            0 => rng.v_ordinary(),
            1 => rng.v_special(),
            2 => rng.v_any_bits(),
            _ => rng.v_mixed(),
        };
        let rc = unsafe { (c.c2MulmvT)(m, b) };
        let rr = unsafe { (r.c2MulmvT)(m, b) };
        d.check(v_eq_bits(rc, rr), || {
            format!(
                "c2MulmvT(m{{{}, {}}}, {}): C {} vs RUST {}",
                vshow(m.x),
                vshow(m.y),
                vshow(b),
                vshow(rc),
                vshow(rr)
            )
        });
    }
    d.finish();
}

/* -------------------------------------------------------------- rows 14-15  */

fn rand_box(rng: &mut Rng, kind: u32) -> C2AABB {
    match kind {
        // proper
        0 => {
            let x0 = rng.range(-50.0, 50.0);
            let y0 = rng.range(-50.0, 50.0);
            C2AABB {
                min: v(x0, y0),
                max: v(x0 + rng.range(0.001, 40.0), y0 + rng.range(0.001, 40.0)),
            }
        }
        // degenerate (point) / 1-D
        1 => {
            let p = v(rng.range(-50.0, 50.0), rng.range(-50.0, 50.0));
            match rng.below(3) {
                0 => C2AABB { min: p, max: p },
                1 => C2AABB {
                    min: p,
                    max: v(p.x, p.y + rng.range(0.001, 20.0)),
                },
                _ => C2AABB {
                    min: p,
                    max: v(p.x + rng.range(0.001, 20.0), p.y),
                },
            }
        }
        // inverted
        2 => {
            let x0 = rng.range(-50.0, 50.0);
            let y0 = rng.range(-50.0, 50.0);
            C2AABB {
                min: v(x0, y0),
                max: v(x0 - rng.range(0.001, 40.0), y0 - rng.range(0.001, 40.0)),
            }
        }
        // pathological values
        _ => C2AABB {
            min: rng.v_mixed(),
            max: rng.v_mixed(),
        },
    }
}

#[test]
fn row14_c2AABBtoAABB_proper() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE00E);
    let mut d = Diff::new("14: c2AABBtoAABB proper boxes");
    let mut overlaps = 0;
    for _ in 0..N {
        let a = rand_box(&mut rng, 0);
        // place B relative to A so that all separation cases occur
        let b = match rng.below(6) {
            0 => C2AABB {
                min: v(a.min.x - 30.0, a.min.y),
                max: v(a.min.x - rng.range(0.0, 10.0), a.max.y),
            }, // left
            1 => C2AABB {
                min: v(a.max.x + rng.range(0.0, 10.0), a.min.y),
                max: v(a.max.x + 30.0, a.max.y),
            }, // right
            2 => C2AABB {
                min: v(a.min.x, a.min.y - 30.0),
                max: v(a.max.x, a.min.y - rng.range(0.0, 10.0)),
            }, // below
            3 => C2AABB {
                min: v(a.min.x, a.max.y + rng.range(0.0, 10.0)),
                max: v(a.max.x, a.max.y + 30.0),
            }, // above
            4 => C2AABB {
                min: v(a.min.x + 1.0, a.min.y + 1.0),
                max: v(a.max.x - 1.0, a.max.y - 1.0),
            }, // contained-ish
            _ => rand_box(&mut rng, 0),
        };
        let rc = unsafe { (c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (r.c2AABBtoAABB)(a, b) };
        if rc != 0 {
            overlaps += 1;
        }
        d.check(rc == rr, || {
            format!(
                "c2AABBtoAABB({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                aabbshow(&b)
            )
        });
    }
    assert!(overlaps > 100 && overlaps < N - 100, "poor overlap balance: {overlaps}");
    d.finish();
}

#[test]
fn row15_c2AABBtoAABB_degenerate_inverted_nan() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xF00F);
    let mut d = Diff::new("15: c2AABBtoAABB degenerate/inverted/NaN");
    for i in 0..N {
        let a = rand_box(&mut rng, (i % 4) as u32);
        let b = rand_box(&mut rng, ((i / 4) % 4) as u32);
        let rc = unsafe { (c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (r.c2AABBtoAABB)(a, b) };
        d.check(rc == rr, || {
            format!(
                "c2AABBtoAABB({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                aabbshow(&b)
            )
        });
    }
    // NaN => the C returns 1 (all four `<` are false)
    let nb = C2AABB {
        min: v(f32::NAN, f32::NAN),
        max: v(f32::NAN, f32::NAN),
    };
    let ordinary = C2AABB {
        min: v(0.0, 0.0),
        max: v(1.0, 1.0),
    };
    let rc = unsafe { (c.c2AABBtoAABB)(ordinary, nb) };
    let rr = unsafe { (r.c2AABBtoAABB)(ordinary, nb) };
    assert_eq!(rc, 1, "C c2AABBtoAABB with NaN box must report overlap");
    assert_eq!(rc, rr);
    d.finish();
}

/* ---------------------------------------------------------------- row 16 -- */

#[test]
fn row16_c2AABBtoPoint() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1111);
    let mut d = Diff::new("16: c2AABBtoPoint");
    let mut inside = 0;
    for i in 0..N {
        let a = rand_box(&mut rng, (i % 4) as u32);
        let p = match rng.below(8) {
            0 => a.min,
            1 => a.max,
            2 => v(a.min.x, a.max.y),
            3 => v(a.max.x, a.min.y),
            4 => v(
                (a.min.x + a.max.x) * 0.5,
                (a.min.y + a.max.y) * 0.5,
            ),
            5 => rng.v_special(),
            6 => rng.v_mixed(),
            _ => v(
                rng.range(a.min.x - 5.0, a.max.x + 5.0),
                rng.range(a.min.y - 5.0, a.max.y + 5.0),
            ),
        };
        let rc = unsafe { (c.c2AABBtoPoint)(a, p) };
        let rr = unsafe { (r.c2AABBtoPoint)(a, p) };
        if rc != 0 {
            inside += 1;
        }
        d.check(rc == rr, || {
            format!(
                "c2AABBtoPoint({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                vshow(p)
            )
        });
    }
    assert!(inside > 100 && inside < N - 100, "poor inside/outside balance: {inside}");
    d.finish();
}

/* ---------------------------------------------------------------- row 17 -- */

#[test]
fn row17_c2CircleToPoint() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2222);
    let mut d = Diff::new("17: c2CircleToPoint");
    let mut inside = 0;
    for _ in 0..N {
        let circ = C2Circle {
            p: rng.v_ordinary(),
            r: match rng.below(6) {
                0 => 0.0,
                1 => -rng.range(0.1, 20.0),
                2 => rng.special(),
                3 => rng.range(1e30, 3e38),
                _ => rng.range(0.001, 20.0),
            },
        };
        let p = match rng.below(5) {
            // exactly on the rim (strict `<` ⇒ miss)
            0 => v(circ.p.x + circ.r, circ.p.y),
            1 => v(circ.p.x, circ.p.y + circ.r),
            2 => rng.v_special(),
            3 => circ.p,
            _ => vadd(circ.p, v(rng.range(-25.0, 25.0), rng.range(-25.0, 25.0))),
        };
        let rc = unsafe { (c.c2CircleToPoint)(circ, p) };
        let rr = unsafe { (r.c2CircleToPoint)(circ, p) };
        if rc != 0 {
            inside += 1;
        }
        d.check(rc == rr, || {
            format!(
                "c2CircleToPoint({}, {}): C {rc} vs RUST {rr}",
                circshow(&circ),
                vshow(p)
            )
        });
    }
    assert!(inside > 100, "too few inside cases: {inside}");
    d.finish();
}
