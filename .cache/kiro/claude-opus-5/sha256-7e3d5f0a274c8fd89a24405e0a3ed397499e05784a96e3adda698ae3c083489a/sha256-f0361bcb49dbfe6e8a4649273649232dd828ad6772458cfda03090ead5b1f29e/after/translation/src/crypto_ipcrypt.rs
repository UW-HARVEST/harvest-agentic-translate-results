//! Translation of `crypto_ipcrypt/crypto_ipcrypt.c` and
//! `crypto_ipcrypt/ipcrypt_soft.c`.
//!
//! Build configuration facts (see CONVENTIONS.md): no `HAVE_*` macros are
//! defined, so the aesni / armcrypto variants are compiled out and
//! `_crypto_ipcrypt_pick_best_implementation()` always selects the soft
//! implementation. `sodium_runtime_has_aesni()` / `has_armcrypto()` return 0.
//!
//! The soft AES primitives live in `crate::crypto_core::softaes`. The
//! `softaes_block_{load,load64x2,store,xor,and}` helpers are `static inline`
//! in `private/softaes.h`, so they are reproduced here as local helpers.

use crate::crypto_core::softaes::SoftAesBlock;
use crate::crypto_core::softaes::{
    _sodium_softaes_block_decrypt, _sodium_softaes_block_decryptlast,
    _sodium_softaes_block_encrypt, _sodium_softaes_block_encryptlast, _sodium_softaes_expand_key128,
    _sodium_softaes_inv_mix_columns, _sodium_softaes_invert_key_schedule128,
};

// ---- crypto_ipcrypt.h constants ----
const CRYPTO_IPCRYPT_BYTES: usize = 16;
const CRYPTO_IPCRYPT_KEYBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_KEYBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_TWEAKBYTES: usize = 8;
const CRYPTO_IPCRYPT_ND_INPUTBYTES: usize = 16;
const CRYPTO_IPCRYPT_ND_OUTPUTBYTES: usize = 24;
const CRYPTO_IPCRYPT_NDX_KEYBYTES: usize = 32;
const CRYPTO_IPCRYPT_NDX_TWEAKBYTES: usize = 16;
const CRYPTO_IPCRYPT_NDX_INPUTBYTES: usize = 16;
const CRYPTO_IPCRYPT_NDX_OUTPUTBYTES: usize = 32;
const CRYPTO_IPCRYPT_PFX_KEYBYTES: usize = 32;
const CRYPTO_IPCRYPT_PFX_BYTES: usize = 16;

// ===========================================================================
// implementations.h: struct of function pointers
// ===========================================================================
#[repr(C)]
pub struct IpcryptImplementation {
    pub encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub nd_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub nd_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub ndx_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8),
    pub ndx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_encrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
    pub pfx_decrypt: unsafe extern "C" fn(out: *mut u8, in_: *const u8, k: *const u8),
}

unsafe impl Sync for IpcryptImplementation {}

// ===========================================================================
// ipcrypt_soft.c
// ===========================================================================

const ROUNDS: usize = 10;

// `typedef aes_block_t KeySchedule[1 + ROUNDS];`
type AesBlock = SoftAesBlock;
type KeySchedule = [AesBlock; 1 + ROUNDS];

// --- static inline helpers from private/softaes.h ---

#[inline(always)]
unsafe fn softaes_block_load(in_: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: crate::common::load32_le(in_.add(0)),
        w1: crate::common::load32_le(in_.add(4)),
        w2: crate::common::load32_le(in_.add(8)),
        w3: crate::common::load32_le(in_.add(12)),
    }
}

#[inline(always)]
unsafe fn softaes_block_store(out: *mut u8, in_: SoftAesBlock) {
    crate::common::store32_le(out.add(0), in_.w0);
    crate::common::store32_le(out.add(4), in_.w1);
    crate::common::store32_le(out.add(8), in_.w2);
    crate::common::store32_le(out.add(12), in_.w3);
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

// --- key schedule primitives ---

fn expand_key(rkeys: &mut KeySchedule, key: *const u8) {
    unsafe {
        _sodium_softaes_expand_key128(rkeys.as_mut_ptr(), key);
    }
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
    let mut rkeys_inv: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
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
    crate::sodium_utils::sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn tweak_expand(tweak: *const u8) -> AesBlock {
    let mut out = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };

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
    let tweak_block = tweak_expand(tweak);
    let mut t: AesBlock;
    let mut i: usize;

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tweak_block), rkeys[0]);
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
    let mut rkeys_inv: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let tweak_block = tweak_expand(tweak);
    let tweak_block_inv = _sodium_softaes_inv_mix_columns(tweak_block);
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
    crate::sodium_utils::sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut core::ffi::c_void,
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
    let tt = aes_xex_tweak(tweak, tkeys);
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
    let mut rkeys_inv: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let tt = aes_xex_tweak(tweak, tkeys);
    let mut t: AesBlock;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = rkeys[i];
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tt), rkeys_inv[ROUNDS]);
    i = ROUNDS - 1;
    while i > 0 {
        t = _sodium_softaes_block_decrypt(t, rkeys_inv[i]);
        i -= 1;
    }
    t = _sodium_softaes_block_decryptlast(t, softaes_block_xor(rkeys_inv[0], tt));
    softaes_block_store(out, t);
    crate::sodium_utils::sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

// --- implementation function pointers (C `static` functions) ---

pub unsafe extern "C" fn encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    aes_encrypt(out, in_, &rkeys);
    crate::sodium_utils::sodium_memzero(
        rkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

pub unsafe extern "C" fn decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    aes_decrypt(out, in_, &rkeys);
    crate::sodium_utils::sodium_memzero(
        rkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

pub unsafe extern "C" fn nd_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    crate::common::memcpy(out, t, 8);
    aes_encrypt_with_tweak(out.add(8), in_, t, &rkeys);
    crate::sodium_utils::sodium_memzero(
        rkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

pub unsafe extern "C" fn nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    expand_key(&mut rkeys, k);
    aes_decrypt_with_tweak(out, in_.add(8), in_, &rkeys);
    crate::sodium_utils::sodium_memzero(
        rkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

pub unsafe extern "C" fn ndx_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
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

    crate::common::memcpy(out, t, 16);
    aes_xex_encrypt(out.add(16), in_, t, &tkeys, &rkeys);
    crate::sodium_utils::sodium_memzero(diff.as_mut_ptr() as *mut core::ffi::c_void, 16);
    crate::sodium_utils::sodium_memzero(
        rkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    crate::sodium_utils::sodium_memzero(
        tkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

pub unsafe extern "C" fn ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
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
    crate::sodium_utils::sodium_memzero(diff.as_mut_ptr() as *mut core::ffi::c_void, 16);
    crate::sodium_utils::sodium_memzero(
        rkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    crate::sodium_utils::sodium_memzero(
        tkeys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

// --- pfx helpers ---

unsafe fn is_ipv4_mapped(ip16: *const u8) -> i32 {
    static IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

    (crate::common::memcmp(ip16, IPV4_MAPPED_PREFIX.as_ptr(), 12) == 0) as i32
}

unsafe fn pfx_get_bit(ip16: *const u8, bit_index: u32) -> u8 {
    (*ip16.add(15 - (bit_index / 8) as usize) >> (bit_index % 8)) & 1
}

unsafe fn pfx_set_bit(ip16: *mut u8, bit_index: u32, bit_value: u8) {
    let byte_index: usize = 15 - (bit_index / 8) as usize;
    let bit_mask: u8 = (1u32 << (bit_index % 8)) as u8;
    let mask: u8 = (0u32.wrapping_sub((bit_value & 1) as u32)) as u8;

    // The C source inserts an `__asm__ __volatile__("" : "+r"(mask) :)` barrier
    // to prevent the compiler from turning the constant-time select into a
    // branch. It has no functional effect on the computed value.
    *ip16.add(byte_index) = (*ip16.add(byte_index) & !bit_mask) | (bit_mask & mask);
}

unsafe fn pfx_shift_left(ip16: *mut u8) {
    let mut i: usize;

    i = 0;
    while i < 15 {
        *ip16.add(i) = (*ip16.add(i) << 1) | (*ip16.add(i + 1) >> 7);
        i += 1;
    }
    *ip16.add(15) <<= 1;
}

unsafe fn pfx_pad_prefix(padded_prefix: *mut u8, prefix_len_bits: u32) {
    crate::common::memset(padded_prefix, 0, 16);
    if prefix_len_bits == 0 {
        *padded_prefix.add(15) = 0x01;
    } else {
        *padded_prefix.add(3) = 0x01;
        *padded_prefix.add(14) = 0xff;
        *padded_prefix.add(15) = 0xff;
    }
}

pub unsafe extern "C" fn pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
    let mut encrypted: [u8; 16] = [0; 16];
    let mut padded_prefix: [u8; 16] = [0; 16];
    let mut t: [u8; 16] = [0; 16];
    let mut e1: AesBlock;
    let mut e2: AesBlock;
    let mut prefix_start: u32 = 0;
    let mut prefix_len_bits: u32;
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

    crate::common::memset(encrypted.as_mut_ptr(), 0, 16);
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

        let e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        let cipher_bit: u8 = t[15] & 1;
        let bit_pos: u32 = 127u32.wrapping_sub(prefix_len_bits);
        let original_bit: u8 = pfx_get_bit(in_, bit_pos);
        pfx_set_bit(encrypted.as_mut_ptr(), bit_pos, original_bit ^ cipher_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    crate::common::memcpy(out, encrypted.as_ptr(), 16);
    crate::sodium_utils::sodium_memzero(diff.as_mut_ptr() as *mut core::ffi::c_void, 16);
    crate::sodium_utils::sodium_memzero(
        k2keys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    crate::sodium_utils::sodium_memzero(
        k1keys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

pub unsafe extern "C" fn pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
    let mut decrypted: [u8; 16] = [0; 16];
    let mut padded_prefix: [u8; 16] = [0; 16];
    let mut t: [u8; 16] = [0; 16];
    let mut e1: AesBlock;
    let mut e2: AesBlock;
    let mut prefix_start: u32 = 0;
    let mut prefix_len_bits: u32;
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

    crate::common::memset(decrypted.as_mut_ptr(), 0, 16);
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

        let cipher_bit = t[15] & 1;
        let bit_pos = 127u32.wrapping_sub(prefix_len_bits);
        let encrypted_bit = pfx_get_bit(in_, bit_pos);
        let original_bit = encrypted_bit ^ cipher_bit;
        pfx_set_bit(decrypted.as_mut_ptr(), bit_pos, original_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    crate::common::memcpy(out, decrypted.as_ptr(), 16);
    crate::sodium_utils::sodium_memzero(diff.as_mut_ptr() as *mut core::ffi::c_void, 16);
    crate::sodium_utils::sodium_memzero(
        k2keys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    crate::sodium_utils::sodium_memzero(
        k1keys.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

// --- exported data symbol: ipcrypt_soft_implementation ---
#[unsafe(no_mangle)]
pub static ipcrypt_soft_implementation: IpcryptImplementation = IpcryptImplementation {
    encrypt,
    decrypt,
    nd_encrypt,
    nd_decrypt,
    ndx_encrypt,
    ndx_decrypt,
    pfx_encrypt,
    pfx_decrypt,
};

// ===========================================================================
// crypto_ipcrypt.c
// ===========================================================================

static mut IMPLEMENTATION: *const IpcryptImplementation = &ipcrypt_soft_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_bytes() -> usize {
    CRYPTO_IPCRYPT_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_keybytes() -> usize {
    CRYPTO_IPCRYPT_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_keybytes() -> usize {
    CRYPTO_IPCRYPT_ND_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_tweakbytes() -> usize {
    CRYPTO_IPCRYPT_ND_TWEAKBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_inputbytes() -> usize {
    CRYPTO_IPCRYPT_ND_INPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_nd_outputbytes() -> usize {
    CRYPTO_IPCRYPT_ND_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_keybytes() -> usize {
    CRYPTO_IPCRYPT_NDX_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_tweakbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_TWEAKBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_inputbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_INPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_ndx_outputbytes() -> usize {
    CRYPTO_IPCRYPT_NDX_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_pfx_keybytes() -> usize {
    CRYPTO_IPCRYPT_PFX_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_ipcrypt_pfx_bytes() -> usize {
    CRYPTO_IPCRYPT_PFX_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_keygen(k: *mut u8) {
    crate::randombytes::randombytes_buf(k as *mut core::ffi::c_void, CRYPTO_IPCRYPT_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keygen(k: *mut u8) {
    crate::randombytes::randombytes_buf(k as *mut core::ffi::c_void, CRYPTO_IPCRYPT_ND_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keygen(k: *mut u8) {
    crate::randombytes::randombytes_buf(k as *mut core::ffi::c_void, CRYPTO_IPCRYPT_NDX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keygen(k: *mut u8) {
    crate::randombytes::randombytes_buf(k as *mut core::ffi::c_void, CRYPTO_IPCRYPT_PFX_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).encrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    ((*IMPLEMENTATION).nd_encrypt)(out, in_, t, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).nd_decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_encrypt(
    out: *mut u8,
    in_: *const u8,
    t: *const u8,
    k: *const u8,
) {
    ((*IMPLEMENTATION).ndx_encrypt)(out, in_, t, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).ndx_decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).pfx_encrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).pfx_decrypt)(out, in_, k);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_ipcrypt_pick_best_implementation() -> i32 {
    IMPLEMENTATION = &ipcrypt_soft_implementation;
    0
}
