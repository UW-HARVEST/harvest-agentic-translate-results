//! Translation of c_src/libsodium/crypto_box/crypto_box_easy.c

use core::ffi::c_int;

use crate::sodium::core::sodium_misuse;

// crypto_box_* constants (crypto_box.h -> crypto_box_curve25519xsalsa20poly1305.h)
const crypto_box_MACBYTES: usize = 16; // crypto_box_curve25519xsalsa20poly1305_MACBYTES
// crypto_box_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - MACBYTES; SODIUM_SIZE_MAX == SIZE_MAX (u64 on x86_64)
const crypto_box_MESSAGEBYTES_MAX: u64 = u64::MAX - crypto_box_MACBYTES as u64;

extern "C" {
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
    fn crypto_box_beforenm(k: *mut u8, pk: *const u8, sk: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
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
    crypto_secretbox_detached(c, mac, m, mlen, n, k)
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
    let mut k: [u8; crypto_box_BEFORENMBYTES] = [0; crypto_box_BEFORENMBYTES];
    let ret: c_int;

    // COMPILER_ASSERT(crypto_box_BEFORENMBYTES >= crypto_secretbox_KEYBYTES);
    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of_val(&k));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_box_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_box_detached_afternm(c.add(crypto_box_MACBYTES), c, m, mlen, n, k)
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
    if mlen > crypto_box_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_box_detached(c.add(crypto_box_MACBYTES), c, m, mlen, n, pk, sk)
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
    crypto_secretbox_open_detached(m, c, mac, clen, n, k)
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
    let mut k: [u8; crypto_box_BEFORENMBYTES] = [0; crypto_box_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut core::ffi::c_void, core::mem::size_of_val(&k));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
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

const crypto_box_BEFORENMBYTES: usize = 32; // crypto_box_curve25519xsalsa20poly1305_BEFORENMBYTES
