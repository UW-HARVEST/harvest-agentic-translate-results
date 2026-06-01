#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;
use std::os::raw::c_void;

#[repr(C)]
#[derive(Copy, Clone)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE = 0,
    C2_TYPE_CIRCLE = 1,
    C2_TYPE_AABB = 2,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

// ------- Helper functions exported as C symbols -------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
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
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
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
    let bb = &*bb;
    *out.add(0) = bb.min;
    *out.add(1) = c2V(bb.max.x, bb.min.y);
    *out.add(2) = bb.max;
    *out.add(3) = c2V(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
    let p = &mut *p;
    match type_ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let c = &*(shape as *const c2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE::C2_TYPE_AABB => {
            let bb = shape as *mut c2AABB;
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(p.verts.as_mut_ptr(), bb);
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    let s = &*s;
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
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
    let s = &mut *s;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s = &mut *s;
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
    let s = &*s;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(*verts, d);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let s = &*s;
    let den = 1.0_f32 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u));
            *b = c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u));
        }
        3 => {
            *a = c2Add(
                c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u)),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2Add(
                c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u)),
                c2Mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s = &*s;
    let den = 1.0_f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

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
) -> f32 {
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
    let mut pA: c2Proxy = std::mem::zeroed();
    let mut pB: c2Proxy = std::mem::zeroed();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    let mut s: c2Simplex = std::mem::zeroed();
    // verts = &s.a; treat the simplex's a/b/c/d as a flat array of 4 c2sv.
    // Index i corresponds to verts[i] in C.
    let verts_ptr: *mut c2sv = &mut s.a as *mut c2sv;
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = &mut *cache;
        let cache_was_good = if cache_ref.count != 0 { 1 } else { 0 };
        if cache_was_good != 0 {
            let mut i: c_int = 0;
            while i < cache_ref.count {
                let iA = cache_ref.iA[i as usize];
                let iB = cache_ref.iB[i as usize];
                let sA_ = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB_ = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut *verts_ptr.offset(i as isize);
                v.iA = iA;
                v.sA = sA_;
                v.iB = iB;
                v.sB = sB_;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
                i += 1;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
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
    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int;
    let mut d0: f32 = 3.402_823_5e38_f32; // FLT_MAX
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        let mut i: c_int = 0;
        while i < save_count {
            let v = &*verts_ptr.offset(i as isize);
            saveA[i as usize] = v.iA;
            saveB[i as usize] = v.iB;
            i += 1;
        }
        match s.count {
            1 => {}
            2 => {
                c22(&mut s);
            }
            3 => {
                c23(&mut s);
            }
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
        if c2Dot(d, d) < 1.192_092_9e-7_f32 * 1.192_092_9e-7_f32 {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA_ = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB_ = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = &mut *verts_ptr.offset(s.count as isize);
        v.iA = iA;
        v.sA = sA_;
        v.iB = iB;
        v.sB = sB_;
        v.p = c2Sub(v.sB, v.sA);
        let mut dup = 0;
        let mut i2: c_int = 0;
        while i2 < save_count {
            if iA == saveA[i2 as usize] && iB == saveB[i2 as usize] {
                dup = 1;
                break;
            }
            i2 += 1;
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }
    let mut a_v: c2v = c2V(0.0, 0.0);
    let mut b_v: c2v = c2V(0.0, 0.0);
    c2Witness(&mut s, &mut a_v, &mut b_v);
    let mut dist = c2Len(c2Sub(a_v, b_v));
    if hit != 0 {
        a_v = b_v;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.192_092_9e-7_f32 {
            dist -= rA + rB;
            let n = c2Norm(c2Sub(b_v, a_v));
            a_v = c2Add(a_v, c2Mulvs(n, rA));
            b_v = c2Sub(b_v, c2Mulvs(n, rB));
            if a_v.x == b_v.x && a_v.y == b_v.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a_v, b_v), 0.5);
            a_v = p;
            b_v = p;
            dist = 0.0;
        }
    }
    if !cache.is_null() {
        let cache_ref = &mut *cache;
        cache_ref.metric = c2GJKSimplexMetric(&mut s);
        cache_ref.count = s.count;
        let mut i: c_int = 0;
        while i < s.count {
            let v = &*verts_ptr.offset(i as isize);
            cache_ref.iA[i as usize] = v.iA;
            cache_ref.iB[i as usize] = v.iB;
            i += 1;
        }
        cache_ref.div = s.div;
    }
    if !outA.is_null() {
        *outA = a_v;
    }
    if !outB.is_null() {
        *outB = b_v;
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
    (!((d0 | d1 | d2 | d3) != 0)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let a_ptr = &A as *const c2AABB as *const c_void;
    let b_ptr = &B as *const c2Capsule as *const c_void;
    if c2GJK(
        a_ptr,
        C2_TYPE::C2_TYPE_AABB,
        std::ptr::null(),
        b_ptr,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        1,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    ) != 0.0
    {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let a_ptr = &A as *const c2Capsule as *const c_void;
    let b_ptr = &B as *const c2Capsule as *const c_void;
    if c2GJK(
        a_ptr,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        b_ptr,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        1,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    ) != 0.0
    {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
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
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        C2_TYPE::C2_TYPE_CIRCLE => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle)),
            C2_TYPE::C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
            C2_TYPE::C2_TYPE_CAPSULE => c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule)),
        },
        C2_TYPE::C2_TYPE_AABB => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),
            C2_TYPE::C2_TYPE_AABB => c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB)),
            C2_TYPE::C2_TYPE_CAPSULE => c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule)),
        },
        C2_TYPE::C2_TYPE_CAPSULE => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => c2CircletoCapsule(*(B as *const c2Circle), *(A as *const c2Capsule)),
            C2_TYPE::C2_TYPE_AABB => c2AABBtoCapsule(*(B as *const c2AABB), *(A as *const c2Capsule)),
            C2_TYPE::C2_TYPE_CAPSULE => c2CapsuletoCapsule(*(A as *const c2Capsule), *(B as *const c2Capsule)),
        },
    }
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: C2_TYPE,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
) -> *mut c_void {
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let circle = malloc(std::mem::size_of::<c2Circle>()) as *mut c2Circle;
            (*circle).p = c2V(a, b);
            (*circle).r = c;
            circle as *mut c_void
        }
        C2_TYPE::C2_TYPE_AABB => {
            let aabb = malloc(std::mem::size_of::<c2AABB>()) as *mut c2AABB;
            (*aabb).min = c2V(a, b);
            (*aabb).max = c2V(c, d);
            aabb as *mut c_void
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let capsule = malloc(std::mem::size_of::<c2Capsule>()) as *mut c2Capsule;
            (*capsule).a = c2V(a, b);
            (*capsule).b = c2V(c, d);
            (*capsule).r = e;
            capsule as *mut c_void
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_collide(
    type_a: C2_TYPE,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: C2_TYPE,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) -> c_int {
    let a_ptr = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b_ptr = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2Collided(a_ptr, type_a, b_ptr, type_b)
}
