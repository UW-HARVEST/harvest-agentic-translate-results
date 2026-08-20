//! Phase B, `CONFIGS.md` rows 72–77: the public surface declared in
//! `c_src/include/lib.h` — `omni_collide` — plus `ptr_from_parts`.
//!
//! `ptr_from_parts` `malloc`s, so the test reads the allocated struct back
//! through the returned pointer and compares its raw bytes; this verifies the
//! Rust wrapper writes the same 12 / 16 / 20 bytes in the same layout.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::c_int;

const N: usize = 30_000;

#[derive(Default)]
struct Tally {
    yes: usize,
    no: usize,
}

impl Tally {
    fn note(&mut self, v: c_int) {
        if v != 0 {
            self.yes += 1
        } else {
            self.no += 1
        }
    }
    #[track_caller]
    fn require_both(&self, name: &str) {
        eprintln!("[coverage] {name}: hit={} miss={}", self.yes, self.no);
        assert!(
            self.yes > 5 && self.no > 5,
            "{name} never produced both outcomes: {} / {}",
            self.yes,
            self.no
        );
    }
}

// ---------------------------------------------------------------------------
// row 72 — ptr_from_parts for each of the 3 valid types
// ---------------------------------------------------------------------------

#[test]
fn row72_ptr_from_parts_valid_types() {
    let (c, r) = fnpair!("ptr_from_parts", FnPtrFromParts);
    let mut rng = Rng::new(SEED ^ 72);

    for i in 0..N {
        for &ty in ALL_TYPES.iter() {
            let f: [f32; 5] = [
                rng.any_f32(),
                rng.any_f32(),
                rng.any_f32(),
                rng.any_f32(),
                rng.any_f32(),
            ];
            let size = match ty {
                C2_TYPE_CIRCLE => std::mem::size_of::<c2Circle>(),
                C2_TYPE_AABB => std::mem::size_of::<c2AABB>(),
                _ => std::mem::size_of::<c2Capsule>(),
            };
            unsafe {
                let cp = c(ty, f[0], f[1], f[2], f[3], f[4]);
                let rp = r(ty, f[0], f[1], f[2], f[3], f[4]);
                assert!(!cp.is_null(), "C ptr_from_parts returned NULL for ty={ty}");
                assert!(
                    !rp.is_null(),
                    "Rust ptr_from_parts returned NULL for ty={ty}"
                );
                let cb = std::slice::from_raw_parts(cp as *const u8, size);
                let rb = std::slice::from_raw_parts(rp as *const u8, size);
                assert_eq!(
                    cb, rb,
                    "DIVERGENCE ptr_from_parts #{i} ty={ty} parts={f:?}\n  C = {cb:02x?}\n  R = {rb:02x?}"
                );
                // and the bytes must be exactly the struct the C source builds
                match ty {
                    C2_TYPE_CIRCLE => {
                        let want = c2Circle {
                            p: c2v { x: f[0], y: f[1] },
                            r: f[2],
                        };
                        assert_eq!(cb, raw(&want), "C circle layout mismatch");
                    }
                    C2_TYPE_AABB => {
                        let want = c2AABB {
                            min: c2v { x: f[0], y: f[1] },
                            max: c2v { x: f[2], y: f[3] },
                        };
                        assert_eq!(cb, raw(&want), "C aabb layout mismatch");
                    }
                    _ => {
                        let want = c2Capsule {
                            a: c2v { x: f[0], y: f[1] },
                            b: c2v { x: f[2], y: f[3] },
                            r: f[4],
                        };
                        assert_eq!(cb, raw(&want), "C capsule layout mismatch");
                    }
                }
                libc_free(cp);
                libc_free(rp);
            }
        }
    }
}

/// `free` from libc, so the test does not leak the `malloc`ed shapes.
unsafe fn libc_free(p: *mut std::os::raw::c_void) {
    extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    free(p);
}

// ---------------------------------------------------------------------------
// rows 73–74 — omni_collide over all 9 pairs
// ---------------------------------------------------------------------------

fn omni_case(
    cf: FnOmniCollide,
    rf: FnOmniCollide,
    ta: C2_TYPE,
    a: [f32; 5],
    tb: C2_TYPE,
    b: [f32; 5],
    ctx: &str,
) -> c_int {
    unsafe {
        let cv = cf(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
        let rv = rf(ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
        eq_int(
            &format!("omni_collide {ctx} ta={ta} a={a:?} tb={tb} b={b:?}"),
            cv,
            rv,
        );
        cv
    }
}

#[test]
fn row73_omni_collide_wide_random() {
    let (cf, rf) = fnpair!("omni_collide", FnOmniCollide);
    let mut rng = Rng::new(SEED ^ 73);
    for i in 0..N {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                let a = random_parts(&mut rng, ta);
                let b = random_parts(&mut rng, tb);
                omni_case(cf, rf, ta, a, tb, b, &format!("wide #{i}"));
            }
        }
    }
}

#[test]
fn row74_omni_collide_clustered() {
    let (cf, rf) = fnpair!("omni_collide", FnOmniCollide);
    let mut rng = Rng::new(SEED ^ 74);
    let mut tallies: Vec<Tally> = (0..9).map(|_| Tally::default()).collect();
    for i in 0..N {
        for (ia, &ta) in ALL_TYPES.iter().enumerate() {
            for (ib, &tb) in ALL_TYPES.iter().enumerate() {
                // tight cluster around the origin -> collisions are likely
                let mk = |rng: &mut Rng, ty: C2_TYPE| -> [f32; 5] {
                    let x = rng.range(-2.0, 2.0);
                    let y = rng.range(-2.0, 2.0);
                    match ty {
                        C2_TYPE_CIRCLE => [x, y, rng.range(0.0, 1.5), 0.0, 0.0],
                        C2_TYPE_AABB => [
                            x - rng.range(0.0, 1.5),
                            y - rng.range(0.0, 1.5),
                            x + rng.range(0.0, 1.5),
                            y + rng.range(0.0, 1.5),
                            0.0,
                        ],
                        _ => [
                            x,
                            y,
                            x + rng.range(-2.0, 2.0),
                            y + rng.range(-2.0, 2.0),
                            rng.range(0.0, 1.0),
                        ],
                    }
                };
                let a = mk(&mut rng, ta);
                let b = mk(&mut rng, tb);
                let v = omni_case(cf, rf, ta, a, tb, b, &format!("cluster #{i}"));
                tallies[ia * 3 + ib].note(v);
            }
        }
    }
    for (ia, &ta) in ALL_TYPES.iter().enumerate() {
        for (ib, &tb) in ALL_TYPES.iter().enumerate() {
            tallies[ia * 3 + ib].require_both(&format!("omni_collide {ta}x{tb}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 75 — special float values in every field slot
// ---------------------------------------------------------------------------

#[test]
fn row75_omni_collide_special_values() {
    let (cf, rf) = fnpair!("omni_collide", FnOmniCollide);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            // baseline: two overlapping unit-ish shapes
            let base = |ty: C2_TYPE| -> [f32; 5] {
                match ty {
                    C2_TYPE_CIRCLE => [0.0, 0.0, 1.0, 0.0, 0.0],
                    C2_TYPE_AABB => [-1.0, -1.0, 1.0, 1.0, 0.0],
                    _ => [-1.0, 0.0, 1.0, 0.0, 0.5],
                }
            };
            for &s in SPECIALS.iter() {
                for slot in 0..10 {
                    let mut a = base(ta);
                    let mut b = base(tb);
                    if slot < 5 {
                        a[slot] = s;
                    } else {
                        b[slot - 5] = s;
                    }
                    omni_case(cf, rf, ta, a, tb, b, &format!("special slot={slot} s={s:?}"));
                }
            }
            // oddball bit patterns in every slot
            for &o in ODDBALLS.iter() {
                let v = f32::from_bits(o);
                for slot in 0..10 {
                    let mut a = base(ta);
                    let mut b = base(tb);
                    if slot < 5 {
                        a[slot] = v;
                    } else {
                        b[slot - 5] = v;
                    }
                    omni_case(cf, rf, ta, a, tb, b, &format!("odd slot={slot} bits=0x{o:08x}"));
                }
            }
            // all-special: every field set to the same special value
            for &s in SPECIALS.iter() {
                omni_case(cf, rf, ta, [s; 5], tb, [s; 5], &format!("all-special {s:?}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 76 — small integer grid sweep (exact touching / boundary cases)
// ---------------------------------------------------------------------------

#[test]
fn row76_omni_collide_integer_grid() {
    let (cf, rf) = fnpair!("omni_collide", FnOmniCollide);
    // A is fixed at the origin; B sweeps a grid of half-integer offsets.
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for gx in -8i32..=8 {
                for gy in -8i32..=8 {
                    let x = gx as f32 * 0.5;
                    let y = gy as f32 * 0.5;
                    for &rr in &[0.0f32, 0.5, 1.0, 2.0] {
                        let a = match ta {
                            C2_TYPE_CIRCLE => [0.0, 0.0, rr, 0.0, 0.0],
                            C2_TYPE_AABB => [-1.0, -1.0, 1.0, 1.0, 0.0],
                            _ => [-1.0, 0.0, 1.0, 0.0, rr],
                        };
                        let b = match tb {
                            C2_TYPE_CIRCLE => [x, y, rr, 0.0, 0.0],
                            C2_TYPE_AABB => [x - 1.0, y - 1.0, x + 1.0, y + 1.0, 0.0],
                            _ => [x - 1.0, y, x + 1.0, y, rr],
                        };
                        omni_case(
                            cf,
                            rf,
                            ta,
                            a,
                            tb,
                            b,
                            &format!("grid gx={gx} gy={gy} rr={rr}"),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 77 — negative radii
// ---------------------------------------------------------------------------

#[test]
fn row77_omni_collide_negative_radii() {
    let (cf, rf) = fnpair!("omni_collide", FnOmniCollide);
    let mut rng = Rng::new(SEED ^ 77);
    let radii = [
        -0.0f32, -1e-30, -FLT_EPSILON, -0.5, -1.0, -100.0, -1e18, -f32::MAX,
    ];
    for i in 0..(N / 10) {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                for &ra in radii.iter() {
                    for &rb in radii.iter() {
                        let x = rng.range(-3.0, 3.0);
                        let y = rng.range(-3.0, 3.0);
                        let a = match ta {
                            C2_TYPE_CIRCLE => [0.0, 0.0, ra, 0.0, 0.0],
                            C2_TYPE_AABB => [-1.0, -1.0, 1.0, 1.0, 0.0],
                            _ => [-1.0, 0.0, 1.0, 0.0, ra],
                        };
                        let b = match tb {
                            C2_TYPE_CIRCLE => [x, y, rb, 0.0, 0.0],
                            C2_TYPE_AABB => [x - 1.0, y - 1.0, x + 1.0, y + 1.0, 0.0],
                            _ => [x - 1.0, y, x + 1.0, y, rb],
                        };
                        omni_case(
                            cf,
                            rf,
                            ta,
                            a,
                            tb,
                            b,
                            &format!("negrad #{i} ra={ra:?} rb={rb:?}"),
                        );
                    }
                }
            }
        }
    }
}
