//! Simplex reduction (`c22`, `c23`), search direction, witness points and the GJK
//! distance routine itself.

use crate::fp;
use crate::math::{
    c2Add, c2CCW90, c2Det2, c2Dot, c2Len, c2Mulvs, c2Mulxv, c2MulrvT, c2Neg, c2Norm, c2Skew,
    c2Sub, c2V, c2xIdentity,
};
use crate::shapes::{c2MakeProxy, c2Support};
use crate::types::{
    c2GJKCache, c2Proxy, c2Simplex, c2sv, c2v, c2x, C2_FLT_EPSILON, C2_FLT_MAX, C2_SIMPLEX_SLOTS,
    C2_TYPE,
};
use core::ffi::{c_float, c_int, c_void};

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> c_float {
    unsafe {
        // C: `default:` falls into `case 1:`, so anything other than 2 or 3 yields 0.
        match (*s).count {
            2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
            3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
            _ => 0.0,
        }
    }
}

/// Reduce a 2-vertex simplex.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else {
            (*s).a.u = u;
            (*s).b.u = v;
            // GCC: `addss %xmm0,%xmm2` with u as destination.
            (*s).div = fp::add(u, v);
            (*s).count = 2;
        }
    }
}

/// Reduce a 3-vertex simplex.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let c = (*s).c.p;
        let uAB = c2Dot(b, c2Sub(b, a));
        let vAB = c2Dot(a, c2Sub(a, b));
        let uBC = c2Dot(c, c2Sub(c, b));
        let vBC = c2Dot(b, c2Sub(b, c));
        let uCA = c2Dot(a, c2Sub(a, c));
        let vCA = c2Dot(c, c2Sub(c, a));
        let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
        // GCC emits `mulss %xmm4,<det>` for all three, i.e. the c2Det2 result is the
        // destination operand and `area` is the source.
        let uABC = fp::mul(c2Det2(b, c), area);
        let vABC = fp::mul(c2Det2(c, a), area);
        let wABC = fp::mul(c2Det2(a, b), area);
        if vAB <= 0.0 && uCA <= 0.0 {
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            (*s).a = (*s).c;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            (*s).a.u = uAB;
            (*s).b.u = vAB;
            // GCC: `addss %xmm3,%xmm2` with uAB as destination.
            (*s).div = fp::add(uAB, vAB);
            (*s).count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            (*s).a = (*s).b;
            (*s).b = (*s).c;
            (*s).a.u = uBC;
            (*s).b.u = vBC;
            // GCC: `addss %xmm5,%xmm6` with vBC as destination -- operands reversed
            // with respect to the source order.
            (*s).div = fp::add(vBC, uBC);
            (*s).count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            (*s).b = (*s).a;
            (*s).a = (*s).c;
            (*s).a.u = uCA;
            (*s).b.u = vCA;
            (*s).div = fp::add(uCA, vCA);
            (*s).count = 2;
        } else {
            (*s).a.u = uABC;
            (*s).b.u = vABC;
            (*s).c.u = wABC;
            // GCC: `addss %xmm3,%xmm2` (uABC+vABC) then `addss %xmm2,%xmm0` with
            // wABC as the destination of the outer add.
            (*s).div = fp::add(wABC, fp::add(uABC, vABC));
            (*s).count = 3;
        }
    }
}

/// Next search direction.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        match (*s).count {
            1 => c2Neg((*s).a.p),
            2 => {
                let ab = c2Sub((*s).b.p, (*s).a.p);
                if c2Det2(ab, c2Neg((*s).a.p)) > 0.0 {
                    c2Skew(ab)
                } else {
                    c2CCW90(ab)
                }
            }
            // C: `case 3: default:` both return c2V(0, 0)
            _ => c2V(0.0, 0.0),
        }
    }
}

/// Closest points on each shape, in world space.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => {
                *a = (*s).a.sA;
                *b = (*s).a.sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs((*s).a.sA, fp::mul(den, (*s).a.u)),
                    c2Mulvs((*s).b.sA, fp::mul(den, (*s).b.u)),
                );
                *b = c2Add(
                    c2Mulvs((*s).a.sB, fp::mul(den, (*s).a.u)),
                    c2Mulvs((*s).b.sB, fp::mul(den, (*s).b.u)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.sA, fp::mul(den, (*s).a.u)),
                        c2Mulvs((*s).b.sA, fp::mul(den, (*s).b.u)),
                    ),
                    c2Mulvs((*s).c.sA, fp::mul(den, (*s).c.u)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.sB, fp::mul(den, (*s).a.u)),
                        c2Mulvs((*s).b.sB, fp::mul(den, (*s).b.u)),
                    ),
                    c2Mulvs((*s).c.sB, fp::mul(den, (*s).c.u)),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
        }
    }
}

/// Closest point on the simplex to the origin, in Minkowski-difference space.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => (*s).a.p,
            // GCC is asymmetric here: for the b term it emits `mulss 0x3c(%rdi),%xmm1`
            // (den is the destination), but for the a term `mulss %xmm2,%xmm1` with
            // s->a.u as the destination.
            2 => c2Add(
                c2Mulvs((*s).a.p, fp::mul((*s).a.u, den)),
                c2Mulvs((*s).b.p, fp::mul(den, (*s).b.u)),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

/// GJK distance between two shapes.
///
/// # Uninitialized locals in the C original
///
/// C declares `c2Proxy pA, pB;` and `c2Simplex s;` without an initializer. Two of
/// those genuinely matter:
///
/// 1. `c2MakeProxy` has no `C2_TYPE_POLY` case, so when `typeA`/`typeB` is
///    `C2_TYPE_POLY` the corresponding proxy is never written. This is reachable
///    from the public API via `c2CapsuletoPolyManifold` (and therefore from
///    `c2AABBtoCapsuleManifold` / `omni_manifold`).
/// 2. `s.b.u` is read by `c2Witness` if the solver exits by exhausting all 20
///    iterations immediately after appending a vertex.
///
/// Measured against the compiled C library, both regions read back as all-zero
/// bytes (they sit on freshly-mapped, zero-filled stack pages, and no shallower
/// call in the library ever writes them). Verified directly: `c2GJK` with
/// `typeB == C2_TYPE_POLY` returns the distance to the *origin* with
/// `outB == (0, 0)` and `iterations == 0` — i.e. the polygon proxy behaves as a
/// single point at the origin with radius 0, which is precisely the all-zero
/// proxy. Zero-initialising here reproduces the C library's observable behaviour
/// exactly, without relying on undefined behaviour.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> c_float {
    unsafe {
        let ax: c2x = if ax_ptr.is_null() {
            c2xIdentity()
        } else {
            *ax_ptr
        };
        let bx: c2x = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            *bx_ptr
        };

        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);

        let mut s = c2Simplex::default();
        // C: `c2sv *verts = &s.a;` — the four c2sv slots are contiguous.
        let verts: *mut c2sv = &raw mut s.a;

        let mut cache_was_read: c_int = 0;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                // `cache->iA` / `cache->iB` are indexed unchecked in C; use raw
                // pointers so an over-long cache aliases the same way it does there.
                let ia_base: *const c_int = (*cache).iA.as_ptr();
                let ib_base: *const c_int = (*cache).iB.as_ptr();
                let n = (*cache).count;
                let mut i: c_int = 0;
                while i < n {
                    let iA = *ia_base.offset(i as isize);
                    let iB = *ib_base.offset(i as isize);
                    let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
                    let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
                    // Only 4 c2sv slots exist; a cache produced by c2GJK never has
                    // count > 3, so this bound is never hit by valid input.
                    if (i as usize) < C2_SIMPLEX_SLOTS {
                        let v = verts.offset(i as isize);
                        (*v).iA = iA;
                        (*v).sA = sA;
                        (*v).iB = iB;
                        (*v).sB = sB;
                        (*v).p = c2Sub((*v).sB, (*v).sA);
                        (*v).u = 0.0;
                    }
                    i += 1;
                }
                s.count = (*cache).count;
                s.div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2GJKSimplexMetric(&mut s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                // Reproduced verbatim, including the `metric < -1.0e8f` test which
                // makes the conjunction essentially always false.
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                    cache_was_read = 1;
                }
            }
        }
        if cache_was_read == 0 {
            s.a.iA = 0;
            s.a.iB = 0;
            s.a.sA = c2Mulxv(ax, pA.verts[0]);
            s.a.sB = c2Mulxv(bx, pB.verts[0]);
            s.a.p = c2Sub(s.a.sB, s.a.sA);
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        // C: `int saveA[3], saveB[3];` — sized to the 4 available simplex slots so
        // that a hostile cache cannot corrupt the stack. Identical for count <= 3.
        let mut saveA = [0 as c_int; C2_SIMPLEX_SLOTS];
        let mut saveB = [0 as c_int; C2_SIMPLEX_SLOTS];
        let mut save_count: c_int = 0;
        let mut d0: c_float = C2_FLT_MAX;
        let mut d1: c_float;
        let mut iter: c_int = 0;
        let mut hit: c_int = 0;

        while iter < 20 {
            save_count = s.count;
            let mut i: c_int = 0;
            while i < save_count {
                if (i as usize) < C2_SIMPLEX_SLOTS {
                    saveA[i as usize] = (*verts.offset(i as isize)).iA;
                    saveB[i as usize] = (*verts.offset(i as isize)).iB;
                }
                i += 1;
            }

            match s.count {
                1 => {}
                2 => c22(&mut s),
                3 => c23(&mut s),
                _ => {}
            }

            if s.count == 3 {
                hit = 1;
                break;
            }

            let p = c2L(&mut s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;

            let d = c2D(&mut s);
            if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {
                break;
            }

            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));

            if (s.count as usize) < C2_SIMPLEX_SLOTS {
                let v = verts.offset(s.count as isize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
            }

            let mut dup: c_int = 0;
            let mut i: c_int = 0;
            while i < save_count {
                let (sa, sb) = if (i as usize) < C2_SIMPLEX_SLOTS {
                    (saveA[i as usize], saveB[i as usize])
                } else {
                    (0, 0)
                };
                if iA == sa && iB == sb {
                    dup = 1;
                    break;
                }
                i += 1;
            }
            if dup != 0 {
                break;
            }

            s.count += 1;
            iter += 1;
        }
        let _ = save_count;

        let mut a = c2v::default();
        let mut b = c2v::default();
        c2Witness(&mut s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));

        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            if dist > fp::add(rA, rB) && dist > C2_FLT_EPSILON {
                dist -= fp::add(rA, rB);
                let n = c2Norm(c2Sub(b, a));
                a = c2Add(a, c2Mulvs(n, rA));
                b = c2Sub(b, c2Mulvs(n, rB));
                if a.x == b.x && a.y == b.y {
                    dist = 0.0;
                }
            } else {
                let p = c2Mulvs(c2Add(a, b), 0.5);
                a = p;
                b = p;
                dist = 0.0;
            }
        }

        if !cache.is_null() {
            (*cache).metric = c2GJKSimplexMetric(&mut s);
            (*cache).count = s.count;
            let ia_base: *mut c_int = (*cache).iA.as_mut_ptr();
            let ib_base: *mut c_int = (*cache).iB.as_mut_ptr();
            let mut i: c_int = 0;
            while i < s.count {
                if (i as usize) < C2_SIMPLEX_SLOTS {
                    let v = verts.offset(i as isize);
                    *ia_base.offset(i as isize) = (*v).iA;
                    *ib_base.offset(i as isize) = (*v).iB;
                }
                i += 1;
            }
            (*cache).div = s.div;
        }

        if !outA.is_null() {
            *outA = a;
        }
        if !outB.is_null() {
            *outB = b;
        }
        if !iterations.is_null() {
            *iterations = iter;
        }
        dist
    }
}
