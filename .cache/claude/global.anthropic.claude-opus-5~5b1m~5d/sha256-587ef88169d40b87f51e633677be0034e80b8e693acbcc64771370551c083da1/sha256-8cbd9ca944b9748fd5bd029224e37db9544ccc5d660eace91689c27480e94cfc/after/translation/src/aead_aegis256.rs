//! Translation of `crypto_aead/aegis256/aead_aegis256.c` and
//! `crypto_aead/aegis256/aegis256_soft.c` (which textually includes
//! `aegis256_common.h` after defining the `AES_BLOCK_*` macro layer over
//! `softaes`).
//!
//! The reference build has no `HAVE_AVX*`/AES-NI/ARM-crypto support, so only
//! the "soft" (bitsliced constant-time) AES implementation is compiled, and
//! `_crypto_aead_aegis256_pick_best_implementation` always selects it.

#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_void};

use crate::common::{load32_le, store32_le, SODIUM_SIZE_MAX};
use crate::csys::{memcpy, memset};

extern "C" {
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
}

// ---------------------------------------------------------------------
// include/sodium/private/softaes.h
// ---------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct SoftAesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

extern "C" {
    #[link_name = "_sodium_softaes_block_encrypt"]
    fn softaes_block_encrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
}

#[inline(always)]
unsafe fn softaes_block_load(inp: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: load32_le(inp),
        w1: load32_le(inp.add(4)),
        w2: load32_le(inp.add(8)),
        w3: load32_le(inp.add(12)),
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
unsafe fn softaes_block_store(out: *mut u8, inb: SoftAesBlock) {
    store32_le(out, inb.w0);
    store32_le(out.add(4), inb.w1);
    store32_le(out.add(8), inb.w2);
    store32_le(out.add(12), inb.w3);
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

// ---------------------------------------------------------------------
// aegis256_soft.c: AES_BLOCK_* macro layer over softaes
// ---------------------------------------------------------------------

type AesBlockT = SoftAesBlock;

#[inline(always)]
unsafe fn aegis256_update(state: *mut AesBlockT, d: AesBlockT) {
    let tmp = *state.add(5);
    *state.add(5) = softaes_block_encrypt(*state.add(4), *state.add(5));
    *state.add(4) = softaes_block_encrypt(*state.add(3), *state.add(4));
    *state.add(3) = softaes_block_encrypt(*state.add(2), *state.add(3));
    *state.add(2) = softaes_block_encrypt(*state.add(1), *state.add(2));
    *state.add(1) = softaes_block_encrypt(*state.add(0), *state.add(1));
    *state.add(0) = softaes_block_xor(softaes_block_encrypt(tmp, *state.add(0)), d);
}

// ---------------------------------------------------------------------
// crypto_aead/aegis256/aegis256_common.h
// ---------------------------------------------------------------------

#[inline(always)]
unsafe fn aegis256_init(key: *const u8, nonce: *const u8, state: *mut AesBlockT) {
    static C0_: [u8; 16] = [
        0x00, 0x01, 0x01, 0x02, 0x03, 0x05, 0x08, 0x0d, 0x15, 0x22, 0x37, 0x59, 0x90, 0xe9, 0x79,
        0x62,
    ];
    static C1_: [u8; 16] = [
        0xdb, 0x3d, 0x18, 0x55, 0x6d, 0xc2, 0x2f, 0xf1, 0x20, 0x11, 0x31, 0x42, 0x73, 0xb5, 0x28,
        0xdd,
    ];

    let c0 = softaes_block_load(C0_.as_ptr());
    let c1 = softaes_block_load(C1_.as_ptr());
    let k0 = softaes_block_load(key);
    let k1 = softaes_block_load(key.add(16));
    let n0 = softaes_block_load(nonce);
    let n1 = softaes_block_load(nonce.add(16));
    let k0_n0 = softaes_block_xor(k0, n0);
    let k1_n1 = softaes_block_xor(k1, n1);

    *state.add(0) = k0_n0;
    *state.add(1) = k1_n1;
    *state.add(2) = c1;
    *state.add(3) = c0;
    *state.add(4) = softaes_block_xor(k0, c0);
    *state.add(5) = softaes_block_xor(k1, c1);
    for _ in 0..4 {
        aegis256_update(state, k0);
        aegis256_update(state, k1);
        aegis256_update(state, k0_n0);
        aegis256_update(state, k1_n1);
    }
}

#[inline(always)]
unsafe fn aegis256_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: *mut AesBlockT,
) -> c_int {
    let mut tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, *state.add(3));

    for _ in 0..7 {
        aegis256_update(state, tmp);
    }

    if maclen == 16 {
        tmp = softaes_block_xor(*state.add(5), *state.add(4));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(3), *state.add(2)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
    } else if maclen == 32 {
        tmp = softaes_block_xor(softaes_block_xor(*state.add(2), *state.add(1)), *state.add(0));
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(softaes_block_xor(*state.add(5), *state.add(4)), *state.add(3));
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac as *mut c_void, 0, maclen);
        return -1;
    }
    0
}

#[inline(always)]
unsafe fn aegis256_absorb(src: *const u8, state: *mut AesBlockT) {
    let msg = softaes_block_load(src);
    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_absorb2(src: *const u8, state: *mut AesBlockT) {
    let msg = softaes_block_load(src.add(0 * 16));
    let msg2 = softaes_block_load(src.add(1 * 16));
    aegis256_update(state, msg);
    aegis256_update(state, msg2);
}

#[inline(always)]
unsafe fn aegis256_enc(dst: *mut u8, src: *const u8, state: *mut AesBlockT) {
    let msg = softaes_block_load(src);
    let mut tmp = softaes_block_xor(msg, *state.add(5));
    tmp = softaes_block_xor(tmp, *state.add(4));
    tmp = softaes_block_xor(tmp, *state.add(1));
    tmp = softaes_block_xor(tmp, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(dst, tmp);

    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_dec(dst: *mut u8, src: *const u8, state: *mut AesBlockT) {
    let mut msg = softaes_block_load(src);
    msg = softaes_block_xor(msg, *state.add(5));
    msg = softaes_block_xor(msg, *state.add(4));
    msg = softaes_block_xor(msg, *state.add(1));
    msg = softaes_block_xor(msg, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(dst, msg);

    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_declast(dst: *mut u8, src: *const u8, len: usize, state: *mut AesBlockT) {
    let mut pad = [0u8; 16];

    memset(pad.as_mut_ptr() as *mut c_void, 0, 16);
    memcpy(pad.as_mut_ptr() as *mut c_void, src as *const c_void, len);

    let mut msg = softaes_block_load(pad.as_ptr());
    msg = softaes_block_xor(msg, *state.add(5));
    msg = softaes_block_xor(msg, *state.add(4));
    msg = softaes_block_xor(msg, *state.add(1));
    msg = softaes_block_xor(msg, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(pad.as_mut_ptr(), msg);

    memset(pad.as_mut_ptr().add(len) as *mut c_void, 0, 16 - len);
    memcpy(dst as *mut c_void, pad.as_ptr() as *const c_void, len);

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
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 6];
    let mut src = [0u8; 16];
    let mut dst = [0u8; 16];
    let mut i: usize;

    aegis256_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 2 * 16 <= adlen {
        aegis256_absorb2(ad.add(i), state.as_mut_ptr());
        i += 2 * 16;
    }
    while i + 16 <= adlen {
        aegis256_absorb(ad.add(i), state.as_mut_ptr());
        i += 16;
    }
    if adlen % 16 != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, 16);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % 16);
        aegis256_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    i = 0;
    while i + 16 <= mlen {
        aegis256_enc(c.add(i), m.add(i), state.as_mut_ptr());
        i += 16;
    }
    if mlen % 16 != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, 16);
        memcpy(src.as_mut_ptr() as *mut c_void, m.add(i) as *const c_void, mlen % 16);
        aegis256_enc(dst.as_mut_ptr(), src.as_ptr(), state.as_mut_ptr());
        memcpy(c.add(i) as *mut c_void, dst.as_ptr() as *const c_void, mlen % 16);
    }

    aegis256_mac(mac, maclen, adlen as u64, mlen as u64, state.as_mut_ptr())
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
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 6];
    let mut src = [0u8; 16];
    let mut dst = [0u8; 16];
    let mut computed_mac = [0u8; 32];
    let mlen = clen;
    let mut i: usize;
    let mut ret: c_int;

    aegis256_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 2 * 16 <= adlen {
        aegis256_absorb2(ad.add(i), state.as_mut_ptr());
        i += 2 * 16;
    }
    while i + 16 <= adlen {
        aegis256_absorb(ad.add(i), state.as_mut_ptr());
        i += 16;
    }
    if adlen % 16 != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, 16);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % 16);
        aegis256_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    if !m.is_null() {
        i = 0;
        while i + 16 <= mlen {
            aegis256_dec(m.add(i), c.add(i), state.as_mut_ptr());
            i += 16;
        }
    } else {
        i = 0;
        while i + 16 <= mlen {
            aegis256_dec(dst.as_mut_ptr(), c.add(i), state.as_mut_ptr());
            i += 16;
        }
    }
    if mlen % 16 != 0 {
        if !m.is_null() {
            aegis256_declast(m.add(i), c.add(i), mlen % 16, state.as_mut_ptr());
        } else {
            aegis256_declast(dst.as_mut_ptr(), c.add(i), mlen % 16, state.as_mut_ptr());
        }
    }

    ret = -1;
    if aegis256_mac(computed_mac.as_mut_ptr(), maclen, adlen as u64, mlen as u64, state.as_mut_ptr()) == 0 {
        if maclen == 16 {
            ret = crypto_verify_16(computed_mac.as_ptr(), mac);
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
    0
}

// ---------------------------------------------------------------------
// crypto_aead/aegis256/implementations.h
// ---------------------------------------------------------------------

#[repr(C)]
pub struct Aegis256Implementation {
    encrypt_detached: unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        usize,
        *const u8,
        usize,
        *const u8,
        usize,
        *const u8,
        *const u8,
    ) -> c_int,
    decrypt_detached: unsafe extern "C" fn(
        *mut u8,
        *const u8,
        usize,
        *const u8,
        usize,
        *const u8,
        usize,
        *const u8,
        *const u8,
    ) -> c_int,
}

#[no_mangle]
pub static aegis256_soft_implementation: Aegis256Implementation = Aegis256Implementation {
    encrypt_detached,
    decrypt_detached,
};

// ---------------------------------------------------------------------
// crypto_aead/aegis256/aead_aegis256.c
// ---------------------------------------------------------------------

static mut IMPLEMENTATION: *const Aegis256Implementation = &aegis256_soft_implementation;

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_nsecbytes() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_npubbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_abytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_messagebytes_max() -> usize {
    let a = SODIUM_SIZE_MAX - 32;
    let b = (1u64 << 61) - 1;
    (if a < b { a } else { b }) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt(
    c: *mut u8,
    clen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut clen: u64 = 0;
    let ret: c_int;

    if mlen > crypto_aead_aegis256_messagebytes_max() as u64 {
        sodium_misuse();
    }
    ret = crypto_aead_aegis256_encrypt_detached(
        c,
        c.add(mlen as usize),
        core::ptr::null_mut(),
        m,
        mlen,
        ad,
        adlen,
        nsec,
        npub,
        k,
    );
    if !clen_p.is_null() {
        if ret == 0 {
            clen = mlen + 32;
        }
        *clen_p = clen;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt(
    m: *mut u8,
    mlen_p: *mut u64,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut mlen: u64 = 0;
    let mut ret: c_int = -1;

    if clen >= 32 {
        ret = crypto_aead_aegis256_decrypt_detached(
            m,
            nsec,
            c,
            clen - 32,
            c.add((clen - 32) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - 32;
        }
        *mlen_p = mlen;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let maclen: usize = 32;

    let _ = nsec;
    if !maclen_p.is_null() {
        *maclen_p = maclen as u64;
    }
    if mlen > crypto_aead_aegis256_messagebytes_max() as u64
        || adlen > crypto_aead_aegis256_messagebytes_max() as u64
    {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).encrypt_detached)(c, mac, maclen, m, mlen as usize, ad, adlen as usize, npub, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let maclen: usize = 32;

    let _ = nsec;
    if clen > crypto_aead_aegis256_messagebytes_max() as u64
        || adlen > crypto_aead_aegis256_messagebytes_max() as u64
    {
        return -1;
    }
    ((*IMPLEMENTATION).decrypt_detached)(m, c, clen as usize, mac, maclen, ad, adlen as usize, npub, k)
}

#[no_mangle]
pub unsafe extern "C" fn _crypto_aead_aegis256_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &aegis256_soft_implementation;
    0
}
