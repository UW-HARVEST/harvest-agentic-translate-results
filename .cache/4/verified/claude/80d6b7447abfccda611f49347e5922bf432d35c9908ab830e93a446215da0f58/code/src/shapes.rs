//! Shape helpers: plane extraction, AABB corner expansion, proxy construction,
//! support mapping and edge-normal generation.
//!
//! These use raw pointer arithmetic rather than slice indexing because the C code
//! indexes arrays without bounds checks and several call sites legitimately rely on
//! that (see `c2Support` with `count == 0` below). Panicking across an `extern "C"`
//! boundary would be a behavioural difference, so we never introduce bounds checks
//! that C does not have.

use crate::math::{c2CCW90, c2Dot, c2Norm, c2Sub, c2V};
use crate::types::{
    c2AABB, c2Capsule, c2Circle, c2Poly, c2Proxy, c2v, c2h, C2_TYPE, C2_TYPE_AABB,
    C2_TYPE_CAPSULE, C2_TYPE_CIRCLE,
};
use core::ffi::{c_int, c_void};

/// # Safety
///
/// `p` must point to a valid, initialised `c2Poly`. `i` is used **unchecked**,
/// exactly as in C: values outside `0..8` read outside `p->verts` /
/// `p->norms`, and the caller is responsible for those bytes being mapped.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const c2Poly, i: c_int) -> c2h {
    unsafe {
        let norms = (*p).norms.as_ptr();
        let verts = (*p).verts.as_ptr();
        // C indexes `p->norms[i]` / `p->verts[i]` unchecked.
        let n = *norms.offset(i as isize);
        c2h {
            n,
            d: c2Dot(*norms.offset(i as isize), *verts.offset(i as isize)),
        }
    }
}

/// # Safety
///
/// `out` must be valid for writes of **4** `c2v` values and `bb` must point to
/// a valid, initialised `c2AABB`. `bb` is not modified (it is `*mut` only
/// because the C signature is).
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        let min = (*bb).min;
        let max = (*bb).max;
        *out.offset(0) = min;
        *out.offset(1) = c2V(max.x, min.y);
        *out.offset(2) = max;
        *out.offset(3) = c2V(min.x, max.y);
    }
}

/// Builds a GJK proxy for a shape.
///
/// **Faithfully reproduces a bug in the C source**: the `switch` has no
/// `C2_TYPE_POLY` case (and no `default`), so for a polygon — or for any
/// out-of-range `C2_TYPE` — this function writes *nothing at all* and leaves `*p`
/// exactly as the caller left it. `c2GJK` therefore operates on an untouched
/// proxy whenever `typeA`/`typeB` is `C2_TYPE_POLY`. See the comment in
/// `crate::gjk::c2GJK` for how that is handled.
///
/// # Safety
///
/// `p` must point to a writable `c2Proxy`. `shape` must point to a shape
/// matching `type_` -- a `c2Circle`, `c2AABB` or `c2Capsule`. For
/// `C2_TYPE_POLY` or any out-of-range value neither pointer is
/// dereferenced and `*p` is left completely untouched.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
    unsafe {
        match type_ {
            C2_TYPE_CIRCLE => {
                let c = shape as *const c2Circle;
                (*p).radius = (*c).r;
                (*p).count = 1;
                (*p).verts[0] = (*c).p;
            }
            C2_TYPE_AABB => {
                let bb = shape as *mut c2AABB;
                (*p).radius = 0.0;
                (*p).count = 4;
                c2BBVerts((*p).verts.as_mut_ptr(), bb);
            }
            C2_TYPE_CAPSULE => {
                let c = shape as *const c2Capsule;
                (*p).radius = (*c).r;
                (*p).count = 2;
                (*p).verts[0] = (*c).a;
                (*p).verts[1] = (*c).b;
            }
            // C2_TYPE_POLY and any other value: no case matches, nothing is written.
            _ => {}
        }
    }
}

/// Index of the vertex furthest along `d`.
///
/// Note that C reads `verts[0]` *before* testing the loop bound, so `count <= 0`
/// still dereferences `verts[0]` and returns `0`. That path is live: it is exactly
/// what happens for the all-zero polygon proxy described in `c2MakeProxy`.
///
/// # Safety
///
/// `verts` must be valid for reads of `max(count, 1)` `c2v` values: note the
/// `max`, because the C original reads `verts[0]` **before** testing the loop
/// bound, so `verts[0]` must be readable even when `count <= 0`.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax: c_int = 0;
        let mut dmax = c2Dot(*verts.offset(0), d);
        let mut i: c_int = 1;
        while i < count {
            let dot = c2Dot(*verts.offset(i as isize), d);
            if dot > dmax {
                imax = i;
                dmax = dot;
            }
            i += 1;
        }
        imax
    }
}

/// # Safety
///
/// `verts` must be valid for reads and `norms` for writes of `count` `c2v`
/// values each. `count <= 0` touches neither. `verts` is not modified (it is
/// `*mut` only because the C signature is).
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut c2v, norms: *mut c2v, count: c_int) {
    unsafe {
        let mut i: c_int = 0;
        while i < count {
            let a = i;
            let b = if i + 1 < count { i + 1 } else { 0 };
            let e = c2Sub(*verts.offset(b as isize), *verts.offset(a as isize));
            *norms.offset(i as isize) = c2Norm(c2CCW90(e));
            i += 1;
        }
    }
}
