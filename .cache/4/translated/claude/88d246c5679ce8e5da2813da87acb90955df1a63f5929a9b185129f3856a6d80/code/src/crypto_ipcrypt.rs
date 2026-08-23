//! Translation of `crypto_ipcrypt/crypto_ipcrypt.c`.
//!
//! Neither `HAVE_ARMCRYPTO`/`NATIVE_LITTLE_ENDIAN` nor
//! `HAVE_AVXINTRIN_H`/`HAVE_WMMINTRIN_H` are defined in the reference build, so
//! the AES-NI and ARM crypto implementations (and the corresponding
//! `sodium_runtime_has_*()` probes) are compiled out entirely; the portable
//! `ipcrypt_soft_implementation` is the only candidate.

use core::ffi::{c_int, c_void};

/* ------------------------------------------------------------------------- */
/* `crypto_ipcrypt/implementations.h`                                        */
/* ------------------------------------------------------------------------- */

#[repr(C)]
pub struct ipcrypt_implementation {
    pub encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub nd_encrypt:
        unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub nd_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub ndx_encrypt:
        unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub ndx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
}

extern "C" {
    /* crypto_ipcrypt/ipcrypt_soft.c */
    static ipcrypt_soft_implementation: ipcrypt_implementation;
    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* `crypto_ipcrypt.h` */
const crypto_ipcrypt_BYTES: usize = 16;
const crypto_ipcrypt_KEYBYTES: usize = 16;
const crypto_ipcrypt_ND_KEYBYTES: usize = 16;
const crypto_ipcrypt_ND_TWEAKBYTES: usize = 8;
const crypto_ipcrypt_ND_INPUTBYTES: usize = 16;
const crypto_ipcrypt_ND_OUTPUTBYTES: usize = 24;
const crypto_ipcrypt_NDX_KEYBYTES: usize = 32;
const crypto_ipcrypt_NDX_TWEAKBYTES: usize = 16;
const crypto_ipcrypt_NDX_INPUTBYTES: usize = 16;
const crypto_ipcrypt_NDX_OUTPUTBYTES: usize = 32;
const crypto_ipcrypt_PFX_KEYBYTES: usize = 32;
const crypto_ipcrypt_PFX_BYTES: usize = 16;

static mut implementation: *const ipcrypt_implementation =
    unsafe { &ipcrypt_soft_implementation };

#[inline(always)]
unsafe fn imp() -> &'static ipcrypt_implementation {
    &*implementation
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_bytes() -> usize {
    crypto_ipcrypt_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_keybytes() -> usize {
    crypto_ipcrypt_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keybytes() -> usize {
    crypto_ipcrypt_ND_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_tweakbytes() -> usize {
    crypto_ipcrypt_ND_TWEAKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_inputbytes() -> usize {
    crypto_ipcrypt_ND_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_outputbytes() -> usize {
    crypto_ipcrypt_ND_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keybytes() -> usize {
    crypto_ipcrypt_NDX_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_tweakbytes() -> usize {
    crypto_ipcrypt_NDX_TWEAKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_inputbytes() -> usize {
    crypto_ipcrypt_NDX_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_outputbytes() -> usize {
    crypto_ipcrypt_NDX_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keybytes() -> usize {
    crypto_ipcrypt_PFX_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_bytes() -> usize {
    crypto_ipcrypt_PFX_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_ipcrypt_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_ipcrypt_ND_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_ipcrypt_NDX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_ipcrypt_PFX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    (imp().encrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    (imp().decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    (imp().nd_encrypt)(out, in_, t, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    (imp().nd_decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    (imp().ndx_encrypt)(out, in_, t, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    (imp().ndx_decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    (imp().pfx_encrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    (imp().pfx_decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_ipcrypt_pick_best_implementation() -> c_int {
    implementation = &ipcrypt_soft_implementation;

    /* No ARM crypto / AES-NI implementations in this build, so the
     * `sodium_runtime_has_armcrypto()` and `sodium_runtime_has_aesni()`
     * branches are removed by the preprocessor. */
    0
}
