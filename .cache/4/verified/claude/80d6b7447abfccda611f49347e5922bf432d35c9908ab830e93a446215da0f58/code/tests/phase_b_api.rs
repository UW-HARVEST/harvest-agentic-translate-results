//! Phase B, CONFIGS.md rows 73-79: the public dispatcher and the two convenience
//! entry points from `c_src/include/lib.h`.
//!
//! `zero_stack()` is called before every call because the `(AABB, CAPSULE)` and
//! `(CAPSULE, AABB)` pairs reach `c2GJK` with `C2_TYPE_POLY`, where the C library
//! reads an uninitialised `c2Proxy` (see `tests/probe_uninit.rs`). It is a no-op for the other pairs.
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 3_000;

// ---------------------------------------------------------------------------
// Rows 73-74: c2Collide over all 9 valid ordered type pairs
// ---------------------------------------------------------------------------

/// Storage large enough for any of the three shapes, so `c2Collide` gets a
/// `const void *` exactly as a C caller would pass it.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
struct Blob([u8; 24]);

fn blob_circle(c: c2Circle) -> Blob {
    let mut b = Blob([0; 24]);
    unsafe { std::ptr::copy_nonoverlapping(&c as *const c2Circle as *const u8, b.0.as_mut_ptr(), 12) };
    b
}
fn blob_aabb(a: c2AABB) -> Blob {
    let mut b = Blob([0; 24]);
    unsafe { std::ptr::copy_nonoverlapping(&a as *const c2AABB as *const u8, b.0.as_mut_ptr(), 16) };
    b
}
fn blob_capsule(c: c2Capsule) -> Blob {
    let mut b = Blob([0; 24]);
    unsafe { std::ptr::copy_nonoverlapping(&c as *const c2Capsule as *const u8, b.0.as_mut_ptr(), 20) };
    b
}

fn rand_blob(rng: &mut Rng, ty: C2_TYPE, lattice: bool, special: bool) -> Blob {
    let vv = |r: &mut Rng| {
        if special {
            r.vec_special()
        } else if lattice {
            v(r.f_half_lattice(3), r.f_half_lattice(3))
        } else {
            r.vec_norm(5.0)
        }
    };
    let rr = |r: &mut Rng| {
        if special {
            r.f_special()
        } else if lattice {
            (r.below(4) as f32) * 0.5
        } else {
            r.f_pos(3.0)
        }
    };
    match ty {
        C2_TYPE_CIRCLE => {
            let p = vv(rng);
            blob_circle(c2Circle { p, r: rr(rng) })
        }
        C2_TYPE_AABB => {
            let min = vv(rng);
            let max = if special { vv(rng) } else { v(min.x + rr(rng) * 2.0, min.y + rr(rng) * 2.0) };
            blob_aabb(c2AABB { min, max })
        }
        _ => {
            let a = vv(rng);
            let b = vv(rng);
            blob_capsule(c2Capsule { a, b, r: rr(rng) })
        }
    }
}

fn collide(
    f: &libloading::Symbol<'_, FnCollide>,
    a: &Blob,
    ta: C2_TYPE,
    b: &Blob,
    tb: C2_TYPE,
    seed: u8,
) -> c2Manifold {
    let mut m = poison_manifold(seed);
    zero_stack();
    unsafe {
        f(
            a as *const Blob as *const c_void,
            ta,
            b as *const Blob as *const c_void,
            tb,
            &mut m,
        );
    }
    m
}

fn warm_collide(cf: &libloading::Symbol<'_, FnCollide>, rf: &libloading::Symbol<'_, FnCollide>) {
    let a = blob_aabb(c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) });
    let b = blob_capsule(c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 });
    warmup(|| {
        for &(ta, tb) in [(C2_TYPE_AABB, C2_TYPE_CAPSULE), (C2_TYPE_CAPSULE, C2_TYPE_AABB)].iter() {
            let _ = collide(cf, &a, ta, &b, tb, 1);
            let _ = collide(rf, &a, ta, &b, tb, 1);
        }
    });
}

#[test]
fn row73_74_collide_all_valid_pairs() {
    let l = libs();
    let (cf, rf) = l.get::<FnCollide>("c2Collide");
    warm_collide(&cf, &rf);
    let mut rng = Rng::new(73);
    let mut contacts = std::collections::BTreeMap::new();
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for mode in 0..3u32 {
                let (lattice, special) = match mode {
                    0 => (false, false), // row 73
                    1 => (true, false),  // row 74
                    _ => (false, true),
                };
                for i in 0..N {
                    let a = rand_blob(&mut rng, ta, lattice, special);
                    let b = rand_blob(&mut rng, tb, lattice, special);
                    for seed in [23u8, 199] {
                        let cm = collide(&cf, &a, ta, &b, tb, seed);
                        let rm = collide(&rf, &a, ta, &b, tb, seed);
                        eq(
                            "c2Collide",
                            &format!("ta={ta} tb={tb} mode={mode} i={i} seed={seed}"),
                            &cm,
                            &rm,
                        );
                        *contacts.entry((ta, tb, cm.count)).or_insert(0u32) += 1;
                    }
                }
            }
        }
    }
    // every ordered valid pair must have produced at least one contact
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            let any = contacts.keys().any(|&(x, y, c)| x == ta && y == tb && c > 0);
            assert!(any, "pair ({ta}, {tb}) never produced a contact");
        }
    }
    println!("row73/74 (ta,tb,count) histogram size: {}", contacts.len());
}

// ---------------------------------------------------------------------------
// Rows 75-78: omni_manifold over all 16 (type_a, type_b) pairs
// ---------------------------------------------------------------------------

fn omni(
    f: &libloading::Symbol<'_, FnOmni>,
    ta: C2_TYPE,
    a: [f32; 5],
    tb: C2_TYPE,
    b: [f32; 5],
    seed: u8,
) -> c2Manifold {
    let mut m = poison_manifold(seed);
    zero_stack();
    unsafe {
        f(&mut m, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
    }
    m
}

fn warm_omni(cf: &libloading::Symbol<'_, FnOmni>, rf: &libloading::Symbol<'_, FnOmni>) {
    warmup(|| {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                let _ = omni(cf, ta, [-1.0, -1.0, 1.0, 1.0, 0.5], tb, [-2.0, 0.0, 2.0, 0.0, 0.5], 1);
                let _ = omni(rf, ta, [-1.0, -1.0, 1.0, 1.0, 0.5], tb, [-2.0, 0.0, 2.0, 0.0, 0.5], 1);
            }
        }
    });
}

fn quintuple(rng: &mut Rng, mode: u32) -> [f32; 5] {
    match mode {
        // row 75: random
        0 => [
            rng.f_norm(6.0),
            rng.f_norm(6.0),
            rng.f_norm(6.0),
            rng.f_norm(6.0),
            rng.f_pos(3.0),
        ],
        // row 76: small half-integer lattice
        1 => [
            rng.f_half_lattice(3),
            rng.f_half_lattice(3),
            rng.f_half_lattice(3),
            rng.f_half_lattice(3),
            (rng.below(4) as f32) * 0.5,
        ],
        // row 77: degenerate pool (includes negative radii)
        _ => [
            rng.f_special(),
            rng.f_special(),
            rng.f_special(),
            rng.f_special(),
            if rng.bool() { rng.f_special() } else { -rng.f_pos(3.0) },
        ],
    }
}

#[test]
fn row75_76_77_78_omni_manifold_all_16_pairs() {
    let l = libs();
    let (cf, rf) = l.get::<FnOmni>("omni_manifold");
    warm_omni(&cf, &rf);
    let mut rng = Rng::new(75);
    let mut seen = std::collections::BTreeSet::new();
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for mode in 0..3u32 {
                for i in 0..N {
                    let a = quintuple(&mut rng, mode);
                    let b = quintuple(&mut rng, mode);
                    // row 78: several distinct poison patterns
                    for seed in [0u8, 37, 211] {
                        let cm = omni(&cf, ta, a, tb, b, seed);
                        let rm = omni(&rf, ta, a, tb, b, seed);
                        eq(
                            "omni_manifold",
                            &format!("ta={ta} tb={tb} mode={mode} i={i} seed={seed} a={a:?} b={b:?}"),
                            &cm,
                            &rm,
                        );
                        seen.insert((ta, tb, cm.count));
                    }
                }
            }
        }
    }
    println!("row75-78 distinct (ta,tb,count) triples: {}", seen.len());
    // POLY / unhandled pairs must always report count == 0
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            if ta == C2_TYPE_POLY || tb == C2_TYPE_POLY {
                assert!(
                    seen.iter().filter(|&&(x, y, _)| x == ta && y == tb).all(|&(_, _, c)| c == 0),
                    "POLY pair ({ta},{tb}) reported a contact"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 79: ptr_from_parts for the three handled types
// ---------------------------------------------------------------------------

#[test]
fn row79_ptr_from_parts() {
    let l = libs();
    let (cf, rf) = l.get::<FnPtrFromParts>("ptr_from_parts");
    let mut rng = Rng::new(79);
    for i in 0..N * 4 {
        let q = quintuple(&mut rng, (i % 3) as u32);
        for &ty in VALID_TYPES.iter() {
            let cp = unsafe { cf(ty, q[0], q[1], q[2], q[3], q[4]) };
            let rp = unsafe { rf(ty, q[0], q[1], q[2], q[3], q[4]) };
            assert!(!cp.is_null(), "C ptr_from_parts returned NULL for ty={ty}");
            assert!(!rp.is_null(), "Rust ptr_from_parts returned NULL for ty={ty}");
            let n = match ty {
                C2_TYPE_CIRCLE => 12,
                C2_TYPE_AABB => 16,
                _ => 20,
            };
            let cb = unsafe { std::slice::from_raw_parts(cp as *const u8, n) };
            let rb = unsafe { std::slice::from_raw_parts(rp as *const u8, n) };
            assert_eq!(
                cb, rb,
                "ptr_from_parts contents differ for ty={ty} q={q:?}\n  C   {}\n  Rust {}",
                hex(cb),
                hex(rb)
            );
            // Both allocate with malloc(3); free them so the test does not leak
            // unboundedly (omni_manifold itself leaks, deliberately).
            unsafe {
                libc_free(cp);
                libc_free(rp);
            }
        }
    }
}

unsafe extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}
