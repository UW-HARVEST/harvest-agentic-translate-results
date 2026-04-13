use std::os::raw::{c_int, c_float};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE,
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct C2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
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
    c2_v(a.x.max(b.x), a.y.max(b.y))
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.min(b.x), a.y.min(b.y))
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

fn c2_make_proxy(shape: &[f32], typ: C2_TYPE) -> C2Proxy {
    let mut p = C2Proxy::default();
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            p.radius = shape[2];
            p.count = 1;
            p.verts[0] = c2_v(shape[0], shape[1]);
        }
        C2_TYPE::C2_TYPE_AABB => {
            p.radius = 0.0;
            p.count = 4;
            let bb = C2AABB {
                min: c2_v(shape[0], shape[1]),
                max: c2_v(shape[2], shape[3]),
            };
            let verts = c2_bb_verts(&bb);
            p.verts[..4].copy_from_slice(&verts);
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            p.radius = shape[4];
            p.count = 2;
            p.verts[0] = c2_v(shape[0], shape[1]);
            p.verts[1] = c2_v(shape[2], shape[3]);
        }
    }
    p
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
    let mut imax = 0;
    let mut dmax = c2_dot(verts[0], d);
    for i in 1..verts.len() {
        let dot = c2_dot(verts[i], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

fn c2_witness(s: &C2Simplex) -> (C2v, C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => (s.a.sA, s.a.sB),
        2 => {
            let a = c2_add(c2_mulvs(s.a.sA, den * s.a.u), c2_mulvs(s.b.sA, den * s.b.u));
            let b = c2_add(c2_mulvs(s.a.sB, den * s.a.u), c2_mulvs(s.b.sB, den * s.b.u));
            (a, b)
        }
        3 => {
            let a = c2_add(
                c2_add(c2_mulvs(s.a.sA, den * s.a.u), c2_mulvs(s.b.sA, den * s.b.u)),
                c2_mulvs(s.c.sA, den * s.c.u),
            );
            let b = c2_add(
                c2_add(c2_mulvs(s.a.sB, den * s.a.u), c2_mulvs(s.b.sB, den * s.b.u)),
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
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_gjk(
    a_verts: &[C2v],
    a_count: usize,
    a_radius: f32,
    b_verts: &[C2v],
    b_count: usize,
    b_radius: f32,
    use_radius: bool,
) -> f32 {
    let ax = c2_x_identity();
    let bx = c2_x_identity();

    let mut s = C2Simplex::default();
    s.a.iA = 0;
    s.a.iB = 0;
    s.a.sA = c2_mulxv(ax, a_verts[0]);
    s.a.sB = c2_mulxv(bx, b_verts[0]);
    s.a.p = c2_sub(s.a.sB, s.a.sA);
    s.a.u = 1.0;
    s.div = 1.0;
    s.count = 1;

    let mut save_a = [0usize; 3];
    let mut save_b = [0usize; 3];
    let mut d0 = f32::MAX;
    let mut d1 = f32::MAX;
    let mut hit = false;

    for _ in 0..20 {
        let save_count = s.count;
        for i in 0..save_count {
            save_a[i] = match i {
                0 => s.a.iA,
                1 => s.b.iA,
                2 => s.c.iA,
                _ => 0,
            };
            save_b[i] = match i {
                0 => s.a.iB,
                1 => s.b.iB,
                2 => s.c.iB,
                _ => 0,
            };
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
        if c2_dot(d, d) < 1e-14 {
            break;
        }

        let i_a = c2_support(&a_verts[..a_count], c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, a_verts[i_a]);
        let i_b = c2_support(&b_verts[..b_count], c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, b_verts[i_b]);

        let v = match s.count {
            1 => &mut s.b,
            2 => &mut s.c,
            _ => &mut s.d,
        };
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
    }

    let (mut a, mut b) = c2_witness(&s);
    let mut dist = c2_len(c2_sub(a, b));

    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius {
        let r_a = a_radius;
        let r_b = b_radius;
        if dist > r_a + r_b && dist > 1e-7 {
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

    dist
}

fn c2_aabb_to_aabb(a: &C2AABB, b: &C2AABB) -> bool {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    !(d0 || d1 || d2 || d3)
}

fn c2_aabb_to_capsule(a: &C2AABB, b: &C2Capsule) -> bool {
    let a_verts = c2_bb_verts(a);
    let b_verts = [b.a, b.b];
    let dist = c2_gjk(&a_verts, 4, 0.0, &b_verts, 2, b.r, true);
    dist == 0.0
}

fn c2_capsule_to_capsule(a: &C2Capsule, b: &C2Capsule) -> bool {
    let a_verts = [a.a, a.b];
    let b_verts = [b.a, b.b];
    let dist = c2_gjk(&a_verts, 2, a.r, &b_verts, 2, b.r, true);
    dist == 0.0
}

fn c2_circle_to_circle(a: &C2Circle, b: &C2Circle) -> bool {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let r2 = a.r + b.r;
    d2 < r2 * r2
}

fn c2_circle_to_aabb(a: &C2Circle, b: &C2AABB) -> bool {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}

fn c2_circle_to_capsule(a: &C2Circle, b: &C2Capsule) -> bool {
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

fn c2_collided(a: &[f32], type_a: C2_TYPE, b: &[f32], type_b: C2_TYPE) -> bool {
    match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let ca = C2Circle { p: c2_v(a[0], a[1]), r: a[2] };
            match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    let cb = C2Circle { p: c2_v(b[0], b[1]), r: b[2] };
                    c2_circle_to_circle(&ca, &cb)
                }
                C2_TYPE::C2_TYPE_AABB => {
                    let bb = C2AABB { min: c2_v(b[0], b[1]), max: c2_v(b[2], b[3]) };
                    c2_circle_to_aabb(&ca, &bb)
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    let cap = C2Capsule { a: c2_v(b[0], b[1]), b: c2_v(b[2], b[3]), r: b[4] };
                    c2_circle_to_capsule(&ca, &cap)
                }
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            let aa = C2AABB { min: c2_v(a[0], a[1]), max: c2_v(a[2], a[3]) };
            match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    let cb = C2Circle { p: c2_v(b[0], b[1]), r: b[2] };
                    c2_circle_to_aabb(&cb, &aa)
                }
                C2_TYPE::C2_TYPE_AABB => {
                    let bb = C2AABB { min: c2_v(b[0], b[1]), max: c2_v(b[2], b[3]) };
                    c2_aabb_to_aabb(&aa, &bb)
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    let cap = C2Capsule { a: c2_v(b[0], b[1]), b: c2_v(b[2], b[3]), r: b[4] };
                    c2_aabb_to_capsule(&aa, &cap)
                }
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let capa = C2Capsule { a: c2_v(a[0], a[1]), b: c2_v(a[2], a[3]), r: a[4] };
            match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    let cb = C2Circle { p: c2_v(b[0], b[1]), r: b[2] };
                    c2_circle_to_capsule(&cb, &capa)
                }
                C2_TYPE::C2_TYPE_AABB => {
                    let bb = C2AABB { min: c2_v(b[0], b[1]), max: c2_v(b[2], b[3]) };
                    c2_aabb_to_capsule(&bb, &capa)
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    let capb = C2Capsule { a: c2_v(b[0], b[1]), b: c2_v(b[2], b[3]), r: b[4] };
                    c2_capsule_to_capsule(&capa, &capb)
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omni_collide(
    type_a: C2_TYPE,
    a1: c_float,
    a2: c_float,
    a3: c_float,
    a4: c_float,
    a5: c_float,
    type_b: C2_TYPE,
    b1: c_float,
    b2: c_float,
    b3: c_float,
    b4: c_float,
    b5: c_float,
) -> c_int {
    let a = match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => vec![a1, a2, a3],
        C2_TYPE::C2_TYPE_AABB => vec![a1, a2, a3, a4],
        C2_TYPE::C2_TYPE_CAPSULE => vec![a1, a2, a3, a4, a5],
    };
    let b = match type_b {
        C2_TYPE::C2_TYPE_CIRCLE => vec![b1, b2, b3],
        C2_TYPE::C2_TYPE_AABB => vec![b1, b2, b3, b4],
        C2_TYPE::C2_TYPE_CAPSULE => vec![b1, b2, b3, b4, b5],
    };

    if c2_collided(&a, type_a, &b, type_b) {
        1
    } else {
        0
    }
}
