//! Phase B — Group 7: `ptr_from_parts`, `c2Collide`, `omni_manifold`.
//! CONFIGS.md rows 109..123.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_int;

const N: usize = 2048;
const KINDS: [u32; 3] = [0, 1, 2];
/// Every type value a caller can pass across the FFI boundary.
const ALL_TYPES: [c_int; 4] = [
    C2_TYPE_CAPSULE,
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_POLY,
];

// -------------------------------------------------------------------------
// rows 109..111 — ptr_from_parts for each handled type
// -------------------------------------------------------------------------
#[test]
fn cfg_ptr_from_parts() {
    let mut acc = DiffAccum::new("cfg_ptr_from_parts");
    let mut rng = Rng::new(0xbeed_0001);
    for i in 0..(N * 2) {
        let (a, b, c, d, e) = (
            rng.special(),
            rng.special(),
            rng.special(),
            rng.special(),
            rng.special(),
        );
        acc.check(format!("circle #{i}"), |s| {
            let p = ptr_from_parts(s, C2_TYPE_CIRCLE, a, b, c, d, e) as *const c2Circle;
            assert!(!p.is_null(), "malloc failed");
            unsafe { std::ptr::read_unaligned(p) }
        });
        acc.check(format!("aabb #{i}"), |s| {
            let p = ptr_from_parts(s, C2_TYPE_AABB, a, b, c, d, e) as *const c2AABB;
            assert!(!p.is_null());
            unsafe { std::ptr::read_unaligned(p) }
        });
        acc.check(format!("capsule #{i}"), |s| {
            let p = ptr_from_parts(s, C2_TYPE_CAPSULE, a, b, c, d, e) as *const c2Capsule;
            assert!(!p.is_null());
            unsafe { std::ptr::read_unaligned(p) }
        });
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// rows 112, 113 — c2Collide over the whole (typeA, typeB) matrix
// -------------------------------------------------------------------------
#[test]
fn cfg_collide_matrix() {
    let mut acc = DiffAccum::new("cfg_collide_matrix");
    let mut rng = Rng::new(0xbeed_0002);
    for ka in KINDS {
        for kb in KINDS {
            // four separation regimes
            for regime in 0..4 {
                for i in 0..N {
                    let sa = rng.nice_shape(ka);
                    let sb0 = rng.nice_shape(kb);
                    let sb = match regime {
                        0 => sb0.translate(rng.sym(0.3), rng.sym(0.3)), // deep
                        1 => sb0.translate(2.0 + rng.sym(0.5), 0.0),    // shallow
                        2 => sb0.translate(3.0, 0.0),                   // near touch
                        _ => sb0.translate(40.0, 40.0),                 // far
                    };
                    acc.check(format!("ka={ka} kb={kb} regime={regime} #{i}"), |s| {
                        with_sentinel(|m| {
                            c2Collide(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), sb.ty(), m)
                        })
                    });
                }
            }
            // fully random incl. degenerate / non-finite
            for i in 0..(N * 2) {
                let sa = rng.shape(ka);
                let sb = rng.shape(kb);
                acc.check(format!("rand ka={ka} kb={kb} #{i} {sa:?} {sb:?}"), |s| {
                    with_sentinel(|m| c2Collide(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), sb.ty(), m))
                });
            }
            for i in 0..N {
                let sa = rng.special_shape(ka);
                let sb = rng.special_shape(kb);
                acc.check(format!("special ka={ka} kb={kb} #{i} {sa:?} {sb:?}"), |s| {
                    with_sentinel(|m| c2Collide(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), sb.ty(), m))
                });
            }
        }
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// rows 114..120 — omni_manifold over all 16 declared type pairs
// -------------------------------------------------------------------------
#[test]
fn cfg_omni_matrix() {
    let mut acc = DiffAccum::new("cfg_omni_matrix");
    let mut rng = Rng::new(0xbeed_0003);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            // parameters are 5 floats regardless of type, so drive them directly
            for i in 0..(N * 2) {
                let p: [f32; 10] = std::array::from_fn(|_| rng.coord());
                acc.check(format!("ta={ta} tb={tb} rand #{i}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8],
                            p[9],
                        )
                    })
                });
            }
            // shape-aware, overlap-biased parameters
            for i in 0..(N * 2) {
                let ka = type_to_kind(ta);
                let kb = type_to_kind(tb);
                let sa = rng.nice_shape(ka);
                let sb = rng.nice_shape(kb).translate(rng.sym(2.0), rng.sym(2.0));
                let (a1, a2, a3, a4, a5) = sa.parts();
                let (b1, b2, b3, b4, b5) = sb.parts();
                acc.check(format!("ta={ta} tb={tb} nice #{i}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(s, m, ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5)
                    })
                });
            }
        }
    }
    acc.finish();
}

fn type_to_kind(t: c_int) -> u32 {
    match t {
        C2_TYPE_CIRCLE => 0,
        C2_TYPE_AABB => 1,
        _ => 2,
    }
}

// -------------------------------------------------------------------------
// rows 121, 122 — exhaustive small-lattice sweep
// -------------------------------------------------------------------------
#[test]
fn cfg_omni_lattice() {
    let mut acc = DiffAccum::new("cfg_omni_lattice");
    // Circle/circle: exhaustive over a 3×3 centre lattice × 3 radii, both sides.
    let coords = [-1.0f32, 0.0, 1.0];
    let radii = [0.0f32, 0.5, 1.0];
    for &ax in &coords {
        for &ay in &coords {
            for &ar in &radii {
                for &bx in &coords {
                    for &by in &coords {
                        for &br in &radii {
                            acc.check(
                                format!("cc {ax},{ay},{ar} / {bx},{by},{br}"),
                                |s| {
                                    with_sentinel(|m| {
                                        omni_manifold(
                                            s,
                                            m,
                                            C2_TYPE_CIRCLE,
                                            ax,
                                            ay,
                                            ar,
                                            0.0,
                                            0.0,
                                            C2_TYPE_CIRCLE,
                                            bx,
                                            by,
                                            br,
                                            0.0,
                                            0.0,
                                        )
                                    })
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    // AABB/AABB: exhaustive over a lattice that includes degenerate and inverted
    // boxes (min == max and min > max).
    for &a0 in &coords {
        for &a1 in &coords {
            for &b0 in &coords {
                for &b1 in &coords {
                    acc.check(format!("bb {a0},{a1} / {b0},{b1}"), |s| {
                        with_sentinel(|m| {
                            omni_manifold(
                                s,
                                m,
                                C2_TYPE_AABB,
                                a0,
                                a0,
                                a1,
                                a1,
                                0.0,
                                C2_TYPE_AABB,
                                b0,
                                b0,
                                b1,
                                b1,
                                0.0,
                            )
                        })
                    });
                }
            }
        }
    }
    // AABB/CAPSULE and CAPSULE/AABB over the same lattice (this is the path that
    // goes through the uninitialised POLY proxy in `c2GJK`).
    for &a0 in &coords {
        for &a1 in &coords {
            for &c0 in &coords {
                for &c1 in &coords {
                    for &r in &radii {
                        acc.check(format!("bb/ca {a0},{a1} / {c0},{c1},{r}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_AABB,
                                    a0,
                                    a0,
                                    a1,
                                    a1,
                                    0.0,
                                    C2_TYPE_CAPSULE,
                                    c0,
                                    c0,
                                    c1,
                                    c1,
                                    r,
                                )
                            })
                        });
                        acc.check(format!("ca/bb {c0},{c1},{r} / {a0},{a1}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_CAPSULE,
                                    c0,
                                    c0,
                                    c1,
                                    c1,
                                    r,
                                    C2_TYPE_AABB,
                                    a0,
                                    a0,
                                    a1,
                                    a1,
                                    0.0,
                                )
                            })
                        });
                    }
                }
            }
        }
    }
    // CAPSULE/CAPSULE, CIRCLE/AABB, CIRCLE/CAPSULE over the lattice
    for &a0 in &coords {
        for &a1 in &coords {
            for &b0 in &coords {
                for &b1 in &coords {
                    for &r in &radii {
                        acc.check(format!("ca/ca {a0},{a1} / {b0},{b1},{r}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_CAPSULE,
                                    a0,
                                    a0,
                                    a1,
                                    a1,
                                    r,
                                    C2_TYPE_CAPSULE,
                                    b0,
                                    b0,
                                    b1,
                                    b1,
                                    r,
                                )
                            })
                        });
                        acc.check(format!("ci/bb {a0},{a1},{r} / {b0},{b1}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_CIRCLE,
                                    a0,
                                    a1,
                                    r,
                                    0.0,
                                    0.0,
                                    C2_TYPE_AABB,
                                    b0,
                                    b0,
                                    b1,
                                    b1,
                                    0.0,
                                )
                            })
                        });
                        acc.check(format!("bb/ci {b0},{b1} / {a0},{a1},{r}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_AABB,
                                    b0,
                                    b0,
                                    b1,
                                    b1,
                                    0.0,
                                    C2_TYPE_CIRCLE,
                                    a0,
                                    a1,
                                    r,
                                    0.0,
                                    0.0,
                                )
                            })
                        });
                        acc.check(format!("ci/ca {a0},{a1},{r} / {b0},{b1}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_CIRCLE,
                                    a0,
                                    a1,
                                    r,
                                    0.0,
                                    0.0,
                                    C2_TYPE_CAPSULE,
                                    b0,
                                    b0,
                                    b1,
                                    b1,
                                    r,
                                )
                            })
                        });
                        acc.check(format!("ca/ci {b0},{b1},{r} / {a0},{a1}"), |s| {
                            with_sentinel(|m| {
                                omni_manifold(
                                    s,
                                    m,
                                    C2_TYPE_CAPSULE,
                                    b0,
                                    b0,
                                    b1,
                                    b1,
                                    r,
                                    C2_TYPE_CIRCLE,
                                    a0,
                                    a1,
                                    r,
                                    0.0,
                                    0.0,
                                )
                            })
                        });
                    }
                }
            }
        }
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// row 123 — non-finite / extreme parameters through omni_manifold
// -------------------------------------------------------------------------
#[test]
fn cfg_omni_extreme() {
    let mut acc = DiffAccum::new("cfg_omni_extreme");
    let mut rng = Rng::new(0xbeed_0004);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for i in 0..(N * 2) {
                let p: [f32; 10] = std::array::from_fn(|_| rng.special());
                acc.check(format!("ta={ta} tb={tb} special #{i} {p:?}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8],
                            p[9],
                        )
                    })
                });
            }
            for i in 0..N {
                let p: [f32; 10] = std::array::from_fn(|_| rng.any_f32());
                acc.check(format!("ta={ta} tb={tb} any #{i} {p:?}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8],
                            p[9],
                        )
                    })
                });
            }
        }
    }
    acc.finish();
}
