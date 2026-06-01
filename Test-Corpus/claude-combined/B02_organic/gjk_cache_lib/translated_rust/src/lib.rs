#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;
use std::os::raw::c_char;

// ---------------- Types ----------------

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
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
    pub r: f32,
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
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
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

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
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
    pub div: f32,
    pub count: c_int,
}

// C2_TYPE enum is passed as an int by C convention
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------- Helper functions ----------------

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
    let bb_ref = &*bb;
    *out.add(0) = bb_ref.min;
    *out.add(1) = c2V(bb_ref.max.x, bb_ref.min.y);
    *out.add(2) = bb_ref.max;
    *out.add(3) = c2V(bb_ref.min.x, bb_ref.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(
    shape: *const core::ffi::c_void,
    type_: c_int,
    p: *mut c2Proxy,
) {
    match type_ {
        x if x == C2_TYPE_CIRCLE => {
            let c = shape as *const c2Circle;
            (*p).radius = (*c).r;
            (*p).count = 1;
            (*p).verts[0] = (*c).p;
        }
        x if x == C2_TYPE_AABB => {
            let bb = shape as *mut c2AABB;
            (*p).radius = 0.0;
            (*p).count = 4;
            c2BBVerts((*p).verts.as_mut_ptr(), bb);
        }
        x if x == C2_TYPE_CAPSULE => {
            let c = shape as *const c2Capsule;
            (*p).radius = (*c).r;
            (*p).count = 2;
            (*p).verts[0] = (*c).a;
            (*p).verts[1] = (*c).b;
        }
        _ => {}
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
    let s_ref = &*s;
    match s_ref.count {
        2 => c2Len(c2Sub(s_ref.b.p, s_ref.a.p)),
        3 => c2Det2(c2Sub(s_ref.b.p, s_ref.a.p), c2Sub(s_ref.c.p, s_ref.a.p)),
        _ => 0.0, // default and case 1
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
    let s_ref = &mut *s;
    let a = s_ref.a.p;
    let b = s_ref.b.p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s_ref.a.u = 1.0;
        s_ref.div = 1.0;
        s_ref.count = 1;
    } else if u <= 0.0 {
        s_ref.a = s_ref.b;
        s_ref.a.u = 1.0;
        s_ref.div = 1.0;
        s_ref.count = 1;
    } else {
        s_ref.a.u = u;
        s_ref.b.u = v;
        s_ref.div = u + v;
        s_ref.count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s_ref = &mut *s;
    let a = s_ref.a.p;
    let b = s_ref.b.p;
    let c = s_ref.c.p;
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
        s_ref.a.u = 1.0;
        s_ref.div = 1.0;
        s_ref.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s_ref.a = s_ref.b;
        s_ref.a.u = 1.0;
        s_ref.div = 1.0;
        s_ref.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s_ref.a = s_ref.c;
        s_ref.a.u = 1.0;
        s_ref.div = 1.0;
        s_ref.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s_ref.a.u = uAB;
        s_ref.b.u = vAB;
        s_ref.div = uAB + vAB;
        s_ref.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s_ref.a = s_ref.b;
        s_ref.b = s_ref.c;
        s_ref.a.u = uBC;
        s_ref.b.u = vBC;
        s_ref.div = uBC + vBC;
        s_ref.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s_ref.b = s_ref.a;
        s_ref.a = s_ref.c;
        s_ref.a.u = uCA;
        s_ref.b.u = vCA;
        s_ref.div = uCA + vCA;
        s_ref.count = 2;
    } else {
        s_ref.a.u = uABC;
        s_ref.b.u = vABC;
        s_ref.c.u = wABC;
        s_ref.div = uABC + vABC + wABC;
        s_ref.count = 3;
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
    let s_ref = &*s;
    match s_ref.count {
        1 => c2Neg(s_ref.a.p),
        2 => {
            let ab = c2Sub(s_ref.b.p, s_ref.a.p);
            if c2Det2(ab, c2Neg(s_ref.a.p)) > 0.0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let s_ref = &*s;
    let den = 1.0f32 / s_ref.div;
    match s_ref.count {
        1 => {
            *a = s_ref.a.sA;
            *b = s_ref.a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s_ref.a.sA, den * s_ref.a.u),
                c2Mulvs(s_ref.b.sA, den * s_ref.b.u),
            );
            *b = c2Add(
                c2Mulvs(s_ref.a.sB, den * s_ref.a.u),
                c2Mulvs(s_ref.b.sB, den * s_ref.b.u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s_ref.a.sA, den * s_ref.a.u),
                    c2Mulvs(s_ref.b.sA, den * s_ref.b.u),
                ),
                c2Mulvs(s_ref.c.sA, den * s_ref.c.u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s_ref.a.sB, den * s_ref.a.u),
                    c2Mulvs(s_ref.b.sB, den * s_ref.b.u),
                ),
                c2Mulvs(s_ref.c.sB, den * s_ref.c.u),
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
    let s_ref = &*s;
    let den = 1.0f32 / s_ref.div;
    match s_ref.count {
        1 => s_ref.a.p,
        2 => c2Add(
            c2Mulvs(s_ref.a.p, den * s_ref.a.u),
            c2Mulvs(s_ref.b.p, den * s_ref.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const core::ffi::c_void,
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const core::ffi::c_void,
    typeB: c_int,
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

    let mut pA: c2Proxy = c2Proxy::default();
    let mut pB: c2Proxy = c2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);

    let mut s: c2Simplex = c2Simplex::default();
    // verts = &s.a in C; treat as pointer to array of c2sv
    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_was_good = (*cache).count != 0;
        if cache_was_good {
            for i in 0..(*cache).count {
                let iA = (*cache).iA[i as usize];
                let iB = (*cache).iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = simplex_vert_mut(&mut s, i);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(&mut s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8f32) {
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
    let mut save_count: c_int = 0;
    let mut d0: f32 = 3.402_823_466_385_288_6e+38_f32;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            let v = simplex_vert(&s, i);
            saveA[i as usize] = v.iA;
            saveB[i as usize] = v.iB;
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
            < 1.192_092_895_507_812_5e-7_f32 * 1.192_092_895_507_812_5e-7_f32
        {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        {
            let s_count = s.count;
            let v = simplex_vert_mut(&mut s, s_count);
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub(v.sB, v.sA);
        }
        let mut dup: c_int = 0;
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
    let _ = save_count; // suppress "unused"
    let mut a: c2v = c2v::default();
    let mut b: c2v = c2v::default();
    c2Witness(&mut s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.192_092_895_507_812_5e-7_f32 {
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
            let v = simplex_vert(&s, i);
            (*cache).iA[i as usize] = v.iA;
            (*cache).iB[i as usize] = v.iB;
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

// Helpers for indexing the simplex like the C code does (verts = &s.a)
#[inline]
fn simplex_vert(s: &c2Simplex, i: c_int) -> &c2sv {
    match i {
        0 => &s.a,
        1 => &s.b,
        2 => &s.c,
        3 => &s.d,
        _ => unreachable!(),
    }
}

#[inline]
fn simplex_vert_mut(s: &mut c2Simplex, i: c_int) -> &mut c2sv {
    match i {
        0 => &mut s.a,
        1 => &mut s.b,
        2 => &mut s.c,
        3 => &mut s.d,
        _ => unreachable!(),
    }
}

// ---------------- Public exported function ----------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk_cache(
    reverse: c_char,
    _a9: *mut c2v,
    _b9: *mut c2v,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    let mut cache: c2GJKCache = c2GJKCache::default();
    cache.count = 0;

    let A = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 15.0,
    };

    let B = c2Capsule {
        a: c2v { x: 100.0, y: -25.0 },
        b: c2v { x: 75.0, y: 100.0 },
        r: 10.0,
    };

    let mut a0: c2v = c2v::default();
    let mut b0: c2v = c2v::default();
    let mut a: c2v = c2v::default();
    let mut b: c2v = c2v::default();

    let mut iterations: c_int = -1;
    let mut cached_iterations: c_int = -1;
    let _d0 = c2GJK(
        &A as *const _ as *const core::ffi::c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const _ as *const core::ffi::c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a0,
        &mut b0,
        1,
        &mut iterations,
        &mut cache,
    );
    let _d1 = c2GJK(
        &A as *const _ as *const core::ffi::c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const _ as *const core::ffi::c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a,
        &mut b,
        1,
        &mut cached_iterations,
        &mut cache,
    );

    let mut bb: c2AABB = c2AABB::default();
    bb.min = c2V(a1, a2);
    bb.max = c2V(a3, a4);

    let mut cap: c2Capsule = c2Capsule::default();
    cap.a = c2V(b1, b2);
    cap.b = c2V(b3, b4);
    cap.r = b5;

    if reverse != 0 {
        c2GJK(
            &cap as *const _ as *const core::ffi::c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &bb as *const _ as *const core::ffi::c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &mut a,
            &mut b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    } else {
        c2GJK(
            &bb as *const _ as *const core::ffi::c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &cap as *const _ as *const core::ffi::c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}
