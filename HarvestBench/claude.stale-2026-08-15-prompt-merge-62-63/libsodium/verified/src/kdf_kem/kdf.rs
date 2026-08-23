// Translation of crypto_kdf/crypto_kdf.c, crypto_kdf/blake2b/kdf_blake2b.c,
// and crypto_kdf/hkdf/{kdf_hkdf_sha256,kdf_hkdf_sha512}.c

use core::ffi::{c_char, c_int, c_void};

use crate::common::store64_le;

// crypto_kdf constants (blake2b primitive)
const CRYPTO_KDF_BYTES_MIN: usize = 16;
const CRYPTO_KDF_BYTES_MAX: usize = 64;
const CRYPTO_KDF_CONTEXTBYTES: usize = 8;
const CRYPTO_KDF_KEYBYTES: usize = 32;
const CRYPTO_KDF_PRIMITIVE: &[u8] = b"blake2b\0";

const BLAKE2B_PERSONALBYTES: usize = 16;
const BLAKE2B_SALTBYTES: usize = 16;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
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
    // HMAC-SHA256 (P3)
    fn crypto_auth_hmacsha256_init(state: *mut c_void, key: *const u8, keylen: usize) -> c_int;
    fn crypto_auth_hmacsha256_update(state: *mut c_void, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_auth_hmacsha256_final(state: *mut c_void, out: *mut u8) -> c_int;
    // HMAC-SHA512 (defined by the aead package; we call, do not redefine)
    fn crypto_auth_hmacsha512_init(state: *mut c_void, key: *const u8, keylen: usize) -> c_int;
    fn crypto_auth_hmacsha512_update(state: *mut c_void, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_auth_hmacsha512_final(state: *mut c_void, out: *mut u8) -> c_int;
}

// errno set for EINVAL == 22 on Linux.
#[inline]
unsafe fn set_einval() {
    extern "C" {
        fn __errno_location() -> *mut c_int;
    }
    *__errno_location() = 22;
}

// ---------------- crypto_kdf (dispatch) ----------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_primitive() -> *const c_char {
    CRYPTO_KDF_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_bytes_min() -> usize {
    CRYPTO_KDF_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_bytes_max() -> usize {
    CRYPTO_KDF_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_contextbytes() -> usize {
    CRYPTO_KDF_CONTEXTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_keybytes() -> usize {
    CRYPTO_KDF_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_derive_from_key(
    subkey: *mut u8,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const u8,
) -> c_int {
    crypto_kdf_blake2b_derive_from_key(subkey, subkey_len, subkey_id, ctx, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_KDF_KEYBYTES);
}

// ---------------- crypto_kdf_blake2b ----------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_bytes_min() -> usize {
    CRYPTO_KDF_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_bytes_max() -> usize {
    CRYPTO_KDF_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_contextbytes() -> usize {
    CRYPTO_KDF_CONTEXTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_keybytes() -> usize {
    CRYPTO_KDF_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_derive_from_key(
    subkey: *mut u8,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const u8,
) -> c_int {
    let mut ctx_padded = [0u8; BLAKE2B_PERSONALBYTES];
    let mut salt = [0u8; BLAKE2B_SALTBYTES];

    core::ptr::copy_nonoverlapping(ctx as *const u8, ctx_padded.as_mut_ptr(), CRYPTO_KDF_CONTEXTBYTES);
    // remaining bytes of ctx_padded already zero
    store64_le(&mut salt[0..8], subkey_id);
    // salt[8..] already zero

    if subkey_len < CRYPTO_KDF_BYTES_MIN || subkey_len > CRYPTO_KDF_BYTES_MAX {
        set_einval();
        return -1;
    }
    crypto_generichash_blake2b_salt_personal(
        subkey,
        subkey_len,
        core::ptr::null(),
        0,
        key,
        CRYPTO_KDF_KEYBYTES,
        salt.as_ptr(),
        ctx_padded.as_ptr(),
    )
}

// ---------------- HKDF ----------------
//
// State layouts mirror the C structs (a single embedded HMAC state).
// hmacsha256 state = 2 * crypto_hash_sha256_state; sha256 state = 8*u32 + u64 + 64
// hmacsha512 state = 2 * crypto_hash_sha512_state; sha512 state = 8*u64 + 2*u64 + 128

const HMACSHA256_BYTES: usize = 32;
const HMACSHA512_BYTES: usize = 64;
const HKDF_SHA256_KEYBYTES: usize = HMACSHA256_BYTES;
const HKDF_SHA512_KEYBYTES: usize = HMACSHA512_BYTES;
const HKDF_SHA256_BYTES_MAX: usize = 0xff * HMACSHA256_BYTES;
const HKDF_SHA512_BYTES_MAX: usize = 0xff * HMACSHA512_BYTES;

#[repr(C)]
struct Sha256State {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

#[repr(C)]
struct HmacSha256State {
    ictx: Sha256State,
    octx: Sha256State,
}

#[repr(C)]
struct Sha512State {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

#[repr(C)]
struct HmacSha512State {
    ictx: Sha512State,
    octx: Sha512State,
}

// hkdf state structs = { hmac state }
#[repr(C)]
struct HkdfSha256State {
    st: HmacSha256State,
}

#[repr(C)]
struct HkdfSha512State {
    st: HmacSha512State,
}

// ---- HKDF-SHA256 ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_init(
    state: *mut HkdfSha256State,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_init(&mut (*state).st as *mut _ as *mut c_void, salt, salt_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_update(
    state: *mut HkdfSha256State,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_update(&mut (*state).st as *mut _ as *mut c_void, ikm, ikm_len as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_final(
    state: *mut HkdfSha256State,
    prk: *mut u8,
) -> c_int {
    crypto_auth_hmacsha256_final(&mut (*state).st as *mut _ as *mut c_void, prk);
    sodium_memzero(state as *mut c_void, core::mem::size_of::<HkdfSha256State>());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract(
    prk: *mut u8,
    salt: *const u8,
    salt_len: usize,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<HkdfSha256State>::uninit();
    let st = state.as_mut_ptr();
    crypto_kdf_hkdf_sha256_extract_init(st, salt, salt_len);
    crypto_kdf_hkdf_sha256_extract_update(st, ikm, ikm_len);
    crypto_kdf_hkdf_sha256_extract_final(st, prk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keygen(prk: *mut u8) {
    randombytes_buf(prk as *mut c_void, HKDF_SHA256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<HmacSha256State>::uninit();
    let stp = st.as_mut_ptr() as *mut c_void;
    let mut tmp = [0u8; HMACSHA256_BYTES];
    let mut counter: u8 = 1;

    if out_len > HKDF_SHA256_BYTES_MAX {
        set_einval();
        return -1;
    }
    let mut i: usize = 0;
    while i + HMACSHA256_BYTES <= out_len {
        crypto_auth_hmacsha256_init(stp, prk, HKDF_SHA256_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha256_update(stp, out.add(i - HMACSHA256_BYTES), HMACSHA256_BYTES as u64);
        }
        crypto_auth_hmacsha256_update(stp, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha256_update(stp, &counter, 1);
        crypto_auth_hmacsha256_final(stp, out.add(i));
        counter = counter.wrapping_add(1);
        i += HMACSHA256_BYTES;
    }
    let left = out_len & (HMACSHA256_BYTES - 1);
    if left != 0 {
        crypto_auth_hmacsha256_init(stp, prk, HKDF_SHA256_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha256_update(stp, out.add(i - HMACSHA256_BYTES), HMACSHA256_BYTES as u64);
        }
        crypto_auth_hmacsha256_update(stp, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha256_update(stp, &counter, 1);
        crypto_auth_hmacsha256_final(stp, tmp.as_mut_ptr());
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), out.add(i), left);
        sodium_memzero(tmp.as_mut_ptr() as *mut c_void, tmp.len());
    }
    sodium_memzero(stp, core::mem::size_of::<HmacSha256State>());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_keybytes() -> usize {
    HKDF_SHA256_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_bytes_min() -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_bytes_max() -> usize {
    HKDF_SHA256_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_statebytes() -> usize {
    core::mem::size_of::<HkdfSha256State>()
}

// ---- HKDF-SHA512 ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract_init(
    state: *mut HkdfSha512State,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha512_init(&mut (*state).st as *mut _ as *mut c_void, salt, salt_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract_update(
    state: *mut HkdfSha512State,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha512_update(&mut (*state).st as *mut _ as *mut c_void, ikm, ikm_len as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract_final(
    state: *mut HkdfSha512State,
    prk: *mut u8,
) -> c_int {
    crypto_auth_hmacsha512_final(&mut (*state).st as *mut _ as *mut c_void, prk);
    sodium_memzero(state as *mut c_void, core::mem::size_of::<HkdfSha512State>());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_extract(
    prk: *mut u8,
    salt: *const u8,
    salt_len: usize,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<HkdfSha512State>::uninit();
    let st = state.as_mut_ptr();
    crypto_kdf_hkdf_sha512_extract_init(st, salt, salt_len);
    crypto_kdf_hkdf_sha512_extract_update(st, ikm, ikm_len);
    crypto_kdf_hkdf_sha512_extract_final(st, prk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_keygen(prk: *mut u8) {
    randombytes_buf(prk as *mut c_void, HKDF_SHA512_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha512_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<HmacSha512State>::uninit();
    let stp = st.as_mut_ptr() as *mut c_void;
    let mut tmp = [0u8; HMACSHA512_BYTES];
    let mut counter: u8 = 1;

    if out_len > HKDF_SHA512_BYTES_MAX {
        set_einval();
        return -1;
    }
    let mut i: usize = 0;
    while i + HMACSHA512_BYTES <= out_len {
        crypto_auth_hmacsha512_init(stp, prk, HKDF_SHA512_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha512_update(stp, out.add(i - HMACSHA512_BYTES), HMACSHA512_BYTES as u64);
        }
        crypto_auth_hmacsha512_update(stp, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha512_update(stp, &counter, 1);
        crypto_auth_hmacsha512_final(stp, out.add(i));
        counter = counter.wrapping_add(1);
        i += HMACSHA512_BYTES;
    }
    let left = out_len & (HMACSHA512_BYTES - 1);
    if left != 0 {
        crypto_auth_hmacsha512_init(stp, prk, HKDF_SHA512_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha512_update(stp, out.add(i - HMACSHA512_BYTES), HMACSHA512_BYTES as u64);
        }
        crypto_auth_hmacsha512_update(stp, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha512_update(stp, &counter, 1);
        crypto_auth_hmacsha512_final(stp, tmp.as_mut_ptr());
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), out.add(i), left);
        sodium_memzero(tmp.as_mut_ptr() as *mut c_void, tmp.len());
    }
    sodium_memzero(stp, core::mem::size_of::<HmacSha512State>());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha512_keybytes() -> usize {
    HKDF_SHA512_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha512_bytes_min() -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha512_bytes_max() -> usize {
    HKDF_SHA512_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha512_statebytes() -> usize {
    core::mem::size_of::<HkdfSha512State>()
}
