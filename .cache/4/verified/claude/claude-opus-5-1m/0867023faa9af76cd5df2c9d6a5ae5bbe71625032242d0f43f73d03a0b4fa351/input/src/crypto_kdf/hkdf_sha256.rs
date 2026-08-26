//! Translation of `crypto_kdf/hkdf/kdf_hkdf_sha256.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::common::{set_errno, EINVAL};
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::sodium_memzero;

// Constants from include/sodium/crypto_auth_hmacsha256.h
const crypto_auth_hmacsha256_BYTES: usize = 32;

// Constants from include/sodium/crypto_kdf_hkdf_sha256.h
pub const crypto_kdf_hkdf_sha256_KEYBYTES: usize = crypto_auth_hmacsha256_BYTES;
pub const crypto_kdf_hkdf_sha256_BYTES_MIN: usize = 0;
pub const crypto_kdf_hkdf_sha256_BYTES_MAX: usize = 0xff * crypto_auth_hmacsha256_BYTES;

/// ```c
/// typedef struct crypto_hash_sha256_state {
///     uint32_t state[8];
///     uint64_t count;
///     uint8_t  buf[64];
/// } crypto_hash_sha256_state;
/// ```
/// `sizeof == 104`, `_Alignof == 8`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/// ```c
/// typedef struct crypto_auth_hmacsha256_state {
///     crypto_hash_sha256_state ictx;
///     crypto_hash_sha256_state octx;
/// } crypto_auth_hmacsha256_state;
/// ```
/// `sizeof == 208`, `_Alignof == 8`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

/// ```c
/// typedef struct crypto_kdf_hkdf_sha256_state {
///     crypto_auth_hmacsha256_state st;
/// } crypto_kdf_hkdf_sha256_state;
/// ```
#[repr(C)]
pub struct crypto_kdf_hkdf_sha256_state {
    pub st: crypto_auth_hmacsha256_state,
}

// Defined in crypto_auth/hmacsha256/auth_hmacsha256.c.
unsafe extern "C" {
    fn crypto_auth_hmacsha256_init(
        state: *mut crypto_auth_hmacsha256_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha256_update(
        state: *mut crypto_auth_hmacsha256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha256_final(
        state: *mut crypto_auth_hmacsha256_state,
        out: *mut u8,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_init(
    state: *mut crypto_kdf_hkdf_sha256_state,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    unsafe { crypto_auth_hmacsha256_init(core::ptr::addr_of_mut!((*state).st), salt, salt_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_update(
    state: *mut crypto_kdf_hkdf_sha256_state,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    unsafe {
        crypto_auth_hmacsha256_update(
            core::ptr::addr_of_mut!((*state).st),
            ikm,
            ikm_len as u64,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_final(
    state: *mut crypto_kdf_hkdf_sha256_state,
    prk: *mut u8,
) -> c_int {
    unsafe {
        crypto_auth_hmacsha256_final(core::ptr::addr_of_mut!((*state).st), prk);
        sodium_memzero(
            state as *mut c_void,
            core::mem::size_of::<crypto_kdf_hkdf_sha256_state>(),
        );
    }

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
    unsafe {
        let mut state = core::mem::MaybeUninit::<crypto_kdf_hkdf_sha256_state>::uninit();

        crypto_kdf_hkdf_sha256_extract_init(state.as_mut_ptr(), salt, salt_len);
        crypto_kdf_hkdf_sha256_extract_update(state.as_mut_ptr(), ikm, ikm_len);

        crypto_kdf_hkdf_sha256_extract_final(state.as_mut_ptr(), prk)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keygen(prk: *mut u8) {
    randombytes_buf(prk as *mut c_void, crypto_kdf_hkdf_sha256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    unsafe {
        let mut st = core::mem::MaybeUninit::<crypto_auth_hmacsha256_state>::uninit();
        let mut tmp = [0u8; crypto_auth_hmacsha256_BYTES];
        let mut i: usize;
        let left: usize;
        let mut counter: u8 = 1;

        if out_len > crypto_kdf_hkdf_sha256_BYTES_MAX {
            set_errno(EINVAL);
            return -1;
        }
        i = 0;
        while i + crypto_auth_hmacsha256_BYTES <= out_len {
            crypto_auth_hmacsha256_init(
                st.as_mut_ptr(),
                prk,
                crypto_kdf_hkdf_sha256_KEYBYTES,
            );
            if i != 0 {
                crypto_auth_hmacsha256_update(
                    st.as_mut_ptr(),
                    out.add(i - crypto_auth_hmacsha256_BYTES),
                    crypto_auth_hmacsha256_BYTES as u64,
                );
            }
            crypto_auth_hmacsha256_update(st.as_mut_ptr(), ctx as *const u8, ctx_len as u64);
            crypto_auth_hmacsha256_update(st.as_mut_ptr(), &counter, 1);
            crypto_auth_hmacsha256_final(st.as_mut_ptr(), out.add(i));
            counter = counter.wrapping_add(1);

            i += crypto_auth_hmacsha256_BYTES;
        }
        left = out_len & (crypto_auth_hmacsha256_BYTES - 1);
        if left != 0 {
            crypto_auth_hmacsha256_init(
                st.as_mut_ptr(),
                prk,
                crypto_kdf_hkdf_sha256_KEYBYTES,
            );
            if i != 0 {
                crypto_auth_hmacsha256_update(
                    st.as_mut_ptr(),
                    out.add(i - crypto_auth_hmacsha256_BYTES),
                    crypto_auth_hmacsha256_BYTES as u64,
                );
            }
            crypto_auth_hmacsha256_update(st.as_mut_ptr(), ctx as *const u8, ctx_len as u64);
            crypto_auth_hmacsha256_update(st.as_mut_ptr(), &counter, 1);
            crypto_auth_hmacsha256_final(st.as_mut_ptr(), tmp.as_mut_ptr());
            core::ptr::copy_nonoverlapping(tmp.as_ptr(), out.add(i), left);
            sodium_memzero(tmp.as_mut_ptr() as *mut c_void, tmp.len());
        }
        sodium_memzero(
            st.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<crypto_auth_hmacsha256_state>(),
        );

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keybytes() -> usize {
    crypto_kdf_hkdf_sha256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_bytes_min() -> usize {
    crypto_kdf_hkdf_sha256_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_bytes_max() -> usize {
    crypto_kdf_hkdf_sha256_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_kdf_hkdf_sha256_state>()
}
