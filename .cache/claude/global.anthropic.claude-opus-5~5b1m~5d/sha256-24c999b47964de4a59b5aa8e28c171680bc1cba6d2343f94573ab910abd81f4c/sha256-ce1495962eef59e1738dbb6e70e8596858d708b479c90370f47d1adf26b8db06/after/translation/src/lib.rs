//! Rust translation of `c_src/src/lib.c` (tinyc2-style 2D GJK routines).
//!
//! The C library is compiled by CMake as one shared object that exports every
//! non-static function in `src/lib.c`.  This crate reproduces that exact ABI:
//! same symbol names, same C signatures, same struct layouts, and bit-for-bit
//! identical floating point results.
//!
//! # Bit-exactness
//!
//! All arithmetic is plain IEEE-754 single precision: no fused multiply-add, no
//! reassociation, no `-ffast-math`.  For *finite* inputs `a + b` in Rust and
//! `a + b` in C are therefore already identical.
//!
//! They are **not** automatically identical when both operands of a commutative
//! SSE operation are NaN: `addss`/`mulss` return the *destination* operand's
//! NaN payload, and which C operand ends up in the destination register is a
//! choice the compiler makes.  gcc at `-O0` does not consistently pick the
//! left-hand operand — e.g. in `c2Dot` it computes `a.x*b.x` with `a.x` as the
//! destination but `a.y*b.y` with `b.y` as the destination, and then adds with
//! the *second* product as the destination.  LLVM makes its own (different)
//! choices and freely commutes `fadd`/`fmul`.
//!
//! To be byte-identical for every input, including NaN, this file does not use
//! `+`/`-`/`*`/`/` on `f32` at all.  It uses [`fp::add`], [`fp::sub`],
//! [`fp::mul`] and [`fp::div`], which are single SSE instructions with the
//! destination operand pinned by inline assembly, and each call site names the
//! destination gcc chose (read off `objdump -d` of the C `.so`).
//!
//! Bugs / quirks in the original C are preserved verbatim, including:
//!   * the inverted cache-validity test in `c2GJK`
//!   * `gjk_cache` never writing through its `a9` / `b9` out-parameters
//!   * NaN-propagating min/max ordering in `c2Maxv` / `c2Minv`
//!   * `c2MakeProxy` writing nothing at all for an out-of-range `C2_TYPE`

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// Operand-order-pinned IEEE-754 single-precision primitives.
//
// `fp::OP(d, s)` computes `d OP s` with `d` in the destination register, i.e.
// exactly `OPss d, s` on x86-64.  That fixes NaN-payload selection (the
// destination wins when both operands are NaN) and forbids the optimiser from
// commuting the operands.
// ---------------------------------------------------------------------------

mod fp {
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn add(mut d: f32, s: f32) -> f32 {
        unsafe {
            core::arch::asm!(
                "addss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) s,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        d
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn sub(mut d: f32, s: f32) -> f32 {
        unsafe {
            core::arch::asm!(
                "subss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) s,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        d
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn mul(mut d: f32, s: f32) -> f32 {
        unsafe {
            core::arch::asm!(
                "mulss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) s,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        d
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn div(mut d: f32, s: f32) -> f32 {
        unsafe {
            core::arch::asm!(
                "divss {d}, {s}",
                d = inout(xmm_reg) d,
                s = in(xmm_reg) s,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        d
    }

    /// `sqrtf`.  Correctly rounded per IEEE-754, so a single `sqrtss` is
    /// bit-identical to glibc's `sqrtf` for every input (including NaN payload
    /// propagation, which `sqrtss` quiets exactly like the libm routine).
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn sqrt(mut d: f32) -> f32 {
        unsafe {
            core::arch::asm!(
                "sqrtss {d}, {d}",
                d = inout(xmm_reg) d,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        d
    }

    // Portable fallbacks.  Operand order cannot be pinned without target
    // specific assembly, so on other architectures the NaN-payload tie-break
    // follows whatever the platform does; every other input is still exact.
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn add(d: f32, s: f32) -> f32 {
        d + s
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn sub(d: f32, s: f32) -> f32 {
        d - s
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn mul(d: f32, s: f32) -> f32 {
        d * s
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn div(d: f32, s: f32) -> f32 {
        d / s
    }
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    pub fn sqrt(d: f32) -> f32 {
        d.sqrt()
    }

    /// Sign flip.  gcc emits `xorps` with a sign mask, which flips the sign bit
    /// of every value including NaNs; Rust's unary `-` does the same.
    #[inline(always)]
    pub fn neg(x: f32) -> f32 {
        f32::from_bits(x.to_bits() ^ 0x8000_0000)
    }
}

// ---------------------------------------------------------------------------
// C2_TYPE (C enum -> unsigned int on the SysV x86-64 ABI)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;
pub const C2_TYPE_CAPSULE: c_uint = 2;

// ---------------------------------------------------------------------------
// Public / internal structs.  Layouts verified against gcc 11.5 / x86-64 SysV:
//   c2v = 8, c2r = 8, c2x = 16, c2Circle = 12, c2AABB = 16, c2Capsule = 20,
//   c2GJKCache = 36 (metric@0 count@4 iA@8 iB@20 div@32),
//   c2Proxy = 72 (radius@0 count@4 verts@8),
//   c2sv = 36 (sA@0 sB@8 p@16 u@24 iA@28 iB@32),
//   c2Simplex = 152 (a@0 b@36 c@72 d@108 div@144 count@148)
// ---------------------------------------------------------------------------

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

/// The C declaration is `c2sv a, b, c, d;` -- four consecutive `c2sv` values,
/// which `c2GJK` walks with `c2sv *verts = &s.a;`.  The array below is
/// layout-identical.
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

// The three float constants the C source spells out, cross-checked against the
// `.rodata` of the compiled C `.so`:
//   0x4024 = 0x7f7fffff  FLT_MAX
//   0x402c = 0x34000000  FLT_EPSILON            (2^-23)
//   0x4028 = 0x28800000  FLT_EPSILON*FLT_EPSILON (2^-46, constant-folded by gcc)
//   0x4020 = 0xccbebc20  -1.0e8f
const C2_FLT_MAX: f32 = f32::MAX;
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;
const C2_FLT_EPSILON_SQ: f32 = C2_FLT_EPSILON * C2_FLT_EPSILON;

// Compile-time proof that the constants above have the exact bit patterns gcc
// baked into the C shared object.
const _: () = {
    assert!(C2_FLT_MAX.to_bits() == 0x7f7f_ffff);
    assert!(C2_FLT_EPSILON.to_bits() == 0x3400_0000);
    assert!(C2_FLT_EPSILON_SQ.to_bits() == 0x2880_0000);
    assert!((-1.0e8f32).to_bits() == 0xccbe_bc20);
    assert!((0.5f32).to_bits() == 0x3f00_0000);
    assert!((15.0f32).to_bits() == 0x4170_0000);
    assert!((100.0f32).to_bits() == 0x42c8_0000);
    assert!((-25.0f32).to_bits() == 0xc1c8_0000);
    assert!((75.0f32).to_bits() == 0x4296_0000);
    assert!((10.0f32).to_bits() == 0x4120_0000);
};

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = ZERO_V;
    a.x = x;
    a.y = y;
    a
}

/// C: `a.x *= b; a.y *= b;`
/// gcc: `mulss` with the vector component as the destination.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x = fp::mul(a.x, b);
    a.y = fp::mul(a.y, b);
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

/// C: `a.x -= b.x; a.y -= b.y;`  gcc: `subss` with `a` as the destination.
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = fp::sub(a.x, b.x);
    a.y = fp::sub(a.y, b.y);
    a
}

/// C: `a.x * b.x + a.y * b.y`
/// gcc: `mulss a.x, b.x` / `mulss b.y, a.y` / `addss (b.y*a.y), (a.x*b.x)`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    fp::add(fp::mul(b.y, a.y), fp::mul(a.x, b.x))
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

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    *out.offset(0) = (*bb).min;
    *out.offset(1) = c2V((*bb).max.x, (*bb).min.y);
    *out.offset(2) = (*bb).max;
    *out.offset(3) = c2V((*bb).min.x, (*bb).max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, r#type: c_uint, p: *mut c2Proxy) {
    match r#type {
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
            c2BBVerts((*p).verts.as_mut_ptr(), bb);
        }
        C2_TYPE_CAPSULE => {
            let c = shape as *mut c2Capsule;
            (*p).radius = (*c).r;
            (*p).count = 2;
            (*p).verts[0] = (*c).a;
            (*p).verts[1] = (*c).b;
        }
        // No `default:` label in the C switch -- nothing is written, and `p` is
        // not even dereferenced.
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    fp::sqrt(c2Dot(a, a))
}

/// C: `a.x * b.y - a.y * b.x`
/// gcc: `mulss b.y, a.x` / `mulss b.x, a.y` / `subss`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    fp::sub(fp::mul(b.y, a.x), fp::mul(b.x, a.y))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
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

/// C: `c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)`
/// gcc: x = `subss (b.x*a.c), (b.y*a.s)`, y = `addss (a.s*b.x), (b.y*a.c)`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(
        fp::sub(fp::mul(b.x, a.c), fp::mul(b.y, a.s)),
        fp::add(fp::mul(a.s, b.x), fp::mul(b.y, a.c)),
    )
}

/// C: `a.x += b.x; a.y += b.y;`
/// gcc: `addss` with **`b`** as the destination register.
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = fp::add(b.x, a.x);
    a.y = fp::add(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
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
        (*s).div = fp::add(u, v);
        (*s).count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
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
    // gcc: `mulss det, area` -- the determinant is the destination.
    let uABC = fp::mul(c2Det2(b, c), area);
    let vABC = fp::mul(c2Det2(c, a), area);
    let wABC = fp::mul(c2Det2(a, b), area);
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
        (*s).div = fp::add(uAB, vAB);
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        (*s).verts[0] = (*s).verts[1];
        (*s).verts[1] = (*s).verts[2];
        (*s).verts[0].u = uBC;
        (*s).verts[1].u = vBC;
        (*s).div = fp::add(uBC, vBC);
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        (*s).verts[1] = (*s).verts[0];
        (*s).verts[0] = (*s).verts[2];
        (*s).verts[0].u = uCA;
        (*s).verts[1].u = vCA;
        (*s).div = fp::add(uCA, vCA);
        (*s).count = 2;
    } else {
        (*s).verts[0].u = uABC;
        (*s).verts[1].u = vABC;
        (*s).verts[2].u = wABC;
        (*s).div = fp::add(fp::add(uABC, vABC), wABC);
        (*s).count = 3;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(fp::neg(a.x), fp::neg(a.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = ZERO_V;
    b.x = fp::neg(a.y);
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = ZERO_V;
    b.x = a.y;
    b.y = fp::neg(a.x);
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    match (*s).count {
        1 => c2Neg((*s).verts[0].p),
        2 => {
            let ab = c2Sub((*s).verts[1].p, (*s).verts[0].p);
            if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {
                return c2Skew(ab);
            }
            c2CCW90(ab)
        }
        // case 3 and default
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    // The C loads `verts[0]` *unconditionally*, before it looks at `count`
    // (line 300 of lib.c), so `c2Support(NULL, 0, d)` faults.  `dmax` is dead
    // when `count <= 1`, and LLVM happily deletes a plain load in that case --
    // which would turn a segfault into a quiet `return 0`.  A volatile read
    // pins the access down so the two libraries fault identically.
    let mut dmax = c2Dot(ptr::read_volatile(verts), d);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    // gcc: `divss 1.0f, s->div` (the literal is the destination).
    let den = fp::div(1.0f32, (*s).div);
    match (*s).count {
        1 => {
            *a = (*s).verts[0].sA;
            *b = (*s).verts[0].sB;
        }
        2 => {
            // gcc: `mulss u, den` -- `u` is the destination.
            *a = c2Add(
                c2Mulvs((*s).verts[0].sA, fp::mul((*s).verts[0].u, den)),
                c2Mulvs((*s).verts[1].sA, fp::mul((*s).verts[1].u, den)),
            );
            *b = c2Add(
                c2Mulvs((*s).verts[0].sB, fp::mul((*s).verts[0].u, den)),
                c2Mulvs((*s).verts[1].sB, fp::mul((*s).verts[1].u, den)),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*s).verts[0].sA, fp::mul((*s).verts[0].u, den)),
                    c2Mulvs((*s).verts[1].sA, fp::mul((*s).verts[1].u, den)),
                ),
                c2Mulvs((*s).verts[2].sA, fp::mul((*s).verts[2].u, den)),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*s).verts[0].sB, fp::mul((*s).verts[0].u, den)),
                    c2Mulvs((*s).verts[1].sB, fp::mul((*s).verts[1].u, den)),
                ),
                c2Mulvs((*s).verts[2].sB, fp::mul((*s).verts[2].u, den)),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, fp::div(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let den = fp::div(1.0f32, (*s).div);
    match (*s).count {
        1 => (*s).verts[0].p,
        2 => c2Add(
            c2Mulvs((*s).verts[0].p, fp::mul((*s).verts[0].u, den)),
            c2Mulvs((*s).verts[1].p, fp::mul((*s).verts[1].u, den)),
        ),
        _ => c2V(0.0, 0.0),
    }
}

/// C: `c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)`
/// gcc: x = `addss (a.c*b.x), (b.y*a.s)`,
///      y = `addss ((-a.s)*b.x), (b.y*a.c)` with the `xorps` sign flip applied
///      to `a.s` *before* the multiply -- that ordering is observable through
///      NaN sign bits, so it must not be folded into a subtraction.
#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(
        fp::add(fp::mul(a.c, b.x), fp::mul(b.y, a.s)),
        fp::add(fp::mul(fp::neg(a.s), b.x), fp::mul(b.y, a.c)),
    )
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

// `save_count`'s and `d1`'s initial values are dead stores in the C as well
// (`int save_count = 0;` / `float d1 = FLT_MAX;`); they are kept so that the
// declarations line up one-for-one with the original.
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_uint,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: c_uint,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
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

    // `c2Proxy pA; c2Proxy pB;` are uninitialised automatics in the C.  For
    // every in-range `C2_TYPE` `c2MakeProxy` writes `radius`, `count` and all
    // the vertices the code goes on to read, so zero-filling here is
    // behaviourally identical.  (For an out-of-range type the C reads
    // indeterminate stack -- undefined behaviour that cannot be reproduced.)
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
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);

    let mut simplex = c2Simplex {
        verts: [ZERO_SV; 4],
        div: 0.0,
        count: 0,
    };
    let s: *mut c2Simplex = &raw mut simplex;
    let verts: *mut c2sv = (*s).verts.as_mut_ptr();

    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_was_good = (*cache).count != 0;
        if cache_was_good {
            let mut i: c_int = 0;
            while i < (*cache).count {
                let iA = *(*cache).iA.as_ptr().offset(i as isize);
                let iB = *(*cache).iB.as_ptr().offset(i as isize);
                let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
                let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
                let v: *mut c2sv = verts.offset(i as isize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
                i += 1;
            }
            (*s).count = (*cache).count;
            (*s).div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            // gcc turns `max_metric * 2.0f` into `addss m, m`.
            if !(min_metric < fp::add(max_metric, max_metric) && metric < -1.0e8f32) {
                cache_was_read = 1;
            }
        }
    }

    if cache_was_read == 0 {
        (*verts.offset(0)).iA = 0;
        (*verts.offset(0)).iB = 0;
        (*verts.offset(0)).sA = c2Mulxv(ax, pA.verts[0]);
        (*verts.offset(0)).sB = c2Mulxv(bx, pB.verts[0]);
        (*verts.offset(0)).p = c2Sub((*verts.offset(0)).sB, (*verts.offset(0)).sA);
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    }

    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int = 0;
    let mut d0: f32 = C2_FLT_MAX;
    let mut d1: f32 = C2_FLT_MAX;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = (*s).count;
        let mut i: c_int = 0;
        while i < save_count {
            *saveA.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iA;
            *saveB.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iB;
            i += 1;
        }

        match (*s).count {
            1 => {}
            2 => c22(s),
            3 => c23(s),
            // The C switch has no `default:` label.
            _ => {}
        }

        if (*s).count == 3 {
            hit = 1;
            break;
        }

        let p = c2L(s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2D(s);
        if c2Dot(d, d) < C2_FLT_EPSILON_SQ {
            break;
        }

        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
        let v: *mut c2sv = verts.offset((*s).count as isize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);

        let mut dup: c_int = 0;
        let mut i: c_int = 0;
        while i < save_count {
            if iA == *saveA.as_ptr().offset(i as isize) && iB == *saveB.as_ptr().offset(i as isize) {
                dup = 1;
                break;
            }
            i += 1;
        }
        if dup != 0 {
            break;
        }

        (*s).count += 1;
        iter += 1;
    }

    let mut a = ZERO_V;
    let mut b = ZERO_V;
    c2Witness(s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        // gcc: `addss rA, rB` (rA is the destination).
        if dist > fp::add(rA, rB) && dist > C2_FLT_EPSILON {
            dist = fp::sub(dist, fp::add(rA, rB));
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
        (*cache).metric = c2GJKSimplexMetric(s);
        (*cache).count = (*s).count;
        let mut i: c_int = 0;
        while i < (*s).count {
            let v: *mut c2sv = verts.offset(i as isize);
            *(*cache).iA.as_mut_ptr().offset(i as isize) = (*v).iA;
            *(*cache).iB.as_mut_ptr().offset(i as isize) = (*v).iB;
            i += 1;
        }
        (*cache).div = (*s).div;
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

// ---------------------------------------------------------------------------
// Public entry point declared in include/lib.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk_cache(
    reverse: c_char,
    a9: *mut c2v,
    b9: *mut c2v,
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
    // `cache` is an uninitialised automatic in the C; only `count` is set, and
    // because `count == 0` none of the other members are ever read before
    // being written, so starting from zeros is behaviourally identical.
    let mut cache = c2GJKCache {
        metric: 0.0,
        count: 0,
        iA: [0; 3],
        iB: [0; 3],
        div: 0.0,
    };
    cache.count = 0;

    let A = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 15.0f32,
    };

    let B = c2Capsule {
        a: c2v { x: 100.0, y: -25.0 },
        b: c2v { x: 75.0, y: 100.0 },
        r: 10.0,
    };

    let mut a0 = ZERO_V;
    let mut b0 = ZERO_V;
    let mut a = ZERO_V;
    let mut b = ZERO_V;

    let mut iterations: c_int = -1;
    let mut cached_iterations: c_int = -1;
    let d0 = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        ptr::null(),
        &mut a0,
        &mut b0,
        1,
        &mut iterations,
        &mut cache,
    );
    let d1 = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        ptr::null(),
        &mut a,
        &mut b,
        1,
        &mut cached_iterations,
        &mut cache,
    );
    let _ = (d0, d1);

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
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            ptr::null(),
            &mut a,
            &mut b,
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        );
    } else {
        c2GJK(
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            ptr::null(),
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &mut a,
            &mut b,
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        );
    }

    // The C never dereferences `a9` / `b9`; neither do we.
    let _ = (a9, b9);
}
