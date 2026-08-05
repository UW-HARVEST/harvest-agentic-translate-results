//! Translated from crypto_secretbox/{crypto_secretbox.c, crypto_secretbox_easy.c,
//! xsalsa20poly1305/secretbox_xsalsa20poly1305.c,
//! xchacha20poly1305/secretbox_xchacha20poly1305.c}
use crate::primitives::poly1305::crypto_onetimeauth_poly1305_state;
use core::ffi::{c_char, c_void};

extern "C" {
    fn crypto_core_hsalsa20(out: *mut u8, input: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_core_hchacha20(out: *mut u8, input: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_stream_xsalsa20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_salsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_salsa20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305(out: *mut u8, input: *const u8, inlen: u64, k: *const u8) -> i32;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        input: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        input: *const u8,
        inlen: u64,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> i32;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const KEYBYTES: usize = 32;
const NONCEBYTES: usize = 24;
const ZEROBYTES: usize = 32;
const BOXZEROBYTES: usize = 16;
const MACBYTES: usize = 16;
const MESSAGEBYTES_MAX: u64 = u64::MAX; // xsalsa20 messagebytes_max is SODIUM_SIZE_MAX
const SECRETBOX_MESSAGEBYTES_MAX: u64 = u64::MAX - MACBYTES as u64;
const STREAM_POLY1305_CHUNK: u64 = 131072;

// ======== xsalsa20poly1305 ========

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k);
    crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c);
    for i in 0..16 {
        *c.add(i) = 0;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut subkey = [0u8; 32];
    if clen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20(subkey.as_mut_ptr(), 32, n, k);
    if crypto_onetimeauth_poly1305_verify(c.add(16), c.add(32), clen - 32, subkey.as_ptr()) != 0 {
        return -1;
    }
    crypto_stream_xsalsa20_xor(m, c, clen, n, k);
    for i in 0..32 {
        *m.add(i) = 0;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xsalsa20poly1305_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xsalsa20poly1305_noncebytes() -> usize {
    NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xsalsa20poly1305_zerobytes() -> usize {
    ZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xsalsa20poly1305_boxzerobytes() -> usize {
    BOXZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xsalsa20poly1305_macbytes() -> usize {
    MACBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xsalsa20poly1305_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

// ======== crypto_secretbox_easy.c ========

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut m = m;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    let cu = c as usize;
    let mu = m as usize;
    if (cu > mu && (cu - mu) < mlen as usize) || (mu > cu && (mu - cu) < mlen as usize) {
        core::ptr::copy(m, c, mlen as usize);
        m = c;
    }
    core::ptr::write_bytes(block0.as_mut_ptr(), 0, ZEROBYTES);
    let mut mlen0 = mlen;
    if mlen0 > 64 - ZEROBYTES as u64 {
        mlen0 = 64 - ZEROBYTES as u64;
    }
    for i in 0..mlen0 {
        block0[(i + ZEROBYTES as u64) as usize] = *m.add(i as usize);
    }
    crypto_stream_salsa20_xor(block0.as_mut_ptr(), block0.as_ptr(), 64, n.add(16), subkey.as_ptr());
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());

    for i in 0..mlen0 {
        *c.add(i as usize) = block0[ZEROBYTES + i as usize];
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut state, c, mlen0);
    {
        let mut off = mlen0;
        let mut ic: u64 = 1;
        while off < mlen {
            let mut cl = mlen - off;
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
            crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
            off += cl;
            ic += cl / 64;
        }
    }
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen > SECRETBOX_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_secretbox_detached(c.add(MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut c = c;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    core::ptr::write_bytes(block0.as_mut_ptr(), 0, ZEROBYTES);
    let mut mlen0 = clen;
    if mlen0 > 64 - ZEROBYTES as u64 {
        mlen0 = 64 - ZEROBYTES as u64;
    }
    for i in 0..mlen0 {
        block0[ZEROBYTES + i as usize] = *c.add(i as usize);
    }
    crypto_stream_salsa20_xor(block0.as_mut_ptr(), block0.as_ptr(), 64, n.add(16), subkey.as_ptr());
    if crypto_onetimeauth_poly1305_verify(mac, c, clen, block0.as_ptr()) != 0 {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);
        return -1;
    }
    if m.is_null() {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);
        return 0;
    }

    let cu = c as usize;
    let mu = m as usize;
    if (cu > mu && (cu - mu) < clen as usize) || (mu > cu && (mu - cu) < clen as usize) {
        core::ptr::copy(c, m, clen as usize);
        c = m;
    }
    for i in 0..mlen0 {
        *m.add(i as usize) = block0[ZEROBYTES + i as usize];
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);
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
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen < MACBYTES as u64 {
        return -1;
    }
    crypto_secretbox_open_detached(m, c.add(MACBYTES), c, clen - MACBYTES as u64, n, k)
}

// ======== crypto_secretbox.c dispatch ========

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_noncebytes() -> usize {
    NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_zerobytes() -> usize {
    ZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_boxzerobytes() -> usize {
    BOXZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_macbytes() -> usize {
    MACBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_messagebytes_max() -> usize {
    SECRETBOX_MESSAGEBYTES_MAX as usize
}

static SECRETBOX_PRIMITIVE: &[u8] = b"xsalsa20poly1305\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_primitive() -> *const c_char {
    SECRETBOX_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

// ======== xchacha20poly1305 ========

const X_KEYBYTES: usize = 32;
const X_NONCEBYTES: usize = 24;
const X_MACBYTES: usize = 16;
const X_ZEROBYTES: usize = 32;
const X_MESSAGEBYTES_MAX: u64 = u64::MAX - X_MACBYTES as u64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut m = m;

    crypto_core_hchacha20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    let cu = c as usize;
    let mu = m as usize;
    if (cu > mu && (cu - mu) < mlen as usize) || (mu > cu && (mu - cu) < mlen as usize) {
        core::ptr::copy(m, c, mlen as usize);
        m = c;
    }
    core::ptr::write_bytes(block0.as_mut_ptr(), 0, X_ZEROBYTES);
    let mut mlen0 = mlen;
    if mlen0 > 64 - X_ZEROBYTES as u64 {
        mlen0 = 64 - X_ZEROBYTES as u64;
    }
    for i in 0..mlen0 {
        block0[(i + X_ZEROBYTES as u64) as usize] = *m.add(i as usize);
    }
    crypto_stream_chacha20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        mlen0 + X_ZEROBYTES as u64,
        n.add(16),
        subkey.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());

    for i in 0..mlen0 {
        *c.add(i as usize) = block0[X_ZEROBYTES + i as usize];
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);
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
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);

    crypto_onetimeauth_poly1305_update(&mut state, c, mlen);
    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen > X_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_secretbox_xchacha20poly1305_detached(c.add(X_MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut block0 = [0u8; 64];
    let mut subkey = [0u8; 32];
    let mut c = c;

    crypto_core_hchacha20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    core::ptr::write_bytes(block0.as_mut_ptr(), 0, X_ZEROBYTES);
    let mut mlen0 = clen;
    if mlen0 > 64 - X_ZEROBYTES as u64 {
        mlen0 = 64 - X_ZEROBYTES as u64;
    }
    for i in 0..mlen0 {
        block0[X_ZEROBYTES + i as usize] = *c.add(i as usize);
    }
    crypto_stream_chacha20_xor(block0.as_mut_ptr(), block0.as_ptr(), 64, n.add(16), subkey.as_ptr());
    if crypto_onetimeauth_poly1305_verify(mac, c, clen, block0.as_ptr()) != 0 {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);
        return -1;
    }
    if m.is_null() {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);
        return 0;
    }

    let cu = c as usize;
    let mu = m as usize;
    if (cu > mu && (cu - mu) < clen as usize) || (mu > cu && (mu - cu) < clen as usize) {
        core::ptr::copy(c, m, clen as usize);
        c = m;
    }
    for i in 0..mlen0 {
        *m.add(i as usize) = block0[X_ZEROBYTES + i as usize];
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
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen < X_MACBYTES as u64 {
        return -1;
    }
    crypto_secretbox_xchacha20poly1305_open_detached(m, c.add(X_MACBYTES), c, clen - X_MACBYTES as u64, n, k)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xchacha20poly1305_keybytes() -> usize {
    X_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xchacha20poly1305_noncebytes() -> usize {
    X_NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xchacha20poly1305_macbytes() -> usize {
    X_MACBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_xchacha20poly1305_messagebytes_max() -> usize {
    X_MESSAGEBYTES_MAX as usize
}
