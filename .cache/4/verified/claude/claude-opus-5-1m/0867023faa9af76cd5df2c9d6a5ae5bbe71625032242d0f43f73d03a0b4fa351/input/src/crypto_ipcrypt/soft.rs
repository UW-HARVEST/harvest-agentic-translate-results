//! Translation of `crypto_ipcrypt/ipcrypt_soft.c`.
//!
//! This is the only ipcrypt backend that is compiled in the reference build:
//! neither `HAVE_ARMCRYPTO` nor the `HAVE_AVXINTRIN_H`/`HAVE_WMMINTRIN_H` pair
//! is defined, so `ipcrypt_armcrypto.c` and `ipcrypt_aesni.c` contribute no
//! symbols at all.
//!
//! The `AES_*` macros of the C file expand to the `softaes_*` helpers of
//! `include/sodium/private/softaes.h` and to the four round primitives of
//! `crypto_core/softaes/softaes.c`; they are used here through
//! `crate::crypto_core::softaes`.
//!
//! `ipcrypt_soft_implementation` is *not* renamed by
//! `include/sodium/private/quirks.h`, so it keeps its plain C name.

use core::ffi::{c_int, c_void};

use crate::common::{memcmp, memcpy, memset};
use crate::crypto_core::softaes::{
    SoftAesBlock, _sodium_softaes_expand_key128, _sodium_softaes_invert_key_schedule128,
    softaes_block_decrypt, softaes_block_decryptlast, softaes_block_encrypt,
    softaes_block_encryptlast, softaes_block_load, softaes_block_store, softaes_block_xor,
    softaes_inv_mix_columns,
};
use crate::sodium::utils::sodium_memzero;

use super::ipcrypt_implementation;

/// `#define ROUNDS 10`
const ROUNDS: usize = 10;

/// `typedef SoftAesBlock aes_block_t;`
type aes_block_t = SoftAesBlock;

/// `typedef aes_block_t KeySchedule[1 + ROUNDS];`
type KeySchedule = [aes_block_t; 1 + ROUNDS];

/// `static void expand_key(KeySchedule rkeys, const uint8_t key[16])`
#[inline]
unsafe fn expand_key(rkeys: &mut KeySchedule, key: *const u8) {
    unsafe { _sodium_softaes_expand_key128(rkeys.as_mut_ptr(), key) };
}

/// `static void aes_encrypt(uint8_t out[16], const uint8_t in[16], const KeySchedule rkeys)`
unsafe fn aes_encrypt(out: *mut u8, in_: *const u8, rkeys: &KeySchedule) {
    let mut t: aes_block_t;

    t = softaes_block_xor(unsafe { softaes_block_load(in_) }, rkeys[0]);
    for i in 1..ROUNDS {
        t = softaes_block_encrypt(t, rkeys[i]);
    }
    t = softaes_block_encryptlast(t, rkeys[ROUNDS]);
    unsafe { softaes_block_store(out, t) };
}

/// `static void aes_decrypt(uint8_t out[16], const uint8_t in[16], const KeySchedule rkeys)`
unsafe fn aes_decrypt(out: *mut u8, in_: *const u8, rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut t: aes_block_t;

    for i in 0..=ROUNDS {
        rkeys_inv[i] = rkeys[i];
    }
    unsafe { _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr()) };

    t = softaes_block_xor(unsafe { softaes_block_load(in_) }, rkeys_inv[ROUNDS]);
    for i in (1..ROUNDS).rev() {
        t = softaes_block_decrypt(t, rkeys_inv[i]);
    }
    t = softaes_block_decryptlast(t, rkeys_inv[0]);
    unsafe { softaes_block_store(out, t) };
    unsafe {
        sodium_memzero(
            rkeys_inv.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static aes_block_t tweak_expand(const uint8_t tweak[8])`
unsafe fn tweak_expand(tweak: *const u8) -> aes_block_t {
    let mut out = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };

    let tw = |i: usize| -> u32 { (unsafe { *tweak.add(i) }) as u32 };

    out.w0 = tw(0) | (tw(1) << 8);
    out.w1 = tw(2) | (tw(3) << 8);
    out.w2 = tw(4) | (tw(5) << 8);
    out.w3 = tw(6) | (tw(7) << 8);

    out
}

/// ```c
/// static void aes_encrypt_with_tweak(uint8_t out[16], const uint8_t in[16], const uint8_t tweak[8],
///                                    const KeySchedule rkeys)
/// ```
unsafe fn aes_encrypt_with_tweak(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    rkeys: &KeySchedule,
) {
    let tweak_block: aes_block_t = unsafe { tweak_expand(tweak) };
    let mut t: aes_block_t;

    t = softaes_block_xor(
        softaes_block_xor(unsafe { softaes_block_load(in_) }, tweak_block),
        rkeys[0],
    );
    for i in 1..ROUNDS {
        t = softaes_block_encrypt(t, softaes_block_xor(tweak_block, rkeys[i]));
    }
    t = softaes_block_encryptlast(t, softaes_block_xor(tweak_block, rkeys[ROUNDS]));
    unsafe { softaes_block_store(out, t) };
}

/// ```c
/// static void aes_decrypt_with_tweak(uint8_t out[16], const uint8_t in[16], const uint8_t tweak[8],
///                                    const KeySchedule rkeys)
/// ```
unsafe fn aes_decrypt_with_tweak(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    rkeys: &KeySchedule,
) {
    let mut rkeys_inv: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let tweak_block: aes_block_t = unsafe { tweak_expand(tweak) };
    let tweak_block_inv: aes_block_t = softaes_inv_mix_columns(tweak_block);
    let mut t: aes_block_t;

    for i in 0..=ROUNDS {
        rkeys_inv[i] = rkeys[i];
    }
    unsafe { _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr()) };

    t = softaes_block_xor(
        softaes_block_xor(unsafe { softaes_block_load(in_) }, tweak_block),
        rkeys_inv[ROUNDS],
    );
    for i in (1..ROUNDS).rev() {
        t = softaes_block_decrypt(t, softaes_block_xor(tweak_block_inv, rkeys_inv[i]));
    }
    t = softaes_block_decryptlast(t, softaes_block_xor(tweak_block, rkeys_inv[0]));
    unsafe { softaes_block_store(out, t) };
    unsafe {
        sodium_memzero(
            rkeys_inv.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static aes_block_t aes_xex_tweak(const uint8_t tweak[16], const KeySchedule tkeys)`
unsafe fn aes_xex_tweak(tweak: *const u8, tkeys: &KeySchedule) -> aes_block_t {
    let mut tt: aes_block_t;

    tt = softaes_block_xor(unsafe { softaes_block_load(tweak) }, tkeys[0]);
    for i in 1..ROUNDS {
        tt = softaes_block_encrypt(tt, tkeys[i]);
    }
    tt = softaes_block_encryptlast(tt, tkeys[ROUNDS]);
    tt
}

/// ```c
/// static void aes_xex_encrypt(uint8_t out[16], const uint8_t in[16], const uint8_t tweak[16],
///                             const KeySchedule tkeys, const KeySchedule rkeys)
/// ```
unsafe fn aes_xex_encrypt(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    tkeys: &KeySchedule,
    rkeys: &KeySchedule,
) {
    let tt: aes_block_t = unsafe { aes_xex_tweak(tweak, tkeys) };
    let mut t: aes_block_t;

    t = softaes_block_xor(
        softaes_block_xor(unsafe { softaes_block_load(in_) }, tt),
        rkeys[0],
    );
    for i in 1..ROUNDS {
        t = softaes_block_encrypt(t, rkeys[i]);
    }
    t = softaes_block_encryptlast(t, softaes_block_xor(rkeys[ROUNDS], tt));
    unsafe { softaes_block_store(out, t) };
}

/// ```c
/// static void aes_xex_decrypt(uint8_t out[16], const uint8_t in[16], const uint8_t tweak[16],
///                             const KeySchedule tkeys, const KeySchedule rkeys)
/// ```
unsafe fn aes_xex_decrypt(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    tkeys: &KeySchedule,
    rkeys: &KeySchedule,
) {
    let mut rkeys_inv: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let tt: aes_block_t = unsafe { aes_xex_tweak(tweak, tkeys) };
    let mut t: aes_block_t;

    for i in 0..=ROUNDS {
        rkeys_inv[i] = rkeys[i];
    }
    unsafe { _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr()) };

    t = softaes_block_xor(
        softaes_block_xor(unsafe { softaes_block_load(in_) }, tt),
        rkeys_inv[ROUNDS],
    );
    for i in (1..ROUNDS).rev() {
        t = softaes_block_decrypt(t, rkeys_inv[i]);
    }
    t = softaes_block_decryptlast(t, softaes_block_xor(rkeys_inv[0], tt));
    unsafe { softaes_block_store(out, t) };
    unsafe {
        sodium_memzero(
            rkeys_inv.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void encrypt(uint8_t *out, const uint8_t *in, const uint8_t *k)`
unsafe extern "C" fn encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    unsafe { expand_key(&mut rkeys, k) };
    unsafe { aes_encrypt(out, in_, &rkeys) };
    unsafe {
        sodium_memzero(
            rkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void decrypt(uint8_t *out, const uint8_t *in, const uint8_t *k)`
unsafe extern "C" fn decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    unsafe { expand_key(&mut rkeys, k) };
    unsafe { aes_decrypt(out, in_, &rkeys) };
    unsafe {
        sodium_memzero(
            rkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void nd_encrypt(uint8_t *out, const uint8_t *in, const uint8_t *t, const uint8_t *k)`
unsafe extern "C" fn nd_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    unsafe { expand_key(&mut rkeys, k) };
    unsafe { memcpy(out, t, 8) };
    unsafe { aes_encrypt_with_tweak(out.add(8), in_, t, &rkeys) };
    unsafe {
        sodium_memzero(
            rkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void nd_decrypt(uint8_t *out, const uint8_t *in, const uint8_t *k)`
unsafe extern "C" fn nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];

    unsafe { expand_key(&mut rkeys, k) };
    unsafe { aes_decrypt_with_tweak(out, in_.add(8), in_, &rkeys) };
    unsafe {
        sodium_memzero(
            rkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void ndx_encrypt(uint8_t *out, const uint8_t *in, const uint8_t *t, const uint8_t *k)`
unsafe extern "C" fn ndx_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut d: u8;

    unsafe { expand_key(&mut tkeys, k.add(16)) };
    unsafe { expand_key(&mut rkeys, k) };

    unsafe {
        softaes_block_store(
            diff.as_mut_ptr(),
            softaes_block_xor(tkeys[ROUNDS / 2], rkeys[ROUNDS / 2]),
        )
    };
    d = 0;
    for i in 0..16usize {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16usize {
            diff[i] = unsafe { *k.add(i) } ^ 0x5a;
        }
        unsafe { expand_key(&mut rkeys, diff.as_ptr()) };
    }

    unsafe { memcpy(out, t, 16) };
    unsafe { aes_xex_encrypt(out.add(16), in_, t, &tkeys, &rkeys) };
    unsafe { sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16) };
    unsafe {
        sodium_memzero(
            rkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
    unsafe {
        sodium_memzero(
            tkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void ndx_decrypt(uint8_t *out, const uint8_t *in, const uint8_t *k)`
unsafe extern "C" fn ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut d: u8;

    unsafe { expand_key(&mut tkeys, k.add(16)) };
    unsafe { expand_key(&mut rkeys, k) };

    unsafe {
        softaes_block_store(
            diff.as_mut_ptr(),
            softaes_block_xor(tkeys[ROUNDS / 2], rkeys[ROUNDS / 2]),
        )
    };
    d = 0;
    for i in 0..16usize {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16usize {
            diff[i] = unsafe { *k.add(i) } ^ 0x5a;
        }
        unsafe { expand_key(&mut rkeys, diff.as_ptr()) };
    }

    unsafe { aes_xex_decrypt(out, in_.add(16), in_, &tkeys, &rkeys) };
    unsafe { sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16) };
    unsafe {
        sodium_memzero(
            rkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
    unsafe {
        sodium_memzero(
            tkeys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static int is_ipv4_mapped(const uint8_t ip16[16])`
unsafe fn is_ipv4_mapped(ip16: *const u8) -> c_int {
    /// `static const uint8_t ipv4_mapped_prefix[12]`
    static ipv4_mapped_prefix: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

    (unsafe {
        memcmp(
            ip16 as *const c_void,
            ipv4_mapped_prefix.as_ptr() as *const c_void,
            12,
        )
    } == 0) as c_int
}

/// `static uint8_t pfx_get_bit(const uint8_t ip16[16], unsigned int bit_index)`
unsafe fn pfx_get_bit(ip16: *const u8, bit_index: u32) -> u8 {
    (unsafe { *ip16.add((15 - bit_index / 8) as usize) } >> (bit_index % 8)) & 1
}

/// ```c
/// static void pfx_set_bit(uint8_t ip16[16], const unsigned int bit_index, const uint8_t bit_value)
/// ```
unsafe fn pfx_set_bit(ip16: *mut u8, bit_index: u32, bit_value: u8) {
    let byte_index: usize = (15 - bit_index / 8) as usize;
    let bit_mask: u8 = (1u32 << (bit_index % 8)) as u8;
    let mut mask: u8 = 0u8.wrapping_sub(bit_value & 1);

    /* `__asm__ __volatile__("" : "+r"(mask) :);` -- an optimisation barrier
     * with no effect on the computed value. */
    mask = core::hint::black_box(mask);

    unsafe { *ip16.add(byte_index) = (*ip16.add(byte_index) & !bit_mask) | (bit_mask & mask) };
}

/// `static void pfx_shift_left(uint8_t ip16[16])`
unsafe fn pfx_shift_left(ip16: *mut u8) {
    for i in 0..15usize {
        unsafe { *ip16.add(i) = (*ip16.add(i) << 1) | (*ip16.add(i + 1) >> 7) };
    }
    unsafe { *ip16.add(15) <<= 1 };
}

/// `static void pfx_pad_prefix(uint8_t padded_prefix[16], unsigned int prefix_len_bits)`
unsafe fn pfx_pad_prefix(padded_prefix: *mut u8, prefix_len_bits: u32) {
    unsafe { memset(padded_prefix, 0, 16) };
    if prefix_len_bits == 0 {
        unsafe { *padded_prefix.add(15) = 0x01 };
    } else {
        unsafe { *padded_prefix.add(3) = 0x01 };
        unsafe { *padded_prefix.add(14) = 0xff };
        unsafe { *padded_prefix.add(15) = 0xff };
    }
}

/// `static void pfx_encrypt(uint8_t *out, const uint8_t *in, const uint8_t *k)`
unsafe extern "C" fn pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut encrypted = [0u8; 16];
    let mut padded_prefix = [0u8; 16];
    let mut t = [0u8; 16];
    let (mut e1, mut e2, mut e): (aes_block_t, aes_block_t, aes_block_t);
    let mut prefix_start: u32 = 0;
    let mut bit_pos: u32;
    let mut cipher_bit: u8;
    let mut original_bit: u8;
    let mut d: u8;

    unsafe { expand_key(&mut k1keys, k) };
    unsafe { expand_key(&mut k2keys, k.add(16)) };

    unsafe {
        softaes_block_store(
            diff.as_mut_ptr(),
            softaes_block_xor(k1keys[ROUNDS / 2], k2keys[ROUNDS / 2]),
        )
    };
    d = 0;
    for i in 0..16usize {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16usize {
            diff[i] = unsafe { *k.add(i) } ^ 0x5a;
        }
        unsafe { expand_key(&mut k2keys, diff.as_ptr()) };
    }

    if unsafe { is_ipv4_mapped(in_) } != 0 {
        prefix_start = 96;
    }

    unsafe { pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start) };

    unsafe { memset(encrypted.as_mut_ptr(), 0, 16) };
    if prefix_start == 96 {
        encrypted[10] = 0xff;
        encrypted[11] = 0xff;
    }

    let mut prefix_len_bits: u32 = prefix_start;
    while prefix_len_bits < 128 {
        e1 = softaes_block_xor(
            unsafe { softaes_block_load(padded_prefix.as_ptr()) },
            k1keys[0],
        );
        e2 = softaes_block_xor(
            unsafe { softaes_block_load(padded_prefix.as_ptr()) },
            k2keys[0],
        );
        for i in 1..ROUNDS {
            e1 = softaes_block_encrypt(e1, k1keys[i]);
            e2 = softaes_block_encrypt(e2, k2keys[i]);
        }
        e1 = softaes_block_encryptlast(e1, k1keys[ROUNDS]);
        e2 = softaes_block_encryptlast(e2, k2keys[ROUNDS]);

        e = softaes_block_xor(e1, e2);
        unsafe { softaes_block_store(t.as_mut_ptr(), e) };

        cipher_bit = t[15] & 1;
        bit_pos = 127 - prefix_len_bits;
        original_bit = unsafe { pfx_get_bit(in_, bit_pos) };
        unsafe { pfx_set_bit(encrypted.as_mut_ptr(), bit_pos, original_bit ^ cipher_bit) };

        unsafe { pfx_shift_left(padded_prefix.as_mut_ptr()) };
        unsafe { pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit) };

        prefix_len_bits = prefix_len_bits.wrapping_add(1);
    }

    unsafe { memcpy(out, encrypted.as_ptr(), 16) };
    unsafe { sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16) };
    unsafe {
        sodium_memzero(
            k2keys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
    unsafe {
        sodium_memzero(
            k1keys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `static void pfx_decrypt(uint8_t *out, const uint8_t *in, const uint8_t *k)`
unsafe extern "C" fn pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 1 + ROUNDS];
    let mut diff = [0u8; 16];
    let mut decrypted = [0u8; 16];
    let mut padded_prefix = [0u8; 16];
    let mut t = [0u8; 16];
    let (mut e1, mut e2, mut e): (aes_block_t, aes_block_t, aes_block_t);
    let mut prefix_start: u32 = 0;
    let mut bit_pos: u32;
    let mut cipher_bit: u8;
    let mut encrypted_bit: u8;
    let mut original_bit: u8;
    let mut d: u8;

    unsafe { expand_key(&mut k1keys, k) };
    unsafe { expand_key(&mut k2keys, k.add(16)) };

    unsafe {
        softaes_block_store(
            diff.as_mut_ptr(),
            softaes_block_xor(k1keys[ROUNDS / 2], k2keys[ROUNDS / 2]),
        )
    };
    d = 0;
    for i in 0..16usize {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16usize {
            diff[i] = unsafe { *k.add(i) } ^ 0x5a;
        }
        unsafe { expand_key(&mut k2keys, diff.as_ptr()) };
    }

    if unsafe { is_ipv4_mapped(in_) } != 0 {
        prefix_start = 96;
    }

    unsafe { pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start) };

    unsafe { memset(decrypted.as_mut_ptr(), 0, 16) };
    if prefix_start == 96 {
        decrypted[10] = 0xff;
        decrypted[11] = 0xff;
    }

    let mut prefix_len_bits: u32 = prefix_start;
    while prefix_len_bits < 128 {
        e1 = softaes_block_xor(
            unsafe { softaes_block_load(padded_prefix.as_ptr()) },
            k1keys[0],
        );
        e2 = softaes_block_xor(
            unsafe { softaes_block_load(padded_prefix.as_ptr()) },
            k2keys[0],
        );
        for i in 1..ROUNDS {
            e1 = softaes_block_encrypt(e1, k1keys[i]);
            e2 = softaes_block_encrypt(e2, k2keys[i]);
        }
        e1 = softaes_block_encryptlast(e1, k1keys[ROUNDS]);
        e2 = softaes_block_encryptlast(e2, k2keys[ROUNDS]);

        e = softaes_block_xor(e1, e2);
        unsafe { softaes_block_store(t.as_mut_ptr(), e) };

        cipher_bit = t[15] & 1;
        bit_pos = 127 - prefix_len_bits;
        encrypted_bit = unsafe { pfx_get_bit(in_, bit_pos) };
        original_bit = encrypted_bit ^ cipher_bit;
        unsafe { pfx_set_bit(decrypted.as_mut_ptr(), bit_pos, original_bit) };

        unsafe { pfx_shift_left(padded_prefix.as_mut_ptr()) };
        unsafe { pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit) };

        prefix_len_bits = prefix_len_bits.wrapping_add(1);
    }

    unsafe { memcpy(out, decrypted.as_ptr(), 16) };
    unsafe { sodium_memzero(diff.as_mut_ptr() as *mut c_void, 16) };
    unsafe {
        sodium_memzero(
            k2keys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
    unsafe {
        sodium_memzero(
            k1keys.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<KeySchedule>(),
        )
    };
}

/// `struct ipcrypt_implementation ipcrypt_soft_implementation = { ... };`
#[unsafe(no_mangle)]
pub static mut ipcrypt_soft_implementation: ipcrypt_implementation = ipcrypt_implementation {
    encrypt: encrypt,
    decrypt: decrypt,
    nd_encrypt: nd_encrypt,
    nd_decrypt: nd_decrypt,
    ndx_encrypt: ndx_encrypt,
    ndx_decrypt: ndx_decrypt,
    pfx_encrypt: pfx_encrypt,
    pfx_decrypt: pfx_decrypt,
};
