//! High-volume adversarial fuzz across every entry point.
//!
//! Each scalar field is drawn independently from a mixture that is deliberately
//! biased towards the values that expose operand-ordering bugs: raw 32-bit
//! patterns, quiet/signalling NaNs with distinct payloads, signed zeros and
//! infinities.  This is the test that caught the original `c2Add` / `c2Mulvs` /
//! `c2Dot` / `c2MulmvT` / `c2SignedDist*` / `c2RayToPlane*` operand-order
//! mistakes, so it is run with a large iteration count.

mod common;
use common::*;

const N: usize = 200_000;

/// Independent, maximally nasty scalar.
fn nasty(rng: &mut Rng) -> f32 {
    match rng.below(12) {
        0 => f32::from_bits(rng.next_u32()),
        1 => f32::from_bits(0x7f80_0000 | (rng.next_u32() & 0x007f_ffff)), // +NaN/inf family
        2 => f32::from_bits(0xff80_0000 | (rng.next_u32() & 0x007f_ffff)), // -NaN/inf family
        3 => f32::from_bits(0x7fc0_0000 | (rng.next_u32() & 0x003f_ffff)), // +qNaN
        4 => f32::from_bits(0xffc0_0000 | (rng.next_u32() & 0x003f_ffff)), // -qNaN
        5 => 0.0,
        6 => -0.0,
        7 => f32::INFINITY,
        8 => f32::NEG_INFINITY,
        9 => f32::from_bits(rng.next_u32() & 0x807f_ffff), // subnormal
        _ => rng.nice(),
    }
}

fn nasty_v(rng: &mut Rng) -> c2v {
    c2v {
        x: nasty(rng),
        y: nasty(rng),
    }
}

fn nasty_ray(rng: &mut Rng) -> c2Ray {
    c2Ray {
        p: nasty_v(rng),
        d: nasty_v(rng),
        t: nasty(rng),
    }
}

#[test]
fn torture_leaf_ops() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x7001);
    for _ in 0..N {
        let a = nasty_v(&mut rng);
        let b = nasty_v(&mut rng);
        let s = nasty(&mut rng);
        let m = c2m {
            x: nasty_v(&mut rng),
            y: nasty_v(&mut rng),
        };
        macro_rules! chk_v {
            ($f:ident, $($arg:expr),*) => {{
                let x = (c.$f)($($arg),*);
                let y = (r.$f)($($arg),*);
                d.check(v_eq(x, y), || format!(
                    "{}({}, {}, s={}): C={} RUST={}",
                    stringify!($f), fmt_v(a), fmt_v(b), fmt_f(s), fmt_v(x), fmt_v(y)));
            }};
        }
        chk_v!(c2Add, a, b);
        chk_v!(c2Sub, a, b);
        chk_v!(c2Minv, a, b);
        chk_v!(c2Maxv, a, b);
        chk_v!(c2Mulvs, a, s);
        chk_v!(c2Div, a, s);
        chk_v!(c2Norm, a);
        chk_v!(c2Skew, a);
        chk_v!(c2Absv, a);
        chk_v!(c2CCW90, a);
        chk_v!(c2MulmvT, m, b);
        let x = (c.c2Dot)(a, b);
        let y = (r.c2Dot)(a, b);
        d.check(f_eq(x, y), || {
            format!(
                "c2Dot({}, {}): C={} RUST={}",
                fmt_v(a),
                fmt_v(b),
                fmt_f(x),
                fmt_f(y)
            )
        });
        let x = (c.c2Len)(a);
        let y = (r.c2Len)(a);
        d.check(f_eq(x, y), || {
            format!("c2Len({}): C={} RUST={}", fmt_v(a), fmt_f(x), fmt_f(y))
        });
        let ib = c2AABB { min: a, max: b };
        let ic = c2AABB { min: b, max: a };
        d.ints("c2AABBtoAABB", || format!("{:?} {:?}", ib, ic),
            (c.c2AABBtoAABB)(ib, ic), (r.c2AABBtoAABB)(ib, ic));
        d.ints("c2AABBtoPoint", || format!("{:?} {:?}", ib, b),
            (c.c2AABBtoPoint)(ib, b), (r.c2AABBtoPoint)(ib, b));
        let circ = c2Circle { p: a, r: s };
        d.ints("c2CircleToPoint", || format!("{:?} {:?}", circ, b),
            (c.c2CircleToPoint)(circ, b), (r.c2CircleToPoint)(circ, b));
    }
    d.finish("torture leaf ops");
}

#[test]
fn torture_ray_circle() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x7002);
    for _ in 0..N {
        let a = nasty_ray(&mut rng);
        let b = c2Circle {
            p: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        d.ray(
            "torture/circle",
            || format!("{:?} {:?}", a, b),
            call_circle(c, a, b),
            call_circle(r, a, b),
        );
    }
    d.finish("torture c2RaytoCircle");
}

#[test]
fn torture_ray_aabb() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x7003);
    for _ in 0..N {
        let a = nasty_ray(&mut rng);
        let b = c2AABB {
            min: nasty_v(&mut rng),
            max: nasty_v(&mut rng),
        };
        d.ray(
            "torture/aabb",
            || format!("{:?} {:?}", a, b),
            call_aabb(c, a, b),
            call_aabb(r, a, b),
        );
    }
    d.finish("torture c2RaytoAABB");
}

#[test]
fn torture_ray_capsule() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x7004);
    for _ in 0..N {
        let a = nasty_ray(&mut rng);
        let b = c2Capsule {
            a: nasty_v(&mut rng),
            b: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        d.ray(
            "torture/capsule",
            || format!("{:?} {:?}", a, b),
            call_capsule(c, a, b),
            call_capsule(r, a, b),
        );
    }
    d.finish("torture c2RaytoCapsule");
}

#[test]
fn torture_castray_and_gen_ray() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x7005);
    for i in 0..N {
        // c2CastRay with random bytes and a valid type
        let mut buf = [0u8; 20];
        for k in 0..5 {
            buf[k * 4..k * 4 + 4].copy_from_slice(&nasty(&mut rng).to_ne_bytes());
        }
        let a = nasty_ray(&mut rng);
        let ty = (i % 3) as i32;
        d.ray(
            "torture/castray",
            || format!("ty={} {:?} {:?}", ty, a, buf),
            call_castray(c, a, &buf, ty),
            call_castray(r, a, &buf, ty),
        );

        // gen_ray with 16 nasty parameters
        let mut args: GenRayArgs = [0.0; 16];
        for v in args.iter_mut() {
            *v = nasty(&mut rng);
        }
        d.gen_cmp(
            "torture/gen_ray",
            || format!("{:?}", args),
            call_gen_ray(c, &args),
            call_gen_ray(r, &args),
        );
    }
    d.finish("torture c2CastRay + gen_ray");
}
