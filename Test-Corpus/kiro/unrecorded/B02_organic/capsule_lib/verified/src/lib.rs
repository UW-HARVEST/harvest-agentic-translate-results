use std::os::raw::c_int;

#[derive(Clone, Copy)]
#[repr(C)]
struct C2v { x: f32, y: f32 }

#[derive(Clone, Copy)]
#[repr(C)]
struct C2r { c: f32, s: f32 }

#[derive(Clone, Copy)]
#[repr(C)]
struct C2x { p: C2v, r: C2r }

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Circle { p: C2v, r: f32 }

#[derive(Clone, Copy)]
#[repr(C)]
struct C2AABB { min: C2v, max: C2v }

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Capsule { a: C2v, b: C2v, r: f32 }

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2GJKCache { metric: f32, count: i32, i_a: [i32; 3], i_b: [i32; 3], div: f32 }

#[repr(i32)]
#[derive(Clone, Copy)]
enum C2Type { Circle = 0, AABB = 1, Capsule = 2 }

#[repr(C)]
struct C2Proxy { radius: f32, count: i32, verts: [C2v; 8] }

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2sv { s_a: C2v, s_b: C2v, p: C2v, u: f32, i_a: i32, i_b: i32 }

#[repr(C)]
struct C2Simplex { a: C2sv, b: C2sv, c: C2sv, _d: C2sv, div: f32, count: i32 }

impl Default for C2v { fn default() -> Self { C2v { x: 0.0, y: 0.0 } } }

fn c2v(x: f32, y: f32) -> C2v { C2v { x, y } }
fn c2mulvs(a: C2v, b: f32) -> C2v { c2v(a.x * b, a.y * b) }
fn c2maxv(a: C2v, b: C2v) -> C2v { c2v(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y }) }
fn c2minv(a: C2v, b: C2v) -> C2v { c2v(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y }) }
fn c2clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2maxv(lo, c2minv(a, hi)) }
fn c2sub(a: C2v, b: C2v) -> C2v { c2v(a.x - b.x, a.y - b.y) }
fn c2add(a: C2v, b: C2v) -> C2v { c2v(a.x + b.x, a.y + b.y) }
fn c2dot(a: C2v, b: C2v) -> f32 { a.x * b.x + a.y * b.y }
fn c2det2(a: C2v, b: C2v) -> f32 { a.x * b.y - a.y * b.x }
fn c2neg(a: C2v) -> C2v { c2v(-a.x, -a.y) }
fn c2skew(a: C2v) -> C2v { c2v(-a.y, a.x) }
fn c2ccw90(a: C2v) -> C2v { c2v(a.y, -a.x) }
fn c2len(a: C2v) -> f32 { c2dot(a, a).sqrt() }
fn c2div(a: C2v, b: f32) -> C2v { c2mulvs(a, 1.0 / b) }
fn c2norm(a: C2v) -> C2v { c2div(a, c2len(a)) }
fn c2mulrv(a: C2r, b: C2v) -> C2v { c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y) }
fn c2mulrvt(a: C2r, b: C2v) -> C2v { c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y) }
fn c2mulxv(a: C2x, b: C2v) -> C2v { c2add(c2mulrv(a.r, b), a.p) }

fn c2x_identity() -> C2x { C2x { p: c2v(0.0, 0.0), r: C2r { c: 1.0, s: 0.0 } } }

fn c2bb_verts(bb: &C2AABB) -> [C2v; 4] {
    [bb.min, c2v(bb.max.x, bb.min.y), bb.max, c2v(bb.min.x, bb.max.y)]
}

fn c2make_proxy(shape: &dyn std::any::Any, typ: C2Type) -> C2Proxy {
    let mut p = C2Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
    match typ {
        C2Type::Circle => {
            let c = shape.downcast_ref::<C2Circle>().unwrap();
            p.radius = c.r; p.count = 1; p.verts[0] = c.p;
        }
        C2Type::AABB => {
            let bb = shape.downcast_ref::<C2AABB>().unwrap();
            p.radius = 0.0; p.count = 4;
            let v = c2bb_verts(bb);
            p.verts[..4].copy_from_slice(&v);
        }
        C2Type::Capsule => {
            let c = shape.downcast_ref::<C2Capsule>().unwrap();
            p.radius = c.r; p.count = 2; p.verts[0] = c.a; p.verts[1] = c.b;
        }
    }
    p
}

fn c2gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2len(c2sub(s.b.p, s.a.p)),
        3 => c2det2(c2sub(s.b.p, s.a.p), c2sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c22_impl(s: &mut C2Simplex) {
    let a = s.a.p; let b = s.b.p;
    let u = c2dot(b, c2sub(b, a));
    let v = c2dot(a, c2sub(a, b));
    if v <= 0.0 {
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if u <= 0.0 {
        s.a = s.b; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else {
        s.a.u = u; s.b.u = v; s.div = u + v; s.count = 2;
    }
}

fn c23_impl(s: &mut C2Simplex) {
    let a = s.a.p; let b = s.b.p; let c = s.c.p;
    let u_ab = c2dot(b, c2sub(b, a));
    let v_ab = c2dot(a, c2sub(a, b));
    let u_bc = c2dot(c, c2sub(c, b));
    let v_bc = c2dot(b, c2sub(b, c));
    let u_ca = c2dot(a, c2sub(a, c));
    let v_ca = c2dot(c, c2sub(c, a));
    let area = c2det2(c2sub(b, a), c2sub(c, a));
    let u_abc = c2det2(b, c) * area;
    let v_abc = c2det2(c, a) * area;
    let w_abc = c2det2(a, b) * area;
    if v_ab <= 0.0 && u_ca <= 0.0 {
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if u_ab <= 0.0 && v_bc <= 0.0 {
        s.a = s.b; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if u_bc <= 0.0 && v_ca <= 0.0 {
        s.a = s.c; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
        s.a.u = u_ab; s.b.u = v_ab; s.div = u_ab + v_ab; s.count = 2;
    } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
        s.a = s.b; s.b = s.c; s.a.u = u_bc; s.b.u = v_bc; s.div = u_bc + v_bc; s.count = 2;
    } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
        s.b = s.a; s.a = s.c; s.a.u = u_ca; s.b.u = v_ca; s.div = u_ca + v_ca; s.count = 2;
    } else {
        s.a.u = u_abc; s.b.u = v_abc; s.c.u = w_abc; s.div = u_abc + v_abc + w_abc; s.count = 3;
    }
}

fn c2d(s: &C2Simplex) -> C2v {
    match s.count {
        1 => c2neg(s.a.p),
        2 => {
            let ab = c2sub(s.b.p, s.a.p);
            if c2det2(ab, c2neg(s.a.p)) > 0.0 { c2skew(ab) } else { c2ccw90(ab) }
        }
        _ => c2v(0.0, 0.0),
    }
}

fn c2support(verts: &[C2v], count: i32, d: C2v) -> i32 {
    let mut imax = 0;
    let mut dmax = c2dot(verts[0], d);
    for i in 1..count {
        let dot = c2dot(verts[i as usize], d);
        if dot > dmax { imax = i; dmax = dot; }
    }
    imax
}

fn c2witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => { *a = s.a.s_a; *b = s.a.s_b; }
        2 => {
            *a = c2add(c2mulvs(s.a.s_a, den * s.a.u), c2mulvs(s.b.s_a, den * s.b.u));
            *b = c2add(c2mulvs(s.a.s_b, den * s.a.u), c2mulvs(s.b.s_b, den * s.b.u));
        }
        3 => {
            *a = c2add(c2add(c2mulvs(s.a.s_a, den * s.a.u), c2mulvs(s.b.s_a, den * s.b.u)), c2mulvs(s.c.s_a, den * s.c.u));
            *b = c2add(c2add(c2mulvs(s.a.s_b, den * s.a.u), c2mulvs(s.b.s_b, den * s.b.u)), c2mulvs(s.c.s_b, den * s.c.u));
        }
        _ => { *a = c2v(0.0, 0.0); *b = c2v(0.0, 0.0); }
    }
}

fn c2l(s: &C2Simplex) -> C2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2add(c2mulvs(s.a.p, den * s.a.u), c2mulvs(s.b.p, den * s.b.u)),
        _ => c2v(0.0, 0.0),
    }
}

// Simplex vertex accessor by index (mirrors C's `verts + i` pointer arithmetic on contiguous a,b,c,d)
fn sv_get(s: &C2Simplex, i: i32) -> &C2sv {
    match i { 0 => &s.a, 1 => &s.b, 2 => &s.c, _ => &s._d }
}
fn sv_set(s: &mut C2Simplex, i: i32, v: C2sv) {
    match i { 0 => s.a = v, 1 => s.b = v, 2 => s.c = v, _ => s._d = v }
}

fn c2gjk(
    a_shape: &dyn std::any::Any, type_a: C2Type, ax_ptr: Option<&C2x>,
    b_shape: &dyn std::any::Any, type_b: C2Type, bx_ptr: Option<&C2x>,
    out_a: Option<&mut C2v>, out_b: Option<&mut C2v>,
    use_radius: bool, iterations: Option<&mut i32>, cache: Option<&mut C2GJKCache>,
) -> f32 {
    let ax = ax_ptr.copied().unwrap_or_else(c2x_identity);
    let bx = bx_ptr.copied().unwrap_or_else(c2x_identity);
    let p_a = c2make_proxy(a_shape, type_a);
    let p_b = c2make_proxy(b_shape, type_b);

    let mut s = C2Simplex { a: C2sv::default(), b: C2sv::default(), c: C2sv::default(), _d: C2sv::default(), div: 0.0, count: 0 };
    let mut cache_was_read = false;

    if let Some(ref cache) = cache {
        if cache.count != 0 {
            for i in 0..cache.count {
                let ia = cache.i_a[i as usize];
                let ib = cache.i_b[i as usize];
                let sa = c2mulxv(ax, p_a.verts[ia as usize]);
                let sb = c2mulxv(bx, p_b.verts[ib as usize]);
                let mut v = C2sv::default();
                v.i_a = ia; v.s_a = sa; v.i_b = ib; v.s_b = sb;
                v.p = c2sub(v.s_b, v.s_a); v.u = 0.0;
                sv_set(&mut s, i, v);
            }
            s.count = cache.count;
            s.div = cache.div;
            let metric_old = cache.metric;
            let metric = c2gjk_simplex_metric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = true;
            }
        }
    }

    if !cache_was_read {
        s.a.i_a = 0; s.a.i_b = 0;
        s.a.s_a = c2mulxv(ax, p_a.verts[0]);
        s.a.s_b = c2mulxv(bx, p_b.verts[0]);
        s.a.p = c2sub(s.a.s_b, s.a.s_a);
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    }

    let mut save_a = [0i32; 3];
    let mut save_b = [0i32; 3];
    let mut d0: f32 = f32::MAX;
    let mut d1: f32 = f32::MAX;
    let mut iter = 0;
    let mut hit = false;

    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count {
            let v = sv_get(&s, i);
            save_a[i as usize] = v.i_a;
            save_b[i as usize] = v.i_b;
        }
        match s.count {
            1 => {}
            2 => c22_impl(&mut s),
            3 => c23_impl(&mut s),
            _ => {}
        }
        if s.count == 3 { hit = true; break; }
        let p = c2l(&s);
        d1 = c2dot(p, p);
        if d1 > d0 { break; }
        d0 = d1;
        let d = c2d(&s);
        if c2dot(d, d) < f32::EPSILON * f32::EPSILON { break; }
        let ia = c2support(&p_a.verts, p_a.count, c2mulrvt(ax.r, c2neg(d)));
        let sa = c2mulxv(ax, p_a.verts[ia as usize]);
        let ib = c2support(&p_b.verts, p_b.count, c2mulrvt(bx.r, d));
        let sb = c2mulxv(bx, p_b.verts[ib as usize]);
        let mut v = C2sv::default();
        v.i_a = ia; v.s_a = sa; v.i_b = ib; v.s_b = sb;
        v.p = c2sub(v.s_b, v.s_a);
        let idx = s.count;
        sv_set(&mut s, idx, v);
        let mut dup = false;
        for i in 0..save_count {
            if ia == save_a[i as usize] && ib == save_b[i as usize] { dup = true; break; }
        }
        if dup { break; }
        s.count += 1;
        iter += 1;
    }

    let mut wa = c2v(0.0, 0.0);
    let mut wb = c2v(0.0, 0.0);
    c2witness(&s, &mut wa, &mut wb);
    let mut dist = c2len(c2sub(wa, wb));

    if hit {
        wa = wb;
        dist = 0.0;
    } else if use_radius {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > f32::EPSILON {
            dist -= r_a + r_b;
            let n = c2norm(c2sub(wb, wa));
            wa = c2add(wa, c2mulvs(n, r_a));
            wb = c2sub(wb, c2mulvs(n, r_b));
            if wa.x == wb.x && wa.y == wb.y { dist = 0.0; }
        } else {
            let p = c2mulvs(c2add(wa, wb), 0.5);
            wa = p; wb = p; dist = 0.0;
        }
    }

    if let Some(cache) = cache {
        cache.metric = c2gjk_simplex_metric(&s);
        cache.count = s.count;
        for i in 0..s.count {
            let v = sv_get(&s, i);
            cache.i_a[i as usize] = v.i_a;
            cache.i_b[i as usize] = v.i_b;
        }
        cache.div = s.div;
    }
    if let Some(out_a) = out_a { *out_a = wa; }
    if let Some(out_b) = out_b { *out_b = wb; }
    if let Some(iterations) = iterations { *iterations = iter; }
    dist
}

fn c2aabb_to_aabb(a: C2AABB, b: C2AABB) -> bool {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    !(d0 | d1 | d2 | d3)
}

fn c2aabb_to_capsule(a: C2AABB, b: C2Capsule) -> bool {
    c2gjk(&a, C2Type::AABB, None, &b, C2Type::Capsule, None, None, None, true, None, None) == 0.0
}

fn c2capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> bool {
    c2gjk(&a, C2Type::Capsule, None, &b, C2Type::Capsule, None, None, None, true, None, None) == 0.0
}

fn c2circle_to_circle(a: C2Circle, b: C2Circle) -> bool {
    let c = c2sub(b.p, a.p);
    let d2 = c2dot(c, c);
    let r2 = a.r + b.r;
    d2 < r2 * r2
}

fn c2circle_to_aabb(a: C2Circle, b: C2AABB) -> bool {
    let l = c2clampv(a.p, b.min, b.max);
    let ab = c2sub(a.p, l);
    let d2 = c2dot(ab, ab);
    d2 < a.r * a.r
}

fn c2circle_to_capsule(a: C2Circle, b: C2Capsule) -> bool {
    let n = c2sub(b.b, b.a);
    let ap = c2sub(a.p, b.a);
    let da = c2dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2dot(ap, ap);
    } else {
        let db = c2dot(c2sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2sub(ap, c2mulvs(n, da / c2dot(n, n)));
            d2 = c2dot(e, e);
        } else {
            let bp = c2sub(a.p, b.b);
            d2 = c2dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    d2 < r * r
}

fn c2collided(a: &dyn std::any::Any, type_a: C2Type, b: &dyn std::any::Any, type_b: C2Type) -> bool {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => c2circle_to_circle(*a.downcast_ref::<C2Circle>().unwrap(), *b.downcast_ref::<C2Circle>().unwrap()),
            C2Type::AABB => c2circle_to_aabb(*a.downcast_ref::<C2Circle>().unwrap(), *b.downcast_ref::<C2AABB>().unwrap()),
            C2Type::Capsule => c2circle_to_capsule(*a.downcast_ref::<C2Circle>().unwrap(), *b.downcast_ref::<C2Capsule>().unwrap()),
        },
        C2Type::AABB => match type_b {
            C2Type::Circle => c2circle_to_aabb(*b.downcast_ref::<C2Circle>().unwrap(), *a.downcast_ref::<C2AABB>().unwrap()),
            C2Type::AABB => c2aabb_to_aabb(*a.downcast_ref::<C2AABB>().unwrap(), *b.downcast_ref::<C2AABB>().unwrap()),
            C2Type::Capsule => c2aabb_to_capsule(*a.downcast_ref::<C2AABB>().unwrap(), *b.downcast_ref::<C2Capsule>().unwrap()),
        },
        C2Type::Capsule => match type_b {
            C2Type::Circle => c2circle_to_capsule(*b.downcast_ref::<C2Circle>().unwrap(), *a.downcast_ref::<C2Capsule>().unwrap()),
            C2Type::AABB => c2aabb_to_capsule(*b.downcast_ref::<C2AABB>().unwrap(), *a.downcast_ref::<C2Capsule>().unwrap()),
            C2Type::Capsule => c2capsule_to_capsule(*a.downcast_ref::<C2Capsule>().unwrap(), *b.downcast_ref::<C2Capsule>().unwrap()),
        },
    }
}

// --- FFI export wrappers matching C symbol names exactly ---

use std::ffi::c_void;

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v { c2v(x, y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: C2v, b: f32) -> C2v { c2mulvs(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v { c2maxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v { c2minv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2clampv(a, lo, hi) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: C2v, b: C2v) -> C2v { c2sub(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: C2v, b: C2v) -> C2v { c2add(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 { c2dot(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 { c2det2(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: C2v) -> C2v { c2neg(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v { c2skew(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v { c2ccw90(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 { c2len(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v { c2div(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v { c2norm(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v { c2mulrv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v { c2mulrvt(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v { c2mulxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r { C2r { c: 1.0, s: 0.0 } }

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x { c2x_identity() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut C2v, bb: *mut C2AABB) {
    let v = c2bb_verts(unsafe { &*bb });
    unsafe {
        *out.add(0) = v[0];
        *out.add(1) = v[1];
        *out.add(2) = v[2];
        *out.add(3) = v[3];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, typ: C2Type, p: *mut C2Proxy) {
    let pp = unsafe { &mut *p };
    match typ {
        C2Type::Circle => {
            let c = unsafe { &*(shape as *const C2Circle) };
            pp.radius = c.r; pp.count = 1; pp.verts[0] = c.p;
        }
        C2Type::AABB => {
            let bb = unsafe { &*(shape as *const C2AABB) };
            pp.radius = 0.0; pp.count = 4;
            let v = c2bb_verts(bb);
            pp.verts[..4].copy_from_slice(&v);
        }
        C2Type::Capsule => {
            let c = unsafe { &*(shape as *const C2Capsule) };
            pp.radius = c.r; pp.count = 2; pp.verts[0] = c.a; pp.verts[1] = c.b;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut C2Simplex) -> f32 {
    c2gjk_simplex_metric(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut C2Simplex) {
    c22_impl(unsafe { &mut *s })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut C2Simplex) {
    c23_impl(unsafe { &mut *s })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut C2Simplex) -> C2v {
    c2d(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
    let slice = unsafe { std::slice::from_raw_parts(verts, count as usize) };
    c2support(slice, count, d)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut C2Simplex, a: *mut C2v, b: *mut C2v) {
    c2witness(unsafe { &*s }, unsafe { &mut *a }, unsafe { &mut *b })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut C2Simplex) -> C2v {
    c2l(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    a: *const c_void, type_a: C2Type, ax_ptr: *const C2x,
    b: *const c_void, type_b: C2Type, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: c_int, iterations: *mut c_int, cache: *mut C2GJKCache,
) -> f32 {
    unsafe {
        let ax_opt = if ax_ptr.is_null() { None } else { Some(&*ax_ptr) };
        let bx_opt = if bx_ptr.is_null() { None } else { Some(&*bx_ptr) };
        let out_a_opt = if out_a.is_null() { None } else { Some(&mut *out_a) };
        let out_b_opt = if out_b.is_null() { None } else { Some(&mut *out_b) };
        let iter_opt = if iterations.is_null() { None } else { Some(&mut *iterations) };
        let cache_opt = if cache.is_null() { None } else { Some(&mut *cache) };

        // Dereference shape pointers based on type
        match type_a {
            C2Type::Circle => {
                let sa = &*(a as *const C2Circle);
                match type_b {
                    C2Type::Circle => c2gjk(sa, type_a, ax_opt, &*(b as *const C2Circle), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                    C2Type::AABB => c2gjk(sa, type_a, ax_opt, &*(b as *const C2AABB), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                    C2Type::Capsule => c2gjk(sa, type_a, ax_opt, &*(b as *const C2Capsule), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                }
            }
            C2Type::AABB => {
                let sa = &*(a as *const C2AABB);
                match type_b {
                    C2Type::Circle => c2gjk(sa, type_a, ax_opt, &*(b as *const C2Circle), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                    C2Type::AABB => c2gjk(sa, type_a, ax_opt, &*(b as *const C2AABB), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                    C2Type::Capsule => c2gjk(sa, type_a, ax_opt, &*(b as *const C2Capsule), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                }
            }
            C2Type::Capsule => {
                let sa = &*(a as *const C2Capsule);
                match type_b {
                    C2Type::Circle => c2gjk(sa, type_a, ax_opt, &*(b as *const C2Circle), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                    C2Type::AABB => c2gjk(sa, type_a, ax_opt, &*(b as *const C2AABB), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                    C2Type::Capsule => c2gjk(sa, type_a, ax_opt, &*(b as *const C2Capsule), type_b, bx_opt, out_a_opt, out_b_opt, use_radius != 0, iter_opt, cache_opt),
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int {
    c2aabb_to_aabb(a, b) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(a: C2AABB, b: C2Capsule) -> c_int {
    if c2gjk(&a, C2Type::AABB, None, &b, C2Type::Capsule, None, None, None, true, None, None) != 0.0 { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(a: C2Capsule, b: C2Capsule) -> c_int {
    if c2gjk(&a, C2Type::Capsule, None, &b, C2Type::Capsule, None, None, None, true, None, None) != 0.0 { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    c2circle_to_circle(a, b) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2AABB) -> c_int {
    c2circle_to_aabb(a, b) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: C2Circle, b: C2Capsule) -> c_int {
    c2circle_to_capsule(a, b) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(a: *const c_void, type_a: C2Type, b: *const c_void, type_b: C2Type) -> c_int {
    unsafe {
        match type_a {
            C2Type::Circle => match type_b {
                C2Type::Circle => c2circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle)) as c_int,
                C2Type::AABB => c2circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB)) as c_int,
                C2Type::Capsule => c2circle_to_capsule(*(a as *const C2Circle), *(b as *const C2Capsule)) as c_int,
            },
            C2Type::AABB => match type_b {
                C2Type::Circle => c2circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB)) as c_int,
                C2Type::AABB => c2aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB)) as c_int,
                C2Type::Capsule => c2aabb_to_capsule(*(a as *const C2AABB), *(b as *const C2Capsule)) as c_int,
            },
            C2Type::Capsule => match type_b {
                C2Type::Circle => c2circle_to_capsule(*(b as *const C2Circle), *(a as *const C2Capsule)) as c_int,
                C2Type::AABB => c2aabb_to_capsule(*(b as *const C2AABB), *(a as *const C2Capsule)) as c_int,
                C2Type::Capsule => c2capsule_to_capsule(*(a as *const C2Capsule), *(b as *const C2Capsule)) as c_int,
            },
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let capsule_in = C2Capsule { a: c2v(min_x, min_y), b: c2v(max_x, max_y), r };
    let circle = C2Circle { p: c2v(-70.0, 0.0), r: 20.0 };
    let aabb = C2AABB { min: c2v(-40.0, -40.0), max: c2v(-15.0, -15.0) };
    let capsule_obj = C2Capsule { a: c2v(-40.0, 40.0), b: c2v(-20.0, 100.0), r: 10.0 };

    result += c2collided(&circle, C2Type::Circle, &capsule_in, C2Type::Capsule) as c_int;
    result += (c2collided(&aabb, C2Type::AABB, &capsule_in, C2Type::Capsule) as c_int) << 1;
    result += (c2collided(&capsule_obj, C2Type::Capsule, &capsule_in, C2Type::Capsule) as c_int) << 2;

    result
}
