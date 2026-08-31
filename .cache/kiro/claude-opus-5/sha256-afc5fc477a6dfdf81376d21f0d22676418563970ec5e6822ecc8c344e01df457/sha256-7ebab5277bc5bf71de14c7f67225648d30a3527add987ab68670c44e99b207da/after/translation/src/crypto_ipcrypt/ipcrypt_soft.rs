//! Translation of c_src/libsodium/crypto_ipcrypt/ipcrypt_soft.c

use core::ffi::{c_int, c_uint, c_void};

use crate::crypto_core::softaes::softaes::{
    softaes_block_load, softaes_block_store, softaes_block_xor, SoftAesBlock,
    _sodium_softaes_block_decrypt, _sodium_softaes_block_decryptlast,
    _sodium_softaes_block_encrypt, _sodium_softaes_block_encryptlast,
    _sodium_softaes_expand_key128, _sodium_softaes_inv_mix_columns,
    _sodium_softaes_invert_key_schedule128,
};

const ROUNDS: usize = 10;

type aes_block_t = SoftAesBlock;

// typedef aes_block_t KeySchedule[1 + ROUNDS];
type KeySchedule = [aes_block_t; 1 + ROUNDS];

const ZERO_BLOCK: aes_block_t = SoftAesBlock {
    w0: 0,
    w1: 0,
    w2: 0,
    w3: 0,
};

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
}

#[inline]
unsafe fn AES_ENC(a: aes_block_t, b: aes_block_t) -> aes_block_t {
    _sodium_softaes_block_encrypt(a, b)
}
#[inline]
unsafe fn AES_ENCLAST(a: aes_block_t, b: aes_block_t) -> aes_block_t {
    _sodium_softaes_block_encryptlast(a, b)
}
#[inline]
unsafe fn AES_DEC(a: aes_block_t, b: aes_block_t) -> aes_block_t {
    _sodium_softaes_block_decrypt(a, b)
}
#[inline]
unsafe fn AES_DECLAST(a: aes_block_t, b: aes_block_t) -> aes_block_t {
    _sodium_softaes_block_decryptlast(a, b)
}
#[inline]
unsafe fn AES_INV_MIX(a: aes_block_t) -> aes_block_t {
    _sodium_softaes_inv_mix_columns(a)
}

unsafe fn expand_key(rkeys: *mut aes_block_t, key: *const u8) {
    _sodium_softaes_expand_key128(rkeys, key);
}

unsafe fn aes_encrypt(out: *mut u8, in_: *const u8, rkeys: *const aes_block_t) {
    let mut t: aes_block_t;
    let mut i: usize;

    t = softaes_block_xor(softaes_block_load(in_), *rkeys.add(0));
    i = 1;
    while i < ROUNDS {
        t = AES_ENC(t, *rkeys.add(i));
        i += 1;
    }
    t = AES_ENCLAST(t, *rkeys.add(ROUNDS));
    softaes_block_store(out, t);
}

unsafe fn aes_decrypt(out: *mut u8, in_: *const u8, rkeys: *const aes_block_t) {
    let mut rkeys_inv: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut t: aes_block_t;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = *rkeys.add(i);
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(softaes_block_load(in_), rkeys_inv[ROUNDS]);
    i = ROUNDS - 1;
    while i > 0 {
        t = AES_DEC(t, rkeys_inv[i]);
        i -= 1;
    }
    t = AES_DECLAST(t, rkeys_inv[0]);
    softaes_block_store(out, t);
    sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn tweak_expand(tweak: *const u8) -> aes_block_t {
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
    rkeys: *const aes_block_t,
) {
    let tweak_block = tweak_expand(tweak);
    let mut t: aes_block_t;
    let mut i: usize;

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tweak_block), *rkeys.add(0));
    i = 1;
    while i < ROUNDS {
        t = AES_ENC(t, softaes_block_xor(tweak_block, *rkeys.add(i)));
        i += 1;
    }
    t = AES_ENCLAST(t, softaes_block_xor(tweak_block, *rkeys.add(ROUNDS)));
    softaes_block_store(out, t);
}

unsafe fn aes_decrypt_with_tweak(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    rkeys: *const aes_block_t,
) {
    let mut rkeys_inv: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let tweak_block = tweak_expand(tweak);
    let tweak_block_inv = AES_INV_MIX(tweak_block);
    let mut t: aes_block_t;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = *rkeys.add(i);
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tweak_block), rkeys_inv[ROUNDS]);
    i = ROUNDS - 1;
    while i > 0 {
        t = AES_DEC(t, softaes_block_xor(tweak_block_inv, rkeys_inv[i]));
        i -= 1;
    }
    t = AES_DECLAST(t, softaes_block_xor(tweak_block, rkeys_inv[0]));
    softaes_block_store(out, t);
    sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn aes_xex_tweak(tweak: *const u8, tkeys: *const aes_block_t) -> aes_block_t {
    let mut tt: aes_block_t;
    let mut i: usize;

    tt = softaes_block_xor(softaes_block_load(tweak), *tkeys.add(0));
    i = 1;
    while i < ROUNDS {
        tt = AES_ENC(tt, *tkeys.add(i));
        i += 1;
    }
    tt = AES_ENCLAST(tt, *tkeys.add(ROUNDS));
    tt
}

unsafe fn aes_xex_encrypt(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    tkeys: *const aes_block_t,
    rkeys: *const aes_block_t,
) {
    let tt = aes_xex_tweak(tweak, tkeys);
    let mut t: aes_block_t;
    let mut i: usize;

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tt), *rkeys.add(0));
    i = 1;
    while i < ROUNDS {
        t = AES_ENC(t, *rkeys.add(i));
        i += 1;
    }
    t = AES_ENCLAST(t, softaes_block_xor(*rkeys.add(ROUNDS), tt));
    softaes_block_store(out, t);
}

unsafe fn aes_xex_decrypt(
    out: *mut u8,
    in_: *const u8,
    tweak: *const u8,
    tkeys: *const aes_block_t,
    rkeys: *const aes_block_t,
) {
    let mut rkeys_inv: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let tt = aes_xex_tweak(tweak, tkeys);
    let mut t: aes_block_t;
    let mut i: usize;

    i = 0;
    while i <= ROUNDS {
        rkeys_inv[i] = *rkeys.add(i);
        i += 1;
    }
    _sodium_softaes_invert_key_schedule128(rkeys_inv.as_mut_ptr());

    t = softaes_block_xor(softaes_block_xor(softaes_block_load(in_), tt), rkeys_inv[ROUNDS]);
    i = ROUNDS - 1;
    while i > 0 {
        t = AES_DEC(t, rkeys_inv[i]);
        i -= 1;
    }
    t = AES_DECLAST(t, softaes_block_xor(rkeys_inv[0], tt));
    softaes_block_store(out, t);
    sodium_memzero(
        rkeys_inv.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(rkeys.as_mut_ptr(), k);
    aes_encrypt(out, in_, rkeys.as_ptr());
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(rkeys.as_mut_ptr(), k);
    aes_decrypt(out, in_, rkeys.as_ptr());
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn nd_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(rkeys.as_mut_ptr(), k);
    memcpy(out as *mut c_void, t as *const c_void, 8);
    aes_encrypt_with_tweak(out.add(8), in_, t, rkeys.as_ptr());
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn nd_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];

    expand_key(rkeys.as_mut_ptr(), k);
    aes_decrypt_with_tweak(out, in_.add(8), in_, rkeys.as_ptr());
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn ndx_encrypt(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
    let mut i: usize;
    let mut d: u8;

    expand_key(tkeys.as_mut_ptr(), k.add(16));
    expand_key(rkeys.as_mut_ptr(), k);

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
        expand_key(rkeys.as_mut_ptr(), diff.as_ptr());
    }

    memcpy(out as *mut c_void, t as *const c_void, 16);
    aes_xex_encrypt(out.add(16), in_, t, tkeys.as_ptr(), rkeys.as_ptr());
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 16]>());
    sodium_memzero(
        rkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        tkeys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn ndx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut tkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut rkeys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
    let mut i: usize;
    let mut d: u8;

    expand_key(tkeys.as_mut_ptr(), k.add(16));
    expand_key(rkeys.as_mut_ptr(), k);

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
        expand_key(rkeys.as_mut_ptr(), diff.as_ptr());
    }

    aes_xex_decrypt(out, in_.add(16), in_, tkeys.as_ptr(), rkeys.as_ptr());
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 16]>());
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

    (memcmp(
        ip16 as *const c_void,
        IPV4_MAPPED_PREFIX.as_ptr() as *const c_void,
        12,
    ) == 0) as c_int
}

unsafe fn pfx_get_bit(ip16: *const u8, bit_index: c_uint) -> u8 {
    (*ip16.add(15 - (bit_index / 8) as usize) >> (bit_index % 8)) & 1
}

unsafe fn pfx_set_bit(ip16: *mut u8, bit_index: c_uint, bit_value: u8) {
    let byte_index: usize = 15 - (bit_index / 8) as usize;
    let bit_mask: u8 = (1u32 << (bit_index % 8)) as u8;
    let mask: u8 = (0i32.wrapping_sub((bit_value & 1) as i32)) as u8;

    // __asm__ __volatile__("" : "+r"(mask) :); barrier is a no-op.
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

unsafe fn pfx_pad_prefix(padded_prefix: *mut u8, prefix_len_bits: c_uint) {
    memset(padded_prefix as *mut c_void, 0, 16);
    if prefix_len_bits == 0 {
        *padded_prefix.add(15) = 0x01;
    } else {
        *padded_prefix.add(3) = 0x01;
        *padded_prefix.add(14) = 0xff;
        *padded_prefix.add(15) = 0xff;
    }
}

unsafe fn pfx_encrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
    let mut encrypted: [u8; 16] = [0; 16];
    let mut padded_prefix: [u8; 16] = [0; 16];
    let mut t: [u8; 16] = [0; 16];
    let mut e1: aes_block_t;
    let mut e2: aes_block_t;
    let mut e: aes_block_t;
    let mut prefix_start: c_uint = 0;
    let mut prefix_len_bits: c_uint;
    let mut i: usize;
    let mut d: u8;

    expand_key(k1keys.as_mut_ptr(), k);
    expand_key(k2keys.as_mut_ptr(), k.add(16));

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
        expand_key(k2keys.as_mut_ptr(), diff.as_ptr());
    }

    if is_ipv4_mapped(in_) != 0 {
        prefix_start = 96;
    }

    pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start);

    memset(encrypted.as_mut_ptr() as *mut c_void, 0, 16);
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
            e1 = AES_ENC(e1, k1keys[i]);
            e2 = AES_ENC(e2, k2keys[i]);
            i += 1;
        }
        e1 = AES_ENCLAST(e1, k1keys[ROUNDS]);
        e2 = AES_ENCLAST(e2, k2keys[ROUNDS]);

        e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        let cipher_bit = t[15] & 1;
        let bit_pos = 127 - prefix_len_bits;
        let original_bit = pfx_get_bit(in_, bit_pos);
        pfx_set_bit(encrypted.as_mut_ptr(), bit_pos, original_bit ^ cipher_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    memcpy(out as *mut c_void, encrypted.as_ptr() as *const c_void, 16);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 16]>());
    sodium_memzero(
        k2keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        k1keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

unsafe fn pfx_decrypt(out: *mut u8, in_: *const u8, k: *const u8) {
    let mut k1keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut k2keys: KeySchedule = [ZERO_BLOCK; 1 + ROUNDS];
    let mut diff: [u8; 16] = [0; 16];
    let mut decrypted: [u8; 16] = [0; 16];
    let mut padded_prefix: [u8; 16] = [0; 16];
    let mut t: [u8; 16] = [0; 16];
    let mut e1: aes_block_t;
    let mut e2: aes_block_t;
    let mut e: aes_block_t;
    let mut prefix_start: c_uint = 0;
    let mut prefix_len_bits: c_uint;
    let mut i: usize;
    let mut d: u8;

    expand_key(k1keys.as_mut_ptr(), k);
    expand_key(k2keys.as_mut_ptr(), k.add(16));

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
        expand_key(k2keys.as_mut_ptr(), diff.as_ptr());
    }

    if is_ipv4_mapped(in_) != 0 {
        prefix_start = 96;
    }

    pfx_pad_prefix(padded_prefix.as_mut_ptr(), prefix_start);

    memset(decrypted.as_mut_ptr() as *mut c_void, 0, 16);
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
            e1 = AES_ENC(e1, k1keys[i]);
            e2 = AES_ENC(e2, k2keys[i]);
            i += 1;
        }
        e1 = AES_ENCLAST(e1, k1keys[ROUNDS]);
        e2 = AES_ENCLAST(e2, k2keys[ROUNDS]);

        e = softaes_block_xor(e1, e2);
        softaes_block_store(t.as_mut_ptr(), e);

        let cipher_bit = t[15] & 1;
        let bit_pos = 127 - prefix_len_bits;
        let encrypted_bit = pfx_get_bit(in_, bit_pos);
        let original_bit = encrypted_bit ^ cipher_bit;
        pfx_set_bit(decrypted.as_mut_ptr(), bit_pos, original_bit);

        pfx_shift_left(padded_prefix.as_mut_ptr());
        pfx_set_bit(padded_prefix.as_mut_ptr(), 0, original_bit);

        prefix_len_bits += 1;
    }

    memcpy(out as *mut c_void, decrypted.as_ptr() as *const c_void, 16);
    sodium_memzero(diff.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 16]>());
    sodium_memzero(
        k2keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
    sodium_memzero(
        k1keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<KeySchedule>(),
    );
}

// ---- exported implementation struct ----
// (see crypto_ipcrypt/implementations.h)

type EncDecFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type NdxEncFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);

#[repr(C)]
pub struct ipcrypt_implementation {
    pub encrypt: Option<EncDecFn>,
    pub decrypt: Option<EncDecFn>,
    pub nd_encrypt: Option<NdxEncFn>,
    pub nd_decrypt: Option<EncDecFn>,
    pub ndx_encrypt: Option<NdxEncFn>,
    pub ndx_decrypt: Option<EncDecFn>,
    pub pfx_encrypt: Option<EncDecFn>,
    pub pfx_decrypt: Option<EncDecFn>,
}

unsafe impl Sync for ipcrypt_implementation {}

unsafe extern "C" fn encrypt_c(out: *mut u8, in_: *const u8, k: *const u8) {
    encrypt(out, in_, k)
}
unsafe extern "C" fn decrypt_c(out: *mut u8, in_: *const u8, k: *const u8) {
    decrypt(out, in_, k)
}
unsafe extern "C" fn nd_encrypt_c(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    nd_encrypt(out, in_, t, k)
}
unsafe extern "C" fn nd_decrypt_c(out: *mut u8, in_: *const u8, k: *const u8) {
    nd_decrypt(out, in_, k)
}
unsafe extern "C" fn ndx_encrypt_c(out: *mut u8, in_: *const u8, t: *const u8, k: *const u8) {
    ndx_encrypt(out, in_, t, k)
}
unsafe extern "C" fn ndx_decrypt_c(out: *mut u8, in_: *const u8, k: *const u8) {
    ndx_decrypt(out, in_, k)
}
unsafe extern "C" fn pfx_encrypt_c(out: *mut u8, in_: *const u8, k: *const u8) {
    pfx_encrypt(out, in_, k)
}
unsafe extern "C" fn pfx_decrypt_c(out: *mut u8, in_: *const u8, k: *const u8) {
    pfx_decrypt(out, in_, k)
}

#[unsafe(no_mangle)]
pub static ipcrypt_soft_implementation: ipcrypt_implementation = ipcrypt_implementation {
    encrypt: Some(encrypt_c),
    decrypt: Some(decrypt_c),
    nd_encrypt: Some(nd_encrypt_c),
    nd_decrypt: Some(nd_decrypt_c),
    ndx_encrypt: Some(ndx_encrypt_c),
    ndx_decrypt: Some(ndx_decrypt_c),
    pfx_encrypt: Some(pfx_encrypt_c),
    pfx_decrypt: Some(pfx_decrypt_c),
};
