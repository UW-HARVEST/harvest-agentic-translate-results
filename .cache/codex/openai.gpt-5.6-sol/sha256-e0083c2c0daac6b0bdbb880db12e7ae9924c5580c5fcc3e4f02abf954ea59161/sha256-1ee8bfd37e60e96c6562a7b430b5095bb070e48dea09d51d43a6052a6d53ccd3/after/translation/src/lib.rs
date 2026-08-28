use std::ffi::{c_float, c_int, c_void};

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn add_left(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "addss xmm0, xmm1",
            inout("xmm0") result,
            in("xmm1") rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn add_left(lhs: c_float, rhs: c_float) -> c_float {
    lhs + rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn sub_left(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "subss xmm0, xmm1",
            inout("xmm0") result,
            in("xmm1") rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn sub_left(lhs: c_float, rhs: c_float) -> c_float {
    lhs - rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn mul_left(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "mulss xmm0, xmm1",
            inout("xmm0") result,
            in("xmm1") rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn mul_left(lhs: c_float, rhs: c_float) -> c_float {
    lhs * rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn div_left(lhs: c_float, rhs: c_float) -> c_float {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "divss xmm0, xmm1",
            inout("xmm0") result,
            in("xmm1") rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn div_left(lhs: c_float, rhs: c_float) -> c_float {
    lhs / rhs
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2r {
    pub c: c_float,
    pub s: c_float,
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
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2GJKCache {
    pub metric: c_float,
    pub count: c_int,
    pub i_a: [c_int; 3],
    pub i_b: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2sv {
    pub s_a: C2v,
    pub s_b: C2v,
    pub p: C2v,
    pub u: c_float,
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
    pub div: c_float,
    pub count: c_int,
}

impl C2Simplex {
    fn vertex(&self, index: usize) -> &C2sv {
        match index {
            0 => &self.a,
            1 => &self.b,
            2 => &self.c,
            3 => &self.d,
            _ => unreachable!(),
        }
    }

    fn vertex_mut(&mut self, index: usize) -> &mut C2sv {
        match index {
            0 => &mut self.a,
            1 => &mut self.b,
            2 => &mut self.c,
            3 => &mut self.d,
            _ => unreachable!(),
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: c_float) -> C2v {
    a.x = mul_left(a.x, b);
    a.y = mul_left(a.y, b);
    a
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x = sub_left(a.x, b.x);
    a.y = sub_left(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> c_float {
    add_left(mul_left(b.y, a.y), mul_left(a.x, b.x))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2RotIdentity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2xIdentity() -> C2x {
    C2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2BBVerts(out: *mut C2v, bb: *mut C2AABB) {
    let bb = unsafe { &*bb };
    unsafe {
        *out.add(0) = bb.min;
        *out.add(1) = c2V(bb.max.x, bb.min.y);
        *out.add(2) = bb.max;
        *out.add(3) = c2V(bb.min.x, bb.max.y);
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, shape_type: c_int, p: *mut C2Proxy) {
    match shape_type {
        C2_TYPE_CIRCLE => {
            let p = unsafe { &mut *p };
            let circle = unsafe { &*shape.cast::<C2Circle>() };
            p.radius = circle.r;
            p.count = 1;
            p.verts[0] = circle.p;
        }
        C2_TYPE_AABB => {
            let p = unsafe { &mut *p };
            let bb = unsafe { &*shape.cast::<C2AABB>() };
            p.radius = 0.0;
            p.count = 4;
            p.verts[0] = bb.min;
            p.verts[1] = c2V(bb.max.x, bb.min.y);
            p.verts[2] = bb.max;
            p.verts[3] = c2V(bb.min.x, bb.max.y);
        }
        C2_TYPE_CAPSULE => {
            let p = unsafe { &mut *p };
            let capsule = unsafe { &*shape.cast::<C2Capsule>() };
            p.radius = capsule.r;
            p.count = 2;
            p.verts[0] = capsule.a;
            p.verts[1] = capsule.b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Len(a: C2v) -> c_float {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> c_float {
    sub_left(mul_left(b.y, a.x), mul_left(b.x, a.y))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut C2Simplex) -> c_float {
    let s = unsafe { &*s };
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v {
    c2V(
        sub_left(mul_left(b.x, a.c), mul_left(b.y, a.s)),
        add_left(mul_left(a.s, b.x), mul_left(b.y, a.c)),
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x = add_left(b.x, a.x);
    a.y = add_left(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c22(s: *mut C2Simplex) {
    let s = unsafe { &mut *s };
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

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c23(s: *mut C2Simplex) {
    let s = unsafe { &mut *s };
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

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Neg(a: C2v) -> C2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2D(s: *mut C2Simplex) -> C2v {
    let s = unsafe { &*s };
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

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2Dot(unsafe { *verts }, d);
    let mut i = 1;
    while i < count {
        let dot = c2Dot(unsafe { *verts.add(i as usize) }, d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2Witness(s: *mut C2Simplex, a: *mut C2v, b: *mut C2v) {
    let s = unsafe { &*s };
    let den = 1.0 / s.div;
    unsafe {
        match s.count {
            1 => {
                *a = s.a.s_a;
                *b = s.a.s_b;
            }
            2 => {
                *a = c2Add(c2Mulvs(s.a.s_a, den * s.a.u), c2Mulvs(s.b.s_a, den * s.b.u));
                *b = c2Add(c2Mulvs(s.a.s_b, den * s.a.u), c2Mulvs(s.b.s_b, den * s.b.u));
            }
            3 => {
                *a = c2Add(
                    c2Add(c2Mulvs(s.a.s_a, den * s.a.u), c2Mulvs(s.b.s_a, den * s.b.u)),
                    c2Mulvs(s.c.s_a, den * s.c.u),
                );
                *b = c2Add(
                    c2Add(c2Mulvs(s.a.s_b, den * s.a.u), c2Mulvs(s.b.s_b, den * s.b.u)),
                    c2Mulvs(s.c.s_b, den * s.c.u),
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
#[inline(never)]
pub extern "C" fn c2Div(a: C2v, b: c_float) -> C2v {
    c2Mulvs(a, div_left(1.0, b))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Norm(a: C2v) -> C2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2L(s: *mut C2Simplex) -> C2v {
    let s = unsafe { &*s };
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v {
    c2V(
        add_left(mul_left(a.c, b.x), mul_left(b.y, a.s)),
        add_left(mul_left(-a.s, b.x), mul_left(b.y, a.c)),
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
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
    cache: *mut C2GJKCache,
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

    let mut p_a = C2Proxy::default();
    let mut p_b = C2Proxy::default();
    unsafe {
        c2MakeProxy(a_shape, type_a, &mut p_a);
        c2MakeProxy(b_shape, type_b, &mut p_b);
    }

    let mut s = C2Simplex::default();
    let mut cache_was_read = false;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        if cache_ref.count != 0 {
            let mut i = 0;
            while i < cache_ref.count {
                let index = i as usize;
                let i_a = cache_ref.i_a[index];
                let i_b = cache_ref.i_b[index];
                let support_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
                let support_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
                let v = s.vertex_mut(index);
                v.i_a = i_a;
                v.s_a = support_a;
                v.i_b = i_b;
                v.s_b = support_b;
                v.p = c2Sub(v.s_b, v.s_a);
                v.u = 0.0;
                i += 1;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = unsafe { c2GJKSimplexMetric(&mut s) };
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
    let mut d0 = c_float::MAX;
    let mut iter = 0;
    let mut hit = false;
    while iter < 20 {
        let save_count = s.count;
        let mut i = 0;
        while i < save_count {
            save_a[i as usize] = s.vertex(i as usize).i_a;
            save_b[i as usize] = s.vertex(i as usize).i_b;
            i += 1;
        }

        match s.count {
            2 => unsafe { c22(&mut s) },
            3 => unsafe { c23(&mut s) },
            _ => {}
        }
        if s.count == 3 {
            hit = true;
            break;
        }

        let p = unsafe { c2L(&mut s) };
        let d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = unsafe { c2D(&mut s) };
        if c2Dot(d, d) < c_float::EPSILON * c_float::EPSILON {
            break;
        }

        let i_a = unsafe { c2Support(p_a.verts.as_ptr(), p_a.count, c2MulrvT(ax.r, c2Neg(d))) };
        let support_a = c2Mulxv(ax, p_a.verts[i_a as usize]);
        let i_b = unsafe { c2Support(p_b.verts.as_ptr(), p_b.count, c2MulrvT(bx.r, d)) };
        let support_b = c2Mulxv(bx, p_b.verts[i_b as usize]);
        let v = s.vertex_mut(s.count as usize);
        v.i_a = i_a;
        v.s_a = support_a;
        v.i_b = i_b;
        v.s_b = support_b;
        v.p = c2Sub(v.s_b, v.s_a);

        let mut duplicate = false;
        let mut i = 0;
        while i < save_count {
            if i_a == save_a[i as usize] && i_b == save_b[i as usize] {
                duplicate = true;
                break;
            }
            i += 1;
        }
        if duplicate {
            break;
        }
        s.count += 1;
        iter += 1;
    }

    let mut a = C2v::default();
    let mut b = C2v::default();
    unsafe { c2Witness(&mut s, &mut a, &mut b) };
    let mut dist = c2Len(c2Sub(a, b));
    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let r_a = p_a.radius;
        let r_b = p_b.radius;
        if dist > r_a + r_b && dist > c_float::EPSILON {
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
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = unsafe { c2GJKSimplexMetric(&mut s) };
        cache_ref.count = s.count;
        let mut i = 0;
        while i < s.count {
            let v = s.vertex(i as usize);
            cache_ref.i_a[i as usize] = v.i_a;
            cache_ref.i_b[i as usize] = v.i_b;
            i += 1;
        }
        cache_ref.div = s.div;
    }
    if !out_a.is_null() {
        unsafe { *out_a = a };
    }
    if !out_b.is_null() {
        unsafe { *out_b = b };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }
    dist
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    (!(d0 || d1 || d2 || d3)) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2AABBtoCapsule(a: C2AABB, b: C2Capsule) -> c_int {
    let distance = unsafe {
        c2GJK(
            (&a as *const C2AABB).cast(),
            C2_TYPE_AABB,
            std::ptr::null(),
            (&b as *const C2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (distance == 0.0) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CapsuletoCapsule(a: C2Capsule, b: C2Capsule) -> c_int {
    let distance = unsafe {
        c2GJK(
            (&a as *const C2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            (&b as *const C2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (distance == 0.0) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CircletoCapsule(a: C2Circle, b: C2Capsule) -> c_int {
    let n = c2Sub(b.b, b.a);
    let ap = c2Sub(a.p, b.a);
    let da = c2Dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(a.p, b.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2Collided(
    a: *const c_void,
    type_a: c_int,
    b: *const c_void,
    type_b: c_int,
) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => unsafe {
                c2CircletoCircle(*a.cast::<C2Circle>(), *b.cast::<C2Circle>())
            },
            C2_TYPE_AABB => unsafe { c2CircletoAABB(*a.cast::<C2Circle>(), *b.cast::<C2AABB>()) },
            C2_TYPE_CAPSULE => unsafe {
                c2CircletoCapsule(*a.cast::<C2Circle>(), *b.cast::<C2Capsule>())
            },
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => unsafe { c2CircletoAABB(*b.cast::<C2Circle>(), *a.cast::<C2AABB>()) },
            C2_TYPE_AABB => unsafe { c2AABBtoAABB(*a.cast::<C2AABB>(), *b.cast::<C2AABB>()) },
            C2_TYPE_CAPSULE => unsafe {
                c2AABBtoCapsule(*a.cast::<C2AABB>(), *b.cast::<C2Capsule>())
            },
            _ => 0,
        },
        C2_TYPE_CAPSULE => match type_b {
            C2_TYPE_CIRCLE => unsafe {
                c2CircletoCapsule(*b.cast::<C2Circle>(), *a.cast::<C2Capsule>())
            },
            C2_TYPE_AABB => unsafe { c2AABBtoCapsule(*b.cast::<C2AABB>(), *a.cast::<C2Capsule>()) },
            C2_TYPE_CAPSULE => unsafe {
                c2CapsuletoCapsule(*a.cast::<C2Capsule>(), *b.cast::<C2Capsule>())
            },
            _ => 0,
        },
        _ => 0,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn reverse_collide(x: c_float, y: c_float, r: c_float) -> c_int {
    let circle_in = C2Circle { p: c2V(x, y), r };
    let circle = C2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };
    let aabb = C2AABB {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };
    let capsule = C2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    let mut result = unsafe {
        c2Collided(
            (&circle as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
            (&circle_in as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
        )
    };
    result += unsafe {
        c2Collided(
            (&aabb as *const C2AABB).cast(),
            C2_TYPE_AABB,
            (&circle_in as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
        ) << 1
    };
    result += unsafe {
        c2Collided(
            (&capsule as *const C2Capsule).cast(),
            C2_TYPE_CAPSULE,
            (&circle_in as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
        ) << 2
    };
    result
}
