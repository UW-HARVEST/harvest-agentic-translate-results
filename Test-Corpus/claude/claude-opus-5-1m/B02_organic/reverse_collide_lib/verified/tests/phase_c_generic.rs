//! Phase C, generic boundaries — `ERRORS.md` rows G1 … G6, plus a
//! per-symbol coverage gate that proves *every* one of the 38 exported symbols
//! is actually invoked differentially somewhere in the suite.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::collections::BTreeSet;
use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(C, align(8))]
#[derive(Copy, Clone)]
struct Buf([u8; 32]);

fn put<T: Copy>(v: &T) -> Buf {
    let mut b = Buf([0xA5; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

// ===========================================================================
// G1 — out-of-range `C2_TYPE` values across the FFI boundary.
//
// A C enum has the range of `int`, so every one of these is a real input.
// `c2MakeProxy` must stay a no-op and `c2Collided` must return 0 for all of
// them, for every combination of the two type parameters.
// ===========================================================================

#[test]
fn g1_out_of_range_enum_values() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x61);

    // A wide sample of int values with no valid variant, including the exact
    // one-past-the-end value, the wraparound values, and the extremes.
    let mut bad: Vec<c_int> = vec![
        3,
        4,
        -1,
        -2,
        255,
        256,
        0x1_0000,
        1 << 30,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX - 1,
        -0x8000_0000i64 as c_int,
    ];
    for _ in 0..64 {
        let v = rng.next_u32() as c_int;
        if !(0..=2).contains(&v) {
            bad.push(v);
        }
    }

    unsafe {
        for &t in bad.iter() {
            // c2MakeProxy: no-op for any invalid type, whatever the payload.
            for payload in 0..3 {
                let mut circle = rng.circle(10.0);
                let mut aabb = rng.aabb(10.0);
                let mut capsule = rng.capsule(10.0);
                let shape: *const c_void = match payload {
                    0 => &mut circle as *mut c2Circle as *const c_void,
                    1 => &mut aabb as *mut c2AABB as *const c_void,
                    _ => &mut capsule as *mut c2Capsule as *const c_void,
                };
                let seed = c2Proxy {
                    radius: 42.5,
                    count: -7,
                    verts: [c2v { x: 1.5, y: -1.5 }; 8],
                };
                let mut pc = seed;
                let mut pr = seed;
                (c.c2MakeProxy)(shape, t, &mut pc);
                (r.c2MakeProxy)(shape, t, &mut pr);
                eq_proxy(&format!("c2MakeProxy type={t} payload={payload}"), &pc, &pr);
                eq_proxy("must be untouched", &seed, &pc);
            }

            // c2Collided: 0 for any pairing that involves an invalid type.
            let ba = put(&rng.circle(10.0));
            let bb = put(&rng.capsule(10.0));
            let pa = ba.0.as_ptr() as *const c_void;
            let pb = bb.0.as_ptr() as *const c_void;
            for other in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, t] {
                let cv = (c.c2Collided)(pa, t, pb, other);
                let rv = (r.c2Collided)(pa, t, pb, other);
                eq_int(&format!("c2Collided(typeA={t}, typeB={other})"), cv, rv);
                eq_int("must be 0", 0, cv);

                let cv = (c.c2Collided)(pa, other, pb, t);
                let rv = (r.c2Collided)(pa, other, pb, t);
                eq_int(&format!("c2Collided(typeA={other}, typeB={t})"), cv, rv);
                eq_int("must be 0", 0, cv);
            }
        }
    }
}

// ===========================================================================
// G6 — the full IEEE-754 boundary grid through every scalar/vector export.
// ===========================================================================

#[test]
fn g6_boundary_grid_through_every_value_entry_point() {
    let (c, r) = libs();
    unsafe {
        for &x in GRID.iter() {
            for &y in GRID.iter() {
                let a = c2v { x, y };
                // 1-arg vector functions
                eq_v("c2Neg", (c.c2Neg)(a), (r.c2Neg)(a));
                eq_v("c2Skew", (c.c2Skew)(a), (r.c2Skew)(a));
                eq_v("c2CCW90", (c.c2CCW90)(a), (r.c2CCW90)(a));
                eq_v("c2Norm", (c.c2Norm)(a), (r.c2Norm)(a));
                eq_f32("c2Len", (c.c2Len)(a), (r.c2Len)(a));
                eq_v("c2V", (c.c2V)(x, y), (r.c2V)(x, y));

                for &z in GRID.iter() {
                    let b = c2v { x: z, y: x };
                    let rot = c2r { c: y, s: z };
                    eq_v("c2Add", (c.c2Add)(a, b), (r.c2Add)(a, b));
                    eq_v("c2Sub", (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                    eq_v("c2Maxv", (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                    eq_v("c2Minv", (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                    eq_v("c2Mulvs", (c.c2Mulvs)(a, z), (r.c2Mulvs)(a, z));
                    eq_v("c2Div", (c.c2Div)(a, z), (r.c2Div)(a, z));
                    eq_f32("c2Dot", (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                    eq_f32("c2Det2", (c.c2Det2)(a, b), (r.c2Det2)(a, b));
                    eq_v("c2Mulrv", (c.c2Mulrv)(rot, b), (r.c2Mulrv)(rot, b));
                    eq_v("c2MulrvT", (c.c2MulrvT)(rot, b), (r.c2MulrvT)(rot, b));
                    let xf = c2x { p: a, r: rot };
                    eq_v("c2Mulxv", (c.c2Mulxv)(xf, b), (r.c2Mulxv)(xf, b));
                    eq_v(
                        "c2Clampv",
                        (c.c2Clampv)(a, b, c2v { x: z, y: z }),
                        (r.c2Clampv)(a, b, c2v { x: z, y: z }),
                    );

                    // boolean shape tests over the grid
                    let cir = c2Circle { p: a, r: z };
                    let cir2 = c2Circle { p: b, r: x };
                    let bb = c2AABB { min: a, max: b };
                    let bb2 = c2AABB { min: b, max: a };
                    let cap = c2Capsule { a, b, r: z };
                    eq_int(
                        "c2CircletoCircle",
                        (c.c2CircletoCircle)(cir, cir2),
                        (r.c2CircletoCircle)(cir, cir2),
                    );
                    eq_int(
                        "c2CircletoAABB",
                        (c.c2CircletoAABB)(cir, bb),
                        (r.c2CircletoAABB)(cir, bb),
                    );
                    eq_int(
                        "c2CircletoCapsule",
                        (c.c2CircletoCapsule)(cir, cap),
                        (r.c2CircletoCapsule)(cir, cap),
                    );
                    eq_int(
                        "c2AABBtoAABB",
                        (c.c2AABBtoAABB)(bb, bb2),
                        (r.c2AABBtoAABB)(bb, bb2),
                    );
                    eq_int(
                        "c2AABBtoCapsule",
                        (c.c2AABBtoCapsule)(bb, cap),
                        (r.c2AABBtoCapsule)(bb, cap),
                    );
                    eq_int(
                        "c2CapsuletoCapsule",
                        (c.c2CapsuletoCapsule)(cap, cap),
                        (r.c2CapsuletoCapsule)(cap, cap),
                    );
                    eq_int(
                        "reverse_collide",
                        (c.reverse_collide)(x, y, z),
                        (r.reverse_collide)(x, y, z),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Per-symbol coverage gate: call every exported symbol at least once, through
// both `.so`s, and assert the resolved set equals the C `.so`'s export list.
// ===========================================================================

#[test]
fn g7_every_exported_symbol_is_exercised_differentially() {
    let (c, r) = libs();
    let mut called: BTreeSet<&'static str> = BTreeSet::new();
    let mut rng = Rng::new(0x67);

    macro_rules! mark {
        ($($n:literal),* $(,)?) => { $( called.insert($n); )* };
    }

    unsafe {
        let a = c2v { x: 3.0, y: -4.0 };
        let b = c2v { x: -1.5, y: 2.25 };
        let rot = c2r { c: 0.6, s: 0.8 };
        let xf = c2x { p: a, r: rot };

        eq_v("c2V", (c.c2V)(1.0, 2.0), (r.c2V)(1.0, 2.0));
        mark!("c2V");
        eq_v("c2Mulvs", (c.c2Mulvs)(a, 2.5), (r.c2Mulvs)(a, 2.5));
        mark!("c2Mulvs");
        eq_v("c2Maxv", (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
        mark!("c2Maxv");
        eq_v("c2Minv", (c.c2Minv)(a, b), (r.c2Minv)(a, b));
        mark!("c2Minv");
        eq_v("c2Clampv", (c.c2Clampv)(a, b, a), (r.c2Clampv)(a, b, a));
        mark!("c2Clampv");
        eq_v("c2Sub", (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        mark!("c2Sub");
        eq_f32("c2Dot", (c.c2Dot)(a, b), (r.c2Dot)(a, b));
        mark!("c2Dot");
        eq_r("c2RotIdentity", (c.c2RotIdentity)(), (r.c2RotIdentity)());
        mark!("c2RotIdentity");
        eq_x("c2xIdentity", (c.c2xIdentity)(), (r.c2xIdentity)());
        mark!("c2xIdentity");

        let mut bbc = c2AABB { min: b, max: a };
        let mut bbr = bbc;
        let mut oc = [c2v::default(); 4];
        let mut orr = [c2v::default(); 4];
        (c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
        (r.c2BBVerts)(orr.as_mut_ptr(), &mut bbr);
        eq_bytes("c2BBVerts", &oc, &orr);
        mark!("c2BBVerts");

        let mut circle = c2Circle { p: a, r: 2.0 };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        (c.c2MakeProxy)(
            &mut circle as *mut c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &mut pc,
        );
        (r.c2MakeProxy)(
            &mut circle as *mut c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &mut pr,
        );
        eq_proxy("c2MakeProxy", &pc, &pr);
        mark!("c2MakeProxy");

        eq_f32("c2Len", (c.c2Len)(a), (r.c2Len)(a));
        mark!("c2Len");
        eq_f32("c2Det2", (c.c2Det2)(a, b), (r.c2Det2)(a, b));
        mark!("c2Det2");

        let mut sc = rng.simplex(3, 10.0);
        let mut sr = sc;
        eq_f32(
            "c2GJKSimplexMetric",
            (c.c2GJKSimplexMetric)(&mut sc),
            (r.c2GJKSimplexMetric)(&mut sr),
        );
        eq_simplex("c2GJKSimplexMetric", &sc, &sr);
        mark!("c2GJKSimplexMetric");

        eq_v("c2Mulrv", (c.c2Mulrv)(rot, b), (r.c2Mulrv)(rot, b));
        mark!("c2Mulrv");
        eq_v("c2Add", (c.c2Add)(a, b), (r.c2Add)(a, b));
        mark!("c2Add");
        eq_v("c2Mulxv", (c.c2Mulxv)(xf, b), (r.c2Mulxv)(xf, b));
        mark!("c2Mulxv");

        let mut sc = rng.simplex(2, 10.0);
        let mut sr = sc;
        (c.c22)(&mut sc);
        (r.c22)(&mut sr);
        eq_simplex("c22", &sc, &sr);
        mark!("c22");

        let mut sc = rng.simplex(3, 10.0);
        let mut sr = sc;
        (c.c23)(&mut sc);
        (r.c23)(&mut sr);
        eq_simplex("c23", &sc, &sr);
        mark!("c23");

        eq_v("c2Neg", (c.c2Neg)(a), (r.c2Neg)(a));
        mark!("c2Neg");
        eq_v("c2Skew", (c.c2Skew)(a), (r.c2Skew)(a));
        mark!("c2Skew");
        eq_v("c2CCW90", (c.c2CCW90)(a), (r.c2CCW90)(a));
        mark!("c2CCW90");

        let mut sc = rng.simplex(2, 10.0);
        let mut sr = sc;
        eq_v("c2D", (c.c2D)(&mut sc), (r.c2D)(&mut sr));
        eq_simplex("c2D", &sc, &sr);
        mark!("c2D");

        let verts = [a, b, c2v { x: 7.0, y: 7.0 }, c2v { x: -7.0, y: 0.0 }];
        eq_int(
            "c2Support",
            (c.c2Support)(verts.as_ptr(), 4, b),
            (r.c2Support)(verts.as_ptr(), 4, b),
        );
        mark!("c2Support");

        let mut sc = rng.simplex(3, 10.0);
        let mut sr = sc;
        let mut wac = c2v::default();
        let mut wbc = c2v::default();
        let mut war = c2v::default();
        let mut wbr = c2v::default();
        (c.c2Witness)(&mut sc, &mut wac, &mut wbc);
        (r.c2Witness)(&mut sr, &mut war, &mut wbr);
        eq_v("c2Witness a", wac, war);
        eq_v("c2Witness b", wbc, wbr);
        mark!("c2Witness");

        eq_v("c2Div", (c.c2Div)(a, 3.0), (r.c2Div)(a, 3.0));
        mark!("c2Div");
        eq_v("c2Norm", (c.c2Norm)(a), (r.c2Norm)(a));
        mark!("c2Norm");

        let mut sc = rng.simplex(2, 10.0);
        let mut sr = sc;
        eq_v("c2L", (c.c2L)(&mut sc), (r.c2L)(&mut sr));
        mark!("c2L");

        eq_v("c2MulrvT", (c.c2MulrvT)(rot, b), (r.c2MulrvT)(rot, b));
        mark!("c2MulrvT");

        // c2GJK
        let ba = put(&c2Circle { p: a, r: 2.0 });
        let bb = put(&c2AABB { min: b, max: a });
        let mut oa1 = c2v::default();
        let mut ob1 = c2v::default();
        let mut it1: c_int = 0;
        let mut cc = c2GJKCache::default();
        let d1 = (c.c2GJK)(
            ba.0.as_ptr() as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            bb.0.as_ptr() as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &mut oa1,
            &mut ob1,
            1,
            &mut it1,
            &mut cc,
        );
        let mut oa2 = c2v::default();
        let mut ob2 = c2v::default();
        let mut it2: c_int = 0;
        let mut cr = c2GJKCache::default();
        let d2 = (r.c2GJK)(
            ba.0.as_ptr() as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            bb.0.as_ptr() as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &mut oa2,
            &mut ob2,
            1,
            &mut it2,
            &mut cr,
        );
        eq_f32("c2GJK", d1, d2);
        eq_v("c2GJK outA", oa1, oa2);
        eq_v("c2GJK outB", ob1, ob2);
        eq_int("c2GJK iters", it1, it2);
        eq_cache("c2GJK cache", &cc, &cr);
        mark!("c2GJK");

        let bx1 = c2AABB { min: b, max: a };
        let bx2 = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        eq_int(
            "c2AABBtoAABB",
            (c.c2AABBtoAABB)(bx1, bx2),
            (r.c2AABBtoAABB)(bx1, bx2),
        );
        mark!("c2AABBtoAABB");
        let cap = c2Capsule { a, b, r: 1.0 };
        eq_int(
            "c2AABBtoCapsule",
            (c.c2AABBtoCapsule)(bx1, cap),
            (r.c2AABBtoCapsule)(bx1, cap),
        );
        mark!("c2AABBtoCapsule");
        eq_int(
            "c2CapsuletoCapsule",
            (c.c2CapsuletoCapsule)(cap, cap),
            (r.c2CapsuletoCapsule)(cap, cap),
        );
        mark!("c2CapsuletoCapsule");
        let ci = c2Circle { p: a, r: 2.0 };
        let ci2 = c2Circle { p: b, r: 3.0 };
        eq_int(
            "c2CircletoCircle",
            (c.c2CircletoCircle)(ci, ci2),
            (r.c2CircletoCircle)(ci, ci2),
        );
        mark!("c2CircletoCircle");
        eq_int(
            "c2CircletoAABB",
            (c.c2CircletoAABB)(ci, bx1),
            (r.c2CircletoAABB)(ci, bx1),
        );
        mark!("c2CircletoAABB");
        eq_int(
            "c2CircletoCapsule",
            (c.c2CircletoCapsule)(ci, cap),
            (r.c2CircletoCapsule)(ci, cap),
        );
        mark!("c2CircletoCapsule");
        let p1 = put(&ci);
        let p2 = put(&cap);
        eq_int(
            "c2Collided",
            (c.c2Collided)(
                p1.0.as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
                p2.0.as_ptr() as *const c_void,
                C2_TYPE_CAPSULE,
            ),
            (r.c2Collided)(
                p1.0.as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
                p2.0.as_ptr() as *const c_void,
                C2_TYPE_CAPSULE,
            ),
        );
        mark!("c2Collided");
        eq_int(
            "reverse_collide",
            (c.reverse_collide)(-70.0, 0.0, 5.0),
            (r.reverse_collide)(-70.0, 0.0, 5.0),
        );
        mark!("reverse_collide");
    }

    let expected: BTreeSet<&str> = Api::SYMBOLS.iter().copied().collect();
    let missing: Vec<&&str> = expected.difference(&called).collect();
    assert!(
        missing.is_empty(),
        "these exported symbols were never called differentially: {missing:?}"
    );
    assert_eq!(called.len(), 38, "expected all 38 exports to be exercised");
}
