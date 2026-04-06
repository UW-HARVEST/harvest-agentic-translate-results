#![allow(non_camel_case_types, non_snake_case)]

use std::os::raw::c_int;

// ── Types ──

#[repr(C)]
#[derive(Clone, Copy)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Proxy {
    radius: f32,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: c_int,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

// ── Helper functions ──

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

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
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

fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
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

fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

// ── Proxy ──

fn c2BBVerts(out: &mut [c2v; 8], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

fn c2MakeProxy(shape: *const u8, typ: C2_TYPE, p: &mut c2Proxy) {
    unsafe {
        match typ {
            C2_TYPE::C2_TYPE_CIRCLE => {
                let c = &*(shape as *const c2Circle);
                p.radius = c.r;
                p.count = 1;
                p.verts[0] = c.p;
            }
            C2_TYPE::C2_TYPE_AABB => {
                let bb = &*(shape as *const c2AABB);
                p.radius = 0.0;
                p.count = 4;
                c2BBVerts(&mut p.verts, bb);
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
}

// ── Simplex operations ──

fn c2GJKSimplexMetric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
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

fn c2Support(verts: &[c2v; 8], count: c_int, d: c2v) -> c_int {
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

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
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

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

// ── GJK ──

fn c2GJK(
    a_shape: *const u8,
    type_a: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const u8,
    type_b: C2_TYPE,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *bx_ptr }
    };

    let mut pA = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2V(0.0, 0.0); 8],
    };
    let mut pB = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2V(0.0, 0.0); 8],
    };
    c2MakeProxy(a_shape, type_a, &mut pA);
    c2MakeProxy(b_shape, type_b, &mut pB);

    let zero_sv = c2sv {
        sA: c2V(0.0, 0.0),
        sB: c2V(0.0, 0.0),
        p: c2V(0.0, 0.0),
        u: 0.0,
        iA: 0,
        iB: 0,
    };
    let mut s = c2Simplex {
        a: zero_sv,
        b: zero_sv,
        c: zero_sv,
        d: zero_sv,
        div: 0.0,
        count: 0,
    };

    let verts_ptr: *mut c2sv = &mut s.a as *mut c2sv;

    let mut cache_was_read = false;
    if !cache.is_null() {
        unsafe {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                for i in 0..(*cache).count {
                    let iA = (*cache).iA[i as usize];
                    let iB = (*cache).iB[i as usize];
                    let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                    let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                    let v = &mut *verts_ptr.add(i as usize);
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
                let metric = c2GJKSimplexMetric(&s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                    cache_was_read = true;
                }
            }
        }
    }

    if !cache_was_read {
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA = [0i32; 3];
    let mut saveB = [0i32; 3];
    let mut save_count: c_int;
    let mut d0: f32 = f32::MAX;
    let mut d1: f32 = f32::MAX;
    let mut iter: c_int = 0;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            unsafe {
                saveA[i as usize] = (*verts_ptr.add(i as usize)).iA;
                saveB[i as usize] = (*verts_ptr.add(i as usize)).iB;
            }
        }

        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }

        if s.count == 3 {
            hit = true;
            break;
        }

        let p = c2L(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2D(&s);
        if c2Dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }

        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);

        unsafe {
            let v = &mut *verts_ptr.add(s.count as usize);
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub(v.sB, v.sA);
        }

        let mut dup = false;
        for i in 0..save_count {
            if iA == saveA[i as usize] && iB == saveB[i as usize] {
                dup = true;
                break;
            }
        }
        if dup {
            break;
        }

        s.count += 1;
        iter += 1;
    }

    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    c2Witness(&s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));

    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > f32::EPSILON {
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
        unsafe {
            (*cache).metric = c2GJKSimplexMetric(&s);
            (*cache).count = s.count;
            for i in 0..s.count {
                let v = &*verts_ptr.add(i as usize);
                (*cache).iA[i as usize] = v.iA;
                (*cache).iB[i as usize] = v.iB;
            }
            (*cache).div = s.div;
        }
    }

    if !out_a.is_null() {
        unsafe { *out_a = a; }
    }
    if !out_b.is_null() {
        unsafe { *out_b = b; }
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter; }
    }

    dist
}

// ── Collision tests ──

fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2AABBtoCapsule(a: c2AABB, b: c2Capsule) -> c_int {
    if c2GJK(
        &a as *const c2AABB as *const u8,
        C2_TYPE::C2_TYPE_AABB,
        core::ptr::null(),
        &b as *const c2Capsule as *const u8,
        C2_TYPE::C2_TYPE_CAPSULE,
        core::ptr::null(),
        core::ptr::null_mut(),
        core::ptr::null_mut(),
        1,
        core::ptr::null_mut(),
        core::ptr::null_mut(),
    ) != 0.0
    {
        return 0;
    }
    1
}

fn c2CapsuletoCapsule(a: c2Capsule, b: c2Capsule) -> c_int {
    if c2GJK(
        &a as *const c2Capsule as *const u8,
        C2_TYPE::C2_TYPE_CAPSULE,
        core::ptr::null(),
        &b as *const c2Capsule as *const u8,
        C2_TYPE::C2_TYPE_CAPSULE,
        core::ptr::null(),
        core::ptr::null_mut(),
        core::ptr::null_mut(),
        1,
        core::ptr::null_mut(),
        core::ptr::null_mut(),
    ) != 0.0
    {
        return 0;
    }
    1
}

fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2CircletoCapsule(a: c2Circle, b: c2Capsule) -> c_int {
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

fn c2Collided(a: *const u8, type_a: C2_TYPE, b: *const u8, type_b: C2_TYPE) -> c_int {
    unsafe {
        match type_a {
            C2_TYPE::C2_TYPE_CIRCLE => match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    c2CircletoCircle(*(a as *const c2Circle), *(b as *const c2Circle))
                }
                C2_TYPE::C2_TYPE_AABB => {
                    c2CircletoAABB(*(a as *const c2Circle), *(b as *const c2AABB))
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    c2CircletoCapsule(*(a as *const c2Circle), *(b as *const c2Capsule))
                }
            },
            C2_TYPE::C2_TYPE_AABB => match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    c2CircletoAABB(*(b as *const c2Circle), *(a as *const c2AABB))
                }
                C2_TYPE::C2_TYPE_AABB => {
                    c2AABBtoAABB(*(a as *const c2AABB), *(b as *const c2AABB))
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    c2AABBtoCapsule(*(a as *const c2AABB), *(b as *const c2Capsule))
                }
            },
            C2_TYPE::C2_TYPE_CAPSULE => match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    c2CircletoCapsule(*(b as *const c2Circle), *(a as *const c2Capsule))
                }
                C2_TYPE::C2_TYPE_AABB => {
                    c2AABBtoCapsule(*(b as *const c2AABB), *(a as *const c2Capsule))
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    c2CapsuletoCapsule(*(a as *const c2Capsule), *(b as *const c2Capsule))
                }
            },
        }
    }
}

// ── Public API ──

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

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

    result += c2Collided(
        &circle as *const c2Circle as *const u8,
        C2_TYPE::C2_TYPE_CIRCLE,
        &circle_in as *const c2Circle as *const u8,
        C2_TYPE::C2_TYPE_CIRCLE,
    );

    result += c2Collided(
        &aabb as *const c2AABB as *const u8,
        C2_TYPE::C2_TYPE_AABB,
        &circle_in as *const c2Circle as *const u8,
        C2_TYPE::C2_TYPE_CIRCLE,
    ) << 1;

    result += c2Collided(
        &capsule as *const c2Capsule as *const u8,
        C2_TYPE::C2_TYPE_CAPSULE,
        &circle_in as *const c2Circle as *const u8,
        C2_TYPE::C2_TYPE_CIRCLE,
    ) << 2;

    result
}
