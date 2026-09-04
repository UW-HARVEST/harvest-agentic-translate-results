/*
 * Translation of
 *   crypto_aead/chacha20poly1305/aead_chacha20poly1305.c
 *   include/sodium/crypto_aead_chacha20poly1305.h
 */

use core::ffi::{c_int, c_void};

use crate::common::{store64_le, SODIUM_SIZE_MAX};
use crate::crypto_onetimeauth::poly1305::{
    crypto_onetimeauth_poly1305_final, crypto_onetimeauth_poly1305_init,
    crypto_onetimeauth_poly1305_state, crypto_onetimeauth_poly1305_update,
};
use crate::crypto_stream::chacha20::{
    crypto_stream_chacha20, crypto_stream_chacha20_ietf, crypto_stream_chacha20_ietf_xor_ic,
    crypto_stream_chacha20_xor_ic,
};
use crate::crypto_verify::crypto_verify_16;
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

/* -- IETF ChaCha20-Poly1305 construction with a 96-bit nonce and a 32-bit internal counter -- */

pub const crypto_aead_chacha20poly1305_ietf_KEYBYTES: usize = 32;
pub const crypto_aead_chacha20poly1305_ietf_NSECBYTES: usize = 0;
pub const crypto_aead_chacha20poly1305_ietf_NPUBBYTES: usize = 12;
pub const crypto_aead_chacha20poly1305_ietf_ABYTES: usize = 16;
/* SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, 64ULL * ((1ULL << 32) - 1ULL)) */
pub const crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX: u64 = {
    let a = (SODIUM_SIZE_MAX - crypto_aead_chacha20poly1305_ietf_ABYTES) as u64;
    let b = 64u64 * ((1u64 << 32) - 1u64);
    if a < b {
        a
    } else {
        b
    }
};

/* -- Original ChaCha20-Poly1305 construction with a 64-bit nonce and a 64-bit internal counter -- */

pub const crypto_aead_chacha20poly1305_KEYBYTES: usize = 32;
pub const crypto_aead_chacha20poly1305_NSECBYTES: usize = 0;
pub const crypto_aead_chacha20poly1305_NPUBBYTES: usize = 8;
pub const crypto_aead_chacha20poly1305_ABYTES: usize = 16;
/* SODIUM_SIZE_MAX - ABYTES */
pub const crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX: u64 =
    (SODIUM_SIZE_MAX - crypto_aead_chacha20poly1305_ABYTES) as u64;

static _pad0: [u8; 16] = [0u8; 16];

const STREAM_POLY1305_CHUNK: u64 = 131072;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_encrypt_detached(
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
    crypto_stream_chacha20(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    {
        let mut off: u64 = 0;
        let mut ic: u64 = 1;

        while off < mlen {
            let mut cl: u64 = mlen - off;
            if cl > STREAM_POLY1305_CHUNK {
                cl = STREAM_POLY1305_CHUNK;
            }
            crypto_stream_chacha20_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                npub,
                ic,
                k,
            );
            crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
            off += cl;
            ic += cl / 64;
        }
    }
    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    if !maclen_p.is_null() {
        *maclen_p = crypto_aead_chacha20poly1305_ABYTES as u64;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_encrypt(
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

    if mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ret = crypto_aead_chacha20poly1305_encrypt_detached(
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
            clen = mlen + crypto_aead_chacha20poly1305_ABYTES as u64;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
    crypto_stream_chacha20_ietf(block0.as_mut_ptr(), block0.len() as u64, npub, k);
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

        while off < mlen {
            let mut cl: u64 = mlen - off;
            if cl > STREAM_POLY1305_CHUNK {
                cl = STREAM_POLY1305_CHUNK;
            }
            crypto_stream_chacha20_ietf_xor_ic(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_encrypt(
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

    if mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ret = crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
            clen = mlen + crypto_aead_chacha20poly1305_ietf_ABYTES as u64;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_decrypt_detached(
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
    let mut computed_mac: [u8; crypto_aead_chacha20poly1305_ABYTES] =
        [0u8; crypto_aead_chacha20poly1305_ABYTES];
    let mlen: u64;
    let ret: c_int;

    let _ = nsec;
    crypto_stream_chacha20(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), slen.len() as u64);

    mlen = clen;
    crypto_onetimeauth_poly1305_update(&mut state, c, mlen);
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
        crate::common::memset(m, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_decrypt(
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

    if clen >= crypto_aead_chacha20poly1305_ABYTES as u64 {
        ret = crypto_aead_chacha20poly1305_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_chacha20poly1305_ABYTES as u64,
            c.add((clen - crypto_aead_chacha20poly1305_ABYTES as u64) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_chacha20poly1305_ABYTES as u64;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_decrypt_detached(
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
    crypto_stream_chacha20_ietf(block0.as_mut_ptr(), block0.len() as u64, npub, k);
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
        crate::common::memset(m, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_decrypt(
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

    if clen >= crypto_aead_chacha20poly1305_ietf_ABYTES as u64 {
        ret = crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_chacha20poly1305_ietf_ABYTES as u64,
            c.add((clen - crypto_aead_chacha20poly1305_ietf_ABYTES as u64) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_chacha20poly1305_ietf_ABYTES as u64;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_keybytes() -> usize {
    crypto_aead_chacha20poly1305_ietf_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_npubbytes() -> usize {
    crypto_aead_chacha20poly1305_ietf_NPUBBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_nsecbytes() -> usize {
    crypto_aead_chacha20poly1305_ietf_NSECBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_abytes() -> usize {
    crypto_aead_chacha20poly1305_ietf_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_messagebytes_max() -> usize {
    crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_aead_chacha20poly1305_ietf_KEYBYTES);
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_keybytes() -> usize {
    crypto_aead_chacha20poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_npubbytes() -> usize {
    crypto_aead_chacha20poly1305_NPUBBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_nsecbytes() -> usize {
    crypto_aead_chacha20poly1305_NSECBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_abytes() -> usize {
    crypto_aead_chacha20poly1305_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_messagebytes_max() -> usize {
    crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_aead_chacha20poly1305_KEYBYTES);
}
