//! Translation of `crypto_aead/aegis128l/aegis128l_soft.c` together with the
//! shared AEGIS-128L core it `#include`s, `crypto_aead/aegis128l/aegis128l_common.h`.
//!
//! `aegis128l_soft.c` wraps its whole body in `#if 1`, so it is always
//! compiled.  It selects the portable software AES round function
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

use super::aegis128l_implementation;

/// `#define AES_BLOCK_LENGTH 16`
const AES_BLOCK_LENGTH: usize = 16;

/// `#define RATE 32` (aegis128l_common.h)
const RATE: usize = 32;

/// ```c
/// static inline void
/// aegis128l_update(aes_block_t *const state, const aes_block_t d1, const aes_block_t d2)
/// ```
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

/// ```c
/// static inline void
/// aegis128l_init(const uint8_t *key, const uint8_t *nonce, aes_block_t *const state)
/// ```
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

    unsafe {
        let c0 = softaes_block_load(c0_.as_ptr());
        let c1 = softaes_block_load(c1_.as_ptr());
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
}

/// ```c
/// static inline int
/// aegis128l_mac(uint8_t *mac, size_t maclen, uint64_t adlen, uint64_t mlen,
///               aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis128l_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: &mut [SoftAesBlock; 8],
) -> c_int {
    unsafe {
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
            tmp = softaes_block_xor(state[6], softaes_block_xor(state[5], state[4]));
            tmp = softaes_block_xor(tmp, softaes_block_xor(state[3], state[2]));
            tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
            softaes_block_store(mac, tmp);
        } else if maclen == 32 {
            tmp = softaes_block_xor(state[3], state[2]);
            tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
            softaes_block_store(mac, tmp);
            tmp = softaes_block_xor(state[7], state[6]);
            tmp = softaes_block_xor(tmp, softaes_block_xor(state[5], state[4]));
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
/// aegis128l_absorb(const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis128l_absorb(src: *const u8, state: &mut [SoftAesBlock; 8]) {
    unsafe {
        let msg0: SoftAesBlock;
        let msg1: SoftAesBlock;

        msg0 = softaes_block_load(src);
        msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
        aegis128l_update(state, msg0, msg1);
    }
}

/// ```c
/// static inline void
/// aegis128l_absorb2(const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis128l_absorb2(src: *const u8, state: &mut [SoftAesBlock; 8]) {
    unsafe {
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
}

/// ```c
/// static inline void
/// aegis128l_enc(uint8_t *const dst, const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis128l_enc(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 8]) {
    unsafe {
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
}

/// ```c
/// static inline void
/// aegis128l_dec(uint8_t *const dst, const uint8_t *const src, aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis128l_dec(dst: *mut u8, src: *const u8, state: &mut [SoftAesBlock; 8]) {
    unsafe {
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
}

/// ```c
/// static inline void
/// aegis128l_declast(uint8_t *const dst, const uint8_t *const src, size_t len,
///                   aes_block_t *const state)
/// ```
#[inline(always)]
unsafe fn aegis128l_declast(
    dst: *mut u8,
    src: *const u8,
    len: usize,
    state: &mut [SoftAesBlock; 8],
) {
    unsafe {
        let mut pad: [u8; RATE] = [0; RATE];
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
        let mut state: [SoftAesBlock; 8] = [SoftAesBlock {
            w0: 0,
            w1: 0,
            w2: 0,
            w3: 0,
        }; 8];
        let mut src: [u8; RATE] = [0; RATE];
        let mut dst: [u8; RATE] = [0; RATE];
        let mut i: usize;

        aegis128l_init(k, npub, &mut state);

        i = 0;
        while i.wrapping_add(RATE * 2) <= adlen {
            aegis128l_absorb2(ad.add(i), &mut state);
            i = i.wrapping_add(RATE * 2);
        }
        while i.wrapping_add(RATE) <= adlen {
            aegis128l_absorb(ad.add(i), &mut state);
            i = i.wrapping_add(RATE);
        }
        if adlen % RATE != 0 {
            memset(src.as_mut_ptr(), 0, RATE);
            memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
            aegis128l_absorb(src.as_ptr(), &mut state);
        }
        i = 0;
        while i.wrapping_add(RATE) <= mlen {
            aegis128l_enc(c.add(i), m.add(i), &mut state);
            i = i.wrapping_add(RATE);
        }
        if mlen % RATE != 0 {
            memset(src.as_mut_ptr(), 0, RATE);
            memcpy(src.as_mut_ptr(), m.add(i), mlen % RATE);
            aegis128l_enc(dst.as_mut_ptr(), src.as_ptr(), &mut state);
            memcpy(c.add(i), dst.as_ptr(), mlen % RATE);
        }

        aegis128l_mac(mac, maclen, adlen as u64, mlen as u64, &mut state)
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
        let mut state: [SoftAesBlock; 8] = [SoftAesBlock {
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

        aegis128l_init(k, npub, &mut state);

        i = 0;
        while i.wrapping_add(RATE * 2) <= adlen {
            aegis128l_absorb2(ad.add(i), &mut state);
            i = i.wrapping_add(RATE * 2);
        }
        while i.wrapping_add(RATE) <= adlen {
            aegis128l_absorb(ad.add(i), &mut state);
            i = i.wrapping_add(RATE);
        }
        if adlen % RATE != 0 {
            memset(src.as_mut_ptr(), 0, RATE);
            memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
            aegis128l_absorb(src.as_ptr(), &mut state);
        }
        if !m.is_null() {
            i = 0;
            while i.wrapping_add(RATE) <= mlen {
                aegis128l_dec(m.add(i), c.add(i), &mut state);
                i = i.wrapping_add(RATE);
            }
        } else {
            i = 0;
            while i.wrapping_add(RATE) <= mlen {
                aegis128l_dec(dst.as_mut_ptr(), c.add(i), &mut state);
                i = i.wrapping_add(RATE);
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
/// struct aegis128l_implementation aegis128l_soft_implementation = {
///     SODIUM_C99(.encrypt_detached =) encrypt_detached,
///     SODIUM_C99(.decrypt_detached =) decrypt_detached
/// };
/// ```
#[unsafe(no_mangle)]
pub static mut aegis128l_soft_implementation: aegis128l_implementation = aegis128l_implementation {
    encrypt_detached: encrypt_detached,
    decrypt_detached: decrypt_detached,
};
