//! Translation of c_src/libsodium/crypto_box/crypto_box_seal.c

use core::ffi::{c_int, c_void};

use crate::sodium::core::sodium_misuse;

// crypto_box_* constants (crypto_box.h -> crypto_box_curve25519xsalsa20poly1305.h)
const crypto_box_NONCEBYTES: usize = 24; // ..._NONCEBYTES
const crypto_box_PUBLICKEYBYTES: usize = 32; // ..._PUBLICKEYBYTES
const crypto_box_SECRETKEYBYTES: usize = 32; // ..._SECRETKEYBYTES
const crypto_box_MACBYTES: usize = 16; // ..._MACBYTES
const crypto_box_SEALBYTES: usize = crypto_box_PUBLICKEYBYTES + crypto_box_MACBYTES;
// crypto_box_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - MACBYTES; SODIUM_SIZE_MAX == SIZE_MAX (u64 on x86_64)
const crypto_box_MESSAGEBYTES_MAX: u64 = u64::MAX - crypto_box_MACBYTES as u64;

// crypto_generichash_state == crypto_generichash_blake2b_state:
// #pragma pack(1) struct CRYPTO_ALIGN(64) { unsigned char opaque[384]; }
#[repr(C, align(64))]
struct crypto_generichash_state {
    opaque: [u8; 384],
}

extern "C" {
    fn crypto_generichash_init(
        state: *mut crypto_generichash_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_update(
        state: *mut crypto_generichash_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_final(
        state: *mut crypto_generichash_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    fn crypto_box_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn crypto_box_easy(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
    fn crypto_box_open_easy(
        m: *mut u8,
        c: *const u8,
        clen: u64,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

unsafe fn _crypto_box_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> c_int {
    let mut st: crypto_generichash_state = crypto_generichash_state { opaque: [0; 384] };

    crypto_generichash_init(&mut st, core::ptr::null(), 0u32 as usize, crypto_box_NONCEBYTES);
    crypto_generichash_update(&mut st, pk1, crypto_box_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut st, pk2, crypto_box_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut st, nonce, crypto_box_NONCEBYTES);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seal(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    let mut nonce: [u8; crypto_box_NONCEBYTES] = [0; crypto_box_NONCEBYTES];
    let mut epk: [u8; crypto_box_PUBLICKEYBYTES] = [0; crypto_box_PUBLICKEYBYTES];
    let mut esk: [u8; crypto_box_SECRETKEYBYTES] = [0; crypto_box_SECRETKEYBYTES];
    let ret: c_int;

    if mlen > crypto_box_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_box_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
    ret = crypto_box_easy(
        c.add(crypto_box_PUBLICKEYBYTES),
        m,
        mlen,
        nonce.as_ptr(),
        pk,
        esk.as_ptr(),
    );
    core::ptr::copy_nonoverlapping(epk.as_ptr(), c, crypto_box_PUBLICKEYBYTES);
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&esk));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut nonce: [u8; crypto_box_NONCEBYTES] = [0; crypto_box_NONCEBYTES];

    if clen < crypto_box_SEALBYTES as u64 {
        return -1;
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), c, pk);

    // COMPILER_ASSERT(crypto_box_PUBLICKEYBYTES < crypto_box_SEALBYTES);
    crypto_box_open_easy(
        m,
        c.add(crypto_box_PUBLICKEYBYTES),
        clen - crypto_box_PUBLICKEYBYTES as u64,
        nonce.as_ptr(),
        c,
        sk,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_sealbytes() -> usize {
    crypto_box_SEALBYTES
}
