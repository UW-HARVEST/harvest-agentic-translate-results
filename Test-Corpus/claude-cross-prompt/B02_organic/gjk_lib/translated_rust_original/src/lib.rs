// Translated from c_src/src/lib.c

#[derive(Copy, Clone, Default, Debug)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum C2Type {
    Circle,
    Aabb,
    Capsule,
}

#[derive(Copy, Clone, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[derive(Copy, Clone, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[derive(Copy, Clone, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[derive(Copy, Clone, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[derive(Copy, Clone, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[derive(Copy, Clone, Default)]
pub struct C2GjkCache {
    pub metric: f32,
    pub count: i32,
    pub i_a: [i32; 3],
    pub i_b: [i32; 3],
    pub div: f32,
}

pub enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

#[inline]
pub fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[inline]
pub fn c2_mulvs(a: C2v, b: f32) -> C2v {
    C2v {
        x: a.x * b,
        y: a.y * b,
    }
}

#[inline]
pub fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

#[inline]
pub fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[inline]
pub fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
pub fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

#[inline]
pub fn c2x_identity() -> C2x {
    C2x {
        p: c2_v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

#[derive(Copy, Clone, Default)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [C2v; 8],
}

pub fn c2_bb_verts(out: &mut [C2v], bb: &C2Aabb) {
    out[0] = bb.min;
    out[1] = c2_v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2_v(bb.min.x, bb.max.y);
}

pub fn c2_make_proxy(shape: &Shape, p: &mut C2Proxy) {
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
pub struct C2sv {
    pub s_a: C2v,
    pub s_b: C2v,
    pub p: C2v,
    pub u: f32,
    pub i_a: i32,
    pub i_b: i32,
}

#[derive(Copy, Clone, Default)]
pub struct C2Simplex {
    pub a: C2sv,
    pub b: C2sv,
    pub c: C2sv,
    pub d: C2sv,
    pub div: f32,
    pub count: i32,
}

#[inline]
pub fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

#[inline]
pub fn c2_det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

pub fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

#[inline]
pub fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
pub fn c2_add(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

#[inline]
pub fn c2_mulxv(a: C2x, b: C2v) -> C2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

pub fn c22(s: &mut C2Simplex) {
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

pub fn c23(s: &mut C2Simplex) {
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

#[inline]
pub fn c2_neg(a: C2v) -> C2v {
    c2_v(-a.x, -a.y)
}

#[inline]
pub fn c2_skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[inline]
pub fn c2_ccw90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

pub fn c2_d(s: &C2Simplex) -> C2v {
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

pub fn c2_support(verts: &[C2v], count: i32, d: C2v) -> i32 {
    let mut imax: i32 = 0;
    let mut dmax = c2_dot(verts[0], d);
    let mut i: i32 = 1;
    while i < count {
        let dot = c2_dot(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

pub fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
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

#[inline]
pub fn c2_div(a: C2v, b: f32) -> C2v {
    c2_mulvs(a, 1.0 / b)
}

#[inline]
pub fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
}

pub fn c2_l(s: &C2Simplex) -> C2v {
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

#[inline]
pub fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn simplex_get(s: &C2Simplex, i: usize) -> C2sv {
    match i {
        0 => s.a,
        1 => s.b,
        2 => s.c,
        _ => s.d,
    }
}

fn simplex_set(s: &mut C2Simplex, i: usize, v: C2sv) {
    match i {
        0 => s.a = v,
        1 => s.b = v,
        2 => s.c = v,
        _ => s.d = v,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn c2_gjk(
    a_shape: &Shape,
    ax_ptr: Option<&C2x>,
    b_shape: &Shape,
    bx_ptr: Option<&C2x>,
    out_a: Option<&mut C2v>,
    out_b: Option<&mut C2v>,
    use_radius: bool,
    iterations: Option<&mut i32>,
    cache: Option<&mut C2GjkCache>,
) -> f32 {
    let ax = match ax_ptr {
        Some(p) => *p,
        None => c2x_identity(),
    };
    let bx = match bx_ptr {
        Some(p) => *p,
        None => c2x_identity(),
    };
    let mut p_a = C2Proxy::default();
    let mut p_b = C2Proxy::default();
    c2_make_proxy(a_shape, &mut p_a);
    c2_make_proxy(b_shape, &mut p_b);
    let mut s = C2Simplex::default();
    let mut cache_was_read = 0;

    // We need a reborrow-friendly cache reference for two uses.
    // Read phase
    if let Some(ref c) = cache {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let i_a = c.i_a[i];
                let i_b = c.i_b[i];
                let s_a_v = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b_v = c2_mulxv(bx, p_b.verts[i_b as usize]);
                let mut v = simplex_get(&s, i);
                v.i_a = i_a;
                v.s_a = s_a_v;
                v.i_b = i_b;
                v.s_b = s_b_v;
                v.p = c2_sub(v.s_b, v.s_a);
                v.u = 0.0;
                simplex_set(&mut s, i, v);
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8_f32) {
                cache_was_read = 1;
            }
        }
    }
    if cache_was_read == 0 {
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
    let mut d0: f32 = 3.402_823_466_385_288_6e+38_f32;
    let mut d1: f32;
    let mut iter: i32 = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = simplex_get(&s, i);
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
        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a_v = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b_v = c2_mulxv(bx, p_b.verts[i_b as usize]);
        let count_idx = s.count as usize;
        let mut v = simplex_get(&s, count_idx);
        v.i_a = i_a;
        v.s_a = s_a_v;
        v.i_b = i_b;
        v.s_b = s_b_v;
        v.p = c2_sub(v.s_b, v.s_a);
        simplex_set(&mut s, count_idx, v);
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
    let mut a = C2v::default();
    let mut b = C2v::default();
    c2_witness(&s, &mut a, &mut b);
    let mut dist = c2_len(c2_sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius {
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
            let v = simplex_get(&s, i);
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

#[allow(clippy::too_many_arguments)]
pub fn gjk(
    reverse: bool,
    a: &mut C2v,
    b: &mut C2v,
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
    let bb = C2Aabb {
        min: c2_v(a1, a2),
        max: c2_v(a3, a4),
    };
    let cap = C2Capsule {
        a: c2_v(b1, b2),
        b: c2_v(b3, b4),
        r: b5,
    };

    let bb_shape = Shape::Aabb(bb);
    let cap_shape = Shape::Capsule(cap);

    if reverse {
        c2_gjk(
            &cap_shape,
            None,
            &bb_shape,
            None,
            Some(a),
            Some(b),
            true,
            None,
            None,
        );
    } else {
        c2_gjk(
            &bb_shape,
            None,
            &cap_shape,
            None,
            Some(a),
            Some(b),
            true,
            None,
            None,
        );
    }
}
