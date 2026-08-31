//! Translation of c_src/libsodium/crypto_kdf/blake2b/kdf_blake2b.c

use crate::common::store64_le;
use core::ffi::{c_char, c_int};

// Constants (see crypto_kdf_blake2b.h and crypto_generichash_blake2b.h).
const crypto_kdf_blake2b_BYTES_MIN: usize = 16;
const crypto_kdf_blake2b_BYTES_MAX: usize = 64;
const crypto_kdf_blake2b_CONTEXTBYTES: usize = 8;
const crypto_kdf_blake2b_KEYBYTES: usize = 32;
const crypto_generichash_blake2b_PERSONALBYTES: usize = 16;
const crypto_generichash_blake2b_SALTBYTES: usize = 16;

extern "C" {
    fn crypto_generichash_blake2b_salt_personal(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
        salt: *const u8,
        personal: *const u8,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_bytes_min() -> usize {
    crypto_kdf_blake2b_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_bytes_max() -> usize {
    crypto_kdf_blake2b_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_contextbytes() -> usize {
    crypto_kdf_blake2b_CONTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_keybytes() -> usize {
    crypto_kdf_blake2b_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_derive_from_key(
    subkey: *mut u8,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const u8,
) -> c_int {
    let mut ctx_padded: [u8; crypto_generichash_blake2b_PERSONALBYTES] =
        [0; crypto_generichash_blake2b_PERSONALBYTES];
    let mut salt: [u8; crypto_generichash_blake2b_SALTBYTES] =
        [0; crypto_generichash_blake2b_SALTBYTES];

    core::ptr::copy_nonoverlapping(
        ctx as *const u8,
        ctx_padded.as_mut_ptr(),
        crypto_kdf_blake2b_CONTEXTBYTES,
    );
    core::ptr::write_bytes(
        ctx_padded.as_mut_ptr().add(crypto_kdf_blake2b_CONTEXTBYTES),
        0,
        core::mem::size_of::<[u8; crypto_generichash_blake2b_PERSONALBYTES]>()
            - crypto_kdf_blake2b_CONTEXTBYTES,
    );
    store64_le(salt.as_mut_ptr(), subkey_id);
    core::ptr::write_bytes(
        salt.as_mut_ptr().add(8),
        0,
        core::mem::size_of::<[u8; crypto_generichash_blake2b_SALTBYTES]>() - 8,
    );
    if subkey_len < crypto_kdf_blake2b_BYTES_MIN || subkey_len > crypto_kdf_blake2b_BYTES_MAX {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    crypto_generichash_blake2b_salt_personal(
        subkey,
        subkey_len,
        core::ptr::null(),
        0,
        key,
        crypto_kdf_blake2b_KEYBYTES,
        salt.as_ptr(),
        ctx_padded.as_ptr(),
    )
}
