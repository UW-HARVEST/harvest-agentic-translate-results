//! Translation of `crypto_ipcrypt/ipcrypt_soft.c`.
//!
//! The C file binds the generic `AES_BLOCK_*` / `AES_ENC*` macros to the
//! portable (`softaes`) backend:
//!
//! ```c
//! typedef SoftAesBlock aes_block_t;
//! #define AES_BLOCK_XOR(A, B)       softaes_block_xor((A), (B))
//! #define AES_BLOCK_AND(A, B)       softaes_block_and((A), (B))
//! #define AES_BLOCK_LOAD(A)         softaes_block_load(A)
//! #define AES_BLOCK_LOAD_64x2(A, B) softaes_block_load64x2((A), (B))
//! #define AES_BLOCK_STORE(A, B)     softaes_block_store((A), (B))
//! #define AES_ENC(A, B)             softaes_block_encrypt((A), (B))
//! #define AES_ENCLAST(A, B)         softaes_block_encryptlast((A), (B))
//! #define AES_DEC(A, B)             softaes_block_decrypt((A), (B))
//! #define AES_DECLAST(A, B)         softaes_block_decryptlast((A), (B))
//! #define AES_INV_MIX(A)            softaes_inv_mix_columns((A))
//! ```
//!
//! Every item here has internal linkage in C except the
//! `ipcrypt_soft_implementation` structure.

use crate::common::*;
use core::ffi::{c_int, c_uint, c_void};

const ROUNDS: usize = 10;

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

type AesBlock = SoftAesBlock;

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
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/* `static inline` helpers from `private/softaes.h`, duplicated here. */

#[inline(always)]
unsafe fn softaes_block_load(in_: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: load32_le(in_.add(0)),
        w1: load32_le(in_.add(4)),
        w2: load32_le(in_.add(8)),
        w3: load32_le(in_.add(12)),
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[inline(always)]
fn softaes_block_and(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: a.w0 & b.w0,
        w1: a.w1 & b.w1,
        w2: a.w2 & b.w2,
        w3: a.w3 & b.w3,
    }
}

/* ------------------------------------------------------------------------- */
/* `crypto_ipcrypt/implementations.h`                                        */
/* ------------------------------------------------------------------------- */

#[repr(C)]
pub struct ipcrypt_implementation {
    pub encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub nd_encrypt:
        unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub nd_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub ndx_encrypt:
        unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub ndx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
}

/* `typedef aes_block_t KeySchedule[1 + ROUNDS];` */
type KeySchedule = [AesBlock; 1 + ROUNDS];

const ZERO_BLOCK: AesBlock = SoftAesBlock {
    w0: 0,
    w1: 0,
    w2: 0,
    w3: 0,
};

#[inline(always)]
unsafe fn expand_key(rkeys: &mut KeySchedule, key: *const u8) {
    _sodium_softaes_expand_key128(rkeys.as_mut_ptr(), key);
}

unsafe fn aes_encrypt(out: *mut u8, in_: *const u8, rkeys: &KeySchedule) {
    let mut t: AesBlock;
    let mut i: usize;

    t = softaes_block_xor(softaes_block_load(in_), rkeys[0]);
    i = 1;
    while i < ROUNDS {
        t = _sodium_softaes_block_encrypt(t, rkeys[i]);
        i += 1;
    }
    t = _sodium_softaes_block_encryptlast(t, rkeys[ROUNDS]);
    softaes_block_store(out, t);
}

unsafe fn aes_decrypt(out: *mut u8, in_: *const u8, rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut t: AesBlock;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = rkeys[i];
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(softaes_block_load(in_), rkeys_inv[ROUNDS]);
    i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, rkeys_inv[i]);
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, rkeys_inv[0]);
    softaes_block_store(out, t);
    sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn tweak_expand(tweak: *const u8) -> AesBlock {
    let mut out = ZERO_BLOCK;

    out.w0 = (*tweak.add(0) as u32) | ((*tweak.add(1) as u32) << 8);
    out.w1 = (*tweak.add(2) as u32) | ((*tweak.add(3) as u32) << 8);
    out.w2 = (*tweak.add(4) as u32) | ((*tweak.add(5) as u32) << 8);
    out.w3 = (*tweak.add(6) as u32) | ((*tweak.add(7) as u32) << 8);

    out
}

unsafe fn aes_encrypt_with_tweak(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    rkeys: &KeySchedule,
) {
    let tweak_block: AesBlock = tweak_expand(tweak);
    let mut t: AesBlock;
    let mut i: usize;

    t = softaes_block_xor(
        softaes_block_xor(softaes_block_load(in_), tweak_block),
        rkeys[0],
    );
    i = 1;
    while i < ROUNDS {
        t = _sodium_softaes_block_encrypt(t, softaes_block_xor(tweak_block, rkeys[i]));
        i += 1;
    }
    t = _sodium_softaes_block_encryptlast(t, softaes_block_xor(tweak_block, rkeys[ROUNDS]));
    softaes_block_store(out, t);
}

unsafe fn aes_decrypt_with_tweak(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    rkeys: &KeySchedule,
) {
    let mut rkeys_inv: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let tweak_block: AesBlock = tweak_expand(tweak);
    let tweak_block_inv: AesBlock = _sodium_softaes_inv_mix_columns(tweak_block);
    let mut t: AesBlock;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = rkeys[i];
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(
        softaes_block_xor(softaes_block_load(in_), tweak_block),
        rkeys_inv[ROUNDS],
    );
    i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, softaes_block_xor(tweak_block_inv, rkeys_inv[i]));
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, softaes_block_xor(tweak_block, rkeys_inv[0]));
    softaes_block_store(out, t);
    sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn aes_xex_tweak(tweak: *const u8, tkeys: &KeySchedule) -> AesBlock {
    let mut tt: AesBlock;
    let mut i: usize;

    tt = softaes_block_xor(softaes_block_load(tweak), tkeys[0]);
    i = 1;
    while i < ROUNDS {
        tt = _sodium_softaes_block_encrypt(tt, tkeys[i]);
        i += 1;
    }
    tt = _sodium_softaes_block_encryptlast(tt, tkeys[ROUNDS]);
    tt
}

unsafe fn aes_xex_encrypt(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    tkeys: &KeySchedule,
    rkeys: &KeySchedule,
) {
    let tt: AesBlock = aes_xex_tweak(tweak, tkeys);
    let mut t: AesBlock;
    let mut i: usize;

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tt), rkeys[0]);
    i = 1;
    while i < ROUNDS {
        t = _sodium_softaes_block_encrypt(t, rkeys[i]);
        i += 1;
    }
    t = _sodium_softaes_block_encryptlast(t, softaes_block_xor(rkeys[ROUNDS], tt));
    softaes_block_store(out, t);
}

unsafe fn aes_xex_decrypt(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    tkeys: &KeySchedule,
    rkeys: &KeySchedule,
) {
    let mut rkeys_inv: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let tt: AesBlock = aes_xex_tweak(tweak, tkeys);
    let mut t: AesBlock;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = rkeys[i];
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(
        softaes_block_xor(softaes_block_load(in_), tt),
        rkeys_inv[ROUNDS],
    );
    i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, rkeys_inv[i]);
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, softaes_block_xor(rkeys_inv[0], tt));
    softaes_block_store(out, t);
    sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    aes_encrypt(out, in_, &rkeys);
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    aes_decrypt(out, in_, &rkeys);
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn nd_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    memcpy(out, t, 8);
    aes_encrypt_with_tweak(out.add(8), in_, t, &rkeys);
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    aes_decrypt_with_tweak(out, in_.add(8), in_, &rkeys);
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn ndx_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut i: usize;
    let mut d: u8;

    expand_key(&mut tkeys, k.add(16));
    expand_key(&mut rkeys, k);

    softaes_block_store(
        diff.as_mut_ptr(),
        softaes_block_xor(tkeys[ROUNDS / 2], rkeys[ROUNDS / 2]),
    );
    d = 0;
    i = 0;
    while i < 16 {
        d |= diff[i];
        i += 1;
    }
    if d == 0 {
        i = 0;
        while i < 16 {
            diff[i] = *k.add(i) ^ 0x5a;
            i += 1;
        }
        expand_key(&mut rkeys, diff.as_ptr());
    }

    memcpy(out, t, 16);
    aes_xex_encrypt(out.add(16), in_, t, &tkeys, &rkeys);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16);
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        tkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut i: usize;
    let mut d: u8;

    expand_key(&mut tkeys, k.add(16));
    expand_key(&mut rkeys, k);

    softaes_block_store(
        diff.as_mut_ptr(),
        softaes_block_xor(tkeys[ROUNDS / 2], rkeys[ROUNDS / 2]),
    );
    d = 0;
    i = 0;
    while i < 16 {
        d |= diff[i];
        i += 1;
    }
    if d == 0 {
        i = 0;
        while i < 16 {
            diff[i] = *k.add(i) ^ 0x5a;
            i += 1;
        }
        expand_key(&mut rkeys, diff.as_ptr());
    }

    aes_xex_decrypt(out, in_.add(16), in_, &tkeys, &rkeys);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16);
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        tkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn is_ipv4_mapped(ip16: *const u8) -> c_int {
    static IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

    let mut i: usize = 0;
    while i < 12 {
        if *ip16.add(i) != IPV4_MAPPED_PREFIX[i] {
            return 0;
        }
        i += 1;
    }
    1
}

unsafe fn pfx_get_bit(ip16: *const u8, bit_index: c_uint) -> u8 {
    (*ip16.add(15 - (bit_index / 8) as usize) >> (bit_index % 8)) & 1
}

unsafe fn pfx_set_bit(ip16: *mut u8, bit_index: c_uint, bit_value: u8) {
    let byte_index: usize = 15 - (bit_index / 8) as usize;
    let bit_mask: u8 = (1u32 << (bit_index % 8)) as u8;
    #[allow(unused_mut)]
    let mut mask: u8 = 0u8.wrapping_sub(bit_value & 1);

    /* `__asm__ __volatile__("" : "+r"(mask) :);` -- optimisation barrier. */
    core::arch::asm!(
        "/* {0} */",
        inout(reg_byte) mask,
        options(nostack, preserves_flags, nomem)
    );

    *ip16.add(byte_index) = (*ip16.add(byte_index) & !bit_mask) | (bit_mask & mask);
}

unsafe fn pfx_shift_left(ip16: *mut u8) {
    let mut i: usize = 0;

    while i < 15 {
        *ip16.add(i) = (*ip16.add(i) << 1) | (*ip16.add(i + 1) >> 7);
        i += 1;
    }
    *ip16.add(15) <<= 1;
}

unsafe fn pfx_pad_prefix(padded_prefix: *mut u8, prefix_len_bits: c_uint) {
    memset(padded_prefix, 0, 16);
    if prefix_len_bits == 0 {
        *padded_prefix.add(15) = 0x01;
    } else {
        *padded_prefix.add(3) = 0x01;
        *padded_prefix.add(14) = 0xff;
        *padded_prefix.add(15) = 0xff;
    }
}

unsafe extern "C" fn pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut encrypted = [0u8; 16];
    let mut padded_prefix = [0u8; 16];
    let mut t = [0u8; 16];
    let mut e1: AesBlock;
    let mut e2: AesBlock;
    let mut e: AesBlock;
    let mut prefix_start: c_uint = 0;
    let mut prefix_len_bits: c_uint;
    let mut bit_pos: c_uint;
    let mut cipher_bit: u8;
    let mut original_bit: u8;
    let mut i: usize;
    let mut d: u8;

    expand_key(&mut k1keys, k);
    expand_key(&mut k2keys, k.add(16));

    softaes_block_store(
        diff.as_mut_ptr(),
        softaes_block_xor(k1keys[ROUNDS / 2], k2keys[ROUNDS / 2]),
    );
    d = 0;
    i = 0;
    while i < 16 {
        d |= diff[i];
        i += 1;
    }
    if d == 0 {
        i = 0;
        while i < 16 {
            diff[i] = *k.add(i) ^ 0x5a;
            i += 1;
        }
        expand_key(&mut k2keys, diff.as_ptr());
    }

    if is_ipv4_mapped(in_) != 0 {
        prefix_start = 96;
    }

    pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start);

    memset(encrypted.as_mut_ptr(), 0, 16);
    if prefix_start == 96 {
        encrypted[10] = 0xff;
        encrypted[11] = 0xff;
    }

    prefix_len_bits = prefix_start;
    while prefix_len_bits < 128 {
        e1 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k1keys[0]);
        e2 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k2keys[0]);
        i = 1;
        while i < ROUNDS {
            e1 = _sodium_softaes_block_encrypt(e1, k1keys[i]);
            e2 = _sodium_softaes_block_encrypt(e2, k2keys[i]);
            i += 1;
        }
        e1 = _sodium_softaes_block_encryptlast(e1, k1keys[ROUNDS]);
        e2 = _sodium_softaes_block_encryptlast(e2, k2keys[ROUNDS]);

        e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        cipher_bit = t[15] & 1;
        bit_pos = 127 - prefix_len_bits;
        original_bit = pfx_get_bit(in_, bit_pos);
        pfx_set_bit(encrypted.as_mut_ptr(), bit_pos, original_bit ^ cipher_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    memcpy(out, encrypted.as_ptr(), 16);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16);
    sodium_memzero(
        k2keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        k1keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe extern "C" fn pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut decrypted = [0u8; 16];
    let mut padded_prefix = [0u8; 16];
    let mut t = [0u8; 16];
    let mut e1: AesBlock;
    let mut e2: AesBlock;
    let mut prefix_start: c_uint = 0;
    let mut prefix_len_bits: c_uint;
    let mut bit_pos: c_uint;
    let mut cipher_bit: u8;
    let mut encrypted_bit: u8;
    let mut original_bit: u8;
    let mut i: usize;
    let mut d: u8;

    expand_key(&mut k1keys, k);
    expand_key(&mut k2keys, k.add(16));

    softaes_block_store(
        diff.as_mut_ptr(),
        softaes_block_xor(k1keys[ROUNDS / 2], k2keys[ROUNDS / 2]),
    );
    d = 0;
    i = 0;
    while i < 16 {
        d |= diff[i];
        i += 1;
    }
    if d == 0 {
        i = 0;
        while i < 16 {
            diff[i] = *k.add(i) ^ 0x5a;
            i += 1;
        }
        expand_key(&mut k2keys, diff.as_ptr());
    }

    if is_ipv4_mapped(in_) != 0 {
        prefix_start = 96;
    }

    pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start);

    memset(decrypted.as_mut_ptr(), 0, 16);
    if prefix_start == 96 {
        decrypted[10] = 0xff;
        decrypted[11] = 0xff;
    }

    prefix_len_bits = prefix_start;
    while prefix_len_bits < 128 {
        e1 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k1keys[0]);
        e2 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k2keys[0]);
        i = 1;
        while i < ROUNDS {
            e1 = _sodium_softaes_block_encrypt(e1, k1keys[i]);
            e2 = _sodium_softaes_block_encrypt(e2, k2keys[i]);
            i += 1;
        }
        e1 = _sodium_softaes_block_encryptlast(e1, k1keys[ROUNDS]);
        e2 = _sodium_softaes_block_encryptlast(e2, k2keys[ROUNDS]);

        let e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        cipher_bit = t[15] & 1;
        bit_pos = 127 - prefix_len_bits;
        encrypted_bit = pfx_get_bit(in_, bit_pos);
        original_bit = encrypted_bit ^ cipher_bit;
        pfx_set_bit(decrypted.as_mut_ptr(), bit_pos, original_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    memcpy(out, decrypted.as_ptr(), 16);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16);
    sodium_memzero(
        k2keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        k1keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

#[unsafe(no_mangle)]
pub static ipcrypt_soft_implementation: ipcrypt_implementation = ipcrypt_implementation {
    encrypt: encrypt,
    decrypt: decrypt,
    nd_encrypt: nd_encrypt,
    nd_decrypt: nd_decrypt,
    ndx_encrypt: ndx_encrypt,
    ndx_decrypt: ndx_decrypt,
    pfx_encrypt: pfx_encrypt,
    pfx_decrypt: pfx_decrypt,
};
