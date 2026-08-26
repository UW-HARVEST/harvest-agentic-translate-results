#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone)]
struct c2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[derive(Copy, Clone)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[derive(Copy, Clone)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[derive(Copy, Clone)]
struct c2GJKCache {
    metric: f32,
    count: i32,
    iA: [i32; 3],
    iB: [i32; 3],
    div: f32,
}

#[derive(Copy, Clone)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

#[derive(Copy, Clone, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: i32,
    iB: i32,
}

#[derive(Copy, Clone, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: i32,
}

enum Shape<'a> {
    Circle(&'a c2Circle),
    Aabb(&'a c2AABB),
    Capsule(&'a c2Capsule),
}

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

fn c2BBVerts(out: &mut [c2v; 4], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

fn c2MakeProxy(shape: Shape, p: &mut c2Proxy) {
    match shape {
        Shape::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        Shape::Aabb(bb) => {
            p.radius = 0.0;
            p.count = 4;
            let mut verts = [c2v::default(); 4];
            c2BBVerts(&mut verts, bb);
            p.verts[0..4].copy_from_slice(&verts);
        }
        Shape::Capsule(c) => {
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> f32 {
    match s.count {
        1 => 0.0,
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c22(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
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

fn c23(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
    let u_ab = c2Dot(b, c2Sub(b, a));
    let v_ab = c2Dot(a, c2Sub(a, b));
    let u_bc = c2Dot(c, c2Sub(c, b));
    let v_bc = c2Dot(b, c2Sub(b, c));
    let u_ca = c2Dot(a, c2Sub(a, c));
    let v_ca = c2Dot(c, c2Sub(c, a));
    let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let u_abc = c2Det2(b, c) * area;
    let v_abc = c2Det2(c, a) * area;
    let w_abc = c2Det2(a, b) * area;
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

fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

fn c2Skew(a: c2v) -> c2v {
    c2V(-a.y, a.x)
}

fn c2CCW90(a: c2v) -> c2v {
    c2V(a.y, -a.x)
}

fn c2D(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg(s.a.p),
        2 => {
            let ab = c2Sub(s.b.p, s.a.p);
            if c2Det2(ab, c2Neg(s.a.p)) > 0.0 {
                c2Skew(ab)
            } else {
                c2CCW90(ab)
            }
        }
        _ => c2V(0.0, 0.0),
    }
}

fn c2Support(verts: &[c2v], count: i32, d: c2v) -> i32 {
    let mut imax = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count as usize {
        let dot = c2Dot(verts[i], d);
        if dot > dmax {
            imax = i as i32;
            dmax = dot;
        }
    }
    imax
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u));
            *b = c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u));
        }
        3 => {
            *a = c2Add(
                c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u)),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2Add(
                c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u)),
                c2Mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2GJK(
    shape_a: Shape,
    ax_ptr: Option<&c2x>,
    shape_b: Shape,
    bx_ptr: Option<&c2x>,
    mut out_a: Option<&mut c2v>,
    mut out_b: Option<&mut c2v>,
    use_radius: i32,
    mut iterations: Option<&mut i32>,
    mut cache: Option<&mut c2GJKCache>,
) -> f32 {
    let ax = ax_ptr.copied().unwrap_or_else(c2xIdentity);
    let bx = bx_ptr.copied().unwrap_or_else(c2xIdentity);

    let mut p_a = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v::default(); 8],
    };
    let mut p_b = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v::default(); 8],
    };
    c2MakeProxy(shape_a, &mut p_a);
    c2MakeProxy(shape_b, &mut p_b);

    let mut s = c2Simplex::default();
    let mut cache_was_read = false;

    if let Some(c) = cache.as_deref_mut() {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let i_a = c.iA[i];
                let i_b = c.iB[i];
                let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
                let v = match i {
                    0 => &mut s.a,
                    1 => &mut s.b,
                    2 => &mut s.c,
                    _ => unreachable!(),
                };
                v.iA = i_a;
                v.sA = s_a;
                v.iB = i_b;
                v.sB = s_b;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
            let metric = c2GJKSimplexMetric(&s);
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
        s.a.sA = c2Mulxv(ax, p_a.verts[0]);
        s.a.sB = c2Mulxv(bx, p_b.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a = [0; 3];
    let mut save_b = [0; 3];
    let mut save_count = 0;
    let mut d0 = 3.40282346638528859811704183484516925e+38;
    let mut d1 = 3.40282346638528859811704183484516925e+38;
    let mut iter = 0;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = match i {
                0 => &s.a,
                1 => &s.b,
                2 => &s.c,
                _ => unreachable!(),
            };
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

        let p = c2L(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2D(&s);
        if c2Dot(d, d) < 1.1920928955078125e-7 * 1.1920928955078125e-7 {
            break;
        }

        let i_a = c2Support(&p_a.verts, p_a.count, c2MulrvT(ax.r, c2Neg(d)));
        let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2Support(&p_b.verts, p_b.count, c2MulrvT(bx.r, d));
        let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);

        let v = match s.count {
            0 => &mut s.a,
            1 => &mut s.b,
            2 => &mut s.c,
            3 => &mut s.d,
            _ => unreachable!(),
        };

        v.iA = i_a;
        v.sA = s_a;
        v.iB = i_b;
        v.sB = s_b;
        v.p = c2Sub(v.sB, v.sA);

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

    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    c2Witness(&s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));

    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > 1.1920928955078125e-7 {
            dist -= r_a + r_b;
            let n = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, r_a));
            b = c2Sub(b, c2Mulvs(n, r_b));
            if a.x == b.x && a.y == b.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a, b), 0.5);
            a = p;
            b = p;
            dist = 0.0;
        }
    }

    if let Some(c) = cache.as_deref_mut() {
        c.metric = c2GJKSimplexMetric(&s);
        c.count = s.count;
        for i in 0..s.count as usize {
            let v = match i {
                0 => &s.a,
                1 => &s.b,
                2 => &s.c,
                _ => unreachable!(),
            };
            c.iA[i] = v.iA;
            c.iB[i] = v.iB;
        }
        c.div = s.div;
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

#[unsafe(no_mangle)]
pub extern "C" fn gjk(
    reverse: std::os::raw::c_char,
    a: *mut c2v,
    b: *mut c2v,
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
    let bb = c2AABB {
        min: c2V(a1, a2),
        max: c2V(a3, a4),
    };

    let cap = c2Capsule {
        a: c2V(b1, b2),
        b: c2V(b3, b4),
        r: b5,
    };

    let out_a = if a.is_null() { None } else { unsafe { Some(&mut *a) } };
    let out_b = if b.is_null() { None } else { unsafe { Some(&mut *b) } };

    if reverse != 0 {
        c2GJK(
            Shape::Capsule(&cap),
            None,
            Shape::Aabb(&bb),
            None,
            out_a,
            out_b,
            1,
            None,
            None,
        );
    } else {
        c2GJK(
            Shape::Aabb(&bb),
            None,
            Shape::Capsule(&cap),
            None,
            out_a,
            out_b,
            1,
            None,
            None,
        );
    }
}
