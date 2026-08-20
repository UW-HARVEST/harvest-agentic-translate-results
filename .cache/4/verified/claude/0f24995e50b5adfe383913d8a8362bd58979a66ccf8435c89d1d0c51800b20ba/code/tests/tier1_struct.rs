//! Phase B, `CONFIGS.md` rows 15–23: the out-pointer / struct-writing tier.
//!
//! `c2BBVerts` and `c2MakeProxy` write through raw pointers, so the whole
//! destination buffer is compared (all 72 bytes of `c2Proxy`, all 4 verts of
//! the corner array) — that catches "wrote too much" / "wrote too little" bugs
//! as well as wrong values.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::{c_int, c_void};

const N: usize = 20_000;

/// A recognisable fill pattern so that any byte the callee does NOT write is
/// still compared for equality (i.e. we detect over-writes).
fn poison_proxy(rng: &mut Rng) -> c2Proxy {
    let mut p = c2Proxy {
        radius: f32::from_bits(0xDEAD_BEEF),
        count: 0x5A5A_5A5A,
        verts: [c2v {
            x: f32::from_bits(0xCAFE_BABE),
            y: f32::from_bits(0xBAAD_F00D),
        }; 8],
    };
    for i in 0..8 {
        p.verts[i] = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
    }
    p
}

// ---------------------------------------------------------------------------
// row 15 — c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn row15_c2BBVerts() {
    let (c, r) = fnpair!("c2BBVerts", FnBBVerts);
    let mut rng = Rng::new(SEED ^ 15);

    let run = |bb: c2AABB, ctx: String| {
        // 8 slots: the callee must only touch the first 4.
        let fill = c2v {
            x: f32::from_bits(0x1234_5678),
            y: f32::from_bits(0x8765_4321),
        };
        let mut co = [fill; 8];
        let mut ro = [fill; 8];
        let mut cbb = bb;
        let mut rbb = bb;
        unsafe {
            c(co.as_mut_ptr(), &mut cbb);
            r(ro.as_mut_ptr(), &mut rbb);
        }
        eq_raw(&format!("c2BBVerts out {ctx}"), &co, &ro);
        // the input struct must not be modified either
        eq_raw(&format!("c2BBVerts in {ctx}"), &cbb, &rbb);
    };

    for i in 0..N {
        let bb = rng.aabb();
        run(bb, format!("#{i} {bb:?}"));
    }
    // explicit shapes: zero-size, inverted, huge, NaN
    for &s in SPECIALS.iter() {
        for shape in [
            c2AABB {
                min: c2v { x: s, y: s },
                max: c2v { x: s, y: s },
            },
            c2AABB {
                min: c2v { x: s, y: 0.0 },
                max: c2v { x: 0.0, y: s },
            },
            c2AABB {
                min: c2v { x: -1.0, y: -1.0 },
                max: c2v { x: s, y: 1.0 },
            },
        ] {
            run(shape, format!("special {s:?} {shape:?}"));
        }
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let shape = c2AABB {
                min: c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                },
                max: c2v {
                    x: f32::from_bits(q),
                    y: f32::from_bits(p),
                },
            };
            run(shape, format!("odd {shape:?}"));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 16–19 — c2MakeProxy for each of the 3 valid types, over a poisoned
//              destination so the untouched tail is verified too.
// ---------------------------------------------------------------------------

fn make_proxy_case(ty: C2_TYPE, shape_bytes: &[u8], rng: &mut Rng, ctx: &str) {
    let (c, r) = fnpair!("c2MakeProxy", FnMakeProxy);
    let seed = poison_proxy(rng);
    let mut cp = seed;
    let mut rp = seed;
    unsafe {
        c(shape_bytes.as_ptr() as *const c_void, ty, &mut cp);
        r(shape_bytes.as_ptr() as *const c_void, ty, &mut rp);
    }
    eq_raw(&format!("c2MakeProxy {ctx}"), &cp, &rp);
}

#[test]
fn row16_c2MakeProxy_circle() {
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..N {
        let sh = rng.circle();
        make_proxy_case(
            C2_TYPE_CIRCLE,
            raw(&sh),
            &mut rng,
            &format!("circle #{i} {sh:?}"),
        );
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let sh = c2Circle {
                p: c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                },
                r: f32::from_bits(q),
            };
            make_proxy_case(
                C2_TYPE_CIRCLE,
                raw(&sh),
                &mut rng,
                &format!("circle odd {sh:?}"),
            );
        }
    }
}

#[test]
fn row17_c2MakeProxy_aabb() {
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..N {
        let sh = rng.aabb();
        make_proxy_case(
            C2_TYPE_AABB,
            raw(&sh),
            &mut rng,
            &format!("aabb #{i} {sh:?}"),
        );
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let sh = c2AABB {
                min: c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                },
                max: c2v {
                    x: f32::from_bits(q),
                    y: f32::from_bits(p),
                },
            };
            make_proxy_case(
                C2_TYPE_AABB,
                raw(&sh),
                &mut rng,
                &format!("aabb odd {sh:?}"),
            );
        }
    }
}

#[test]
fn row18_c2MakeProxy_capsule() {
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..N {
        let sh = rng.capsule();
        make_proxy_case(
            C2_TYPE_CAPSULE,
            raw(&sh),
            &mut rng,
            &format!("capsule #{i} {sh:?}"),
        );
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let sh = c2Capsule {
                a: c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                },
                b: c2v {
                    x: f32::from_bits(q),
                    y: f32::from_bits(p),
                },
                r: f32::from_bits(p),
            };
            make_proxy_case(
                C2_TYPE_CAPSULE,
                raw(&sh),
                &mut rng,
                &format!("capsule odd {sh:?}"),
            );
        }
    }
}

/// row 19 — write each of the 3 types over a proxy that was previously filled
/// by a *different* type, so the stale `verts[n..8]` tail must survive
/// identically in both implementations.
#[test]
fn row19_c2MakeProxy_overwrite_sequence() {
    let (c, r) = fnpair!("c2MakeProxy", FnMakeProxy);
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..N {
        let mut cp = poison_proxy(&mut rng);
        let mut rp = cp;
        // three writes in a row, types chosen at random
        for step in 0..3 {
            let ty = ALL_TYPES[rng.below(3) as usize];
            let bytes: Vec<u8> = match ty {
                C2_TYPE_CIRCLE => raw(&rng.circle()).to_vec(),
                C2_TYPE_AABB => raw(&rng.aabb()).to_vec(),
                _ => raw(&rng.capsule()).to_vec(),
            };
            unsafe {
                c(bytes.as_ptr() as *const c_void, ty, &mut cp);
                r(bytes.as_ptr() as *const c_void, ty, &mut rp);
            }
            eq_raw(&format!("c2MakeProxy seq #{i} step={step} ty={ty}"), &cp, &rp);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 20–23 — c2Support at every proxy vertex count
// ---------------------------------------------------------------------------

fn support_case(verts: &[c2v], count: c_int, d: c2v, ctx: &str) {
    let (c, r) = fnpair!("c2Support", FnSupport);
    let (cv, rv) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
    eq_int(&format!("c2Support {ctx}"), cv, rv);
}

#[test]
fn row20_c2Support_count1() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..N {
        let verts = [rng.any_v(); 1];
        let d = rng.any_v();
        support_case(&verts, 1, d, &format!("#{i} count=1 d={d:?}"));
    }
}

#[test]
fn row21_c2Support_count2() {
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..N {
        let verts = [rng.any_v(), rng.any_v()];
        let d = rng.any_v();
        support_case(&verts, 2, d, &format!("#{i} count=2 d={d:?}"));
    }
    // exact ties: d perpendicular to the segment -> both dots equal -> index 0
    for i in 0..64 {
        let a = c2v {
            x: i as f32,
            y: 0.0,
        };
        let b = c2v {
            x: -(i as f32),
            y: 0.0,
        };
        let verts = [a, b];
        for d in [
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
        ] {
            support_case(&verts, 2, d, &format!("tie #{i} d={d:?}"));
        }
    }
}

#[test]
fn row22_c2Support_count4_aabb() {
    let (mkc, _mkr) = fnpair!("c2MakeProxy", FnMakeProxy);
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..N {
        // build a real AABB proxy, exactly like c2GJK does
        let bb = rng.aabb();
        let mut p = c2Proxy::default();
        unsafe { mkc(raw(&bb).as_ptr() as *const c_void, C2_TYPE_AABB, &mut p) };
        let d = rng.any_v();
        support_case(&p.verts, 4, d, &format!("#{i} aabb={bb:?} d={d:?}"));
    }
    // unit square, all 8 axis + diagonal directions (4 exact ties)
    let bb = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let mut p = c2Proxy::default();
    unsafe { mkc(raw(&bb).as_ptr() as *const c_void, C2_TYPE_AABB, &mut p) };
    for d in [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: -1.0, y: 1.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 0.0, y: 0.0 },
    ] {
        support_case(&p.verts, 4, d, &format!("unit-square d={d:?}"));
    }
}

#[test]
fn row23_c2Support_count8() {
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.any_v();
        }
        let d = rng.any_v();
        support_case(&verts, 8, d, &format!("#{i} count=8 d={d:?}"));
        // and every intermediate count, which is what a shorter proxy sees
        for cnt in 1..=8 {
            support_case(&verts, cnt, d, &format!("#{i} count={cnt} d={d:?}"));
        }
    }
}
