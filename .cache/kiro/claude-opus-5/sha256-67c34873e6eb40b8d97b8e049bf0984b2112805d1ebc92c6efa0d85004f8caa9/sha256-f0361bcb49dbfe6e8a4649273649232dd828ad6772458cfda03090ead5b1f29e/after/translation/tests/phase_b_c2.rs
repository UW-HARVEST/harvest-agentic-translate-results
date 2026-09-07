//! Phase B — rows C1..C12: the tinyc2 geometry layer.
//!
//! Every low-level entry point (`c2V` .. `c2AABBtoAABB`) is called DIRECTLY
//! through its own `.so` export, and then again through the `f2` dispatcher,
//! because `f2`'s `AABB x CIRCLE` arm swaps its arguments and that composition
//! is invisible to per-wrapper tests.

mod harness;

use harness::*;

const N: usize = 20_000;

fn assert_v2_eq(what: &str, ctx: &str, c: C2v, r: C2v) {
    assert_eq!(
        (c.x.to_bits(), c.y.to_bits()),
        (r.x.to_bits(), r.y.to_bits()),
        "{what} diverged for {ctx}: C=({:#010x},{:#010x}) Rust=({:#010x},{:#010x})",
        c.x.to_bits(),
        c.y.to_bits(),
        r.x.to_bits(),
        r.y.to_bits()
    );
}

/// A pool of `c2v` values mixing seeded-random full-bit-space floats with the
/// hand-picked specials (zeros, subnormals, infinities, NaN payloads).
fn v2_pool(rng: &mut Rng) -> Vec<C2v> {
    let sp = all_special_f32();
    let mut out = Vec::new();
    for &x in &sp {
        for &y in &sp {
            out.push(C2v { x, y });
        }
    }
    for _ in 0..N {
        out.push(C2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        });
    }
    for _ in 0..N {
        out.push(C2v {
            x: rng.tame_f32(100.0),
            y: rng.tame_f32(100.0),
        });
    }
    out
}

// ---------------------------------------------------------------------- C1
#[test]
fn c1_c2v() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 1);
    let sp = all_special_f32();
    for &x in &sp {
        for &y in &sp {
            unsafe {
                assert_v2_eq("c2V", &format!("({x:?},{y:?})"), (p.c.c2V)(x, y), (p.r.c2V)(x, y));
            }
        }
    }
    for _ in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        unsafe {
            assert_v2_eq(
                "c2V",
                &format!("bits({:#010x},{:#010x})", x.to_bits(), y.to_bits()),
                (p.c.c2V)(x, y),
                (p.r.c2V)(x, y),
            );
        }
    }
}

// ---------------------------------------------------------------- C2, C4, C5
#[test]
fn c2_c4_c5_binary_vector_ops() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 2);
    let pool = v2_pool(&mut rng);
    // Cross a bounded prefix of the special grid with itself, plus the random
    // tail paired up sequentially (full cross product would be quadratic).
    let sp_len = all_special_f32().len() * all_special_f32().len();
    let mut pairs: Vec<(C2v, C2v)> = Vec::new();
    for i in 0..sp_len {
        for j in (0..sp_len).step_by(7) {
            pairs.push((pool[i], pool[j]));
        }
    }
    for i in sp_len..pool.len() - 1 {
        pairs.push((pool[i], pool[i + 1]));
    }
    for (a, b) in pairs {
        let ctx = format!(
            "a=({:#010x},{:#010x}) b=({:#010x},{:#010x})",
            a.x.to_bits(),
            a.y.to_bits(),
            b.x.to_bits(),
            b.y.to_bits()
        );
        unsafe {
            assert_v2_eq("c2Maxv", &ctx, (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
            assert_v2_eq("c2Minv", &ctx, (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
            assert_v2_eq("c2Sub", &ctx, (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
            let (dc, dr) = ((p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
            assert_eq!(
                dc.to_bits(),
                dr.to_bits(),
                "c2Dot diverged for {ctx}: C={:#010x} Rust={:#010x}",
                dc.to_bits(),
                dr.to_bits()
            );
        }
    }
}

// ---------------------------------------------------------------------- C3
#[test]
fn c3_c2clampv() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 3);
    let sp = all_special_f32();
    // Triples over the specials (one axis at a time held random) ...
    for &a in &sp {
        for &l in &sp {
            for &h in &sp {
                let av = C2v { x: a, y: -a };
                let lo = C2v { x: l, y: l };
                let hi = C2v { x: h, y: h };
                unsafe {
                    assert_v2_eq(
                        "c2Clampv",
                        &format!("a={a:?} lo={l:?} hi={h:?}"),
                        (p.c.c2Clampv)(av, lo, hi),
                        (p.r.c2Clampv)(av, lo, hi),
                    );
                }
            }
        }
    }
    // ... and fully random, including inverted ranges (lo > hi).
    for _ in 0..N {
        let av = C2v { x: rng.any_f32(), y: rng.any_f32() };
        let lo = C2v { x: rng.any_f32(), y: rng.any_f32() };
        let hi = C2v { x: rng.any_f32(), y: rng.any_f32() };
        unsafe {
            assert_v2_eq(
                "c2Clampv",
                "random",
                (p.c.c2Clampv)(av, lo, hi),
                (p.r.c2Clampv)(av, lo, hi),
            );
        }
    }
    // Explicit inverted / equal ranges with tame numbers.
    for _ in 0..N {
        let x = rng.tame_f32(10.0);
        let lo = C2v { x: 5.0, y: 5.0 };
        let hi = C2v { x: -5.0, y: -5.0 }; // inverted on purpose
        let av = C2v { x, y: x };
        unsafe {
            assert_v2_eq(
                "c2Clampv/inverted",
                "lo>hi",
                (p.c.c2Clampv)(av, lo, hi),
                (p.r.c2Clampv)(av, lo, hi),
            );
        }
        let eq = C2v { x: 1.0, y: 1.0 };
        unsafe {
            assert_v2_eq(
                "c2Clampv/equal",
                "lo==hi",
                (p.c.c2Clampv)(av, eq, eq),
                (p.r.c2Clampv)(av, eq, eq),
            );
        }
    }
}

// ---------------------------------------------------------- circle/aabb pools

fn circle_pool(rng: &mut Rng) -> Vec<C2Circle> {
    let mut out = Vec::new();
    // Hand-picked degenerate shapes.
    for &r in &[0.0f32, -0.0, -1.0, 1.0, 1e-45, f32::MAX, f32::INFINITY, f32::NAN] {
        for &(x, y) in &[(0.0f32, 0.0f32), (1.0, 1.0), (-3.0, 2.0), (f32::INFINITY, 0.0)] {
            out.push(C2Circle { p: C2v { x, y }, r });
        }
    }
    for &b in NAN_BITS {
        out.push(C2Circle {
            p: C2v { x: f32::from_bits(b), y: 1.0 },
            r: f32::from_bits(b),
        });
    }
    for _ in 0..N {
        out.push(C2Circle {
            p: C2v { x: rng.tame_f32(10.0), y: rng.tame_f32(10.0) },
            r: rng.range_f32(-2.0, 5.0),
        });
    }
    for _ in 0..N / 2 {
        out.push(C2Circle {
            p: C2v { x: rng.any_f32(), y: rng.any_f32() },
            r: rng.any_f32(),
        });
    }
    out
}

fn aabb_pool(rng: &mut Rng) -> Vec<C2Aabb> {
    let mut out = Vec::new();
    let corners: &[(f32, f32, f32, f32)] = &[
        (0.0, 0.0, 0.0, 0.0),          // degenerate point
        (-1.0, -1.0, 1.0, 1.0),        // unit box
        (1.0, 1.0, -1.0, -1.0),        // inverted
        (0.0, 0.0, f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY),
        (-0.0, 0.0, 0.0, -0.0),        // signed-zero corners
    ];
    for &(a, b, c, d) in corners {
        out.push(C2Aabb { min: C2v { x: a, y: b }, max: C2v { x: c, y: d } });
    }
    for &nb in NAN_BITS {
        let n = f32::from_bits(nb);
        out.push(C2Aabb { min: C2v { x: n, y: -1.0 }, max: C2v { x: 1.0, y: n } });
    }
    for _ in 0..N {
        let (x0, y0) = (rng.tame_f32(10.0), rng.tame_f32(10.0));
        let (w, h) = (rng.range_f32(-1.0, 6.0), rng.range_f32(-1.0, 6.0));
        out.push(C2Aabb {
            min: C2v { x: x0, y: y0 },
            max: C2v { x: x0 + w, y: y0 + h },
        });
    }
    for _ in 0..N / 2 {
        out.push(C2Aabb {
            min: C2v { x: rng.any_f32(), y: rng.any_f32() },
            max: C2v { x: rng.any_f32(), y: rng.any_f32() },
        });
    }
    out
}

// ---------------------------------------------------------------- C6, C9
#[test]
fn c6_c9_circle_to_circle_direct_and_via_f2() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 6);
    let pool = circle_pool(&mut rng);
    // Exactly-touching pairs: d == rA + rB, so d2 == r2 and the strict `<`
    // must reject.  Also one ULP inside and outside.
    let mut touching = Vec::new();
    for _ in 0..2000 {
        let ra = rng.range_f32(0.1, 4.0);
        let rb = rng.range_f32(0.1, 4.0);
        let cx = rng.tame_f32(5.0);
        let cy = rng.tame_f32(5.0);
        let a = C2Circle { p: C2v { x: cx, y: cy }, r: ra };
        for d in [ra + rb, (ra + rb) * 0.999_999, (ra + rb) * 1.000_001] {
            touching.push((
                a,
                C2Circle { p: C2v { x: cx + d, y: cy }, r: rb },
            ));
        }
    }
    for i in 0..pool.len() {
        let a = pool[i];
        let b = pool[(i * 7 + 13) % pool.len()];
        check_cc(&p, a, b);
    }
    for (a, b) in touching {
        check_cc(&p, a, b);
    }
}

fn check_cc(p: &Pair, a: C2Circle, b: C2Circle) {
    let ctx = format!("A={:?} B={:?}", a, b);
    unsafe {
        // direct low-level entry point
        assert_eq!(
            (p.c.c2CircletoCircle)(a, b),
            (p.r.c2CircletoCircle)(a, b),
            "c2CircletoCircle diverged for {ctx}"
        );
        // and through the f2 dispatcher (row C9)
        let cc = (p.c.f2)(
            &a as *const _ as *const _,
            C2_TYPE_CIRCLE,
            &b as *const _ as *const _,
            C2_TYPE_CIRCLE,
        );
        let rr = (p.r.f2)(
            &a as *const _ as *const _,
            C2_TYPE_CIRCLE,
            &b as *const _ as *const _,
            C2_TYPE_CIRCLE,
        );
        assert_eq!(cc, rr, "f2(CIRCLE,CIRCLE) diverged for {ctx}");
    }
}

// ------------------------------------------------------------ C7, C10, C11
#[test]
fn c7_c10_c11_circle_to_aabb_direct_and_both_f2_orders() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 7);
    let cs = circle_pool(&mut rng);
    let bs = aabb_pool(&mut rng);
    let n = cs.len().max(bs.len());

    // Face / corner / inside / outside placements against a unit box.
    let unit = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let mut placed = Vec::new();
    for &(x, y) in &[
        (0.0f32, 0.0f32),   // inside
        (1.0, 0.0),         // on +x face
        (-1.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (1.0, 1.0),         // on corner
        (-1.0, -1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (2.0, 0.0),         // outside
        (0.0, 5.0),
        (3.0, 3.0),
    ] {
        for &r in &[0.0f32, -0.0, -1.0, 0.5, 1.0, 1.000_001, 1.414_213_6, 2.0] {
            placed.push(C2Circle { p: C2v { x, y }, r });
        }
    }
    for c in &placed {
        check_ca(&p, *c, unit);
    }
    for i in 0..n {
        let c = cs[i % cs.len()];
        let b = bs[(i * 5 + 3) % bs.len()];
        check_ca(&p, c, b);
    }
}

fn check_ca(p: &Pair, c: C2Circle, b: C2Aabb) {
    let ctx = format!("C={:?} B={:?}", c, b);
    unsafe {
        assert_eq!(
            (p.c.c2CircletoAABB)(c, b),
            (p.r.c2CircletoAABB)(c, b),
            "c2CircletoAABB diverged for {ctx}"
        );
        // Row C10: f2(CIRCLE, AABB) -> A is the circle, B the box.
        let a10 = (p.c.f2)(
            &c as *const _ as *const _,
            C2_TYPE_CIRCLE,
            &b as *const _ as *const _,
            C2_TYPE_AABB,
        );
        let b10 = (p.r.f2)(
            &c as *const _ as *const _,
            C2_TYPE_CIRCLE,
            &b as *const _ as *const _,
            C2_TYPE_AABB,
        );
        assert_eq!(a10, b10, "f2(CIRCLE,AABB) diverged for {ctx}");
        // Row C11: f2(AABB, CIRCLE) -> the C SWAPS: reads A as the AABB and
        // B as the circle.  Pass the box first and the circle second.
        let a11 = (p.c.f2)(
            &b as *const _ as *const _,
            C2_TYPE_AABB,
            &c as *const _ as *const _,
            C2_TYPE_CIRCLE,
        );
        let b11 = (p.r.f2)(
            &b as *const _ as *const _,
            C2_TYPE_AABB,
            &c as *const _ as *const _,
            C2_TYPE_CIRCLE,
        );
        assert_eq!(a11, b11, "f2(AABB,CIRCLE) diverged for {ctx}");
        assert_eq!(
            a10, a11,
            "sanity: both f2 orders should agree with each other for {ctx}"
        );
    }
}

// ----------------------------------------------------------------- C8, C12
#[test]
fn c8_c12_aabb_to_aabb_direct_and_via_f2() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 8);
    let bs = aabb_pool(&mut rng);

    // Edge-touching on each of the four sides: `B.max.x == A.min.x` is NOT
    // `<`, so the C reports an overlap.
    let a = C2Aabb {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let mut edge = Vec::new();
    for d in [-1.0f32, -1.000_001, -0.999_999, 0.0, 1.0, 1.000_001, 0.999_999] {
        edge.push(C2Aabb {
            min: C2v { x: d, y: 0.0 },
            max: C2v { x: d + 1.0, y: 1.0 },
        });
        edge.push(C2Aabb {
            min: C2v { x: 0.0, y: d },
            max: C2v { x: 1.0, y: d + 1.0 },
        });
    }
    for b in &edge {
        check_aa(&p, a, *b);
    }
    for i in 0..bs.len() {
        let x = bs[i];
        let y = bs[(i * 11 + 5) % bs.len()];
        check_aa(&p, x, y);
    }
}

fn check_aa(p: &Pair, a: C2Aabb, b: C2Aabb) {
    let ctx = format!("A={:?} B={:?}", a, b);
    unsafe {
        assert_eq!(
            (p.c.c2AABBtoAABB)(a, b),
            (p.r.c2AABBtoAABB)(a, b),
            "c2AABBtoAABB diverged for {ctx}"
        );
        let cc = (p.c.f2)(
            &a as *const _ as *const _,
            C2_TYPE_AABB,
            &b as *const _ as *const _,
            C2_TYPE_AABB,
        );
        let rr = (p.r.f2)(
            &a as *const _ as *const _,
            C2_TYPE_AABB,
            &b as *const _ as *const _,
            C2_TYPE_AABB,
        );
        assert_eq!(cc, rr, "f2(AABB,AABB) diverged for {ctx}");
    }
}
