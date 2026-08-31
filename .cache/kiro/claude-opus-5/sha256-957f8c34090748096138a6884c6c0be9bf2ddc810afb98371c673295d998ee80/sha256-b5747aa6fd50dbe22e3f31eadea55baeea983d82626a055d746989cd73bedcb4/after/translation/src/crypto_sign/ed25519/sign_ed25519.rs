//! Translation of c_src/libsodium/crypto_sign/ed25519/sign_ed25519.c

use core::ffi::c_int;

// crypto_hash_sha512_BYTES
const crypto_hash_sha512_BYTES: usize = 64;

// crypto_sign_ed25519_* constants (crypto_sign_ed25519.h).
const crypto_sign_ed25519_BYTES: usize = 64;
const crypto_sign_ed25519_SEEDBYTES: usize = 32;
const crypto_sign_ed25519_PUBLICKEYBYTES: usize = 32;
const crypto_sign_ed25519_SECRETKEYBYTES: usize = 32 + 32;
const SODIUM_SIZE_MAX: usize = usize::MAX;
const crypto_sign_ed25519_MESSAGEBYTES_MAX: usize =
    SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES;

// Local repr(C) copy of crypto_hash_sha512_state (rule 4).
#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

// Public struct from include/sodium/crypto_sign_ed25519.h.
#[repr(C)]
pub struct crypto_sign_ed25519ph_state {
    hs: crypto_hash_sha512_state,
}

extern "C" {
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(
        state: *mut crypto_hash_sha512_state,
        out: *mut u8,
    ) -> c_int;

    // Defined in ref10/sign.c and ref10/open.c (not renamed by quirks.h).
    fn _crypto_sign_ed25519_detached(
        sig: *mut u8,
        siglen_p: *mut u64,
        m: *const u8,
        mlen: u64,
        sk: *const u8,
        prehashed: c_int,
    ) -> c_int;
    fn _crypto_sign_ed25519_verify_detached(
        sig: *const u8,
        m: *const u8,
        mlen: u64,
        pk: *const u8,
        prehashed: c_int,
    ) -> c_int;

    // libc
    fn memmove(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize)
        -> *mut core::ffi::c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_ed25519ph_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_bytes() -> usize {
    crypto_sign_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seedbytes() -> usize {
    crypto_sign_ed25519_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_publickeybytes() -> usize {
    crypto_sign_ed25519_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_secretkeybytes() -> usize {
    crypto_sign_ed25519_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_messagebytes_max() -> usize {
    crypto_sign_ed25519_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_seed(
    seed: *mut u8,
    sk: *const u8,
) -> c_int {
    memmove(
        seed as *mut core::ffi::c_void,
        sk as *const core::ffi::c_void,
        crypto_sign_ed25519_SEEDBYTES,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_pk(
    pk: *mut u8,
    sk: *const u8,
) -> c_int {
    memmove(
        pk as *mut core::ffi::c_void,
        sk.add(crypto_sign_ed25519_SEEDBYTES) as *const core::ffi::c_void,
        crypto_sign_ed25519_PUBLICKEYBYTES,
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_init(
    state: *mut crypto_sign_ed25519ph_state,
) -> c_int {
    crypto_hash_sha512_init(core::ptr::addr_of_mut!((*state).hs));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_update(
    state: *mut crypto_sign_ed25519ph_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    crypto_hash_sha512_update(core::ptr::addr_of_mut!((*state).hs), m, mlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_create(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    let mut ph: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512_final(core::ptr::addr_of_mut!((*state).hs), ph.as_mut_ptr());

    _crypto_sign_ed25519_detached(
        sig,
        siglen_p,
        ph.as_ptr(),
        core::mem::size_of::<[u8; crypto_hash_sha512_BYTES]>() as u64,
        sk,
        1,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_verify(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    let mut ph: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512_final(core::ptr::addr_of_mut!((*state).hs), ph.as_mut_ptr());

    _crypto_sign_ed25519_verify_detached(
        sig,
        ph.as_ptr(),
        core::mem::size_of::<[u8; crypto_hash_sha512_BYTES]>() as u64,
        pk,
        1,
    )
}
