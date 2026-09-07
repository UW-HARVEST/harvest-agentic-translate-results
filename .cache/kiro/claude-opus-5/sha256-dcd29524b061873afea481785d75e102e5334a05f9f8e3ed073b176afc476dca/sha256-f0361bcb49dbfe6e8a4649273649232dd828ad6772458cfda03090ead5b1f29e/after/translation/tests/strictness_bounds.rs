//! Bounds the one comparison exemption the suite uses.
//!
//! `common::feq` treats "both NaN" as equal instead of requiring identical bit
//! patterns. That exemption is only legitimate if it never hides a *real*
//! difference, so this file asserts, over a large sweep, that:
//!
//!   1. every strict-bit difference between the two libraries has BOTH sides
//!      NaN (never NaN vs a number, never differing infinities or finite
//!      values, never a signed-zero mismatch), and
//!   2. such differences occur only for inputs that already contain NaN/inf.
//!
//! Justification that NaN sign is not part of the C's contract: the C library
//! diverges *from itself* here. Built at `-O0` (the reference CMake build) and
//! at `-O2`, `gjk` returns different NaN sign bits for the same input, because
//! SSE propagates whichever NaN operand the compiler happened to place first.
//! IEEE-754 leaves the sign and payload of a produced NaN unspecified.

mod common;
use common::*;

/// Strict bit comparison, deliberately *not* using `feq`.
fn bits_eq(a: c2v, b: c2v) -> bool {
    a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
}

#[test]
fn every_strict_bit_difference_is_nan_only() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0x5A17);
    let mut total = 0usize;
    let mut diffs = 0usize;
    let mut nan_input = 0usize;

    for i in 0..200_000 {
        let rev = (rng.next_u32() & 1) as i8;
        let p = [
            rng.wild(), rng.wild(), rng.wild(), rng.wild(),
            rng.wild(), rng.wild(), rng.wild(), rng.wild(),
            rng.wild(),
        ];
        let (mut ca, mut cb) = (c2v::default(), c2v::default());
        let (mut ra, mut rb) = (c2v::default(), c2v::default());
        unsafe {
            (c.gjk)(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            (r.gjk)(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        }
        total += 1;
        for (which, (cv, rv)) in [("outA", (ca, ra)), ("outB", (cb, rb))] {
            if bits_eq(cv, rv) {
                continue;
            }
            diffs += 1;
            // (1) both sides must be NaN in the differing component
            for (comp, (x, y)) in [("x", (cv.x, rv.x)), ("y", (cv.y, rv.y))] {
                if x.to_bits() == y.to_bits() {
                    continue;
                }
                assert!(
                    x.is_nan() && y.is_nan(),
                    "#{i} {which}.{comp}: strict-bit difference that is NOT NaN-only: \
                     C={x:?} (0x{:08x}) vs Rust={y:?} (0x{:08x}); inputs rev={rev} p={p:?}",
                    x.to_bits(),
                    y.to_bits()
                );
                // and only the sign / payload may differ, never NaN-ness
                assert_eq!(
                    x.is_nan(),
                    y.is_nan(),
                    "#{i} {which}.{comp}: NaN-ness differs"
                );
            }
            // (2) the input must itself contain a NaN or an infinity
            assert!(
                p.iter().any(|v| v.is_nan() || v.is_infinite()),
                "#{i} {which}: NaN divergence from an all-finite input: p={p:?}"
            );
            nan_input += 1;
        }
    }
    eprintln!(
        "strict-bit sweep: {total} calls, {diffs} differing out-vectors \
         (all NaN-only, all from NaN/inf inputs: {nan_input})"
    );
    assert_eq!(diffs, nan_input, "some divergence was not attributed");
}

/// The same bound for `c2GJK` driven directly, including transforms and caches,
/// where far more of the pipeline is in play.
#[test]
fn c2gjk_strict_bit_differences_are_nan_only() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0x5A18);
    let mut diffs = 0usize;
    let mut finite_only_calls = 0usize;

    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for i in 0..8000 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                let ax = if i % 3 == 0 { Some(rng.unit_x()) } else { None };
                let bx = if i % 4 == 0 { Some(rng.wild_x()) } else { None };
                let ur = (i % 2) as i32;
                let cache = if i % 5 == 0 {
                    Some(c2GJKCache::default())
                } else {
                    None
                };
                let co = call_gjk(c, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, cache);
                let ro = call_gjk(r, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, cache);

                // `dist` must match bit-for-bit unless both are NaN
                if co.dist.to_bits() != ro.dist.to_bits() {
                    assert!(
                        co.dist.is_nan() && ro.dist.is_nan(),
                        "c2GJK dist strict difference that is not NaN-only: \
                         C={:?} vs Rust={:?}",
                        co.dist,
                        ro.dist
                    );
                    diffs += 1;
                }
                for (which, (cv, rv)) in [("outA", (co.a, ro.a)), ("outB", (co.b, ro.b))] {
                    if !bits_eq(cv, rv) {
                        for (x, y) in [(cv.x, rv.x), (cv.y, rv.y)] {
                            if x.to_bits() != y.to_bits() {
                                assert!(
                                    x.is_nan() && y.is_nan(),
                                    "c2GJK {which} strict difference that is not NaN-only: \
                                     C={x:?} (0x{:08x}) vs Rust={y:?} (0x{:08x})",
                                    x.to_bits(),
                                    y.to_bits()
                                );
                            }
                        }
                        diffs += 1;
                    }
                }
                // integer outputs and the cache must ALWAYS match exactly —
                // no exemption applies to them
                assert_eq!(co.iters, ro.iters, "c2GJK iterations must match exactly");
                assert_eq!(
                    co.cache.count, ro.cache.count,
                    "c2GJK cache count must match exactly"
                );
                assert_eq!(co.cache.iA, ro.cache.iA, "c2GJK cache iA must match exactly");
                assert_eq!(co.cache.iB, ro.cache.iB, "c2GJK cache iB must match exactly");
                if !co.dist.is_nan() && !ro.dist.is_nan() {
                    finite_only_calls += 1;
                }
            }
        }
    }
    eprintln!(
        "c2GJK strict-bit sweep: {diffs} NaN-only differences; \
         {finite_only_calls} calls with non-NaN dist all matched exactly"
    );
}

/// Every *exported leaf* function: strict bit equality is required for all
/// non-NaN results, and any difference at all must have BOTH sides NaN. The
/// count of NaN-sign-only differences per function is reported so the exemption
/// stays visible and bounded rather than blanket-applied.
#[test]
fn exported_leaf_functions_differences_are_nan_sign_only() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0x5A19);
    let names = [
        "c2V", "c2Mulvs", "c2Maxv", "c2Minv", "c2Clampv", "c2Sub", "c2Add", "c2Neg",
        "c2Skew", "c2CCW90", "c2Div", "c2Norm", "c2Mulrv", "c2MulrvT", "c2Mulxv",
        "c2Dot", "c2Det2", "c2Len",
    ];
    let mut nan_only = [0usize; 18];
    let mut calls = 0usize;

    for i in 0..50_000 {
        let a = rng.wild_v();
        let b = rng.wild_v();
        let s = rng.wild();
        let rot = rng.wild_r();
        let x = rng.wild_x();

        // (index, C result, Rust result) for every vector-returning leaf
        let vpairs: [(usize, c2v, c2v); 15] = [
            (0, (c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y)),
            (1, (c.c2Mulvs)(a, s), (r.c2Mulvs)(a, s)),
            (2, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b)),
            (3, (c.c2Minv)(a, b), (r.c2Minv)(a, b)),
            (4, (c.c2Clampv)(a, b, c2v { x: s, y: s }), (r.c2Clampv)(a, b, c2v { x: s, y: s })),
            (5, (c.c2Sub)(a, b), (r.c2Sub)(a, b)),
            (6, (c.c2Add)(a, b), (r.c2Add)(a, b)),
            (7, (c.c2Neg)(a), (r.c2Neg)(a)),
            (8, (c.c2Skew)(a), (r.c2Skew)(a)),
            (9, (c.c2CCW90)(a), (r.c2CCW90)(a)),
            (10, (c.c2Div)(a, s), (r.c2Div)(a, s)),
            (11, (c.c2Norm)(a), (r.c2Norm)(a)),
            (12, (c.c2Mulrv)(rot, a), (r.c2Mulrv)(rot, a)),
            (13, (c.c2MulrvT)(rot, a), (r.c2MulrvT)(rot, a)),
            (14, (c.c2Mulxv)(x, a), (r.c2Mulxv)(x, a)),
        ];
        for (idx, cv, rv) in vpairs {
            calls += 1;
            if bits_eq(cv, rv) {
                continue;
            }
            for (comp, (p, q)) in [("x", (cv.x, rv.x)), ("y", (cv.y, rv.y))] {
                if p.to_bits() == q.to_bits() {
                    continue;
                }
                assert!(
                    p.is_nan() && q.is_nan(),
                    "{} #{i}.{comp}: NON-NaN strict difference: C=0x{:08x} ({p:?}) vs Rust=0x{:08x} ({q:?})",
                    names[idx],
                    p.to_bits(),
                    q.to_bits()
                );
            }
            nan_only[idx] += 1;
        }

        let fpairs: [(usize, f32, f32); 3] = [
            (15, (c.c2Dot)(a, b), (r.c2Dot)(a, b)),
            (16, (c.c2Det2)(a, b), (r.c2Det2)(a, b)),
            (17, (c.c2Len)(a), (r.c2Len)(a)),
        ];
        for (idx, cv, rv) in fpairs {
            calls += 1;
            if cv.to_bits() == rv.to_bits() {
                continue;
            }
            assert!(
                cv.is_nan() && rv.is_nan(),
                "{} #{i}: NON-NaN strict difference: C=0x{:08x} ({cv:?}) vs Rust=0x{:08x} ({rv:?})",
                names[idx],
                cv.to_bits(),
                rv.to_bits()
            );
            nan_only[idx] += 1;
        }
    }
    let report: Vec<String> = names
        .iter()
        .zip(nan_only.iter())
        .filter(|&(_, &n)| n > 0)
        .map(|(n, c)| format!("{n}={c}"))
        .collect();
    eprintln!(
        "leaf functions: {calls} calls; NaN-sign-only differences: [{}]; \
         every non-NaN result bit-identical",
        report.join(", ")
    );
    // No stable *set* of affected functions exists: which ones differ depends on
    // the optimization level of BOTH libraries, because SSE propagates whichever
    // NaN operand the compiler placed in the destination register. Measured:
    //   Rust release: c2Add, c2Mulrv, c2MulrvT, c2Mulxv, c2Dot, c2Det2
    //   Rust debug:   c2Mulvs, c2Div, c2Norm, c2Mulrv, c2MulrvT, c2Mulxv,
    //                 c2Dot, c2Det2, c2Len
    // Even single-operation leaves are affected when *both* operands are NaN.
    // The guarantee asserted above — every difference has BOTH sides NaN, and
    // every non-NaN result is bit-identical — is therefore the strongest
    // invariant that is actually a property of the C rather than of GCC's
    // register allocation. `normal_range_inputs_are_bit_identical_with_no_exemption`
    // covers the no-exemption case.
    assert!(
        calls > 800_000,
        "leaf sweep did not run the expected number of comparisons ({calls})"
    );
}

/// The guarantee that actually matters: over inputs drawn only from normal,
/// moderate-magnitude floats — no NaN, no infinity, no subnormal, nothing near
/// `FLT_MAX` — the two libraries are bit-identical with **no exemption at all**,
/// for every exported function.
#[test]
fn normal_range_inputs_are_bit_identical_with_no_exemption() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0x5A1B);
    let mut checks = 0usize;

    macro_rules! strict_v {
        ($name:literal, $cv:expr, $rv:expr) => {{
            let (cv, rv) = ($cv, $rv);
            checks += 1;
            assert!(
                cv.x.to_bits() == rv.x.to_bits() && cv.y.to_bits() == rv.y.to_bits(),
                concat!($name, ": C=(0x{:08x},0x{:08x}) != Rust=(0x{:08x},0x{:08x})"),
                cv.x.to_bits(), cv.y.to_bits(), rv.x.to_bits(), rv.y.to_bits()
            );
        }};
    }
    macro_rules! strict_f {
        ($name:literal, $cv:expr, $rv:expr) => {{
            let (cv, rv) = ($cv, $rv);
            checks += 1;
            assert_eq!(cv.to_bits(), rv.to_bits(), concat!($name, ": {:?} != {:?}"), cv, rv);
        }};
    }

    for _ in 0..40_000 {
        // normal, moderate magnitudes only
        let nv = |rng: &mut Rng| c2v { x: rng.sym(1000.0), y: rng.sym(1000.0) };
        let (a, b) = (nv(&mut rng), nv(&mut rng));
        let s = rng.sym(1000.0);
        let rot = rng.unit_r();
        let x = rng.unit_x();

        strict_v!("c2V", (c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y));
        strict_v!("c2Mulvs", (c.c2Mulvs)(a, s), (r.c2Mulvs)(a, s));
        strict_v!("c2Maxv", (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
        strict_v!("c2Minv", (c.c2Minv)(a, b), (r.c2Minv)(a, b));
        let hi = nv(&mut rng);
        strict_v!("c2Clampv", (c.c2Clampv)(a, b, hi), (r.c2Clampv)(a, b, hi));
        strict_v!("c2Sub", (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        strict_v!("c2Add", (c.c2Add)(a, b), (r.c2Add)(a, b));
        strict_v!("c2Neg", (c.c2Neg)(a), (r.c2Neg)(a));
        strict_v!("c2Skew", (c.c2Skew)(a), (r.c2Skew)(a));
        strict_v!("c2CCW90", (c.c2CCW90)(a), (r.c2CCW90)(a));
        strict_v!("c2Div", (c.c2Div)(a, s), (r.c2Div)(a, s));
        strict_v!("c2Norm", (c.c2Norm)(a), (r.c2Norm)(a));
        strict_v!("c2Mulrv", (c.c2Mulrv)(rot, a), (r.c2Mulrv)(rot, a));
        strict_v!("c2MulrvT", (c.c2MulrvT)(rot, a), (r.c2MulrvT)(rot, a));
        strict_v!("c2Mulxv", (c.c2Mulxv)(x, a), (r.c2Mulxv)(x, a));
        strict_f!("c2Dot", (c.c2Dot)(a, b), (r.c2Dot)(a, b));
        strict_f!("c2Det2", (c.c2Det2)(a, b), (r.c2Det2)(a, b));
        strict_f!("c2Len", (c.c2Len)(a), (r.c2Len)(a));
    }

    // and the whole pipeline, end to end, over normal inputs
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for i in 0..4000 {
                let mk = |rng: &mut Rng, ty: i32| match ty {
                    C2_TYPE_CIRCLE => ShapeBuf::circle(c2Circle {
                        p: c2v { x: rng.sym(100.0), y: rng.sym(100.0) },
                        r: rng.unit() * 20.0,
                    }),
                    C2_TYPE_AABB => {
                        let p = c2v { x: rng.sym(100.0), y: rng.sym(100.0) };
                        let q = c2v { x: rng.sym(100.0), y: rng.sym(100.0) };
                        ShapeBuf::aabb(c2AABB {
                            min: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                            max: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                        })
                    }
                    _ => ShapeBuf::capsule(c2Capsule {
                        a: c2v { x: rng.sym(100.0), y: rng.sym(100.0) },
                        b: c2v { x: rng.sym(100.0), y: rng.sym(100.0) },
                        r: rng.unit() * 20.0,
                    }),
                };
                let a = mk(&mut rng, ta);
                let b = mk(&mut rng, tb);
                let ax = if i % 3 == 0 { Some(rng.unit_x()) } else { None };
                let bx = if i % 4 == 0 { Some(rng.unit_x()) } else { None };
                let ur = (i % 2) as i32;
                let cache = if i % 5 == 0 { Some(c2GJKCache::default()) } else { None };
                let co = call_gjk(c, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, cache);
                let ro = call_gjk(r, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, cache);
                checks += 1;
                assert_eq!(co.dist.to_bits(), ro.dist.to_bits(), "c2GJK dist (normal inputs)");
                assert!(bits_eq(co.a, ro.a), "c2GJK outA (normal inputs)");
                assert!(bits_eq(co.b, ro.b), "c2GJK outB (normal inputs)");
                assert_eq!(co.iters, ro.iters, "c2GJK iterations (normal inputs)");
                assert_eq!(co.cache.metric.to_bits(), ro.cache.metric.to_bits(), "cache metric");
                assert_eq!(co.cache.div.to_bits(), ro.cache.div.to_bits(), "cache div");
                assert_eq!(co.cache.count, ro.cache.count, "cache count");
                assert_eq!(co.cache.iA, ro.cache.iA, "cache iA");
                assert_eq!(co.cache.iB, ro.cache.iB, "cache iB");
            }
        }
    }

    // and `gjk` itself
    for _ in 0..40_000 {
        let p = [
            rng.sym(100.0), rng.sym(100.0), rng.sym(100.0), rng.sym(100.0),
            rng.sym(100.0), rng.sym(100.0), rng.sym(100.0), rng.sym(100.0),
            rng.unit() * 20.0,
        ];
        for rev in [0i8, 1] {
            let (mut ca, mut cb) = (c2v::default(), c2v::default());
            let (mut ra, mut rb) = (c2v::default(), c2v::default());
            unsafe {
                (c.gjk)(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                (r.gjk)(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            }
            checks += 1;
            assert!(bits_eq(ca, ra), "gjk outA (normal inputs) rev={rev} p={p:?}");
            assert!(bits_eq(cb, rb), "gjk outB (normal inputs) rev={rev} p={p:?}");
        }
    }

    eprintln!("normal-range sweep: {checks} strict comparisons, ZERO differences, no exemption used");
}

/// The simplex functions: identical for every non-NaN result, and any byte that
/// differs must be part of a NaN on both sides.
#[test]
fn exported_simplex_functions_differences_are_nan_sign_only() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0x5A1A);
    let mut nan_only = 0usize;
    let mut compared = 0usize;

    /// Compares two `c2Simplex` values field by field: `f32` fields may differ
    /// only when both are NaN; `int` fields must be exactly equal.
    fn simplex_nan_only(a: &c2Simplex, b: &c2Simplex, ctx: &str) -> bool {
        let mut any = false;
        for k in 0..4 {
            let (x, y) = (&a.verts[k], &b.verts[k]);
            assert_eq!(x.iA, y.iA, "{ctx} verts[{k}].iA");
            assert_eq!(x.iB, y.iB, "{ctx} verts[{k}].iB");
            for (name, (p, q)) in [
                ("sA.x", (x.sA.x, y.sA.x)), ("sA.y", (x.sA.y, y.sA.y)),
                ("sB.x", (x.sB.x, y.sB.x)), ("sB.y", (x.sB.y, y.sB.y)),
                ("p.x", (x.p.x, y.p.x)),    ("p.y", (x.p.y, y.p.y)),
                ("u", (x.u, y.u)),
            ] {
                if p.to_bits() != q.to_bits() {
                    assert!(
                        p.is_nan() && q.is_nan(),
                        "{ctx} verts[{k}].{name}: NON-NaN difference C=0x{:08x} ({p:?}) vs Rust=0x{:08x} ({q:?})",
                        p.to_bits(), q.to_bits()
                    );
                    any = true;
                }
            }
        }
        assert_eq!(a.count, b.count, "{ctx} count");
        if a.div.to_bits() != b.div.to_bits() {
            assert!(
                a.div.is_nan() && b.div.is_nan(),
                "{ctx} div: NON-NaN difference C={:?} vs Rust={:?}",
                a.div, b.div
            );
            any = true;
        }
        any
    }

    for count in [-1i32, 0, 1, 2, 3, 4, 9] {
        for i in 0..4000 {
            let d = rng.wild();
            let s = rand_simplex(&mut rng, count, d, true);

            let (mut cs, mut rs) = (s, s);
            let cm = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
            let rm = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
            compared += 1;
            if cm.to_bits() != rm.to_bits() {
                assert!(cm.is_nan() && rm.is_nan(), "c2GJKSimplexMetric count={count} #{i}: {cm:?} vs {rm:?}");
                nan_only += 1;
            }

            let (mut cs, mut rs) = (s, s);
            let cd = unsafe { (c.c2D)(&mut cs) };
            let rd = unsafe { (r.c2D)(&mut rs) };
            if !bits_eq(cd, rd) {
                for (p, q) in [(cd.x, rd.x), (cd.y, rd.y)] {
                    if p.to_bits() != q.to_bits() {
                        assert!(p.is_nan() && q.is_nan(), "c2D count={count} #{i}: {p:?} vs {q:?}");
                    }
                }
                nan_only += 1;
            }

            let (mut cs, mut rs) = (s, s);
            let cl = unsafe { (c.c2L)(&mut cs) };
            let rl = unsafe { (r.c2L)(&mut rs) };
            if !bits_eq(cl, rl) {
                for (p, q) in [(cl.x, rl.x), (cl.y, rl.y)] {
                    if p.to_bits() != q.to_bits() {
                        assert!(p.is_nan() && q.is_nan(), "c2L count={count} #{i}: {p:?} vs {q:?}");
                    }
                }
                nan_only += 1;
            }

            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (c2v::default(), c2v::default());
            let (mut ra, mut rb) = (c2v::default(), c2v::default());
            unsafe {
                (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (r.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            for (which, (cv, rv)) in [("outA", (ca, ra)), ("outB", (cb, rb))] {
                if !bits_eq(cv, rv) {
                    for (p, q) in [(cv.x, rv.x), (cv.y, rv.y)] {
                        if p.to_bits() != q.to_bits() {
                            assert!(
                                p.is_nan() && q.is_nan(),
                                "c2Witness {which} count={count} #{i}: {p:?} vs {q:?}"
                            );
                        }
                    }
                    nan_only += 1;
                }
            }

            for (name, f_c, f_r) in [("c22", c.c22, r.c22), ("c23", c.c23, r.c23)] {
                let (mut cs, mut rs) = (s, s);
                unsafe {
                    f_c(&mut cs);
                    f_r(&mut rs);
                }
                if simplex_nan_only(&cs, &rs, &format!("{name} count={count} #{i}")) {
                    nan_only += 1;
                }
            }
        }
    }
    eprintln!(
        "simplex functions: {compared} metric calls; {nan_only} NaN-sign-only differences; \
         every non-NaN field and every int field bit-identical"
    );
}
