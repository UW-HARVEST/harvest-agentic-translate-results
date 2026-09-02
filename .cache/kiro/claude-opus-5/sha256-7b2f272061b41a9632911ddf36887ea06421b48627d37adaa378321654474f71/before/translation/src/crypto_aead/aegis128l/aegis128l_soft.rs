//! Translation of c_src/libsodium/crypto_aead/aegis128l/aegis128l_soft.c
//!
//! `#if 1` so this file is compiled. It `#include`s `aegis128l_common.h`
//! whose `encrypt_detached` / `decrypt_detached` static functions live here.

use core::ffi::{c_int, c_void};

use crate::crypto_core::softaes::softaes::{
    softaes_block_and, softaes_block_load, softaes_block_load64x2, softaes_block_store,
    softaes_block_xor, SoftAesBlock, _sodium_softaes_block_encrypt,
};

type aes_block_t = SoftAesBlock;

const AES_BLOCK_LENGTH: usize = 16;

// crypto_aead_aegis128l_ABYTES etc.; only ABYTES is used inside this file
// indirectly (maclen is passed by the accessor layer).

#[inline]
unsafe fn AES_ENC(a: aes_block_t, b: aes_block_t) -> aes_block_t {
    _sodium_softaes_block_encrypt(a, b)
}

extern "C" {
    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[inline]
unsafe fn aegis128l_update(state: *mut aes_block_t, d1: aes_block_t, d2: aes_block_t) {
    let tmp: aes_block_t;

    tmp = *state.add(7);
    *state.add(7) = AES_ENC(*state.add(6), *state.add(7));
    *state.add(6) = AES_ENC(*state.add(5), *state.add(6));
    *state.add(5) = AES_ENC(*state.add(4), *state.add(5));
    *state.add(4) = AES_ENC(*state.add(3), *state.add(4));
    *state.add(3) = AES_ENC(*state.add(2), *state.add(3));
    *state.add(2) = AES_ENC(*state.add(1), *state.add(2));
    *state.add(1) = AES_ENC(*state.add(0), *state.add(1));
    *state.add(0) = AES_ENC(tmp, *state.add(0));

    *state.add(0) = softaes_block_xor(*state.add(0), d1);
    *state.add(4) = softaes_block_xor(*state.add(4), d2);
}

// ---- aegis128l_common.h ----

const RATE: usize = 32;

#[inline]
unsafe fn aegis128l_init(key: *const u8, nonce: *const u8, state: *mut aes_block_t) {
    static C0_: [u8; AES_BLOCK_LENGTH] = [
        0x00, 0x01, 0x01, 0x02, 0x03, 0x05, 0x08, 0x0d, 0x15, 0x22, 0x37, 0x59, 0x90, 0xe9, 0x79,
        0x62,
    ];
    static C1_: [u8; AES_BLOCK_LENGTH] = [
        0xdb, 0x3d, 0x18, 0x55, 0x6d, 0xc2, 0x2f, 0xf1, 0x20, 0x11, 0x31, 0x42, 0x73, 0xb5, 0x28,
        0xdd,
    ];

    let c0 = softaes_block_load(C0_.as_ptr());
    let c1 = softaes_block_load(C1_.as_ptr());
    let k: aes_block_t;
    let n: aes_block_t;
    let mut i: c_int;

    k = softaes_block_load(key);
    n = softaes_block_load(nonce);

    *state.add(0) = softaes_block_xor(k, n);
    *state.add(1) = c1;
    *state.add(2) = c0;
    *state.add(3) = c1;
    *state.add(4) = softaes_block_xor(k, n);
    *state.add(5) = softaes_block_xor(k, c0);
    *state.add(6) = softaes_block_xor(k, c1);
    *state.add(7) = softaes_block_xor(k, c0);
    i = 0;
    while i < 10 {
        aegis128l_update(state, n, k);
        i += 1;
    }
}

#[inline]
unsafe fn aegis128l_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: *mut aes_block_t,
) -> c_int {
    let mut tmp: aes_block_t;
    let mut i: c_int;

    tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, *state.add(2));

    i = 0;
    while i < 7 {
        aegis128l_update(state, tmp, tmp);
        i += 1;
    }

    if maclen == 16 {
        /* LCOV_EXCL_START */
        tmp = softaes_block_xor(*state.add(6), softaes_block_xor(*state.add(5), *state.add(4)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(3), *state.add(2)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
    /* LCOV_EXCL_STOP */
    } else if maclen == 32 {
        tmp = softaes_block_xor(*state.add(3), *state.add(2));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(*state.add(7), *state.add(6));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(5), *state.add(4)));
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac as *mut c_void, 0, maclen); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[inline]
unsafe fn aegis128l_absorb(src: *const u8, state: *mut aes_block_t) {
    let msg0: aes_block_t;
    let msg1: aes_block_t;

    msg0 = softaes_block_load(src);
    msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
}

#[inline]
unsafe fn aegis128l_absorb2(src: *const u8, state: *mut aes_block_t) {
    let msg0: aes_block_t;
    let msg1: aes_block_t;
    let msg2: aes_block_t;
    let msg3: aes_block_t;

    msg0 = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    msg1 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    msg2 = softaes_block_load(src.add(2 * AES_BLOCK_LENGTH));
    msg3 = softaes_block_load(src.add(3 * AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
    aegis128l_update(state, msg2, msg3);
}

#[inline]
unsafe fn aegis128l_enc(dst: *mut u8, src: *const u8, state: *mut aes_block_t) {
    let msg0: aes_block_t;
    let msg1: aes_block_t;
    let mut tmp0: aes_block_t;
    let mut tmp1: aes_block_t;

    msg0 = softaes_block_load(src);
    msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    tmp0 = softaes_block_xor(msg0, *state.add(6));
    tmp0 = softaes_block_xor(tmp0, *state.add(1));
    tmp1 = softaes_block_xor(msg1, *state.add(5));
    tmp1 = softaes_block_xor(tmp1, *state.add(2));
    tmp0 = softaes_block_xor(tmp0, softaes_block_and(*state.add(2), *state.add(3)));
    tmp1 = softaes_block_xor(tmp1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(dst, tmp0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), tmp1);

    aegis128l_update(state, msg0, msg1);
}

#[inline]
unsafe fn aegis128l_dec(dst: *mut u8, src: *const u8, state: *mut aes_block_t) {
    let mut msg0: aes_block_t;
    let mut msg1: aes_block_t;

    msg0 = softaes_block_load(src);
    msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, *state.add(6));
    msg0 = softaes_block_xor(msg0, *state.add(1));
    msg1 = softaes_block_xor(msg1, *state.add(5));
    msg1 = softaes_block_xor(msg1, *state.add(2));
    msg0 = softaes_block_xor(msg0, softaes_block_and(*state.add(2), *state.add(3)));
    msg1 = softaes_block_xor(msg1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(dst, msg0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), msg1);

    aegis128l_update(state, msg0, msg1);
}

#[inline]
unsafe fn aegis128l_declast(dst: *mut u8, src: *const u8, len: usize, state: *mut aes_block_t) {
    let mut pad: [u8; RATE] = [0; RATE];
    let mut msg0: aes_block_t;
    let mut msg1: aes_block_t;

    memset(pad.as_mut_ptr() as *mut c_void, 0, core::mem::size_of::<[u8; RATE]>());
    memcpy(pad.as_mut_ptr() as *mut c_void, src as *const c_void, len);

    msg0 = softaes_block_load(pad.as_ptr());
    msg1 = softaes_block_load(pad.as_ptr().add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, *state.add(6));
    msg0 = softaes_block_xor(msg0, *state.add(1));
    msg1 = softaes_block_xor(msg1, *state.add(5));
    msg1 = softaes_block_xor(msg1, *state.add(2));
    msg0 = softaes_block_xor(msg0, softaes_block_and(*state.add(2), *state.add(3)));
    msg1 = softaes_block_xor(msg1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(pad.as_mut_ptr(), msg0);
    softaes_block_store(pad.as_mut_ptr().add(AES_BLOCK_LENGTH), msg1);

    memset(
        pad.as_mut_ptr().add(len) as *mut c_void,
        0,
        core::mem::size_of::<[u8; RATE]>() - len,
    );
    memcpy(dst as *mut c_void, pad.as_ptr() as *const c_void, len);

    msg0 = softaes_block_load(pad.as_ptr());
    msg1 = softaes_block_load(pad.as_ptr().add(AES_BLOCK_LENGTH));

    aegis128l_update(state, msg0, msg1);
}

unsafe fn encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen: usize,
    m: *const u8,
    mlen: usize,
    ad: *const u8,
    adlen: usize,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut state: [aes_block_t; 8] = [SoftAesBlock {
        w0: 0,
        w1: 0,
        w2: 0,
        w3: 0,
    }; 8];
    let mut src: [u8; RATE] = [0; RATE];
    let mut dst: [u8; RATE] = [0; RATE];
    let mut i: usize;

    aegis128l_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), state.as_mut_ptr());
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, RATE);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % RATE);
        aegis128l_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    i = 0;
    while i + RATE <= mlen {
        aegis128l_enc(c.add(i), m.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if mlen % RATE != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, RATE);
        memcpy(src.as_mut_ptr() as *mut c_void, m.add(i) as *const c_void, mlen % RATE);
        aegis128l_enc(dst.as_mut_ptr(), src.as_ptr(), state.as_mut_ptr());
        memcpy(c.add(i) as *mut c_void, dst.as_ptr() as *const c_void, mlen % RATE);
    }

    aegis128l_mac(mac, maclen, adlen as u64, mlen as u64, state.as_mut_ptr())
}

unsafe fn decrypt_detached(
    m: *mut u8,
    c: *const u8,
    clen: usize,
    mac: *const u8,
    maclen: usize,
    ad: *const u8,
    adlen: usize,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut state: [aes_block_t; 8] = [SoftAesBlock {
        w0: 0,
        w1: 0,
        w2: 0,
        w3: 0,
    }; 8];
    let mut src: [u8; RATE] = [0; RATE];
    let mut dst: [u8; RATE] = [0; RATE];
    let mut computed_mac: [u8; 32] = [0; 32];
    let mlen: usize = clen;
    let mut i: usize;
    let mut ret: c_int;

    aegis128l_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), state.as_mut_ptr());
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, RATE);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % RATE);
        aegis128l_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(m.add(i), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(dst.as_mut_ptr(), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis128l_declast(m.add(i), c.add(i), mlen % RATE, state.as_mut_ptr());
        } else {
            aegis128l_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, state.as_mut_ptr());
        }
    }

    // COMPILER_ASSERT(sizeof computed_mac >= 32);
    ret = -1;
    if aegis128l_mac(
        computed_mac.as_mut_ptr(),
        maclen,
        adlen as u64,
        mlen as u64,
        state.as_mut_ptr(),
    ) == 0
    {
        if maclen == 16 {
            ret = crypto_verify_16(computed_mac.as_ptr(), mac); /* LCOV_EXCL_LINE */
        } else if maclen == 32 {
            ret = crypto_verify_32(computed_mac.as_ptr(), mac);
        }
    }
    if ret != 0 {
        if !m.is_null() {
            memset(m as *mut c_void, 0, mlen);
        }
        return ret;
    }
    // ACQUIRE_FENCE;
    0
}

// ---- exported implementation struct ----
//
// struct aegis128l_implementation { encrypt_detached; decrypt_detached; }
// (see crypto_aead/aegis128l/implementations.h)

type EncryptDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;

type DecryptDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;

#[repr(C)]
pub struct aegis128l_implementation {
    pub encrypt_detached: Option<EncryptDetachedFn>,
    pub decrypt_detached: Option<DecryptDetachedFn>,
}

unsafe impl Sync for aegis128l_implementation {}

// The two static functions must be reachable through C function pointers with
// the exact C signature, so wrap them in `extern "C"` trampolines.
unsafe extern "C" fn encrypt_detached_c(
    c: *mut u8,
    mac: *mut u8,
    maclen: usize,
    m: *const u8,
    mlen: usize,
    ad: *const u8,
    adlen: usize,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    encrypt_detached(c, mac, maclen, m, mlen, ad, adlen, npub, k)
}

unsafe extern "C" fn decrypt_detached_c(
    m: *mut u8,
    c: *const u8,
    clen: usize,
    mac: *const u8,
    maclen: usize,
    ad: *const u8,
    adlen: usize,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    decrypt_detached(m, c, clen, mac, maclen, ad, adlen, npub, k)
}

#[unsafe(no_mangle)]
pub static aegis128l_soft_implementation: aegis128l_implementation = aegis128l_implementation {
    encrypt_detached: Some(encrypt_detached_c),
    decrypt_detached: Some(decrypt_detached_c),
};
