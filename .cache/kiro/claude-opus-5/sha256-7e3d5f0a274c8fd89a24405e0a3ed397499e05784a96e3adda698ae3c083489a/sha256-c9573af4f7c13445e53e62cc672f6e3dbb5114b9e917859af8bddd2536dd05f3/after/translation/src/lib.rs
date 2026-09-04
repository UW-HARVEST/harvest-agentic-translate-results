#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(
    unused_imports,
    unused_variables,
    dead_code,
    unused_mut,
    unused_assignments
)]
#![allow(unused_parens, unused_unsafe, unused_labels)]
#![allow(clippy::all)]

use core::ffi::c_int;

pub mod common;

pub mod sodium_codecs;
pub mod sodium_core;
pub mod sodium_runtime;
pub mod sodium_utils;
pub mod sodium_version;

pub mod randombytes;

pub mod crypto_verify;

pub mod crypto_aead;
pub mod crypto_auth;
pub mod crypto_box;
pub mod crypto_core;
pub mod crypto_generichash;
pub mod crypto_hash;
pub mod crypto_ipcrypt;
pub mod crypto_kdf;
pub mod crypto_kem;
pub mod crypto_kx;
pub mod crypto_onetimeauth;
pub mod crypto_pwhash;
pub mod crypto_scalarmult;
pub mod crypto_secretbox;
pub mod crypto_secretstream;
pub mod crypto_shorthash;
pub mod crypto_sign;
pub mod crypto_stream;
pub mod crypto_xof;

/* ---- errno / abort helpers ---- */

pub const ENOSYS: c_int = libc::ENOSYS;
pub const ENOMEM: c_int = libc::ENOMEM;
pub const ERANGE: c_int = libc::ERANGE;
pub const EINVAL: c_int = libc::EINVAL;

#[inline]
pub fn set_errno(v: c_int) {
    unsafe {
        *libc::__errno_location() = v;
    }
}

#[inline]
pub fn get_errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

#[inline]
pub fn abort() -> ! {
    unsafe { libc::abort() }
}
