//! Phase B, levels 0-2: rows B01..B30 of CONFIGS.md.
//!
//! Lowest-level entry points first. Every call goes through `dlsym` on both
//! shared objects; every result is compared bit-for-bit.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Level 0 — pure vector / scalar math
// ---------------------------------------------------------------------------

/// Exercises the whole "cheap vector op" family with one generator.
fn vec_family(
    d: &mut Diff,
    c: &Api,
    r: &Api,
    tag: &str,
    n: usize,
    seed: u64,
    mut vgen: impl FnMut(&mut Rng) -> C2v,
    mut sgen: impl FnMut(&mut Rng) -> f32,
) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let a = vgen(&mut rng);
        let b = vgen(&mut rng);
        let s = sgen(&mut rng);
        let ctx = |f: &str| format!("{tag}#{i}/{f} a={a:?} b={b:?} s={s:?}");
        d.v(&ctx("c2V"), (c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y));
        d.v(&ctx("c2Sub"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        d.v(&ctx("c2Add"), (c.c2Add)(a, b), (r.c2Add)(a, b));
        d.v(&ctx("c2Neg"), (c.c2Neg)(a), (r.c2Neg)(a));
        d.v(&ctx("c2Skew"), (c.c2Skew)(a), (r.c2Skew)(a));
        d.v(&ctx("c2CCW90"), (c.c2CCW90)(a), (r.c2CCW90)(a));
        d.v(&ctx("c2Mulvs"), (c.c2Mulvs)(a, s), (r.c2Mulvs)(a, s));
    }
}

#[test]
fn B01_vector_ops_ordinary() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B01 vector ops, finite +-200");
    vec_family(
        &mut d,
        &c,
        &r,
        "B01",
        4096,
        SEED ^ 1,
        |g| g.v_coord(),
        |g| g.f32_in(-10.0, 10.0),
    );
    d.finish();
}

#[test]
fn B02_vector_ops_huge() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B02 vector ops, huge magnitudes");
    vec_family(
        &mut d,
        &c,
        &r,
        "B02",
        2048,
        SEED ^ 2,
        |g| g.v_huge(),
        |g| g.huge(),
    );
    d.finish();
}

#[test]
fn B03_vector_ops_denormal() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B03 vector ops, denormal / tiny");
    vec_family(
        &mut d,
        &c,
        &r,
        "B03",
        2048,
        SEED ^ 3,
        |g| g.v_tiny(),
        |g| g.tiny(),
    );
    d.finish();
}

#[test]
fn B04_vector_ops_specials_cross_product() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B04 vector ops, full special cross-product");
    // Exhaustive: every special x every special for both components.
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &bx in SPECIALS {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: ay };
                let ctx = |f: &str| format!("B04/{f} a={a:?} b={b:?}");
                d.v(&ctx("c2Sub"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                d.v(&ctx("c2Add"), (c.c2Add)(a, b), (r.c2Add)(a, b));
                d.v(&ctx("c2Neg"), (c.c2Neg)(a), (r.c2Neg)(a));
                d.v(&ctx("c2Skew"), (c.c2Skew)(a), (r.c2Skew)(a));
                d.v(&ctx("c2CCW90"), (c.c2CCW90)(a), (r.c2CCW90)(a));
                d.v(&ctx("c2Mulvs"), (c.c2Mulvs)(a, bx), (r.c2Mulvs)(a, bx));
                d.v(&ctx("c2V"), (c.c2V)(ax, bx), (r.c2V)(ax, bx));
            }
        }
    }
    d.finish();
}

#[test]
fn B05_dot_det2_finite_and_cancellation() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B05 c2Dot/c2Det2 finite + cancellation");
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..8192 {
        let a = rng.v_coord();
        // Half the cases: b nearly parallel / nearly equal to a, which makes
        // c2Det2 and c2Dot cancel catastrophically.
        let b = match rng.below(4) {
            0 => a,
            1 => {
                let k = rng.f32_in(-2.0, 2.0);
                C2v { x: a.x * k, y: a.y * k }
            }
            2 => C2v {
                x: a.x + rng.tiny(),
                y: a.y + rng.tiny(),
            },
            _ => rng.v_coord(),
        };
        let ctx = |f: &str| format!("B05#{i}/{f} a={a:?} b={b:?}");
        d.f32(&ctx("c2Dot"), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
        d.f32(&ctx("c2Det2"), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
        d.f32(&ctx("c2Dot rev"), (c.c2Dot)(b, a), (r.c2Dot)(b, a));
        d.f32(&ctx("c2Det2 rev"), (c.c2Det2)(b, a), (r.c2Det2)(b, a));
    }
    d.finish();
}

#[test]
fn B06_dot_det2_specials() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B06 c2Dot/c2Det2 specials (inf*0 -> NaN)");
    for &ax in SPECIALS {
        for &ay in SPECIALS {
            for &bx in SPECIALS {
                for &by in SPECIALS {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let ctx = |f: &str| format!("B06/{f} a={a:?} b={b:?}");
                    d.f32(&ctx("c2Dot"), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                    d.f32(&ctx("c2Det2"), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B07_len_div_norm() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B07 c2Len/c2Div/c2Norm");
    let mut rng = Rng::new(SEED ^ 7);
    // Deterministic edge cases first.
    let mut fixed = vec![
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 3.0, y: 4.0 },
        C2v { x: f32::MAX, y: f32::MAX },
        C2v { x: f32::INFINITY, y: 0.0 },
        C2v { x: f32::NAN, y: 1.0 },
        C2v { x: 1e-40, y: 1e-40 },
        C2v { x: f32::MIN_POSITIVE, y: 0.0 },
    ];
    for _ in 0..4096 {
        fixed.push(match rng.below(4) {
            0 => rng.v_huge(),
            1 => rng.v_tiny(),
            2 => rng.v_special(),
            _ => rng.v_coord(),
        });
    }
    let divisors: Vec<f32> = {
        let mut v = vec![0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN, f32::MIN_POSITIVE, 1e-40, f32::MAX];
        for _ in 0..8 {
            v.push(rng.f32_in(-50.0, 50.0));
        }
        v
    };
    for (i, &a) in fixed.iter().enumerate() {
        let ctx = |f: &str| format!("B07#{i}/{f} a={a:?}");
        d.f32(&ctx("c2Len"), (c.c2Len)(a), (r.c2Len)(a));
        d.v(&ctx("c2Norm"), (c.c2Norm)(a), (r.c2Norm)(a));
        let b = divisors[i % divisors.len()];
        d.v(
            &format!("B07#{i}/c2Div a={a:?} b={b:?}"),
            (c.c2Div)(a, b),
            (r.c2Div)(a, b),
        );
    }
    d.finish();
}

#[test]
fn B08_minv_maxv_clampv_ordinary() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B08 c2Minv/c2Maxv/c2Clampv, incl. inverted range");
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..8192 {
        let a = rng.v_coord();
        let (lo, hi) = match rng.below(4) {
            0 => {
                let v = rng.v_coord();
                (v, v) // lo == hi
            }
            1 => (rng.v_coord(), rng.v_coord()), // possibly inverted
            _ => {
                let p = rng.v_coord();
                let q = rng.v_coord();
                (
                    C2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                    C2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                )
            }
        };
        let ctx = |f: &str| format!("B08#{i}/{f} a={a:?} lo={lo:?} hi={hi:?}");
        d.v(&ctx("c2Minv"), (c.c2Minv)(a, hi), (r.c2Minv)(a, hi));
        d.v(&ctx("c2Maxv"), (c.c2Maxv)(lo, a), (r.c2Maxv)(lo, a));
        d.v(&ctx("c2Clampv"), (c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
        // argument order matters for ties and NaN
        d.v(&ctx("c2Minv swap"), (c.c2Minv)(hi, a), (r.c2Minv)(hi, a));
        d.v(&ctx("c2Maxv swap"), (c.c2Maxv)(a, lo), (r.c2Maxv)(a, lo));
    }
    d.finish();
}

#[test]
fn B09_minv_maxv_clampv_specials() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B09 c2Minv/c2Maxv/c2Clampv with +-0 and NaN");
    // Exhaustive over the special list for both components of both operands.
    for &ax in SPECIALS {
        for &bx in SPECIALS {
            let a = C2v { x: ax, y: bx };
            let b = C2v { x: bx, y: ax };
            for &cx in SPECIALS {
                let e = C2v { x: cx, y: cx };
                let ctx = |f: &str| format!("B09/{f} a={a:?} b={b:?} c={e:?}");
                d.v(&ctx("c2Minv"), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                d.v(&ctx("c2Maxv"), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                d.v(&ctx("c2Clampv"), (c.c2Clampv)(a, b, e), (r.c2Clampv)(a, b, e));
                d.v(&ctx("c2Clampv2"), (c.c2Clampv)(e, a, b), (r.c2Clampv)(e, a, b));
            }
        }
    }
    d.finish();
}

#[test]
fn B10_identities() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B10 c2RotIdentity/c2xIdentity");
    for _ in 0..4 {
        let (cr, rr) = ((c.c2RotIdentity)(), (r.c2RotIdentity)());
        d.rot("B10/c2RotIdentity", cr, rr);
        let (cx, rx) = ((c.c2xIdentity)(), (r.c2xIdentity)());
        d.xform("B10/c2xIdentity", cx, rx);
    }
    d.finish();
}

fn rot_family(
    d: &mut Diff,
    c: &Api,
    r: &Api,
    tag: &str,
    n: usize,
    seed: u64,
    mut rgen: impl FnMut(&mut Rng) -> C2r,
    mut vgen: impl FnMut(&mut Rng) -> C2v,
) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let rot = rgen(&mut rng);
        let v = vgen(&mut rng);
        let p = vgen(&mut rng);
        let x = C2x { p, r: rot };
        let ctx = |f: &str| format!("{tag}#{i}/{f} rot={rot:?} v={v:?} p={p:?}");
        d.v(&ctx("c2Mulrv"), (c.c2Mulrv)(rot, v), (r.c2Mulrv)(rot, v));
        d.v(&ctx("c2MulrvT"), (c.c2MulrvT)(rot, v), (r.c2MulrvT)(rot, v));
        d.v(&ctx("c2Mulxv"), (c.c2Mulxv)(x, v), (r.c2Mulxv)(x, v));
        // composed: transform then untransform (round trip through both libs)
        let cv = (c.c2MulrvT)(rot, (c.c2Mulrv)(rot, v));
        let rv = (r.c2MulrvT)(rot, (r.c2Mulrv)(rot, v));
        d.v(&ctx("roundtrip"), cv, rv);
    }
}

#[test]
fn B11_rotations_normalised() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B11 c2Mulrv/c2MulrvT/c2Mulxv, normalised rotations");
    rot_family(&mut d, &c, &r, "B11", 4096, SEED ^ 11, |g| g.rot(), |g| g.v_coord());
    d.finish();
}

#[test]
fn B12_rotations_non_normalised_and_specials() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B12 c2Mulrv/c2MulrvT/c2Mulxv, non-normalised + specials");
    rot_family(
        &mut d,
        &c,
        &r,
        "B12a",
        2048,
        SEED ^ 12,
        |g| g.rot_weird(),
        |g| g.v_coord(),
    );
    rot_family(
        &mut d,
        &c,
        &r,
        "B12b",
        2048,
        SEED ^ 13,
        |g| C2r { c: g.any(), s: g.any() },
        |g| g.v_any(),
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// Level 1 — proxy construction
// ---------------------------------------------------------------------------

fn bbverts_case(d: &mut Diff, c: &Api, r: &Api, ctx: &str, bb: C2Aabb) {
    let mut cb: [C2v; 8] = poison(0x11);
    let mut rb: [C2v; 8] = cb;
    let mut cbb = bb;
    let mut rbb = bb;
    unsafe {
        (c.c2BBVerts)(cb.as_mut_ptr(), &mut cbb);
        (r.c2BBVerts)(rb.as_mut_ptr(), &mut rbb);
    }
    d.varr(&format!("{ctx}/out"), &cb, &rb);
    // the input struct must not be modified by either implementation
    d.aabb(&format!("{ctx}/in"), &cbb, &rbb);
}

#[test]
fn B13_bbverts_wellformed() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B13 c2BBVerts, min < max");
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..4096 {
        let p = rng.v_coord();
        let q = rng.v_coord();
        let bb = C2Aabb {
            min: C2v { x: p.x.min(q.x), y: p.y.min(q.y) },
            max: C2v { x: p.x.max(q.x), y: p.y.max(q.y) },
        };
        bbverts_case(&mut d, &c, &r, &format!("B13#{i} bb={bb:?}"), bb);
    }
    d.finish();
}

#[test]
fn B14_bbverts_degenerate_and_specials() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B14 c2BBVerts, degenerate/inverted/specials");
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..2048 {
        let bb = match rng.below(4) {
            0 => {
                let v = rng.v_coord();
                C2Aabb { min: v, max: v }
            }
            1 => C2Aabb { min: rng.v_coord(), max: rng.v_coord() },
            2 => C2Aabb { min: rng.v_special(), max: rng.v_special() },
            _ => C2Aabb { min: rng.v_huge(), max: rng.v_tiny() },
        };
        bbverts_case(&mut d, &c, &r, &format!("B14#{i} bb={bb:?}"), bb);
    }
    d.finish();
}

fn makeproxy_case(d: &mut Diff, c: &Api, r: &Api, ctx: &str, shape: &Shape, ty: c_int, seed: u8) {
    let mut cp: C2Proxy = poison(seed);
    let mut rp: C2Proxy = cp;
    unsafe {
        (c.c2MakeProxy)(shape.as_ptr(), ty, &mut cp);
        (r.c2MakeProxy)(shape.as_ptr(), ty, &mut rp);
    }
    d.proxy(ctx, &cp, &rp);
}

#[test]
fn B15_makeproxy_circle() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B15 c2MakeProxy CIRCLE");
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..4096 {
        let s = Shape::Circle(if rng.chance(6) {
            C2Circle { p: rng.v_special(), r: rng.any() }
        } else {
            rng.circle()
        });
        makeproxy_case(&mut d, &c, &r, &format!("B15#{i} {s:?}"), &s, C2_TYPE_CIRCLE, i as u8);
    }
    d.finish();
}

#[test]
fn B16_makeproxy_aabb() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B16 c2MakeProxy AABB");
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..4096 {
        let s = Shape::Aabb(if rng.chance(6) {
            C2Aabb { min: rng.v_special(), max: rng.v_special() }
        } else {
            rng.aabb()
        });
        makeproxy_case(&mut d, &c, &r, &format!("B16#{i} {s:?}"), &s, C2_TYPE_AABB, i as u8);
    }
    d.finish();
}

#[test]
fn B17_makeproxy_capsule() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B17 c2MakeProxy CAPSULE");
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..4096 {
        let s = Shape::Capsule(if rng.chance(6) {
            C2Capsule { a: rng.v_special(), b: rng.v_special(), r: rng.any() }
        } else {
            rng.capsule()
        });
        makeproxy_case(&mut d, &c, &r, &format!("B17#{i} {s:?}"), &s, C2_TYPE_CAPSULE, i as u8);
    }
    d.finish();
}

#[test]
fn B18_makeproxy_untouched_slots_keep_poison() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B18 c2MakeProxy leaves unwritten verts untouched");
    let mut rng = Rng::new(SEED ^ 25);
    // For each valid type, the number of vertex slots the C actually writes.
    for (ty, written) in [(C2_TYPE_CIRCLE, 1usize), (C2_TYPE_AABB, 4), (C2_TYPE_CAPSULE, 2)] {
        for i in 0..512 {
            let s = shape_of(&mut rng, ty as usize);
            let base: C2Proxy = poison(0x5A_u8.wrapping_add(i as u8));
            let mut cp = base;
            let mut rp = base;
            unsafe {
                (c.c2MakeProxy)(s.as_ptr(), ty, &mut cp);
                (r.c2MakeProxy)(s.as_ptr(), ty, &mut rp);
            }
            d.proxy(&format!("B18 ty={ty}#{i}/proxy"), &cp, &rp);
            // The untouched tail must still hold the poison in *both*.
            for k in written..8 {
                d.v(&format!("B18 ty={ty}#{i}/vert{k} poison kept"), cp.verts[k], base.verts[k]);
                d.v(&format!("B18 ty={ty}#{i}/vert{k} rust poison kept"), rp.verts[k], base.verts[k]);
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Level 2 — simplex primitives
// ---------------------------------------------------------------------------

fn support_row(d: &mut Diff, c: &Api, r: &Api, tag: &str, count: c_int, n: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let mut verts: [C2v; 8] = [C2v::default(); 8];
        let mode = rng.below(5);
        for k in 0..8 {
            verts[k] = match mode {
                0 => rng.v_coord(),
                1 => C2v { x: k as f32, y: 0.0 },        // strictly increasing
                2 => C2v { x: (8 - k) as f32, y: 0.0 },  // strictly decreasing
                3 => C2v { x: 1.0, y: 2.0 },             // all equal -> tie
                _ => rng.v_special(),
            };
        }
        let dir = match rng.below(4) {
            0 => C2v { x: 0.0, y: 0.0 },
            1 => rng.v_special(),
            _ => rng.v_coord(),
        };
        let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, dir) };
        let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, dir) };
        d.int(
            &format!("{tag}#{i} count={count} dir={dir:?} verts={verts:?}"),
            ci,
            ri,
        );
    }
}

#[test]
fn B19_support_count1() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B19 c2Support count=1 (circle proxy)");
    support_row(&mut d, &c, &r, "B19", 1, 2048, SEED ^ 30);
    d.finish();
}

#[test]
fn B20_support_count2() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B20 c2Support count=2 (capsule proxy)");
    support_row(&mut d, &c, &r, "B20", 2, 2048, SEED ^ 31);
    d.finish();
}

#[test]
fn B21_support_count4() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B21 c2Support count=4 (aabb proxy)");
    support_row(&mut d, &c, &r, "B21", 4, 2048, SEED ^ 32);
    d.finish();
}

#[test]
fn B22_support_count8_and_ties() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B22 c2Support count=8, ties and NaN");
    support_row(&mut d, &c, &r, "B22", 8, 4096, SEED ^ 33);
    // Explicit tie / NaN cases.
    let verts = [
        C2v { x: 1.0, y: 1.0 },
        C2v { x: 1.0, y: 1.0 },
        C2v { x: f32::NAN, y: 0.0 },
        C2v { x: f32::INFINITY, y: 0.0 },
        C2v { x: -f32::INFINITY, y: 0.0 },
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: f32::MAX, y: f32::MAX },
    ];
    for &dir in &[
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 0.0, y: 0.0 },
        C2v { x: f32::NAN, y: f32::NAN },
        C2v { x: -1.0, y: -1.0 },
        C2v { x: f32::INFINITY, y: 0.0 },
    ] {
        for count in 1..=8 {
            let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, dir) };
            let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, dir) };
            d.int(&format!("B22 fixed count={count} dir={dir:?}"), ci, ri);
        }
    }
    d.finish();
}

/// Which branch of `c22` a given simplex takes (mirrors the C predicates).
fn c22_branch(s: &C2Simplex) -> usize {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
    let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

#[test]
fn B23_c22_all_branches() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B23 c22, all three branches");
    let mut rng = Rng::new(SEED ^ 40);
    let mut cover = [0u32; 3];
    for i in 0..8192 {
        let mut s = rnd_simplex(&mut rng, 2);
        // occasionally use extreme p values
        if rng.chance(8) {
            s.verts[0].p = rng.v_huge();
            s.verts[1].p = rng.v_tiny();
        }
        if rng.chance(16) {
            s.verts[1].p = s.verts[0].p; // coincident
        }
        cover[c22_branch(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            (c.c22)(&mut cs);
            (r.c22)(&mut rs);
        }
        d.simplex(&format!("B23#{i} in={s:?}"), &cs, &rs);
    }
    d.finish();
    assert!(cover.iter().all(|&x| x >= 20), "c22 branch coverage {cover:?}");
    eprintln!("B23 c22 branch coverage: {cover:?}");
}

/// Which branch of `c23` a given simplex takes (mirrors the C predicates).
fn c23_branch(s: &C2Simplex) -> usize {
    let dot = |a: C2v, b: C2v| a.x * b.x + a.y * b.y;
    let sub = |a: C2v, b: C2v| C2v { x: a.x - b.x, y: a.y - b.y };
    let det = |a: C2v, b: C2v| a.x * b.y - a.y * b.x;
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let e = s.verts[2].p;
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(e, sub(e, b));
    let vBC = dot(b, sub(b, e));
    let uCA = dot(a, sub(a, e));
    let vCA = dot(e, sub(e, a));
    let area = det(sub(b, a), sub(e, a));
    let uABC = det(b, e) * area;
    let vABC = det(e, a) * area;
    let wABC = det(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn B24_c23_all_branches() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B24 c23, all seven branches");
    let mut rng = Rng::new(SEED ^ 41);
    let mut cover = [0u32; 7];
    for i in 0..20000 {
        let mut s = rnd_simplex(&mut rng, 3);
        match rng.below(6) {
            // A triangle straddling the origin makes the interior branch likely.
            0 => {
                s.verts[0].p = C2v { x: rng.f32_in(-6.0, -0.5), y: rng.f32_in(-6.0, -0.5) };
                s.verts[1].p = C2v { x: rng.f32_in(0.5, 6.0), y: rng.f32_in(-6.0, -0.5) };
                s.verts[2].p = C2v { x: rng.f32_in(-3.0, 3.0), y: rng.f32_in(0.5, 6.0) };
            }
            1 => {
                // all on one side -> vertex/edge branches
                s.verts[0].p = C2v { x: rng.f32_in(1.0, 6.0), y: rng.f32_in(-6.0, 6.0) };
                s.verts[1].p = C2v { x: rng.f32_in(1.0, 6.0), y: rng.f32_in(-6.0, 6.0) };
                s.verts[2].p = C2v { x: rng.f32_in(1.0, 6.0), y: rng.f32_in(-6.0, 6.0) };
            }
            2 => {
                s.verts[1].p = s.verts[0].p;
                s.verts[2].p = rng.v_small();
            }
            _ => {}
        }
        cover[c23_branch(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            (c.c23)(&mut cs);
            (r.c23)(&mut rs);
        }
        d.simplex(&format!("B24#{i} in={s:?}"), &cs, &rs);
    }
    d.finish();
    assert!(cover.iter().all(|&x| x >= 10), "c23 branch coverage {cover:?}");
    eprintln!("B24 c23 branch coverage: {cover:?}");
}

#[test]
fn B25_c23_colinear_and_duplicate() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B25 c23 colinear/duplicate (area == 0)");
    let mut rng = Rng::new(SEED ^ 42);
    for i in 0..4096 {
        let mut s = rnd_simplex(&mut rng, 3);
        let base = rng.v_small();
        let dir = rng.v_small();
        let (t0, t1, t2) = (rng.f32_in(-4.0, 4.0), rng.f32_in(-4.0, 4.0), rng.f32_in(-4.0, 4.0));
        s.verts[0].p = C2v { x: base.x + dir.x * t0, y: base.y + dir.y * t0 };
        s.verts[1].p = C2v { x: base.x + dir.x * t1, y: base.y + dir.y * t1 };
        s.verts[2].p = C2v { x: base.x + dir.x * t2, y: base.y + dir.y * t2 };
        if rng.chance(4) {
            s.verts[2].p = s.verts[1].p;
        }
        if rng.chance(6) {
            s.verts[0].p = C2v { x: 0.0, y: 0.0 };
        }
        let mut cs = s;
        let mut rs = s;
        unsafe {
            (c.c23)(&mut cs);
            (r.c23)(&mut rs);
        }
        d.simplex(&format!("B25#{i} in={s:?}"), &cs, &rs);
    }
    d.finish();
}

#[test]
fn B26_simplex_metric() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B26 c2GJKSimplexMetric, count 1/2/3 + others");
    let mut rng = Rng::new(SEED ^ 43);
    for i in 0..8192 {
        let count = match rng.below(6) {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 0,
            4 => 4,
            _ => -1,
        };
        let mut s = rnd_simplex(&mut rng, count);
        if rng.chance(5) {
            s.verts[0].p = rng.v_huge();
            s.verts[1].p = rng.v_huge();
            s.verts[2].p = rng.v_huge();
        }
        let mut cs = s;
        let mut rs = s;
        let cv = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
        let rv = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
        d.f32(&format!("B26#{i} count={count} in={s:?}"), cv, rv);
        d.simplex(&format!("B26#{i} struct untouched"), &cs, &rs);
    }
    d.finish();
}

#[test]
fn B27_c2D() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B27 c2D, count 1/2/3 + others");
    let mut rng = Rng::new(SEED ^ 44);
    for i in 0..8192 {
        let count = match rng.below(6) {
            0 => 1,
            1 | 2 => 2,
            3 => 3,
            4 => 0,
            _ => 7,
        };
        let mut s = rnd_simplex(&mut rng, count);
        if rng.chance(8) {
            s.verts[1].p = s.verts[0].p; // ab == 0 -> det2 == 0 -> CCW90 path
        }
        let mut cs = s;
        let mut rs = s;
        let cv = unsafe { (c.c2D)(&mut cs) };
        let rv = unsafe { (r.c2D)(&mut rs) };
        d.v(&format!("B27#{i} count={count} in={s:?}"), cv, rv);
        d.simplex(&format!("B27#{i} struct untouched"), &cs, &rs);
    }
    d.finish();
}

#[test]
fn B28_c2L() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B28 c2L, count 1/2 + others, div incl. 0");
    let mut rng = Rng::new(SEED ^ 45);
    for i in 0..8192 {
        let count = match rng.below(5) {
            0 => 1,
            1 | 2 => 2,
            3 => 3,
            _ => -3,
        };
        let mut s = rnd_simplex(&mut rng, count);
        if rng.chance(4) {
            s.div = 0.0;
        }
        if rng.chance(9) {
            s.verts[0].u = rng.huge();
            s.verts[1].u = rng.huge();
        }
        let mut cs = s;
        let mut rs = s;
        let cv = unsafe { (c.c2L)(&mut cs) };
        let rv = unsafe { (r.c2L)(&mut rs) };
        d.v(&format!("B28#{i} count={count} div={:?} in={s:?}", s.div), cv, rv);
        d.simplex(&format!("B28#{i} struct untouched"), &cs, &rs);
    }
    d.finish();
}

#[test]
fn B29_c2Witness() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B29 c2Witness, count 1/2/3 + others");
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..8192 {
        let count = match rng.below(6) {
            0 => 1,
            1 => 2,
            2 | 3 => 3,
            4 => 0,
            _ => 9,
        };
        let mut s = rnd_simplex(&mut rng, count);
        if rng.chance(5) {
            s.div = if rng.chance(2) { 0.0 } else { -0.0 };
        }
        let mut cs = s;
        let mut rs = s;
        let mut ca: C2v = poison(1);
        let mut cb: C2v = poison(2);
        let mut ra: C2v = ca;
        let mut rb: C2v = cb;
        unsafe {
            (c.c2Witness)(&mut cs, &mut ca, &mut cb);
            (r.c2Witness)(&mut rs, &mut ra, &mut rb);
        }
        d.v(&format!("B29#{i} count={count} a"), ca, ra);
        d.v(&format!("B29#{i} count={count} b"), cb, rb);
        d.simplex(&format!("B29#{i} struct untouched"), &cs, &rs);
    }
    d.finish();
}

#[test]
fn B30_open_coded_gjk_iteration() {
    // Drives the *composed* low-level pipeline by hand, the way c2GJK does:
    // solver -> c2L -> c2D -> c2MulrvT -> c2Support -> c2Mulxv -> c2Sub.
    let (c, r) = load_pair();
    let mut d = Diff::new("B30 open-coded GJK iteration through the low-level API");
    let mut rng = Rng::new(SEED ^ 47);
    for i in 0..3000 {
        let ta = rng.below(3) as usize;
        let tb = rng.below(3) as usize;
        let sh_a = shape_near(&mut rng, ta, C2v { x: 0.0, y: 0.0 }, 20.0);
        let sh_b = shape_near(&mut rng, tb, C2v { x: 8.0, y: 3.0 }, 20.0);
        let ax = C2x { p: rng.v_coord(), r: rng.rot() };
        let bx = C2x { p: rng.v_coord(), r: rng.rot() };

        // Build both proxies through each library.
        let mut cpa: C2Proxy = poison(3);
        let mut cpb: C2Proxy = poison(4);
        let mut rpa: C2Proxy = cpa;
        let mut rpb: C2Proxy = cpb;
        unsafe {
            (c.c2MakeProxy)(sh_a.as_ptr(), sh_a.ty(), &mut cpa);
            (c.c2MakeProxy)(sh_b.as_ptr(), sh_b.ty(), &mut cpb);
            (r.c2MakeProxy)(sh_a.as_ptr(), sh_a.ty(), &mut rpa);
            (r.c2MakeProxy)(sh_b.as_ptr(), sh_b.ty(), &mut rpb);
        }
        d.proxy(&format!("B30#{i}/proxyA"), &cpa, &rpa);
        d.proxy(&format!("B30#{i}/proxyB"), &cpb, &rpb);

        // Seed the simplex exactly like c2GJK does.
        let seed_simplex = |api: &Api, pa: &C2Proxy, pb: &C2Proxy| {
            let mut s = C2Simplex::default();
            s.verts[0].iA = 0;
            s.verts[0].iB = 0;
            s.verts[0].sA = (api.c2Mulxv)(ax, pa.verts[0]);
            s.verts[0].sB = (api.c2Mulxv)(bx, pb.verts[0]);
            s.verts[0].p = (api.c2Sub)(s.verts[0].sB, s.verts[0].sA);
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
            s
        };
        let mut cs = seed_simplex(&c, &cpa, &cpb);
        let mut rs = seed_simplex(&r, &rpa, &rpb);
        d.simplex(&format!("B30#{i}/seed"), &cs, &rs);

        // Three hand-rolled iterations.
        for it in 0..3 {
            unsafe {
                match cs.count {
                    2 => (c.c22)(&mut cs),
                    3 => (c.c23)(&mut cs),
                    _ => {}
                }
                match rs.count {
                    2 => (r.c22)(&mut rs),
                    3 => (r.c23)(&mut rs),
                    _ => {}
                }
            }
            d.simplex(&format!("B30#{i}/iter{it}/solver"), &cs, &rs);
            if cs.count == 3 || rs.count == 3 {
                break;
            }
            let (cl, rl) = unsafe { ((c.c2L)(&mut cs), (r.c2L)(&mut rs)) };
            d.v(&format!("B30#{i}/iter{it}/c2L"), cl, rl);
            d.f32(
                &format!("B30#{i}/iter{it}/d1"),
                (c.c2Dot)(cl, cl),
                (r.c2Dot)(rl, rl),
            );
            let (cd, rd) = unsafe { ((c.c2D)(&mut cs), (r.c2D)(&mut rs)) };
            d.v(&format!("B30#{i}/iter{it}/c2D"), cd, rd);
            let cdir_a = (c.c2MulrvT)(ax.r, (c.c2Neg)(cd));
            let rdir_a = (r.c2MulrvT)(ax.r, (r.c2Neg)(rd));
            d.v(&format!("B30#{i}/iter{it}/dirA"), cdir_a, rdir_a);
            let cia = unsafe { (c.c2Support)(cpa.verts.as_ptr(), cpa.count, cdir_a) };
            let ria = unsafe { (r.c2Support)(rpa.verts.as_ptr(), rpa.count, rdir_a) };
            d.int(&format!("B30#{i}/iter{it}/iA"), cia, ria);
            let cdir_b = (c.c2MulrvT)(bx.r, cd);
            let rdir_b = (r.c2MulrvT)(bx.r, rd);
            let cib = unsafe { (c.c2Support)(cpb.verts.as_ptr(), cpb.count, cdir_b) };
            let rib = unsafe { (r.c2Support)(rpb.verts.as_ptr(), rpb.count, rdir_b) };
            d.int(&format!("B30#{i}/iter{it}/iB"), cib, rib);
            if cia != ria || cib != rib {
                break;
            }
            let n = cs.count.clamp(0, 3) as usize;
            cs.verts[n].iA = cia;
            cs.verts[n].sA = (c.c2Mulxv)(ax, cpa.verts[cia.clamp(0, 7) as usize]);
            cs.verts[n].iB = cib;
            cs.verts[n].sB = (c.c2Mulxv)(bx, cpb.verts[cib.clamp(0, 7) as usize]);
            cs.verts[n].p = (c.c2Sub)(cs.verts[n].sB, cs.verts[n].sA);
            cs.count += 1;
            let m = rs.count.clamp(0, 3) as usize;
            rs.verts[m].iA = ria;
            rs.verts[m].sA = (r.c2Mulxv)(ax, rpa.verts[ria.clamp(0, 7) as usize]);
            rs.verts[m].iB = rib;
            rs.verts[m].sB = (r.c2Mulxv)(bx, rpb.verts[rib.clamp(0, 7) as usize]);
            rs.verts[m].p = (r.c2Sub)(rs.verts[m].sB, rs.verts[m].sA);
            rs.count += 1;
            d.simplex(&format!("B30#{i}/iter{it}/extended"), &cs, &rs);
        }

        // Witness + distance, again through both libraries.
        let mut ca = C2v::default();
        let mut cb = C2v::default();
        let mut ra = C2v::default();
        let mut rb = C2v::default();
        unsafe {
            (c.c2Witness)(&mut cs, &mut ca, &mut cb);
            (r.c2Witness)(&mut rs, &mut ra, &mut rb);
        }
        d.v(&format!("B30#{i}/witnessA"), ca, ra);
        d.v(&format!("B30#{i}/witnessB"), cb, rb);
        d.f32(
            &format!("B30#{i}/dist"),
            (c.c2Len)((c.c2Sub)(ca, cb)),
            (r.c2Len)((r.c2Sub)(ra, rb)),
        );
        d.f32(
            &format!("B30#{i}/metric"),
            unsafe { (c.c2GJKSimplexMetric)(&mut cs) },
            unsafe { (r.c2GJKSimplexMetric)(&mut rs) },
        );
    }
    d.finish();
}
