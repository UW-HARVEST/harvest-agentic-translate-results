use std::os::raw::{c_float, c_int};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum C2Type {
    Circle,
    Aabb,
    Capsule,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2GjkCache {
    pub metric: c_float,
    pub count: c_int,
    pub i_a: [c_int; 3],
    pub i_b: [c_int; 3],
    pub div: c_float,
}

#[derive(Clone, Copy, Debug)]
struct C2Proxy {
    radius: c_float,
    count: usize,
    verts: [C2v; 8],
}

#[derive(Clone, Copy, Debug)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: c_float,
    i_a: usize,
    i_b: usize,
}

#[derive(Clone, Copy, Debug)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: c_float,
    count: usize,
}

fn c2_v(x: c_float, y: c_float) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(a: C2v, b: c_float) -> C2v {
    C2v {
        x: a.x * b,
        y: a.y * b,
    }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2_add(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn c2_dot(a: C2v, b: C2v) -> c_float {
    a.x * b.x + a.y * b.y
}

fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

fn c2_x_identity() -> C2x {
    C2x {
        p: c2_v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

fn c2_bb_verts(bb: &C2Aabb) -> [C2v; 4] {
    [
        bb.min,
        c2_v(bb.max.x, bb.min.y),
        bb.max,
        c2_v(bb.min.x, bb.max.y),
    ]
}

fn c2_make_proxy(shape: *const u8, shape_type: C2Type) -> C2Proxy {
    unsafe {
        match shape_type {
            C2Type::Circle => {
                let c = &*(shape as *const C2Circle);
                C2Proxy {
                    radius: c.r,
                    count: 1,
                    verts: {
                        let mut v = [C2v::default(); 8];
                        v[0] = c.p;
                        v
                    },
                }
            }
            C2Type::Aabb => {
                let bb = &*(shape as *const C2Aabb);
                let verts = c2_bb_verts(bb);
                C2Proxy {
                    radius: 0.0,
                    count: 4,
                    verts: {
                        let mut v = [C2v::default(); 8];
                        v[0] = verts[0];
                        v[1] = verts[1];
                        v[2] = verts[2];
                        v[3] = verts[3];
                        v
                    },
                }
            }
            C2Type::Capsule => {
                let c = &*(shape as *const C2Capsule);
                C2Proxy {
                    radius: c.r,
                    count: 2,
                    verts: {
                        let mut v = [C2v::default(); 8];
                        v[0] = c.a;
                        v[1] = c.b;
                        v
                    },
                }
            }
        }
    }
}

fn c2_len(a: C2v) -> c_float {
    c2_dot(a, a).sqrt()
}

fn c2_det2(a: C2v, b: C2v) -> c_float {
    a.x * b.y - a.y * b.x
}

fn c2_gjk_simplex_metric(s: &C2Simplex) -> c_float {
    match s.count {
        1 => 0.0,
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        _ => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2_v(
        a.c * b.x - a.s * b.y,
        a.s * b.x + a.c * b.y,
    )
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(
        a.c * b.x + a.s * b.y,
        -a.s * b.x + a.c * b.y,
    )
}

fn c2_mulxv(a: C2x, b: C2v) -> C2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

fn c2_2(s: &mut C2Simplex) {
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

fn c2_3(s: &mut C2Simplex) {
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
    c2_v(-a.y, a.x)
}

fn c2_ccw90(a: C2v) -> C2v {
    c2_v(a.y, -a.x)
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

fn c2_support(verts: &[C2v], d: C2v) -> usize {
    let mut i_max = 0;
    let mut d_max = c2_dot(verts[0], d);
    for i in 1..verts.len() {
        let dot = c2_dot(verts[i], d);
        if dot > d_max {
            i_max = i;
            d_max = dot;
        }
    }
    i_max
}

fn c2_witness(s: &C2Simplex) -> (C2v, C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => (s.a.s_a, s.a.s_b),
        2 => {
            let a = c2_add(
                c2_mulvs(s.a.s_a, den * s.a.u),
                c2_mulvs(s.b.s_a, den * s.b.u),
            );
            let b = c2_add(
                c2_mulvs(s.a.s_b, den * s.a.u),
                c2_mulvs(s.b.s_b, den * s.b.u),
            );
            (a, b)
        }
        3 => {
            let a = c2_add(
                c2_add(
                    c2_mulvs(s.a.s_a, den * s.a.u),
                    c2_mulvs(s.b.s_a, den * s.b.u),
                ),
                c2_mulvs(s.c.s_a, den * s.c.u),
            );
            let b = c2_add(
                c2_add(
                    c2_mulvs(s.a.s_b, den * s.a.u),
                    c2_mulvs(s.b.s_b, den * s.b.u),
                ),
                c2_mulvs(s.c.s_b, den * s.c.u),
            );
            (a, b)
        }
        _ => (c2_v(0.0, 0.0), c2_v(0.0, 0.0)),
    }
}

fn c2_div(a: C2v, b: c_float) -> C2v {
    c2_mulvs(a, 1.0 / b)
}

fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
}

fn c2_l(s: &C2Simplex) -> C2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(
            c2_mulvs(s.a.p, den * s.a.u),
            c2_mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_gjk(
    a: *const u8,
    type_a: C2Type,
    ax_ptr: *const C2x,
    b: *const u8,
    type_b: C2Type,
    bx_ptr: *const C2x,
    out_a: *mut C2v,
    out_b: *mut C2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut C2GjkCache,
) -> c_float {
    let ax = if ax_ptr.is_null() {
        c2_x_identity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx = if bx_ptr.is_null() {
        c2_x_identity()
    } else {
        unsafe { *bx_ptr }
    };

    let p_a = c2_make_proxy(a, type_a);
    let p_b = c2_make_proxy(b, type_b);

    let mut s = C2Simplex {
        a: C2sv {
            s_a: C2v::default(),
            s_b: C2v::default(),
            p: C2v::default(),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        b: C2sv {
            s_a: C2v::default(),
            s_b: C2v::default(),
            p: C2v::default(),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        c: C2sv {
            s_a: C2v::default(),
            s_b: C2v::default(),
            p: C2v::default(),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        d: C2sv {
            s_a: C2v::default(),
            s_b: C2v::default(),
            p: C2v::default(),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        div: 1.0,
        count: 1,
    };

    let verts: &mut [C2sv; 4] = unsafe {
        std::slice::from_raw_parts_mut(&mut s.a as *mut C2sv, 4)
            .try_into()
            .unwrap_unchecked()
    };

    let mut cache_was_read = false;
    if !cache.is_null() {
        unsafe {
            let cache_ref = &*cache;
            let cache_was_good = cache_ref.count != 0;
            if cache_was_good {
                for i in 0..(cache_ref.count as usize) {
                    let i_a = cache_ref.i_a[i] as usize;
                    let i_b = cache_ref.i_b[i] as usize;
                    let s_a = c2_mulxv(ax, p_a.verts[i_a]);
                    let s_b = c2_mulxv(bx, p_b.verts[i_b]);
                    let v = &mut verts[i];
                    v.i_a = i_a;
                    v.s_a = s_a;
                    v.i_b = i_b;
                    v.s_b = s_b;
                    v.p = c2_sub(v.s_b, v.s_a);
                    v.u = 0.0;
                }
                s.count = cache_ref.count as usize;
                s.div = cache_ref.div;
                let metric_old = cache_ref.metric;
                let metric = c2_gjk_simplex_metric(&s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if min_metric < max_metric * 2.0 && metric < -1.0e8 {
                    cache_was_read = true;
                }
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

    let mut save_a: [usize; 3] = [0; 3];
    let mut save_b: [usize; 3] = [0; 3];
    let mut save_count: usize = 0;
    let mut d0: c_float = f32::MAX;
    let mut d1: c_float = f32::MAX;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            save_a[i] = verts[i].i_a;
            save_b[i] = verts[i].i_b;
        }

        match s.count {
            1 => {}
            2 => c2_2(&mut s),
            _ => c2_3(&mut s),
        }

        if s.count == 3 {
            hit = 1;
            break;
        }

        let p = c2_l(&s);
        d1 = c2_dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2_d(&s);
        let eps = 1.1920928955078125e-7;
        if c2_dot(d, d) < eps * eps {
            break;
        }

        let i_a = c2_support(&p_a.verts[..p_a.count], c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a]);
        let i_b = c2_support(&p_b.verts[..p_b.count], c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b]);

        let v = &mut verts[s.count];
        v.i_a = i_a;
        v.s_a = s_a;
        v.i_b = i_b;
        v.s_b = s_b;
        v.p = c2_sub(v.s_b, v.s_a);

        let mut dup = false;
        for i in 0..save_count {
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

    let (mut a_out, mut b_out) = c2_witness(&s);
    let mut dist = c2_len(c2_sub(a_out, b_out));

    if hit != 0 {
        a_out = b_out;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        let eps = 1.1920928955078125e-7;
        if dist > r_a + r_b && dist > eps {
            dist -= r_a + r_b;
            let n = c2_norm(c2_sub(b_out, a_out));
            a_out = c2_add(a_out, c2_mulvs(n, r_a));
            b_out = c2_sub(b_out, c2_mulvs(n, r_b));
            if a_out.x == b_out.x && a_out.y == b_out.y {
                dist = 0.0;
            }
        } else {
            let p = c2_mulvs(c2_add(a_out, b_out), 0.5);
            a_out = p;
            b_out = p;
            dist = 0.0;
        }
    }

    if !cache.is_null() {
        unsafe {
            let cache_ref = &mut *cache;
            cache_ref.metric = c2_gjk_simplex_metric(&s);
            cache_ref.count = s.count as c_int;
            for i in 0..s.count {
                let v = &verts[i];
                cache_ref.i_a[i] = v.i_a as c_int;
                cache_ref.i_b[i] = v.i_b as c_int;
            }
            cache_ref.div = s.div;
        }
    }

    if !out_a.is_null() {
        unsafe { *out_a = a_out };
    }
    if !out_b.is_null() {
        unsafe { *out_b = b_out };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }

    dist
}

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    (!(d0 | d1 | d2 | d3)) as c_int
}

fn c2_aabb_to_capsule(a: C2Aabb, b: C2Capsule) -> c_int {
    let a_ptr = &a as *const C2Aabb as *const u8;
    let b_ptr = &b as *const C2Capsule as *const u8;
    if c2_gjk(
        a_ptr,
        C2Type::Aabb,
        std::ptr::null(),
        b_ptr,
        C2Type::Capsule,
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

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> c_int {
    let a_ptr = &a as *const C2Capsule as *const u8;
    let b_ptr = &b as *const C2Capsule as *const u8;
    if c2_gjk(
        a_ptr,
        C2Type::Capsule,
        std::ptr::null(),
        b_ptr,
        C2Type::Capsule,
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

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> c_int {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let da = c2_dot(ap, n);
    let d2: c_float;
    if da < 0.0 {
        d2 = c2_dot(ap, ap);
    } else {
        let db = c2_dot(c2_sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2_sub(ap, c2_mulvs(n, da / c2_dot(n, n)));
            d2 = c2_dot(e, e);
        } else {
            let bp = c2_sub(a.p, b.b);
            d2 = c2_dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

fn c2_collided(a: *const u8, type_a: C2Type, b: *const u8, type_b: C2Type) -> c_int {
    unsafe {
        match type_a {
            C2Type::Circle => {
                let circle_a = &*(a as *const C2Circle);
                match type_b {
                    C2Type::Circle => {
                        let circle_b = &*(b as *const C2Circle);
                        c2_circle_to_circle(*circle_a, *circle_b)
                    }
                    C2Type::Aabb => {
                        let aabb_b = &*(b as *const C2Aabb);
                        c2_circle_to_aabb(*circle_a, *aabb_b)
                    }
                    C2Type::Capsule => {
                        let capsule_b = &*(b as *const C2Capsule);
                        c2_circle_to_capsule(*circle_a, *capsule_b)
                    }
                }
            }
            C2Type::Aabb => {
                let aabb_a = &*(a as *const C2Aabb);
                match type_b {
                    C2Type::Circle => {
                        let circle_b = &*(b as *const C2Circle);
                        c2_circle_to_aabb(*circle_b, *aabb_a)
                    }
                    C2Type::Aabb => {
                        let aabb_b = &*(b as *const C2Aabb);
                        c2_aabb_to_aabb(*aabb_a, *aabb_b)
                    }
                    C2Type::Capsule => {
                        let capsule_b = &*(b as *const C2Capsule);
                        c2_aabb_to_capsule(*aabb_a, *capsule_b)
                    }
                }
            }
            C2Type::Capsule => {
                let capsule_a = &*(a as *const C2Capsule);
                match type_b {
                    C2Type::Circle => {
                        let circle_b = &*(b as *const C2Circle);
                        c2_circle_to_capsule(*circle_b, *capsule_a)
                    }
                    C2Type::Aabb => {
                        let aabb_b = &*(b as *const C2Aabb);
                        c2_aabb_to_capsule(*aabb_b, *capsule_a)
                    }
                    C2Type::Capsule => {
                        let capsule_b = &*(b as *const C2Capsule);
                        c2_capsule_to_capsule(*capsule_a, *capsule_b)
                    }
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn aabb(min_x: c_float, min_y: c_float, max_x: c_float, max_y: c_float) -> c_int {
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

    result += c2_collided(
        &circle as *const C2Circle as *const u8,
        C2Type::Circle,
        &aabb_in as *const C2Aabb as *const u8,
        C2Type::Aabb,
    );

    result += c2_collided(
        &aabb as *const C2Aabb as *const u8,
        C2Type::Aabb,
        &aabb_in as *const C2Aabb as *const u8,
        C2Type::Aabb,
    ) << 1;

    result += c2_collided(
        &capsule as *const C2Capsule as *const u8,
        C2Type::Capsule,
        &aabb_in as *const C2Aabb as *const u8,
        C2Type::Aabb,
    ) << 2;

    result
}
