#![allow(non_snake_case)]
//! Phase B — valid-path differential tests for `c2GJK`, the boolean
//! predicates, the dispatcher and the one-shot wrapper.
//! CONFIGS.md rows 34..68.

mod common;
use common::*;

const N: usize = 600;

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

// -------------------------------------------------------------- rows 34..43
/// Full 3x3 type matrix x use_radius in {0,1}, NULL transforms and NULL cache.
#[test]
fn cfg_gjk_matrix() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x2001);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..N {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                    same(&format!("c2GJK ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

// -------------------------------------------------------------- rows 44..48
fn transform_case(seed: u64, tag: &str, mk: fn(&mut Rng) -> c2x) {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(seed);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..N {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let ax = mk(&mut g);
                    let bx = mk(&mut g);
                    let co = call_gjk(cf, &a, ta, Some(&ax), &b, tb, Some(&bx), ur, None);
                    let ro = call_gjk(rf, &a, ta, Some(&ax), &b, tb, Some(&bx), ur, None);
                    same(&format!("c2GJK {tag} ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

#[test]
fn cfg_gjk_transform_identity() {
    transform_case(0x2002, "identity", |_| c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    });
}

/// A non-NULL identity transform must give exactly the NULL-pointer result.
#[test]
fn cfg_gjk_transform_identity_equals_null() {
    let (cf, rf) = gjk_pair();
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut g = Rng::new(0x2003);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..N {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                let with = call_gjk(cf, &a, ta, Some(&ident), &b, tb, Some(&ident), 1, None);
                let without = call_gjk(cf, &a, ta, None, &b, tb, None, 1, None);
                same("C: identity xform == NULL xform", with, without);
                let rwith = call_gjk(rf, &a, ta, Some(&ident), &b, tb, Some(&ident), 1, None);
                same("Rust matches C for identity xform", with, rwith);
            }
        }
    }
}

#[test]
fn cfg_gjk_transform_translate() {
    transform_case(0x2004, "translate", |g| c2x {
        p: g.vec(),
        r: c2r { c: 1.0, s: 0.0 },
    });
}

#[test]
fn cfg_gjk_transform_rotate() {
    transform_case(0x2005, "rotate", |g| {
        let t = g.range(-6.3, 6.3);
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r {
                c: t.cos(),
                s: t.sin(),
            },
        }
    });
}

#[test]
fn cfg_gjk_transform_full() {
    transform_case(0x2006, "rot+trans", |g| {
        let t = g.range(-6.3, 6.3);
        c2x {
            p: g.vec(),
            r: c2r {
                c: t.cos(),
                s: t.sin(),
            },
        }
    });
}

/// `c2r` is never normalised by the library, so a scaling rotor is a valid
/// input that exercises different support directions.
#[test]
fn cfg_gjk_transform_unnormalised() {
    transform_case(0x2007, "unnormalised", |g| c2x {
        p: g.vec(),
        r: c2r {
            c: g.range(-3.0, 3.0),
            s: g.range(-3.0, 3.0),
        },
    });
}

// -------------------------------------------------------------- rows 49..52
#[test]
fn cfg_gjk_cache_cold() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x2008);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..N {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let cold = c2GJKCache::default();
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, Some(cold));
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, Some(cold));
                    same(&format!("c2GJK cold-cache ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

#[test]
fn cfg_gjk_cache_warm_same() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x2009);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..N {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let first_c = call_gjk(
                        cf,
                        &a,
                        ta,
                        None,
                        &b,
                        tb,
                        None,
                        ur,
                        Some(c2GJKCache::default()),
                    );
                    let first_r = call_gjk(
                        rf,
                        &a,
                        ta,
                        None,
                        &b,
                        tb,
                        None,
                        ur,
                        Some(c2GJKCache::default()),
                    );
                    same("warm-cache pass 1", first_c, first_r);
                    // feed the produced cache straight back in
                    let second_c =
                        call_gjk(cf, &a, ta, None, &b, tb, None, ur, first_c.cache);
                    let second_r =
                        call_gjk(rf, &a, ta, None, &b, tb, None, ur, first_r.cache);
                    same(
                        &format!("c2GJK warm-cache pass 2 ta={ta} tb={tb} ur={ur}"),
                        second_c,
                        second_r,
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_gjk_cache_warm_moved() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x200a);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..N {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                let c1 = call_gjk(cf, &a, ta, None, &b, tb, None, 1, Some(c2GJKCache::default()));
                let r1 = call_gjk(rf, &a, ta, None, &b, tb, None, 1, Some(c2GJKCache::default()));
                same("moved-cache pass 1", c1, r1);
                // now MOVE both shapes and reuse the stale cache
                let a2 = rand_shape(&mut g, ta);
                let b2 = rand_shape(&mut g, tb);
                let c2_ = call_gjk(cf, &a2, ta, None, &b2, tb, None, 1, c1.cache);
                let r2_ = call_gjk(rf, &a2, ta, None, &b2, tb, None, 1, r1.cache);
                same(
                    &format!("c2GJK stale-cache ta={ta} tb={tb}"),
                    c2_,
                    r2_,
                );
            }
        }
    }
}

#[test]
fn cfg_gjk_cache_sequence() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x200b);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..N / 4 {
                let mut ccache = Some(c2GJKCache::default());
                let mut rcache = Some(c2GJKCache::default());
                // A random walk: shapes drift a little each step, cache threaded
                let mut ax = c2x {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: c2r { c: 1.0, s: 0.0 },
                };
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                for step in 0..10 {
                    ax.p.x += g.range(-4.0, 4.0);
                    ax.p.y += g.range(-4.0, 4.0);
                    let t = g.range(-0.4, 0.4);
                    ax.r = c2r {
                        c: t.cos(),
                        s: t.sin(),
                    };
                    let co = call_gjk(cf, &a, ta, Some(&ax), &b, tb, None, 1, ccache);
                    let ro = call_gjk(rf, &a, ta, Some(&ax), &b, tb, None, 1, rcache);
                    same(
                        &format!("c2GJK cache-sequence ta={ta} tb={tb} step={step}"),
                        co,
                        ro,
                    );
                    ccache = co.cache;
                    rcache = ro.cache;
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 53
#[test]
fn cfg_gjk_iterations() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x200c);
    let mut max_iters = 0i32;
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..N {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                let mut ci: i32 = -1;
                let mut ri: i32 = -2;
                let cd = unsafe {
                    cf(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        1,
                        &mut ci,
                        std::ptr::null_mut(),
                    )
                };
                let rd = unsafe {
                    rf(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        1,
                        &mut ri,
                        std::ptr::null_mut(),
                    )
                };
                same("c2GJK iterations-only", (cd, ci), (rd, ri));
                max_iters = max_iters.max(ci);
            }
        }
    }
    assert!(max_iters > 0, "iteration counter never advanced");
    assert!(max_iters <= 20, "iteration cap violated: {max_iters}");
}

// --------------------------------------------------------------------- row 54
#[test]
fn cfg_gjk_mixed_outputs() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x200d);
    let sentinel = v(f32::from_bits(0xCAFE_BABE), f32::from_bits(0xCAFE_BABE));
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for mask in 0u32..8 {
                for _ in 0..N / 4 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let mut cA = sentinel;
                    let mut cB = sentinel;
                    let mut cI = -7i32;
                    let mut rA = sentinel;
                    let mut rB = sentinel;
                    let mut rI = -7i32;
                    let pa = |on: bool, p: &mut c2v| {
                        if on {
                            p as *mut c2v
                        } else {
                            std::ptr::null_mut()
                        }
                    };
                    let pi = |on: bool, p: &mut i32| {
                        if on {
                            p as *mut i32
                        } else {
                            std::ptr::null_mut()
                        }
                    };
                    let (oa, ob, oi) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
                    let cd = unsafe {
                        cf(
                            a.ptr(),
                            ta,
                            std::ptr::null(),
                            b.ptr(),
                            tb,
                            std::ptr::null(),
                            pa(oa, &mut cA),
                            pa(ob, &mut cB),
                            1,
                            pi(oi, &mut cI),
                            std::ptr::null_mut(),
                        )
                    };
                    let rd = unsafe {
                        rf(
                            a.ptr(),
                            ta,
                            std::ptr::null(),
                            b.ptr(),
                            tb,
                            std::ptr::null(),
                            pa(oa, &mut rA),
                            pa(ob, &mut rB),
                            1,
                            pi(oi, &mut rI),
                            std::ptr::null_mut(),
                        )
                    };
                    same(
                        &format!("c2GJK out-mask={mask} ta={ta} tb={tb}"),
                        (cd, cA, cB, cI),
                        (rd, rA, rB, rI),
                    );
                    // untouched outputs must keep the sentinel on BOTH sides
                    if !oa {
                        assert_eq!(cA.x.to_bits(), sentinel.x.to_bits());
                        assert_eq!(rA.x.to_bits(), sentinel.x.to_bits());
                    }
                    if !oi {
                        assert_eq!((cI, rI), (-7, -7));
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 55
#[test]
fn cfg_gjk_deep_overlap() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x200e);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..N {
                    // both shapes centred at the same random point => deep overlap
                    let ctr = g.vec();
                    let (sa, sb) = (g.range(10.0, 40.0), g.range(10.0, 40.0));
                    let a = concentric(&mut g, ta, ctr, sa);
                    let b = concentric(&mut g, tb, ctr, sb);
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                    same(&format!("c2GJK deep-overlap ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

fn concentric(g: &mut Rng, ty: C2_TYPE, ctr: c2v, size: f32) -> Blob {
    match ty {
        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: ctr, r: size }),
        C2_TYPE_AABB => Blob::of_aabb(c2AABB {
            min: v(ctr.x - size, ctr.y - size),
            max: v(ctr.x + size, ctr.y + size),
        }),
        _ => {
            let d = g.range(0.0, size);
            Blob::of_capsule(c2Capsule {
                a: v(ctr.x - d, ctr.y),
                b: v(ctr.x + d, ctr.y),
                r: size,
            })
        }
    }
}

// --------------------------------------------------------------------- row 56
#[test]
fn cfg_gjk_touching() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x200f);
    for ur in [0i32, 1] {
        for _ in 0..N * 4 {
            // Two circles at exactly the sum of their radii, and at
            // one ULP either side of it.
            let ra = g.range(1.0, 30.0);
            let rb = g.range(1.0, 30.0);
            let base = ra + rb;
            for dx in [
                base,
                f32::from_bits(base.to_bits() - 1),
                f32::from_bits(base.to_bits() + 1),
                base + FLT_EPSILON,
                base - FLT_EPSILON,
            ] {
                let a = Blob::of_circle(c2Circle {
                    p: v(0.0, 0.0),
                    r: ra,
                });
                let b = Blob::of_circle(c2Circle {
                    p: v(dx, 0.0),
                    r: rb,
                });
                let co = call_gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, ur, None);
                let ro = call_gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, ur, None);
                same("c2GJK touching-circles", co, ro);
            }
            // AABBs sharing an edge exactly
            let x = g.coord();
            let a = Blob::of_aabb(c2AABB {
                min: v(x - 10.0, -5.0),
                max: v(x, 5.0),
            });
            let b = Blob::of_aabb(c2AABB {
                min: v(x, -5.0),
                max: v(x + 10.0, 5.0),
            });
            let co = call_gjk(cf, &a, C2_TYPE_AABB, None, &b, C2_TYPE_AABB, None, ur, None);
            let ro = call_gjk(rf, &a, C2_TYPE_AABB, None, &b, C2_TYPE_AABB, None, ur, None);
            same("c2GJK edge-touching-aabbs", co, ro);
        }
    }
}

// --------------------------------------------------------------------- row 57
#[test]
fn cfg_gjk_coincident() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x2010);
    for &ty in &ALL_TYPES {
        for ur in [0i32, 1] {
            for _ in 0..N * 2 {
                let a = rand_shape(&mut g, ty);
                let co = call_gjk(cf, &a, ty, None, &a, ty, None, ur, None);
                let ro = call_gjk(rf, &a, ty, None, &a, ty, None, ur, None);
                same(&format!("c2GJK coincident ty={ty} ur={ur}"), co, ro);
            }
        }
    }
}

// --------------------------------------------------------------------- row 58
#[test]
fn cfg_gjk_degenerate_shapes() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x2011);
    let degen = |g: &mut Rng, ty: C2_TYPE| -> Blob {
        match ty {
            C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: g.vec(), r: 0.0 }),
            C2_TYPE_AABB => {
                let p = g.vec();
                Blob::of_aabb(c2AABB { min: p, max: p })
            }
            _ => {
                let p = g.vec();
                Blob::of_capsule(c2Capsule { a: p, b: p, r: 0.0 })
            }
        }
    };
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for k in 0..N * 2 {
                    // mix degenerate and normal shapes
                    let a = if k % 3 == 0 {
                        rand_shape(&mut g, ta)
                    } else {
                        degen(&mut g, ta)
                    };
                    let b = if k % 3 == 1 {
                        rand_shape(&mut g, tb)
                    } else {
                        degen(&mut g, tb)
                    };
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                    same(&format!("c2GJK degenerate ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 59
#[test]
fn cfg_gjk_extreme_scales() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x2012);
    let scales = [1e-20f32, 1e-8, 1.0, 1e8, 1e18];
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for s in scales {
                for ur in [0i32, 1] {
                    for _ in 0..N / 2 {
                        let a = scaled_shape(&mut g, ta, s);
                        let b = scaled_shape(&mut g, tb, s);
                        let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                        let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                        same(
                            &format!("c2GJK scale={s:e} ta={ta} tb={tb} ur={ur}"),
                            co,
                            ro,
                        );
                    }
                }
            }
        }
    }
}

fn scaled_shape(g: &mut Rng, ty: C2_TYPE, s: f32) -> Blob {
    match ty {
        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle {
            p: v(g.range(-2.0, 2.0) * s, g.range(-2.0, 2.0) * s),
            r: g.range(0.0, 1.0) * s,
        }),
        C2_TYPE_AABB => {
            let p = v(g.range(-2.0, 2.0) * s, g.range(-2.0, 2.0) * s);
            Blob::of_aabb(c2AABB {
                min: p,
                max: v(p.x + g.range(0.0, 1.0) * s, p.y + g.range(0.0, 1.0) * s),
            })
        }
        _ => Blob::of_capsule(c2Capsule {
            a: v(g.range(-2.0, 2.0) * s, g.range(-2.0, 2.0) * s),
            b: v(g.range(-2.0, 2.0) * s, g.range(-2.0, 2.0) * s),
            r: g.range(0.0, 1.0) * s,
        }),
    }
}

// --------------------------------------------------------------------- row 60
#[test]
fn cfg_aabb_to_aabb() {
    let (cf, rf) = pair::<FnI_AABB_AABB>("c2AABBtoAABB");
    let mut g = Rng::new(0x2013);
    let mut hits = 0usize;
    for i in 0..N * 20 {
        let a = g.aabb();
        let b = match i % 5 {
            0 => a,                       // identical
            1 => {
                // exactly edge-touching
                c2AABB {
                    min: v(a.max.x, a.min.y),
                    max: v(a.max.x + 10.0, a.max.y),
                }
            }
            2 => {
                // nested
                c2AABB {
                    min: v(a.min.x + 1.0, a.min.y + 1.0),
                    max: v(a.max.x - 1.0, a.max.y - 1.0),
                }
            }
            3 => c2AABB {
                min: v(a.min.x + 400.0, a.min.y),
                max: v(a.max.x + 400.0, a.max.y),
            }, // disjoint
            _ => g.aabb(),
        };
        let cv = cf(a, b);
        hits += (cv != 0) as usize;
        same("c2AABBtoAABB", cv, rf(a, b));
        same("c2AABBtoAABB swapped", cf(b, a), rf(b, a));
    }
    assert!(hits > 0 && hits < N * 20, "AABBtoAABB coverage: {hits}");
}

// --------------------------------------------------------------------- row 61
#[test]
fn cfg_aabb_to_capsule() {
    let (cf, rf) = pair::<FnI_AABB_Cap>("c2AABBtoCapsule");
    let mut g = Rng::new(0x2014);
    let mut hits = 0usize;
    let total = N * 10;
    for i in 0..total {
        let a = g.aabb();
        let b = match i % 5 {
            0 => c2Capsule {
                // crossing a corner
                a: v(a.min.x - 5.0, a.min.y - 5.0),
                b: v(a.max.x + 5.0, a.max.y + 5.0),
                r: g.range(0.0, 5.0),
            },
            1 => c2Capsule {
                // zero-length
                a: g.vec(),
                b: g.vec(),
                r: 0.0,
            },
            2 => {
                let p = g.vec();
                c2Capsule {
                    a: p,
                    b: p,
                    r: g.range(0.0, 20.0),
                }
            }
            3 => c2Capsule {
                // far away
                a: v(a.min.x + 500.0, a.min.y),
                b: v(a.max.x + 500.0, a.max.y),
                r: g.range(0.0, 5.0),
            },
            _ => g.capsule(),
        };
        let cv = cf(a, b);
        hits += (cv != 0) as usize;
        same("c2AABBtoCapsule", cv, rf(a, b));
    }
    assert!(hits > 0 && hits < total, "AABBtoCapsule coverage: {hits}");
}

// --------------------------------------------------------------------- row 62
#[test]
fn cfg_capsule_to_capsule() {
    let (cf, rf) = pair::<FnI_Cap_Cap>("c2CapsuletoCapsule");
    let mut g = Rng::new(0x2015);
    let mut hits = 0usize;
    let total = N * 10;
    for i in 0..total {
        let a = g.capsule();
        let b = match i % 6 {
            0 => a,
            1 => c2Capsule {
                // crossing (perpendicular)
                a: v(a.a.x, a.b.y),
                b: v(a.b.x, a.a.y),
                r: g.range(0.0, 10.0),
            },
            2 => c2Capsule {
                // parallel, offset
                a: v(a.a.x + 3.0, a.a.y + 3.0),
                b: v(a.b.x + 3.0, a.b.y + 3.0),
                r: a.r,
            },
            3 => c2Capsule {
                // collinear extension
                a: a.b,
                b: v(a.b.x * 2.0 - a.a.x, a.b.y * 2.0 - a.a.y),
                r: a.r,
            },
            4 => c2Capsule {
                a: a.a,
                b: a.a,
                r: 0.0,
            },
            _ => g.capsule(),
        };
        let cv = cf(a, b);
        hits += (cv != 0) as usize;
        same("c2CapsuletoCapsule", cv, rf(a, b));
        same("c2CapsuletoCapsule swapped", cf(b, a), rf(b, a));
    }
    assert!(hits > 0 && hits < total, "CapsuletoCapsule coverage: {hits}");
}

// --------------------------------------------------------------------- row 63
#[test]
fn cfg_circle_to_circle() {
    let (cf, rf) = pair::<FnI_Cir_Cir>("c2CircletoCircle");
    let mut g = Rng::new(0x2016);
    let mut hits = 0usize;
    let total = N * 20;
    for i in 0..total {
        let a = g.circle();
        let b = match i % 4 {
            0 => c2Circle {
                // exactly tangent
                p: v(a.p.x + a.r + 5.0, a.p.y),
                r: 5.0,
            },
            1 => c2Circle { p: a.p, r: 0.0 },
            2 => a,
            _ => g.circle(),
        };
        let cv = cf(a, b);
        hits += (cv != 0) as usize;
        same("c2CircletoCircle", cv, rf(a, b));
        same("c2CircletoCircle swapped", cf(b, a), rf(b, a));
    }
    assert!(hits > 0 && hits < total, "CircletoCircle coverage: {hits}");
}

// --------------------------------------------------------------------- row 64
#[test]
fn cfg_circle_to_aabb() {
    let (cf, rf) = pair::<FnI_Cir_AABB>("c2CircletoAABB");
    let mut g = Rng::new(0x2017);
    let mut hits = 0usize;
    let total = N * 10;
    for i in 0..total {
        let bb = g.aabb();
        let mid = v((bb.min.x + bb.max.x) * 0.5, (bb.min.y + bb.max.y) * 0.5);
        let corners = [
            bb.min,
            v(bb.max.x, bb.min.y),
            bb.max,
            v(bb.min.x, bb.max.y),
            v(mid.x, bb.min.y),
            v(bb.max.x, mid.y),
            v(mid.x, bb.max.y),
            v(bb.min.x, mid.y),
            mid,
        ];
        let a = match i % 4 {
            0 => c2Circle {
                p: corners[i % 9],
                r: g.range(0.0, 10.0),
            },
            1 => c2Circle {
                p: mid,
                r: g.range(0.0, 50.0),
            },
            2 => c2Circle {
                p: v(bb.min.x - 100.0, bb.min.y),
                r: g.range(0.0, 10.0),
            },
            _ => g.circle(),
        };
        let cv = cf(a, bb);
        hits += (cv != 0) as usize;
        same("c2CircletoAABB", cv, rf(a, bb));
    }
    assert!(hits > 0 && hits < total, "CircletoAABB coverage: {hits}");
}

// --------------------------------------------------------------------- row 65
#[test]
fn cfg_circle_to_capsule() {
    let (cf, rf) = pair::<FnI_Cir_Cap>("c2CircletoCapsule");
    let mut g = Rng::new(0x2018);
    let mut branches = [0usize; 3];
    let mut hits = 0usize;
    let total = N * 10;
    for i in 0..total {
        let cap = match i % 4 {
            0 => {
                let p = g.vec();
                c2Capsule {
                    a: p,
                    b: p,
                    r: g.range(0.0, 20.0),
                } // zero length
            }
            1 => c2Capsule {
                a: v(-50.0, 0.0),
                b: v(50.0, 0.0),
                r: g.range(0.0, 20.0),
            },
            _ => g.capsule(),
        };
        // sample the circle centre along/around the capsule axis
        let t = g.range(-1.5, 2.5);
        let n = v(cap.b.x - cap.a.x, cap.b.y - cap.a.y);
        let base = v(cap.a.x + n.x * t, cap.a.y + n.y * t);
        let circ = c2Circle {
            p: v(
                base.x + g.range(-20.0, 20.0),
                base.y + g.range(-20.0, 20.0),
            ),
            r: g.range(0.0, 20.0),
        };
        // branch bookkeeping (mirrors lib.c:558-570)
        let ap = v(circ.p.x - cap.a.x, circ.p.y - cap.a.y);
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            branches[0] += 1;
        } else {
            let db = (circ.p.x - cap.b.x) * n.x + (circ.p.y - cap.b.y) * n.y;
            if db < 0.0 {
                branches[1] += 1;
            } else {
                branches[2] += 1;
            }
        }
        let cv = cf(circ, cap);
        hits += (cv != 0) as usize;
        same("c2CircletoCapsule", cv, rf(circ, cap));
    }
    assert!(
        branches.iter().all(|&b| b > 0),
        "c2CircletoCapsule branch coverage: {branches:?}"
    );
    assert!(hits > 0 && hits < total, "CircletoCapsule coverage: {hits}");
}

// --------------------------------------------------------------------- row 66
#[test]
fn cfg_collided_matrix() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut g = Rng::new(0x2019);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let mut hits = 0usize;
            let total = N * 4;
            for k in 0..total {
                // half the samples are deliberately overlapping
                let (a, b) = if k % 2 == 0 {
                    let ctr = g.vec();
                    (
                        {
                            let s1 = g.range(5.0, 30.0);
                            concentric(&mut g, ta, ctr, s1)
                        },
                        {
                            let s2 = g.range(5.0, 30.0);
                            concentric(&mut g, tb, ctr, s2)
                        },
                    )
                } else {
                    (rand_shape(&mut g, ta), rand_shape(&mut g, tb))
                };
                let cv = unsafe { cf(a.ptr(), ta, b.ptr(), tb) };
                let rv = unsafe { rf(a.ptr(), ta, b.ptr(), tb) };
                same(&format!("c2Collided ta={ta} tb={tb}"), cv, rv);
                hits += (cv != 0) as usize;
            }
            assert!(hits > 0, "c2Collided ta={ta} tb={tb} never reported a hit");
            assert!(hits < total, "c2Collided ta={ta} tb={tb} always hit");
        }
    }
}

// --------------------------------------------------------------------- row 67
#[test]
fn cfg_reverse_collide_random() {
    let (cf, rf) = pair::<FnReverseCollide>("reverse_collide");
    let mut g = Rng::new(0x201a);
    let mut seen = [0usize; 8];
    for _ in 0..200_000 {
        let x = g.range(-170.0, 170.0);
        let y = g.range(-60.0, 170.0);
        let r = g.range(0.0, 70.0);
        let cv = cf(x, y, r);
        same(&format!("reverse_collide({x},{y},{r})"), cv, rf(x, y, r));
        if (0..8).contains(&cv) {
            seen[cv as usize] += 1;
        }
    }
    eprintln!("reverse_collide bitmask histogram: {seen:?}");
    let covered = seen.iter().filter(|&&c| c > 0).count();
    assert!(
        covered >= 7,
        "expected nearly all 8 bitmask values, got {covered}: {seen:?}"
    );
}

// --------------------------------------------------------------------- row 68
#[test]
fn cfg_reverse_collide_grid() {
    let (cf, rf) = pair::<FnReverseCollide>("reverse_collide");
    // Boundary-relevant coordinates for the three hard-coded shapes:
    // circle (-70,0) r20 ; aabb [-40,-40]..[-15,-15] ; capsule (-40,40)-(-20,100) r10
    let interesting: [f32; 21] = [
        -170.0, -110.0, -90.0, -70.0, -50.0, -41.0, -40.0, -39.0, -30.0, -20.0, -15.0, -14.0, 0.0,
        1.0, 15.0, 20.0, 40.0, 60.0, 100.0, 101.0, 170.0,
    ];
    let radii: [f32; 12] = [
        0.0, f32::from_bits(1), FLT_EPSILON, 0.5, 1.0, 5.0, 9.999999, 10.0, 10.000001, 20.0, 50.0,
        200.0,
    ];
    for &x in &interesting {
        for &y in &interesting {
            for &r in &radii {
                same(
                    &format!("reverse_collide grid ({x},{y},{r})"),
                    cf(x, y, r),
                    rf(x, y, r),
                );
            }
        }
    }
    // A dense sweep along the boundary of each shape.
    let mut g = Rng::new(0x201b);
    for _ in 0..40_000 {
        // ring around the fixed circle at exactly radius 20+r
        let t = g.range(0.0, 6.283_185);
        let r = g.range(0.0, 30.0);
        let d = 20.0 + r;
        let x = -70.0 + d * t.cos();
        let y = d * t.sin();
        same("reverse_collide ring", cf(x, y, r), rf(x, y, r));
        // exactly on the AABB faces / corners
        let fx = [-40.0f32, -27.5, -15.0][g.below(3) as usize];
        let fy = [-40.0f32, -27.5, -15.0][g.below(3) as usize];
        same("reverse_collide aabb face", cf(fx, fy, r), rf(fx, fy, r));
        // along the capsule axis
        let s = g.range(-0.5, 1.5);
        let cx = -40.0 + 20.0 * s;
        let cy = 40.0 + 60.0 * s;
        same("reverse_collide capsule axis", cf(cx, cy, r), rf(cx, cy, r));
    }
}
