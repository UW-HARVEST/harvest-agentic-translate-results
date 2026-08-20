//! Clipping helpers and the per-shape-pair manifold generators.
//!
//! `c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep` and `c2Incident`
//! are `static` in the C source and are therefore *not* exported here either — the
//! C shared object does not list them in its dynamic symbol table.

use crate::math::{
    c2Absv, c2Add, c2CCW90, c2Clampv, c2Dist, c2Dot, c2Intersect, c2Mulvs, c2Mulxv, c2MulxvT,
    c2Neg, c2Norm, c2Skew, c2Sub, c2V, c2xIdentity,
};
use crate::shapes::{c2BBVerts, c2Norms, c2PlaneAt, c2Support};
use crate::fp;
use crate::gjk::c2GJK;
use crate::types::{
    c2AABB, c2Capsule, c2Circle, c2Manifold, c2Poly, c2v, c2x, c2h, C2_FLT_MAX, C2_TYPE_CAPSULE,
    C2_TYPE_CIRCLE, C2_TYPE_POLY,
};
use core::ffi::{c_float, c_int, c_void};

/// Clip a 2-point segment against a half-space, keeping the negative side.
///
/// The C original declares `c2v out[2]` uninitialized and can push up to three
/// entries (when `d0 < 0`, `d1 < 0` and `d0 * d1` underflows to exactly `0`), so we
/// give the buffer four slots to avoid stepping outside it. Only `out[0]` and
/// `out[1]` are ever copied back, and whenever fewer than two entries were pushed
/// every caller bails out immediately and discards `seg`, so the initial contents
/// are not observable.
fn c2Clip(seg: *mut c2v, h: c2h) -> c_int {
    unsafe {
        let mut out = [c2v::default(); 4];
        let mut sp: usize = 0;
        let d0 = c2Dist(h, *seg.offset(0));
        if d0 < 0.0 {
            out[sp] = *seg.offset(0);
            sp += 1;
        }
        let d1 = c2Dist(h, *seg.offset(1));
        if d1 < 0.0 {
            out[sp] = *seg.offset(1);
            sp += 1;
        }
        if d0 == 0.0 && d1 == 0.0 {
            out[sp] = *seg.offset(0);
            sp += 1;
            out[sp] = *seg.offset(1);
            sp += 1;
        } else if fp::mul(d0, d1) <= 0.0 {
            // GCC -O0: `movss -0x18(d0),%xmm0; movaps %xmm0,%xmm1;
            // mulss -0x1c(d1),%xmm1` -> the destination operand is d0.
            out[sp] = c2Intersect(*seg.offset(0), *seg.offset(1), d0, d1);
            sp += 1;
        }
        *seg.offset(0) = out[0];
        *seg.offset(1) = out[1];
        sp as c_int
    }
}

/// Clip `seg` against the two planes bounding the edge `ra` -> `rb`.
fn c2SidePlanes(seg: *mut c2v, ra: c2v, rb: c2v, h: *mut c2h) -> c_int {
    unsafe {
        let in_ = c2Norm(c2Sub(rb, ra));
        let left = c2h {
            n: c2Neg(in_),
            d: c2Dot(c2Neg(in_), ra),
        };
        let right = c2h {
            n: in_,
            d: c2Dot(in_, rb),
        };
        if c2Clip(seg, left) < 2 {
            return 0;
        }
        if c2Clip(seg, right) < 2 {
            return 0;
        }
        if !h.is_null() {
            (*h).n = c2CCW90(in_);
            (*h).d = c2Dot(c2CCW90(in_), ra);
        }
        1
    }
}

fn c2SidePlanesFromPoly(seg: *mut c2v, x: c2x, p: *const c2Poly, e: c_int, h: *mut c2h) -> c_int {
    unsafe {
        let verts = (*p).verts.as_ptr();
        let ra = c2Mulxv(x, *verts.offset(e as isize));
        let next = if e + 1 == (*p).count { 0 } else { e + 1 };
        let rb = c2Mulxv(x, *verts.offset(next as isize));
        c2SidePlanes(seg, ra, rb, h)
    }
}

/// Keep whichever of the two segment points lie behind `h`.
fn c2KeepDeep(seg: *mut c2v, h: c2h, m: *mut c2Manifold) {
    unsafe {
        let mut cp: usize = 0;
        for i in 0..2usize {
            let p = *seg.add(i);
            let d = c2Dist(h, p);
            if d <= 0.0 {
                (*m).contact_points[cp] = p;
                (*m).depths[cp] = -d;
                cp += 1;
            }
        }
        (*m).count = cp as c_int;
        (*m).n = h.n;
    }
}

/// Find the incident edge on `ip` most anti-parallel to `rn_in_incident_space`.
///
/// Faithfully keeps C's `int index = ~0;` initialiser. If `ip->count <= 0` the loop
/// never runs and `index` stays `-1`, so `ip->verts[-1]` is read out of bounds —
/// that is reachable through `c2CapsuletoPolyManifold` with an empty polygon, so the
/// raw offset is preserved rather than clamped.
fn c2Incident(incident: *mut c2v, ip: *const c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    unsafe {
        let mut index: c_int = !0;
        let mut min_dot: c_float = C2_FLT_MAX;
        let norms = (*ip).norms.as_ptr();
        let verts = (*ip).verts.as_ptr();
        let count = (*ip).count;
        let mut i: c_int = 0;
        while i < count {
            let dot = c2Dot(rn_in_incident_space, *norms.offset(i as isize));
            if dot < min_dot {
                min_dot = dot;
                index = i;
            }
            i += 1;
        }
        *incident.offset(0) = c2Mulxv(ix, *verts.offset(index as isize));
        let next = if index + 1 == count { 0 } else { index + 1 };
        *incident.offset(1) = c2Mulxv(ix, *verts.offset(next as isize));
    }
}

/// # Safety
///
/// `m` must point to a writable `c2Manifold`. Only `m->count` is written when
/// the circles do not overlap; the other fields keep the caller's bytes.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let d = c2Sub(B.p, A.p);
        let d2 = c2Dot(d, d);
        // GCC -O0: `movss -0x38(A.r),%xmm1; movss -0x48(B.r),%xmm0;
        // addss %xmm1,%xmm0` -> B.r is the destination operand.
        let r = fp::add(B.r, A.r);
        if d2 < fp::mul(r, r) {
            let l = d2.sqrt();
            let n = if l != 0.0 {
                c2Mulvs(d, 1.0 / l)
            } else {
                c2V(0.0, 1.0)
            };
            (*m).count = 1;
            (*m).depths[0] = r - l;
            (*m).contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

/// # Safety
///
/// `m` must point to a writable `c2Manifold`. Only `m->count` is written when
/// the circle is outside the box; the other fields keep the caller's bytes.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let L = c2Clampv(A.p, B.min, B.max);
        let ab = c2Sub(L, A.p);
        let d2 = c2Dot(ab, ab);
        let r2 = fp::mul(A.r, A.r);
        if d2 < r2 {
            if d2 != 0.0 {
                let d = d2.sqrt();
                let n = c2Norm(ab);
                (*m).count = 1;
                (*m).depths[0] = A.r - d;
                (*m).contact_points[0] = c2Add(A.p, c2Mulvs(n, d));
                (*m).n = n;
            } else {
                let mid = c2Mulvs(c2Add(B.min, B.max), 0.5);
                let e = c2Mulvs(c2Sub(B.max, B.min), 0.5);
                let d = c2Sub(A.p, mid);
                let abs_d = c2Absv(d);
                let x_overlap = e.x - abs_d.x;
                let y_overlap = e.y - abs_d.y;
                let depth: c_float;
                let mut n: c2v;
                if x_overlap < y_overlap {
                    depth = x_overlap;
                    n = c2V(1.0, 0.0);
                    n = c2Mulvs(n, if d.x < 0.0 { 1.0 } else { -1.0 });
                } else {
                    depth = y_overlap;
                    n = c2V(0.0, 1.0);
                    n = c2Mulvs(n, if d.y < 0.0 { 1.0 } else { -1.0 });
                }
                (*m).count = 1;
                (*m).depths[0] = fp::add(A.r, depth);
                (*m).contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
                (*m).n = n;
            }
        }
    }
}

/// # Safety
///
/// `m` must point to a writable `c2Manifold`. Only `m->count` is written when
/// the shapes do not overlap; the other fields keep the caller's bytes.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        // GCC -O0: `movss -0x38(A.r),%xmm1; movss 0x20(B.r),%xmm0;
        // addss %xmm1,%xmm0` -> B.r is the destination operand.
        let r = fp::add(B.r, A.r);
        let d = c2GJK(
            &A as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            core::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &mut a,
            &mut b,
            0,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        if d < r {
            let n = if d == 0.0 {
                c2Norm(c2Skew(c2Sub(B.b, B.a)))
            } else {
                c2Norm(c2Sub(b, a))
            };
            (*m).count = 1;
            (*m).depths[0] = r - d;
            (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

/// # Safety
///
/// `m` must point to a writable `c2Manifold`. Only `m->count` is written when
/// the boxes are separated on either axis; the other fields keep the
/// caller's bytes.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mid_a = c2Mulvs(c2Add(A.min, A.max), 0.5);
        let mid_b = c2Mulvs(c2Add(B.min, B.max), 0.5);
        let eA = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5));
        let eB = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5));
        let d = c2Sub(mid_b, mid_a);
        // C: eA.x + eB.x - ((d.x) < 0 ? -(d.x) : (d.x))
        // GCC -O0 keeps eA at -0x24(%rbp) and eB at -0x2c(%rbp) and emits
        // `movss -0x24(%rbp),%xmm1; movss -0x2c(%rbp),%xmm0; addss %xmm0,%xmm1`,
        // so eA.x is the destination operand -- the source order.
        let dx = fp::add(eA.x, eB.x) - (if d.x < 0.0 { -d.x } else { d.x });
        if dx < 0.0 {
            return;
        }
        // `movss -0x20(eA.y),%xmm1; movss -0x28(eB.y),%xmm0; addss %xmm0,%xmm1`
        let dy = fp::add(eA.y, eB.y) - (if d.y < 0.0 { -d.y } else { d.y });
        if dy < 0.0 {
            return;
        }
        let n: c2v;
        let depth: c_float;
        let p: c2v;
        if dx < dy {
            depth = dx;
            if d.x < 0.0 {
                n = c2V(-1.0, 0.0);
                p = c2Sub(mid_a, c2V(eA.x, 0.0));
            } else {
                n = c2V(1.0, 0.0);
                p = c2Add(mid_a, c2V(eA.x, 0.0));
            }
        } else {
            depth = dy;
            if d.y < 0.0 {
                n = c2V(0.0, -1.0);
                p = c2Sub(mid_a, c2V(0.0, eA.y));
            } else {
                n = c2V(0.0, 1.0);
                p = c2Add(mid_a, c2V(0.0, eA.y));
            }
        }
        (*m).count = 1;
        (*m).contact_points[0] = p;
        (*m).depths[0] = depth;
        (*m).n = n;
    }
}

/// # Safety
///
/// * `m` must point to a writable `c2Manifold` and `B` to a valid, initialised
///   `c2Poly`. `bx_ptr` may be null.
/// * **`B->verts[-1]` is read** -- four bytes before the struct -- whenever
///   every candidate face distance is `NaN`, because `index` keeps its `~0`
///   initialiser. The caller must ensure those bytes are mapped. This
///   reproduces the C original exactly; see the `c2Incident` comment.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    A: c2Capsule,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let d = c2GJK(
            &A as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            B as *const c_void,
            C2_TYPE_POLY,
            bx_ptr,
            &mut a,
            &mut b,
            0,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        if d < 1.0e-6 {
            let bx: c2x = if !bx_ptr.is_null() {
                *bx_ptr
            } else {
                c2xIdentity()
            };
            let mut A_in_B = c2Capsule::default();
            A_in_B.a = c2MulxvT(bx, A.a);
            A_in_B.b = c2MulxvT(bx, A.b);
            let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));

            let b_verts = (*B).verts.as_ptr();
            let b_count = (*B).count;

            let mut ab_h0 = c2h::default();
            ab_h0.n = c2CCW90(ab);
            ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
            let v0 = c2Support(b_verts, b_count, c2Neg(ab_h0.n));
            let s0 = c2Dist(ab_h0, *b_verts.offset(v0 as isize));

            let mut ab_h1 = c2h::default();
            ab_h1.n = c2Skew(ab);
            ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
            let v1 = c2Support(b_verts, b_count, c2Neg(ab_h1.n));
            let s1 = c2Dist(ab_h1, *b_verts.offset(v1 as isize));

            let mut index: c_int = !0;
            let mut sep: c_float = -C2_FLT_MAX;
            let mut code: c_int = 0;
            let mut i: c_int = 0;
            while i < b_count {
                let h = c2PlaneAt(B, i);
                let da = c2Dot(A_in_B.a, c2Neg(h.n));
                let db = c2Dot(A_in_B.b, c2Neg(h.n));
                let d: c_float = if da > db {
                    c2Dist(h, A_in_B.a)
                } else {
                    c2Dist(h, A_in_B.b)
                };
                if d > sep {
                    sep = d;
                    index = i;
                }
                i += 1;
            }
            if s0 > sep {
                sep = s0;
                index = v0;
                code = 1;
            }
            if s1 > sep {
                sep = s1;
                index = v1;
                code = 2;
            }
            let _ = sep;

            match code {
                0 => {
                    let mut seg: [c2v; 2] = [A.a, A.b];
                    let mut h = c2h::default();
                    if c2SidePlanesFromPoly(seg.as_mut_ptr(), bx, B, index, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(seg.as_mut_ptr(), h, m);
                    (*m).n = c2Neg((*m).n);
                }
                1 => {
                    let mut incident = [c2v::default(); 2];
                    c2Incident(incident.as_mut_ptr(), B, bx, ab_h0.n);
                    let mut h = c2h::default();
                    if c2SidePlanes(incident.as_mut_ptr(), A_in_B.b, A_in_B.a, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(incident.as_mut_ptr(), h, m);
                }
                2 => {
                    let mut incident = [c2v::default(); 2];
                    c2Incident(incident.as_mut_ptr(), B, bx, ab_h1.n);
                    let mut h = c2h::default();
                    if c2SidePlanes(incident.as_mut_ptr(), A_in_B.a, A_in_B.b, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(incident.as_mut_ptr(), h, m);
                }
                _ => return,
            }
            let mut i: c_int = 0;
            while i < (*m).count {
                if (i as usize) < 2 {
                    // GCC -O0: `movss 0x4(%rax,%rdx,4),%xmm1  ; m->depths[i]
                    //           movss 0x20(%rbp),%xmm0        ; A.r
                    //           addss %xmm1,%xmm0`            ; dst = A.r
                    (*m).depths[i as usize] =
                        fp::add(A.r, (*m).depths[i as usize]);
                }
                i += 1;
            }
        } else if d < A.r {
            (*m).count = 1;
            (*m).n = c2Norm(c2Sub(b, a));
            (*m).contact_points[0] = c2Add(a, c2Mulvs((*m).n, A.r));
            (*m).depths[0] = A.r - d;
        }
    }
}

/// GCC's stack frame for `c2AABBtoCapsuleManifold`, reproduced exactly.
///
/// This matters because `c2CapsuletoPolyManifold` can read `B->verts[-1]` — four
/// bytes *before* the `c2Poly` — and here the `c2Poly` is a local of
/// `c2AABBtoCapsuleManifold`, so that read lands inside this very frame. GCC -O0
/// lays the frame out as
///
/// ```text
///   -0xb8(%rbp)  c2Manifold *m        (the incoming pointer)
///   -0xb0(%rbp)  c2AABB A             (the by-value copy, 16 bytes)
///   -0xa0(%rbp)  c2Poly p             (132 bytes: count@0, verts@4, norms@0x44)
/// ```
///
/// so `A` ends at `-0xa1` and `p` starts at `-0xa0`, immediately adjacent. `p.verts`
/// therefore begins at `-0x9c`, and `p.verts[-1]` spans `-0xa4 .. -0x9d`, i.e.
///
/// * `verts[-1].x` == `A.max.y`  (`A.max` is at `-0xa8`, so `A.max.y` is at `-0xa4`)
/// * `verts[-1].y` == `p.count`  (== `4`, reinterpreted as `5.6e-45`)
///
/// Putting the two locals in one `#[repr(C)]` struct pins the same adjacency, so the
/// out-of-bounds read observes the same bytes. Reachable whenever `c2Norms` produces
/// NaN normals — e.g. a degenerate AABB with `min == max` — which leaves `index` at
/// its `~0` initialiser inside `c2Incident` / `c2SidePlanesFromPoly`.
#[repr(C)]
struct AABBtoCapsuleFrame {
    a_local: c2AABB,
    p: c2Poly,
}

/// AABB vs capsule, by promoting the AABB to a 4-gon.
///
/// Note the trailing `m->n = c2Neg(m->n);`: when `c2CapsuletoPolyManifold` bails out
/// early it leaves `m->n` untouched, so this negates whatever the *caller* had in
/// that field. Reading through `m` reproduces that exactly.
///
/// # Safety
///
/// `m` must point to a writable `c2Manifold`. On every early-return path
/// `m->n` is still sign-flipped, so the caller's existing `n` is modified even
/// when no contact is reported.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut f = AABBtoCapsuleFrame {
            a_local: A,
            p: c2Poly::default(),
        };
        c2BBVerts(f.p.verts.as_mut_ptr(), &mut f.a_local);
        f.p.count = 4;
        c2Norms(f.p.verts.as_mut_ptr(), f.p.norms.as_mut_ptr(), 4);
        c2CapsuletoPolyManifold(B, &f.p, core::ptr::null(), m);
        (*m).n = c2Neg((*m).n);
    }
}

/// # Safety
///
/// `m` must point to a writable `c2Manifold`. Only `m->count` is written when
/// the capsules do not overlap; the other fields keep the caller's bytes.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(
    A: c2Capsule,
    B: c2Capsule,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        // GCC -O0: `movss 0x20(A.r),%xmm1; movss 0x38(B.r),%xmm0;
        // addss %xmm1,%xmm0` -> B.r is the destination operand.
        let r = fp::add(B.r, A.r);
        let d = c2GJK(
            &A as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &mut a,
            &mut b,
            0,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        if d < r {
            let n = if d == 0.0 {
                c2Norm(c2Skew(c2Sub(A.b, A.a)))
            } else {
                c2Norm(c2Sub(b, a))
            };
            (*m).count = 1;
            (*m).depths[0] = r - d;
            (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}
