#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum C2Type {
    Circle = 0,
    Aabb = 1,
    Capsule = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2GjkCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Simplex {
    pub a: C2sv,
    pub b: C2sv,
    pub c: C2sv,
    pub d: C2sv,
    pub div: f32,
    pub count: c_int,
}

const ZERO_V: C2v = C2v { x: 0.0, y: 0.0 };
const ZERO_SV: C2sv = C2sv {
    sA: ZERO_V,
    sB: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    iA: 0,
    iB: 0,
};

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn fp_add(lhs: f32, rhs: f32) -> f32 {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "addss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn fp_add(lhs: f32, rhs: f32) -> f32 {
    lhs + rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn fp_sub(lhs: f32, rhs: f32) -> f32 {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "subss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn fp_sub(lhs: f32, rhs: f32) -> f32 {
    lhs - rhs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn fp_mul(lhs: f32, rhs: f32) -> f32 {
    let mut result = lhs;
    unsafe {
        std::arch::asm!(
            "mulss {result}, {rhs}",
            result = inout(xmm_reg) result,
            rhs = in(xmm_reg) rhs,
            options(pure, nomem, nostack, preserves_flags),
        );
    }
    result
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn fp_mul(lhs: f32, rhs: f32) -> f32 {
    lhs * rhs
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: f32) -> C2v {
    a.x = fp_mul(a.x, b);
    a.y = fp_mul(a.y, b);
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
    a.x = fp_sub(a.x, b.x);
    a.y = fp_sub(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    fp_add(fp_mul(a.x, b.x), fp_mul(a.y, b.y))
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
pub extern "C" fn c2BBVerts(out: *mut C2v, bb: *mut C2Aabb) {
    unsafe {
        let bb = *bb;
        *out.add(0) = bb.min;
        *out.add(1) = c2V(bb.max.x, bb.min.y);
        *out.add(2) = bb.max;
        *out.add(3) = c2V(bb.min.x, bb.max.y);
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2MakeProxy(shape: *const c_void, shape_type: c_int, p: *mut C2Proxy) {
    unsafe {
        match shape_type {
            0 => {
                let circle = &*(shape.cast::<C2Circle>());
                (*p).radius = circle.r;
                (*p).count = 1;
                (*p).verts[0] = circle.p;
            }
            1 => {
                let bb = shape.cast::<C2Aabb>().cast_mut();
                (*p).radius = 0.0;
                (*p).count = 4;
                c2BBVerts((*p).verts.as_mut_ptr(), bb);
            }
            2 => {
                let capsule = &*(shape.cast::<C2Capsule>());
                (*p).radius = capsule.r;
                (*p).count = 2;
                (*p).verts[0] = capsule.a;
                (*p).verts[1] = capsule.b;
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Len(a: C2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 {
    fp_sub(fp_mul(a.x, b.y), fp_mul(a.y, b.x))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2GJKSimplexMetric(s: *mut C2Simplex) -> f32 {
    unsafe {
        match (*s).count {
            2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
            3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v {
    c2V(
        fp_sub(fp_mul(a.c, b.x), fp_mul(a.s, b.y)),
        fp_add(fp_mul(a.s, b.x), fp_mul(a.c, b.y)),
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x = fp_add(a.x, b.x);
    a.y = fp_add(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c22(s: *mut C2Simplex) {
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
            (*s).div = fp_add(u, v);
            (*s).count = 2;
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c23(s: *mut C2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let c = (*s).c.p;
        let uAB = c2Dot(b, c2Sub(b, a));
        let vAB = c2Dot(a, c2Sub(a, b));
        let uBC = c2Dot(c, c2Sub(c, b));
        let vBC = c2Dot(b, c2Sub(b, c));
        let uCA = c2Dot(a, c2Sub(a, c));
        let vCA = c2Dot(c, c2Sub(c, a));
        let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
        let uABC = fp_mul(c2Det2(b, c), area);
        let vABC = fp_mul(c2Det2(c, a), area);
        let wABC = fp_mul(c2Det2(a, b), area);
        if vAB <= 0.0 && uCA <= 0.0 {
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            (*s).a = (*s).c;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            (*s).a.u = uAB;
            (*s).b.u = vAB;
            (*s).div = fp_add(uAB, vAB);
            (*s).count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            (*s).a = (*s).b;
            (*s).b = (*s).c;
            (*s).a.u = uBC;
            (*s).b.u = vBC;
            (*s).div = fp_add(uBC, vBC);
            (*s).count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            (*s).b = (*s).a;
            (*s).a = (*s).c;
            (*s).a.u = uCA;
            (*s).b.u = vCA;
            (*s).div = fp_add(uCA, vCA);
            (*s).count = 2;
        } else {
            (*s).a.u = uABC;
            (*s).b.u = vABC;
            (*s).c.u = wABC;
            (*s).div = fp_add(fp_add(uABC, vABC), wABC);
            (*s).count = 3;
        }
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
pub extern "C" fn c2D(s: *mut C2Simplex) -> C2v {
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
#[inline(never)]
pub extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
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
#[inline(never)]
pub extern "C" fn c2Witness(s: *mut C2Simplex, a: *mut C2v, b: *mut C2v) {
    unsafe {
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => {
                *a = (*s).a.sA;
                *b = (*s).a.sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs((*s).a.sA, fp_mul(den, (*s).a.u)),
                    c2Mulvs((*s).b.sA, fp_mul(den, (*s).b.u)),
                );
                *b = c2Add(
                    c2Mulvs((*s).a.sB, fp_mul(den, (*s).a.u)),
                    c2Mulvs((*s).b.sB, fp_mul(den, (*s).b.u)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.sA, fp_mul(den, (*s).a.u)),
                        c2Mulvs((*s).b.sA, fp_mul(den, (*s).b.u)),
                    ),
                    c2Mulvs((*s).c.sA, fp_mul(den, (*s).c.u)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.sB, fp_mul(den, (*s).a.u)),
                        c2Mulvs((*s).b.sB, fp_mul(den, (*s).b.u)),
                    ),
                    c2Mulvs((*s).c.sB, fp_mul(den, (*s).c.u)),
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
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Norm(a: C2v) -> C2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2L(s: *mut C2Simplex) -> C2v {
    unsafe {
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => (*s).a.p,
            2 => c2Add(
                c2Mulvs((*s).a.p, fp_mul(den, (*s).a.u)),
                c2Mulvs((*s).b.p, fp_mul(den, (*s).b.u)),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v {
    c2V(
        fp_add(fp_mul(a.c, b.x), fp_mul(a.s, b.y)),
        fp_add(fp_mul(-a.s, b.x), fp_mul(a.c, b.y)),
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_int,
    ax_ptr: *const C2x,
    B: *const c_void,
    typeB: c_int,
    bx_ptr: *const C2x,
    outA: *mut C2v,
    outB: *mut C2v,
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
        let mut pA = C2Proxy {
            radius: 0.0,
            count: 0,
            verts: [ZERO_V; 8],
        };
        let mut pB = pA;
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);
        let mut s = C2Simplex {
            a: ZERO_SV,
            b: ZERO_SV,
            c: ZERO_SV,
            d: ZERO_SV,
            div: 0.0,
            count: 0,
        };
        let verts = (&mut s.a as *mut C2sv).cast::<C2sv>();
        let mut cache_was_read = false;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                let mut i = 0;
                while i < (*cache).count {
                    let iA = (*cache).iA[i as usize];
                    let iB = (*cache).iB[i as usize];
                    let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                    let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                    let v = &mut *verts.add(i as usize);
                    v.iA = iA;
                    v.sA = sA;
                    v.iB = iB;
                    v.sB = sB;
                    v.p = c2Sub(v.sB, v.sA);
                    v.u = 0.0;
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
                    cache_was_read = true;
                }
            }
        }
        if !cache_was_read {
            s.a.iA = 0;
            s.a.iB = 0;
            s.a.sA = c2Mulxv(ax, pA.verts[0]);
            s.a.sB = c2Mulxv(bx, pB.verts[0]);
            s.a.p = c2Sub(s.a.sB, s.a.sA);
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }
        let mut saveA = [0; 3];
        let mut saveB = [0; 3];
        let mut d0 = f32::MAX;
        let mut iter = 0;
        let mut hit = false;
        while iter < 20 {
            let save_count = s.count;
            let mut i = 0;
            while i < save_count {
                saveA[i as usize] = (*verts.add(i as usize)).iA;
                saveB[i as usize] = (*verts.add(i as usize)).iB;
                i += 1;
            }
            match s.count {
                2 => c22(&mut s),
                3 => c23(&mut s),
                _ => {}
            }
            if s.count == 3 {
                hit = true;
                break;
            }
            let p = c2L(&mut s);
            let d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;
            let d = c2D(&mut s);
            if c2Dot(d, d) < f32::EPSILON * f32::EPSILON {
                break;
            }
            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, pA.verts[iA as usize]);
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, pB.verts[iB as usize]);
            let v = &mut *verts.add(s.count as usize);
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub(v.sB, v.sA);
            let mut dup = false;
            let mut i = 0;
            while i < save_count {
                if iA == saveA[i as usize] && iB == saveB[i as usize] {
                    dup = true;
                    break;
                }
                i += 1;
            }
            if dup {
                break;
            }
            s.count += 1;
            iter += 1;
        }
        let mut a = ZERO_V;
        let mut b = ZERO_V;
        c2Witness(&mut s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));
        if hit {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            let radius_sum = fp_add(rA, rB);
            if dist > radius_sum && dist > f32::EPSILON {
                dist = fp_sub(dist, radius_sum);
                let n = c2Norm(c2Sub(b, a));
                a = c2Add(a, c2Mulvs(n, rA));
                b = c2Sub(b, c2Mulvs(n, rB));
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
                let v = &*verts.add(i as usize);
                (*cache).iA[i as usize] = v.iA;
                (*cache).iB[i as usize] = v.iB;
                i += 1;
            }
            (*cache).div = s.div;
        }
        if !outA.is_null() {
            *outA = a;
        }
        if !outB.is_null() {
            *outB = b;
        }
        if !iterations.is_null() {
            *iterations = iter;
        }
        dist
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn gjk(
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
    let bb = C2Aabb {
        min: c2V(a1, a2),
        max: c2V(a3, a4),
    };
    let cap = C2Capsule {
        a: c2V(b1, b2),
        b: c2V(b3, b4),
        r: b5,
    };
    if reverse != 0 {
        c2GJK(
            (&cap as *const C2Capsule).cast(),
            C2Type::Capsule as c_int,
            std::ptr::null(),
            (&bb as *const C2Aabb).cast(),
            C2Type::Aabb as c_int,
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
            C2Type::Aabb as c_int,
            std::ptr::null(),
            (&cap as *const C2Capsule).cast(),
            C2Type::Capsule as c_int,
            std::ptr::null(),
            a,
            b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}
