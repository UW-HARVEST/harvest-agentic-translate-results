use std::os::raw::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2GjkCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        Self {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.max(b.x), a.y.max(b.y))
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.min(b.x), a.y.min(b.y))
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

fn c2x_identity() -> C2x {
    C2x {
        p: c2_v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

fn c2_bb_verts(out: &mut [C2v; 8], bb: &C2Aabb) {
    out[0] = bb.min;
    out[1] = c2_v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2_v(bb.min.x, bb.max.y);
}

unsafe fn c2_make_proxy(shape: *const core::ffi::c_void, type_: C2Type, p: &mut C2Proxy) {
    match type_ {
        C2Type::Circle => {
            let c = unsafe { &*(shape as *const C2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2Type::Aabb => {
            let bb = unsafe { &*(shape as *const C2Aabb) };
            p.radius = 0.0;
            p.count = 4;
            c2_bb_verts(&mut p.verts, bb);
        }
        C2Type::Capsule => {
            let c = unsafe { &*(shape as *const C2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        1 => 0.0,
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2_mulxv(a: C2x, b: C2v) -> C2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

fn c22(s: &mut C2Simplex) {
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

fn c23(s: &mut C2Simplex) {
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

fn c2_neg(a: C2v) -> C2v {
    c2_v(-a.x, -a.y)
}

fn c2_skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

fn c2_ccw90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

fn c2_d(s: &C2Simplex) -> C2v {
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
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_support(verts: &[C2v], count: c_int, d: C2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2_dot(verts[0], d);
    for i in 1..count as usize {
        let dot = c2_dot(verts[i], d);
        if dot > dmax {
            imax = i as c_int;
            dmax = dot;
        }
    }
    imax
}

fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0 / s.div;
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
            *a = c2_add(
                c2_add(c2_mulvs(s.a.s_a, den * s.a.u), c2_mulvs(s.b.s_a, den * s.b.u)),
                c2_mulvs(s.c.s_a, den * s.c.u),
            );
            *b = c2_add(
                c2_add(c2_mulvs(s.a.s_b, den * s.a.u), c2_mulvs(s.b.s_b, den * s.b.u)),
                c2_mulvs(s.c.s_b, den * s.c.u),
            );
        }
        _ => {
            *a = c2_v(0.0, 0.0);
            *b = c2_v(0.0, 0.0);
        }
    }
}

fn c2_div(a: C2v, b: f32) -> C2v {
    c2_mulvs(a, 1.0 / b)
}

fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
}

fn c2_l(s: &C2Simplex) -> C2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

unsafe fn c2_gjk(
    a_ptr: *const core::ffi::c_void,
    type_a: C2Type,
    ax_ptr: *const C2x,
    b_ptr: *const core::ffi::c_void,
    type_b: C2Type,
    bx_ptr: *const C2x,
    out_a: *mut C2v,
    out_b: *mut C2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut C2GjkCache,
) -> f32 {
    let ax = if ax_ptr.is_null() { c2x_identity() } else { unsafe { *ax_ptr } };
    let bx = if bx_ptr.is_null() { c2x_identity() } else { unsafe { *bx_ptr } };
    let mut p_a = C2Proxy::default();
    let mut p_b = C2Proxy::default();
    unsafe {
        c2_make_proxy(a_ptr, type_a, &mut p_a);
        c2_make_proxy(b_ptr, type_b, &mut p_b);
    }
    let mut s = C2Simplex::default();
    let mut cache_was_read = false;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let i_a = cache_ref.i_a[i] as usize;
                let i_b = cache_ref.i_b[i] as usize;
                let s_a = c2_mulxv(ax, p_a.verts[i_a]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b]);
                let v = match i {
                    0 => &mut s.a,
                    1 => &mut s.b,
                    _ => &mut s.c,
                };
                v.i_a = i_a as c_int;
                v.s_a = s_a;
                v.i_b = i_b as c_int;
                v.s_b = s_b;
                v.p = c2_sub(v.s_b, v.s_a);
                v.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = metric.min(metric_old);
            let max_metric = metric.max(metric_old);
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8_f32) {
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
    let mut save_a = [0 as c_int; 3];
    let mut save_b = [0 as c_int; 3];
    let mut d0 = f32::MAX;
    let mut d1 = f32::MAX;
    let mut iter: c_int = 0;
    let mut hit = false;
    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count as usize {
            let v = match i {
                0 => s.a,
                1 => s.b,
                _ => s.c,
            };
            save_a[i] = v.i_a;
            save_b[i] = v.i_b;
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
        let p = c2_l(&s);
        d1 = c2_dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2_d(&s);
        if c2_dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }
        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
        let v = match s.count {
            1 => &mut s.b,
            2 => &mut s.c,
            _ => &mut s.d,
        };
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
    let mut a = C2v::default();
    let mut b = C2v::default();
    c2_witness(&s, &mut a, &mut b);
    let mut dist = c2_len(c2_sub(a, b));
    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > f32::EPSILON {
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
        let cache_mut = unsafe { &mut *cache };
        cache_mut.metric = c2_gjk_simplex_metric(&s);
        cache_mut.count = s.count;
        for i in 0..s.count as usize {
            let v = match i {
                0 => s.a,
                1 => s.b,
                _ => s.c,
            };
            cache_mut.i_a[i] = v.i_a;
            cache_mut.i_b[i] = v.i_b;
        }
        cache_mut.div = s.div;
    }
    if !out_a.is_null() {
        unsafe { *out_a = a };
    }
    if !out_b.is_null() {
        unsafe { *out_b = b };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }
    dist
}

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    if (d0 | d1 | d2 | d3) == 0 { 1 } else { 0 }
}

fn c2_aabb_to_capsule(a: C2Aabb, b: C2Capsule) -> c_int {
    let dist = unsafe {
        c2_gjk(
            &a as *const _ as *const core::ffi::c_void,
            C2Type::Aabb,
            core::ptr::null(),
            &b as *const _ as *const core::ffi::c_void,
            C2Type::Capsule,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };
    if dist != 0.0 { 0 } else { 1 }
}

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> c_int {
    let dist = unsafe {
        c2_gjk(
            &a as *const _ as *const core::ffi::c_void,
            C2Type::Capsule,
            core::ptr::null(),
            &b as *const _ as *const core::ffi::c_void,
            C2Type::Capsule,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };
    if dist != 0.0 { 0 } else { 1 }
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> c_int {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let da = c2_dot(ap, n);
    let d2 = if da < 0.0 {
        c2_dot(ap, ap)
    } else {
        let db = c2_dot(c2_sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2_sub(ap, c2_mulvs(n, da / c2_dot(n, n)));
            c2_dot(e, e)
        } else {
            let bp = c2_sub(a.p, b.b);
            c2_dot(bp, bp)
        }
    };
    let r = a.r + b.r;
    if d2 < r * r { 1 } else { 0 }
}

unsafe fn c2_collided(
    a: *const core::ffi::c_void,
    type_a: C2Type,
    b: *const core::ffi::c_void,
    type_b: C2Type,
) -> c_int {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => c2_circle_to_circle(unsafe { *(a as *const C2Circle) }, unsafe { *(b as *const C2Circle) }),
            C2Type::Aabb => c2_circle_to_aabb(unsafe { *(a as *const C2Circle) }, unsafe { *(b as *const C2Aabb) }),
            C2Type::Capsule => c2_circle_to_capsule(unsafe { *(a as *const C2Circle) }, unsafe { *(b as *const C2Capsule) }),
        },
        C2Type::Aabb => match type_b {
            C2Type::Circle => c2_circle_to_aabb(unsafe { *(b as *const C2Circle) }, unsafe { *(a as *const C2Aabb) }),
            C2Type::Aabb => c2_aabb_to_aabb(unsafe { *(a as *const C2Aabb) }, unsafe { *(b as *const C2Aabb) }),
            C2Type::Capsule => c2_aabb_to_capsule(unsafe { *(a as *const C2Aabb) }, unsafe { *(b as *const C2Capsule) }),
        },
        C2Type::Capsule => match type_b {
            C2Type::Circle => c2_circle_to_capsule(unsafe { *(b as *const C2Circle) }, unsafe { *(a as *const C2Capsule) }),
            C2Type::Aabb => c2_aabb_to_capsule(unsafe { *(b as *const C2Aabb) }, unsafe { *(a as *const C2Capsule) }),
            C2Type::Capsule => c2_capsule_to_capsule(unsafe { *(a as *const C2Capsule) }, unsafe { *(b as *const C2Capsule) }),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> c_int {
    let mut result: c_int = 0;

    let aabb_in = C2Aabb {
        min: c2_v(min_x, min_y),
        max: c2_v(max_x, max_y),
    };

    let circle = C2Circle {
        p: c2_v(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = C2Aabb {
        min: c2_v(-40.0, -40.0),
        max: c2_v(-15.0, -15.0),
    };

    let capsule = C2Capsule {
        a: c2_v(-40.0, 40.0),
        b: c2_v(-20.0, 100.0),
        r: 10.0,
    };

    result += unsafe {
        c2_collided(
            &circle as *const _ as *const core::ffi::c_void,
            C2Type::Circle,
            &aabb_in as *const _ as *const core::ffi::c_void,
            C2Type::Aabb,
        )
    };

    result += unsafe {
        c2_collided(
            &aabb as *const _ as *const core::ffi::c_void,
            C2Type::Aabb,
            &aabb_in as *const _ as *const core::ffi::c_void,
            C2Type::Aabb,
        ) << 1
    };

    result += unsafe {
        c2_collided(
            &capsule as *const _ as *const core::ffi::c_void,
            C2Type::Capsule,
            &aabb_in as *const _ as *const core::ffi::c_void,
            C2Type::Aabb,
        ) << 2
    };

    result
}
