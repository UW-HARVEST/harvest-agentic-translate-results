//! Translation of c_src/libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c

use core::ffi::c_int;

// Local repr(C) copy of crypto_hash_sha256_state (rule 4).
#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

// crypto_auth_hmacsha256_state from include/sodium/crypto_auth_hmacsha256.h.
#[repr(C)]
pub struct crypto_auth_hmacsha256_state {
    ictx: crypto_hash_sha256_state,
    octx: crypto_hash_sha256_state,
}

const crypto_auth_hmacsha256_BYTES: usize = 32;
const crypto_auth_hmacsha256_KEYBYTES: usize = 32;

extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> c_int;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
    fn sodium_memcmp(b1: *const core::ffi::c_void, b2: *const core::ffi::c_void, len: usize)
        -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_bytes() -> usize {
    crypto_auth_hmacsha256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keybytes() -> usize {
    crypto_auth_hmacsha256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha256_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_hmacsha256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_init(
    state: *mut crypto_auth_hmacsha256_state,
    mut key: *const u8,
    mut keylen: usize,
) -> c_int {
    let mut pad: [u8; 64] = [0; 64];
    let mut khash: [u8; 32] = [0; 32];
    let mut i: usize;

    let ictx = core::ptr::addr_of_mut!((*state).ictx);
    let octx = core::ptr::addr_of_mut!((*state).octx);

    if keylen > 64 {
        crypto_hash_sha256_init(ictx);
        crypto_hash_sha256_update(ictx, key, keylen as u64);
        crypto_hash_sha256_final(ictx, khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 32;
    } else if key.is_null() {
        if keylen > 0 {
            crate::sodium::core::sodium_misuse(); // LCOV_EXCL_LINE
        }
    }
    crypto_hash_sha256_init(ictx);
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x36, 64);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha256_update(ictx, pad.as_ptr(), 64);

    crypto_hash_sha256_init(octx);
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x5c, 64);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha256_update(octx, pad.as_ptr(), 64);

    sodium_memzero(pad.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 64]>());
    sodium_memzero(khash.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 32]>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_update(
    state: *mut crypto_auth_hmacsha256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_hash_sha256_update(core::ptr::addr_of_mut!((*state).ictx), in_, inlen);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_final(
    state: *mut crypto_auth_hmacsha256_state,
    out: *mut u8,
) -> c_int {
    let mut ihash: [u8; 32] = [0; 32];

    crypto_hash_sha256_final(core::ptr::addr_of_mut!((*state).ictx), ihash.as_mut_ptr());
    crypto_hash_sha256_update(core::ptr::addr_of_mut!((*state).octx), ihash.as_ptr(), 32);
    crypto_hash_sha256_final(core::ptr::addr_of_mut!((*state).octx), out);

    sodium_memzero(ihash.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 32]>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha256_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_auth_hmacsha256_init(state, k, crypto_auth_hmacsha256_KEYBYTES);
    crypto_auth_hmacsha256_update(state, in_, inlen);
    crypto_auth_hmacsha256_final(state, out);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 32] = [0; 32];

    crypto_auth_hmacsha256(correct.as_mut_ptr(), in_, inlen, k);

    // return crypto_verify_32(h, correct) | (-(h == correct)) |
    //        sodium_memcmp(correct, h, 32);
    let eq: c_int = (h == correct.as_ptr()) as c_int;
    crypto_verify_32(h, correct.as_ptr())
        | eq.wrapping_neg()
        | sodium_memcmp(
            correct.as_ptr() as *const core::ffi::c_void,
            h as *const core::ffi::c_void,
            32,
        )
}
