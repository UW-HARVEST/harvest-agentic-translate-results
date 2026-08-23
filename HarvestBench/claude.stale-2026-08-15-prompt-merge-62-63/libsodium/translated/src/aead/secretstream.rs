//! Translated from crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c
use crate::primitives::cutil::store64_le;
use crate::primitives::poly1305::crypto_onetimeauth_poly1305_state;
use core::ffi::c_void;

extern "C" {
    fn crypto_core_hchacha20(out: *mut u8, input: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32;
    fn crypto_stream_chacha20_ietf_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_chacha20_ietf_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
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
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> i32;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn sodium_increment(n: *mut u8, nlen: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> i32;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// crypto_stream_chacha20_ietf_KEYBYTES=32, NONCEBYTES=12
#[repr(C)]
pub struct crypto_secretstream_xchacha20poly1305_state {
    pub k: [u8; 32],
    pub nonce: [u8; 12],
    pub _pad: [u8; 8],
}

const KEYBYTES: usize = 32; // xchacha20poly1305_ietf_KEYBYTES
const HEADERBYTES: usize = 24; // xchacha20poly1305_ietf_NPUBBYTES
const ABYTES: usize = 1 + 16; // 1 + xchacha20poly1305_ietf_ABYTES
// min(SODIUM_SIZE_MAX - ABYTES, 64*(2^32-2))
const MESSAGEBYTES_MAX: u64 = 64 * ((1u64 << 32) - 2);

const TAG_MESSAGE: u8 = 0x00;
const TAG_PUSH: u8 = 0x01;
const TAG_REKEY: u8 = 0x02;
const TAG_FINAL: u8 = TAG_PUSH | TAG_REKEY;

const COUNTERBYTES: usize = 4;
const INONCEBYTES: usize = 8;
const HCHACHA20_INPUTBYTES: usize = 16;
const POLY1305_BYTES: usize = 16;
const CHACHA20_IETF_KEYBYTES: usize = 32;

const _PAD0: [u8; 16] = [0; 16];

#[inline]
unsafe fn counter_reset(state: *mut crypto_secretstream_xchacha20poly1305_state) {
    let counter = (*state).nonce.as_mut_ptr(); // STATE_COUNTER
    core::ptr::write_bytes(counter, 0, COUNTERBYTES);
    *counter = 1;
}

#[inline]
unsafe fn state_inonce(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut u8 {
    (*state).nonce.as_mut_ptr().add(COUNTERBYTES)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    k: *const u8,
) -> i32 {
    randombytes_buf(out as *mut c_void, HEADERBYTES);
    crypto_core_hchacha20((*state).k.as_mut_ptr(), out, k, core::ptr::null());
    counter_reset(state);
    core::ptr::copy_nonoverlapping(
        out.add(HCHACHA20_INPUTBYTES),
        state_inonce(state),
        INONCEBYTES,
    );
    core::ptr::write_bytes((*state)._pad.as_mut_ptr(), 0, 8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    input: *const u8,
    k: *const u8,
) -> i32 {
    crypto_core_hchacha20((*state).k.as_mut_ptr(), input, k, core::ptr::null());
    counter_reset(state);
    core::ptr::copy_nonoverlapping(
        input.add(HCHACHA20_INPUTBYTES),
        state_inonce(state),
        INONCEBYTES,
    );
    core::ptr::write_bytes((*state)._pad.as_mut_ptr(), 0, 8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_rekey(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    let mut new_key_and_inonce = [0u8; CHACHA20_IETF_KEYBYTES + INONCEBYTES];
    for i in 0..CHACHA20_IETF_KEYBYTES {
        new_key_and_inonce[i] = (*state).k[i];
    }
    for i in 0..INONCEBYTES {
        new_key_and_inonce[CHACHA20_IETF_KEYBYTES + i] = *state_inonce(state).add(i);
    }
    crypto_stream_chacha20_ietf_xor(
        new_key_and_inonce.as_mut_ptr(),
        new_key_and_inonce.as_ptr(),
        new_key_and_inonce.len() as u64,
        (*state).nonce.as_ptr(),
        (*state).k.as_ptr(),
    );
    for i in 0..CHACHA20_IETF_KEYBYTES {
        (*state).k[i] = new_key_and_inonce[i];
    }
    for i in 0..INONCEBYTES {
        *state_inonce(state).add(i) = new_key_and_inonce[CHACHA20_IETF_KEYBYTES + i];
    }
    counter_reset(state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    outlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    tag: u8,
) -> i32 {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block = [0u8; 64];
    let mut slen = [0u8; 8];

    if !outlen_p.is_null() {
        *outlen_p = 0;
    }
    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf(block.as_mut_ptr(), 64, (*state).nonce.as_ptr(), (*state).k.as_ptr());
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, _PAD0.as_ptr(), (0x10 - adlen) & 0xf);
    core::ptr::write_bytes(block.as_mut_ptr(), 0, 64);
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

    let c = out.add(1);
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
        _PAD0.as_ptr(),
        (0x10u64.wrapping_sub(64).wrapping_add(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);
    store64_le(slen.as_mut_ptr(), 64u64.wrapping_add(mlen));
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);

    let mac = c.add(mlen as usize);
    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac);
    sodium_memzero(&mut poly1305_state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    xor_buf(state_inonce(state), mac, INONCEBYTES);
    sodium_increment((*state).nonce.as_mut_ptr(), COUNTERBYTES);
    if (tag & TAG_REKEY) != 0 || sodium_is_zero((*state).nonce.as_ptr(), COUNTERBYTES) != 0 {
        crypto_secretstream_xchacha20poly1305_rekey(state);
    }
    if !outlen_p.is_null() {
        *outlen_p = ABYTES as u64 + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    m: *mut u8,
    mlen_p: *mut u64,
    tag_p: *mut u8,
    input: *const u8,
    inlen: u64,
    ad: *const u8,
    adlen: u64,
) -> i32 {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut mac = [0u8; POLY1305_BYTES];

    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    if !tag_p.is_null() {
        *tag_p = 0xff;
    }
    if inlen < ABYTES as u64 {
        return -1;
    }
    let mlen = inlen - ABYTES as u64;
    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf(block.as_mut_ptr(), 64, (*state).nonce.as_ptr(), (*state).k.as_ptr());
    crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, _PAD0.as_ptr(), (0x10 - adlen) & 0xf);

    core::ptr::write_bytes(block.as_mut_ptr(), 0, 64);
    block[0] = *input.add(0);
    crypto_stream_chacha20_ietf_xor_ic(
        block.as_mut_ptr(),
        block.as_ptr(),
        64,
        (*state).nonce.as_ptr(),
        1,
        (*state).k.as_ptr(),
    );
    let tag = block[0];
    block[0] = *input.add(0);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, block.as_ptr(), 64);

    let c = input.add(1);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
    crypto_onetimeauth_poly1305_update(
        &mut poly1305_state,
        _PAD0.as_ptr(),
        (0x10u64.wrapping_sub(64).wrapping_add(mlen)) & 0xf,
    );

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);
    store64_le(slen.as_mut_ptr(), 64u64.wrapping_add(mlen));
    crypto_onetimeauth_poly1305_update(&mut poly1305_state, slen.as_ptr(), 8);

    crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac.as_mut_ptr());
    sodium_memzero(&mut poly1305_state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    let stored_mac = c.add(mlen as usize);
    if sodium_memcmp(mac.as_ptr() as *const c_void, stored_mac as *const c_void, POLY1305_BYTES) != 0 {
        sodium_memzero(mac.as_mut_ptr() as *mut c_void, POLY1305_BYTES);
        return -1;
    }

    crypto_stream_chacha20_ietf_xor_ic(
        m,
        c,
        mlen,
        (*state).nonce.as_ptr(),
        2,
        (*state).k.as_ptr(),
    );
    xor_buf(state_inonce(state), mac.as_ptr(), INONCEBYTES);
    sodium_increment((*state).nonce.as_mut_ptr(), COUNTERBYTES);
    if (tag & TAG_REKEY) != 0 || sodium_is_zero((*state).nonce.as_ptr(), COUNTERBYTES) != 0 {
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

#[inline]
unsafe fn xor_buf(out: *mut u8, input: *const u8, n: usize) {
    for i in 0..n {
        *out.add(i) ^= *input.add(i);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_secretstream_xchacha20poly1305_state>()
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_abytes() -> usize {
    ABYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_headerbytes() -> usize {
    HEADERBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_message() -> u8 {
    TAG_MESSAGE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_push() -> u8 {
    TAG_PUSH
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_rekey() -> u8 {
    TAG_REKEY
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretstream_xchacha20poly1305_tag_final() -> u8 {
    TAG_FINAL
}
