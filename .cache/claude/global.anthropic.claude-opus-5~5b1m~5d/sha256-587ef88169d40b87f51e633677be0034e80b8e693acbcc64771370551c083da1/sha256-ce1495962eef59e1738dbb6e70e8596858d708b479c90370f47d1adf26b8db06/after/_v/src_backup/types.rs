//! Shared `#[repr(C)]` types that cross module boundaries.
#![allow(dead_code, non_camel_case_types)]

/// `typedef int32_t fe25519[10];` (HAVE_TI_MODE is not defined in the reference build)
pub type fe25519 = [i32; 10];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ge25519_p2 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ge25519_p3 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ge25519_p1p1 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ge25519_precomp {
    pub yplusx: fe25519,
    pub yminusx: fe25519,
    pub xy2d: fe25519,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ge25519_cached {
    pub YplusX: fe25519,
    pub YminusX: fe25519,
    pub Z: fe25519,
    pub T2d: fe25519,
}

impl ge25519_p2 {
    pub const fn zero() -> Self {
        Self { X: [0; 10], Y: [0; 10], Z: [0; 10] }
    }
}
impl ge25519_p3 {
    pub const fn zero() -> Self {
        Self { X: [0; 10], Y: [0; 10], Z: [0; 10], T: [0; 10] }
    }
}
impl ge25519_p1p1 {
    pub const fn zero() -> Self {
        Self { X: [0; 10], Y: [0; 10], Z: [0; 10], T: [0; 10] }
    }
}
impl ge25519_precomp {
    pub const fn zero() -> Self {
        Self { yplusx: [0; 10], yminusx: [0; 10], xy2d: [0; 10] }
    }
}
impl ge25519_cached {
    pub const fn zero() -> Self {
        Self { YplusX: [0; 10], YminusX: [0; 10], Z: [0; 10], T2d: [0; 10] }
    }
}

/// `crypto_hash_sha256_state`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/// `crypto_hash_sha512_state`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

/// `crypto_generichash_blake2b_state` — declared as
/// `CRYPTO_ALIGN(64) unsigned char opaque[384]`
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct crypto_generichash_blake2b_state {
    pub opaque: [u8; 384],
}

/// `blake2b_state` (internal, from crypto_generichash/blake2b/ref/blake2.h).
/// The C declaration is inside `#pragma pack(push, 1)`, but every field is
/// already naturally aligned, so field offsets are identical to `repr(C)`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct blake2b_state {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; 256],
    pub buflen: usize,
    pub last_node: u8,
}

/// `randombytes_implementation`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct randombytes_implementation {
    pub implementation_name: Option<unsafe extern "C" fn() -> *const core::ffi::c_char>,
    pub random: Option<unsafe extern "C" fn() -> u32>,
    pub stir: Option<unsafe extern "C" fn()>,
    pub uniform: Option<unsafe extern "C" fn(upper_bound: u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(buf: *mut core::ffi::c_void, size: usize)>,
    pub close: Option<unsafe extern "C" fn() -> core::ffi::c_int>,
}

unsafe impl Sync for randombytes_implementation {}
