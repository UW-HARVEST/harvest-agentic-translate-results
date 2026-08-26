//! Rust translation of the C library in `c_src/` (cute_c2 style GJK routines).
//!
//! The translation is intentionally literal: every exported C function is
//! reproduced with the same name, signature, order of operations, order of
//! error/validation checks and (buggy) behaviour as the original C code.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Public types (include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

// ---------------------------------------------------------------------------
// Private types (src/lib.c)
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

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
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
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

/// Layout identical to the C `c2Simplex { c2sv a, b, c, d; float div; int count; }`.
/// The four vertices are stored as an array because the C code walks them via
/// `c2sv* verts = &s.a;`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

const ZERO_V: c2v = c2v { x: 0.0, y: 0.0 };

const ZERO_SV: c2sv = c2sv {
    sA: ZERO_V,
    sB: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    iA: 0,
    iB: 0,
};

// FLT_MAX / FLT_EPSILON as spelled out in the C source.
const C_FLT_MAX: f32 = 3.402_823_466_385_288_6e+38_f32;
const C_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

// ---------------------------------------------------------------------------
// Bit-exact scalar arithmetic helpers
//
// `ADDSS`/`MULSS` on x86 give NaN-propagation priority to their *destination*
// operand: if the destination is a NaN the result is that NaN (quieted),
// otherwise if the source is a NaN the result is the source NaN (quieted).
// Since `+` and `*` are commutative the compiler is free to choose which
// operand ends up in the destination register, and that choice decides which
// NaN payload survives.  To stay bit-identical with the C build we spell the
// destination operand out explicitly: `addp(dst, src)` / `mulp(dst, src)`
// reproduce `ADDSS dst, src` / `MULSS dst, src`.
//
// For non-NaN operands these helpers are exactly `dst + src` / `dst * src`
// (both operations are bit-exactly commutative on finite/infinite values), so
// ordinary arithmetic is completely unaffected.
//
// Subtraction and division are not commutative, so `SUBSS`/`DIVSS` always take
// the left-hand operand as destination — plain `-` and `/` already match.
// ---------------------------------------------------------------------------

/// x86 NaN quieting: set the quiet bit, keep sign and payload.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `ADDSS dst, src`
#[inline(always)]
fn addp(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet(dst)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dst + src
    }
}

/// `MULSS dst, src`
#[inline(always)]
fn mulp(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet(dst)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dst * src
    }
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x = mulp(a.x, b);
    a.y = mulp(a.y, b);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // a.x * b.x + a.y * b.y
    let t1 = mulp(a.x, b.x);
    let t2 = mulp(b.y, a.y);
    addp(t2, t1)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    let mut r = c2r { c: 0.0, s: 0.0 };
    r.c = 1.0f32;
    r.s = 0.0;
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x {
        p: ZERO_V,
        r: c2r { c: 0.0, s: 0.0 },
    };
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        *out.offset(0) = (*bb).min;
        *out.offset(1) = c2V((*bb).max.x, (*bb).min.y);
        *out.offset(2) = (*bb).max;
        *out.offset(3) = c2V((*bb).min.x, (*bb).max.y);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    unsafe {
        match type_ {
            C2_TYPE_CIRCLE => {
                let c = shape as *mut c2Circle;
                (*p).radius = (*c).r;
                (*p).count = 1;
                (*p).verts[0] = (*c).p;
            }
            C2_TYPE_AABB => {
                let bb = shape as *mut c2AABB;
                (*p).radius = 0.0;
                (*p).count = 4;
                c2BBVerts((&raw mut (*p).verts) as *mut c2v, bb);
            }
            C2_TYPE_CAPSULE => {
                let c = shape as *mut c2Capsule;
                (*p).radius = (*c).r;
                (*p).count = 2;
                (*p).verts[0] = (*c).a;
                (*p).verts[1] = (*c).b;
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    // a.x * b.y - a.y * b.x
    let t1 = mulp(b.y, a.x);
    let t2 = mulp(b.x, a.y);
    t1 - t2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe {
        match (*s).count {
            2 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p)),
            3 => c2Det2(
                c2Sub((*s).verts[1].p, (*s).verts[0].p),
                c2Sub((*s).verts[2].p, (*s).verts[0].p),
            ),
            // `default:` falls through to `case 1:` in the C source.
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
    let x = mulp(b.x, a.c) - mulp(b.y, a.s);
    let y = addp(mulp(a.s, b.x), mulp(b.y, a.c));
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = addp(b.x, a.x);
    a.y = addp(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).verts[0].p;
        let b = (*s).verts[1].p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            (*s).verts[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if u <= 0.0 {
            (*s).verts[0] = (*s).verts[1];
            (*s).verts[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else {
            (*s).verts[0].u = u;
            (*s).verts[1].u = v;
            (*s).div = addp(u, v);
            (*s).count = 2;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).verts[0].p;
        let b = (*s).verts[1].p;
        let c = (*s).verts[2].p;
        let uAB = c2Dot(b, c2Sub(b, a));
        let vAB = c2Dot(a, c2Sub(a, b));
        let uBC = c2Dot(c, c2Sub(c, b));
        let vBC = c2Dot(b, c2Sub(b, c));
        let uCA = c2Dot(a, c2Sub(a, c));
        let vCA = c2Dot(c, c2Sub(c, a));
        let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
        let uABC = mulp(c2Det2(b, c), area);
        let vABC = mulp(c2Det2(c, a), area);
        let wABC = mulp(c2Det2(a, b), area);
        if vAB <= 0.0 && uCA <= 0.0 {
            (*s).verts[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            (*s).verts[0] = (*s).verts[1];
            (*s).verts[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            (*s).verts[0] = (*s).verts[2];
            (*s).verts[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            (*s).verts[0].u = uAB;
            (*s).verts[1].u = vAB;
            (*s).div = addp(uAB, vAB);
            (*s).count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            (*s).verts[0] = (*s).verts[1];
            (*s).verts[1] = (*s).verts[2];
            (*s).verts[0].u = uBC;
            (*s).verts[1].u = vBC;
            (*s).div = addp(uBC, vBC);
            (*s).count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            (*s).verts[1] = (*s).verts[0];
            (*s).verts[0] = (*s).verts[2];
            (*s).verts[0].u = uCA;
            (*s).verts[1].u = vCA;
            (*s).div = addp(uCA, vCA);
            (*s).count = 2;
        } else {
            (*s).verts[0].u = uABC;
            (*s).verts[1].u = vABC;
            (*s).verts[2].u = wABC;
            (*s).div = addp(addp(uABC, vABC), wABC);
            (*s).count = 3;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = -a.x;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        match (*s).count {
            1 => c2Neg((*s).verts[0].p),
            2 => {
                let ab = c2Sub((*s).verts[1].p, (*s).verts[0].p);
                if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {
                    return c2Skew(ab);
                }
                c2CCW90(ab)
            }
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax: c_int = 0;
        let mut dmax = c2Dot(*verts.offset(0), d);
        let mut i: c_int = 1;
        while i < count {
            let dot = c2Dot(*verts.offset(i as isize), d);
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
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let den = 1.0f32 / (*s).div;
        match (*s).count {
            1 => {
                *a = (*s).verts[0].sA;
                *b = (*s).verts[0].sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs((*s).verts[0].sA, mulp((*s).verts[0].u, den)),
                    c2Mulvs((*s).verts[1].sA, mulp((*s).verts[1].u, den)),
                );
                *b = c2Add(
                    c2Mulvs((*s).verts[0].sB, mulp((*s).verts[0].u, den)),
                    c2Mulvs((*s).verts[1].sB, mulp((*s).verts[1].u, den)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs((*s).verts[0].sA, mulp((*s).verts[0].u, den)),
                        c2Mulvs((*s).verts[1].sA, mulp((*s).verts[1].u, den)),
                    ),
                    c2Mulvs((*s).verts[2].sA, mulp((*s).verts[2].u, den)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs((*s).verts[0].sB, mulp((*s).verts[0].u, den)),
                        c2Mulvs((*s).verts[1].sB, mulp((*s).verts[1].u, den)),
                    ),
                    c2Mulvs((*s).verts[2].sB, mulp((*s).verts[2].u, den)),
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
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let den = 1.0f32 / (*s).div;
        match (*s).count {
            1 => (*s).verts[0].p,
            2 => c2Add(
                c2Mulvs((*s).verts[0].p, mulp((*s).verts[0].u, den)),
                c2Mulvs((*s).verts[1].p, mulp((*s).verts[1].u, den)),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
    let x = addp(mulp(a.c, b.x), mulp(b.y, a.s));
    let y = addp(mulp(-a.s, b.x), mulp(b.y, a.c));
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    unsafe {
        let ax: c2x;
        let bx: c2x;
        if ax_ptr.is_null() {
            ax = c2xIdentity();
        } else {
            ax = *ax_ptr;
        }
        if bx_ptr.is_null() {
            bx = c2xIdentity();
        } else {
            bx = *bx_ptr;
        }
        let mut pA = c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [ZERO_V; 8],
        };
        let mut pB = c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [ZERO_V; 8],
        };
        c2MakeProxy(A, typeA, &raw mut pA);
        c2MakeProxy(B, typeB, &raw mut pB);
        let mut s = c2Simplex {
            verts: [ZERO_SV; 4],
            div: 0.0,
            count: 0,
        };
        // `c2sv* verts = &s.a;` — the simplex vertices walked as an array.
        // Re-derived from the local at every use so the pointer provenance
        // always covers the whole vertex array.
        macro_rules! verts {
            ($i:expr) => {
                ((&raw mut s.verts) as *mut c2sv).offset($i as isize)
            };
        }
        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = ((*cache).count != 0) as c_int;
            if cache_was_good != 0 {
                let mut i: c_int = 0;
                while i < (*cache).count {
                    let iA = *((&raw const (*cache).iA) as *const c_int).offset(i as isize);
                    let iB = *((&raw const (*cache).iB) as *const c_int).offset(i as isize);
                    let sA = c2Mulxv(ax, *((&raw const pA.verts) as *const c2v).offset(iA as isize));
                    let sB = c2Mulxv(bx, *((&raw const pB.verts) as *const c2v).offset(iB as isize));
                    let v: *mut c2sv = verts!(i);
                    (*v).iA = iA;
                    (*v).sA = sA;
                    (*v).iB = iB;
                    (*v).sB = sB;
                    (*v).p = c2Sub((*v).sB, (*v).sA);
                    (*v).u = 0.0;
                    i += 1;
                }
                s.count = (*cache).count;
                s.div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2GJKSimplexMetric(&raw mut s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                    cache_was_read = 1;
                }
            }
        }
        if cache_was_read == 0 {
            s.verts[0].iA = 0;
            s.verts[0].iB = 0;
            s.verts[0].sA = c2Mulxv(ax, pA.verts[0]);
            s.verts[0].sB = c2Mulxv(bx, pB.verts[0]);
            s.verts[0].p = c2Sub(s.verts[0].sB, s.verts[0].sA);
            s.verts[0].u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        }
        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let mut save_count: c_int = 0;
        let mut d0: f32 = C_FLT_MAX;
        let mut d1: f32;
        let mut iter: c_int = 0;
        let mut hit: c_int = 0;
        while iter < 20 {
            save_count = s.count;
            let mut i: c_int = 0;
            while i < save_count {
                *((&raw mut saveA) as *mut c_int).offset(i as isize) = (*verts!(i)).iA;
                *((&raw mut saveB) as *mut c_int).offset(i as isize) = (*verts!(i)).iB;
                i += 1;
            }
            match s.count {
                1 => {}
                2 => c22(&raw mut s),
                3 => c23(&raw mut s),
                _ => {}
            }
            if s.count == 3 {
                hit = 1;
                break;
            }
            let p = c2L(&raw mut s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;
            let d = c2D(&raw mut s);
            if c2Dot(d, d) < C_FLT_EPSILON * C_FLT_EPSILON {
                break;
            }
            let iA = c2Support(
                (&raw const pA.verts) as *const c2v,
                pA.count,
                c2MulrvT(ax.r, c2Neg(d)),
            );
            let sA = c2Mulxv(ax, *((&raw const pA.verts) as *const c2v).offset(iA as isize));
            let iB = c2Support((&raw const pB.verts) as *const c2v, pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, *((&raw const pB.verts) as *const c2v).offset(iB as isize));
            let v: *mut c2sv = verts!(s.count);
            (*v).iA = iA;
            (*v).sA = sA;
            (*v).iB = iB;
            (*v).sB = sB;
            (*v).p = c2Sub((*v).sB, (*v).sA);
            let mut dup = 0;
            let mut i: c_int = 0;
            while i < save_count {
                if iA == *((&raw const saveA) as *const c_int).offset(i as isize)
                    && iB == *((&raw const saveB) as *const c_int).offset(i as isize)
                {
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
        let _ = save_count;
        let mut a = ZERO_V;
        let mut b = ZERO_V;
        c2Witness(&raw mut s, &raw mut a, &raw mut b);
        let mut dist = c2Len(c2Sub(a, b));
        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            if dist > addp(rA, rB) && dist > C_FLT_EPSILON {
                dist -= addp(rA, rB);
                let n = c2Norm(c2Sub(b, a));
                a = c2Add(a, c2Mulvs(n, rA));
                b = c2Sub(b, c2Mulvs(n, rB));
                if a.x == b.x && a.y == b.y {
                    dist = 0.0;
                }
            } else {
                let p = c2Mulvs(c2Add(a, b), 0.5f32);
                a = p;
                b = p;
                dist = 0.0;
            }
        }
        if !cache.is_null() {
            (*cache).metric = c2GJKSimplexMetric(&raw mut s);
            (*cache).count = s.count;
            let mut i: c_int = 0;
            while i < s.count {
                let v: *mut c2sv = verts!(i);
                *((&raw mut (*cache).iA) as *mut c_int).offset(i as isize) = (*v).iA;
                *((&raw mut (*cache).iB) as *mut c_int).offset(i as isize) = (*v).iB;
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

// ---------------------------------------------------------------------------
// Public entry point (include/lib.h)
// ---------------------------------------------------------------------------

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
    unsafe {
        let mut bb = c2AABB {
            min: ZERO_V,
            max: ZERO_V,
        };
        bb.min = c2V(a1, a2);
        bb.max = c2V(a3, a4);

        let mut cap = c2Capsule {
            a: ZERO_V,
            b: ZERO_V,
            r: 0.0,
        };
        cap.a = c2V(b1, b2);
        cap.b = c2V(b3, b4);
        cap.r = b5;

        if reverse != 0 {
            c2GJK(
                &mut cap as *mut c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                core::ptr::null(),
                &mut bb as *mut c2AABB as *const c_void,
                C2_TYPE_AABB,
                core::ptr::null(),
                a,
                b,
                1,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
        } else {
            c2GJK(
                &mut bb as *mut c2AABB as *const c_void,
                C2_TYPE_AABB,
                core::ptr::null(),
                &mut cap as *mut c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                core::ptr::null(),
                a,
                b,
                1,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
        }
    }
}
