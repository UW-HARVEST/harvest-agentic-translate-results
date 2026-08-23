//! Translation of `crypto_kdf/hkdf/kdf_hkdf_sha256.c`.
//!
//! Nothing in this file is affected by `private/quirks.h`, so every exported
//! function keeps its plain C name.

use crate::common::memcpy;
use core::ffi::{c_char, c_int, c_ulonglong, c_void};
use core::ptr::addr_of_mut;

/* crypto_hash_sha256.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/* crypto_auth_hmacsha256.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

/* crypto_kdf_hkdf_sha256.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_kdf_hkdf_sha256_state {
    pub st: crypto_auth_hmacsha256_state,
}

pub const crypto_auth_hmacsha256_BYTES: usize = 32;

/* #define crypto_kdf_hkdf_sha256_KEYBYTES crypto_auth_hmacsha256_BYTES */
pub const crypto_kdf_hkdf_sha256_KEYBYTES: usize = crypto_auth_hmacsha256_BYTES;
/* #define crypto_kdf_hkdf_sha256_BYTES_MIN 0U */
pub const crypto_kdf_hkdf_sha256_BYTES_MIN: usize = 0;
/* #define crypto_kdf_hkdf_sha256_BYTES_MAX (0xff * crypto_auth_hmacsha256_BYTES) */
pub const crypto_kdf_hkdf_sha256_BYTES_MAX: usize = 0xff * crypto_auth_hmacsha256_BYTES;

/* <errno.h> */
const EINVAL: c_int = 22;

extern "C" {
    fn crypto_auth_hmacsha256_init(
        state: *mut crypto_auth_hmacsha256_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha256_update(
        state: *mut crypto_auth_hmacsha256_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_auth_hmacsha256_final(
        state: *mut crypto_auth_hmacsha256_state,
        out: *mut u8,
    ) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn __errno_location() -> *mut c_int;
}

/* int crypto_kdf_hkdf_sha256_extract_init(crypto_kdf_hkdf_sha256_state *state,
                                           const unsigned char *salt,
                                           size_t salt_len) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_init(
    state: *mut crypto_kdf_hkdf_sha256_state,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_init(addr_of_mut!((*state).st), salt, salt_len)
}

/* int crypto_kdf_hkdf_sha256_extract_update(crypto_kdf_hkdf_sha256_state *state,
                                             const unsigned char *ikm,
                                             size_t ikm_len) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_update(
    state: *mut crypto_kdf_hkdf_sha256_state,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_update(addr_of_mut!((*state).st), ikm, ikm_len as c_ulonglong)
}

/* int crypto_kdf_hkdf_sha256_extract_final(crypto_kdf_hkdf_sha256_state *state,
                                            unsigned char prk[32]) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_final(
    state: *mut crypto_kdf_hkdf_sha256_state,
    prk: *mut u8,
) -> c_int {
    crypto_auth_hmacsha256_final(addr_of_mut!((*state).st), prk);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_kdf_hkdf_sha256_state>(),
    );

    0
}

/* int crypto_kdf_hkdf_sha256_extract(unsigned char prk[32],
                                      const unsigned char *salt, size_t salt_len,
                                      const unsigned char *ikm, size_t ikm_len) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract(
    prk: *mut u8,
    salt: *const u8,
    salt_len: usize,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    let mut state_storage = core::mem::MaybeUninit::<crypto_kdf_hkdf_sha256_state>::uninit();
    let state: *mut crypto_kdf_hkdf_sha256_state = state_storage.as_mut_ptr();

    crypto_kdf_hkdf_sha256_extract_init(state, salt, salt_len);
    crypto_kdf_hkdf_sha256_extract_update(state, ikm, ikm_len);

    crypto_kdf_hkdf_sha256_extract_final(state, prk)
}

/* void crypto_kdf_hkdf_sha256_keygen(unsigned char prk[32]) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keygen(prk: *mut u8) {
    randombytes_buf(prk as *mut c_void, crypto_kdf_hkdf_sha256_KEYBYTES);
}

/* int crypto_kdf_hkdf_sha256_expand(unsigned char *out, size_t out_len,
                                     const char *ctx, size_t ctx_len,
                                     const unsigned char prk[32]) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    let mut st_storage = core::mem::MaybeUninit::<crypto_auth_hmacsha256_state>::uninit();
    let st: *mut crypto_auth_hmacsha256_state = st_storage.as_mut_ptr();
    let mut tmp: [u8; crypto_auth_hmacsha256_BYTES] = [0; crypto_auth_hmacsha256_BYTES];
    let mut i: usize;
    let left: usize;
    let mut counter: u8 = 1;

    if out_len > crypto_kdf_hkdf_sha256_BYTES_MAX {
        *__errno_location() = EINVAL;
        return -1;
    }
    i = 0usize;
    while i.wrapping_add(crypto_auth_hmacsha256_BYTES) <= out_len {
        crypto_auth_hmacsha256_init(st, prk, crypto_kdf_hkdf_sha256_KEYBYTES);
        if i != 0usize {
            crypto_auth_hmacsha256_update(
                st,
                out.add(i - crypto_auth_hmacsha256_BYTES),
                crypto_auth_hmacsha256_BYTES as c_ulonglong,
            );
        }
        crypto_auth_hmacsha256_update(st, ctx as *const u8, ctx_len as c_ulonglong);
        crypto_auth_hmacsha256_update(st, &counter as *const u8, 1 as c_ulonglong);
        crypto_auth_hmacsha256_final(st, out.add(i));
        counter = counter.wrapping_add(1);

        i = i.wrapping_add(crypto_auth_hmacsha256_BYTES);
    }
    left = out_len & (crypto_auth_hmacsha256_BYTES - 1);
    if left != 0usize {
        crypto_auth_hmacsha256_init(st, prk, crypto_kdf_hkdf_sha256_KEYBYTES);
        if i != 0usize {
            crypto_auth_hmacsha256_update(
                st,
                out.add(i - crypto_auth_hmacsha256_BYTES),
                crypto_auth_hmacsha256_BYTES as c_ulonglong,
            );
        }
        crypto_auth_hmacsha256_update(st, ctx as *const u8, ctx_len as c_ulonglong);
        crypto_auth_hmacsha256_update(st, &counter as *const u8, 1 as c_ulonglong);
        crypto_auth_hmacsha256_final(st, tmp.as_mut_ptr());
        memcpy(out.add(i), tmp.as_ptr(), left);
        sodium_memzero(
            tmp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&tmp),
        );
    }
    sodium_memzero(
        st as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );

    0
}

/* size_t crypto_kdf_hkdf_sha256_keybytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keybytes() -> usize {
    crypto_kdf_hkdf_sha256_KEYBYTES
}

/* size_t crypto_kdf_hkdf_sha256_bytes_min(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_bytes_min() -> usize {
    crypto_kdf_hkdf_sha256_BYTES_MIN
}

/* size_t crypto_kdf_hkdf_sha256_bytes_max(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_bytes_max() -> usize {
    crypto_kdf_hkdf_sha256_BYTES_MAX
}

/* size_t crypto_kdf_hkdf_sha256_statebytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_kdf_hkdf_sha256_state>()
}
