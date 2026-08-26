#![allow(non_camel_case_types, dead_code, unused_variables)]
use std::os::raw::c_char;

const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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
struct c2Circle {
    p: c2v,
    r: f32,
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
    r: f32,
}

#[repr(C)]
struct c2GJKCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
}

#[derive(Clone, Copy)]
#[repr(i32)]
enum C2Type {
    Circle = 0,
    AABB = 1,
    Capsule = 2,
}

#[repr(C)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2sv {
    s_a: c2v,
    s_b: c2v,
    p: c2v,
    u: f32,
    i_a: i32,
    i_b: i32,
}

#[repr(C)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    _d: c2sv,
    div: f32,
    count: i32,
}

fn c2v_new(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2_mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2_maxv(a: c2v, b: c2v) -> c2v {
    c2v_new(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

fn c2_minv(a: c2v, b: c2v) -> c2v {
    c2v_new(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

fn c2_clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2_dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_rot_identity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

fn c2x_identity() -> c2x {
    c2x { p: c2v_new(0.0, 0.0), r: c2_rot_identity() }
}

fn c2_bb_verts(out: &mut [c2v; 8], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2v_new(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v_new(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: *const u8, typ: C2Type, p: &mut c2Proxy) {
    match typ {
        C2Type::Circle => unsafe {
            let c = &*(shape as *const c2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        },
        C2Type::AABB => unsafe {
            let bb = &*(shape as *const c2AABB);
            p.radius = 0.0;
            p.count = 4;
            c2_bb_verts(&mut p.verts, bb);
        },
        C2Type::Capsule => unsafe {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        },
    }
}

fn c2_len(a: c2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2_gjk_simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2_mulrv(a: c2r, b: c2v) -> c2v {
    c2v_new(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_mulxv(a: c2x, b: c2v) -> c2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

fn c22_inner(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = c2_dot(b, c2_sub(b, a));
    let v = c2_dot(a, c2_sub(a, b));
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

fn c23_inner(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
    let u_ab = c2_dot(b, c2_sub(b, a));
    let v_ab = c2_dot(a, c2_sub(a, b));
    let u_bc = c2_dot(c, c2_sub(c, b));
    let v_bc = c2_dot(b, c2_sub(b, c));
    let u_ca = c2_dot(a, c2_sub(a, c));
    let v_ca = c2_dot(c, c2_sub(c, a));
    let area = c2_det2(c2_sub(b, a), c2_sub(c, a));
    let u_abc = c2_det2(b, c) * area;
    let v_abc = c2_det2(c, a) * area;
    let w_abc = c2_det2(a, b) * area;
    if v_ab <= 0.0 && u_ca <= 0.0 {
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u_ab <= 0.0 && v_bc <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u_bc <= 0.0 && v_ca <= 0.0 {
        s.a = s.c;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
        s.a.u = u_ab;
        s.b.u = v_ab;
        s.div = u_ab + v_ab;
        s.count = 2;
    } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
        s.a = s.b;
        s.b = s.c;
        s.a.u = u_bc;
        s.b.u = v_bc;
        s.div = u_bc + v_bc;
        s.count = 2;
    } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
        s.b = s.a;
        s.a = s.c;
        s.a.u = u_ca;
        s.b.u = v_ca;
        s.div = u_ca + v_ca;
        s.count = 2;
    } else {
        s.a.u = u_abc;
        s.b.u = v_abc;
        s.c.u = w_abc;
        s.div = u_abc + v_abc + w_abc;
        s.count = 3;
    }
}

fn c2_neg(a: c2v) -> c2v {
    c2v_new(-a.x, -a.y)
}

fn c2_skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

fn c2_ccw90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2_d(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2_neg(s.a.p),
        2 => {
            let ab = c2_sub(s.b.p, s.a.p);
            if c2_det2(ab, c2_neg(s.a.p)) > 0.0 {
                c2_skew(ab)
            } else {
                c2_ccw90(ab)
            }
        }
        _ => c2v_new(0.0, 0.0),
    }
}

fn c2_support(verts: &[c2v; 8], count: i32, d: c2v) -> i32 {
    let mut imax = 0;
    let mut dmax = c2_dot(verts[0], d);
    for i in 1..count {
        let dot = c2_dot(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

fn c2_witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => {
            *a = s.a.s_a;
            *b = s.a.s_b;
        }
        2 => {
            *a = c2_add(c2_mulvs(s.a.s_a, den * s.a.u), c2_mulvs(s.b.s_a, den * s.b.u));
            *b = c2_add(c2_mulvs(s.a.s_b, den * s.a.u), c2_mulvs(s.b.s_b, den * s.b.u));
        }
        3 => {
            *a = c2_add(c2_add(c2_mulvs(s.a.s_a, den * s.a.u), c2_mulvs(s.b.s_a, den * s.b.u)), c2_mulvs(s.c.s_a, den * s.c.u));
            *b = c2_add(c2_add(c2_mulvs(s.a.s_b, den * s.a.u), c2_mulvs(s.b.s_b, den * s.b.u)), c2_mulvs(s.c.s_b, den * s.c.u));
        }
        _ => {
            *a = c2v_new(0.0, 0.0);
            *b = c2v_new(0.0, 0.0);
        }
    }
}

fn c2_norm(a: c2v) -> c2v {
    c2_mulvs(a, 1.0 / c2_len(a))
}

fn c2_l(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2v_new(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: c2r, b: c2v) -> c2v {
    c2v_new(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_gjk(
    a_shape: *const u8, type_a: C2Type, ax_ptr: *const c2x,
    b_shape: *const u8, type_b: C2Type, bx_ptr: *const c2x,
    out_a: *mut c2v, out_b: *mut c2v,
    use_radius: i32, iterations: *mut i32, cache: *mut c2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() { c2x_identity() } else { unsafe { *ax_ptr } };
    let bx = if bx_ptr.is_null() { c2x_identity() } else { unsafe { *bx_ptr } };

    let mut p_a = c2Proxy { radius: 0.0, count: 0, verts: [c2v_new(0.0, 0.0); 8] };
    let mut p_b = c2Proxy { radius: 0.0, count: 0, verts: [c2v_new(0.0, 0.0); 8] };
    c2_make_proxy(a_shape, type_a, &mut p_a);
    c2_make_proxy(b_shape, type_b, &mut p_b);

    let zero_sv = c2sv { s_a: c2v_new(0.0, 0.0), s_b: c2v_new(0.0, 0.0), p: c2v_new(0.0, 0.0), u: 0.0, i_a: 0, i_b: 0 };
    let mut s = c2Simplex { a: zero_sv, b: zero_sv, c: zero_sv, _d: zero_sv, div: 0.0, count: 0 };

    let mut cache_was_read = false;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count {
                let i_a = cache_ref.i_a[i as usize];
                let i_b = cache_ref.i_b[i as usize];
                let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
                let v = match i {
                    0 => &mut s.a,
                    1 => &mut s.b,
                    2 => &mut s.c,
                    _ => &mut s._d,
                };
                v.i_a = i_a;
                v.s_a = s_a;
                v.i_b = i_b;
                v.s_b = s_b;
                v.p = c2_sub(v.s_b, v.s_a);
                v.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = true;
            }
        }
    }

    if !cache_was_read {
        s.a.i_a = 0;
        s.a.i_b = 0;
        s.a.s_a = c2_mulxv(ax, p_a.verts[0]);
        s.a.s_b = c2_mulxv(bx, p_b.verts[0]);
        s.a.p = c2_sub(s.a.s_b, s.a.s_a);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a = [0i32; 3];
    let mut save_b = [0i32; 3];
    let mut save_count;
    let mut d0 = FLT_MAX;
    let mut d1;
    let mut iter = 0;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = match i { 0 => &s.a, 1 => &s.b, 2 => &s.c, _ => &s._d };
            save_a[i] = v.i_a;
            save_b[i] = v.i_b;
        }

        match s.count {
            1 => {}
            2 => c22_inner(&mut s),
            3 => c23_inner(&mut s),
            _ => {}
        }

        if s.count == 3 {
            hit = true;
            break;
        }

        let p = c2_l(&s);
        d1 = c2_dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2_d(&s);
        if c2_dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }

        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);

        let v = match s.count { 0 => &mut s.a, 1 => &mut s.b, 2 => &mut s.c, _ => &mut s._d };
        v.i_a = i_a;
        v.s_a = s_a;
        v.i_b = i_b;
        v.s_b = s_b;
        v.p = c2_sub(v.s_b, v.s_a);

        let mut dup = false;
        for i in 0..save_count as usize {
            if i_a == save_a[i] && i_b == save_b[i] {
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

    let mut a = c2v_new(0.0, 0.0);
    let mut b = c2v_new(0.0, 0.0);
    c2_witness(&s, &mut a, &mut b);
    let mut dist = c2_len(c2_sub(a, b));

    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > FLT_EPSILON {
            dist -= r_a + r_b;
            let n = c2_norm(c2_sub(b, a));
            a = c2_add(a, c2_mulvs(n, r_a));
            b = c2_sub(b, c2_mulvs(n, r_b));
            if a.x == b.x && a.y == b.y {
                dist = 0.0;
            }
        } else {
            let p = c2_mulvs(c2_add(a, b), 0.5);
            a = p;
            b = p;
            dist = 0.0;
        }
    }

    if !cache.is_null() {
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = c2_gjk_simplex_metric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            let v = match i { 0 => &s.a, 1 => &s.b, 2 => &s.c, _ => &s._d };
            cache_ref.i_a[i] = v.i_a;
            cache_ref.i_b[i] = v.i_b;
        }
        cache_ref.div = s.div;
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

#[unsafe(no_mangle)]
pub extern "C" fn gjk_cache(
    reverse: c_char, a9: *mut c2v, b9: *mut c2v,
    a1: f32, a2: f32, a3: f32, a4: f32,
    b1: f32, b2: f32, b3: f32, b4: f32, b5: f32,
) {
    let mut cache = c2GJKCache { metric: 0.0, count: 0, i_a: [0; 3], i_b: [0; 3], div: 0.0 };

    let circle_a = c2Circle { p: c2v_new(0.0, 0.0), r: 15.0 };
    let cap_b = c2Capsule { a: c2v_new(100.0, -25.0), b: c2v_new(75.0, 100.0), r: 10.0 };

    let mut a0 = c2v_new(0.0, 0.0);
    let mut b0 = c2v_new(0.0, 0.0);
    let mut a = c2v_new(0.0, 0.0);
    let mut b = c2v_new(0.0, 0.0);

    let mut iterations: i32 = -1;
    let mut cached_iterations: i32 = -1;

    let _d0 = c2_gjk(
        &circle_a as *const c2Circle as *const u8, C2Type::Circle, std::ptr::null(),
        &cap_b as *const c2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
        &mut a0, &mut b0, 1, &mut iterations, &mut cache,
    );
    let _d1 = c2_gjk(
        &circle_a as *const c2Circle as *const u8, C2Type::Circle, std::ptr::null(),
        &cap_b as *const c2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
        &mut a, &mut b, 1, &mut cached_iterations, &mut cache,
    );

    let bb = c2AABB { min: c2v_new(a1, a2), max: c2v_new(a3, a4) };
    let cap = c2Capsule { a: c2v_new(b1, b2), b: c2v_new(b3, b4), r: b5 };

    if reverse != 0 {
        c2_gjk(
            &cap as *const c2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
            &bb as *const c2AABB as *const u8, C2Type::AABB, std::ptr::null(),
            &mut a, &mut b, 1, std::ptr::null_mut(), std::ptr::null_mut(),
        );
    } else {
        c2_gjk(
            &bb as *const c2AABB as *const u8, C2Type::AABB, std::ptr::null(),
            &cap as *const c2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
            &mut a, &mut b, 1, std::ptr::null_mut(), std::ptr::null_mut(),
        );
    }
}

// ---- C-ABI export wrappers ----

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v_new(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    c2_mulvs(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2_maxv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2_minv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2_clampv(a, lo, hi)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2_sub(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    c2_add(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    c2_dot(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2_rot_identity()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x_identity()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2BBVerts(out: *mut c2v, bb: *const c2AABB) {
    unsafe {
        let out_arr = &mut *(out as *mut [c2v; 8]);
        c2_bb_verts(out_arr, &*bb);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MakeProxy(shape: *const u8, typ: C2Type, p: *mut c2Proxy) {
    unsafe { c2_make_proxy(shape, typ, &mut *p) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2_len(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    c2_det2(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe { c2_gjk_simplex_metric(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2_mulrv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2_mulxv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c22(s: *mut c2Simplex) {
    unsafe { self::c22_inner(&mut *s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c23(s: *mut c2Simplex) {
    unsafe { self::c23_inner(&mut *s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2_neg(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2_skew(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2_ccw90(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe { c2_d(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Support(verts: *const c2v, count: i32, d: c2v) -> i32 {
    unsafe { c2_support(&*(verts as *const [c2v; 8]), count, d) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe { c2_witness(&*s, &mut *a, &mut *b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2_mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2_norm(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe { c2_l(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2_mulrv_t(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2GJK(
    a_shape: *const u8, type_a: C2Type, ax_ptr: *const c2x,
    b_shape: *const u8, type_b: C2Type, bx_ptr: *const c2x,
    out_a: *mut c2v, out_b: *mut c2v,
    use_radius: i32, iterations: *mut i32, cache: *mut c2GJKCache,
) -> f32 {
    c2_gjk(a_shape, type_a, ax_ptr, b_shape, type_b, bx_ptr, out_a, out_b, use_radius, iterations, cache)
}
