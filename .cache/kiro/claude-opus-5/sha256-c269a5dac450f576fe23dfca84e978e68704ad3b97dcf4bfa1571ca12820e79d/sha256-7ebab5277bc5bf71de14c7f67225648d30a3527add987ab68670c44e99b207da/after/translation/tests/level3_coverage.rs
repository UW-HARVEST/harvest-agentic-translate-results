//! Level 3: path coverage for `c2GJK`.
//!
//! `c2GJK` is a loop with six distinct exits and a cache-reuse decision, none
//! of which are directly observable from its return values. This file rebuilds
//! the algorithm out of the *C library's own exported leaf functions*
//! (`c22`, `c23`, `c2L`, `c2D`, `c2Support`, `c2Witness`, ...) so it can both
//!
//!   * report which exit each input took, proving the suite reaches all of
//!     them, and
//!   * act as an independent cross-check that agrees with `c2GJK` and with
//!     the Rust `c2GJK`.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const FLT_MAX: f32 = 3.402_823_466_385_288_6e38_f32;
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

type GjkFn = unsafe extern "C" fn(
    *const u8,
    c_int,
    *const c2x,
    *const u8,
    c_int,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;

/// Which statement ended the `while (iter < 20)` loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Exit {
    /// `if (s.count == 3) { hit = 1; break; }`
    Hit,
    /// `if (d1 > d0) break;`
    NotDescending,
    /// `if (c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON) break;`
    TinyDirection,
    /// `if (dup) break;`
    DuplicateSupport,
    /// loop condition `iter < 20` became false
    IterationCap,
}

/// Leaf functions, all resolved from one library.
struct Leaves<'a> {
    make_proxy: libloading::Symbol<'a, unsafe extern "C" fn(*const u8, c_int, *mut c2Proxy)>,
    metric: libloading::Symbol<'a, unsafe extern "C" fn(*mut c2Simplex) -> f32>,
    c22: libloading::Symbol<'a, unsafe extern "C" fn(*mut c2Simplex)>,
    c23: libloading::Symbol<'a, unsafe extern "C" fn(*mut c2Simplex)>,
    cl: libloading::Symbol<'a, unsafe extern "C" fn(*mut c2Simplex) -> c2v>,
    cd: libloading::Symbol<'a, unsafe extern "C" fn(*mut c2Simplex) -> c2v>,
    witness: libloading::Symbol<'a, unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)>,
    support: libloading::Symbol<'a, unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int>,
    dot: libloading::Symbol<'a, unsafe extern "C" fn(c2v, c2v) -> f32>,
    sub: libloading::Symbol<'a, unsafe extern "C" fn(c2v, c2v) -> c2v>,
    add: libloading::Symbol<'a, unsafe extern "C" fn(c2v, c2v) -> c2v>,
    mulvs: libloading::Symbol<'a, unsafe extern "C" fn(c2v, f32) -> c2v>,
    mulxv: libloading::Symbol<'a, unsafe extern "C" fn(c2x, c2v) -> c2v>,
    mulrvT: libloading::Symbol<'a, unsafe extern "C" fn(c2r, c2v) -> c2v>,
    neg: libloading::Symbol<'a, unsafe extern "C" fn(c2v) -> c2v>,
    len: libloading::Symbol<'a, unsafe extern "C" fn(c2v) -> f32>,
    norm: libloading::Symbol<'a, unsafe extern "C" fn(c2v) -> c2v>,
    xid: libloading::Symbol<'a, unsafe extern "C" fn() -> c2x>,
}

impl<'a> Leaves<'a> {
    fn new(lib: &'a libloading::Library) -> Self {
        macro_rules! g {
            ($n:literal) => {
                unsafe { lib.get($n) }.expect(concat!("missing ", stringify!($n)))
            };
        }
        Leaves {
            make_proxy: g!(b"c2MakeProxy"),
            metric: g!(b"c2GJKSimplexMetric"),
            c22: g!(b"c22"),
            c23: g!(b"c23"),
            cl: g!(b"c2L"),
            cd: g!(b"c2D"),
            witness: g!(b"c2Witness"),
            support: g!(b"c2Support"),
            dot: g!(b"c2Dot"),
            sub: g!(b"c2Sub"),
            add: g!(b"c2Add"),
            mulvs: g!(b"c2Mulvs"),
            mulxv: g!(b"c2Mulxv"),
            mulrvT: g!(b"c2MulrvT"),
            neg: g!(b"c2Neg"),
            len: g!(b"c2Len"),
            norm: g!(b"c2Norm"),
            xid: g!(b"c2xIdentity"),
        }
    }
}

struct RefResult {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    exit: Exit,
    cache_was_read: bool,
    cache_out: Option<c2GJKCache>,
}

/// Faithful re-execution of `c2GJK` using only exported leaf symbols.
#[allow(clippy::too_many_arguments)]
unsafe fn gjk_reference(
    lv: &Leaves,
    A: *const u8,
    typeA: c_int,
    ax_in: Option<c2x>,
    B: *const u8,
    typeB: c_int,
    bx_in: Option<c2x>,
    use_radius: c_int,
    cache_in: Option<c2GJKCache>,
) -> RefResult {
    unsafe {
        let ax = ax_in.unwrap_or_else(|| (lv.xid)());
        let bx = bx_in.unwrap_or_else(|| (lv.xid)());

        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        (lv.make_proxy)(A, typeA, &mut pA);
        (lv.make_proxy)(B, typeB, &mut pB);

        let mut s = c2Simplex::default();
        let mut cache_was_read = false;

        if let Some(cache) = cache_in {
            if cache.count != 0 {
                for i in 0..cache.count as usize {
                    let iA = cache.iA[i];
                    let iB = cache.iB[i];
                    let sA = (lv.mulxv)(ax, pA.verts[iA as usize]);
                    let sB = (lv.mulxv)(bx, pB.verts[iB as usize]);
                    s.verts[i].iA = iA;
                    s.verts[i].sA = sA;
                    s.verts[i].iB = iB;
                    s.verts[i].sB = sB;
                    s.verts[i].p = (lv.sub)(sB, sA);
                    s.verts[i].u = 0.0;
                }
                s.count = cache.count;
                s.div = cache.div;
                let metric_old = cache.metric;
                let metric = (lv.metric)(&mut s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                    cache_was_read = true;
                }
            }
        }

        if !cache_was_read {
            s.verts[0].iA = 0;
            s.verts[0].iB = 0;
            s.verts[0].sA = (lv.mulxv)(ax, pA.verts[0]);
            s.verts[0].sB = (lv.mulxv)(bx, pB.verts[0]);
            s.verts[0].p = (lv.sub)(s.verts[0].sB, s.verts[0].sA);
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        let mut saveA = [0i32; 3];
        let mut saveB = [0i32; 3];
        let mut d0 = FLT_MAX;
        let mut iter: c_int = 0;
        let mut hit = false;
        let mut exit = Exit::IterationCap;

        while iter < 20 {
            let save_count = s.count;
            for i in 0..save_count as usize {
                saveA[i] = s.verts[i].iA;
                saveB[i] = s.verts[i].iB;
            }

            match s.count {
                2 => (lv.c22)(&mut s),
                3 => (lv.c23)(&mut s),
                _ => {}
            }

            if s.count == 3 {
                hit = true;
                exit = Exit::Hit;
                break;
            }

            let p = (lv.cl)(&mut s);
            let d1 = (lv.dot)(p, p);
            if d1 > d0 {
                exit = Exit::NotDescending;
                break;
            }
            d0 = d1;

            let d = (lv.cd)(&mut s);
            if (lv.dot)(d, d) < FLT_EPSILON * FLT_EPSILON {
                exit = Exit::TinyDirection;
                break;
            }

            let iA = (lv.support)(pA.verts.as_ptr(), pA.count, (lv.mulrvT)(ax.r, (lv.neg)(d)));
            let sA = (lv.mulxv)(ax, pA.verts[iA as usize]);
            let iB = (lv.support)(pB.verts.as_ptr(), pB.count, (lv.mulrvT)(bx.r, d));
            let sB = (lv.mulxv)(bx, pB.verts[iB as usize]);

            let idx = s.count as usize;
            s.verts[idx].iA = iA;
            s.verts[idx].sA = sA;
            s.verts[idx].iB = iB;
            s.verts[idx].sB = sB;
            s.verts[idx].p = (lv.sub)(sB, sA);

            let mut dup = false;
            for i in 0..save_count as usize {
                if iA == saveA[i] && iB == saveB[i] {
                    dup = true;
                    break;
                }
            }
            if dup {
                exit = Exit::DuplicateSupport;
                break;
            }

            s.count += 1;
            iter += 1;
        }

        let mut a = c2v::default();
        let mut b = c2v::default();
        (lv.witness)(&mut s, &mut a, &mut b);
        let mut dist = (lv.len)((lv.sub)(a, b));

        if hit {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            if dist > rA + rB && dist > FLT_EPSILON {
                dist -= rA + rB;
                let n = (lv.norm)((lv.sub)(b, a));
                a = (lv.add)(a, (lv.mulvs)(n, rA));
                b = (lv.sub)(b, (lv.mulvs)(n, rB));
                if a.x == b.x && a.y == b.y {
                    dist = 0.0;
                }
            } else {
                let p = (lv.mulvs)((lv.add)(a, b), 0.5f32);
                a = p;
                b = p;
                dist = 0.0;
            }
        }

        // Note: the C code only writes `cache->iA/iB[0 .. s.count]`, so the
        // remaining slots keep whatever the caller passed in.
        let cache_out = cache_in.map(|prev| {
            let mut co = c2GJKCache {
                metric: (lv.metric)(&mut s),
                count: s.count,
                iA: prev.iA,
                iB: prev.iB,
                div: s.div,
            };
            for i in 0..s.count as usize {
                co.iA[i] = s.verts[i].iA;
                co.iB[i] = s.verts[i].iB;
            }
            co
        });

        RefResult {
            dist,
            a,
            b,
            iters: iter,
            exit,
            cache_was_read,
            cache_out,
        }
    }
}

// ---------------------------------------------------------------------------
// Shape generation (mirrors level2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const u8 {
        match self {
            Shape::Circle(c) => (c as *const c2Circle).cast(),
            Shape::Aabb(a) => (a as *const c2AABB).cast(),
            Shape::Capsule(c) => (c as *const c2Capsule).cast(),
        }
    }
    fn vert_count(&self) -> usize {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

fn rand_shape(rng: &mut Rng, kind: usize, scale: f32) -> Shape {
    match kind % 3 {
        0 => Shape::Circle(c2Circle {
            p: c2v {
                x: rng.unit() * scale,
                y: rng.unit() * scale,
            },
            r: rng.unit().abs() * scale * 0.3,
        }),
        1 => {
            let (x, y) = (rng.unit() * scale, rng.unit() * scale);
            Shape::Aabb(c2AABB {
                min: c2v { x, y },
                max: c2v {
                    x: x + rng.unit().abs() * scale * 0.5,
                    y: y + rng.unit().abs() * scale * 0.5,
                },
            })
        }
        _ => Shape::Capsule(c2Capsule {
            a: c2v {
                x: rng.unit() * scale,
                y: rng.unit() * scale,
            },
            b: c2v {
                x: rng.unit() * scale,
                y: rng.unit() * scale,
            },
            r: rng.unit().abs() * scale * 0.2,
        }),
    }
}

// ---------------------------------------------------------------------------
// The coverage test
// ---------------------------------------------------------------------------

/// `c2MakeProxy` puts this vertex at `verts[0]`, which is where the initial
/// simplex point comes from.
fn first_vert(s: &Shape) -> c2v {
    match s {
        Shape::Circle(c) => c.p,
        Shape::Aabb(a) => a.min,
        Shape::Capsule(c) => c.a,
    }
}

/// Move `s` so that its first proxy vertex lands exactly on `p`.
fn with_first_vert(s: Shape, p: c2v) -> Shape {
    match s {
        Shape::Circle(mut c) => {
            c.p = p;
            Shape::Circle(c)
        }
        Shape::Aabb(mut a) => {
            let w = a.max.x - a.min.x;
            let h = a.max.y - a.min.y;
            a.min = p;
            a.max = c2v {
                x: p.x + w,
                y: p.y + h,
            };
            Shape::Aabb(a)
        }
        Shape::Capsule(mut c) => {
            let d = c2v {
                x: c.b.x - c.a.x,
                y: c.b.y - c.a.y,
            };
            c.a = p;
            c.b = c2v {
                x: p.x + d.x,
                y: p.y + d.y,
            };
            Shape::Capsule(c)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compare_all(
    c_gjk: &GjkFn,
    r_gjk: &GjkFn,
    lv: &Leaves,
    A: &Shape,
    ax: Option<c2x>,
    B: &Shape,
    bx: Option<c2x>,
    use_radius: c_int,
    cache: Option<c2GJKCache>,
    ctx: &str,
) -> RefResult {
    let call = |f: &GjkFn, cache_in: Option<c2GJKCache>| {
        let mut a = c2v::default();
        let mut b = c2v::default();
        let mut iters: c_int = 0;
        let mut cc = cache_in;
        let cp = match cc.as_mut() {
            Some(x) => x as *mut c2GJKCache,
            None => std::ptr::null_mut(),
        };
        let axp = ax.as_ref().map(|x| x as *const c2x).unwrap_or(std::ptr::null());
        let bxp = bx.as_ref().map(|x| x as *const c2x).unwrap_or(std::ptr::null());
        let dist = unsafe {
            f(
                A.ptr(), A.ty(), axp, B.ptr(), B.ty(), bxp,
                &mut a, &mut b, use_radius, &mut iters, cp,
            )
        };
        (dist, a, b, iters, cc)
    };

    let (cd, ca, cb, ci, ccache) = call(c_gjk, cache);
    let (rd, ra, rb, ri, rcache) = call(r_gjk, cache);

    assert!(
        f32_eq_nan_ok(cd, rd),
        "dist: C={cd:?} Rust={rd:?} | {ctx}"
    );
    assert!(c2v_eq_nan_ok(ca, ra), "outA: C={ca:?} Rust={ra:?} | {ctx}");
    assert!(c2v_eq_nan_ok(cb, rb), "outB: C={cb:?} Rust={rb:?} | {ctx}");
    assert_eq!(ci, ri, "iterations | {ctx}");
    if let (Some(x), Some(y)) = (ccache, rcache) {
        assert_eq!(x.count, y.count, "cache.count | {ctx}");
        assert_eq!(x.iA, y.iA, "cache.iA | {ctx}");
        assert_eq!(x.iB, y.iB, "cache.iB | {ctx}");
        assert!(f32_eq_nan_ok(x.metric, y.metric), "cache.metric | {ctx}");
        assert!(f32_eq_nan_ok(x.div, y.div), "cache.div | {ctx}");
    }

    // Independent reference built from the C .so's leaf exports.
    let refr = unsafe {
        gjk_reference(
            lv, A.ptr(), A.ty(), ax, B.ptr(), B.ty(), bx, use_radius, cache,
        )
    };
    assert!(
        f32_eq_nan_ok(refr.dist, cd),
        "reference driver disagrees with c2GJK: ref={:?} C={cd:?} | {ctx}",
        refr.dist
    );
    assert!(
        c2v_eq_nan_ok(refr.a, ca) && c2v_eq_nan_ok(refr.b, cb),
        "reference driver witness disagrees | {ctx}"
    );
    assert_eq!(refr.iters, ci, "reference driver iterations disagree | {ctx}");
    if let (Some(x), Some(y)) = (refr.cache_out, ccache) {
        assert_eq!(x.count, y.count, "reference cache.count | {ctx}");
        assert_eq!(x.iA, y.iA, "reference cache.iA | {ctx}");
        assert_eq!(x.iB, y.iB, "reference cache.iB | {ctx}");
    }

    refr
}

#[test]
fn t_gjk_loop_exit_coverage() {
    let l = libs();
    let (c_gjk, r_gjk) = l.sym::<GjkFn>(b"c2GJK");
    let lv = Leaves::new(&l.c);

    let mut exits: std::collections::HashMap<Exit, usize> = Default::default();
    let mut cache_read = [0usize; 2];
    let mut iter_hist = [0usize; 21];

    let mut rng = Rng::new(70);
    // Broad sweep: three magnitude regimes, all type pairs, both use_radius,
    // transforms on/off, and caches on/off.
    for scale in [1e-3f32, 1.0, 150.0, 3.0e4, 1.0e9] {
        for ka in 0..3usize {
            for kb in 0..3usize {
                for use_radius in [0i32, 1] {
                    for variant in 0..4u32 {
                        for i in 0..60 {
                            let A = rand_shape(&mut rng, ka, scale);
                            // Every third case shares its first proxy vertex
                            // with A, which drives `d` to zero and reaches the
                            // `c2Dot(d, d) < FLT_EPSILON^2` exit.
                            let B = if i % 3 == 1 {
                                with_first_vert(rand_shape(&mut rng, kb, scale), first_vert(&A))
                            } else {
                                rand_shape(&mut rng, kb, scale)
                            };
                            let ang = rng.unit() * std::f32::consts::PI;
                            let x = c2x {
                                p: c2v {
                                    x: rng.unit() * scale,
                                    y: rng.unit() * scale,
                                },
                                r: c2r {
                                    c: ang.cos(),
                                    s: ang.sin(),
                                },
                            };
                            let (ax, bx) = match variant {
                                0 => (None, None),
                                1 => (Some(x), None),
                                2 => (None, Some(x)),
                                _ => (Some(x), Some(x)),
                            };
                            let cache = if i % 3 == 0 {
                                let na = A.vert_count();
                                let nb = B.vert_count();
                                let mut cc = c2GJKCache {
                                    metric: [0.0f32, -1e9, 1.0, -3e8][i % 4],
                                    count: (i % 4) as c_int,
                                    iA: [0; 3],
                                    iB: [0; 3],
                                    div: [1.0f32, 2.0, 0.5][i % 3],
                                };
                                for k in 0..3 {
                                    cc.iA[k] = (rng.next_u32() as usize % na) as c_int;
                                    cc.iB[k] = (rng.next_u32() as usize % nb) as c_int;
                                }
                                Some(cc)
                            } else {
                                None
                            };
                            let ctx = format!(
                                "scale={scale} ka={ka} kb={kb} ur={use_radius} \
                                 variant={variant} i={i} A={A:?} B={B:?} cache={cache:?}"
                            );
                            let refr = compare_all(
                                &c_gjk, &r_gjk, &lv, &A, ax, &B, bx, use_radius, cache, &ctx,
                            );
                            *exits.entry(refr.exit).or_default() += 1;
                            cache_read[refr.cache_was_read as usize] += 1;
                            if (0..=20).contains(&refr.iters) {
                                iter_hist[refr.iters as usize] += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    eprintln!("GJK loop exits: {exits:?}");
    eprintln!("cache_was_read false/true: {cache_read:?}");
    eprintln!("iteration histogram: {iter_hist:?}");

    for e in [
        Exit::Hit,
        Exit::TinyDirection,
        Exit::DuplicateSupport,
        Exit::NotDescending,
    ] {
        assert!(
            exits.get(&e).copied().unwrap_or(0) > 0,
            "GJK exit {e:?} never reached; exits seen: {exits:?}"
        );
    }
    // `Exit::IterationCap` (iter reaching 20) is not asserted: with proxies of
    // at most four vertices, the duplicate-support check plus the monotone
    // `d1 > d0` descent test bound the loop well below 20. See
    // `t_gjk_iteration_cap_search`, which searches for it explicitly.
    eprintln!(
        "IterationCap reached {} time(s)",
        exits.get(&Exit::IterationCap).copied().unwrap_or(0)
    );
    assert!(
        cache_read[1] > 0,
        "the cache-reuse path was never taken: {cache_read:?}"
    );
    assert!(
        iter_hist[0] > 0 && iter_hist[1..].iter().sum::<usize>() > 0,
        "iteration counts not varied: {iter_hist:?}"
    );
}

/// Deliberately drive the two rarest exits: `d1 > d0` (the objective failed to
/// decrease) and the 20-iteration cap, plus the `cache_was_read == false`
/// branch, which needs a cached simplex whose metric is below -1e8.
#[test]
fn t_gjk_rare_paths() {
    let l = libs();
    let (c_gjk, r_gjk) = l.sym::<GjkFn>(b"c2GJK");
    let lv = Leaves::new(&l.c);

    let mut exits: std::collections::HashMap<Exit, usize> = Default::default();
    let mut rejected_cache = 0usize;

    let mut rng = Rng::new(71);
    // Huge, wildly-scaled AABB/capsule pairs plus count==3 caches with very
    // negative metrics: this is where `metric < -1.0e8f` becomes reachable and
    // where the descent test can fail.
    for scale in [1.0e4f32, 1.0e5, 1.0e6, 1.0e8, 1.0e18, 1.0e30] {
        for ka in 0..3usize {
            for kb in 0..3usize {
                for use_radius in [0i32, 1] {
                    for i in 0..120 {
                        let A = rand_shape(&mut rng, ka, scale);
                        let B = rand_shape(&mut rng, kb, scale);
                        let na = A.vert_count();
                        let nb = B.vert_count();
                        let mut cc = c2GJKCache {
                            metric: [
                                -1.0e9f32,
                                -1.0e8,
                                -1.00001e8,
                                -f32::MAX,
                                f32::NEG_INFINITY,
                                0.0,
                            ][i % 6],
                            count: 3,
                            iA: [0; 3],
                            iB: [0; 3],
                            div: [1.0f32, 1e-9, -1.0][i % 3],
                        };
                        for k in 0..3 {
                            cc.iA[k] = (rng.next_u32() as usize % na) as c_int;
                            cc.iB[k] = (rng.next_u32() as usize % nb) as c_int;
                        }
                        let ctx = format!(
                            "rare scale={scale} ka={ka} kb={kb} ur={use_radius} i={i} \
                             A={A:?} B={B:?} cache={cc:?}"
                        );
                        let refr = compare_all(
                            &c_gjk, &r_gjk, &lv, &A, None, &B, None, use_radius, Some(cc), &ctx,
                        );
                        *exits.entry(refr.exit).or_default() += 1;
                        if !refr.cache_was_read {
                            rejected_cache += 1;
                        }
                    }
                }
            }
        }
    }

    eprintln!("rare-path exits: {exits:?}");
    eprintln!("caches rejected (cache_was_read == false with count != 0): {rejected_cache}");
    assert!(
        rejected_cache > 0,
        "never exercised the `metric < -1.0e8f` cache-rejection branch"
    );
    // NotDescending / IterationCap are not guaranteed by construction, but
    // report whether they were reached so the coverage gap is visible.
    for e in [Exit::NotDescending, Exit::IterationCap] {
        eprintln!("{e:?} reached {} time(s)", exits.get(&e).copied().unwrap_or(0));
    }
}

/// Search hard for inputs that make the loop run to `iter == 20`. Whether or
/// not any are found, C and Rust must agree on every case tried, and the
/// highest observed iteration count is reported.
#[test]
fn t_gjk_iteration_cap_search() {
    let l = libs();
    let (c_gjk, r_gjk) = l.sym::<GjkFn>(b"c2GJK");
    let lv = Leaves::new(&l.c);

    let mut best = -1i32;
    let mut hist = [0usize; 21];
    let mut rng = Rng::new(72);

    // AABB/AABB and AABB/capsule pairs give the largest vertex sets (4), so
    // they offer the most room for the support sequence to keep finding new
    // index pairs. Also throw in adversarial transforms with non-unit rotors,
    // which distort the support mapping.
    for &(ka, kb) in &[(1usize, 1usize), (1, 2), (2, 1), (2, 2), (1, 0), (0, 1)] {
        for use_radius in [0i32, 1] {
            for i in 0..6000 {
                let scale = [1.0f32, 1e-6, 1e3, 1e7, 1e20, 1e35][i % 6];
                let A = rand_shape(&mut rng, ka, scale);
                let B = rand_shape(&mut rng, kb, scale);
                // Deliberately non-normalised rotors: legal input, and they
                // break the usual geometric invariants.
                let mk = |rng: &mut Rng| c2x {
                    p: c2v {
                        x: rng.unit() * scale,
                        y: rng.unit() * scale,
                    },
                    r: c2r {
                        c: rng.unit() * 4.0,
                        s: rng.unit() * 4.0,
                    },
                };
                let ax = if i % 2 == 0 { Some(mk(&mut rng)) } else { None };
                let bx = if i % 3 == 0 { Some(mk(&mut rng)) } else { None };
                let ctx = format!(
                    "itercap ka={ka} kb={kb} ur={use_radius} i={i} scale={scale} \
                     A={A:?} B={B:?} ax={ax:?} bx={bx:?}"
                );
                let refr = compare_all(
                    &c_gjk, &r_gjk, &lv, &A, ax, &B, bx, use_radius, None, &ctx,
                );
                if (0..=20).contains(&refr.iters) {
                    hist[refr.iters as usize] += 1;
                }
                best = best.max(refr.iters);
            }
        }
    }

    eprintln!("iteration-cap search: max iterations = {best}, histogram = {hist:?}");
    assert!(best >= 1, "search produced no multi-iteration cases");
}
