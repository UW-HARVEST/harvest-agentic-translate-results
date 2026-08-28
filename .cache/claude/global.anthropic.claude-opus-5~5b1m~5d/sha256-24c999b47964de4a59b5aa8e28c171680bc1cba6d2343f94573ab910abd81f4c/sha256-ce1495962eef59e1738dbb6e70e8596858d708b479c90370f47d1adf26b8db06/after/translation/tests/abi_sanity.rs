//! ABI / semantic sanity anchors.
//!
//! Every other test in this suite is *differential*: it only says "C and Rust
//! agree".  That would still pass if this test harness declared a signature
//! incorrectly in a way that happened to break both sides identically.  These
//! tests therefore pin down absolute, hand-computed answers from the C library
//! (the ground truth) as well as from the Rust one, which proves that the
//! harness really is speaking the x86-64 SysV C ABI:
//!
//!   * `c2v` / `c2r` (8 bytes, two floats) are returned packed in `xmm0`;
//!   * `c2x` (16 bytes, four floats) is returned in `xmm0`+`xmm1` and passed in
//!     two SSE registers;
//!   * `c2Circle` / `c2AABB` / `c2Capsule` are passed by pointer through a
//!     `const void *`;
//!   * `C2_TYPE` is an `unsigned int`-sized enum;
//!   * `c2GJKCache` / `c2Simplex` / `c2Proxy` have the offsets this crate uses.

mod common;
use common::*;

#[test]
fn abi_struct_returns_and_known_values() {
    let (cv, rv): (FnV2, FnV2) = sym(b"c2V");
    for lib in ["C", "Rust"] {
        let f = if lib == "C" { cv } else { rv };
        let v = unsafe { f(1.5, -2.25) };
        assert!(
            f32_same(v.x, 1.5) && f32_same(v.y, -2.25),
            "{lib}: c2V(1.5, -2.25) returned {}",
            fmt_v(v)
        );
    }

    let (cr, rr): (FnR, FnR) = sym(b"c2RotIdentity");
    for (lib, f) in [("C", cr), ("Rust", rr)] {
        let r = unsafe { f() };
        assert!(
            f32_same(r.c, 1.0) && f32_same(r.s, 0.0),
            "{lib}: c2RotIdentity returned ({}, {})",
            fmt_f32(r.c),
            fmt_f32(r.s)
        );
    }

    let (cx, rx): (FnX, FnX) = sym(b"c2xIdentity");
    for (lib, f) in [("C", cx), ("Rust", rx)] {
        let x = unsafe { f() };
        assert!(
            f32_same(x.p.x, 0.0)
                && f32_same(x.p.y, 0.0)
                && f32_same(x.r.c, 1.0)
                && f32_same(x.r.s, 0.0),
            "{lib}: c2xIdentity returned p={} r=({}, {})",
            fmt_v(x.p),
            fmt_f32(x.r.c),
            fmt_f32(x.r.s)
        );
    }

    // c2x passed BY VALUE: rotate (1,0) by 90 degrees and translate by (10,20).
    let (cm, rm): (FnMulxv, FnMulxv) = sym(b"c2Mulxv");
    let tx = C2x {
        p: C2v { x: 10.0, y: 20.0 },
        r: C2r { c: 0.0, s: 1.0 },
    };
    for (lib, f) in [("C", cm), ("Rust", rm)] {
        let got = unsafe { f(tx, C2v { x: 1.0, y: 0.0 }) };
        assert!(
            f32_same(got.x, 10.0) && f32_same(got.y, 21.0),
            "{lib}: c2Mulxv gave {} (expected (10, 21))",
            fmt_v(got)
        );
    }

    // c2r passed BY VALUE.
    let (cmr, rmr): (FnMulrv, FnMulrv) = sym(b"c2Mulrv");
    for (lib, f) in [("C", cmr), ("Rust", rmr)] {
        let got = unsafe { f(C2r { c: 0.0, s: 1.0 }, C2v { x: 3.0, y: 4.0 }) };
        assert!(
            f32_same(got.x, -4.0) && f32_same(got.y, 3.0),
            "{lib}: c2Mulrv gave {} (expected (-4, 3))",
            fmt_v(got)
        );
    }
    let (cmt, rmt): (FnMulrv, FnMulrv) = sym(b"c2MulrvT");
    for (lib, f) in [("C", cmt), ("Rust", rmt)] {
        let got = unsafe { f(C2r { c: 0.0, s: 1.0 }, C2v { x: 3.0, y: 4.0 }) };
        assert!(
            f32_same(got.x, 4.0) && f32_same(got.y, -3.0),
            "{lib}: c2MulrvT gave {} (expected (4, -3))",
            fmt_v(got)
        );
    }

    // Scalar helpers with hand-computed answers.
    let (cd, rd): (FnFvv, FnFvv) = sym(b"c2Dot");
    let (cdet, rdet): (FnFvv, FnFvv) = sym(b"c2Det2");
    let (cl, rl): (FnFv, FnFv) = sym(b"c2Len");
    for (lib, (d, det, len)) in [("C", (cd, cdet, cl)), ("Rust", (rd, rdet, rl))] {
        let a = C2v { x: 3.0, y: 4.0 };
        let b = C2v { x: -1.0, y: 2.0 };
        assert!(
            f32_same(unsafe { d(a, b) }, 5.0),
            "{lib}: c2Dot((3,4),(-1,2)) should be 5"
        );
        assert!(
            f32_same(unsafe { det(a, b) }, 10.0),
            "{lib}: c2Det2((3,4),(-1,2)) should be 10"
        );
        assert!(
            f32_same(unsafe { len(a) }, 5.0),
            "{lib}: c2Len((3,4)) should be 5"
        );
    }
}

#[test]
fn abi_proxy_and_enum_layout() {
    let (c, r): (FnMakeProxy, FnMakeProxy) = sym(b"c2MakeProxy");
    // C2_TYPE_AABB must be the *second* enumerator, i.e. the value 1 passed as
    // an unsigned int, and c2BBVerts's corner order is min, (max.x,min.y), max,
    // (min.x,max.y).
    let bb = ShapeBlob::aabb(C2AABB {
        min: C2v { x: -1.0, y: -2.0 },
        max: C2v { x: 3.0, y: 4.0 },
    });
    for (lib, f) in [("C", c), ("Rust", r)] {
        let mut p = C2Proxy {
            radius: 7.0,
            count: -1,
            verts: [C2v { x: 9.0, y: 9.0 }; 8],
        };
        unsafe { f(bb.as_ptr(), C2_TYPE_AABB, &mut p) };
        assert!(f32_same(p.radius, 0.0), "{lib}: AABB proxy radius");
        assert_eq!(p.count, 4, "{lib}: AABB proxy count");
        let want = [
            C2v { x: -1.0, y: -2.0 },
            C2v { x: 3.0, y: -2.0 },
            C2v { x: 3.0, y: 4.0 },
            C2v { x: -1.0, y: 4.0 },
        ];
        for k in 0..4 {
            assert!(
                v_same(p.verts[k], want[k]),
                "{lib}: AABB corner {k} = {} (expected {})",
                fmt_v(p.verts[k]),
                fmt_v(want[k])
            );
        }
        // The remaining vertices must be untouched.
        for k in 4..8 {
            assert!(
                v_same(p.verts[k], C2v { x: 9.0, y: 9.0 }),
                "{lib}: verts[{k}] was overwritten"
            );
        }
    }

    let circle = ShapeBlob::circle(C2Circle {
        p: C2v { x: 5.0, y: 6.0 },
        r: 2.5,
    });
    let capsule = ShapeBlob::capsule(C2Capsule {
        a: C2v { x: 1.0, y: 2.0 },
        b: C2v { x: 3.0, y: 4.0 },
        r: 0.75,
    });
    for (lib, f) in [("C", c), ("Rust", r)] {
        let mut p = C2Proxy::default();
        unsafe { f(circle.as_ptr(), C2_TYPE_CIRCLE, &mut p) };
        assert!(f32_same(p.radius, 2.5) && p.count == 1, "{lib}: circle proxy");
        assert!(v_same(p.verts[0], C2v { x: 5.0, y: 6.0 }));
        let mut p = C2Proxy::default();
        unsafe { f(capsule.as_ptr(), C2_TYPE_CAPSULE, &mut p) };
        assert!(f32_same(p.radius, 0.75) && p.count == 2, "{lib}: capsule proxy");
        assert!(v_same(p.verts[0], C2v { x: 1.0, y: 2.0 }));
        assert!(v_same(p.verts[1], C2v { x: 3.0, y: 4.0 }));
    }
}

#[test]
fn abi_gjk_known_distances() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    // Two circles: centres 5 apart, radii 1 and 1.  Minkowski (use_radius = 0)
    // distance is the centre distance 5; with use_radius it is 5 - 2 = 3 and the
    // witness points sit on the two surfaces.
    let a = ShapeBlob::circle(C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 1.0,
    });
    let b = ShapeBlob::circle(C2Circle {
        p: C2v { x: 5.0, y: 0.0 },
        r: 1.0,
    });
    for (lib, f) in [("C", c), ("Rust", r)] {
        for (ur, want_d, want_a, want_b) in [
            (0i32, 5.0f32, C2v { x: 0.0, y: 0.0 }, C2v { x: 5.0, y: 0.0 }),
            (1, 3.0, C2v { x: 1.0, y: 0.0 }, C2v { x: 4.0, y: 0.0 }),
        ] {
            let mut oa = C2v::default();
            let mut ob = C2v::default();
            let mut it = -1i32;
            let d = unsafe {
                f(
                    a.as_ptr(),
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    b.as_ptr(),
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    &mut oa,
                    &mut ob,
                    ur,
                    &mut it,
                    std::ptr::null_mut(),
                )
            };
            assert!(
                f32_same(d, want_d),
                "{lib}: c2GJK circles ur={ur} distance {} (expected {want_d})",
                fmt_f32(d)
            );
            assert!(
                v_same(oa, want_a) && v_same(ob, want_b),
                "{lib}: witness points {} / {} (expected {} / {})",
                fmt_v(oa),
                fmt_v(ob),
                fmt_v(want_a),
                fmt_v(want_b)
            );
        }
    }

    // Two unit AABBs, one at [0,1]^2 and one at [3,4]x[0,1]: the gap is 2.
    let a = ShapeBlob::aabb(C2AABB {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    });
    let b = ShapeBlob::aabb(C2AABB {
        min: C2v { x: 3.0, y: 0.0 },
        max: C2v { x: 4.0, y: 1.0 },
    });
    for (lib, f) in [("C", c), ("Rust", r)] {
        let mut oa = C2v::default();
        let mut ob = C2v::default();
        let d = unsafe {
            f(
                a.as_ptr(),
                C2_TYPE_AABB,
                std::ptr::null(),
                b.as_ptr(),
                C2_TYPE_AABB,
                std::ptr::null(),
                &mut oa,
                &mut ob,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert!(
            f32_same(d, 2.0),
            "{lib}: AABB gap {} (expected 2)",
            fmt_f32(d)
        );
    }

    // Overlapping AABBs must report distance 0 and equal witness points.
    let b = ShapeBlob::aabb(C2AABB {
        min: C2v { x: 0.5, y: 0.5 },
        max: C2v { x: 2.0, y: 2.0 },
    });
    for (lib, f) in [("C", c), ("Rust", r)] {
        let mut oa = C2v { x: 1.0, y: 1.0 };
        let mut ob = C2v { x: 2.0, y: 2.0 };
        let d = unsafe {
            f(
                a.as_ptr(),
                C2_TYPE_AABB,
                std::ptr::null(),
                b.as_ptr(),
                C2_TYPE_AABB,
                std::ptr::null(),
                &mut oa,
                &mut ob,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert!(
            f32_same(d, 0.0) && v_same(oa, ob),
            "{lib}: overlap should give 0 with a == b, got {} {} {}",
            fmt_f32(d),
            fmt_v(oa),
            fmt_v(ob)
        );
    }

    // A transform really is applied: shift B by (+10, 0) and the gap grows by 10.
    let b = ShapeBlob::aabb(C2AABB {
        min: C2v { x: 3.0, y: 0.0 },
        max: C2v { x: 4.0, y: 1.0 },
    });
    let bx = C2x {
        p: C2v { x: 10.0, y: 0.0 },
        r: C2r { c: 1.0, s: 0.0 },
    };
    for (lib, f) in [("C", c), ("Rust", r)] {
        let d = unsafe {
            f(
                a.as_ptr(),
                C2_TYPE_AABB,
                std::ptr::null(),
                b.as_ptr(),
                C2_TYPE_AABB,
                &bx,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert!(
            f32_same(d, 12.0),
            "{lib}: transformed gap {} (expected 12)",
            fmt_f32(d)
        );
    }
}

#[test]
fn abi_cache_write_back_layout() {
    // After a cold call the cache must contain the simplex the C left behind:
    // count in 1..3, indices inside the proxies, div matching.
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let a = ShapeBlob::aabb(C2AABB {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    });
    let b = ShapeBlob::capsule(C2Capsule {
        a: C2v { x: 5.0, y: 0.0 },
        b: C2v { x: 6.0, y: 3.0 },
        r: 0.5,
    });
    for (lib, f) in [("C", c), ("Rust", r)] {
        let mut cache = C2GJKCache {
            metric: 1234.5,
            count: 0,
            iA: [-1; 3],
            iB: [-1; 3],
            div: -9.0,
        };
        let _ = unsafe {
            f(
                a.as_ptr(),
                C2_TYPE_AABB,
                std::ptr::null(),
                b.as_ptr(),
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                std::ptr::null_mut(),
                &mut cache,
            )
        };
        assert!(
            (1..=3).contains(&cache.count),
            "{lib}: cache.count = {} after a cold call",
            cache.count
        );
        for k in 0..cache.count as usize {
            assert!(
                (0..4).contains(&cache.iA[k]),
                "{lib}: cache.iA[{k}] = {} out of the AABB's 4 vertices",
                cache.iA[k]
            );
            assert!(
                (0..2).contains(&cache.iB[k]),
                "{lib}: cache.iB[{k}] = {} out of the capsule's 2 vertices",
                cache.iB[k]
            );
        }
        assert!(cache.div > 0.0, "{lib}: cache.div = {}", fmt_f32(cache.div));
        assert!(
            !f32_same(cache.metric, 1234.5),
            "{lib}: cache.metric was not written"
        );
    }
}
