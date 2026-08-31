//! Translation of c_src/libsodium/crypto_ipcrypt/crypto_ipcrypt.c
//!
//! HAVE_ARMCRYPTO / HAVE_AVXINTRIN_H / HAVE_WMMINTRIN_H are all undefined, so
//! only the soft implementation exists.

use core::ffi::{c_int, c_void};

// Constants from crypto_ipcrypt.h
const CRYPTO_IPCRYPT_BYTES: usize = 16;
const CRYPTO_IPCRYPT_KEYBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_KEYBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_TWEAKBYTES: usize = 8;
const CRYPTO_IPCRYPT_ND_INPUTBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_OUTPUTBYTES: usize = 24;
const CRYPTO_IPCRYPT_NDX_KEYBYTES: usize = 32;
const CRYPTO_IPCRYPT_NDX_TWEAKBYTES: usize = 16;
const CRYPTO_IPCRYPT_NDX_INPUTBYTES: usize = 16;
const CRYPTO_IPCRYPT_NDX_OUTPUTBYTES: usize = 32;
const CRYPTO_IPCRYPT_PFX_KEYBYTES: usize = 32;
const CRYPTO_IPCRYPT_PFX_BYTES: usize = 16;

// struct ipcrypt_implementation (see crypto_ipcrypt/implementations.h)
type EncDecFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type NdxEncFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);

#[repr(C)]
struct ipcrypt_implementation {
    encrypt: Option<EncDecFn>,
    decrypt: Option<EncDecFn>,
    nd_encrypt: Option<NdxEncFn>,
    nd_decrypt: Option<EncDecFn>,
    ndx_encrypt: Option<NdxEncFn>,
    ndx_decrypt: Option<EncDecFn>,
    pfx_encrypt: Option<EncDecFn>,
    pfx_decrypt: Option<EncDecFn>,
}
unsafe impl Sync for ipcrypt_implementation {}

extern "C" {
    static ipcrypt_soft_implementation: ipcrypt_implementation;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// static const ipcrypt_implementation *implementation = &ipcrypt_soft_implementation;
static mut implementation: *const ipcrypt_implementation = core::ptr::null();

#[inline]
unsafe fn impl_ptr() -> *const ipcrypt_implementation {
    if implementation.is_null() {
        implementation = &ipcrypt_soft_implementation as *const ipcrypt_implementation;
    }
    implementation
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_bytes() -> usize {
    CRYPTO_IPCRYPT_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_keybytes() -> usize {
    CRYPTO_IPCRYPT_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keybytes() -> usize {
    CRYPTO_IPCRYPT_ND_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_tweakbytes() -> usize {
    CRYPTO_IPCRYPT_ND_TWEAKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_inputbytes() -> usize {
    CRYPTO_IPCRYPT_ND_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_outputbytes() -> usize {
    CRYPTO_IPCRYPT_ND_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keybytes() -> usize {
    CRYPTO_IPCRYPT_NDX_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_tweakbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_TWEAKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_inputbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_outputbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keybytes() -> usize {
    CRYPTO_IPCRYPT_PFX_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_bytes() -> usize {
    CRYPTO_IPCRYPT_PFX_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_ND_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_NDX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_IPCRYPT_PFX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*impl_ptr()).encrypt.unwrap_unchecked())(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*impl_ptr()).decrypt.unwrap_unchecked())(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    ((*impl_ptr()).nd_encrypt.unwrap_unchecked())(out, in_, t, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*impl_ptr()).nd_decrypt.unwrap_unchecked())(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    ((*impl_ptr()).ndx_encrypt.unwrap_unchecked())(out, in_, t, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*impl_ptr()).ndx_decrypt.unwrap_unchecked())(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*impl_ptr()).pfx_encrypt.unwrap_unchecked())(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*impl_ptr()).pfx_decrypt.unwrap_unchecked())(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_ipcrypt_pick_best_implementation() -> c_int {
    implementation = &ipcrypt_soft_implementation as *const ipcrypt_implementation;

    // HAVE_ARMCRYPTO / HAVE_AVXINTRIN_H / HAVE_WMMINTRIN_H undefined.
    0
}
