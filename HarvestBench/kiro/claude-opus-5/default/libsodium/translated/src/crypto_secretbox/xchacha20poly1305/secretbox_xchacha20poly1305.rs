//! Translation of c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c

use core::ffi::{c_int, c_void};

use crate::sodium::core::sodium_misuse;

// crypto_onetimeauth_poly1305_state: public 256-byte aligned struct.
#[repr(C, align(16))]
struct CryptoOnetimeauthPoly1305State {
    opaque: [u8; 256],
}

// #define crypto_secretbox_xchacha20poly1305_ZEROBYTES 32U (local to the .c)
const CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES: usize = 32;
const CRYPTO_SECRETBOX_XCHACHA20POLY1305_KEYBYTES: usize = 32;
const CRYPTO_SECRETBOX_XCHACHA20POLY1305_NONCEBYTES: usize = 24;
const CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES: usize = 16;
// (crypto_stream_xchacha20_MESSAGEBYTES_MAX - MACBYTES) == SODIUM_SIZE_MAX - MACBYTES
const CRYPTO_SECRETBOX_XCHACHA20POLY1305_MESSAGEBYTES_MAX: u64 =
    (usize::MAX as u64) - CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES as u64;
const CRYPTO_ONETIMEAUTH_POLY1305_KEYBYTES: usize = 32;
const CRYPTO_STREAM_CHACHA20_KEYBYTES: usize = 32;

extern "C" {
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut CryptoOnetimeauthPoly1305State,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut CryptoOnetimeauthPoly1305State,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut CryptoOnetimeauthPoly1305State,
        out: *mut u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_detached(
    c: *mut u8,
    mac: *mut u8,
    mut m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<CryptoOnetimeauthPoly1305State>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; CRYPTO_STREAM_CHACHA20_KEYBYTES];
    let mut i: u64;
    let mut mlen0: u64;

    crypto_core_hchacha20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    // Allow the m and c buffers to partially overlap, via memmove().
    if ((c as usize) > (m as usize) && (c as usize) - (m as usize) < mlen as usize)
        || ((m as usize) > (c as usize) && (m as usize) - (c as usize) < mlen as usize)
    {
        /* LCOV_EXCL_LINE */
        memmove(c as *mut c_void, m as *const c_void, mlen as usize);
        m = c;
    }
    memset(
        block0.as_mut_ptr() as *mut c_void,
        0,
        CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES,
    );
    // COMPILER_ASSERT(64U >= crypto_secretbox_xchacha20poly1305_ZEROBYTES);
    mlen0 = mlen;
    if mlen0 > (64 - CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES) as u64 {
        mlen0 = (64 - CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES) as u64;
    }
    i = 0;
    while i < mlen0 {
        block0[(i as usize) + CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES] = *m.add(i as usize);
        i += 1;
    }
    crypto_stream_chacha20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        mlen0 + CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES as u64,
        n.add(16),
        subkey.as_ptr(),
    );
    // COMPILER_ASSERT(ZEROBYTES >= crypto_onetimeauth_poly1305_KEYBYTES);
    let _ = CRYPTO_ONETIMEAUTH_POLY1305_KEYBYTES;
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());

    i = 0;
    while i < mlen0 {
        *c.add(i as usize) = block0[CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES + (i as usize)];
        i += 1;
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());
    if mlen > mlen0 {
        crypto_stream_chacha20_xor_ic(
            c.add(mlen0 as usize),
            m.add(mlen0 as usize),
            mlen - mlen0,
            n.add(16),
            1,
            subkey.as_ptr(),
        );
    }
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, subkey.len());

    crypto_onetimeauth_poly1305_update(state, c, mlen);
    crypto_onetimeauth_poly1305_final(state, mac);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<CryptoOnetimeauthPoly1305State>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > CRYPTO_SECRETBOX_XCHACHA20POLY1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_secretbox_xchacha20poly1305_detached(
        c.add(CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES),
        c,
        m,
        mlen,
        n,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_open_detached(
    m: *mut u8,
    mut c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; CRYPTO_STREAM_CHACHA20_KEYBYTES];
    let mut i: u64;
    let mut mlen0: u64;

    crypto_core_hchacha20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    memset(
        block0.as_mut_ptr() as *mut c_void,
        0,
        CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES,
    );
    mlen0 = clen;
    if mlen0 > (64 - CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES) as u64 {
        mlen0 = (64 - CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES) as u64;
    }
    i = 0;
    while i < mlen0 {
        block0[CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES + (i as usize)] = *c.add(i as usize);
        i += 1;
    }
    crypto_stream_chacha20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        64,
        n.add(16),
        subkey.as_ptr(),
    );
    if crypto_onetimeauth_poly1305_verify(mac, c, clen, block0.as_ptr()) != 0 {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, subkey.len());
        return -1;
    }
    if m.is_null() {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, subkey.len());
        return 0;
    }
    // ACQUIRE_FENCE;

    // Allow the m and c buffers to partially overlap, via memmove().
    if ((c as usize) > (m as usize) && (c as usize) - (m as usize) < clen as usize)
        || ((m as usize) > (c as usize) && (m as usize) - (c as usize) < clen as usize)
    {
        /* LCOV_EXCL_LINE */
        memmove(m as *mut c_void, c as *const c_void, clen as usize);
        c = m;
    }
    i = 0;
    while i < mlen0 {
        *m.add(i as usize) = block0[CRYPTO_SECRETBOX_XCHACHA20POLY1305_ZEROBYTES + (i as usize)];
        i += 1;
    }
    if clen > mlen0 {
        crypto_stream_chacha20_xor_ic(
            m.add(mlen0 as usize),
            c.add(mlen0 as usize),
            clen - mlen0,
            n.add(16),
            1,
            subkey.as_ptr(),
        );
    }
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, subkey.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES as u64 {
        return -1;
    }
    crypto_secretbox_xchacha20poly1305_open_detached(
        m,
        c.add(CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES),
        c,
        clen - CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES as u64,
        n,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_keybytes() -> usize {
    CRYPTO_SECRETBOX_XCHACHA20POLY1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_noncebytes() -> usize {
    CRYPTO_SECRETBOX_XCHACHA20POLY1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_macbytes() -> usize {
    CRYPTO_SECRETBOX_XCHACHA20POLY1305_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_messagebytes_max() -> usize {
    CRYPTO_SECRETBOX_XCHACHA20POLY1305_MESSAGEBYTES_MAX as usize
}
