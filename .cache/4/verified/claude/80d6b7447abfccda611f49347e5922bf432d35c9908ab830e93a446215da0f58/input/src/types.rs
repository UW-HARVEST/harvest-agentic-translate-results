//! Type definitions mirroring `c_src/include/lib.h` and the private types at the
//! top of `c_src/src/lib.c`.
//!
//! Every layout here has been verified against GCC (`sizeof`/`_Alignof`/`offsetof`)
//! for the x86-64 SysV ABI:
//!
//! ```text
//! c2v 8/4   c2h 12/4  c2r 8/4   c2x 16/4   c2Circle 12/4  c2AABB 16/4
//! c2Capsule 20/4      c2Poly 132/4 (verts@4, norms@68)    c2GJKCache 36/4 (iA@8, iB@20, div@32)
//! c2Manifold 36/4 (depths@4, contact_points@12, n@28)     c2Proxy 72/4
//! c2sv 36/4 (sA@0, sB@8, p@16, u@24, iA@28, iB@32)
//! c2Simplex 152/4 (b@36, c@72, d@108, div@144, count@148)
//! ```

use core::ffi::{c_float, c_int, c_uint};

/// `C2_TYPE` is a C enum whose enumerators are all non-negative, so GCC gives it
/// the underlying type `unsigned int`. Only the four values below are meaningful;
/// any other value falls through every `switch` exactly as it does in C.
pub type C2_TYPE = c_uint;

pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [c_float; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug, Default)]
pub struct c2GJKCache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

/// Anonymous `typedef struct { ... } c2Proxy;` in `lib.c`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

/// Anonymous `typedef struct { ... } c2sv;` in `lib.c` (a simplex vertex).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

/// Anonymous `typedef struct { ... } c2Simplex;` in `lib.c`.
///
/// `a`, `b`, `c`, `d` are contiguous so that C's `c2sv *verts = &s.a;` followed by
/// `verts[i]` indexing works identically. There are exactly 4 vertex slots.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: c_float,
    pub count: c_int,
}

/// Number of `c2sv` slots in a `c2Simplex` (`a`, `b`, `c`, `d`).
pub const C2_SIMPLEX_SLOTS: usize = 4;

/// `FLT_MAX`, spelled exactly as the C source spells it.
pub const C2_FLT_MAX: c_float = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
/// `FLT_EPSILON`, spelled exactly as the C source spells it.
pub const C2_FLT_EPSILON: c_float = 1.192_092_895_507_812_5e-7;
