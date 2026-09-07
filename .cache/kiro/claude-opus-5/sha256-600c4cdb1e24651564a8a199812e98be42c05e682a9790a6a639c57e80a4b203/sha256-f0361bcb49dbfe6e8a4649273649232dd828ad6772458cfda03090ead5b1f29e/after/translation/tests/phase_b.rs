//! Phase B — valid-path differential tests. One test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through their `.so` exports.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

fn pair() -> Pair {
    load_pair()
}

// ---------------------------------------------------------------------------
// Rows 1-2 — trivial vector algebra
// ---------------------------------------------------------------------------

#[test]
fn row01_vector_algebra_finite() {
    let p = pair();
    let mut g = Rng::new(0x01);
    for i in 0..N {
        let (a, b) = (g.vec(), g.vec());
        let ctx = format!("row01 #{i} a={a:?} b={b:?}");
        diff(&ctx, &p, |x| unsafe {
            (
                (x.c2V)(a.x, a.y),
                (x.c2Sub)(a, b),
                (x.c2Add)(a, b),
                ((x.c2Neg)(a), (x.c2Skew)(a), (x.c2CCW90)(a)),
            )
        });
    }
}

#[test]
fn row02_vector_algebra_special_values() {
    let p = pair();
    let mut g = Rng::new(0x02);
    for i in 0..N {
        let (a, b) = (g.vec_wide(), g.vec_wide());
        let ctx = format!("row02 #{i} a={a:?} b={b:?}");
        diff(&ctx, &p, |x| unsafe {
            (
                (x.c2V)(a.x, a.y),
                (x.c2Sub)(a, b),
                (x.c2Add)(a, b),
                ((x.c2Neg)(a), (x.c2Skew)(a), (x.c2CCW90)(a)),
            )
        });
    }
}

// ---------------------------------------------------------------------------
// Row 3 — scalar multiply / divide
// ---------------------------------------------------------------------------

#[test]
fn row03_mulvs_div() {
    let p = pair();
    let mut g = Rng::new(0x03);
    for i in 0..N {
        let a = g.vec_wide();
        let s = g.coord_wide();
        let ctx = format!("row03 #{i} a={a:?} s={s:e}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Mulvs)(a, s), (x.c2Div)(a, s)) });
    }
}

// ---------------------------------------------------------------------------
// Row 4 — dot / det
// ---------------------------------------------------------------------------

#[test]
fn row04_dot_det2() {
    let p = pair();
    let mut g = Rng::new(0x04);
    for i in 0..N {
        let (a, b) = if i % 2 == 0 {
            (g.vec(), g.vec())
        } else {
            (g.vec_wide(), g.vec_wide())
        };
        let ctx = format!("row04 #{i} a={a:?} b={b:?}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Dot)(a, b), (x.c2Det2)(a, b)) });
    }
}

// ---------------------------------------------------------------------------
// Row 5 — length / normalise (incl. divide-by-zero)
// ---------------------------------------------------------------------------

#[test]
fn row05_len_norm() {
    let p = pair();
    let mut g = Rng::new(0x05);
    let fixed = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v {
            x: f32::MAX,
            y: f32::MAX,
        },
        c2v {
            x: f32::NAN,
            y: 1.0,
        },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
        c2v {
            x: f32::from_bits(1),
            y: f32::from_bits(1),
        },
    ];
    for (i, a) in fixed.iter().enumerate() {
        let a = *a;
        let ctx = format!("row05 fixed #{i} a={a:?}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Len)(a), (x.c2Norm)(a)) });
    }
    for i in 0..N {
        let a = if i % 3 == 0 { g.vec_wide() } else { g.vec() };
        let ctx = format!("row05 #{i} a={a:?}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Len)(a), (x.c2Norm)(a)) });
    }
}

// ---------------------------------------------------------------------------
// Row 6 — max / min (raw ternary NaN semantics)
// ---------------------------------------------------------------------------

#[test]
fn row06_maxv_minv() {
    let p = pair();
    let mut g = Rng::new(0x06);
    let nan = f32::NAN;
    let fixed: [(c2v, c2v); 5] = [
        (c2v { x: nan, y: 1.0 }, c2v { x: 1.0, y: nan }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: 1.0, y: 2.0 }),
        (
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            c2v { x: nan, y: 0.0 },
        ),
    ];
    for (i, (a, b)) in fixed.iter().enumerate() {
        let (a, b) = (*a, *b);
        let ctx = format!("row06 fixed #{i}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Maxv)(a, b), (x.c2Minv)(a, b)) });
    }
    for i in 0..N {
        let (a, b) = if i % 2 == 0 {
            (g.vec(), g.vec())
        } else {
            (g.vec_wide(), g.vec_wide())
        };
        let ctx = format!("row06 #{i} a={a:?} b={b:?}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Maxv)(a, b), (x.c2Minv)(a, b)) });
    }
}

// ---------------------------------------------------------------------------
// Row 7 — clamp, incl. inverted bounds
// ---------------------------------------------------------------------------

#[test]
fn row07_clampv() {
    let p = pair();
    let mut g = Rng::new(0x07);
    for i in 0..N {
        let a = if i % 4 == 0 { g.vec_wide() } else { g.vec() };
        let (mut lo, mut hi) = (g.vec(), g.vec());
        match i % 4 {
            0 => {}                                  // possibly inverted / NaN
            1 => {
                // well-ordered
                if lo.x > hi.x {
                    core::mem::swap(&mut lo.x, &mut hi.x);
                }
                if lo.y > hi.y {
                    core::mem::swap(&mut lo.y, &mut hi.y);
                }
            }
            2 => hi = lo, // lo == hi
            _ => {
                core::mem::swap(&mut lo, &mut hi); // deliberately inverted
            }
        }
        let ctx = format!("row07 #{i} a={a:?} lo={lo:?} hi={hi:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2Clampv)(a, lo, hi) });
    }
}

// ---------------------------------------------------------------------------
// Row 8 — identities
// ---------------------------------------------------------------------------

#[test]
fn row08_identities() {
    let p = pair();
    diff("row08", &p, |x| unsafe {
        ((x.c2RotIdentity)(), (x.c2xIdentity)())
    });
}

// ---------------------------------------------------------------------------
// Rows 9-10 — rotations and transforms
// ---------------------------------------------------------------------------

#[test]
fn row09_mulrv_mulrvT() {
    let p = pair();
    let mut g = Rng::new(0x09);
    for i in 0..N {
        let r = if i % 5 == 0 {
            c2r {
                c: g.coord_wide(),
                s: g.coord_wide(),
            }
        } else {
            g.rot()
        };
        let b = if i % 3 == 0 { g.vec_wide() } else { g.vec() };
        let ctx = format!("row09 #{i} r={r:?} b={b:?}");
        diff(&ctx, &p, |x| unsafe { ((x.c2Mulrv)(r, b), (x.c2MulrvT)(r, b)) });
    }
}

#[test]
fn row10_mulxv() {
    let p = pair();
    let mut g = Rng::new(0x0A);
    for i in 0..N {
        let mut t = g.xform();
        match i % 5 {
            0 => t.p = c2v { x: 0.0, y: 0.0 },
            1 => t.r = c2r { c: 1.0, s: 0.0 },
            2 => {
                t.p = c2v {
                    x: g.coord_wide(),
                    y: g.coord_wide(),
                }
            }
            _ => {}
        }
        let b = if i % 3 == 0 { g.vec_wide() } else { g.vec() };
        let ctx = format!("row10 #{i} t={t:?} b={b:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2Mulxv)(t, b) });
    }
}

// ---------------------------------------------------------------------------
// Row 11 — c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn row11_bbverts() {
    let p = pair();
    let mut g = Rng::new(0x0B);
    for i in 0..N {
        let bb = if i % 5 == 0 {
            c2AABB {
                min: g.vec_wide(),
                max: g.vec_wide(),
            }
        } else {
            g.aabb()
        };
        let ctx = format!("row11 #{i} bb={bb:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut out = [c2v::default(); 4];
            let mut b = bb;
            (x.c2BBVerts)(out.as_mut_ptr(), &mut b);
            (out, b)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 12 — c2Support with every vert count
// ---------------------------------------------------------------------------

#[test]
fn row12_support() {
    let p = pair();
    let mut g = Rng::new(0x0C);
    for i in 0..N {
        let count = [1i32, 2, 3, 4, 8][i % 5];
        let mut verts = [c2v::default(); 8];
        for k in 0..8 {
            verts[k] = g.vec();
        }
        // Force exact ties every few cases so the `>` (keep-first) rule is
        // exercised, not just strict maxima.
        if i % 7 == 0 && count >= 2 {
            let v0 = verts[0];
            for k in 0..count as usize {
                verts[k] = v0;
            }
        }
        let d = match i % 6 {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => g.vec_wide(),
            _ => g.vec(),
        };
        let ctx = format!("row12 #{i} count={count} d={d:?} verts={verts:?}");
        diff(&ctx, &p, |x| unsafe {
            (x.c2Support)(verts.as_ptr(), count, d)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 13-15 — c2MakeProxy for each type
// ---------------------------------------------------------------------------

#[test]
fn row13_makeproxy_circle() {
    let p = pair();
    let mut g = Rng::new(0x0D);
    for i in 0..N {
        let c = g.circle();
        let ctx = format!("row13 #{i} c={c:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut pr = c2Proxy::default();
            let mut c = c;
            (x.c2MakeProxy)(&mut c as *mut _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
            pr
        });
    }
}

#[test]
fn row14_makeproxy_aabb() {
    let p = pair();
    let mut g = Rng::new(0x0E);
    for i in 0..N {
        let bb = if i % 5 == 0 {
            c2AABB {
                min: g.vec_wide(),
                max: g.vec_wide(),
            }
        } else {
            g.aabb()
        };
        let ctx = format!("row14 #{i} bb={bb:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut pr = c2Proxy::default();
            let mut bb = bb;
            (x.c2MakeProxy)(&mut bb as *mut _ as *const c_void, C2_TYPE_AABB, &mut pr);
            pr
        });
    }
}

#[test]
fn row15_makeproxy_capsule() {
    let p = pair();
    let mut g = Rng::new(0x0F);
    for i in 0..N {
        let c = g.capsule();
        let ctx = format!("row15 #{i} c={c:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut pr = c2Proxy::default();
            let mut c = c;
            (x.c2MakeProxy)(&mut c as *mut _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
            pr
        });
    }
}

// ---------------------------------------------------------------------------
// Row 16 — c2GJKSimplexMetric for counts 1..3
// ---------------------------------------------------------------------------

#[test]
fn row16_simplex_metric() {
    let p = pair();
    let mut g = Rng::new(0x10);
    for i in 0..N {
        let count = [1i32, 2, 3][i % 3];
        let s = g.simplex(count);
        let ctx = format!("row16 #{i} count={count}");
        diff(&ctx, &p, |x| unsafe {
            let mut s = s;
            let m = (x.c2GJKSimplexMetric)(&mut s);
            (m, s)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 17-24 — the low-level simplex reductions c22 / c23
//
// The branch classifiers below mirror the C `if` chain purely so the tests can
// *target* each branch; correctness is still decided by the C/Rust diff.
// ---------------------------------------------------------------------------

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// 0 = `v<=0` vertex-a, 1 = `u<=0` vertex-b, 2 = interior edge.
fn branch_c22(s: &c2Simplex) -> usize {
    let (a, b) = (s.v[0].p, s.v[1].p);
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

/// 0..=6 in the same order as the C `if / else if` chain in `c23`.
fn branch_c23(s: &c2Simplex) -> usize {
    let (a, b, c) = (s.v[0].p, s.v[1].p, s.v[2].p);
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det2(sub(b, a), sub(c, a));
    let uABC = det2(b, c) * area;
    let vABC = det2(c, a) * area;
    let wABC = det2(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

/// Runs `c22` on `n` random simplexes whose branch index satisfies `want`.
fn run_c22(p: &Pair, seed: u64, tag: &str, n: usize, want: impl Fn(usize) -> bool) -> [usize; 3] {
    let mut g = Rng::new(seed);
    let mut hits = [0usize; 3];
    let mut done = 0usize;
    let mut tries = 0usize;
    while done < n && tries < n * 400 + 100_000 {
        tries += 1;
        let mut s = g.simplex(2);
        // `c2L`/`c2Witness` are not involved here, but `p` drives the branch.
        if tries % 9 == 0 {
            s.v[1].p = s.v[0].p; // degenerate a==b
        }
        let br = branch_c22(&s);
        if !want(br) {
            continue;
        }
        hits[br] += 1;
        done += 1;
        let ctx = format!("{tag} #{done} branch={br} s={s:?}");
        diff(&ctx, p, |x| unsafe {
            let mut s = s;
            (x.c22)(&mut s);
            s
        });
    }
    assert!(done >= n, "{tag}: only produced {done}/{n} targeted cases");
    hits
}

fn run_c23(p: &Pair, seed: u64, tag: &str, n: usize, want: impl Fn(usize) -> bool) -> [usize; 7] {
    let mut g = Rng::new(seed);
    let mut hits = [0usize; 7];
    let mut done = 0usize;
    let mut tries = 0usize;
    while done < n && tries < n * 800 + 400_000 {
        tries += 1;
        let mut s = g.simplex(3);
        match tries % 11 {
            0 => s.v[2].p = s.v[0].p,             // degenerate
            1 => s.v[1].p = s.v[0].p,             // degenerate
            2 => {
                // collinear => area == 0
                let a = s.v[0].p;
                let d = sub(s.v[1].p, a);
                s.v[2].p = c2v {
                    x: a.x + 2.0 * d.x,
                    y: a.y + 2.0 * d.y,
                };
            }
            _ => {}
        }
        let br = branch_c23(&s);
        if !want(br) {
            continue;
        }
        hits[br] += 1;
        done += 1;
        let ctx = format!("{tag} #{done} branch={br} s={s:?}");
        diff(&ctx, p, |x| unsafe {
            let mut s = s;
            (x.c23)(&mut s);
            s
        });
    }
    assert!(done >= n, "{tag}: only produced {done}/{n} targeted cases");
    hits
}

#[test]
fn row17_c22_branch_v_le_zero() {
    let p = pair();
    run_c22(&p, 0x11, "row17", 800, |b| b == 0);
}

#[test]
fn row18_c22_branch_u_le_zero() {
    let p = pair();
    run_c22(&p, 0x12, "row18", 800, |b| b == 1);
}

#[test]
fn row19_c22_branch_interior() {
    let p = pair();
    run_c22(&p, 0x13, "row19", 800, |b| b == 2);
}

#[test]
fn row20_c22_random() {
    let p = pair();
    let hits = run_c22(&p, 0x14, "row20", N, |_| true);
    for (b, h) in hits.iter().enumerate() {
        assert!(*h > 0, "row20: c22 branch {b} never taken ({hits:?})");
    }
}

#[test]
fn row21_c23_vertex_regions() {
    let p = pair();
    for (i, br) in [0usize, 1, 2].iter().enumerate() {
        let br = *br;
        run_c23(&p, 0x21 + i as u64, &format!("row21.{br}"), 400, |b| b == br);
    }
}

#[test]
fn row22_c23_edge_regions() {
    let p = pair();
    for (i, br) in [3usize, 4, 5].iter().enumerate() {
        let br = *br;
        run_c23(&p, 0x31 + i as u64, &format!("row22.{br}"), 400, |b| b == br);
    }
}

#[test]
fn row23_c23_interior() {
    let p = pair();
    run_c23(&p, 0x41, "row23", 800, |b| b == 6);
}

#[test]
fn row24_c23_random() {
    let p = pair();
    let hits = run_c23(&p, 0x42, "row24", N, |_| true);
    for (b, h) in hits.iter().enumerate() {
        assert!(*h > 0, "row24: c23 branch {b} never taken ({hits:?})");
    }
}

// ---------------------------------------------------------------------------
// Rows 25-27 — c2D / c2L / c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row25_c2D() {
    let p = pair();
    let mut g = Rng::new(0x51);
    let mut skew = 0usize;
    let mut ccw = 0usize;
    for i in 0..N {
        let count = [1i32, 2, 2, 3][i % 4];
        let mut s = g.simplex(count);
        if i % 13 == 0 {
            s.v[1].p = s.v[0].p;
        }
        if count == 2 {
            let ab = sub(s.v[1].p, s.v[0].p);
            let neg = c2v {
                x: -s.v[0].p.x,
                y: -s.v[0].p.y,
            };
            if det2(ab, neg) > 0.0 {
                skew += 1
            } else {
                ccw += 1
            }
        }
        let ctx = format!("row25 #{i} count={count} s={s:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut s = s;
            let d = (x.c2D)(&mut s);
            (d, s)
        });
    }
    assert!(skew > 0 && ccw > 0, "row25: skew={skew} ccw={ccw}");
}

#[test]
fn row26_c2L() {
    let p = pair();
    let mut g = Rng::new(0x52);
    for i in 0..N {
        let count = [1i32, 2, 2, 2][i % 4];
        let mut s = g.simplex(count);
        if i % 6 == 0 {
            s.div = 0.0; // 1/0 => Inf/NaN, reproduced verbatim by both sides
        }
        if i % 17 == 0 {
            s.v[0].u = f32::NAN;
        }
        let ctx = format!("row26 #{i} count={count} s={s:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut s = s;
            let l = (x.c2L)(&mut s);
            (l, s)
        });
    }
}

#[test]
fn row27_c2Witness() {
    let p = pair();
    let mut g = Rng::new(0x53);
    for i in 0..N {
        let count = [1i32, 2, 3][i % 3];
        let mut s = g.simplex(count);
        if i % 6 == 0 {
            s.div = 0.0;
        }
        if i % 19 == 0 {
            s.v[2].u = f32::INFINITY;
        }
        let ctx = format!("row27 #{i} count={count} s={s:?}");
        diff(&ctx, &p, |x| unsafe {
            let mut s = s;
            let mut a = c2v {
                x: 12.5,
                y: -3.25,
            };
            let mut b = c2v { x: -7.0, y: 9.0 };
            (x.c2Witness)(&mut s, &mut a, &mut b);
            (a, b, s)
        });
    }
}


// ---------------------------------------------------------------------------
// Rows 28-34 — c2GJK per type pair, use_radius on/off, NULL transforms
// ---------------------------------------------------------------------------

/// Places the two shapes so that separated / touching / overlapping /
/// identical / nested all occur.
fn placed_pair(g: &mut Rng, ta: c_int, tb: c_int) -> (Shape, Shape) {
    let a = rand_shape(g, ta);
    let mut b = rand_shape(g, tb);
    // Bias half the cases to be close together so the overlap and touching
    // paths (hit=1, s.count==3) are reached often.
    if g.bool() {
        let d = c2v {
            x: g.range(-4.0, 4.0),
            y: g.range(-4.0, 4.0),
        };
        b = match (a, b) {
            (Shape::Circle(ca), Shape::Circle(mut cb)) => {
                cb.p = c2v {
                    x: ca.p.x + d.x,
                    y: ca.p.y + d.y,
                };
                Shape::Circle(cb)
            }
            (Shape::Circle(ca), Shape::Aabb(_)) => Shape::Aabb(c2AABB {
                min: c2v {
                    x: ca.p.x + d.x,
                    y: ca.p.y + d.y,
                },
                max: c2v {
                    x: ca.p.x + d.x + g.range(0.0, 6.0),
                    y: ca.p.y + d.y + g.range(0.0, 6.0),
                },
            }),
            (Shape::Circle(ca), Shape::Capsule(mut cb)) => {
                cb.a = c2v {
                    x: ca.p.x + d.x,
                    y: ca.p.y + d.y,
                };
                cb.b = c2v {
                    x: cb.a.x + g.range(-6.0, 6.0),
                    y: cb.a.y + g.range(-6.0, 6.0),
                };
                Shape::Capsule(cb)
            }
            (Shape::Aabb(aa), Shape::Circle(mut cb)) => {
                cb.p = c2v {
                    x: aa.min.x + d.x,
                    y: aa.min.y + d.y,
                };
                Shape::Circle(cb)
            }
            (Shape::Aabb(aa), Shape::Aabb(_)) => Shape::Aabb(c2AABB {
                min: c2v {
                    x: aa.min.x + d.x,
                    y: aa.min.y + d.y,
                },
                max: c2v {
                    x: aa.max.x + d.x,
                    y: aa.max.y + d.y,
                },
            }),
            (Shape::Aabb(aa), Shape::Capsule(mut cb)) => {
                cb.a = c2v {
                    x: aa.min.x + d.x,
                    y: aa.min.y + d.y,
                };
                cb.b = c2v {
                    x: aa.max.x + d.x,
                    y: aa.max.y + d.y,
                };
                Shape::Capsule(cb)
            }
            (Shape::Capsule(ka), Shape::Circle(mut cb)) => {
                cb.p = c2v {
                    x: ka.a.x + d.x,
                    y: ka.a.y + d.y,
                };
                Shape::Circle(cb)
            }
            (Shape::Capsule(ka), Shape::Aabb(_)) => Shape::Aabb(c2AABB {
                min: c2v {
                    x: ka.a.x + d.x,
                    y: ka.a.y + d.y,
                },
                max: c2v {
                    x: ka.b.x + d.x + 1.0,
                    y: ka.b.y + d.y + 1.0,
                },
            }),
            (Shape::Capsule(ka), Shape::Capsule(mut cb)) => {
                if g.bool() {
                    // parallel / collinear / identical
                    cb.a = c2v {
                        x: ka.a.x + d.x,
                        y: ka.a.y + d.y,
                    };
                    cb.b = c2v {
                        x: ka.b.x + d.x,
                        y: ka.b.y + d.y,
                    };
                } else {
                    cb.a = c2v {
                        x: ka.a.x + d.x,
                        y: ka.b.y + d.y,
                    };
                    cb.b = c2v {
                        x: ka.b.x + d.x,
                        y: ka.a.y + d.y,
                    };
                }
                Shape::Capsule(cb)
            }
        };
    }
    (a, b)
}

macro_rules! gjk_pair_test {
    ($name:ident, $row:expr, $seed:expr, $ta:expr, $tb:expr) => {
        #[test]
        fn $name() {
            let p = pair();
            gjk_row(&p, $row, $seed, N, |g| {
                let (a, b) = placed_pair(g, $ta, $tb);
                let o = GjkOpts {
                    use_radius: if g.bool() { 1 } else { 0 },
                    ..Default::default()
                };
                (a, b, o)
            });
        }
    };
}

#[test]
fn row28_gjk_circle_circle_no_radius() {
    let p = pair();
    gjk_row(&p, "row28", 0x61, N, |g| {
        let (a, b) = placed_pair(g, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
        (a, b, GjkOpts::default())
    });
}

#[test]
fn row29_gjk_circle_circle_radius() {
    let p = pair();
    gjk_row(&p, "row29", 0x62, N, |g| {
        let (a, b) = placed_pair(g, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
        (
            a,
            b,
            GjkOpts {
                use_radius: 1,
                ..Default::default()
            },
        )
    });
}

gjk_pair_test!(row30a_gjk_circle_aabb, "row30a", 0x63, C2_TYPE_CIRCLE, C2_TYPE_AABB);
gjk_pair_test!(row30b_gjk_aabb_circle, "row30b", 0x64, C2_TYPE_AABB, C2_TYPE_CIRCLE);
gjk_pair_test!(row31a_gjk_circle_capsule, "row31a", 0x65, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE);
gjk_pair_test!(row31b_gjk_capsule_circle, "row31b", 0x66, C2_TYPE_CAPSULE, C2_TYPE_CIRCLE);
gjk_pair_test!(row32_gjk_aabb_aabb, "row32", 0x67, C2_TYPE_AABB, C2_TYPE_AABB);
gjk_pair_test!(row33a_gjk_aabb_capsule, "row33a", 0x68, C2_TYPE_AABB, C2_TYPE_CAPSULE);
gjk_pair_test!(row33b_gjk_capsule_aabb, "row33b", 0x69, C2_TYPE_CAPSULE, C2_TYPE_AABB);
gjk_pair_test!(row34_gjk_capsule_capsule, "row34", 0x6A, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE);

// ---------------------------------------------------------------------------
// Rows 35-37 — transform handling
// ---------------------------------------------------------------------------

#[test]
fn row35_gjk_asymmetric_transform() {
    let p = pair();
    for (i, ta) in TYPES.iter().enumerate() {
        for (j, tb) in TYPES.iter().enumerate() {
            let (ta, tb) = (*ta, *tb);
            gjk_row(
                &p,
                &format!("row35 {ta}x{tb}"),
                0x70 + (i * 3 + j) as u64,
                N / 4,
                |g| {
                    let (a, b) = placed_pair(g, ta, tb);
                    let o = GjkOpts {
                        use_radius: if g.bool() { 1 } else { 0 },
                        ax: Some(c2x {
                            p: c2v { x: 0.0, y: 0.0 },
                            r: c2r { c: 1.0, s: 0.0 },
                        }),
                        bx: None,
                        ..Default::default()
                    };
                    (a, b, o)
                },
            );
        }
    }
}

#[test]
fn row36_gjk_both_transforms_rotated() {
    let p = pair();
    for (i, ta) in TYPES.iter().enumerate() {
        for (j, tb) in TYPES.iter().enumerate() {
            let (ta, tb) = (*ta, *tb);
            gjk_row(
                &p,
                &format!("row36 {ta}x{tb}"),
                0x80 + (i * 3 + j) as u64,
                N / 4,
                |g| {
                    let (a, b) = placed_pair(g, ta, tb);
                    let t1 = g.range(-3.141_592_7, 3.141_592_7);
                    let t2 = g.range(-3.141_592_7, 3.141_592_7);
                    let o = GjkOpts {
                        use_radius: if g.bool() { 1 } else { 0 },
                        ax: Some(c2x {
                            p: g.vec(),
                            r: c2r {
                                c: t1.cos(),
                                s: t1.sin(),
                            },
                        }),
                        bx: Some(c2x {
                            p: g.vec(),
                            r: c2r {
                                c: t2.cos(),
                                s: t2.sin(),
                            },
                        }),
                        ..Default::default()
                    };
                    (a, b, o)
                },
            );
        }
    }
}

#[test]
fn row37_gjk_non_normalised_transforms() {
    let p = pair();
    for (i, ta) in TYPES.iter().enumerate() {
        for (j, tb) in TYPES.iter().enumerate() {
            let (ta, tb) = (*ta, *tb);
            gjk_row(
                &p,
                &format!("row37 {ta}x{tb}"),
                0x90 + (i * 3 + j) as u64,
                N / 4,
                |g| {
                    let (a, b) = placed_pair(g, ta, tb);
                    let o = GjkOpts {
                        use_radius: if g.bool() { 1 } else { 0 },
                        ax: Some(c2x {
                            p: g.vec(),
                            r: c2r {
                                c: g.range(-2.0, 2.0),
                                s: g.range(-2.0, 2.0),
                            },
                        }),
                        bx: Some(c2x {
                            p: g.vec(),
                            r: c2r {
                                c: g.range(-2.0, 2.0),
                                s: g.range(-2.0, 2.0),
                            },
                        }),
                        ..Default::default()
                    };
                    (a, b, o)
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 38 — NULL out-parameter cross product
// ---------------------------------------------------------------------------

#[test]
fn row38_gjk_null_outparams() {
    let p = pair();
    let mut variant = 0usize;
    for want_a in [false, true] {
        for want_b in [false, true] {
            for want_it in [false, true] {
                for ur in [0, 1] {
                    let v = variant;
                    variant += 1;
                    gjk_row(&p, &format!("row38 v{v}"), 0xA0 + v as u64, N / 8, |g| {
                        let ta = TYPES[g.below(3) as usize];
                        let tb = TYPES[g.below(3) as usize];
                        let (a, b) = placed_pair(g, ta, tb);
                        (
                            a,
                            b,
                            GjkOpts {
                                use_radius: ur,
                                want_a,
                                want_b,
                                want_iters: want_it,
                                ..Default::default()
                            },
                        )
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 39-42 — the GJK cache
// ---------------------------------------------------------------------------

#[test]
fn row39_gjk_cold_cache() {
    let p = pair();
    gjk_row(&p, "row39", 0xB0, N, |g| {
        let ta = TYPES[g.below(3) as usize];
        let tb = TYPES[g.below(3) as usize];
        let (a, b) = placed_pair(g, ta, tb);
        (
            a,
            b,
            GjkOpts {
                use_radius: if g.bool() { 1 } else { 0 },
                // count == 0 => cache_was_good false; other fields are garbage
                // on purpose, the C must ignore them.
                cache: Some(c2GJKCache {
                    metric: g.coord(),
                    count: 0,
                    iA: [7, 6, 5],
                    iB: [4, 3, 2],
                    div: g.coord(),
                }),
                ..Default::default()
            },
        )
    });
}

#[test]
fn row40_gjk_warm_cache_roundtrip() {
    let p = pair();
    let mut g = Rng::new(0xB1);
    for i in 0..N {
        let ta = TYPES[g.below(3) as usize];
        let tb = TYPES[g.below(3) as usize];
        let (sa, sb) = placed_pair(&mut g, ta, tb);
        let ur = if g.bool() { 1 } else { 0 };
        let ctx = format!("row40 #{i} A={sa:?} B={sb:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let mut o = GjkOpts {
                use_radius: ur,
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let first = call_gjk(x, &sa, &sb, &o);
            o.cache = first.cache; // carry the warmed cache into call #2
            let second = call_gjk(x, &sa, &sb, &o);
            let third = call_gjk(x, &sa, &sb, &GjkOpts { cache: second.cache, ..o });
            vec![first, second, third]
        });
    }
}

#[test]
fn row41_gjk_stale_cache_moved_shapes() {
    let p = pair();
    let mut g = Rng::new(0xB2);
    for i in 0..N {
        let ta = TYPES[g.below(3) as usize];
        let tb = TYPES[g.below(3) as usize];
        let (sa1, sb1) = placed_pair(&mut g, ta, tb);
        let (sa2, sb2) = placed_pair(&mut g, ta, tb);
        let ur = if g.bool() { 1 } else { 0 };
        let ctx = format!("row41 #{i} A1={sa1:?} B1={sb1:?} A2={sa2:?} B2={sb2:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let o = GjkOpts {
                use_radius: ur,
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let warm = call_gjk(x, &sa1, &sb1, &o);
            let stale = call_gjk(
                x,
                &sa2,
                &sb2,
                &GjkOpts {
                    cache: warm.cache,
                    ..o
                },
            );
            vec![warm, stale]
        });
    }
}

#[test]
fn row42_gjk_cache_reused_across_type_pairs() {
    let p = pair();
    let mut g = Rng::new(0xB3);
    for i in 0..N {
        // Cross-type cache reuse. The C never validates the cached vertex
        // indices against the NEW proxy's `count`, so if the cache came from a
        // 4-vert AABB and is replayed against a 1-vert circle, the C reads
        // `pA.verts[3]` of an *uninitialised* `c2Proxy pA;` stack local — real
        // undefined behaviour with no reproducible value (ERRORS.md row 19/20).
        // The DEFINED part of this configuration is reuse where every cached
        // index is still in range for the new proxy, so the warm-up pair is a
        // circle/circle pair (all indices are 0 and therefore always valid).
        let (sa1, sb1) = placed_pair(&mut g, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
        let ta = TYPES[g.below(3) as usize];
        let tb = TYPES[g.below(3) as usize];
        let (sa2, sb2) = placed_pair(&mut g, ta, tb);
        let ur = if g.bool() { 1 } else { 0 };
        let ctx = format!("row42 #{i} A1={sa1:?} B1={sb1:?} A2={sa2:?} B2={sb2:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let o = GjkOpts {
                use_radius: ur,
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let warm = call_gjk(x, &sa1, &sb1, &o);
            // Sanity: a circle/circle cache only ever holds index 0.
            if let Some(c) = warm.cache {
                assert!(
                    c.iA[..].iter().all(|v| *v == 0) && c.iB[..].iter().all(|v| *v == 0),
                    "row42: unexpected circle/circle cache indices {c:?}"
                );
            }
            let reused = call_gjk(
                x,
                &sa2,
                &sb2,
                &GjkOpts {
                    cache: warm.cache,
                    ..o
                },
            );
            vec![warm, reused]
        });
    }
}

/// Companion to row 42: reuse a *capsule* cache (indices 0..=1) against
/// capsule/AABB proxies, which have >= 2 verts, so every cached index stays in
/// range and the comparison remains well defined.
#[test]
fn row42b_gjk_cache_reused_capsule_to_wider() {
    let p = pair();
    let mut g = Rng::new(0xB4);
    for i in 0..N {
        let (sa1, sb1) = placed_pair(&mut g, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE);
        let wide = [C2_TYPE_AABB, C2_TYPE_CAPSULE];
        let ta = wide[g.below(2) as usize];
        let tb = wide[g.below(2) as usize];
        let (sa2, sb2) = placed_pair(&mut g, ta, tb);
        let ur = if g.bool() { 1 } else { 0 };
        let ctx = format!("row42b #{i} A1={sa1:?} B1={sb1:?} A2={sa2:?} B2={sb2:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let o = GjkOpts {
                use_radius: ur,
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let warm = call_gjk(x, &sa1, &sb1, &o);
            if let Some(c) = warm.cache {
                assert!(
                    c.iA[..].iter().all(|v| *v <= 1) && c.iB[..].iter().all(|v| *v <= 1),
                    "row42b: unexpected capsule cache indices {c:?}"
                );
            }
            let reused = call_gjk(
                x,
                &sa2,
                &sb2,
                &GjkOpts {
                    cache: warm.cache,
                    ..o
                },
            );
            vec![warm, reused]
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 43-44 — iteration cap, early breaks, degenerate shapes
// ---------------------------------------------------------------------------

#[test]
fn row43_gjk_iteration_and_early_break() {
    let p = pair();
    gjk_row(&p, "row43", 0xC0, N, |g| {
        let ta = TYPES[g.below(3) as usize];
        let tb = TYPES[g.below(3) as usize];
        let scale = [1.0e-30f32, 1.0e-8, 1.0e-3, 1.0, 1.0e6, 1.0e18, 1.0e30][g.below(7) as usize];
        let mk = |g: &mut Rng, t: c_int| -> Shape {
            let v = |g: &mut Rng| c2v {
                x: g.range(-1.0, 1.0) * scale,
                y: g.range(-1.0, 1.0) * scale,
            };
            match t {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: v(g),
                    r: g.range(0.0, 1.0) * scale,
                }),
                C2_TYPE_AABB => {
                    let a = v(g);
                    let b = v(g);
                    Shape::Aabb(c2AABB {
                        min: c2v {
                            x: a.x.min(b.x),
                            y: a.y.min(b.y),
                        },
                        max: c2v {
                            x: a.x.max(b.x),
                            y: a.y.max(b.y),
                        },
                    })
                }
                _ => Shape::Capsule(c2Capsule {
                    a: v(g),
                    b: v(g),
                    r: g.range(0.0, 1.0) * scale,
                }),
            }
        };
        let a = mk(g, ta);
        let b = mk(g, tb);
        (
            a,
            b,
            GjkOpts {
                use_radius: if g.bool() { 1 } else { 0 },
                ..Default::default()
            },
        )
    });
}

#[test]
fn row44_gjk_degenerate_shapes() {
    let p = pair();
    gjk_row(&p, "row44", 0xC1, N, |g| {
        let mk = |g: &mut Rng| -> Shape {
            match g.below(6) {
                0 => Shape::Circle(c2Circle {
                    p: g.vec(),
                    r: 0.0,
                }),
                1 => Shape::Circle(c2Circle {
                    p: g.vec(),
                    r: g.range(-8.0, 0.0),
                }),
                2 => {
                    let v = g.vec();
                    Shape::Aabb(c2AABB { min: v, max: v })
                }
                3 => {
                    // inverted box
                    let a = g.vec();
                    let b = g.vec();
                    Shape::Aabb(c2AABB {
                        min: c2v {
                            x: a.x.max(b.x),
                            y: a.y.max(b.y),
                        },
                        max: c2v {
                            x: a.x.min(b.x),
                            y: a.y.min(b.y),
                        },
                    })
                }
                4 => {
                    let v = g.vec();
                    Shape::Capsule(c2Capsule {
                        a: v,
                        b: v,
                        r: g.radius(),
                    })
                }
                _ => Shape::Capsule(c2Capsule {
                    a: g.vec(),
                    b: g.vec(),
                    r: 0.0,
                }),
            }
        };
        let a = mk(g);
        let b = mk(g);
        (
            a,
            b,
            GjkOpts {
                use_radius: if g.bool() { 1 } else { 0 },
                ..Default::default()
            },
        )
    });
}

// ---------------------------------------------------------------------------
// Rows 45-50 — the boolean convenience routines
// ---------------------------------------------------------------------------

#[test]
fn row45_aabb_to_aabb() {
    let p = pair();
    let mut g = Rng::new(0xD0);
    for i in 0..N {
        let a = g.aabb();
        let b = match i % 6 {
            0 => a,
            1 => c2AABB {
                min: a.max,
                max: c2v {
                    x: a.max.x + 1.0,
                    y: a.max.y + 1.0,
                },
            }, // exactly touching
            2 => c2AABB {
                min: g.vec_wide(),
                max: g.vec_wide(),
            },
            3 => c2AABB {
                min: c2v {
                    x: a.min.x + g.range(-2.0, 2.0),
                    y: a.min.y,
                },
                max: a.max,
            },
            _ => g.aabb(),
        };
        let ctx = format!("row45 #{i} A={a:?} B={b:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2AABBtoAABB)(a, b) });
    }
}

#[test]
fn row46_circle_to_circle() {
    let p = pair();
    let mut g = Rng::new(0xD1);
    for i in 0..N {
        let a = g.circle();
        let b = match i % 5 {
            0 => a,
            1 => c2Circle {
                // exactly touching: |c| == rA + rB  =>  d2 == r2  =>  strict < is false
                p: c2v {
                    x: a.p.x + a.r + 3.0,
                    y: a.p.y,
                },
                r: 3.0,
            },
            2 => c2Circle {
                p: g.vec_wide(),
                r: g.coord_wide(),
            },
            _ => g.circle(),
        };
        let ctx = format!("row46 #{i} A={a:?} B={b:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2CircletoCircle)(a, b) });
    }
}

#[test]
fn row47_circle_to_aabb() {
    let p = pair();
    let mut g = Rng::new(0xD2);
    for i in 0..N {
        let bb = g.aabb();
        let c = match i % 8 {
            0 => c2Circle {
                p: bb.min,
                r: g.radius(),
            },
            1 => c2Circle {
                p: bb.max,
                r: g.radius(),
            },
            2 => c2Circle {
                p: c2v { x: bb.min.x, y: bb.max.y },
                r: g.radius(),
            },
            3 => c2Circle {
                p: c2v {
                    x: (bb.min.x + bb.max.x) * 0.5,
                    y: (bb.min.y + bb.max.y) * 0.5,
                },
                r: g.radius(),
            },
            4 => c2Circle {
                p: c2v { x: bb.min.x, y: (bb.min.y + bb.max.y) * 0.5 },
                r: g.radius(),
            },
            5 => c2Circle {
                p: g.vec_wide(),
                r: g.coord_wide(),
            },
            _ => g.circle(),
        };
        let ctx = format!("row47 #{i} A={c:?} B={bb:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2CircletoAABB)(c, bb) });
    }
}

#[test]
fn row48_circle_to_capsule() {
    let p = pair();
    let mut g = Rng::new(0xD3);
    let mut br = [0usize; 3];
    for i in 0..N {
        let k = g.capsule();
        let c = match i % 6 {
            0 => c2Circle {
                p: k.a,
                r: g.radius(),
            },
            1 => c2Circle {
                p: k.b,
                r: g.radius(),
            },
            2 => c2Circle {
                p: c2v {
                    x: (k.a.x + k.b.x) * 0.5,
                    y: (k.a.y + k.b.y) * 0.5,
                },
                r: g.radius(),
            },
            3 => c2Circle {
                p: g.vec_wide(),
                r: g.coord_wide(),
            },
            _ => g.circle(),
        };
        // Classify which of the three C branches this hits, for coverage.
        {
            let n = sub(k.b, k.a);
            let ap = sub(c.p, k.a);
            let da = dot(ap, n);
            if da < 0.0 {
                br[0] += 1;
            } else if dot(sub(c.p, k.b), n) < 0.0 {
                br[1] += 1;
            } else {
                br[2] += 1;
            }
        }
        let ctx = format!("row48 #{i} A={c:?} B={k:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2CircletoCapsule)(c, k) });
    }
    assert!(
        br[0] > 0 && br[1] > 0 && br[2] > 0,
        "row48: branch coverage {br:?}"
    );
}

#[test]
fn row49_aabb_to_capsule() {
    let p = pair();
    let mut g = Rng::new(0xD4);
    for i in 0..N {
        let bb = g.aabb();
        let k = match i % 5 {
            0 => c2Capsule {
                a: bb.min,
                b: bb.max,
                r: g.radius(),
            },
            1 => {
                let v = g.vec();
                c2Capsule {
                    a: v,
                    b: v,
                    r: g.radius(),
                }
            }
            2 => c2Capsule {
                a: c2v { x: bb.min.x - 5.0, y: (bb.min.y + bb.max.y) * 0.5 },
                b: c2v { x: bb.max.x + 5.0, y: (bb.min.y + bb.max.y) * 0.5 },
                r: g.radius(),
            },
            _ => g.capsule(),
        };
        let ctx = format!("row49 #{i} A={bb:?} B={k:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2AABBtoCapsule)(bb, k) });
    }
}

#[test]
fn row50_capsule_to_capsule() {
    let p = pair();
    let mut g = Rng::new(0xD5);
    for i in 0..N {
        let a = g.capsule();
        let b = match i % 6 {
            0 => a,
            1 => c2Capsule {
                a: c2v { x: a.a.x, y: a.a.y + 3.0 },
                b: c2v { x: a.b.x, y: a.b.y + 3.0 },
                r: g.radius(),
            }, // parallel
            2 => c2Capsule {
                a: c2v { x: a.a.x, y: a.b.y },
                b: c2v { x: a.b.x, y: a.a.y },
                r: g.radius(),
            }, // crossing
            3 => {
                let d = sub(a.b, a.a);
                c2Capsule {
                    a: a.b,
                    b: c2v { x: a.b.x + d.x, y: a.b.y + d.y },
                    r: g.radius(),
                }
            } // collinear
            4 => {
                let v = g.vec();
                c2Capsule { a: v, b: v, r: 0.0 }
            }
            _ => g.capsule(),
        };
        let ctx = format!("row50 #{i} A={a:?} B={b:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2CapsuletoCapsule)(a, b) });
    }
}

// ---------------------------------------------------------------------------
// Row 51 — c2Collided over all 9 valid type pairs (incl. the arg swaps)
// ---------------------------------------------------------------------------

#[test]
fn row51_collided_all_type_pairs() {
    let p = pair();
    for (i, ta) in TYPES.iter().enumerate() {
        for (j, tb) in TYPES.iter().enumerate() {
            let (ta, tb) = (*ta, *tb);
            let mut g = Rng::new(0xE0 + (i * 3 + j) as u64);
            for k in 0..N {
                let (sa, sb) = placed_pair(&mut g, ta, tb);
                let ctx = format!("row51 {ta}x{tb} #{k} A={sa:?} B={sb:?}");
                diff(&ctx, &p, |x| unsafe {
                    with_shape(&sa, |pa| {
                        with_shape(&sb, |pb| (x.c2Collided)(pa, ta, pb, tb))
                    })
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 52-53 — the public `capsule()` entry point from include/lib.h
// ---------------------------------------------------------------------------

#[test]
fn row52_capsule_entry_point() {
    let p = pair();
    let mut g = Rng::new(0xF0);
    let mut seen = [0usize; 8];
    for i in 0..N * 4 {
        // The three hard-coded shapes live in x ∈ [-90,-5], y ∈ [-60,110].
        let (min_x, min_y, max_x, max_y, r) = match i % 4 {
            0 => (
                g.range(-100.0, 10.0),
                g.range(-70.0, 120.0),
                g.range(-100.0, 10.0),
                g.range(-70.0, 120.0),
                g.range(0.0, 40.0),
            ),
            1 => (
                g.range(-60.0, -20.0),
                g.range(-50.0, 60.0),
                g.range(-60.0, -20.0),
                g.range(-50.0, 60.0),
                g.range(0.0, 25.0),
            ),
            2 => (
                g.below(41) as f32 - 90.0,
                g.below(61) as f32 - 60.0,
                g.below(41) as f32 - 90.0,
                g.below(61) as f32 - 60.0,
                g.below(30) as f32,
            ),
            _ => (g.coord(), g.coord(), g.coord(), g.coord(), g.radius()),
        };
        let ctx = format!("row52 #{i} ({min_x},{min_y},{max_x},{max_y},{r})");
        diff(&ctx, &p, |x| unsafe { (x.capsule)(min_x, min_y, max_x, max_y, r) });
        let got = unsafe { (p.c.capsule)(min_x, min_y, max_x, max_y, r) };
        if (0..8).contains(&got) {
            seen[got as usize] += 1;
        }
    }
    let covered = seen.iter().filter(|c| **c > 0).count();
    assert!(
        covered >= 6,
        "row52: only {covered}/8 bitmask outcomes reached: {seen:?}"
    );
}

#[test]
fn row53_capsule_special_values() {
    let p = pair();
    let mut g = Rng::new(0xF1);
    let specials = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        1.192_092_895_507_812_5e-7,
        -70.0,
        -40.0,
        -20.0,
        100.0,
    ];
    // Full cross product on the first three args, randomized for the rest.
    for (i, a) in specials.iter().enumerate() {
        for (j, b) in specials.iter().enumerate() {
            for (k, c) in specials.iter().enumerate() {
                let d = specials[(i + j + k) % specials.len()];
                let e = specials[(i * 3 + j * 5 + k * 7) % specials.len()];
                let (a, b, c) = (*a, *b, *c);
                let ctx = format!("row53 ({a},{b},{c},{d},{e})");
                diff(&ctx, &p, |x| unsafe { (x.capsule)(a, b, c, d, e) });
            }
        }
    }
    for i in 0..N {
        let pick = |g: &mut Rng| specials[g.below(specials.len() as u32) as usize];
        let (a, b, c, d, e) = (
            pick(&mut g),
            pick(&mut g),
            pick(&mut g),
            g.coord_wide(),
            g.coord_wide(),
        );
        let ctx = format!("row53 rand #{i} ({a},{b},{c},{d},{e})");
        diff(&ctx, &p, |x| unsafe { (x.capsule)(a, b, c, d, e) });
    }
}

// ---------------------------------------------------------------------------
// Row 61 — the cache interacting with non-identity transforms
//
// Rows 39-42 exercise the cache with NULL transforms only, but the cached
// vertex INDICES are resolved through `c2Mulxv(ax, pA.verts[iA])` and the
// support search goes through `c2MulrvT(ax.r, ...)`, so a rotated/translated
// frame changes which index the cache should have stored. This drives the two
// axes together, and re-feeds the cache across a transform change.
// ---------------------------------------------------------------------------

#[test]
fn row61_gjk_cache_with_transforms() {
    let p = pair();
    let mut g = Rng::new(0xB5);
    for i in 0..N {
        let ta = TYPES[g.below(3) as usize];
        let tb = TYPES[g.below(3) as usize];
        let (sa, sb) = placed_pair(&mut g, ta, tb);
        let t1 = g.range(-3.141_592_7, 3.141_592_7);
        let t2 = g.range(-3.141_592_7, 3.141_592_7);
        let t3 = g.range(-3.141_592_7, 3.141_592_7);
        let ax1 = c2x {
            p: g.vec(),
            r: c2r {
                c: t1.cos(),
                s: t1.sin(),
            },
        };
        let bx1 = c2x {
            p: g.vec(),
            r: c2r {
                c: t2.cos(),
                s: t2.sin(),
            },
        };
        // A *different* frame for the replay, so the cached indices are stale
        // with respect to the new support directions.
        let ax2 = c2x {
            p: g.vec(),
            r: c2r {
                c: t3.cos(),
                s: t3.sin(),
            },
        };
        let ur = if g.bool() { 1 } else { 0 };
        let ctx = format!("row61 #{i} A={sa:?} B={sb:?} ax1={ax1:?} bx1={bx1:?} ax2={ax2:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let o = GjkOpts {
                use_radius: ur,
                ax: Some(ax1),
                bx: Some(bx1),
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let warm = call_gjk(x, &sa, &sb, &o);
            let same_frame = call_gjk(
                x,
                &sa,
                &sb,
                &GjkOpts {
                    cache: warm.cache,
                    ..o
                },
            );
            let moved_frame = call_gjk(
                x,
                &sa,
                &sb,
                &GjkOpts {
                    cache: same_frame.cache,
                    ax: Some(ax2),
                    ..o
                },
            );
            // And once more with the transforms dropped to NULL mid-sequence.
            let null_frame = call_gjk(
                x,
                &sa,
                &sb,
                &GjkOpts {
                    cache: moved_frame.cache,
                    ax: None,
                    bx: None,
                    ..o
                },
            );
            vec![warm, same_frame, moved_frame, null_frame]
        });
    }
}
