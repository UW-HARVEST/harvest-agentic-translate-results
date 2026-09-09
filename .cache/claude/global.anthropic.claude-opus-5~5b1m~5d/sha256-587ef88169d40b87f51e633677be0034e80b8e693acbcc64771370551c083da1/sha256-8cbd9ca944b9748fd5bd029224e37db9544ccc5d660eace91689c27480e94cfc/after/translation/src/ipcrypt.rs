//! Rust translation of `crypto_ipcrypt/crypto_ipcrypt.c` and
//! `crypto_ipcrypt/ipcrypt_soft.c`.
//!
//! In the reference build configuration (no `HAVE_*` macros defined),
//! `ipcrypt_aesni.c` and `ipcrypt_armcrypto.c` are entirely `#ifdef`-ed out,
//! so `_crypto_ipcrypt_pick_best_implementation()` always selects the soft
//! (`ipcrypt_soft_implementation`) backend.

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------
// softaes glue (include/sodium/private/softaes.h)
// ---------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SoftAesBlock {
    pub w0: u32,
    pub w1: u32,
    pub w2: u32,
    pub w3: u32,
}

const ZERO_BLOCK: SoftAesBlock = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };

extern "C" {
    #[link_name = "_sodium_softaes_expand_key128"]
    fn softaes_expand_key128(rkeys: *mut SoftAesBlock, key: *const u8);
    #[link_name = "_sodium_softaes_invert_key_schedule128"]
    fn softaes_invert_key_schedule128(rkeys: *mut SoftAesBlock);
    #[link_name = "_sodium_softaes_inv_mix_columns"]
    fn softaes_inv_mix_columns(block: SoftAesBlock) -> SoftAesBlock;
    #[link_name = "_sodium_softaes_block_encrypt"]
    fn softaes_block_encrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
    #[link_name = "_sodium_softaes_block_decrypt"]
    fn softaes_block_decrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
    #[link_name = "_sodium_softaes_block_encryptlast"]
    fn softaes_block_encryptlast(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
    #[link_name = "_sodium_softaes_block_decryptlast"]
    fn softaes_block_decryptlast(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, val: c_int, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// `softaes_block_{load,store,xor}` are `static inline` in
// `private/softaes.h`, so they are translated locally rather than declared
// `extern`.
#[inline(always)]
unsafe fn softaes_block_load(inp: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: crate::common::load32_le(inp),
        w1: crate::common::load32_le(inp.add(4)),
        w2: crate::common::load32_le(inp.add(8)),
        w3: crate::common::load32_le(inp.add(12)),
    }
}

#[inline(always)]
unsafe fn softaes_block_store(out: *mut u8, inb: SoftAesBlock) {
    crate::common::store32_le(out, inb.w0);
    crate::common::store32_le(out.add(4), inb.w1);
    crate::common::store32_le(out.add(8), inb.w2);
    crate::common::store32_le(out.add(12), inb.w3);
}

#[inline(always)]
fn softaes_block_xor(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock { w0: a.w0 ^ b.w0, w1: a.w1 ^ b.w1, w2: a.w2 ^ b.w2, w3: a.w3 ^ b.w3 }
}

// ---------------------------------------------------------------------
// ipcrypt_soft.c
// ---------------------------------------------------------------------

type KeySchedule = [SoftAesBlock; 11]; // 1 + 10

#[inline(always)]
unsafe fn expand_key(rkeys: &mut KeySchedule, key: *const u8) {
    softaes_expand_key128(rkeys.as_mut_ptr(), key);
}

unsafe fn aes_encrypt(out: *mut u8, inp: *const u8, rkeys: &KeySchedule) {
    let mut t = softaes_block_xor(softaes_block_load(inp), rkeys[0]);
    for i in 1..10 {
        t = softaes_block_encrypt(t, rkeys[i]);
    }
    t = softaes_block_encryptlast(t, rkeys[10]);
    softaes_block_store(out, t);
}

unsafe fn aes_decrypt(out: *mut u8, inp: *const u8, rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = *rkeys;

    softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    let mut t = softaes_block_xor(softaes_block_load(inp), rkeys_inv[10]);
    let mut i: isize = 9;
    while i > 0 {
        t = softaes_block_decrypt(t, rkeys_inv[i as usize]);
        i -= 1;
    }
    t = softaes_block_decryptlast(t, rkeys_inv[0]);
    softaes_block_store(out, t);
    sodium_memzero(rkeys_inv.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe fn tweak_expand(tweak: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: (*tweak.add(0) as u32) | ((*tweak.add(1) as u32) << 8),
        w1: (*tweak.add(2) as u32) | ((*tweak.add(3) as u32) << 8),
        w2: (*tweak.add(4) as u32) | ((*tweak.add(5) as u32) << 8),
        w3: (*tweak.add(6) as u32) | ((*tweak.add(7) as u32) << 8),
    }
}

unsafe fn aes_encrypt_with_tweak(out: *mut u8, inp: *const u8, tweak: *const u8, rkeys: &KeySchedule) {
    let tweak_block = tweak_expand(tweak);
    let mut t = softaes_block_xor(softaes_block_xor(softaes_block_load(inp), tweak_block), rkeys[0]);
    for i in 1..10 {
        t = softaes_block_encrypt(t, softaes_block_xor(tweak_block, rkeys[i]));
    }
    t = softaes_block_encryptlast(t, softaes_block_xor(tweak_block, rkeys[10]));
    softaes_block_store(out, t);
}

unsafe fn aes_decrypt_with_tweak(out: *mut u8, inp: *const u8, tweak: *const u8, rkeys: &KeySchedule) {
    let mut rkeys_inv: KeySchedule = *rkeys;
    let tweak_block = tweak_expand(tweak);
    let tweak_block_inv = softaes_inv_mix_columns(tweak_block);

    softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    let mut t = softaes_block_xor(softaes_block_xor(softaes_block_load(inp), tweak_block), rkeys_inv[10]);
    let mut i: isize = 9;
    while i > 0 {
        t = softaes_block_decrypt(t, softaes_block_xor(tweak_block_inv, rkeys_inv[i as usize]));
        i -= 1;
    }
    t = softaes_block_decryptlast(t, softaes_block_xor(tweak_block, rkeys_inv[0]));
    softaes_block_store(out, t);
    sodium_memzero(rkeys_inv.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe fn aes_xex_tweak(tweak: *const u8, tkeys: &KeySchedule) -> SoftAesBlock {
    let mut tt = softaes_block_xor(softaes_block_load(tweak), tkeys[0]);
    for i in 1..10 {
        tt = softaes_block_encrypt(tt, tkeys[i]);
    }
    tt = softaes_block_encryptlast(tt, tkeys[10]);
    tt
}

unsafe fn aes_xex_encrypt(
    out: *mut u8,
    inp: *const u8,
    tweak: *const u8,
    tkeys: &KeySchedule,
    rkeys: &KeySchedule,
) {
    let tt = aes_xex_tweak(tweak, tkeys);
    let mut t = softaes_block_xor(softaes_block_xor(softaes_block_load(inp), tt), rkeys[0]);
    for i in 1..10 {
        t = softaes_block_encrypt(t, rkeys[i]);
    }
    t = softaes_block_encryptlast(t, softaes_block_xor(rkeys[10], tt));
    softaes_block_store(out, t);
}

unsafe fn aes_xex_decrypt(
    out: *mut u8,
    inp: *const u8,
    tweak: *const u8,
    tkeys: &KeySchedule,
    rkeys: &KeySchedule,
) {
    let mut rkeys_inv: KeySchedule = *rkeys;
    let tt = aes_xex_tweak(tweak, tkeys);

    softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    let mut t = softaes_block_xor(softaes_block_xor(softaes_block_load(inp), tt), rkeys_inv[10]);
    let mut i: isize = 9;
    while i > 0 {
        t = softaes_block_decrypt(t, rkeys_inv[i as usize]);
        i -= 1;
    }
    t = softaes_block_decryptlast(t, softaes_block_xor(rkeys_inv[0], tt));
    softaes_block_store(out, t);
    sodium_memzero(rkeys_inv.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 11];

    expand_key(&mut rkeys, k);
    aes_encrypt(out, inp, &rkeys);
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 11];

    expand_key(&mut rkeys, k);
    aes_decrypt(out, inp, &rkeys);
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn nd_encrypt(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 11];

    expand_key(&mut rkeys, k);
    memcpy(out as *mut c_void, t as *const c_void, 8);
    aes_encrypt_with_tweak(out.add(8), inp, t, &rkeys);
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn nd_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 11];

    expand_key(&mut rkeys, k);
    aes_decrypt_with_tweak(out, inp.add(8), inp, &rkeys);
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn ndx_encrypt(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 11];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 11];
    let mut diff = [0u8; 16];
    let mut d: u8;

    expand_key(&mut tkeys, k.add(16));
    expand_key(&mut rkeys, k);

    softaes_block_store(diff.as_mut_ptr(), softaes_block_xor(tkeys[10 / 2], rkeys[10 / 2]));
    d = 0;
    for i in 0..16 {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16 {
            diff[i] = *k.add(i) ^ 0x5a;
        }
        expand_key(&mut rkeys, diff.as_ptr());
    }

    memcpy(out as *mut c_void, t as *const c_void, 16);
    aes_xex_encrypt(out.add(16), inp, t, &tkeys, &rkeys);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&diff));
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(tkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn ndx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 11];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 11];
    let mut diff = [0u8; 16];
    let mut d: u8;

    expand_key(&mut tkeys, k.add(16));
    expand_key(&mut rkeys, k);

    softaes_block_store(diff.as_mut_ptr(), softaes_block_xor(tkeys[10 / 2], rkeys[10 / 2]));
    d = 0;
    for i in 0..16 {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16 {
            diff[i] = *k.add(i) ^ 0x5a;
        }
        expand_key(&mut rkeys, diff.as_ptr());
    }

    aes_xex_decrypt(out, inp.add(16), inp, &tkeys, &rkeys);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&diff));
    sodium_memzero(rkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(tkeys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

const IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

unsafe fn is_ipv4_mapped(ip16: *const u8) -> bool {
    memcmp(ip16 as *const c_void, IPV4_MAPPED_PREFIX.as_ptr() as *const c_void, 12) == 0
}

unsafe fn pfx_get_bit(ip16: *const u8, bit_index: u32) -> u8 {
    (*ip16.add(15 - (bit_index / 8) as usize) >> (bit_index % 8)) & 1
}

unsafe fn pfx_set_bit(ip16: *mut u8, bit_index: u32, bit_value: u8) {
    let byte_index = 15 - (bit_index / 8) as usize;
    let bit_mask: u8 = 1u8 << (bit_index % 8);
    // The C code uses `__asm__ __volatile__("" : "+r"(mask) :);` here purely
    // as a compiler barrier to discourage branchy codegen for this
    // constant-time bit select; it has no effect on the computed value.
    let mask: u8 = (bit_value & 1).wrapping_neg();

    *ip16.add(byte_index) = (*ip16.add(byte_index) & !bit_mask) | (bit_mask & mask);
}

unsafe fn pfx_shift_left(ip16: *mut u8) {
    for i in 0..15 {
        *ip16.add(i) = (*ip16.add(i) << 1) | (*ip16.add(i + 1) >> 7);
    }
    let v = *ip16.add(15);
    *ip16.add(15) = v << 1;
}

unsafe fn pfx_pad_prefix(padded_prefix: *mut u8, prefix_len_bits: u32) {
    memset(padded_prefix as *mut c_void, 0, 16);
    if prefix_len_bits == 0 {
        *padded_prefix.add(15) = 0x01;
    } else {
        *padded_prefix.add(3) = 0x01;
        *padded_prefix.add(14) = 0xff;
        *padded_prefix.add(15) = 0xff;
    }
}

unsafe extern "C" fn pfx_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 11];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 11];
    let mut diff = [0u8; 16];
    let mut encrypted = [0u8; 16];
    let mut padded_prefix = [0u8; 16];
    let mut t = [0u8; 16];
    let mut prefix_start: u32 = 0;
    let mut d: u8;

    expand_key(&mut k1keys, k);
    expand_key(&mut k2keys, k.add(16));

    softaes_block_store(diff.as_mut_ptr(), softaes_block_xor(k1keys[10 / 2], k2keys[10 / 2]));
    d = 0;
    for i in 0..16 {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16 {
            diff[i] = *k.add(i) ^ 0x5a;
        }
        expand_key(&mut k2keys, diff.as_ptr());
    }

    if is_ipv4_mapped(inp) {
        prefix_start = 96;
    }

    pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start);

    memset(encrypted.as_mut_ptr() as *mut c_void, 0, 16);
    if prefix_start == 96 {
        encrypted[10] = 0xff;
        encrypted[11] = 0xff;
    }

    let mut prefix_len_bits = prefix_start;
    while prefix_len_bits < 128 {
        let mut e1 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k1keys[0]);
        let mut e2 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k2keys[0]);
        for i in 1..10 {
            e1 = softaes_block_encrypt(e1, k1keys[i]);
            e2 = softaes_block_encrypt(e2, k2keys[i]);
        }
        e1 = softaes_block_encryptlast(e1, k1keys[10]);
        e2 = softaes_block_encryptlast(e2, k2keys[10]);

        let e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        let cipher_bit = t[15] & 1;
        let bit_pos = 127 - prefix_len_bits;
        let original_bit = pfx_get_bit(inp, bit_pos);
        pfx_set_bit(encrypted.as_mut_ptr(), bit_pos, original_bit ^ cipher_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    memcpy(out as *mut c_void, encrypted.as_ptr() as *const c_void, 16);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&diff));
    sodium_memzero(k2keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(k1keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

unsafe extern "C" fn pfx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 11];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 11];
    let mut diff = [0u8; 16];
    let mut decrypted = [0u8; 16];
    let mut padded_prefix = [0u8; 16];
    let mut t = [0u8; 16];
    let mut prefix_start: u32 = 0;
    let mut d: u8;

    expand_key(&mut k1keys, k);
    expand_key(&mut k2keys, k.add(16));

    softaes_block_store(diff.as_mut_ptr(), softaes_block_xor(k1keys[10 / 2], k2keys[10 / 2]));
    d = 0;
    for i in 0..16 {
        d |= diff[i];
    }
    if d == 0 {
        for i in 0..16 {
            diff[i] = *k.add(i) ^ 0x5a;
        }
        expand_key(&mut k2keys, diff.as_ptr());
    }

    if is_ipv4_mapped(inp) {
        prefix_start = 96;
    }

    pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start);

    memset(decrypted.as_mut_ptr() as *mut c_void, 0, 16);
    if prefix_start == 96 {
        decrypted[10] = 0xff;
        decrypted[11] = 0xff;
    }

    let mut prefix_len_bits = prefix_start;
    while prefix_len_bits < 128 {
        let mut e1 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k1keys[0]);
        let mut e2 = softaes_block_xor(softaes_block_load(padded_prefix.as_ptr()), k2keys[0]);
        for i in 1..10 {
            e1 = softaes_block_encrypt(e1, k1keys[i]);
            e2 = softaes_block_encrypt(e2, k2keys[i]);
        }
        e1 = softaes_block_encryptlast(e1, k1keys[10]);
        e2 = softaes_block_encryptlast(e2, k2keys[10]);

        let e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        let cipher_bit = t[15] & 1;
        let bit_pos = 127 - prefix_len_bits;
        let encrypted_bit = pfx_get_bit(inp, bit_pos);
        let original_bit = encrypted_bit ^ cipher_bit;
        pfx_set_bit(decrypted.as_mut_ptr(), bit_pos, original_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    memcpy(out as *mut c_void, decrypted.as_ptr() as *const c_void, 16);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&diff));
    sodium_memzero(k2keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
    sodium_memzero(k1keys.as_mut_ptr() as *mut c_void, core::mem::size_of::<KeySchedule>());
}

#[repr(C)]
pub struct ipcrypt_implementation {
    pub encrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, k: *const u8),
    pub decrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, k: *const u8),
    pub nd_encrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8),
    pub nd_decrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, k: *const u8),
    pub ndx_encrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, t: *const u8, k: *const u8),
    pub ndx_decrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, k: *const u8),
    pub pfx_encrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, k: *const u8),
    pub pfx_decrypt: unsafe extern "C" fn(out: *mut u8, inp: *const u8, k: *const u8),
}

/// `crypto_ipcrypt/ipcrypt_soft.h`: `extern struct ipcrypt_implementation
/// ipcrypt_soft_implementation;` — exported data object.
#[no_mangle]
pub static ipcrypt_soft_implementation: ipcrypt_implementation = ipcrypt_implementation {
    encrypt,
    decrypt,
    nd_encrypt,
    nd_decrypt,
    ndx_encrypt,
    ndx_decrypt,
    pfx_encrypt,
    pfx_decrypt,
};

// ---------------------------------------------------------------------
// crypto_ipcrypt.c
// ---------------------------------------------------------------------

/// `static const ipcrypt_implementation *implementation =
/// &ipcrypt_soft_implementation;` — reassigned (to the same value) by
/// `_crypto_ipcrypt_pick_best_implementation()`, hence `static mut` per
/// convention.
static mut IMPLEMENTATION: *const ipcrypt_implementation = &ipcrypt_soft_implementation;

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_bytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_keybytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keybytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_tweakbytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_inputbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_outputbytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_tweakbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_inputbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_outputbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_bytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 16);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 16);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).encrypt)(out, inp, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).decrypt)(out, inp, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_encrypt(
    out: *mut u8,
    inp: *const u8,
    t: *const u8,
    k: *const u8,
) {
    ((*IMPLEMENTATION).nd_encrypt)(out, inp, t, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_nd_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).nd_decrypt)(out, inp, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_encrypt(
    out: *mut u8,
    inp: *const u8,
    t: *const u8,
    k: *const u8,
) {
    ((*IMPLEMENTATION).ndx_encrypt)(out, inp, t, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_ndx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).ndx_decrypt)(out, inp, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_encrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).pfx_encrypt)(out, inp, k);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_ipcrypt_pfx_decrypt(out: *mut u8, inp: *const u8, k: *const u8) {
    ((*IMPLEMENTATION).pfx_decrypt)(out, inp, k);
}

#[no_mangle]
pub unsafe extern "C" fn _crypto_ipcrypt_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &ipcrypt_soft_implementation;
    0
}
