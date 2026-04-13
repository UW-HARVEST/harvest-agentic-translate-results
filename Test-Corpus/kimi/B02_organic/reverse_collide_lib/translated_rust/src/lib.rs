use std::os::raw::{c_int, c_float};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum C2Type {
    Circle = 0,
    AABB = 1,
    Capsule = 2,
}

#[derive(Clone, Copy, Debug, Default)]
struct C2Proxy {
    radius: f32,
    count: usize,
    verts: [C2v; 8],
}

#[derive(Clone, Copy, Debug, Default)]
struct C2sv {
    sA: C2v,
    sB: C2v,
    p: C2v,
    u: f32,
    iA: usize,
    iB: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: usize,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(a: C2v, b: f32) -> C2v {
    C2v { x: a.x * b, y: a.y * b }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v { x: a.x - b.x, y: a.y - b.y }
}

fn c2_add(a: C2v, b: C2v) -> C2v {
    C2v { x: a.x + b.x, y: a.y + b.y }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

fn c2_x_identity() -> C2x {
    C2x { p: c2_v(0.0, 0.0), r: c2_rot_identity() }
}

fn c2_bb_verts(bb: &C2AABB) -> [C2v; 4] {
    [
        bb.min,
        c2_v(bb.max.x, bb.min.y),
        bb.max,
        c2_v(bb.min.x, bb.max.y),
    ]
}

fn c2_make_proxy(shape: *const u8, shape_type: C2Type) -> C2Proxy {
    let mut p = C2Proxy::default();
    match shape_type {
        C2Type::Circle => {
            let c = unsafe { &*(shape as *const C2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2Type::AABB => {
            let bb = unsafe { &*(shape as *const C2AABB) };
            p.radius = 0.0;
            p.count = 4;
            let verts = c2_bb_verts(bb);
            p.verts[0..4].copy_from_slice(&verts);
        }
        C2Type::Capsule => {
            let c = unsafe { &*(shape as *const C2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
    p
}

fn c2_len(a: C2v) -> f32 {
    (c2_dot(a, a)).sqrt()
}

fn c2_det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        1 => 0.0,
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        _ => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_mulxv(a: C2x, b: C2v) -> C2v {
    c2_add(c2_mulrv(a.r, b), a.p)
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

fn c2_div(a: C2v, b: f32) -> C2v {
    c2_mulvs(a, 1.0 / b)
}

fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
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
        1 => (s.a.sA, s.a.sB),
        2 => {
            let a = c2_add(
                c2_mulvs(s.a.sA, den * s.a.u),
                c2_mulvs(s.b.sA, den * s.b.u),
            );
            let b = c2_add(
                c2_mulvs(s.a.sB, den * s.a.u),
                c2_mulvs(s.b.sB, den * s.b.u),
            );
            (a, b)
        }
        3 => {
            let a = c2_add(
                c2_add(
                    c2_mulvs(s.a.sA, den * s.a.u),
                    c2_mulvs(s.b.sA, den * s.b.u),
                ),
                c2_mulvs(s.c.sA, den * s.c.u),
            );
            let b = c2_add(
                c2_add(
                    c2_mulvs(s.a.sB, den * s.a.u),
                    c2_mulvs(s.b.sB, den * s.b.u),
                ),
                c2_mulvs(s.c.sB, den * s.c.u),
            );
            (a, b)
        }
        _ => (c2_v(0.0, 0.0), c2_v(0.0, 0.0)),
    }
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
    use_radius: bool,
    cache: Option<&mut C2GJKCache>,
) -> (f32, C2v, C2v, usize) {
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
    let mut s = C2Simplex::default();
    let verts: &mut [C2sv; 4] = unsafe {
        std::slice::from_raw_parts_mut(&mut s.a as *mut C2sv, 4)
            .try_into()
            .unwrap_unchecked()
    };
    let mut cache_was_read = false;
    if let Some(cache) = cache.as_ref() {
        let cache_was_good = cache.count != 0;
        if cache_was_good {
            let count = cache.count as usize;
            for i in 0..count {
                let i_a = cache.iA[i] as usize;
                let i_b = cache.iB[i] as usize;
                let s_a = c2_mulxv(ax, p_a.verts[i_a]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b]);
                let v = &mut verts[i];
                v.iA = i_a;
                v.sA = s_a;
                v.iB = i_b;
                v.sB = s_b;
                v.p = c2_sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = count;
            s.div = cache.div;
            let metric_old = cache.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if min_metric < max_metric * 2.0 && metric < -1.0e8 {
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
    let mut save_a: [usize; 3] = [0; 3];
    let mut save_b: [usize; 3] = [0; 3];
    let mut save_count = 0;
    let mut d0 = f32::MAX;
    let mut d1 = f32::MAX;
    let mut iter = 0;
    let mut hit = false;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            save_a[i] = verts[i].iA;
            save_b[i] = verts[i].iB;
        }
        match s.count {
            1 => {}
            2 => c2_2(&mut s),
            3 => c2_3(&mut s),
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
        if c2_dot(d, d) < 1.1920928955078125e-7_f32 * 1.1920928955078125e-7_f32 {
            break;
        }
        let i_a = c2_support(&p_a.verts[..p_a.count], c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a]);
        let i_b = c2_support(&p_b.verts[..p_b.count], c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b]);
        let v = &mut verts[s.count];
        v.iA = i_a;
        v.sA = s_a;
        v.iB = i_b;
        v.sB = s_b;
        v.p = c2_sub(v.sB, v.sA);
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
    if hit {
        a_out = b_out;
        dist = 0.0;
    } else if use_radius {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > 1.1920928955078125e-7_f32 {
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
    if let Some(cache) = cache {
        cache.metric = c2_gjk_simplex_metric(&s);
        cache.count = s.count as c_int;
        for i in 0..s.count {
            let v = &verts[i];
            cache.iA[i] = v.iA as c_int;
            cache.iB[i] = v.iB as c_int;
        }
        cache.div = s.div;
    }
    (dist, a_out, b_out, iter)
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> bool {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    !(d0 || d1 || d2 || d3)
}

fn c2_aabb_to_capsule(a: C2AABB, b: C2Capsule) -> bool {
    let (dist, _, _, _) = c2_gjk(
        &a as *const C2AABB as *const u8,
        C2Type::AABB,
        std::ptr::null(),
        &b as *const C2Capsule as *const u8,
        C2Type::Capsule,
        std::ptr::null(),
        true,
        None,
    );
    dist == 0.0
}

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> bool {
    let (dist, _, _, _) = c2_gjk(
        &a as *const C2Capsule as *const u8,
        C2Type::Capsule,
        std::ptr::null(),
        &b as *const C2Capsule as *const u8,
        C2Type::Capsule,
        std::ptr::null(),
        true,
        None,
    );
    dist == 0.0
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> bool {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    d2 < r2
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> bool {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> bool {
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
    d2 < r * r
}

fn c2_collided(a: *const u8, type_a: C2Type, b: *const u8, type_b: C2Type) -> bool {
    match type_a {
        C2Type::Circle => {
            let circle_a = unsafe { &*(a as *const C2Circle) };
            match type_b {
                C2Type::Circle => {
                    let circle_b = unsafe { &*(b as *const C2Circle) };
                    c2_circle_to_circle(*circle_a, *circle_b)
                }
                C2Type::AABB => {
                    let aabb_b = unsafe { &*(b as *const C2AABB) };
                    c2_circle_to_aabb(*circle_a, *aabb_b)
                }
                C2Type::Capsule => {
                    let capsule_b = unsafe { &*(b as *const C2Capsule) };
                    c2_circle_to_capsule(*circle_a, *capsule_b)
                }
            }
        }
        C2Type::AABB => {
            let aabb_a = unsafe { &*(a as *const C2AABB) };
            match type_b {
                C2Type::Circle => {
                    let circle_b = unsafe { &*(b as *const C2Circle) };
                    c2_circle_to_aabb(*circle_b, *aabb_a)
                }
                C2Type::AABB => {
                    let aabb_b = unsafe { &*(b as *const C2AABB) };
                    c2_aabb_to_aabb(*aabb_a, *aabb_b)
                }
                C2Type::Capsule => {
                    let capsule_b = unsafe { &*(b as *const C2Capsule) };
                    c2_aabb_to_capsule(*aabb_a, *capsule_b)
                }
            }
        }
        C2Type::Capsule => {
            let capsule_a = unsafe { &*(a as *const C2Capsule) };
            match type_b {
                C2Type::Circle => {
                    let circle_b = unsafe { &*(b as *const C2Circle) };
                    c2_circle_to_capsule(*circle_b, *capsule_a)
                }
                C2Type::AABB => {
                    let aabb_b = unsafe { &*(b as *const C2AABB) };
                    c2_aabb_to_capsule(*aabb_b, *capsule_a)
                }
                C2Type::Capsule => {
                    let capsule_b = unsafe { &*(b as *const C2Capsule) };
                    c2_capsule_to_capsule(*capsule_a, *capsule_b)
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: c_float, y: c_float, r: c_float) -> c_int {
    let mut result = 0;
    let circle_in = C2Circle {
        p: c2_v(x, y),
        r,
    };
    let circle = C2Circle {
        p: c2_v(-70.0, 0.0),
        r: 20.0,
    };
    let aabb = C2AABB {
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
        &circle_in as *const C2Circle as *const u8,
        C2Type::Circle,
    ) as c_int;
    result += (c2_collided(
        &aabb as *const C2AABB as *const u8,
        C2Type::AABB,
        &circle_in as *const C2Circle as *const u8,
        C2Type::Circle,
    ) as c_int) << 1;
    result += (c2_collided(
        &capsule as *const C2Capsule as *const u8,
        C2Type::Capsule,
        &circle_in as *const C2Circle as *const u8,
        C2Type::Circle,
    ) as c_int) << 2;
    result
}
