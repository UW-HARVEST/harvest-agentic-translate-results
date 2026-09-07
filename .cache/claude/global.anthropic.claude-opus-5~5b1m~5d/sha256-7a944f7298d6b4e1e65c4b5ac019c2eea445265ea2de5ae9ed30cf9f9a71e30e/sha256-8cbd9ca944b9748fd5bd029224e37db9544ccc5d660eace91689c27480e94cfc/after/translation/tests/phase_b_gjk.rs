//! Phase B — CONFIGS.md rows 48..77: `c2GJK`, the lowest-level general entry
//! point, driven over the full cross product of
//! (typeA × typeB) × (transform kind) × use_radius × cache mode × out-pointer
//! nullability × geometric relation × magnitude.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Shape wrapper
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Shape {
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
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(b) => b as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
    }
    fn ty_name(&self) -> &'static str {
        match self {
            Shape::Circle(_) => "CIRCLE",
            Shape::Aabb(_) => "AABB",
            Shape::Capsule(_) => "CAPSULE",
        }
    }
}

/// Build a shape of the requested type centred on `at` with size scale `sc`.
fn shape_of(r: &Rng, ty: c_int, at: c2v, sc: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: at, r: r.unit() * sc }),
        C2_TYPE_AABB => {
            let hw = r.unit() * sc;
            let hh = r.unit() * sc;
            Shape::Aabb(c2AABB {
                min: c2v { x: at.x - hw, y: at.y - hh },
                max: c2v { x: at.x + hw, y: at.y + hh },
            })
        }
        _ => Shape::Capsule(c2Capsule {
            a: c2v { x: at.x - r.sym(sc), y: at.y - r.sym(sc) },
            b: c2v { x: at.x + r.sym(sc), y: at.y + r.sym(sc) },
            r: r.unit() * sc * 0.5,
        }),
    }
}

/// Degenerate variants (row 73).
fn degenerate_of(r: &Rng, ty: c_int, at: c2v) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: at, r: 0.0 }),
        C2_TYPE_AABB => Shape::Aabb(c2AABB { min: at, max: at }),
        _ => {
            if r.below(2) == 0 {
                Shape::Capsule(c2Capsule { a: at, b: at, r: r.unit() * 5.0 })
            } else {
                Shape::Capsule(c2Capsule { a: at, b: c2v { x: at.x + 10.0, y: at.y }, r: 0.0 })
            }
        }
    }
}

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Number of vertices `c2MakeProxy` writes for a shape type. A hand-made GJK
/// cache may only reference indices `0..vert_count`, because the C leaves the
/// remaining `c2Proxy::verts` slots **uninitialised** (see ERRORS.md row 59) —
/// reading them is unspecified behaviour that no translation can reproduce.
fn vert_count(ty: c_int) -> u32 {
    match ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum XKind {
    Null,
    Identity,
    Translate,
    Rotate,
    RotTrans,
    Unnormalized,
}

fn make_x(r: &Rng, k: XKind) -> Option<c2x> {
    match k {
        XKind::Null => None,
        XKind::Identity => Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }),
        XKind::Translate => Some(c2x { p: r.geo_v(), r: c2r { c: 1.0, s: 0.0 } }),
        XKind::Rotate => Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: r.rot() }),
        XKind::RotTrans => Some(c2x { p: r.geo_v(), r: r.rot() }),
        XKind::Unnormalized => Some(c2x { p: r.geo_v(), r: c2r { c: r.sym(3.0), s: r.sym(3.0) } }),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CacheMode {
    None,
    Zeroed,
    Warm,
    Handmade(c_int),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum OutMode {
    All,
    NoA,
    NoB,
    NoIter,
    NoneAtAll,
}

#[derive(Default)]
struct Stats {
    hit: usize,
    nohit: usize,
    radius_shrink: usize,
    radius_midpoint: usize,
    iters: [usize; 21],
}

#[allow(clippy::too_many_arguments)]
fn run_case(
    d: &mut Diff,
    st: &mut Stats,
    r: &Rng,
    A: &Shape,
    B: &Shape,
    xa: Option<c2x>,
    xb: Option<c2x>,
    use_radius: c_int,
    cm: CacheMode,
    om: OutMode,
    label: &str,
) {
    let a = api();
    let ap: *const c2x = xa.as_ref().map(|v| v as *const c2x).unwrap_or(std::ptr::null());
    let bp: *const c2x = xb.as_ref().map(|v| v as *const c2x).unwrap_or(std::ptr::null());

    // Cache setup — the *same* starting image for both libraries.
    let mut start_cache: Option<c2GJKCache> = match cm {
        CacheMode::None => None,
        CacheMode::Zeroed => Some(c2GJKCache::default()),
        CacheMode::Warm => Some(c2GJKCache::default()),
        CacheMode::Handmade(n) => {
            let (na, nb) = (vert_count(A.ty()), vert_count(B.ty()));
            let ia = |r: &Rng| (r.below(na)) as c_int;
            let ib = |r: &Rng| (r.below(nb)) as c_int;
            Some(c2GJKCache {
                metric: match r.below(4) {
                    0 => 0.0,
                    1 => r.sym(100.0),
                    2 => -1.0e9,
                    _ => r.sym(1.0),
                },
                count: n,
                iA: [ia(r), ia(r), ia(r)],
                iB: [ib(r), ib(r), ib(r)],
                div: match r.below(4) {
                    0 => 1.0,
                    1 => 0.0,
                    _ => r.unit() * 10.0 + 0.5,
                },
            })
        }
    };

    // Warm mode: prime the cache with a first (already-verified) call so that
    // the measured call actually takes the `cache_was_read` path.
    if cm == CacheMode::Warm {
        let mut cc = start_cache.unwrap();
        let mut rc = cc;
        let (cv, rv) = unsafe {
            (
                (a.c2GJK.0)(A.ptr(), A.ty(), ap, B.ptr(), B.ty(), bp, std::ptr::null_mut(), std::ptr::null_mut(), use_radius, std::ptr::null_mut(), &mut cc),
                (a.c2GJK.1)(A.ptr(), A.ty(), ap, B.ptr(), B.ty(), bp, std::ptr::null_mut(), std::ptr::null_mut(), use_radius, std::ptr::null_mut(), &mut rc),
            )
        };
        d.check(feq(cv, rv) && cacheeq(&cc, &rc), || {
            format!("[{label}] cache-priming call diverged: C={} R={}\n C cache={cc:?}\n R cache={rc:?}", fmt_f(cv), fmt_f(rv))
        });
        start_cache = Some(cc);
    }

    let mut c_cache = start_cache;
    let mut r_cache = start_cache;

    let sentinel = c2v { x: -1234.5, y: 6789.0 };
    let (mut c_oa, mut c_ob) = (sentinel, sentinel);
    let (mut r_oa, mut r_ob) = (sentinel, sentinel);
    let mut c_it: c_int = -999;
    let mut r_it: c_int = -999;

    let (want_a, want_b, want_it) = match om {
        OutMode::All => (true, true, true),
        OutMode::NoA => (false, true, true),
        OutMode::NoB => (true, false, true),
        OutMode::NoIter => (true, true, false),
        OutMode::NoneAtAll => (false, false, false),
    };

    let cd = unsafe {
        (a.c2GJK.0)(
            A.ptr(), A.ty(), ap, B.ptr(), B.ty(), bp,
            if want_a { &mut c_oa } else { std::ptr::null_mut() },
            if want_b { &mut c_ob } else { std::ptr::null_mut() },
            use_radius,
            if want_it { &mut c_it } else { std::ptr::null_mut() },
            c_cache.as_mut().map(|c| c as *mut c2GJKCache).unwrap_or(std::ptr::null_mut()),
        )
    };
    let rd = unsafe {
        (a.c2GJK.1)(
            A.ptr(), A.ty(), ap, B.ptr(), B.ty(), bp,
            if want_a { &mut r_oa } else { std::ptr::null_mut() },
            if want_b { &mut r_ob } else { std::ptr::null_mut() },
            use_radius,
            if want_it { &mut r_it } else { std::ptr::null_mut() },
            r_cache.as_mut().map(|c| c as *mut c2GJKCache).unwrap_or(std::ptr::null_mut()),
        )
    };

    let ctx = || {
        format!(
            "[{label}] A={} {A:?} B={} {B:?}\n  xa={xa:?} xb={xb:?} use_radius={use_radius} cache={cm:?} out={om:?}\n  \
             dist  C={} R={}\n  outA  C={} R={}\n  outB  C={} R={}\n  iter  C={c_it} R={r_it}\n  cache C={c_cache:?}\n  cache R={r_cache:?}",
            A.ty_name(), B.ty_name(),
            fmt_f(cd), fmt_f(rd), fmt_v(c_oa), fmt_v(r_oa), fmt_v(c_ob), fmt_v(r_ob)
        )
    };

    d.check(feq(cd, rd), ctx);
    d.check(veq(c_oa, r_oa), ctx);
    d.check(veq(c_ob, r_ob), ctx);
    d.check(c_it == r_it, ctx);
    match (&c_cache, &r_cache) {
        (Some(c), Some(rr)) => d.check(cacheeq(c, rr), ctx),
        (None, None) => {}
        _ => d.check(false, ctx),
    }

    // Sanity: unwritten out-params must keep the sentinel in BOTH builds.
    if !want_a {
        d.check(veq(c_oa, sentinel) && veq(r_oa, sentinel), ctx);
    }
    if !want_it {
        d.check(c_it == -999 && r_it == -999, ctx);
    }

    // Coverage bookkeeping.
    if cd == 0.0 {
        st.hit += 1;
    } else {
        st.nohit += 1;
    }
    if use_radius != 0 && cd > 0.0 {
        st.radius_shrink += 1;
    }
    if use_radius != 0 && cd == 0.0 {
        st.radius_midpoint += 1;
    }
    if want_it && (0..=20).contains(&c_it) {
        st.iters[c_it as usize] += 1;
    }
}

// ---------------------------------------------------------------------------
// rows 48-57: every type pair x use_radius, no transforms, no cache
// ---------------------------------------------------------------------------

#[test]
fn row48_57_type_pairs_x_use_radius() {
    let a = api();
    let _ = a;
    let mut d = Diff::new("rows 48-57: c2GJK all 9 type pairs x use_radius in {0,1}");
    let mut st = Stats::default();
    let r = Rng::new(4857);
    for &ta in &TYPES {
        for &tb in &TYPES {
            for &ur in &[0i32, 1] {
                for _ in 0..300 {
                    // Draw centres from a range that produces disjoint, touching,
                    // overlapping and contained relations.
                    let ca = c2v { x: r.sym(60.0), y: r.sym(60.0) };
                    let cb = c2v { x: ca.x + r.sym(40.0), y: ca.y + r.sym(40.0) };
                    let A = shape_of(&r, ta, ca, 25.0);
                    let B = shape_of(&r, tb, cb, 25.0);
                    run_case(
                        &mut d, &mut st, &r, &A, &B, None, None, ur,
                        CacheMode::None, OutMode::All, "rows48-57",
                    );
                }
            }
        }
    }
    eprintln!("rows48-57 stats: hit={} nohit={} shrink={} midpoint={}", st.hit, st.nohit, st.radius_shrink, st.radius_midpoint);
    assert!(st.hit > 0 && st.nohit > 0, "did not cover both the hit and the separated paths: {st:?}", st = (st.hit, st.nohit));
    assert!(st.radius_shrink > 0 && st.radius_midpoint > 0, "use_radius branch coverage incomplete");
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 58-62: transform kinds
// ---------------------------------------------------------------------------

#[test]
fn row58_62_transforms() {
    let mut d = Diff::new("rows 58-62: c2GJK x transform kind (null/identity/translate/rotate/rot+trans/unnormalized)");
    let mut st = Stats::default();
    let r = Rng::new(5862);
    let kinds = [XKind::Null, XKind::Identity, XKind::Translate, XKind::Rotate, XKind::RotTrans, XKind::Unnormalized];
    for &ka in &kinds {
        for &kb in &kinds {
            for &ta in &TYPES {
                for &tb in &TYPES {
                    for _ in 0..12 {
                        let A = shape_of(&r, ta, r.geo_v(), 25.0);
                        let B = shape_of(&r, tb, r.geo_v(), 25.0);
                        let xa = make_x(&r, ka);
                        let xb = make_x(&r, kb);
                        let ur = (r.below(2)) as c_int;
                        run_case(&mut d, &mut st, &r, &A, &B, xa, xb, ur, CacheMode::None, OutMode::All, "rows58-62");
                    }
                }
            }
        }
    }
    d.finish();
}

/// Row 58 specifically: `NULL` transform and an explicitly-passed identity
/// transform must give *identical* results in both libraries.
#[test]
fn row58_null_equals_identity() {
    let a = api();
    let mut d = Diff::new("row 58: NULL transform == explicit identity transform (in both libs)");
    let r = Rng::new(58);
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    for &ta in &TYPES {
        for &tb in &TYPES {
            for _ in 0..200 {
                let A = shape_of(&r, ta, r.geo_v(), 25.0);
                let B = shape_of(&r, tb, r.geo_v(), 25.0);
                for lib in 0..2 {
                    let f = if lib == 0 { a.c2GJK.0 } else { a.c2GJK.1 };
                    let (mut n1, mut n2) = (c2v::default(), c2v::default());
                    let (mut i1, mut i2) = (c2v::default(), c2v::default());
                    let d1 = unsafe {
                        f(A.ptr(), A.ty(), std::ptr::null(), B.ptr(), B.ty(), std::ptr::null(), &mut n1, &mut n2, 1, std::ptr::null_mut(), std::ptr::null_mut())
                    };
                    let d2 = unsafe {
                        f(A.ptr(), A.ty(), &ident, B.ptr(), B.ty(), &ident, &mut i1, &mut i2, 1, std::ptr::null_mut(), std::ptr::null_mut())
                    };
                    d.check(feq(d1, d2) && veq(n1, i1) && veq(n2, i2), || {
                        format!("lib{lib}: NULL xform != identity xform: {} vs {}", fmt_f(d1), fmt_f(d2))
                    });
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 63-67: cache modes
// ---------------------------------------------------------------------------

#[test]
fn row63_67_cache_modes() {
    let mut d = Diff::new("rows 63-67: c2GJK x cache mode (none/zeroed/warm/handmade count=1,2,3)");
    let mut st = Stats::default();
    let r = Rng::new(6367);
    let modes = [
        CacheMode::None,
        CacheMode::Zeroed,
        CacheMode::Warm,
        CacheMode::Handmade(1),
        CacheMode::Handmade(2),
        CacheMode::Handmade(3),
    ];
    for &cm in &modes {
        for &ta in &TYPES {
            for &tb in &TYPES {
                for _ in 0..120 {
                    let A = shape_of(&r, ta, c2v { x: r.sym(60.0), y: r.sym(60.0) }, 25.0);
                    let B = shape_of(&r, tb, c2v { x: r.sym(60.0), y: r.sym(60.0) }, 25.0);
                    let xa = make_x(&r, if r.below(2) == 0 { XKind::Null } else { XKind::RotTrans });
                    let xb = make_x(&r, if r.below(2) == 0 { XKind::Null } else { XKind::RotTrans });
                    let ur = (r.below(2)) as c_int;
                    run_case(&mut d, &mut st, &r, &A, &B, xa, xb, ur, cm, OutMode::All, "rows63-67");
                }
            }
        }
    }
    d.finish();
}

/// Row 66: warm cache, then MOVE the shapes so the cached indices are stale.
#[test]
fn row66_stale_warm_cache() {
    let a = api();
    let mut d = Diff::new("row 66: warm cache reused after the shapes have moved");
    let r = Rng::new(66);
    for &ta in &TYPES {
        for &tb in &TYPES {
            for _ in 0..250 {
                let A1 = shape_of(&r, ta, r.geo_v(), 25.0);
                let B1 = shape_of(&r, tb, r.geo_v(), 25.0);
                let A2 = shape_of(&r, ta, r.geo_v(), 25.0);
                let B2 = shape_of(&r, tb, r.geo_v(), 25.0);
                let mut cc = c2GJKCache::default();
                let mut rc = c2GJKCache::default();
                for (i, (A, B)) in [(A1, B1), (A2, B2), (A1, B2), (A2, B1)].iter().enumerate() {
                    let (mut coa, mut cob) = (c2v::default(), c2v::default());
                    let (mut roa, mut rob) = (c2v::default(), c2v::default());
                    let (mut ci, mut ri) = (0 as c_int, 0 as c_int);
                    let cd = unsafe {
                        (a.c2GJK.0)(A.ptr(), A.ty(), std::ptr::null(), B.ptr(), B.ty(), std::ptr::null(), &mut coa, &mut cob, 1, &mut ci, &mut cc)
                    };
                    let rd = unsafe {
                        (a.c2GJK.1)(A.ptr(), A.ty(), std::ptr::null(), B.ptr(), B.ty(), std::ptr::null(), &mut roa, &mut rob, 1, &mut ri, &mut rc)
                    };
                    d.check(feq(cd, rd) && veq(coa, roa) && veq(cob, rob) && ci == ri && cacheeq(&cc, &rc), || {
                        format!(
                            "stale-cache step {i}: A={A:?} B={B:?}\n dist C={} R={}\n outA C={} R={}\n outB C={} R={}\n iter C={ci} R={ri}\n cache C={cc:?}\n cache R={rc:?}",
                            fmt_f(cd), fmt_f(rd), fmt_v(coa), fmt_v(roa), fmt_v(cob), fmt_v(rob)
                        )
                    });
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// row 68: out-pointer nullability
// ---------------------------------------------------------------------------

#[test]
fn row68_out_pointer_modes() {
    let mut d = Diff::new("row 68: c2GJK x out-pointer nullability");
    let mut st = Stats::default();
    let r = Rng::new(68);
    for om in [OutMode::All, OutMode::NoA, OutMode::NoB, OutMode::NoIter, OutMode::NoneAtAll] {
        for &ta in &TYPES {
            for &tb in &TYPES {
                for _ in 0..80 {
                    let A = shape_of(&r, ta, r.geo_v(), 25.0);
                    let B = shape_of(&r, tb, r.geo_v(), 25.0);
                    let ur = (r.below(2)) as c_int;
                    let cm = if r.below(2) == 0 { CacheMode::None } else { CacheMode::Zeroed };
                    run_case(&mut d, &mut st, &r, &A, &B, None, None, ur, cm, om, "row68");
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 69-72: geometric relations
// ---------------------------------------------------------------------------

#[test]
fn row69_72_geometric_relations() {
    let mut d = Diff::new("rows 69-72: overlapping / touching / contained / identical, all type pairs");
    let mut st = Stats::default();
    let r = Rng::new(6972);
    for &ta in &TYPES {
        for &tb in &TYPES {
            for rel in 0..4 {
                for _ in 0..200 {
                    let c = c2v { x: r.sym(40.0), y: r.sym(40.0) };
                    let (A, B) = match rel {
                        // row 69: heavily overlapping (same centre, big shapes)
                        0 => (shape_of(&r, ta, c, 30.0), shape_of(&r, tb, c, 30.0)),
                        // row 70: exactly touching along +x
                        1 => {
                            let A = shape_of(&r, ta, c, 10.0);
                            let ext = match A {
                                Shape::Circle(cc) => cc.r,
                                Shape::Aabb(bb) => bb.max.x - c.x,
                                Shape::Capsule(cp) => {
                                    let m = if cp.a.x > cp.b.x { cp.a.x } else { cp.b.x };
                                    m - c.x + cp.r
                                }
                            };
                            let B = match tb {
                                C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: c2v { x: c.x + ext + 5.0, y: c.y }, r: 5.0 }),
                                C2_TYPE_AABB => Shape::Aabb(c2AABB {
                                    min: c2v { x: c.x + ext, y: c.y - 5.0 },
                                    max: c2v { x: c.x + ext + 10.0, y: c.y + 5.0 },
                                }),
                                _ => Shape::Capsule(c2Capsule {
                                    a: c2v { x: c.x + ext + 3.0, y: c.y - 5.0 },
                                    b: c2v { x: c.x + ext + 3.0, y: c.y + 5.0 },
                                    r: 3.0,
                                }),
                            };
                            (A, B)
                        }
                        // row 71: B fully inside A
                        2 => (shape_of(&r, ta, c, 60.0), shape_of(&r, tb, c, 1.0)),
                        // row 72: identical shape objects
                        _ => {
                            let A = shape_of(&r, ta, c, 20.0);
                            let B = if ta == tb { A } else { shape_of(&r, tb, c, 20.0) };
                            (A, B)
                        }
                    };
                    for &ur in &[0i32, 1] {
                        run_case(&mut d, &mut st, &r, &A, &B, None, None, ur, CacheMode::Zeroed, OutMode::All, "rows69-72");
                    }
                }
            }
        }
    }
    eprintln!("rows69-72 stats: hit={} nohit={}", st.hit, st.nohit);
    d.finish();
}

// ---------------------------------------------------------------------------
// row 73: degenerate shapes
// ---------------------------------------------------------------------------

#[test]
fn row73_degenerate_shapes() {
    let mut d = Diff::new("row 73: degenerate shapes (r==0 circle, min==max AABB, a==b capsule, r==0 capsule)");
    let mut st = Stats::default();
    let r = Rng::new(73);
    for &ta in &TYPES {
        for &tb in &TYPES {
            for _ in 0..400 {
                let A = degenerate_of(&r, ta, r.geo_v());
                let B = if r.below(2) == 0 {
                    degenerate_of(&r, tb, r.geo_v())
                } else {
                    shape_of(&r, tb, r.geo_v(), 20.0)
                };
                for &ur in &[0i32, 1] {
                    run_case(&mut d, &mut st, &r, &A, &B, None, None, ur, CacheMode::Zeroed, OutMode::All, "row73");
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 74-76: extreme magnitudes
// ---------------------------------------------------------------------------

#[test]
fn row74_76_magnitudes() {
    let mut d = Diff::new("rows 74-76: far apart (~1e6), huge coordinates (~1e18), tiny shapes");
    let mut st = Stats::default();
    let r = Rng::new(7476);
    for &ta in &TYPES {
        for &tb in &TYPES {
            for band in 0..4 {
                let scale = match band {
                    0 => 1.0e6f32,
                    1 => 1.0e18f32,
                    2 => 1.0e-20f32,
                    _ => 1.0e30f32,
                };
                for _ in 0..120 {
                    let ca = c2v { x: r.sym(1.0) * scale, y: r.sym(1.0) * scale };
                    let cb = c2v { x: r.sym(1.0) * scale, y: r.sym(1.0) * scale };
                    let A = shape_of(&r, ta, ca, scale * 0.1);
                    let B = shape_of(&r, tb, cb, scale * 0.1);
                    for &ur in &[0i32, 1] {
                        run_case(&mut d, &mut st, &r, &A, &B, None, None, ur, CacheMode::Zeroed, OutMode::All, "rows74-76");
                    }
                }
            }
        }
    }
    d.finish();
}

/// Row 75 specifically: check that the reported iteration count agrees and that
/// multi-iteration cases are actually exercised.
#[test]
fn row75_iteration_counts() {
    let mut d = Diff::new("row 75: iteration counts agree, multi-iteration configurations reached");
    let mut st = Stats::default();
    let r = Rng::new(75);
    for _ in 0..3000 {
        let A = shape_of(&r, C2_TYPE_AABB, r.geo_v(), 30.0);
        let B = shape_of(&r, C2_TYPE_AABB, r.geo_v(), 30.0);
        let xa = make_x(&r, XKind::RotTrans);
        let xb = make_x(&r, XKind::RotTrans);
        run_case(&mut d, &mut st, &r, &A, &B, xa, xb, 1, CacheMode::None, OutMode::All, "row75");
    }
    for _ in 0..1500 {
        let A = shape_of(&r, C2_TYPE_CAPSULE, r.geo_v(), 30.0);
        let B = shape_of(&r, C2_TYPE_AABB, r.geo_v(), 30.0);
        let xa = make_x(&r, XKind::Rotate);
        run_case(&mut d, &mut st, &r, &A, &B, xa, None, 0, CacheMode::None, OutMode::All, "row75");
    }
    eprintln!("row75 iteration histogram: {:?}", st.iters);
    let multi: usize = st.iters[2..].iter().sum();
    assert!(st.iters[0] > 0 && st.iters[1] > 0 && multi > 0, "iteration coverage too narrow: {:?}", st.iters);
    d.finish();
}

// ---------------------------------------------------------------------------
// row 77: use_radius values other than 0/1
// ---------------------------------------------------------------------------

#[test]
fn row77_use_radius_truthiness() {
    let a = api();
    let mut d = Diff::new("row 77: any nonzero use_radius behaves like 1, in both libs");
    let mut st = Stats::default();
    let r = Rng::new(77);
    for &ur in &[2i32, -1, i32::MIN, i32::MAX, 0x1000] {
        for &ta in &TYPES {
            for &tb in &TYPES {
                for _ in 0..80 {
                    let A = shape_of(&r, ta, r.geo_v(), 25.0);
                    let B = shape_of(&r, tb, r.geo_v(), 25.0);
                    run_case(&mut d, &mut st, &r, &A, &B, None, None, ur, CacheMode::Zeroed, OutMode::All, "row77");
                    // and it must equal use_radius == 1 in each library separately
                    for lib in 0..2 {
                        let f = if lib == 0 { a.c2GJK.0 } else { a.c2GJK.1 };
                        let d_ur = unsafe {
                            f(A.ptr(), A.ty(), std::ptr::null(), B.ptr(), B.ty(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), ur, std::ptr::null_mut(), std::ptr::null_mut())
                        };
                        let d_1 = unsafe {
                            f(A.ptr(), A.ty(), std::ptr::null(), B.ptr(), B.ty(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut())
                        };
                        d.check(feq(d_ur, d_1), || format!("lib{lib}: use_radius={ur} gave {} but 1 gave {}", fmt_f(d_ur), fmt_f(d_1)));
                    }
                }
            }
        }
    }
    d.finish();
}
