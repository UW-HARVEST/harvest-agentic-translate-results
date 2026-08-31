//! Translation of c_src/libsodium/crypto_auth/hmacsha512256/auth_hmacsha512256.c

use core::ffi::c_int;

// Local repr(C) copy of crypto_hash_sha512_state (rule 4).
#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

// crypto_auth_hmacsha512_state (the concrete layout behind the
// crypto_auth_hmacsha512256_state typedef).
#[repr(C)]
struct crypto_auth_hmacsha512_state {
    ictx: crypto_hash_sha512_state,
    octx: crypto_hash_sha512_state,
}

// typedef crypto_auth_hmacsha512_state crypto_auth_hmacsha512256_state;
#[repr(C)]
pub struct crypto_auth_hmacsha512256_state {
    inner: crypto_auth_hmacsha512_state,
}

const crypto_auth_hmacsha512256_BYTES: usize = 32;
const crypto_auth_hmacsha512256_KEYBYTES: usize = 32;

extern "C" {
    fn crypto_auth_hmacsha512_init(
        state: *mut crypto_auth_hmacsha512_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha512_update(
        state: *mut crypto_auth_hmacsha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha512_final(
        state: *mut crypto_auth_hmacsha512_state,
        out: *mut u8,
    ) -> c_int;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
    fn sodium_memcmp(b1: *const core::ffi::c_void, b2: *const core::ffi::c_void, len: usize)
        -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_bytes() -> usize {
    crypto_auth_hmacsha512256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keybytes() -> usize {
    crypto_auth_hmacsha512256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512256_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_hmacsha512256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_init(
    state: *mut crypto_auth_hmacsha512256_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    crypto_auth_hmacsha512_init(state as *mut crypto_auth_hmacsha512_state, key, keylen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_update(
    state: *mut crypto_auth_hmacsha512256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_auth_hmacsha512_update(state as *mut crypto_auth_hmacsha512_state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_final(
    state: *mut crypto_auth_hmacsha512256_state,
    out: *mut u8,
) -> c_int {
    let mut out0: [u8; 64] = [0; 64];

    crypto_auth_hmacsha512_final(state as *mut crypto_auth_hmacsha512_state, out0.as_mut_ptr());
    core::ptr::copy_nonoverlapping(out0.as_ptr(), out, 32);
    sodium_memzero(out0.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 64]>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha512256_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_auth_hmacsha512256_init(state, k, crypto_auth_hmacsha512256_KEYBYTES);
    crypto_auth_hmacsha512256_update(state, in_, inlen);
    crypto_auth_hmacsha512256_final(state, out);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 32] = [0; 32];

    crypto_auth_hmacsha512256(correct.as_mut_ptr(), in_, inlen, k);

    let eq: c_int = (h == correct.as_ptr()) as c_int;
    crypto_verify_32(h, correct.as_ptr())
        | eq.wrapping_neg()
        | sodium_memcmp(
            correct.as_ptr() as *const core::ffi::c_void,
            h as *const core::ffi::c_void,
            32,
        )
}
