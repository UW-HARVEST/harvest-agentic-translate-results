//! Phase B, level 3 — `c2GJK`, the lowest-level composed entry point.
//! CONFIGS.md rows 29..=46.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 400;

#[derive(Clone, Copy, Debug)]
struct GjkOpts {
    ax: Option<C2x>,
    bx: Option<C2x>,
    use_radius: c_int,
    want_outs: bool,
}

/// Full observable result of one `c2GJK` call.
#[derive(Clone, Copy, Debug)]
struct GjkOut {
    dist: f32,
    a: C2v,
    b: C2v,
    iters: c_int,
    cache: C2GJKCache,
    cache_used: bool,
}

impl BitEq for GjkOut {
    fn bits(&self) -> String {
        format!(
            "dist={} a={} b={} iters={} cache={}",
            self.dist.bits(),
            self.a.bits(),
            self.b.bits(),
            self.iters,
            if self.cache_used { self.cache.bits() } else { "-".into() }
        )
    }
    fn beq(&self, o: &Self) -> bool {
        self.dist.beq(&o.dist)
            && self.a.beq(&o.a)
            && self.b.beq(&o.b)
            && self.iters == o.iters
            && (!self.cache_used || self.cache.beq(&o.cache))
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn call(
    f: &FnGJK,
    A: &[u8; 20],
    tA: c_int,
    B: &[u8; 20],
    tB: c_int,
    o: GjkOpts,
    cache: Option<C2GJKCache>,
) -> GjkOut {
    let mut a = C2v { x: -12345.0, y: -54321.0 };
    let mut b = C2v { x: -11111.0, y: -22222.0 };
    let mut iters: c_int = -12345;
    let mut cch = cache.unwrap_or(C2GJKCache {
        metric: 0.0,
        count: 0,
        iA: [0; 3],
        iB: [0; 3],
        div: 0.0,
    });
    let axp = o.ax.as_ref().map_or(std::ptr::null(), |x| x as *const C2x);
    let bxp = o.bx.as_ref().map_or(std::ptr::null(), |x| x as *const C2x);
    let dist = f(
        A.as_ptr() as *const c_void,
        tA,
        axp,
        B.as_ptr() as *const c_void,
        tB,
        bxp,
        if o.want_outs { &mut a } else { std::ptr::null_mut() },
        if o.want_outs { &mut b } else { std::ptr::null_mut() },
        o.use_radius,
        if o.want_outs { &mut iters } else { std::ptr::null_mut() },
        if cache.is_some() { &mut cch } else { std::ptr::null_mut() },
    );
    GjkOut {
        dist,
        a,
        b,
        iters,
        cache: cch,
        cache_used: cache.is_some(),
    }
}

/// Drive every `typeA x typeB` pair with the given options and shape scale.
fn gjk_all_pairs(row: &str, seed: u64, scale: f32, opts_of: impl Fn(&mut Rng) -> GjkOpts) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..N {
                let A = shape_bytes(&mut rng, tA, scale);
                let B = shape_bytes(&mut rng, tB, scale);
                let o = opts_of(&mut rng);
                unsafe {
                    d.check(
                        (row, ty_name(tA), ty_name(tB), o, &A[..], &B[..]),
                        call(&c, &A, tA, &B, tB, o, None),
                        call(&r, &A, tA, &B, tB, o, None),
                    );
                }
            }
        }
    }
    d.finish();
}

const PLAIN: GjkOpts = GjkOpts { ax: None, bx: None, use_radius: 1, want_outs: true };

#[test]
fn row29_all_pairs_default() {
    gjk_all_pairs("29: all pairs, identity xforms, use_radius=1", 0x3001, 10.0, |_| PLAIN);
}

#[test]
fn row30_all_pairs_use_radius_zero() {
    gjk_all_pairs("30: all pairs, use_radius=0", 0x3002, 10.0, |_| GjkOpts {
        use_radius: 0,
        ..PLAIN
    });
}

#[test]
fn row31_ax_transform_only() {
    gjk_all_pairs("31: all pairs, ax=xform bx=NULL", 0x3003, 10.0, |rng| GjkOpts {
        ax: Some(x_mixed(rng, 10.0)),
        bx: None,
        use_radius: 1,
        want_outs: true,
    });
}

#[test]
fn row32_bx_transform_only() {
    gjk_all_pairs("32: all pairs, ax=NULL bx=xform", 0x3004, 10.0, |rng| GjkOpts {
        ax: None,
        bx: Some(x_mixed(rng, 10.0)),
        use_radius: 1,
        want_outs: true,
    });
}

#[test]
fn row33_both_transforms_both_radius_modes() {
    gjk_all_pairs("33: all pairs, both xforms, both use_radius", 0x3005, 10.0, |rng| {
        GjkOpts {
            ax: Some(x_mixed(rng, 10.0)),
            bx: Some(x_mixed(rng, 10.0)),
            use_radius: if rng.boolean() { 1 } else { 0 },
            want_outs: true,
        }
    });
}

/// Row 34 — cache supplied but cold (`count == 0`); the written-back cache is
/// compared field by field.
#[test]
fn row34_cache_cold() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x3006);
    let mut d = Diff::new("34: cache cold (count=0)");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..N {
                let A = shape_bytes(&mut rng, tA, 10.0);
                let B = shape_bytes(&mut rng, tB, 10.0);
                let cold = C2GJKCache {
                    metric: mixed(&mut rng, 100.0),
                    count: 0,
                    iA: [7, 7, 7],
                    iB: [7, 7, 7],
                    div: mixed(&mut rng, 100.0),
                };
                unsafe {
                    d.check(
                        (ty_name(tA), ty_name(tB), cold.metric, cold.div),
                        call(&c, &A, tA, &B, tB, PLAIN, Some(cold)),
                        call(&r, &A, tA, &B, tB, PLAIN, Some(cold)),
                    );
                }
            }
        }
    }
    d.finish();
}

/// Row 35 — the REAL consumer pattern: one cache reused across a sequence of
/// perturbed calls, so the warm-start path feeds itself.
#[test]
fn row35_cache_reused_sequence() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x3007);
    let mut d = Diff::new("35: cache reused across a perturbed sequence");
    let mut warm_starts = 0usize;
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..N / 4 {
                // Two independent caches, one per library, evolved in lockstep.
                let mut ccache = C2GJKCache {
                    metric: 0.0,
                    count: 0,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 0.0,
                };
                let mut rcache = ccache;
                let mut A = shape_bytes(&mut rng, tA, 10.0);
                let mut B = shape_bytes(&mut rng, tB, 10.0);
                for step in 0..12 {
                    // perturb the raw shape bytes slightly, float by float
                    for buf in [&mut A, &mut B] {
                        let k = rng.below(5) * 4;
                        let mut f = f32::from_le_bytes([
                            buf[k], buf[k + 1], buf[k + 2], buf[k + 3],
                        ]);
                        f += rng.sym(0.75);
                        buf[k..k + 4].copy_from_slice(&f.to_le_bytes());
                    }
                    if ccache.count != 0 {
                        warm_starts += 1;
                    }
                    let co = unsafe { call(&c, &A, tA, &B, tB, PLAIN, Some(ccache)) };
                    let ro = unsafe { call(&r, &A, tA, &B, tB, PLAIN, Some(rcache)) };
                    d.check((ty_name(tA), ty_name(tB), step, &A[..], &B[..]), co, ro);
                    ccache = co.cache;
                    rcache = ro.cache;
                }
            }
        }
    }
    d.finish();
    assert!(warm_starts > 100, "warm-start path barely exercised: {warm_starts}");
}

/// Row 36 — cache warm with a hand-built `count` of 1/2/3, random valid indices,
/// random `div` and random `metric` (including hostile values).
#[test]
fn row36_cache_warm_synthetic() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x3008);
    let mut d = Diff::new("36: cache warm, synthetic count=1|2|3");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            // max valid vertex index per shape kind
            let maxA = match tA { C2_TYPE_CIRCLE => 1, C2_TYPE_CAPSULE => 2, _ => 4 };
            let maxB = match tB { C2_TYPE_CIRCLE => 1, C2_TYPE_CAPSULE => 2, _ => 4 };
            for count in [1, 2, 3] {
                for _ in 0..N / 2 {
                    let A = shape_bytes(&mut rng, tA, 10.0);
                    let B = shape_bytes(&mut rng, tB, 10.0);
                    let warm = C2GJKCache {
                        metric: match rng.below(6) {
                            0 => f32::NAN,
                            1 => -1e9,
                            2 => -1e8,
                            3 => 0.0,
                            _ => mixed(&mut rng, 50.0),
                        },
                        count,
                        iA: [
                            rng.below(maxA) as c_int,
                            rng.below(maxA) as c_int,
                            rng.below(maxA) as c_int,
                        ],
                        iB: [
                            rng.below(maxB) as c_int,
                            rng.below(maxB) as c_int,
                            rng.below(maxB) as c_int,
                        ],
                        div: match rng.below(5) {
                            0 => 0.0,
                            1 => 1.0,
                            _ => rng.unit() * 10.0 + 0.001,
                        },
                    };
                    unsafe {
                        d.check(
                            (ty_name(tA), ty_name(tB), count, warm.metric, warm.div, warm.iA, warm.iB),
                            call(&c, &A, tA, &B, tB, PLAIN, Some(warm)),
                            call(&r, &A, tA, &B, tB, PLAIN, Some(warm)),
                        );
                    }
                }
            }
        }
    }
    d.finish();
}

/// Row 37 — `outA` / `outB` / `iterations` each independently NULL
/// (all 8 combinations), asserting the return value is unaffected and that
/// non-NULL outputs still match.
#[test]
fn row37_null_output_combinations() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x3009);
    let mut d = Diff::new("37: NULL out pointer combinations");
    for mask in 0u32..8 {
        for &tA in ALL_TYPES.iter() {
            for &tB in ALL_TYPES.iter() {
                for _ in 0..N / 4 {
                    let A = shape_bytes(&mut rng, tA, 10.0);
                    let B = shape_bytes(&mut rng, tB, 10.0);
                    let mut got = [C2v { x: -1.0, y: -2.0 }; 2];
                    let mut cres = (0.0f32, got[0], got[1], -777 as c_int);
                    let mut rres = cres;
                    for (which, f) in [(0, &c), (1, &r)] {
                        let mut a = C2v { x: 91.0, y: 92.0 };
                        let mut b = C2v { x: 93.0, y: 94.0 };
                        let mut it: c_int = -777;
                        let dist = unsafe {
                            f(
                                A.as_ptr() as *const c_void,
                                tA,
                                std::ptr::null(),
                                B.as_ptr() as *const c_void,
                                tB,
                                std::ptr::null(),
                                if mask & 1 != 0 { std::ptr::null_mut() } else { &mut a },
                                if mask & 2 != 0 { std::ptr::null_mut() } else { &mut b },
                                1,
                                if mask & 4 != 0 { std::ptr::null_mut() } else { &mut it },
                                std::ptr::null_mut(),
                            )
                        };
                        let tup = (dist, a, b, it);
                        if which == 0 { cres = tup } else { rres = tup }
                    }
                    got[0] = cres.1;
                    d.check((mask, ty_name(tA), ty_name(tB), &A[..], &B[..]), cres, rres);
                }
            }
        }
    }
    d.finish();
}

/// Rows 38..=46 — geometric / numeric shape families.
fn gjk_family(row: &str, seed: u64, gen: impl Fn(&mut Rng, c_int) -> [u8; 20]) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for use_radius in [0, 1] {
                for _ in 0..N {
                    let A = gen(&mut rng, tA);
                    let B = gen(&mut rng, tB);
                    let o = GjkOpts { use_radius, ..PLAIN };
                    unsafe {
                        d.check(
                            (ty_name(tA), ty_name(tB), use_radius, &A[..], &B[..]),
                            call(&c, &A, tA, &B, tB, o, None),
                            call(&r, &A, tA, &B, tB, o, None),
                        );
                    }
                }
            }
        }
    }
    d.finish();
}

/// Serialise a shape built from explicit floats.
fn pack(ty: c_int, v: [f32; 5]) -> [u8; 20] {
    let mut buf = [0u8; 20];
    for (i, f) in v.iter().enumerate() {
        buf[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
    }
    let _ = ty;
    buf
}

#[test]
fn row38_far_apart_many_iterations() {
    // shapes separated by a large distance -> the loop runs to its limit
    gjk_family("38: shapes far apart (max iterations)", 0x300a, |rng, ty| {
        let o = if rng.boolean() { 1e6 } else { -1e6 };
        pack(
            ty,
            [
                o + rng.sym(5.0),
                o + rng.sym(5.0),
                o + rng.sym(5.0),
                o + rng.sym(5.0),
                rng.unit() * 2.0,
            ],
        )
    });
}

#[test]
fn row39_deep_overlap_hit_branch() {
    gjk_family("39: deeply overlapping (hit branch)", 0x300b, |rng, ty| {
        // everything crammed around the origin with generous extents
        match ty {
            C2_TYPE_CIRCLE => pack(ty, [rng.sym(0.5), rng.sym(0.5), 5.0 + rng.unit(), 0.0, 0.0]),
            C2_TYPE_AABB => pack(ty, [-5.0 - rng.unit(), -5.0 - rng.unit(), 5.0 + rng.unit(), 5.0 + rng.unit(), 0.0]),
            _ => pack(ty, [rng.sym(2.0), rng.sym(2.0), rng.sym(2.0), rng.sym(2.0), 4.0 + rng.unit()]),
        }
    });
}

#[test]
fn row40_exactly_touching() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x300c);
    let mut d = Diff::new("40: exactly touching (dist == rA+rB boundary)");
    for _ in 0..N * 8 {
        // two circles whose centre distance is exactly rA + rB (and +-1ulp)
        let rA = (rng.below(8) + 1) as f32 * 0.5;
        let rB = (rng.below(8) + 1) as f32 * 0.5;
        let gap = rA + rB;
        let nudge = [0.0f32, f32::EPSILON, -f32::EPSILON, 1.0e-6, -1.0e-6][rng.below(5)];
        let A = pack(C2_TYPE_CIRCLE, [0.0, 0.0, rA, 0.0, 0.0]);
        let B = pack(C2_TYPE_CIRCLE, [gap + nudge, 0.0, rB, 0.0, 0.0]);
        for use_radius in [0, 1] {
            let o = GjkOpts { use_radius, ..PLAIN };
            unsafe {
                d.check(
                    (rA, rB, nudge, use_radius),
                    call(&c, &A, C2_TYPE_CIRCLE, &B, C2_TYPE_CIRCLE, o, None),
                    call(&r, &A, C2_TYPE_CIRCLE, &B, C2_TYPE_CIRCLE, o, None),
                );
            }
        }
        // AABB edge-to-edge exact touch
        let e = (rng.below(5) + 1) as f32;
        let A2 = pack(C2_TYPE_AABB, [0.0, 0.0, e, e, 0.0]);
        let B2 = pack(C2_TYPE_AABB, [e + nudge, 0.0, e * 2.0, e, 0.0]);
        for use_radius in [0, 1] {
            let o = GjkOpts { use_radius, ..PLAIN };
            unsafe {
                d.check(
                    ("aabb touch", e, nudge, use_radius),
                    call(&c, &A2, C2_TYPE_AABB, &B2, C2_TYPE_AABB, o, None),
                    call(&r, &A2, C2_TYPE_AABB, &B2, C2_TYPE_AABB, o, None),
                );
            }
        }
    }
    d.finish();
}

#[test]
fn row41_identical_and_coincident() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x300d);
    let mut d = Diff::new("41: identical / coincident shapes (dist <= FLT_EPSILON)");
    for &ty in ALL_TYPES.iter() {
        for _ in 0..N * 4 {
            let A = shape_bytes(&mut rng, ty, 10.0);
            let B = A; // byte-identical shape
            for use_radius in [0, 1] {
                let o = GjkOpts { use_radius, ..PLAIN };
                unsafe {
                    d.check(
                        (ty_name(ty), use_radius, &A[..]),
                        call(&c, &A, ty, &B, ty, o, None),
                        call(&r, &A, ty, &B, ty, o, None),
                    );
                }
            }
        }
    }
    // coincident zero-radius circles: dist == 0 exactly
    for &use_radius in [0, 1].iter() {
        let A = pack(C2_TYPE_CIRCLE, [1.0, 2.0, 0.0, 0.0, 0.0]);
        let o = GjkOpts { use_radius, ..PLAIN };
        unsafe {
            d.check(
                ("coincident zero-radius circles", use_radius),
                call(&c, &A, C2_TYPE_CIRCLE, &A, C2_TYPE_CIRCLE, o, None),
                call(&r, &A, C2_TYPE_CIRCLE, &A, C2_TYPE_CIRCLE, o, None),
            );
        }
    }
    d.finish();
}

#[test]
fn row42_degenerate_shapes() {
    gjk_family("42: degenerate shapes (zero radius/length/area)", 0x300e, |rng, ty| {
        match ty {
            C2_TYPE_CIRCLE => pack(ty, [rng.grid(), rng.grid(), 0.0, 0.0, 0.0]),
            C2_TYPE_AABB => {
                let p = rng.grid();
                let q = rng.grid();
                // zero-area: point AABB, or a zero-width/height slab
                match rng.below(3) {
                    0 => pack(ty, [p, q, p, q, 0.0]),
                    1 => pack(ty, [p, q, p, q + 2.0, 0.0]),
                    _ => pack(ty, [p, q, p + 2.0, q, 0.0]),
                }
            }
            _ => {
                let p = rng.grid();
                let q = rng.grid();
                pack(ty, [p, q, p, q, if rng.boolean() { 0.0 } else { rng.unit() }])
            }
        }
    });
}

#[test]
fn row43_huge_coordinates() {
    gjk_family("43: huge coordinates (overflow to inf)", 0x300f, |rng, ty| {
        let s = [1e18f32, 1e30, 1e35, f32::MAX / 4.0][rng.below(4)];
        pack(
            ty,
            [
                rng.sym(s),
                rng.sym(s),
                rng.sym(s),
                rng.sym(s),
                rng.unit() * s,
            ],
        )
    });
}

#[test]
fn row44_tiny_coordinates() {
    gjk_family("44: tiny coordinates / denormals", 0x3010, |rng, ty| {
        let s = [1e-30f32, 1e-38, 1e-44, 1.1920929e-7][rng.below(4)];
        pack(
            ty,
            [
                rng.sym(s),
                rng.sym(s),
                rng.sym(s),
                rng.sym(s),
                rng.unit() * s,
            ],
        )
    });
}

#[test]
fn row45_negative_radii() {
    gjk_family("45: negative radii", 0x3011, |rng, ty| {
        pack(
            ty,
            [
                rng.sym(5.0),
                rng.sym(5.0),
                rng.sym(5.0),
                rng.sym(5.0),
                -(rng.unit() * 5.0 + 0.01),
            ],
        )
    });
}

#[test]
fn row46_inverted_aabb() {
    gjk_family("46: inverted AABB winding", 0x3012, |rng, ty| {
        match ty {
            C2_TYPE_AABB => {
                // deliberately min > max
                let x = rng.sym(5.0);
                let y = rng.sym(5.0);
                pack(ty, [x, y, x - rng.unit() * 4.0 - 0.5, y - rng.unit() * 4.0 - 0.5, 0.0])
            }
            _ => shape_bytes(rng, ty, 5.0),
        }
    });
}
