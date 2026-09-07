//! Phase B — differential tests for the shape/proxy layer.
//! CONFIGS.md rows 20–29.

mod common;
use common::*;
use std::ffi::c_int;

const N: usize = 3000;

/// Poison pattern so we can prove which bytes the callee left untouched.
fn poison_proxy(rng: &mut Rng) -> c2Proxy {
    let mut p = c2Proxy {
        radius: f32::from_bits(0xDEAD_BEEF),
        count: -0x1234_5678,
        verts: [c2v::default(); 8],
    };
    for i in 0..8 {
        p.verts[i] = c2v {
            x: f32::from_bits(0xCAFE_0000 | i as u32),
            y: rng.coord(),
        };
    }
    p
}

#[test]
fn row20_row21_c2BBVerts() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 20);

    let mut run = |bb: c2AABB, ctx: &str| {
        let mut cb = bb;
        let mut rb = bb;
        let mut co = [c2v { x: 7.0, y: -7.0 }; 4];
        let mut ro = co;
        unsafe {
            (c.c2BBVerts)(co.as_mut_ptr(), &mut cb);
            (r.c2BBVerts)(ro.as_mut_ptr(), &mut rb);
        }
        for i in 0..4 {
            assert_veq(&format!("{ctx} out[{i}]"), co[i], ro[i]);
        }
        // the AABB itself must be unmodified (and identically so)
        assert_veq(&format!("{ctx} bb.min"), cb.min, rb.min);
        assert_veq(&format!("{ctx} bb.max"), cb.max, rb.max);
    };

    for i in 0..N {
        run(rng.aabb(), &format!("row20 c2BBVerts #{i}"));
    }

    // row 21: explicit degenerate / inverted / special-value boxes
    let z = 0.0f32;
    let specials: [c2AABB; 10] = [
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
        c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } }, // inverted
        c2AABB { min: c2v { x: -1.0, y: 0.0 }, max: c2v { x: 1.0, y: 0.0 } },  // zero height
        c2AABB { min: c2v { x: 0.0, y: -1.0 }, max: c2v { x: 0.0, y: 1.0 } },  // zero width
        c2AABB { min: c2v { x: -z, y: z }, max: c2v { x: z, y: -z } },         // signed zeros
        c2AABB { min: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY }, max: c2v { x: f32::INFINITY, y: f32::INFINITY } },
        c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: f32::NAN } },
        c2AABB { min: c2v { x: f32::MIN, y: f32::MIN }, max: c2v { x: f32::MAX, y: f32::MAX } },
        c2AABB { min: c2v { x: f32::from_bits(1), y: 0.0 }, max: c2v { x: f32::from_bits(2), y: 0.0 } },
        c2AABB { min: c2v { x: f32::INFINITY, y: 0.0 }, max: c2v { x: f32::NEG_INFINITY, y: 0.0 } },
    ];
    for (i, bb) in specials.iter().enumerate() {
        run(*bb, &format!("row21 c2BBVerts special #{i}"));
    }
}

/// The C writes `out[0..3]` while *re-reading* `bb` between writes, so if `out`
/// overlaps `bb` the interleaving is observable. Exercised in-bounds by placing
/// the AABB at the front of a 4-vector buffer and passing that buffer as `out`.
#[test]
fn row21_c2BBVerts_aliasing() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..500 {
        let bb = rng.aabb();
        let mut cbuf = [c2v { x: 0.0, y: 0.0 }; 4];
        cbuf[0] = bb.min;
        cbuf[1] = bb.max;
        let mut rbuf = cbuf;
        unsafe {
            (c.c2BBVerts)(cbuf.as_mut_ptr(), cbuf.as_mut_ptr() as *mut c2AABB);
            (r.c2BBVerts)(rbuf.as_mut_ptr(), rbuf.as_mut_ptr() as *mut c2AABB);
        }
        for k in 0..4 {
            assert_veq(&format!("row21 aliasing #{i} out[{k}]"), cbuf[k], rbuf[k]);
        }
    }
}

fn make_proxy_case(ctx: &str, shape: &ShapeBuf, ty: c_int, base: c2Proxy) {
    let (c, r) = (&libs().c, &libs().r);
    let mut cp = base;
    let mut rp = base;
    unsafe {
        (c.c2MakeProxy)(shape.ptr(), ty, &mut cp);
        (r.c2MakeProxy)(shape.ptr(), ty, &mut rp);
    }
    assert!(
        proxyeq(&cp, &rp),
        "{ctx}: C={:?} != Rust={:?}",
        cp,
        rp
    );
}

#[test]
fn row22_c2MakeProxy_circle() {
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..N {
        let mut cir = rng.circle();
        if i % 11 == 0 {
            cir.r = rng.wild();
        }
        if i % 13 == 0 {
            cir.p = rng.wild_v();
        }
        let s = ShapeBuf::circle(cir);
        make_proxy_case(
            &format!("row22 circle #{i} {cir:?}"),
            &s,
            C2_TYPE_CIRCLE,
            poison_proxy(&mut rng),
        );
    }
}

#[test]
fn row23_c2MakeProxy_aabb() {
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..N {
        let mut bb = rng.aabb();
        if i % 11 == 0 {
            bb.min = rng.wild_v();
        }
        if i % 13 == 0 {
            bb.max = rng.wild_v();
        }
        let s = ShapeBuf::aabb(bb);
        make_proxy_case(
            &format!("row23 aabb #{i} {bb:?}"),
            &s,
            C2_TYPE_AABB,
            poison_proxy(&mut rng),
        );
    }
}

#[test]
fn row24_c2MakeProxy_capsule() {
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..N {
        let mut cap = rng.capsule();
        if i % 11 == 0 {
            cap.r = rng.wild();
        }
        if i % 13 == 0 {
            cap.b = cap.a;
        }
        if i % 17 == 0 {
            cap.a = rng.wild_v();
        }
        let s = ShapeBuf::capsule(cap);
        make_proxy_case(
            &format!("row24 capsule #{i} {cap:?}"),
            &s,
            C2_TYPE_CAPSULE,
            poison_proxy(&mut rng),
        );
    }
}

fn support_case(ctx: &str, verts: &[c2v], count: c_int, d: c2v) {
    let (c, r) = (&libs().c, &libs().r);
    let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
    let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
    assert_eq!(ci, ri, "{ctx}: verts={verts:?} count={count} d={d:?}");
}

#[test]
fn row25_to_row28_c2Support_counts() {
    let mut rng = Rng::new(SEED ^ 25);
    for count in [1i32, 2, 4, 8] {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = if i % 7 == 0 { rng.wild_v() } else { rng.coord_v() };
            }
            let d = if i % 5 == 0 { rng.wild_v() } else { rng.coord_v() };
            support_case(
                &format!("row2x c2Support count={count} #{i}"),
                &verts,
                count,
                d,
            );
        }
    }
}

#[test]
fn row29_c2Support_ties_and_specials() {
    let mut rng = Rng::new(SEED ^ 29);
    // all verts identical => strict `>` keeps index 0
    for count in [1i32, 2, 4, 8] {
        for i in 0..300 {
            let v = rng.coord_v();
            let verts = [v; 8];
            let d = rng.coord_v();
            support_case(&format!("row29 ties count={count} #{i}"), &verts, count, d);
        }
    }
    // d == (0,0) => every dot is 0 (or NaN) => index 0
    for count in [1i32, 2, 4, 8] {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.coord_v();
        }
        support_case(
            &format!("row29 zero-d count={count}"),
            &verts,
            count,
            c2v { x: 0.0, y: 0.0 },
        );
        support_case(
            &format!("row29 nan-d count={count}"),
            &verts,
            count,
            c2v { x: f32::NAN, y: f32::NAN },
        );
        support_case(
            &format!("row29 inf-d count={count}"),
            &verts,
            count,
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        );
    }
    // NaN inside the vertex array, at each slot
    for slot in 0..8usize {
        let mut verts = [c2v { x: 1.0, y: 1.0 }; 8];
        verts[slot] = c2v { x: f32::NAN, y: f32::NAN };
        support_case(
            &format!("row29 nan-vert slot={slot}"),
            &verts,
            8,
            c2v { x: 1.0, y: 1.0 },
        );
    }
    // axis-aligned box verts with an exactly diagonal direction (double tie)
    let bx = [
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: -1.0, y: 1.0 },
        c2v::default(),
        c2v::default(),
        c2v::default(),
        c2v::default(),
    ];
    for d in [
        c2v { x: 1.0, y: 1.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: -1.0, y: 1.0 },
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
    ] {
        support_case(&format!("row29 box tie d={d:?}"), &bx, 4, d);
    }
}
