//! Canonical `#[repr(C)]` layouts for `include/sodium/private/ed25519_ref10.h`
//! in the `fe_25_5` configuration (`HAVE_TI_MODE` undefined).

/// C: `typedef int32_t fe25519[10];`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fe25519(pub [i32; 10]);

impl Fe25519 {
    pub const ZERO: Fe25519 = Fe25519([0; 10]);

    #[inline(always)]
    pub const fn new() -> Self {
        Fe25519([0; 10])
    }
}

impl Default for Fe25519 {
    fn default() -> Self {
        Fe25519([0; 10])
    }
}

impl core::ops::Index<usize> for Fe25519 {
    type Output = i32;
    #[inline(always)]
    fn index(&self, i: usize) -> &i32 {
        &self.0[i]
    }
}

impl core::ops::IndexMut<usize> for Fe25519 {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut i32 {
        &mut self.0[i]
    }
}

/// C: `ge25519_p2` (projective) `(X:Y:Z)`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Ge25519P2 {
    pub X: Fe25519,
    pub Y: Fe25519,
    pub Z: Fe25519,
}

/// C: `ge25519_p3` (extended) `(X:Y:Z:T)`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Ge25519P3 {
    pub X: Fe25519,
    pub Y: Fe25519,
    pub Z: Fe25519,
    pub T: Fe25519,
}

/// C: `ge25519_p1p1` (completed) `((X:Z),(Y:T))`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Ge25519P1p1 {
    pub X: Fe25519,
    pub Y: Fe25519,
    pub Z: Fe25519,
    pub T: Fe25519,
}

/// C: `ge25519_precomp` (Duif) `(y+x, y-x, 2dxy)`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Ge25519Precomp {
    pub yplusx: Fe25519,
    pub yminusx: Fe25519,
    pub xy2d: Fe25519,
}

/// C: `ge25519_cached`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Ge25519Cached {
    pub YplusX: Fe25519,
    pub YminusX: Fe25519,
    pub Z: Fe25519,
    pub T2d: Fe25519,
}
