//! Phase B — CONFIGS.md rows 18..21: `c2BBVerts` and `c2MakeProxy`.
//!
//! Both write through caller-owned buffers, so the buffers are pre-poisoned
//! with an identical recognisable pattern in both libraries and then compared
//! byte-for-byte afterwards.  That also proves the untouched slots really are
//! untouched (the C `switch` has no `default:`).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const N: u32 = 3000;

/// Fill a proxy with a recognisable pattern so that "not written" is visible.
fn poison(seed: u32) -> c2Proxy {
    let mut p = c2Proxy { radius: f32::from_bits(0xDEAD_0000 | seed), count: -0x5A5A, verts: [c2v::default(); 8] };
    for i in 0..8 {
        p.verts[i] = c2v {
            x: f32::from_bits(0xCAFE_0000 | (i as u32) | (seed << 8)),
            y: f32::from_bits(0xBEEF_0000 | (i as u32) | (seed << 8)),
        };
    }
    p
}

fn aabbs(rng: &mut Rng) -> Vec<c2AABB> {
    let mut v = vec![
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } }, // degenerate point
        c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } },
        c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } }, // inverted
        c2AABB { min: c2v { x: -0.0, y: 0.0 }, max: c2v { x: 0.0, y: -0.0 } }, // signed zeros
        c2AABB {
            min: c2v { x: f32::NAN, y: f32::NEG_INFINITY },
            max: c2v { x: f32::INFINITY, y: f32::from_bits(0xFFC0_0033) },
        },
        c2AABB { min: c2v { x: -FLT_MAX, y: -FLT_MAX }, max: c2v { x: FLT_MAX, y: FLT_MAX } },
    ];
    for _ in 0..64 {
        v.push(c2AABB { min: rng.v(1e3), max: rng.v(1e3) });
        v.push(c2AABB { min: rng.v_spicy(1e3), max: rng.v_spicy(1e3) });
        v.push(c2AABB { min: rng.v_bits(), max: rng.v_bits() });
    }
    v
}

// -------------------------------------------------------------------- row 18
#[test]
fn row18_c2BBVerts() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2018);
    for _ in 0..N {
        for mut bb in aabbs(&mut rng) {
            // 6 slots: the C writes exactly 4; slots 4..6 must stay poisoned.
            let mut cout = [c2v { x: f32::from_bits(0x1111_2222), y: f32::from_bits(0x3333_4444) }; 6];
            let mut rout = cout;
            let mut bbc = bb;
            let mut bbr = bb;
            unsafe {
                (c.c2BBVerts)(cout.as_mut_ptr(), &mut bbc);
                (r.c2BBVerts)(rout.as_mut_ptr(), &mut bbr);
            }
            diff_eq!(format!("c2BBVerts out {bb:?}"), raw_bytes(&cout), raw_bytes(&rout));
            // the AABB itself must not be modified
            diff_eq!(format!("c2BBVerts bb {bb:?}"), raw_bytes(&bbc), raw_bytes(&bbr));
            let _ = &mut bb;
        }
        if rng.below(4) == 0 {
            break; // aabbs() is already a large batch; keep runtime sane
        }
    }
}

// -------------------------------------------------------------------- row 19
#[test]
fn row19_makeproxy_circle() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2019);
    let mut shapes = vec![
        c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
        c2Circle { p: c2v { x: 1.0, y: -1.0 }, r: -5.0 },
        c2Circle { p: c2v { x: f32::NAN, y: 0.0 }, r: f32::NAN },
        c2Circle { p: c2v { x: f32::INFINITY, y: f32::NEG_INFINITY }, r: FLT_MAX },
        c2Circle { p: c2v { x: -0.0, y: -0.0 }, r: -0.0 },
    ];
    for _ in 0..N {
        shapes.push(c2Circle { p: rng.v_spicy(1e3), r: rng.spicy(1e3) });
        shapes.push(c2Circle { p: rng.v_bits(), r: rng.any_bits() });
    }
    for (i, sh) in shapes.iter().enumerate() {
        let mut pc = poison(i as u32 & 0xFF);
        let mut pr = pc;
        unsafe {
            (c.c2MakeProxy)(sh as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut pc);
            (r.c2MakeProxy)(sh as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut pr);
        }
        diff_eq!(format!("makeproxy circle {sh:?}"), raw_bytes(&pc), raw_bytes(&pr));
        assert_eq!(pc.count, 1, "C wrote count != 1 for a circle");
    }
}

// -------------------------------------------------------------------- row 20
#[test]
fn row20_makeproxy_aabb() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2020);
    let mut shapes = aabbs(&mut rng);
    for _ in 0..N {
        shapes.push(c2AABB { min: rng.v_spicy(1e3), max: rng.v_spicy(1e3) });
        shapes.push(c2AABB { min: rng.v_bits(), max: rng.v_bits() });
    }
    for (i, sh) in shapes.iter().enumerate() {
        let mut pc = poison(i as u32 & 0xFF);
        let mut pr = pc;
        unsafe {
            (c.c2MakeProxy)(sh as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut pc);
            (r.c2MakeProxy)(sh as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut pr);
        }
        diff_eq!(format!("makeproxy aabb {sh:?}"), raw_bytes(&pc), raw_bytes(&pr));
        assert_eq!(pc.count, 4, "C wrote count != 4 for an AABB");
    }
}

// -------------------------------------------------------------------- row 21
#[test]
fn row21_makeproxy_capsule() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2021);
    let mut shapes = vec![
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 }, // a == b
        c2Capsule { a: c2v { x: 1.0, y: 2.0 }, b: c2v { x: 1.0, y: 2.0 }, r: -3.0 },
        c2Capsule {
            a: c2v { x: f32::NAN, y: f32::INFINITY },
            b: c2v { x: -0.0, y: f32::NEG_INFINITY },
            r: f32::from_bits(0xFFC0_0077),
        },
    ];
    for _ in 0..N {
        shapes.push(c2Capsule { a: rng.v_spicy(1e3), b: rng.v_spicy(1e3), r: rng.spicy(1e3) });
        shapes.push(c2Capsule { a: rng.v_bits(), b: rng.v_bits(), r: rng.any_bits() });
    }
    for (i, sh) in shapes.iter().enumerate() {
        let mut pc = poison(i as u32 & 0xFF);
        let mut pr = pc;
        unsafe {
            (c.c2MakeProxy)(sh as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut pc);
            (r.c2MakeProxy)(sh as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut pr);
        }
        diff_eq!(format!("makeproxy capsule {sh:?}"), raw_bytes(&pc), raw_bytes(&pr));
        assert_eq!(pc.count, 2, "C wrote count != 2 for a capsule");
    }
}
