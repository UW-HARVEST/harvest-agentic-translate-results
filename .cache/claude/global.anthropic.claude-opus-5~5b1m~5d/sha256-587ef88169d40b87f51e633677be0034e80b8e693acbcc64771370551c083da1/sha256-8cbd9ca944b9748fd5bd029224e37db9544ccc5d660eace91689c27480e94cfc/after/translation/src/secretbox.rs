//! Translation of:
//! * `crypto_secretbox/crypto_secretbox.c`
//! * `crypto_secretbox/crypto_secretbox_easy.c`
//! * `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c`
//! * `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::csys::{memmove, memset};

/// `crypto_onetimeauth_poly1305_state`: `CRYPTO_ALIGN(16) unsigned char opaque[256]`
/// (`include/sodium/crypto_onetimeauth_poly1305.h`). Declared locally per the
/// cross-module-call convention; layout must match `poly1305.rs`.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

extern "C" {
    // ---- crypto_stream_xsalsa20 (include/sodium/crypto_stream_xsalsa20.h) ----
    fn crypto_stream_xsalsa20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    // ---- crypto_stream_salsa20 (include/sodium/crypto_stream_salsa20.h) ----
    fn crypto_stream_salsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_salsa20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;

    // ---- crypto_stream_chacha20 (include/sodium/crypto_stream_chacha20.h) ----
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

    // ---- crypto_core_hsalsa20 / crypto_core_hchacha20 ----
    fn crypto_core_hsalsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;

    // ---- crypto_onetimeauth_poly1305 ----
    fn crypto_onetimeauth_poly1305(
        out: *mut u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int;

    // ---- misc ----
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// =====================================================================
// crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k);
    crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c);
    for i in 0..16isize {
        *c.offset(i) = 0;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut subkey = [0u8; 32];

    if clen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20(subkey.as_mut_ptr(), 32, n, k);
    if crypto_onetimeauth_poly1305_verify(c.add(16), c.add(32), clen - 32, subkey.as_ptr()) != 0 {
        return -1;
    }
    crypto_stream_xsalsa20_xor(m, c, clen, n, k);
    for i in 0..32isize {
        *m.offset(i) = 0;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_zerobytes() -> usize {
    16 + 16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_boxzerobytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_macbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

// =====================================================================
// crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_detached(
    c: *mut u8,
    mac: *mut u8,
    mut m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut mlen0: u64;

    crypto_core_hchacha20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    if (c as usize > m as usize && (c as usize - m as usize) < mlen as usize)
        || (m as usize > c as usize && (m as usize - c as usize) < mlen as usize)
    {
        memmove(c as *mut c_void, m as *const c_void, mlen as usize);
        m = c;
    }
    memset(block0.as_mut_ptr() as *mut c_void, 0, 32);
    mlen0 = mlen;
    if mlen0 > 64 - 32 {
        mlen0 = 64 - 32;
    }
    for i in 0..mlen0 {
        block0[(i + 32) as usize] = *m.add(i as usize);
    }
    crypto_stream_chacha20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        mlen0 + 32,
        n.add(16),
        subkey.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());

    for i in 0..mlen0 {
        *c.add(i as usize) = block0[(32 + i) as usize];
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

    crypto_onetimeauth_poly1305_update(&mut state, c, mlen);
    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    crypto_secretbox_xchacha20poly1305_detached(c.add(16), c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_open_detached(
    m: *mut u8,
    mut c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut mlen0: u64;

    crypto_core_hchacha20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    memset(block0.as_mut_ptr() as *mut c_void, 0, 32);
    mlen0 = clen;
    if mlen0 > 64 - 32 {
        mlen0 = 64 - 32;
    }
    for i in 0..mlen0 {
        block0[(32 + i) as usize] = *c.add(i as usize);
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
    if (c as usize > m as usize && (c as usize - m as usize) < clen as usize)
        || (m as usize > c as usize && (m as usize - c as usize) < clen as usize)
    {
        memmove(m as *mut c_void, c as *const c_void, clen as usize);
        c = m;
    }
    for i in 0..mlen0 {
        *m.add(i as usize) = block0[(32 + i) as usize];
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

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < 16 {
        return -1;
    }
    crypto_secretbox_xchacha20poly1305_open_detached(m, c.add(16), c, clen - 16, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_macbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

// =====================================================================
// crypto_secretbox/crypto_secretbox_easy.c
//
// These re-implement the xsalsa20poly1305 NaCl "box" transform directly
// (hsalsa20 -> salsa20), rather than calling the deprecated
// crypto_secretbox_xsalsa20poly1305{,_open} functions above.
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_detached(
    c: *mut u8,
    mac: *mut u8,
    mut m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut mlen0: u64;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    if (c as usize > m as usize && (c as usize - m as usize) < mlen as usize)
        || (m as usize > c as usize && (m as usize - c as usize) < mlen as usize)
    {
        memmove(c as *mut c_void, m as *const c_void, mlen as usize);
        m = c;
    }
    memset(block0.as_mut_ptr() as *mut c_void, 0, 32);
    mlen0 = mlen;
    if mlen0 > 64 - 32 {
        mlen0 = 64 - 32;
    }
    for i in 0..mlen0 {
        block0[(i + 32) as usize] = *m.add(i as usize);
    }
    crypto_stream_salsa20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        64,
        n.add(16),
        subkey.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());

    for i in 0..mlen0 {
        *c.add(i as usize) = block0[(32 + i) as usize];
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(&mut state, c, mlen0);
    {
        let mut off: u64 = mlen0;
        let mut ic: u64 = 1;

        while off < mlen {
            let mut cl: u64 = mlen - off;
            if cl > 131072 {
                cl = 131072;
            }
            crypto_stream_salsa20_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                n.add(16),
                ic,
                subkey.as_ptr(),
            );
            crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
            off += cl;
            ic += cl / 64;
        }
    }
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, subkey.len());

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    crypto_secretbox_detached(c.add(16), c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_open_detached(
    m: *mut u8,
    mut c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut mlen0: u64;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    memset(block0.as_mut_ptr() as *mut c_void, 0, 32);
    mlen0 = clen;
    if mlen0 > 64 - 32 {
        mlen0 = 64 - 32;
    }
    for i in 0..mlen0 {
        block0[(32 + i) as usize] = *c.add(i as usize);
    }
    crypto_stream_salsa20_xor(
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
    if (c as usize > m as usize && (c as usize - m as usize) < clen as usize)
        || (m as usize > c as usize && (m as usize - c as usize) < clen as usize)
    {
        memmove(m as *mut c_void, c as *const c_void, clen as usize);
        c = m;
    }
    for i in 0..mlen0 {
        *m.add(i as usize) = block0[(32 + i) as usize];
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());
    if clen > mlen0 {
        crypto_stream_salsa20_xor_ic(
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

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < 16 {
        return -1;
    }
    crypto_secretbox_open_detached(m, c.add(16), c, clen - 16, n, k)
}

// =====================================================================
// crypto_secretbox/crypto_secretbox.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_zerobytes() -> usize {
    16 + 16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_boxzerobytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_macbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_primitive() -> *const c_char {
    b"xsalsa20poly1305\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretbox_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
