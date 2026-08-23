//! Translation of `crypto_aead/aegis256/aegis256_soft.c`.
//!
//! That C file `#include`s `aegis256_common.h`, which contains the whole
//! AEGIS-256 implementation parameterised over the `AES_BLOCK_*` macros. Here
//! the macros are bound to the portable (`softaes`) backend, exactly as in
//! `aegis256_soft.c`:
//!
//! ```c
//! typedef SoftAesBlock aes_block_t;
//! #define AES_BLOCK_XOR(A, B)       softaes_block_xor((A), (B))
//! #define AES_BLOCK_AND(A, B)       softaes_block_and((A), (B))
//! #define AES_BLOCK_LOAD(A)         softaes_block_load(A)
//! #define AES_BLOCK_LOAD_64x2(A, B) softaes_block_load64x2((A), (B))
//! #define AES_BLOCK_STORE(A, B)     softaes_block_store((A), (B))
//! #define AES_ENC(A, B)             softaes_block_encrypt((A), (B))
//! ```
//!
//! Everything in this file has internal linkage in C except the
//! `aegis256_soft_implementation` structure.

use crate::common::*;
use core::ffi::c_int;

/* ------------------------------------------------------------------------- */
/* `private/softaes.h`                                                       */
/* ------------------------------------------------------------------------- */

#[repr(C)]
#[derive(Copy, Clone)]
struct SoftAesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

extern "C" {
    fn _sodium_softaes_expand_key128(rkeys: *mut SoftAesBlock, key: *const u8);
    fn _sodium_softaes_expand_key256(rkeys: *mut SoftAesBlock, key: *const u8);
    fn _sodium_softaes_inv_mix_columns(block: SoftAesBlock) -> SoftAesBlock;
    fn _sodium_softaes_invert_key_schedule128(rkeys: *mut SoftAesBlock);
    fn _sodium_softaes_invert_key_schedule256(rkeys: *mut SoftAesBlock);
    fn _sodium_softaes_block_encrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
    fn _sodium_softaes_block_decrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
    fn _sodium_softaes_block_encryptlast(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
    fn _sodium_softaes_block_decryptlast(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
}

extern "C" {
    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
}

/* `static inline` helpers from `private/softaes.h`. */

#[inline(always)]
unsafe fn softaes_block_load(in_: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: load32_le(in_.add(0)),
        w1: load32_le(in_.add(4)),
        w2: load32_le(in_.add(8)),
        w3: load32_le(in_.add(12)),
    }
}

#[inline(always)]
fn softaes_block_load64x2(a: u64, b: u64) -> SoftAesBlock {
    SoftAesBlock {
        w0: b as u32,
        w1: (b >> 32) as u32,
        w2: a as u32,
        w3: (a >> 32) as u32,
    }
}

#[inline(always)]
unsafe fn softaes_block_store(out: *mut u8, in_: SoftAesBlock) {
    store32_le(out.add(0), in_.w0);
    store32_le(out.add(4), in_.w1);
    store32_le(out.add(8), in_.w2);
    store32_le(out.add(12), in_.w3);
}

#[inline(always)]
fn softaes_block_xor(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: a.w0 ^ b.w0,
        w1: a.w1 ^ b.w1,
        w2: a.w2 ^ b.w2,
        w3: a.w3 ^ b.w3,
    }
}

#[inline(always)]
fn softaes_block_and(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: a.w0 & b.w0,
        w1: a.w1 & b.w1,
        w2: a.w2 & b.w2,
        w3: a.w3 & b.w3,
    }
}

#[inline(always)]
fn softaes_block_encrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock {
    unsafe { _sodium_softaes_block_encrypt(block, rk) }
}

/* ------------------------------------------------------------------------- */
/* `implementations.h`                                                       */
/* ------------------------------------------------------------------------- */

#[repr(C)]
pub struct aegis256_implementation {
    pub encrypt_detached: unsafe extern "C" fn(
        c: *mut u8,
        mac: *mut u8,
        maclen: usize,
        m: *const u8,
        mlen: usize,
        ad: *const u8,
        adlen: usize,
        npub: *const u8,
        k: *const u8,
    ) -> c_int,
    pub decrypt_detached: unsafe extern "C" fn(
        m: *mut u8,
        c: *const u8,
        clen: usize,
        mac: *const u8,
        maclen: usize,
        ad: *const u8,
        adlen: usize,
        npub: *const u8,
        k: *const u8,
    ) -> c_int,
}

/* ------------------------------------------------------------------------- */
/* `aegis256_soft.c` / `aegis256_common.h`                                   */
/* ------------------------------------------------------------------------- */

const AES_BLOCK_LENGTH: usize = 16;
const RATE: usize = 16;

const ZERO_BLOCK: SoftAesBlock = SoftAesBlock {
    w0: 0,
    w1: 0,
    w2: 0,
    w3: 0,
};

#[inline(always)]
fn aegis256_update(state: &mut [SoftAesBlock; 6], d: SoftAesBlock) {
    let tmp: SoftAesBlock;

    tmp = state[5];
    state[5] = softaes_block_encrypt(state[4], state[5]);
    state[4] = softaes_block_encrypt(state[3], state[4]);
    state[3] = softaes_block_encrypt(state[2], state[3]);
    state[2] = softaes_block_encrypt(state[1], state[2]);
    state[1] = softaes_block_encrypt(state[0], state[1]);
    state[0] = softaes_block_xor(softaes_block_encrypt(tmp, state[0]), d);
}

#[inline(always)]
unsafe fn aegis256_init(key: *const u8, nonce: *const u8, state: &mut [SoftAesBlock; 6]) {
    static c0_: [u8; AES_BLOCK_LENGTH] = [
        0x00, 0x01, 0x01, 0x02, 0x03, 0x05, 0x08, 0x0d, 0x15, 0x22, 0x37, 0x59, 0x90, 0xe9, 0x79,
        0x62,
    ];
    static c1_: [u8; AES_BLOCK_LENGTH] = [
        0xdb, 0x3d, 0x18, 0x55, 0x6d, 0xc2, 0x2f, 0xf1, 0x20, 0x11, 0x31, 0x42, 0x73, 0xb5, 0x28,
        0xdd,
    ];

    let c0: SoftAesBlock = softaes_block_load(c0_.as_ptr());
    let c1: SoftAesBlock = softaes_block_load(c1_.as_ptr());
    let k0: SoftAesBlock = softaes_block_load(key);
    let k1: SoftAesBlock = softaes_block_load(key.add(AES_BLOCK_LENGTH));
    let n0: SoftAesBlock = softaes_block_load(nonce);
    let n1: SoftAesBlock = softaes_block_load(nonce.add(AES_BLOCK_LENGTH));
    let k0_n0: SoftAesBlock = softaes_block_xor(k0, n0);
    let k1_n1: SoftAesBlock = softaes_block_xor(k1, n1);
    let mut i: c_int;

    state[0] = k0_n0;
    state[1] = k1_n1;
    state[2] = c1;
    state[3] = c0;
    state[4] = softaes_block_xor(k0, c0);
    state[5] = softaes_block_xor(k1, c1);
    i = 0;
    while i < 4 {
        aegis256_update(state, k0);
        aegis256_update(state, k1);
        aegis256_update(state, k0_n0);
        aegis256_update(state, k1_n1);
        i += 1;
    }
}

#[inline(always)]
unsafe fn aegis256_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: &mut [SoftAesBlock; 6],
) -> c_int {
    let mut tmp: SoftAesBlock;
    let mut i: c_int;

    tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, state[3]);

    i = 0;
    while i < 7 {
        aegis256_update(state, tmp);
        i += 1;
    }

    if maclen == 16 {
        /* LCOV_EXCL_START */
        tmp = softaes_block_xor(state[5], state[4]);
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[3], state[2]));
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
        softaes_block_store(mac, tmp);
        /* LCOV_EXCL_STOP */
    } else if maclen == 32 {
        tmp = softaes_block_xor(softaes_block_xor(state[2], state[1]), state[0]);
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(softaes_block_xor(state[5], state[4]), state[3]);
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac, 0, maclen); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[inline(always)]
unsafe fn aegis256_absorb(src: *const u8, state: &mut [SoftAesBlock; 6]) {
    let msg: SoftAesBlock;

    msg = softaes_block_load(src);
    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_absorb2(src: *const u8, state: &mut [SoftAesBlock; 6]) {
    let msg: SoftAesBlock;
    let msg2: SoftAesBlock;

    msg = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    msg2 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    aegis256_update(state, msg);
    aegis256_update(state, msg2);
}

#[inline(always)]
unsafe fn aegis256_enc(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 6]) {
    let msg: SoftAesBlock;
    let mut tmp: SoftAesBlock;

    msg = softaes_block_load(src);
    tmp = softaes_block_xor(msg, state[5]);
    tmp = softaes_block_xor(tmp, state[4]);
    tmp = softaes_block_xor(tmp, state[1]);
    tmp = softaes_block_xor(tmp, softaes_block_and(state[2], state[3]));
    softaes_block_store(dst, tmp);

    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_dec(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 6]) {
    let mut msg: SoftAesBlock;

    msg = softaes_block_load(src);
    msg = softaes_block_xor(msg, state[5]);
    msg = softaes_block_xor(msg, state[4]);
    msg = softaes_block_xor(msg, state[1]);
    msg = softaes_block_xor(msg, softaes_block_and(state[2], state[3]));
    softaes_block_store(dst, msg);

    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_declast(
    dst: *mut u8,
    src: *const u8,
    len: usize,
    state: &mut [SoftAesBlock; 6],
) {
    let mut pad: [u8; RATE] = [0u8; RATE];
    let mut msg: SoftAesBlock;

    memset(pad.as_mut_ptr(), 0, RATE);
    memcpy(pad.as_mut_ptr(), src, len);

    msg = softaes_block_load(pad.as_ptr());
    msg = softaes_block_xor(msg, state[5]);
    msg = softaes_block_xor(msg, state[4]);
    msg = softaes_block_xor(msg, state[1]);
    msg = softaes_block_xor(msg, softaes_block_and(state[2], state[3]));
    softaes_block_store(pad.as_mut_ptr(), msg);

    memset(pad.as_mut_ptr().add(len), 0, RATE - len);
    memcpy(dst, pad.as_ptr(), len);

    msg = softaes_block_load(pad.as_ptr());

    aegis256_update(state, msg);
}

unsafe extern "C" fn encrypt_detached(
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
    let mut state: [SoftAesBlock; 6] = [ZERO_BLOCK; 6];
    let mut src: [u8; RATE] = [0u8; RATE];
    let mut dst: [u8; RATE] = [0u8; RATE];
    let mut i: usize;

    aegis256_init(k, npub, &mut state);

    i = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), &mut state);
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis256_absorb(src.as_ptr(), &mut state);
    }
    i = 0;
    while i + RATE <= mlen {
        aegis256_enc(c.add(i), m.add(i), &mut state);
        i += RATE;
    }
    if mlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), m.add(i), mlen % RATE);
        aegis256_enc(dst.as_mut_ptr(), src.as_ptr(), &mut state);
        memcpy(c.add(i), dst.as_ptr(), mlen % RATE);
    }

    aegis256_mac(mac, maclen, adlen as u64, mlen as u64, &mut state)
}

unsafe extern "C" fn decrypt_detached(
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
    let mut state: [SoftAesBlock; 6] = [ZERO_BLOCK; 6];
    let mut src: [u8; RATE] = [0u8; RATE];
    let mut dst: [u8; RATE] = [0u8; RATE];
    let mut computed_mac: [u8; 32] = [0u8; 32];
    let mlen: usize = clen;
    let mut i: usize;
    let mut ret: c_int;

    aegis256_init(k, npub, &mut state);

    i = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), &mut state);
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis256_absorb(src.as_ptr(), &mut state);
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(m.add(i), c.add(i), &mut state);
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(dst.as_mut_ptr(), c.add(i), &mut state);
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis256_declast(m.add(i), c.add(i), mlen % RATE, &mut state);
        } else {
            aegis256_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, &mut state);
        }
    }

    /* COMPILER_ASSERT(sizeof computed_mac >= 32); */
    ret = -1;
    if aegis256_mac(
        computed_mac.as_mut_ptr(),
        maclen,
        adlen as u64,
        mlen as u64,
        &mut state,
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
            memset(m, 0, mlen);
        }
        return ret;
    }
    /* ACQUIRE_FENCE expands to `(void) 0` in this build. */
    0
}

#[unsafe(no_mangle)]
pub static aegis256_soft_implementation: aegis256_implementation = aegis256_implementation {
    encrypt_detached: encrypt_detached,
    decrypt_detached: decrypt_detached,
};
