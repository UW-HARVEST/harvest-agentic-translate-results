//! Level 2: `c2GJK`, the shape-vs-shape predicates, `c2Collided` and the
//! public `capsule` entry point.
//!
//! `c2GJK` has out-params (`outA`, `outB`, `iterations`) and an in/out `cache`;
//! all of them are compared, and the cache is also round-tripped so the
//! warm-start path is exercised.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_void;

type GjkFn = unsafe extern "C" fn(
    *const c_void, // A
    i32,           // typeA
    *const c2x,    // ax
    *const c_void, // B
    i32,           // typeB
    *const c2x,    // bx
    *mut c2v,      // outA
    *mut c2v,      // outB
    i32,           // use_radius
    *mut i32,      // iterations
    *mut c2GJKCache,
) -> f32;

/// One of the three concrete shapes, kept alive so its address stays valid.
#[derive(Copy, Clone, Debug)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> i32 {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
    }

    /// Number of proxy vertices `c2MakeProxy` will populate.
    fn vert_count(&self) -> u32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }

    fn random(rng: &mut Rng) -> Shape {
        match rng.below(3) {
            0 => Shape::Circle(rng.circle()),
            1 => Shape::Aabb(rng.aabb()),
            _ => Shape::Capsule(rng.capsule()),
        }
    }
}

/// Result bundle of one `c2GJK` invocation.
#[derive(Copy, Clone, Debug, PartialEq)]
struct GjkOut {
    dist: f32,
    a: c2v,
    b: c2v,
    iterations: i32,
    cache: Option<c2GJKCache>,
}

#[allow(clippy::too_many_arguments)]
unsafe fn call_gjk(
    f: &GjkFn,
    a: &Shape,
    ax: Option<&c2x>,
    b: &Shape,
    bx: Option<&c2x>,
    use_radius: i32,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    let sentinel = c2v { x: -31337.0, y: 27182.0 };
    let mut outA = sentinel;
    let mut outB = sentinel;
    let mut iterations: i32 = -1;
    let mut cache = cache_in;

    let dist = unsafe {
        f(
            a.ptr(),
            a.ty(),
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            b.ptr(),
            b.ty(),
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            &mut outA,
            &mut outB,
            use_radius,
            &mut iterations,
            cache.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache),
        )
    };

    GjkOut { dist, a: outA, b: outB, iterations, cache }
}

fn assert_gjk_eq(c: &GjkOut, r: &GjkOut, ctx: &str) {
    assert_f32_eq(c.dist, r.dist, &format!("{ctx}: dist"));
    assert_bytes_eq(&c.a, &r.a, &format!("{ctx}: outA"));
    assert_bytes_eq(&c.b, &r.b, &format!("{ctx}: outB"));
    assert_eq!(c.iterations, r.iterations, "{ctx}: iterations");
    match (&c.cache, &r.cache) {
        (Some(cc), Some(rc)) => assert_bytes_eq(cc, rc, &format!("{ctx}: cache")),
        (None, None) => {}
        _ => panic!("{ctx}: cache presence mismatch"),
    }
}

// ---------------------------------------------------------------------------
// c2GJK
// ---------------------------------------------------------------------------

/// All nine type pairs, both identity (null) and explicit transforms, both
/// `use_radius` settings, with no cache.
#[test]
fn c2GJK_matches_without_cache() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x22_0001);
    for i in 0..scale(6000) {
        let sa = Shape::random(&mut rng);
        let sb = Shape::random(&mut rng);
        let ax = if rng.below(2) == 0 { Some(rng.xform()) } else { None };
        let bx = if rng.below(2) == 0 { Some(rng.xform()) } else { None };
        let use_radius = (rng.below(2)) as i32;

        let cv = unsafe { call_gjk(&c, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, None) };
        let rv = unsafe { call_gjk(&r, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, None) };
        assert_gjk_eq(
            &cv,
            &rv,
            &format!(
                "c2GJK #{i} A={sa:?} ax={ax:?} B={sb:?} bx={bx:?} use_radius={use_radius}"
            ),
        );
    }
}

/// Every ordered `(typeA, typeB)` pair gets dedicated coverage so no
/// combination can be missed by chance.
#[test]
fn c2GJK_covers_all_type_pairs() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x22_0002);
    for ta in 0..3u32 {
        for tb in 0..3u32 {
            for i in 0..scale(400) {
                let mk = |rng: &mut Rng, t: u32| match t {
                    0 => Shape::Circle(rng.circle()),
                    1 => Shape::Aabb(rng.aabb()),
                    _ => Shape::Capsule(rng.capsule()),
                };
                let sa = mk(&mut rng, ta);
                let sb = mk(&mut rng, tb);
                for use_radius in [0, 1] {
                    let cv = unsafe { call_gjk(&c, &sa, None, &sb, None, use_radius, None) };
                    let rv = unsafe { call_gjk(&r, &sa, None, &sb, None, use_radius, None) };
                    assert_gjk_eq(
                        &cv,
                        &rv,
                        &format!("c2GJK pair({ta},{tb}) #{i} r={use_radius} A={sa:?} B={sb:?}"),
                    );
                }
            }
        }
    }
}

/// Warm start: run once to populate a cache, feed it back in, and compare both
/// the second result and the resulting cache contents.
#[test]
fn c2GJK_cache_roundtrip_matches() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x22_0003);
    for i in 0..scale(3000) {
        let sa = Shape::random(&mut rng);
        let sb = Shape::random(&mut rng);
        let ax = if rng.below(2) == 0 { Some(rng.xform()) } else { None };
        let bx = if rng.below(2) == 0 { Some(rng.xform()) } else { None };
        let use_radius = (rng.below(2)) as i32;
        let ctx = format!("c2GJK cache #{i} A={sa:?} B={sb:?} r={use_radius}");

        // Pass 1: cold cache.
        let cold = c2GJKCache::default();
        let c1 = unsafe {
            call_gjk(&c, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, Some(cold))
        };
        let r1 = unsafe {
            call_gjk(&r, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, Some(cold))
        };
        assert_gjk_eq(&c1, &r1, &format!("{ctx} pass1"));

        // Pass 2: warm cache produced by pass 1 (identical on both sides).
        let warm = c1.cache.unwrap();
        let c2 = unsafe {
            call_gjk(&c, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, Some(warm))
        };
        let r2 = unsafe {
            call_gjk(&r, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, Some(warm))
        };
        assert_gjk_eq(&c2, &r2, &format!("{ctx} pass2"));

        // Pass 3: same shapes nudged, still warm-started.
        let sa2 = match sa {
            Shape::Circle(mut x) => {
                x.p = c2Add_host(x.p, rng.finite_vec());
                Shape::Circle(x)
            }
            Shape::Aabb(mut x) => {
                let d = rng.finite_vec();
                x.min = c2Add_host(x.min, d);
                x.max = c2Add_host(x.max, d);
                Shape::Aabb(x)
            }
            Shape::Capsule(mut x) => {
                let d = rng.finite_vec();
                x.a = c2Add_host(x.a, d);
                x.b = c2Add_host(x.b, d);
                Shape::Capsule(x)
            }
        };
        let warm2 = c2.cache.unwrap();
        let c3 = unsafe {
            call_gjk(&c, &sa2, ax.as_ref(), &sb, bx.as_ref(), use_radius, Some(warm2))
        };
        let r3 = unsafe {
            call_gjk(&r, &sa2, ax.as_ref(), &sb, bx.as_ref(), use_radius, Some(warm2))
        };
        assert_gjk_eq(&c3, &r3, &format!("{ctx} pass3"));
    }
}

fn c2Add_host(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x + b.x, y: a.y + b.y }
}

/// Synthetic caches (not produced by a previous call), constrained to vertex
/// indices the proxies actually populate — reading beyond `count` would be
/// indeterminate in C.
#[test]
fn c2GJK_synthetic_cache_matches() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x22_0004);
    for i in 0..scale(4000) {
        let sa = Shape::random(&mut rng);
        let sb = Shape::random(&mut rng);
        let na = sa.vert_count();
        let nb = sb.vert_count();

        let mut cache = c2GJKCache {
            metric: match rng.below(8) {
                0 => 0.0,
                1 => -1.0e9,
                2 => -1.0e7,
                3 => f32::MAX,
                _ => rng.uniform(200.0),
            },
            count: rng.below(4) as i32, // 0 => "cache not good"
            iA: [0; 3],
            iB: [0; 3],
            div: match rng.below(6) {
                0 => 0.0,
                1 => 1.0,
                _ => rng.uniform(20.0),
            },
        };
        for k in 0..3 {
            cache.iA[k] = rng.below(na) as i32;
            cache.iB[k] = rng.below(nb) as i32;
        }
        let use_radius = (rng.below(2)) as i32;

        let cv = unsafe { call_gjk(&c, &sa, None, &sb, None, use_radius, Some(cache)) };
        let rv = unsafe { call_gjk(&r, &sa, None, &sb, None, use_radius, Some(cache)) };
        assert_gjk_eq(
            &cv,
            &rv,
            &format!("c2GJK synth-cache #{i} A={sa:?} B={sb:?} cache={cache:?} r={use_radius}"),
        );
    }
}

/// Null out-params must be tolerated identically.
#[test]
fn c2GJK_null_outparams_match() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x22_0005);
    for i in 0..scale(2000) {
        let sa = Shape::random(&mut rng);
        let sb = Shape::random(&mut rng);
        let use_radius = (rng.below(2)) as i32;
        let (cd, rd) = unsafe {
            (
                c(
                    sa.ptr(),
                    sa.ty(),
                    std::ptr::null(),
                    sb.ptr(),
                    sb.ty(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    use_radius,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ),
                r(
                    sa.ptr(),
                    sa.ty(),
                    std::ptr::null(),
                    sb.ptr(),
                    sb.ty(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    use_radius,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ),
            )
        };
        assert_f32_eq(cd, rd, &format!("c2GJK null-outparams #{i} A={sa:?} B={sb:?}"));
    }
}

/// Deeply overlapping and exactly-touching configurations, which is where the
/// `hit` / radius-shrink branches live.
#[test]
fn c2GJK_overlap_and_touching_match() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0x22_0006);
    for i in 0..scale(3000) {
        // Two shapes generated near the origin at small scale => frequent
        // overlap, and integral coordinates => frequent exact touching.
        let s = 6.0f32;
        let q = |rng: &mut Rng| c2v {
            x: (rng.uniform(s)).round(),
            y: (rng.uniform(s)).round(),
        };
        let rad = |rng: &mut Rng| (rng.uniform(s).abs()).round();
        let sa = match rng.below(3) {
            0 => Shape::Circle(c2Circle { p: q(&mut rng), r: rad(&mut rng) }),
            1 => {
                let a = q(&mut rng);
                let b = q(&mut rng);
                Shape::Aabb(c2AABB {
                    min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                    max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
                })
            }
            _ => Shape::Capsule(c2Capsule { a: q(&mut rng), b: q(&mut rng), r: rad(&mut rng) }),
        };
        let sb = match rng.below(3) {
            0 => Shape::Circle(c2Circle { p: q(&mut rng), r: rad(&mut rng) }),
            1 => {
                let a = q(&mut rng);
                let b = q(&mut rng);
                Shape::Aabb(c2AABB {
                    min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                    max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
                })
            }
            _ => Shape::Capsule(c2Capsule { a: q(&mut rng), b: q(&mut rng), r: rad(&mut rng) }),
        };
        for use_radius in [0, 1] {
            let cv = unsafe { call_gjk(&c, &sa, None, &sb, None, use_radius, None) };
            let rv = unsafe { call_gjk(&r, &sa, None, &sb, None, use_radius, None) };
            assert_gjk_eq(
                &cv,
                &rv,
                &format!("c2GJK overlap #{i} r={use_radius} A={sa:?} B={sb:?}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shape-vs-shape predicates
// ---------------------------------------------------------------------------

#[test]
fn c2AABBtoAABB_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2AABB, c2AABB) -> i32>("c2AABBtoAABB");
    let mut rng = Rng::new(0x22_0010);
    for i in 0..scale(8000) {
        let (a, b) = (rng.aabb(), rng.aabb());
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_eq!(cv, rv, "c2AABBtoAABB #{i} A={a:?} B={b:?}");
    }
}

#[test]
fn c2CircletoCircle_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2Circle, c2Circle) -> i32>("c2CircletoCircle");
    let mut rng = Rng::new(0x22_0011);
    for i in 0..scale(8000) {
        let (a, b) = (rng.circle(), rng.circle());
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_eq!(cv, rv, "c2CircletoCircle #{i} A={a:?} B={b:?}");
    }
}

#[test]
fn c2CircletoAABB_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2Circle, c2AABB) -> i32>("c2CircletoAABB");
    let mut rng = Rng::new(0x22_0012);
    for i in 0..scale(8000) {
        let (a, b) = (rng.circle(), rng.aabb());
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_eq!(cv, rv, "c2CircletoAABB #{i} A={a:?} B={b:?}");
    }
}

#[test]
fn c2CircletoCapsule_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2Circle, c2Capsule) -> i32>("c2CircletoCapsule");
    let mut rng = Rng::new(0x22_0013);
    for i in 0..scale(8000) {
        let (a, b) = (rng.circle(), rng.capsule());
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_eq!(cv, rv, "c2CircletoCapsule #{i} A={a:?} B={b:?}");
    }
}

#[test]
fn c2AABBtoCapsule_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2AABB, c2Capsule) -> i32>("c2AABBtoCapsule");
    let mut rng = Rng::new(0x22_0014);
    for i in 0..scale(6000) {
        let (a, b) = (rng.aabb(), rng.capsule());
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_eq!(cv, rv, "c2AABBtoCapsule #{i} A={a:?} B={b:?}");
    }
}

#[test]
fn c2CapsuletoCapsule_matches() {
    let l = libs();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x22_0015);
    for i in 0..scale(6000) {
        let (a, b) = (rng.capsule(), rng.capsule());
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_eq!(cv, rv, "c2CapsuletoCapsule #{i} A={a:?} B={b:?}");
    }
}

/// Small-integer shapes so the "exactly touching" boundary is hit often.
#[test]
fn predicates_match_on_lattice_inputs() {
    let l = libs();
    let (c_aa, r_aa) = l.pair::<unsafe extern "C" fn(c2AABB, c2AABB) -> i32>("c2AABBtoAABB");
    let (c_cc, r_cc) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Circle) -> i32>("c2CircletoCircle");
    let (c_ca, r_ca) = l.pair::<unsafe extern "C" fn(c2Circle, c2AABB) -> i32>("c2CircletoAABB");
    let (c_ck, r_ck) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Capsule) -> i32>("c2CircletoCapsule");
    let (c_ak, r_ak) =
        l.pair::<unsafe extern "C" fn(c2AABB, c2Capsule) -> i32>("c2AABBtoCapsule");
    let (c_kk, r_kk) =
        l.pair::<unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32>("c2CapsuletoCapsule");

    let mut rng = Rng::new(0x22_0016);
    let lat = |rng: &mut Rng| (rng.below(13) as f32) - 6.0;
    for i in 0..scale(4000) {
        let v = |rng: &mut Rng| c2v { x: lat(rng), y: lat(rng) };
        let circ = |rng: &mut Rng| c2Circle { p: v(rng), r: rng.below(7) as f32 };
        let cap = |rng: &mut Rng| c2Capsule {
            a: v(rng),
            b: v(rng),
            r: rng.below(7) as f32,
        };
        let box_ = |rng: &mut Rng| {
            let a = v(rng);
            let b = v(rng);
            c2AABB {
                min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
            }
        };

        let (a, b) = (box_(&mut rng), box_(&mut rng));
        unsafe { assert_eq!(c_aa(a, b), r_aa(a, b), "c2AABBtoAABB lattice #{i}") };
        let (a, b) = (circ(&mut rng), circ(&mut rng));
        unsafe { assert_eq!(c_cc(a, b), r_cc(a, b), "c2CircletoCircle lattice #{i}") };
        let (a, b) = (circ(&mut rng), box_(&mut rng));
        unsafe { assert_eq!(c_ca(a, b), r_ca(a, b), "c2CircletoAABB lattice #{i}") };
        let (a, b) = (circ(&mut rng), cap(&mut rng));
        unsafe { assert_eq!(c_ck(a, b), r_ck(a, b), "c2CircletoCapsule lattice #{i}") };
        let (a, b) = (box_(&mut rng), cap(&mut rng));
        unsafe { assert_eq!(c_ak(a, b), r_ak(a, b), "c2AABBtoCapsule lattice #{i}") };
        let (a, b) = (cap(&mut rng), cap(&mut rng));
        unsafe { assert_eq!(c_kk(a, b), r_kk(a, b), "c2CapsuletoCapsule lattice #{i}") };
    }
}

// ---------------------------------------------------------------------------
// c2Collided
// ---------------------------------------------------------------------------

type CollidedFn = unsafe extern "C" fn(*const c_void, i32, *const c_void, i32) -> i32;

#[test]
fn c2Collided_matches() {
    let l = libs();
    let (c, r) = l.pair::<CollidedFn>("c2Collided");
    let mut rng = Rng::new(0x22_0020);
    for i in 0..scale(6000) {
        let sa = Shape::random(&mut rng);
        let sb = Shape::random(&mut rng);
        let (cv, rv) =
            unsafe { (c(sa.ptr(), sa.ty(), sb.ptr(), sb.ty()), r(sa.ptr(), sa.ty(), sb.ptr(), sb.ty())) };
        assert_eq!(cv, rv, "c2Collided #{i} A={sa:?} B={sb:?}");
    }
}

/// `c2Collided` returns 0 for unrecognised tags without touching the pointers.
#[test]
fn c2Collided_unknown_types_match() {
    let l = libs();
    let (c, r) = l.pair::<CollidedFn>("c2Collided");
    let mut rng = Rng::new(0x22_0021);
    let bad = [-2147483648i32, -1, 3, 4, 2147483647];
    for i in 0..scale(200) {
        let sa = Shape::random(&mut rng);
        let sb = Shape::random(&mut rng);
        for &tb in &bad {
            for &ta in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                let (cv, rv) = unsafe {
                    (c(sa.ptr(), ta, sb.ptr(), tb), r(sa.ptr(), ta, sb.ptr(), tb))
                };
                assert_eq!(cv, rv, "c2Collided #{i} typeA={ta} typeB={tb}");
                assert_eq!(cv, 0, "unknown typeB should yield 0");
            }
        }
        for &ta in &bad {
            let tb = C2_TYPE_CIRCLE;
            let (cv, rv) =
                unsafe { (c(sa.ptr(), ta, sb.ptr(), tb), r(sa.ptr(), ta, sb.ptr(), tb)) };
            assert_eq!(cv, rv, "c2Collided #{i} typeA={ta} typeB={tb}");
            assert_eq!(cv, 0, "unknown typeA should yield 0");
        }
    }
}

/// `c2Collided` swaps its arguments for the mixed pairs; verify against the
/// underlying predicate so the (deliberately asymmetric) wiring is pinned.
#[test]
fn c2Collided_argument_swapping_matches_c() {
    let l = libs();
    let (c, r) = l.pair::<CollidedFn>("c2Collided");
    let mut rng = Rng::new(0x22_0022);
    for i in 0..scale(2000) {
        // AABB vs CIRCLE: C calls c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A).
        let circle = rng.circle();
        let aabb = rng.aabb();
        let pa = &aabb as *const _ as *const c_void;
        let pb = &circle as *const _ as *const c_void;
        let (cv, rv) = unsafe {
            (
                c(pa, C2_TYPE_AABB, pb, C2_TYPE_CIRCLE),
                r(pa, C2_TYPE_AABB, pb, C2_TYPE_CIRCLE),
            )
        };
        assert_eq!(cv, rv, "c2Collided(AABB,CIRCLE) #{i}");

        // CAPSULE vs CIRCLE and CAPSULE vs AABB also swap.
        let cap = rng.capsule();
        let pk = &cap as *const _ as *const c_void;
        let (cv, rv) = unsafe {
            (
                c(pk, C2_TYPE_CAPSULE, pb, C2_TYPE_CIRCLE),
                r(pk, C2_TYPE_CAPSULE, pb, C2_TYPE_CIRCLE),
            )
        };
        assert_eq!(cv, rv, "c2Collided(CAPSULE,CIRCLE) #{i}");
        let (cv, rv) = unsafe {
            (
                c(pk, C2_TYPE_CAPSULE, pa, C2_TYPE_AABB),
                r(pk, C2_TYPE_CAPSULE, pa, C2_TYPE_AABB),
            )
        };
        assert_eq!(cv, rv, "c2Collided(CAPSULE,AABB) #{i}");
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

type CapsuleFn = unsafe extern "C" fn(f32, f32, f32, f32, f32) -> i32;

#[test]
fn capsule_entry_point_matches() {
    let l = libs();
    let (c, r) = l.pair::<CapsuleFn>("capsule");
    let mut rng = Rng::new(0x22_0030);
    for i in 0..scale(20000) {
        let (a, b, cc, d, e) =
            (rng.coord(), rng.coord(), rng.coord(), rng.coord(), rng.radius());
        let (cv, rv) = unsafe { (c(a, b, cc, d, e), r(a, b, cc, d, e)) };
        assert_eq!(cv, rv, "capsule #{i}({a:?}, {b:?}, {cc:?}, {d:?}, {e:?})");
    }
}

/// Sweep the region the hard-coded scene actually occupies, so all three result
/// bits get set and cleared.
#[test]
fn capsule_entry_point_scene_sweep() {
    let l = libs();
    let (c, r) = l.pair::<CapsuleFn>("capsule");
    let mut seen = [false; 8];
    let mut n = 0usize;
    let mut x = -110.0f32;
    while x <= 30.0 {
        let mut y = -70.0f32;
        while y <= 130.0 {
            for (dx, dy) in [(0.0f32, 0.0f32), (20.0, 15.0), (-5.0, 40.0), (35.0, -20.0)] {
                for rad in [0.0f32, 1.0, 5.0, 12.5, 30.0] {
                    let v = unsafe { c(x, y, x + dx, y + dy, rad) };
                    let w = unsafe { r(x, y, x + dx, y + dy, rad) };
                    assert_eq!(v, w, "capsule sweep({x}, {y}, {}, {}, {rad})", x + dx, y + dy);
                    assert!((0..8).contains(&v), "unexpected result {v}");
                    seen[v as usize] = true;
                    n += 1;
                }
            }
            y += 7.0;
        }
        x += 7.0;
    }
    assert!(n > 1000, "sweep was too small: {n}");
    // The scene must at least produce "no collision" and each individual bit.
    assert!(seen[0], "never observed result 0");
    assert!(seen.iter().filter(|s| **s).count() >= 4, "coverage too thin: {seen:?}");
}

#[test]
fn capsule_entry_point_edge_floats() {
    let l = libs();
    let (c, r) = l.pair::<CapsuleFn>("capsule");
    let edge = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        -70.0,
        -40.0,
        -15.0,
        20.0,
        40.0,
        100.0,
        f32::MIN_POSITIVE,
        1e-45,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &a in &edge {
        for &b in &edge {
            for &e in &[0.0f32, 10.0, 20.0, f32::INFINITY] {
                let (cv, rv) = unsafe { (c(a, b, b, a, e), r(a, b, b, a, e)) };
                assert_eq!(cv, rv, "capsule edge({a:?}, {b:?}, {b:?}, {a:?}, {e:?})");
            }
        }
    }
}
