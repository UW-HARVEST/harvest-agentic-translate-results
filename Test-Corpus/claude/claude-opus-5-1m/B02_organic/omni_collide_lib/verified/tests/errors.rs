//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input / degenerate condition the C
//! checks, calls BOTH `.so`s, and asserts the SAME rejection value (same error
//! code / sentinel / bit pattern), not merely "both failed somehow".
//!
//! Rows marked `UB` in `ERRORS.md` are documented at the bottom of this file
//! with the reason they cannot be differentially asserted.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::{c_int, c_void};

// ===========================================================================
// rows 1–8 — out-of-range C2_TYPE enum values across the FFI boundary
// ===========================================================================

/// A `C2_TYPE` value with no valid variant is a real input: C enums accept any
/// `int`, and `c2Collided`'s `default:` labels are the C's rejection path.
#[test]
fn rows01to04_c2Collided_invalid_enums() {
    let (c, r) = fnpair!("c2Collided", FnCollided);
    let mut rng = Rng::new(SEED ^ 101);

    // A big shape buffer that is valid for every type, so the only thing under
    // test is the enum dispatch.
    let buf = [0u8; 32];

    for _ in 0..200 {
        // row 1: typeA invalid (outer default) -- typeB both valid and invalid
        for &bad_a in BAD_TYPES.iter() {
            for &tb in ALL_TYPES.iter().chain(BAD_TYPES.iter()) {
                let (cv, rv) = unsafe {
                    (
                        c(
                            buf.as_ptr() as *const c_void,
                            bad_a,
                            buf.as_ptr() as *const c_void,
                            tb,
                        ),
                        r(
                            buf.as_ptr() as *const c_void,
                            bad_a,
                            buf.as_ptr() as *const c_void,
                            tb,
                        ),
                    )
                };
                eq_int(&format!("c2Collided typeA={bad_a} typeB={tb}"), cv, rv);
                assert_eq!(cv, 0, "C must reject typeA={bad_a} with 0");
            }
        }

        // rows 2–4: typeA valid, typeB invalid (each of the three inner
        // `default:` labels)
        for &ta in ALL_TYPES.iter() {
            let ab = match ta {
                C2_TYPE_CIRCLE => raw(&rng.circle()).to_vec(),
                C2_TYPE_AABB => raw(&rng.aabb()).to_vec(),
                _ => raw(&rng.capsule()).to_vec(),
            };
            for &bad_b in BAD_TYPES.iter() {
                let (cv, rv) = unsafe {
                    (
                        c(
                            ab.as_ptr() as *const c_void,
                            ta,
                            buf.as_ptr() as *const c_void,
                            bad_b,
                        ),
                        r(
                            ab.as_ptr() as *const c_void,
                            ta,
                            buf.as_ptr() as *const c_void,
                            bad_b,
                        ),
                    )
                };
                eq_int(&format!("c2Collided typeA={ta} typeB={bad_b}"), cv, rv);
                assert_eq!(cv, 0, "C must reject typeB={bad_b} with 0");
            }
        }
    }
}

/// rows 5–8 — the same through the *public* entry point, where the garbage
/// pointer that `ptr_from_parts` returns for an invalid type would be
/// dereferenced if `c2Collided` did not filter it first.
#[test]
fn rows05to08_omni_collide_invalid_enums() {
    let (c, r) = fnpair!("omni_collide", FnOmniCollide);
    let mut rng = Rng::new(SEED ^ 105);

    let all: Vec<C2_TYPE> = ALL_TYPES.iter().chain(BAD_TYPES.iter()).copied().collect();

    for i in 0..500 {
        for &ta in all.iter() {
            for &tb in all.iter() {
                if ALL_TYPES.contains(&ta) && ALL_TYPES.contains(&tb) {
                    continue; // that is Phase B's job
                }
                let a = random_parts(&mut rng, ta);
                let b = random_parts(&mut rng, tb);
                let (cv, rv) = unsafe {
                    (
                        c(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
                        r(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
                    )
                };
                eq_int(
                    &format!("omni_collide #{i} ta={ta} tb={tb} a={a:?} b={b:?}"),
                    cv,
                    rv,
                );
                assert_eq!(cv, 0, "C must return 0 for ta={ta} tb={tb}");
            }
        }
    }

    // row 8 specifically: the value one past the last variant.
    const ONE_PAST: C2_TYPE = 3;
    for &ok in ALL_TYPES.iter() {
        for (ta, tb) in [
            (ONE_PAST, ok),
            (ok, ONE_PAST),
            (ONE_PAST, ONE_PAST),
            (ONE_PAST, 0xFFFF_FFFF),
        ] {
            let a = random_parts(&mut rng, ta);
            let b = random_parts(&mut rng, tb);
            let (cv, rv) = unsafe {
                (
                    c(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
                    r(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]),
                )
            };
            eq_int(&format!("omni_collide one-past ta={ta} tb={tb}"), cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

// ===========================================================================
// row 9 — c2MakeProxy with an invalid type writes NOTHING
// ===========================================================================

#[test]
fn row09_c2MakeProxy_invalid_type_writes_nothing() {
    let (c, r) = fnpair!("c2MakeProxy", FnMakeProxy);
    let mut rng = Rng::new(SEED ^ 109);
    let shape = [0x5Au8; 32];

    for i in 0..2_000 {
        // A distinctive pre-existing content; the C `switch` has no `default:`
        // so every byte must survive untouched.
        let mut seed = c2Proxy {
            radius: f32::from_bits(rng.u32()),
            count: rng.u32() as c_int,
            verts: [c2v::default(); 8],
        };
        for v in seed.verts.iter_mut() {
            *v = c2v {
                x: f32::from_bits(rng.u32()),
                y: f32::from_bits(rng.u32()),
            };
        }
        for &bad in BAD_TYPES.iter() {
            let mut cp = seed;
            let mut rp = seed;
            unsafe {
                c(shape.as_ptr() as *const c_void, bad, &mut cp);
                r(shape.as_ptr() as *const c_void, bad, &mut rp);
            }
            eq_raw(&format!("c2MakeProxy #{i} bad type={bad}"), &cp, &rp);
            eq_raw(
                &format!("c2MakeProxy #{i} type={bad} must not write"),
                &cp,
                &seed,
            );
        }
    }
}

// ===========================================================================
// rows 10–11 — c2GJKSimplexMetric with count outside 1..3
// ===========================================================================

#[test]
fn rows10to11_c2GJKSimplexMetric_bad_count() {
    let (c, r) = fnpair!("c2GJKSimplexMetric", FnSimplexF);
    let mut rng = Rng::new(SEED ^ 110);
    let counts: [c_int; 11] = [1, 0, -1, 4, 5, 99, -99, i32::MIN, i32::MAX, 2, 3];

    for i in 0..2_000 {
        let mut s = c2Simplex {
            verts: [c2sv::default(); 4],
            div: f32::from_bits(rng.u32()),
            count: 0,
        };
        for v in s.verts.iter_mut() {
            v.p = c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            };
            v.u = rng.any_f32();
        }
        for &cnt in counts.iter() {
            s.count = cnt;
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            eq_f32(&format!("metric #{i} count={cnt}"), cv, rv);
            if cnt != 2 && cnt != 3 {
                assert_eq!(
                    cv.to_bits(),
                    0.0f32.to_bits(),
                    "C must return +0.0 for count={cnt}, got {cv}"
                );
            }
        }
    }
}

// ===========================================================================
// rows 12–13 — c2D
// ===========================================================================

#[test]
fn rows12to13_c2D_bad_count_and_ccw90_fallback() {
    let (c, r) = fnpair!("c2D", FnSimplexV);
    let mut rng = Rng::new(SEED ^ 112);
    let counts: [c_int; 9] = [3, 0, -1, 4, 5, 99, i32::MIN, i32::MAX, 2];

    for i in 0..2_000 {
        let mut s = c2Simplex {
            verts: [c2sv::default(); 4],
            div: rng.any_f32(),
            count: 0,
        };
        for v in s.verts.iter_mut() {
            v.p = c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            };
        }
        for &cnt in counts.iter() {
            s.count = cnt;
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            eq_raw(&format!("c2D #{i} count={cnt}"), &cv, &rv);
            if cnt != 1 && cnt != 2 {
                // row 12: `case 3: default:` -> c2V(0,0)
                eq_raw(
                    &format!("c2D #{i} count={cnt} must be (0,0)"),
                    &cv,
                    &c2v { x: 0.0, y: 0.0 },
                );
            }
        }
    }

    // row 13: count == 2 with c2Det2(ab, -a) <= 0 -> c2CCW90 fallback.
    // Collinear a/b make the determinant exactly 0, which is `<= 0`.
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div: 1.0,
        count: 2,
    };
    for k in 1..200 {
        let base = c2v {
            x: k as f32 * 0.5,
            y: k as f32 * -0.25,
        };
        s.verts[0].p = base;
        s.verts[1].p = c2v {
            x: base.x * 3.0,
            y: base.y * 3.0,
        };
        let mut cs = s;
        let mut rs = s;
        let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
        eq_raw(&format!("c2D collinear k={k}"), &cv, &rv);
    }
}

// ===========================================================================
// rows 14–16 — c2L
// ===========================================================================

#[test]
fn rows14to16_c2L_bad_count_and_zero_div() {
    let (c, r) = fnpair!("c2L", FnSimplexV);
    let mut rng = Rng::new(SEED ^ 114);
    let counts: [c_int; 9] = [0, 3, 4, -1, 99, i32::MIN, i32::MAX, 1, 2];
    // row 15 / 16: div == +0.0 and -0.0 -> den = ±inf
    let divs: [f32; 8] = [
        0.0,
        -0.0,
        1.0,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
    ];

    for i in 0..2_000 {
        let mut s = c2Simplex {
            verts: [c2sv::default(); 4],
            div: 1.0,
            count: 0,
        };
        for v in s.verts.iter_mut() {
            v.p = c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            };
            v.u = rng.any_f32();
        }
        for &cnt in counts.iter() {
            for &d in divs.iter() {
                s.count = cnt;
                s.div = d;
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
                eq_raw(&format!("c2L #{i} count={cnt} div={d:?}"), &cv, &rv);
                if cnt != 1 && cnt != 2 {
                    // row 14: `default:` -> c2V(0,0)
                    eq_raw(
                        &format!("c2L #{i} count={cnt} must be (0,0)"),
                        &cv,
                        &c2v { x: 0.0, y: 0.0 },
                    );
                }
            }
        }
    }
}

// ===========================================================================
// rows 17–19 — c2Witness
// ===========================================================================

#[test]
fn rows17to19_c2Witness_bad_count_and_zero_div() {
    let (c, r) = fnpair!("c2Witness", FnWitness);
    let mut rng = Rng::new(SEED ^ 117);
    let counts: [c_int; 9] = [0, 4, -1, 5, 99, i32::MIN, i32::MAX, 1, 2];
    let divs: [f32; 8] = [
        0.0,
        -0.0,
        1.0,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
    ];
    let poison = c2v {
        x: f32::from_bits(0xFEED_FACE),
        y: f32::from_bits(0x0BAD_C0DE),
    };

    for i in 0..1_500 {
        let mut s = c2Simplex {
            verts: [c2sv::default(); 4],
            div: 1.0,
            count: 0,
        };
        for v in s.verts.iter_mut() {
            v.sA = c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            };
            v.sB = c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            };
            v.u = rng.any_f32();
        }
        for &cnt in counts.iter() {
            for &d in divs.iter() {
                s.count = cnt;
                s.div = d;
                let mut cs = s;
                let mut rs = s;
                let (mut ca, mut cb) = (poison, poison);
                let (mut ra, mut rb) = (poison, poison);
                unsafe {
                    c(&mut cs, &mut ca, &mut cb);
                    r(&mut rs, &mut ra, &mut rb);
                }
                let ctx = format!("c2Witness #{i} count={cnt} div={d:?}");
                eq_raw(&format!("{ctx} a"), &ca, &ra);
                eq_raw(&format!("{ctx} b"), &cb, &rb);
                if !(1..=3).contains(&cnt) {
                    // row 17: `default:` -> both (0,0)
                    eq_raw(&format!("{ctx} a must be (0,0)"), &ca, &c2v { x: 0.0, y: 0.0 });
                    eq_raw(&format!("{ctx} b must be (0,0)"), &cb, &c2v { x: 0.0, y: 0.0 });
                }
                if cnt == 1 {
                    // row 19: den computed but unused
                    eq_raw(&format!("{ctx} a == sA"), &ca, &cs.verts[0].sA);
                    eq_raw(&format!("{ctx} b == sB"), &cb, &cs.verts[0].sB);
                }
            }
        }
    }
}

// ===========================================================================
// rows 20–24 — c2Support with zero / negative / boundary counts
// ===========================================================================

#[test]
fn rows20to24_c2Support_bad_counts_and_ties() {
    let (c, r) = fnpair!("c2Support", FnSupport);
    let mut rng = Rng::new(SEED ^ 120);
    // rows 20/21/22: count 0, negative, and the no-loop boundary 1
    let counts: [c_int; 8] = [0, 1, -1, -2, -100, i32::MIN, 2, 8];

    for i in 0..3_000 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            };
        }
        for &cnt in counts.iter() {
            let d = rng.any_v();
            let (cv, rv) = unsafe { (c(verts.as_ptr(), cnt, d), r(verts.as_ptr(), cnt, d)) };
            eq_int(&format!("c2Support #{i} count={cnt} d={d:?}"), cv, rv);
            if cnt <= 1 {
                assert_eq!(cv, 0, "C must return 0 for count={cnt}");
            }
        }
    }

    // row 23: all dots equal (d == (0,0) -> every dot is +0.0, `>` never true)
    for i in 0..200 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v();
        }
        for &cnt in &[1i32, 2, 3, 4, 8] {
            for d in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -0.0, y: -0.0 },
                c2v { x: 0.0, y: -0.0 },
            ] {
                let (cv, rv) = unsafe { (c(verts.as_ptr(), cnt, d), r(verts.as_ptr(), cnt, d)) };
                eq_int(&format!("c2Support zero-d #{i} count={cnt} d={d:?}"), cv, rv);
                assert_eq!(cv, 0, "ties must resolve to index 0");
            }
        }
        // identical verts -> all dots identical -> index 0
        let same = [rng.v(); 8];
        for &cnt in &[1i32, 2, 4, 8] {
            let d = rng.v();
            let (cv, rv) = unsafe { (c(same.as_ptr(), cnt, d), r(same.as_ptr(), cnt, d)) };
            eq_int(&format!("c2Support same-verts #{i} count={cnt}"), cv, rv);
            assert_eq!(cv, 0);
        }
    }

    // row 24: d contains NaN -> every `dot > dmax` is false -> 0
    for i in 0..200 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v();
        }
        for d in [
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: 0.0, y: f32::NAN },
            c2v {
                x: f32::NAN,
                y: f32::NAN,
            },
            c2v {
                x: f32::from_bits(0x7FC0_1234),
                y: 1.0,
            },
        ] {
            for &cnt in &[1i32, 2, 4, 8] {
                let (cv, rv) = unsafe { (c(verts.as_ptr(), cnt, d), r(verts.as_ptr(), cnt, d)) };
                eq_int(&format!("c2Support NaN-d #{i} count={cnt} d={d:?}"), cv, rv);
                assert_eq!(cv, 0, "NaN direction must yield index 0");
            }
        }
        // NaN inside the verts: a NaN dot is never `>` dmax, so it is skipped
        let mut nv = verts;
        nv[rng.below(8) as usize] = c2v {
            x: f32::NAN,
            y: f32::NAN,
        };
        let d = rng.v();
        let (cv, rv) = unsafe { (c(nv.as_ptr(), 8, d), r(nv.as_ptr(), 8, d)) };
        eq_int(&format!("c2Support NaN-vert #{i}"), cv, rv);
    }
}

// ===========================================================================
// rows 25–30 — division by zero / NaN propagation in c2Div, c2Norm, c2Len
// ===========================================================================

#[test]
fn rows25to26_c2Div_by_zero() {
    let (c, r) = fnpair!("c2Div", FnVvF);
    let mut rng = Rng::new(SEED ^ 125);
    let zeros: [f32; 2] = [0.0, -0.0];

    for i in 0..2_000 {
        let a = rng.any_v();
        for &z in zeros.iter() {
            let (cv, rv) = (c(a, z), r(a, z));
            eq_raw(&format!("c2Div #{i} {a:?} / {z:?}"), &cv, &rv);
        }
    }
    // exhaustive over the interesting numerators
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            for &z in zeros.iter() {
                eq_raw(&format!("c2Div odd {a:?} / {z:?}"), &c(a, z), &r(a, z));
            }
        }
    }
    // 0/0 must give NaN with the same payload
    for &z in zeros.iter() {
        for &z2 in zeros.iter() {
            let a = c2v { x: z, y: z2 };
            let cv = c(a, z);
            assert!(cv.x.is_nan() && cv.y.is_nan(), "0/0 should be NaN");
            eq_raw(&format!("c2Div 0/0 {a:?} / {z:?}"), &cv, &r(a, z));
        }
    }
}

#[test]
fn rows27to28_c2Norm_zero_and_nan() {
    let (c, r) = fnpair!("c2Norm", FnVv);
    // row 27: zero-length input -> c2Len == 0 -> 0/0 -> NaN
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        let cv = c(a);
        assert!(
            cv.x.is_nan() && cv.y.is_nan(),
            "C c2Norm({a:?}) should be NaN, got {cv:?}"
        );
        eq_raw(&format!("c2Norm zero {a:?}"), &cv, &r(a));
    }
    // row 28: NaN / inf components
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            eq_raw(&format!("c2Norm odd {a:?}"), &c(a), &r(a));
        }
    }
    for &s in SPECIALS.iter() {
        for other in [0.0f32, 1.0, -1.0, f32::INFINITY, f32::NAN] {
            for a in [c2v { x: s, y: other }, c2v { x: other, y: s }] {
                eq_raw(&format!("c2Norm special {a:?}"), &c(a), &r(a));
            }
        }
    }
}

#[test]
fn rows29to30_c2Len_nan_and_overflow() {
    let (c, r) = fnpair!("c2Len", FnFv);
    // row 29: NaN component -> sqrtf(NaN)
    for a in [
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: 0.0, y: f32::NAN },
        c2v {
            x: f32::from_bits(0x7FC0_1234),
            y: 1.0,
        },
        c2v {
            x: f32::from_bits(0xFFC0_0000),
            y: 1.0,
        },
        c2v {
            x: f32::from_bits(0x7F80_0001), // sNaN
            y: 1.0,
        },
    ] {
        let cv = c(a);
        assert!(cv.is_nan(), "C c2Len({a:?}) should be NaN, got {cv}");
        eq_f32(&format!("c2Len NaN {a:?}"), cv, r(a));
    }
    // row 30: dot overflows to +inf -> sqrtf(inf) == inf
    for a in [
        c2v { x: 1e30, y: 1e30 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MAX, y: 0.0 },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
        c2v {
            x: f32::NEG_INFINITY,
            y: f32::INFINITY,
        },
        c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
    ] {
        eq_f32(&format!("c2Len overflow {a:?}"), c(a), r(a));
    }
    // inf * 0 inside the dot -> NaN
    for a in [
        c2v {
            x: f32::INFINITY,
            y: f32::NAN,
        },
        c2v {
            x: 0.0,
            y: f32::INFINITY,
        },
    ] {
        eq_f32(&format!("c2Len mixed {a:?}"), c(a), r(a));
    }
}

// ===========================================================================
// rows 31–41 — c2GJK NULL-pointer guards and the cache predicate
// ===========================================================================

/// All of `c2GJK`'s observable outputs for one call.
#[derive(Debug, Clone, Copy)]
struct GjkOut {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

const POISON_V: c2v = c2v {
    x: -1.234_567_9e33,
    y: 9.876_543e-21,
};
const POISON_IT: c_int = -987_654_321;

#[allow(clippy::too_many_arguments)]
fn gjk_raw(
    f: FnGJK,
    ab: &[u8],
    ta: C2_TYPE,
    ax: *const c2x,
    bb: &[u8],
    tb: C2_TYPE,
    bx: *const c2x,
    outa: bool,
    outb: bool,
    iters: bool,
    cache: Option<c2GJKCache>,
    use_radius: c_int,
) -> GjkOut {
    let mut a = POISON_V;
    let mut b = POISON_V;
    let mut it = POISON_IT;
    let mut ch = cache.unwrap_or_default();
    let dist = unsafe {
        f(
            ab.as_ptr() as *const c_void,
            ta,
            ax,
            bb.as_ptr() as *const c_void,
            tb,
            bx,
            if outa { &mut a } else { std::ptr::null_mut() },
            if outb { &mut b } else { std::ptr::null_mut() },
            use_radius,
            if iters {
                &mut it
            } else {
                std::ptr::null_mut()
            },
            if cache.is_some() {
                &mut ch
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOut {
        dist,
        a,
        b,
        iters: it,
        cache: ch,
    }
}

#[track_caller]
fn eq_gjk(ctx: &str, c: &GjkOut, r: &GjkOut) {
    eq_f32(&format!("{ctx} [return]"), c.dist, r.dist);
    eq_raw(&format!("{ctx} [outA]"), &c.a, &r.a);
    eq_raw(&format!("{ctx} [outB]"), &c.b, &r.b);
    eq_int(&format!("{ctx} [iterations]"), c.iters, r.iters);
    eq_raw(&format!("{ctx} [cache]"), &c.cache, &r.cache);
}

fn shape_bytes(rng: &mut Rng, ty: C2_TYPE) -> Vec<u8> {
    match ty {
        C2_TYPE_CIRCLE => raw(&rng.circle()).to_vec(),
        C2_TYPE_AABB => raw(&rng.aabb()).to_vec(),
        _ => raw(&rng.capsule()).to_vec(),
    }
}

/// rows 31–32 — `ax_ptr` / `bx_ptr` NULL must behave exactly like passing
/// `c2xIdentity()`, and must not be dereferenced.
#[test]
fn rows31to32_gjk_null_transforms() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let (cxid, _) = fnpair!("c2xIdentity", FnX);
    let ident = cxid();
    let mut rng = Rng::new(SEED ^ 131);

    for i in 0..2_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let ab = shape_bytes(&mut rng, ta);
        let bb = shape_bytes(&mut rng, tb);
        for ur in [0i32, 1] {
            // NULL / NULL
            let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, None, ur);
            let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, None, ur);
            eq_gjk(&format!("row31/32 null/null #{i} ur={ur}"), &co, &ro);

            // NULL must equal explicit identity (the C's documented default)
            let ci = gjk_raw(cf, &ab, ta, &ident, &bb, tb, &ident, true, true, true, None, ur);
            eq_f32(
                &format!("row31/32 NULL==identity #{i} ur={ur}"),
                co.dist,
                ci.dist,
            );
            eq_raw(&format!("row31/32 NULL==identity outA #{i}"), &co.a, &ci.a);
            eq_raw(&format!("row31/32 NULL==identity outB #{i}"), &co.b, &ci.b);

            // one NULL, one real
            let x = rng.x();
            let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, &x, true, true, true, None, ur);
            let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, &x, true, true, true, None, ur);
            eq_gjk(&format!("row31 axNULL #{i} ur={ur}"), &co, &ro);
            let co = gjk_raw(cf, &ab, ta, &x, &bb, tb, std::ptr::null(), true, true, true, None, ur);
            let ro = gjk_raw(rf, &ab, ta, &x, &bb, tb, std::ptr::null(), true, true, true, None, ur);
            eq_gjk(&format!("row32 bxNULL #{i} ur={ur}"), &co, &ro);
        }
    }
}

/// rows 33–37 — `cache` / `outA` / `outB` / `iterations` NULL, individually
/// and all at once.  The NULL-ed slots must be left untouched (poison intact)
/// and the return value must not change.
#[test]
fn rows33to37_gjk_null_out_params() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 133);

    for i in 0..2_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let ab = shape_bytes(&mut rng, ta);
        let bb = shape_bytes(&mut rng, tb);
        for ur in [0i32, 1] {
            // reference: everything provided
            let full_c = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);

            for mask in 0..8u32 {
                for with_cache in [false, true] {
                    let (oa, ob, oi) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
                    let cache = if with_cache {
                        Some(c2GJKCache::default())
                    } else {
                        None
                    };
                    let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), oa, ob, oi, cache, ur);
                    let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), oa, ob, oi, cache, ur);
                    let ctx = format!("rows33-37 #{i} mask={mask} cache={with_cache} ur={ur}");
                    eq_gjk(&ctx, &co, &ro);

                    // the NULL-ed slots must be untouched in BOTH
                    if !oa {
                        eq_raw(&format!("{ctx} outA NULL untouched (C)"), &co.a, &POISON_V);
                        eq_raw(&format!("{ctx} outA NULL untouched (R)"), &ro.a, &POISON_V);
                    }
                    if !ob {
                        eq_raw(&format!("{ctx} outB NULL untouched (C)"), &co.b, &POISON_V);
                        eq_raw(&format!("{ctx} outB NULL untouched (R)"), &ro.b, &POISON_V);
                    }
                    if !oi {
                        eq_int(&format!("{ctx} iters NULL untouched (C)"), co.iters, POISON_IT);
                        eq_int(&format!("{ctx} iters NULL untouched (R)"), ro.iters, POISON_IT);
                    }
                    if !with_cache {
                        // row 33: cache neither read nor written
                        eq_raw(
                            &format!("{ctx} cache NULL untouched (C)"),
                            &co.cache,
                            &c2GJKCache::default(),
                        );
                        eq_raw(
                            &format!("{ctx} cache NULL untouched (R)"),
                            &ro.cache,
                            &c2GJKCache::default(),
                        );
                    }
                    // row 37: the return value must not depend on which out
                    // params were supplied
                    eq_f32(&format!("{ctx} return independent of out params"), co.dist, full_c.dist);
                }
            }
        }
    }
}

/// rows 38–41 — the cache-validity predicate.
///   * `count == 0`  -> `!!count` false  -> cache not read (row 38)
///   * `count  < 0`  -> `!!count` TRUE   -> negative count copied (row 39)
///   * predicate at lib.c:400 true       -> cached simplex discarded (row 40)
///   * predicate false                   -> warm start accepted (row 41)
#[test]
fn rows38to41_gjk_cache_predicate() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 138);

    for i in 0..1_500 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let ab = shape_bytes(&mut rng, ta);
        let bb = shape_bytes(&mut rng, tb);
        let na = match ta {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let nb = match tb {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };

        for ur in [0i32, 1] {
            // row 38: count == 0
            let z = c2GJKCache::default();
            let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(z), ur);
            let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(z), ur);
            eq_gjk(&format!("row38 count=0 #{i} ur={ur}"), &co, &ro);
            // it must behave exactly like passing no cache at all
            let noc = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, None, ur);
            eq_f32(&format!("row38 count=0 == no cache #{i}"), co.dist, noc.dist);

            // row 39: negative count -- `!!cache->count` is TRUE, but the
            // copy loop never runs, so the simplex count becomes negative and
            // every downstream switch takes its `default:`.
            for &neg in &[-1i32, -2, -100, i32::MIN] {
                let ch = c2GJKCache {
                    metric: rng.any_f32(),
                    count: neg,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: rng.any_f32(),
                };
                let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(ch), ur);
                let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(ch), ur);
                eq_gjk(&format!("row39 count={neg} #{i} ur={ur}"), &co, &ro);
                // documented consequences of the C's own logic
                assert_eq!(
                    co.dist.to_bits(),
                    0.0f32.to_bits(),
                    "row39: C must return +0.0 for count={neg}"
                );
                eq_raw(
                    &format!("row39 outA must be (0,0) count={neg}"),
                    &co.a,
                    &c2v { x: 0.0, y: 0.0 },
                );
                eq_int(&format!("row39 iters must be 0 count={neg}"), co.iters, 0);
                eq_int(
                    &format!("row39 cache.count preserved count={neg}"),
                    co.cache.count,
                    neg,
                );
            }

            // rows 40/41: metric chosen to sit on both sides of
            //   min_metric < max_metric*2.0f  &&  metric < -1.0e8f
            for &m in &[
                0.0f32,
                -1.0e8,
                -1.000_000_1e8,
                -1e9,
                -3.4e38,
                f32::NEG_INFINITY,
                1e8,
                f32::NAN,
            ] {
                let mut ch = c2GJKCache {
                    metric: m,
                    count: 1 + (rng.below(3) as c_int),
                    iA: [0; 3],
                    iB: [0; 3],
                    div: if rng.bool() { 1.0 } else { rng.any_f32() },
                };
                for k in 0..3 {
                    ch.iA[k] = rng.below(na) as c_int;
                    ch.iB[k] = rng.below(nb) as c_int;
                }
                let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(ch), ur);
                let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(ch), ur);
                eq_gjk(&format!("rows40/41 metric={m:?} #{i} ur={ur} in={ch:?}"), &co, &ro);
            }
        }
    }
}

// ===========================================================================
// rows 42–46 — the five ways the GJK loop terminates
// ===========================================================================

/// row 42 — the simplex reaches count 3 (`hit`), so `a = b` and the result is
/// exactly `+0.0` even with `use_radius == 0`.
#[test]
fn row42_gjk_hit_sets_zero() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 142);
    let mut hits = 0usize;
    for i in 0..3_000 {
        // deeply overlapping boxes: the origin is strictly inside the
        // Minkowski difference, so c23 must produce count == 3.
        let a = c2AABB {
            min: c2v { x: -5.0, y: -5.0 },
            max: c2v { x: 5.0, y: 5.0 },
        };
        let b = c2AABB {
            min: c2v {
                x: rng.range(-1.0, 1.0),
                y: rng.range(-1.0, 1.0),
            },
            max: c2v {
                x: rng.range(1.5, 3.0),
                y: rng.range(1.5, 3.0),
            },
        };
        let ab = raw(&a).to_vec();
        let bb = raw(&b).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab, C2_TYPE_AABB, std::ptr::null(), &bb, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab, C2_TYPE_AABB, std::ptr::null(), &bb, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row42 #{i} ur={ur}"), &co, &ro);
            if ur == 0 {
                assert_eq!(
                    co.dist.to_bits(),
                    0.0f32.to_bits(),
                    "row42: overlapping boxes must give +0.0 (hit path)"
                );
                eq_raw(&format!("row42 #{i} hit sets a=b"), &co.a, &co.b);
                if co.cache.count == 3 {
                    hits += 1;
                }
            }
        }
    }
    eprintln!("[coverage] row42 simplex-count-3 exits = {hits}");
    assert!(hits > 100, "row42: the `hit` branch was reached only {hits} times");
}

/// row 43 — `d1 > d0` terminates the loop.  `d0` starts at `FLT_MAX`, so a
/// simplex point whose squared length overflows to `+inf` triggers it on the
/// very first pass (`iter` stays 0).
#[test]
fn row43_gjk_no_progress_break() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 143);
    let mut triggered = 0usize;
    for i in 0..1_000 {
        // circles separated by ~2e30 -> dot(p,p) overflows to +inf > FLT_MAX
        let big = 1.0e30f32 * rng.range(1.0, 3.0);
        let a = c2Circle {
            p: c2v { x: -big, y: 0.0 },
            r: rng.range(0.0, 1.0),
        };
        let b = c2Circle {
            p: c2v { x: big, y: 0.0 },
            r: rng.range(0.0, 1.0),
        };
        let ab = raw(&a).to_vec();
        let bb = raw(&b).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row43 #{i} ur={ur} big={big:?}"), &co, &ro);
            if ur == 0 && co.iters == 0 && co.dist.is_infinite() {
                triggered += 1;
            }
        }
        // and via huge AABBs / capsules too
        let aa = c2AABB {
            min: c2v { x: -f32::MAX, y: -f32::MAX },
            max: c2v { x: -1e30, y: -1e30 },
        };
        let bbx = c2AABB {
            min: c2v { x: 1e30, y: 1e30 },
            max: c2v { x: f32::MAX, y: f32::MAX },
        };
        let ab2 = raw(&aa).to_vec();
        let bb2 = raw(&bbx).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab2, C2_TYPE_AABB, std::ptr::null(), &bb2, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab2, C2_TYPE_AABB, std::ptr::null(), &bb2, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row43 huge-aabb #{i} ur={ur}"), &co, &ro);
        }
    }
    eprintln!("[coverage] row43 `d1 > d0` exits = {triggered}");
    assert!(
        triggered > 100,
        "row43: the `d1 > d0` break was reached only {triggered} times"
    );
}

/// row 44 — `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` terminates the loop.
/// Two coincident circles put the origin exactly on the 1-vertex simplex, so
/// `d == (0,0)` on the very first pass.
#[test]
fn row44_gjk_degenerate_direction_break() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 144);
    let mut triggered = 0usize;
    for i in 0..2_000 {
        let p = rng.v();
        let a = c2Circle {
            p,
            r: rng.range(0.0, 3.0),
        };
        let b = c2Circle {
            p,
            r: rng.range(0.0, 3.0),
        };
        let ab = raw(&a).to_vec();
        let bb = raw(&b).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row44 #{i} ur={ur} p={p:?}"), &co, &ro);
            if ur == 0 {
                // simplex stayed at count 1 with d == (0,0)
                eq_int(&format!("row44 #{i} iters"), co.iters, 0);
                eq_int(&format!("row44 #{i} cache.count"), co.cache.count, 1);
                triggered += 1;
            }
        }
        // and a sub-epsilon (but non-zero) separation, which also satisfies
        // dot(d,d) < FLT_EPSILON^2
        let tiny = FLT_EPSILON * rng.range(0.0, 0.9);
        let b2 = c2Circle {
            p: c2v { x: p.x + tiny, y: p.y },
            r: 1.0,
        };
        let bb2 = raw(&b2).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb2, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb2, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row44 sub-eps #{i} ur={ur} tiny={tiny:?}"), &co, &ro);
        }
    }
    eprintln!("[coverage] row44 epsilon-direction exits = {triggered}");
    assert!(triggered > 100);
}

/// row 45 — duplicate support point.  Two *separated* circles have 1-vertex
/// proxies, so `c2Support` can only ever return index 0, which duplicates the
/// initial simplex entry: the loop breaks with `iter == 0` and `s.count` is
/// NOT incremented (cache.count stays 1).
#[test]
fn row45_gjk_duplicate_support_break() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 145);
    let mut triggered = 0usize;
    for i in 0..3_000 {
        let a = c2Circle {
            p: c2v {
                x: rng.range(-2.0, 2.0),
                y: rng.range(-2.0, 2.0),
            },
            r: rng.range(0.0, 1.0),
        };
        let b = c2Circle {
            p: c2v {
                x: rng.range(20.0, 60.0),
                y: rng.range(-60.0, 60.0),
            },
            r: rng.range(0.0, 1.0),
        };
        let ab = raw(&a).to_vec();
        let bb = raw(&b).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row45 #{i} ur={ur}"), &co, &ro);
            if ur == 0 {
                eq_int(&format!("row45 #{i} iters"), co.iters, 0);
                eq_int(&format!("row45 #{i} count not incremented"), co.cache.count, 1);
                assert!(co.dist > 0.0, "row45: separated circles must have dist > 0");
                triggered += 1;
            }
        }
        // capsule vs capsule far apart also exits via `dup` after a few passes
        let ca = c2Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.range(0.0, 1.0),
        };
        let cb = c2Capsule {
            a: c2v { x: 500.0, y: 500.0 },
            b: c2v { x: 501.0, y: 502.0 },
            r: rng.range(0.0, 1.0),
        };
        let ab2 = raw(&ca).to_vec();
        let bb2 = raw(&cb).to_vec();
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab2, C2_TYPE_CAPSULE, std::ptr::null(), &bb2, C2_TYPE_CAPSULE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab2, C2_TYPE_CAPSULE, std::ptr::null(), &bb2, C2_TYPE_CAPSULE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row45 capsules #{i} ur={ur}"), &co, &ro);
        }
    }
    eprintln!("[coverage] row45 duplicate-support exits = {triggered}");
    assert!(triggered > 100);
}

/// row 46 — the `while (iter < 20)` bound.
///
/// The bound is **not reachable** through this library's public surface: the
/// proxies built by `c2MakeProxy` have at most 4 vertices (AABB), and the
/// combination of the `d1 > d0` monotonicity check and the duplicate-support
/// check terminates GJK long before 20 passes.  A 6.4-million-sample search
/// (`tests/search_iter_cap.rs`, random + bit-level mutation hill-climbing over
/// shapes, transforms and warm caches) found a maximum of **7**.
///
/// This test therefore (a) pins the highest-iteration input that search found
/// and asserts C and Rust agree there, and (b) re-runs a randomised sweep
/// asserting the two libraries report *identical* `iterations` for every input
/// and that neither ever exceeds the other.
#[test]
fn row46_gjk_iteration_bound() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);

    // The search's best case: capsule x AABB, iter == 7.
    let av = [0x3f400080u32, 0x41e80e0a, 0x53205e20, 0x41500108, 0xc0d00000];
    let bv = [0x4cbebc30u32, 0xc1a47102, 0x7f79ea0f, 0x3f000000, 0x60080080];
    let af: Vec<f32> = av.iter().map(|&b| f32::from_bits(b)).collect();
    let bf: Vec<f32> = bv.iter().map(|&b| f32::from_bits(b)).collect();
    let cap = c2Capsule {
        a: c2v { x: af[0], y: af[1] },
        b: c2v { x: af[2], y: af[3] },
        r: af[4],
    };
    let bx = c2AABB {
        min: c2v { x: bf[0], y: bf[1] },
        max: c2v { x: bf[2], y: bf[3] },
    };
    let ab = raw(&cap).to_vec();
    let bb = raw(&bx).to_vec();
    for ur in [0i32, 1] {
        let co = gjk_raw(cf, &ab, C2_TYPE_CAPSULE, std::ptr::null(), &bb, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
        let ro = gjk_raw(rf, &ab, C2_TYPE_CAPSULE, std::ptr::null(), &bb, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
        eq_gjk(&format!("row46 max-iter case ur={ur}"), &co, &ro);
        assert_eq!(co.iters, 7, "row46: pinned case should report iter == 7");
    }

    // Randomised sweep: `iterations` must agree exactly, and must always stay
    // within the documented [0, 20] range.
    let mut rng = Rng::new(SEED ^ 146);
    let mut maxc = -1;
    let mut maxr = -1;
    for i in 0..40_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let ab = shape_bytes(&mut rng, ta);
        let bb = shape_bytes(&mut rng, tb);
        let axv = rng.x();
        let bxv = rng.x();
        let (axp, bxp): (*const c2x, *const c2x) = (&axv, &bxv);
        let ur = if rng.bool() { 1 } else { 0 };
        let co = gjk_raw(cf, &ab, ta, axp, &bb, tb, bxp, true, true, true, Some(c2GJKCache::default()), ur);
        let ro = gjk_raw(rf, &ab, ta, axp, &bb, tb, bxp, true, true, true, Some(c2GJKCache::default()), ur);
        eq_gjk(&format!("row46 sweep #{i}"), &co, &ro);
        assert!(
            (0..=20).contains(&co.iters),
            "row46: C reported iter={} outside [0,20]",
            co.iters
        );
        maxc = maxc.max(co.iters);
        maxr = maxr.max(ro.iters);
    }
    eprintln!("[coverage] row46 max iterations: C={maxc} Rust={maxr} (cap is 20, unreachable)");
    assert_eq!(maxc, maxr, "row46: C and Rust must agree on the max iteration count");
}

// ===========================================================================
// rows 47–53 — the `use_radius` clamp predicate (lib.c:477-493)
// ===========================================================================

/// row 47 — `use_radius == 0`: the raw simplex distance is returned and `a`/`b`
/// are left exactly as `c2Witness` produced them (no radius subtraction).
#[test]
fn row47_gjk_use_radius_zero() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let (clen, _) = fnpair!("c2Len", FnFv);
    let (csub, _) = fnpair!("c2Sub", FnVvv);
    let mut rng = Rng::new(SEED ^ 147);

    for i in 0..3_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let ab = shape_bytes(&mut rng, ta);
        let bb = shape_bytes(&mut rng, tb);
        let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 0);
        let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 0);
        eq_gjk(&format!("row47 #{i} ta={ta} tb={tb}"), &co, &ro);
        // with use_radius == 0 the returned distance IS |a-b| (unless `hit`,
        // which forces a == b and dist == 0)
        let recomputed = clen(csub(co.a, co.b));
        if co.dist != 0.0 {
            eq_f32(&format!("row47 #{i} dist == |a-b|"), co.dist, recomputed);
        }
    }
}

/// rows 48–49 — `use_radius != 0` but `dist <= rA + rB` (row 48) or
/// `dist <= FLT_EPSILON` (row 49): `a = b = 0.5*(a+b)` and the result is
/// exactly `+0.0`.
#[test]
fn rows48to49_gjk_radius_midpoint_branch() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let (cadd, _) = fnpair!("c2Add", FnVvv);
    let (cmulvs, _) = fnpair!("c2Mulvs", FnVvF);
    let mut rng = Rng::new(SEED ^ 148);
    let (mut n48, mut n49) = (0usize, 0usize);

    for i in 0..3_000 {
        // row 48: separated centres but overlapping radii -> dist <= rA+rB
        let gap = rng.range(0.1, 8.0);
        let ra = rng.range(gap * 0.5, gap * 2.0);
        let rb = rng.range(gap * 0.5, gap * 2.0);
        let a = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: ra,
        };
        let b = c2Circle {
            p: c2v { x: gap, y: 0.0 },
            r: rb,
        };
        let ab = raw(&a).to_vec();
        let bb = raw(&b).to_vec();

        // reference run with use_radius == 0 gives the pre-clamp a/b/dist
        let raw0 = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, None, 0);
        let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
        let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
        eq_gjk(&format!("row48 #{i} gap={gap} ra={ra} rb={rb}"), &co, &ro);
        if raw0.dist <= ra + rb {
            n48 += 1;
            assert_eq!(
                co.dist.to_bits(),
                0.0f32.to_bits(),
                "row48: C must return +0.0 when dist <= rA+rB"
            );
            // a = b = 0.5*(a+b) of the *pre-clamp* witness points
            let mid = cmulvs(cadd(raw0.a, raw0.b), 0.5);
            eq_raw(&format!("row48 #{i} outA == midpoint"), &co.a, &mid);
            eq_raw(&format!("row48 #{i} outB == midpoint"), &co.b, &mid);
        }

        // row 49: separation at or below FLT_EPSILON with ZERO radii, so the
        // `dist > rA+rB` test passes (0.0 > 0.0 is false, actually the first
        // conjunct fails) -- use a sub-epsilon gap and zero radii, which is
        // the `dist > FLT_EPSILON` conjunct.
        let tiny = FLT_EPSILON * rng.range(0.0, 1.0);
        let a2 = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        let b2 = c2Circle {
            p: c2v { x: tiny, y: 0.0 },
            r: 0.0,
        };
        let ab2 = raw(&a2).to_vec();
        let bb2 = raw(&b2).to_vec();
        let raw0 = gjk_raw(cf, &ab2, C2_TYPE_CIRCLE, std::ptr::null(), &bb2, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, None, 0);
        let co = gjk_raw(cf, &ab2, C2_TYPE_CIRCLE, std::ptr::null(), &bb2, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
        let ro = gjk_raw(rf, &ab2, C2_TYPE_CIRCLE, std::ptr::null(), &bb2, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
        eq_gjk(&format!("row49 #{i} tiny={tiny:?}"), &co, &ro);
        if raw0.dist <= FLT_EPSILON {
            n49 += 1;
            assert_eq!(
                co.dist.to_bits(),
                0.0f32.to_bits(),
                "row49: C must return +0.0 when dist <= FLT_EPSILON"
            );
            let mid = cmulvs(cadd(raw0.a, raw0.b), 0.5);
            eq_raw(&format!("row49 #{i} outA == midpoint"), &co.a, &mid);
            eq_raw(&format!("row49 #{i} outB == midpoint"), &co.b, &mid);
        }
    }
    eprintln!("[coverage] row48 dist<=rA+rB hits = {n48}, row49 dist<=FLT_EPSILON hits = {n49}");
    assert!(n48 > 100, "row48 reached only {n48} times");
    assert!(n49 > 100, "row49 reached only {n49} times");
}

/// row 50 — the radius-subtraction branch IS taken (`dist > rA+rB` and
/// `dist > FLT_EPSILON`), but afterwards `a.x == b.x && a.y == b.y`, so the C
/// force-zeroes `dist` while KEEPING the shifted `a`/`b`.
///
/// Trigger (verified against the C `.so`): put the two circle centres exactly
/// 1 ulp apart at y = 2.0, with radii `0.625*ulp` and `0.25*ulp`.  Then
/// `dist == ulp > rA+rB == 0.875*ulp`, and rounding makes both shifted points
/// land on `2.0 + ulp`.
#[test]
fn row50_gjk_radius_shift_collapses_to_zero() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut hit = 0usize;

    // sweep the base magnitude and the radius split so the case is not a
    // single lucky constant
    for k in 0..24 {
        let y = 2.0f32 * (1u32 << (k % 12)) as f32;
        let ulp = f32::from_bits(y.to_bits() + 1) - y;
        for &(fa, fb) in &[
            (0.625f32, 0.25f32),
            (0.75, 0.125),
            (0.5625, 0.25),
            (0.875, 0.0625),
            (0.625, 0.125),
            (0.5, 0.25),
        ] {
            let a = c2Circle {
                p: c2v { x: 0.0, y },
                r: ulp * fa,
            };
            let b = c2Circle {
                p: c2v { x: 0.0, y: y + ulp },
                r: ulp * fb,
            };
            let ab = raw(&a).to_vec();
            let bb = raw(&b).to_vec();

            let raw0 = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, None, 0);
            let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
            let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
            eq_gjk(
                &format!("row50 y={y} ulp={ulp:?} fa={fa} fb={fb}"),
                &co,
                &ro,
            );

            let took_radius_branch =
                raw0.dist > a.r + b.r && raw0.dist > FLT_EPSILON;
            if took_radius_branch && co.dist == 0.0 {
                // the C reached line 486/487
                hit += 1;
                assert_eq!(
                    co.dist.to_bits(),
                    0.0f32.to_bits(),
                    "row50: must be exactly +0.0"
                );
                eq_raw(&format!("row50 y={y}: a == b"), &co.a, &co.b);
                // and NOT the midpoint of the pre-clamp points: the branch
                // keeps the *shifted* values
                assert_ne!(
                    (co.a.x.to_bits(), co.a.y.to_bits()),
                    (raw0.a.x.to_bits(), raw0.a.y.to_bits()),
                    "row50: outA should be the shifted point, not the raw witness"
                );
            }
        }
    }
    eprintln!("[coverage] row50 `a==b after radius shift` hits = {hit}");
    assert!(
        hit > 0,
        "row50 was never reached -- the constructed trigger no longer works"
    );
}

/// row 51 — `use_radius` is tested for truthiness, so ANY non-zero int behaves
/// like 1 (including negative values and `INT_MIN`).
#[test]
fn row51_gjk_use_radius_truthiness() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 151);
    let truthy: [c_int; 8] = [1, 2, -1, 7, 255, 65536, i32::MIN, i32::MAX];

    for i in 0..1_500 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let ab = shape_bytes(&mut rng, ta);
        let bb = shape_bytes(&mut rng, tb);

        let one_c = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
        for &ur in truthy.iter() {
            let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
            eq_gjk(&format!("row51 #{i} use_radius={ur}"), &co, &ro);
            // every truthy value must equal use_radius == 1
            eq_f32(&format!("row51 #{i} ur={ur} == ur=1"), co.dist, one_c.dist);
            eq_raw(&format!("row51 #{i} ur={ur} outA == ur=1"), &co.a, &one_c.a);
            eq_raw(&format!("row51 #{i} ur={ur} outB == ur=1"), &co.b, &one_c.b);
        }
        // and 0 is the only falsy value
        let z = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 0);
        let zr = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 0);
        eq_gjk(&format!("row51 #{i} use_radius=0"), &z, &zr);
    }
}

/// row 52 — negative radii make `rA + rB < 0`, so `dist > rA+rB` is satisfied
/// more easily and `dist -= rA+rB` *increases* the distance.
#[test]
fn row52_gjk_negative_radii() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 152);
    let neg: [f32; 7] = [-0.0, -1e-30, -FLT_EPSILON, -0.5, -10.0, -1e18, -f32::MAX];
    let mut grew = 0usize;

    for i in 0..1_000 {
        let p = c2v {
            x: rng.range(-3.0, 3.0),
            y: rng.range(-3.0, 3.0),
        };
        let q = c2v {
            x: rng.range(5.0, 20.0),
            y: rng.range(-3.0, 3.0),
        };
        for &ra in neg.iter() {
            for &rb in neg.iter() {
                let a = c2Circle { p, r: ra };
                let b = c2Circle { p: q, r: rb };
                let ab = raw(&a).to_vec();
                let bb = raw(&b).to_vec();
                let base = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, None, 0);
                let co = gjk_raw(cf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
                let ro = gjk_raw(rf, &ab, C2_TYPE_CIRCLE, std::ptr::null(), &bb, C2_TYPE_CIRCLE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
                eq_gjk(&format!("row52 #{i} ra={ra:?} rb={rb:?}"), &co, &ro);
                if co.dist > base.dist {
                    grew += 1;
                }
                // capsules too (negative capsule radius)
                let ca = c2Capsule { a: p, b: q, r: ra };
                let cb = c2Capsule {
                    a: c2v { x: q.x + 30.0, y: q.y },
                    b: c2v { x: q.x + 40.0, y: q.y },
                    r: rb,
                };
                let ab2 = raw(&ca).to_vec();
                let bb2 = raw(&cb).to_vec();
                let co = gjk_raw(cf, &ab2, C2_TYPE_CAPSULE, std::ptr::null(), &bb2, C2_TYPE_CAPSULE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
                let ro = gjk_raw(rf, &ab2, C2_TYPE_CAPSULE, std::ptr::null(), &bb2, C2_TYPE_CAPSULE, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), 1);
                eq_gjk(&format!("row52 caps #{i} ra={ra:?} rb={rb:?}"), &co, &ro);
            }
        }
    }
    eprintln!("[coverage] row52 negative-radius distance growth = {grew}");
    assert!(grew > 100, "row52: dist never increased ({grew})");
}

/// row 53 — `NaN` anywhere in a shape makes every `<`/`>` predicate false, so
/// GJK takes the midpoint branch and returns `+0.0`.
#[test]
fn row53_gjk_nan_shapes() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 153);
    let nans: [f32; 4] = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7F80_0001),
    ];

    for i in 0..600 {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                let mut ab = shape_bytes(&mut rng, ta);
                let mut bb = shape_bytes(&mut rng, tb);
                let nfa = ab.len() / 4;
                let nfb = bb.len() / 4;
                // poke a NaN into a random float slot of A and/or B
                let nan = nans[rng.below(4) as usize];
                match rng.below(3) {
                    0 => {
                        let s = rng.below(nfa as u32) as usize;
                        ab[s * 4..s * 4 + 4].copy_from_slice(&nan.to_bits().to_le_bytes());
                    }
                    1 => {
                        let s = rng.below(nfb as u32) as usize;
                        bb[s * 4..s * 4 + 4].copy_from_slice(&nan.to_bits().to_le_bytes());
                    }
                    _ => {
                        let s = rng.below(nfa as u32) as usize;
                        ab[s * 4..s * 4 + 4].copy_from_slice(&nan.to_bits().to_le_bytes());
                        let s = rng.below(nfb as u32) as usize;
                        bb[s * 4..s * 4 + 4].copy_from_slice(&nan.to_bits().to_le_bytes());
                    }
                }
                for ur in [0i32, 1] {
                    let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
                    let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(c2GJKCache::default()), ur);
                    eq_gjk(
                        &format!("row53 #{i} ta={ta} tb={tb} ur={ur} A={ab:02x?} B={bb:02x?}"),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// rows 54–68 — the boolean shape predicates on degenerate / invalid input
// ===========================================================================

/// rows 54–55 — `c2AABBtoAABB` has no validation at all: inverted boxes are
/// evaluated as written, and a `NaN` component makes all four `<` false so the
/// function reports a hit (`!(0|0|0|0) == 1`).
#[test]
fn rows54to55_c2AABBtoAABB_inverted_and_nan() {
    let (c, r) = fnpair!("c2AABBtoAABB", FnAABBtoAABB);
    let mut rng = Rng::new(SEED ^ 154);

    // row 54: inverted (min > max) boxes, i.e. the empty set
    for i in 0..5_000 {
        let p = rng.v();
        let q = rng.v();
        let inv_a = c2AABB {
            min: c2v {
                x: p.x.max(q.x),
                y: p.y.max(q.y),
            },
            max: c2v {
                x: p.x.min(q.x),
                y: p.y.min(q.y),
            },
        };
        let inv_b = c2AABB {
            min: rng.v(),
            max: rng.v(),
        };
        for (a, b) in [(inv_a, inv_b), (inv_b, inv_a), (inv_a, inv_a)] {
            eq_int(&format!("row54 #{i} A={a:?} B={b:?}"), c(a, b), r(a, b));
        }
    }

    // row 55: NaN in ANY of the 8 slots -> all four `<` are false -> 1
    for &nan in &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7F80_0001),
    ] {
        // NaN in all slots: definitely returns 1
        let n = c2AABB {
            min: c2v { x: nan, y: nan },
            max: c2v { x: nan, y: nan },
        };
        let cv = c(n, n);
        eq_int("row55 all-NaN", cv, r(n, n));
        assert_eq!(cv, 1, "row55: all-NaN AABBs must report a hit");

        // NaN in one slot at a time, with otherwise clearly-separated boxes
        for slot in 0..8 {
            let mut a = c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            let mut b = c2AABB {
                min: c2v { x: 10.0, y: 10.0 },
                max: c2v { x: 11.0, y: 11.0 },
            };
            let f: &mut f32 = match slot {
                0 => &mut a.min.x,
                1 => &mut a.min.y,
                2 => &mut a.max.x,
                3 => &mut a.max.y,
                4 => &mut b.min.x,
                5 => &mut b.min.y,
                6 => &mut b.max.x,
                _ => &mut b.max.y,
            };
            *f = nan;
            eq_int(&format!("row55 slot={slot} nan"), c(a, b), r(a, b));
        }
    }
}

/// rows 56–57 — `c2AABBtoCapsule` / `c2CapsuletoCapsule` test the `float`
/// result of `c2GJK` with `!= 0`.  This test pins the exact sign/zero
/// behaviour: `+0.0` and `-0.0` both compare equal to 0 (=> return 1) while any
/// `NaN` compares unequal (=> return 0).
#[test]
fn rows56to57_float_ne_zero_test() {
    let (cac, rac) = fnpair!("c2AABBtoCapsule", FnAABBtoCapsule);
    let (ccc, rcc) = fnpair!("c2CapsuletoCapsule", FnCapsuletoCapsule);
    let (cgjk, _) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(SEED ^ 156);

    for i in 0..5_000 {
        let bb = rng.aabb();
        let ca = rng.capsule();
        let cb = rng.capsule();

        // row 56
        let cv = cac(bb, ca);
        eq_int(&format!("row56 #{i} A={bb:?} B={ca:?}"), cv, rac(bb, ca));
        // cross-check against the underlying c2GJK result
        let d = gjk_raw(cgjk, raw(&bb), C2_TYPE_AABB, std::ptr::null(), raw(&ca), C2_TYPE_CAPSULE, std::ptr::null(), false, false, false, None, 1).dist;
        assert_eq!(
            cv,
            if d != 0.0 { 0 } else { 1 },
            "row56: `c2GJK(..) != 0` mismatch, dist bits = 0x{:08x}",
            d.to_bits()
        );

        // row 57
        let cv = ccc(ca, cb);
        eq_int(&format!("row57 #{i} A={ca:?} B={cb:?}"), cv, rcc(ca, cb));
        let d = gjk_raw(cgjk, raw(&ca), C2_TYPE_CAPSULE, std::ptr::null(), raw(&cb), C2_TYPE_CAPSULE, std::ptr::null(), false, false, false, None, 1).dist;
        assert_eq!(
            cv,
            if d != 0.0 { 0 } else { 1 },
            "row57: `c2GJK(..) != 0` mismatch, dist bits = 0x{:08x}",
            d.to_bits()
        );
    }

    // degenerate / special-value inputs for both
    for &s in SPECIALS.iter() {
        for slot in 0..9 {
            let mut bb = c2AABB {
                min: c2v { x: -1.0, y: -1.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            let mut ca = c2Capsule {
                a: c2v { x: -2.0, y: 0.0 },
                b: c2v { x: 2.0, y: 0.0 },
                r: 0.5,
            };
            match slot {
                0 => bb.min.x = s,
                1 => bb.min.y = s,
                2 => bb.max.x = s,
                3 => bb.max.y = s,
                4 => ca.a.x = s,
                5 => ca.a.y = s,
                6 => ca.b.x = s,
                7 => ca.b.y = s,
                _ => ca.r = s,
            }
            eq_int(
                &format!("row56 special slot={slot} s={s:?}"),
                cac(bb, ca),
                rac(bb, ca),
            );
            eq_int(
                &format!("row57 special slot={slot} s={s:?}"),
                ccc(ca, ca),
                rcc(ca, ca),
            );
        }
    }
}

/// rows 58–59 — `c2CircletoCircle` with negative radii (`(rA+rB)^2` is still
/// >= 0, so a "collision" can be reported) and with `NaN`.
#[test]
fn rows58to59_c2CircletoCircle_negative_and_nan() {
    let (c, r) = fnpair!("c2CircletoCircle", FnCircletoCircle);
    let mut rng = Rng::new(SEED ^ 158);
    let mut neg_hits = 0usize;

    // row 58
    for i in 0..5_000 {
        let d = rng.range(0.0, 10.0);
        let ra = -rng.range(0.0, 10.0);
        let rb = -rng.range(0.0, 10.0);
        let a = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: ra,
        };
        let b = c2Circle {
            p: c2v { x: d, y: 0.0 },
            r: rb,
        };
        let cv = c(a, b);
        eq_int(&format!("row58 #{i} d={d} ra={ra} rb={rb}"), cv, r(a, b));
        if cv != 0 {
            neg_hits += 1;
        }
    }
    eprintln!("[coverage] row58 negative-radius hits = {neg_hits}");
    assert!(
        neg_hits > 10,
        "row58: negative radii never reported a hit ({neg_hits})"
    );

    // row 59: NaN -> `d2 < r2` is false -> 0
    for &nan in &[f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_1234)] {
        for slot in 0..6 {
            let mut a = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            };
            let mut b = c2Circle {
                p: c2v { x: 0.5, y: 0.0 },
                r: 1.0,
            };
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.p.x = nan,
                4 => b.p.y = nan,
                _ => b.r = nan,
            }
            let cv = c(a, b);
            eq_int(&format!("row59 slot={slot}"), cv, r(a, b));
            assert_eq!(cv, 0, "row59: NaN must make `d2 < r2` false");
        }
    }
}

/// rows 60–62 — `c2CircletoAABB`: inverted box (no validation), `NaN` through
/// the `c2Maxv`/`c2Minv` selects, and negative radius.
#[test]
fn rows60to62_c2CircletoAABB_degenerate() {
    let (c, r) = fnpair!("c2CircletoAABB", FnCircletoAABB);
    let (cclamp, _) = fnpair!("c2Clampv", FnVvvv);
    let mut rng = Rng::new(SEED ^ 160);

    // row 60: inverted box -> c2Clampv(a, lo, hi) with lo > hi returns lo
    for i in 0..5_000 {
        let lo = rng.v();
        let hi = rng.v();
        let inv = c2AABB {
            min: c2v {
                x: lo.x.max(hi.x),
                y: lo.y.max(hi.y),
            },
            max: c2v {
                x: lo.x.min(hi.x),
                y: lo.y.min(hi.y),
            },
        };
        let circ = rng.circle();
        eq_int(&format!("row60 #{i} A={circ:?} B={inv:?}"), c(circ, inv), r(circ, inv));
        // and the documented clamp result
        let clamped = cclamp(circ.p, inv.min, inv.max);
        if inv.min.x > inv.max.x {
            eq_f32(&format!("row60 #{i} clamp.x == lo.x"), clamped.x, inv.min.x);
        }
        if inv.min.y > inv.max.y {
            eq_f32(&format!("row60 #{i} clamp.y == lo.y"), clamped.y, inv.min.y);
        }
    }

    // row 61: NaN through the `>`/`<` ternary selects
    for &nan in &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7F80_0001),
    ] {
        for slot in 0..7 {
            let mut a = c2Circle {
                p: c2v { x: 0.5, y: 0.5 },
                r: 1.0,
            };
            let mut b = c2AABB {
                min: c2v { x: -1.0, y: -1.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.min.x = nan,
                4 => b.min.y = nan,
                5 => b.max.x = nan,
                _ => b.max.y = nan,
            }
            eq_int(&format!("row61 slot={slot} nan"), c(a, b), r(a, b));
        }
    }

    // row 62: negative radius -> r2 = r*r > 0 -> a hit is possible
    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let mut neg_hits = 0usize;
    for i in 0..2_000 {
        let rr = -rng.range(0.0, 6.0);
        let p = c2v {
            x: rng.range(-4.0, 4.0),
            y: rng.range(-4.0, 4.0),
        };
        let a = c2Circle { p, r: rr };
        let cv = c(a, unit);
        eq_int(&format!("row62 #{i} r={rr}"), cv, r(a, unit));
        if cv != 0 {
            neg_hits += 1;
        }
    }
    eprintln!("[coverage] row62 negative-radius hits = {neg_hits}");
    assert!(neg_hits > 10, "row62: never reported a hit ({neg_hits})");
}

/// rows 63–68 — `c2CircletoCapsule`: all three `da`/`db` branches, the
/// degenerate capsule (which does NOT reach the `/c2Dot(n,n)` division), a
/// forced division by zero, and NaN.
#[test]
fn rows63to68_c2CircletoCapsule_branches() {
    let (c, r) = fnpair!("c2CircletoCapsule", FnCircletoCapsule);
    let (dot, _) = fnpair!("c2Dot", FnFvv);
    let (sub, _) = fnpair!("c2Sub", FnVvv);
    let mut rng = Rng::new(SEED ^ 163);
    let mut branch = [0usize; 3];

    let classify = |a: &c2Circle, b: &c2Capsule| -> usize {
        let n = sub(b.b, b.a);
        let ap = sub(a.p, b.a);
        let da = dot(ap, n);
        if da < 0.0 {
            0
        } else if dot(sub(a.p, b.b), n) < 0.0 {
            1
        } else {
            2
        }
    };

    // rows 63–65: the three branches, hit deliberately
    let cap = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 4.0, y: 0.0 },
        r: 0.5,
    };
    for i in 0..5_000 {
        let p = c2v {
            x: rng.range(-6.0, 12.0),
            y: rng.range(-4.0, 4.0),
        };
        let a = c2Circle {
            p,
            r: rng.range(0.0, 2.0),
        };
        branch[classify(&a, &cap)] += 1;
        eq_int(&format!("rows63-65 #{i} A={a:?}"), c(a, cap), r(a, cap));
    }
    // explicit boundary points: da == 0 exactly, db == 0 exactly
    for a in [
        c2Circle {
            p: c2v { x: -1.0, y: 0.0 },
            r: 0.25,
        }, // da < 0
        c2Circle {
            p: c2v { x: 0.0, y: 3.0 },
            r: 0.25,
        }, // da == 0 -> else
        c2Circle {
            p: c2v { x: 2.0, y: 3.0 },
            r: 0.25,
        }, // db < 0
        c2Circle {
            p: c2v { x: 4.0, y: 3.0 },
            r: 0.25,
        }, // db == 0 -> bp
        c2Circle {
            p: c2v { x: 9.0, y: 0.0 },
            r: 0.25,
        }, // db > 0
    ] {
        branch[classify(&a, &cap)] += 1;
        eq_int(&format!("rows63-65 boundary A={a:?}"), c(a, cap), r(a, cap));
    }
    eprintln!("[coverage] rows63-65 da/db branches = {branch:?}");
    assert!(
        branch.iter().all(|&x| x > 10),
        "rows63-65: not all branches reached: {branch:?}"
    );

    // row 66: degenerate capsule (a == b) -> n == (0,0), da == 0 (NOT < 0),
    // db == 0 (NOT < 0) -> the `bp` branch, so /c2Dot(n,n) is NOT reached and
    // there is no division by zero.
    for i in 0..2_000 {
        let p = rng.v();
        let deg = c2Capsule {
            a: p,
            b: p,
            r: rng.radius(),
        };
        let circ = rng.circle();
        assert_eq!(
            classify(&circ, &deg),
            2,
            "row66: degenerate capsule must take the `bp` branch"
        );
        let cv = c(circ, deg);
        eq_int(&format!("row66 #{i} A={circ:?} B={deg:?}"), cv, r(circ, deg));
        // the result must equal the equivalent circle-vs-circle test
        // (d2 = |A.p - B.b|^2 vs (A.r + B.r)^2)
        let (cc, _) = fnpair!("c2CircletoCircle", FnCircletoCircle);
        let equiv = cc(circ, c2Circle { p, r: deg.r });
        eq_int(&format!("row66 #{i} == circle-circle"), cv, equiv);
    }

    // row 67: force the `/c2Dot(n,n)` division with c2Dot(n,n) == 0 while
    // taking the `db < 0` branch.  `n` must be non-zero-but-underflowing so
    // that dot(n,n) rounds to 0 yet `da >= 0` and `db < 0`.
    let mut div_zero_seen = 0usize;
    for k in 1..200u32 {
        let e = f32::from_bits(k); // subnormal
        let cap2 = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: e, y: 0.0 },
            r: 0.5,
        };
        for &cx in &[0.0f32, e * 0.5, e, -e, 1.0, -1.0] {
            let a = c2Circle {
                p: c2v { x: cx, y: 1.0 },
                r: 0.25,
            };
            let n = sub(cap2.b, cap2.a);
            if dot(n, n) == 0.0 && classify(&a, &cap2) == 1 {
                div_zero_seen += 1;
            }
            eq_int(
                &format!("row67 k={k} cx={cx:?} B={cap2:?}"),
                c(a, cap2),
                r(a, cap2),
            );
        }
    }
    // also inf-based: n = (inf,0) -> dot(n,n) = inf, da/inf = 0 or NaN
    for &big in &[f32::MAX, 1e30f32, f32::INFINITY] {
        let cap3 = c2Capsule {
            a: c2v { x: -big, y: 0.0 },
            b: c2v { x: big, y: 0.0 },
            r: 0.5,
        };
        for p in [
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 1.0, y: -1.0 },
            c2v { x: big, y: 1.0 },
        ] {
            let a = c2Circle { p, r: 0.25 };
            eq_int(
                &format!("row67 inf big={big:?} p={p:?}"),
                c(a, cap3),
                r(a, cap3),
            );
        }
    }
    eprintln!("[coverage] row67 dot(n,n)==0 with db<0 occurrences = {div_zero_seen}");

    // row 68: NaN -> both `< 0` tests false -> `bp` branch -> `d2 < r*r` false
    for &nan in &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7F80_0001),
    ] {
        for slot in 0..8 {
            let mut a = c2Circle {
                p: c2v { x: 1.0, y: 0.0 },
                r: 1.0,
            };
            let mut b = cap;
            match slot {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.a.x = nan,
                4 => b.a.y = nan,
                5 => b.b.x = nan,
                6 => b.b.y = nan,
                _ => b.r = nan,
            }
            let cv = c(a, b);
            eq_int(&format!("row68 slot={slot}"), cv, r(a, b));
            assert_eq!(cv, 0, "row68: NaN must yield 0 (slot {slot})");
        }
    }
}

// ===========================================================================
// rows 69–84 — the simplex solvers' branch predicates and the unvalidated
//              vector helpers
// ===========================================================================

fn mk_simplex(pts: &[c2v], div: f32) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div,
        count: pts.len() as c_int,
    };
    for (i, p) in pts.iter().enumerate() {
        s.verts[i].p = *p;
        s.verts[i].sA = c2v {
            x: 10.0 + i as f32,
            y: 20.0 + i as f32,
        };
        s.verts[i].sB = c2v {
            x: -30.0 - i as f32,
            y: -40.0 - i as f32,
        };
        s.verts[i].u = 100.0 + i as f32;
        s.verts[i].iA = i as c_int;
        s.verts[i].iB = (3 - i) as c_int;
    }
    s
}

/// rows 69–72 — `c22`'s three branches plus the NaN fall-through.
#[test]
fn rows69to72_c22_branch_predicates() {
    let (c, r) = fnpair!("c22", FnSimplexVoid);
    let (dot, _) = fnpair!("c2Dot", FnFvv);
    let (sub, _) = fnpair!("c2Sub", FnVvv);
    let branch_of = |a: c2v, b: c2v| -> usize {
        let u = dot(b, sub(b, a));
        let v = dot(a, sub(a, b));
        if v <= 0.0 {
            0
        } else if u <= 0.0 {
            1
        } else {
            2
        }
    };

    // row 69: v <= 0  (incl. v == 0 and v == -0.0)
    // row 70: v > 0 && u <= 0
    // row 71: u > 0 && v > 0
    let cases: [(c2v, c2v, usize); 6] = [
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }, 0),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, 0), // v == +0
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, 0), // a == b -> v == 0
        (c2v { x: -2.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }, 1),
        (c2v { x: 0.0, y: -3.0 }, c2v { x: 0.0, y: -1.0 }, 1),
        (c2v { x: -1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, 2),
    ];
    for (k, &(a, b, want)) in cases.iter().enumerate() {
        assert_eq!(
            branch_of(a, b),
            want,
            "rows69-71 case {k}: expected branch {want}"
        );
        for &div in &[1.0f32, 0.0, -1.0, f32::NAN] {
            let s = mk_simplex(&[a, b], div);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            eq_raw(&format!("rows69-71 case {k} branch={want} div={div:?}"), &cs, &rs);
            // documented structural consequence
            match want {
                0 | 1 => {
                    eq_int(&format!("case {k} count"), cs.count, 1);
                    eq_f32(&format!("case {k} div"), cs.div, 1.0);
                    eq_f32(&format!("case {k} a.u"), cs.verts[0].u, 1.0);
                    if want == 1 {
                        // s->a = s->b happened: the whole vertex was copied
                        eq_int(&format!("case {k} iA copied"), cs.verts[0].iA, s.verts[1].iA);
                        eq_int(&format!("case {k} iB copied"), cs.verts[0].iB, s.verts[1].iB);
                        eq_raw(&format!("case {k} sA copied"), &cs.verts[0].sA, &s.verts[1].sA);
                    }
                }
                _ => eq_int(&format!("case {k} count"), cs.count, 2),
            }
        }
    }

    // row 72: NaN -> both `<= 0` false -> the `else` with div = NaN
    for &nan in &[f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_1234)] {
        for slot in 0..4 {
            let mut a = c2v { x: 1.0, y: 2.0 };
            let mut b = c2v { x: -3.0, y: 4.0 };
            match slot {
                0 => a.x = nan,
                1 => a.y = nan,
                2 => b.x = nan,
                _ => b.y = nan,
            }
            let s = mk_simplex(&[a, b], 1.0);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            eq_raw(&format!("row72 slot={slot}"), &cs, &rs);
            eq_int(&format!("row72 slot={slot} count==2"), cs.count, 2);
            assert!(cs.div.is_nan(), "row72: div should be NaN, got {}", cs.div);
        }
    }
}

/// rows 73–81 — `c23`'s seven branches, the degenerate (zero-area) triangle,
/// and the NaN fall-through.
#[test]
fn rows73to81_c23_branch_predicates() {
    let (c, r) = fnpair!("c23", FnSimplexVoid);
    let (dot, _) = fnpair!("c2Dot", FnFvv);
    let (sub, _) = fnpair!("c2Sub", FnVvv);
    let (det2, _) = fnpair!("c2Det2", FnFvv);

    let branch_of = |a: c2v, b: c2v, cc: c2v| -> usize {
        let uAB = dot(b, sub(b, a));
        let vAB = dot(a, sub(a, b));
        let uBC = dot(cc, sub(cc, b));
        let vBC = dot(b, sub(b, cc));
        let uCA = dot(a, sub(a, cc));
        let vCA = dot(cc, sub(cc, a));
        let area = det2(sub(b, a), sub(cc, a));
        let uABC = det2(b, cc) * area;
        let vABC = det2(cc, a) * area;
        let wABC = det2(a, b) * area;
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
    };

    // Deterministically find one triangle per branch by sweeping a lattice.
    let mut found: [Option<[c2v; 3]>; 7] = [None; 7];
    let mut rng = Rng::new(SEED ^ 173);
    for _ in 0..400_000 {
        if found.iter().all(|f| f.is_some()) {
            break;
        }
        let mk = |rng: &mut Rng| -> c2v {
            if rng.bool() {
                c2v {
                    x: (rng.below(13) as f32) - 6.0,
                    y: (rng.below(13) as f32) - 6.0,
                }
            } else {
                let th = rng.range(0.0, 6.283_185_5);
                let rr = rng.range(0.5, 6.0);
                c2v {
                    x: rr * th.cos(),
                    y: rr * th.sin(),
                }
            }
        };
        let t = [mk(&mut rng), mk(&mut rng), mk(&mut rng)];
        let br = branch_of(t[0], t[1], t[2]);
        if found[br].is_none() {
            found[br] = Some(t);
        }
    }
    for (br, f) in found.iter().enumerate() {
        let t = f.unwrap_or_else(|| panic!("rows73-81: no triangle found for branch {br}"));
        for &div in &[1.0f32, 0.0, -2.5, f32::NAN] {
            let s = mk_simplex(&t, div);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            eq_raw(&format!("rows73-81 branch={br} t={t:?} div={div:?}"), &cs, &rs);
            // structural consequences documented in ERRORS.md
            let want_count = match br {
                0 | 1 | 2 => 1,
                3 | 4 | 5 => 2,
                _ => 3,
            };
            eq_int(&format!("branch={br} count"), cs.count, want_count);
            match br {
                1 => eq_raw(&format!("branch=1 a=b"), &cs.verts[0].sA, &s.verts[1].sA),
                2 => eq_raw(&format!("branch=2 a=c"), &cs.verts[0].sA, &s.verts[2].sA),
                4 => {
                    eq_raw(&format!("branch=4 a=b"), &cs.verts[0].sA, &s.verts[1].sA);
                    eq_raw(&format!("branch=4 b=c"), &cs.verts[1].sA, &s.verts[2].sA);
                }
                5 => {
                    eq_raw(&format!("branch=5 b=a"), &cs.verts[1].sA, &s.verts[0].sA);
                    eq_raw(&format!("branch=5 a=c"), &cs.verts[0].sA, &s.verts[2].sA);
                }
                _ => {}
            }
        }
    }
    eprintln!("[coverage] rows73-81 all 7 c23 branches constructed");

    // row 80: degenerate triangle a == b == c -> area == 0, all u/v == 0, so
    // branch 1 (`0 <= 0 && 0 <= 0`) wins and count becomes 1.
    for k in 0..64 {
        let p = c2v {
            x: (k as f32) * 0.5 - 16.0,
            y: (k as f32) * -0.25 + 3.0,
        };
        let s = mk_simplex(&[p, p, p], 1.0);
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        eq_raw(&format!("row80 k={k} p={p:?}"), &cs, &rs);
        eq_int(&format!("row80 k={k} count==1"), cs.count, 1);
    }
    // collinear (zero area but distinct points)
    for k in 1..64 {
        let d = c2v {
            x: (k as f32) * 0.25,
            y: (k as f32) * -0.5,
        };
        let t = [
            c2v { x: 0.0, y: 0.0 },
            d,
            c2v {
                x: d.x * 3.0,
                y: d.y * 3.0,
            },
        ];
        let s = mk_simplex(&t, 1.0);
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        eq_raw(&format!("row80 collinear k={k}"), &cs, &rs);
    }

    // row 81: NaN -> every comparison false -> the fall-through `else`
    for &nan in &[f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_1234)] {
        for slot in 0..6 {
            let mut t = [
                c2v { x: 1.0, y: 0.0 },
                c2v { x: -1.0, y: 1.0 },
                c2v { x: -1.0, y: -1.0 },
            ];
            match slot {
                0 => t[0].x = nan,
                1 => t[0].y = nan,
                2 => t[1].x = nan,
                3 => t[1].y = nan,
                4 => t[2].x = nan,
                _ => t[2].y = nan,
            }
            let s = mk_simplex(&t, 1.0);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            eq_raw(&format!("row81 slot={slot}"), &cs, &rs);
            eq_int(&format!("row81 slot={slot} count==3"), cs.count, 3);
            assert!(cs.div.is_nan(), "row81: div should be NaN");
        }
    }
}

/// rows 82–83 — `c2Maxv`/`c2Minv` are `>`/`<` ternary selects (so a `NaN`
/// operand is silently dropped in an operand-order-dependent way), and
/// `c2Clampv` never validates `lo <= hi`.
#[test]
fn rows82to83_maxv_minv_clampv_unvalidated() {
    let (cmax, rmax) = fnpair!("c2Maxv", FnVvv);
    let (cmin, rmin) = fnpair!("c2Minv", FnVvv);
    let (cclamp, rclamp) = fnpair!("c2Clampv", FnVvvv);
    let mut rng = Rng::new(SEED ^ 182);

    // row 82: every oddball x oddball pair, both orders
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            let b = c2v {
                x: f32::from_bits(q),
                y: f32::from_bits(p),
            };
            eq_raw(&format!("row82 max {a:?} {b:?}"), &cmax(a, b), &rmax(a, b));
            eq_raw(&format!("row82 max {b:?} {a:?}"), &cmax(b, a), &rmax(b, a));
            eq_raw(&format!("row82 min {a:?} {b:?}"), &cmin(a, b), &rmin(a, b));
            eq_raw(&format!("row82 min {b:?} {a:?}"), &cmin(b, a), &rmin(b, a));
        }
    }
    // ±0.0 pairs: `a > b` is false for (+0,-0), so the ternary picks b
    for x in [0.0f32, -0.0] {
        for y in [0.0f32, -0.0] {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            eq_raw(&format!("row82 zeros max {a:?} {b:?}"), &cmax(a, b), &rmax(a, b));
            eq_raw(&format!("row82 zeros min {a:?} {b:?}"), &cmin(a, b), &rmin(a, b));
        }
    }

    // row 83: lo > hi -> c2Maxv(lo, c2Minv(a,hi)) == lo
    for i in 0..5_000 {
        let a = rng.any_v();
        let lo = rng.any_v();
        let hi = rng.any_v();
        let cv = cclamp(a, lo, hi);
        eq_raw(&format!("row83 #{i} a={a:?} lo={lo:?} hi={hi:?}"), &cv, &rclamp(a, lo, hi));
        // for a well-defined inverted range the C must return lo
        if lo.x > hi.x && lo.x.is_finite() && hi.x.is_finite() && a.x.is_finite() {
            eq_f32(&format!("row83 #{i} x == lo.x"), cv.x, lo.x);
        }
        if lo.y > hi.y && lo.y.is_finite() && hi.y.is_finite() && a.y.is_finite() {
            eq_f32(&format!("row83 #{i} y == lo.y"), cv.y, lo.y);
        }
    }
}

/// row 84 — `c2BBVerts` performs no validation whatsoever: inverted and NaN
/// boxes have their 4 corners written verbatim.
#[test]
fn row84_c2BBVerts_unvalidated() {
    let (c, r) = fnpair!("c2BBVerts", FnBBVerts);
    let mut rng = Rng::new(SEED ^ 184);
    let fill = c2v {
        x: f32::from_bits(0xA5A5_A5A5),
        y: f32::from_bits(0x5A5A_5A5A),
    };

    let go = |bb: c2AABB, ctx: String| {
        let mut co = [fill; 8];
        let mut ro = [fill; 8];
        let (mut cb, mut rb) = (bb, bb);
        unsafe {
            c(co.as_mut_ptr(), &mut cb);
            r(ro.as_mut_ptr(), &mut rb);
        }
        eq_raw(&format!("row84 {ctx}"), &co, &ro);
        // the documented corner order, verbatim
        eq_raw(&format!("row84 {ctx} v0"), &co[0], &bb.min);
        eq_raw(
            &format!("row84 {ctx} v1"),
            &co[1],
            &c2v { x: bb.max.x, y: bb.min.y },
        );
        eq_raw(&format!("row84 {ctx} v2"), &co[2], &bb.max);
        eq_raw(
            &format!("row84 {ctx} v3"),
            &co[3],
            &c2v { x: bb.min.x, y: bb.max.y },
        );
        // slots 4..8 untouched
        for k in 4..8 {
            eq_raw(&format!("row84 {ctx} slot{k} untouched"), &co[k], &fill);
        }
    };

    for i in 0..3_000 {
        // inverted
        let p = rng.any_v();
        let q = rng.any_v();
        go(c2AABB { min: p, max: q }, format!("#{i} raw"));
        go(c2AABB { min: q, max: p }, format!("#{i} swapped"));
        go(c2AABB { min: p, max: p }, format!("#{i} degenerate"));
    }
    for &s in SPECIALS.iter() {
        go(
            c2AABB {
                min: c2v { x: s, y: s },
                max: c2v { x: s, y: s },
            },
            format!("special {s:?}"),
        );
    }
}

// ===========================================================================
// row 85 — `ptr_from_parts` never checks the `malloc` result
// ===========================================================================

/// `malloc` failure cannot be forced portably from a test, but the *success*
/// path is verified byte-for-byte (see `tier5_public.rs` row 72), and this test
/// documents that both implementations dereference the result unconditionally
/// and would fault identically on OOM.
#[test]
fn row85_ptr_from_parts_unchecked_malloc() {
    let (c, r) = fnpair!("ptr_from_parts", FnPtrFromParts);
    // A large number of calls, all of which must succeed and agree, proving the
    // allocation + initialisation path is identical.
    let mut rng = Rng::new(SEED ^ 185);
    let mut allocs = Vec::new();
    for _ in 0..5_000 {
        let ty = ALL_TYPES[rng.below(3) as usize];
        let f = [
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
        ];
        let size = match ty {
            C2_TYPE_CIRCLE => 12,
            C2_TYPE_AABB => 16,
            _ => 20,
        };
        unsafe {
            let cp = c(ty, f[0], f[1], f[2], f[3], f[4]);
            let rp = r(ty, f[0], f[1], f[2], f[3], f[4]);
            assert!(!cp.is_null() && !rp.is_null());
            assert_eq!(
                std::slice::from_raw_parts(cp as *const u8, size),
                std::slice::from_raw_parts(rp as *const u8, size),
                "row85: ty={ty} parts={f:?}"
            );
            allocs.push(cp);
            allocs.push(rp);
        }
    }
    extern "C" {
        fn free(p: *mut c_void);
    }
    for p in allocs {
        unsafe { free(p) };
    }
}

// ===========================================================================
// rows 86–94 — UNDEFINED BEHAVIOUR: documented, deliberately not asserted
// ===========================================================================
//
// row 86  `ptr_from_parts` with an invalid `typ` falls off the end of a
//         non-`void` function.  gcc emits no store to `%rax`, so the caller
//         observes a leftover register value that changes with the preceding
//         call; the Rust translation returns NULL.  The only consumer,
//         `omni_collide`, never dereferences it because `c2Collided` filters
//         invalid enums first -- which is exactly what
//         `rows05to08_omni_collide_invalid_enums` proves.
//
// row 87  `c2GJK` with an out-of-range `typeA`/`typeB`: `c2MakeProxy` writes
//         nothing, so `c2Proxy pA;` (an uninitialised stack local at lib.c:371)
//         is read at lib.c:407.  gcc's and rustc's stack frames differ, so a
//         byte comparison would be asserting on garbage.
//
// row 88  `c2GJK` / `c2Collided` with `A` or `B` NULL: unconditional NULL
//         dereference; both libraries fault identically (SIGSEGV).  The
//         pointers the C actually *does* check (`ax_ptr`, `bx_ptr`, `outA`,
//         `outB`, `iterations`, `cache`) are covered by rows 31-37.
//
// row 89  `cache->count > 3` indexes `cache->iA[i]`/`iB[i]` and
//         `saveA[i]`/`saveB[i]` past their 3-element bounds.  In C this reads
//         adjacent struct/stack members whose layout is compiler-specific.
//         `rows38to41_gjk_cache_predicate` covers the in-range counts 1..3 and
//         the negative-count case, which ARE well defined.
//
// row 90  `cache->iA[i]` / `iB[i]` outside `0..proxy_vert_count(type)`:
//         `pA.verts[iA]` (lib.c:384) then reads the uninitialised tail of
//         `c2Proxy::verts[8]`.  This was found empirically -- an early version
//         of `tier3_gjk.rs::row59_handcrafted_cache` used indices in `0..4` for
//         a 2-vertex capsule proxy and diverged, because gcc leaves the tail as
//         stack garbage while rustc zeroes it.  The test now constrains indices
//         to the initialised range.
//
// rows 91-93  NULL `c2Simplex*` / `c2v*` / `c2AABB*` / `c2Proxy*`: every one of
//         these is dereferenced unconditionally, so both libraries fault.
//
// row 94  A valid `typeA`/`typeB` with `A`/`B` pointing at a shape of the wrong
//         type (e.g. a 12-byte `c2Circle` read as a 20-byte `c2Capsule`) reads
//         past the object.  Both libraries do the identical bad read, but what
//         they read is whatever the allocator left there.
//
// The single check that IS meaningful for this group: the well-defined subset of
// each is already exercised above, and the following test asserts that the one
// UB row reachable from the *public* API (row 86) is neutralised by the C's own
// enum filtering, so `omni_collide` remains fully deterministic.

#[test]
fn row86_ptr_from_parts_ub_is_neutralised_by_c2Collided() {
    let (com, rom) = fnpair!("omni_collide", FnOmniCollide);
    let mut rng = Rng::new(SEED ^ 186);
    // Call omni_collide with an invalid type many times, interleaved with
    // *valid* calls, so that whatever stale register/stack state the C's
    // fall-off-the-end path picks up varies between calls.  The result must be
    // a stable 0 in both libraries regardless.
    for i in 0..20_000 {
        let bad = BAD_TYPES[rng.below(BAD_TYPES.len() as u32) as usize];
        let good = ALL_TYPES[rng.below(3) as usize];
        let a = random_parts(&mut rng, good);
        let b = random_parts(&mut rng, good);
        unsafe {
            // a valid call first, to leave non-trivial register/stack state
            let _ = com(good, a[0], a[1], a[2], a[3], a[4], good, b[0], b[1], b[2], b[3], b[4]);
            let _ = rom(good, a[0], a[1], a[2], a[3], a[4], good, b[0], b[1], b[2], b[3], b[4]);
            for (ta, tb) in [(bad, good), (good, bad), (bad, bad)] {
                let cv = com(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
                let rv = rom(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
                eq_int(&format!("row86 #{i} ta={ta} tb={tb}"), cv, rv);
                assert_eq!(cv, 0, "row86: must be a stable 0");
            }
        }
    }
}

/// rows 40/41 (refined) — the SECOND conjunct of the cache predicate at
/// lib.c:400, `metric < -1.0e8f`.
///
/// `metric` is *recomputed* from the cached simplex, so it cannot be set
/// directly: it is `c2Det2(b.p-a.p, c.p-a.p)` for `count == 3`, i.e. an area,
/// which only reaches the 1e8 magnitude for shapes ~1e4 units across.  A
/// mutation test showed that without such shapes the `-1.0e8f` constant is
/// unobservable, so this test deliberately sweeps the scale until `metric`
/// lands on BOTH sides of the threshold, and asserts all four combinations of
/// the two conjuncts are exercised.
#[test]
fn rows40to41_cache_predicate_both_conjuncts() {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);
    let (mkproxy, _) = fnpair!("c2MakeProxy", FnMakeProxy);
    let (mulxv, _) = fnpair!("c2Mulxv", FnVxv);
    let (sub, _) = fnpair!("c2Sub", FnVvv);
    let (metricf, _) = fnpair!("c2GJKSimplexMetric", FnSimplexF);
    let (xid, _) = fnpair!("c2xIdentity", FnX);
    let ident = xid();

    // Replicate lib.c:378-401 exactly, using the C library's own primitives,
    // so we know which side of each conjunct a given input falls on.
    let recomputed_metric = |ab: &[u8], ta: C2_TYPE, bb: &[u8], tb: C2_TYPE, ch: &c2GJKCache| -> f32 {
        let mut pa = c2Proxy::default();
        let mut pb = c2Proxy::default();
        unsafe {
            mkproxy(ab.as_ptr() as *const c_void, ta, &mut pa);
            mkproxy(bb.as_ptr() as *const c_void, tb, &mut pb);
        }
        let mut s = c2Simplex {
            verts: [c2sv::default(); 4],
            div: ch.div,
            count: ch.count,
        };
        for i in 0..(ch.count.clamp(0, 3) as usize) {
            let sa = mulxv(ident, pa.verts[ch.iA[i] as usize]);
            let sb = mulxv(ident, pb.verts[ch.iB[i] as usize]);
            s.verts[i].iA = ch.iA[i];
            s.verts[i].iB = ch.iB[i];
            s.verts[i].sA = sa;
            s.verts[i].sB = sb;
            s.verts[i].p = sub(sb, sa);
            s.verts[i].u = 0.0;
        }
        unsafe { metricf(&mut s) }
    };

    // quadrant counters: [conj1][conj2]
    let mut quad = [[0usize; 2]; 2];
    let mut rng = Rng::new(SEED ^ 401);

    // Shapes large enough that the count-3 determinant spans +-1e8, swept over
    // several decades so `metric` crosses -1.0e8 many times.
    let scales: [f32; 12] = [
        1.0, 10.0, 100.0, 3_000.0, 7_000.0, 9_000.0, 10_000.0, 10_500.0, 11_000.0, 14_000.0,
        30_000.0, 1.0e6,
    ];
    // index triples that give both determinant signs for a 4-vertex AABB proxy
    let triples: [[c_int; 3]; 8] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 2, 3],
        [1, 3, 2],
        [0, 1, 3],
        [0, 3, 1],
        [2, 3, 0],
        [3, 2, 0],
    ];
    let metric_olds: [f32; 9] = [
        0.0,
        -1.0,
        1.0,
        -1.0e8,
        -1.0e9,
        1.0e9,
        f32::NEG_INFINITY,
        f32::INFINITY,
        -3.4e38,
    ];

    for &sc in scales.iter() {
        for &tri_a in triples.iter() {
            for &tri_b in triples.iter() {
                let a = c2AABB {
                    min: c2v { x: -sc, y: -sc },
                    max: c2v { x: sc, y: sc },
                };
                let b = c2AABB {
                    min: c2v {
                        x: -sc * 0.5,
                        y: -sc * 0.75,
                    },
                    max: c2v {
                        x: sc * 1.25,
                        y: sc * 0.5,
                    },
                };
                let ab = raw(&a).to_vec();
                let bb = raw(&b).to_vec();
                for &mo in metric_olds.iter() {
                    let ch = c2GJKCache {
                        metric: mo,
                        count: 3,
                        iA: tri_a,
                        iB: tri_b,
                        div: if rng.bool() { 1.0 } else { 3.0 },
                    };
                    let metric = recomputed_metric(&ab, C2_TYPE_AABB, &bb, C2_TYPE_AABB, &ch);
                    let min_m = if metric < mo { metric } else { mo };
                    let max_m = if metric > mo { metric } else { mo };
                    let c1 = min_m < max_m * 2.0f32;
                    let c2 = metric < -1.0e8f32;
                    quad[c1 as usize][c2 as usize] += 1;

                    for ur in [0i32, 1] {
                        let co = gjk_raw(cf, &ab, C2_TYPE_AABB, std::ptr::null(), &bb, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(ch), ur);
                        let ro = gjk_raw(rf, &ab, C2_TYPE_AABB, std::ptr::null(), &bb, C2_TYPE_AABB, std::ptr::null(), true, true, true, Some(ch), ur);
                        eq_gjk(
                            &format!(
                                "rows40/41 sc={sc} tri_a={tri_a:?} tri_b={tri_b:?} mo={mo:?} metric={metric:?} c1={c1} c2={c2} ur={ur}"
                            ),
                            &co,
                            &ro,
                        );
                    }
                }
            }
        }
    }

    // capsules and circles too (count 2 / count 1 metrics), plus transforms
    for k in 0..600 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let sc = [1.0f32, 100.0, 1e4, 1e5, 1e6][rng.below(5) as usize];
        let mk = |rng: &mut Rng, ty: C2_TYPE| -> Vec<u8> {
            match ty {
                C2_TYPE_CIRCLE => raw(&c2Circle {
                    p: c2v {
                        x: rng.range(-sc, sc),
                        y: rng.range(-sc, sc),
                    },
                    r: rng.range(0.0, sc),
                })
                .to_vec(),
                C2_TYPE_AABB => raw(&c2AABB {
                    min: c2v {
                        x: rng.range(-sc, 0.0),
                        y: rng.range(-sc, 0.0),
                    },
                    max: c2v {
                        x: rng.range(0.0, sc),
                        y: rng.range(0.0, sc),
                    },
                })
                .to_vec(),
                _ => raw(&c2Capsule {
                    a: c2v {
                        x: rng.range(-sc, sc),
                        y: rng.range(-sc, sc),
                    },
                    b: c2v {
                        x: rng.range(-sc, sc),
                        y: rng.range(-sc, sc),
                    },
                    r: rng.range(0.0, sc),
                })
                .to_vec(),
            }
        };
        let ab = mk(&mut rng, ta);
        let bb = mk(&mut rng, tb);
        let na = match ta {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let nb = match tb {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let mut ch = c2GJKCache {
            metric: metric_olds[rng.below(9) as usize],
            count: 1 + rng.below(3) as c_int,
            iA: [0; 3],
            iB: [0; 3],
            div: if rng.bool() { 1.0 } else { rng.range(-1e3, 1e3) },
        };
        for i in 0..3 {
            ch.iA[i] = rng.below(na) as c_int;
            ch.iB[i] = rng.below(nb) as c_int;
        }
        let metric = recomputed_metric(&ab, ta, &bb, tb, &ch);
        let mo = ch.metric;
        let min_m = if metric < mo { metric } else { mo };
        let max_m = if metric > mo { metric } else { mo };
        quad[(min_m < max_m * 2.0f32) as usize][(metric < -1.0e8f32) as usize] += 1;
        for ur in [0i32, 1] {
            let co = gjk_raw(cf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(ch), ur);
            let ro = gjk_raw(rf, &ab, ta, std::ptr::null(), &bb, tb, std::ptr::null(), true, true, true, Some(ch), ur);
            eq_gjk(&format!("rows40/41 mixed #{k} ta={ta} tb={tb} metric={metric:?} in={ch:?} ur={ur}"), &co, &ro);
        }
    }

    eprintln!(
        "[coverage] cache predicate quadrants [min<max*2][metric<-1e8] = {:?}",
        quad
    );
    for c1 in 0..2 {
        for c2 in 0..2 {
            assert!(
                quad[c1][c2] > 0,
                "cache predicate quadrant c1={c1} c2={c2} never reached: {quad:?}"
            );
        }
    }
}
