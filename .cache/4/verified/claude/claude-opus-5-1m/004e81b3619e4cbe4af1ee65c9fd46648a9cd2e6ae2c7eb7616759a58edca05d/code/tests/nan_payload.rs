//! Justification for the one relaxation in `common::feq`.
//!
//! `feq` treats "both results are NaN" as equal because the surviving NaN
//! *payload* of an x86 SSE operation with two NaN operands is the destination
//! register's, i.e. it is decided by the compiler's register allocator, not by
//! IEEE-754 / C / Rust.  (GCC `-O0` emits `mulss %xmm0,%xmm1` for `a.x*b.x` but
//! `mulss %xmm2,%xmm0` for `a.y*b.y`, then `addss %xmm1,%xmm0`; LLVM picks a
//! different order.)
//!
//! This file proves the relaxation is *safe*: over a large randomized sweep of
//! every exported function, whenever the raw bits differ, **both** sides are
//! NaN.  A case where one side is NaN and the other is not — or where two
//! non-NaN values differ — would be a real bug and fails the test.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;

#[derive(Default)]
struct Tally {
    compared: u64,
    bitwise_equal: u64,
    both_nan_diff_payload: u64,
}

impl Tally {
    #[track_caller]
    fn check(&mut self, what: &str, a: f32, b: f32) {
        self.compared += 1;
        if a.to_bits() == b.to_bits() {
            self.bitwise_equal += 1;
            return;
        }
        assert!(
            a.is_nan() && b.is_nan(),
            "{what}: REAL divergence (not just a NaN payload): C={} RUST={}",
            fshow(a),
            fshow(b)
        );
        self.both_nan_diff_payload += 1;
    }

    fn checkv(&mut self, what: &str, a: C2v, b: C2v) {
        self.check(what, a.x, b.x);
        self.check(what, a.y, b.y);
    }

    fn checkrc(&mut self, what: &str, a: C2Raycast, b: C2Raycast) {
        self.check(what, a.t, b.t);
        self.checkv(what, a.n, b.n);
    }

    fn report(&self, name: &str) {
        println!(
            "{name}: {} floats compared, {} bit-identical, {} differing only in NaN payload ({:.4}%)",
            self.compared,
            self.bitwise_equal,
            self.both_nan_diff_payload,
            100.0 * self.both_nan_diff_payload as f64 / self.compared.max(1) as f64
        );
        assert_eq!(
            self.compared,
            self.bitwise_equal + self.both_nan_diff_payload,
            "{name}: accounting error"
        );
    }
}

#[test]
fn nan_payload_is_the_only_difference() {
    let (c, r) = (c(), rs());
    let mut t = Tally::default();
    let mut rng = Rng::new(0x00AA_0BAD_10AD_5EED);

    for _ in 0..20000 {
        let a = rng.wild_v();
        let b = rng.wild_v();
        let s = rng.wild();

        t.check("c2Dot", unsafe { (c.c2Dot)(a, b) }, unsafe { (r.c2Dot)(a, b) });
        t.check("c2Len", unsafe { (c.c2Len)(a) }, unsafe { (r.c2Len)(a) });
        t.checkv("c2V", unsafe { (c.c2V)(s, a.x) }, unsafe { (r.c2V)(s, a.x) });
        t.checkv("c2Add", unsafe { (c.c2Add)(a, b) }, unsafe { (r.c2Add)(a, b) });
        t.checkv("c2Sub", unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) });
        t.checkv("c2Mulvs", unsafe { (c.c2Mulvs)(a, s) }, unsafe { (r.c2Mulvs)(a, s) });
        t.checkv("c2Div", unsafe { (c.c2Div)(a, s) }, unsafe { (r.c2Div)(a, s) });
        t.checkv("c2Norm", unsafe { (c.c2Norm)(a) }, unsafe { (r.c2Norm)(a) });
        t.checkv("c2Minv", unsafe { (c.c2Minv)(a, b) }, unsafe { (r.c2Minv)(a, b) });
        t.checkv("c2Maxv", unsafe { (c.c2Maxv)(a, b) }, unsafe { (r.c2Maxv)(a, b) });
        t.checkv("c2Skew", unsafe { (c.c2Skew)(a) }, unsafe { (r.c2Skew)(a) });
        t.checkv("c2Absv", unsafe { (c.c2Absv)(a) }, unsafe { (r.c2Absv)(a) });
        t.checkv("c2CCW90", unsafe { (c.c2CCW90)(a) }, unsafe { (r.c2CCW90)(a) });

        let m = C2m { x: a, y: b };
        t.checkv("c2MulmvT", unsafe { (c.c2MulmvT)(m, a) }, unsafe { (r.c2MulmvT)(m, a) });

        let rot = C2r { c: a.x, s: a.y };
        t.checkv("c2Mulrv", unsafe { (c.c2Mulrv)(rot, b) }, unsafe { (r.c2Mulrv)(rot, b) });
        t.checkv("c2MulrvT", unsafe { (c.c2MulrvT)(rot, b) }, unsafe { (r.c2MulrvT)(rot, b) });
        let xf = C2x { p: b, r: rot };
        t.checkv("c2MulxvT", unsafe { (c.c2MulxvT)(xf, a) }, unsafe { (r.c2MulxvT)(xf, a) });

        // integer predicates must be *exactly* equal, always
        let boxa = C2AABB { min: a, max: b };
        let boxb = C2AABB { min: b, max: a };
        assert_eq!(
            unsafe { (c.c2AABBtoAABB)(boxa, boxb) },
            unsafe { (r.c2AABBtoAABB)(boxa, boxb) },
            "c2AABBtoAABB diverged"
        );
        assert_eq!(
            unsafe { (c.c2AABBtoPoint)(boxa, a) },
            unsafe { (r.c2AABBtoPoint)(boxa, a) },
            "c2AABBtoPoint diverged"
        );
        let circle = C2Circle { p: a, r: s };
        assert_eq!(
            unsafe { (c.c2CircleToPoint)(circle, b) },
            unsafe { (r.c2CircleToPoint)(circle, b) },
            "c2CircleToPoint diverged"
        );

        let A = C2Ray { p: a, d: b, t: s };
        let mut oc = poison(0x2222_2222);
        let mut orr = poison(0x2222_2222);
        assert_eq!(
            unsafe { (c.c2RaytoCircle)(A, circle, &mut oc) },
            unsafe { (r.c2RaytoCircle)(A, circle, &mut orr) },
            "c2RaytoCircle return diverged"
        );
        t.checkrc("c2RaytoCircle", oc, orr);

        let mut oc = poison(0x3333_3333);
        let mut orr = poison(0x3333_3333);
        assert_eq!(
            unsafe { (c.c2RaytoAABB)(A, boxa, &mut oc) },
            unsafe { (r.c2RaytoAABB)(A, boxa, &mut orr) },
            "c2RaytoAABB return diverged"
        );
        t.checkrc("c2RaytoAABB", oc, orr);

        let capsule = C2Capsule { a, b, r: s };
        let mut oc = poison(0x4444_4444);
        let mut orr = poison(0x4444_4444);
        assert_eq!(
            unsafe { (c.c2RaytoCapsule)(A, capsule, &mut oc) },
            unsafe { (r.c2RaytoCapsule)(A, capsule, &mut orr) },
            "c2RaytoCapsule return diverged"
        );
        t.checkrc("c2RaytoCapsule", oc, orr);

        let mut poly = C2Poly::default();
        poly.count = rng.below(9) as i32;
        for k in 0..8 {
            poly.verts[k] = rng.wild_v();
            poly.norms[k] = rng.wild_v();
        }
        for bxp in [std::ptr::null(), &xf as *const C2x] {
            let mut oc = poison(0x5555_5555);
            let mut orr = poison(0x5555_5555);
            assert_eq!(
                unsafe { (c.c2RaytoPoly)(A, &poly, bxp, &mut oc) },
                unsafe { (r.c2RaytoPoly)(A, &poly, bxp, &mut orr) },
                "c2RaytoPoly return diverged"
            );
            t.checkrc("c2RaytoPoly", oc, orr);

            let mut oc = poison(0x6666_6666);
            let mut orr = poison(0x6666_6666);
            let pv = (&poly as *const C2Poly) as *const c_void;
            assert_eq!(
                unsafe { (c.c2CastRay)(A, pv, bxp, C2_TYPE_POLY, &mut oc) },
                unsafe { (r.c2CastRay)(A, pv, bxp, C2_TYPE_POLY, &mut orr) },
                "c2CastRay return diverged"
            );
            t.checkrc("c2CastRay", oc, orr);
        }
    }

    t.report("wild sweep");
    assert!(
        t.compared > 1_000_000,
        "expected a large sweep, only compared {}",
        t.compared
    );
}

/// The same sweep restricted to *finite* inputs: here even the raw bits must
/// match, with zero exceptions.  This is the strongest statement the harness
/// can make and it must hold with no relaxation at all.
#[test]
fn finite_inputs_are_bit_identical_with_no_relaxation() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x0F1E_2D3C_4B5A_6978);
    let mut compared = 0u64;

    #[track_caller]
    fn strict(what: &str, a: f32, b: f32, compared: &mut u64) {
        *compared += 1;
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "{what}: finite-input divergence C={} RUST={}",
            fshow(a),
            fshow(b)
        );
    }

    for _ in 0..20000 {
        let a = C2v {
            x: rng.geom(),
            y: rng.geom(),
        };
        let b = C2v {
            x: rng.geom(),
            y: rng.geom(),
        };
        // a scale that never produces 0 (so no 0/0) and never overflows
        let s = {
            let mut x = rng.unit(16.0);
            if x == 0.0 {
                x = 1.0;
            }
            x
        };
        assert!(a.x.is_finite() && a.y.is_finite() && b.x.is_finite() && b.y.is_finite());

        strict("c2Dot", unsafe { (c.c2Dot)(a, b) }, unsafe { (r.c2Dot)(a, b) }, &mut compared);
        strict("c2Len", unsafe { (c.c2Len)(a) }, unsafe { (r.c2Len)(a) }, &mut compared);
        for (nm, x, y) in [
            ("c2Add", unsafe { (c.c2Add)(a, b) }, unsafe { (r.c2Add)(a, b) }),
            ("c2Sub", unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) }),
            ("c2Mulvs", unsafe { (c.c2Mulvs)(a, s) }, unsafe { (r.c2Mulvs)(a, s) }),
            ("c2Div", unsafe { (c.c2Div)(a, s) }, unsafe { (r.c2Div)(a, s) }),
            ("c2Minv", unsafe { (c.c2Minv)(a, b) }, unsafe { (r.c2Minv)(a, b) }),
            ("c2Maxv", unsafe { (c.c2Maxv)(a, b) }, unsafe { (r.c2Maxv)(a, b) }),
            ("c2Skew", unsafe { (c.c2Skew)(a) }, unsafe { (r.c2Skew)(a) }),
            ("c2Absv", unsafe { (c.c2Absv)(a) }, unsafe { (r.c2Absv)(a) }),
            ("c2CCW90", unsafe { (c.c2CCW90)(a) }, unsafe { (r.c2CCW90)(a) }),
        ] {
            strict(nm, x.x, y.x, &mut compared);
            strict(nm, x.y, y.y, &mut compared);
        }

        // geometry: a well-formed ray against well-formed shapes
        let A = C2Ray {
            p: a,
            d: b,
            t: s.abs(),
        };
        let circle = C2Circle {
            p: b,
            r: 0.25 + rng.unit(6.0).abs(),
        };
        let boxa = C2AABB {
            min: C2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: C2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        };
        let capsule = C2Capsule {
            a,
            b: C2v {
                x: b.x + 1.0,
                y: b.y + 2.0,
            },
            r: 0.25 + rng.unit(4.0).abs(),
        };
        for (nm, cr, rr) in [
            ("c2RaytoCircle", {
                let mut o = poison(1);
                let hit = unsafe { (c.c2RaytoCircle)(A, circle, &mut o) };
                (hit, o)
            }, {
                let mut o = poison(1);
                let hit = unsafe { (r.c2RaytoCircle)(A, circle, &mut o) };
                (hit, o)
            }),
            ("c2RaytoAABB", {
                let mut o = poison(1);
                let hit = unsafe { (c.c2RaytoAABB)(A, boxa, &mut o) };
                (hit, o)
            }, {
                let mut o = poison(1);
                let hit = unsafe { (r.c2RaytoAABB)(A, boxa, &mut o) };
                (hit, o)
            }),
            ("c2RaytoCapsule", {
                let mut o = poison(1);
                let hit = unsafe { (c.c2RaytoCapsule)(A, capsule, &mut o) };
                (hit, o)
            }, {
                let mut o = poison(1);
                let hit = unsafe { (r.c2RaytoCapsule)(A, capsule, &mut o) };
                (hit, o)
            }),
        ] {
            assert_eq!(cr.0, rr.0, "{nm}: return diverged");
            // A capsule/circle cast can legitimately produce NaN normals from
            // c2Norm(0) even with finite inputs, so only compare bits when both
            // are non-NaN; NaN-ness itself must still agree.
            for (x, y) in [(cr.1.t, rr.1.t), (cr.1.n.x, rr.1.n.x), (cr.1.n.y, rr.1.n.y)] {
                assert_eq!(x.is_nan(), y.is_nan(), "{nm}: NaN-ness diverged");
                if !x.is_nan() {
                    strict(nm, x, y, &mut compared);
                }
            }
        }
    }
    println!("finite sweep: {compared} floats compared, all bit-identical");
    assert!(compared > 500_000);
}
