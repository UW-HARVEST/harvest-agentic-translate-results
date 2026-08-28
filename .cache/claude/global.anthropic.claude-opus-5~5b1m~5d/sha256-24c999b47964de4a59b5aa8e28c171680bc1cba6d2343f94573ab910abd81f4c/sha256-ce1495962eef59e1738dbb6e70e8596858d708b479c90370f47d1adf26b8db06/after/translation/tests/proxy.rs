//! Phase B, Group 2 — CONFIGS.md rows C17..C20 (proxy construction),
//! plus ERRORS.md rows E12..E15, E68..E70.
//!
//! `c2MakeProxy`'s `switch` has no `default:` label, so the *untouched* fields
//! of `*p` are observable behaviour.  Every call therefore starts from a
//! poisoned `c2Proxy` and the whole 72-byte image is compared afterwards.

mod common;
use common::*;

const N: u32 = 4096;

fn poison(rng: &mut Rng) -> C2Proxy {
    let mut p = C2Proxy {
        radius: rng.spicy(),
        count: rng.next_u32() as i32,
        verts: [C2v::default(); 8],
    };
    for v in p.verts.iter_mut() {
        *v = rng.v_spicy();
    }
    p
}

// ---------------------------------------------------------------------------
// C17 c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn c17_bbverts() {
    let (c, r): (FnBBVerts, FnBBVerts) = sym(b"c2BBVerts");
    let mut rng = Rng::new(0xC17);
    for i in 0..N {
        // Random, plus normal / inverted / empty / spicy boxes.
        let boxes = [
            rand_aabb(&mut rng, 500.0),
            {
                let a = rand_aabb(&mut rng, 500.0);
                C2AABB { min: a.max, max: a.min } // inverted
            },
            {
                let v = rng.v_range(-500.0, 500.0);
                C2AABB { min: v, max: v } // empty
            },
            C2AABB {
                min: rng.v_spicy(),
                max: rng.v_spicy(),
            },
        ];
        for mut bb in boxes {
            let mut oc = [C2v { x: -7.5, y: 13.25 }; 4];
            let mut or_ = oc;
            unsafe { c(oc.as_mut_ptr(), &mut bb) };
            unsafe { r(or_.as_mut_ptr(), &mut bb) };
            assert_raw(&oc, &or_, &format!("c2BBVerts #{i}"));
        }
    }
    // fixed boundary boxes
    let fixed = [
        C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 0.0, y: 0.0 },
        },
        C2AABB {
            min: C2v { x: -0.0, y: -0.0 },
            max: C2v { x: 0.0, y: 0.0 },
        },
        C2AABB {
            min: C2v {
                x: f32::NEG_INFINITY,
                y: f32::NEG_INFINITY,
            },
            max: C2v {
                x: f32::INFINITY,
                y: f32::INFINITY,
            },
        },
        C2AABB {
            min: C2v { x: f32::NAN, y: 1.0 },
            max: C2v { x: 2.0, y: f32::NAN },
        },
        C2AABB {
            min: C2v { x: FLT_MAX, y: FLT_MAX },
            max: C2v {
                x: -FLT_MAX,
                y: -FLT_MAX,
            },
        },
    ];
    for mut bb in fixed {
        let mut oc = [C2v { x: 1.0, y: 2.0 }; 4];
        let mut or_ = oc;
        unsafe { c(oc.as_mut_ptr(), &mut bb) };
        unsafe { r(or_.as_mut_ptr(), &mut bb) };
        assert_raw(&oc, &or_, "c2BBVerts fixed");
    }
}

// ---------------------------------------------------------------------------
// C18/C19/C20 + E15 — c2MakeProxy for each valid type, poisoned destination
// ---------------------------------------------------------------------------

fn make_proxy_sweep(ty: u32, seed: u64) {
    let (c, r): (FnMakeProxy, FnMakeProxy) = sym(b"c2MakeProxy");
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let shape = match ty {
            C2_TYPE_CIRCLE => {
                // random / zero radius / negative radius / spicy
                match i % 4 {
                    0 => ShapeBlob::circle(rand_circle(&mut rng, 500.0)),
                    1 => ShapeBlob::circle(C2Circle {
                        p: rng.v_range(-500.0, 500.0),
                        r: 0.0,
                    }),
                    2 => ShapeBlob::circle(C2Circle {
                        p: rng.v_range(-500.0, 500.0),
                        r: -rng.range(0.0, 100.0),
                    }),
                    _ => ShapeBlob::circle(C2Circle {
                        p: rng.v_spicy(),
                        r: rng.spicy(),
                    }),
                }
            }
            C2_TYPE_AABB => match i % 4 {
                0 => ShapeBlob::aabb(rand_aabb(&mut rng, 500.0)),
                1 => {
                    let a = rand_aabb(&mut rng, 500.0);
                    ShapeBlob::aabb(C2AABB { min: a.max, max: a.min })
                }
                2 => {
                    let v = rng.v_range(-500.0, 500.0);
                    ShapeBlob::aabb(C2AABB { min: v, max: v })
                }
                _ => ShapeBlob::aabb(C2AABB {
                    min: rng.v_spicy(),
                    max: rng.v_spicy(),
                }),
            },
            _ => match i % 4 {
                0 => ShapeBlob::capsule(rand_capsule(&mut rng, 500.0)),
                1 => {
                    let v = rng.v_range(-500.0, 500.0);
                    ShapeBlob::capsule(C2Capsule {
                        a: v,
                        b: v,
                        r: rng.range(0.0, 50.0),
                    })
                }
                2 => ShapeBlob::capsule(C2Capsule {
                    a: rng.v_range(-500.0, 500.0),
                    b: rng.v_range(-500.0, 500.0),
                    r: -rng.range(0.0, 50.0),
                }),
                _ => ShapeBlob::capsule(C2Capsule {
                    a: rng.v_spicy(),
                    b: rng.v_spicy(),
                    r: rng.spicy(),
                }),
            },
        };
        let base = poison(&mut rng);
        let mut pc = base;
        let mut pr = base;
        unsafe { c(shape.as_ptr(), ty, &mut pc) };
        unsafe { r(shape.as_ptr(), ty, &mut pr) };
        assert_raw(
            &pc,
            &pr,
            &format!("c2MakeProxy {} #{i}", type_name(ty)),
        );
    }
}

#[test]
fn c18_makeproxy_circle() {
    make_proxy_sweep(C2_TYPE_CIRCLE, 0xC18);
}

#[test]
fn c19_makeproxy_aabb() {
    make_proxy_sweep(C2_TYPE_AABB, 0xC19);
}

#[test]
fn c20_makeproxy_capsule() {
    make_proxy_sweep(C2_TYPE_CAPSULE, 0xC20);
}
