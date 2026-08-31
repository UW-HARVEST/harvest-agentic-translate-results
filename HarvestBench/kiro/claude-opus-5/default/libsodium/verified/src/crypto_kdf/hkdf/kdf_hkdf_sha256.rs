//! Translation of c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c

use core::ffi::{c_char, c_int};

// Local repr(C) copy of crypto_hash_sha256_state (rule 4).
#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

// crypto_auth_hmacsha256_state.
#[repr(C)]
struct crypto_auth_hmacsha256_state {
    ictx: crypto_hash_sha256_state,
    octx: crypto_hash_sha256_state,
}

// crypto_kdf_hkdf_sha256_state from include/sodium/crypto_kdf_hkdf_sha256.h.
#[repr(C)]
pub struct crypto_kdf_hkdf_sha256_state {
    st: crypto_auth_hmacsha256_state,
}

// crypto_auth_hmacsha256_BYTES == 32U
const crypto_auth_hmacsha256_BYTES: usize = 32;
// crypto_kdf_hkdf_sha256_KEYBYTES == crypto_auth_hmacsha256_BYTES == 32
const crypto_kdf_hkdf_sha256_KEYBYTES: usize = 32;
// crypto_kdf_hkdf_sha256_BYTES_MIN == 0U
const crypto_kdf_hkdf_sha256_BYTES_MIN: usize = 0;
// crypto_kdf_hkdf_sha256_BYTES_MAX == 0xff * crypto_auth_hmacsha256_BYTES
const crypto_kdf_hkdf_sha256_BYTES_MAX: usize = 0xff * crypto_auth_hmacsha256_BYTES;

extern "C" {
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
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_init(
    state: *mut crypto_kdf_hkdf_sha256_state,
    salt: *const u8,
    salt_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_init(core::ptr::addr_of_mut!((*state).st), salt, salt_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_update(
    state: *mut crypto_kdf_hkdf_sha256_state,
    ikm: *const u8,
    ikm_len: usize,
) -> c_int {
    crypto_auth_hmacsha256_update(core::ptr::addr_of_mut!((*state).st), ikm, ikm_len as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_extract_final(
    state: *mut crypto_kdf_hkdf_sha256_state,
    prk: *mut u8,
) -> c_int {
    crypto_auth_hmacsha256_final(core::ptr::addr_of_mut!((*state).st), prk);
    sodium_memzero(
        state as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_kdf_hkdf_sha256_state>(),
    );

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
    let mut state = core::mem::MaybeUninit::<crypto_kdf_hkdf_sha256_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_kdf_hkdf_sha256_extract_init(state, salt, salt_len);
    crypto_kdf_hkdf_sha256_extract_update(state, ikm, ikm_len);

    crypto_kdf_hkdf_sha256_extract_final(state, prk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_keygen(prk: *mut u8) {
    randombytes_buf(prk as *mut core::ffi::c_void, crypto_kdf_hkdf_sha256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_hkdf_sha256_expand(
    out: *mut u8,
    out_len: usize,
    ctx: *const c_char,
    ctx_len: usize,
    prk: *const u8,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<crypto_auth_hmacsha256_state>::uninit();
    let st = st.as_mut_ptr();
    let mut tmp: [u8; crypto_auth_hmacsha256_BYTES] = [0; crypto_auth_hmacsha256_BYTES];
    let mut i: usize;
    let left: usize;
    let mut counter: u8 = 1u8;

    if out_len > crypto_kdf_hkdf_sha256_BYTES_MAX {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    i = 0;
    while i + crypto_auth_hmacsha256_BYTES <= out_len {
        crypto_auth_hmacsha256_init(st, prk, crypto_kdf_hkdf_sha256_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha256_update(
                st,
                out.add(i - crypto_auth_hmacsha256_BYTES),
                crypto_auth_hmacsha256_BYTES as u64,
            );
        }
        crypto_auth_hmacsha256_update(st, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha256_update(st, &counter, 1u64);
        crypto_auth_hmacsha256_final(st, out.add(i));
        counter = counter.wrapping_add(1);

        i += crypto_auth_hmacsha256_BYTES;
    }
    left = out_len & (crypto_auth_hmacsha256_BYTES - 1);
    if left != 0 {
        crypto_auth_hmacsha256_init(st, prk, crypto_kdf_hkdf_sha256_KEYBYTES);
        if i != 0 {
            crypto_auth_hmacsha256_update(
                st,
                out.add(i - crypto_auth_hmacsha256_BYTES),
                crypto_auth_hmacsha256_BYTES as u64,
            );
        }
        crypto_auth_hmacsha256_update(st, ctx as *const u8, ctx_len as u64);
        crypto_auth_hmacsha256_update(st, &counter, 1u64);
        crypto_auth_hmacsha256_final(st, tmp.as_mut_ptr());
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), out.add(i), left);
        sodium_memzero(
            tmp.as_mut_ptr() as *mut core::ffi::c_void,
            core::mem::size_of::<[u8; crypto_auth_hmacsha256_BYTES]>(),
        );
    }
    sodium_memzero(
        st as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );

    0
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
