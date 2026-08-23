//! Translation of `crypto_aead/aegis256/aegis256_soft.c` together with the
//! shared AEGIS-256 core it `#include`s, `crypto_aead/aegis256/aegis256_common.h`.
//!
//! `aegis256_soft.c` wraps its whole body in `#if 1`, so it is always compiled.
//! It selects the portable software AES round function
//! (`softaes_block_encrypt`), hence:
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
//! The reference build defines no `HAVE_*_MEMORY_FENCES`, so `ACQUIRE_FENCE`
//! expands to `(void) 0`, and `NDEBUG` is set so `COMPILER_ASSERT` is a no-op
//! (it is a compile-time construct anyway).

use core::ffi::c_int;

use crate::common::{memcpy, memset};
use crate::crypto_core::softaes::{
    SoftAesBlock, softaes_block_and, softaes_block_encrypt, softaes_block_load,
    softaes_block_load64x2, softaes_block_store, softaes_block_xor,
};
use crate::crypto_verify::{crypto_verify_16, crypto_verify_32};

use super::aegis256_implementation;

/// `#define AES_BLOCK_LENGTH 16`
const AES_BLOCK_LENGTH: usize = 16;

/// `#define RATE 16` (aegis256_common.h)
const RATE: usize = 16;

/// ```c
/// static inline void
/// aegis256_update(aes_block_t *const state, const aes_block_t d)
/// ```
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

/// ```c
/// static inline void
/// aegis256_init(const uint8_t *key, const uint8_t *nonce, aes_block_t *const state)
/// ```
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

    unsafe {
        let c0 = softaes_block_load(c0_.as_ptr());
        let c1 = softaes_block_load(c1_.as_ptr());
        let k0 = softaes_block_load(key);
        let k1 = softaes_block_load(key.add(AES_BLOCK_LENGTH));
        let n0 = softaes_block_load(nonce);
        let n1 = softaes_block_load(nonce.add(AES_BLOCK_LENGTH));
        let k0_n0 = softaes_block_xor(k0, n0);
        let k1_n1 = softaes_block_xor(k1, n1);
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
}

/// ```c
/// static inline int
/// aegis256_mac(uint8_t *mac, size_t maclen, uint64_t adlen, uint64_t mlen,
///              aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis256_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: &mut [SoftAesBlock; 6],
) -> c_int {
    unsafe {
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
            tmp = softaes_block_xor(state[5], state[4]);
            tmp = softaes_block_xor(tmp, softaes_block_xor(state[3], state[2]));
            tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
            softaes_block_store(mac, tmp);
        } else if maclen == 32 {
            tmp = softaes_block_xor(softaes_block_xor(state[2], state[1]), state[0]);
            softaes_block_store(mac, tmp);
            tmp = softaes_block_xor(softaes_block_xor(state[5], state[4]), state[3]);
            softaes_block_store(mac.add(16), tmp);
        } else {
            memset(mac, 0, maclen);
            return -1;
        }
        0
    }
}

/// ```c
/// static inline void
/// aegis256_absorb(const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis256_absorb(src: *const u8, state: &mut [SoftAesBlock; 6]) {
    unsafe {
        let msg: SoftAesBlock;

        msg = softaes_block_load(src);
        aegis256_update(state, msg);
    }
}

/// ```c
/// static inline void
/// aegis256_absorb2(const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis256_absorb2(src: *const u8, state: &mut [SoftAesBlock; 6]) {
    unsafe {
        let msg: SoftAesBlock;
        let msg2: SoftAesBlock;

        msg = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
        msg2 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
        aegis256_update(state, msg);
        aegis256_update(state, msg2);
    }
}

/// ```c
/// static inline void
/// aegis256_enc(uint8_t *const dst, const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis256_enc(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 6]) {
    unsafe {
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
}

/// ```c
/// static inline void
/// aegis256_dec(uint8_t *const dst, const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis256_dec(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 6]) {
    unsafe {
        let mut msg: SoftAesBlock;

        msg = softaes_block_load(src);
        msg = softaes_block_xor(msg, state[5]);
        msg = softaes_block_xor(msg, state[4]);
        msg = softaes_block_xor(msg, state[1]);
        msg = softaes_block_xor(msg, softaes_block_and(state[2], state[3]));
        softaes_block_store(dst, msg);

        aegis256_update(state, msg);
    }
}

/// ```c
/// static inline void
/// aegis256_declast(uint8_t *const dst, const uint8_t *const src, size_t len,
///                  aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis256_declast(
    dst: *mut u8,
    src: *const u8,
    len: usize,
    state: &mut [SoftAesBlock; 6],
) {
    unsafe {
        let mut pad: [u8; RATE] = [0; RATE];
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
}

/// ```c
/// static int
/// encrypt_detached(uint8_t *c, uint8_t *mac, size_t maclen, const uint8_t *m, size_t mlen,
///                  const uint8_t *ad, size_t adlen, const uint8_t *npub, const uint8_t *k)
/// ```
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
    unsafe {
        let mut state: [SoftAesBlock; 6] = [SoftAesBlock {
            w0: 0,
            w1: 0,
            w2: 0,
            w3: 0,
        }; 6];
        let mut src: [u8; RATE] = [0; RATE];
        let mut dst: [u8; RATE] = [0; RATE];
        let mut i: usize;

        aegis256_init(k, npub, &mut state);

        i = 0;
        while i.wrapping_add(2 * RATE) <= adlen {
            aegis256_absorb2(ad.add(i), &mut state);
            i = i.wrapping_add(2 * RATE);
        }
        while i.wrapping_add(RATE) <= adlen {
            aegis256_absorb(ad.add(i), &mut state);
            i = i.wrapping_add(RATE);
        }
        if adlen % RATE != 0 {
            memset(src.as_mut_ptr(), 0, RATE);
            memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
            aegis256_absorb(src.as_ptr(), &mut state);
        }
        i = 0;
        while i.wrapping_add(RATE) <= mlen {
            aegis256_enc(c.add(i), m.add(i), &mut state);
            i = i.wrapping_add(RATE);
        }
        if mlen % RATE != 0 {
            memset(src.as_mut_ptr(), 0, RATE);
            memcpy(src.as_mut_ptr(), m.add(i), mlen % RATE);
            aegis256_enc(dst.as_mut_ptr(), src.as_ptr(), &mut state);
            memcpy(c.add(i), dst.as_ptr(), mlen % RATE);
        }

        aegis256_mac(mac, maclen, adlen as u64, mlen as u64, &mut state)
    }
}

/// ```c
/// static int
/// decrypt_detached(uint8_t *m, const uint8_t *c, size_t clen, const uint8_t *mac, size_t maclen,
///                  const uint8_t *ad, size_t adlen, const uint8_t *npub, const uint8_t *k)
/// ```
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
    unsafe {
        let mut state: [SoftAesBlock; 6] = [SoftAesBlock {
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

        aegis256_init(k, npub, &mut state);

        i = 0;
        while i.wrapping_add(2 * RATE) <= adlen {
            aegis256_absorb2(ad.add(i), &mut state);
            i = i.wrapping_add(2 * RATE);
        }
        while i.wrapping_add(RATE) <= adlen {
            aegis256_absorb(ad.add(i), &mut state);
            i = i.wrapping_add(RATE);
        }
        if adlen % RATE != 0 {
            memset(src.as_mut_ptr(), 0, RATE);
            memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
            aegis256_absorb(src.as_ptr(), &mut state);
        }
        if !m.is_null() {
            i = 0;
            while i.wrapping_add(RATE) <= mlen {
                aegis256_dec(m.add(i), c.add(i), &mut state);
                i = i.wrapping_add(RATE);
            }
        } else {
            i = 0;
            while i.wrapping_add(RATE) <= mlen {
                aegis256_dec(dst.as_mut_ptr(), c.add(i), &mut state);
                i = i.wrapping_add(RATE);
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
                ret = crypto_verify_16(computed_mac.as_ptr(), mac);
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
        /* ACQUIRE_FENCE; -> (void) 0 */
        0
    }
}

/// ```c
/// struct aegis256_implementation aegis256_soft_implementation = {
///     SODIUM_C99(.encrypt_detached =) encrypt_detached,
///     SODIUM_C99(.decrypt_detached =) decrypt_detached
/// };
/// ```
#[unsafe(no_mangle)]
pub static mut aegis256_soft_implementation: aegis256_implementation = aegis256_implementation {
    encrypt_detached: encrypt_detached,
    decrypt_detached: decrypt_detached,
};
