//! Translation of crypto_kdf/blake2b/kdf_blake2b.c
//! and include/sodium/crypto_kdf_blake2b.h

use core::ffi::{c_char, c_int, c_uchar};

use crate::common::{memcpy, memset, store64_le};
use crate::crypto_generichash::blake2b::{
    crypto_generichash_blake2b_PERSONALBYTES, crypto_generichash_blake2b_SALTBYTES,
    crypto_generichash_blake2b_salt_personal,
};

pub const crypto_kdf_blake2b_BYTES_MIN: usize = 16;
pub const crypto_kdf_blake2b_BYTES_MAX: usize = 64;
pub const crypto_kdf_blake2b_CONTEXTBYTES: usize = 8;
pub const crypto_kdf_blake2b_KEYBYTES: usize = 32;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_bytes_min() -> usize {
    crypto_kdf_blake2b_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_bytes_max() -> usize {
    crypto_kdf_blake2b_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_contextbytes() -> usize {
    crypto_kdf_blake2b_CONTEXTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_blake2b_keybytes() -> usize {
    crypto_kdf_blake2b_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_blake2b_derive_from_key(
    subkey: *mut c_uchar,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const c_uchar,
) -> c_int {
    let mut ctx_padded: [c_uchar; crypto_generichash_blake2b_PERSONALBYTES] =
        [0; crypto_generichash_blake2b_PERSONALBYTES];
    let mut salt: [c_uchar; crypto_generichash_blake2b_SALTBYTES] =
        [0; crypto_generichash_blake2b_SALTBYTES];

    memcpy(
        ctx_padded.as_mut_ptr(),
        ctx as *const c_uchar,
        crypto_kdf_blake2b_CONTEXTBYTES,
    );
    memset(
        ctx_padded.as_mut_ptr().add(crypto_kdf_blake2b_CONTEXTBYTES),
        0,
        core::mem::size_of_val(&ctx_padded) - crypto_kdf_blake2b_CONTEXTBYTES,
    );
    store64_le(salt.as_mut_ptr(), subkey_id);
    memset(
        salt.as_mut_ptr().add(8),
        0,
        core::mem::size_of_val(&salt) - 8,
    );
    if subkey_len < crypto_kdf_blake2b_BYTES_MIN || subkey_len > crypto_kdf_blake2b_BYTES_MAX {
        crate::set_errno(crate::EINVAL);
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
