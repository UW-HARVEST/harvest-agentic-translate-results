use std::ffi::c_int;

#[derive(Clone, Copy)]
struct C2v { x: f32, y: f32 }

#[derive(Clone, Copy)]
struct C2r { c: f32, s: f32 }

#[derive(Clone, Copy)]
struct C2x { p: C2v, r: C2r }

#[derive(Clone, Copy)]
struct C2Circle { p: C2v, r: f32 }

#[derive(Clone, Copy)]
struct C2AABB { min: C2v, max: C2v }

#[derive(Clone, Copy)]
struct C2Capsule { a: C2v, b: C2v, r: f32 }

#[derive(Clone, Copy)]
struct C2GJKCache { metric: f32, count: i32, i_a: [i32; 3], i_b: [i32; 3], div: f32 }

#[derive(Clone, Copy)]
struct C2Proxy { radius: f32, count: i32, verts: [C2v; 8] }

#[derive(Clone, Copy)]
struct C2sv { s_a: C2v, s_b: C2v, p: C2v, u: f32, i_a: i32, i_b: i32 }

#[derive(Clone, Copy)]
struct C2Simplex { a: C2sv, b: C2sv, c: C2sv, _d: C2sv, div: f32, count: i32 }

const C2_TYPE_CAPSULE: c_int = 0;
const C2_TYPE_CIRCLE: c_int = 1;
const C2_TYPE_AABB: c_int = 2;

fn c2v(x: f32, y: f32) -> C2v { C2v { x, y } }

fn c2mulvs(mut a: C2v, b: f32) -> C2v { a.x *= b; a.y *= b; a }

fn c2maxv(a: C2v, b: C2v) -> C2v {
    c2v(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

fn c2minv(a: C2v, b: C2v) -> C2v {
    c2v(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

fn c2clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2maxv(lo, c2minv(a, hi)) }

fn c2sub(a: C2v, b: C2v) -> C2v { c2v(a.x - b.x, a.y - b.y) }
fn c2add(a: C2v, b: C2v) -> C2v { c2v(a.x + b.x, a.y + b.y) }
fn c2dot(a: C2v, b: C2v) -> f32 { a.x * b.x + a.y * b.y }
fn c2neg(a: C2v) -> C2v { c2v(-a.x, -a.y) }
fn c2det2(a: C2v, b: C2v) -> f32 { a.x * b.y - a.y * b.x }
fn c2len(a: C2v) -> f32 { c2dot(a, a).sqrt() }
fn c2norm(a: C2v) -> C2v { c2mulvs(a, 1.0 / c2len(a)) }

fn c2rot_identity() -> C2r { C2r { c: 1.0, s: 0.0 } }
fn c2x_identity() -> C2x { C2x { p: c2v(0.0, 0.0), r: c2rot_identity() } }

fn c2mulrv(a: C2r, b: C2v) -> C2v { c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y) }
fn c2mulrv_t(a: C2r, b: C2v) -> C2v { c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y) }
fn c2mulxv(a: C2x, b: C2v) -> C2v { c2add(c2mulrv(a.r, b), a.p) }

fn c2skew(a: C2v) -> C2v { C2v { x: -a.y, y: a.x } }
fn c2ccw90(a: C2v) -> C2v { C2v { x: a.y, y: -a.x } }

fn c2bb_verts(bb: &C2AABB) -> [C2v; 4] {
    [bb.min, c2v(bb.max.x, bb.min.y), bb.max, c2v(bb.min.x, bb.max.y)]
}

fn c2make_proxy(shape: *const u8, typ: c_int) -> C2Proxy {
    let mut p = C2Proxy { radius: 0.0, count: 0, verts: [c2v(0.0, 0.0); 8] };
    unsafe {
        match typ {
            C2_TYPE_CIRCLE => {
                let c = &*(shape as *const C2Circle);
                p.radius = c.r; p.count = 1; p.verts[0] = c.p;
            }
            C2_TYPE_AABB => {
                let bb = &*(shape as *const C2AABB);
                p.radius = 0.0; p.count = 4;
                let v = c2bb_verts(bb);
                p.verts[..4].copy_from_slice(&v);
            }
            C2_TYPE_CAPSULE => {
                let c = &*(shape as *const C2Capsule);
                p.radius = c.r; p.count = 2; p.verts[0] = c.a; p.verts[1] = c.b;
            }
            _ => {}
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

fn c22(s: &mut C2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
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

fn c23(s: &mut C2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
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

fn vert_at(s: &C2Simplex, i: i32) -> &C2sv {
    match i { 0 => &s.a, 1 => &s.b, 2 => &s.c, _ => &s._d }
}

fn vert_at_mut(s: &mut C2Simplex, i: i32) -> &mut C2sv {
    match i { 0 => &mut s.a, 1 => &mut s.b, 2 => &mut s.c, _ => &mut s._d }
}

const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

fn c2gjk(
    a_ptr: *const u8, type_a: c_int, ax_ptr: *const C2x,
    b_ptr: *const u8, type_b: c_int, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: i32, iterations: *mut i32, cache: *mut C2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() { c2x_identity() } else { unsafe { *ax_ptr } };
    let bx = if bx_ptr.is_null() { c2x_identity() } else { unsafe { *bx_ptr } };
    let p_a = c2make_proxy(a_ptr, type_a);
    let p_b = c2make_proxy(b_ptr, type_b);

    let zero_sv = C2sv { s_a: c2v(0.0,0.0), s_b: c2v(0.0,0.0), p: c2v(0.0,0.0), u: 0.0, i_a: 0, i_b: 0 };
    let mut s = C2Simplex { a: zero_sv, b: zero_sv, c: zero_sv, _d: zero_sv, div: 0.0, count: 0 };

    let mut cache_was_read = false;
    if !cache.is_null() {
        let ca = unsafe { &*cache };
        if ca.count != 0 {
            for i in 0..ca.count {
                let ia = ca.i_a[i as usize];
                let ib = ca.i_b[i as usize];
                let sa = c2mulxv(ax, p_a.verts[ia as usize]);
                let sb = c2mulxv(bx, p_b.verts[ib as usize]);
                let v = vert_at_mut(&mut s, i);
                v.i_a = ia; v.s_a = sa; v.i_b = ib; v.s_b = sb;
                v.p = c2sub(v.s_b, v.s_a); v.u = 0.0;
            }
            s.count = ca.count;
            s.div = ca.div;
            let metric_old = ca.metric;
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
    let mut d0 = FLT_MAX;
    let mut d1: f32;
    let mut iter = 0;
    let mut hit = false;

    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count {
            let v = vert_at(&s, i);
            save_a[i as usize] = v.i_a;
            save_b[i as usize] = v.i_b;
        }
        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }
        if s.count == 3 { hit = true; break; }
        let p = c2l(&s);
        d1 = c2dot(p, p);
        if d1 > d0 { break; }
        d0 = d1;
        let d = c2d(&s);
        if c2dot(d, d) < FLT_EPSILON * FLT_EPSILON { break; }

        let ia = c2support(&p_a.verts, p_a.count, c2mulrv_t(ax.r, c2neg(d)));
        let sa = c2mulxv(ax, p_a.verts[ia as usize]);
        let ib = c2support(&p_b.verts, p_b.count, c2mulrv_t(bx.r, d));
        let sb = c2mulxv(bx, p_b.verts[ib as usize]);

        let idx = s.count;
        let v = vert_at_mut(&mut s, idx);
        v.i_a = ia; v.s_a = sa; v.i_b = ib; v.s_b = sb;
        v.p = c2sub(v.s_b, v.s_a);

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
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > FLT_EPSILON {
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

    if !cache.is_null() {
        let ca = unsafe { &mut *cache };
        ca.metric = c2gjk_simplex_metric(&s);
        ca.count = s.count;
        for i in 0..s.count {
            let v = vert_at(&s, i);
            ca.i_a[i as usize] = v.i_a;
            ca.i_b[i as usize] = v.i_b;
        }
        ca.div = s.div;
    }
    if !out_a.is_null() { unsafe { *out_a = wa; } }
    if !out_b.is_null() { unsafe { *out_b = wb; } }
    if !iterations.is_null() { unsafe { *iterations = iter; } }
    dist
}

fn c2aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    if d0 | d1 | d2 | d3 { 0 } else { 1 }
}

fn c2aabb_to_capsule(a: C2AABB, b: C2Capsule) -> i32 {
    let ptr_a = &a as *const C2AABB as *const u8;
    let ptr_b = &b as *const C2Capsule as *const u8;
    if c2gjk(ptr_a, C2_TYPE_AABB, std::ptr::null(), ptr_b, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut()) != 0.0 { 0 } else { 1 }
}

fn c2capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> i32 {
    let ptr_a = &a as *const C2Capsule as *const u8;
    let ptr_b = &b as *const C2Capsule as *const u8;
    if c2gjk(ptr_a, C2_TYPE_CAPSULE, std::ptr::null(), ptr_b, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut()) != 0.0 { 0 } else { 1 }
}

fn c2circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
    let c = c2sub(b.p, a.p);
    let d2 = c2dot(c, c);
    let r2 = (a.r + b.r) * (a.r + b.r);
    if d2 < r2 { 1 } else { 0 }
}

fn c2circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
    let l = c2clampv(a.p, b.min, b.max);
    let ab = c2sub(a.p, l);
    let d2 = c2dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 { 1 } else { 0 }
}

fn c2circle_to_capsule(a: C2Circle, b: C2Capsule) -> i32 {
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
    if d2 < r * r { 1 } else { 0 }
}

fn c2collided(a: *const u8, type_a: c_int, b: *const u8, type_b: c_int) -> c_int {
    unsafe {
        match type_a {
            C2_TYPE_CIRCLE => match type_b {
                C2_TYPE_CIRCLE => c2circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle)),
                C2_TYPE_AABB => c2circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB)),
                C2_TYPE_CAPSULE => c2circle_to_capsule(*(a as *const C2Circle), *(b as *const C2Capsule)),
                _ => 0,
            },
            C2_TYPE_AABB => match type_b {
                C2_TYPE_CIRCLE => c2circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB)),
                C2_TYPE_AABB => c2aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB)),
                C2_TYPE_CAPSULE => c2aabb_to_capsule(*(a as *const C2AABB), *(b as *const C2Capsule)),
                _ => 0,
            },
            C2_TYPE_CAPSULE => match type_b {
                C2_TYPE_CIRCLE => c2circle_to_capsule(*(b as *const C2Circle), *(a as *const C2Capsule)),
                C2_TYPE_AABB => c2aabb_to_capsule(*(b as *const C2AABB), *(a as *const C2Capsule)),
                C2_TYPE_CAPSULE => c2capsule_to_capsule(*(a as *const C2Capsule), *(b as *const C2Capsule)),
                _ => 0,
            },
            _ => 0,
        }
    }
}

fn ptr_from_parts(typ: c_int, a: f32, b: f32, c: f32, d: f32, e: f32) -> *mut u8 {
    match typ {
        C2_TYPE_CIRCLE => {
            let p = Box::new(C2Circle { p: c2v(a, b), r: c });
            Box::into_raw(p) as *mut u8
        }
        C2_TYPE_AABB => {
            let p = Box::new(C2AABB { min: c2v(a, b), max: c2v(c, d) });
            Box::into_raw(p) as *mut u8
        }
        C2_TYPE_CAPSULE => {
            let p = Box::new(C2Capsule { a: c2v(a, b), b: c2v(c, d), r: e });
            Box::into_raw(p) as *mut u8
        }
        _ => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omni_collide(
    type_a: c_int, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
    type_b: c_int, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32,
) -> c_int {
    let a = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    let result = c2collided(a, type_a, b, type_b);
    // Note: C code leaks memory here (no free). We reproduce the leak for byte-identical behavior,
    // but we could also drop. Since the C doesn't free, we don't either — but Box leak is fine.
    // Actually let's not leak in Rust — the C leaks but the behavior (return value) is identical.
    if !a.is_null() { unsafe { drop_alloc(a, type_a); } }
    if !b.is_null() { unsafe { drop_alloc(b, type_b); } }
    result
}

unsafe fn drop_alloc(ptr: *mut u8, typ: c_int) {
    match typ {
        C2_TYPE_CIRCLE => { let _ = unsafe { Box::from_raw(ptr as *mut C2Circle) }; }
        C2_TYPE_AABB => { let _ = unsafe { Box::from_raw(ptr as *mut C2AABB) }; }
        C2_TYPE_CAPSULE => { let _ = unsafe { Box::from_raw(ptr as *mut C2Capsule) }; }
        _ => {}
    }
}
