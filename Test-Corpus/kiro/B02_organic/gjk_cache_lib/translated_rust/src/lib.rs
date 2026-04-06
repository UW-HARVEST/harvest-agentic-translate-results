use std::os::raw::c_char;

#[repr(C)]
#[derive(Clone, Copy)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2GJKCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Proxy {
    radius: f32,
    count: i32,
    verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: i32,
    i_b: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: i32,
}

#[repr(i32)]
#[derive(Clone, Copy)]
enum C2Type {
    Circle = 0,
    AABB = 1,
    Capsule = 2,
}

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_mulvs(a: C2v, b: f32) -> C2v {
    C2v { x: a.x * b, y: a.y * b }
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

fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v { x: a.x - b.x, y: a.y - b.y }
}

fn c2_add(a: C2v, b: C2v) -> C2v {
    C2v { x: a.x + b.x, y: a.y + b.y }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

fn c2x_identity() -> C2x {
    C2x { p: c2v(0.0, 0.0), r: c2_rot_identity() }
}

fn c2_bb_verts(out: &mut [C2v], bb: &C2AABB) {
    out[0] = bb.min;
    out[1] = c2v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2v(bb.min.x, bb.max.y);
}

fn c2_make_proxy(shape: *const u8, typ: C2Type, p: &mut C2Proxy) {
    match typ {
        C2Type::Circle => unsafe {
            let c = &*(shape as *const C2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        },
        C2Type::AABB => unsafe {
            let bb = &*(shape as *const C2AABB);
            p.radius = 0.0;
            p.count = 4;
            c2_bb_verts(&mut p.verts, bb);
        },
        C2Type::Capsule => unsafe {
            let c = &*(shape as *const C2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        },
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
        2 => c2_len(c2_sub(s.b.p, s.a.p)),
        3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
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
    let mut imax = 0;
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

fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
    let den = 1.0f32 / s.div;
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
            *a = c2v(0.0, 0.0);
            *b = c2v(0.0, 0.0);
        }
    }
}

fn c2_norm(a: C2v) -> C2v {
    c2_mulvs(a, 1.0 / c2_len(a))
}

fn c2_l(s: &C2Simplex) -> C2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2_add(c2_mulvs(s.a.p, den * s.a.u), c2_mulvs(s.b.p, den * s.b.u)),
        _ => c2v(0.0, 0.0),
    }
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_gjk(
    a_shape: *const u8,
    type_a: C2Type,
    ax_ptr: *const C2x,
    b_shape: *const u8,
    type_b: C2Type,
    bx_ptr: *const C2x,
    out_a: *mut C2v,
    out_b: *mut C2v,
    use_radius: i32,
    iterations: *mut i32,
    cache: *mut C2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() { c2x_identity() } else { unsafe { *ax_ptr } };
    let bx = if bx_ptr.is_null() { c2x_identity() } else { unsafe { *bx_ptr } };

    let mut p_a = C2Proxy { radius: 0.0, count: 0, verts: [c2v(0.0, 0.0); 8] };
    let mut p_b = C2Proxy { radius: 0.0, count: 0, verts: [c2v(0.0, 0.0); 8] };
    c2_make_proxy(a_shape, type_a, &mut p_a);
    c2_make_proxy(b_shape, type_b, &mut p_b);

    let mut s = C2Simplex {
        a: C2sv { s_a: c2v(0.0,0.0), s_b: c2v(0.0,0.0), p: c2v(0.0,0.0), u: 0.0, i_a: 0, i_b: 0 },
        b: C2sv { s_a: c2v(0.0,0.0), s_b: c2v(0.0,0.0), p: c2v(0.0,0.0), u: 0.0, i_a: 0, i_b: 0 },
        c: C2sv { s_a: c2v(0.0,0.0), s_b: c2v(0.0,0.0), p: c2v(0.0,0.0), u: 0.0, i_a: 0, i_b: 0 },
        d: C2sv { s_a: c2v(0.0,0.0), s_b: c2v(0.0,0.0), p: c2v(0.0,0.0), u: 0.0, i_a: 0, i_b: 0 },
        div: 0.0,
        count: 0,
    };

    // Access verts as array: verts[0]=s.a, verts[1]=s.b, verts[2]=s.c, verts[3]=s.d
    // In C, `c2sv *verts = &s.a;` and then `verts[i]` is used.
    // We'll use a helper to index into the simplex.

    let mut cache_was_read = false;
    if !cache.is_null() {
        let ca = unsafe { &*cache };
        let cache_was_good = ca.count != 0;
        if cache_was_good {
            for i in 0..ca.count {
                let i_a = ca.i_a[i as usize];
                let i_b = ca.i_b[i as usize];
                let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
                let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);
                let v = simplex_vert_mut(&mut s, i);
                v.i_a = i_a;
                v.s_a = s_a;
                v.i_b = i_b;
                v.s_b = s_b;
                v.p = c2_sub(v.s_b, v.s_a);
                v.u = 0.0;
            }
            s.count = ca.count;
            s.div = ca.div;
            let metric_old = ca.metric;
            let metric = c2_gjk_simplex_metric(&s);
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
        s.a.s_a = c2_mulxv(ax, p_a.verts[0]);
        s.a.s_b = c2_mulxv(bx, p_b.verts[0]);
        s.a.p = c2_sub(s.a.s_b, s.a.s_a);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut save_a = [0i32; 3];
    let mut save_b = [0i32; 3];
    let mut d0: f32 = f32::MAX;
    let mut d1: f32 = f32::MAX;
    let mut iter = 0i32;
    let mut hit = false;

    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count {
            let v = simplex_vert(&s, i);
            save_a[i as usize] = v.i_a;
            save_b[i as usize] = v.i_b;
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
        if c2_dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }

        let i_a = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
        let s_a = c2_mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
        let s_b = c2_mulxv(bx, p_b.verts[i_b as usize]);

        let idx = s.count;
        let v = simplex_vert_mut(&mut s, idx);
        v.i_a = i_a;
        v.s_a = s_a;
        v.i_b = i_b;
        v.s_b = s_b;
        v.p = c2_sub(v.s_b, v.s_a);

        let mut dup = false;
        for i in 0..save_count {
            if i_a == save_a[i as usize] && i_b == save_b[i as usize] {
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

    let mut a = c2v(0.0, 0.0);
    let mut b = c2v(0.0, 0.0);
    c2_witness(&s, &mut a, &mut b);
    let mut dist = c2_len(c2_sub(a, b));

    if hit {
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

    if !cache.is_null() {
        let ca = unsafe { &mut *cache };
        ca.metric = c2_gjk_simplex_metric(&s);
        ca.count = s.count;
        for i in 0..s.count {
            let v = simplex_vert(&s, i);
            ca.i_a[i as usize] = v.i_a;
            ca.i_b[i as usize] = v.i_b;
        }
        ca.div = s.div;
    }

    if !out_a.is_null() {
        unsafe { *out_a = a; }
    }
    if !out_b.is_null() {
        unsafe { *out_b = b; }
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter; }
    }

    dist
}

// Helper to index into simplex verts like C's `verts[i]` where verts = &s.a
fn simplex_vert(s: &C2Simplex, i: i32) -> &C2sv {
    match i {
        0 => &s.a,
        1 => &s.b,
        2 => &s.c,
        3 => &s.d,
        _ => &s.a,
    }
}

fn simplex_vert_mut(s: &mut C2Simplex, i: i32) -> &mut C2sv {
    match i {
        0 => &mut s.a,
        1 => &mut s.b,
        2 => &mut s.c,
        3 => &mut s.d,
        _ => &mut s.a,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gjk_cache(
    reverse: c_char,
    a9: *mut C2v,
    b9: *mut C2v,
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
    let mut cache = C2GJKCache {
        metric: 0.0,
        count: 0,
        i_a: [0; 3],
        i_b: [0; 3],
        div: 0.0,
    };

    let a_shape = C2Circle { p: c2v(0.0, 0.0), r: 15.0 };
    let b_shape = C2Capsule { a: c2v(100.0, -25.0), b: c2v(75.0, 100.0), r: 10.0 };

    let mut a0 = c2v(0.0, 0.0);
    let mut b0 = c2v(0.0, 0.0);
    let mut a = c2v(0.0, 0.0);
    let mut b = c2v(0.0, 0.0);

    let mut iterations: i32 = -1;
    let mut cached_iterations: i32 = -1;

    let _d0 = c2_gjk(
        &a_shape as *const C2Circle as *const u8, C2Type::Circle, std::ptr::null(),
        &b_shape as *const C2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
        &mut a0, &mut b0, 1, &mut iterations, &mut cache,
    );
    let _d1 = c2_gjk(
        &a_shape as *const C2Circle as *const u8, C2Type::Circle, std::ptr::null(),
        &b_shape as *const C2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
        &mut a, &mut b, 1, &mut cached_iterations, &mut cache,
    );

    let mut bb = C2AABB { min: c2v(a1, a2), max: c2v(a3, a4) };
    let mut cap = C2Capsule { a: c2v(b1, b2), b: c2v(b3, b4), r: b5 };

    if reverse != 0 {
        c2_gjk(
            &cap as *const C2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
            &bb as *const C2AABB as *const u8, C2Type::AABB, std::ptr::null(),
            &mut a, &mut b, 1, std::ptr::null_mut(), std::ptr::null_mut(),
        );
    } else {
        c2_gjk(
            &bb as *const C2AABB as *const u8, C2Type::AABB, std::ptr::null(),
            &cap as *const C2Capsule as *const u8, C2Type::Capsule, std::ptr::null(),
            &mut a, &mut b, 1, std::ptr::null_mut(), std::ptr::null_mut(),
        );
    }
}

// Exported wrappers matching C symbol names exactly
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v { c2v(x, y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: C2v, b: f32) -> C2v { c2_mulvs(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v { c2_maxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v { c2_minv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: C2v, b: C2v) -> C2v { c2_sub(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: C2v, b: C2v) -> C2v { c2_add(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 { c2_dot(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r { c2_rot_identity() }

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x { c2x_identity() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut C2v, bb: *const C2AABB) {
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    let bb_ref = unsafe { &*bb };
    c2_bb_verts(out_slice, bb_ref);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const u8, typ: C2Type, p: *mut C2Proxy) {
    c2_make_proxy(shape, typ, unsafe { &mut *p });
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 { c2_len(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 { c2_det2(a, b) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *const C2Simplex) -> f32 {
    c2_gjk_simplex_metric(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v { c2_mulrv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v { c2_mulxv(a, b) }

#[export_name = "c22"]
pub unsafe extern "C" fn c22_export(s: *mut C2Simplex) { c22(unsafe { &mut *s }) }

#[export_name = "c23"]
pub unsafe extern "C" fn c23_export(s: *mut C2Simplex) { c23(unsafe { &mut *s }) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: C2v) -> C2v { c2_neg(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v { c2_skew(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v { c2_ccw90(a) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *const C2Simplex) -> C2v { c2_d(unsafe { &*s }) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const C2v, count: i32, d: C2v) -> i32 {
    let slice = unsafe { std::slice::from_raw_parts(verts, count as usize) };
    c2_support(slice, count, d)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *const C2Simplex, a: *mut C2v, b: *mut C2v) {
    c2_witness(unsafe { &*s }, unsafe { &mut *a }, unsafe { &mut *b });
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v { c2_mulvs(a, 1.0 / b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v { c2_norm(a) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *const C2Simplex) -> C2v { c2_l(unsafe { &*s }) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v { c2_mulrv_t(a, b) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    a: *const u8, type_a: C2Type, ax_ptr: *const C2x,
    b: *const u8, type_b: C2Type, bx_ptr: *const C2x,
    out_a: *mut C2v, out_b: *mut C2v,
    use_radius: i32, iterations: *mut i32, cache: *mut C2GJKCache,
) -> f32 {
    c2_gjk(a, type_a, ax_ptr, b, type_b, bx_ptr, out_a, out_b, use_radius, iterations, cache)
}
