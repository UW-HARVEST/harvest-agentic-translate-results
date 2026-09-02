//! Rust translation of the C library in `c_src/` (a cute_c2 derivative).
//!
//! Goals: identical exported ABI (46 symbols) and byte-identical results.
//!
//! Notes on faithfulness:
//!   * `c2MakeProxy` in the C source has **no** `C2_TYPE_POLY` case, so for a
//!     poly it leaves the caller's `c2Proxy` untouched. That is reproduced here
//!     exactly (the `_ => {}` arm). `c2GJK` declares its two proxies as
//!     uninitialized locals, so on the poly path the C reads whatever the
//!     *caller* left on the stack — demonstrably so: the C returns different
//!     manifolds for identical inputs depending only on call depth (see
//!     `tests/phase_c_indeterminate_stack.rs`). There is no portable value to
//!     match, so `c2GJK` zero-initializes its proxies here, and the differential
//!     tests zero-fill the stack below each FFI call, which pins the C to the
//!     same state. See `ERRORS.md` rows #37/#41.
//!   * `c2AABBtoCapsuleManifold` builds a `c2Poly` on the stack, and with a
//!     degenerate AABB the C reaches `verts[-1]`, reading the 8 bytes below
//!     `p.verts`. gcc's frame puts `A.max.y` and `p.count` there, which
//!     `AabbCapsulePolyFrame` reproduces exactly. See `ERRORS.md` row #69.
//!   * `ptr_from_parts` falls off the end of the function for `C2_TYPE_POLY`
//!     (no `return`). A null pointer is produced here; `c2Collide` has no poly
//!     arm, so the pointer is never dereferenced.
//!   * Sign-of-zero and NaN behaviour of the C ternary min/max/abs idioms is
//!     preserved by using the same comparisons rather than `f32::min`/`abs`.
//!   * Commutative float sites go through `fx::{add_l, add_r, mul_l, mul_r}`,
//!     which name the `addss`/`mulss` destination register explicitly. On x86
//!     the destination operand wins a NaN tie, and gcc -O0 picks it per
//!     expression in a way the C source does not express; each choice below was
//!     read off `objdump -d` of the C `.so` and is checked by
//!     `tests/phase_c_nan_payload.rs`.
//!   * Array indexing that the C never range-checks (`poly_vert`, `poly_norm`,
//!     `proxy_vert`, `c2Clip`'s `out[]`, `saveA`/`saveB`) uses raw pointer
//!     arithmetic or an over-sized buffer, so an out-of-range input behaves like
//!     the C instead of tripping a Rust bounds check (which, with
//!     `panic = "abort"`, would kill the process).
//!   * The `malloc` in `ptr_from_parts` is the real libc `malloc` and, as in the
//!     C code, the allocations made by `omni_manifold` are never freed.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Public enum values (C2_TYPE)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

/// `FLT_MAX`, matching the literal 3.40282346638528859811704183484516925e+38F.
const FLT_MAX: f32 = f32::MAX;
/// `FLT_EPSILON`, matching the literal 1.19209289550781250000000000000000000e-7F.
const FLT_EPSILON: f32 = f32::EPSILON;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Exact-operand-order scalar arithmetic
// ---------------------------------------------------------------------------
//
// `addss`/`mulss` are commutative in *value* but not in NaN propagation: when
// both operands are NaN the hardware returns the one in the DESTINATION
// register (Intel SDM, "SIMD Floating-Point Exceptions"/NaN operand tables).
// gcc -O0 picks the destination register per expression in a way that is not
// derivable from the C source (e.g. in `c2Dot` the first product keeps its
// left operand in the destination while the second keeps its right one), and
// LLVM makes its own independent choice.
//
// That difference is observable: a caller passing `+NaN` (0x7FC00000) while the
// library internally generates the x86 default `-NaN` (0xFFC00000) gets
// different NaN bits out of the two builds. So instead of writing `a + b` and
// hoping the register allocator agrees, every commutative site below names the
// destination explicitly via the SSE intrinsic, matching what
// `objdump -d` shows the C `.so` doing at that exact site.
//
// `sub`/`div` need no such treatment: `subss`/`divss` are not commutative, so
// the destination is always the left operand in both compilers.
mod fx {
    #![allow(dead_code)]

    /// `addss dst=a, src=b` -> `a + b`, `a` wins a NaN tie.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn add_l(a: f32, b: f32) -> f32 {
        let mut d = a;
        unsafe {
            core::arch::asm!(
                "addss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) b,
                options(pure, nomem, nostack, preserves_flags)
            );
        }
        d
    }

    /// `addss dst=b, src=a` -> `a + b`, `b` wins a NaN tie.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn add_r(a: f32, b: f32) -> f32 {
        add_l(b, a)
    }

    /// `mulss dst=a, src=b` -> `a * b`, `a` wins a NaN tie.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn mul_l(a: f32, b: f32) -> f32 {
        let mut d = a;
        unsafe {
            core::arch::asm!(
                "mulss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) b,
                options(pure, nomem, nostack, preserves_flags)
            );
        }
        d
    }

    /// `mulss dst=b, src=a` -> `a * b`, `b` wins a NaN tie.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn mul_r(a: f32, b: f32) -> f32 {
        mul_l(b, a)
    }

    /// `subss dst=a, src=b` -> `a - b`. Not commutative, so the destination is
    /// always the left operand; provided for symmetry and to stop LLVM from
    /// reassociating around it.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn sub(a: f32, b: f32) -> f32 {
        let mut d = a;
        unsafe {
            core::arch::asm!(
                "subss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) b,
                options(pure, nomem, nostack, preserves_flags)
            );
        }
        d
    }

    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn add_l(a: f32, b: f32) -> f32 {
        a + b
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn add_r(a: f32, b: f32) -> f32 {
        a + b
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn mul_l(a: f32, b: f32) -> f32 {
        a * b
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn mul_r(a: f32, b: f32) -> f32 {
        a * b
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn sub(a: f32, b: f32) -> f32 {
        a - b
    }
}

#[allow(unused_imports)]
use fx::{add_l, add_r, mul_l, mul_r, sub};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Layout-compatible with the C `c2Simplex { c2sv a, b, c, d; float div; int count; }`.
/// The C code takes `&s.a` and indexes it as an array, which the array field models.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Raw helpers that mirror C's unchecked array indexing (indices may be < 0 or
// >= count in the original code; reproduce the arithmetic rather than panic).
// ---------------------------------------------------------------------------

#[inline]
unsafe fn poly_vert(p: *const c2Poly, i: c_int) -> c2v {
    unsafe { *(&raw const (*p).verts).cast::<c2v>().offset(i as isize) }
}

#[inline]
unsafe fn poly_norm(p: *const c2Poly, i: c_int) -> c2v {
    unsafe { *(&raw const (*p).norms).cast::<c2v>().offset(i as isize) }
}

/// `c2GJK` indexes `pA.verts[iA]` with indices taken straight from a caller
/// supplied `c2GJKCache`, which the C never range-checks. Mirror the raw
/// pointer arithmetic instead of using a bounds-checked Rust index, so an
/// out-of-range cache index behaves like the C rather than panicking.
#[inline]
unsafe fn proxy_vert(p: *const c2Proxy, i: c_int) -> c2v {
    unsafe { *(&raw const (*p).verts).cast::<c2v>().offset(i as isize) }
}

// ---------------------------------------------------------------------------
// Vector / rotation / transform primitives
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    // C: `mulss -0xc(%rbp),%xmm0` with a.x/a.y in the destination.
    c2v {
        x: mul_l(a.x, b),
        y: mul_l(a.y, b),
    }
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
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // C: `mulss %xmm0,%xmm1` (a.x in dst), `mulss %xmm2,%xmm0` (b.y in dst),
    //    `addss %xmm1,%xmm0` (the a.y*b.y product in dst).
    add_r(mul_l(a.x, b.x), mul_r(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> f32 {
    c2Dot(h.n, p) - h.d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const c2Poly, i: c_int) -> c2h {
    unsafe {
        c2h {
            n: poly_norm(p, i),
            d: c2Dot(poly_norm(p, i), poly_vert(p, i)),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
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

/// Mirrors the C `c2MakeProxy`, which has no `C2_TYPE_POLY` case and therefore
/// leaves `*p` completely untouched for a poly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, ty: c_int, p: *mut c2Proxy) {
    unsafe {
        match ty {
            C2_TYPE_CIRCLE => {
                let c = shape as *const c2Circle;
                (*p).radius = (*c).r;
                (*p).count = 1;
                (*p).verts[0] = (*c).p;
            }
            C2_TYPE_AABB => {
                let bb = shape as *mut c2AABB;
                (*p).radius = 0.0;
                (*p).count = 4;
                c2BBVerts((*p).verts.as_mut_ptr(), bb);
            }
            C2_TYPE_CAPSULE => {
                let c = shape as *const c2Capsule;
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
    // C: `mulss %xmm1,%xmm0` / `mulss %xmm2,%xmm1` -> b.y and b.x in dst.
    mul_r(a.x, b.y) - mul_r(a.y, b.x)
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
            // C: `default:` and `case 1:` both return 0.
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // C: x = mulss(dst=b.x) - mulss(dst=b.y);
    //    y = addss(dst = a.s*b.x term) of mulss(dst=a.s) and mulss(dst=b.y).
    c2V(
        mul_r(a.c, b.x) - mul_r(a.s, b.y),
        add_l(mul_l(a.s, b.x), mul_r(a.c, b.y)),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // C: both components are `addss` with the LEFT product in the destination;
    // the left product keeps its own left operand, the right product its right.
    c2V(
        add_l(mul_l(a.c, b.x), mul_r(a.s, b.y)),
        add_l(mul_l(-a.s, b.x), mul_r(a.c, b.y)),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    // C: `addss %xmm1,%xmm0` with b.x / b.y in the destination.
    c2v {
        x: add_r(a.x, b.x),
        y: add_r(a.y, b.y),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
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
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    // Note: the C uses `x < 0 ? -x : x`, which returns -0.0 unchanged.
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

// ---------------------------------------------------------------------------
// Clipping helpers (`static` in C -> private here, not exported)
// ---------------------------------------------------------------------------

unsafe fn c2Clip(seg: *mut c2v, h: c2h) -> c_int {
    unsafe {
        // `out` is uninitialized in C; when fewer than 2 points are produced the
        // caller discards `seg`, so zeroing is equivalent for all observers.
        //
        // The C declares `c2v out[2]` but can push a THIRD element: with
        // `d0 < 0 && d1 < 0` whose product underflows to +0, the
        // `d0 * d1 <= 0` arm also fires, so `out[sp++]` writes out[2] past the
        // end of the array. Only `out[0]`/`out[1]` and the returned `sp` are
        // ever observed, so a 4-slot buffer reproduces the observable
        // behaviour without the out-of-bounds write.
        let mut out = [c2v::default(); 4];
        let mut sp: usize = 0;
        let d0 = c2Dist(h, *seg.offset(0));
        if d0 < 0.0 {
            out[sp] = *seg.offset(0);
            sp += 1;
        }
        let d1 = c2Dist(h, *seg.offset(1));
        if d1 < 0.0 {
            out[sp] = *seg.offset(1);
            sp += 1;
        }
        if d0 == 0.0 && d1 == 0.0 {
            out[sp] = *seg.offset(0);
            sp += 1;
            out[sp] = *seg.offset(1);
            sp += 1;
        } else if d0 * d1 <= 0.0 {
            out[sp] = c2Intersect(*seg.offset(0), *seg.offset(1), d0, d1);
            sp += 1;
        }
        *seg.offset(0) = out[0];
        *seg.offset(1) = out[1];
        sp as c_int
    }
}

unsafe fn c2SidePlanes(seg: *mut c2v, ra: c2v, rb: c2v, h: *mut c2h) -> c_int {
    unsafe {
        let inn = c2Norm(c2Sub(rb, ra));
        let left = c2h {
            n: c2Neg(inn),
            d: c2Dot(c2Neg(inn), ra),
        };
        let right = c2h {
            n: inn,
            d: c2Dot(inn, rb),
        };
        if c2Clip(seg, left) < 2 {
            return 0;
        }
        if c2Clip(seg, right) < 2 {
            return 0;
        }
        if !h.is_null() {
            (*h).n = c2CCW90(inn);
            (*h).d = c2Dot(c2CCW90(inn), ra);
        }
        1
    }
}

unsafe fn c2SidePlanesFromPoly(
    seg: *mut c2v,
    x: c2x,
    p: *const c2Poly,
    e: c_int,
    h: *mut c2h,
) -> c_int {
    unsafe {
        let ra = c2Mulxv(x, poly_vert(p, e));
        let nxt = if e + 1 == (*p).count { 0 } else { e + 1 };
        let rb = c2Mulxv(x, poly_vert(p, nxt));
        c2SidePlanes(seg, ra, rb, h)
    }
}

unsafe fn c2KeepDeep(seg: *mut c2v, h: c2h, m: *mut c2Manifold) {
    unsafe {
        let mut cp: usize = 0;
        for i in 0..2isize {
            let p = *seg.offset(i);
            let d = c2Dist(h, p);
            if d <= 0.0 {
                (*m).contact_points[cp] = p;
                (*m).depths[cp] = -d;
                cp += 1;
            }
        }
        (*m).count = cp as c_int;
        (*m).n = h.n;
    }
}

unsafe fn c2Incident(incident: *mut c2v, ip: *const c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    unsafe {
        let mut index: c_int = !0;
        let mut min_dot = FLT_MAX;
        let mut i: c_int = 0;
        while i < (*ip).count {
            let dot = c2Dot(rn_in_incident_space, poly_norm(ip, i));
            if dot < min_dot {
                min_dot = dot;
                index = i;
            }
            i += 1;
        }
        *incident.offset(0) = c2Mulxv(ix, poly_vert(ip, index));
        let nxt = if index + 1 == (*ip).count { 0 } else { index + 1 };
        *incident.offset(1) = c2Mulxv(ix, poly_vert(ip, nxt));
    }
}

// ---------------------------------------------------------------------------
// Simplex solvers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let s = &mut *s;
        let a = s.verts[0].p;
        let b = s.verts[1].p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if u <= 0.0 {
            s.verts[0] = s.verts[1];
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else {
            s.verts[0].u = u;
            s.verts[1].u = v;
            // C: `movss u,%xmm0; addss v,%xmm0` -> u in the destination.
            s.div = add_l(u, v);
            s.count = 2;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let s = &mut *s;
        let a = s.verts[0].p;
        let b = s.verts[1].p;
        let c = s.verts[2].p;
        let uAB = c2Dot(b, c2Sub(b, a));
        let vAB = c2Dot(a, c2Sub(a, b));
        let uBC = c2Dot(c, c2Sub(c, b));
        let vBC = c2Dot(b, c2Sub(b, c));
        let uCA = c2Dot(a, c2Sub(a, c));
        let vCA = c2Dot(c, c2Sub(c, a));
        let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
        // C: `mulss %xmm1,%xmm0` with the c2Det2 result in the destination.
        let uABC = mul_l(c2Det2(b, c), area);
        let vABC = mul_l(c2Det2(c, a), area);
        let wABC = mul_l(c2Det2(a, b), area);
        if vAB <= 0.0 && uCA <= 0.0 {
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            s.verts[0] = s.verts[1];
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            s.verts[0] = s.verts[2];
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            s.verts[0].u = uAB;
            s.verts[1].u = vAB;
            s.div = add_l(uAB, vAB);
            s.count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            s.verts[0] = s.verts[1];
            s.verts[1] = s.verts[2];
            s.verts[0].u = uBC;
            s.verts[1].u = vBC;
            s.div = add_l(uBC, vBC);
            s.count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            s.verts[1] = s.verts[0];
            s.verts[0] = s.verts[2];
            s.verts[0].u = uCA;
            s.verts[1].u = vCA;
            s.div = add_l(uCA, vCA);
            s.count = 2;
        } else {
            s.verts[0].u = uABC;
            s.verts[1].u = vABC;
            s.verts[2].u = wABC;
            s.div = add_l(add_l(uABC, vABC), wABC);
            s.count = 3;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        match (*s).count {
            1 => c2Neg((*s).verts[0].p),
            2 => {
                let ab = c2Sub((*s).verts[1].p, (*s).verts[0].p);
                if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {
                    c2Skew(ab)
                } else {
                    c2CCW90(ab)
                }
            }
            // C: `case 3:` and `default:` both return (0, 0).
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
        let s = &*s;
        let den = 1.0f32 / s.div;
        match s.count {
            1 => {
                *a = s.verts[0].sA;
                *b = s.verts[0].sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs(s.verts[0].sA, mul_r(den, s.verts[0].u)),
                    c2Mulvs(s.verts[1].sA, mul_r(den, s.verts[1].u)),
                );
                *b = c2Add(
                    c2Mulvs(s.verts[0].sB, mul_r(den, s.verts[0].u)),
                    c2Mulvs(s.verts[1].sB, mul_r(den, s.verts[1].u)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs(s.verts[0].sA, mul_r(den, s.verts[0].u)),
                        c2Mulvs(s.verts[1].sA, mul_r(den, s.verts[1].u)),
                    ),
                    c2Mulvs(s.verts[2].sA, mul_r(den, s.verts[2].u)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs(s.verts[0].sB, mul_r(den, s.verts[0].u)),
                        c2Mulvs(s.verts[1].sB, mul_r(den, s.verts[1].u)),
                    ),
                    c2Mulvs(s.verts[2].sB, mul_r(den, s.verts[2].u)),
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
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let s = &*s;
        let den = 1.0f32 / s.div;
        match s.count {
            1 => s.verts[0].p,
            2 => c2Add(
                c2Mulvs(s.verts[0].p, mul_r(den, s.verts[0].u)),
                c2Mulvs(s.verts[1].p, mul_r(den, s.verts[1].u)),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

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
        let ax: c2x = if ax_ptr.is_null() {
            c2xIdentity()
        } else {
            *ax_ptr
        };
        let bx: c2x = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            *bx_ptr
        };

        // The C locals are uninitialized; the poly case of c2MakeProxy writes
        // nothing and in practice reads back as zeros (fresh stack pages).
        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);

        let mut s = c2Simplex::default();
        let verts: *mut c2sv = s.verts.as_mut_ptr();

        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                let mut i: c_int = 0;
                while i < (*cache).count {
                    let iA = *(&raw const (*cache).iA).cast::<c_int>().offset(i as isize);
                    let iB = *(&raw const (*cache).iB).cast::<c_int>().offset(i as isize);
                    let sA = c2Mulxv(ax, proxy_vert(&pA, iA));
                    let sB = c2Mulxv(bx, proxy_vert(&pB, iB));
                    let v = &mut *verts.offset(i as isize);
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
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        // C: `int saveA[3], saveB[3];` -- two adjacent 3-int stack arrays,
        // written with `saveA[i] = ...` for `i < s.count`. A caller-forged
        // `c2GJKCache` with `count > 3` makes the C write past `saveA[2]` into
        // whatever follows. Keeping them adjacent in one `#[repr(C)]` struct
        // reproduces that as a write into `saveB` instead of an out-of-bounds
        // Rust index (which would abort under `panic = "abort"`).
        #[repr(C)]
        struct SaveIdx {
            a: [c_int; 3],
            b: [c_int; 3],
        }
        let mut save = SaveIdx { a: [0; 3], b: [0; 3] };
        let save_a: *mut c_int = save.a.as_mut_ptr();
        let save_b: *mut c_int = save.b.as_mut_ptr();
        let mut save_count: c_int;
        let mut d0 = FLT_MAX;
        let mut d1;
        let mut iter: c_int = 0;
        let mut hit = 0;

        while iter < 20 {
            save_count = s.count;
            let mut i: c_int = 0;
            while i < save_count {
                *save_a.offset(i as isize) = (*verts.offset(i as isize)).iA;
                *save_b.offset(i as isize) = (*verts.offset(i as isize)).iB;
                i += 1;
            }

            match s.count {
                1 => {}
                2 => c22(&mut s),
                3 => c23(&mut s),
                _ => {}
            }

            if s.count == 3 {
                hit = 1;
                break;
            }

            let p = c2L(&mut s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;

            let d = c2D(&mut s);
            if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
                break;
            }

            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, proxy_vert(&pA, iA));
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, proxy_vert(&pB, iB));

            {
                let v = &mut *verts.offset(s.count as isize);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
            }

            let mut dup = 0;
            let mut i: c_int = 0;
            while i < save_count {
                if iA == *save_a.offset(i as isize) && iB == *save_b.offset(i as isize) {
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

        let mut a = c2v::default();
        let mut b = c2v::default();
        c2Witness(&mut s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));

        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            // C: `movss rA,%xmm0; movaps %xmm0,%xmm1; addss rB,%xmm1`.
            if dist > add_l(rA, rB) && dist > FLT_EPSILON {
                dist -= add_l(rA, rB);
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
            let mut i: c_int = 0;
            while i < s.count {
                let v = &*verts.offset(i as isize);
                *(&raw mut (*cache).iA).cast::<c_int>().offset(i as isize) = v.iA;
                *(&raw mut (*cache).iB).cast::<c_int>().offset(i as isize) = v.iB;
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
// Manifold generation
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let d = c2Sub(B.p, A.p);
        let d2 = c2Dot(d, d);
        let r = add_r(A.r, B.r);
        if d2 < r * r {
            let l = d2.sqrt();
            let n = if l != 0.0 {
                c2Mulvs(d, 1.0f32 / l)
            } else {
                c2V(0.0, 1.0)
            };
            (*m).count = 1;
            (*m).depths[0] = r - l;
            (*m).contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let L = c2Clampv(A.p, B.min, B.max);
        let ab = c2Sub(L, A.p);
        let d2 = c2Dot(ab, ab);
        let r2 = A.r * A.r;
        if d2 < r2 {
            if d2 != 0.0 {
                let d = d2.sqrt();
                let n = c2Norm(ab);
                (*m).count = 1;
                (*m).depths[0] = A.r - d;
                (*m).contact_points[0] = c2Add(A.p, c2Mulvs(n, d));
                (*m).n = n;
            } else {
                let mid = c2Mulvs(c2Add(B.min, B.max), 0.5);
                let e = c2Mulvs(c2Sub(B.max, B.min), 0.5);
                let d = c2Sub(A.p, mid);
                let abs_d = c2Absv(d);
                let x_overlap = e.x - abs_d.x;
                let y_overlap = e.y - abs_d.y;
                let depth;
                let mut n;
                if x_overlap < y_overlap {
                    depth = x_overlap;
                    n = c2V(1.0, 0.0);
                    n = c2Mulvs(n, if d.x < 0.0 { 1.0 } else { -1.0 });
                } else {
                    depth = y_overlap;
                    n = c2V(0.0, 1.0);
                    n = c2Mulvs(n, if d.y < 0.0 { 1.0 } else { -1.0 });
                }
                (*m).count = 1;
                // C: `movss A.r,%xmm0; addss depth,%xmm0`.
                (*m).depths[0] = add_l(A.r, depth);
                (*m).contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
                (*m).n = n;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let r = add_r(A.r, B.r);
        let d = c2GJK(
            (&raw const A).cast(),
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            (&raw const B).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if d < r {
            let n = if d == 0.0 {
                c2Norm(c2Skew(c2Sub(B.b, B.a)))
            } else {
                c2Norm(c2Sub(b, a))
            };
            (*m).count = 1;
            (*m).depths[0] = r - d;
            (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mid_a = c2Mulvs(c2Add(A.min, A.max), 0.5);
        let mid_b = c2Mulvs(c2Add(B.min, B.max), 0.5);
        let eA = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5));
        let eB = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5));
        let d = c2Sub(mid_b, mid_a);
        // C: `movss eA.x,%xmm1; addss eB.x,%xmm0 -> dst=xmm1`, then `subss`.
        let dx = add_l(eA.x, eB.x) - (if d.x < 0.0 { -d.x } else { d.x });
        if dx < 0.0 {
            return;
        }
        let dy = add_l(eA.y, eB.y) - (if d.y < 0.0 { -d.y } else { d.y });
        if dy < 0.0 {
            return;
        }
        let n;
        let depth;
        let p;
        if dx < dy {
            depth = dx;
            if d.x < 0.0 {
                n = c2V(-1.0, 0.0);
                p = c2Sub(mid_a, c2V(eA.x, 0.0));
            } else {
                n = c2V(1.0, 0.0);
                p = c2Add(mid_a, c2V(eA.x, 0.0));
            }
        } else {
            depth = dy;
            if d.y < 0.0 {
                n = c2V(0.0, -1.0);
                p = c2Sub(mid_a, c2V(0.0, eA.y));
            } else {
                n = c2V(0.0, 1.0);
                p = c2Add(mid_a, c2V(0.0, eA.y));
            }
        }
        (*m).count = 1;
        (*m).contact_points[0] = p;
        (*m).depths[0] = depth;
        (*m).n = n;
    }
}

// `sep` is written but never re-read after the last comparison, exactly as in
// the C source; the assignment is kept for fidelity.
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    A: c2Capsule,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let d = c2GJK(
            (&raw const A).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            B.cast(),
            C2_TYPE_POLY,
            bx_ptr,
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if d < 1.0e-6f32 {
            let bx = if !bx_ptr.is_null() {
                *bx_ptr
            } else {
                c2xIdentity()
            };
            let mut A_in_B = c2Capsule::default();
            A_in_B.a = c2MulxvT(bx, A.a);
            A_in_B.b = c2MulxvT(bx, A.b);
            let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));

            let mut ab_h0 = c2h::default();
            ab_h0.n = c2CCW90(ab);
            ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
            let v0 = c2Support((&raw const (*B).verts).cast(), (*B).count, c2Neg(ab_h0.n));
            let s0 = c2Dist(ab_h0, poly_vert(B, v0));

            let mut ab_h1 = c2h::default();
            ab_h1.n = c2Skew(ab);
            ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
            let v1 = c2Support((&raw const (*B).verts).cast(), (*B).count, c2Neg(ab_h1.n));
            let s1 = c2Dist(ab_h1, poly_vert(B, v1));

            let mut index: c_int = !0;
            let mut sep = -FLT_MAX;
            let mut code: c_int = 0;
            let mut i: c_int = 0;
            while i < (*B).count {
                let h = c2PlaneAt(B, i);
                let da = c2Dot(A_in_B.a, c2Neg(h.n));
                let db = c2Dot(A_in_B.b, c2Neg(h.n));
                let dd = if da > db {
                    c2Dist(h, A_in_B.a)
                } else {
                    c2Dist(h, A_in_B.b)
                };
                if dd > sep {
                    sep = dd;
                    index = i;
                }
                i += 1;
            }
            if s0 > sep {
                sep = s0;
                index = v0;
                code = 1;
            }
            if s1 > sep {
                sep = s1;
                index = v1;
                code = 2;
            }

            match code {
                0 => {
                    let mut seg: [c2v; 2] = [A.a, A.b];
                    let mut h = c2h::default();
                    if c2SidePlanesFromPoly(seg.as_mut_ptr(), bx, B, index, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(seg.as_mut_ptr(), h, m);
                    (*m).n = c2Neg((*m).n);
                }
                1 => {
                    let mut incident = [c2v::default(); 2];
                    c2Incident(incident.as_mut_ptr(), B, bx, ab_h0.n);
                    let mut h = c2h::default();
                    if c2SidePlanes(incident.as_mut_ptr(), A_in_B.b, A_in_B.a, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(incident.as_mut_ptr(), h, m);
                }
                2 => {
                    let mut incident = [c2v::default(); 2];
                    c2Incident(incident.as_mut_ptr(), B, bx, ab_h1.n);
                    let mut h = c2h::default();
                    if c2SidePlanes(incident.as_mut_ptr(), A_in_B.a, A_in_B.b, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(incident.as_mut_ptr(), h, m);
                }
                _ => return,
            }

            let mut i: c_int = 0;
            while i < (*m).count {
                // C: `movss depths[i],%xmm1; movss A.r,%xmm0; addss %xmm1,%xmm0`
                // -> A.r sits in the destination.
                (*m).depths[i as usize] = add_r((*m).depths[i as usize], A.r);
                i += 1;
            }
        } else if d < A.r {
            (*m).count = 1;
            (*m).n = c2Norm(c2Sub(b, a));
            (*m).contact_points[0] = c2Add(a, c2Mulvs((*m).n, A.r));
            (*m).depths[0] = A.r - d;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut c2v, norms: *mut c2v, count: c_int) {
    unsafe {
        let mut i: c_int = 0;
        while i < count {
            let a = i;
            let b = if i + 1 < count { i + 1 } else { 0 };
            let e = c2Sub(*verts.offset(b as isize), *verts.offset(a as isize));
            *norms.offset(i as isize) = c2Norm(c2CCW90(e));
            i += 1;
        }
    }
}

/// Mirrors the stack layout gcc gives `c2AABBtoCapsuleManifold`.
///
/// The C builds a local `c2Poly p` and hands it to `c2CapsuletoPolyManifold`.
/// If the AABB is degenerate (`min == max`) every `p.norms[i]` becomes `NaN`,
/// so `c2Incident`'s `dot < min_dot` test is never true and its `index` stays
/// `~0 == -1`; the C then evaluates `ip->verts[-1]`, reading the 8 bytes
/// *below* `p.verts`. In gcc's frame (`p` at `rbp-0xa0`, the by-value `c2AABB A`
/// at `rbp-0xb0`) those are `A.max.y` followed by `p.count`.
///
/// Placing an explicit `f32` in front of the poly and seeding it with `A.max.y`
/// reproduces that read exactly. Verified against the C `.so`.
#[repr(C)]
struct AabbCapsulePolyFrame {
    /// Occupies the slot the C reads as `verts[-1].x`.
    before_verts: f32,
    poly: c2Poly,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut frame = AabbCapsulePolyFrame {
            before_verts: A.max.y,
            poly: c2Poly::default(),
        };
        let p = &mut frame.poly;
        let mut aabb = A;
        c2BBVerts(p.verts.as_mut_ptr(), &mut aabb);
        p.count = 4;
        c2Norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), 4);
        c2CapsuletoPolyManifold(B, p, std::ptr::null(), m);
        // Note: runs unconditionally, so it negates whatever `m->n` holds even
        // when no manifold was produced -- as in the C.
        (*m).n = c2Neg((*m).n);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(A: c2Capsule, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let r = add_r(A.r, B.r);
        let d = c2GJK(
            (&raw const A).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            (&raw const B).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if d < r {
            let n = if d == 0.0 {
                c2Norm(c2Skew(c2Sub(A.b, A.a)))
            } else {
                c2Norm(c2Sub(b, a))
            };
            (*m).count = 1;
            (*m).depths[0] = r - d;
            (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCircleManifold(
                        *(A as *const c2Circle),
                        *(B as *const c2Circle),
                        m,
                    );
                }
                C2_TYPE_AABB => {
                    c2CircletoAABBManifold(*(A as *const c2Circle), *(B as *const c2AABB), m);
                }
                C2_TYPE_CAPSULE => {
                    c2CircletoCapsuleManifold(
                        *(A as *const c2Circle),
                        *(B as *const c2Capsule),
                        m,
                    );
                }
                _ => {}
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoAABBManifold(*(B as *const c2Circle), *(A as *const c2AABB), m);
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_AABB => {
                    c2AABBtoAABBManifold(*(A as *const c2AABB), *(B as *const c2AABB), m);
                }
                C2_TYPE_CAPSULE => {
                    c2AABBtoCapsuleManifold(*(A as *const c2AABB), *(B as *const c2Capsule), m);
                }
                _ => {}
            },
            C2_TYPE_CAPSULE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCapsuleManifold(
                        *(B as *const c2Circle),
                        *(A as *const c2Capsule),
                        m,
                    );
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_AABB => {
                    c2AABBtoCapsuleManifold(*(B as *const c2AABB), *(A as *const c2Capsule), m);
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_CAPSULE => {
                    c2CapsuletoCapsuleManifold(
                        *(A as *const c2Capsule),
                        *(B as *const c2Capsule),
                        m,
                    );
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// Allocates a shape from loose floats. As in the C, the `C2_TYPE_POLY` case
/// falls off the end of the function without a `return`; a null pointer is
/// produced here (`c2Collide` has no poly arm, so it is never dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: c_int,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
) -> *mut c_void {
    unsafe {
        match typ {
            C2_TYPE_CIRCLE => {
                let circle = malloc(std::mem::size_of::<c2Circle>()) as *mut c2Circle;
                (*circle).p = c2V(a, b);
                (*circle).r = c;
                circle as *mut c_void
            }
            C2_TYPE_AABB => {
                let aabb = malloc(std::mem::size_of::<c2AABB>()) as *mut c2AABB;
                (*aabb).min = c2V(a, b);
                (*aabb).max = c2V(c, d);
                aabb as *mut c_void
            }
            C2_TYPE_CAPSULE => {
                let capsule = malloc(std::mem::size_of::<c2Capsule>()) as *mut c2Capsule;
                (*capsule).a = c2V(a, b);
                (*capsule).b = c2V(c, d);
                (*capsule).r = e;
                capsule as *mut c_void
            }
            _ => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_manifold(
    m: *mut c2Manifold,
    type_a: c_int,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: c_int,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    unsafe {
        // Deliberately leaked, matching the C (no free).
        let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
        let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
        c2Collide(A, type_a, B, type_b, m);
    }
}
