//! Translation of `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* -------------------------------------------------------------------------- */
/* Constants from the public headers                                          */
/* -------------------------------------------------------------------------- */

const crypto_aead_chacha20poly1305_ietf_NPUBBYTES: usize = 12;
const crypto_aead_chacha20poly1305_ietf_ABYTES: usize = 16;

const crypto_core_hchacha20_OUTPUTBYTES: usize = 32;
const crypto_core_hchacha20_INPUTBYTES: usize = 16;

const crypto_aead_xchacha20poly1305_ietf_KEYBYTES: usize = 32;
const crypto_aead_xchacha20poly1305_ietf_NSECBYTES: usize = 0;
const crypto_aead_xchacha20poly1305_ietf_NPUBBYTES: usize = 24;
const crypto_aead_xchacha20poly1305_ietf_ABYTES: usize = 16;

/// `SODIUM_SIZE_MAX - crypto_aead_xchacha20poly1305_ietf_ABYTES`
const crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_aead_xchacha20poly1305_ietf_ABYTES as u64;

/* -------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)         */
/* -------------------------------------------------------------------------- */

/// `crypto_onetimeauth_poly1305_state` — `CRYPTO_ALIGN(16)`, 256 opaque bytes.
#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

extern "C" {
    /* private/chacha20_ietf_ext.h — the counter may overflow into the IV */
    fn crypto_stream_chacha20_ietf_ext(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf_ext_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int;

    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;

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

    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_misuse() -> !;
}

/* -------------------------------------------------------------------------- */

static _pad0: [u8; 16] = [0; 16];

const STREAM_POLY1305_CHUNK: c_ulonglong = 131072;

/* -------------------------------------------------------------------------- */

unsafe fn _encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];

    let _ = nsec;
    crypto_stream_chacha20_ietf_ext(
        block0.as_mut_ptr(),
        core::mem::size_of_val(&block0) as c_ulonglong,
        npub,
        k,
    );
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(
        block0.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&block0),
    );

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    {
        let mut off: c_ulonglong = 0;
        let mut ic: u32 = 1;
        /*
         * The ietf_ext counter is 32-bit and overflows into the IV past
         * ~256 GiB, which a chunk restart cannot reproduce, so oversized
         * messages make a single pass instead.
         */
        let chunk: c_ulonglong = if mlen <= 64u64 * (0xffffffffu64 - 1u64) {
            STREAM_POLY1305_CHUNK
        } else {
            mlen
        };

        while off < mlen {
            let mut cl: c_ulonglong = mlen - off;
            if cl > chunk {
                cl = chunk;
            }
            crypto_stream_chacha20_ietf_ext_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                npub,
                ic,
                k,
            );
            crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
            off = off.wrapping_add(cl);
            ic = ic.wrapping_add((cl / 64) as u32);
        }
    }
    crypto_onetimeauth_poly1305_update(
        &mut state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen as u64);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as c_ulonglong,
    );

    store64_le(slen.as_mut_ptr(), mlen as u64);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as c_ulonglong,
    );

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    if !maclen_p.is_null() {
        *maclen_p = crypto_aead_chacha20poly1305_ietf_ABYTES as c_ulonglong;
    }
    0
}

unsafe fn _decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut computed_mac = [0u8; crypto_aead_chacha20poly1305_ietf_ABYTES];
    let mlen: c_ulonglong;
    let ret: c_int;

    let _ = nsec;
    crypto_stream_chacha20_ietf_ext(
        block0.as_mut_ptr(),
        core::mem::size_of_val(&block0) as c_ulonglong,
        npub,
        k,
    );
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(
        block0.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&block0),
    );

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    mlen = clen;
    crypto_onetimeauth_poly1305_update(&mut state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen as u64);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as c_ulonglong,
    );

    store64_le(slen.as_mut_ptr(), mlen as u64);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as c_ulonglong,
    );

    crypto_onetimeauth_poly1305_final(&mut state, computed_mac.as_mut_ptr());
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(
        computed_mac.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&computed_mac),
    );
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        memset(m, 0, mlen as usize);
        return -1;
    }
    /* ACQUIRE_FENCE expands to `(void) 0` in the reference build */
    crypto_stream_chacha20_ietf_ext_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2 = [0u8; crypto_core_hchacha20_OUTPUTBYTES];
    let mut npub2 = [0u8; crypto_aead_chacha20poly1305_ietf_NPUBBYTES];
    let ret: c_int;

    crypto_core_hchacha20(k2.as_mut_ptr(), npub, k, core::ptr::null());
    memcpy(
        npub2.as_mut_ptr().add(4),
        npub.add(crypto_core_hchacha20_INPUTBYTES),
        crypto_aead_chacha20poly1305_ietf_NPUBBYTES - 4,
    );
    ret = _encrypt_detached(
        c,
        mac,
        maclen_p,
        m,
        mlen,
        ad,
        adlen,
        nsec,
        npub2.as_ptr(),
        k2.as_ptr(),
    );
    sodium_memzero(
        k2.as_mut_ptr() as *mut c_void,
        crypto_core_hchacha20_OUTPUTBYTES,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_encrypt(
    c: *mut u8,
    clen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut clen: c_ulonglong = 0;
    let ret: c_int;

    if mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ret = crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
        c,
        c.add(mlen as usize),
        core::ptr::null_mut(),
        m,
        mlen,
        ad,
        adlen,
        nsec,
        npub,
        k,
    );
    if !clen_p.is_null() {
        if ret == 0 {
            clen = mlen.wrapping_add(crypto_aead_xchacha20poly1305_ietf_ABYTES as c_ulonglong);
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2 = [0u8; crypto_core_hchacha20_OUTPUTBYTES];
    let mut npub2 = [0u8; crypto_aead_chacha20poly1305_ietf_NPUBBYTES];
    let ret: c_int;

    crypto_core_hchacha20(k2.as_mut_ptr(), npub, k, core::ptr::null());
    memcpy(
        npub2.as_mut_ptr().add(4),
        npub.add(crypto_core_hchacha20_INPUTBYTES),
        crypto_aead_chacha20poly1305_ietf_NPUBBYTES - 4,
    );
    ret = _decrypt_detached(
        m,
        nsec,
        c,
        clen,
        mac,
        ad,
        adlen,
        npub2.as_ptr(),
        k2.as_ptr(),
    );
    sodium_memzero(
        k2.as_mut_ptr() as *mut c_void,
        crypto_core_hchacha20_OUTPUTBYTES,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_decrypt(
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut mlen: c_ulonglong = 0;
    let mut ret: c_int = -1;

    if clen >= crypto_aead_xchacha20poly1305_ietf_ABYTES as c_ulonglong {
        ret = crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_xchacha20poly1305_ietf_ABYTES as c_ulonglong,
            c.add(clen as usize - crypto_aead_xchacha20poly1305_ietf_ABYTES),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_xchacha20poly1305_ietf_ABYTES as c_ulonglong;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_keybytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_npubbytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_nsecbytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_abytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_messagebytes_max() -> usize {
    crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_aead_xchacha20poly1305_ietf_KEYBYTES,
    );
}
