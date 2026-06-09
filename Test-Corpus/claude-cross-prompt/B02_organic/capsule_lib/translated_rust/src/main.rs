use std::io::{self, Read, Write};

#[derive(Copy, Clone, PartialEq, Eq)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
}

#[derive(Copy, Clone, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Copy, Clone, Default)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone, Default)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Copy, Clone, Default)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Copy, Clone, Default)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[derive(Copy, Clone, Default)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Copy, Clone, Default)]
struct C2GjkCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
}

enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(a: C2v, b: f32) -> C2v {
    C2v {
        x: a.x * b,
        y: a.y * b,
    }
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
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
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

#[derive(Copy, Clone, Default)]
struct C2Proxy {
    radius: f32,
    count: i32,
    verts: [C2v; 8],
}

fn c2_bb_verts(out: &mut [C2v], bb: &C2Aabb) {
    out[0] = bb.min;
    out[1] = c2v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v(bb.min.x, bb.max.y);
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
    verts: [C2sv; 4], // a, b, c, d
    div: f32,
    count: i32,
}

fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2_len(c2_sub(s.verts[1].p, s.verts[0].p)),
        3 => c2_det2(
            c2_sub(s.verts[1].p, s.verts[0].p),
            c2_sub(s.verts[2].p, s.verts[0].p),
        ),
        _ => 0.0,
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
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

fn c22(s: &mut C2Simplex) {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = c2_dot(b, c2_sub(b, a));
    let v = c2_dot(a, c2_sub(a, b));
    if v <= 0.0 {
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else {
        s.verts[0].u = u;
        s.verts[1].u = v;
        s.div = u + v;
        s.count = 2;
    }
}

fn c23(s: &mut C2Simplex) {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let c = s.verts[2].p;
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
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u_ab <= 0.0 && v_bc <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u_bc <= 0.0 && v_ca <= 0.0 {
        s.verts[0] = s.verts[2];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
        s.verts[0].u = u_ab;
        s.verts[1].u = v_ab;
        s.div = u_ab + v_ab;
        s.count = 2;
    } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[1] = s.verts[2];
        s.verts[0].u = u_bc;
        s.verts[1].u = v_bc;
        s.div = u_bc + v_bc;
        s.count = 2;
    } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
        s.verts[1] = s.verts[0];
        s.verts[0] = s.verts[2];
        s.verts[0].u = u_ca;
        s.verts[1].u = v_ca;
        s.div = u_ca + v_ca;
        s.count = 2;
    } else {
        s.verts[0].u = u_abc;
        s.verts[1].u = v_abc;
        s.verts[2].u = w_abc;
        s.div = u_abc + v_abc + w_abc;
        s.count = 3;
    }
}

fn c2_neg(a: C2v) -> C2v {
    c2v(-a.x, -a.y)
}

fn c2_skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

fn c2_ccw90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

fn c2_d(s: &C2Simplex) -> C2v {
    match s.count {
        1 => c2_neg(s.verts[0].p),
        2 => {
            let ab = c2_sub(s.verts[1].p, s.verts[0].p);
            if c2_det2(ab, c2_neg(s.verts[0].p)) > 0.0 {
                c2_skew(ab)
            } else {
                c2_ccw90(ab)
            }
        }
        _ => c2v(0.0, 0.0),
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
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.verts[0].s_a;
            *b = s.verts[0].s_b;
        }
        2 => {
            *a = c2_add(
                c2_mulvs(s.verts[0].s_a, den * s.verts[0].u),
                c2_mulvs(s.verts[1].s_a, den * s.verts[1].u),
            );
            *b = c2_add(
                c2_mulvs(s.verts[0].s_b, den * s.verts[0].u),
                c2_mulvs(s.verts[1].s_b, den * s.verts[1].u),
            );
        }
        3 => {
            *a = c2_add(
                c2_add(
                    c2_mulvs(s.verts[0].s_a, den * s.verts[0].u),
                    c2_mulvs(s.verts[1].s_a, den * s.verts[1].u),
                ),
                c2_mulvs(s.verts[2].s_a, den * s.verts[2].u),
            );
            *b = c2_add(
                c2_add(
                    c2_mulvs(s.verts[0].s_b, den * s.verts[0].u),
                    c2_mulvs(s.verts[1].s_b, den * s.verts[1].u),
                ),
                c2_mulvs(s.verts[2].s_b, den * s.verts[2].u),
            );
        }
        _ => {
            *a = c2v(0.0, 0.0);
            *b = c2v(0.0, 0.0);
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
        1 => s.verts[0].p,
        2 => c2_add(
            c2_mulvs(s.verts[0].p, den * s.verts[0].u),
            c2_mulvs(s.verts[1].p, den * s.verts[1].u),
        ),
        _ => c2v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[allow(clippy::too_many_arguments)]
fn c2_gjk(
    a_shape: &Shape,
    ax_ptr: Option<&C2x>,
    b_shape: &Shape,
    bx_ptr: Option<&C2x>,
    out_a: Option<&mut C2v>,
    out_b: Option<&mut C2v>,
    use_radius: i32,
    iterations: Option<&mut i32>,
    cache: Option<&mut C2GjkCache>,
) -> f32 {
    let ax = match ax_ptr {
        Some(a) => *a,
        None => c2x_identity(),
    };
    let bx = match bx_ptr {
        Some(b) => *b,
        None => c2x_identity(),
    };

    let mut p_a = C2Proxy::default();
    let mut p_b = C2Proxy::default();
    c2_make_proxy(a_shape, &mut p_a);
    c2_make_proxy(b_shape, &mut p_b);

    let mut s = C2Simplex::default();
    let mut cache_was_read = 0;

    if let Some(ref c) = cache {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let i_a = c.i_a[i];
                let i_b = c.i_b[i];
                let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
                let v = &mut s.verts[i];
                v.i_a = i_a;
                v.s_a = s_a;
                v.i_b = i_b;
                v.s_b = s_b;
                v.p = c2_sub(v.s_b, v.s_a);
                v.u = 0.0;
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = 1;
            }
        }
    }

    if cache_was_read == 0 {
        s.verts[0].i_a = 0;
        s.verts[0].i_b = 0;
        s.verts[0].s_a = c2_mulxv(ax, p_a.verts[0]);
        s.verts[0].s_b = c2_mulxv(bx, p_b.verts[0]);
        s.verts[0].p = c2_sub(s.verts[0].s_b, s.verts[0].s_a);
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a: [i32; 3] = [0; 3];
    let mut save_b: [i32; 3] = [0; 3];
    let mut save_count: i32 = 0;
    let mut d0: f32 = 3.402_823_466_385_288_6e+38_f32;
    let mut d1: f32;
    let mut iter: i32 = 0;
    let mut hit: i32 = 0;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            save_a[i] = s.verts[i].i_a;
            save_b[i] = s.verts[i].i_b;
        }
        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
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
        if c2_dot(d, d)
            < 1.192_092_895_507_812_5e-7_f32 * 1.192_092_895_507_812_5e-7_f32
        {
            break;
        }
        let i_a = c2_support(
            &p_a.verts,
            p_a.count,
            c2_mulrv_t(ax.r, c2_neg(d)),
        );
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);

        let idx = s.count as usize;
        let v = &mut s.verts[idx];
        v.i_a = i_a;
        v.s_a = s_a;
        v.i_b = i_b;
        v.s_b = s_b;
        v.p = c2_sub(v.s_b, v.s_a);

        let mut dup = 0;
        for i in 0..save_count as usize {
            if i_a == save_a[i] && i_b == save_b[i] {
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

    let _ = save_count; // silence unused

    let mut a = C2v::default();
    let mut b = C2v::default();
    c2_witness(&s, &mut a, &mut b);
    let mut dist = c2_len(c2_sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > 1.192_092_895_507_812_5e-7_f32 {
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
        for i in 0..s.count as usize {
            let v = &s.verts[i];
            c.i_a[i] = v.i_a;
            c.i_b[i] = v.i_b;
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

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    (!((d0 | d1 | d2 | d3) != 0)) as i32
}

fn c2_aabb_to_capsule(a: C2Aabb, b: C2Capsule) -> i32 {
    if c2_gjk(
        &Shape::Aabb(a),
        None,
        &Shape::Capsule(b),
        None,
        None,
        None,
        1,
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
        1,
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
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    (d2 < r2) as i32
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> i32 {
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

fn c2_collided(a: &Shape, b: &Shape) -> i32 {
    match (a, b) {
        (Shape::Circle(a), Shape::Circle(b)) => c2_circle_to_circle(*a, *b),
        (Shape::Circle(a), Shape::Aabb(b)) => c2_circle_to_aabb(*a, *b),
        (Shape::Circle(a), Shape::Capsule(b)) => c2_circle_to_capsule(*a, *b),
        (Shape::Aabb(a), Shape::Circle(b)) => c2_circle_to_aabb(*b, *a),
        (Shape::Aabb(a), Shape::Aabb(b)) => c2_aabb_to_aabb(*a, *b),
        (Shape::Aabb(a), Shape::Capsule(b)) => c2_aabb_to_capsule(*a, *b),
        (Shape::Capsule(a), Shape::Circle(b)) => c2_circle_to_capsule(*b, *a),
        (Shape::Capsule(a), Shape::Aabb(b)) => c2_aabb_to_capsule(*b, *a),
        (Shape::Capsule(a), Shape::Capsule(b)) => c2_capsule_to_capsule(*a, *b),
    }
}

#[allow(dead_code)]
fn _suppress_unused() {
    // Reference unused C2Type so it isn't dropped if needed
    let _ = (C2Type::Circle, C2Type::Aabb, C2Type::Capsule);
}

fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> i32 {
    let mut result: i32 = 0;

    let capsule_in = C2Capsule {
        a: c2v(min_x, min_y),
        b: c2v(max_x, max_y),
        r,
    };

    let circle = C2Circle {
        p: c2v(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = C2Aabb {
        min: c2v(-40.0, -40.0),
        max: c2v(-15.0, -15.0),
    };

    let capsule = C2Capsule {
        a: c2v(-40.0, 40.0),
        b: c2v(-20.0, 100.0),
        r: 10.0,
    };

    result += c2_collided(&Shape::Circle(circle), &Shape::Capsule(capsule_in));
    result += c2_collided(&Shape::Aabb(aabb), &Shape::Capsule(capsule_in)) << 1;
    result += c2_collided(&Shape::Capsule(capsule), &Shape::Capsule(capsule_in)) << 2;

    result
}

fn read_floats_scanf(n: usize) -> Vec<f32> {
    // C scanf("%f") reads whitespace-separated tokens (across newlines).
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok();
    let mut out = Vec::with_capacity(n);
    let mut iter = buf.split_ascii_whitespace();
    for _ in 0..n {
        match iter.next() {
            Some(tok) => match tok.parse::<f32>() {
                Ok(v) => out.push(v),
                Err(_) => out.push(0.0),
            },
            None => out.push(0.0),
        }
    }
    out
}

fn main() {
    let vals = read_floats_scanf(5);
    let result = capsule(vals[0], vals[1], vals[2], vals[3], vals[4]);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", result).unwrap();
}
