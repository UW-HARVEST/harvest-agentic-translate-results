//! Translation of `crypto_aead/aegis128l/aegis128l_soft.c`.
//!
//! That C file `#include`s `aegis128l_common.h`, which contains the whole
//! AEGIS-128L implementation parameterised over the `AES_BLOCK_*` macros. Here
//! the macros are bound to the portable (`softaes`) backend, exactly as in
//! `aegis128l_soft.c`:
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
//! `aegis128l_soft_implementation` structure.

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
pub struct aegis128l_implementation {
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
/* `aegis128l_soft.c` / `aegis128l_common.h`                                 */
/* ------------------------------------------------------------------------- */

const AES_BLOCK_LENGTH: usize = 16;
const RATE: usize = 32;

const ZERO_BLOCK: SoftAesBlock = SoftAesBlock {
    w0: 0,
    w1: 0,
    w2: 0,
    w3: 0,
};

#[inline(always)]
fn aegis128l_update(state: &mut [SoftAesBlock; 8], d1: SoftAesBlock, d2: SoftAesBlock) {
    let tmp: SoftAesBlock;

    tmp = state[7];
    state[7] = softaes_block_encrypt(state[6], state[7]);
    state[6] = softaes_block_encrypt(state[5], state[6]);
    state[5] = softaes_block_encrypt(state[4], state[5]);
    state[4] = softaes_block_encrypt(state[3], state[4]);
    state[3] = softaes_block_encrypt(state[2], state[3]);
    state[2] = softaes_block_encrypt(state[1], state[2]);
    state[1] = softaes_block_encrypt(state[0], state[1]);
    state[0] = softaes_block_encrypt(tmp, state[0]);

    state[0] = softaes_block_xor(state[0], d1);
    state[4] = softaes_block_xor(state[4], d2);
}

#[inline(always)]
unsafe fn aegis128l_init(key: *const u8, nonce: *const u8, state: &mut [SoftAesBlock; 8]) {
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
    let k: SoftAesBlock;
    let n: SoftAesBlock;
    let mut i: c_int;

    k = softaes_block_load(key);
    n = softaes_block_load(nonce);

    state[0] = softaes_block_xor(k, n);
    state[1] = c1;
    state[2] = c0;
    state[3] = c1;
    state[4] = softaes_block_xor(k, n);
    state[5] = softaes_block_xor(k, c0);
    state[6] = softaes_block_xor(k, c1);
    state[7] = softaes_block_xor(k, c0);
    i = 0;
    while i < 10 {
        aegis128l_update(state, n, k);
        i += 1;
    }
}

#[inline(always)]
unsafe fn aegis128l_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: &mut [SoftAesBlock; 8],
) -> c_int {
    let mut tmp: SoftAesBlock;
    let mut i: c_int;

    tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, state[2]);

    i = 0;
    while i < 7 {
        aegis128l_update(state, tmp, tmp);
        i += 1;
    }

    if maclen == 16 {
        /* LCOV_EXCL_START */
        tmp = softaes_block_xor(state[6], softaes_block_xor(state[5], state[4]));
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[3], state[2]));
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
        softaes_block_store(mac, tmp);
        /* LCOV_EXCL_STOP */
    } else if maclen == 32 {
        tmp = softaes_block_xor(state[3], state[2]);
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(state[7], state[6]);
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[5], state[4]));
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac, 0, maclen); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[inline(always)]
unsafe fn aegis128l_absorb(src: *const u8, state: &mut [SoftAesBlock; 8]) {
    let msg0: SoftAesBlock;
    let msg1: SoftAesBlock;

    msg0 = softaes_block_load(src);
    msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_absorb2(src: *const u8, state: &mut [SoftAesBlock; 8]) {
    let msg0: SoftAesBlock;
    let msg1: SoftAesBlock;
    let msg2: SoftAesBlock;
    let msg3: SoftAesBlock;

    msg0 = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    msg1 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    msg2 = softaes_block_load(src.add(2 * AES_BLOCK_LENGTH));
    msg3 = softaes_block_load(src.add(3 * AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
    aegis128l_update(state, msg2, msg3);
}

#[inline(always)]
unsafe fn aegis128l_enc(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 8]) {
    let msg0: SoftAesBlock;
    let msg1: SoftAesBlock;
    let mut tmp0: SoftAesBlock;
    let mut tmp1: SoftAesBlock;

    msg0 = softaes_block_load(src);
    msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    tmp0 = softaes_block_xor(msg0, state[6]);
    tmp0 = softaes_block_xor(tmp0, state[1]);
    tmp1 = softaes_block_xor(msg1, state[5]);
    tmp1 = softaes_block_xor(tmp1, state[2]);
    tmp0 = softaes_block_xor(tmp0, softaes_block_and(state[2], state[3]));
    tmp1 = softaes_block_xor(tmp1, softaes_block_and(state[6], state[7]));
    softaes_block_store(dst, tmp0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), tmp1);

    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_dec(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 8]) {
    let mut msg0: SoftAesBlock;
    let mut msg1: SoftAesBlock;

    msg0 = softaes_block_load(src);
    msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, state[6]);
    msg0 = softaes_block_xor(msg0, state[1]);
    msg1 = softaes_block_xor(msg1, state[5]);
    msg1 = softaes_block_xor(msg1, state[2]);
    msg0 = softaes_block_xor(msg0, softaes_block_and(state[2], state[3]));
    msg1 = softaes_block_xor(msg1, softaes_block_and(state[6], state[7]));
    softaes_block_store(dst, msg0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), msg1);

    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_declast(
    dst: *mut u8,
    src: *const u8,
    len: usize,
    state: &mut [SoftAesBlock; 8],
) {
    let mut pad: [u8; RATE] = [0u8; RATE];
    let mut msg0: SoftAesBlock;
    let mut msg1: SoftAesBlock;

    memset(pad.as_mut_ptr(), 0, RATE);
    memcpy(pad.as_mut_ptr(), src, len);

    msg0 = softaes_block_load(pad.as_ptr());
    msg1 = softaes_block_load(pad.as_ptr().add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, state[6]);
    msg0 = softaes_block_xor(msg0, state[1]);
    msg1 = softaes_block_xor(msg1, state[5]);
    msg1 = softaes_block_xor(msg1, state[2]);
    msg0 = softaes_block_xor(msg0, softaes_block_and(state[2], state[3]));
    msg1 = softaes_block_xor(msg1, softaes_block_and(state[6], state[7]));
    softaes_block_store(pad.as_mut_ptr(), msg0);
    softaes_block_store(pad.as_mut_ptr().add(AES_BLOCK_LENGTH), msg1);

    memset(pad.as_mut_ptr().add(len), 0, RATE - len);
    memcpy(dst, pad.as_ptr(), len);

    msg0 = softaes_block_load(pad.as_ptr());
    msg1 = softaes_block_load(pad.as_ptr().add(AES_BLOCK_LENGTH));

    aegis128l_update(state, msg0, msg1);
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
    let mut state: [SoftAesBlock; 8] = [ZERO_BLOCK; 8];
    let mut src: [u8; RATE] = [0u8; RATE];
    let mut dst: [u8; RATE] = [0u8; RATE];
    let mut i: usize;

    aegis128l_init(k, npub, &mut state);

    i = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), &mut state);
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis128l_absorb(src.as_ptr(), &mut state);
    }
    i = 0;
    while i + RATE <= mlen {
        aegis128l_enc(c.add(i), m.add(i), &mut state);
        i += RATE;
    }
    if mlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), m.add(i), mlen % RATE);
        aegis128l_enc(dst.as_mut_ptr(), src.as_ptr(), &mut state);
        memcpy(c.add(i), dst.as_ptr(), mlen % RATE);
    }

    aegis128l_mac(mac, maclen, adlen as u64, mlen as u64, &mut state)
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
    let mut state: [SoftAesBlock; 8] = [ZERO_BLOCK; 8];
    let mut src: [u8; RATE] = [0u8; RATE];
    let mut dst: [u8; RATE] = [0u8; RATE];
    let mut computed_mac: [u8; 32] = [0u8; 32];
    let mlen: usize = clen;
    let mut i: usize;
    let mut ret: c_int;

    aegis128l_init(k, npub, &mut state);

    i = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), &mut state);
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis128l_absorb(src.as_ptr(), &mut state);
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(m.add(i), c.add(i), &mut state);
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(dst.as_mut_ptr(), c.add(i), &mut state);
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis128l_declast(m.add(i), c.add(i), mlen % RATE, &mut state);
        } else {
            aegis128l_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, &mut state);
        }
    }

    /* COMPILER_ASSERT(sizeof computed_mac >= 32); */
    ret = -1;
    if aegis128l_mac(
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
pub static aegis128l_soft_implementation: aegis128l_implementation = aegis128l_implementation {
    encrypt_detached: encrypt_detached,
    decrypt_detached: decrypt_detached,
};
