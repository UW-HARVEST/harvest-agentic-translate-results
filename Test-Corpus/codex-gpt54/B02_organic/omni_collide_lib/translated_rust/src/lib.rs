#![allow(non_camel_case_types)]

use std::ffi::c_int;

pub type C2_TYPE = c_int;

pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;

#[derive(Clone, Copy, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Clone, Copy)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
struct C2GjkCache {
    metric: f32,
    count: c_int,
    ia: [c_int; 3],
    ib: [c_int; 3],
    div: f32,
}

#[derive(Clone, Copy)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

#[derive(Clone, Copy, Default)]
struct C2sv {
    sa: C2v,
    sb: C2v,
    p: C2v,
    u: f32,
    ia: c_int,
    ib: c_int,
}

#[derive(Clone, Copy, Default)]
struct C2Simplex {
    verts: [C2sv; 4],
    div: f32,
    count: c_int,
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
    Invalid,
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
    c2_v(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
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

fn c2_make_proxy(shape: &Shape, typ: C2_TYPE) -> C2Proxy {
    let mut p = C2Proxy {
        radius: 0.0,
        count: 0,
        verts: [C2v::default(); 8],
    };

    match typ {
        C2_TYPE_CIRCLE => {
            if let Shape::Circle(c) = shape {
                p.radius = c.r;
                p.count = 1;
                p.verts[0] = c.p;
            }
        }
        C2_TYPE_AABB => {
            if let Shape::Aabb(bb) = shape {
                p.radius = 0.0;
                p.count = 4;
                c2_bb_verts(&mut p.verts, bb);
            }
        }
        C2_TYPE_CAPSULE => {
            if let Shape::Capsule(c) = shape {
                p.radius = c.r;
                p.count = 2;
                p.verts[0] = c.a;
                p.verts[1] = c.b;
            }
        }
        _ => {}
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
        2 => c2_len(c2_sub(s.verts[1].p, s.verts[0].p)),
        3 => c2_det2(c2_sub(s.verts[1].p, s.verts[0].p), c2_sub(s.verts[2].p, s.verts[0].p)),
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
    let uab = c2_dot(b, c2_sub(b, a));
    let vab = c2_dot(a, c2_sub(a, b));
    let ubc = c2_dot(c, c2_sub(c, b));
    let vbc = c2_dot(b, c2_sub(b, c));
    let uca = c2_dot(a, c2_sub(a, c));
    let vca = c2_dot(c, c2_sub(c, a));
    let area = c2_det2(c2_sub(b, a), c2_sub(c, a));
    let uabc = c2_det2(b, c) * area;
    let vabc = c2_det2(c, a) * area;
    let wabc = c2_det2(a, b) * area;
    if vab <= 0.0 && uca <= 0.0 {
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uab <= 0.0 && vbc <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if ubc <= 0.0 && vca <= 0.0 {
        s.verts[0] = s.verts[2];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uab > 0.0 && vab > 0.0 && wabc <= 0.0 {
        s.verts[0].u = uab;
        s.verts[1].u = vab;
        s.div = uab + vab;
        s.count = 2;
    } else if ubc > 0.0 && vbc > 0.0 && uabc <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[1] = s.verts[2];
        s.verts[0].u = ubc;
        s.verts[1].u = vbc;
        s.div = ubc + vbc;
        s.count = 2;
    } else if uca > 0.0 && vca > 0.0 && vabc <= 0.0 {
        s.verts[1] = s.verts[0];
        s.verts[0] = s.verts[2];
        s.verts[0].u = uca;
        s.verts[1].u = vca;
        s.div = uca + vca;
        s.count = 2;
    } else {
        s.verts[0].u = uabc;
        s.verts[1].u = vabc;
        s.verts[2].u = wabc;
        s.div = uabc + vabc + wabc;
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
        1 => c2_neg(s.verts[0].p),
        2 => {
            let ab = c2_sub(s.verts[1].p, s.verts[0].p);
            if c2_det2(ab, c2_neg(s.verts[0].p)) > 0.0 {
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

fn c2_witness(s: &C2Simplex) -> (C2v, C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => (s.verts[0].sa, s.verts[0].sb),
        2 => (
            c2_add(
                c2_mulvs(s.verts[0].sa, den * s.verts[0].u),
                c2_mulvs(s.verts[1].sa, den * s.verts[1].u),
            ),
            c2_add(
                c2_mulvs(s.verts[0].sb, den * s.verts[0].u),
                c2_mulvs(s.verts[1].sb, den * s.verts[1].u),
            ),
        ),
        3 => (
            c2_add(
                c2_add(
                    c2_mulvs(s.verts[0].sa, den * s.verts[0].u),
                    c2_mulvs(s.verts[1].sa, den * s.verts[1].u),
                ),
                c2_mulvs(s.verts[2].sa, den * s.verts[2].u),
            ),
            c2_add(
                c2_add(
                    c2_mulvs(s.verts[0].sb, den * s.verts[0].u),
                    c2_mulvs(s.verts[1].sb, den * s.verts[1].u),
                ),
                c2_mulvs(s.verts[2].sb, den * s.verts[2].u),
            ),
        ),
        _ => (c2_v(0.0, 0.0), c2_v(0.0, 0.0)),
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
        _ => c2_v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2_v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_gjk(
    a: &Shape,
    type_a: C2_TYPE,
    ax_ptr: Option<&C2x>,
    b: &Shape,
    type_b: C2_TYPE,
    bx_ptr: Option<&C2x>,
    out_a: Option<&mut C2v>,
    out_b: Option<&mut C2v>,
    use_radius: c_int,
    iterations: Option<&mut c_int>,
    cache: Option<&mut C2GjkCache>,
) -> f32 {
    let ax = ax_ptr.copied().unwrap_or_else(c2x_identity);
    let bx = bx_ptr.copied().unwrap_or_else(c2x_identity);
    let pa = c2_make_proxy(a, type_a);
    let pb = c2_make_proxy(b, type_b);
    let mut s = C2Simplex::default();
    let mut cache_was_read = 0;

    if let Some(cache_ref) = cache.as_ref() {
        let cache_was_good = (cache_ref.count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i = 0;
            while i < cache_ref.count {
                let ia = cache_ref.ia[i as usize];
                let ib = cache_ref.ib[i as usize];
                let sa = c2_mulxv(ax, pa.verts[ia as usize]);
                let sb = c2_mulxv(bx, pb.verts[ib as usize]);
                let v = &mut s.verts[i as usize];
                v.ia = ia;
                v.sa = sa;
                v.ib = ib;
                v.sb = sb;
                v.p = c2_sub(v.sb, v.sa);
                v.u = 0.0;
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
        s.verts[0].ia = 0;
        s.verts[0].ib = 0;
        s.verts[0].sa = c2_mulxv(ax, pa.verts[0]);
        s.verts[0].sb = c2_mulxv(bx, pb.verts[0]);
        s.verts[0].p = c2_sub(s.verts[0].sb, s.verts[0].sa);
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a = [0; 3];
    let mut save_b = [0; 3];
    let mut save_count: c_int;
    let mut d0 = f32::MAX;
    let mut d1: f32;
    let mut iter = 0;
    let mut hit = 0;

    while iter < 20 {
        save_count = s.count;
        let mut i = 0;
        while i < save_count {
            save_a[i as usize] = s.verts[i as usize].ia;
            save_b[i as usize] = s.verts[i as usize].ib;
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
        d1 = c2_dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2_d(&s);
        if c2_dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }

        let ia = c2_support(&pa.verts, pa.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let sa = c2_mulxv(ax, pa.verts[ia as usize]);
        let ib = c2_support(&pb.verts, pb.count, c2_mulrv_t(bx.r, d));
        let sb = c2_mulxv(bx, pb.verts[ib as usize]);
        let v = &mut s.verts[s.count as usize];
        v.ia = ia;
        v.sa = sa;
        v.ib = ib;
        v.sb = sb;
        v.p = c2_sub(v.sb, v.sa);

        let mut dup = 0;
        let mut j = 0;
        while j < save_count {
            if ia == save_a[j as usize] && ib == save_b[j as usize] {
                dup = 1;
                break;
            }
            j += 1;
        }
        if dup != 0 {
            break;
        }

        s.count += 1;
        iter += 1;
    }

    let (mut out_a_value, mut out_b_value) = c2_witness(&s);
    let mut dist = c2_len(c2_sub(out_a_value, out_b_value));
    if hit != 0 {
        out_a_value = out_b_value;
        dist = 0.0;
    } else if use_radius != 0 {
        let ra = pa.radius;
        let rb = pb.radius;
        if dist > ra + rb && dist > f32::EPSILON {
            dist -= ra + rb;
            let n = c2_norm(c2_sub(out_b_value, out_a_value));
            out_a_value = c2_add(out_a_value, c2_mulvs(n, ra));
            out_b_value = c2_sub(out_b_value, c2_mulvs(n, rb));
            if out_a_value.x == out_b_value.x && out_a_value.y == out_b_value.y {
                dist = 0.0;
            }
        } else {
            let p = c2_mulvs(c2_add(out_a_value, out_b_value), 0.5);
            out_a_value = p;
            out_b_value = p;
            dist = 0.0;
        }
    }

    if let Some(cache_ref) = cache {
        cache_ref.metric = c2_gjk_simplex_metric(&s);
        cache_ref.count = s.count;
        let mut i = 0;
        while i < s.count {
            let v = s.verts[i as usize];
            cache_ref.ia[i as usize] = v.ia;
            cache_ref.ib[i as usize] = v.ib;
            i += 1;
        }
        cache_ref.div = s.div;
    }

    if let Some(out_ref) = out_a {
        *out_ref = out_a_value;
    }
    if let Some(out_ref) = out_b {
        *out_ref = out_b_value;
    }
    if let Some(iter_ref) = iterations {
        *iter_ref = iter;
    }

    dist
}

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2_aabb_to_capsule(a: C2Aabb, b: C2Capsule) -> c_int {
    let sa = Shape::Aabb(a);
    let sb = Shape::Capsule(b);
    if c2_gjk(&sa, C2_TYPE_AABB, None, &sb, C2_TYPE_CAPSULE, None, None, None, 1, None, None) != 0.0
    {
        0
    } else {
        1
    }
}

fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> c_int {
    let sa = Shape::Capsule(a);
    let sb = Shape::Capsule(b);
    if c2_gjk(
        &sa,
        C2_TYPE_CAPSULE,
        None,
        &sb,
        C2_TYPE_CAPSULE,
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
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> c_int {
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
    (d2 < r * r) as c_int
}

fn c2_collided(a: &Shape, type_a: C2_TYPE, b: &Shape, type_b: C2_TYPE) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => {
                if let (Shape::Circle(aa), Shape::Circle(bb)) = (*a, *b) {
                    c2_circle_to_circle(aa, bb)
                } else {
                    0
                }
            }
            C2_TYPE_AABB => {
                if let (Shape::Circle(aa), Shape::Aabb(bb)) = (*a, *b) {
                    c2_circle_to_aabb(aa, bb)
                } else {
                    0
                }
            }
            C2_TYPE_CAPSULE => {
                if let (Shape::Circle(aa), Shape::Capsule(bb)) = (*a, *b) {
                    c2_circle_to_capsule(aa, bb)
                } else {
                    0
                }
            }
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => {
                if let (Shape::Aabb(aa), Shape::Circle(bb)) = (*a, *b) {
                    c2_circle_to_aabb(bb, aa)
                } else {
                    0
                }
            }
            C2_TYPE_AABB => {
                if let (Shape::Aabb(aa), Shape::Aabb(bb)) = (*a, *b) {
                    c2_aabb_to_aabb(aa, bb)
                } else {
                    0
                }
            }
            C2_TYPE_CAPSULE => {
                if let (Shape::Aabb(aa), Shape::Capsule(bb)) = (*a, *b) {
                    c2_aabb_to_capsule(aa, bb)
                } else {
                    0
                }
            }
            _ => 0,
        },
        C2_TYPE_CAPSULE => match type_b {
            C2_TYPE_CIRCLE => {
                if let (Shape::Capsule(aa), Shape::Circle(bb)) = (*a, *b) {
                    c2_circle_to_capsule(bb, aa)
                } else {
                    0
                }
            }
            C2_TYPE_AABB => {
                if let (Shape::Capsule(aa), Shape::Aabb(bb)) = (*a, *b) {
                    c2_aabb_to_capsule(bb, aa)
                } else {
                    0
                }
            }
            C2_TYPE_CAPSULE => {
                if let (Shape::Capsule(aa), Shape::Capsule(bb)) = (*a, *b) {
                    c2_capsule_to_capsule(aa, bb)
                } else {
                    0
                }
            }
            _ => 0,
        },
        _ => 0,
    }
}

fn ptr_from_parts(typ: C2_TYPE, a: f32, b: f32, c: f32, d: f32, e: f32) -> Shape {
    match typ {
        C2_TYPE_CIRCLE => Shape::Circle(C2Circle {
            p: c2_v(a, b),
            r: c,
        }),
        C2_TYPE_AABB => Shape::Aabb(C2Aabb {
            min: c2_v(a, b),
            max: c2_v(c, d),
        }),
        C2_TYPE_CAPSULE => Shape::Capsule(C2Capsule {
            a: c2_v(a, b),
            b: c2_v(c, d),
            r: e,
        }),
        _ => Shape::Invalid,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_collide(
    type_a: C2_TYPE,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: C2_TYPE,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) -> c_int {
    let a = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2_collided(&a, type_a, &b, type_b)
}
