#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl c2Proxy {
    pub fn new() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v { x: 0.0, y: 0.0 }; 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

impl c2sv {
    pub fn new() -> Self {
        c2sv {
            sA: c2v { x: 0.0, y: 0.0 },
            sB: c2v { x: 0.0, y: 0.0 },
            p: c2v { x: 0.0, y: 0.0 },
            u: 0.0,
            iA: 0,
            iB: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

impl c2Simplex {
    pub fn new() -> Self {
        c2Simplex {
            a: c2sv::new(),
            b: c2sv::new(),
            c: c2sv::new(),
            d: c2sv::new(),
            div: 0.0,
            count: 0,
        }
    }

    /// Get a mutable reference to one of the simplex vertices, by index.
    /// Mirrors the C pattern `verts = &s.a; verts[i]` where the four
    /// `c2sv` members are laid out contiguously in memory.
    pub fn vert(&self, i: usize) -> &c2sv {
        match i {
            0 => &self.a,
            1 => &self.b,
            2 => &self.c,
            3 => &self.d,
            _ => panic!("simplex index out of bounds"),
        }
    }

    pub fn vert_mut(&mut self, i: usize) -> &mut c2sv {
        match i {
            0 => &mut self.a,
            1 => &mut self.b,
            2 => &mut self.c,
            3 => &mut self.d,
            _ => panic!("simplex index out of bounds"),
        }
    }
}

// =============================================================================
// Internal implementations. Each public-facing FFI export below calls into
// these. The C-equivalent functions in this module take Rust references rather
// than pointers and therefore can't share names with the `extern "C"` exports
// further down.
// =============================================================================

#[inline]
pub(crate) fn c2V_i(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
pub(crate) fn c2Mulvs_i(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
pub(crate) fn c2Maxv_i(a: c2v, b: c2v) -> c2v {
    c2V_i(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
pub(crate) fn c2Minv_i(a: c2v, b: c2v) -> c2v {
    c2V_i(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
pub(crate) fn c2Clampv_i(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv_i(lo, c2Minv_i(a, hi))
}

#[inline]
pub(crate) fn c2Sub_i(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
pub(crate) fn c2Dot_i(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
pub(crate) fn c2RotIdentity_i() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[inline]
pub(crate) fn c2xIdentity_i() -> c2x {
    c2x {
        p: c2V_i(0.0, 0.0),
        r: c2RotIdentity_i(),
    }
}

pub(crate) fn c2BBVerts_i(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V_i(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V_i(bb.min.x, bb.max.y);
}

pub(crate) unsafe fn c2MakeProxy_i(shape: *const c_void, ty: C2_TYPE, p: &mut c2Proxy) {
    match ty {
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
            c2BBVerts_i(&mut p.verts, bb);
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

#[inline]
pub(crate) fn c2Len_i(a: c2v) -> f32 {
    c2Dot_i(a, a).sqrt()
}

#[inline]
pub(crate) fn c2Det2_i(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

pub(crate) fn c2GJKSimplexMetric_i(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len_i(c2Sub_i(s.b.p, s.a.p)),
        3 => c2Det2_i(c2Sub_i(s.b.p, s.a.p), c2Sub_i(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

#[inline]
pub(crate) fn c2Mulrv_i(a: c2r, b: c2v) -> c2v {
    c2V_i(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
pub(crate) fn c2Add_i(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
pub(crate) fn c2Mulxv_i(a: c2x, b: c2v) -> c2v {
    c2Add_i(c2Mulrv_i(a.r, b), a.p)
}

pub(crate) fn c22_i(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = c2Dot_i(b, c2Sub_i(b, a));
    let v = c2Dot_i(a, c2Sub_i(a, b));
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

pub(crate) fn c23_i(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
    let uAB = c2Dot_i(b, c2Sub_i(b, a));
    let vAB = c2Dot_i(a, c2Sub_i(a, b));
    let uBC = c2Dot_i(c, c2Sub_i(c, b));
    let vBC = c2Dot_i(b, c2Sub_i(b, c));
    let uCA = c2Dot_i(a, c2Sub_i(a, c));
    let vCA = c2Dot_i(c, c2Sub_i(c, a));
    let area = c2Det2_i(c2Sub_i(b, a), c2Sub_i(c, a));
    let uABC = c2Det2_i(b, c) * area;
    let vABC = c2Det2_i(c, a) * area;
    let wABC = c2Det2_i(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.a = s.c;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.a.u = uAB;
        s.b.u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.a = s.b;
        s.b = s.c;
        s.a.u = uBC;
        s.b.u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.b = s.a;
        s.a = s.c;
        s.a.u = uCA;
        s.b.u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.a.u = uABC;
        s.b.u = vABC;
        s.c.u = wABC;
        s.div = uABC + vABC + wABC;
        s.count = 3;
    }
}

#[inline]
pub(crate) fn c2Neg_i(a: c2v) -> c2v {
    c2V_i(-a.x, -a.y)
}

#[inline]
pub(crate) fn c2Skew_i(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline]
pub(crate) fn c2CCW90_i(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

pub(crate) fn c2D_i(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg_i(s.a.p),
        2 => {
            let ab = c2Sub_i(s.b.p, s.a.p);
            if c2Det2_i(ab, c2Neg_i(s.a.p)) > 0.0 {
                c2Skew_i(ab)
            } else {
                c2CCW90_i(ab)
            }
        }
        _ => c2V_i(0.0, 0.0),
    }
}

pub(crate) fn c2Support_i(verts: &[c2v], count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot_i(verts[0], d);
    let mut i: c_int = 1;
    while i < count {
        let dot = c2Dot_i(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

pub(crate) fn c2Witness_i(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add_i(
                c2Mulvs_i(s.a.sA, den * s.a.u),
                c2Mulvs_i(s.b.sA, den * s.b.u),
            );
            *b = c2Add_i(
                c2Mulvs_i(s.a.sB, den * s.a.u),
                c2Mulvs_i(s.b.sB, den * s.b.u),
            );
        }
        3 => {
            *a = c2Add_i(
                c2Add_i(
                    c2Mulvs_i(s.a.sA, den * s.a.u),
                    c2Mulvs_i(s.b.sA, den * s.b.u),
                ),
                c2Mulvs_i(s.c.sA, den * s.c.u),
            );
            *b = c2Add_i(
                c2Add_i(
                    c2Mulvs_i(s.a.sB, den * s.a.u),
                    c2Mulvs_i(s.b.sB, den * s.b.u),
                ),
                c2Mulvs_i(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2V_i(0.0, 0.0);
            *b = c2V_i(0.0, 0.0);
        }
    }
}

#[inline]
pub(crate) fn c2Div_i(a: c2v, b: f32) -> c2v {
    c2Mulvs_i(a, 1.0 / b)
}

#[inline]
pub(crate) fn c2Norm_i(a: c2v) -> c2v {
    c2Div_i(a, c2Len_i(a))
}

pub(crate) fn c2L_i(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add_i(
            c2Mulvs_i(s.a.p, den * s.a.u),
            c2Mulvs_i(s.b.p, den * s.b.u),
        ),
        _ => c2V_i(0.0, 0.0),
    }
}

#[inline]
pub(crate) fn c2MulrvT_i(a: c2r, b: c2v) -> c2v {
    c2V_i(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

pub(crate) unsafe fn c2GJK_i(
    a_shape: *const c_void,
    type_a: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const c_void,
    type_b: C2_TYPE,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax: c2x = if ax_ptr.is_null() {
        c2xIdentity_i()
    } else {
        unsafe { *ax_ptr }
    };
    let bx: c2x = if bx_ptr.is_null() {
        c2xIdentity_i()
    } else {
        unsafe { *bx_ptr }
    };

    let mut pA = c2Proxy::new();
    let mut pB = c2Proxy::new();
    unsafe {
        c2MakeProxy_i(a_shape, type_a, &mut pA);
        c2MakeProxy_i(b_shape, type_b, &mut pB);
    }

    let mut s = c2Simplex::new();
    let mut cache_was_read = 0;

    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let iA = cache_ref.iA[i];
                let iB = cache_ref.iB[i];
                let sA = c2Mulxv_i(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv_i(bx, pB.verts[iB as usize]);
                let v = s.vert_mut(i);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub_i(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric_i(&s);
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
        s.a.sA = c2Mulxv_i(ax, pA.verts[0]);
        s.a.sB = c2Mulxv_i(bx, pB.verts[0]);
        s.a.p = c2Sub_i(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int;
    let mut d0: f32 = 3.40282346638528859811704183484516925e+38_f32;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = s.vert(i);
            saveA[i] = v.iA;
            saveB[i] = v.iB;
        }
        match s.count {
            1 => {}
            2 => c22_i(&mut s),
            3 => c23_i(&mut s),
            _ => {}
        }
        if s.count == 3 {
            hit = 1;
            break;
        }
        let p = c2L_i(&s);
        d1 = c2Dot_i(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D_i(&s);
        if c2Dot_i(d, d)
            < 1.19209289550781250000000000000000000e-7_f32
                * 1.19209289550781250000000000000000000e-7_f32
        {
            break;
        }
        let iA = c2Support_i(&pA.verts, pA.count, c2MulrvT_i(ax.r, c2Neg_i(d)));
        let sA = c2Mulxv_i(ax, pA.verts[iA as usize]);
        let iB = c2Support_i(&pB.verts, pB.count, c2MulrvT_i(bx.r, d));
        let sB = c2Mulxv_i(bx, pB.verts[iB as usize]);
        {
            let count = s.count as usize;
            let v = s.vert_mut(count);
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub_i(v.sB, v.sA);
        }
        let mut dup = 0;
        for i in 0..save_count as usize {
            if iA == saveA[i] && iB == saveB[i] {
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

    let mut a_pt = c2V_i(0.0, 0.0);
    let mut b_pt = c2V_i(0.0, 0.0);
    c2Witness_i(&s, &mut a_pt, &mut b_pt);
    let mut dist = c2Len_i(c2Sub_i(a_pt, b_pt));
    if hit != 0 {
        a_pt = b_pt;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7_f32 {
            dist -= rA + rB;
            let n = c2Norm_i(c2Sub_i(b_pt, a_pt));
            a_pt = c2Add_i(a_pt, c2Mulvs_i(n, rA));
            b_pt = c2Sub_i(b_pt, c2Mulvs_i(n, rB));
            if a_pt.x == b_pt.x && a_pt.y == b_pt.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs_i(c2Add_i(a_pt, b_pt), 0.5);
            a_pt = p;
            b_pt = p;
            dist = 0.0;
        }
    }

    if !cache.is_null() {
        let cache_mut = unsafe { &mut *cache };
        cache_mut.metric = c2GJKSimplexMetric_i(&s);
        cache_mut.count = s.count;
        for i in 0..s.count as usize {
            let v = s.vert(i);
            cache_mut.iA[i] = v.iA;
            cache_mut.iB[i] = v.iB;
        }
        cache_mut.div = s.div;
    }

    if !out_a.is_null() {
        unsafe { *out_a = a_pt; }
    }
    if !out_b.is_null() {
        unsafe { *out_b = b_pt; }
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter; }
    }
    dist
}

// =============================================================================
// FFI EXPORTS — these match every symbol exported by the C .so so external
// callers (and tests) can call into the Rust translation by the same names.
// =============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2V_i(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    c2Mulvs_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2Maxv_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2Minv_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Clampv_i(a, lo, hi)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2Sub_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    c2Dot_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2RotIdentity_i()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2xIdentity_i()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        let slice = std::slice::from_raw_parts_mut(out, 4);
        let bb_ref = &*bb;
        c2BBVerts_i(slice, bb_ref);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, ty: C2_TYPE, p: *mut c2Proxy) {
    unsafe {
        let p_ref = &mut *p;
        c2MakeProxy_i(shape, ty, p_ref);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Len_i(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    c2Det2_i(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    let s = unsafe { &*s };
    c2GJKSimplexMetric_i(s)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2Mulrv_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    c2Add_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Mulxv_i(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let s = unsafe { &mut *s };
    c22_i(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s = unsafe { &mut *s };
    c23_i(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2Neg_i(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2Skew_i(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2CCW90_i(a)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    let s = unsafe { &*s };
    c2D_i(s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    if count <= 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(verts, count as usize) };
    c2Support_i(slice, count, d)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let s = &*s;
        c2Witness_i(s, &mut *a, &mut *b);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Div_i(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Norm_i(a)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s = unsafe { &*s };
    c2L_i(s)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2MulrvT_i(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    a_shape: *const c_void,
    type_a: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const c_void,
    type_b: C2_TYPE,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    unsafe {
        c2GJK_i(
            a_shape, type_a, ax_ptr, b_shape, type_b, bx_ptr, out_a, out_b,
            use_radius, iterations, cache,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk(
    reverse: c_char,
    a: *mut c2v,
    b: *mut c2v,
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
    let bb = c2AABB {
        min: c2V_i(a1, a2),
        max: c2V_i(a3, a4),
    };

    let cap = c2Capsule {
        a: c2V_i(b1, b2),
        b: c2V_i(b3, b4),
        r: b5,
    };

    if reverse != 0 {
        unsafe {
            c2GJK_i(
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE::C2_TYPE_CAPSULE,
                std::ptr::null(),
                &bb as *const c2AABB as *const c_void,
                C2_TYPE::C2_TYPE_AABB,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    } else {
        unsafe {
            c2GJK_i(
                &bb as *const c2AABB as *const c_void,
                C2_TYPE::C2_TYPE_AABB,
                std::ptr::null(),
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE::C2_TYPE_CAPSULE,
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
