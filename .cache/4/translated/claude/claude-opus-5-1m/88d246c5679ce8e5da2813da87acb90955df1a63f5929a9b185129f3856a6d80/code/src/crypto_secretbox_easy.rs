//! Translation of `crypto_secretbox/crypto_secretbox_easy.c`.
//!
//! The reference build defines neither `HAVE_GCC_MEMORY_FENCES` nor
//! `HAVE_C11_MEMORY_FENCES` (no `config.h`), so `ACQUIRE_FENCE` from
//! `private/common.h` expands to `(void) 0`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* #define STREAM_POLY1305_CHUNK 131072 */
const STREAM_POLY1305_CHUNK: u64 = 131072;

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_secretbox.h                          */
/* ------------------------------------------------------------------------- */

/* #define crypto_secretbox_MACBYTES ... (16U) */
const crypto_secretbox_MACBYTES: usize = 16;
/* #define crypto_secretbox_BOXZEROBYTES ... (16U) */
const crypto_secretbox_BOXZEROBYTES: usize = 16;
/* #define crypto_secretbox_ZEROBYTES ... (BOXZEROBYTES + MACBYTES) == 32 */
const crypto_secretbox_ZEROBYTES: usize =
    crypto_secretbox_BOXZEROBYTES + crypto_secretbox_MACBYTES;
/* #define crypto_secretbox_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX - MACBYTES) */
const crypto_secretbox_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_secretbox_MACBYTES as u64;
/* #define crypto_stream_salsa20_KEYBYTES 32U */
const crypto_stream_salsa20_KEYBYTES: usize = 32;

/// `crypto_onetimeauth_poly1305_state` from
/// `include/sodium/crypto_onetimeauth_poly1305.h`:
/// `typedef struct CRYPTO_ALIGN(16) { unsigned char opaque[256]; }`.
#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_core/hsalsa20/core_hsalsa20.c */
    fn crypto_core_hsalsa20(
        out: *mut u8,
        in_: *const u8,
        k: *const u8,
        c: *const u8,
    ) -> c_int;

    /* crypto_stream/salsa20/stream_salsa20.c */
    fn crypto_stream_salsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_salsa20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;

    /* crypto_onetimeauth/poly1305/onetimeauth_poly1305.c */
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int;

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_onetimeauth_poly1305_state>::uninit();
    let state_p = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; crypto_stream_salsa20_KEYBYTES];
    let mut i: c_ulonglong;
    let mut mlen0: c_ulonglong;
    let mut m = m;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    /*
     * Allow the m and c buffers to partially overlap, by calling
     * memmove() if necessary.
     */
    if ((c as usize) > (m as usize)
        && (((c as usize) - (m as usize)) as u64) < mlen)
        || ((m as usize) > (c as usize)
            && (((m as usize) - (c as usize)) as u64) < mlen)
    {
        /* LCOV_EXCL_LINE */
        memmove(c, m, mlen as usize);
        m = c as *const u8;
    }
    memset(block0.as_mut_ptr(), 0u8, crypto_secretbox_ZEROBYTES);
    mlen0 = mlen;
    if mlen0 > (64u64 - crypto_secretbox_ZEROBYTES as u64) {
        mlen0 = 64u64 - crypto_secretbox_ZEROBYTES as u64;
    }
    i = 0;
    while i < mlen0 {
        block0[(i as usize) + crypto_secretbox_ZEROBYTES] = *m.add(i as usize);
        i += 1;
    }
    crypto_stream_salsa20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        64,
        n.add(16),
        subkey.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(state_p, block0.as_ptr());

    i = 0;
    while i < mlen0 {
        *c.add(i as usize) = block0[crypto_secretbox_ZEROBYTES + (i as usize)];
        i += 1;
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(state_p, c as *const u8, mlen0);
    {
        let mut off: c_ulonglong = mlen0;
        let mut ic: u64 = 1;

        while off < mlen {
            let mut cl: c_ulonglong = mlen - off;
            if cl > STREAM_POLY1305_CHUNK {
                cl = STREAM_POLY1305_CHUNK;
            }
            crypto_stream_salsa20_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                n.add(16),
                ic,
                subkey.as_ptr(),
            );
            crypto_onetimeauth_poly1305_update(state_p, c.add(off as usize) as *const u8, cl);
            off += cl;
            ic += cl / 64u64;
        }
    }
    sodium_memzero(
        subkey.as_mut_ptr() as *mut c_void,
        crypto_stream_salsa20_KEYBYTES,
    );

    crypto_onetimeauth_poly1305_final(state_p, mac);
    sodium_memzero(
        state_p as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_easy(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_secretbox_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_secretbox_detached(c.add(crypto_secretbox_MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; crypto_stream_salsa20_KEYBYTES];
    let mut i: c_ulonglong;
    let mut mlen0: c_ulonglong;
    let mut c = c;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    memset(block0.as_mut_ptr(), 0u8, crypto_secretbox_ZEROBYTES);
    mlen0 = clen;
    if mlen0 > (64u64 - crypto_secretbox_ZEROBYTES as u64) {
        mlen0 = 64u64 - crypto_secretbox_ZEROBYTES as u64;
    }
    i = 0;
    while i < mlen0 {
        block0[crypto_secretbox_ZEROBYTES + (i as usize)] = *c.add(i as usize);
        i += 1;
    }
    crypto_stream_salsa20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        64,
        n.add(16),
        subkey.as_ptr(),
    );
    if crypto_onetimeauth_poly1305_verify(mac, c, clen, block0.as_ptr()) != 0 {
        sodium_memzero(
            subkey.as_mut_ptr() as *mut c_void,
            crypto_stream_salsa20_KEYBYTES,
        );
        return -1;
    }
    if m.is_null() {
        sodium_memzero(
            subkey.as_mut_ptr() as *mut c_void,
            crypto_stream_salsa20_KEYBYTES,
        );
        return 0;
    }
    /* ACQUIRE_FENCE; -> (void) 0 in the reference build */

    /*
     * Allow the m and c buffers to partially overlap, by calling
     * memmove() if necessary.
     */
    if ((c as usize) > (m as usize)
        && (((c as usize) - (m as usize)) as u64) < clen)
        || ((m as usize) > (c as usize)
            && (((m as usize) - (c as usize)) as u64) < clen)
    {
        /* LCOV_EXCL_LINE */
        memmove(m, c, clen as usize);
        c = m as *const u8;
    }
    i = 0;
    while i < mlen0 {
        *m.add(i as usize) = block0[crypto_secretbox_ZEROBYTES + (i as usize)];
        i += 1;
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);
    if clen > mlen0 {
        crypto_stream_salsa20_xor_ic(
            m.add(mlen0 as usize),
            c.add(mlen0 as usize),
            clen - mlen0,
            n.add(16),
            1u64,
            subkey.as_ptr(),
        );
    }
    sodium_memzero(
        subkey.as_mut_ptr() as *mut c_void,
        crypto_stream_salsa20_KEYBYTES,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < crypto_secretbox_MACBYTES as u64 {
        return -1;
    }
    crypto_secretbox_open_detached(
        m,
        c.add(crypto_secretbox_MACBYTES),
        c,
        clen - crypto_secretbox_MACBYTES as u64,
        n,
        k,
    )
}
