//! Translation of `crypto_core/ed25519/ref10/` and
//! `include/sodium/private/ed25519_ref10.h`.
//!
//! `HAVE_TI_MODE` is undefined, so `fe25519` is `int32_t[10]` (the fe_25_5
//! representation).

pub mod base;
pub mod fe;
pub mod ge;
pub mod sc;

/// `typedef int32_t fe25519[10];`
pub type fe25519 = [i32; 10];

/// (X:Y:Z) satisfying x=X/Z, y=Y/Z
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p2 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
}

/// (X:Y:Z:T) satisfying x=X/Z, y=Y/Z, XY=ZT
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p3 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

/// ((X:Z),(Y:T)) satisfying x=X/Z, y=Y/T
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p1p1 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

/// Duif representation: (y+x, y-x, 2dxy)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_precomp {
    pub yplusx: fe25519,
    pub yminusx: fe25519,
    pub xy2d: fe25519,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_cached {
    pub YplusX: fe25519,
    pub YminusX: fe25519,
    pub Z: fe25519,
    pub T2d: fe25519,
}

/// `load_3()` from ed25519_ref10.c
#[inline]
pub unsafe fn load_3(inp: *const u8) -> u64 {
    let mut result: u64;

    result = *inp.add(0) as u64;
    result |= (*inp.add(1) as u64) << 8;
    result |= (*inp.add(2) as u64) << 16;

    result
}

/// `load_4()` from ed25519_ref10.c
#[inline]
pub unsafe fn load_4(inp: *const u8) -> u64 {
    let mut result: u64;

    result = *inp.add(0) as u64;
    result |= (*inp.add(1) as u64) << 8;
    result |= (*inp.add(2) as u64) << 16;
    result |= (*inp.add(3) as u64) << 24;

    result
}
