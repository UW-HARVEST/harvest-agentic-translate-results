#![allow(non_camel_case_types, non_snake_case, unsafe_op_in_unsafe_fn)]

use std::ffi::{c_float, c_int, c_void};

#[cfg(target_arch = "x86_64")]
use std::arch::asm;

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;
const FLT_EPSILON: c_float = 1.192_092_9e-7;

#[link(name = "m")]
unsafe extern "C" {
    fn sqrtf(value: c_float) -> c_float;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2GJKCache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: c_float,
    pub count: c_int,
}

#[inline]
unsafe fn simplex_vertex(s: *mut c2Simplex, index: usize) -> *mut c2sv {
    match index {
        0 => &mut (*s).a,
        1 => &mut (*s).b,
        2 => &mut (*s).c,
        3 => &mut (*s).d,
        _ => std::hint::unreachable_unchecked(),
    }
}

#[inline]
fn add_component(a: c_float, b: c_float) -> c_float {
    #[cfg(target_arch = "x86_64")]
    {
        // Match GCC's scalar-add operand order, including NaN payload selection.
        let mut result = b;
        unsafe {
            asm!(
                "addss {result}, {a}",
                result = inout(xmm_reg) result,
                a = in(xmm_reg) a,
                options(pure, nomem, nostack)
            );
        }
        result
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        b + a
    }
}

#[inline]
fn add_component_lhs(a: c_float, b: c_float) -> c_float {
    #[cfg(target_arch = "x86_64")]
    {
        let mut result = a;
        unsafe {
            asm!(
                "addss {result}, {b}",
                result = inout(xmm_reg) result,
                b = in(xmm_reg) b,
                options(pure, nomem, nostack)
            );
        }
        result
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        a + b
    }
}

#[inline]
fn mul_component(a: c_float, b: c_float, rhs_destination: bool) -> c_float {
    #[cfg(target_arch = "x86_64")]
    {
        let mut result = if rhs_destination { b } else { a };
        let source = if rhs_destination { a } else { b };
        unsafe {
            asm!(
                "mulss {result}, {source}",
                result = inout(xmm_reg) result,
                source = in(xmm_reg) source,
                options(pure, nomem, nostack)
            );
        }
        result
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = rhs_destination;
        a * b
    }
}

#[inline]
fn sub_component(a: c_float, b: c_float) -> c_float {
    #[cfg(target_arch = "x86_64")]
    {
        let mut result = a;
        unsafe {
            asm!(
                "subss {result}, {b}",
                result = inout(xmm_reg) result,
                b = in(xmm_reg) b,
                options(pure, nomem, nostack)
            );
        }
        result
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        a - b
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: c_float) -> c2v {
    a.x = mul_component(a.x, b, false);
    a.y = mul_component(a.y, b, false);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> c_float {
    let x = mul_component(a.x, b.x, false);
    let y = mul_component(a.y, b.y, true);
    add_component(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    *out.add(0) = (*bb).min;
    *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
    *out.add(2) = (*bb).max;
    *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    match type_ {
        C2_TYPE_CIRCLE => {
            let circle = shape.cast::<c2Circle>();
            (*p).radius = (*circle).r;
            (*p).count = 1;
            (*p).verts[0] = (*circle).p;
        }
        C2_TYPE_AABB => {
            let bb = shape.cast_mut().cast::<c2AABB>();
            (*p).radius = 0.0;
            (*p).count = 4;
            c2BBVerts((*p).verts.as_mut_ptr(), bb);
        }
        C2_TYPE_CAPSULE => {
            let capsule = shape.cast::<c2Capsule>();
            (*p).radius = (*capsule).r;
            (*p).count = 2;
            (*p).verts[0] = (*capsule).a;
            (*p).verts[1] = (*capsule).b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> c_float {
    unsafe { sqrtf(c2Dot(a, a)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> c_float {
    let first = mul_component(a.x, b.y, true);
    let second = mul_component(a.y, b.x, true);
    sub_component(first, second)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> c_float {
    match (*s).count {
        2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    let x = sub_component(mul_component(a.c, b.x, true), mul_component(a.s, b.y, true));
    let y = add_component_lhs(
        mul_component(a.s, b.x, false),
        mul_component(a.c, b.y, true),
    );
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x = add_component(a.x, b.x);
    a.y = add_component(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
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
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
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
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
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
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2Dot(*verts, d);
    let mut i = 1;
    while i < count {
        let dot = c2Dot(*verts.add(i as usize), d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
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
pub extern "C" fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
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
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    let x = add_component_lhs(
        mul_component(a.c, b.x, false),
        mul_component(a.s, b.y, true),
    );
    let y = add_component_lhs(
        mul_component(-a.s, b.x, false),
        mul_component(a.c, b.y, true),
    );
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
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
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    let mut s = c2Simplex::default();
    let s_ptr = &mut s as *mut c2Simplex;
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_was_good = ((*cache).count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i = 0;
            while i < (*cache).count {
                let iA = (*cache).iA[i as usize];
                let iB = (*cache).iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = simplex_vertex(s_ptr, i as usize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
                i += 1;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(s_ptr);
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
    let mut save_count;
    let mut d0 = c_float::MAX;
    let mut iter = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        let mut i = 0;
        while i < save_count {
            let v = simplex_vertex(s_ptr, i as usize);
            saveA[i as usize] = (*v).iA;
            saveB[i as usize] = (*v).iB;
            i += 1;
        }
        match s.count {
            2 => c22(s_ptr),
            3 => c23(s_ptr),
            _ => {}
        }
        if s.count == 3 {
            hit = 1;
            break;
        }
        let p = c2L(s_ptr);
        let d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(s_ptr);
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = simplex_vertex(s_ptr, s.count as usize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup = 0;
        let mut i = 0;
        while i < save_count {
            if iA == saveA[i as usize] && iB == saveB[i as usize] {
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
    let mut a = c2v::default();
    let mut b = c2v::default();
    c2Witness(s_ptr, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > FLT_EPSILON {
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
        (*cache).metric = c2GJKSimplexMetric(s_ptr);
        (*cache).count = s.count;
        let mut i = 0;
        while i < s.count {
            let v = simplex_vertex(s_ptr, i as usize);
            (*cache).iA[i as usize] = (*v).iA;
            (*cache).iB[i as usize] = (*v).iB;
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

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let distance = unsafe {
        c2GJK(
            (&A as *const c2AABB).cast(),
            C2_TYPE_AABB,
            std::ptr::null(),
            (&B as *const c2Capsule).cast(),
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
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let distance = unsafe {
        c2GJK(
            (&A as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            (&B as *const c2Capsule).cast(),
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
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
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
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
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
pub extern "C" fn aabb(min_x: c_float, min_y: c_float, max_x: c_float, max_y: c_float) -> c_int {
    let aabb_in = c2AABB {
        min: c2V(min_x, min_y),
        max: c2V(max_x, max_y),
    };
    let circle = c2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };
    let aabb = c2AABB {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };
    let capsule = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };
    let mut result = unsafe {
        c2Collided(
            (&circle as *const c2Circle).cast(),
            C2_TYPE_CIRCLE,
            (&aabb_in as *const c2AABB).cast(),
            C2_TYPE_AABB,
        )
    };
    result += unsafe {
        c2Collided(
            (&aabb as *const c2AABB).cast(),
            C2_TYPE_AABB,
            (&aabb_in as *const c2AABB).cast(),
            C2_TYPE_AABB,
        ) << 1
    };
    result += unsafe {
        c2Collided(
            (&capsule as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
            (&aabb_in as *const c2AABB).cast(),
            C2_TYPE_AABB,
        ) << 2
    };
    result
}
