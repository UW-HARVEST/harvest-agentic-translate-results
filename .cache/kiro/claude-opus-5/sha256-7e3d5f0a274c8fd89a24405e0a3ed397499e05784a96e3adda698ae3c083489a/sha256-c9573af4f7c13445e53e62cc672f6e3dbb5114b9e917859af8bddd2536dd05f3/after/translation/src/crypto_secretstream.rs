//! Translation of crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c
//! and include/sodium/crypto_secretstream_xchacha20poly1305.h

use core::ffi::{c_int, c_uchar, c_void};

use crate::common::{memset, store64_le, xor_buf, SODIUM_SIZE_MAX};
use crate::crypto_aead::xchacha20poly1305::{
    crypto_aead_xchacha20poly1305_ietf_ABYTES, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES,
};
use crate::crypto_core::hchacha20::crypto_core_hchacha20;
use crate::crypto_onetimeauth::poly1305::{
    crypto_onetimeauth_poly1305_BYTES, crypto_onetimeauth_poly1305_final,
    crypto_onetimeauth_poly1305_init, crypto_onetimeauth_poly1305_state,
    crypto_onetimeauth_poly1305_update,
};
use crate::crypto_stream::chacha20::{
    crypto_stream_chacha20_ietf, crypto_stream_chacha20_ietf_KEYBYTES,
    crypto_stream_chacha20_ietf_NONCEBYTES, crypto_stream_chacha20_ietf_xor,
    crypto_stream_chacha20_ietf_xor_ic,
};
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::{sodium_increment, sodium_is_zero, sodium_memcmp, sodium_memzero};

pub const crypto_secretstream_xchacha20poly1305_ABYTES: usize =
    1 + crypto_aead_xchacha20poly1305_ietf_ABYTES;
pub const crypto_secretstream_xchacha20poly1305_HEADERBYTES: usize =
    crypto_aead_xchacha20poly1305_ietf_NPUBBYTES;
pub const crypto_secretstream_xchacha20poly1305_KEYBYTES: usize =
    crypto_stream_chacha20_ietf_KEYBYTES;

// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, 64ULL * ((1ULL << 32) - 2ULL))
pub const crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX: usize = {
    let a = SODIUM_SIZE_MAX - crypto_secretstream_xchacha20poly1305_ABYTES;
    let b = (64u64 * ((1u64 << 32) - 2u64)) as usize;
    if a < b {
        a
    } else {
        b
    }
};

pub const crypto_secretstream_xchacha20poly1305_TAG_MESSAGE: u8 = 0x00;
pub const crypto_secretstream_xchacha20poly1305_TAG_PUSH: u8 = 0x01;
pub const crypto_secretstream_xchacha20poly1305_TAG_REKEY: u8 = 0x02;
pub const crypto_secretstream_xchacha20poly1305_TAG_FINAL: u8 =
    crypto_secretstream_xchacha20poly1305_TAG_PUSH
        | crypto_secretstream_xchacha20poly1305_TAG_REKEY;

const crypto_secretstream_xchacha20poly1305_COUNTERBYTES: usize = 4;
const crypto_secretstream_xchacha20poly1305_INONCEBYTES: usize = 8;

const crypto_core_hchacha20_INPUTBYTES: usize = 16;

#[repr(C)]
pub struct crypto_secretstream_xchacha20poly1305_state {
    pub k: [c_uchar; crypto_stream_chacha20_ietf_KEYBYTES],
    pub nonce: [c_uchar; crypto_stream_chacha20_ietf_NONCEBYTES],
    pub _pad: [c_uchar; 8],
}

#[inline(always)]
unsafe fn state_counter(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut c_uchar {
    (*state).nonce.as_mut_ptr()
}

#[inline(always)]
unsafe fn state_inonce(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut c_uchar {
    (*state)
        .nonce
        .as_mut_ptr()
        .add(crypto_secretstream_xchacha20poly1305_COUNTERBYTES)
}

static _pad0: [c_uchar; 16] = [0; 16];

#[inline]
unsafe fn _crypto_secretstream_xchacha20poly1305_counter_reset(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    memset(
        state_counter(state),
        0,
        crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
    );
    *state_counter(state).add(0) = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keygen(k: *mut c_uchar) {
    randombytes_buf(
        k as *mut c_void,
        crypto_secretstream_xchacha20poly1305_KEYBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut c_uchar,
    k: *const c_uchar,
) -> c_int {
    randombytes_buf(
        out as *mut c_void,
        crypto_secretstream_xchacha20poly1305_HEADERBYTES,
    );
    crypto_core_hchacha20((*state).k.as_mut_ptr(), out, k, core::ptr::null());
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    crate::common::memcpy(
        state_inonce(state),
        out.add(crypto_core_hchacha20_INPUTBYTES),
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    memset(
        (*state)._pad.as_mut_ptr(),
        0,
        core::mem::size_of_val(&(*state)._pad),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    in_: *const c_uchar,
    k: *const c_uchar,
) -> c_int {
    crypto_core_hchacha20((*state).k.as_mut_ptr(), in_, k, core::ptr::null());
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    crate::common::memcpy(
        state_inonce(state),
        in_.add(crypto_core_hchacha20_INPUTBYTES),
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    memset(
        (*state)._pad.as_mut_ptr(),
        0,
        core::mem::size_of_val(&(*state)._pad),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_rekey(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    let mut new_key_and_inonce: [c_uchar;
        crypto_stream_chacha20_ietf_KEYBYTES + crypto_secretstream_xchacha20poly1305_INONCEBYTES] =
        [0; crypto_stream_chacha20_ietf_KEYBYTES
            + crypto_secretstream_xchacha20poly1305_INONCEBYTES];
    let mut i: usize;

    i = 0;
    while i < crypto_stream_chacha20_ietf_KEYBYTES {
        new_key_and_inonce[i] = (*state).k[i];
        i += 1;
    }
    i = 0;
    while i < crypto_secretstream_xchacha20poly1305_INONCEBYTES {
        new_key_and_inonce[crypto_stream_chacha20_ietf_KEYBYTES + i] = *state_inonce(state).add(i);
        i += 1;
    }
    crypto_stream_chacha20_ietf_xor(
        new_key_and_inonce.as_mut_ptr(),
        new_key_and_inonce.as_ptr(),
        core::mem::size_of_val(&new_key_and_inonce) as u64,
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
        *state_inonce(state).add(i) = new_key_and_inonce[crypto_stream_chacha20_ietf_KEYBYTES + i];
        i += 1;
    }
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut c_uchar,
    outlen_p: *mut u64,
    m: *const c_uchar,
    mlen: u64,
    ad: *const c_uchar,
    adlen: u64,
    tag: c_uchar,
) -> c_int {
    let mut poly1305_state: crypto_onetimeauth_poly1305_state = core::mem::zeroed();
    let mut block: [c_uchar; 64] = [0; 64];
    let mut slen: [c_uchar; 8] = [0; 8];
    let c: *mut c_uchar;
    let mac: *mut c_uchar;

    if !outlen_p.is_null() {
        *outlen_p = 0;
    }
    if mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        block.as_mut_ptr(),
        core::mem::size_of_val(&block) as u64,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(
        block.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&block),
    );

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );
    memset(block.as_mut_ptr(), 0, core::mem::size_of_val(&block));
    block[0] = tag;

    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        core::mem::size_of_val(&block) as u64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        block.as_ptr(),
        core::mem::size_of_val(&block) as u64,
    );
    *out.add(0) = block[0];

    c = out.add(core::mem::size_of_val(&tag));
    crypto_stream_chacha20_ietf_xor_ic(c, m, mlen, (*state).nonce.as_ptr(), 2, (*state).k.as_ptr());
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64
            .wrapping_sub(core::mem::size_of_val(&block) as u64)
            .wrapping_add(mlen))
            & 0xf,
    );
    /* should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned */

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as u64,
    );
    store64_le(
        slen.as_mut_ptr(),
        (core::mem::size_of_val(&block) as u64).wrapping_add(mlen),
    );
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as u64,
    );

    mac = c.add(mlen as usize);
    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac);
    sodium_memzero(
        &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    xor_buf(
        state_inonce(state),
        mac,
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    sodium_increment(
        state_counter(state),
        crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
    );
    if (tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0
        || sodium_is_zero(
            state_counter(state),
            crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
        ) != 0
    {
        crypto_secretstream_xchacha20poly1305_rekey(state);
    }
    if !outlen_p.is_null() {
        *outlen_p = (crypto_secretstream_xchacha20poly1305_ABYTES as u64).wrapping_add(mlen);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    m: *mut c_uchar,
    mlen_p: *mut u64,
    tag_p: *mut c_uchar,
    in_: *const c_uchar,
    inlen: u64,
    ad: *const c_uchar,
    adlen: u64,
) -> c_int {
    let mut poly1305_state: crypto_onetimeauth_poly1305_state = core::mem::zeroed();
    let mut block: [c_uchar; 64] = [0; 64];
    let mut slen: [c_uchar; 8] = [0; 8];
    let mut mac: [c_uchar; crypto_onetimeauth_poly1305_BYTES] =
        [0; crypto_onetimeauth_poly1305_BYTES];
    let c: *const c_uchar;
    let stored_mac: *const c_uchar;
    let mlen: u64;
    let tag: c_uchar;

    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    if !tag_p.is_null() {
        *tag_p = 0xff;
    }
    if inlen < crypto_secretstream_xchacha20poly1305_ABYTES as u64 {
        return -1;
    }
    mlen = inlen - crypto_secretstream_xchacha20poly1305_ABYTES as u64;
    if mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        block.as_mut_ptr(),
        core::mem::size_of_val(&block) as u64,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(
        block.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&block),
    );

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    memset(block.as_mut_ptr(), 0, core::mem::size_of_val(&block));
    block[0] = *in_.add(0);
    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        core::mem::size_of_val(&block) as u64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    tag = block[0];
    block[0] = *in_.add(0);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        block.as_ptr(),
        core::mem::size_of_val(&block) as u64,
    );

    c = in_.add(core::mem::size_of_val(&tag));
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _pad0.as_ptr(),
        (0x10u64
            .wrapping_sub(core::mem::size_of_val(&block) as u64)
            .wrapping_add(mlen))
            & 0xf,
    );
    /* should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned */

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as u64,
    );
    store64_le(
        slen.as_mut_ptr(),
        (core::mem::size_of_val(&block) as u64).wrapping_add(mlen),
    );
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        slen.as_ptr(),
        core::mem::size_of_val(&slen) as u64,
    );

    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac.as_mut_ptr());
    sodium_memzero(
        &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

    stored_mac = c.add(mlen as usize);
    if sodium_memcmp(
        mac.as_ptr() as *const c_void,
        stored_mac as *const c_void,
        core::mem::size_of_val(&mac),
    ) != 0
    {
        sodium_memzero(
            mac.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&mac),
        );
        return -1;
    }

    crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, (*state).nonce.as_ptr(), 2, (*state).k.as_ptr());
    xor_buf(
        state_inonce(state),
        mac.as_ptr(),
        crypto_secretstream_xchacha20poly1305_INONCEBYTES,
    );
    sodium_increment(
        state_counter(state),
        crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
    );
    if (tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0
        || sodium_is_zero(
            state_counter(state),
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
pub extern "C" fn crypto_secretstream_xchacha20poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_secretstream_xchacha20poly1305_state>()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_abytes() -> usize {
    crypto_secretstream_xchacha20poly1305_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_headerbytes() -> usize {
    crypto_secretstream_xchacha20poly1305_HEADERBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_keybytes() -> usize {
    crypto_secretstream_xchacha20poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_messagebytes_max() -> usize {
    crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_message() -> c_uchar {
    crypto_secretstream_xchacha20poly1305_TAG_MESSAGE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_push() -> c_uchar {
    crypto_secretstream_xchacha20poly1305_TAG_PUSH
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_rekey() -> c_uchar {
    crypto_secretstream_xchacha20poly1305_TAG_REKEY
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_final() -> c_uchar {
    crypto_secretstream_xchacha20poly1305_TAG_FINAL
}
