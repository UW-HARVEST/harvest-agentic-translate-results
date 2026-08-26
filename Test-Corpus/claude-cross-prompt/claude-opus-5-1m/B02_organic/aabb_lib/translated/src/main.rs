// Translated from C to Rust. Preserves exact behavior, including any quirks.

use std::io::{self, Read, Write};

#[derive(Copy, Clone, PartialEq, Eq)]
enum C2Type {
    Circle = 0,
    Aabb = 1,
    Capsule = 2,
}

#[derive(Copy, Clone, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Copy, Clone)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Copy, Clone)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Copy, Clone)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Copy, Clone, Default)]
struct C2GJKCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
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

#[derive(Copy, Clone)]
struct C2Proxy {
    radius: f32,
    count: i32,
    verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        C2Proxy {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(C2Circle),
    Aabb(C2AABB),
    Capsule(C2Capsule),
}

fn c2_bb_verts(out: &mut [C2v], bb: &C2AABB) {
    out[0] = bb.min;
    out[1] = c2_v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2_v(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: &Shape, p: &mut C2Proxy) {
    match shape {
        Shape::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        Shape::Aabb(bb) => {
            p.radius = 0.0;
            p.count = 4;
            c2_bb_verts(&mut p.verts, bb);
        }
        Shape::Capsule(c) => {
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

#[derive(Copy, Clone, Default)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: i32,
    i_b: i32,
}

#[derive(Copy, Clone, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: i32,
}

impl C2Simplex {
    fn get(&self, i: usize) -> C2sv {
        match i {
            0 => self.a,
            1 => self.b,
            2 => self.c,
            3 => self.d,
            _ => panic!("simplex index out of bounds"),
        }
    }
    fn set(&mut self, i: usize, v: C2sv) {
        match i {
            0 => self.a = v,
            1 => self.b = v,
            2 => self.c = v,
            3 => self.d = v,
            _ => panic!("simplex index out of bounds"),
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

fn c2_support(verts: &[C2v], count: i32, d: C2v) -> i32 {
    let mut imax: i32 = 0;
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
    let den = 1.0_f32 / s.div;
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
        3 => {
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
    let den = 1.0_f32 / s.div;
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

#[allow(clippy::too_many_arguments)]
fn c2_gjk(
    a_shape: &Shape,
    ax_ptr: Option<&C2x>,
    b_shape: &Shape,
    bx_ptr: Option<&C2x>,
    out_a: Option<&mut C2v>,
    out_b: Option<&mut C2v>,
    use_radius: bool,
    iterations: Option<&mut i32>,
    cache: Option<&mut C2GJKCache>,
) -> f32 {
    let ax = match ax_ptr {
        None => c2x_identity(),
        Some(p) => *p,
    };
    let bx = match bx_ptr {
        None => c2x_identity(),
        Some(p) => *p,
    };
    let mut p_a = C2Proxy::default();
    let mut p_b = C2Proxy::default();
    c2_make_proxy(a_shape, &mut p_a);
    c2_make_proxy(b_shape, &mut p_b);
    let mut s = C2Simplex::default();
    let mut cache_was_read = false;
    if let Some(ref c) = cache {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count {
                let i_a = c.i_a[i as usize];
                let i_b = c.i_b[i as usize];
                let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
                let mut v = s.get(i as usize);
                v.i_a = i_a;
                v.s_a = s_a;
                v.i_b = i_b;
                v.s_b = s_b;
                v.p = c2_sub(v.s_b, v.s_a);
                v.u = 0.0;
                s.set(i as usize, v);
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
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
    let mut save_a: [i32; 3] = [0; 3];
    let mut save_b: [i32; 3] = [0; 3];
    let mut save_count: i32;
    let mut d0: f32 = f32::from_bits(0x7F7FFFFF); // FLT_MAX
    let mut d1: f32;
    let mut iter: i32 = 0;
    let mut hit = false;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            let v = s.get(i as usize);
            save_a[i as usize] = v.i_a;
            save_b[i as usize] = v.i_b;
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
        if c2_dot(d, d) < 1.192_092_9e-7_f32 * 1.192_092_9e-7_f32 {
            break;
        }
        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
        let count_idx = s.count as usize;
        let mut v = s.get(count_idx);
        v.i_a = i_a;
        v.s_a = s_a;
        v.i_b = i_b;
        v.s_b = s_b;
        v.p = c2_sub(v.s_b, v.s_a);
        s.set(count_idx, v);
        let mut dup = false;
        for i in 0..save_count {
            if i_a == save_a[i as usize] && i_b == save_b[i as usize] {
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
    } else if use_radius {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > 1.192_092_9e-7_f32 {
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
    if let Some(c) = cache {
        c.metric = c2_gjk_simplex_metric(&s);
        c.count = s.count;
        for i in 0..s.count {
            let v = s.get(i as usize);
            c.i_a[i as usize] = v.i_a;
            c.i_b[i as usize] = v.i_b;
        }
        c.div = s.div;
    }
    if let Some(o) = out_a {
        *o = a;
    }
    if let Some(o) = out_b {
        *o = b;
    }
    if let Some(it) = iterations {
        *it = iter;
    }
    dist
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    if (d0 | d1 | d2 | d3) != 0 {
        0
    } else {
        1
    }
}

fn c2_aabb_to_capsule(a: C2AABB, b: C2Capsule) -> i32 {
    if c2_gjk(
        &Shape::Aabb(a),
        None,
        &Shape::Capsule(b),
        None,
        None,
        None,
        true,
        None,
        None,
    ) != 0.0
    {
        return 0;
    }
    1
}

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> i32 {
    if c2_gjk(
        &Shape::Capsule(a),
        None,
        &Shape::Capsule(b),
        None,
        None,
        None,
        true,
        None,
        None,
    ) != 0.0
    {
        return 0;
    }
    1
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> i32 {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let da = c2_dot(ap, n);
    let d2: f32;
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
    if d2 < r * r { 1 } else { 0 }
}

fn c2_collided(a: &Shape, type_a: C2Type, b: &Shape, type_b: C2Type) -> i32 {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => {
                if let (Shape::Circle(ca), Shape::Circle(cb)) = (a, b) {
                    return c2_circle_to_circle(*ca, *cb);
                }
                0
            }
            C2Type::Aabb => {
                if let (Shape::Circle(ca), Shape::Aabb(bb)) = (a, b) {
                    return c2_circle_to_aabb(*ca, *bb);
                }
                0
            }
            C2Type::Capsule => {
                if let (Shape::Circle(ca), Shape::Capsule(cap)) = (a, b) {
                    return c2_circle_to_capsule(*ca, *cap);
                }
                0
            }
        },
        C2Type::Aabb => match type_b {
            C2Type::Circle => {
                if let (Shape::Aabb(bb), Shape::Circle(cb)) = (a, b) {
                    return c2_circle_to_aabb(*cb, *bb);
                }
                0
            }
            C2Type::Aabb => {
                if let (Shape::Aabb(aa), Shape::Aabb(bb)) = (a, b) {
                    return c2_aabb_to_aabb(*aa, *bb);
                }
                0
            }
            C2Type::Capsule => {
                if let (Shape::Aabb(bb), Shape::Capsule(cap)) = (a, b) {
                    return c2_aabb_to_capsule(*bb, *cap);
                }
                0
            }
        },
        C2Type::Capsule => match type_b {
            C2Type::Circle => {
                if let (Shape::Capsule(cap), Shape::Circle(cb)) = (a, b) {
                    return c2_circle_to_capsule(*cb, *cap);
                }
                0
            }
            C2Type::Aabb => {
                if let (Shape::Capsule(cap), Shape::Aabb(bb)) = (a, b) {
                    return c2_aabb_to_capsule(*bb, *cap);
                }
                0
            }
            C2Type::Capsule => {
                if let (Shape::Capsule(ca), Shape::Capsule(cb)) = (a, b) {
                    return c2_capsule_to_capsule(*ca, *cb);
                }
                0
            }
        },
    }
}

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> i32 {
    let mut result: i32 = 0;

    let aabb_in = C2AABB {
        min: c2_v(min_x, min_y),
        max: c2_v(max_x, max_y),
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
        &Shape::Circle(circle),
        C2Type::Circle,
        &Shape::Aabb(aabb_in),
        C2Type::Aabb,
    );

    result += c2_collided(
        &Shape::Aabb(aabb),
        C2Type::Aabb,
        &Shape::Aabb(aabb_in),
        C2Type::Aabb,
    ) << 1;

    result += c2_collided(
        &Shape::Capsule(capsule),
        C2Type::Capsule,
        &Shape::Aabb(aabb_in),
        C2Type::Aabb,
    ) << 2;

    result
}

// scanf("%f", ...) style parser: read whitespace-delimited tokens and parse as f32.
fn read_floats(n: usize) -> Vec<f32> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();
    let mut out: Vec<f32> = Vec::with_capacity(n);
    for tok in input.split_ascii_whitespace() {
        if out.len() >= n {
            break;
        }
        // Mimic strtof: try to parse the prefix that forms a valid float.
        if let Ok(v) = tok.parse::<f32>() {
            out.push(v);
        } else {
            // Try to parse the longest valid prefix.
            let bytes = tok.as_bytes();
            let mut end = bytes.len();
            while end > 0 {
                if let Ok(v) = std::str::from_utf8(&bytes[..end])
                    .unwrap_or("")
                    .parse::<f32>()
                {
                    out.push(v);
                    break;
                }
                end -= 1;
            }
            if end == 0 {
                break;
            }
        }
    }
    out
}

fn main() {
    let vals = read_floats(4);
    if vals.len() < 4 {
        return;
    }
    let result = aabb(vals[0], vals[1], vals[2], vals[3]);
    let stdout = io::stdout();
    let mut h = stdout.lock();
    writeln!(h, "{}", result).ok();
}
