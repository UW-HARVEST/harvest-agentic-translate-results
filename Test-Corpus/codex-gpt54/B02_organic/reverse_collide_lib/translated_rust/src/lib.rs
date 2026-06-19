use std::ffi::c_int;

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2v {
    x: f32,
    y: f32,
}

impl C2v {
    const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2GjkCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[derive(Clone, Copy)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
}

#[derive(Clone, Copy)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        Self {
            radius: 0.0,
            count: 0,
            verts: [C2v::ZERO; 8],
        }
    }
}

#[derive(Clone, Copy, Default)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[derive(Clone, Copy, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
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

fn c2_bb_verts(out: &mut [C2v; 8], bb: &C2Aabb) {
    out[0] = bb.min;
    out[1] = c2_v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2_v(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: ShapeRef<'_>, type_: C2Type, p: &mut C2Proxy) {
    match (type_, shape) {
        (C2Type::Circle, ShapeRef::Circle(c)) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        (C2Type::Aabb, ShapeRef::Aabb(bb)) => {
            p.radius = 0.0;
            p.count = 4;
            c2_bb_verts(&mut p.verts, bb);
        }
        (C2Type::Capsule, ShapeRef::Capsule(c)) => {
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
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
        1 => 0.0,
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

fn c2_support(verts: &[C2v; 8], count: c_int, d: C2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2_dot(verts[0], d);
    let mut i = 1;
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

fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0 / s.div;
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
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[derive(Clone, Copy)]
enum ShapeRef<'a> {
    Circle(&'a C2Circle),
    Aabb(&'a C2Aabb),
    Capsule(&'a C2Capsule),
}

fn c2_gjk(
    a_shape: ShapeRef<'_>,
    type_a: C2Type,
    ax_ptr: Option<&C2x>,
    b_shape: ShapeRef<'_>,
    type_b: C2Type,
    bx_ptr: Option<&C2x>,
    out_a: Option<&mut C2v>,
    out_b: Option<&mut C2v>,
    use_radius: c_int,
    iterations: Option<&mut c_int>,
    cache: Option<&mut C2GjkCache>,
) -> f32 {
    let ax = if let Some(ax) = ax_ptr { *ax } else { c2x_identity() };
    let bx = if let Some(bx) = bx_ptr { *bx } else { c2x_identity() };

    let mut p_a = C2Proxy::default();
    let mut p_b = C2Proxy::default();
    c2_make_proxy(a_shape, type_a, &mut p_a);
    c2_make_proxy(b_shape, type_b, &mut p_b);

    let mut s = C2Simplex::default();
    let mut cache_was_read = 0;
    if let Some(cache_ref) = cache.as_ref() {
        let cache_was_good = if cache_ref.count != 0 { 1 } else { 0 };
        if cache_was_good != 0 {
            let mut i = 0;
            while i < cache_ref.count {
                let i_a = cache_ref.i_a[i as usize];
                let i_b = cache_ref.i_b[i as usize];
                let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
                match i {
                    0 => {
                        s.a.i_a = i_a;
                        s.a.s_a = s_a;
                        s.a.i_b = i_b;
                        s.a.s_b = s_b;
                        s.a.p = c2_sub(s_b, s_a);
                        s.a.u = 0.0;
                    }
                    1 => {
                        s.b.i_a = i_a;
                        s.b.s_a = s_a;
                        s.b.i_b = i_b;
                        s.b.s_b = s_b;
                        s.b.p = c2_sub(s_b, s_a);
                        s.b.u = 0.0;
                    }
                    _ => {
                        s.c.i_a = i_a;
                        s.c.s_a = s_a;
                        s.c.i_b = i_b;
                        s.c.s_b = s_b;
                        s.c.p = c2_sub(s_b, s_a);
                        s.c.u = 0.0;
                    }
                }
                i += 1;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
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

    let mut save_a = [0; 3];
    let mut save_b = [0; 3];
    let mut d0 = f32::MAX;
    let mut iter = 0;
    let mut hit = 0;

    while iter < 20 {
        let save_count = s.count;
        let mut i = 0;
        while i < save_count {
            let v = match i {
                0 => s.a,
                1 => s.b,
                _ => s.c,
            };
            save_a[i as usize] = v.i_a;
            save_b[i as usize] = v.i_b;
            i += 1;
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
        let d1 = c2_dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2_d(&s);
        if c2_dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }

        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
        let mut v = C2sv::default();
        v.i_a = i_a;
        v.s_a = s_a;
        v.i_b = i_b;
        v.s_b = s_b;
        v.p = c2_sub(v.s_b, v.s_a);

        let mut dup = 0;
        let mut j = 0;
        while j < save_count {
            if i_a == save_a[j as usize] && i_b == save_b[j as usize] {
                dup = 1;
                break;
            }
            j += 1;
        }
        if dup != 0 {
            break;
        }

        match s.count {
            1 => s.b = v,
            2 => s.c = v,
            _ => s.d = v,
        }
        s.count += 1;
        iter += 1;
    }

    let mut a = C2v::ZERO;
    let mut b = C2v::ZERO;
    c2_witness(&s, &mut a, &mut b);
    let mut dist = c2_len(c2_sub(a, b));
    if hit != 0 {
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

    if let Some(cache_ref) = cache {
        cache_ref.metric = c2_gjk_simplex_metric(&s);
        cache_ref.count = s.count;
        let mut i = 0;
        while i < s.count {
            let v = match i {
                0 => s.a,
                1 => s.b,
                _ => s.c,
            };
            cache_ref.i_a[i as usize] = v.i_a;
            cache_ref.i_b[i as usize] = v.i_b;
            i += 1;
        }
        cache_ref.div = s.div;
    }

    if let Some(out_a_ref) = out_a {
        *out_a_ref = a;
    }
    if let Some(out_b_ref) = out_b {
        *out_b_ref = b;
    }
    if let Some(iterations_ref) = iterations {
        *iterations_ref = iter;
    }
    dist
}

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = if b.max.x < a.min.x { 1 } else { 0 };
    let d1 = if a.max.x < b.min.x { 1 } else { 0 };
    let d2 = if b.max.y < a.min.y { 1 } else { 0 };
    let d3 = if a.max.y < b.min.y { 1 } else { 0 };
    if (d0 | d1 | d2 | d3) == 0 { 1 } else { 0 }
}

fn c2_aabb_to_capsule(a: C2Aabb, b: C2Capsule) -> c_int {
    if c2_gjk(
        ShapeRef::Aabb(&a),
        C2Type::Aabb,
        None,
        ShapeRef::Capsule(&b),
        C2Type::Capsule,
        None,
        None,
        None,
        1,
        None,
        None,
    ) != 0.0
    {
        0
    } else {
        1
    }
}

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> c_int {
    if c2_gjk(
        ShapeRef::Capsule(&a),
        C2Type::Capsule,
        None,
        ShapeRef::Capsule(&b),
        C2Type::Capsule,
        None,
        None,
        None,
        1,
        None,
        None,
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
    let mut r2 = a.r + b.r;
    r2 *= r2;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> c_int {
    let n = c2_sub(b.b, b.a);
    let ap = c2_sub(a.p, b.a);
    let d2;
    let da = c2_dot(ap, n);
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

fn c2_collided(a: ShapeRef<'_>, type_a: C2Type, b: ShapeRef<'_>, type_b: C2Type) -> c_int {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => {
                if let (ShapeRef::Circle(a), ShapeRef::Circle(b)) = (a, b) {
                    c2_circle_to_circle(*a, *b)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (ShapeRef::Circle(a), ShapeRef::Aabb(b)) = (a, b) {
                    c2_circle_to_aabb(*a, *b)
                } else {
                    0
                }
            }
            C2Type::Capsule => {
                if let (ShapeRef::Circle(a), ShapeRef::Capsule(b)) = (a, b) {
                    c2_circle_to_capsule(*a, *b)
                } else {
                    0
                }
            }
        },
        C2Type::Aabb => match type_b {
            C2Type::Circle => {
                if let (ShapeRef::Aabb(a), ShapeRef::Circle(b)) = (a, b) {
                    c2_circle_to_aabb(*b, *a)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (ShapeRef::Aabb(a), ShapeRef::Aabb(b)) = (a, b) {
                    c2_aabb_to_aabb(*a, *b)
                } else {
                    0
                }
            }
            C2Type::Capsule => {
                if let (ShapeRef::Aabb(a), ShapeRef::Capsule(b)) = (a, b) {
                    c2_aabb_to_capsule(*a, *b)
                } else {
                    0
                }
            }
        },
        C2Type::Capsule => match type_b {
            C2Type::Circle => {
                if let (ShapeRef::Capsule(a), ShapeRef::Circle(b)) = (a, b) {
                    c2_circle_to_capsule(*b, *a)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (ShapeRef::Capsule(a), ShapeRef::Aabb(b)) = (a, b) {
                    c2_aabb_to_capsule(*b, *a)
                } else {
                    0
                }
            }
            C2Type::Capsule => {
                if let (ShapeRef::Capsule(a), ShapeRef::Capsule(b)) = (a, b) {
                    c2_capsule_to_capsule(*a, *b)
                } else {
                    0
                }
            }
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result = 0;

    let circle_in = C2Circle {
        p: c2_v(x, y),
        r,
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
        ShapeRef::Circle(&circle),
        C2Type::Circle,
        ShapeRef::Circle(&circle_in),
        C2Type::Circle,
    );

    result += c2_collided(
        ShapeRef::Aabb(&aabb),
        C2Type::Aabb,
        ShapeRef::Circle(&circle_in),
        C2Type::Circle,
    ) << 1;

    result += c2_collided(
        ShapeRef::Capsule(&capsule),
        C2Type::Capsule,
        ShapeRef::Circle(&circle_in),
        C2Type::Circle,
    ) << 2;

    result
}
