//! Translation of `crypto_core/ed25519/ref10/ed25519_ref10.c` together with the
//! inline field arithmetic from `include/sodium/private/ed25519_ref10_fe_25_5.h`
//! and `crypto_core/ed25519/ref10/fe_25_5/{fe.h,constants.h,base.h,base2.h}`.
//!
//! `HAVE_TI_MODE` is NOT defined in the reference build, so the 10 x 25.5-bit
//! limb representation (`int32_t fe25519[10]`) is used.

pub mod fe;
pub mod ge;
pub mod h2c;
pub mod ristretto;
pub mod sc;
pub mod sc_mul;
pub mod tables;

/// `typedef int32_t fe25519[10];`
pub type Fe25519 = [i32; 10];

/// `ge25519_p2` — projective (X:Y:Z)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ge25519P2 {
    pub X: Fe25519,
    pub Y: Fe25519,
    pub Z: Fe25519,
}

/// `ge25519_p3` — extended (X:Y:Z:T)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ge25519P3 {
    pub X: Fe25519,
    pub Y: Fe25519,
    pub Z: Fe25519,
    pub T: Fe25519,
}

/// `ge25519_p1p1` — completed ((X:Z),(Y:T))
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ge25519P1p1 {
    pub X: Fe25519,
    pub Y: Fe25519,
    pub Z: Fe25519,
    pub T: Fe25519,
}

/// `ge25519_precomp` — Duif (y+x, y-x, 2dxy)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ge25519Precomp {
    pub yplusx: Fe25519,
    pub yminusx: Fe25519,
    pub xy2d: Fe25519,
}

/// `ge25519_cached`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ge25519Cached {
    pub YplusX: Fe25519,
    pub YminusX: Fe25519,
    pub Z: Fe25519,
    pub T2d: Fe25519,
}

impl Ge25519P2 {
    pub const fn zeroed() -> Self {
        Ge25519P2 { X: [0; 10], Y: [0; 10], Z: [0; 10] }
    }
}
impl Ge25519P3 {
    pub const fn zeroed() -> Self {
        Ge25519P3 { X: [0; 10], Y: [0; 10], Z: [0; 10], T: [0; 10] }
    }
}
impl Ge25519P1p1 {
    pub const fn zeroed() -> Self {
        Ge25519P1p1 { X: [0; 10], Y: [0; 10], Z: [0; 10], T: [0; 10] }
    }
}
impl Ge25519Precomp {
    pub const fn zeroed() -> Self {
        Ge25519Precomp { yplusx: [0; 10], yminusx: [0; 10], xy2d: [0; 10] }
    }
}
impl Ge25519Cached {
    pub const fn zeroed() -> Self {
        Ge25519Cached { YplusX: [0; 10], YminusX: [0; 10], Z: [0; 10], T2d: [0; 10] }
    }
}

#[inline(always)]
pub unsafe fn load_3(input: *const u8) -> u64 {
    let mut result = *input.add(0) as u64;
    result |= (*input.add(1) as u64) << 8;
    result |= (*input.add(2) as u64) << 16;
    result
}

#[inline(always)]
pub unsafe fn load_4(input: *const u8) -> u64 {
    let mut result = *input.add(0) as u64;
    result |= (*input.add(1) as u64) << 8;
    result |= (*input.add(2) as u64) << 16;
    result |= (*input.add(3) as u64) << 24;
    result
}
