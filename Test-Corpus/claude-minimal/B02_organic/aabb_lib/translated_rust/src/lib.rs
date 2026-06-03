#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub enum C2Type {
    Circle = 0,
    Aabb = 1,
    Capsule = 2,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(C2Circle),
    Aabb(C2AABB),
    Capsule(C2Capsule),
}

impl Shape {
    fn shape_type(&self) -> C2Type {
        match self {
            Shape::Circle(_) => C2Type::Circle,
            Shape::Aabb(_) => C2Type::Aabb,
            Shape::Capsule(_) => C2Type::Capsule,
        }
    }
}

pub fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

pub fn c2_mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

pub fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

pub fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

pub fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

pub fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

pub fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

pub fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

pub fn c2x_identity() -> C2x {
    C2x {
        p: c2v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [C2v; 8],
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

pub fn c2_bb_verts(out: &mut [C2v], bb: &C2AABB) {
    out[0] = bb.min;
    out[1] = c2v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v(bb.min.x, bb.max.y);
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

#[derive(Clone, Copy, Debug, Default)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct C2Simplex {
    pub a: C2sv,
    pub b: C2sv,
    pub c: C2sv,
    pub d: C2sv,
    pub div: f32,
    pub count: i32,
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

pub fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

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

pub fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

pub fn c2_add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

pub fn c2_mulxv(a: C2x, b: C2v) -> C2v {
    c2_add(c2_mulrv(a.r, b), a.p)
}

pub fn c2_2(s: &mut C2Simplex) {
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

pub fn c2_3(s: &mut C2Simplex) {
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

pub fn c2_neg(a: C2v) -> C2v {
    c2v(-a.x, -a.y)
}

pub fn c2_skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

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
        _ => c2v(0.0, 0.0),
    }
}

pub fn c2_support(verts: &[C2v], count: i32, d: C2v) -> i32 {
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

pub fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
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
            *a = c2v(0.0, 0.0);
            *b = c2v(0.0, 0.0);
        }
    }
}

pub fn c2_div(a: C2v, b: f32) -> C2v {
    c2_mulvs(a, 1.0 / b)
}

pub fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
}

pub fn c2_l(s: &C2Simplex) -> C2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2v(0.0, 0.0),
    }
}

pub fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

const FLT_MAX: f32 = 3.402_823_5e+38;
const FLT_EPSILON: f32 = 1.192_092_9e-7;

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

    // We need to handle Option<&mut C2GJKCache> in two different reads.
    // We'll re-borrow as needed; keep it as Option<&mut> pattern.
    if let Some(ref c) = cache.as_ref() {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let i_a = c.iA[i];
                let i_b = c.iB[i];
                let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
                let mut v = s.get(i);
                v.iA = i_a;
                v.sA = s_a;
                v.iB = i_b;
                v.sB = s_b;
                v.p = c2_sub(v.sB, v.sA);
                v.u = 0.0;
                s.set(i, v);
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
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2_mulxv(ax, p_a.verts[0]);
        s.a.sB = c2_mulxv(bx, p_b.verts[0]);
        s.a.p = c2_sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a: [i32; 3] = [0; 3];
    let mut save_b: [i32; 3] = [0; 3];
    let mut save_count: i32;
    let mut d0 = FLT_MAX;
    let mut d1;
    let mut iter: i32 = 0;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = s.get(i);
            save_a[i] = v.iA;
            save_b[i] = v.iB;
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
        if c2_dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }
        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);

        let idx = s.count as usize;
        let mut v = s.get(idx);
        v.iA = i_a;
        v.sA = s_a;
        v.iB = i_b;
        v.sB = s_b;
        v.p = c2_sub(v.sB, v.sA);
        s.set(idx, v);

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
    } else if use_radius {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > FLT_EPSILON {
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
            let v = s.get(i);
            c.iA[i] = v.iA;
            c.iB[i] = v.iB;
        }
        c.div = s.div;
    }

    if let Some(out) = out_a {
        *out = a;
    }
    if let Some(out) = out_b {
        *out = b;
    }
    if let Some(it) = iterations {
        *it = iter;
    }

    dist
}

pub fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> bool {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    !(d0 || d1 || d2 || d3)
}

pub fn c2_aabb_to_capsule(a: C2AABB, b: C2Capsule) -> bool {
    let dist = c2_gjk(
        &Shape::Aabb(a),
        None,
        &Shape::Capsule(b),
        None,
        None,
        None,
        true,
        None,
        None,
    );
    dist == 0.0
}

pub fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> bool {
    let dist = c2_gjk(
        &Shape::Capsule(a),
        None,
        &Shape::Capsule(b),
        None,
        None,
        None,
        true,
        None,
        None,
    );
    dist == 0.0
}

pub fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> bool {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    d2 < r2
}

pub fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> bool {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}

pub fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> bool {
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

pub fn c2_collided(a: &Shape, b: &Shape) -> bool {
    match (a, b) {
        (Shape::Circle(ca), Shape::Circle(cb)) => c2_circle_to_circle(*ca, *cb),
        (Shape::Circle(ca), Shape::Aabb(bb)) => c2_circle_to_aabb(*ca, *bb),
        (Shape::Circle(ca), Shape::Capsule(cb)) => c2_circle_to_capsule(*ca, *cb),
        (Shape::Aabb(bb), Shape::Circle(ca)) => c2_circle_to_aabb(*ca, *bb),
        (Shape::Aabb(ba), Shape::Aabb(bb)) => c2_aabb_to_aabb(*ba, *bb),
        (Shape::Aabb(ba), Shape::Capsule(cb)) => c2_aabb_to_capsule(*ba, *cb),
        (Shape::Capsule(ca), Shape::Circle(cb)) => c2_circle_to_capsule(*cb, *ca),
        (Shape::Capsule(ca), Shape::Aabb(bb)) => c2_aabb_to_capsule(*bb, *ca),
        (Shape::Capsule(ca), Shape::Capsule(cb)) => c2_capsule_to_capsule(*ca, *cb),
    }
}

#[no_mangle]
pub extern "C" fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> i32 {
    let mut result: i32 = 0;

    let aabb_in = C2AABB {
        min: c2v(min_x, min_y),
        max: c2v(max_x, max_y),
    };

    let circle = C2Circle {
        p: c2v(-70.0, 0.0),
        r: 20.0,
    };

    let aabb_shape = C2AABB {
        min: c2v(-40.0, -40.0),
        max: c2v(-15.0, -15.0),
    };

    let capsule = C2Capsule {
        a: c2v(-40.0, 40.0),
        b: c2v(-20.0, 100.0),
        r: 10.0,
    };

    result += c2_collided(&Shape::Circle(circle), &Shape::Aabb(aabb_in)) as i32;
    result += (c2_collided(&Shape::Aabb(aabb_shape), &Shape::Aabb(aabb_in)) as i32) << 1;
    result += (c2_collided(&Shape::Capsule(capsule), &Shape::Aabb(aabb_in)) as i32) << 2;

    result
}
