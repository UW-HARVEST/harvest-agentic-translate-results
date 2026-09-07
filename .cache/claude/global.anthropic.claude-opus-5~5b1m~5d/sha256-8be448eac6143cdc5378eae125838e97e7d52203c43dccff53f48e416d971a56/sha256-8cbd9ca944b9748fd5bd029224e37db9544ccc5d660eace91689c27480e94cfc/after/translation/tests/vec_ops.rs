//! Phase B — CONFIGS.md rows 1..29 and 69..70: the low-level vector /
//! predicate entry points, driven directly through both `.so` exports.

mod common;
use common::*;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Rows 1-2: c2V
// ---------------------------------------------------------------------------

#[test]
fn row01_c2v_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0001);
    let mut d = Diffs::new("row01 c2V random bits");
    for _ in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        let a = (l.c.c2V)(x, y);
        let b = (l.r.c2V)(x, y);
        d.check(veq(a, b), || {
            format!("c2V({}, {}) -> C {} vs R {}", fs(x), fs(y), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row02_c2v_specials() {
    let l = libs();
    let mut d = Diffs::new("row02 c2V specials");
    for &x in SPECIALS {
        for &y in SPECIALS {
            let a = (l.c.c2V)(x, y);
            let b = (l.r.c2V)(x, y);
            d.check(veq(a, b), || {
                format!("c2V({}, {}) -> C {} vs R {}", fs(x), fs(y), vs(a), vs(b))
            });
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 3-5: c2Dot
// ---------------------------------------------------------------------------

#[test]
fn row03_c2dot_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0003);
    let mut d = Diffs::new("row03 c2Dot random bits");
    for _ in 0..N {
        let (u, v) = (rng.any_v(), rng.any_v());
        let a = (l.c.c2Dot)(u, v);
        let b = (l.r.c2Dot)(u, v);
        d.check(feq(a, b), || {
            format!("c2Dot({}, {}) -> C {} vs R {}", vs(u), vs(v), fs(a), fs(b))
        });
    }
    d.finish();
}

#[test]
fn row04_c2dot_specials() {
    let l = libs();
    let mut d = Diffs::new("row04 c2Dot specials cross-product");
    // Full 4-deep cross product would be 24^4 = 331k; that is cheap enough.
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &bx in SPECIALS {
                for &by in SPECIALS {
                    let u = C2v { x: ax, y: ay };
                    let v = C2v { x: bx, y: by };
                    let a = (l.c.c2Dot)(u, v);
                    let b = (l.r.c2Dot)(u, v);
                    d.check(feq(a, b), || {
                        format!("c2Dot({}, {}) -> C {} vs R {}", vs(u), vs(v), fs(a), fs(b))
                    });
                }
            }
        }
    }
    d.finish();
}

#[test]
fn row05_c2dot_near_overflow() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0005);
    let mut d = Diffs::new("row05 c2Dot near-overflow");
    let big = [1e38f32, -1e38, 3.4028235e38, -3.4028235e38, 1e19, -1e19, 2e19];
    for _ in 0..N {
        let u = C2v {
            x: big[rng.below(big.len() as u32) as usize],
            y: big[rng.below(big.len() as u32) as usize],
        };
        let v = C2v {
            x: big[rng.below(big.len() as u32) as usize],
            y: big[rng.below(big.len() as u32) as usize],
        };
        let a = (l.c.c2Dot)(u, v);
        let b = (l.r.c2Dot)(u, v);
        d.check(feq(a, b), || {
            format!("c2Dot({}, {}) -> C {} vs R {}", vs(u), vs(v), fs(a), fs(b))
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 6-7: c2Len
// ---------------------------------------------------------------------------

#[test]
fn row06_c2len_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0006);
    let mut d = Diffs::new("row06 c2Len random bits");
    for _ in 0..N {
        let u = rng.mixed_v();
        let a = (l.c.c2Len)(u);
        let b = (l.r.c2Len)(u);
        d.check(feq(a, b), || {
            format!("c2Len({}) -> C {} vs R {}", vs(u), fs(a), fs(b))
        });
    }
    d.finish();
}

#[test]
fn row07_c2len_specials() {
    let l = libs();
    let mut d = Diffs::new("row07 c2Len specials");
    for &x in SPECIALS {
        for &y in SPECIALS {
            let u = C2v { x, y };
            let a = (l.c.c2Len)(u);
            let b = (l.r.c2Len)(u);
            d.check(feq(a, b), || {
                format!("c2Len({}) -> C {} vs R {}", vs(u), fs(a), fs(b))
            });
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 8-11: c2Add / c2Sub
// ---------------------------------------------------------------------------

#[test]
fn row08_c2add_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0008);
    let mut d = Diffs::new("row08 c2Add random bits");
    for _ in 0..N {
        let (u, v) = (rng.any_v(), rng.any_v());
        let a = (l.c.c2Add)(u, v);
        let b = (l.r.c2Add)(u, v);
        d.check(veq(a, b), || {
            format!("c2Add({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row09_c2add_specials() {
    let l = libs();
    let mut d = Diffs::new("row09 c2Add specials");
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &bx in SPECIALS {
                for &by in SPECIALS {
                    let u = C2v { x: ax, y: ay };
                    let v = C2v { x: bx, y: by };
                    let a = (l.c.c2Add)(u, v);
                    let b = (l.r.c2Add)(u, v);
                    d.check(veq(a, b), || {
                        format!("c2Add({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(a), vs(b))
                    });
                }
            }
        }
    }
    d.finish();
}

#[test]
fn row10_c2sub_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0010);
    let mut d = Diffs::new("row10 c2Sub random bits");
    for _ in 0..N {
        let (u, v) = (rng.any_v(), rng.any_v());
        let a = (l.c.c2Sub)(u, v);
        let b = (l.r.c2Sub)(u, v);
        d.check(veq(a, b), || {
            format!("c2Sub({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row11_c2sub_specials() {
    let l = libs();
    let mut d = Diffs::new("row11 c2Sub specials");
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &bx in SPECIALS {
                for &by in SPECIALS {
                    let u = C2v { x: ax, y: ay };
                    let v = C2v { x: bx, y: by };
                    let a = (l.c.c2Sub)(u, v);
                    let b = (l.r.c2Sub)(u, v);
                    d.check(veq(a, b), || {
                        format!("c2Sub({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(a), vs(b))
                    });
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 12-15: c2Mulvs / c2Div
// ---------------------------------------------------------------------------

#[test]
fn row12_c2mulvs_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0012);
    let mut d = Diffs::new("row12 c2Mulvs random bits");
    for _ in 0..N {
        let u = rng.any_v();
        let s = rng.any_f32();
        let a = (l.c.c2Mulvs)(u, s);
        let b = (l.r.c2Mulvs)(u, s);
        d.check(veq(a, b), || {
            format!("c2Mulvs({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row13_c2mulvs_specials() {
    let l = libs();
    let mut d = Diffs::new("row13 c2Mulvs specials");
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &s in SPECIALS {
                let u = C2v { x: ax, y: ay };
                let a = (l.c.c2Mulvs)(u, s);
                let b = (l.r.c2Mulvs)(u, s);
                d.check(veq(a, b), || {
                    format!("c2Mulvs({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(a), vs(b))
                });
            }
        }
    }
    d.finish();
}

#[test]
fn row14_c2div_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0014);
    let mut d = Diffs::new("row14 c2Div random bits");
    for _ in 0..N {
        let u = rng.any_v();
        let s = rng.any_f32();
        let a = (l.c.c2Div)(u, s);
        let b = (l.r.c2Div)(u, s);
        d.check(veq(a, b), || {
            format!("c2Div({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row15_c2div_specials_and_zero() {
    let l = libs();
    let mut d = Diffs::new("row15 c2Div specials / div-by-zero");
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &s in SPECIALS {
                let u = C2v { x: ax, y: ay };
                let a = (l.c.c2Div)(u, s);
                let b = (l.r.c2Div)(u, s);
                d.check(veq(a, b), || {
                    format!("c2Div({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(a), vs(b))
                });
            }
        }
    }
    // Explicit ERRORS.md rows 34 & 35.
    for (u, s) in [
        (C2v { x: 3.0, y: 4.0 }, 0.0f32),
        (C2v { x: 3.0, y: 4.0 }, -0.0f32),
        (C2v { x: 0.0, y: 0.0 }, 0.0f32),
        (C2v { x: -0.0, y: 0.0 }, -0.0f32),
        (C2v { x: 1e-45, y: -1e-45 }, 1e-45f32),
    ] {
        let a = (l.c.c2Div)(u, s);
        let b = (l.r.c2Div)(u, s);
        d.check(veq(a, b), || {
            format!("c2Div({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(a), vs(b))
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 16-17: c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row16_c2norm_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0016);
    let mut d = Diffs::new("row16 c2Norm random bits");
    for _ in 0..N {
        let u = rng.any_v();
        let a = (l.c.c2Norm)(u);
        let b = (l.r.c2Norm)(u);
        d.check(veq(a, b), || {
            format!("c2Norm({}) -> C {} vs R {}", vs(u), vs(a), vs(b))
        });
    }
    // and geometric ones
    for _ in 0..N {
        let u = rng.geo_v(1000.0);
        let a = (l.c.c2Norm)(u);
        let b = (l.r.c2Norm)(u);
        d.check(veq(a, b), || {
            format!("c2Norm({}) -> C {} vs R {}", vs(u), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row17_c2norm_specials_and_zero() {
    let l = libs();
    let mut d = Diffs::new("row17 c2Norm specials / zero vector");
    for &x in SPECIALS {
        for &y in SPECIALS {
            let u = C2v { x, y };
            let a = (l.c.c2Norm)(u);
            let b = (l.r.c2Norm)(u);
            d.check(veq(a, b), || {
                format!("c2Norm({}) -> C {} vs R {}", vs(u), vs(a), vs(b))
            });
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 18-19: c2Minv / c2Maxv (ternary, not fminf/fmaxf)
// ---------------------------------------------------------------------------

#[test]
fn row18_minv_maxv_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0018);
    let mut d = Diffs::new("row18 c2Minv/c2Maxv random bits");
    for _ in 0..N {
        let (u, v) = (rng.mixed_v(), rng.mixed_v());
        let a = (l.c.c2Minv)(u, v);
        let b = (l.r.c2Minv)(u, v);
        d.check(veq(a, b), || {
            format!("c2Minv({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(a), vs(b))
        });
        let a = (l.c.c2Maxv)(u, v);
        let b = (l.r.c2Maxv)(u, v);
        d.check(veq(a, b), || {
            format!("c2Maxv({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(a), vs(b))
        });
    }
    d.finish();
}

#[test]
fn row19_minv_maxv_specials() {
    let l = libs();
    let mut d = Diffs::new("row19 c2Minv/c2Maxv specials, both orders");
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &bx in SPECIALS {
                for &by in SPECIALS {
                    let u = C2v { x: ax, y: ay };
                    let v = C2v { x: bx, y: by };
                    for (p, q) in [(u, v), (v, u)] {
                        let a = (l.c.c2Minv)(p, q);
                        let b = (l.r.c2Minv)(p, q);
                        d.check(veq(a, b), || {
                            format!("c2Minv({}, {}) -> C {} vs R {}", vs(p), vs(q), vs(a), vs(b))
                        });
                        let a = (l.c.c2Maxv)(p, q);
                        let b = (l.r.c2Maxv)(p, q);
                        d.check(veq(a, b), || {
                            format!("c2Maxv({}, {}) -> C {} vs R {}", vs(p), vs(q), vs(a), vs(b))
                        });
                    }
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Row 20: c2Skew / c2CCW90 / c2Absv
// ---------------------------------------------------------------------------

#[test]
fn row20_skew_ccw90_absv() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0020);
    let mut d = Diffs::new("row20 c2Skew/c2CCW90/c2Absv");
    let mut inputs: Vec<C2v> = Vec::new();
    for _ in 0..N {
        inputs.push(rng.any_v());
    }
    for &x in SPECIALS {
        for &y in SPECIALS {
            inputs.push(C2v { x, y });
        }
    }
    for u in inputs {
        let a = (l.c.c2Skew)(u);
        let b = (l.r.c2Skew)(u);
        d.check(veq(a, b), || {
            format!("c2Skew({}) -> C {} vs R {}", vs(u), vs(a), vs(b))
        });
        let a = (l.c.c2CCW90)(u);
        let b = (l.r.c2CCW90)(u);
        d.check(veq(a, b), || {
            format!("c2CCW90({}) -> C {} vs R {}", vs(u), vs(a), vs(b))
        });
        let a = (l.c.c2Absv)(u);
        let b = (l.r.c2Absv)(u);
        d.check(veq(a, b), || {
            format!("c2Absv({}) -> C {} vs R {}", vs(u), vs(a), vs(b))
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 21-22: c2MulmvT
// ---------------------------------------------------------------------------

#[test]
fn row21_mulmvt_random_bits() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0021);
    let mut d = Diffs::new("row21 c2MulmvT random bits");
    for _ in 0..N {
        let m = C2m {
            x: rng.any_v(),
            y: rng.any_v(),
        };
        let v = rng.any_v();
        let a = (l.c.c2MulmvT)(m, v);
        let b = (l.r.c2MulmvT)(m, v);
        d.check(veq(a, b), || {
            format!(
                "c2MulmvT({{x:{}, y:{}}}, {}) -> C {} vs R {}",
                vs(m.x),
                vs(m.y),
                vs(v),
                vs(a),
                vs(b)
            )
        });
    }
    d.finish();
}

#[test]
fn row22_mulmvt_structured_and_specials() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0022);
    let mut d = Diffs::new("row22 c2MulmvT rotation matrices + specials");
    // The way c2RaytoCapsule builds M: M.y = c2Norm(b - a); M.x = c2CCW90(M.y).
    for _ in 0..N {
        let dir = rng.geo_v(100.0);
        let my = (l.c.c2Norm)(dir);
        let mx = (l.c.c2CCW90)(my);
        let m = C2m { x: mx, y: my };
        let v = rng.geo_v(100.0);
        let a = (l.c.c2MulmvT)(m, v);
        let b = (l.r.c2MulmvT)(m, v);
        d.check(veq(a, b), || {
            format!(
                "c2MulmvT(rot{{x:{}, y:{}}}, {}) -> C {} vs R {}",
                vs(m.x),
                vs(m.y),
                vs(v),
                vs(a),
                vs(b)
            )
        });
    }
    // Special-value matrices (4 matrix entries + 2 vector entries would be
    // 24^6; sample the special set randomly instead, plus all-same matrices).
    for _ in 0..N {
        let pick = |r: &mut Rng| SPECIALS[r.below(SPECIALS.len() as u32) as usize];
        let m = C2m {
            x: C2v {
                x: pick(&mut rng),
                y: pick(&mut rng),
            },
            y: C2v {
                x: pick(&mut rng),
                y: pick(&mut rng),
            },
        };
        let v = C2v {
            x: pick(&mut rng),
            y: pick(&mut rng),
        };
        let a = (l.c.c2MulmvT)(m, v);
        let b = (l.r.c2MulmvT)(m, v);
        d.check(veq(a, b), || {
            format!(
                "c2MulmvT({{x:{}, y:{}}}, {}) -> C {} vs R {}",
                vs(m.x),
                vs(m.y),
                vs(v),
                vs(a),
                vs(b)
            )
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 23-25: c2AABBtoAABB
// ---------------------------------------------------------------------------

fn aabb_cases(rng: &mut Rng) -> Vec<(C2AABB, C2AABB)> {
    let mut v = Vec::new();
    let b = |x0: f32, y0: f32, x1: f32, y1: f32| C2AABB {
        min: C2v { x: x0, y: y0 },
        max: C2v { x: x1, y: y1 },
    };
    let a = b(0.0, 0.0, 10.0, 10.0);
    // overlapping / separating d0..d3 / touching / contained / identical /
    // degenerate / inverted
    v.push((a, b(5.0, 5.0, 15.0, 15.0))); // overlap
    v.push((a, b(-20.0, 0.0, -10.0, 10.0))); // d0
    v.push((a, b(20.0, 0.0, 30.0, 10.0))); // d1
    v.push((a, b(0.0, -20.0, 10.0, -10.0))); // d2
    v.push((a, b(0.0, 20.0, 10.0, 30.0))); // d3
    v.push((a, b(-10.0, 0.0, 0.0, 10.0))); // touching min.x == max.x
    v.push((a, b(10.0, 0.0, 20.0, 10.0))); // touching max.x == min.x
    v.push((a, b(2.0, 2.0, 3.0, 3.0))); // contained
    v.push((a, a)); // identical
    v.push((b(5.0, 5.0, 5.0, 5.0), a)); // degenerate point box
    v.push((b(10.0, 10.0, 0.0, 0.0), a)); // inverted
    v.push((b(-0.0, -0.0, 0.0, 0.0), b(0.0, 0.0, -0.0, -0.0))); // signed zeros
    for _ in 0..2000 {
        let p = rng.geo_v(20.0);
        let q = rng.geo_v(20.0);
        let r = rng.geo_v(20.0);
        let s = rng.geo_v(20.0);
        v.push((
            C2AABB {
                min: (crate::common::libs().c.c2Minv)(p, q),
                max: (crate::common::libs().c.c2Maxv)(p, q),
            },
            C2AABB {
                min: (crate::common::libs().c.c2Minv)(r, s),
                max: (crate::common::libs().c.c2Maxv)(r, s),
            },
        ));
    }
    for _ in 0..4000 {
        v.push((
            C2AABB {
                min: rng.mixed_v(),
                max: rng.mixed_v(),
            },
            C2AABB {
                min: rng.mixed_v(),
                max: rng.mixed_v(),
            },
        ));
    }
    v
}

#[test]
fn rows23_25_aabb_to_aabb() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0023);
    let mut d = Diffs::new("rows23-25 c2AABBtoAABB");
    for (p, q) in aabb_cases(&mut rng) {
        for (u, v) in [(p, q), (q, p)] {
            let a = (l.c.c2AABBtoAABB)(u, v);
            let b = (l.r.c2AABBtoAABB)(u, v);
            d.check(a == b, || {
                format!(
                    "c2AABBtoAABB({}, {}) -> C {} vs R {}",
                    aabbs(u),
                    aabbs(v),
                    a,
                    b
                )
            });
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 26-27: c2AABBtoPoint
// ---------------------------------------------------------------------------

#[test]
fn rows26_27_aabb_to_point() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0026);
    let mut d = Diffs::new("rows26-27 c2AABBtoPoint");
    let bx = C2AABB {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 10.0, y: 10.0 },
    };
    let mut cases: Vec<(C2AABB, C2v)> = vec![
        (bx, C2v { x: 5.0, y: 5.0 }),     // inside
        (bx, C2v { x: -1.0, y: 5.0 }),    // d0
        (bx, C2v { x: 5.0, y: -1.0 }),    // d1
        (bx, C2v { x: 11.0, y: 5.0 }),    // d2
        (bx, C2v { x: 5.0, y: 11.0 }),    // d3
        (bx, C2v { x: 0.0, y: 0.0 }),     // on min corner (accepted)
        (bx, C2v { x: 10.0, y: 10.0 }),   // on max corner (accepted)
        (
            C2AABB {
                min: C2v { x: 10.0, y: 10.0 },
                max: C2v { x: 0.0, y: 0.0 },
            },
            C2v { x: 5.0, y: 5.0 },
        ), // inverted
        (
            C2AABB {
                min: C2v { x: -0.0, y: -0.0 },
                max: C2v { x: 0.0, y: 0.0 },
            },
            C2v { x: 0.0, y: -0.0 },
        ), // signed zeros
    ];
    for &x in SPECIALS {
        for &y in SPECIALS {
            cases.push((bx, C2v { x, y }));
        }
    }
    for _ in 0..8000 {
        cases.push((
            C2AABB {
                min: rng.mixed_v(),
                max: rng.mixed_v(),
            },
            rng.mixed_v(),
        ));
    }
    for _ in 0..8000 {
        let p = rng.geo_v(20.0);
        let q = rng.geo_v(20.0);
        cases.push((
            C2AABB {
                min: (l.c.c2Minv)(p, q),
                max: (l.c.c2Maxv)(p, q),
            },
            rng.geo_v(20.0),
        ));
    }
    for (u, v) in cases {
        let a = (l.c.c2AABBtoPoint)(u, v);
        let b = (l.r.c2AABBtoPoint)(u, v);
        d.check(a == b, || {
            format!(
                "c2AABBtoPoint({}, {}) -> C {} vs R {}",
                aabbs(u),
                vs(v),
                a,
                b
            )
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 28-29: c2CircleToPoint
// ---------------------------------------------------------------------------

#[test]
fn rows28_29_circle_to_point() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0028);
    let mut d = Diffs::new("rows28-29 c2CircleToPoint");
    let c = |x: f32, y: f32, r: f32| C2Circle {
        p: C2v { x, y },
        r,
    };
    let mut cases: Vec<(C2Circle, C2v)> = vec![
        (c(0.0, 0.0, 5.0), C2v { x: 1.0, y: 1.0 }),  // inside
        (c(0.0, 0.0, 5.0), C2v { x: 5.0, y: 0.0 }),  // exactly on rim -> reject
        (c(0.0, 0.0, 5.0), C2v { x: 6.0, y: 0.0 }),  // outside
        (c(0.0, 0.0, 0.0), C2v { x: 0.0, y: 0.0 }),  // r == 0
        (c(0.0, 0.0, -5.0), C2v { x: 1.0, y: 1.0 }), // r < 0 (behaves as |r|)
        (c(0.0, 0.0, f32::INFINITY), C2v { x: 1e30, y: 1e30 }),
        (c(0.0, 0.0, f32::NAN), C2v { x: 0.0, y: 0.0 }),
    ];
    for &x in SPECIALS {
        for &y in SPECIALS {
            for &r in SPECIALS {
                cases.push((c(x, y, r), C2v { x: 1.0, y: -2.0 }));
                cases.push((c(1.0, -2.0, r), C2v { x, y }));
            }
        }
    }
    for _ in 0..8000 {
        cases.push((
            C2Circle {
                p: rng.mixed_v(),
                r: rng.mixed_f32(),
            },
            rng.mixed_v(),
        ));
    }
    for _ in 0..8000 {
        cases.push((
            C2Circle {
                p: rng.geo_v(20.0),
                r: rng.range(0.0, 20.0),
            },
            rng.geo_v(20.0),
        ));
    }
    for (u, v) in cases {
        let a = (l.c.c2CircleToPoint)(u, v);
        let b = (l.r.c2CircleToPoint)(u, v);
        d.check(a == b, || {
            format!(
                "c2CircleToPoint({}, {}) -> C {} vs R {}",
                circs(u),
                vs(v),
                a,
                b
            )
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 69-70: signed-zero and NaN-payload sweeps across every scalar/vector op
// ---------------------------------------------------------------------------

#[test]
fn row69_signed_zero_sweep() {
    let l = libs();
    let mut d = Diffs::new("row69 signed-zero sweep");
    let zs = [0.0f32, -0.0f32];
    for &ax in &zs {
        for &ay in &zs {
            for &bx in &zs {
                for &by in &zs {
                    let u = C2v { x: ax, y: ay };
                    let v = C2v { x: bx, y: by };
                    macro_rules! cmp_v {
                        ($name:literal, $call:expr) => {{
                            let (ca, ra) = $call;
                            d.check(veq(ca, ra), || {
                                format!("{}({}, {}) -> C {} vs R {}", $name, vs(u), vs(v), vs(ca), vs(ra))
                            });
                        }};
                    }
                    cmp_v!("c2Add", ((l.c.c2Add)(u, v), (l.r.c2Add)(u, v)));
                    cmp_v!("c2Sub", ((l.c.c2Sub)(u, v), (l.r.c2Sub)(u, v)));
                    cmp_v!("c2Minv", ((l.c.c2Minv)(u, v), (l.r.c2Minv)(u, v)));
                    cmp_v!("c2Maxv", ((l.c.c2Maxv)(u, v), (l.r.c2Maxv)(u, v)));
                    cmp_v!("c2Absv", ((l.c.c2Absv)(u), (l.r.c2Absv)(u)));
                    cmp_v!("c2Skew", ((l.c.c2Skew)(u), (l.r.c2Skew)(u)));
                    cmp_v!("c2CCW90", ((l.c.c2CCW90)(u), (l.r.c2CCW90)(u)));
                    cmp_v!("c2Norm", ((l.c.c2Norm)(u), (l.r.c2Norm)(u)));
                    for &s in &zs {
                        let (ca, ra) = ((l.c.c2Mulvs)(u, s), (l.r.c2Mulvs)(u, s));
                        d.check(veq(ca, ra), || {
                            format!("c2Mulvs({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(ca), vs(ra))
                        });
                        let (ca, ra) = ((l.c.c2Div)(u, s), (l.r.c2Div)(u, s));
                        d.check(veq(ca, ra), || {
                            format!("c2Div({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(ca), vs(ra))
                        });
                    }
                    let (ca, ra) = ((l.c.c2Dot)(u, v), (l.r.c2Dot)(u, v));
                    d.check(feq(ca, ra), || {
                        format!("c2Dot({}, {}) -> C {} vs R {}", vs(u), vs(v), fs(ca), fs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Len)(u), (l.r.c2Len)(u));
                    d.check(feq(ca, ra), || {
                        format!("c2Len({}) -> C {} vs R {}", vs(u), fs(ca), fs(ra))
                    });
                }
            }
        }
    }
    d.finish();
}

/// Distinct NaN payloads through every op: this is what pins down the
/// `mulss`/`addss`/`subss` **destination** operand of every arithmetic
/// expression in the C source, because on x86 the destination NaN wins.
#[test]
fn row70_nan_payload_sweep() {
    let l = libs();
    let mut d = Diffs::new("row70 NaN-payload sweep");
    let nans: Vec<f32> = (0..8)
        .map(|i| f32::from_bits(0x7fc0_0000 | (i * 0x1111) | if i % 2 == 1 { 0x8000_0000 } else { 0 }))
        .collect();
    for &n0 in &nans {
        for &n1 in &nans {
            for &n2 in &nans {
                for &n3 in &nans {
                    let u = C2v { x: n0, y: n1 };
                    let v = C2v { x: n2, y: n3 };
                    let (ca, ra) = ((l.c.c2Dot)(u, v), (l.r.c2Dot)(u, v));
                    d.check(feq(ca, ra), || {
                        format!("c2Dot({}, {}) -> C {} vs R {}", vs(u), vs(v), fs(ca), fs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Add)(u, v), (l.r.c2Add)(u, v));
                    d.check(veq(ca, ra), || {
                        format!("c2Add({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(ca), vs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Sub)(u, v), (l.r.c2Sub)(u, v));
                    d.check(veq(ca, ra), || {
                        format!("c2Sub({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(ca), vs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Mulvs)(u, n2), (l.r.c2Mulvs)(u, n2));
                    d.check(veq(ca, ra), || {
                        format!("c2Mulvs({}, {}) -> C {} vs R {}", vs(u), fs(n2), vs(ca), vs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Div)(u, n2), (l.r.c2Div)(u, n2));
                    d.check(veq(ca, ra), || {
                        format!("c2Div({}, {}) -> C {} vs R {}", vs(u), fs(n2), vs(ca), vs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Len)(u), (l.r.c2Len)(u));
                    d.check(feq(ca, ra), || {
                        format!("c2Len({}) -> C {} vs R {}", vs(u), fs(ca), fs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Norm)(u), (l.r.c2Norm)(u));
                    d.check(veq(ca, ra), || {
                        format!("c2Norm({}) -> C {} vs R {}", vs(u), vs(ca), vs(ra))
                    });
                    let m = C2m { x: u, y: v };
                    let (ca, ra) = ((l.c.c2MulmvT)(m, v), (l.r.c2MulmvT)(m, v));
                    d.check(veq(ca, ra), || {
                        format!(
                            "c2MulmvT({{x:{},y:{}}}, {}) -> C {} vs R {}",
                            vs(u),
                            vs(v),
                            vs(v),
                            vs(ca),
                            vs(ra)
                        )
                    });
                    let (ca, ra) = ((l.c.c2Minv)(u, v), (l.r.c2Minv)(u, v));
                    d.check(veq(ca, ra), || {
                        format!("c2Minv({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(ca), vs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Maxv)(u, v), (l.r.c2Maxv)(u, v));
                    d.check(veq(ca, ra), || {
                        format!("c2Maxv({}, {}) -> C {} vs R {}", vs(u), vs(v), vs(ca), vs(ra))
                    });
                    let (ca, ra) = ((l.c.c2Absv)(u), (l.r.c2Absv)(u));
                    d.check(veq(ca, ra), || {
                        format!("c2Absv({}) -> C {} vs R {}", vs(u), vs(ca), vs(ra))
                    });
                }
            }
        }
    }
    d.finish();
}
