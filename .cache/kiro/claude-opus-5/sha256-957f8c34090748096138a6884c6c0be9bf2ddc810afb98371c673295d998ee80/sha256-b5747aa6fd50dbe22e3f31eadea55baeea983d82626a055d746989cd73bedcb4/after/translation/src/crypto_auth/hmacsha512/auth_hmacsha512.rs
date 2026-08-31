//! Translation of c_src/libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c

use core::ffi::c_int;

// Local repr(C) copy of crypto_hash_sha512_state (rule 4).
#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

// crypto_auth_hmacsha512_state from include/sodium/crypto_auth_hmacsha512.h.
#[repr(C)]
pub struct crypto_auth_hmacsha512_state {
    ictx: crypto_hash_sha512_state,
    octx: crypto_hash_sha512_state,
}

const crypto_auth_hmacsha512_BYTES: usize = 64;
const crypto_auth_hmacsha512_KEYBYTES: usize = 32;

extern "C" {
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;
    fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int;
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
    fn sodium_memcmp(b1: *const core::ffi::c_void, b2: *const core::ffi::c_void, len: usize)
        -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_bytes() -> usize {
    crypto_auth_hmacsha512_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keybytes() -> usize {
    crypto_auth_hmacsha512_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_hmacsha512_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_init(
    state: *mut crypto_auth_hmacsha512_state,
    mut key: *const u8,
    mut keylen: usize,
) -> c_int {
    let mut pad: [u8; 128] = [0; 128];
    let mut khash: [u8; 64] = [0; 64];
    let mut i: usize;

    let ictx = core::ptr::addr_of_mut!((*state).ictx);
    let octx = core::ptr::addr_of_mut!((*state).octx);

    if keylen > 128 {
        crypto_hash_sha512_init(ictx);
        crypto_hash_sha512_update(ictx, key, keylen as u64);
        crypto_hash_sha512_final(ictx, khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 64;
    } else if key.is_null() {
        if keylen > 0 {
            crate::sodium::core::sodium_misuse(); // LCOV_EXCL_LINE
        }
    }
    crypto_hash_sha512_init(ictx);
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x36, 128);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha512_update(ictx, pad.as_ptr(), 128);

    crypto_hash_sha512_init(octx);
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x5c, 128);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha512_update(octx, pad.as_ptr(), 128);

    sodium_memzero(pad.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 128]>());
    sodium_memzero(khash.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 64]>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_update(
    state: *mut crypto_auth_hmacsha512_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_hash_sha512_update(core::ptr::addr_of_mut!((*state).ictx), in_, inlen);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_final(
    state: *mut crypto_auth_hmacsha512_state,
    out: *mut u8,
) -> c_int {
    let mut ihash: [u8; 64] = [0; 64];

    crypto_hash_sha512_final(core::ptr::addr_of_mut!((*state).ictx), ihash.as_mut_ptr());
    crypto_hash_sha512_update(core::ptr::addr_of_mut!((*state).octx), ihash.as_ptr(), 64);
    crypto_hash_sha512_final(core::ptr::addr_of_mut!((*state).octx), out);

    sodium_memzero(ihash.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of::<[u8; 64]>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha512_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_auth_hmacsha512_init(state, k, crypto_auth_hmacsha512_KEYBYTES);
    crypto_auth_hmacsha512_update(state, in_, inlen);
    crypto_auth_hmacsha512_final(state, out);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 64] = [0; 64];

    crypto_auth_hmacsha512(correct.as_mut_ptr(), in_, inlen, k);

    let eq: c_int = (h == correct.as_ptr()) as c_int;
    crypto_verify_64(h, correct.as_ptr())
        | eq.wrapping_neg()
        | sodium_memcmp(
            correct.as_ptr() as *const core::ffi::c_void,
            h as *const core::ffi::c_void,
            64,
        )
}
