//! Translation of c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c

use core::ffi::{c_int, c_void};

// crypto_generichash_blake2b_state, packed with CRYPTO_ALIGN(64) (rule 4).
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

// blake2b_state (packed) — only used as an opaque cast target here.
#[repr(C, packed)]
struct blake2b_state {
    h: [u64; 8],
    t: [u64; 2],
    f: [u64; 2],
    buf: [u8; 2 * 128],
    buflen: usize,
    last_node: u8,
}

// enum blake2b_constant
const BLAKE2B_OUTBYTES: usize = 64;
const BLAKE2B_KEYBYTES: usize = 64;

extern "C" {
    // quirks.h renames (see private/quirks.h)
    fn _sodium_blake2b(
        out: *mut u8,
        in_: *const c_void,
        key: *const c_void,
        outlen: u8,
        inlen: u64,
        keylen: u8,
    ) -> c_int;
    fn _sodium_blake2b_salt_personal(
        out: *mut u8,
        in_: *const c_void,
        key: *const c_void,
        outlen: u8,
        inlen: u64,
        keylen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_init(S: *mut blake2b_state, outlen: u8) -> c_int;
    fn _sodium_blake2b_init_salt_personal(
        S: *mut blake2b_state,
        outlen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_init_key(
        S: *mut blake2b_state,
        outlen: u8,
        key: *const c_void,
        keylen: u8,
    ) -> c_int;
    fn _sodium_blake2b_init_key_salt_personal(
        S: *mut blake2b_state,
        outlen: u8,
        key: *const c_void,
        keylen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_update(S: *mut blake2b_state, in_: *const u8, inlen: u64) -> c_int;
    fn _sodium_blake2b_final(S: *mut blake2b_state, out: *mut u8, outlen: u8) -> c_int;
    fn _sodium_blake2b_pick_best_implementation() -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        // inlen > UINT64_MAX is always false for a u64
        return -1;
    }
    // assert(outlen <= UINT8_MAX);
    // assert(keylen <= UINT8_MAX);

    _sodium_blake2b(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
        keylen as u8,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_salt_personal(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    // assert(outlen <= UINT8_MAX);
    // assert(keylen <= UINT8_MAX);

    _sodium_blake2b_salt_personal(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_init(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    // assert(outlen <= UINT8_MAX);
    // assert(keylen <= UINT8_MAX);
    // COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state);
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init(state as *mut c_void as *mut blake2b_state, outlen as u8) != 0 {
            return -1; /* LCOV_EXCL_LINE */
        }
    } else if _sodium_blake2b_init_key(
        state as *mut c_void as *mut blake2b_state,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
    ) != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_init_salt_personal(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    // assert(outlen <= UINT8_MAX);
    // assert(keylen <= UINT8_MAX);
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init_salt_personal(
            state as *mut c_void as *mut blake2b_state,
            outlen as u8,
            salt as *const c_void,
            personal as *const c_void,
        ) != 0
        {
            return -1; /* LCOV_EXCL_LINE */
        }
    } else if _sodium_blake2b_init_key_salt_personal(
        state as *mut c_void as *mut blake2b_state,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    ) != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_update(
    state: *mut crypto_generichash_blake2b_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    _sodium_blake2b_update(state as *mut c_void as *mut blake2b_state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    // assert(outlen <= UINT8_MAX);
    _sodium_blake2b_final(state as *mut c_void as *mut blake2b_state, out, outlen as u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_generichash_blake2b_pick_best_implementation() -> c_int {
    _sodium_blake2b_pick_best_implementation()
}
