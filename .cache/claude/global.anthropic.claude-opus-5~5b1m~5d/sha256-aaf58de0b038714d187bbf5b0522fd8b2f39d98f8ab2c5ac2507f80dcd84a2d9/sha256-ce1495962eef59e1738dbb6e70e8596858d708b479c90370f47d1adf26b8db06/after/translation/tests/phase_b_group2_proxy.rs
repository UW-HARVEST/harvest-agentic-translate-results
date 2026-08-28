//! Phase B, Group 2 — CONFIGS.md rows C16..C20 (`c2BBVerts`, `c2MakeProxy`).

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 3000;

#[test]
fn c16_c2BBVerts() {
    let p = load();
    let c: FnBBVerts = p.c.sym("c2BBVerts");
    let r: FnBBVerts = p.rs.sym("c2BBVerts");
    let mut rng = Rng::new(0x16);
    unsafe {
        let run = |bb: c2AABB| {
            // 8-slot destination so an over-write past index 3 would be caught.
            let mut co = [c2v { x: 1.5, y: -2.5 }; 8];
            let mut ro = co;
            let mut bb_c = bb;
            let mut bb_r = bb;
            c(co.as_mut_ptr(), &mut bb_c);
            r(ro.as_mut_ptr(), &mut bb_r);
            assert_bits_eq!(co, ro, "c2BBVerts out for min={} max={}", v_hex(&bb.min), v_hex(&bb.max));
            // the C never writes to *bb — prove the Rust doesn't either
            assert_bits_eq!(bb_c, bb_r, "c2BBVerts must not modify *bb");
            assert_bits_eq!(bb_c, bb, "c2BBVerts modified *bb (C)");
        };
        for _ in 0..N {
            run(rng.aabb());
        }
        // degenerate / inverted / special corners
        for a in specials() {
            for b in specials() {
                run(c2AABB {
                    min: c2v { x: a, y: b },
                    max: c2v { x: b, y: a },
                });
            }
        }
        let z = c2v { x: 0.0, y: 0.0 };
        run(c2AABB { min: z, max: z });
        run(c2AABB {
            min: c2v { x: 5.0, y: 5.0 },
            max: c2v { x: -5.0, y: -5.0 },
        });
    }
}

/// Shared driver for rows C17..C20: fills the proxy with caller-supplied
/// garbage first, so the *untouched* slots are compared too.
fn make_proxy_case(p: &Pair, seed_proxy: c2Proxy, shape: &[u8], ty: c_int, ctx: &str) {
    let c: FnMakeProxy = p.c.sym("c2MakeProxy");
    let r: FnMakeProxy = p.rs.sym("c2MakeProxy");
    let mut cp = seed_proxy;
    let mut rp = seed_proxy;
    unsafe {
        c(shape.as_ptr() as *const c_void, ty, &mut cp);
        r(shape.as_ptr() as *const c_void, ty, &mut rp);
    }
    if raw(&cp) != raw(&rp) {
        panic!(
            "DIVERGENCE c2MakeProxy type={ty} {ctx}\n  C   : {}\n  Rust: {}",
            proxy_hex(&cp),
            proxy_hex(&rp)
        );
    }
}

fn garbage_proxy(rng: &mut Rng) -> c2Proxy {
    let mut pr = c2Proxy {
        radius: rng.wild(),
        count: rng.next_u32() as c_int,
        verts: Default::default(),
    };
    for i in 0..8 {
        pr.verts[i] = rng.v_wild();
    }
    pr
}

#[test]
fn c17_c2MakeProxy_circle() {
    let p = load();
    let mut rng = Rng::new(0x17);
    for _ in 0..N {
        let sh = rng.circle();
        make_proxy_case(&p, garbage_proxy(&mut rng), &raw(&sh), C2_TYPE_CIRCLE, "circle");
    }
    // r == 0, r < 0, r special
    for rv in specials() {
        let sh = c2Circle {
            p: c2v { x: 1.0, y: -2.0 },
            r: rv,
        };
        make_proxy_case(&p, c2Proxy::default(), &raw(&sh), C2_TYPE_CIRCLE, "circle special r");
    }
}

#[test]
fn c18_c2MakeProxy_aabb() {
    let p = load();
    let mut rng = Rng::new(0x18);
    for _ in 0..N {
        let sh = rng.aabb();
        make_proxy_case(&p, garbage_proxy(&mut rng), &raw(&sh), C2_TYPE_AABB, "aabb");
    }
    for a in specials() {
        let sh = c2AABB {
            min: c2v { x: a, y: -a },
            max: c2v { x: -a, y: a },
        };
        make_proxy_case(&p, c2Proxy::default(), &raw(&sh), C2_TYPE_AABB, "aabb special");
    }
    // inverted + zero-area
    let z = c2v { x: 3.0, y: 4.0 };
    make_proxy_case(
        &p,
        c2Proxy::default(),
        &raw(&c2AABB { min: z, max: z }),
        C2_TYPE_AABB,
        "aabb zero-area",
    );
    make_proxy_case(
        &p,
        c2Proxy::default(),
        &raw(&c2AABB {
            min: c2v { x: 9.0, y: 9.0 },
            max: c2v { x: -9.0, y: -9.0 },
        }),
        C2_TYPE_AABB,
        "aabb inverted",
    );
}

#[test]
fn c19_c2MakeProxy_capsule() {
    let p = load();
    let mut rng = Rng::new(0x19);
    for _ in 0..N {
        let sh = rng.capsule();
        make_proxy_case(&p, garbage_proxy(&mut rng), &raw(&sh), C2_TYPE_CAPSULE, "capsule");
    }
    for a in specials() {
        let sh = c2Capsule {
            a: c2v { x: a, y: 1.0 },
            b: c2v { x: 2.0, y: a },
            r: a,
        };
        make_proxy_case(&p, c2Proxy::default(), &raw(&sh), C2_TYPE_CAPSULE, "capsule special");
    }
    // a == b (zero-length)
    let z = c2v { x: -7.0, y: 11.0 };
    make_proxy_case(
        &p,
        c2Proxy::default(),
        &raw(&c2Capsule { a: z, b: z, r: 3.0 }),
        C2_TYPE_CAPSULE,
        "capsule a==b",
    );
}

/// C20: prove the slots the C leaves alone (`verts[count..8]`, and `radius`
/// for no valid type) are preserved bit-for-bit by the Rust as well.
#[test]
fn c20_c2MakeProxy_preserves_untouched_slots() {
    let p = load();
    let mut rng = Rng::new(0x20);
    for _ in 0..N {
        let seed = garbage_proxy(&mut rng);
        let circle = rng.circle();
        let aabb = rng.aabb();
        let caps = rng.capsule();
        make_proxy_case(&p, seed, &raw(&circle), C2_TYPE_CIRCLE, "preserve/circle");
        make_proxy_case(&p, seed, &raw(&aabb), C2_TYPE_AABB, "preserve/aabb");
        make_proxy_case(&p, seed, &raw(&caps), C2_TYPE_CAPSULE, "preserve/capsule");
    }
}
