#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_float, c_int, c_void};
use std::ptr;

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2GJKCache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: c_float,
    pub count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: c_float) -> c2v {
    a.x *= b;
    a.y *= b;
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
    a.x * b.x + a.y * b.y
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
    unsafe {
        *out.add(0) = (*bb).min;
        *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
        *out.add(2) = (*bb).max;
        *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
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
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> c_float {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> c_float {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> c_float {
    unsafe {
        match (*s).count {
            2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
            3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

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
            (*s).div = u + v;
            (*s).count = 2;
        }
    }
}

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
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax = 0;
        let mut dmax = c2Dot(*verts.add(0), d);
        for i in 1..count {
            let dot = c2Dot(*verts.add(i as usize), d);
            if dot > dmax {
                imax = i;
                dmax = dot;
            }
        }
        imax
    }
}

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
    unsafe {
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
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
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
    unsafe {
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
        let verts = ptr::addr_of_mut!(s.a);
        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = ((*cache).count != 0) as c_int;
            if cache_was_good != 0 {
                for i in 0..(*cache).count {
                    let idx = i as usize;
                    let iA = (*cache).iA[idx];
                    let iB = (*cache).iB[idx];
                    let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                    let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                    let v = verts.add(idx);
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
        let mut saveA = [0 as c_int; 3];
        let mut saveB = [0 as c_int; 3];
        let mut save_count: c_int;
        let mut d0 = c_float::MAX;
        let mut d1: c_float;
        let mut iter: c_int = 0;
        let mut hit = 0;
        while iter < 20 {
            save_count = s.count;
            for i in 0..save_count {
                let idx = i as usize;
                saveA[idx] = (*verts.add(idx)).iA;
                saveB[idx] = (*verts.add(idx)).iB;
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
            if c2Dot(d, d)
                < 1.1920928955078125e-7_f32 * 1.1920928955078125e-7_f32
            {
                break;
            }
            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
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
                let idx = i as usize;
                if iA == saveA[idx] && iB == saveB[idx] {
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
            if dist > rA + rB && dist > 1.1920928955078125e-7_f32 {
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
                let idx = i as usize;
                let v = verts.add(idx);
                (*cache).iA[idx] = (*v).iA;
                (*cache).iB[idx] = (*v).iB;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk_cache(
    reverse: c_char,
    _a9: *mut c2v,
    _b9: *mut c2v,
    a1: c_float,
    a2: c_float,
    a3: c_float,
    a4: c_float,
    b1: c_float,
    b2: c_float,
    b3: c_float,
    b4: c_float,
    b5: c_float,
) {
    unsafe {
        let mut cache = c2GJKCache {
            count: 0,
            ..Default::default()
        };
        let A = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 15.0,
        };
        let B = c2Capsule {
            a: c2v { x: 100.0, y: -25.0 },
            b: c2v { x: 75.0, y: 100.0 },
            r: 10.0,
        };
        let mut a0 = c2v::default();
        let mut b0 = c2v::default();
        let mut a = c2v::default();
        let mut b = c2v::default();
        let mut iterations: c_int = -1;
        let mut cached_iterations: c_int = -1;
        let _d0 = c2GJK(
            ptr::addr_of!(A).cast(),
            C2_TYPE_CIRCLE,
            ptr::null(),
            ptr::addr_of!(B).cast(),
            C2_TYPE_CAPSULE,
            ptr::null(),
            &mut a0,
            &mut b0,
            1,
            &mut iterations,
            &mut cache,
        );
        let _d1 = c2GJK(
            ptr::addr_of!(A).cast(),
            C2_TYPE_CIRCLE,
            ptr::null(),
            ptr::addr_of!(B).cast(),
            C2_TYPE_CAPSULE,
            ptr::null(),
            &mut a,
            &mut b,
            1,
            &mut cached_iterations,
            &mut cache,
        );
        let bb = c2AABB {
            min: c2V(a1, a2),
            max: c2V(a3, a4),
        };
        let cap = c2Capsule {
            a: c2V(b1, b2),
            b: c2V(b3, b4),
            r: b5,
        };
        if reverse != 0 {
            c2GJK(
                ptr::addr_of!(cap).cast(),
                C2_TYPE_CAPSULE,
                ptr::null(),
                ptr::addr_of!(bb).cast(),
                C2_TYPE_AABB,
                ptr::null(),
                &mut a,
                &mut b,
                1,
                ptr::null_mut(),
                ptr::null_mut(),
            );
        } else {
            c2GJK(
                ptr::addr_of!(bb).cast(),
                C2_TYPE_AABB,
                ptr::null(),
                ptr::addr_of!(cap).cast(),
                C2_TYPE_CAPSULE,
                ptr::null(),
                &mut a,
                &mut b,
                1,
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
    }
}
