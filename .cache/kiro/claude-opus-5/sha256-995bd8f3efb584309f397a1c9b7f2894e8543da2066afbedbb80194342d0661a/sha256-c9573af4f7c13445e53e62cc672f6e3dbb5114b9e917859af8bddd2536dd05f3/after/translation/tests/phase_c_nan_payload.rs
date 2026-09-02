//! Phase C (generic boundary coverage) — NaN *payload* fidelity.
//!
//! `addss`/`mulss` return the operand in the DESTINATION register when both
//! inputs are NaN, and gcc -O0 picks that register per expression in a way the
//! C source does not express. A caller passing Rust's `f32::NAN` (0x7FC00000)
//! while the library internally generates the x86 default `-NaN` (0xFFC00000)
//! therefore observes which operand each site favours.
//!
//! These tests hammer every exported entry point with a set of distinct NaN /
//! inf / signed-zero bit patterns and require the two `.so`s to agree
//! bit-for-bit, which is what pins `src/lib.rs`'s `fx::{add_l,add_r,mul_l,mul_r}`
//! choices to the ones `objdump -d` shows in the C build.
#![allow(non_snake_case)]
mod common;
use common::*;
use std::ffi::c_int;

/// NaN-heavy generator: distinct NaN bit patterns so operand selection shows up.
fn nanish(rng: &mut Rng) -> f32 {
    const SET: [u32; 12] = [
        0x7fc0_0000, // +qNaN (Rust's f32::NAN)
        0xffc0_0000, // -qNaN (the x86 default "indefinite")
        0x7fc0_0001,
        0xffc0_0abc,
        0x7f80_0001, // +sNaN
        0xff80_0001, // -sNaN
        0x7f80_0000, // +inf
        0xff80_0000, // -inf
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x3f80_0000, // 1.0
        0xbf80_0000, // -1.0
    ];
    f32::from_bits(SET[rng.below(SET.len())])
}

fn nv(rng: &mut Rng) -> c2v {
    c2v {
        x: nanish(rng),
        y: nanish(rng),
    }
}

const ITERS: usize = 60_000;

macro_rules! check {
    ($p:expr, $name:literal, $ty:ty, $rng:expr, $mk:expr) => {{
        let (c, r) = $p.get::<$ty>($name);
        let mut bad = 0usize;
        let mut first = String::new();
        for _ in 0..ITERS {
            let args = $mk(&mut $rng);
            unsafe {
                let cv = c.clone();
                let rv = r.clone();
                let a = (*cv)(args.0, args.1);
                let b = (*rv)(args.0, args.1);
                if raw(&a) != raw(&b) {
                    bad += 1;
                    if first.is_empty() {
                        first = format!("{:?} -> C {} R {}", args, hex(&raw(&a)), hex(&raw(&b)));
                    }
                }
            }
        }
        if bad != 0 {
            panic!(
                "{}: {} NaN-payload divergences, first: {}",
                String::from_utf8_lossy($name), bad, first
            );
        }
        bad
    }};
}

#[test]
fn nan_leaves() {
    let p = pair();
    let mut rng = Rng::new(7);
    let mut total = 0usize;

    total += check!(p, b"c2Dot", FnVVF, rng, |r: &mut Rng| (nv(r), nv(r)));
    total += check!(p, b"c2Det2", FnVVF, rng, |r: &mut Rng| (nv(r), nv(r)));
    total += check!(p, b"c2Add", FnVVV, rng, |r: &mut Rng| (nv(r), nv(r)));
    total += check!(p, b"c2Sub", FnVVV, rng, |r: &mut Rng| (nv(r), nv(r)));
    total += check!(p, b"c2Maxv", FnVVV, rng, |r: &mut Rng| (nv(r), nv(r)));
    total += check!(p, b"c2Minv", FnVVV, rng, |r: &mut Rng| (nv(r), nv(r)));

    // one-arg
    {
        for (name, _) in [("c2Len", 0)] {
            let (c, r) = p.get::<FnVF>(name.as_bytes());
            let mut bad = 0;
            for _ in 0..ITERS {
                let a = nv(&mut rng);
                unsafe {
                    if raw(&c(a)) != raw(&r(a)) {
                        bad += 1;
                    }
                }
            }
            println!("{name:<28} {bad} diffs");
            total += bad;
        }
        for name in ["c2Neg", "c2Skew", "c2CCW90", "c2Absv", "c2Norm"] {
            let (c, r) = p.get::<FnVV>(name.as_bytes());
            let mut bad = 0;
            for _ in 0..ITERS {
                let a = nv(&mut rng);
                unsafe {
                    if raw(&c(a)) != raw(&r(a)) {
                        bad += 1;
                    }
                }
            }
            println!("{name:<28} {bad} diffs");
            total += bad;
        }
    }

    // c2Mulvs / c2Div (c2v, f32)
    for name in ["c2Mulvs", "c2Div"] {
        let (c, r) = p.get::<FnVFV>(name.as_bytes());
        let mut bad = 0;
        for _ in 0..ITERS {
            let a = nv(&mut rng);
            let b = nanish(&mut rng);
            unsafe {
                if raw(&c(a, b)) != raw(&r(a, b)) {
                    bad += 1;
                }
            }
        }
        println!("{name:<28} {bad} diffs");
        total += bad;
    }

    // c2Mulrv / c2MulrvT
    for name in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = p.get::<FnRVV>(name.as_bytes());
        let mut bad = 0;
        let mut first = String::new();
        for _ in 0..ITERS {
            let rr = c2r {
                c: nanish(&mut rng),
                s: nanish(&mut rng),
            };
            let b = nv(&mut rng);
            unsafe {
                let x = c(rr, b);
                let y = r(rr, b);
                if raw(&x) != raw(&y) {
                    bad += 1;
                    if first.is_empty() {
                        first = format!(
                            "r=({:08x},{:08x}) b=({:08x},{:08x}) C={} R={}",
                            rr.c.to_bits(), rr.s.to_bits(), b.x.to_bits(), b.y.to_bits(),
                            hex(&raw(&x)), hex(&raw(&y))
                        );
                    }
                }
            }
        }
        println!("{name:<28} {bad} diffs {first}");
        total += bad;
    }

    // c2Mulxv / c2MulxvT
    for name in ["c2Mulxv", "c2MulxvT"] {
        let (c, r) = p.get::<FnXVV>(name.as_bytes());
        let mut bad = 0;
        for _ in 0..ITERS {
            let x = c2x {
                p: nv(&mut rng),
                r: c2r { c: nanish(&mut rng), s: nanish(&mut rng) },
            };
            let b = nv(&mut rng);
            unsafe {
                if raw(&c(x, b)) != raw(&r(x, b)) {
                    bad += 1;
                }
            }
        }
        println!("{name:<28} {bad} diffs");
        total += bad;
    }

    // c2Intersect
    {
        let (c, r) = p.get::<FnIntersect>(b"c2Intersect");
        let mut bad = 0;
        for _ in 0..ITERS {
            let (a, b, da, db) = (nv(&mut rng), nv(&mut rng), nanish(&mut rng), nanish(&mut rng));
            unsafe {
                if raw(&c(a, b, da, db)) != raw(&r(a, b, da, db)) {
                    bad += 1;
                }
            }
        }
        println!("{:<28} {bad} diffs", "c2Intersect");
        total += bad;
    }

    // c2Clampv
    {
        let (c, r) = p.get::<FnVVVV>(b"c2Clampv");
        let mut bad = 0;
        for _ in 0..ITERS {
            let (a, lo, hi) = (nv(&mut rng), nv(&mut rng), nv(&mut rng));
            unsafe {
                if raw(&c(a, lo, hi)) != raw(&r(a, lo, hi)) {
                    bad += 1;
                }
            }
        }
        println!("{:<28} {bad} diffs", "c2Clampv");
        total += bad;
    }

    // c22 / c23 with NaN simplices
    for name in ["c22", "c23"] {
        let (c, r) = p.get::<FnSimplex>(name.as_bytes());
        let mut bad = 0;
        for _ in 0..ITERS {
            let mut s = c2Simplex::default();
            s.count = if name == "c22" { 2 } else { 3 };
            for k in 0..4 {
                s.verts[k].p = nv(&mut rng);
                s.verts[k].sA = nv(&mut rng);
                s.verts[k].sB = nv(&mut rng);
                s.verts[k].u = nanish(&mut rng);
                s.verts[k].iA = rng.below(8) as c_int;
                s.verts[k].iB = rng.below(8) as c_int;
            }
            s.div = nanish(&mut rng);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            if raw(&cs) != raw(&rs) {
                bad += 1;
            }
        }
        println!("{name:<28} {bad} diffs");
        total += bad;
    }

    // c2Witness / c2L / c2D / metric
    {
        let (cw, rw) = p.get::<FnWitness>(b"c2Witness");
        let (cl, rl) = p.get::<FnSimplexV>(b"c2L");
        let (cd, rd) = p.get::<FnSimplexV>(b"c2D");
        let (cm, rm) = p.get::<FnSimplexF>(b"c2GJKSimplexMetric");
        let mut bad = [0usize; 4];
        for i in 0..ITERS {
            let mut s = c2Simplex::default();
            s.count = (i % 5) as c_int;
            for k in 0..4 {
                s.verts[k].p = nv(&mut rng);
                s.verts[k].sA = nv(&mut rng);
                s.verts[k].sB = nv(&mut rng);
                s.verts[k].u = nanish(&mut rng);
            }
            s.div = nanish(&mut rng);
            let (mut a1, mut b1, mut a2, mut b2) = (c2v::default(), c2v::default(), c2v::default(), c2v::default());
            let mut s1 = s;
            let mut s2 = s;
            unsafe {
                cw(&mut s1, &mut a1, &mut b1);
                rw(&mut s2, &mut a2, &mut b2);
                if raw(&a1) != raw(&a2) || raw(&b1) != raw(&b2) {
                    bad[0] += 1;
                }
                let mut s1 = s;
                let mut s2 = s;
                if raw(&cl(&mut s1)) != raw(&rl(&mut s2)) {
                    bad[1] += 1;
                }
                let mut s1 = s;
                let mut s2 = s;
                if raw(&cd(&mut s1)) != raw(&rd(&mut s2)) {
                    bad[2] += 1;
                }
                let mut s1 = s;
                let mut s2 = s;
                if raw(&cm(&mut s1)) != raw(&rm(&mut s2)) {
                    bad[3] += 1;
                }
            }
        }
        println!("{:<28} {} diffs", "c2Witness", bad[0]);
        println!("{:<28} {} diffs", "c2L", bad[1]);
        println!("{:<28} {} diffs", "c2D", bad[2]);
        println!("{:<28} {} diffs", "c2GJKSimplexMetric", bad[3]);
        total += bad.iter().sum::<usize>();
    }

    println!("TOTAL LEAF DIFFS = {total}");
    assert_eq!(total, 0, "NaN-payload divergences remain");
}

#[test]
fn nan_manifolds() {
    let p = pair();
    let mut rng = Rng::new(11);
    let mut total = 0usize;

    macro_rules! mani {
        ($name:literal, $ty:ty, $mk:expr) => {{
            let (c, r) = p.get::<$ty>($name);
            let mut bad = 0usize;
            let mut first = String::new();
            for i in 0..ITERS {
                let (x, y) = $mk(&mut rng);
                let mut cm = poison_manifold(i as u8);
                let mut rm = cm;
                scrub_stack();
                unsafe { c(x, y, &mut cm) };
                scrub_stack();
                unsafe { r(x, y, &mut rm) };
                if raw(&cm) != raw(&rm) {
                    bad += 1;
                    if first.is_empty() {
                        first = format!("C={} R={}", hex(&raw(&cm)), hex(&raw(&rm)));
                    }
                }
            }
            println!("{:<32} {bad} diffs {first}", String::from_utf8_lossy($name));
            total += bad;
        }};
    }

    let circ = |r: &mut Rng| c2Circle { p: nv(r), r: nanish(r) };
    let bb = |r: &mut Rng| c2AABB { min: nv(r), max: nv(r) };
    let cap = |r: &mut Rng| c2Capsule { a: nv(r), b: nv(r), r: nanish(r) };

    mani!(b"c2CircletoCircleManifold", FnCC, |r: &mut Rng| (circ(r), circ(r)));
    mani!(b"c2CircletoAABBManifold", FnCA, |r: &mut Rng| (circ(r), bb(r)));
    mani!(b"c2CircletoCapsuleManifold", FnCCap, |r: &mut Rng| (circ(r), cap(r)));
    mani!(b"c2AABBtoAABBManifold", FnAA, |r: &mut Rng| (bb(r), bb(r)));
    mani!(b"c2AABBtoCapsuleManifold", FnACap, |r: &mut Rng| (bb(r), cap(r)));
    mani!(b"c2CapsuletoCapsuleManifold", FnCapCap, |r: &mut Rng| (cap(r), cap(r)));

    println!("TOTAL MANIFOLD DIFFS = {total}");
    assert_eq!(total, 0, "NaN-payload divergences remain in manifolds");
}
