//! Translation of `crypto_box/crypto_box_seal.c`.

use core::ffi::{c_int, c_void};

use crate::common::memcpy;
use crate::crypto_box::easy::{crypto_box_easy, crypto_box_open_easy};
use crate::crypto_box::{
    crypto_box_keypair, crypto_box_MESSAGEBYTES_MAX, crypto_box_NONCEBYTES,
    crypto_box_PUBLICKEYBYTES, crypto_box_SEALBYTES, crypto_box_SECRETKEYBYTES,
};
use crate::crypto_generichash::{
    crypto_generichash_final, crypto_generichash_init, crypto_generichash_state,
    crypto_generichash_update,
};
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

unsafe fn _crypto_box_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> c_int {
    let mut st: crypto_generichash_state = core::mem::zeroed();

    crypto_generichash_init(&mut st, core::ptr::null(), 0usize, crypto_box_NONCEBYTES);
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

    if mlen > crypto_box_MESSAGEBYTES_MAX as u64 {
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
    memcpy(c, epk.as_ptr(), crypto_box_PUBLICKEYBYTES);
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
pub extern "C" fn crypto_box_sealbytes() -> usize {
    crypto_box_SEALBYTES
}
