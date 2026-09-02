//! Translation of crypto_kdf/hkdf/kdf_hkdf_sha256.c
//! and include/sodium/crypto_kdf_hkdf_sha256.h

use core::ffi::{c_char, c_int, c_uchar, c_void};

use crate::common::memcpy;
use crate::crypto_auth::hmacsha256::{
    crypto_auth_hmacsha256_BYTES, crypto_auth_hmacsha256_final, crypto_auth_hmacsha256_init,
    crypto_auth_hmacsha256_state, crypto_auth_hmacsha256_update,
};
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::sodium_memzero;

pub const crypto_kdf_hkdf_sha256_KEYBYTES: usize = crypto_auth_hmacsha256_BYTES;
pub const crypto_kdf_hkdf_sha256_BYTES_MIN: usize = 0;
pub const crypto_kdf_hkdf_sha256_BYTES_MAX: usize = 0xff * crypto_auth_hmacsha256_BYTES;

#[repr(C)]
pub struct crypto_kdf_hkdf_sha256_state {
    pub st: crypto_auth_hmacsha256_state,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_init(
    state: *mut crypto_kdf_hkdf_sha256_state,
    salt: *const c_uchar,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_init(&mut (*state).st, salt, salt_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_update(
    state: *mut crypto_kdf_hkdf_sha256_state,
    ikm: *const c_uchar,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_update(&mut (*state).st, ikm, ikm_len as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_final(
    state: *mut crypto_kdf_hkdf_sha256_state,
    prk: *mut c_uchar,
) -> c_int {
    crypto_auth_hmacsha256_final(&mut (*state).st, prk);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_kdf_hkdf_sha256_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract(
    prk: *mut c_uchar,
    salt: *const c_uchar,
    salt_len: usize,
    ikm: *const c_uchar,
    ikm_len: usize,
) -> c_int {
    let mut state: crypto_kdf_hkdf_sha256_state = core::mem::zeroed();

    crypto_kdf_hkdf_sha256_extract_init(&mut state, salt, salt_len);
    crypto_kdf_hkdf_sha256_extract_update(&mut state, ikm, ikm_len);

    crypto_kdf_hkdf_sha256_extract_final(&mut state, prk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keygen(prk: *mut c_uchar) {
    randombytes_buf(prk as *mut c_void, crypto_kdf_hkdf_sha256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_expand(
    out: *mut c_uchar,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const c_uchar,
) -> c_int {
    let mut st: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut tmp: [c_uchar; crypto_auth_hmacsha256_BYTES] = [0; crypto_auth_hmacsha256_BYTES];
    let mut i: usize;
    let left: usize;
    let mut counter: c_uchar = 1u8;

    if out_len > crypto_kdf_hkdf_sha256_BYTES_MAX {
        crate::set_errno(crate::EINVAL);
        return -1;
    }
    i = 0;
    while i + crypto_auth_hmacsha256_BYTES <= out_len {
        crypto_auth_hmacsha256_init(&mut st, prk, crypto_kdf_hkdf_sha256_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha256_update(
                &mut st,
                out.add(i - crypto_auth_hmacsha256_BYTES),
                crypto_auth_hmacsha256_BYTES as u64,
            );
        }
        crypto_auth_hmacsha256_update(&mut st, ctx as *const c_uchar, ctx_len as u64);
        crypto_auth_hmacsha256_update(&mut st, &counter, 1u64);
        crypto_auth_hmacsha256_final(&mut st, out.add(i));
        counter = counter.wrapping_add(1);

        i += crypto_auth_hmacsha256_BYTES;
    }
    left = out_len & (crypto_auth_hmacsha256_BYTES - 1);
    if left != 0 {
        crypto_auth_hmacsha256_init(&mut st, prk, crypto_kdf_hkdf_sha256_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha256_update(
                &mut st,
                out.add(i - crypto_auth_hmacsha256_BYTES),
                crypto_auth_hmacsha256_BYTES as u64,
            );
        }
        crypto_auth_hmacsha256_update(&mut st, ctx as *const c_uchar, ctx_len as u64);
        crypto_auth_hmacsha256_update(&mut st, &counter, 1u64);
        crypto_auth_hmacsha256_final(&mut st, tmp.as_mut_ptr());
        memcpy(out.add(i), tmp.as_ptr(), left);
        sodium_memzero(tmp.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&tmp));
    }
    sodium_memzero(
        &mut st as *mut crypto_auth_hmacsha256_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_keybytes() -> usize {
    crypto_kdf_hkdf_sha256_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_bytes_min() -> usize {
    crypto_kdf_hkdf_sha256_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_bytes_max() -> usize {
    crypto_kdf_hkdf_sha256_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_hkdf_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_kdf_hkdf_sha256_state>()
}
