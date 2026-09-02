//! Translation of c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c

use core::ffi::{c_int, c_void};

use crate::common::store64_le;
use crate::sodium::core::sodium_misuse;

// crypto_stream_chacha20_ietf_KEYBYTES = 32, _NONCEBYTES = 12.
const CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES: usize = 32;
const CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES: usize = 12;

// Public state struct. Header declares it with no #pragma pack; all fields are
// byte arrays (align 1), so repr(C, packed) matches the header layout exactly.
#[repr(C, packed)]
struct CryptoSecretstreamXchacha20poly1305State {
    k: [u8; CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES],
    nonce: [u8; CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES],
    _pad: [u8; 8],
}

// crypto_onetimeauth_poly1305_state: public 256-byte aligned struct.
#[repr(C, align(16))]
struct CryptoOnetimeauthPoly1305State {
    opaque: [u8; 256],
}

const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES: usize = 4;
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES: usize = 8;

// crypto_aead_xchacha20poly1305_ietf_ABYTES = 16
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES: usize = 1 + 16;
// crypto_aead_xchacha20poly1305_ietf_NPUBBYTES = 24
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES: usize = 24;
// crypto_aead_xchacha20poly1305_ietf_KEYBYTES = 32
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES: usize = 32;
// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, 64ULL*((1ULL<<32)-2ULL))
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_MESSAGEBYTES_MAX: u64 = {
    let a = (usize::MAX as u64).wrapping_sub(CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES as u64);
    let b = 64u64 * ((1u64 << 32) - 2u64);
    if a < b {
        a
    } else {
        b
    }
};

const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_MESSAGE: u8 = 0x00;
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_PUSH: u8 = 0x01;
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY: u8 = 0x02;
const CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_FINAL: u8 =
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_PUSH | CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY;

const CRYPTO_ONETIMEAUTH_POLY1305_BYTES: usize = 16;

const CRYPTO_CORE_HCHACHA20_INPUTBYTES: usize = 16;

// STATE_COUNTER(STATE) ((STATE)->nonce)
unsafe fn state_counter(state: *mut CryptoSecretstreamXchacha20poly1305State) -> *mut u8 {
    core::ptr::addr_of_mut!((*state).nonce) as *mut u8
}

// STATE_INONCE(STATE) ((STATE)->nonce + COUNTERBYTES)
unsafe fn state_inonce(state: *mut CryptoSecretstreamXchacha20poly1305State) -> *mut u8 {
    (core::ptr::addr_of_mut!((*state).nonce) as *mut u8)
        .add(CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES)
}

static _PAD0: [u8; 16] = [0; 16];

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
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;
    fn sodium_increment(n: *mut u8, nlen: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

// static inline
unsafe fn _crypto_secretstream_xchacha20poly1305_counter_reset(
    state: *mut CryptoSecretstreamXchacha20poly1305State,
) {
    memset(
        state_counter(state) as *mut c_void,
        0,
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES,
    );
    *state_counter(state).add(0) = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_push(
    state: *mut CryptoSecretstreamXchacha20poly1305State,
    out: *mut u8,
    k: *const u8,
) -> c_int {
    randombytes_buf(
        out as *mut c_void,
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES,
    );
    crypto_core_hchacha20(
        core::ptr::addr_of_mut!((*state).k) as *mut u8,
        out,
        k,
        core::ptr::null(),
    );
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    memcpy(
        state_inonce(state) as *mut c_void,
        out.add(CRYPTO_CORE_HCHACHA20_INPUTBYTES) as *const c_void,
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES,
    );
    memset(
        core::ptr::addr_of_mut!((*state)._pad) as *mut c_void,
        0,
        core::mem::size_of::<[u8; 8]>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_pull(
    state: *mut CryptoSecretstreamXchacha20poly1305State,
    in_: *const u8,
    k: *const u8,
) -> c_int {
    crypto_core_hchacha20(
        core::ptr::addr_of_mut!((*state).k) as *mut u8,
        in_,
        k,
        core::ptr::null(),
    );
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    memcpy(
        state_inonce(state) as *mut c_void,
        in_.add(CRYPTO_CORE_HCHACHA20_INPUTBYTES) as *const c_void,
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES,
    );
    memset(
        core::ptr::addr_of_mut!((*state)._pad) as *mut c_void,
        0,
        core::mem::size_of::<[u8; 8]>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_rekey(
    state: *mut CryptoSecretstreamXchacha20poly1305State,
) {
    let mut new_key_and_inonce = [0u8; CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES
        + CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES];
    let mut i: usize;

    i = 0;
    while i < CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES {
        new_key_and_inonce[i] = *(core::ptr::addr_of!((*state).k) as *const u8).add(i);
        i += 1;
    }
    i = 0;
    while i < CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES {
        new_key_and_inonce[CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES + i] = *state_inonce(state).add(i);
        i += 1;
    }
    crypto_stream_chacha20_ietf_xor(
        new_key_and_inonce.as_mut_ptr(),
        new_key_and_inonce.as_ptr(),
        new_key_and_inonce.len() as u64,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    i = 0;
    while i < CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES {
        *(core::ptr::addr_of_mut!((*state).k) as *mut u8).add(i) = new_key_and_inonce[i];
        i += 1;
    }
    i = 0;
    while i < CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES {
        *state_inonce(state).add(i) =
            new_key_and_inonce[CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES + i];
        i += 1;
    }
    _crypto_secretstream_xchacha20poly1305_counter_reset(state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_push(
    state: *mut CryptoSecretstreamXchacha20poly1305State,
    out: *mut u8,
    outlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    tag: u8,
) -> c_int {
    let mut poly1305_state = core::mem::MaybeUninit::<CryptoOnetimeauthPoly1305State>::uninit();
    let poly1305_state = poly1305_state.as_mut_ptr();
    let mut block = [0u8; 64];
    let mut slen = [0u8; 8];
    let c: *mut u8;
    let mac: *mut u8;

    if !outlen_p.is_null() {
        *outlen_p = 0;
    }
    if mlen > CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        block.as_mut_ptr(),
        block.len() as u64,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    crypto_onetimeauth_poly1305_init(poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, block.len());

    crypto_onetimeauth_poly1305_update(poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        poly1305_state,
        _PAD0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );
    memset(block.as_mut_ptr() as *mut c_void, 0, block.len());
    block[0] = tag;

    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        block.len() as u64,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        1,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    crypto_onetimeauth_poly1305_update(poly1305_state, block.as_ptr(), block.len() as u64);
    *out.add(0) = block[0];

    c = out.add(core::mem::size_of::<u8>()); // sizeof tag
    crypto_stream_chacha20_ietf_xor_ic(
        c,
        m,
        mlen,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        2,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    crypto_onetimeauth_poly1305_update(poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        poly1305_state,
        _PAD0.as_ptr(),
        (0x10u64.wrapping_sub(block.len() as u64).wrapping_add(mlen)) & 0xf,
    );
    // should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(poly1305_state, slen.as_ptr(), slen.len() as u64);
    store64_le(slen.as_mut_ptr(), (block.len() as u64).wrapping_add(mlen));
    crypto_onetimeauth_poly1305_update(poly1305_state, slen.as_ptr(), slen.len() as u64);

    mac = c.add(mlen as usize);
    crypto_onetimeauth_poly1305_final(poly1305_state, mac);
    sodium_memzero(
        poly1305_state as *mut c_void,
        core::mem::size_of::<CryptoOnetimeauthPoly1305State>(),
    );

    crate::common::xor_buf(
        state_inonce(state),
        mac,
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES,
    );
    sodium_increment(
        state_counter(state),
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES,
    );
    if (tag & CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY) != 0
        || sodium_is_zero(
            state_counter(state),
            CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES,
        ) != 0
    {
        crypto_secretstream_xchacha20poly1305_rekey(state);
    }
    if !outlen_p.is_null() {
        *outlen_p = CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES as u64 + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_pull(
    state: *mut CryptoSecretstreamXchacha20poly1305State,
    m: *mut u8,
    mlen_p: *mut u64,
    tag_p: *mut u8,
    in_: *const u8,
    inlen: u64,
    ad: *const u8,
    adlen: u64,
) -> c_int {
    let mut poly1305_state = core::mem::MaybeUninit::<CryptoOnetimeauthPoly1305State>::uninit();
    let poly1305_state = poly1305_state.as_mut_ptr();
    let mut block = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut mac = [0u8; CRYPTO_ONETIMEAUTH_POLY1305_BYTES];
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
    if inlen < CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES as u64 {
        return -1;
    }
    mlen = inlen - CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES as u64;
    if mlen > CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        block.as_mut_ptr(),
        block.len() as u64,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    crypto_onetimeauth_poly1305_init(poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, block.len());

    crypto_onetimeauth_poly1305_update(poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(
        poly1305_state,
        _PAD0.as_ptr(),
        (0x10u64.wrapping_sub(adlen)) & 0xf,
    );

    memset(block.as_mut_ptr() as *mut c_void, 0, block.len());
    block[0] = *in_.add(0);
    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        block.len() as u64,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        1,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    tag = block[0];
    block[0] = *in_.add(0);
    crypto_onetimeauth_poly1305_update(poly1305_state, block.as_ptr(), block.len() as u64);

    c = in_.add(core::mem::size_of::<u8>()); // sizeof tag
    crypto_onetimeauth_poly1305_update(poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        poly1305_state,
        _PAD0.as_ptr(),
        (0x10u64.wrapping_sub(block.len() as u64).wrapping_add(mlen)) & 0xf,
    );
    // should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(poly1305_state, slen.as_ptr(), slen.len() as u64);
    store64_le(slen.as_mut_ptr(), (block.len() as u64).wrapping_add(mlen));
    crypto_onetimeauth_poly1305_update(poly1305_state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(poly1305_state, mac.as_mut_ptr());
    sodium_memzero(
        poly1305_state as *mut c_void,
        core::mem::size_of::<CryptoOnetimeauthPoly1305State>(),
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

    // ACQUIRE_FENCE;
    crypto_stream_chacha20_ietf_xor_ic(
        m,
        c,
        mlen,
        core::ptr::addr_of!((*state).nonce) as *const u8,
        2,
        core::ptr::addr_of!((*state).k) as *const u8,
    );
    crate::common::xor_buf(
        state_inonce(state),
        mac.as_ptr(),
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_INONCEBYTES,
    );
    sodium_increment(
        state_counter(state),
        CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES,
    );
    if (tag & CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY) != 0
        || sodium_is_zero(
            state_counter(state),
            CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_COUNTERBYTES,
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
    core::mem::size_of::<CryptoSecretstreamXchacha20poly1305State>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_abytes() -> usize {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_headerbytes() -> usize {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keybytes() -> usize {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_messagebytes_max() -> usize {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_message() -> u8 {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_MESSAGE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_push() -> u8 {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_PUSH
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_rekey() -> u8 {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_final() -> u8 {
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_FINAL
}
