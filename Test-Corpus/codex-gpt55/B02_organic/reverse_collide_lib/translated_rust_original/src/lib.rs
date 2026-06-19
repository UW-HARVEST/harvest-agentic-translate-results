#![allow(non_snake_case)]

use std::ffi::{c_float, c_int, c_void};

const C2_TYPE_CIRCLE: C2Type = 0;
const C2_TYPE_AABB: C2Type = 1;
const C2_TYPE_CAPSULE: C2Type = 2;

type C2Type = c_int;

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
    radius: c_float,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: c_float,
    count: c_int,
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
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2Type, p: *mut c2Proxy) {
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
        let u_ab = c2Dot(b, c2Sub(b, a));
        let v_ab = c2Dot(a, c2Sub(a, b));
        let u_bc = c2Dot(c, c2Sub(c, b));
        let v_bc = c2Dot(b, c2Sub(b, c));
        let u_ca = c2Dot(a, c2Sub(a, c));
        let v_ca = c2Dot(c, c2Sub(c, a));
        let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
        let u_abc = c2Det2(b, c) * area;
        let v_abc = c2Det2(c, a) * area;
        let w_abc = c2Det2(a, b) * area;
        if v_ab <= 0.0 && u_ca <= 0.0 {
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u_ab <= 0.0 && v_bc <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u_bc <= 0.0 && v_ca <= 0.0 {
            (*s).a = (*s).c;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
            (*s).a.u = u_ab;
            (*s).b.u = v_ab;
            (*s).div = u_ab + v_ab;
            (*s).count = 2;
        } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
            (*s).a = (*s).b;
            (*s).b = (*s).c;
            (*s).a.u = u_bc;
            (*s).b.u = v_bc;
            (*s).div = u_bc + v_bc;
            (*s).count = 2;
        } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
            (*s).b = (*s).a;
            (*s).a = (*s).c;
            (*s).a.u = u_ca;
            (*s).b.u = v_ca;
            (*s).div = u_ca + v_ca;
            (*s).count = 2;
        } else {
            (*s).a.u = u_abc;
            (*s).b.u = v_abc;
            (*s).c.u = w_abc;
            (*s).div = u_abc + v_abc + w_abc;
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
    a_ptr: *const c_void,
    type_a: C2Type,
    ax_ptr: *const c2x,
    b_ptr: *const c_void,
    type_b: C2Type,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
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
        let mut p_a = c2Proxy::default();
        let mut p_b = c2Proxy::default();
        c2MakeProxy(a_ptr, type_a, &mut p_a);
        c2MakeProxy(b_ptr, type_b, &mut p_b);

        let mut s = c2Simplex::default();
        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = ((*cache).count != 0) as c_int;
            if cache_was_good != 0 {
                for i in 0..(*cache).count {
                    let i_a = (*cache).iA[i as usize];
                    let i_b = (*cache).iB[i as usize];
                    let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
                    let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
                    let v = simplex_vert_mut(&mut s, i);
                    v.iA = i_a;
                    v.sA = s_a;
                    v.iB = i_b;
                    v.sB = s_b;
                    v.p = c2Sub(v.sB, v.sA);
                    v.u = 0.0;
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
            s.a.sA = c2Mulxv(ax, p_a.verts[0]);
            s.a.sB = c2Mulxv(bx, p_b.verts[0]);
            s.a.p = c2Sub(s.a.sB, s.a.sA);
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        let mut save_a = [0; 3];
        let mut save_b = [0; 3];
        let mut save_count: c_int;
        let mut d0 = c_float::MAX;
        let mut d1;
        let mut iter = 0;
        let mut hit = 0;
        while iter < 20 {
            save_count = s.count;
            for i in 0..save_count {
                let v = simplex_vert(&s, i);
                save_a[i as usize] = v.iA;
                save_b[i as usize] = v.iB;
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
            let i_a = c2Support(p_a.verts.as_ptr(), p_a.count, c2MulrvT(ax.r, c2Neg(d)));
            let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
            let i_b = c2Support(p_b.verts.as_ptr(), p_b.count, c2MulrvT(bx.r, d));
            let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
            let next = s.count;
            let v = simplex_vert_mut(&mut s, next);
            v.iA = i_a;
            v.sA = s_a;
            v.iB = i_b;
            v.sB = s_b;
            v.p = c2Sub(v.sB, v.sA);
            let mut dup = 0;
            for i in 0..save_count {
                if i_a == save_a[i as usize] && i_b == save_b[i as usize] {
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
            let r_a = p_a.radius;
            let r_b = p_b.radius;
            if dist > r_a + r_b && dist > 1.1920928955078125e-7_f32 {
                dist -= r_a + r_b;
                let n = c2Norm(c2Sub(b, a));
                a = c2Add(a, c2Mulvs(n, r_a));
                b = c2Sub(b, c2Mulvs(n, r_b));
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
        if !out_a.is_null() {
            *out_a = a;
        }
        if !out_b.is_null() {
            *out_b = b;
        }
        if !iterations.is_null() {
            *iterations = iter;
        }
        dist
    }
}

fn simplex_vert(s: &c2Simplex, i: c_int) -> &c2sv {
    match i {
        0 => &s.a,
        1 => &s.b,
        2 => &s.c,
        _ => &s.d,
    }
}

fn simplex_vert_mut(s: &mut c2Simplex, i: c_int) -> &mut c2sv {
    match i {
        0 => &mut s.a,
        1 => &mut s.b,
        2 => &mut s.c,
        _ => &mut s.d,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(a: c2AABB, b: c2Capsule) -> c_int {
    unsafe {
        if c2GJK(
            &a as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) != 0.0
        {
            0
        } else {
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(a: c2Capsule, b: c2Capsule) -> c_int {
    unsafe {
        if c2GJK(
            &a as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) != 0.0
        {
            0
        } else {
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: c2Circle, b: c2Capsule) -> c_int {
    let n = c2Sub(b.b, b.a);
    let ap = c2Sub(a.p, b.a);
    let da = c2Dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(a.p, b.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    a: *const c_void,
    type_a: C2Type,
    b: *const c_void,
    type_b: C2Type,
) -> c_int {
    unsafe {
        match type_a {
            C2_TYPE_CIRCLE => match type_b {
                C2_TYPE_CIRCLE => c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle)),
                C2_TYPE_AABB => c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    c2CircletoCapsule(*(a as *const c2Circle), *(b as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_AABB => match type_b {
                C2_TYPE_CIRCLE => c2CircletoAABB(*(b as *const c2Circle), *(a as *const c2AABB)),
                C2_TYPE_AABB => c2AABBtoAABB(*(a as *const c2AABB), *(b as *const c2AABB)),
                C2_TYPE_CAPSULE => c2AABBtoCapsule(*(a as *const c2AABB), *(b as *const c2Capsule)),
                _ => 0,
            },
            C2_TYPE_CAPSULE => match type_b {
                C2_TYPE_CIRCLE => {
                    c2CircletoCapsule(*(b as *const c2Circle), *(a as *const c2Capsule))
                }
                C2_TYPE_AABB => c2AABBtoCapsule(*(b as *const c2AABB), *(a as *const c2Capsule)),
                C2_TYPE_CAPSULE => {
                    c2CapsuletoCapsule(*(a as *const c2Capsule), *(b as *const c2Capsule))
                }
                _ => 0,
            },
            _ => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: c_float, y: c_float, r: c_float) -> c_int {
    let mut result = 0;

    let circle_in = c2Circle {
        p: c2V(x, y),
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

    let capsule = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    unsafe {
        result += c2Collided(
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        );
        result += c2Collided(
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        ) << 1;
        result += c2Collided(
            &capsule as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        ) << 2;
    }

    result
}
