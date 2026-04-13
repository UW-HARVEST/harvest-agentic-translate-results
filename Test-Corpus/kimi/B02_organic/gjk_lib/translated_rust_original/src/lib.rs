use std::f32::consts::PI;
use std::os::raw::{c_char, c_float, c_int};

#[repr(C)]
pub struct C2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
struct C2Circle {
    p: C2v,
    r: c_float,
}

#[repr(C)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: c_float,
}

#[repr(C)]
struct C2GjkCache {
    metric: c_float,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: c_float,
}

#[derive(Clone, Copy)]
struct C2Proxy {
    radius: c_float,
    count: usize,
    verts: [C2v; 8],
}

#[derive(Clone, Copy)]
struct C2Sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: c_float,
    i_a: usize,
    i_b: usize,
}

struct C2Simplex {
    a: C2Sv,
    b: C2Sv,
    c: C2Sv,
    d: C2Sv,
    div: c_float,
    count: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
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

fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
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
    let mut p = C2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2_v(0.0, 0.0); 8],
    };
    match shape_type {
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
            let verts = c2_bb_verts(bb);
            p.verts[0] = verts[0];
            p.verts[1] = verts[1];
            p.verts[2] = verts[2];
            p.verts[3] = verts[3];
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
    c2_v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_add(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
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

fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.s_a;
            *b = s.a.s_b;
        }
        2 => {
            *a = c2_add(
                c2_mulvs(s.a.s_a, den * s.a.u),
                c2_mulvs(s.b.s_a, den * s.b.u),
            );
            *b = c2_add(
                c2_mulvs(s.a.s_b, den * s.a.u),
                c2_mulvs(s.b.s_b, den * s.b.u),
            );
        }
        _ => {
            *a = c2_add(
                c2_add(
                    c2_mulvs(s.a.s_a, den * s.a.u),
                    c2_mulvs(s.b.s_a, den * s.b.u),
                ),
                c2_mulvs(s.c.s_a, den * s.c.u),
            );
            *b = c2_add(
                c2_add(
                    c2_mulvs(s.a.s_b, den * s.a.u),
                    c2_mulvs(s.b.s_b, den * s.b.u),
                ),
                c2_mulvs(s.c.s_b, den * s.c.u),
            );
        }
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

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
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
    use_radius: bool,
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
        a: C2Sv {
            s_a: c2_v(0.0, 0.0),
            s_b: c2_v(0.0, 0.0),
            p: c2_v(0.0, 0.0),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        b: C2Sv {
            s_a: c2_v(0.0, 0.0),
            s_b: c2_v(0.0, 0.0),
            p: c2_v(0.0, 0.0),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        c: C2Sv {
            s_a: c2_v(0.0, 0.0),
            s_b: c2_v(0.0, 0.0),
            p: c2_v(0.0, 0.0),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        d: C2Sv {
            s_a: c2_v(0.0, 0.0),
            s_b: c2_v(0.0, 0.0),
            p: c2_v(0.0, 0.0),
            u: 0.0,
            i_a: 0,
            i_b: 0,
        },
        div: 1.0,
        count: 1,
    };
    s.a.i_a = 0;
    s.a.i_b = 0;
    s.a.s_a = c2_mulxv(ax, p_a.verts[0]);
    s.a.s_b = c2_mulxv(bx, p_b.verts[0]);
    s.a.p = c2_sub(s.a.s_b, s.a.s_a);
    s.a.u = 1.0;
    let mut save_a: [usize; 3] = [0; 3];
    let mut save_b: [usize; 3] = [0; 3];
    let mut save_count: usize = 0;
    let mut d0: c_float = f32::MAX;
    let mut d1: c_float = f32::MAX;
    let mut iter = 0;
    let mut hit = false;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            let verts = [&s.a, &s.b, &s.c];
            save_a[i] = verts[i].i_a;
            save_b[i] = verts[i].i_b;
        }
        match s.count {
            1 => {}
            2 => c2_2(&mut s),
            _ => c2_3(&mut s),
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
        if c2_dot(d, d) < 1.1920929e-7 * 1.1920929e-7 {
            break;
        }
        let i_a = c2_support(&p_a.verts[..p_a.count], c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a]);
        let i_b = c2_support(&p_b.verts[..p_b.count], c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b]);
        let v = match s.count {
            0 => &mut s.a,
            1 => &mut s.b,
            _ => &mut s.c,
        };
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
    let mut a_out: C2v = c2_v(0.0, 0.0);
    let mut b_out: C2v = c2_v(0.0, 0.0);
    c2_witness(&s, &mut a_out, &mut b_out);
    let mut dist = c2_len(c2_sub(a_out, b_out));
    if hit {
        a_out = b_out;
        dist = 0.0;
    } else if use_radius {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > 1.1920929e-7 {
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
    if !out_a.is_null() {
        unsafe { *out_a = a_out };
    }
    if !out_b.is_null() {
        unsafe { *out_b = b_out };
    }
    dist
}

#[unsafe(no_mangle)]
pub extern "C" fn gjk(
    reverse: c_char,
    a: *mut C2v,
    b: *mut C2v,
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
    let bb = C2Aabb {
        min: c2_v(a1, a2),
        max: c2_v(a3, a4),
    };
    let cap = C2Capsule {
        a: c2_v(b1, b2),
        b: c2_v(b3, b4),
        r: b5,
    };
    if reverse != 0 {
        c2_gjk(
            &cap as *const _ as *const u8,
            C2Type::Capsule,
            std::ptr::null(),
            &bb as *const _ as *const u8,
            C2Type::Aabb,
            std::ptr::null(),
            a,
            b,
            true,
        );
    } else {
        c2_gjk(
            &bb as *const _ as *const u8,
            C2Type::Aabb,
            std::ptr::null(),
            &cap as *const _ as *const u8,
            C2Type::Capsule,
            std::ptr::null(),
            a,
            b,
            true,
        );
    }
}
