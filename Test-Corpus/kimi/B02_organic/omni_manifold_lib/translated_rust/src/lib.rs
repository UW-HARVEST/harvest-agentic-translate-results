use std::f32;
use std::os::raw::{c_float, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE = 0,
    C2_TYPE_CIRCLE = 1,
    C2_TYPE_AABB = 2,
    C2_TYPE_POLY = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [c_float; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2h {
    n: c2v,
    d: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        Self {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2GJKCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[derive(Clone, Copy, Debug, Default)]
struct c2Proxy {
    radius: c_float,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[derive(Clone, Copy, Debug, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: c_float,
    count: c_int,
}

fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

fn c2Mulvs(a: c2v, b: c_float) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2Add(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn c2Dot(a: c2v, b: c2v) -> c_float {
    a.x * b.x + a.y * b.y
}

fn c2Dist(h: c2h, p: c2v) -> c_float {
    c2Dot(h.n, p) - h.d
}

fn c2PlaneAt(p: &c2Poly, i: c_int) -> c2h {
    let i = i as usize;
    c2h {
        n: p.norms[i],
        d: c2Dot(p.norms[i], p.verts[i]),
    }
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

fn c2BBVerts(bb: &c2AABB) -> [c2v; 4] {
    [
        bb.min,
        c2V(bb.max.x, bb.min.y),
        bb.max,
        c2V(bb.min.x, bb.max.y),
    ]
}

fn c2MakeProxy(shape: *const c_void, typ: C2_TYPE) -> c2Proxy {
    let mut p = c2Proxy::default();
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let c = unsafe { &*(shape as *const c2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE::C2_TYPE_AABB => {
            let bb = unsafe { &*(shape as *const c2AABB) };
            p.radius = 0.0;
            p.count = 4;
            let verts = c2BBVerts(bb);
            p.verts[..4].copy_from_slice(&verts);
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
    }
    p
}

fn c2Len(a: c2v) -> c_float {
    c2Dot(a, a).sqrt()
}

fn c2Det2(a: c2v, b: c2v) -> c_float {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> c_float {
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

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

fn c2Intersect(a: c2v, b: c2v, da: c_float, db: c_float) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

fn c2Clip(seg: &mut [c2v; 2], h: c2h) -> c_int {
    let mut out: [c2v; 2] = [c2v::default(); 2];
    let mut sp = 0;
    let d0 = c2Dist(h, seg[0]);
    let d1 = c2Dist(h, seg[1]);
    if d0 < 0.0 {
        out[sp] = seg[0];
        sp += 1;
    }
    if d1 < 0.0 {
        out[sp] = seg[1];
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        out[sp] = seg[0];
        sp += 1;
        out[sp] = seg[1];
        sp += 1;
    } else if d0 * d1 <= 0.0 {
        out[sp] = c2Intersect(seg[0], seg[1], d0, d1);
        sp += 1;
    }
    seg[0] = out[0];
    seg[1] = out[1];
    sp as c_int
}

fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

fn c2CCW90(a: c2v) -> c2v {
    c2V(a.y, -a.x)
}

fn c2Skew(a: c2v) -> c2v {
    c2V(-a.y, a.x)
}

fn c2SidePlanes(seg: &mut [c2v; 2], ra: c2v, rb: c2v, h: Option<&mut c2h>) -> c_int {
    let inn = c2Norm(c2Sub(rb, ra));
    let left = c2h {
        n: c2Neg(inn),
        d: c2Dot(c2Neg(inn), ra),
    };
    let right = c2h {
        n: inn,
        d: c2Dot(inn, rb),
    };
    if c2Clip(seg, left) < 2 {
        return 0;
    }
    if c2Clip(seg, right) < 2 {
        return 0;
    }
    if let Some(h_out) = h {
        h_out.n = c2CCW90(inn);
        h_out.d = c2Dot(c2CCW90(inn), ra);
    }
    1
}

fn c2SidePlanesFromPoly(seg: &mut [c2v; 2], x: c2x, p: &c2Poly, e: c_int, h: Option<&mut c2h>) -> c_int {
    let e = e as usize;
    let ra = c2Mulxv(x, p.verts[e]);
    let rb = c2Mulxv(x, p.verts[if e + 1 == p.count as usize { 0 } else { e + 1 }]);
    c2SidePlanes(seg, ra, rb, h)
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

fn c2Support(verts: &[c2v], count: c_int, d: c2v) -> c_int {
    let count = count as usize;
    let mut imax = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count {
        let dot = c2Dot(verts[i], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax as c_int
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s.a.sA, den * s.a.u),
                c2Mulvs(s.b.sA, den * s.b.u),
            );
            *b = c2Add(
                c2Mulvs(s.a.sB, den * s.a.u),
                c2Mulvs(s.b.sB, den * s.b.u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s.a.sA, den * s.a.u),
                    c2Mulvs(s.b.sA, den * s.b.u),
                ),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s.a.sB, den * s.a.u),
                    c2Mulvs(s.b.sB, den * s.b.u),
                ),
                c2Mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(
            c2Mulvs(s.a.p, den * s.a.u),
            c2Mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

fn c2GJK(
    a: *const c_void,
    type_a: C2_TYPE,
    ax_ptr: *const c2x,
    b: *const c_void,
    type_b: C2_TYPE,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> c_float {
    let ax = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *bx_ptr }
    };
    let p_a = c2MakeProxy(a, type_a);
    let p_b = c2MakeProxy(b, type_b);
    let mut s = c2Simplex::default();
    let verts: &mut [c2sv] = unsafe {
        std::slice::from_raw_parts_mut(&mut s.a as *mut c2sv, 4)
    };
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let i_a = cache_ref.iA[i] as usize;
                let i_b = cache_ref.iB[i] as usize;
                let s_a = c2Mulxv(ax, p_a.verts[i_a]);
                let s_b = c2Mulxv(bx, p_b.verts[i_b]);
                let v = &mut verts[i];
                v.iA = i_a as c_int;
                v.sA = s_a;
                v.iB = i_b as c_int;
                v.sB = s_b;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = 1;
            }
        }
    }
    if cache_was_read == 0 {
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, p_a.verts[0]);
        s.a.sB = c2Mulxv(bx, p_b.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }
    let mut save_a: [c_int; 3] = [0; 3];
    let mut save_b: [c_int; 3] = [0; 3];
    let mut save_count = 0;
    let mut d0: c_float = f32::MAX;
    let mut d1: c_float = f32::MAX;
    let mut iter = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            save_a[i] = verts[i].iA;
            save_b[i] = verts[i].iB;
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
        let i_a = c2Support(&p_a.verts[..p_a.count as usize], p_a.count, c2MulrvT(ax.r, c2Neg(d)));
        let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2Support(&p_b.verts[..p_b.count as usize], p_b.count, c2MulrvT(bx.r, d));
        let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
        let v = &mut verts[s.count as usize];
        v.iA = i_a;
        v.sA = s_a;
        v.iB = i_b;
        v.sB = s_b;
        v.p = c2Sub(v.sB, v.sA);
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
    let mut a_out = c2v::default();
    let mut b_out = c2v::default();
    c2Witness(&s, &mut a_out, &mut b_out);
    let mut dist = c2Len(c2Sub(a_out, b_out));
    if hit != 0 {
        a_out = b_out;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > 1.1920928955078125e-7 {
            dist -= r_a + r_b;
            let n = c2Norm(c2Sub(b_out, a_out));
            a_out = c2Add(a_out, c2Mulvs(n, r_a));
            b_out = c2Sub(b_out, c2Mulvs(n, r_b));
            if a_out.x == b_out.x && a_out.y == b_out.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a_out, b_out), 0.5);
            a_out = p;
            b_out = p;
            dist = 0.0;
        }
    }
    if !cache.is_null() {
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = c2GJKSimplexMetric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            let v = &verts[i];
            cache_ref.iA[i] = v.iA;
            cache_ref.iB[i] = v.iB;
        }
        cache_ref.div = s.div;
    }
    if !out_a.is_null() {
        unsafe { *out_a = a_out };
    }
    if !out_b.is_null() {
        unsafe { *out_b = b_out };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }
    dist
}

fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2CircletoCircleManifold(a: c2Circle, b: c2Circle, m: &mut c2Manifold) {
    m.count = 0;
    let d = c2Sub(b.p, a.p);
    let d2 = c2Dot(d, d);
    let r = a.r + b.r;
    if d2 < r * r {
        let l = d2.sqrt();
        let n = if l != 0.0 {
            c2Mulvs(d, 1.0 / l)
        } else {
            c2V(0.0, 1.0)
        };
        m.count = 1;
        m.depths[0] = r - l;
        m.contact_points[0] = c2Sub(b.p, c2Mulvs(n, b.r));
        m.n = n;
    }
}

fn c2CircletoAABBManifold(a: c2Circle, b: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(l, a.p);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 {
        if d2 != 0.0 {
            let d = d2.sqrt();
            let n = c2Norm(ab);
            m.count = 1;
            m.depths[0] = a.r - d;
            m.contact_points[0] = c2Add(a.p, c2Mulvs(n, d));
            m.n = n;
        } else {
            let mid = c2Mulvs(c2Add(b.min, b.max), 0.5);
            let e = c2Mulvs(c2Sub(b.max, b.min), 0.5);
            let d = c2Sub(a.p, mid);
            let abs_d = c2Absv(d);
            let x_overlap = e.x - abs_d.x;
            let y_overlap = e.y - abs_d.y;
            let (depth, n) = if x_overlap < y_overlap {
                let n = c2V(1.0, 0.0);
                (x_overlap, c2Mulvs(n, if d.x < 0.0 { 1.0 } else { -1.0 }))
            } else {
                let n = c2V(0.0, 1.0);
                (y_overlap, c2Mulvs(n, if d.y < 0.0 { 1.0 } else { -1.0 }))
            };
            m.count = 1;
            m.depths[0] = a.r + depth;
            m.contact_points[0] = c2Sub(a.p, c2Mulvs(n, depth));
            m.n = n;
        }
    }
}

fn c2CircletoCapsuleManifold(a: c2Circle, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let r = a.r + b.r;
    let mut a_out = c2v::default();
    let mut b_out = c2v::default();
    let d = c2GJK(
        &a as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CIRCLE,
        std::ptr::null(),
        &b as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a_out,
        &mut b_out,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < r {
        let n = if d == 0.0 {
            c2Norm(c2Skew(c2Sub(b.b, b.a)))
        } else {
            c2Norm(c2Sub(b_out, a_out))
        };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(b_out, c2Mulvs(n, b.r));
        m.n = n;
    }
}

fn c2AABBtoAABBManifold(a: c2AABB, b: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let mid_a = c2Mulvs(c2Add(a.min, a.max), 0.5);
    let mid_b = c2Mulvs(c2Add(b.min, b.max), 0.5);
    let e_a = c2Absv(c2Mulvs(c2Sub(a.max, a.min), 0.5));
    let e_b = c2Absv(c2Mulvs(c2Sub(b.max, b.min), 0.5));
    let d = c2Sub(mid_b, mid_a);
    let dx = e_a.x + e_b.x - if d.x < 0.0 { -d.x } else { d.x };
    if dx < 0.0 {
        return;
    }
    let dy = e_a.y + e_b.y - if d.y < 0.0 { -d.y } else { d.y };
    if dy < 0.0 {
        return;
    }
    let (n, depth, p) = if dx < dy {
        if d.x < 0.0 {
            (c2V(-1.0, 0.0), dx, c2Sub(mid_a, c2V(e_a.x, 0.0)))
        } else {
            (c2V(1.0, 0.0), dx, c2Add(mid_a, c2V(e_a.x, 0.0)))
        }
    } else {
        if d.y < 0.0 {
            (c2V(0.0, -1.0), dy, c2Sub(mid_a, c2V(0.0, e_a.y)))
        } else {
            (c2V(0.0, 1.0), dy, c2Add(mid_a, c2V(0.0, e_a.y)))
        }
    };
    m.count = 1;
    m.contact_points[0] = p;
    m.depths[0] = depth;
    m.n = n;
}

fn c2KeepDeep(seg: &[c2v; 2], h: c2h, m: &mut c2Manifold) {
    let mut cp = 0;
    for i in 0..2 {
        let p = seg[i];
        let d = c2Dist(h, p);
        if d <= 0.0 {
            m.contact_points[cp] = p;
            m.depths[cp] = -d;
            cp += 1;
        }
    }
    m.count = cp as c_int;
    m.n = h.n;
}

fn c2Incident(incident: &mut [c2v; 2], ip: &c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    let mut index: usize = usize::MAX;
    let mut min_dot: c_float = f32::MAX;
    for i in 0..ip.count as usize {
        let dot = c2Dot(rn_in_incident_space, ip.norms[i]);
        if dot < min_dot {
            min_dot = dot;
            index = i;
        }
    }
    incident[0] = c2Mulxv(ix, ip.verts[index]);
    incident[1] = c2Mulxv(ix, ip.verts[if index + 1 == ip.count as usize { 0 } else { index + 1 }]);
}

fn c2CapsuletoPolyManifold(a: c2Capsule, b: &c2Poly, bx_ptr: *const c2x, m: &mut c2Manifold) {
    m.count = 0;
    let mut a_out = c2v::default();
    let mut b_out = c2v::default();
    let d = c2GJK(
        &a as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        b as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_POLY,
        bx_ptr,
        &mut a_out,
        &mut b_out,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < 1.0e-6 {
        let bx = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            unsafe { *bx_ptr }
        };
        let a_in_b = c2Capsule {
            a: c2MulxvT(bx, a.a),
            b: c2MulxvT(bx, a.b),
            r: a.r,
        };
        let ab = c2Norm(c2Sub(a_in_b.a, a_in_b.b));
        let ab_h0 = c2h {
            n: c2CCW90(ab),
            d: c2Dot(a_in_b.a, c2CCW90(ab)),
        };
        let v0 = c2Support(&b.verts[..b.count as usize], b.count, c2Neg(ab_h0.n));
        let s0 = c2Dist(ab_h0, b.verts[v0 as usize]);
        let ab_h1 = c2h {
            n: c2Skew(ab),
            d: c2Dot(a_in_b.a, c2Skew(ab)),
        };
        let v1 = c2Support(&b.verts[..b.count as usize], b.count, c2Neg(ab_h1.n));
        let s1 = c2Dist(ab_h1, b.verts[v1 as usize]);
        let mut index: usize = usize::MAX;
        let mut sep: c_float = f32::MIN;
        let mut code = 0;
        for i in 0..b.count as usize {
            let h = c2PlaneAt(b, i as c_int);
            let da = c2Dot(a_in_b.a, c2Neg(h.n));
            let db = c2Dot(a_in_b.b, c2Neg(h.n));
            let d = if da > db {
                c2Dist(h, a_in_b.a)
            } else {
                c2Dist(h, a_in_b.b)
            };
            if d > sep {
                sep = d;
                index = i;
            }
        }
        if s0 > sep {
            sep = s0;
            index = v0 as usize;
            code = 1;
        }
        if s1 > sep {
            sep = s1;
            index = v1 as usize;
            code = 2;
        }
        match code {
            0 => {
                let mut seg = [a.a, a.b];
                let mut h = c2h::default();
                if c2SidePlanesFromPoly(&mut seg, bx, b, index as c_int, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&seg, h, m);
                m.n = c2Neg(m.n);
            }
            1 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(&mut incident, b, bx, ab_h0.n);
                let mut h = c2h::default();
                if c2SidePlanes(&mut incident, a_in_b.b, a_in_b.a, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&incident, h, m);
            }
            2 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(&mut incident, b, bx, ab_h1.n);
                let mut h = c2h::default();
                if c2SidePlanes(&mut incident, a_in_b.a, a_in_b.b, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&incident, h, m);
            }
            _ => return,
        }
        for i in 0..m.count as usize {
            m.depths[i] += a.r;
        }
    } else if d < a.r {
        m.count = 1;
        m.n = c2Norm(c2Sub(b_out, a_out));
        m.contact_points[0] = c2Add(a_out, c2Mulvs(m.n, a.r));
        m.depths[0] = a.r - d;
    }
}

fn c2Norms(verts: &[c2v], norms: &mut [c2v], count: c_int) {
    let count = count as usize;
    for i in 0..count {
        let a = i;
        let b = if i + 1 < count { i + 1 } else { 0 };
        let e = c2Sub(verts[b], verts[a]);
        norms[i] = c2Norm(c2CCW90(e));
    }
}

fn c2AABBtoCapsuleManifold(a: c2AABB, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut p = c2Poly::default();
    let verts = c2BBVerts(&a);
    p.verts[..4].copy_from_slice(&verts);
    p.count = 4;
    c2Norms(&p.verts, &mut p.norms, 4);
    c2CapsuletoPolyManifold(b, &p, std::ptr::null(), m);
    m.n = c2Neg(m.n);
}

fn c2CapsuletoCapsuleManifold(a: c2Capsule, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let r = a.r + b.r;
    let mut a_out = c2v::default();
    let mut b_out = c2v::default();
    let d = c2GJK(
        &a as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        &b as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a_out,
        &mut b_out,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < r {
        let n = if d == 0.0 {
            c2Norm(c2Skew(c2Sub(a.b, a.a)))
        } else {
            c2Norm(c2Sub(b_out, a_out))
        };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(b_out, c2Mulvs(n, b.r));
        m.n = n;
    }
}

fn c2Collide(a: *const c_void, type_a: C2_TYPE, b: *const c_void, type_b: C2_TYPE, m: &mut c2Manifold) {
    m.count = 0;
    match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let a_circle = unsafe { &*(a as *const c2Circle) };
            match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    let b_circle = unsafe { &*(b as *const c2Circle) };
                    c2CircletoCircleManifold(*a_circle, *b_circle, m);
                }
                C2_TYPE::C2_TYPE_AABB => {
                    let b_aabb = unsafe { &*(b as *const c2AABB) };
                    c2CircletoAABBManifold(*a_circle, *b_aabb, m);
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    let b_capsule = unsafe { &*(b as *const c2Capsule) };
                    c2CircletoCapsuleManifold(*a_circle, *b_capsule, m);
                }
                _ => {}
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            let a_aabb = unsafe { &*(a as *const c2AABB) };
            match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    let b_circle = unsafe { &*(b as *const c2Circle) };
                    c2CircletoAABBManifold(*b_circle, *a_aabb, m);
                    m.n = c2Neg(m.n);
                }
                C2_TYPE::C2_TYPE_AABB => {
                    let b_aabb = unsafe { &*(b as *const c2AABB) };
                    c2AABBtoAABBManifold(*a_aabb, *b_aabb, m);
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    let b_capsule = unsafe { &*(b as *const c2Capsule) };
                    c2AABBtoCapsuleManifold(*a_aabb, *b_capsule, m);
                }
                _ => {}
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let a_capsule = unsafe { &*(a as *const c2Capsule) };
            match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    let b_circle = unsafe { &*(b as *const c2Circle) };
                    c2CircletoCapsuleManifold(*b_circle, *a_capsule, m);
                    m.n = c2Neg(m.n);
                }
                C2_TYPE::C2_TYPE_AABB => {
                    let b_aabb = unsafe { &*(b as *const c2AABB) };
                    c2AABBtoCapsuleManifold(*b_aabb, *a_capsule, m);
                    m.n = c2Neg(m.n);
                }
                C2_TYPE::C2_TYPE_CAPSULE => {
                    let b_capsule = unsafe { &*(b as *const c2Capsule) };
                    c2CapsuletoCapsuleManifold(*a_capsule, *b_capsule, m);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn ptr_from_parts(typ: C2_TYPE, a: c_float, b: c_float, c: c_float, d: c_float, e: c_float) -> Box<dyn std::any::Any> {
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let circle = c2Circle {
                p: c2V(a, b),
                r: c,
            };
            Box::new(circle)
        }
        C2_TYPE::C2_TYPE_AABB => {
            let aabb = c2AABB {
                min: c2V(a, b),
                max: c2V(c, d),
            };
            Box::new(aabb)
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let capsule = c2Capsule {
                a: c2V(a, b),
                b: c2V(c, d),
                r: e,
            };
            Box::new(capsule)
        }
        _ => panic!("Invalid type"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omni_manifold(
    m: *mut c2Manifold,
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
) {
    let a_boxed = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b_boxed = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    
    let a_ptr: *const c_void = match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => a_boxed.downcast_ref::<c2Circle>().unwrap() as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_AABB => a_boxed.downcast_ref::<c2AABB>().unwrap() as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE => a_boxed.downcast_ref::<c2Capsule>().unwrap() as *const _ as *const c_void,
        _ => std::ptr::null(),
    };
    
    let b_ptr: *const c_void = match type_b {
        C2_TYPE::C2_TYPE_CIRCLE => b_boxed.downcast_ref::<c2Circle>().unwrap() as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_AABB => b_boxed.downcast_ref::<c2AABB>().unwrap() as *const _ as *const c_void,
        C2_TYPE::C2_TYPE_CAPSULE => b_boxed.downcast_ref::<c2Capsule>().unwrap() as *const _ as *const c_void,
        _ => std::ptr::null(),
    };
    
    let m_ref = unsafe { &mut *m };
    c2Collide(a_ptr, type_a, b_ptr, type_b, m_ref);
    
    std::mem::drop(a_boxed);
    std::mem::drop(b_boxed);
}