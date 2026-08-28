use std::ffi::{c_char, c_int, c_void};

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;
const FLT_EPSILON: f32 = 1.192_092_9e-7;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2GjkCache {
    pub metric: f32,
    pub count: c_int,
    pub i_a: [c_int; 3],
    pub i_b: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2sv {
    pub s_a: C2v,
    pub s_b: C2v,
    pub p: C2v,
    pub u: f32,
    pub i_a: c_int,
    pub i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Simplex {
    pub a: C2sv,
    pub b: C2sv,
    pub c: C2sv,
    pub d: C2sv,
    pub div: f32,
    pub count: c_int,
}

const _: () = {
    assert!(std::mem::size_of::<C2v>() == 8);
    assert!(std::mem::size_of::<C2r>() == 8);
    assert!(std::mem::size_of::<C2x>() == 16);
    assert!(std::mem::size_of::<C2Circle>() == 12);
    assert!(std::mem::size_of::<C2Aabb>() == 16);
    assert!(std::mem::size_of::<C2Capsule>() == 20);
    assert!(std::mem::size_of::<C2GjkCache>() == 36);
    assert!(std::mem::size_of::<C2Proxy>() == 72);
    assert!(std::mem::size_of::<C2sv>() == 36);
    assert!(std::mem::size_of::<C2Simplex>() == 152);
};

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x {
    C2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut C2v, bb: *mut C2Aabb) {
    unsafe {
        *out.add(0) = (*bb).min;
        *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
        *out.add(2) = (*bb).max;
        *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, shape_type: c_int, p: *mut C2Proxy) {
    unsafe {
        match shape_type {
            C2_TYPE_CIRCLE => {
                let circle = shape.cast::<C2Circle>();
                (*p).radius = (*circle).r;
                (*p).count = 1;
                (*p).verts[0] = (*circle).p;
            }
            C2_TYPE_AABB => {
                let bb = shape.cast::<C2Aabb>() as *mut C2Aabb;
                (*p).radius = 0.0;
                (*p).count = 4;
                c2BBVerts((*p).verts.as_mut_ptr(), bb);
            }
            C2_TYPE_CAPSULE => {
                let capsule = shape.cast::<C2Capsule>();
                (*p).radius = (*capsule).r;
                (*p).count = 2;
                (*p).verts[0] = (*capsule).a;
                (*p).verts[1] = (*capsule).b;
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut C2Simplex) -> f32 {
    unsafe {
        match (*s).count {
            2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
            3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut C2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else {
            (*s).a.u = u;
            (*s).b.u = v;
            (*s).div = u + v;
            (*s).count = 2;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut C2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let c = (*s).c.p;
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
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u_ab <= 0.0 && v_bc <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u_bc <= 0.0 && v_ca <= 0.0 {
            (*s).a = (*s).c;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
            (*s).a.u = u_ab;
            (*s).b.u = v_ab;
            (*s).div = u_ab + v_ab;
            (*s).count = 2;
        } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
            (*s).a = (*s).b;
            (*s).b = (*s).c;
            (*s).a.u = u_bc;
            (*s).b.u = v_bc;
            (*s).div = u_bc + v_bc;
            (*s).count = 2;
        } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
            (*s).b = (*s).a;
            (*s).a = (*s).c;
            (*s).a.u = u_ca;
            (*s).b.u = v_ca;
            (*s).div = u_ca + v_ca;
            (*s).count = 2;
        } else {
            (*s).a.u = u_abc;
            (*s).b.u = v_abc;
            (*s).c.u = w_abc;
            (*s).div = u_abc + v_abc + w_abc;
            (*s).count = 3;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: C2v) -> C2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut C2Simplex) -> C2v {
    unsafe {
        match (*s).count {
            1 => c2Neg((*s).a.p),
            2 => {
                let ab = c2Sub((*s).b.p, (*s).a.p);
                if c2Det2(ab, c2Neg((*s).a.p)) > 0.0 {
                    c2Skew(ab)
                } else {
                    c2CCW90(ab)
                }
            }
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
    unsafe {
        let mut imax = 0;
        let mut dmax = c2Dot(*verts, d);
        let mut i = 1;
        while i < count {
            let dot = c2Dot(*verts.add(i as usize), d);
            if dot > dmax {
                imax = i;
                dmax = dot;
            }
            i += 1;
        }
        imax
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut C2Simplex, a: *mut C2v, b: *mut C2v) {
    unsafe {
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => {
                *a = (*s).a.s_a;
                *b = (*s).a.s_b;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs((*s).a.s_a, den * (*s).a.u),
                    c2Mulvs((*s).b.s_a, den * (*s).b.u),
                );
                *b = c2Add(
                    c2Mulvs((*s).a.s_b, den * (*s).a.u),
                    c2Mulvs((*s).b.s_b, den * (*s).b.u),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.s_a, den * (*s).a.u),
                        c2Mulvs((*s).b.s_a, den * (*s).b.u),
                    ),
                    c2Mulvs((*s).c.s_a, den * (*s).c.u),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.s_b, den * (*s).a.u),
                        c2Mulvs((*s).b.s_b, den * (*s).b.u),
                    ),
                    c2Mulvs((*s).c.s_b, den * (*s).c.u),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut C2Simplex) -> C2v {
    unsafe {
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => (*s).a.p,
            2 => c2Add(
                c2Mulvs((*s).a.p, den * (*s).a.u),
                c2Mulvs((*s).b.p, den * (*s).b.u),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    a_shape: *const c_void,
    type_a: c_int,
    ax_ptr: *const C2x,
    b_shape: *const c_void,
    type_b: c_int,
    bx_ptr: *const C2x,
    out_a: *mut C2v,
    out_b: *mut C2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut C2GjkCache,
) -> f32 {
    unsafe {
        let ax = if ax_ptr.is_null() {
            c2xIdentity()
        } else {
            *ax_ptr
        };
        let bx = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            *bx_ptr
        };

        let mut p_a = C2Proxy::default();
        let mut p_b = C2Proxy::default();
        c2MakeProxy(a_shape, type_a, &mut p_a);
        c2MakeProxy(b_shape, type_b, &mut p_b);

        let mut s = C2Simplex::default();
        let verts = (&mut s.a as *mut C2sv).cast::<C2sv>();
        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = ((*cache).count != 0) as c_int;
            if cache_was_good != 0 {
                let mut i = 0;
                while i < (*cache).count {
                    let i_a = (*cache).i_a[i as usize];
                    let i_b = (*cache).i_b[i as usize];
                    let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
                    let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
                    let v = verts.add(i as usize);
                    (*v).i_a = i_a;
                    (*v).s_a = s_a;
                    (*v).i_b = i_b;
                    (*v).s_b = s_b;
                    (*v).p = c2Sub((*v).s_b, (*v).s_a);
                    (*v).u = 0.0;
                    i += 1;
                }
                s.count = (*cache).count;
                s.div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2GJKSimplexMetric(&mut s);
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
                    cache_was_read = 1;
                }
            }
        }

        if cache_was_read == 0 {
            s.a.i_a = 0;
            s.a.i_b = 0;
            s.a.s_a = c2Mulxv(ax, p_a.verts[0]);
            s.a.s_b = c2Mulxv(bx, p_b.verts[0]);
            s.a.p = c2Sub(s.a.s_b, s.a.s_a);
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
                save_a[i as usize] = (*verts.add(i as usize)).i_a;
                save_b[i as usize] = (*verts.add(i as usize)).i_b;
                i += 1;
            }

            match s.count {
                2 => c22(&mut s),
                3 => c23(&mut s),
                _ => {}
            }
            if s.count == 3 {
                hit = 1;
                break;
            }

            let p = c2L(&mut s);
            let d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;
            let d = c2D(&mut s);
            if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
                break;
            }

            let i_a = c2Support(
                p_a.verts.as_ptr(),
                p_a.count,
                c2MulrvT(ax.r, c2Neg(d)),
            );
            let s_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
            let i_b = c2Support(p_b.verts.as_ptr(), p_b.count, c2MulrvT(bx.r, d));
            let s_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
            let v = verts.add(s.count as usize);
            (*v).i_a = i_a;
            (*v).s_a = s_a;
            (*v).i_b = i_b;
            (*v).s_b = s_b;
            (*v).p = c2Sub((*v).s_b, (*v).s_a);

            let mut dup = 0;
            let mut i = 0;
            while i < save_count {
                if i_a == save_a[i as usize] && i_b == save_b[i as usize] {
                    dup = 1;
                    break;
                }
                i += 1;
            }
            if dup != 0 {
                break;
            }
            s.count += 1;
            iter += 1;
        }

        let mut a = C2v::default();
        let mut b = C2v::default();
        c2Witness(&mut s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));
        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let r_a = p_a.radius;
            let r_b = p_b.radius;
            if dist > r_a + r_b && dist > FLT_EPSILON {
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

        if !cache.is_null() {
            (*cache).metric = c2GJKSimplexMetric(&mut s);
            (*cache).count = s.count;
            let mut i = 0;
            while i < s.count {
                let v = verts.add(i as usize);
                (*cache).i_a[i as usize] = (*v).i_a;
                (*cache).i_b[i as usize] = (*v).i_b;
                i += 1;
            }
            (*cache).div = s.div;
        }
        if !out_a.is_null() {
            *out_a = a;
        }
        if !out_b.is_null() {
            *out_b = b;
        }
        if !iterations.is_null() {
            *iterations = iter;
        }
        dist
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk(
    reverse: c_char,
    a: *mut C2v,
    b: *mut C2v,
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
    unsafe {
        let bb = C2Aabb {
            min: c2V(a1, a2),
            max: c2V(a3, a4),
        };
        let capsule = C2Capsule {
            a: c2V(b1, b2),
            b: c2V(b3, b4),
            r: b5,
        };
        if reverse != 0 {
            c2GJK(
                (&capsule as *const C2Capsule).cast(),
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                (&bb as *const C2Aabb).cast(),
                C2_TYPE_AABB,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        } else {
            c2GJK(
                (&bb as *const C2Aabb).cast(),
                C2_TYPE_AABB,
                std::ptr::null(),
                (&capsule as *const C2Capsule).cast(),
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}
