#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_float, c_int, c_uint, c_void};

type C2Type = c_uint;

const C2_TYPE_CIRCLE: C2Type = 0;
const C2_TYPE_AABB: C2Type = 1;
const C2_TYPE_CAPSULE: C2Type = 2;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn ordered_mul(mut lhs: c_float, rhs: c_float) -> c_float {
    unsafe {
        std::arch::asm!(
            "mulss {lhs}, {rhs}",
            lhs = inout(xmm_reg) lhs,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    lhs
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn ordered_mul(lhs: c_float, rhs: c_float) -> c_float {
    lhs * rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn ordered_add(mut lhs: c_float, rhs: c_float) -> c_float {
    unsafe {
        std::arch::asm!(
            "addss {lhs}, {rhs}",
            lhs = inout(xmm_reg) lhs,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    lhs
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn ordered_add(lhs: c_float, rhs: c_float) -> c_float {
    lhs + rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn ordered_sub(mut lhs: c_float, rhs: c_float) -> c_float {
    unsafe {
        std::arch::asm!(
            "subss {lhs}, {rhs}",
            lhs = inout(xmm_reg) lhs,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack)
        );
    }
    lhs
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn ordered_sub(lhs: c_float, rhs: c_float) -> c_float {
    lhs - rhs
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2v {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Circle {
    p: C2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Capsule {
    a: C2v,
    b: C2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2GjkCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Proxy {
    radius: c_float,
    count: c_int,
    verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2sv {
    sA: C2v,
    sB: C2v,
    p: C2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: c_float,
    count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: c_float) -> C2v {
    a.x = ordered_mul(b, a.x);
    a.y = ordered_mul(b, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> c_float {
    ordered_add(ordered_mul(a.x, b.x), ordered_mul(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x {
    C2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut C2v, bb: *mut C2Aabb) {
    *out.add(0) = (*bb).min;
    *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
    *out.add(2) = (*bb).max;
    *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(
    shape: *const c_void,
    shape_type: C2Type,
    p: *mut C2Proxy,
) {
    match shape_type {
        C2_TYPE_CIRCLE => {
            let c = shape.cast::<C2Circle>();
            (*p).radius = (*c).r;
            (*p).count = 1;
            (*p).verts[0] = (*c).p;
        }
        C2_TYPE_AABB => {
            (*p).radius = 0.0;
            (*p).count = 4;
            c2BBVerts((*p).verts.as_mut_ptr(), shape.cast_mut().cast::<C2Aabb>());
        }
        C2_TYPE_CAPSULE => {
            let c = shape.cast::<C2Capsule>();
            (*p).radius = (*c).r;
            (*p).count = 2;
            (*p).verts[0] = (*c).a;
            (*p).verts[1] = (*c).b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> c_float {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> c_float {
    ordered_sub(ordered_mul(a.x, b.y), ordered_mul(a.y, b.x))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut C2Simplex) -> c_float {
    match (*s).count {
        2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v {
    c2V(
        ordered_sub(ordered_mul(a.c, b.x), ordered_mul(a.s, b.y)),
        ordered_add(ordered_mul(a.c, b.y), ordered_mul(a.s, b.x)),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut C2Simplex) {
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
        (*s).div = u + v;
        (*s).count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut C2Simplex) {
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
    let uABC = c2Det2(b, c) * area;
    let vABC = c2Det2(c, a) * area;
    let wABC = c2Det2(a, b) * area;
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
        (*s).div = uAB + vAB;
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        (*s).a = (*s).b;
        (*s).b = (*s).c;
        (*s).a.u = uBC;
        (*s).b.u = vBC;
        (*s).div = uBC + vBC;
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        (*s).b = (*s).a;
        (*s).a = (*s).c;
        (*s).a.u = uCA;
        (*s).b.u = vCA;
        (*s).div = uCA + vCA;
        (*s).count = 2;
    } else {
        (*s).a.u = uABC;
        (*s).b.u = vABC;
        (*s).c.u = wABC;
        (*s).div = uABC + vABC + wABC;
        (*s).count = 3;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: C2v) -> C2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut C2Simplex) -> C2v {
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
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(
    verts: *const C2v,
    count: c_int,
    d: C2v,
) -> c_int {
    let mut imax = 0;
    let mut dmax = c2Dot(std::ptr::read_volatile(verts), d);
    for i in 1..count {
        let dot = c2Dot(*verts.add(i as usize), d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut C2Simplex, a: *mut C2v, b: *mut C2v) {
    let den = 1.0 / (*s).div;
    match (*s).count {
        1 => {
            *a = (*s).a.sA;
            *b = (*s).a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs((*s).a.sA, den * (*s).a.u),
                c2Mulvs((*s).b.sA, den * (*s).b.u),
            );
            *b = c2Add(
                c2Mulvs((*s).a.sB, den * (*s).a.u),
                c2Mulvs((*s).b.sB, den * (*s).b.u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*s).a.sA, den * (*s).a.u),
                    c2Mulvs((*s).b.sA, den * (*s).b.u),
                ),
                c2Mulvs((*s).c.sA, den * (*s).c.u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*s).a.sB, den * (*s).a.u),
                    c2Mulvs((*s).b.sB, den * (*s).b.u),
                ),
                c2Mulvs((*s).c.sB, den * (*s).c.u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: c_float) -> C2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut C2Simplex) -> C2v {
    let den = 1.0 / (*s).div;
    match (*s).count {
        1 => (*s).a.p,
        2 => c2Add(
            c2Mulvs((*s).a.p, den * (*s).a.u),
            c2Mulvs((*s).b.p, den * (*s).b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v {
    c2V(
        ordered_add(ordered_mul(a.c, b.x), ordered_mul(a.s, b.y)),
        ordered_sub(ordered_mul(a.c, b.y), ordered_mul(a.s, b.x)),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2Type,
    ax_ptr: *const C2x,
    B: *const c_void,
    typeB: C2Type,
    bx_ptr: *const C2x,
    outA: *mut C2v,
    outB: *mut C2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut C2GjkCache,
) -> c_float {
    let ax = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        *ax_ptr
    };
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        *bx_ptr
    };

    let mut pA = C2Proxy::default();
    let mut pB = C2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);

    let mut s = C2Simplex::default();
    let verts = std::ptr::addr_of_mut!(s.a);
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_was_good = ((*cache).count != 0) as c_int;
        if cache_was_good != 0 {
            for i in 0..(*cache).count {
                let iA = (*cache).iA[i as usize];
                let iB = (*cache).iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = verts.add(i as usize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(&mut s);
            let min_metric = if metric < metric_old {
                metric
            } else {
                metric_old
            };
            let max_metric = if metric > metric_old {
                metric
            } else {
                metric_old
            };
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

    let mut saveA = [0; 3];
    let mut saveB = [0; 3];
    let mut d0 = c_float::MAX;
    let mut iter = 0;
    let mut hit = 0;
    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count {
            saveA[i as usize] = (*verts.add(i as usize)).iA;
            saveB[i as usize] = (*verts.add(i as usize)).iB;
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
        let d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&mut s);
        if c2Dot(d, d) < c_float::EPSILON * c_float::EPSILON {
            break;
        }
        let iA = c2Support(
            pA.verts.as_ptr(),
            pA.count,
            c2MulrvT(ax.r, c2Neg(d)),
        );
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = verts.add(s.count as usize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup = 0;
        for i in 0..save_count {
            if iA == saveA[i as usize] && iB == saveB[i as usize] {
                dup = 1;
                break;
            }
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }

    let mut a = C2v::default();
    let mut b = C2v::default();
    c2Witness(&mut s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > c_float::EPSILON {
            dist -= rA + rB;
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
        for i in 0..s.count {
            let v = verts.add(i as usize);
            (*cache).iA[i as usize] = (*v).iA;
            (*cache).iB[i as usize] = (*v).iB;
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

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: C2Aabb, B: C2Aabb) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: C2Aabb, B: C2Capsule) -> c_int {
    let distance = unsafe {
        c2GJK(
            std::ptr::addr_of!(A).cast(),
            C2_TYPE_AABB,
            std::ptr::null(),
            std::ptr::addr_of!(B).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (distance == 0.0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: C2Capsule, B: C2Capsule) -> c_int {
    let distance = unsafe {
        c2GJK(
            std::ptr::addr_of!(A).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::addr_of!(B).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (distance == 0.0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: C2Circle, B: C2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: C2Circle, B: C2Aabb) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: C2Circle, B: C2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = A.r + B.r;
    (d2 < r * r) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    A: *const c_void,
    typeA: C2Type,
    B: *const c_void,
    typeB: C2Type,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(*A.cast(), *B.cast()),
            C2_TYPE_AABB => c2CircletoAABB(*A.cast(), *B.cast()),
            C2_TYPE_CAPSULE => c2CircletoCapsule(*A.cast(), *B.cast()),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(*B.cast(), *A.cast()),
            C2_TYPE_AABB => c2AABBtoAABB(*A.cast(), *B.cast()),
            C2_TYPE_CAPSULE => c2AABBtoCapsule(*A.cast(), *B.cast()),
            _ => 0,
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCapsule(*B.cast(), *A.cast()),
            C2_TYPE_AABB => c2AABBtoCapsule(*B.cast(), *A.cast()),
            C2_TYPE_CAPSULE => c2CapsuletoCapsule(*A.cast(), *B.cast()),
            _ => 0,
        },
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn aabb(
    min_x: c_float,
    min_y: c_float,
    max_x: c_float,
    max_y: c_float,
) -> c_int {
    let aabb_in = C2Aabb {
        min: c2V(min_x, min_y),
        max: c2V(max_x, max_y),
    };
    let circle = C2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };
    let aabb = C2Aabb {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };
    let capsule = C2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    let mut result = unsafe {
        c2Collided(
            std::ptr::addr_of!(circle).cast(),
            C2_TYPE_CIRCLE,
            std::ptr::addr_of!(aabb_in).cast(),
            C2_TYPE_AABB,
        )
    };
    result += unsafe {
        c2Collided(
            std::ptr::addr_of!(aabb).cast(),
            C2_TYPE_AABB,
            std::ptr::addr_of!(aabb_in).cast(),
            C2_TYPE_AABB,
        )
    } << 1;
    result += unsafe {
        c2Collided(
            std::ptr::addr_of!(capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::addr_of!(aabb_in).cast(),
            C2_TYPE_AABB,
        )
    } << 2;
    result
}
