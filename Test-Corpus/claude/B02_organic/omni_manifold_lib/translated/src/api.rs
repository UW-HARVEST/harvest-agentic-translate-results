//! The dispatcher plus the two convenience entry points declared in `lib.h`.

use crate::manifold::{
    c2AABBtoAABBManifold, c2AABBtoCapsuleManifold, c2CapsuletoCapsuleManifold,
    c2CircletoAABBManifold, c2CircletoCapsuleManifold, c2CircletoCircleManifold,
};
use crate::math::{c2Neg, c2V};
use crate::types::{
    c2AABB, c2Capsule, c2Circle, c2Manifold, C2_TYPE, C2_TYPE_AABB, C2_TYPE_CAPSULE,
    C2_TYPE_CIRCLE,
};
use core::ffi::{c_float, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(
    A: *const c_void,
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        // No `C2_TYPE_POLY` case at either level, and no `default`: a polygon (or any
        // unrecognised C2_TYPE) leaves the manifold with just `count = 0` and every
        // other field exactly as the caller supplied it.
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => c2CircletoCircleManifold(
                    core::ptr::read_unaligned(A as *const c2Circle),
                    core::ptr::read_unaligned(B as *const c2Circle),
                    m,
                ),
                C2_TYPE_AABB => c2CircletoAABBManifold(
                    core::ptr::read_unaligned(A as *const c2Circle),
                    core::ptr::read_unaligned(B as *const c2AABB),
                    m,
                ),
                C2_TYPE_CAPSULE => c2CircletoCapsuleManifold(
                    core::ptr::read_unaligned(A as *const c2Circle),
                    core::ptr::read_unaligned(B as *const c2Capsule),
                    m,
                ),
                _ => {}
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoAABBManifold(
                        core::ptr::read_unaligned(B as *const c2Circle),
                        core::ptr::read_unaligned(A as *const c2AABB),
                        m,
                    );
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_AABB => c2AABBtoAABBManifold(
                    core::ptr::read_unaligned(A as *const c2AABB),
                    core::ptr::read_unaligned(B as *const c2AABB),
                    m,
                ),
                C2_TYPE_CAPSULE => c2AABBtoCapsuleManifold(
                    core::ptr::read_unaligned(A as *const c2AABB),
                    core::ptr::read_unaligned(B as *const c2Capsule),
                    m,
                ),
                _ => {}
            },
            C2_TYPE_CAPSULE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCapsuleManifold(
                        core::ptr::read_unaligned(B as *const c2Circle),
                        core::ptr::read_unaligned(A as *const c2Capsule),
                        m,
                    );
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_AABB => {
                    c2AABBtoCapsuleManifold(
                        core::ptr::read_unaligned(B as *const c2AABB),
                        core::ptr::read_unaligned(A as *const c2Capsule),
                        m,
                    );
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_CAPSULE => c2CapsuletoCapsuleManifold(
                    core::ptr::read_unaligned(A as *const c2Capsule),
                    core::ptr::read_unaligned(B as *const c2Capsule),
                    m,
                ),
                _ => {}
            },
            _ => {}
        }
    }
}

/// Heap-allocate a shape from five loose floats.
///
/// Uses `malloc` (not Rust's allocator) so that the returned pointer can be handed
/// to / freed by C exactly as before. The allocation is intentionally never freed:
/// `omni_manifold` leaks both shapes in the original too.
///
/// **Faithfully reproduces a bug in the C source**: there is no `C2_TYPE_POLY` case
/// and no `default`, so control falls off the end of the function without a `return`.
/// GCC compiles that path to `mov %r12, %rax`, i.e. it returns the *caller's* stale
/// callee-saved register — an indeterminate value that cannot be replicated. It is
/// unobservable in practice: `c2Collide` has no `C2_TYPE_POLY` case either, so the
/// pointer is never dereferenced for polygons. We return null.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: C2_TYPE,
    a: c_float,
    b: c_float,
    c: c_float,
    d: c_float,
    e: c_float,
) -> *mut c_void {
    unsafe {
        match typ {
            C2_TYPE_CIRCLE => {
                let circle = malloc(core::mem::size_of::<c2Circle>()) as *mut c2Circle;
                (*circle).p = c2V(a, b);
                (*circle).r = c;
                circle as *mut c_void
            }
            C2_TYPE_AABB => {
                let aabb = malloc(core::mem::size_of::<c2AABB>()) as *mut c2AABB;
                (*aabb).min = c2V(a, b);
                (*aabb).max = c2V(c, d);
                aabb as *mut c_void
            }
            C2_TYPE_CAPSULE => {
                let capsule = malloc(core::mem::size_of::<c2Capsule>()) as *mut c2Capsule;
                (*capsule).a = c2V(a, b);
                (*capsule).b = c2V(c, d);
                (*capsule).r = e;
                capsule as *mut c_void
            }
            // C2_TYPE_POLY and any other value: falls off the end of the function.
            _ => core::ptr::null_mut(),
        }
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_manifold(
    m: *mut c2Manifold,
    type_a: C2_TYPE,
    a1: c_float,
    a2: c_float,
    a3: c_float,
    a4: c_float,
    a5: c_float,
    type_b: C2_TYPE,
    b1: c_float,
    b2: c_float,
    b3: c_float,
    b4: c_float,
    b5: c_float,
) {
    unsafe {
        let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
        let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
        c2Collide(A, type_a, B, type_b, m);
    }
}
