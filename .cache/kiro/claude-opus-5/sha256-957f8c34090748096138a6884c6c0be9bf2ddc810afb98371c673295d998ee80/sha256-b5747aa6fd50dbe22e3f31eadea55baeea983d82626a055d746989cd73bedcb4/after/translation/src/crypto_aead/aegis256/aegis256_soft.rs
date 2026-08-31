//! Translation of c_src/libsodium/crypto_aead/aegis256/aegis256_soft.c
//!
//! `#if 1` so this file is compiled. It `#include`s `aegis256_common.h`
//! whose `encrypt_detached` / `decrypt_detached` static functions live here.

use core::ffi::{c_int, c_void};

use crate::crypto_core::softaes::softaes::{
    softaes_block_and, softaes_block_load, softaes_block_load64x2, softaes_block_store,
    softaes_block_xor, SoftAesBlock, _sodium_softaes_block_encrypt,
};

type aes_block_t = SoftAesBlock;

const AES_BLOCK_LENGTH: usize = 16;

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
unsafe fn aegis256_update(state: *mut aes_block_t, d: aes_block_t) {
    let tmp: aes_block_t;

    tmp = *state.add(5);
    *state.add(5) = AES_ENC(*state.add(4), *state.add(5));
    *state.add(4) = AES_ENC(*state.add(3), *state.add(4));
    *state.add(3) = AES_ENC(*state.add(2), *state.add(3));
    *state.add(2) = AES_ENC(*state.add(1), *state.add(2));
    *state.add(1) = AES_ENC(*state.add(0), *state.add(1));
    *state.add(0) = softaes_block_xor(AES_ENC(tmp, *state.add(0)), d);
}

// ---- aegis256_common.h ----

const RATE: usize = 16;

#[inline]
unsafe fn aegis256_init(key: *const u8, nonce: *const u8, state: *mut aes_block_t) {
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
    let k0 = softaes_block_load(key);
    let k1 = softaes_block_load(key.add(AES_BLOCK_LENGTH));
    let n0 = softaes_block_load(nonce);
    let n1 = softaes_block_load(nonce.add(AES_BLOCK_LENGTH));
    let k0_n0 = softaes_block_xor(k0, n0);
    let k1_n1 = softaes_block_xor(k1, n1);
    let mut i: c_int;

    *state.add(0) = k0_n0;
    *state.add(1) = k1_n1;
    *state.add(2) = c1;
    *state.add(3) = c0;
    *state.add(4) = softaes_block_xor(k0, c0);
    *state.add(5) = softaes_block_xor(k1, c1);
    i = 0;
    while i < 4 {
        aegis256_update(state, k0);
        aegis256_update(state, k1);
        aegis256_update(state, k0_n0);
        aegis256_update(state, k1_n1);
        i += 1;
    }
}

#[inline]
unsafe fn aegis256_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: *mut aes_block_t,
) -> c_int {
    let mut tmp: aes_block_t;
    let mut i: c_int;

    tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, *state.add(3));

    i = 0;
    while i < 7 {
        aegis256_update(state, tmp);
        i += 1;
    }

    if maclen == 16 {
        /* LCOV_EXCL_START */
        tmp = softaes_block_xor(*state.add(5), *state.add(4));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(3), *state.add(2)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
    /* LCOV_EXCL_STOP */
    } else if maclen == 32 {
        tmp = softaes_block_xor(softaes_block_xor(*state.add(2), *state.add(1)), *state.add(0));
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(softaes_block_xor(*state.add(5), *state.add(4)), *state.add(3));
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac as *mut c_void, 0, maclen); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[inline]
unsafe fn aegis256_absorb(src: *const u8, state: *mut aes_block_t) {
    let msg: aes_block_t;

    msg = softaes_block_load(src);
    aegis256_update(state, msg);
}

#[inline]
unsafe fn aegis256_absorb2(src: *const u8, state: *mut aes_block_t) {
    let msg: aes_block_t;
    let msg2: aes_block_t;

    msg = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    msg2 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    aegis256_update(state, msg);
    aegis256_update(state, msg2);
}

#[inline]
unsafe fn aegis256_enc(dst: *mut u8, src: *const u8, state: *mut aes_block_t) {
    let msg: aes_block_t;
    let mut tmp: aes_block_t;

    msg = softaes_block_load(src);
    tmp = softaes_block_xor(msg, *state.add(5));
    tmp = softaes_block_xor(tmp, *state.add(4));
    tmp = softaes_block_xor(tmp, *state.add(1));
    tmp = softaes_block_xor(tmp, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(dst, tmp);

    aegis256_update(state, msg);
}

#[inline]
unsafe fn aegis256_dec(dst: *mut u8, src: *const u8, state: *mut aes_block_t) {
    let mut msg: aes_block_t;

    msg = softaes_block_load(src);
    msg = softaes_block_xor(msg, *state.add(5));
    msg = softaes_block_xor(msg, *state.add(4));
    msg = softaes_block_xor(msg, *state.add(1));
    msg = softaes_block_xor(msg, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(dst, msg);

    aegis256_update(state, msg);
}

#[inline]
unsafe fn aegis256_declast(dst: *mut u8, src: *const u8, len: usize, state: *mut aes_block_t) {
    let mut pad: [u8; RATE] = [0; RATE];
    let mut msg: aes_block_t;

    memset(pad.as_mut_ptr() as *mut c_void, 0, core::mem::size_of::<[u8; RATE]>());
    memcpy(pad.as_mut_ptr() as *mut c_void, src as *const c_void, len);

    msg = softaes_block_load(pad.as_ptr());
    msg = softaes_block_xor(msg, *state.add(5));
    msg = softaes_block_xor(msg, *state.add(4));
    msg = softaes_block_xor(msg, *state.add(1));
    msg = softaes_block_xor(msg, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(pad.as_mut_ptr(), msg);

    memset(
        pad.as_mut_ptr().add(len) as *mut c_void,
        0,
        core::mem::size_of::<[u8; RATE]>() - len,
    );
    memcpy(dst as *mut c_void, pad.as_ptr() as *const c_void, len);

    msg = softaes_block_load(pad.as_ptr());

    aegis256_update(state, msg);
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
    let mut state: [aes_block_t; 6] = [SoftAesBlock {
        w0: 0,
        w1: 0,
        w2: 0,
        w3: 0,
    }; 6];
    let mut src: [u8; RATE] = [0; RATE];
    let mut dst: [u8; RATE] = [0; RATE];
    let mut i: usize;

    aegis256_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), state.as_mut_ptr());
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, RATE);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % RATE);
        aegis256_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    i = 0;
    while i + RATE <= mlen {
        aegis256_enc(c.add(i), m.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if mlen % RATE != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, RATE);
        memcpy(src.as_mut_ptr() as *mut c_void, m.add(i) as *const c_void, mlen % RATE);
        aegis256_enc(dst.as_mut_ptr(), src.as_ptr(), state.as_mut_ptr());
        memcpy(c.add(i) as *mut c_void, dst.as_ptr() as *const c_void, mlen % RATE);
    }

    aegis256_mac(mac, maclen, adlen as u64, mlen as u64, state.as_mut_ptr())
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
    let mut state: [aes_block_t; 6] = [SoftAesBlock {
        w0: 0,
        w1: 0,
        w2: 0,
        w3: 0,
    }; 6];
    let mut src: [u8; RATE] = [0; RATE];
    let mut dst: [u8; RATE] = [0; RATE];
    let mut computed_mac: [u8; 32] = [0; 32];
    let mlen: usize = clen;
    let mut i: usize;
    let mut ret: c_int;

    aegis256_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), state.as_mut_ptr());
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, RATE);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % RATE);
        aegis256_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(m.add(i), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(dst.as_mut_ptr(), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis256_declast(m.add(i), c.add(i), mlen % RATE, state.as_mut_ptr());
        } else {
            aegis256_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, state.as_mut_ptr());
        }
    }

    // COMPILER_ASSERT(sizeof computed_mac >= 32);
    ret = -1;
    if aegis256_mac(
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
pub struct aegis256_implementation {
    pub encrypt_detached: Option<EncryptDetachedFn>,
    pub decrypt_detached: Option<DecryptDetachedFn>,
}

unsafe impl Sync for aegis256_implementation {}

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
pub static aegis256_soft_implementation: aegis256_implementation = aegis256_implementation {
    encrypt_detached: Some(encrypt_detached_c),
    decrypt_detached: Some(decrypt_detached_c),
};
