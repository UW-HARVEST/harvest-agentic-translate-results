//! Phase B — rows 67..75 of CONFIGS.md: the dispatch layer (`c2Collide`),
//! `ptr_from_parts`, and the top-level `omni_manifold` across the full
//! `type_a x type_b` cross-product and every value family.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 3000;

/// Storage for a shape addressed as `const void *`, exactly as `c2Collide`
/// expects. Over-sized so a mis-typed read stays inside our own buffer in both
/// libraries (and therefore still compares equal).
#[repr(C, align(8))]
struct Blob([u8; 64]);

impl Blob {
    fn zeroed() -> Self {
        Blob([0; 64])
    }
    fn ptr(&self) -> *const c_void {
        self.0.as_ptr() as *const c_void
    }
    fn write<T: Copy>(&mut self, v: T) {
        assert!(std::mem::size_of::<T>() <= 64);
        unsafe { std::ptr::write_unaligned(self.0.as_mut_ptr() as *mut T, v) };
    }
}

fn build(rng: &mut Rng, ty: c_int, family: usize, i: usize) -> Blob {
    let mut b = Blob::zeroed();
    let coord = |r: &mut Rng| -> f32 {
        match family {
            0 => r.sym(5.0),
            1 => r.grid(0.5, 8),
            2 => r.sym(1e18),
            3 => r.sym(1e-20),
            _ => r.spicy(),
        }
    };
    let v = |r: &mut Rng| c2v {
        x: coord(r),
        y: coord(r),
    };
    match ty {
        C2_TYPE_CIRCLE => b.write(c2Circle {
            p: v(rng),
            r: if i % 4 == 0 { 0.0 } else { coord(rng) },
        }),
        C2_TYPE_AABB => {
            let a = v(rng);
            let c = v(rng);
            b.write(match i % 3 {
                0 => c2AABB {
                    min: c2v {
                        x: a.x.min(c.x),
                        y: a.y.min(c.y),
                    },
                    max: c2v {
                        x: a.x.max(c.x),
                        y: a.y.max(c.y),
                    },
                },
                1 => c2AABB { min: a, max: a },
                _ => c2AABB { min: a, max: c },
            })
        }
        C2_TYPE_CAPSULE => {
            let a = v(rng);
            b.write(c2Capsule {
                a,
                b: if i % 5 == 0 { a } else { v(rng) },
                r: if i % 4 == 1 { 0.0 } else { coord(rng) },
            })
        }
        _ => {
            let mut p = c2Poly::default();
            p.count = 4;
            for k in 0..4 {
                p.verts[k] = v(rng);
            }
            // c2Poly is 132 bytes, larger than Blob; only the header matters
            // because c2Collide has no poly arm and never dereferences it.
            b.write(p.count);
        }
    }
    b
}

// ===========================================================================
// Rows 67, 68 — c2Collide over the full 4x4 dispatch plus out-of-range enums.
// ===========================================================================

#[test]
fn rows67_68_collide_all_type_pairs() {
    let p = pair();
    let (cf, rf) = p.get::<FnCollide>(b"c2Collide");
    let mut rng = Rng::new(0x6700);
    let mut tys: Vec<c_int> = ALL_TYPES.to_vec();
    tys.extend_from_slice(&BAD_TYPES);
    for i in 0..N {
        let family = i % 5;
        for &ta in &tys {
            for &tb in &tys {
                let A = build(&mut rng, ta, family, i);
                let B = build(&mut rng, tb, family, i + 1);
                let mut cm = poison_manifold(i as u8);
                let mut rm = cm;
                scrub_stack();
                unsafe { cf(A.ptr(), ta, B.ptr(), tb, &mut cm) };
                scrub_stack();
                unsafe { rf(A.ptr(), ta, B.ptr(), tb, &mut rm) };
                same(&format!("c2Collide ta={ta} tb={tb} family={family}"), &cm, &rm);
            }
        }
    }
}

// ===========================================================================
// Rows 69, 70 — ptr_from_parts. For the three real types the allocated struct
// is compared byte-for-byte. For POLY / out-of-range the C falls off the end of
// a non-void function, so only "did not crash" is checked (see ERRORS.md #61).
// ===========================================================================

#[test]
fn rows69_70_ptr_from_parts() {
    let p = pair();
    let (cf, rf) = p.get::<FnPtrFromParts>(b"ptr_from_parts");
    let mut rng = Rng::new(0x6900);
    for i in 0..N * 8 {
        let vals: [f32; 5] = std::array::from_fn(|_| match i % 5 {
            0 => rng.sym(5.0),
            1 => rng.grid(0.5, 8),
            2 => rng.sym(1e18),
            3 => rng.sym(1e-20),
            _ => rng.spicy(),
        });
        for &ty in &ALL_TYPES {
            let cp = unsafe { cf(ty, vals[0], vals[1], vals[2], vals[3], vals[4]) };
            let rp = unsafe { rf(ty, vals[0], vals[1], vals[2], vals[3], vals[4]) };
            match ty {
                C2_TYPE_CIRCLE => unsafe {
                    same(
                        "ptr_from_parts circle",
                        &*(cp as *const c2Circle),
                        &*(rp as *const c2Circle),
                    )
                },
                C2_TYPE_AABB => unsafe {
                    same(
                        "ptr_from_parts aabb",
                        &*(cp as *const c2AABB),
                        &*(rp as *const c2AABB),
                    )
                },
                C2_TYPE_CAPSULE => unsafe {
                    same(
                        "ptr_from_parts capsule",
                        &*(cp as *const c2Capsule),
                        &*(rp as *const c2Capsule),
                    )
                },
                // POLY: the C returns an indeterminate value; nothing to assert.
                _ => {}
            }
        }
        // Row 70: out-of-range enum values must not crash either library.
        for &ty in &BAD_TYPES {
            let _ = unsafe { cf(ty, vals[0], vals[1], vals[2], vals[3], vals[4]) };
            let _ = unsafe { rf(ty, vals[0], vals[1], vals[2], vals[3], vals[4]) };
        }
    }
}

// ===========================================================================
// Rows 71..75 — omni_manifold: the whole library through its public entry
// point, all 16 type combinations x every value family, plus out-of-range enums.
// ===========================================================================

fn omni_sweep(seed: u64, iters: usize, family: usize, types: &[c_int], label: &str) {
    let p = pair();
    let (cf, rf) = p.get::<FnOmni>(b"omni_manifold");
    let mut rng = Rng::new(seed);
    let mut counts = [0usize; 4];
    for i in 0..iters {
        let ta = types[i % types.len()];
        let tb = types[(i / types.len()) % types.len()];
        let v: [f32; 10] = std::array::from_fn(|_| match family {
            0 => rng.sym(5.0),
            1 => rng.grid(0.5, 8),
            2 => rng.sym(1e18),
            3 => rng.sym(1e-20),
            _ => rng.spicy(),
        });
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        scrub_stack();
        unsafe {
            cf(
                &mut cm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9],
            )
        };
        scrub_stack();
        unsafe {
            rf(
                &mut rm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9],
            )
        };
        same(&format!("{label} i={i} ta={ta} tb={tb}"), &cm, &rm);
        if (0..=2).contains(&cm.count) {
            counts[cm.count as usize] += 1;
        }
    }
    // Coverage sanity: the sweep must actually produce manifolds, not just
    // bail out with count == 0 every time.
    if family < 4 && types == ALL_TYPES {
        assert!(
            counts[1] > 0,
            "{label}: never produced a 1-point manifold ({counts:?})"
        );
    }
}

#[test]
fn row71_omni_dense_overlap() {
    omni_sweep(0x7100, 120_000, 0, &ALL_TYPES, "row71 tame");
}

#[test]
fn row72_omni_wide_range() {
    omni_sweep(0x7200, 60_000, 2, &ALL_TYPES, "row72 large");
    omni_sweep(0x7201, 60_000, 3, &ALL_TYPES, "row72 tiny");
}

#[test]
fn row73_omni_grid_snapped() {
    omni_sweep(0x7300, 120_000, 1, &ALL_TYPES, "row73 grid");
}

#[test]
fn row74_omni_pathological_values() {
    omni_sweep(0x7400, 120_000, 4, &ALL_TYPES, "row74 spicy");
}

#[test]
fn row75_omni_out_of_range_types() {
    let mut tys: Vec<c_int> = ALL_TYPES.to_vec();
    tys.extend_from_slice(&BAD_TYPES);
    for family in 0..5 {
        omni_sweep(
            0x7500 + family as u64,
            30_000,
            family,
            &tys,
            &format!("row75 family={family}"),
        );
    }
}
