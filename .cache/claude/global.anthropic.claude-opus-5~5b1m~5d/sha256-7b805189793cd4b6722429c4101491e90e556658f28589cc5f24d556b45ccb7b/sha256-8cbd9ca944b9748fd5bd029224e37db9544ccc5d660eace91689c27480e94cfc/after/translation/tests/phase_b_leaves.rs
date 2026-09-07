//! Phase B, Groups 1 & 2 — `CONFIGS.md` rows 1-22.
//!
//! The lowest-level entry points first: every vector/scalar helper, then proxy
//! construction and support-point selection. All calls go through `dlsym` on
//! both `.so`s; results are compared bit-for-bit.

mod common;
use common::*;

const N: usize = 4000;

// ---------------------------------------------------------------------------
// Row 1 — c2V
// ---------------------------------------------------------------------------

#[test]
fn row01_c2v() {
    let p = api();
    let mut rng = Rng::new(0x01);
    for i in 0..N {
        let (x, y) = if i % 2 == 0 {
            (rng.coord(), rng.coord())
        } else {
            (rng.wild_f32(), rng.wild_f32())
        };
        let cv = unsafe { (p.c.c2V)(x, y) };
        let rv = unsafe { (p.r.c2V)(x, y) };
        eq_v("c2V", &format!("x={x:?} y={y:?}"), cv, rv);
    }
    // Exhaustive over the interesting bit patterns.
    let pats: [u32; 12] = [
        0x0000_0000,
        0x8000_0000,
        0x7F80_0000,
        0xFF80_0000,
        0x7FC0_0000,
        0xFFC0_0000,
        0x7F80_0001,
        0xFFBF_FFFF,
        0x0000_0001,
        0x8000_0001,
        0x7F7F_FFFF,
        0xFF7F_FFFF,
    ];
    for &a in &pats {
        for &b in &pats {
            let (x, y) = (f32::from_bits(a), f32::from_bits(b));
            let cv = unsafe { (p.c.c2V)(x, y) };
            let rv = unsafe { (p.r.c2V)(x, y) };
            eq_v("c2V pat", &format!("{a:08x},{b:08x}"), cv, rv);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 2, 3 — c2Dot, c2Det2
// ---------------------------------------------------------------------------

#[test]
fn row02_c2dot() {
    let p = api();
    let mut rng = Rng::new(0x02);
    for i in 0..N {
        let (a, b) = match i % 5 {
            0 => (rng.vec(), rng.vec()),
            1 => (rng.wild_vec(), rng.wild_vec()),
            2 => (rng.vec(), rng.wild_vec()),
            3 => (
                c2v::new(rng.big_f32(), rng.big_f32()),
                c2v::new(rng.big_f32(), rng.big_f32()),
            ),
            _ => (
                c2v::new(rng.tiny_f32(), rng.tiny_f32()),
                c2v::new(rng.big_f32(), rng.tiny_f32()),
            ),
        };
        let cv = unsafe { (p.c.c2Dot)(a, b) };
        let rv = unsafe { (p.r.c2Dot)(a, b) };
        eq_f32("c2Dot", &format!("a={a:?} b={b:?}"), cv, rv);
    }
}

#[test]
fn row03_c2det2() {
    let p = api();
    let mut rng = Rng::new(0x03);
    for i in 0..N {
        let (a, b) = match i % 6 {
            0 => (rng.vec(), rng.vec()),
            1 => (rng.wild_vec(), rng.wild_vec()),
            2 => (rng.vec(), rng.wild_vec()),
            3 => {
                let v = rng.vec();
                (v, v) // det == 0 exactly
            }
            4 => {
                let v = rng.vec();
                (v, c2v::new(-v.x, -v.y)) // antiparallel
            }
            _ => (
                c2v::new(rng.big_f32(), rng.big_f32()),
                c2v::new(rng.big_f32(), rng.big_f32()),
            ),
        };
        let cv = unsafe { (p.c.c2Det2)(a, b) };
        let rv = unsafe { (p.r.c2Det2)(a, b) };
        eq_f32("c2Det2", &format!("a={a:?} b={b:?}"), cv, rv);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — c2Sub, c2Add
// ---------------------------------------------------------------------------

#[test]
fn row04_c2sub_c2add() {
    let p = api();
    let mut rng = Rng::new(0x04);
    for i in 0..N {
        let (a, b) = match i % 5 {
            0 => (rng.vec(), rng.vec()),
            1 => (rng.wild_vec(), rng.wild_vec()),
            2 => (rng.wild_vec(), rng.vec()),
            3 => {
                let v = rng.vec();
                (v, v) // a-a => +0
            }
            _ => (
                c2v::new(f32::INFINITY, f32::NEG_INFINITY),
                c2v::new(f32::INFINITY, f32::NEG_INFINITY),
            ),
        };
        let ctx = format!("a={a:?} b={b:?}");
        eq_v(
            "c2Sub",
            &ctx,
            unsafe { (p.c.c2Sub)(a, b) },
            unsafe { (p.r.c2Sub)(a, b) },
        );
        eq_v(
            "c2Add",
            &ctx,
            unsafe { (p.c.c2Add)(a, b) },
            unsafe { (p.r.c2Add)(a, b) },
        );
    }
    // NaN-survival matrix: which of two NaNs comes out.
    let nans = [
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFC0_0002),
        f32::from_bits(0x7F80_0003),
        f32::from_bits(0xFFA0_0004),
    ];
    for &n1 in &nans {
        for &n2 in &nans {
            let (a, b) = (c2v::new(n1, n2), c2v::new(n2, n1));
            let ctx = format!("nanmix {:08x}/{:08x}", n1.to_bits(), n2.to_bits());
            eq_v(
                "c2Sub nan",
                &ctx,
                unsafe { (p.c.c2Sub)(a, b) },
                unsafe { (p.r.c2Sub)(a, b) },
            );
            eq_v(
                "c2Add nan",
                &ctx,
                unsafe { (p.c.c2Add)(a, b) },
                unsafe { (p.r.c2Add)(a, b) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5, 6 — c2Mulvs, c2Div
// ---------------------------------------------------------------------------

#[test]
fn row05_c2mulvs() {
    let p = api();
    let mut rng = Rng::new(0x05);
    let scalars = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFFC0_5555),
        f32::MIN_POSITIVE,
        FLT_MAX,
    ];
    for &s in &scalars {
        for _ in 0..64 {
            let a = rng.wild_vec();
            eq_v(
                "c2Mulvs fixed-scalar",
                &format!("a={a:?} s={s:?}"),
                unsafe { (p.c.c2Mulvs)(a, s) },
                unsafe { (p.r.c2Mulvs)(a, s) },
            );
        }
    }
    for i in 0..N {
        let a = if i % 2 == 0 { rng.vec() } else { rng.wild_vec() };
        let s = if i % 3 == 0 { rng.coord() } else { rng.wild_f32() };
        eq_v(
            "c2Mulvs",
            &format!("a={a:?} s={s:?}"),
            unsafe { (p.c.c2Mulvs)(a, s) },
            unsafe { (p.r.c2Mulvs)(a, s) },
        );
    }
}

#[test]
fn row06_c2div() {
    let p = api();
    let mut rng = Rng::new(0x06);
    let divs = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        3.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7FC0_9999),
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        FLT_MAX,
    ];
    for &d in &divs {
        for _ in 0..64 {
            let a = rng.wild_vec();
            eq_v(
                "c2Div fixed",
                &format!("a={a:?} d={d:?}"),
                unsafe { (p.c.c2Div)(a, d) },
                unsafe { (p.r.c2Div)(a, d) },
            );
        }
    }
    for i in 0..N {
        let a = if i % 2 == 0 { rng.vec() } else { rng.wild_vec() };
        let d = if i % 3 == 0 { rng.coord() } else { rng.wild_f32() };
        eq_v(
            "c2Div",
            &format!("a={a:?} d={d:?}"),
            unsafe { (p.c.c2Div)(a, d) },
            unsafe { (p.r.c2Div)(a, d) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 7, 8 — c2Len, c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row07_c2len() {
    let p = api();
    let mut rng = Rng::new(0x07);
    let fixed = [
        ZV,
        c2v::new(-0.0, 0.0),
        c2v::new(3.0, 4.0),
        c2v::new(1.0, 0.0),
        c2v::new(FLT_MAX, FLT_MAX),
        c2v::new(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        c2v::new(f32::from_bits(1), f32::from_bits(1)),
        c2v::new(f32::INFINITY, 0.0),
        c2v::new(f32::NEG_INFINITY, f32::INFINITY),
        c2v::new(f32::NAN, 1.0),
        c2v::new(1.0, f32::from_bits(0xFFC0_1111)),
    ];
    for &a in &fixed {
        eq_f32(
            "c2Len fixed",
            &format!("a={a:?}"),
            unsafe { (p.c.c2Len)(a) },
            unsafe { (p.r.c2Len)(a) },
        );
    }
    for i in 0..N {
        let a = match i % 4 {
            0 => rng.vec(),
            1 => rng.wild_vec(),
            2 => c2v::new(rng.big_f32(), rng.big_f32()),
            _ => c2v::new(rng.tiny_f32(), rng.tiny_f32()),
        };
        eq_f32(
            "c2Len",
            &format!("a={a:?}"),
            unsafe { (p.c.c2Len)(a) },
            unsafe { (p.r.c2Len)(a) },
        );
    }
}

#[test]
fn row08_c2norm() {
    let p = api();
    let mut rng = Rng::new(0x08);
    let fixed = [
        ZV,
        c2v::new(-0.0, -0.0),
        c2v::new(1.0, 0.0),
        c2v::new(3.0, 4.0),
        c2v::new(FLT_MAX, FLT_MAX),
        c2v::new(f32::INFINITY, 1.0),
        c2v::new(f32::INFINITY, f32::INFINITY),
        c2v::new(f32::NAN, f32::NAN),
        c2v::new(f32::from_bits(1), 0.0),
    ];
    for &a in &fixed {
        eq_v(
            "c2Norm fixed",
            &format!("a={a:?}"),
            unsafe { (p.c.c2Norm)(a) },
            unsafe { (p.r.c2Norm)(a) },
        );
    }
    for i in 0..N {
        let a = match i % 4 {
            0 => rng.vec(),
            1 => rng.wild_vec(),
            2 => c2v::new(rng.big_f32(), rng.big_f32()),
            _ => c2v::new(rng.tiny_f32(), rng.tiny_f32()),
        };
        eq_v(
            "c2Norm",
            &format!("a={a:?}"),
            unsafe { (p.c.c2Norm)(a) },
            unsafe { (p.r.c2Norm)(a) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 — c2Neg, c2Skew, c2CCW90
// ---------------------------------------------------------------------------

#[test]
fn row09_unary_sign_ops() {
    let p = api();
    let mut rng = Rng::new(0x09);
    let pats: [u32; 14] = [
        0x0000_0000,
        0x8000_0000,
        0x3F80_0000,
        0xBF80_0000,
        0x7F80_0000,
        0xFF80_0000,
        0x7FC0_0000,
        0xFFC0_0000,
        0x7F80_0001,
        0xFFBF_FFFF,
        0x0000_0001,
        0x8000_0001,
        0x7F7F_FFFF,
        0xFF7F_FFFF,
    ];
    let mut cases: Vec<c2v> = Vec::new();
    for &a in &pats {
        for &b in &pats {
            cases.push(c2v::new(f32::from_bits(a), f32::from_bits(b)));
        }
    }
    for _ in 0..N {
        cases.push(rng.wild_vec());
    }
    for a in cases {
        let ctx = format!("a=({:08x},{:08x})", a.x.to_bits(), a.y.to_bits());
        eq_v(
            "c2Neg",
            &ctx,
            unsafe { (p.c.c2Neg)(a) },
            unsafe { (p.r.c2Neg)(a) },
        );
        eq_v(
            "c2Skew",
            &ctx,
            unsafe { (p.c.c2Skew)(a) },
            unsafe { (p.r.c2Skew)(a) },
        );
        eq_v(
            "c2CCW90",
            &ctx,
            unsafe { (p.c.c2CCW90)(a) },
            unsafe { (p.r.c2CCW90)(a) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 10, 11 — c2Maxv, c2Minv, c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn row10_c2maxv_c2minv() {
    let p = api();
    let mut rng = Rng::new(0x0A);
    // +0 / -0 ties and NaN operands are where `>` / `<` semantics show.
    let interesting = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::NAN,
        f32::from_bits(0xFFC0_3333),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &ax in &interesting {
        for &ay in &interesting {
            for &bx in &interesting {
                for &by in &interesting {
                    let (a, b) = (c2v::new(ax, ay), c2v::new(bx, by));
                    let ctx = format!("a={a:?} b={b:?}");
                    eq_v(
                        "c2Maxv",
                        &ctx,
                        unsafe { (p.c.c2Maxv)(a, b) },
                        unsafe { (p.r.c2Maxv)(a, b) },
                    );
                    eq_v(
                        "c2Minv",
                        &ctx,
                        unsafe { (p.c.c2Minv)(a, b) },
                        unsafe { (p.r.c2Minv)(a, b) },
                    );
                }
            }
        }
    }
    for i in 0..N {
        let (a, b) = if i % 3 == 0 {
            let v = rng.vec();
            (v, v) // exact tie
        } else if i % 3 == 1 {
            (rng.vec(), rng.vec())
        } else {
            (rng.wild_vec(), rng.wild_vec())
        };
        let ctx = format!("a={a:?} b={b:?}");
        eq_v(
            "c2Maxv",
            &ctx,
            unsafe { (p.c.c2Maxv)(a, b) },
            unsafe { (p.r.c2Maxv)(a, b) },
        );
        eq_v(
            "c2Minv",
            &ctx,
            unsafe { (p.c.c2Minv)(a, b) },
            unsafe { (p.r.c2Minv)(a, b) },
        );
    }
}

#[test]
fn row11_c2clampv() {
    let p = api();
    let mut rng = Rng::new(0x0B);
    for i in 0..N * 2 {
        let (a, lo, hi) = match i % 6 {
            // inside
            0 => (
                c2v::new(0.0, 0.0),
                c2v::new(-1.0, -1.0),
                c2v::new(1.0, 1.0),
            ),
            // below / above
            1 => (
                c2v::new(-5.0, 5.0),
                c2v::new(-1.0, -1.0),
                c2v::new(1.0, 1.0),
            ),
            // inverted range (lo > hi) — no validation in C
            2 => (rng.vec(), c2v::new(1.0, 1.0), c2v::new(-1.0, -1.0)),
            3 => (rng.vec(), rng.vec(), rng.vec()),
            4 => (rng.wild_vec(), rng.vec(), rng.vec()),
            _ => (rng.wild_vec(), rng.wild_vec(), rng.wild_vec()),
        };
        eq_v(
            "c2Clampv",
            &format!("a={a:?} lo={lo:?} hi={hi:?}"),
            unsafe { (p.c.c2Clampv)(a, lo, hi) },
            unsafe { (p.r.c2Clampv)(a, lo, hi) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 — c2RotIdentity, c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn row12_identities() {
    let p = api();
    let cr = unsafe { (p.c.c2RotIdentity)() };
    let rr = unsafe { (p.r.c2RotIdentity)() };
    eq_bits("c2RotIdentity", "", &cr, &rr);
    let cx = unsafe { (p.c.c2xIdentity)() };
    let rx = unsafe { (p.r.c2xIdentity)() };
    eq_bits("c2xIdentity", "", &cx, &rx);
    // Called repeatedly: must be pure.
    for _ in 0..100 {
        eq_bits(
            "c2xIdentity repeat",
            "",
            &unsafe { (p.c.c2xIdentity)() },
            &unsafe { (p.r.c2xIdentity)() },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — c2Mulrv, c2MulrvT
// ---------------------------------------------------------------------------

#[test]
fn row13_c2mulrv_transpose() {
    let p = api();
    let mut rng = Rng::new(0x0D);
    let mut rots = vec![
        c2r { c: 1.0, s: 0.0 },
        c2r { c: 0.0, s: 1.0 },
        c2r { c: -1.0, s: 0.0 },
        c2r { c: 0.0, s: 0.0 },
        c2r {
            c: f32::NAN,
            s: 1.0,
        },
        c2r {
            c: 1.0,
            s: f32::from_bits(0xFFC0_2222),
        },
        c2r {
            c: f32::INFINITY,
            s: f32::NEG_INFINITY,
        },
        c2r {
            c: FLT_MAX,
            s: FLT_MAX,
        },
    ];
    for _ in 0..64 {
        rots.push(rng.rot());
    }
    for _ in 0..64 {
        rots.push(rng.rot_wild());
    }
    for r in rots {
        for i in 0..48 {
            let b = if i % 3 == 0 { rng.wild_vec() } else { rng.vec() };
            let ctx = format!("r={r:?} b={b:?}");
            eq_v(
                "c2Mulrv",
                &ctx,
                unsafe { (p.c.c2Mulrv)(r, b) },
                unsafe { (p.r.c2Mulrv)(r, b) },
            );
            eq_v(
                "c2MulrvT",
                &ctx,
                unsafe { (p.c.c2MulrvT)(r, b) },
                unsafe { (p.r.c2MulrvT)(r, b) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — c2Mulxv
// ---------------------------------------------------------------------------

#[test]
fn row14_c2mulxv() {
    let p = api();
    let mut rng = Rng::new(0x0E);
    let mut xs = vec![
        c2x {
            p: ZV,
            r: c2r { c: 1.0, s: 0.0 },
        },
        // pure translation
        c2x {
            p: c2v::new(3.0, -4.0),
            r: c2r { c: 1.0, s: 0.0 },
        },
        // pure rotation
        c2x {
            p: ZV,
            r: c2r { c: 0.0, s: 1.0 },
        },
        // non-finite
        c2x {
            p: c2v::new(f32::NAN, f32::INFINITY),
            r: c2r {
                c: f32::NEG_INFINITY,
                s: f32::NAN,
            },
        },
    ];
    for _ in 0..64 {
        xs.push(rng.xform());
    }
    for _ in 0..64 {
        xs.push(rng.xform_wild());
    }
    for x in xs {
        for i in 0..48 {
            let b = if i % 3 == 0 { rng.wild_vec() } else { rng.vec() };
            eq_v(
                "c2Mulxv",
                &format!("x={x:?} b={b:?}"),
                unsafe { (p.c.c2Mulxv)(x, b) },
                unsafe { (p.r.c2Mulxv)(x, b) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn row15_c2bbverts() {
    let p = api();
    let mut rng = Rng::new(0x0F);
    // Poison the out-buffer so unwritten slots are detectable.
    const POISON: c2v = c2v {
        x: -7.7e28,
        y: 4.2e27,
    };
    let mut cases: Vec<c2AABB> = vec![
        c2AABB {
            min: ZV,
            max: ZV,
        },
        c2AABB {
            min: c2v::new(1.0, 1.0),
            max: c2v::new(-1.0, -1.0),
        }, // inverted
        c2AABB {
            min: c2v::new(f32::NAN, f32::INFINITY),
            max: c2v::new(f32::NEG_INFINITY, f32::from_bits(0xFFC0_7777)),
        },
        c2AABB {
            min: c2v::new(-FLT_MAX, -FLT_MAX),
            max: c2v::new(FLT_MAX, FLT_MAX),
        },
        c2AABB {
            min: c2v::new(-0.0, 0.0),
            max: c2v::new(0.0, -0.0),
        },
    ];
    for i in 0..N {
        cases.push(if i % 3 == 0 {
            c2AABB {
                min: rng.wild_vec(),
                max: rng.wild_vec(),
            }
        } else {
            let (a, b) = (rng.vec(), rng.vec());
            c2AABB { min: a, max: b }
        });
    }
    for bb in cases {
        // Use a 6-slot buffer: writing outside [0,4) must not happen in either.
        let mut cbuf = [POISON; 6];
        let mut rbuf = [POISON; 6];
        let mut cbb = bb;
        let mut rbb = bb;
        unsafe {
            (p.c.c2BBVerts)(cbuf.as_mut_ptr(), &mut cbb);
            (p.r.c2BBVerts)(rbuf.as_mut_ptr(), &mut rbb);
        }
        let ctx = format!("bb={bb:?}");
        eq_bits("c2BBVerts out", &ctx, &cbuf, &rbuf);
        eq_bits("c2BBVerts input (unmodified)", &ctx, &cbb, &rbb);
    }
}

// ---------------------------------------------------------------------------
// Rows 16-18 — c2MakeProxy for each valid C2_TYPE
// ---------------------------------------------------------------------------

/// Pre-seed the proxy with a recognisable pattern so that fields the C code
/// leaves untouched (`verts` beyond `count`) are compared too.
fn seeded_proxy(rng: &mut Rng) -> c2Proxy {
    let mut pr = c2Proxy {
        radius: -1.5e30,
        count: -777,
        verts: [ZV; 8],
    };
    for i in 0..8 {
        pr.verts[i] = c2v::new(1000.0 + i as f32, -1000.0 - i as f32);
    }
    // occasionally randomise the seed pattern too
    if rng.bool() {
        pr.radius = rng.wild_f32();
        pr.count = rng.next_u32() as i32;
        for i in 0..8 {
            pr.verts[i] = rng.wild_vec();
        }
    }
    pr
}

#[track_caller]
fn diff_make_proxy(ctx: &str, seed: c2Proxy, shape_ptr: *const std::ffi::c_void, ty: i32) {
    let p = api();
    let mut cp = seed;
    let mut rp = seed;
    unsafe {
        (p.c.c2MakeProxy)(shape_ptr, ty, &mut cp);
        (p.r.c2MakeProxy)(shape_ptr, ty, &mut rp);
    }
    eq_bits("c2MakeProxy out", ctx, &cp, &rp);
}

#[test]
fn row16_makeproxy_circle() {
    let mut rng = Rng::new(0x10);
    let mut cases: Vec<c2Circle> = vec![
        c2Circle { p: ZV, r: 0.0 },
        c2Circle { p: ZV, r: -3.0 },
        c2Circle {
            p: c2v::new(f32::NAN, f32::INFINITY),
            r: f32::NAN,
        },
        c2Circle {
            p: c2v::new(FLT_MAX, -FLT_MAX),
            r: FLT_MAX,
        },
    ];
    for _ in 0..500 {
        cases.push(c2Circle {
            p: rng.vec(),
            r: rng.radius(),
        });
    }
    for _ in 0..200 {
        cases.push(c2Circle {
            p: rng.wild_vec(),
            r: rng.wild_f32(),
        });
    }
    for c in cases {
        let seed = seeded_proxy(&mut rng);
        diff_make_proxy(
            &format!("circle {c:?}"),
            seed,
            &c as *const c2Circle as *const std::ffi::c_void,
            C2_TYPE_CIRCLE,
        );
    }
}

#[test]
fn row17_makeproxy_aabb() {
    let mut rng = Rng::new(0x11);
    let mut cases: Vec<c2AABB> = vec![
        c2AABB { min: ZV, max: ZV },
        c2AABB {
            min: c2v::new(2.0, 2.0),
            max: c2v::new(-2.0, -2.0),
        },
        c2AABB {
            min: c2v::new(f32::NAN, f32::NEG_INFINITY),
            max: c2v::new(f32::INFINITY, f32::from_bits(0x7F80_0005)),
        },
    ];
    for _ in 0..500 {
        cases.push(c2AABB {
            min: rng.vec(),
            max: rng.vec(),
        });
    }
    for _ in 0..200 {
        cases.push(c2AABB {
            min: rng.wild_vec(),
            max: rng.wild_vec(),
        });
    }
    for b in cases {
        let seed = seeded_proxy(&mut rng);
        diff_make_proxy(
            &format!("aabb {b:?}"),
            seed,
            &b as *const c2AABB as *const std::ffi::c_void,
            C2_TYPE_AABB,
        );
    }
}

#[test]
fn row18_makeproxy_capsule() {
    let mut rng = Rng::new(0x12);
    let mut cases: Vec<c2Capsule> = vec![
        c2Capsule {
            a: ZV,
            b: ZV,
            r: 0.0,
        },
        c2Capsule {
            a: c2v::new(1.0, 1.0),
            b: c2v::new(1.0, 1.0),
            r: -2.0,
        }, // zero length
        c2Capsule {
            a: c2v::new(f32::NAN, 0.0),
            b: c2v::new(f32::INFINITY, f32::NEG_INFINITY),
            r: f32::from_bits(0xFFC0_1010),
        },
    ];
    for _ in 0..500 {
        cases.push(c2Capsule {
            a: rng.vec(),
            b: rng.vec(),
            r: rng.radius(),
        });
    }
    for _ in 0..200 {
        cases.push(c2Capsule {
            a: rng.wild_vec(),
            b: rng.wild_vec(),
            r: rng.wild_f32(),
        });
    }
    for c in cases {
        let seed = seeded_proxy(&mut rng);
        diff_make_proxy(
            &format!("capsule {c:?}"),
            seed,
            &c as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE_CAPSULE,
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 19-22 — c2Support at counts 1, 2, 4, 8
// ---------------------------------------------------------------------------

fn support_rows(count: usize, seed: u64) {
    let p = api();
    let mut rng = Rng::new(seed);
    for iter in 0..2000usize {
        let mut verts = vec![ZV; count.max(1)];
        for v in verts.iter_mut() {
            *v = match iter % 5 {
                0 => rng.vec(),
                1 => rng.wild_vec(),
                2 => c2v::new(rng.big_f32(), rng.big_f32()),
                3 => ZV, // all identical => every dot ties
                _ => rng.vec(),
            };
        }
        // Directions include exact axis alignment (ties for an AABB) and
        // degenerate/NaN directions.
        let ds: [c2v; 8] = [
            rng.vec(),
            rng.wild_vec(),
            ZV,
            c2v::new(1.0, 0.0),
            c2v::new(0.0, 1.0),
            c2v::new(-1.0, 0.0),
            c2v::new(f32::NAN, f32::NAN),
            c2v::new(f32::INFINITY, f32::NEG_INFINITY),
        ];
        for d in ds {
            let cv = unsafe { (p.c.c2Support)(verts.as_ptr(), count as i32, d) };
            let rv = unsafe { (p.r.c2Support)(verts.as_ptr(), count as i32, d) };
            eq_i32(
                "c2Support",
                &format!("count={count} verts={verts:?} d={d:?}"),
                cv,
                rv,
            );
        }
    }
}

#[test]
fn row19_support_count1() {
    support_rows(1, 0x13);
}

#[test]
fn row20_support_count2() {
    support_rows(2, 0x14);
}

#[test]
fn row21_support_count4() {
    support_rows(4, 0x15);
}

#[test]
fn row22_support_count8() {
    support_rows(8, 0x16);
}
