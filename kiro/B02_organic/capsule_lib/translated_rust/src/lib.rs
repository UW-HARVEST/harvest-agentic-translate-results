use std::os::raw::c_int;

// --- Types ---

#[derive(Clone, Copy)]
#[repr(C)]
pub struct C2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct C2r {
    c: f32,
    s: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
pub struct C2GJKCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
}

#[derive(Clone, Copy)]
pub struct C2Proxy {
    radius: f32,
    count: i32,
    verts: [C2v; 8],
}

#[derive(Clone, Copy)]
pub struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: i32,
    i_b: i32,
}

#[derive(Clone, Copy)]
pub struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: i32,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub enum C2Type {
    Circle,
    AABB,
    Capsule,
}

// --- Helper functions ---

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(a: C2v, b: f32) -> C2v {
    c2v(a.x * b, a.y * b)
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(a: C2v, b: C2v) -> C2v {
    c2v(a.x - b.x, a.y - b.y)
}

fn c2_add(a: C2v, b: C2v) -> C2v {
    c2v(a.x + b.x, a.y + b.y)
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2_neg(a: C2v) -> C2v {
    c2v(-a.x, -a.y)
}

fn c2_norm(a: C2v) -> C2v {
    c2_mulvs(a, 1.0f32 / c2_len(a))
}

fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

fn c2x_identity() -> C2x {
    C2x {
        p: c2v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_mulxv(a: C2x, b: C2v) -> C2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_skew(a: C2v) -> C2v {
    c2v(-a.y, a.x)
}

fn c2_ccw90(a: C2v) -> C2v {
    c2v(a.y, -a.x)
}

// --- Proxy ---

fn c2_bb_verts(out: &mut [C2v; 8], bb: &C2AABB) {
    out[0] = bb.min;
    out[1] = c2v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: *const u8, typ: C2Type, p: &mut C2Proxy) {
    unsafe {
        match typ {
            C2Type::Circle => {
                let c = &*(shape as *const C2Circle);
                p.radius = c.r;
                p.count = 1;
                p.verts[0] = c.p;
            }
            C2Type::AABB => {
                let bb = &*(shape as *const C2AABB);
                p.radius = 0.0;
                p.count = 4;
                c2_bb_verts(&mut p.verts, bb);
            }
            C2Type::Capsule => {
                let c = &*(shape as *const C2Capsule);
                p.radius = c.r;
                p.count = 2;
                p.verts[0] = c.a;
                p.verts[1] = c.b;
            }
        }
    }
}

// --- Simplex operations ---

fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c22_impl(s: &mut C2Simplex) {
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

fn c23_impl(s: &mut C2Simplex) {
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
        _ => c2v(0.0, 0.0),
    }
}

fn c2_support(verts: &[C2v; 8], count: i32, d: C2v) -> i32 {
    let mut imax = 0i32;
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

fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
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
            *a = c2v(0.0, 0.0);
            *b = c2v(0.0, 0.0);
        }
    }
}

fn c2_l(s: &C2Simplex) -> C2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2v(0.0, 0.0),
    }
}

// --- GJK ---

const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

fn c2_gjk(
    a_shape: *const u8,
    type_a: C2Type,
    ax_ptr: *const C2x,
    b_shape: *const u8,
    type_b: C2Type,
    bx_ptr: *const C2x,
    out_a: *mut C2v,
    out_b: *mut C2v,
    use_radius: i32,
    iterations: *mut i32,
    cache: *mut C2GJKCache,
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

    let zero_sv = C2sv {
        s_a: c2v(0.0, 0.0),
        s_b: c2v(0.0, 0.0),
        p: c2v(0.0, 0.0),
        u: 0.0,
        i_a: 0,
        i_b: 0,
    };

    let mut p_a = C2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v(0.0, 0.0); 8],
    };
    let mut p_b = C2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v(0.0, 0.0); 8],
    };
    c2_make_proxy(a_shape, type_a, &mut p_a);
    c2_make_proxy(b_shape, type_b, &mut p_b);

    let mut s = C2Simplex {
        a: zero_sv,
        b: zero_sv,
        c: zero_sv,
        d: zero_sv,
        div: 0.0,
        count: 0,
    };

    // verts is an array view: s.a=verts[0], s.b=verts[1], s.c=verts[2], s.d=verts[3]
    // We access them via a helper macro-like approach using indices.

    let mut cache_was_read = false;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count {
                let ia = cache_ref.i_a[i as usize];
                let ib = cache_ref.i_b[i as usize];
                let sa = c2_mulxv(ax, p_a.verts[ia as usize]);
                let sb = c2_mulxv(bx, p_b.verts[ib as usize]);
                let v = C2sv {
                    i_a: ia,
                    s_a: sa,
                    i_b: ib,
                    s_b: sb,
                    p: c2_sub(sb, sa),
                    u: 0.0,
                };
                match i {
                    0 => s.a = v,
                    1 => s.b = v,
                    2 => s.c = v,
                    _ => s.d = v,
                }
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
    let mut d0 = FLT_MAX;
    let mut d1: f32;
    let mut iter = 0i32;
    let mut hit = false;

    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count as usize {
            let sv = match i {
                0 => &s.a,
                1 => &s.b,
                2 => &s.c,
                _ => &s.d,
            };
            save_a[i] = sv.i_a;
            save_b[i] = sv.i_b;
        }

        match s.count {
            1 => {}
            2 => c22_impl(&mut s),
            3 => c23_impl(&mut s),
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

        let ia = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let sa = c2_mulxv(ax, p_a.verts[ia as usize]);
        let ib = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let sb = c2_mulxv(bx, p_b.verts[ib as usize]);

        let v = C2sv {
            i_a: ia,
            s_a: sa,
            i_b: ib,
            s_b: sb,
            p: c2_sub(sb, sa),
            u: 0.0,
        };
        match s.count {
            0 => s.a = v,
            1 => s.b = v,
            2 => s.c = v,
            _ => s.d = v,
        }

        let mut dup = false;
        for i in 0..save_count as usize {
            if ia == save_a[i] && ib == save_b[i] {
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

    let mut a_out = c2v(0.0, 0.0);
    let mut b_out = c2v(0.0, 0.0);
    c2_witness(&s, &mut a_out, &mut b_out);
    let mut dist = c2_len(c2_sub(a_out, b_out));

    if hit {
        a_out = b_out;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > FLT_EPSILON {
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
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = c2_gjk_simplex_metric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            let sv = match i {
                0 => &s.a,
                1 => &s.b,
                2 => &s.c,
                _ => &s.d,
            };
            cache_ref.i_a[i] = sv.i_a;
            cache_ref.i_b[i] = sv.i_b;
        }
        cache_ref.div = s.div;
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

// --- Collision tests ---

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    ((d0 | d1 | d2 | d3) == 0) as i32
}

fn c2_aabb_to_capsule(a: C2AABB, b: C2Capsule) -> i32 {
    let ptr_a = &a as *const C2AABB as *const u8;
    let ptr_b = &b as *const C2Capsule as *const u8;
    if c2_gjk(
        ptr_a,
        C2Type::AABB,
        std::ptr::null(),
        ptr_b,
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

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> i32 {
    let ptr_a = &a as *const C2Capsule as *const u8;
    let ptr_b = &b as *const C2Capsule as *const u8;
    if c2_gjk(
        ptr_a,
        C2Type::Capsule,
        std::ptr::null(),
        ptr_b,
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

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = (a.r + b.r) * (a.r + b.r);
    (d2 < r2) as i32
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as i32
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> i32 {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let da = c2_dot(ap, n);
    let d2;
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
    (d2 < r * r) as i32
}

fn c2_collided(a: *const u8, type_a: C2Type, b: *const u8, type_b: C2Type) -> i32 {
    unsafe {
        match type_a {
            C2Type::Circle => match type_b {
                C2Type::Circle => {
                    c2_circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle))
                }
                C2Type::AABB => {
                    c2_circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB))
                }
                C2Type::Capsule => {
                    c2_circle_to_capsule(*(a as *const C2Circle), *(b as *const C2Capsule))
                }
            },
            C2Type::AABB => match type_b {
                C2Type::Circle => {
                    c2_circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB))
                }
                C2Type::AABB => {
                    c2_aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB))
                }
                C2Type::Capsule => {
                    c2_aabb_to_capsule(*(a as *const C2AABB), *(b as *const C2Capsule))
                }
            },
            C2Type::Capsule => match type_b {
                C2Type::Circle => {
                    c2_circle_to_capsule(*(b as *const C2Circle), *(a as *const C2Capsule))
                }
                C2Type::AABB => {
                    c2_aabb_to_capsule(*(b as *const C2AABB), *(a as *const C2Capsule))
                }
                C2Type::Capsule => {
                    c2_capsule_to_capsule(*(a as *const C2Capsule), *(b as *const C2Capsule))
                }
            },
        }
    }
}

// --- Exported C-compatible symbols ---

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v { c2v(x, y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: C2v, b: f32) -> C2v { c2_mulvs(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v { c2_maxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v { c2_minv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2_clampv(a, lo, hi) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: C2v, b: C2v) -> C2v { c2_sub(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: C2v, b: C2v) -> C2v { c2_add(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 { c2_dot(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 { c2_len(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 { c2_det2(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: C2v) -> C2v { c2_neg(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v { c2_norm(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v { c2_mulvs(a, 1.0f32 / b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r { c2_rot_identity() }

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x { c2x_identity() }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v { c2_mulrv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v { c2_mulxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v { c2_mulrv_t(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v { c2_skew(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v { c2_ccw90(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2BBVerts(out: *mut C2v, bb: *const C2AABB) {
    unsafe {
        let out_arr = &mut *(out as *mut [C2v; 8]);
        c2_bb_verts(out_arr, &*bb);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MakeProxy(shape: *const u8, typ: C2Type, p: *mut C2Proxy) {
    unsafe { c2_make_proxy(shape, typ, &mut *p); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2GJKSimplexMetric(s: *const C2Simplex) -> f32 {
    unsafe { c2_gjk_simplex_metric(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c22(s: *mut C2Simplex) {
    unsafe { self::c22_impl(&mut *s); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c23(s: *mut C2Simplex) {
    unsafe { self::c23_impl(&mut *s); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2D(s: *const C2Simplex) -> C2v {
    unsafe { c2_d(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
    unsafe {
        let arr = &*(verts as *const [C2v; 8]);
        c2_support(arr, count, d)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Witness(s: *const C2Simplex, a: *mut C2v, b: *mut C2v) {
    unsafe { c2_witness(&*s, &mut *a, &mut *b); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2L(s: *const C2Simplex) -> C2v {
    unsafe { c2_l(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2GJK(
    a: *const u8, type_a: C2Type, ax_ptr: *const C2x,
    b: *const u8, type_b: C2Type, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: c_int, iterations: *mut c_int, cache: *mut C2GJKCache,
) -> f32 {
    c2_gjk(a, type_a, ax_ptr, b, type_b, bx_ptr, out_a, out_b, use_radius, iterations, cache)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int { c2_aabb_to_aabb(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(a: C2AABB, b: C2Capsule) -> c_int { c2_aabb_to_capsule(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(a: C2Capsule, b: C2Capsule) -> c_int { c2_capsule_to_capsule(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int { c2_circle_to_circle(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2AABB) -> c_int { c2_circle_to_aabb(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: C2Circle, b: C2Capsule) -> c_int { c2_circle_to_capsule(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Collided(a: *const u8, type_a: C2Type, b: *const u8, type_b: C2Type) -> c_int {
    c2_collided(a, type_a, b, type_b)
}

// --- Public API ---

#[unsafe(no_mangle)]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let capsule_in = C2Capsule {
        a: c2v(min_x, min_y),
        b: c2v(max_x, max_y),
        r,
    };

    let circle = C2Circle {
        p: c2v(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = C2AABB {
        min: c2v(-40.0, -40.0),
        max: c2v(-15.0, -15.0),
    };

    let capsule_other = C2Capsule {
        a: c2v(-40.0, 40.0),
        b: c2v(-20.0, 100.0),
        r: 10.0,
    };

    result += c2_collided(
        &circle as *const C2Circle as *const u8,
        C2Type::Circle,
        &capsule_in as *const C2Capsule as *const u8,
        C2Type::Capsule,
    );

    result += c2_collided(
        &aabb as *const C2AABB as *const u8,
        C2Type::AABB,
        &capsule_in as *const C2Capsule as *const u8,
        C2Type::Capsule,
    ) << 1;

    result += c2_collided(
        &capsule_other as *const C2Capsule as *const u8,
        C2Type::Capsule,
        &capsule_in as *const C2Capsule as *const u8,
        C2Type::Capsule,
    ) << 2;

    result
}
