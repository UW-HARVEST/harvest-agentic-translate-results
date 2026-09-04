//! Translation of `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

use crate::common::{store64_le, xor_buf, SODIUM_SIZE_MAX};
use crate::csys::{memcpy, memset};

/// `crypto_secretstream_xchacha20poly1305_state`
/// (`include/sodium/crypto_secretstream_xchacha20poly1305.h`):
/// ```c
/// typedef struct crypto_secretstream_xchacha20poly1305_state {
///     unsigned char k[crypto_stream_chacha20_ietf_KEYBYTES];    /* 32 */
///     unsigned char nonce[crypto_stream_chacha20_ietf_NONCEBYTES]; /* 12 */
///     unsigned char _pad[8];
/// } crypto_secretstream_xchacha20poly1305_state;
/// ```
#[repr(C)]
pub struct crypto_secretstream_xchacha20poly1305_state {
    pub k: [u8; 32],
    pub nonce: [u8; 12],
    pub _pad: [u8; 8],
}

/// `crypto_onetimeauth_poly1305_state`: `CRYPTO_ALIGN(16) unsigned char opaque[256]`.
/// Declared locally per the cross-module-call convention; layout must match
/// `poly1305.rs`.
#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

extern "C" {
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;

    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_ietf_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
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

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    fn sodium_increment(n: *mut u8, nlen: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/// `static const unsigned char _pad0[16] = { 0 };`
static _pad0: [u8; 16] = [0u8; 16];

/// `crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX`:
/// `SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, 64ULL * ((1ULL << 32) - 2ULL))`
#[inline(always)]
unsafe fn messagebytes_max() -> u64 {
    let a = SODIUM_SIZE_MAX - (1 + 16);
    let b: u64 = 64u64 * ((1u64 << 32) - 2);
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
unsafe fn counter_reset(state: *mut crypto_secretstream_xchacha20poly1305_state) {
    memset((*state).nonce.as_mut_ptr() as *mut c_void, 0, 4);
    (*state).nonce[0] = 1;
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    k: *const u8,
) -> c_int {
    randombytes_buf(out as *mut c_void, 24);
    crypto_core_hchacha20((*state).k.as_mut_ptr(), out, k, core::ptr::null());
    counter_reset(state);
    memcpy(
        (*state).nonce.as_mut_ptr().add(4) as *mut c_void,
        out.add(16) as *const c_void,
        8,
    );
    memset((*state)._pad.as_mut_ptr() as *mut c_void, 0, (*state)._pad.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    in_: *const u8,
    k: *const u8,
) -> c_int {
    crypto_core_hchacha20((*state).k.as_mut_ptr(), in_, k, core::ptr::null());
    counter_reset(state);
    memcpy(
        (*state).nonce.as_mut_ptr().add(4) as *mut c_void,
        in_.add(16) as *const c_void,
        8,
    );
    memset((*state)._pad.as_mut_ptr() as *mut c_void, 0, (*state)._pad.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_rekey(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    let mut new_key_and_inonce = [0u8; 32 + 8];
    let mut i: usize;

    i = 0;
    while i < 32 {
        new_key_and_inonce[i] = (*state).k[i];
        i += 1;
    }
    i = 0;
    while i < 8 {
        new_key_and_inonce[32 + i] = (*state).nonce[4 + i];
        i += 1;
    }
    crypto_stream_chacha20_ietf_xor(
        new_key_and_inonce.as_mut_ptr(),
        new_key_and_inonce.as_ptr(),
        new_key_and_inonce.len() as u64,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    i = 0;
    while i < 32 {
        (*state).k[i] = new_key_and_inonce[i];
        i += 1;
    }
    i = 0;
    while i < 8 {
        (*state).nonce[4 + i] = new_key_and_inonce[32 + i];
        i += 1;
    }
    counter_reset(state);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    outlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    tag: u8,
) -> c_int {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block = [0u8; 64];
    let mut slen = [0u8; 8];
    let c: *mut u8;
    let mac: *mut u8;

    if !outlen_p.is_null() {
        *outlen_p = 0;
    }
    if mlen > messagebytes_max() {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf(block.as_mut_ptr(), block.len() as u64, (*state).nonce.as_ptr(), (*state).k.as_ptr());
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, block.len());

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );
    memset(block.as_mut_ptr() as *mut c_void, 0, block.len());
    block[0] = tag;

    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        block.len() as u64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, block.as_ptr(), block.len() as u64);
    *out = block[0];

    c = out.add(core::mem::size_of::<u8>());
    crypto_stream_chacha20_ietf_xor_ic(c, m, mlen, (*state).nonce.as_ptr(), 2, (*state).k.as_ptr());
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(block.len() as u64).wrapping_add(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), slen.len() as u64);
    store64_le(slen.as_mut_ptr(), block.len() as u64 + mlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), slen.len() as u64);

    mac = c.add(mlen as usize);
    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac);
    sodium_memzero(
        &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    xor_buf((*state).nonce.as_mut_ptr().add(4), mac, 8);
    sodium_increment((*state).nonce.as_mut_ptr(), 4);
    if (tag & 0x02) != 0 || sodium_is_zero((*state).nonce.as_ptr(), 4) != 0 {
        crypto_secretstream_xchacha20poly1305_rekey(state);
    }
    if !outlen_p.is_null() {
        *outlen_p = (1 + 16) + mlen;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    m: *mut u8,
    mlen_p: *mut u64,
    tag_p: *mut u8,
    in_: *const u8,
    inlen: u64,
    ad: *const u8,
    adlen: u64,
) -> c_int {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut mac = [0u8; 16];
    let c: *const u8;
    let stored_mac: *const u8;
    let mlen: u64;
    let tag: u8;

    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    if !tag_p.is_null() {
        *tag_p = 0xff;
    }
    if inlen < (1 + 16) {
        return -1;
    }
    mlen = inlen - (1 + 16);
    if mlen > messagebytes_max() {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf(block.as_mut_ptr(), block.len() as u64, (*state).nonce.as_ptr(), (*state).k.as_ptr());
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, block.len());

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    memset(block.as_mut_ptr() as *mut c_void, 0, block.len());
    block[0] = *in_;
    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        block.len() as u64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    tag = block[0];
    block[0] = *in_;
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, block.as_ptr(), block.len() as u64);

    c = in_.add(core::mem::size_of::<u8>());
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(block.len() as u64).wrapping_add(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), slen.len() as u64);
    store64_le(slen.as_mut_ptr(), block.len() as u64 + mlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac.as_mut_ptr());
    sodium_memzero(
        &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    stored_mac = c.add(mlen as usize);
    if sodium_memcmp(
        mac.as_ptr() as *const c_void,
        stored_mac as *const c_void,
        mac.len(),
    ) != 0
    {
        sodium_memzero(mac.as_mut_ptr() as *mut c_void, mac.len());
        return -1;
    }

    crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, (*state).nonce.as_ptr(), 2, (*state).k.as_ptr());
    xor_buf((*state).nonce.as_mut_ptr().add(4), mac.as_ptr(), 8);
    sodium_increment((*state).nonce.as_mut_ptr(), 4);
    if (tag & 0x02) != 0 || sodium_is_zero((*state).nonce.as_ptr(), 4) != 0 {
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

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_secretstream_xchacha20poly1305_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_abytes() -> usize {
    1 + 16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_headerbytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_messagebytes_max() -> usize {
    messagebytes_max() as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_message() -> u8 {
    0x00
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_push() -> u8 {
    0x01
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_rekey() -> u8 {
    0x02
}

#[no_mangle]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_final() -> u8 {
    0x01 | 0x02
}
