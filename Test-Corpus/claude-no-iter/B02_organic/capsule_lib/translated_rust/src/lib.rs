#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_int;
use std::ptr;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2r {
    c: f32,
    s: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct c2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[derive(Clone, Copy, Default)]
struct c2Proxy {
    radius: f32,
    count: c_int,
    verts: [c2v; 8],
}

fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

unsafe fn c2MakeProxy(shape: *const core::ffi::c_void, ty: C2_TYPE, p: &mut c2Proxy) {
    match ty {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let c = unsafe { &*(shape as *const c2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE::C2_TYPE_AABB => {
            let bb = unsafe { &*(shape as *const c2AABB) };
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

#[derive(Clone, Copy, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[derive(Clone, Copy, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: c_int,
}

fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c22(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else {
        s.a.u = u;
        s.b.u = v;
        s.div = u + v;
        s.count = 2;
    }
}

fn c23(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
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
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.a = s.c;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.a.u = uAB;
        s.b.u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.a = s.b;
        s.b = s.c;
        s.a.u = uBC;
        s.b.u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.b = s.a;
        s.a = s.c;
        s.a.u = uCA;
        s.b.u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.a.u = uABC;
        s.b.u = vABC;
        s.c.u = wABC;
        s.div = uABC + vABC + wABC;
        s.count = 3;
    }
}

fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2D(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg(s.a.p),
        2 => {
            let ab = c2Sub(s.b.p, s.a.p);
            if c2Det2(ab, c2Neg(s.a.p)) > 0.0 {
                c2Skew(ab)
            } else {
                c2CCW90(ab)
            }
        }
        _ => c2V(0.0, 0.0),
    }
}

fn c2Support(verts: &[c2v], count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count {
        let dot = c2Dot(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

fn simplex_get(s: &c2Simplex, i: usize) -> c2sv {
    match i {
        0 => s.a,
        1 => s.b,
        2 => s.c,
        3 => s.d,
        _ => unreachable!(),
    }
}

fn simplex_set(s: &mut c2Simplex, i: usize, v: c2sv) {
    match i {
        0 => s.a = v,
        1 => s.b = v,
        2 => s.c = v,
        3 => s.d = v,
        _ => unreachable!(),
    }
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s.a.sA, den * s.a.u),
                c2Mulvs(s.b.sA, den * s.b.u),
            );
            *b = c2Add(
                c2Mulvs(s.a.sB, den * s.a.u),
                c2Mulvs(s.b.sB, den * s.b.u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s.a.sA, den * s.a.u),
                    c2Mulvs(s.b.sA, den * s.b.u),
                ),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s.a.sB, den * s.a.u),
                    c2Mulvs(s.b.sB, den * s.b.u),
                ),
                c2Mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(
            c2Mulvs(s.a.p, den * s.a.u),
            c2Mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

unsafe fn c2GJK(
    a_shape: *const core::ffi::c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const core::ffi::c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax: c2x = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx: c2x = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *bx_ptr }
    };
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    unsafe {
        c2MakeProxy(a_shape, typeA, &mut pA);
        c2MakeProxy(b_shape, typeB, &mut pB);
    }
    let mut s = c2Simplex::default();
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let iA = cache_ref.iA[i];
                let iB = cache_ref.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let mut v = simplex_get(&s, i);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
                simplex_set(&mut s, i, v);
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
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
    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int;
    let mut d0: f32 = 3.40282346638528859811704183484516925e+38;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = simplex_get(&s, i);
            saveA[i] = v.iA;
            saveB[i] = v.iB;
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
        let p = c2L(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&s);
        if c2Dot(d, d)
            < 1.19209289550781250000000000000000000e-7_f32
                * 1.19209289550781250000000000000000000e-7_f32
        {
            break;
        }
        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let s_count = s.count as usize;
        let mut v = simplex_get(&s, s_count);
        v.iA = iA;
        v.sA = sA;
        v.iB = iB;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        simplex_set(&mut s, s_count, v);
        let mut dup = 0;
        for i in 0..save_count as usize {
            if iA == saveA[i] && iB == saveB[i] {
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
    let mut a_out = c2v::default();
    let mut b_out = c2v::default();
    c2Witness(&s, &mut a_out, &mut b_out);
    let mut dist = c2Len(c2Sub(a_out, b_out));
    if hit != 0 {
        a_out = b_out;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7_f32 {
            dist -= rA + rB;
            let n = c2Norm(c2Sub(b_out, a_out));
            a_out = c2Add(a_out, c2Mulvs(n, rA));
            b_out = c2Sub(b_out, c2Mulvs(n, rB));
            if a_out.x == b_out.x && a_out.y == b_out.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a_out, b_out), 0.5);
            a_out = p;
            b_out = p;
            dist = 0.0;
        }
    }
    if !cache.is_null() {
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = c2GJKSimplexMetric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            let v = simplex_get(&s, i);
            cache_ref.iA[i] = v.iA;
            cache_ref.iB[i] = v.iB;
        }
        cache_ref.div = s.div;
    }
    if !outA.is_null() {
        unsafe { *outA = a_out };
    }
    if !outB.is_null() {
        unsafe { *outB = b_out };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }
    dist
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    (!(d0 | d1 | d2 | d3)) & 1
}

fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let a_ref = &A as *const c2AABB as *const core::ffi::c_void;
    let b_ref = &B as *const c2Capsule as *const core::ffi::c_void;
    let dist = unsafe {
        c2GJK(
            a_ref,
            C2_TYPE::C2_TYPE_AABB,
            ptr::null(),
            b_ref,
            C2_TYPE::C2_TYPE_CAPSULE,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if dist != 0.0 {
        0
    } else {
        1
    }
}

fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let a_ref = &A as *const c2Capsule as *const core::ffi::c_void;
    let b_ref = &B as *const c2Capsule as *const core::ffi::c_void;
    let dist = unsafe {
        c2GJK(
            a_ref,
            C2_TYPE::C2_TYPE_CAPSULE,
            ptr::null(),
            b_ref,
            C2_TYPE::C2_TYPE_CAPSULE,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if dist != 0.0 {
        0
    } else {
        1
    }
}

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
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

unsafe fn c2Collided(
    a: *const core::ffi::c_void,
    typeA: C2_TYPE,
    b: *const core::ffi::c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        C2_TYPE::C2_TYPE_CIRCLE => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => unsafe {
                c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle))
            },
            C2_TYPE::C2_TYPE_AABB => unsafe {
                c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB))
            },
            C2_TYPE::C2_TYPE_CAPSULE => unsafe {
                c2CircletoCapsule(*(a as *const c2Circle), *(b as *const c2Capsule))
            },
        },
        C2_TYPE::C2_TYPE_AABB => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => unsafe {
                c2CircletoAABB(*(b as *const c2Circle), *(a as *const c2AABB))
            },
            C2_TYPE::C2_TYPE_AABB => unsafe {
                c2AABBtoAABB(*(a as *const c2AABB), *(b as *const c2AABB))
            },
            C2_TYPE::C2_TYPE_CAPSULE => unsafe {
                c2AABBtoCapsule(*(a as *const c2AABB), *(b as *const c2Capsule))
            },
        },
        C2_TYPE::C2_TYPE_CAPSULE => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => unsafe {
                c2CircletoCapsule(*(b as *const c2Circle), *(a as *const c2Capsule))
            },
            C2_TYPE::C2_TYPE_AABB => unsafe {
                c2AABBtoCapsule(*(b as *const c2AABB), *(a as *const c2Capsule))
            },
            C2_TYPE::C2_TYPE_CAPSULE => unsafe {
                c2CapsuletoCapsule(*(a as *const c2Capsule), *(b as *const c2Capsule))
            },
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let capsule_in = c2Capsule {
        a: c2V(min_x, min_y),
        b: c2V(max_x, max_y),
        r,
    };

    let circle = c2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = c2AABB {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };

    let capsule_var = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    unsafe {
        result += c2Collided(
            &circle as *const c2Circle as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CIRCLE,
            &capsule_in as *const c2Capsule as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
        );

        result += c2Collided(
            &aabb as *const c2AABB as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_AABB,
            &capsule_in as *const c2Capsule as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
        ) << 1;

        result += c2Collided(
            &capsule_var as *const c2Capsule as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            &capsule_in as *const c2Capsule as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
        ) << 2;
    }

    result
}
