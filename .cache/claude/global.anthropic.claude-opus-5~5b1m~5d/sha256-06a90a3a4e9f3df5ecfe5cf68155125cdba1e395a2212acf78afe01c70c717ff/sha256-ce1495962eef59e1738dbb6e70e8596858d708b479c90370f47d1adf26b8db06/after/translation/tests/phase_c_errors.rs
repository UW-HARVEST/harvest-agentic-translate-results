//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! Each test constructs the exact invalid input or rejecting condition, calls
//! BOTH shared libraries and asserts they reject in the *same* way: the same
//! sentinel return value AND the same effect (or non-effect) on the `c2Raycast`
//! out-parameter, which is pre-filled with a poison pattern so that "not
//! written" is observable.

mod common;
use common::paths::*;
use common::*;
use std::ffi::c_int;

/* ------------------------------- utilities -------------------------------- */

/// Calls both libraries and returns `(c_ret, c_out, rust_ret, rust_out)`.
fn both_circle(ray: C2Ray, c: C2Circle) -> (c_int, C2Raycast, c_int, C2Raycast) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let rc = unsafe { (p.c.c2RaytoCircle)(ray, c, &mut oc) };
    let rr = unsafe { (p.r.c2RaytoCircle)(ray, c, &mut or_) };
    (rc, oc, rr, or_)
}

fn both_aabb(ray: C2Ray, b: C2AABB) -> (c_int, C2Raycast, c_int, C2Raycast) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let rc = unsafe { (p.c.c2RaytoAABB)(ray, b, &mut oc) };
    let rr = unsafe { (p.r.c2RaytoAABB)(ray, b, &mut or_) };
    (rc, oc, rr, or_)
}

fn both_capsule(ray: C2Ray, cap: C2Capsule) -> (c_int, C2Raycast, c_int, C2Raycast) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let rc = unsafe { (p.c.c2RaytoCapsule)(ray, cap, &mut oc) };
    let rr = unsafe { (p.r.c2RaytoCapsule)(ray, cap, &mut or_) };
    (rc, oc, rr, or_)
}

fn is_poison(o: C2Raycast) -> bool {
    o.t.to_bits() == POISON.t.to_bits()
        && o.n.x.to_bits() == POISON.n.x.to_bits()
        && o.n.y.to_bits() == POISON.n.y.to_bits()
}

fn same_cast(a: C2Raycast, b: C2Raycast) -> bool {
    let eq = |x: f32, y: f32| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan());
    eq(a.t, b.t) && eq(a.n.x, b.n.x) && eq(a.n.y, b.n.y)
}

/// Asserts both libraries rejected identically and left `out` untouched.
fn assert_rejected_untouched(row: &str, what: String, r: (c_int, C2Raycast, c_int, C2Raycast)) {
    let (rc, oc, rr, or_) = r;
    assert_eq!(rc, 0, "[{row}] the C did not reject: {what}");
    assert_eq!(rr, rc, "[{row}] rust returned {rr}, C returned {rc}: {what}");
    assert!(is_poison(oc), "[{row}] the C wrote *out on rejection: {what} -> {oc:?}");
    assert!(
        is_poison(or_),
        "[{row}] rust wrote *out on rejection while the C did not: {what} -> {or_:?}"
    );
}

/// Searches for `n` random inputs whose classified sub-path equals `want`.
fn search<T, G, C>(row: &str, want: usize, n: usize, mut gen: G, classify: C) -> Vec<T>
where
    G: FnMut(&mut Rng) -> T,
    C: Fn(&T) -> usize,
    T: Copy,
{
    let mut rng = Rng::new(0xC0FFEE ^ (want as u64) << 8);
    let mut out = Vec::new();
    for _ in 0..4_000_000 {
        let cand = gen(&mut rng);
        if classify(&cand) == want {
            out.push(cand);
            if out.len() == n {
                return out;
            }
        }
    }
    panic!("[{row}] could not construct {n} inputs for sub-path {want}");
}

/* ==================== rows 1-5: c2RaytoCircle rejects ==================== */

#[test]
fn err_01_raytocircle_disc_negative() {
    let mut rng = Rng::new(1);
    let mut n = 0;
    for _ in 0..2000 {
        // a ray that passes far from the circle: disc = b*b - c < 0
        let center = rng.coord_v();
        let r = rng.range(0.1, 5.0);
        let ang = rng.range(-3.15, 3.15);
        let off = r * rng.range(1.5, 20.0); // lateral offset > r  => miss
        let (ca, sa) = (ang.cos(), ang.sin());
        let ray = C2Ray {
            p: v(center.x - ca * 50.0 - sa * off, center.y - sa * 50.0 + ca * off),
            d: v(ca, sa),
            t: 1000.0,
        };
        let c = C2Circle { p: center, r };
        assert_eq!(circle_path(ray, c), CIRCLE_DISC_NEG, "wrong sub-path");
        assert_rejected_untouched(
            "err_01",
            format!("{} {}", fray(ray), fcircle(c)),
            both_circle(ray, c),
        );
        n += 1;
    }
    assert!(n > 0);
}

#[test]
fn err_02_raytocircle_nan_disc() {
    // every field, one at a time, made NaN -> disc or t is NaN -> reject
    let base_ray = C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 };
    let base_c = C2Circle { p: v(0.0, 0.0), r: 2.0 };
    let nans = [f32::NAN, -f32::NAN, f32::from_bits(0x7F80_0001), f32::from_bits(0xFFFF_FFFF)];
    let mut cases = 0;
    for nan in nans {
        let variants: [(C2Ray, C2Circle, &str); 6] = [
            (C2Ray { p: v(nan, 0.0), ..base_ray }, base_c, "ray.p.x"),
            (C2Ray { p: v(-10.0, nan), ..base_ray }, base_c, "ray.p.y"),
            (C2Ray { d: v(nan, 0.0), ..base_ray }, base_c, "ray.d.x"),
            (C2Ray { t: nan, ..base_ray }, base_c, "ray.t"),
            (base_ray, C2Circle { r: nan, ..base_c }, "circle.r"),
            (base_ray, C2Circle { p: v(nan, nan), ..base_c }, "circle.p"),
        ];
        for (ray, c, what) in variants {
            let r = both_circle(ray, c);
            assert_eq!(r.0, 0, "C accepted a NaN input ({what})");
            assert_rejected_untouched("err_02", format!("{what} = NaN 0x{:08x}", nan.to_bits()), r);
            cases += 1;
        }
    }
    // ... and NaN-heavy random inputs classified as the NaN path
    let found = search(
        "err_02",
        CIRCLE_NAN,
        200,
        |rng| (rng.wild_ray(), rng.wild_circle()),
        |(ray, c)| circle_path(*ray, *c),
    );
    for (ray, c) in found {
        let r = both_circle(ray, c);
        assert_eq!(r.0, 0);
        assert_rejected_untouched("err_02", format!("{} {}", fray(ray), fcircle(c)), r);
        cases += 1;
    }
    eprintln!("[err_02] {cases} NaN rejection cases");
}

#[test]
fn err_03_raytocircle_t_negative() {
    // ray origin already past the circle: the near intersection is behind it
    let mut rng = Rng::new(3);
    let mut n = 0;
    for _ in 0..2000 {
        let center = rng.coord_v();
        let r = rng.range(0.5, 10.0);
        let ang = rng.range(-3.15, 3.15);
        let (ca, sa) = (ang.cos(), ang.sin());
        // origin inside the circle => -b - sqrt(disc) < 0
        let f = rng.range(0.0, 0.99);
        let ray = C2Ray {
            p: v(center.x + ca * r * f, center.y + sa * r * f),
            d: v(ca, sa),
            t: 1000.0,
        };
        let c = C2Circle { p: center, r };
        assert_eq!(circle_path(ray, c), CIRCLE_T_NEG);
        assert_rejected_untouched(
            "err_03",
            format!("{} {}", fray(ray), fcircle(c)),
            both_circle(ray, c),
        );
        n += 1;
    }
    assert!(n > 0);
}

#[test]
fn err_04_raytocircle_t_beyond_len() {
    let mut rng = Rng::new(4);
    let mut n = 0;
    for _ in 0..2000 {
        let center = rng.coord_v();
        let r = rng.range(0.5, 10.0);
        let ang = rng.range(-3.15, 3.15);
        let (ca, sa) = (ang.cos(), ang.sin());
        let dist = r + rng.range(1.0, 50.0);
        let ray_p = v(center.x - ca * dist, center.y - sa * dist);
        // A.t too short to reach the circle, plus the boundary values 0 / <0
        for t in [
            0.0f32,
            -rng.range(0.0, 10.0),
            (dist - r) * rng.range(0.0, 0.99),
            f32::from_bits(0x8000_0000), // -0.0
        ] {
            let ray = C2Ray { p: ray_p, d: v(ca, sa), t };
            let c = C2Circle { p: center, r };
            let path = circle_path(ray, c);
            if path != CIRCLE_T_BEYOND {
                continue; // t<0 cases can classify as t>A.t too; keep it strict
            }
            assert_rejected_untouched(
                "err_04",
                format!("{} {}", fray(ray), fcircle(c)),
                both_circle(ray, c),
            );
            n += 1;
        }
    }
    assert!(n > 100, "only {n} t>A.t cases");
    eprintln!("[err_04] {n} cases");
}

#[test]
fn err_05_raytocircle_null_out_on_miss() {
    // The C only dereferences `out` after it has decided to return 1, so a
    // rejecting call with out == NULL must NOT crash in either library.
    let p = apis();
    let nul: *mut C2Raycast = std::ptr::null_mut();
    let cases = [
        // disc < 0
        (C2Ray { p: v(0.0, 100.0), d: v(1.0, 0.0), t: 100.0 }, C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // t < 0 (origin inside)
        (C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 100.0 }, C2Circle { p: v(0.0, 0.0), r: 5.0 }),
        // t > A.t
        (C2Ray { p: v(-50.0, 0.0), d: v(1.0, 0.0), t: 1.0 }, C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // NaN
        (C2Ray { p: v(f32::NAN, 0.0), d: v(1.0, 0.0), t: 1.0 }, C2Circle { p: v(0.0, 0.0), r: 1.0 }),
    ];
    for (ray, c) in cases {
        // verify it really is a rejection before passing NULL
        let (probe_c, _, probe_r, _) = both_circle(ray, c);
        assert_eq!(probe_r, probe_c, "rust={probe_r} C={probe_c}");
        assert_eq!(probe_c, 0, "test case is not a rejection: {} {}", fray(ray), fcircle(c));
        let rc = unsafe { (p.c.c2RaytoCircle)(ray, c, nul) };
        let rr = unsafe { (p.r.c2RaytoCircle)(ray, c, nul) };
        assert_eq!(rc, 0, "C hit with out==NULL (would have crashed)");
        assert_eq!(rr, rc, "rust={rr} C={rc} for {} {}", fray(ray), fcircle(c));
    }
    // randomized rejecting inputs, verified to reject first
    let mut rng = Rng::new(5);
    let mut n = 0;
    for _ in 0..4000 {
        let ray = rng.nice_ray();
        let c = C2Circle { p: rng.coord_v(), r: rng.radius() };
        let (probe_c, _, probe_r, _) = both_circle(ray, c);
        assert_eq!(probe_r, probe_c);
        if probe_c != 0 {
            continue;
        }
        let rc = unsafe { (p.c.c2RaytoCircle)(ray, c, nul) };
        let rr = unsafe { (p.r.c2RaytoCircle)(ray, c, nul) };
        assert_eq!(rc, 0);
        assert_eq!(rr, rc);
        n += 1;
    }
    assert!(n > 100, "only {n} randomized NULL-out rejects");
    eprintln!("[err_05] {n} randomized NULL-out rejections");
}

/* ==================== rows 6-10: c2AABBtoAABB rejects ==================== */

fn aabb_flags(a: C2AABB, b: C2AABB) -> [bool; 4] {
    [
        b.max.x < a.min.x,
        a.max.x < b.min.x,
        b.max.y < a.min.y,
        a.max.y < b.min.y,
    ]
}

fn check_aabbtoaabb(row: &str, a: C2AABB, b: C2AABB, expect: c_int) {
    let p = apis();
    let rc = unsafe { (p.c.c2AABBtoAABB)(a, b) };
    let rr = unsafe { (p.r.c2AABBtoAABB)(a, b) };
    assert_eq!(rc, expect, "[{row}] C returned {rc}, expected {expect}: {} {}", faabb(a), faabb(b));
    assert_eq!(rr, rc, "[{row}] rust={rr} C={rc}: {} {}", faabb(a), faabb(b));
}

#[test]
fn err_06_aabbtoaabb_d0() {
    let mut rng = Rng::new(6);
    for _ in 0..2000 {
        let a = rng.proper_aabb();
        // B entirely to the left of A  => d0 only
        let w = rng.range(0.1, 10.0);
        let b = C2AABB {
            min: v(a.min.x - w - rng.range(0.01, 10.0), a.min.y),
            max: v(a.min.x - rng.range(0.01, 10.0), a.max.y),
        };
        let f = aabb_flags(a, b);
        if !(f[0] && !f[1] && !f[2] && !f[3]) {
            continue;
        }
        check_aabbtoaabb("err_06", a, b, 0);
    }
}

#[test]
fn err_07_aabbtoaabb_d1() {
    let mut rng = Rng::new(7);
    for _ in 0..2000 {
        let a = rng.proper_aabb();
        let b = C2AABB {
            min: v(a.max.x + rng.range(0.01, 10.0), a.min.y),
            max: v(a.max.x + rng.range(10.01, 20.0), a.max.y),
        };
        let f = aabb_flags(a, b);
        if !(!f[0] && f[1] && !f[2] && !f[3]) {
            continue;
        }
        check_aabbtoaabb("err_07", a, b, 0);
    }
}

#[test]
fn err_08_aabbtoaabb_d2() {
    let mut rng = Rng::new(8);
    for _ in 0..2000 {
        let a = rng.proper_aabb();
        let b = C2AABB {
            min: v(a.min.x, a.min.y - rng.range(10.01, 20.0)),
            max: v(a.max.x, a.min.y - rng.range(0.01, 10.0)),
        };
        let f = aabb_flags(a, b);
        if !(!f[0] && !f[1] && f[2] && !f[3]) {
            continue;
        }
        check_aabbtoaabb("err_08", a, b, 0);
    }
}

#[test]
fn err_09_aabbtoaabb_d3() {
    let mut rng = Rng::new(9);
    for _ in 0..2000 {
        let a = rng.proper_aabb();
        let b = C2AABB {
            min: v(a.min.x, a.max.y + rng.range(0.01, 10.0)),
            max: v(a.max.x, a.max.y + rng.range(10.01, 20.0)),
        };
        let f = aabb_flags(a, b);
        if !(!f[0] && !f[1] && !f[2] && f[3]) {
            continue;
        }
        check_aabbtoaabb("err_09", a, b, 0);
    }
}

#[test]
fn err_10_aabbtoaabb_nan_accepts() {
    // Every `<` against NaN is false, so `!(d0|d1|d2|d3)` == 1: a NaN box is
    // reported as OVERLAPPING.  Both libraries must agree on that.
    let nan = f32::NAN;
    let good = C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) };
    let all_nan = C2AABB { min: v(nan, nan), max: v(nan, nan) };
    check_aabbtoaabb("err_10", all_nan, good, 1);
    check_aabbtoaabb("err_10", good, all_nan, 1);
    check_aabbtoaabb("err_10", all_nan, all_nan, 1);
    // a single NaN coordinate is enough to disable the corresponding test
    let far = C2AABB { min: v(1000.0, 1000.0), max: v(2000.0, 2000.0) };
    check_aabbtoaabb("err_10", good, far, 0); // sanity: disjoint without NaN
    let mut nan_min = far;
    nan_min.min = v(nan, nan);
    check_aabbtoaabb("err_10", good, nan_min, 1);
    let mut nan_max = good;
    nan_max.max = v(nan, nan);
    check_aabbtoaabb("err_10", nan_max, far, 1);
    // also -NaN and signalling NaN payloads
    for bits in [0xFFC0_0000u32, 0x7F80_0001, 0xFFFF_FFFF] {
        let n = f32::from_bits(bits);
        check_aabbtoaabb("err_10", C2AABB { min: v(n, n), max: v(n, n) }, good, 1);
    }
}

/* ==================== rows 11-14: c2RaytoAABB rejects =================== */

#[test]
fn err_11_raytoaabb_broadphase_reject() {
    let mut rng = Rng::new(11);
    let mut n = 0;
    for _ in 0..4000 {
        let b = rng.proper_aabb();
        // a short ray far away on the -x side
        let ray = C2Ray {
            p: v(b.min.x - rng.range(10.0, 100.0), rng.coord()),
            d: v(-1.0, 0.0),
            t: rng.range(0.0, 5.0),
        };
        if aabb_path(ray, b) != AABB_BROAD_REJECT {
            continue;
        }
        assert_rejected_untouched(
            "err_11",
            format!("{} {}", fray(ray), faabb(b)),
            both_aabb(ray, b),
        );
        n += 1;
    }
    assert!(n > 100, "only {n} broadphase rejects");
    eprintln!("[err_11] {n} cases");
}

#[test]
fn err_12_raytoaabb_sat_reject() {
    // d > 0: the box is entirely on one side of the ray's line even though the
    // swept bounding boxes overlap (a diagonal ray past a corner).
    let found = search(
        "err_12",
        AABB_SAT_REJECT,
        500,
        |rng| {
            let b = rng.proper_aabb();
            let ang = rng.range(-3.15, 3.15);
            let ray = C2Ray {
                p: v(b.min.x - rng.range(0.0, 5.0), b.max.y + rng.range(0.0, 5.0)),
                d: v(ang.cos(), ang.sin()),
                t: rng.range(0.0, 100.0),
            };
            (ray, b)
        },
        |(ray, b)| aabb_path(*ray, *b),
    );
    for (ray, b) in &found {
        assert_rejected_untouched(
            "err_12",
            format!("{} {}", fray(*ray), faabb(*b)),
            both_aabb(*ray, *b),
        );
    }
    eprintln!("[err_12] {} cases", found.len());
}

#[test]
fn err_13_raytoaabb_no_plane_hit() {
    // hit == 0: every t_i > 1.0f — reachable when the plane distances are NaN
    // (NaN <= 1.0f is false for all four axes).
    let found = search(
        "err_13",
        AABB_NO_HIT,
        300,
        |rng| (rng.wild_ray(), rng.wild_aabb()),
        |(ray, b)| aabb_path(*ray, *b),
    );
    for (ray, b) in &found {
        assert_rejected_untouched(
            "err_13",
            format!("{} {}", fray(*ray), faabb(*b)),
            both_aabb(*ray, *b),
        );
    }
    eprintln!("[err_13] {} cases", found.len());
}

#[test]
fn err_14_raytoaabb_null_out_on_miss() {
    let p = apis();
    let nul: *mut C2Raycast = std::ptr::null_mut();
    let b = C2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let cases = [
        C2Ray { p: v(-100.0, 0.0), d: v(-1.0, 0.0), t: 5.0 },   // broadphase
        C2Ray { p: v(-5.0, 5.0), d: v(1.0, 1.0), t: 100.0 },     // SAT / diagonal
        C2Ray { p: v(f32::NAN, 0.0), d: v(1.0, 0.0), t: 1.0 },   // NaN
        C2Ray { p: v(0.0, 0.0), d: v(f32::NAN, f32::NAN), t: 1.0 },
    ];
    let mut tested = 0;
    for ray in cases {
        // probe with a real out-buffer first: passing NULL is only defined for
        // inputs the C actually rejects (it writes *out on every hit).
        let (probe_c, _, probe_r, _) = both_aabb(ray, b);
        assert_eq!(probe_r, probe_c, "rust={probe_r} C={probe_c}: {}", fray(ray));
        if probe_c != 0 {
            eprintln!("[err_14] skipping {} — it is a HIT, so NULL out is UB in both", fray(ray));
            continue;
        }
        let rc = unsafe { (p.c.c2RaytoAABB)(ray, b, nul) };
        let rr = unsafe { (p.r.c2RaytoAABB)(ray, b, nul) };
        assert_eq!(rc, 0, "C hit with out == NULL: {}", fray(ray));
        assert_eq!(rr, rc, "rust={rr} C={rc}: {}", fray(ray));
        tested += 1;
    }
    assert!(tested >= 2, "only {tested} rejecting NULL-out cases");
    // plus randomized rejecting rays, verified to reject before going NULL
    let mut rng = Rng::new(14);
    let mut n = 0;
    for _ in 0..4000 {
        let bx = rng.proper_aabb();
        let ray = rng.nice_ray();
        let (probe_c, _, probe_r, _) = both_aabb(ray, bx);
        assert_eq!(probe_r, probe_c);
        if probe_c != 0 {
            continue;
        }
        let rc = unsafe { (p.c.c2RaytoAABB)(ray, bx, nul) };
        let rr = unsafe { (p.r.c2RaytoAABB)(ray, bx, nul) };
        assert_eq!(rc, 0);
        assert_eq!(rr, rc);
        n += 1;
    }
    assert!(n > 100, "only {n} randomized NULL-out rejects");
    eprintln!("[err_14] {} fixed + {n} randomized NULL-out rejections", tested);
}

/* =================== rows 15-18: c2AABBtoPoint rejects ================== */

fn check_aabbtopoint(row: &str, b: C2AABB, q: C2v, expect: c_int) {
    let p = apis();
    let rc = unsafe { (p.c.c2AABBtoPoint)(b, q) };
    let rr = unsafe { (p.r.c2AABBtoPoint)(b, q) };
    assert_eq!(rc, expect, "[{row}] C={rc} expected {expect}: {} {}", faabb(b), fv(q));
    assert_eq!(rr, rc, "[{row}] rust={rr} C={rc}: {} {}", faabb(b), fv(q));
}

#[test]
fn err_15_aabbtopoint_d0() {
    let mut rng = Rng::new(15);
    for _ in 0..2000 {
        let b = rng.proper_aabb();
        let q = v(b.min.x - rng.range(0.001, 50.0), rng.range(b.min.y, b.max.y));
        if !(q.x < b.min.x) || q.y < b.min.y || q.x > b.max.x || q.y > b.max.y {
            continue;
        }
        check_aabbtopoint("err_15", b, q, 0);
    }
}

#[test]
fn err_16_aabbtopoint_d1() {
    let mut rng = Rng::new(16);
    for _ in 0..2000 {
        let b = rng.proper_aabb();
        let q = v(rng.range(b.min.x, b.max.x), b.min.y - rng.range(0.001, 50.0));
        if q.x < b.min.x || !(q.y < b.min.y) || q.x > b.max.x || q.y > b.max.y {
            continue;
        }
        check_aabbtopoint("err_16", b, q, 0);
    }
}

#[test]
fn err_17_aabbtopoint_d2() {
    let mut rng = Rng::new(17);
    for _ in 0..2000 {
        let b = rng.proper_aabb();
        let q = v(b.max.x + rng.range(0.001, 50.0), rng.range(b.min.y, b.max.y));
        if q.x < b.min.x || q.y < b.min.y || !(q.x > b.max.x) || q.y > b.max.y {
            continue;
        }
        check_aabbtopoint("err_17", b, q, 0);
    }
}

#[test]
fn err_18_aabbtopoint_d3() {
    let mut rng = Rng::new(18);
    for _ in 0..2000 {
        let b = rng.proper_aabb();
        let q = v(rng.range(b.min.x, b.max.x), b.max.y + rng.range(0.001, 50.0));
        if q.x < b.min.x || q.y < b.min.y || q.x > b.max.x || !(q.y > b.max.y) {
            continue;
        }
        check_aabbtopoint("err_18", b, q, 0);
    }
}

/* ==================== row 19: c2CircleToPoint rejects =================== */

#[test]
fn err_19_circletopoint_outside() {
    let p = apis();
    let mut rng = Rng::new(19);
    let check = |c: C2Circle, q: C2v, expect: Option<c_int>| {
        let rc = unsafe { (p.c.c2CircleToPoint)(c, q) };
        let rr = unsafe { (p.r.c2CircleToPoint)(c, q) };
        if let Some(e) = expect {
            assert_eq!(rc, e, "C={rc} expected {e}: {} {}", fcircle(c), fv(q));
        }
        assert_eq!(rr, rc, "rust={rr} C={rc}: {} {}", fcircle(c), fv(q));
    };
    // exact, representable rim points: d2 == r*r, so `d2 < r*r` is false
    for (cx, cy, r, qx, qy) in [
        (0.0f32, 0.0f32, 3.0f32, 3.0f32, 0.0f32),
        (0.0, 0.0, 4.0, 0.0, 4.0),
        (0.0, 0.0, 5.0, 3.0, 4.0),   // 3-4-5 triangle: exactly on the rim
        (1.0, 2.0, 5.0, 4.0, 6.0),   // translated 3-4-5
        (0.0, 0.0, 5.0, -3.0, -4.0),
        (0.0, 0.0, 0.5, 0.5, 0.0),
    ] {
        check(C2Circle { p: v(cx, cy), r }, v(qx, qy), Some(0));
        // one ulp inside is a hit
        let inside = v(qx - (qx - cx) * f32::EPSILON, qy);
        check(C2Circle { p: v(cx, cy), r }, inside, None);
    }
    for _ in 0..2000 {
        let center = rng.coord_v();
        let ang = rng.range(-3.15, 3.15);
        let (ca, sa) = (ang.cos(), ang.sin());
        // r == 0: nothing is ever inside, not even the centre (0 < 0 is false)
        let c0 = C2Circle { p: center, r: 0.0 };
        check(c0, center, Some(0));
        check(c0, v(center.x + ca, center.y + sa), Some(0));
        // on the rim: `center + r*(cos,sin)` is only approximately on the rim
        // for random floats, so only C/rust agreement is required here
        let r = rng.range(0.5, 20.0);
        let c = C2Circle { p: center, r };
        check(c, v(center.x + ca * r, center.y + sa * r), None);
        // outside
        check(c, v(center.x + ca * r * 1.5, center.y + sa * r * 1.5), Some(0));
        // NEGATIVE radius: r*r > 0, so it behaves like |r| (NOT an error)
        let cn = C2Circle { p: center, r: -r };
        check(cn, center, Some(1));
        check(cn, v(center.x + ca * r * 0.5, center.y + sa * r * 0.5), Some(1));
        check(cn, v(center.x + ca * r * 1.5, center.y + sa * r * 1.5), Some(0));
        // NaN radius / NaN point -> every comparison false -> 0
        check(C2Circle { p: center, r: f32::NAN }, center, Some(0));
        check(c, v(f32::NAN, 0.0), Some(0));
        // inf radius accepts everything finite
        check(C2Circle { p: center, r: f32::INFINITY }, rng.coord_v(), Some(1));
        // random specials: only C/rust agreement is required
        check(rng.wild_circle(), rng.wild_v(), None);
    }
}

/* =================== rows 20-23: c2RaytoCapsule rejects ================= */

#[test]
fn err_20_raytocapsule_fallthrough_writes_out() {
    // The fall-through `return 0` happens AFTER out->n / out->t were written at
    // L243/244, so `*out` is modified even though the function reports a miss.
    let found = search(
        "err_20",
        CAP_FALLTHROUGH,
        1000,
        |rng| {
            let r = rng.range(0.1, 3.0);
            let len = rng.range(1.0, 30.0);
            let phi = rng.range(-3.15, 3.15);
            let my = v(phi.cos(), phi.sin());
            let a = rng.coord_v();
            let cap = C2Capsule { a, b: v(a.x + my.x * len, a.y + my.y * len), r };
            let ray = C2Ray { p: rng.coord_v(), d: rng.coord_v(), t: rng.range(0.0, 20.0) };
            (ray, cap)
        },
        |(ray, cap)| capsule_path(*ray, *cap),
    );
    let p = apis();
    for (ray, cap) in &found {
        let (rc, oc, rr, or_) = both_capsule(*ray, *cap);
        assert_eq!(rc, 0, "expected a miss");
        assert_eq!(rr, 0, "rust={rr} C={rc}");
        assert!(
            !is_poison(oc),
            "the C left *out untouched on the capsule fall-through path: {oc:?}"
        );
        assert!(
            same_cast(oc, or_),
            "out differs on the fall-through path: C={oc:?} rust={or_:?} for {} {}",
            fray(*ray),
            fcap(*cap)
        );
        // it must be exactly the pre-write: t = 0 and n = c2Norm(b - a)
        let expect_n = unsafe { (p.c.c2Norm)((p.c.c2Sub)(cap.b, cap.a)) };
        assert_eq!(oc.t.to_bits(), 0u32, "t should be +0.0, got {:?}", oc.t);
        assert_eq!(oc.n.x.to_bits(), expect_n.x.to_bits());
        assert_eq!(oc.n.y.to_bits(), expect_n.y.to_bits());
    }
    eprintln!("[err_20] {} cases", found.len());
}

#[test]
fn err_21_raytocapsule_delegated_miss() {
    // c2RaytoCapsule delegates to c2RaytoCircle, which can itself reject; the
    // out-struct then keeps the L243/244 pre-write.
    let mut n = 0;
    let p = apis();
    for want in [CAP_DELEG_A_BY_X, CAP_DELEG_B_BY_X, CAP_DELEG_A_BY_Y, CAP_DELEG_B_BY_Y] {
        let found = search(
            "err_21",
            want,
            2000,
            |rng| {
                let r = rng.range(0.1, 3.0);
                let len = rng.range(1.0, 30.0);
                let phi = rng.range(-3.15, 3.15);
                let my = v(phi.cos(), phi.sin());
                let a = rng.coord_v();
                let cap = C2Capsule { a, b: v(a.x + my.x * len, a.y + my.y * len), r };
                let ray = C2Ray {
                    p: rng.coord_v(),
                    d: rng.coord_v(),
                    t: rng.range(0.0, 20.0),
                };
                (ray, cap)
            },
            |(ray, cap)| capsule_path(*ray, *cap),
        );
        for (ray, cap) in &found {
            let (rc, oc, rr, or_) = both_capsule(*ray, *cap);
            assert_eq!(rr, rc, "rust={rr} C={rc} for {} {}", fray(*ray), fcap(*cap));
            assert!(same_cast(oc, or_), "out differs: C={oc:?} rust={or_:?}");
            if rc == 0 {
                // rejected by the delegated c2RaytoCircle: out is the pre-write
                let expect_n = unsafe { (p.c.c2Norm)((p.c.c2Sub)(cap.b, cap.a)) };
                assert_eq!(oc.t.to_bits(), 0u32);
                assert_eq!(oc.n.x.to_bits(), expect_n.x.to_bits());
                assert_eq!(oc.n.y.to_bits(), expect_n.y.to_bits());
                n += 1;
            }
        }
    }
    assert!(n > 100, "only {n} delegated misses");
    eprintln!("[err_21] {n} delegated-miss cases");
}

#[test]
fn err_22_raytocapsule_degenerate_ab() {
    // a == b -> c2Norm(0,0) = (NaN,NaN) -> the whole frame is NaN.
    let mut rng = Rng::new(22);
    let mut ret1 = 0;
    for _ in 0..2000 {
        let a = rng.coord_v();
        let cap = C2Capsule { a, b: a, r: rng.radius() };
        let ray = if rng.below(2) == 0 { rng.nice_ray() } else { rng.wild_ray() };
        let (rc, oc, rr, or_) = both_capsule(ray, cap);
        assert_eq!(rr, rc, "rust={rr} C={rc} for {} {}", fray(ray), fcap(cap));
        assert!(
            same_cast(oc, or_),
            "out differs for a degenerate capsule: C={oc:?} rust={or_:?} ({} {})",
            fray(ray),
            fcap(cap)
        );
        if rc != 0 {
            ret1 += 1;
        }
    }
    // ±0 sized capsule with ±0 radius, and a == b at the origin
    for (ax, ay, r) in [
        (0.0f32, 0.0f32, 0.0f32),
        (-0.0, -0.0, -0.0),
        (1.0, 1.0, f32::NAN),
        (f32::INFINITY, 0.0, 1.0),
    ] {
        let cap = C2Capsule { a: v(ax, ay), b: v(ax, ay), r };
        let ray = C2Ray { p: v(0.5, 0.5), d: v(1.0, 0.0), t: 10.0 };
        let (rc, oc, rr, or_) = both_capsule(ray, cap);
        assert_eq!(rr, rc, "rust={rr} C={rc} for {}", fcap(cap));
        assert!(same_cast(oc, or_), "C={oc:?} rust={or_:?} for {}", fcap(cap));
    }
    eprintln!("[err_22] {ret1}/2000 degenerate capsules reported a hit (both libs agree)");
}

/* ================== rows 23/25: NULL-pointer SIGSEGV =================== */

/// Runs this same test binary as a child process, in the crash-probe mode
/// selected by `$SPEC_RAY_CRASH`, and reports how the child died.
struct Fault {
    code: Option<i32>,
    signal: Option<i32>,
    /// Rust's `-C debug-assertions=on` builds turn a null-pointer dereference
    /// into a non-unwinding panic (SIGABRT) instead of letting the hardware
    /// raise SIGSEGV.  Detected from the child's stderr.
    rust_null_check: bool,
}

fn crash_probe(target: &str) -> Fault {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "crash_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env("SPEC_RAY_CRASH", target)
        .output()
        .expect("spawn crash child");
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        !err.contains("NO CRASH"),
        "the {target} probe did NOT fault:\n{err}"
    );
    Fault {
        code: out.status.code(),
        signal: out.status.signal(),
        rust_null_check: err.contains("null pointer dereference occurred"),
    }
}

/// Both libraries must fault on the same invalid pointer, in the same way.
fn assert_same_fault(row: &str, what: &str, c: &Fault, r: &Fault) {
    eprintln!(
        "[{row}] {what}: C code={:?} signal={:?} | rust code={:?} signal={:?} \
         (rust debug null-check: {})",
        c.code, c.signal, r.code, r.signal, r.rust_null_check
    );
    assert_eq!(
        c.signal,
        Some(11),
        "[{row}] {what}: the C did not SIGSEGV (code={:?})",
        c.code
    );
    if r.rust_null_check {
        // `cargo build` (dev profile) enables debug assertions, which detect the
        // null dereference and abort deliberately instead of faulting.  Same
        // rejection, reported by Rust's own runtime check; the release cdylib —
        // the artifact an external consumer links — SIGSEGVs exactly like the C.
        assert_eq!(
            r.signal,
            Some(6),
            "[{row}] {what}: rust reported the null dereference but died with {:?}",
            r.signal
        );
    } else {
        assert_eq!(
            r.signal, c.signal,
            "[{row}] {what}: rust signal {:?} != C signal {:?}",
            r.signal, c.signal
        );
    }
}

/// Not run normally: `crash_probe` invokes this in a child process.
#[test]
#[ignore]
fn crash_child() {
    let target = match std::env::var("SPEC_RAY_CRASH") {
        Ok(t) => t,
        Err(_) => return,
    };
    let p = apis();
    let nul_out: *mut C2Raycast = std::ptr::null_mut();
    let ray = C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 10.0 };
    let cap = C2Capsule { a: v(0.0, -1.0), b: v(0.0, 1.0), r: 1.0 };
    let circle = C2Circle { p: v(5.0, 0.0), r: 1.0 };
    eprintln!("[crash_child] target={target}");
    let r = unsafe {
        match target.as_str() {
            "capsule_null_out_c" => (p.c.c2RaytoCapsule)(ray, cap, nul_out),
            "capsule_null_out_rust" => (p.r.c2RaytoCapsule)(ray, cap, nul_out),
            "castray_null_shape_c" => {
                (p.c.c2CastRay)(ray, std::ptr::null(), C2_TYPE_CIRCLE, nul_out)
            }
            "castray_null_shape_rust" => {
                (p.r.c2CastRay)(ray, std::ptr::null(), C2_TYPE_CIRCLE, nul_out)
            }
            "castray_null_shape_aabb_c" => {
                (p.c.c2CastRay)(ray, std::ptr::null(), C2_TYPE_AABB, nul_out)
            }
            "castray_null_shape_aabb_rust" => {
                (p.r.c2CastRay)(ray, std::ptr::null(), C2_TYPE_AABB, nul_out)
            }
            // not a crash: report the UB return value of the invalid switch
            "castray_bad_type_c" => {
                let mut o = POISON;
                let circ = C2Circle { p: v(0.0, 0.0), r: 2.0 };
                let sh = &circ as *const C2Circle as *const std::ffi::c_void;
                let got = (p.c.c2CastRay)(ray, sh, 3, &mut o);
                println!("UB_RETURN={got}");
                assert!(
                    o.t.to_bits() == POISON.t.to_bits(),
                    "*out was written for an invalid typeB"
                );
                return;
            }
            // control: a hit that writes through a NULL out pointer
            "circle_hit_null_out_c" => {
                (p.c.c2RaytoCircle)(ray, circle, nul_out)
            }
            "circle_hit_null_out_rust" => {
                (p.r.c2RaytoCircle)(ray, circle, nul_out)
            }
            other => panic!("unknown crash target {other}"),
        }
    };
    eprintln!("[crash_child] NO CRASH, returned {r}");
}

#[test]
fn err_23_raytocapsule_null_out_segv() {
    // c2RaytoCapsule writes *out unconditionally (L243) before any check, so a
    // NULL out is an immediate fault in BOTH libraries.
    let c = crash_probe("capsule_null_out_c");
    let r = crash_probe("capsule_null_out_rust");
    assert_same_fault("err_23", "c2RaytoCapsule(out = NULL)", &c, &r);
}

#[test]
fn err_25_castray_null_shape_segv() {
    for (cn, rn, what) in [
        ("castray_null_shape_c", "castray_null_shape_rust", "c2CastRay(CIRCLE, B = NULL)"),
        (
            "castray_null_shape_aabb_c",
            "castray_null_shape_aabb_rust",
            "c2CastRay(AABB, B = NULL)",
        ),
        // control: a genuine hit writing through a NULL out pointer
        (
            "circle_hit_null_out_c",
            "circle_hit_null_out_rust",
            "c2RaytoCircle(hit, out = NULL)",
        ),
    ] {
        let c = crash_probe(cn);
        let r = crash_probe(rn);
        assert_same_fault("err_25", what, &c, &r);
    }
}

/* ==================== row 24/26: c2CastRay dispatch ==================== */

#[test]
fn err_24_castray_out_of_range_type() {
    // The C `switch` has NO `default` label, so for a type with no valid
    // variant control falls off the end of an `int` function (UB — the compiled
    // code returns whatever is left in eax).  What IS defined and observable is
    // that no branch runs: `*out` must stay untouched and nothing may crash.
    let p = apis();
    let ray = C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 };
    let circle = C2Circle { p: v(0.0, 0.0), r: 2.0 };
    let shape = &circle as *const C2Circle as *const std::ffi::c_void;
    let bad: [c_int; 15] = [
        3,
        4,
        5,
        7,
        255,
        256,
        1000,
        -1,
        -2,
        -1000,
        i32::MAX,
        i32::MIN,
        0x7FFF_FFFF,
        -0x8000_0000,
        0x0001_0000,
    ];
    for t in bad {
        let mut oc = POISON;
        let mut or_ = POISON;
        let rc = unsafe { (p.c.c2CastRay)(ray, shape, t, &mut oc) };
        let rr = unsafe { (p.r.c2CastRay)(ray, shape, t, &mut or_) };
        eprintln!("[err_24] typeB={t:>12}: C returned {rc:>3} (UB), rust returned {rr}");
        assert!(
            is_poison(oc),
            "[err_24] the C wrote *out for the invalid type {t}: {oc:?}"
        );
        assert!(
            is_poison(or_),
            "[err_24] rust wrote *out for the invalid type {t} while the C did not: {or_:?}"
        );
        assert!(
            is_poison(or_) && is_poison(oc),
            "out-parameter effect differs for typeB={t}"
        );
        assert_eq!(rr, 0, "[err_24] rust must return the documented 0, got {rr}");
    }
    // Evidence that the C's return value here is not behaviour that *can* be
    // reproduced: it is whatever the CALLER happened to leave in eax.  Calling
    // the very same function with the very same arguments from three different
    // call sites yields three different "return values".
    let mut observed = Vec::new();
    {
        let mut o = POISON;
        observed.push(unsafe { (p.c.c2CastRay)(ray, shape, 3, &mut o) });
    }
    {
        // prime eax with the result of another call in the same library
        let bx = C2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
        let mut o = POISON;
        let pre = unsafe { (p.c.c2AABBtoAABB)(bx, bx) };
        let got = unsafe { (p.c.c2CastRay)(ray, shape, 3, &mut o) };
        eprintln!("[err_24] after c2AABBtoAABB -> {pre}: c2CastRay(3) returned {got}");
        observed.push(got);
    }
    {
        let mut o = POISON;
        let pre = unsafe { (p.c.c2CircleToPoint)(circle, v(1000.0, 1000.0)) };
        let got = unsafe { (p.c.c2CastRay)(ray, shape, 3, &mut o) };
        eprintln!("[err_24] after c2CircleToPoint -> {pre}: c2CastRay(3) returned {got}");
        observed.push(got);
    }
    let distinct: std::collections::BTreeSet<c_int> = observed.iter().copied().collect();
    eprintln!(
        "[err_24] the C returned {:?} for typeB=3 from {} different call sites => {}",
        observed,
        observed.len(),
        if distinct.len() > 1 {
            "PROVEN caller-dependent (uninitialised eax), no behaviour to reproduce"
        } else {
            "the same leftover value from these call sites (still UB: the switch has no default)"
        }
    );

    // A valid type immediately afterwards must still work in both libraries —
    // the invalid call must not have corrupted any state.
    let mut oc = POISON;
    let mut or_ = POISON;
    let rc = unsafe { (p.c.c2CastRay)(ray, shape, C2_TYPE_CIRCLE, &mut oc) };
    let rr = unsafe { (p.r.c2CastRay)(ray, shape, C2_TYPE_CIRCLE, &mut or_) };
    assert_eq!(rc, 1);
    assert_eq!(rr, rc);
    assert!(same_cast(oc, or_), "C={oc:?} rust={or_:?}");
}

/// Runs the `castray_bad_type_c` probe in a fresh process and returns the value
/// the C "returned" for the invalid switch value.
fn ub_return_probe() -> i64 {
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "crash_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env("SPEC_RAY_CRASH", "castray_bad_type_c")
        .output()
        .expect("spawn probe");
    let s = String::from_utf8_lossy(&out.stdout);
    // libtest prints "test crash_child ... " without a newline, so the marker
    // can appear in the middle of a line.
    let idx = s
        .find("UB_RETURN=")
        .unwrap_or_else(|| panic!("probe produced no UB_RETURN:\n{s}"));
    s[idx + "UB_RETURN=".len()..]
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

/// Documents (and proves) that the C's return value for an out-of-range
/// `C2_TYPE` is not reproducible behaviour: it changes from process to process.
#[test]
fn err_24b_castray_ub_return_is_not_reproducible() {
    let vals: Vec<i64> = (0..5).map(|_| ub_return_probe()).collect();
    let distinct: std::collections::BTreeSet<i64> = vals.iter().copied().collect();
    eprintln!(
        "[err_24b] the C 'returned' {vals:?} for typeB=3 in 5 separate processes \
         ({} distinct values)",
        distinct.len()
    );
    eprintln!(
        "[err_24b] => falling off the end of the non-void c2CastRay leaves the return \
         value in eax, which varies with ASLR; there is no behaviour to reproduce, so the \
         Rust translation returns a deterministic 0.  The DEFINED part of the contract \
         (no *out write, no crash, subsequent valid calls unaffected) is asserted in err_24."
    );
    // Whatever the values are, the C must never have written *out (checked in
    // the probe itself, which asserts and would have exited non-zero).
    assert_eq!(vals.len(), 5);
}

#[test]
fn err_26_castray_delegated_miss() {
    // A valid typeB whose shape rejects: the dispatcher must return the
    // delegate's 0 and leave *out exactly as the direct call would.
    let p = apis();
    let mut rng = Rng::new(26);
    let mut misses = 0;
    for _ in 0..4000 {
        let ray = rng.nice_ray();
        let which = rng.below(3);
        let (rc, oc, rr, or_, direct_c, dout_c) = unsafe {
            match which {
                0 => {
                    let c = C2Circle { p: rng.coord_v(), r: rng.radius() };
                    let s = &c as *const C2Circle as *const std::ffi::c_void;
                    let (mut a, mut b, mut d) = (POISON, POISON, POISON);
                    let x = (p.c.c2CastRay)(ray, s, C2_TYPE_CIRCLE, &mut a);
                    let y = (p.r.c2CastRay)(ray, s, C2_TYPE_CIRCLE, &mut b);
                    let z = (p.c.c2RaytoCircle)(ray, c, &mut d);
                    (x, a, y, b, z, d)
                }
                1 => {
                    let c = rng.proper_aabb();
                    let s = &c as *const C2AABB as *const std::ffi::c_void;
                    let (mut a, mut b, mut d) = (POISON, POISON, POISON);
                    let x = (p.c.c2CastRay)(ray, s, C2_TYPE_AABB, &mut a);
                    let y = (p.r.c2CastRay)(ray, s, C2_TYPE_AABB, &mut b);
                    let z = (p.c.c2RaytoAABB)(ray, c, &mut d);
                    (x, a, y, b, z, d)
                }
                _ => {
                    let c = C2Capsule { a: rng.coord_v(), b: rng.coord_v(), r: rng.radius() };
                    let s = &c as *const C2Capsule as *const std::ffi::c_void;
                    let (mut a, mut b, mut d) = (POISON, POISON, POISON);
                    let x = (p.c.c2CastRay)(ray, s, C2_TYPE_CAPSULE, &mut a);
                    let y = (p.r.c2CastRay)(ray, s, C2_TYPE_CAPSULE, &mut b);
                    let z = (p.c.c2RaytoCapsule)(ray, c, &mut d);
                    (x, a, y, b, z, d)
                }
            }
        };
        assert_eq!(rr, rc, "dispatch: rust={rr} C={rc}");
        assert_eq!(rc, direct_c, "dispatch != direct call in the C itself");
        assert!(same_cast(oc, or_), "dispatch out differs: C={oc:?} rust={or_:?}");
        assert!(same_cast(oc, dout_c), "dispatch out != direct out in the C");
        if rc == 0 {
            misses += 1;
        }
    }
    assert!(misses > 100, "only {misses} delegated misses");
    eprintln!("[err_26] {misses} delegated-miss cases");
}

/* ============ rows 27-30: degenerate arithmetic (div/len/norm) ========== */

fn check_div(row: &str, a: C2v, b: f32) {
    let p = apis();
    let rc = unsafe { (p.c.c2Div)(a, b) };
    let rr = unsafe { (p.r.c2Div)(a, b) };
    let eq = |x: f32, y: f32| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan());
    assert!(
        eq(rc.x, rr.x) && eq(rc.y, rr.y),
        "[{row}] c2Div({}, {:e}): C={} rust={}",
        fv(a),
        b,
        fv(rc),
        fv(rr)
    );
}

#[test]
fn err_27_div_by_zero() {
    let mut rng = Rng::new(27);
    let p = apis();
    for _ in 0..2000 {
        let a = rng.coord_v();
        check_div("err_27", a, 0.0);
        check_div("err_27", v(0.0, 0.0), 0.0);
        check_div("err_27", v(f32::INFINITY, -f32::INFINITY), 0.0);
        // c2Norm of the zero vector: len = 0 -> 1/0 = inf -> 0*inf = NaN
        for z in [v(0.0, 0.0), v(-0.0, -0.0), v(0.0, -0.0)] {
            let rc = unsafe { (p.c.c2Norm)(z) };
            let rr = unsafe { (p.r.c2Norm)(z) };
            assert!(rc.x.is_nan() && rc.y.is_nan(), "C c2Norm(0,0) = {}", fv(rc));
            assert!(
                rr.x.is_nan() && rr.y.is_nan(),
                "rust c2Norm(0,0) = {} but C = {}",
                fv(rr),
                fv(rc)
            );
        }
        // exact bit equality for the non-NaN lanes
        let a2 = v(rng.coord(), 0.0);
        let rc = unsafe { (p.c.c2Div)(a2, 0.0) };
        let rr = unsafe { (p.r.c2Div)(a2, 0.0) };
        assert_eq!(rc.x.to_bits(), rr.x.to_bits(), "C={} rust={}", fv(rc), fv(rr));
    }
}

#[test]
fn err_28_div_by_negative_zero() {
    let mut rng = Rng::new(28);
    for _ in 0..2000 {
        let a = rng.coord_v();
        check_div("err_28", a, -0.0);
        check_div("err_28", v(1.0, -1.0), -0.0);
        check_div("err_28", v(-0.0, 0.0), -0.0);
        check_div("err_28", rng.wild_v(), -0.0);
        // the sign of the infinity must match: 1/-0 = -inf
        let p = apis();
        let rc = unsafe { (p.c.c2Div)(v(1.0, -2.0), -0.0) };
        let rr = unsafe { (p.r.c2Div)(v(1.0, -2.0), -0.0) };
        assert_eq!(rc.x.to_bits(), f32::NEG_INFINITY.to_bits());
        assert_eq!(rc.x.to_bits(), rr.x.to_bits());
        assert_eq!(rc.y.to_bits(), rr.y.to_bits());
    }
}

#[test]
fn err_29_len_overflow_inf() {
    let p = apis();
    let cases = [
        v(f32::MAX, f32::MAX),
        v(f32::MAX, 0.0),
        v(1.0e30, 1.0e30),
        v(f32::INFINITY, 0.0),
        v(-f32::INFINITY, f32::INFINITY),
        v(f32::MIN, f32::MIN),
        v(1.0e-45, 1.0e-45), // underflow to +0 -> len 0
        v(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
    ];
    for a in cases {
        let lc = unsafe { (p.c.c2Len)(a) };
        let lr = unsafe { (p.r.c2Len)(a) };
        assert_eq!(lc.to_bits(), lr.to_bits(), "c2Len({}) C={lc:e} rust={lr:e}", fv(a));
        let nc = unsafe { (p.c.c2Norm)(a) };
        let nr = unsafe { (p.r.c2Norm)(a) };
        let eq = |x: f32, y: f32| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan());
        assert!(
            eq(nc.x, nr.x) && eq(nc.y, nr.y),
            "c2Norm({}) C={} rust={}",
            fv(a),
            fv(nc),
            fv(nr)
        );
    }
    // len of an overflowing vector is +inf, and normalizing it yields ±0
    let big = v(f32::MAX, f32::MAX);
    assert_eq!(unsafe { (p.c.c2Len)(big) }.to_bits(), f32::INFINITY.to_bits());
    let n = unsafe { (p.c.c2Norm)(big) };
    assert_eq!(n.x.to_bits(), 0u32, "expected +0.0, got {:e}", n.x);
}

#[test]
fn err_30_len_nan() {
    let p = apis();
    let mut rng = Rng::new(30);
    for _ in 0..4000 {
        let a = match rng.below(3) {
            0 => v(f32::NAN, rng.coord()),
            1 => v(rng.coord(), -f32::NAN),
            _ => v(f32::from_bits(rng.next_u32() | 0x7F80_0000), rng.coord()),
        };
        let lc = unsafe { (p.c.c2Len)(a) };
        let lr = unsafe { (p.r.c2Len)(a) };
        assert_eq!(
            lc.is_nan(),
            lr.is_nan(),
            "c2Len({}) C={lc:e}/0x{:08x} rust={lr:e}/0x{:08x}",
            fv(a),
            lc.to_bits(),
            lr.to_bits()
        );
        let dc = unsafe { (p.c.c2Dot)(a, a) };
        let dr = unsafe { (p.r.c2Dot)(a, a) };
        assert_eq!(dc.is_nan(), dr.is_nan(), "c2Dot({0}, {0})", fv(a));
        if !dc.is_nan() {
            assert_eq!(dc.to_bits(), dr.to_bits());
        }
    }
}

/* ======================= rows 31-33: spec_ray ========================== */

fn check_spec(row: &str, a: [f32; 7], expect: Option<c_int>) -> c_int {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let rc = unsafe { (p.c.spec_ray)(&mut oc, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
    let rr = unsafe { (p.r.spec_ray)(&mut or_, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
    assert_eq!(rr, rc, "[{row}] rust={rr} C={rc} for {a:?}");
    assert!(
        same_cast(oc, or_),
        "[{row}] out differs: C={oc:?} rust={or_:?} for {a:?}"
    );
    if let Some(e) = expect {
        assert_eq!(rc, e, "[{row}] C returned {rc}, expected {e} for {a:?}");
    }
    rc
}

#[test]
fn err_31_spec_ray_degenerate_direction() {
    // mp == ray.p  ->  c2Norm(0,0) = NaN  ->  ray.d = NaN  ->  reject
    let mut rng = Rng::new(31);
    let p = apis();
    for _ in 0..2000 {
        let q = rng.coord_v();
        let c = rng.coord_v();
        let r = rng.radius();
        let mut oc = POISON;
        let mut or_ = POISON;
        let rc = unsafe { (p.c.spec_ray)(&mut oc, q.x, q.y, c.x, c.y, r, q.x, q.y) };
        let rr = unsafe { (p.r.spec_ray)(&mut or_, q.x, q.y, c.x, c.y, r, q.x, q.y) };
        assert_eq!(rc, 0, "the C reported a hit for a degenerate direction");
        assert_eq!(rr, rc, "rust={rr} C={rc}");
        assert!(is_poison(oc) && is_poison(or_), "*cast was written: {oc:?} {or_:?}");
    }
    // and the same with ±0 / NaN / inf coordinates
    for (x, y) in [
        (0.0f32, 0.0f32),
        (-0.0, 0.0),
        (f32::NAN, 1.0),
        (f32::INFINITY, f32::INFINITY),
        (f32::MAX, f32::MIN),
    ] {
        check_spec("err_31", [x, y, 0.0, 0.0, 1.0, x, y], Some(0));
    }
}

#[test]
fn err_32_spec_ray_negative_radius() {
    // a negative radius is squared, so the circle behaves like |r| — not an
    // error, and the two libraries must agree bit for bit.
    let mut rng = Rng::new(32);
    let mut hits = 0;
    for _ in 0..4000 {
        let center = rng.coord_v();
        let r = rng.range(0.5, 20.0);
        let ang = rng.range(-3.15, 3.15);
        let (ca, sa) = (ang.cos(), ang.sin());
        let d0 = r + rng.range(0.1, 30.0);
        let d1 = r + rng.range(0.1, 30.0);
        let rp = v(center.x - ca * d0, center.y - sa * d0);
        let mp = v(center.x + ca * d1, center.y + sa * d1);
        let pos = check_spec("err_32", [mp.x, mp.y, center.x, center.y, r, rp.x, rp.y], None);
        let neg = check_spec("err_32", [mp.x, mp.y, center.x, center.y, -r, rp.x, rp.y], None);
        assert_eq!(pos, neg, "|r| and -|r| must behave the same");
        if pos != 0 {
            hits += 1;
        }
        // -0.0 radius and NaN radius
        check_spec("err_32", [mp.x, mp.y, center.x, center.y, -0.0, rp.x, rp.y], None);
        check_spec("err_32", [mp.x, mp.y, center.x, center.y, f32::NAN, rp.x, rp.y], Some(0));
        check_spec(
            "err_32",
            [mp.x, mp.y, center.x, center.y, f32::NEG_INFINITY, rp.x, rp.y],
            None,
        );
    }
    assert!(hits > 100);
    eprintln!("[err_32] {hits} negative-radius hits agreed");
}

#[test]
fn err_33_spec_ray_null_cast_on_miss() {
    // On a miss `spec_ray` never dereferences `cast`, so NULL is safe in both.
    let p = apis();
    let nul: *mut C2Raycast = std::ptr::null_mut();
    let cases: [[f32; 7]; 6] = [
        // circle far away laterally -> disc < 0
        [10.0, 0.0, 0.0, 100.0, 1.0, -10.0, 0.0],
        // mouse point short of the circle -> t > A.t
        [-5.0, 0.0, 10.0, 0.0, 1.0, -10.0, 0.0],
        // degenerate direction -> NaN
        [1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        // NaN inputs
        [f32::NAN, 0.0, 0.0, 0.0, 1.0, -10.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, f32::NAN, -10.0, 0.0],
        // r = 0 -> disc < 0 unless exactly on the line
        [10.0, 5.0, 0.0, 0.0, 0.0, -10.0, 5.0],
    ];
    for a in cases {
        // probe with a real buffer first — NULL is only safe on the miss path
        assert_eq!(check_spec("err_33", a, Some(0)), 0);
        let rc = unsafe { (p.c.spec_ray)(nul, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        let rr = unsafe { (p.r.spec_ray)(nul, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        assert_eq!(rc, 0, "C reported a hit with cast == NULL for {a:?}");
        assert_eq!(rr, rc, "rust={rr} C={rc} for {a:?}");
    }
    // randomized misses
    let mut rng = Rng::new(33);
    let mut n = 0;
    for _ in 0..4000 {
        let a = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
            rng.coord(),
            rng.coord(),
        ];
        if check_spec("err_33", a, None) != 0 {
            continue;
        }
        let rc = unsafe { (p.c.spec_ray)(nul, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        let rr = unsafe { (p.r.spec_ray)(nul, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        assert_eq!(rc, 0);
        assert_eq!(rr, rc);
        n += 1;
    }
    assert!(n > 100, "only {n} randomized NULL-cast misses");
    eprintln!("[err_33] {n} randomized NULL-cast rejections");
}

/* ============ rows 34-35: specials through every helper ================ */

#[test]
fn err_34_helpers_special_values() {
    // ±inf / NaN / denormal / -0.0 through all 14 vector helpers: the whole f32
    // domain is valid input for this API, so this is the "one past the end of
    // the valid range" class.
    let p = apis();
    let mut ck = Checker::new("err_34_helpers_special_values");
    for a in SPECIALS {
        for b in SPECIALS {
            let va = v(a, b);
            let vb = v(b, a);
            let ctx = || format!("a={} b={}", fv(va), fv(vb));
            unsafe {
                ck.vec("c2V", (p.c.c2V)(a, b), (p.r.c2V)(a, b), &ctx);
                ck.f32("c2Dot", (p.c.c2Dot)(va, vb), (p.r.c2Dot)(va, vb), &ctx);
                ck.f32("c2Len", (p.c.c2Len)(va), (p.r.c2Len)(va), &ctx);
                ck.vec("c2Add", (p.c.c2Add)(va, vb), (p.r.c2Add)(va, vb), &ctx);
                ck.vec("c2Sub", (p.c.c2Sub)(va, vb), (p.r.c2Sub)(va, vb), &ctx);
                ck.vec("c2Mulvs", (p.c.c2Mulvs)(va, b), (p.r.c2Mulvs)(va, b), &ctx);
                ck.vec("c2Div", (p.c.c2Div)(va, b), (p.r.c2Div)(va, b), &ctx);
                ck.vec("c2Norm", (p.c.c2Norm)(va), (p.r.c2Norm)(va), &ctx);
                ck.vec("c2Minv", (p.c.c2Minv)(va, vb), (p.r.c2Minv)(va, vb), &ctx);
                ck.vec("c2Maxv", (p.c.c2Maxv)(va, vb), (p.r.c2Maxv)(va, vb), &ctx);
                ck.vec("c2Skew", (p.c.c2Skew)(va), (p.r.c2Skew)(va), &ctx);
                ck.vec("c2Absv", (p.c.c2Absv)(va), (p.r.c2Absv)(va), &ctx);
                ck.vec("c2CCW90", (p.c.c2CCW90)(va), (p.r.c2CCW90)(va), &ctx);
                let m = C2m { x: va, y: vb };
                ck.vec("c2MulmvT", (p.c.c2MulmvT)(m, va), (p.r.c2MulmvT)(m, va), &ctx);
                let bx = C2AABB { min: va, max: vb };
                ck.int("c2AABBtoAABB", (p.c.c2AABBtoAABB)(bx, bx), (p.r.c2AABBtoAABB)(bx, bx), &ctx);
                ck.int("c2AABBtoPoint", (p.c.c2AABBtoPoint)(bx, va), (p.r.c2AABBtoPoint)(bx, va), &ctx);
                let ci = C2Circle { p: va, r: b };
                ck.int(
                    "c2CircleToPoint",
                    (p.c.c2CircleToPoint)(ci, vb),
                    (p.r.c2CircleToPoint)(ci, vb),
                    &ctx,
                );
            }
        }
    }
    ck.finish();
}

#[test]
fn err_35_minv_maxv_absv_nan_asymmetry() {
    // The C uses raw ternaries, NOT fminf/fmaxf/fabsf:
    //   c2Minv: `a.x < b.x ? a.x : b.x`  -> NaN in `a` yields `b`, NaN in `b`
    //   yields NaN.  c2Absv: `a < 0 ? -a : a` -> -NaN stays -NaN (the sign is
    //   NOT cleared, unlike fabsf).
    let p = apis();
    let nan = f32::NAN; // 0x7fc00000
    let neg_nan = -f32::NAN; // 0xffc00000
    let x = 1.5f32;

    let m1 = unsafe { (p.c.c2Minv)(v(nan, nan), v(x, x)) };
    let m1r = unsafe { (p.r.c2Minv)(v(nan, nan), v(x, x)) };
    assert_eq!(m1.x.to_bits(), x.to_bits(), "C c2Minv(NaN, x) should be x");
    assert_eq!(m1r.x.to_bits(), m1.x.to_bits());
    assert_eq!(m1r.y.to_bits(), m1.y.to_bits());

    let m2 = unsafe { (p.c.c2Minv)(v(x, x), v(nan, nan)) };
    let m2r = unsafe { (p.r.c2Minv)(v(x, x), v(nan, nan)) };
    assert!(m2.x.is_nan(), "C c2Minv(x, NaN) should be NaN");
    assert_eq!(m2r.x.to_bits(), m2.x.to_bits());

    let x1 = unsafe { (p.c.c2Maxv)(v(nan, nan), v(x, x)) };
    let x1r = unsafe { (p.r.c2Maxv)(v(nan, nan), v(x, x)) };
    assert_eq!(x1.x.to_bits(), x.to_bits(), "C c2Maxv(NaN, x) should be x");
    assert_eq!(x1r.x.to_bits(), x1.x.to_bits());

    let x2 = unsafe { (p.c.c2Maxv)(v(x, x), v(nan, nan)) };
    let x2r = unsafe { (p.r.c2Maxv)(v(x, x), v(nan, nan)) };
    assert!(x2.x.is_nan(), "C c2Maxv(x, NaN) should be NaN");
    assert_eq!(x2r.x.to_bits(), x2.x.to_bits());

    // c2Absv keeps the NaN sign bit and maps -0.0 to -0.0 (`-0.0 < 0` is false)
    for a in [nan, neg_nan, -0.0f32, 0.0f32, -1.0, f32::NEG_INFINITY] {
        let c = unsafe { (p.c.c2Absv)(v(a, a)) };
        let r = unsafe { (p.r.c2Absv)(v(a, a)) };
        assert_eq!(
            c.x.to_bits(),
            r.x.to_bits(),
            "c2Absv(0x{:08x}): C=0x{:08x} rust=0x{:08x}",
            a.to_bits(),
            c.x.to_bits(),
            r.x.to_bits()
        );
    }
    let neg_zero = unsafe { (p.c.c2Absv)(v(-0.0, -0.0)) };
    assert_eq!(
        neg_zero.x.to_bits(),
        0x8000_0000,
        "the C ternary must keep -0.0 as -0.0 (fabsf would return +0.0)"
    );
    let nn = unsafe { (p.c.c2Absv)(v(neg_nan, neg_nan)) };
    assert_eq!(nn.x.to_bits(), 0xFFC0_0000, "the C ternary must keep -NaN");

    // exhaustive cross product of the specials, both operand orders
    let mut ck = Checker::new("err_35_minv_maxv_absv_nan_asymmetry");
    for a in SPECIALS {
        for b in SPECIALS {
            let (va, vb) = (v(a, b), v(b, a));
            let ctx = || format!("a={} b={}", fv(va), fv(vb));
            unsafe {
                ck.vec("c2Minv(a,b)", (p.c.c2Minv)(va, vb), (p.r.c2Minv)(va, vb), &ctx);
                ck.vec("c2Minv(b,a)", (p.c.c2Minv)(vb, va), (p.r.c2Minv)(vb, va), &ctx);
                ck.vec("c2Maxv(a,b)", (p.c.c2Maxv)(va, vb), (p.r.c2Maxv)(va, vb), &ctx);
                ck.vec("c2Maxv(b,a)", (p.c.c2Maxv)(vb, va), (p.r.c2Maxv)(vb, va), &ctx);
                ck.vec("c2Absv(a)", (p.c.c2Absv)(va), (p.r.c2Absv)(va), &ctx);
            }
        }
    }
    ck.finish();
}
