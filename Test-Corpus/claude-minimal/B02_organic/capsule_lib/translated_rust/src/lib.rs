// Translation of c_src/src/lib.c into Rust.
// Implements a small 2D collision library and a `capsule` entry point.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

#[derive(Copy, Clone, PartialEq, Eq)]
enum C2Type {
    Circle,
    AABB,
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
struct C2AABB {
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
struct C2GJKCache {
    metric: f32,
    count: i32,
    iA: [i32; 3],
    iB: [i32; 3],
    div: f32,
}

#[derive(Copy, Clone)]
enum Shape {
    Circle(C2Circle),
    AABB(C2AABB),
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

fn c2_x_identity() -> C2x {
    C2x {
        p: c2v(0.0, 0.0),
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
        Self {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

fn c2_bb_verts(out: &mut [C2v; 8], bb: &C2AABB) {
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
        Shape::AABB(bb) => {
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
    sA: C2v,
    sB: C2v,
    p: C2v,
    u: f32,
    iA: i32,
    iB: i32,
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

// Helpers to access simplex vertices by index (mimicking pointer arithmetic
// in the original C code where verts = &s.a).
fn simplex_get(s: &C2Simplex, i: usize) -> C2sv {
    match i {
        0 => s.a,
        1 => s.b,
        2 => s.c,
        3 => s.d,
        _ => panic!("simplex index out of bounds"),
    }
}

fn simplex_set(s: &mut C2Simplex, i: usize, v: C2sv) {
    match i {
        0 => s.a = v,
        1 => s.b = v,
        2 => s.c = v,
        3 => s.d = v,
        _ => panic!("simplex index out of bounds"),
    }
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
    let uAB = c2_dot(b, c2_sub(b, a));
    let vAB = c2_dot(a, c2_sub(a, b));
    let uBC = c2_dot(c, c2_sub(c, b));
    let vBC = c2_dot(b, c2_sub(b, c));
    let uCA = c2_dot(a, c2_sub(a, c));
    let vCA = c2_dot(c, c2_sub(c, a));
    let area = c2_det2(c2_sub(b, a), c2_sub(c, a));
    let uABC = c2_det2(b, c) * area;
    let vABC = c2_det2(c, a) * area;
    let wABC = c2_det2(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.a = s.c;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.a.u = uAB;
        s.b.u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.a = s.b;
        s.b = s.c;
        s.a.u = uBC;
        s.b.u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.b = s.a;
        s.a = s.c;
        s.a.u = uCA;
        s.b.u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.a.u = uABC;
        s.b.u = vABC;
        s.c.u = wABC;
        s.div = uABC + vABC + wABC;
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

fn c2_witness(s: &C2Simplex, a_out: &mut C2v, b_out: &mut C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a_out = s.a.sA;
            *b_out = s.a.sB;
        }
        2 => {
            *a_out = c2_add(
                c2_mulvs(s.a.sA, den * s.a.u),
                c2_mulvs(s.b.sA, den * s.b.u),
            );
            *b_out = c2_add(
                c2_mulvs(s.a.sB, den * s.a.u),
                c2_mulvs(s.b.sB, den * s.b.u),
            );
        }
        3 => {
            *a_out = c2_add(
                c2_add(
                    c2_mulvs(s.a.sA, den * s.a.u),
                    c2_mulvs(s.b.sA, den * s.b.u),
                ),
                c2_mulvs(s.c.sA, den * s.c.u),
            );
            *b_out = c2_add(
                c2_add(
                    c2_mulvs(s.a.sB, den * s.a.u),
                    c2_mulvs(s.b.sB, den * s.b.u),
                ),
                c2_mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a_out = c2v(0.0, 0.0);
            *b_out = c2v(0.0, 0.0);
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
        2 => c2_add(
            c2_mulvs(s.a.p, den * s.a.u),
            c2_mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

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
        Some(p) => *p,
        None => c2_x_identity(),
    };
    let bx = match bx_ptr {
        Some(p) => *p,
        None => c2_x_identity(),
    };
    let mut pA = C2Proxy::default();
    let mut pB = C2Proxy::default();
    c2_make_proxy(a_shape, &mut pA);
    c2_make_proxy(b_shape, &mut pB);
    let mut s = C2Simplex::default();
    let mut cache_was_read = false;
    if let Some(ref c) = cache {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let iA = c.iA[i];
                let iB = c.iB[i];
                let sA = c2_mulxv(ax, pA.verts[iA as usize]);
                let sB = c2_mulxv(bx, pB.verts[iB as usize]);
                let v = C2sv {
                    iA,
                    sA,
                    iB,
                    sB,
                    p: c2_sub(sB, sA),
                    u: 0.0,
                };
                simplex_set(&mut s, i, v);
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
            let metric = c2_gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old {
                metric
            } else {
                metric_old
            };
            let max_metric = if metric > metric_old {
                metric
            } else {
                metric_old
            };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = true;
            }
        }
    }
    if !cache_was_read {
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2_mulxv(ax, pA.verts[0]);
        s.a.sB = c2_mulxv(bx, pB.verts[0]);
        s.a.p = c2_sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }
    let mut save_a: [i32; 3] = [0; 3];
    let mut save_b: [i32; 3] = [0; 3];
    let mut save_count: i32;
    let mut d0: f32 = f32::MAX;
    let mut d1: f32;
    let mut iter: i32 = 0;
    let mut hit = false;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = simplex_get(&s, i);
            save_a[i] = v.iA;
            save_b[i] = v.iB;
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
        let iA = c2_support(&pA.verts, pA.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let sA = c2_mulxv(ax, pA.verts[iA as usize]);
        let iB = c2_support(&pB.verts, pB.count, c2_mulrv_t(bx.r, d));
        let sB = c2_mulxv(bx, pB.verts[iB as usize]);
        let v = C2sv {
            iA,
            sA,
            iB,
            sB,
            p: c2_sub(sB, sA),
            u: 0.0,
        };
        let idx = s.count as usize;
        simplex_set(&mut s, idx, v);
        let mut dup = false;
        for i in 0..save_count as usize {
            if iA == save_a[i] && iB == save_b[i] {
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
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.192_092_9e-7_f32 {
            dist -= rA + rB;
            let n = c2_norm(c2_sub(b, a));
            a = c2_add(a, c2_mulvs(n, rA));
            b = c2_sub(b, c2_mulvs(n, rB));
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
            c.iA[i] = v.iA;
            c.iB[i] = v.iB;
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

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> bool {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    !(d0 || d1 || d2 || d3)
}

fn c2_aabb_to_capsule(a: C2AABB, b: C2Capsule) -> bool {
    let dist = c2_gjk(
        &Shape::AABB(a),
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

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> bool {
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

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> bool {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
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

fn c2_collided(a: &Shape, b: &Shape) -> bool {
    match (a, b) {
        (Shape::Circle(ac), Shape::Circle(bc)) => c2_circle_to_circle(*ac, *bc),
        (Shape::Circle(ac), Shape::AABB(bb)) => c2_circle_to_aabb(*ac, *bb),
        (Shape::Circle(ac), Shape::Capsule(bc)) => c2_circle_to_capsule(*ac, *bc),
        (Shape::AABB(ab), Shape::Circle(bc)) => c2_circle_to_aabb(*bc, *ab),
        (Shape::AABB(ab), Shape::AABB(bb)) => c2_aabb_to_aabb(*ab, *bb),
        (Shape::AABB(ab), Shape::Capsule(bc)) => c2_aabb_to_capsule(*ab, *bc),
        (Shape::Capsule(ac), Shape::Circle(bc)) => c2_circle_to_capsule(*bc, *ac),
        (Shape::Capsule(ac), Shape::AABB(bb)) => c2_aabb_to_capsule(*bb, *ac),
        (Shape::Capsule(ac), Shape::Capsule(bc)) => c2_capsule_to_capsule(*ac, *bc),
    }
}

#[no_mangle]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> i32 {
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

    let aabb = C2AABB {
        min: c2v(-40.0, -40.0),
        max: c2v(-15.0, -15.0),
    };

    let capsule_local = C2Capsule {
        a: c2v(-40.0, 40.0),
        b: c2v(-20.0, 100.0),
        r: 10.0,
    };

    result += c2_collided(
        &Shape::Circle(circle),
        &Shape::Capsule(capsule_in),
    ) as i32;

    result += (c2_collided(
        &Shape::AABB(aabb),
        &Shape::Capsule(capsule_in),
    ) as i32)
        << 1;

    result += (c2_collided(
        &Shape::Capsule(capsule_local),
        &Shape::Capsule(capsule_in),
    ) as i32)
        << 2;

    result
}
