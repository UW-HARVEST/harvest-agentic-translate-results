//! Rust translation of `crypto_aead/aes256gcm/aead_aes256gcm.c`.
//!
//! In the reference build configuration (no `HAVE_*` macros defined), the
//! AES-NI (`aead_aes256gcm_aesni.c`) and ARM crypto
//! (`aead_aes256gcm_armcrypto.c`) implementations are entirely `#ifdef`-ed
//! out, so `aead_aes256gcm.c` is compiled standalone: the hardware AES-GCM
//! implementation is unavailable, `crypto_aead_aes256gcm_is_available()`
//! returns `0`, and every actual crypto operation sets `errno = ENOSYS` and
//! returns `-1`.

use core::ffi::c_int;

#[repr(C, align(16))]
pub struct crypto_aead_aes256gcm_state {
    pub opaque: [u8; 512],
}

extern "C" {
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
    fn __errno_location() -> *mut c_int;
}

const ENOSYS: c_int = 38;

#[inline]
unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_nsecbytes() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_npubbytes() -> usize {
    12
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_abytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_statebytes() -> usize {
    (core::mem::size_of::<crypto_aead_aes256gcm_state>() + 15) & !15usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_messagebytes_max() -> usize {
    let a: u64 = u64::MAX - 16;
    let b: u64 = 16u64 * ((1u64 << 32) - 2);
    (if a < b { a } else { b }) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached(
    _c: *mut u8,
    _mac: *mut u8,
    _maclen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt(
    _c: *mut u8,
    _clen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached(
    _m: *mut u8,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _mac: *const u8,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt(
    _m: *mut u8,
    _mlen_p: *mut u64,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_beforenm(
    _st_: *mut crypto_aead_aes256gcm_state,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached_afternm(
    _c: *mut u8,
    _mac: *mut u8,
    _maclen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_afternm(
    _c: *mut u8,
    _clen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached_afternm(
    _m: *mut u8,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _mac: *const u8,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_afternm(
    _m: *mut u8,
    _mlen_p: *mut u64,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aes256gcm_is_available() -> c_int {
    0
}
