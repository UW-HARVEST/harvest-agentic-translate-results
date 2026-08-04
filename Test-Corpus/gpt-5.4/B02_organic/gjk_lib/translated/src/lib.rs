use std::os::raw::c_char;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
enum C2_TYPE {
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_CAPSULE,
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
    count: i32,
    iA: [i32; 3],
    iB: [i32; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: i32,
    iB: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: i32,
}

fn c2_v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2_mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2_maxv(a: c2v, b: c2v) -> c2v {
    c2_v(a.x.max(b.x), a.y.max(b.y))
}

fn c2_minv(a: c2v, b: c2v) -> c2v {
    c2_v(a.x.min(b.x), a.y.min(b.y))
}

fn c2_clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_rot_identity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

fn c2x_identity() -> c2x {
    c2x {
        p: c2_v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

fn c2_bb_verts(out: &mut [c2v; 8], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2_v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2_v(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: *const u8, type_: C2_TYPE, p: &mut c2Proxy) {
    match type_ {
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
            c2_bb_verts(&mut p.verts, bb);
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

fn c2_len(a: c2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2_gjk_simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        1 => 0.0,
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2_mulrv(a: c2r, b: c2v) -> c2v {
    c2_v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2_mulxv(a: c2x, b: c2v) -> c2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

fn c22(s: &mut c2Simplex) {
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

fn c23(s: &mut c2Simplex) {
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
    c2_v(-a.x, -a.y)
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
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_support(verts: &[c2v], count: i32, d: c2v) -> i32 {
    let mut imax = 0;
    let mut dmax = c2_dot(verts[0], d);
    let mut i = 1;
    while i < count as usize {
        let dot = c2_dot(verts[i], d);
        if dot > dmax {
            imax = i as i32;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

fn c2_witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2_add(c2_mulvs(s.a.sA, den * s.a.u), c2_mulvs(s.b.sA, den * s.b.u));
            *b = c2_add(c2_mulvs(s.a.sB, den * s.a.u), c2_mulvs(s.b.sB, den * s.b.u));
        }
        3 => {
            *a = c2_add(
                c2_add(c2_mulvs(s.a.sA, den * s.a.u), c2_mulvs(s.b.sA, den * s.b.u)),
                c2_mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2_add(
                c2_add(c2_mulvs(s.a.sB, den * s.a.u), c2_mulvs(s.b.sB, den * s.b.u)),
                c2_mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2_v(0.0, 0.0);
            *b = c2_v(0.0, 0.0);
        }
    }
}

fn c2_div(a: c2v, b: f32) -> c2v {
    c2_mulvs(a, 1.0 / b)
}

fn c2_norm(a: c2v) -> c2v {
    c2_div(a, c2_len(a))
}

fn c2_l(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: c2r, b: c2v) -> c2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn simplex_get(s: &c2Simplex, idx: usize) -> c2sv {
    match idx {
        0 => s.a,
        1 => s.b,
        2 => s.c,
        _ => s.d,
    }
}

fn simplex_set(s: &mut c2Simplex, idx: usize, v: c2sv) {
    match idx {
        0 => s.a = v,
        1 => s.b = v,
        2 => s.c = v,
        _ => s.d = v,
    }
}

fn c2_gjk(
    a_shape: *const u8,
    type_a: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const u8,
    type_b: C2_TYPE,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: i32,
    iterations: *mut i32,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() {
        c2x_identity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx = if bx_ptr.is_null() {
        c2x_identity()
    } else {
        unsafe { *bx_ptr }
    };

    let mut p_a = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v::default(); 8],
    };
    let mut p_b = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v::default(); 8],
    };
    c2_make_proxy(a_shape, type_a, &mut p_a);
    c2_make_proxy(b_shape, type_b, &mut p_b);

    let mut s = c2Simplex::default();
    let mut cache_was_read = false;

    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            let mut i = 0;
            while i < cache_ref.count as usize {
                let i_a = cache_ref.iA[i] as usize;
                let i_b = cache_ref.iB[i] as usize;
                let s_a = c2_mulxv(ax, p_a.verts[i_a]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b]);
                let mut v = simplex_get(&s, i);
                v.iA = i_a as i32;
                v.sA = s_a;
                v.iB = i_b as i32;
                v.sB = s_b;
                v.p = c2_sub(v.sB, v.sA);
                v.u = 0.0;
                simplex_set(&mut s, i, v);
                i += 1;
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
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2_mulxv(ax, p_a.verts[0]);
        s.a.sB = c2_mulxv(bx, p_b.verts[0]);
        s.a.p = c2_sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a = [0_i32; 3];
    let mut save_b = [0_i32; 3];
    let mut d0 = f32::MAX;
    let mut d1 = f32::MAX;
    let mut iter = 0_i32;
    let mut hit = false;

    while iter < 20 {
        let save_count = s.count as usize;
        let mut i = 0;
        while i < save_count {
            let v = simplex_get(&s, i);
            save_a[i] = v.iA;
            save_b[i] = v.iB;
            i += 1;
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

        let i_a = c2_support(&p_a.verts[..p_a.count as usize], p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts[..p_b.count as usize], p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);

        let mut v = c2sv::default();
        v.iA = i_a;
        v.sA = s_a;
        v.iB = i_b;
        v.sB = s_b;
        v.p = c2_sub(v.sB, v.sA);
        simplex_set(&mut s, s.count as usize, v);

        let mut dup = false;
        let mut j = 0;
        while j < save_count {
            if i_a == save_a[j] && i_b == save_b[j] {
                dup = true;
                break;
            }
            j += 1;
        }
        if dup {
            break;
        }

        s.count += 1;
        iter += 1;
    }

    let mut a = c2v::default();
    let mut b = c2v::default();
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
        let mut i = 0;
        while i < s.count as usize {
            let v = simplex_get(&s, i);
            cache_mut.iA[i] = v.iA;
            cache_mut.iB[i] = v.iB;
            i += 1;
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

#[unsafe(no_mangle)]
pub extern "C" fn gjk(
    reverse: c_char,
    a: *mut c2v,
    b: *mut c2v,
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
    let bb = c2AABB {
        min: c2_v(a1, a2),
        max: c2_v(a3, a4),
    };

    let cap = c2Capsule {
        a: c2_v(b1, b2),
        b: c2_v(b3, b4),
        r: b5,
    };

    if reverse != 0 {
        c2_gjk(
            &cap as *const _ as *const u8,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            &bb as *const _ as *const u8,
            C2_TYPE::C2_TYPE_AABB,
            std::ptr::null(),
            a,
            b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    } else {
        c2_gjk(
            &bb as *const _ as *const u8,
            C2_TYPE::C2_TYPE_AABB,
            std::ptr::null(),
            &cap as *const _ as *const u8,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            a,
            b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}
