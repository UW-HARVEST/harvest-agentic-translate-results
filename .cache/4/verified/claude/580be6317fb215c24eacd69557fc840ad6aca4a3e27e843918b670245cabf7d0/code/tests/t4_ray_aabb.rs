//! Phase B rows B32..B41 and Phase C rows E11..E15 for `c2RaytoAABB`.

mod common;
use common::*;
use std::collections::HashSet;

fn ray(px: f32, py: f32, dx: f32, dy: f32, t: f32) -> c2Ray {
    c2Ray {
        p: c2v { x: px, y: py },
        d: c2v { x: dx, y: dy },
        t,
    }
}

fn bx(minx: f32, miny: f32, maxx: f32, maxy: f32) -> c2AABB {
    c2AABB {
        min: c2v { x: minx, y: miny },
        max: c2v { x: maxx, y: maxy },
    }
}

const UNIT: c2AABB = c2AABB {
    min: c2v { x: -1.0, y: -1.0 },
    max: c2v { x: 1.0, y: 1.0 },
};

fn both(d: &mut Diff, label: &str, a: c2Ray, b: c2AABB) -> RayResult {
    let (c, r) = apis();
    let rc = call_aabb(c, a, b);
    let rr = call_aabb(r, a, b);
    d.ray(label, || format!("{:?} {:?}", a, b), rc, rr);
    rc
}

/// B32 + B39 + E14 + E15: a dense deterministic sweep of ray origins,
/// directions and lengths against a fixed box.  This is what actually covers
/// every combination of `hit0..hit3` and all four face-normal outcomes.
#[test]
fn b32_b39_grid_sweep() {
    let mut d = Diff::new();
    let mut normals: HashSet<(u32, u32)> = HashSet::new();
    let mut zero_t = 0usize;
    let mut hits = 0usize;
    let angles: Vec<c2v> = (0..16)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            c2v {
                x: a.cos(),
                y: a.sin(),
            }
        })
        .collect();
    let coords: Vec<f32> = (-6..=6).map(|i| i as f32 * 0.5).collect();
    for &ox in &coords {
        for &oy in &coords {
            for dir in &angles {
                for &t in &[0.0f32, 0.5, 1.0, 2.0, 5.0, 10.0] {
                    let a = c2Ray {
                        p: c2v { x: ox, y: oy },
                        d: *dir,
                        t,
                    };
                    let (ret, out) = both(&mut d, "B32/grid", a, UNIT);
                    if ret != 0 {
                        hits += 1;
                        normals.insert((out.n.x.to_bits(), out.n.y.to_bits()));
                        if out.t == 0.0 {
                            zero_t += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(hits > 1000, "too few hits: {hits}");
    // All four face-normal branches must have been taken.
    assert_eq!(
        normals.len(),
        4,
        "expected all 4 out->n branches, saw {:?}",
        normals
    );
    // E14/E15: `t_i == 0` outcomes really occur in this sweep.
    assert!(zero_t > 0, "no t_i == 0 (E14/E15) case in the sweep");
    d.finish("B32/B39 c2RaytoAABB grid sweep");
}

/// B33 + E14: ray entirely inside the box (`da < 0` for the inner planes).
#[test]
fn b33_e14_ray_inside_box() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB33);
    let mut zero_t = 0;
    for _ in 0..20_000 {
        let b = rng.aabb_proper();
        let hx = (b.max.x - b.min.x) * 0.5;
        let hy = (b.max.y - b.min.y) * 0.5;
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        let a = c2Ray {
            p: c2v {
                x: cx + rng.uniform(hx * 0.5),
                y: cy + rng.uniform(hy * 0.5),
            },
            d: rng.unit(),
            t: hx.min(hy) * 0.25,
        };
        let (ret, out) = both(&mut d, "B33", a, b);
        if ret != 0 && out.t == 0.0 {
            zero_t += 1;
        }
    }
    assert!(zero_t > 0, "expected E14 (t_i == 0) hits from inside the box");
    d.finish("B33/E14 c2RaytoAABB ray inside box");
}

/// B34 + E12: `a_box` overlaps `B`, but the ray-normal separating axis rejects
/// (`d > 0`).
#[test]
fn b34_e12_separating_axis_reject() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB34);
    let mut rejects = 0;
    for _ in 0..20_000 {
        // Diagonal segment offset so its bbox overlaps the unit box but the
        // line itself is more than sqrt(2) away from the origin.
        let off = 2.5 + (rng.uniform(7.0)).abs().min(7.0);
        let a = ray(-10.0, -10.0 + off, 0.707_106_77, 0.707_106_77, 28.284_271);
        let (ret, _) = both(&mut d, "B34", a, UNIT);
        if ret == 0 {
            rejects += 1;
        }
        // mirrored variant
        let a2 = ray(-10.0, -10.0 - off, 0.707_106_77, 0.707_106_77, 28.284_271);
        both(&mut d, "B34/mirror", a2, UNIT);
    }
    assert!(rejects > 0, "expected E12 rejections");
    d.finish("B34/E12 c2RaytoAABB separating axis");
}

/// B35: axis-aligned rays (`d.x == 0` or `d.y == 0`), incl. `-0.0`.
#[test]
fn b35_axis_aligned() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB35);
    let dirs = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 1.0, y: -0.0 },
        c2v { x: -0.0, y: 1.0 },
    ];
    for _ in 0..3_000 {
        let b = rng.aabb_proper();
        let p = rng.vec_nice();
        for dir in dirs {
            for t in [0.0f32, 1.0, 10.0, 1e6, -1.0] {
                both(&mut d, "B35", c2Ray { p, d: dir, t }, b);
            }
        }
    }
    d.finish("B35 c2RaytoAABB axis-aligned rays");
}

/// B36 + E15: `A.t == 0` => `p1 == p0` => every `da == db` => `d == 0` in
/// `c2RayToPlane_OneDimensional` (must yield `0`, not `0/0` NaN).
#[test]
fn b36_e15_zero_length_ray() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB36);
    let mut hits = 0;
    for _ in 0..20_000 {
        let b = rng.aabb_proper();
        let inside = c2v {
            x: (b.min.x + b.max.x) * 0.5,
            y: (b.min.y + b.max.y) * 0.5,
        };
        for (p, t) in [
            (inside, 0.0f32),
            (inside, -0.0f32),
            (rng.vec_nice(), 0.0f32),
        ] {
            let a = c2Ray {
                p,
                d: rng.unit(),
                t,
            };
            let (ret, out) = both(&mut d, "B36", a, b);
            if ret != 0 {
                hits += 1;
                // A.t == 0 => out->t == 0 * A.t
                d.check(out.t == 0.0, || {
                    format!("expected out->t == 0, got {}", fmt_f(out.t))
                });
            }
        }
    }
    assert!(hits > 0, "expected some zero-length-ray hits");
    d.finish("B36/E15 c2RaytoAABB zero-length ray");
}

/// B37: degenerate (`min == max`) and inverted (`min > max`) boxes.
#[test]
fn b37_degenerate_inverted_box() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB37);
    for _ in 0..20_000 {
        let p = rng.vec_nice();
        let q = rng.vec_nice();
        let boxes = [
            c2AABB { min: p, max: p },              // degenerate point box
            c2AABB { min: q, max: p },              // possibly inverted
            bx(1.0, 1.0, -1.0, -1.0),               // definitely inverted
            bx(0.0, 0.0, 0.0, 0.0),                 // origin point box
            bx(-0.0, -0.0, 0.0, 0.0),               // signed zero box
        ];
        let a = c2Ray {
            p: rng.vec_nice(),
            d: rng.unit(),
            t: (rng.uniform(20.0)).abs(),
        };
        for b in boxes {
            both(&mut d, "B37", a, b);
        }
    }
    d.finish("B37 c2RaytoAABB degenerate/inverted box");
}

/// B38: ray endpoints exactly on faces / corners of the box.
#[test]
fn b38_endpoints_on_faces() {
    let mut d = Diff::new();
    let pts = [
        c2v { x: -1.0, y: -1.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: -1.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: 0.0, y: 0.0 },
    ];
    for p0 in pts {
        for p1 in pts {
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let len = (dx * dx + dy * dy).sqrt();
            let (dir, t) = if len == 0.0 {
                (c2v { x: 1.0, y: 0.0 }, 0.0)
            } else {
                (c2v { x: dx / len, y: dy / len }, len)
            };
            let a = c2Ray { p: p0, d: dir, t };
            both(&mut d, "B38", a, UNIT);
            // also a longer ray through the same two points
            let a2 = c2Ray { t: t * 3.0, ..a };
            both(&mut d, "B38/long", a2, UNIT);
        }
    }
    d.finish("B38 c2RaytoAABB endpoints on faces");
}

/// B40 + E11 + E13: specials in every field; NaN geometry reaching `hit == 0`.
#[test]
fn b40_e11_e13_specials_per_field() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB40);
    for _ in 0..200 {
        let b = rng.aabb_proper();
        let base = c2Ray {
            p: rng.vec_nice(),
            d: rng.unit(),
            t: (rng.uniform(20.0)).abs() + 1.0,
        };
        for &s in &SPECIALS {
            both(&mut d, "B40/A.p.x", c2Ray { p: c2v { x: s, ..base.p }, ..base }, b);
            both(&mut d, "B40/A.p.y", c2Ray { p: c2v { y: s, ..base.p }, ..base }, b);
            both(&mut d, "B40/A.d.x", c2Ray { d: c2v { x: s, ..base.d }, ..base }, b);
            both(&mut d, "B40/A.d.y", c2Ray { d: c2v { y: s, ..base.d }, ..base }, b);
            both(&mut d, "B40/A.t", c2Ray { t: s, ..base }, b);
            both(&mut d, "B40/B.min.x", base, c2AABB { min: c2v { x: s, ..b.min }, ..b });
            both(&mut d, "B40/B.min.y", base, c2AABB { min: c2v { y: s, ..b.min }, ..b });
            both(&mut d, "B40/B.max.x", base, c2AABB { max: c2v { x: s, ..b.max }, ..b });
            both(&mut d, "B40/B.max.y", base, c2AABB { max: c2v { y: s, ..b.max }, ..b });
        }
    }

    // E11: ray bbox completely outside the box (all four separation axes).
    for (p, dir) in [
        (c2v { x: -100.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }),
        (c2v { x: 100.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 0.0, y: -100.0 }, c2v { x: 0.0, y: -1.0 }),
        (c2v { x: 0.0, y: 100.0 }, c2v { x: 0.0, y: 1.0 }),
    ] {
        let a = c2Ray { p, d: dir, t: 10.0 };
        let (ret, _) = both(&mut d, "E11", a, UNIT);
        d.check(ret == 0, || format!("E11 expected reject for {:?}", a));
    }

    // E13: all four `t_i` NaN => `hit == 0` even though nothing rejected
    // earlier (NaN passes the AABB overlap test and the `d > 0` test).
    let nan_ray = ray(f32::NAN, f32::NAN, f32::NAN, f32::NAN, f32::NAN);
    let nan_box = bx(f32::NAN, f32::NAN, f32::NAN, f32::NAN);
    let (ret, _) = both(&mut d, "E13/all-nan", nan_ray, nan_box);
    d.check(ret == 0, || {
        format!("E13 expected hit == 0 for all-NaN geometry, C returned {ret}")
    });
    for a in [
        ray(f32::INFINITY, f32::INFINITY, 1.0, 1.0, f32::INFINITY),
        ray(f32::NEG_INFINITY, 0.0, 1.0, 0.0, f32::INFINITY),
        ray(0.0, 0.0, f32::INFINITY, f32::INFINITY, f32::INFINITY),
    ] {
        for b in [
            UNIT,
            bx(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY),
            bx(f32::NAN, 0.0, 1.0, 1.0),
        ] {
            both(&mut d, "E13/inf", a, b);
        }
    }
    d.finish("B40/E11/E13 c2RaytoAABB specials");
}

/// B41: unconstrained fuzz.
#[test]
fn b41_fuzz() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB41);
    for _ in 0..20_000 {
        both(&mut d, "B41/nice", rng.ray_nice(), rng.aabb_nice());
    }
    for _ in 0..20_000 {
        both(&mut d, "B41/hostile", rng.ray_hostile(), rng.aabb_hostile());
    }
    for _ in 0..20_000 {
        both(&mut d, "B41/mix1", rng.ray_nice(), rng.aabb_hostile());
        both(&mut d, "B41/mix2", rng.ray_hostile(), rng.aabb_nice());
        both(&mut d, "B41/proper", rng.ray_nice(), rng.aabb_proper());
    }
    d.finish("B41 c2RaytoAABB fuzz");
}

/// Coverage evidence: the `else if (da * db > 0) return 1.0f;` arm of
/// `c2RayToPlane_OneDimensional` (`c_src/src/lib.c:128-129`) is **dead code**
/// when reached through its only caller, `c2RaytoAABB`.
///
/// Proof sketch (per plane, using `signedDist(p, n, d) == p*n - d*n`):
///   * plane 0 (`n = -1`, `d = B.min.x`): `da > 0 && db > 0`
///     <=> `p0.x < B.min.x && p1.x < B.min.x`
///     => `a_box.max.x = c2Maxv(p0,p1).x < B.min.x`
///     => `d1` in `c2AABBtoAABB(a_box, B)` is 1 => `c2RaytoAABB` already
///        returned 0 before any plane was evaluated.
///   * plane 1 (`n = +1`, `d = B.max.x`): symmetric, forces `d0`.
///   * planes 2/3: the same argument on `y`, forcing `d2`/`d3`.
///   The NaN escape hatch does not exist: `c2Maxv`/`c2Minv` return NaN only when
///   their SECOND argument (`p1`) is NaN, and then `db` is NaN, so `da*db > 0`
///   is false.  Infinities behave the same way in the extended reals.
///
/// This test hunts for a counterexample: it recomputes the C's own prologue via
/// the C library's exported leaf functions and asserts that no input ever
/// reaches `c2RayToPlane_OneDimensional` with `da > 0 && db > 0`.  gcov of the C
/// reference confirms the same (0 of 1 633 554 evaluations).
#[test]
fn dead_code_da_times_db_positive_is_unreachable() {
    let (c, _) = apis();
    let mut rng = Rng::new(0xDEAD);
    let sd = |p: f32, n: f32, dd: f32| p * n - dd * n;
    let mut evaluated = 0u64;
    let mut reached = 0u64;
    let mut counterexample: Option<String> = None;
    for i in 0..400_000u64 {
        let a = match i % 3 {
            0 => rng.ray_hostile(),
            1 => rng.ray_nice(),
            _ => c2Ray {
                p: c2v {
                    x: rng.hostile(),
                    y: rng.nice(),
                },
                d: c2v {
                    x: rng.nice(),
                    y: rng.hostile(),
                },
                t: rng.hostile(),
            },
        };
        let b = match i % 4 {
            0 => rng.aabb_hostile(),
            1 => rng.aabb_proper(),
            2 => rng.aabb_nice(),
            _ => c2AABB {
                min: c2v {
                    x: rng.hostile(),
                    y: rng.nice(),
                },
                max: c2v {
                    x: rng.nice(),
                    y: rng.hostile(),
                },
            },
        };
        // Replicate the C prologue with the C's own exports.
        let p0 = a.p;
        let p1 = (c.c2Add)(a.p, (c.c2Mulvs)(a.d, a.t));
        let a_box = c2AABB {
            min: (c.c2Minv)(p0, p1),
            max: (c.c2Maxv)(p0, p1),
        };
        if (c.c2AABBtoAABB)(a_box, b) == 0 {
            continue;
        }
        let n = (c.c2Skew)((c.c2Sub)(p1, p0));
        let abs_n = (c.c2Absv)(n);
        let he = (c.c2Mulvs)((c.c2Sub)(b.max, b.min), 0.5);
        let centre = (c.c2Mulvs)((c.c2Add)(b.min, b.max), 0.5);
        let dot0 = (c.c2Dot)(n, (c.c2Sub)(p0, centre));
        let dd = (if dot0 < 0.0 { -dot0 } else { dot0 }) - (c.c2Dot)(abs_n, he);
        if dd > 0.0 {
            continue;
        }
        for (da, db) in [
            (sd(p0.x, -1.0, b.min.x), sd(p1.x, -1.0, b.min.x)),
            (sd(p0.x, 1.0, b.max.x), sd(p1.x, 1.0, b.max.x)),
            (sd(p0.y, -1.0, b.min.y), sd(p1.y, -1.0, b.min.y)),
            (sd(p0.y, 1.0, b.max.y), sd(p1.y, 1.0, b.max.y)),
        ] {
            evaluated += 1;
            if !(da < 0.0) && da * db > 0.0 {
                reached += 1;
                if counterexample.is_none() {
                    counterexample = Some(format!(
                        "ray={:?} box={:?} da={} db={}",
                        a,
                        b,
                        fmt_f(da),
                        fmt_f(db)
                    ));
                }
            }
        }
    }
    eprintln!(
        "plane evaluations reached: {evaluated}, `da*db > 0` occurrences: {reached}"
    );
    assert!(evaluated > 100_000, "search did not reach enough planes");
    assert_eq!(
        reached, 0,
        "counterexample found for the supposedly dead `da*db > 0` branch: {}",
        counterexample.unwrap_or_default()
    );
}
