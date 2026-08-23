//! Translation of
//! `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c`.
//!
//! Exports:
//!   * `crypto_secretstream_xchacha20poly1305_abytes`
//!   * `crypto_secretstream_xchacha20poly1305_headerbytes`
//!   * `crypto_secretstream_xchacha20poly1305_init_pull`
//!   * `crypto_secretstream_xchacha20poly1305_init_push`
//!   * `crypto_secretstream_xchacha20poly1305_keybytes`
//!   * `crypto_secretstream_xchacha20poly1305_keygen`
//!   * `crypto_secretstream_xchacha20poly1305_messagebytes_max`
//!   * `crypto_secretstream_xchacha20poly1305_pull`
//!   * `crypto_secretstream_xchacha20poly1305_push`
//!   * `crypto_secretstream_xchacha20poly1305_rekey`
//!   * `crypto_secretstream_xchacha20poly1305_statebytes`
//!   * `crypto_secretstream_xchacha20poly1305_tag_final`
//!   * `crypto_secretstream_xchacha20poly1305_tag_message`
//!   * `crypto_secretstream_xchacha20poly1305_tag_push`
//!   * `crypto_secretstream_xchacha20poly1305_tag_rekey`

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* crypto_stream_chacha20.h */
const crypto_stream_chacha20_ietf_KEYBYTES: usize = 32;
const crypto_stream_chacha20_ietf_NONCEBYTES: usize = 12;

/* crypto_core_hchacha20.h */
const crypto_core_hchacha20_INPUTBYTES: usize = 16;

/* crypto_onetimeauth_poly1305.h */
const crypto_onetimeauth_poly1305_BYTES: usize = 16;

/* crypto_secretstream_xchacha20poly1305.h
 * ABYTES       == 1 + crypto_aead_xchacha20poly1305_ietf_ABYTES (16)
 * HEADERBYTES  == crypto_aead_xchacha20poly1305_ietf_NPUBBYTES  (24)
 * KEYBYTES     == crypto_aead_xchacha20poly1305_ietf_KEYBYTES   (32)
 * MESSAGEBYTES_MAX ==
 *     SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, 64ULL * ((1ULL << 32) - 2ULL))
 *  == 64 * (2^32 - 2) == 274877906816 on 64-bit hosts. */
const crypto_secretstream_xchacha20poly1305_ABYTES: usize = 17;
const crypto_secretstream_xchacha20poly1305_HEADERBYTES: usize = 24;
const crypto_secretstream_xchacha20poly1305_KEYBYTES: usize = 32;
const crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX: u64 = {
    let a = SODIUM_SIZE_MAX - crypto_secretstream_xchacha20poly1305_ABYTES as u64;
    let b = 64u64 * ((1u64 << 32) - 2u64);
    if a < b {
        a
    } else {
        b
    }
};

const crypto_secretstream_xchacha20poly1305_TAG_MESSAGE: u8 = 0x00;
const crypto_secretstream_xchacha20poly1305_TAG_PUSH: u8 = 0x01;
const crypto_secretstream_xchacha20poly1305_TAG_REKEY: u8 = 0x02;
const crypto_secretstream_xchacha20poly1305_TAG_FINAL: u8 =
    crypto_secretstream_xchacha20poly1305_TAG_PUSH | crypto_secretstream_xchacha20poly1305_TAG_REKEY;

const crypto_secretstream_xchacha20poly1305_COUNTERBYTES: usize = 4;
const crypto_secretstream_xchacha20poly1305_INONCEBYTES: usize = 8;

/// ```c
/// typedef struct crypto_secretstream_xchacha20poly1305_state {
///     unsigned char k[crypto_stream_chacha20_ietf_KEYBYTES];
///     unsigned char nonce[crypto_stream_chacha20_ietf_NONCEBYTES];
///     unsigned char _pad[8];
/// } crypto_secretstream_xchacha20poly1305_state;
/// ```
#[repr(C)]
pub struct crypto_secretstream_xchacha20poly1305_state {
    pub k: [u8; crypto_stream_chacha20_ietf_KEYBYTES],
    pub nonce: [u8; crypto_stream_chacha20_ietf_NONCEBYTES],
    pub _pad: [u8; 8],
}

/* typedef struct CRYPTO_ALIGN(16) crypto_onetimeauth_poly1305_state {
 *     unsigned char opaque[256];
 * } crypto_onetimeauth_poly1305_state; */
#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

extern "C" {
    /* crypto_core/hchacha20/core_hchacha20.c */
    fn crypto_core_hchacha20(
        out: *mut u8,
        in_: *const u8,
        k: *const u8,
        c: *const u8,
    ) -> c_int;
    /* crypto_stream/chacha20/stream_chacha20.c */
    fn crypto_stream_chacha20_ietf(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf_xor(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int;
    /* crypto_onetimeauth/poly1305/onetimeauth_poly1305.c */
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
    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn sodium_increment(n: *mut u8, nlen: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

/* static const unsigned char _pad0[16] = { 0 }; */
static _pad0: [u8; 16] = [0u8; 16];

/* #define STATE_COUNTER(STATE) ((STATE)->nonce) */
#[inline(always)]
unsafe fn STATE_COUNTER(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut u8 {
    (*state).nonce.as_mut_ptr()
}

/* #define STATE_INONCE(STATE) ((STATE)->nonce + COUNTERBYTES) */
#[inline(always)]
unsafe fn STATE_INONCE(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut u8 {
    (*state)
        .nonce
        .as_mut_ptr()
        .add(crypto_secretstream_xchacha20poly1305_COUNTERBYTES)
}

#[inline(always)]
unsafe fn _crypto_secretstream_xchacha20poly1305_counter_reset(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    memset(
        STATE_COUNTER(state),
        0,
        crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
    );
    *STATE_COUNTER(state).add(0) = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_secretstream_xchacha20poly1305_KEYBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    k: *const u8,
) -> c_int {
    randombytes_buf(
        out as *mut c_void,
        crypto_secretstream_xchacha20poly1305_HEADERBYTES,
    );
    crypto_core_hchacha20((*state).k.as_mut_ptr(), out, k, core::ptr::null());
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    memcpy(
        STATE_INONCE(state),
        out.add(crypto_core_hchacha20_INPUTBYTES),
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    memset((*state)._pad.as_mut_ptr(), 0, 8);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    in_: *const u8,
    k: *const u8,
) -> c_int {
    crypto_core_hchacha20((*state).k.as_mut_ptr(), in_, k, core::ptr::null());
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    memcpy(
        STATE_INONCE(state),
        in_.add(crypto_core_hchacha20_INPUTBYTES),
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    memset((*state)._pad.as_mut_ptr(), 0, 8);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_rekey(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    let mut new_key_and_inonce: [u8; crypto_stream_chacha20_ietf_KEYBYTES
        + crypto_secretstream_xchacha20poly1305_INONCEBYTES] = [0u8;
        crypto_stream_chacha20_ietf_KEYBYTES
            + crypto_secretstream_xchacha20poly1305_INONCEBYTES];
    let mut i: usize;

    i = 0;
    while i < crypto_stream_chacha20_ietf_KEYBYTES {
        new_key_and_inonce[i] = (*state).k[i];
        i += 1;
    }
    i = 0;
    while i < crypto_secretstream_xchacha20poly1305_INONCEBYTES {
        new_key_and_inonce[crypto_stream_chacha20_ietf_KEYBYTES + i] =
            *STATE_INONCE(state).add(i);
        i += 1;
    }
    crypto_stream_chacha20_ietf_xor(
        new_key_and_inonce.as_mut_ptr(),
        new_key_and_inonce.as_ptr(),
        new_key_and_inonce.len() as c_ulonglong,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    i = 0;
    while i < crypto_stream_chacha20_ietf_KEYBYTES {
        (*state).k[i] = new_key_and_inonce[i];
        i += 1;
    }
    i = 0;
    while i < crypto_secretstream_xchacha20poly1305_INONCEBYTES {
        *STATE_INONCE(state).add(i) =
            new_key_and_inonce[crypto_stream_chacha20_ietf_KEYBYTES + i];
        i += 1;
    }
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    outlen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    tag: u8,
) -> c_int {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block: [u8; 64] = [0u8; 64];
    let mut slen: [u8; 8] = [0u8; 8];
    let c: *mut u8;
    let mac: *mut u8;

    if !outlen_p.is_null() {
        *outlen_p = 0;
    }
    if mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        block.as_mut_ptr(),
        64,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );
    memset(block.as_mut_ptr(), 0, 64);
    block[0] = tag;

    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, block.as_ptr(), 64);
    *out.add(0) = block[0];

    c = out.add(1 /* sizeof tag */);
    crypto_stream_chacha20_ietf_xor_ic(
        c,
        m,
        mlen,
        (*state).nonce.as_ptr(),
        2,
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(64u64).wrapping_add(mlen)) & 0xf,
    );
    /* should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned */

    store64_le(slen.as_mut_ptr(), adlen as u64);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);
    store64_le(slen.as_mut_ptr(), 64u64.wrapping_add(mlen));
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);

    mac = c.add(mlen as usize);
    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac);
    sodium_memzero(
        &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    xor_buf(
        STATE_INONCE(state),
        mac,
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    sodium_increment(
        STATE_COUNTER(state),
        crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
    );
    if (tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0
        || sodium_is_zero(
            STATE_COUNTER(state),
            crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
        ) != 0
    {
        crypto_secretstream_xchacha20poly1305_rekey(state);
    }
    if !outlen_p.is_null() {
        *outlen_p =
            crypto_secretstream_xchacha20poly1305_ABYTES as c_ulonglong + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    tag_p: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
) -> c_int {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block: [u8; 64] = [0u8; 64];
    let mut slen: [u8; 8] = [0u8; 8];
    let mut mac: [u8; crypto_onetimeauth_poly1305_BYTES] =
        [0u8; crypto_onetimeauth_poly1305_BYTES];
    let c: *const u8;
    let stored_mac: *const u8;
    let mlen: c_ulonglong;
    let tag: u8;

    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    if !tag_p.is_null() {
        *tag_p = 0xff;
    }
    if inlen < crypto_secretstream_xchacha20poly1305_ABYTES as c_ulonglong {
        return -1;
    }
    mlen = inlen - crypto_secretstream_xchacha20poly1305_ABYTES as c_ulonglong;
    if mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        block.as_mut_ptr(),
        64,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    memset(block.as_mut_ptr(), 0, 64);
    block[0] = *in_.add(0);
    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    tag = block[0];
    block[0] = *in_.add(0);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, block.as_ptr(), 64);

    c = in_.add(1 /* sizeof tag */);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(64u64).wrapping_add(mlen)) & 0xf,
    );
    /* should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned */

    store64_le(slen.as_mut_ptr(), adlen as u64);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);
    store64_le(slen.as_mut_ptr(), 64u64.wrapping_add(mlen));
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);

    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac.as_mut_ptr());
    sodium_memzero(
        &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    stored_mac = c.add(mlen as usize);
    if sodium_memcmp(
        mac.as_ptr() as *const c_void,
        stored_mac as *const c_void,
        crypto_onetimeauth_poly1305_BYTES,
    ) != 0
    {
        sodium_memzero(
            mac.as_mut_ptr() as *mut c_void,
            crypto_onetimeauth_poly1305_BYTES,
        );
        return -1;
    }

    /* ACQUIRE_FENCE -- (void) 0 in the reference build */
    crypto_stream_chacha20_ietf_xor_ic(
        m,
        c,
        mlen,
        (*state).nonce.as_ptr(),
        2,
        (*state).k.as_ptr(),
    );
    xor_buf(
        STATE_INONCE(state),
        mac.as_ptr(),
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    sodium_increment(
        STATE_COUNTER(state),
        crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
    );
    if (tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0
        || sodium_is_zero(
            STATE_COUNTER(state),
            crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
        ) != 0
    {
        crypto_secretstream_xchacha20poly1305_rekey(state);
    }
    if !mlen_p.is_null() {
        *mlen_p = mlen;
    }
    if !tag_p.is_null() {
        *tag_p = tag;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_secretstream_xchacha20poly1305_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_abytes() -> usize {
    crypto_secretstream_xchacha20poly1305_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_headerbytes() -> usize {
    crypto_secretstream_xchacha20poly1305_HEADERBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keybytes() -> usize {
    crypto_secretstream_xchacha20poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_messagebytes_max() -> usize {
    crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_message() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_MESSAGE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_push() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_PUSH
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_rekey() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_REKEY
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_final() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_FINAL
}
