pub mod soft;

// Translation of `crypto_ipcrypt/crypto_ipcrypt.c` plus the
// `ipcrypt_implementation` type of `crypto_ipcrypt/implementations.h`.
//
// The reference build defines neither `HAVE_ARMCRYPTO`/`NATIVE_LITTLE_ENDIAN`
// nor the `HAVE_AVXINTRIN_H`/`HAVE_WMMINTRIN_H` pair, so the two accelerated
// backends are not compiled and `_crypto_ipcrypt_pick_best_implementation()`
// reduces to "select the soft backend, return 0".
//
// `crypto_ipcrypt_*` and `_crypto_ipcrypt_pick_best_implementation` are not
// renamed by `include/sodium/private/quirks.h`.

use core::ffi::{c_int, c_void};

use crate::randombytes::randombytes_buf;

/* ------------------------------------------------------------------ */
/* crypto_ipcrypt/implementations.h                                   */
/* ------------------------------------------------------------------ */

/// ```c
/// typedef struct ipcrypt_implementation {
///     void (*encrypt)(uint8_t *out, const uint8_t *in, const uint8_t *k);
///     void (*decrypt)(uint8_t *out, const uint8_t *in, const uint8_t *k);
///     void (*nd_encrypt)(uint8_t *out, const uint8_t *in, const uint8_t *t, const uint8_t *k);
///     void (*nd_decrypt)(uint8_t *out, const uint8_t *in, const uint8_t *k);
///     void (*ndx_encrypt)(uint8_t *out, const uint8_t *in, const uint8_t *t, const uint8_t *k);
///     void (*ndx_decrypt)(uint8_t *out, const uint8_t *in, const uint8_t *k);
///     void (*pfx_encrypt)(uint8_t *out, const uint8_t *in, const uint8_t *k);
///     void (*pfx_decrypt)(uint8_t *out, const uint8_t *in, const uint8_t *k);
/// } ipcrypt_implementation;
/// ```
#[repr(C)]
pub struct ipcrypt_implementation {
    pub encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub nd_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub nd_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub ndx_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub ndx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
}

/* ------------------------------------------------------------------ */
/* include/sodium/crypto_ipcrypt.h                                    */
/* ------------------------------------------------------------------ */

pub const crypto_ipcrypt_BYTES: usize = 16;
pub const crypto_ipcrypt_KEYBYTES: usize = 16;
pub const crypto_ipcrypt_ND_KEYBYTES: usize = 16;
pub const crypto_ipcrypt_ND_TWEAKBYTES: usize = 8;
pub const crypto_ipcrypt_ND_INPUTBYTES: usize = 16;
pub const crypto_ipcrypt_ND_OUTPUTBYTES: usize = 24;
pub const crypto_ipcrypt_NDX_KEYBYTES: usize = 32;
pub const crypto_ipcrypt_NDX_TWEAKBYTES: usize = 16;
pub const crypto_ipcrypt_NDX_INPUTBYTES: usize = 16;
pub const crypto_ipcrypt_NDX_OUTPUTBYTES: usize = 32;
pub const crypto_ipcrypt_PFX_KEYBYTES: usize = 32;
pub const crypto_ipcrypt_PFX_BYTES: usize = 16;

/* ------------------------------------------------------------------ */
/* crypto_ipcrypt.c                                                   */
/* ------------------------------------------------------------------ */

/// `static const ipcrypt_implementation *implementation = &ipcrypt_soft_implementation;`
static mut implementation: *const ipcrypt_implementation =
    &raw const soft::ipcrypt_soft_implementation;

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
    unsafe { ((*implementation).encrypt)(out, in_, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    unsafe { ((*implementation).decrypt)(out, in_, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    unsafe { ((*implementation).nd_encrypt)(out, in_, t, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    unsafe { ((*implementation).nd_decrypt)(out, in_, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    unsafe { ((*implementation).ndx_encrypt)(out, in_, t, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    unsafe { ((*implementation).ndx_decrypt)(out, in_, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    unsafe { ((*implementation).pfx_encrypt)(out, in_, k) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    unsafe { ((*implementation).pfx_decrypt)(out, in_, k) };
}

/// `int _crypto_ipcrypt_pick_best_implementation(void)`
///
/// Both accelerated backends are `#if`-ed out in the reference build, so the
/// function unconditionally installs the soft backend and returns 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_ipcrypt_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &raw const soft::ipcrypt_soft_implementation;
    }

    0
}
