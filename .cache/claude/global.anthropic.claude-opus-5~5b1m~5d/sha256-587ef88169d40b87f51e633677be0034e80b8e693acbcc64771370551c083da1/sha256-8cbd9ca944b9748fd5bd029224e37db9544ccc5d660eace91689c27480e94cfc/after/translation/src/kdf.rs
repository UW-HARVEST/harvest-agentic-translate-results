//! Translated from:
//! * `crypto_kdf/crypto_kdf.c`
//! * `crypto_kdf/blake2b/kdf_blake2b.c`
//! * `crypto_kdf/hkdf/kdf_hkdf_sha256.c`
//! * `crypto_kdf/hkdf/kdf_hkdf_sha512.c`

use core::ffi::{c_char, c_int, c_void};

use crate::csys::{set_errno, EINVAL};
use crate::types::{crypto_hash_sha256_state, crypto_hash_sha512_state};

extern "C" {
    fn crypto_generichash_blake2b_salt_personal(
        out: *mut u8,
        outlen: usize,
        inp: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
        salt: *const u8,
        personal: *const u8,
    ) -> c_int;

    fn crypto_auth_hmacsha256_init(
        state: *mut crypto_auth_hmacsha256_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha256_update(
        state: *mut crypto_auth_hmacsha256_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha256_final(state: *mut crypto_auth_hmacsha256_state, out: *mut u8)
        -> c_int;

    fn crypto_auth_hmacsha512_init(
        state: *mut crypto_auth_hmacsha512_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha512_update(
        state: *mut crypto_auth_hmacsha512_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha512_final(state: *mut crypto_auth_hmacsha512_state, out: *mut u8)
        -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut u8, size: usize);
}

/// `crypto_auth_hmacsha256_state` — from `crypto_auth_hmacsha256.h`
#[repr(C)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

/// `crypto_auth_hmacsha512_state` — from `crypto_auth_hmacsha512.h`
#[repr(C)]
pub struct crypto_auth_hmacsha512_state {
    pub ictx: crypto_hash_sha512_state,
    pub octx: crypto_hash_sha512_state,
}

/// `crypto_kdf_hkdf_sha256_state` — from `crypto_kdf_hkdf_sha256.h`
#[repr(C)]
pub struct crypto_kdf_hkdf_sha256_state {
    pub st: crypto_auth_hmacsha256_state,
}

/// `crypto_kdf_hkdf_sha512_state` — from `crypto_kdf_hkdf_sha512.h`
#[repr(C)]
pub struct crypto_kdf_hkdf_sha512_state {
    pub st: crypto_auth_hmacsha512_state,
}

// ===================== crypto_kdf/crypto_kdf.c =====================

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_primitive() -> *const c_char {
    static PRIMITIVE: &[u8] = b"blake2b\0";
    PRIMITIVE.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_bytes_min() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_bytes_max() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_contextbytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_derive_from_key(
    subkey: *mut u8,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const u8,
) -> c_int {
    crypto_kdf_blake2b_derive_from_key(subkey, subkey_len, subkey_id, ctx, key)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

// ===================== crypto_kdf/blake2b/kdf_blake2b.c =====================

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_blake2b_bytes_min() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_blake2b_bytes_max() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_blake2b_contextbytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_blake2b_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_blake2b_derive_from_key(
    subkey: *mut u8,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const u8,
) -> c_int {
    let mut ctx_padded = [0u8; 16];
    let mut salt = [0u8; 16];

    core::ptr::copy_nonoverlapping(ctx as *const u8, ctx_padded.as_mut_ptr(), 8);
    // ctx_padded[8..] already zero from initialization (matches memset).

    store64_le(salt.as_mut_ptr(), subkey_id);
    // salt[8..] already zero from initialization (matches memset).

    if subkey_len < 16 || subkey_len > 64 {
        set_errno(EINVAL);
        return -1;
    }
    crypto_generichash_blake2b_salt_personal(
        subkey,
        subkey_len,
        core::ptr::null(),
        0,
        key,
        32,
        salt.as_ptr(),
        ctx_padded.as_ptr(),
    )
}

#[inline]
unsafe fn store64_le(dst: *mut u8, mut w: u64) {
    *dst.add(0) = w as u8;
    w >>= 8;
    *dst.add(1) = w as u8;
    w >>= 8;
    *dst.add(2) = w as u8;
    w >>= 8;
    *dst.add(3) = w as u8;
    w >>= 8;
    *dst.add(4) = w as u8;
    w >>= 8;
    *dst.add(5) = w as u8;
    w >>= 8;
    *dst.add(6) = w as u8;
    w >>= 8;
    *dst.add(7) = w as u8;
}

// ===================== crypto_kdf/hkdf/kdf_hkdf_sha256.c =====================

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_init(
    state: *mut crypto_kdf_hkdf_sha256_state,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_init(&mut (*state).st, salt, salt_len)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_update(
    state: *mut crypto_kdf_hkdf_sha256_state,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_update(&mut (*state).st, ikm, ikm_len as u64)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_final(
    state: *mut crypto_kdf_hkdf_sha256_state,
    prk: *mut u8,
) -> c_int {
    crypto_auth_hmacsha256_final(&mut (*state).st, prk);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_kdf_hkdf_sha256_state>(),
    );
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract(
    prk: *mut u8,
    salt: *const u8,
    salt_len: usize,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    let mut state: crypto_kdf_hkdf_sha256_state = core::mem::zeroed();

    crypto_kdf_hkdf_sha256_extract_init(&mut state, salt, salt_len);
    crypto_kdf_hkdf_sha256_extract_update(&mut state, ikm, ikm_len);

    crypto_kdf_hkdf_sha256_extract_final(&mut state, prk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keygen(prk: *mut u8) {
    randombytes_buf(prk, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    let mut st: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut tmp = [0u8; 32];
    let mut i: usize = 0;
    let left: usize;
    let mut counter: u8 = 1;

    if out_len > (0xffusize * 32) {
        set_errno(EINVAL);
        return -1;
    }
    while i + 32 <= out_len {
        crypto_auth_hmacsha256_init(&mut st, prk, 32);
        if i != 0 {
            crypto_auth_hmacsha256_update(&mut st, out.add(i - 32), 32);
        }
        crypto_auth_hmacsha256_update(&mut st, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha256_update(&mut st, &counter as *const u8, 1);
        crypto_auth_hmacsha256_final(&mut st, out.add(i));
        counter = counter.wrapping_add(1);
        i += 32;
    }
    left = out_len & (32 - 1);
    if left != 0 {
        crypto_auth_hmacsha256_init(&mut st, prk, 32);
        if i != 0 {
            crypto_auth_hmacsha256_update(&mut st, out.add(i - 32), 32);
        }
        crypto_auth_hmacsha256_update(&mut st, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha256_update(&mut st, &counter as *const u8, 1);
        crypto_auth_hmacsha256_final(&mut st, tmp.as_mut_ptr());
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), out.add(i), left);
        sodium_memzero(tmp.as_mut_ptr() as *mut c_void, tmp.len());
    }
    sodium_memzero(
        &mut st as *mut crypto_auth_hmacsha256_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_bytes_min() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_bytes_max() -> usize {
    0xffusize * 32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_kdf_hkdf_sha256_state>()
}

// ===================== crypto_kdf/hkdf/kdf_hkdf_sha512.c =====================

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract_init(
    state: *mut crypto_kdf_hkdf_sha512_state,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha512_init(&mut (*state).st, salt, salt_len)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract_update(
    state: *mut crypto_kdf_hkdf_sha512_state,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha512_update(&mut (*state).st, ikm, ikm_len as u64)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract_final(
    state: *mut crypto_kdf_hkdf_sha512_state,
    prk: *mut u8,
) -> c_int {
    crypto_auth_hmacsha512_final(&mut (*state).st, prk);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_kdf_hkdf_sha512_state>(),
    );
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract(
    prk: *mut u8,
    salt: *const u8,
    salt_len: usize,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    let mut state: crypto_kdf_hkdf_sha512_state = core::mem::zeroed();

    crypto_kdf_hkdf_sha512_extract_init(&mut state, salt, salt_len);
    crypto_kdf_hkdf_sha512_extract_update(&mut state, ikm, ikm_len);

    crypto_kdf_hkdf_sha512_extract_final(&mut state, prk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_keygen(prk: *mut u8) {
    randombytes_buf(prk, 64);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    let mut st: crypto_auth_hmacsha512_state = core::mem::zeroed();
    let mut tmp = [0u8; 64];
    let mut i: usize = 0;
    let left: usize;
    let mut counter: u8 = 1;

    if out_len > (0xffusize * 64) {
        set_errno(EINVAL);
        return -1;
    }
    while i + 64 <= out_len {
        crypto_auth_hmacsha512_init(&mut st, prk, 64);
        if i != 0 {
            crypto_auth_hmacsha512_update(&mut st, out.add(i - 64), 64);
        }
        crypto_auth_hmacsha512_update(&mut st, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha512_update(&mut st, &counter as *const u8, 1);
        crypto_auth_hmacsha512_final(&mut st, out.add(i));
        counter = counter.wrapping_add(1);
        i += 64;
    }
    left = out_len & (64 - 1);
    if left != 0 {
        crypto_auth_hmacsha512_init(&mut st, prk, 64);
        if i != 0 {
            crypto_auth_hmacsha512_update(&mut st, out.add(i - 64), 64);
        }
        crypto_auth_hmacsha512_update(&mut st, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha512_update(&mut st, &counter as *const u8, 1);
        crypto_auth_hmacsha512_final(&mut st, tmp.as_mut_ptr());
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), out.add(i), left);
        sodium_memzero(tmp.as_mut_ptr() as *mut c_void, tmp.len());
    }
    sodium_memzero(
        &mut st as *mut crypto_auth_hmacsha512_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha512_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_keybytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_bytes_min() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_bytes_max() -> usize {
    0xffusize * 64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_statebytes() -> usize {
    core::mem::size_of::<crypto_kdf_hkdf_sha512_state>()
}
