use std::os::raw::c_int;

#[derive(Clone, Copy)]
#[repr(C)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct C2GJKCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

#[repr(C)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
enum C2Type {
    Circle,
    AABB,
    Capsule,
}

impl Default for C2v {
    fn default() -> Self { C2v { x: 0.0, y: 0.0 } }
}

fn c2v(x: f32, y: f32) -> C2v { C2v { x, y } }

fn c2mulvs(mut a: C2v, b: f32) -> C2v { a.x *= b; a.y *= b; a }

fn c2maxv(a: C2v, b: C2v) -> C2v {
    c2v(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

fn c2minv(a: C2v, b: C2v) -> C2v {
    c2v(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

fn c2clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2maxv(lo, c2minv(a, hi)) }

fn c2sub(mut a: C2v, b: C2v) -> C2v { a.x -= b.x; a.y -= b.y; a }

fn c2add(mut a: C2v, b: C2v) -> C2v { a.x += b.x; a.y += b.y; a }

fn c2dot(a: C2v, b: C2v) -> f32 { a.x * b.x + a.y * b.y }

fn c2neg(a: C2v) -> C2v { c2v(-a.x, -a.y) }

fn c2skew(a: C2v) -> C2v { C2v { x: -a.y, y: a.x } }

fn c2ccw90(a: C2v) -> C2v { C2v { x: a.y, y: -a.x } }

fn c2len(a: C2v) -> f32 { c2dot(a, a).sqrt() }

fn c2det2(a: C2v, b: C2v) -> f32 { a.x * b.y - a.y * b.x }

fn c2mulrv(a: C2r, b: C2v) -> C2v { c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y) }

fn c2mulrvt(a: C2r, b: C2v) -> C2v { c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y) }

fn c2mulxv(a: C2x, b: C2v) -> C2v { c2add(c2mulrv(a.r, b), a.p) }

fn c2div(a: C2v, b: f32) -> C2v { c2mulvs(a, 1.0 / b) }

fn c2norm(a: C2v) -> C2v { c2div(a, c2len(a)) }

fn c2x_identity() -> C2x {
    C2x { p: c2v(0.0, 0.0), r: C2r { c: 1.0, s: 0.0 } }
}

fn c2bb_verts(out: &mut [C2v; 8], bb: &C2AABB) {
    out[0] = bb.min;
    out[1] = c2v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v(bb.min.x, bb.max.y);
}

fn c2make_proxy(shape: *const u8, typ: C2Type, p: &mut C2Proxy) {
    unsafe {
        match typ {
            C2Type::Circle => {
                let c = &*(shape as *const C2Circle);
                p.radius = c.r;
                p.count = 1;
                p.verts[0] = c.p;
            }
            C2Type::AABB => {
                let bb = &*(shape as *const C2AABB);
                p.radius = 0.0;
                p.count = 4;
                c2bb_verts(&mut p.verts, bb);
            }
            C2Type::Capsule => {
                let c = &*(shape as *const C2Capsule);
                p.radius = c.r;
                p.count = 2;
                p.verts[0] = c.a;
                p.verts[1] = c.b;
            }
        }
    }
}

fn c2gjk_simplex_metric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2len(c2sub(s.b.p, s.a.p)),
        3 => c2det2(c2sub(s.b.p, s.a.p), c2sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c22_inner(s: &mut C2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = c2dot(b, c2sub(b, a));
    let v = c2dot(a, c2sub(a, b));
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

fn c23_inner(s: &mut C2Simplex) {
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

fn c2support(verts: &[C2v; 8], count: c_int, d: C2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2dot(verts[0], d);
    for i in 1..count {
        let dot = c2dot(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

fn c2witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0f32 / s.div;
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
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2add(c2mulvs(s.a.p, den * s.a.u), c2mulvs(s.b.p, den * s.b.u)),
        _ => c2v(0.0, 0.0),
    }
}

fn vert_mut(s: &mut C2Simplex, i: usize) -> &mut C2sv {
    match i {
        0 => &mut s.a,
        1 => &mut s.b,
        2 => &mut s.c,
        3 => &mut s.d,
        _ => unreachable!(),
    }
}

fn vert_ref(s: &C2Simplex, i: usize) -> &C2sv {
    match i {
        0 => &s.a,
        1 => &s.b,
        2 => &s.c,
        3 => &s.d,
        _ => unreachable!(),
    }
}

const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

#[allow(clippy::too_many_arguments)]
fn c2gjk(
    a_shape: *const u8, type_a: C2Type, ax_ptr: *const C2x,
    b_shape: *const u8, type_b: C2Type, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: c_int, iterations: *mut c_int, cache: *mut C2GJKCache,
) -> f32 {
    unsafe {
        let ax = if ax_ptr.is_null() { c2x_identity() } else { *ax_ptr };
        let bx = if bx_ptr.is_null() { c2x_identity() } else { *bx_ptr };

        let mut p_a = C2Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        let mut p_b = C2Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        c2make_proxy(a_shape, type_a, &mut p_a);
        c2make_proxy(b_shape, type_b, &mut p_b);

        let mut s = C2Simplex {
            a: C2sv::default(), b: C2sv::default(), c: C2sv::default(), d: C2sv::default(),
            div: 0.0, count: 0,
        };

        let mut cache_was_read = false;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                for i in 0..(*cache).count as usize {
                    let i_a = (*cache).i_a[i];
                    let i_b = (*cache).i_b[i];
                    let s_a = c2mulxv(ax, p_a.verts[i_a as usize]);
                    let s_b = c2mulxv(bx, p_b.verts[i_b as usize]);
                    let v = vert_mut(&mut s, i);
                    v.i_a = i_a;
                    v.s_a = s_a;
                    v.i_b = i_b;
                    v.s_b = s_b;
                    v.p = c2sub(v.s_b, v.s_a);
                    v.u = 0.0;
                }
                s.count = (*cache).count;
                s.div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2gjk_simplex_metric(&s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                    cache_was_read = true;
                }
            }
        }

        if !cache_was_read {
            s.a.i_a = 0;
            s.a.i_b = 0;
            s.a.s_a = c2mulxv(ax, p_a.verts[0]);
            s.a.s_b = c2mulxv(bx, p_b.verts[0]);
            s.a.p = c2sub(s.a.s_b, s.a.s_a);
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        let mut save_a = [0i32; 3];
        let mut save_b = [0i32; 3];
        let mut d0 = FLT_MAX;
        let mut d1: f32;
        let mut iter = 0;
        let mut hit = false;

        while iter < 20 {
            let save_count = s.count;
            for i in 0..save_count as usize {
                save_a[i] = vert_ref(&s, i).i_a;
                save_b[i] = vert_ref(&s, i).i_b;
            }

            match s.count {
                1 => {}
                2 => c22_inner(&mut s),
                3 => c23_inner(&mut s),
                _ => {}
            }

            if s.count == 3 {
                hit = true;
                break;
            }

            let p = c2l(&s);
            d1 = c2dot(p, p);
            if d1 > d0 { break; }
            d0 = d1;

            let d = c2d(&s);
            if c2dot(d, d) < FLT_EPSILON * FLT_EPSILON { break; }

            let i_a = c2support(&p_a.verts, p_a.count, c2mulrvt(ax.r, c2neg(d)));
            let s_a = c2mulxv(ax, p_a.verts[i_a as usize]);
            let i_b = c2support(&p_b.verts, p_b.count, c2mulrvt(bx.r, d));
            let s_b = c2mulxv(bx, p_b.verts[i_b as usize]);

            {
                let idx = s.count as usize;
                let v = vert_mut(&mut s, idx);
                v.i_a = i_a;
                v.s_a = s_a;
                v.i_b = i_b;
                v.s_b = s_b;
                v.p = c2sub(v.s_b, v.s_a);
            }

            let mut dup = false;
            for i in 0..save_count as usize {
                if i_a == save_a[i] && i_b == save_b[i] {
                    dup = true;
                    break;
                }
            }
            if dup { break; }

            s.count += 1;
            iter += 1;
        }

        let mut a_out = c2v(0.0, 0.0);
        let mut b_out = c2v(0.0, 0.0);
        c2witness(&s, &mut a_out, &mut b_out);
        let mut dist = c2len(c2sub(a_out, b_out));

        if hit {
            a_out = b_out;
            dist = 0.0;
        } else if use_radius != 0 {
            let r_a = p_a.radius;
            let r_b = p_b.radius;
            if dist > r_a + r_b && dist > FLT_EPSILON {
                dist -= r_a + r_b;
                let n = c2norm(c2sub(b_out, a_out));
                a_out = c2add(a_out, c2mulvs(n, r_a));
                b_out = c2sub(b_out, c2mulvs(n, r_b));
                if a_out.x == b_out.x && a_out.y == b_out.y {
                    dist = 0.0;
                }
            } else {
                let p = c2mulvs(c2add(a_out, b_out), 0.5);
                a_out = p;
                b_out = p;
                dist = 0.0;
            }
        }

        if !cache.is_null() {
            (*cache).metric = c2gjk_simplex_metric(&s);
            (*cache).count = s.count;
            for i in 0..s.count as usize {
                (*cache).i_a[i] = vert_ref(&s, i).i_a;
                (*cache).i_b[i] = vert_ref(&s, i).i_b;
            }
            (*cache).div = s.div;
        }

        if !out_a.is_null() { *out_a = a_out; }
        if !out_b.is_null() { *out_b = b_out; }
        if !iterations.is_null() { *iterations = iter; }

        dist
    }
}

fn c2aabb_to_aabb(a: C2AABB, b: C2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2aabb_to_capsule(a: C2AABB, b: C2Capsule) -> c_int {
    if c2gjk(
        &a as *const _ as *const u8, C2Type::AABB, std::ptr::null(),
        &b as *const _ as *const u8, C2Type::Capsule, std::ptr::null(),
        std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut(),
    ) != 0.0 { 0 } else { 1 }
}

fn c2capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> c_int {
    if c2gjk(
        &a as *const _ as *const u8, C2Type::Capsule, std::ptr::null(),
        &b as *const _ as *const u8, C2Type::Capsule, std::ptr::null(),
        std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut(),
    ) != 0.0 { 0 } else { 1 }
}

fn c2circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2sub(b.p, a.p);
    let d2 = c2dot(c, c);
    let r2 = a.r + b.r;
    let r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2circle_to_aabb(a: C2Circle, b: C2AABB) -> c_int {
    let l = c2clampv(a.p, b.min, b.max);
    let ab = c2sub(a.p, l);
    let d2 = c2dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2circle_to_capsule(a: C2Circle, b: C2Capsule) -> c_int {
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
    (d2 < r * r) as c_int
}

fn c2collided(a: *const u8, type_a: C2Type, b: *const u8, type_b: C2Type) -> c_int {
    unsafe {
        match type_a {
            C2Type::Circle => match type_b {
                C2Type::Circle => c2circle_to_circle(*(a as *const C2Circle), *(b as *const C2Circle)),
                C2Type::AABB => c2circle_to_aabb(*(a as *const C2Circle), *(b as *const C2AABB)),
                C2Type::Capsule => c2circle_to_capsule(*(a as *const C2Circle), *(b as *const C2Capsule)),
            },
            C2Type::AABB => match type_b {
                C2Type::Circle => c2circle_to_aabb(*(b as *const C2Circle), *(a as *const C2AABB)),
                C2Type::AABB => c2aabb_to_aabb(*(a as *const C2AABB), *(b as *const C2AABB)),
                C2Type::Capsule => c2aabb_to_capsule(*(a as *const C2AABB), *(b as *const C2Capsule)),
            },
            C2Type::Capsule => match type_b {
                C2Type::Circle => c2circle_to_capsule(*(b as *const C2Circle), *(a as *const C2Capsule)),
                C2Type::AABB => c2aabb_to_capsule(*(b as *const C2AABB), *(a as *const C2Capsule)),
                C2Type::Capsule => c2capsule_to_capsule(*(a as *const C2Capsule), *(b as *const C2Capsule)),
            },
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let circle_in = C2Circle { p: c2v(x, y), r };

    let circle = C2Circle { p: c2v(-70.0, 0.0), r: 20.0 };

    let aabb = C2AABB { min: c2v(-40.0, -40.0), max: c2v(-15.0, -15.0) };

    let capsule = C2Capsule { a: c2v(-40.0, 40.0), b: c2v(-20.0, 100.0), r: 10.0 };

    result += c2collided(
        &circle as *const _ as *const u8, C2Type::Circle,
        &circle_in as *const _ as *const u8, C2Type::Circle,
    );

    result += c2collided(
        &aabb as *const _ as *const u8, C2Type::AABB,
        &circle_in as *const _ as *const u8, C2Type::Circle,
    ) << 1;

    result += c2collided(
        &capsule as *const _ as *const u8, C2Type::Capsule,
        &circle_in as *const _ as *const u8, C2Type::Circle,
    ) << 2;

    result
}

// ---- FFI export wrappers matching C symbol names ----

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
pub extern "C" fn c2Neg(a: C2v) -> C2v { c2neg(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v { c2skew(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v { c2ccw90(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 { c2len(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 { c2det2(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v { c2mulrv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v { c2mulrvt(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v { c2mulxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v { c2div(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v { c2norm(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r { C2r { c: 1.0, s: 0.0 } }

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x { c2x_identity() }

#[unsafe(no_mangle)]
pub extern "C" fn c2BBVerts(out: *mut C2v, bb: *const C2AABB) {
    unsafe {
        let out_slice = &mut *(out as *mut [C2v; 8]);
        c2bb_verts(out_slice, &*bb);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MakeProxy(shape: *const u8, typ: C2Type, p: *mut C2Proxy) {
    unsafe { c2make_proxy(shape, typ, &mut *p); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2GJKSimplexMetric(s: *const C2Simplex) -> f32 {
    unsafe { c2gjk_simplex_metric(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c22(s: *mut C2Simplex) {
    unsafe { c22_inner(&mut *s); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c23(s: *mut C2Simplex) {
    unsafe { c23_inner(&mut *s); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2D(s: *const C2Simplex) -> C2v {
    unsafe { c2d(&*s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
    unsafe { c2support(&*(verts as *const [C2v; 8]), count, d) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Witness(s: *const C2Simplex, a: *mut C2v, b: *mut C2v) {
    unsafe { c2witness(&*s, &mut *a, &mut *b); }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2L(s: *const C2Simplex) -> C2v {
    unsafe { c2l(&*s) }
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn c2GJK(
    a: *const u8, type_a: C2Type, ax_ptr: *const C2x,
    b: *const u8, type_b: C2Type, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: c_int, iterations: *mut c_int, cache: *mut C2GJKCache,
) -> f32 {
    c2gjk(a, type_a, ax_ptr, b, type_b, bx_ptr, out_a, out_b, use_radius, iterations, cache)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int { c2aabb_to_aabb(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(a: C2AABB, b: C2Capsule) -> c_int { c2aabb_to_capsule(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(a: C2Capsule, b: C2Capsule) -> c_int { c2capsule_to_capsule(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int { c2circle_to_circle(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2AABB) -> c_int { c2circle_to_aabb(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: C2Circle, b: C2Capsule) -> c_int { c2circle_to_capsule(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Collided(a: *const u8, type_a: C2Type, b: *const u8, type_b: C2Type) -> c_int {
    c2collided(a, type_a, b, type_b)
}
