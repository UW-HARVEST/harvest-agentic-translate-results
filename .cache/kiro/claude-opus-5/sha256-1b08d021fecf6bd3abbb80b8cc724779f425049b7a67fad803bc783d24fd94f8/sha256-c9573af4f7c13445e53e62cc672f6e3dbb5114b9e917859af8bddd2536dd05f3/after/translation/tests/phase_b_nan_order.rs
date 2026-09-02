//! Phase B, sharpest edge: NaN **payload propagation** through every
//! commutative float operation the C performs.
//!
//! `addss`/`mulss` return the *destination* operand's payload when both
//! operands are NaN. Source-level operand order does not determine the
//! destination -- the compiler's register allocator does -- so this is the one
//! part of the translation that cannot be derived from the C text alone and
//! must be matched against the reference build's codegen. The Rust crate pins
//! each destination with inline assembly; these tests are what hold it honest.
//!
//! Every input here is a NaN *pair* with distinct payloads, so a wrong
//! destination choice is guaranteed to show up rather than being masked.

mod common;
use common::*;

const N: usize = 150_000;

/// A quiet NaN with a random, non-zero, distinguishable payload.
fn nan_payload(g: &mut Rng) -> f32 {
    let sign = (g.next_u32() & 1) << 31;
    // mantissa in 1..=0x3FFFFF so it is a quiet NaN with a non-zero payload
    let m = 1 + (g.next_u32() % 0x3F_FFFF);
    f32::from_bits(sign | 0x7FC0_0000 | m)
}

/// A NaN drawn from the whole NaN encoding space, including signalling ones.
fn nan_any(g: &mut Rng) -> f32 {
    let sign = (g.next_u32() & 1) << 31;
    let m = 1 + (g.next_u32() % 0x7F_FFFF);
    f32::from_bits(sign | 0x7F80_0000 | m)
}

fn nan_v(g: &mut Rng) -> c2v {
    if g.below(2) == 0 {
        c2v { x: nan_payload(g), y: nan_payload(g) }
    } else {
        c2v { x: nan_any(g), y: nan_any(g) }
    }
}

#[test]
fn n01_dot_nan_pairs() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: c2Dot (2 mulss + 1 addss destinations)");
    let mut g = Rng::new(0xA01);
    for _ in 0..N {
        let a = nan_v(&mut g);
        let b = nan_v(&mut g);
        d.f32_bits(
            || format!("c2Dot({},{})", fv(a), fv(b)),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
    }
    // mixed: only one operand NaN (the payload must survive unchanged)
    for _ in 0..N {
        let a = c2v { x: nan_any(&mut g), y: g.sym(1e3) };
        let b = c2v { x: g.sym(1e3), y: nan_any(&mut g) };
        d.f32_bits(
            || format!("c2Dot mixed({},{})", fv(a), fv(b)),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
        d.f32_bits(
            || format!("c2Dot mixed rev({},{})", fv(b), fv(a)),
            unsafe { (c.c2Dot)(b, a) },
            unsafe { (r.c2Dot)(b, a) },
        );
    }
    d.finish();
}

#[test]
fn n02_add_nan_pairs() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: c2Add (2 addss destinations)");
    let mut g = Rng::new(0xA02);
    for _ in 0..N {
        let a = nan_v(&mut g);
        let b = nan_v(&mut g);
        d.v_bits(
            || format!("c2Add({},{})", fv(a), fv(b)),
            unsafe { (c.c2Add)(a, b) },
            unsafe { (r.c2Add)(a, b) },
        );
        d.v_bits(
            || format!("c2Add rev({},{})", fv(b), fv(a)),
            unsafe { (c.c2Add)(b, a) },
            unsafe { (r.c2Add)(b, a) },
        );
    }
    d.finish();
}

#[test]
fn n03_sub_nan_pairs() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: c2Sub (subss is non-commutative, dest is fixed)");
    let mut g = Rng::new(0xA03);
    for _ in 0..N {
        let a = nan_v(&mut g);
        let b = nan_v(&mut g);
        d.v_bits(
            || format!("c2Sub({},{})", fv(a), fv(b)),
            unsafe { (c.c2Sub)(a, b) },
            unsafe { (r.c2Sub)(a, b) },
        );
        d.v_bits(
            || format!("c2Sub rev({},{})", fv(b), fv(a)),
            unsafe { (c.c2Sub)(b, a) },
            unsafe { (r.c2Sub)(b, a) },
        );
    }
    d.finish();
}

#[test]
fn n04_mulvs_div_nan_pairs() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: c2Mulvs / c2Div (mulss + divss destinations)");
    let mut g = Rng::new(0xA04);
    for _ in 0..N {
        let a = nan_v(&mut g);
        let s = if g.below(2) == 0 {
            nan_any(&mut g)
        } else {
            g.sym(1e3)
        };
        d.v_bits(
            || format!("c2Mulvs({},{:#010x})", fv(a), s.to_bits()),
            unsafe { (c.c2Mulvs)(a, s) },
            unsafe { (r.c2Mulvs)(a, s) },
        );
        d.v_bits(
            || format!("c2Div({},{:#010x})", fv(a), s.to_bits()),
            unsafe { (c.c2Div)(a, s) },
            unsafe { (r.c2Div)(a, s) },
        );
    }
    d.finish();
}

#[test]
fn n05_mulmvt_nan_pairs() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: c2MulmvT (4 mulss + 2 addss destinations)");
    let mut g = Rng::new(0xA05);
    for _ in 0..N {
        let m = c2m {
            x: nan_v(&mut g),
            y: nan_v(&mut g),
        };
        let b = nan_v(&mut g);
        d.v_bits(
            || format!("c2MulmvT({{{},{}}},{})", fv(m.x), fv(m.y), fv(b)),
            unsafe { (c.c2MulmvT)(m, b) },
            unsafe { (r.c2MulmvT)(m, b) },
        );
    }
    // one NaN at a time, so each of the six destinations is isolated
    for i in 0..N {
        let mut m = c2m {
            x: c2v { x: g.sym(10.0), y: g.sym(10.0) },
            y: c2v { x: g.sym(10.0), y: g.sym(10.0) },
        };
        let mut b = c2v { x: g.sym(10.0), y: g.sym(10.0) };
        let p = nan_any(&mut g);
        match i % 6 {
            0 => m.x.x = p,
            1 => m.x.y = p,
            2 => m.y.x = p,
            3 => m.y.y = p,
            4 => b.x = p,
            _ => b.y = p,
        }
        d.v_bits(
            || format!("c2MulmvT single-nan {i}({{{},{}}},{})", fv(m.x), fv(m.y), fv(b)),
            unsafe { (c.c2MulmvT)(m, b) },
            unsafe { (r.c2MulmvT)(m, b) },
        );
    }
    d.finish();
}

#[test]
fn n06_len_norm_nan_pairs() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: c2Len / c2Norm (sqrtf vs sqrtss, then reciprocal-multiply)");
    let mut g = Rng::new(0xA06);
    for _ in 0..N {
        let a = nan_v(&mut g);
        d.f32_bits(
            || format!("c2Len({})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
        d.v_bits(
            || format!("c2Norm({})", fv(a)),
            unsafe { (c.c2Norm)(a) },
            unsafe { (r.c2Norm)(a) },
        );
    }
    // single-NaN vectors, both slots
    for i in 0..N {
        let p = nan_any(&mut g);
        let a = if i % 2 == 0 {
            c2v { x: p, y: g.sym(1e3) }
        } else {
            c2v { x: g.sym(1e3), y: p }
        };
        d.f32_bits(
            || format!("c2Len single({})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
        d.v_bits(
            || format!("c2Norm single({})", fv(a)),
            unsafe { (c.c2Norm)(a) },
            unsafe { (r.c2Norm)(a) },
        );
    }
    d.finish();
}

#[test]
fn n07_minv_maxv_absv_nan() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: ternary min/max/abs (NOT f32::min/max/abs)");
    let mut g = Rng::new(0xA07);
    for i in 0..N {
        let a = nan_v(&mut g);
        let b = if i % 3 == 0 {
            nan_v(&mut g)
        } else {
            c2v { x: g.sym(1e3), y: g.sym(1e3) }
        };
        d.v_bits(
            || format!("c2Minv({},{})", fv(a), fv(b)),
            unsafe { (c.c2Minv)(a, b) },
            unsafe { (r.c2Minv)(a, b) },
        );
        d.v_bits(
            || format!("c2Minv rev({},{})", fv(b), fv(a)),
            unsafe { (c.c2Minv)(b, a) },
            unsafe { (r.c2Minv)(b, a) },
        );
        d.v_bits(
            || format!("c2Maxv({},{})", fv(a), fv(b)),
            unsafe { (c.c2Maxv)(a, b) },
            unsafe { (r.c2Maxv)(a, b) },
        );
        d.v_bits(
            || format!("c2Maxv rev({},{})", fv(b), fv(a)),
            unsafe { (c.c2Maxv)(b, a) },
            unsafe { (r.c2Maxv)(b, a) },
        );
        d.v_bits(
            || format!("c2Absv({})", fv(a)),
            unsafe { (c.c2Absv)(a) },
            unsafe { (r.c2Absv)(a) },
        );
        d.v_bits(
            || format!("c2Skew({})", fv(a)),
            unsafe { (c.c2Skew)(a) },
            unsafe { (r.c2Skew)(a) },
        );
        d.v_bits(
            || format!("c2CCW90({})", fv(a)),
            unsafe { (c.c2CCW90)(a) },
            unsafe { (r.c2CCW90)(a) },
        );
    }
    d.finish();
}

#[test]
fn n08_nan_through_the_raycasts() {
    let (c, r) = pair();
    let mut d = Diff::new("NaN: payloads through the composed raycast pipelines");
    let mut g = Rng::new(0xA08);
    for i in 0..N / 3 {
        let poison = nan_any(&mut g);
        let mut ray = c2Ray {
            p: g.v(20.0),
            d: g.dir(),
            t: g.unit() * 40.0,
        };
        match i % 5 {
            0 => ray.p.x = poison,
            1 => ray.p.y = poison,
            2 => ray.d.x = poison,
            3 => ray.d.y = poison,
            _ => ray.t = poison,
        }
        let mut cir = c2Circle {
            p: g.v(20.0),
            r: g.unit() * 10.0,
        };
        let mut bx = g.aabb(10.0);
        let a = g.v(15.0);
        let u = g.dir();
        let len = 1.0 + g.unit() * 10.0;
        let mut cap = c2Capsule {
            a,
            b: c2v { x: a.x + u.x * len, y: a.y + u.y * len },
            r: 0.1 + g.unit() * 3.0,
        };
        if i % 2 == 0 {
            let p2 = nan_any(&mut g);
            match i % 6 {
                0 => cir.r = p2,
                1 => cir.p.x = p2,
                2 => bx.min.x = p2,
                3 => bx.max.y = p2,
                4 => cap.r = p2,
                _ => cap.b.x = p2,
            }
        }
        cmp_ray_circle(&mut d, c, r, ray, cir);
        cmp_ray_aabb(&mut d, c, r, ray, bx);
        cmp_ray_capsule(&mut d, c, r, ray, cap);
        cmp_spec_ray(&mut d, c, r, ray.p, cir.p, cir.r, ray.d);
    }
    // fully NaN everything
    for _ in 0..N / 6 {
        let ray = c2Ray {
            p: nan_v(&mut g),
            d: nan_v(&mut g),
            t: nan_any(&mut g),
        };
        cmp_ray_circle(
            &mut d,
            c,
            r,
            ray,
            c2Circle { p: nan_v(&mut g), r: nan_any(&mut g) },
        );
        cmp_ray_aabb(
            &mut d,
            c,
            r,
            ray,
            c2AABB { min: nan_v(&mut g), max: nan_v(&mut g) },
        );
        cmp_ray_capsule(
            &mut d,
            c,
            r,
            ray,
            c2Capsule {
                a: nan_v(&mut g),
                b: nan_v(&mut g),
                r: nan_any(&mut g),
            },
        );
        let mp = nan_v(&mut g);
        let cp = nan_v(&mut g);
        cmp_spec_ray(&mut d, c, r, mp, cp, nan_any(&mut g), nan_v(&mut g));
    }
    d.finish();
}
