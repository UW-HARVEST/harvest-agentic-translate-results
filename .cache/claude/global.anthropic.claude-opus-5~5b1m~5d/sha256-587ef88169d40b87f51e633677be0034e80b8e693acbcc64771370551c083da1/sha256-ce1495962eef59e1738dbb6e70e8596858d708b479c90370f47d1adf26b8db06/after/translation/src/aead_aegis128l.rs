//! Translation of `crypto_aead/aegis128l/aead_aegis128l.c` and
//! `crypto_aead/aegis128l/aegis128l_soft.c` (which textually includes
//! `aegis128l_common.h` after defining the `AES_BLOCK_*` macro layer over
//! `softaes`).
//!
//! The reference build has no `HAVE_AVX*`/AES-NI/ARM-crypto support, so only
//! the "soft" (bitsliced constant-time) AES implementation is compiled, and
//! `_crypto_aead_aegis128l_pick_best_implementation` always selects it.

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
// aegis128l_soft.c: AES_BLOCK_* macro layer over softaes
// ---------------------------------------------------------------------

type AesBlockT = SoftAesBlock;

#[inline(always)]
unsafe fn aegis128l_update(state: *mut AesBlockT, d1: AesBlockT, d2: AesBlockT) {
    let tmp = *state.add(7);
    *state.add(7) = softaes_block_encrypt(*state.add(6), *state.add(7));
    *state.add(6) = softaes_block_encrypt(*state.add(5), *state.add(6));
    *state.add(5) = softaes_block_encrypt(*state.add(4), *state.add(5));
    *state.add(4) = softaes_block_encrypt(*state.add(3), *state.add(4));
    *state.add(3) = softaes_block_encrypt(*state.add(2), *state.add(3));
    *state.add(2) = softaes_block_encrypt(*state.add(1), *state.add(2));
    *state.add(1) = softaes_block_encrypt(*state.add(0), *state.add(1));
    *state.add(0) = softaes_block_encrypt(tmp, *state.add(0));

    *state.add(0) = softaes_block_xor(*state.add(0), d1);
    *state.add(4) = softaes_block_xor(*state.add(4), d2);
}

// ---------------------------------------------------------------------
// crypto_aead/aegis128l/aegis128l_common.h
// ---------------------------------------------------------------------

#[inline(always)]
unsafe fn aegis128l_init(key: *const u8, nonce: *const u8, state: *mut AesBlockT) {
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

    let k = softaes_block_load(key);
    let n = softaes_block_load(nonce);

    *state.add(0) = softaes_block_xor(k, n);
    *state.add(1) = c1;
    *state.add(2) = c0;
    *state.add(3) = c1;
    *state.add(4) = softaes_block_xor(k, n);
    *state.add(5) = softaes_block_xor(k, c0);
    *state.add(6) = softaes_block_xor(k, c1);
    *state.add(7) = softaes_block_xor(k, c0);
    for _ in 0..10 {
        aegis128l_update(state, n, k);
    }
}

#[inline(always)]
unsafe fn aegis128l_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: *mut AesBlockT,
) -> c_int {
    let mut tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, *state.add(2));

    for _ in 0..7 {
        aegis128l_update(state, tmp, tmp);
    }

    if maclen == 16 {
        tmp = softaes_block_xor(*state.add(6), softaes_block_xor(*state.add(5), *state.add(4)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(3), *state.add(2)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
    } else if maclen == 32 {
        tmp = softaes_block_xor(*state.add(3), *state.add(2));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(*state.add(7), *state.add(6));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(5), *state.add(4)));
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac as *mut c_void, 0, maclen);
        return -1;
    }
    0
}

#[inline(always)]
unsafe fn aegis128l_absorb(src: *const u8, state: *mut AesBlockT) {
    let msg0 = softaes_block_load(src);
    let msg1 = softaes_block_load(src.add(16));
    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_absorb2(src: *const u8, state: *mut AesBlockT) {
    let msg0 = softaes_block_load(src.add(0 * 16));
    let msg1 = softaes_block_load(src.add(1 * 16));
    let msg2 = softaes_block_load(src.add(2 * 16));
    let msg3 = softaes_block_load(src.add(3 * 16));
    aegis128l_update(state, msg0, msg1);
    aegis128l_update(state, msg2, msg3);
}

#[inline(always)]
unsafe fn aegis128l_enc(dst: *mut u8, src: *const u8, state: *mut AesBlockT) {
    let msg0 = softaes_block_load(src);
    let msg1 = softaes_block_load(src.add(16));
    let mut tmp0 = softaes_block_xor(msg0, *state.add(6));
    tmp0 = softaes_block_xor(tmp0, *state.add(1));
    let mut tmp1 = softaes_block_xor(msg1, *state.add(5));
    tmp1 = softaes_block_xor(tmp1, *state.add(2));
    tmp0 = softaes_block_xor(tmp0, softaes_block_and(*state.add(2), *state.add(3)));
    tmp1 = softaes_block_xor(tmp1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(dst, tmp0);
    softaes_block_store(dst.add(16), tmp1);

    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_dec(dst: *mut u8, src: *const u8, state: *mut AesBlockT) {
    let mut msg0 = softaes_block_load(src);
    let mut msg1 = softaes_block_load(src.add(16));
    msg0 = softaes_block_xor(msg0, *state.add(6));
    msg0 = softaes_block_xor(msg0, *state.add(1));
    msg1 = softaes_block_xor(msg1, *state.add(5));
    msg1 = softaes_block_xor(msg1, *state.add(2));
    msg0 = softaes_block_xor(msg0, softaes_block_and(*state.add(2), *state.add(3)));
    msg1 = softaes_block_xor(msg1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(dst, msg0);
    softaes_block_store(dst.add(16), msg1);

    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_declast(dst: *mut u8, src: *const u8, len: usize, state: *mut AesBlockT) {
    let mut pad = [0u8; 32];

    memset(pad.as_mut_ptr() as *mut c_void, 0, 32);
    memcpy(pad.as_mut_ptr() as *mut c_void, src as *const c_void, len);

    let mut msg0 = softaes_block_load(pad.as_ptr());
    let mut msg1 = softaes_block_load(pad.as_ptr().add(16));
    msg0 = softaes_block_xor(msg0, *state.add(6));
    msg0 = softaes_block_xor(msg0, *state.add(1));
    msg1 = softaes_block_xor(msg1, *state.add(5));
    msg1 = softaes_block_xor(msg1, *state.add(2));
    msg0 = softaes_block_xor(msg0, softaes_block_and(*state.add(2), *state.add(3)));
    msg1 = softaes_block_xor(msg1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(pad.as_mut_ptr(), msg0);
    softaes_block_store(pad.as_mut_ptr().add(16), msg1);

    memset(pad.as_mut_ptr().add(len) as *mut c_void, 0, 32 - len);
    memcpy(dst as *mut c_void, pad.as_ptr() as *const c_void, len);

    msg0 = softaes_block_load(pad.as_ptr());
    msg1 = softaes_block_load(pad.as_ptr().add(16));

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
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 8];
    let mut src = [0u8; 32];
    let mut dst = [0u8; 32];
    let mut i: usize;

    aegis128l_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 32 * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), state.as_mut_ptr());
        i += 32 * 2;
    }
    while i + 32 <= adlen {
        aegis128l_absorb(ad.add(i), state.as_mut_ptr());
        i += 32;
    }
    if adlen % 32 != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, 32);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % 32);
        aegis128l_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    i = 0;
    while i + 32 <= mlen {
        aegis128l_enc(c.add(i), m.add(i), state.as_mut_ptr());
        i += 32;
    }
    if mlen % 32 != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, 32);
        memcpy(src.as_mut_ptr() as *mut c_void, m.add(i) as *const c_void, mlen % 32);
        aegis128l_enc(dst.as_mut_ptr(), src.as_ptr(), state.as_mut_ptr());
        memcpy(c.add(i) as *mut c_void, dst.as_ptr() as *const c_void, mlen % 32);
    }

    aegis128l_mac(mac, maclen, adlen as u64, mlen as u64, state.as_mut_ptr())
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
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 8];
    let mut src = [0u8; 32];
    let mut dst = [0u8; 32];
    let mut computed_mac = [0u8; 32];
    let mlen = clen;
    let mut i: usize;
    let mut ret: c_int;

    aegis128l_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 32 * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), state.as_mut_ptr());
        i += 32 * 2;
    }
    while i + 32 <= adlen {
        aegis128l_absorb(ad.add(i), state.as_mut_ptr());
        i += 32;
    }
    if adlen % 32 != 0 {
        memset(src.as_mut_ptr() as *mut c_void, 0, 32);
        memcpy(src.as_mut_ptr() as *mut c_void, ad.add(i) as *const c_void, adlen % 32);
        aegis128l_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    if !m.is_null() {
        i = 0;
        while i + 32 <= mlen {
            aegis128l_dec(m.add(i), c.add(i), state.as_mut_ptr());
            i += 32;
        }
    } else {
        i = 0;
        while i + 32 <= mlen {
            aegis128l_dec(dst.as_mut_ptr(), c.add(i), state.as_mut_ptr());
            i += 32;
        }
    }
    if mlen % 32 != 0 {
        if !m.is_null() {
            aegis128l_declast(m.add(i), c.add(i), mlen % 32, state.as_mut_ptr());
        } else {
            aegis128l_declast(dst.as_mut_ptr(), c.add(i), mlen % 32, state.as_mut_ptr());
        }
    }

    ret = -1;
    if aegis128l_mac(computed_mac.as_mut_ptr(), maclen, adlen as u64, mlen as u64, state.as_mut_ptr()) == 0 {
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
// crypto_aead/aegis128l/implementations.h
// ---------------------------------------------------------------------

#[repr(C)]
pub struct Aegis128LImplementation {
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
pub static aegis128l_soft_implementation: Aegis128LImplementation = Aegis128LImplementation {
    encrypt_detached,
    decrypt_detached,
};

// ---------------------------------------------------------------------
// crypto_aead/aegis128l/aead_aegis128l.c
// ---------------------------------------------------------------------

static mut IMPLEMENTATION: *const Aegis128LImplementation = &aegis128l_soft_implementation;

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_keybytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_nsecbytes() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_npubbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_abytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_messagebytes_max() -> usize {
    let a = SODIUM_SIZE_MAX - 32;
    let b = (1u64 << 61) - 1;
    (if a < b { a } else { b }) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 16);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt(
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

    if mlen > crypto_aead_aegis128l_messagebytes_max() as u64 {
        sodium_misuse();
    }
    ret = crypto_aead_aegis128l_encrypt_detached(
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
pub unsafe extern "C" fn crypto_aead_aegis128l_decrypt(
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
        ret = crypto_aead_aegis128l_decrypt_detached(
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
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt_detached(
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
    if mlen > crypto_aead_aegis128l_messagebytes_max() as u64
        || adlen > crypto_aead_aegis128l_messagebytes_max() as u64
    {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).encrypt_detached)(c, mac, maclen, m, mlen as usize, ad, adlen as usize, npub, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_aegis128l_decrypt_detached(
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
    if clen > crypto_aead_aegis128l_messagebytes_max() as u64
        || adlen > crypto_aead_aegis128l_messagebytes_max() as u64
    {
        return -1;
    }
    ((*IMPLEMENTATION).decrypt_detached)(m, c, clen as usize, mac, maclen, ad, adlen as usize, npub, k)
}

#[no_mangle]
pub unsafe extern "C" fn _crypto_aead_aegis128l_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &aegis128l_soft_implementation;
    0
}
