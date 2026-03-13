#![allow(non_camel_case_types)]
use std::os::raw::c_int;

// --- C2_TYPE enum ---
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE = 0,
    C2_TYPE_CIRCLE = 1,
    C2_TYPE_AABB = 2,
}

// --- Structs ---
#[derive(Clone, Copy, Default)]
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
struct C2GJKCache { metric: f32, count: c_int, i_a: [c_int; 3], i_b: [c_int; 3], div: f32 }

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2Proxy { radius: f32, count: c_int, verts: [C2v; 8] }

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct C2sv { s_a: C2v, s_b: C2v, p: C2v, u: f32, i_a: c_int, i_b: c_int }

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Simplex { a: C2sv, b: C2sv, c: C2sv, d: C2sv, div: f32, count: c_int }

// --- Vector helpers ---
fn c2v_new(x: f32, y: f32) -> C2v { C2v { x, y } }
fn c2_mulvs(mut a: C2v, b: f32) -> C2v { a.x *= b; a.y *= b; a }
fn c2_maxv(a: C2v, b: C2v) -> C2v { c2v_new(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y }) }
fn c2_minv(a: C2v, b: C2v) -> C2v { c2v_new(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y }) }
fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2_maxv(lo, c2_minv(a, hi)) }
fn c2_sub(mut a: C2v, b: C2v) -> C2v { a.x -= b.x; a.y -= b.y; a }
fn c2_add(mut a: C2v, b: C2v) -> C2v { a.x += b.x; a.y += b.y; a }
fn c2_dot(a: C2v, b: C2v) -> f32 { a.x * b.x + a.y * b.y }
fn c2_neg(a: C2v) -> C2v { c2v_new(-a.x, -a.y) }
fn c2_len(a: C2v) -> f32 { c2_dot(a, a).sqrt() }
fn c2_det2(a: C2v, b: C2v) -> f32 { a.x * b.y - a.y * b.x }
fn c2_skew(a: C2v) -> C2v { C2v { x: -a.y, y: a.x } }
fn c2_ccw90(a: C2v) -> C2v { C2v { x: a.y, y: -a.x } }
fn c2_div(a: C2v, b: f32) -> C2v { c2_mulvs(a, 1.0 / b) }
fn c2_norm(a: C2v) -> C2v { c2_div(a, c2_len(a)) }

fn c2_rot_identity() -> C2r { C2r { c: 1.0, s: 0.0 } }
fn c2x_identity() -> C2x { C2x { p: c2v_new(0.0, 0.0), r: c2_rot_identity() } }
fn c2_mulrv(a: C2r, b: C2v) -> C2v { c2v_new(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y) }
fn c2_mulxv(a: C2x, b: C2v) -> C2v { c2_add(c2_mulrv(a.r, b), a.p) }
fn c2_mulrv_t(a: C2r, b: C2v) -> C2v { c2v_new(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y) }

// --- Proxy ---
fn c2_bb_verts(out: &mut [C2v; 8], bb: &C2AABB) {
    out[0] = bb.min;
    out[1] = c2v_new(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v_new(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: *const u8, typ: C2_TYPE, p: &mut C2Proxy) {
    unsafe {
        match typ {
            C2_TYPE::C2_TYPE_CIRCLE => {
                let c = &*(shape as *const C2Circle);
                p.radius = c.r; p.count = 1; p.verts[0] = c.p;
            }
            C2_TYPE::C2_TYPE_AABB => {
                let bb = &*(shape as *const C2AABB);
                p.radius = 0.0; p.count = 4;
                c2_bb_verts(&mut p.verts, bb);
            }
            C2_TYPE::C2_TYPE_CAPSULE => {
                let c = &*(shape as *const C2Capsule);
                p.radius = c.r; p.count = 2;
                p.verts[0] = c.a; p.verts[1] = c.b;
            }
        }
    }
}

// --- Simplex operations ---
fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c22(s: &mut C2Simplex) {
    let a = s.a.p; let b = s.b.p;
    let u = c2_dot(b, c2_sub(b, a));
    let v = c2_dot(a, c2_sub(a, b));
    if v <= 0.0 {
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if u <= 0.0 {
        s.a = s.b; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else {
        s.a.u = u; s.b.u = v; s.div = u + v; s.count = 2;
    }
}

fn c23(s: &mut C2Simplex) {
    let a = s.a.p; let b = s.b.p; let c = s.c.p;
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

fn c2_d(s: &C2Simplex) -> C2v {
    match s.count {
        1 => c2_neg(s.a.p),
        2 => {
            let ab = c2_sub(s.b.p, s.a.p);
            if c2_det2(ab, c2_neg(s.a.p)) > 0.0 { c2_skew(ab) } else { c2_ccw90(ab) }
        }
        _ => c2v_new(0.0, 0.0),
    }
}

fn c2_support(verts: &[C2v; 8], count: c_int, d: C2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2_dot(verts[0], d);
    for i in 1..count {
        let dot = c2_dot(verts[i as usize], d);
        if dot > dmax { imax = i; dmax = dot; }
    }
    imax
}

fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => { *a = s.a.s_a; *b = s.a.s_b; }
        2 => {
            *a = c2_add(c2_mulvs(s.a.s_a, den * s.a.u), c2_mulvs(s.b.s_a, den * s.b.u));
            *b = c2_add(c2_mulvs(s.a.s_b, den * s.a.u), c2_mulvs(s.b.s_b, den * s.b.u));
        }
        3 => {
            *a = c2_add(c2_add(c2_mulvs(s.a.s_a, den * s.a.u), c2_mulvs(s.b.s_a, den * s.b.u)), c2_mulvs(s.c.s_a, den * s.c.u));
            *b = c2_add(c2_add(c2_mulvs(s.a.s_b, den * s.a.u), c2_mulvs(s.b.s_b, den * s.b.u)), c2_mulvs(s.c.s_b, den * s.c.u));
        }
        _ => { *a = c2v_new(0.0, 0.0); *b = c2v_new(0.0, 0.0); }
    }
}

fn c2_l(s: &C2Simplex) -> C2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2v_new(0.0, 0.0),
    }
}

// --- GJK ---
const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

fn c2_gjk(
    a_shape: *const u8, type_a: C2_TYPE, ax_ptr: *const C2x,
    b_shape: *const u8, type_b: C2_TYPE, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: c_int, iterations: *mut c_int, cache: *mut C2GJKCache,
) -> f32 {
    unsafe {
        let ax = if ax_ptr.is_null() { c2x_identity() } else { *ax_ptr };
        let bx = if bx_ptr.is_null() { c2x_identity() } else { *bx_ptr };
        let mut p_a = C2Proxy::default();
        let mut p_b = C2Proxy::default();
        c2_make_proxy(a_shape, type_a, &mut p_a);
        c2_make_proxy(b_shape, type_b, &mut p_b);

        let mut s = C2Simplex {
            a: C2sv::default(), b: C2sv::default(), c: C2sv::default(), d: C2sv::default(),
            div: 0.0, count: 0,
        };
        let verts_base: *mut C2sv = &mut s.a as *mut C2sv;

        let mut cache_was_read = false;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                for i in 0..(*cache).count {
                    let ia = (*cache).i_a[i as usize];
                    let ib = (*cache).i_b[i as usize];
                    let sa = c2_mulxv(ax, p_a.verts[ia as usize]);
                    let sb = c2_mulxv(bx, p_b.verts[ib as usize]);
                    let v = &mut *verts_base.add(i as usize);
                    v.i_a = ia; v.s_a = sa; v.i_b = ib; v.s_b = sb;
                    v.p = c2_sub(v.s_b, v.s_a); v.u = 0.0;
                }
                s.count = (*cache).count;
                s.div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2_gjk_simplex_metric(&s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                    cache_was_read = true;
                }
            }
        }

        if !cache_was_read {
            s.a.i_a = 0; s.a.i_b = 0;
            s.a.s_a = c2_mulxv(ax, p_a.verts[0]);
            s.a.s_b = c2_mulxv(bx, p_b.verts[0]);
            s.a.p = c2_sub(s.a.s_b, s.a.s_a);
            s.a.u = 1.0; s.div = 1.0; s.count = 1;
        }

        let mut save_a = [0i32; 3];
        let mut save_b = [0i32; 3];
        let mut d0 = FLT_MAX;
        let mut d1: f32;
        let mut iter = 0i32;
        let mut hit = false;

        while iter < 20 {
            let save_count = s.count;
            for i in 0..save_count as usize {
                let v = &*verts_base.add(i);
                save_a[i] = v.i_a;
                save_b[i] = v.i_b;
            }
            match s.count {
                1 => {}
                2 => c22(&mut s),
                3 => c23(&mut s),
                _ => {}
            }
            if s.count == 3 { hit = true; break; }

            let p = c2_l(&s);
            d1 = c2_dot(p, p);
            if d1 > d0 { break; }
            d0 = d1;

            let d = c2_d(&s);
            if c2_dot(d, d) < FLT_EPSILON * FLT_EPSILON { break; }

            let ia = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
            let sa = c2_mulxv(ax, p_a.verts[ia as usize]);
            let ib = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
            let sb = c2_mulxv(bx, p_b.verts[ib as usize]);

            let v = &mut *verts_base.add(s.count as usize);
            v.i_a = ia; v.s_a = sa; v.i_b = ib; v.s_b = sb;
            v.p = c2_sub(v.s_b, v.s_a);

            let mut dup = false;
            for i in 0..save_count as usize {
                if ia == save_a[i] && ib == save_b[i] { dup = true; break; }
            }
            if dup { break; }
            s.count += 1;
            iter += 1;
        }

        let mut wa = c2v_new(0.0, 0.0);
        let mut wb = c2v_new(0.0, 0.0);
        c2_witness(&s, &mut wa, &mut wb);
        let mut dist = c2_len(c2_sub(wa, wb));

        if hit {
            wa = wb;
            dist = 0.0;
        } else if use_radius != 0 {
            let r_a = p_a.radius;
            let r_b = p_b.radius;
            if dist > r_a + r_b && dist > FLT_EPSILON {
                dist -= r_a + r_b;
                let n = c2_norm(c2_sub(wb, wa));
                wa = c2_add(wa, c2_mulvs(n, r_a));
                wb = c2_sub(wb, c2_mulvs(n, r_b));
                if wa.x == wb.x && wa.y == wb.y { dist = 0.0; }
            } else {
                let p = c2_mulvs(c2_add(wa, wb), 0.5);
                wa = p; wb = p; dist = 0.0;
            }
        }

        if !cache.is_null() {
            (*cache).metric = c2_gjk_simplex_metric(&s);
            (*cache).count = s.count;
            for i in 0..s.count as usize {
                let v = &*verts_base.add(i);
                (*cache).i_a[i] = v.i_a;
                (*cache).i_b[i] = v.i_b;
            }
            (*cache).div = s.div;
        }
        if !out_a.is_null() { *out_a = wa; }
        if !out_b.is_null() { *out_b = wb; }
        if !iterations.is_null() { *iterations = iter; }
        dist
    }
}

// --- Collision tests ---
fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2_aabb_to_capsule(a: &C2AABB, b: &C2Capsule) -> c_int {
    if c2_gjk(
        a as *const C2AABB as *const u8, C2_TYPE::C2_TYPE_AABB, std::ptr::null(),
        b as *const C2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, std::ptr::null(),
        std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut(),
    ) != 0.0 { return 0; }
    1
}

fn c2_capsule_to_capsule(a: &C2Capsule, b: &C2Capsule) -> c_int {
    if c2_gjk(
        a as *const C2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, std::ptr::null(),
        b as *const C2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, std::ptr::null(),
        std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut(),
    ) != 0.0 { return 0; }
    1
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> c_int {
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

fn c2_collided(a: *const u8, type_a: C2_TYPE, b: *const u8, type_b: C2_TYPE) -> c_int {
    unsafe {
        match type_a {
            C2_TYPE::C2_TYPE_CIRCLE => match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => c2_circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle)),
                C2_TYPE::C2_TYPE_AABB => c2_circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB)),
                C2_TYPE::C2_TYPE_CAPSULE => c2_circle_to_capsule(*(a as *const C2Circle), *(b as *const C2Capsule)),
            },
            C2_TYPE::C2_TYPE_AABB => match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => c2_circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB)),
                C2_TYPE::C2_TYPE_AABB => c2_aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB)),
                C2_TYPE::C2_TYPE_CAPSULE => c2_aabb_to_capsule(&*(a as *const C2AABB), &*(b as *const C2Capsule)),
            },
            C2_TYPE::C2_TYPE_CAPSULE => match type_b {
                C2_TYPE::C2_TYPE_CIRCLE => c2_circle_to_capsule(*(b as *const C2Circle), *(a as *const C2Capsule)),
                C2_TYPE::C2_TYPE_AABB => c2_aabb_to_capsule(&*(b as *const C2AABB), &*(a as *const C2Capsule)),
                C2_TYPE::C2_TYPE_CAPSULE => c2_capsule_to_capsule(&*(a as *const C2Capsule), &*(b as *const C2Capsule)),
            },
        }
    }
}

fn ptr_from_parts(typ: C2_TYPE, a: f32, b: f32, c: f32, d: f32, e: f32) -> *mut u8 {
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let shape = Box::new(C2Circle { p: c2v_new(a, b), r: c });
            Box::into_raw(shape) as *mut u8
        }
        C2_TYPE::C2_TYPE_AABB => {
            let shape = Box::new(C2AABB { min: c2v_new(a, b), max: c2v_new(c, d) });
            Box::into_raw(shape) as *mut u8
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let shape = Box::new(C2Capsule { a: c2v_new(a, b), b: c2v_new(c, d), r: e });
            Box::into_raw(shape) as *mut u8
        }
    }
}

// --- Public API ---
#[unsafe(no_mangle)]
pub extern "C" fn omni_collide(
    type_a: C2_TYPE, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
    type_b: C2_TYPE, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32,
) -> c_int {
    let a = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2_collided(a, type_a, b, type_b)
    // Note: intentionally leaks memory, matching C behavior
}
