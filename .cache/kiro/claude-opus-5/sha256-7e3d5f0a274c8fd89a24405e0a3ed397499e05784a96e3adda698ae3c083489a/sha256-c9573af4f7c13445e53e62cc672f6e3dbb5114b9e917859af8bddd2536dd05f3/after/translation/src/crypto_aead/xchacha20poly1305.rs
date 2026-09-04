/*
 * Translation of
 *   crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c
 *   include/sodium/crypto_aead_xchacha20poly1305.h
 */

use core::ffi::{c_int, c_void};

use crate::common::{memcpy, memset, store64_le, SODIUM_SIZE_MAX};
use crate::crypto_aead::chacha20poly1305::{
    crypto_aead_chacha20poly1305_ietf_ABYTES, crypto_aead_chacha20poly1305_ietf_NPUBBYTES,
};
use crate::crypto_core::hchacha20::crypto_core_hchacha20;
use crate::crypto_onetimeauth::poly1305::{
    crypto_onetimeauth_poly1305_final, crypto_onetimeauth_poly1305_init,
    crypto_onetimeauth_poly1305_state, crypto_onetimeauth_poly1305_update,
};
use crate::crypto_stream::chacha20::{
    crypto_stream_chacha20_ietf_ext, crypto_stream_chacha20_ietf_ext_xor_ic,
};
use crate::crypto_verify::crypto_verify_16;
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

/* crypto_core_hchacha20 sizes */
const crypto_core_hchacha20_OUTPUTBYTES: usize = 32;
const crypto_core_hchacha20_INPUTBYTES: usize = 16;

pub const crypto_aead_xchacha20poly1305_ietf_KEYBYTES: usize = 32;
pub const crypto_aead_xchacha20poly1305_ietf_NSECBYTES: usize = 0;
pub const crypto_aead_xchacha20poly1305_ietf_NPUBBYTES: usize = 24;
pub const crypto_aead_xchacha20poly1305_ietf_ABYTES: usize = 16;
/* SODIUM_SIZE_MAX - ABYTES */
pub const crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX: u64 =
    (SODIUM_SIZE_MAX - crypto_aead_xchacha20poly1305_ietf_ABYTES) as u64;

static _pad0: [u8; 16] = [0u8; 16];

const STREAM_POLY1305_CHUNK: u64 = 131072;

unsafe fn _encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut state: crypto_onetimeauth_poly1305_state =
        crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0: [u8; 64] = [0u8; 64];
    let mut slen: [u8; 8] = [0u8; 8];

    let _ = nsec;
    crypto_stream_chacha20_ietf_ext(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    {
        let mut off: u64 = 0;
        let mut ic: u32 = 1;
        /*
         * The ietf_ext counter is 32-bit and overflows into the IV past
         * ~256 GiB, which a chunk restart cannot reproduce, so oversized
         * messages make a single pass instead.
         */
        let chunk: u64 = if mlen <= 64u64 * (0xffffffffu64 - 1u64) {
            STREAM_POLY1305_CHUNK
        } else {
            mlen
        };

        while off < mlen {
            let mut cl: u64 = mlen - off;
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
            off += cl;
            ic = ic.wrapping_add((cl / 64) as u32);
        }
    }
    crypto_onetimeauth_poly1305_update(
        &mut state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    if !maclen_p.is_null() {
        *maclen_p = crypto_aead_chacha20poly1305_ietf_ABYTES as u64;
    }
    0
}

unsafe fn _decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut state: crypto_onetimeauth_poly1305_state =
        crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0: [u8; 64] = [0u8; 64];
    let mut slen: [u8; 8] = [0u8; 8];
    let mut computed_mac: [u8; crypto_aead_chacha20poly1305_ietf_ABYTES] =
        [0u8; crypto_aead_chacha20poly1305_ietf_ABYTES];
    let mlen: u64;
    let ret: c_int;

    let _ = nsec;
    crypto_stream_chacha20_ietf_ext(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

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

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(&mut state, computed_mac.as_mut_ptr());
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(computed_mac.as_mut_ptr() as *mut c_void, computed_mac.len());
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        memset(m, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_ietf_ext_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; crypto_core_hchacha20_OUTPUTBYTES] = [0u8; crypto_core_hchacha20_OUTPUTBYTES];
    let mut npub2: [u8; crypto_aead_chacha20poly1305_ietf_NPUBBYTES] =
        [0u8; crypto_aead_chacha20poly1305_ietf_NPUBBYTES];
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
    clen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut clen: u64 = 0;
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
            clen = mlen + crypto_aead_xchacha20poly1305_ietf_ABYTES as u64;
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
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; crypto_core_hchacha20_OUTPUTBYTES] = [0u8; crypto_core_hchacha20_OUTPUTBYTES];
    let mut npub2: [u8; crypto_aead_chacha20poly1305_ietf_NPUBBYTES] =
        [0u8; crypto_aead_chacha20poly1305_ietf_NPUBBYTES];
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
    mlen_p: *mut u64,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut mlen: u64 = 0;
    let mut ret: c_int = -1;

    if clen >= crypto_aead_xchacha20poly1305_ietf_ABYTES as u64 {
        ret = crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_xchacha20poly1305_ietf_ABYTES as u64,
            c.add((clen - crypto_aead_xchacha20poly1305_ietf_ABYTES as u64) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_xchacha20poly1305_ietf_ABYTES as u64;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_xchacha20poly1305_ietf_keybytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_xchacha20poly1305_ietf_npubbytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_NPUBBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_xchacha20poly1305_ietf_nsecbytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_NSECBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_xchacha20poly1305_ietf_abytes() -> usize {
    crypto_aead_xchacha20poly1305_ietf_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_xchacha20poly1305_ietf_messagebytes_max() -> usize {
    crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_aead_xchacha20poly1305_ietf_KEYBYTES,
    );
}
