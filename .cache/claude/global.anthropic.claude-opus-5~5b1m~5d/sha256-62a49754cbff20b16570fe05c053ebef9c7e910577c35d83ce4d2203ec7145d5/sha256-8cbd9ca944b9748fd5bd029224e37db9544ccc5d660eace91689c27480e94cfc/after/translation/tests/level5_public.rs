//! Phase B, level 5 — `c2Collided`, `ptr_from_parts`, `omni_collide`.
//! CONFIGS.md rows 53..=60.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 5000;

/// Rows 53..=55 — `c2Collided` for every `typeA` with all three `typeB`s.
/// This is what exercises the *argument-swapping* calls the C makes
/// (`AABB x CIRCLE` -> `c2CircletoAABB(B, A)`, etc.).
fn collided_row(row: &str, tA: c_int, seed: u64) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCollided>("c2Collided") };
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for &tB in ALL_TYPES.iter() {
        let mut yes = 0;
        let mut no = 0;
        for _ in 0..N {
            // small scale -> plenty of collisions; occasionally wide -> misses
            let scale = if rng.boolean() { 2.0 } else { 20.0 };
            let A = shape_bytes(&mut rng, tA, scale);
            let B = shape_bytes(&mut rng, tB, scale);
            let (cv, rv) = unsafe {
                (
                    c(A.as_ptr() as *const c_void, tA, B.as_ptr() as *const c_void, tB),
                    r(A.as_ptr() as *const c_void, tA, B.as_ptr() as *const c_void, tB),
                )
            };
            d.check((ty_name(tA), ty_name(tB), &A[..], &B[..]), cv, rv);
            if cv != 0 { yes += 1 } else { no += 1 }
        }
        assert!(
            yes > 0 && no > 0,
            "{row} / {}: one-sided results yes={yes} no={no}",
            ty_name(tB)
        );
    }
    d.finish();
}

#[test]
fn row53_collided_typeA_circle() {
    collided_row("53: c2Collided typeA=CIRCLE", C2_TYPE_CIRCLE, 0x5001);
}

#[test]
fn row54_collided_typeA_aabb() {
    collided_row("54: c2Collided typeA=AABB (swapped args)", C2_TYPE_AABB, 0x5002);
}

#[test]
fn row55_collided_typeA_capsule() {
    collided_row("55: c2Collided typeA=CAPSULE (swapped args)", C2_TYPE_CAPSULE, 0x5003);
}

/// Row 56 — `ptr_from_parts`: the heap struct it builds is read back and
/// compared byte-for-byte between the two libraries.
#[test]
fn row56_ptr_from_parts() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnPtrFromParts>("ptr_from_parts") };
    let mut rng = Rng::new(0x5004);
    let mut d = Diff::new("56: ptr_from_parts CIRCLE|AABB|CAPSULE");
    for &ty in ALL_TYPES.iter() {
        let nbytes = match ty {
            C2_TYPE_CIRCLE => 12usize,
            C2_TYPE_AABB => 16,
            _ => 20,
        };
        for _ in 0..N {
            let v: [f32; 5] = [
                mixed(&mut rng, 100.0),
                mixed(&mut rng, 100.0),
                mixed(&mut rng, 100.0),
                mixed(&mut rng, 100.0),
                mixed(&mut rng, 100.0),
            ];
            unsafe {
                let cp = c(ty, v[0], v[1], v[2], v[3], v[4]);
                let rp = r(ty, v[0], v[1], v[2], v[3], v[4]);
                assert!(!cp.is_null(), "C ptr_from_parts returned NULL for {ty}");
                assert!(!rp.is_null(), "Rust ptr_from_parts returned NULL for {ty}");
                // Compare the constructed shape as floats (bit-exact, NaN-aware).
                let cf = std::slice::from_raw_parts(cp as *const f32, nbytes / 4);
                let rf = std::slice::from_raw_parts(rp as *const f32, nbytes / 4);
                for k in 0..nbytes / 4 {
                    d.check((ty_name(ty), k, v), cf[k], rf[k]);
                }
                libc_free(cp);
                libc_free(rp);
            }
        }
    }
    d.finish();
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

/// Drive `omni_collide` over all 9 type pairs with a caller-supplied parameter
/// generator.
fn omni_row(row: &str, seed: u64, gen: impl Fn(&mut Rng) -> [f32; 5], require_mix: bool) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnOmniCollide>("omni_collide") };
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            let mut yes = 0;
            let mut no = 0;
            for _ in 0..N / 2 {
                let a = gen(&mut rng);
                let b = gen(&mut rng);
                let (cv, rv) = unsafe {
                    (
                        c(tA, a[0], a[1], a[2], a[3], a[4], tB, b[0], b[1], b[2], b[3], b[4]),
                        r(tA, a[0], a[1], a[2], a[3], a[4], tB, b[0], b[1], b[2], b[3], b[4]),
                    )
                };
                d.check((ty_name(tA), ty_name(tB), a, b), cv, rv);
                if cv != 0 { yes += 1 } else { no += 1 }
            }
            if require_mix {
                assert!(
                    yes > 0 && no > 0,
                    "{row} / {}x{}: one-sided results yes={yes} no={no}",
                    ty_name(tA),
                    ty_name(tB)
                );
            }
        }
    }
    d.finish();
}

#[test]
fn row57_omni_collide_mostly_colliding() {
    omni_row(
        "57: omni_collide all 9 pairs, tight range (mostly colliding)",
        0x5005,
        |rng| {
            [
                rng.sym(2.0),
                rng.sym(2.0),
                rng.sym(2.0),
                rng.sym(2.0),
                rng.unit() * 2.0,
            ]
        },
        true,
    );
}

#[test]
fn row58_omni_collide_mostly_disjoint() {
    omni_row(
        "58: omni_collide all 9 pairs, wide range (mostly disjoint)",
        0x5006,
        |rng| {
            [
                rng.sym(500.0),
                rng.sym(500.0),
                rng.sym(500.0),
                rng.sym(500.0),
                rng.unit() * 5.0,
            ]
        },
        false,
    );
}

#[test]
fn row59_omni_collide_integer_grid() {
    omni_row(
        "59: omni_collide all 9 pairs, small integer grid (many exact ties)",
        0x5007,
        |rng| [rng.grid(), rng.grid(), rng.grid(), rng.grid(), rng.grid()],
        true,
    );
}

#[test]
fn row60_omni_collide_specials() {
    omni_row(
        "60: omni_collide all 9 pairs, NaN / inf / +-0 / FLT_MAX / denormals",
        0x5008,
        |rng| {
            let pick = |rng: &mut Rng| {
                if rng.below(3) == 0 {
                    rng.sym(4.0)
                } else {
                    SPECIALS[rng.below(SPECIALS.len())]
                }
            };
            [pick(rng), pick(rng), pick(rng), pick(rng), pick(rng)]
        },
        false,
    );
}

/// Extra: exhaustively sweep `omni_collide` over the whole SPECIALS table in the
/// first two slots for every type pair (a deterministic, non-random sweep).
#[test]
fn row60b_omni_collide_specials_sweep() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnOmniCollide>("omni_collide") };
    let mut d = Diff::new("60b: omni_collide exhaustive SPECIALS sweep");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for i in 0..SPECIALS.len() {
                for j in 0..SPECIALS.len() {
                    let a = [SPECIALS[i], SPECIALS[j], SPECIALS[j], SPECIALS[i], SPECIALS[i]];
                    let b = [SPECIALS[j], SPECIALS[i], SPECIALS[i], SPECIALS[j], SPECIALS[j]];
                    let (cv, rv) = unsafe {
                        (
                            c(tA, a[0], a[1], a[2], a[3], a[4], tB, b[0], b[1], b[2], b[3], b[4]),
                            r(tA, a[0], a[1], a[2], a[3], a[4], tB, b[0], b[1], b[2], b[3], b[4]),
                        )
                    };
                    d.check((ty_name(tA), ty_name(tB), a, b), cv, rv);
                }
            }
        }
    }
    d.finish();
}
