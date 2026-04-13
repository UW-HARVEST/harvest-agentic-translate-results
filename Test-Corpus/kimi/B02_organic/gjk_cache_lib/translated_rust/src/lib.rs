use std::os::raw::{c_char, c_float, c_int, c_void};

#[repr(C)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
struct c2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c2v,
}

#[repr(C)]
struct c2GJKCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

struct c2Proxy {
    radius: c_float,
    count: c_int,
    verts: [c2v; 8],
}

#[derive(Clone, Copy)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: c_float,
    count: c_int,
}

fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

fn c2Mulvs(a: c2v, b: c_float) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2Dot(a: c2v, b: c2v) -> c_float {
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

fn c2BBVerts(out: &mut [c2v; 4], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

unsafe fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
    let p = &mut *p;
    match type_ {
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
            c2BBVerts(
                std::slice::from_raw_parts_mut(p.verts.as_mut_ptr(), 4)
                    .try_into()
                    .unwrap(),
                bb,
            );
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r.x;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

fn c2Len(a: c2v) -> c_float {
    (c2Dot(a, a)).sqrt()
}

fn c2Det2(a: c2v, b: c2v) -> c_float {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> c_float {
    match s.count {
        1 => 0.0,
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        _ => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
    }
}

fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2v {
        x: a.c * b.x - a.s * b.y,
        y: a.s * b.x + a.c * b.y,
    }
}

fn c2Add(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
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
    c2v {
        x: -a.x,
        y: -a.y,
    }
}

fn c2Skew(a: c2v) -> c2v {
    c2v {
        x: -a.y,
        y: a.x,
    }
}

fn c2CCW90(a: c2v) -> c2v {
    c2v {
        x: a.y,
        y: -a.x,
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

fn c2Support(verts: &[c2v], count: c_int, d: c2v) -> c_int {
    let count = count as usize;
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count {
        let dot = c2Dot(verts[i], d);
        if dot > dmax {
            imax = i as c_int;
            dmax = dot;
        }
    }
    imax
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
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

fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
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
    c2v {
        x: a.c * b.x + a.s * b.y,
        y: -a.s * b.x + a.c * b.y,
    }
}

unsafe fn c2GJK(
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
    let mut pA: c2Proxy = std::mem::zeroed();
    let mut pB: c2Proxy = std::mem::zeroed();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    let mut s: c2Simplex = std::mem::zeroed();
    let verts: *mut c2sv = &mut s.a;
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = &*cache;
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count {
                let iA = cache_ref.iA[i as usize];
                let iB = cache_ref.iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut *verts.add(i as usize);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric(&s);
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
    let mut save_count: c_int = 0;
    let mut d0: c_float = f32::MAX;
    let mut d1: c_float = f32::MAX;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            let v = &*verts.add(i as usize);
            saveA[i as usize] = v.iA;
            saveB[i as usize] = v.iB;
        }
        match s.count {
            1 => {}
            2 => c22(&mut s),
            _ => c23(&mut s),
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
        if c2Dot(d, d) < 1.1920929e-7 * 1.1920929e-7 {
            break;
        }
        let iA = c2Support(
            &pA.verts[..pA.count as usize],
            pA.count,
            c2MulrvT(ax.r, c2Neg(d)),
        );
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(
            &pB.verts[..pB.count as usize],
            pB.count,
            c2MulrvT(bx.r, d),
        );
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = &mut *verts.add(s.count as usize);
        v.iA = iA as c_int;
        v.sA = sA;
        v.iB = iB as c_int;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
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
    let mut a: c2v = std::mem::zeroed();
    let mut b: c2v = std::mem::zeroed();
    c2Witness(&s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.1920929e-7 {
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
        let cache_ref = &mut *cache;
        cache_ref.metric = c2GJKSimplexMetric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count {
            let v = &*verts.add(i as usize);
            cache_ref.iA[i as usize] = v.iA;
            cache_ref.iB[i as usize] = v.iB;
        }
        cache_ref.div = s.div;
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
pub unsafe extern "C" fn gjk_cache(
    reverse: c_char,
    a9: *mut c2v,
    b9: *mut c2v,
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
    let mut cache: c2GJKCache = std::mem::zeroed();
    cache.count = 0;
    let A = c2Circle {
        p: c2V(0.0, 0.0),
        r: 15.0,
    };
    let B = c2Capsule {
        a: c2V(100.0, -25.0),
        b: c2V(75.0, 100.0),
        r: c2V(10.0, 0.0),
    };
    let mut a0: c2v = std::mem::zeroed();
    let mut b0: c2v = std::mem::zeroed();
    let mut a: c2v = std::mem::zeroed();
    let mut b: c2v = std::mem::zeroed();
    let mut iterations: c_int = -1;
    let mut cached_iterations: c_int = -1;
    let _d0 = c2GJK(
        &A as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a0,
        &mut b0,
        1,
        &mut iterations,
        &mut cache,
    );
    let _d1 = c2GJK(
        &A as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
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
        r: c2V(b5, 0.0),
    };
    if reverse != 0 {
        c2GJK(
            &cap as *const _ as *const c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            &bb as *const _ as *const c_void,
            C2_TYPE::C2_TYPE_AABB,
            std::ptr::null(),
            &mut a,
            &mut b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    } else {
        c2GJK(
            &bb as *const _ as *const c_void,
            C2_TYPE::C2_TYPE_AABB,
            std::ptr::null(),
            &cap as *const _ as *const c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}
