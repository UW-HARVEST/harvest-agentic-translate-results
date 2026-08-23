//! Translation of `crypto_box/crypto_box_easy.c`.

use core::ffi::{c_int, c_void};

use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::sodium_memzero;

use super::{
    crypto_box_BEFORENMBYTES, crypto_box_MACBYTES, crypto_box_MESSAGEBYTES_MAX,
    crypto_box_beforenm,
};

// Defined in crypto_secretbox/crypto_secretbox_easy.c.
unsafe extern "C" {
    fn crypto_secretbox_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_secretbox_detached(c, mac, m, mlen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mut k: [u8; crypto_box_BEFORENMBYTES] = [0; crypto_box_BEFORENMBYTES];
        let ret: c_int;

        // COMPILER_ASSERT(crypto_box_BEFORENMBYTES >= crypto_secretbox_KEYBYTES);
        const _: () = assert!(crypto_box_BEFORENMBYTES >= 32);
        if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
            return -1;
        }
        ret = crypto_box_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
        sodium_memzero(k.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k));

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        if mlen > crypto_box_MESSAGEBYTES_MAX as u64 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        crypto_box_detached_afternm(c.add(crypto_box_MACBYTES), c, m, mlen, n, k)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        if mlen > crypto_box_MESSAGEBYTES_MAX as u64 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        crypto_box_detached(c.add(crypto_box_MACBYTES), c, m, mlen, n, pk, sk)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_secretbox_open_detached(m, c, mac, clen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mut k: [u8; crypto_box_BEFORENMBYTES] = [0; crypto_box_BEFORENMBYTES];
        let ret: c_int;

        if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
            return -1;
        }
        ret = crypto_box_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
        sodium_memzero(k.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k));

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        if clen < crypto_box_MACBYTES as u64 {
            return -1;
        }
        crypto_box_open_detached_afternm(
            m,
            c.add(crypto_box_MACBYTES),
            c,
            clen - crypto_box_MACBYTES as u64,
            n,
            k,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        if clen < crypto_box_MACBYTES as u64 {
            return -1;
        }
        crypto_box_open_detached(
            m,
            c.add(crypto_box_MACBYTES),
            c,
            clen - crypto_box_MACBYTES as u64,
            n,
            pk,
            sk,
        )
    }
}
