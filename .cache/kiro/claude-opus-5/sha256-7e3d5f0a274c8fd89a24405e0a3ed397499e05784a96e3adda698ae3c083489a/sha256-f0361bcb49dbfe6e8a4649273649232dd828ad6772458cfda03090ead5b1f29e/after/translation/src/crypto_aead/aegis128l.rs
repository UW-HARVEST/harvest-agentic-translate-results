//! Translation of `crypto_aead/aegis128l/aead_aegis128l.c` plus the soft
//! implementation (`aegis128l_soft.c` + `aegis128l_common.h` template) and the
//! vtable declared in `aegis128l/implementations.h`.
//!
//! Build config facts: no `HAVE_*` macros -> only the soft implementation
//! exists, and `_crypto_aead_aegis128l_pick_best_implementation()` always
//! selects it. The AES-block macros expand to the softaes helpers.

use crate::common::{load64_le, memcpy, memset, store64_le};
use crate::crypto_core::softaes::{self, SoftAesBlock};
use crate::crypto_verify::{crypto_verify_16, crypto_verify_32};
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;

// ---- constants from include/sodium/crypto_aead_aegis128l.h ----
pub const crypto_aead_aegis128l_KEYBYTES: usize = 16;
pub const crypto_aead_aegis128l_NSECBYTES: usize = 0;
pub const crypto_aead_aegis128l_NPUBBYTES: usize = 16;
pub const crypto_aead_aegis128l_ABYTES: usize = 32;
// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, (1ULL << 61) - 1)
pub const crypto_aead_aegis128l_MESSAGEBYTES_MAX: usize = {
    let a = usize::MAX - crypto_aead_aegis128l_ABYTES;
    let b = (1u64 << 61) - 1;
    let b = b as usize;
    if a < b { a } else { b }
};

// ---------------------------------------------------------------------------
// implementations.h : the vtable type.
// ---------------------------------------------------------------------------
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
    ) -> core::ffi::c_int,
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
    ) -> core::ffi::c_int,
}

unsafe impl Sync for aegis128l_implementation {}

// ---------------------------------------------------------------------------
// softaes helper wrappers (static inline functions in private/softaes.h).
// ---------------------------------------------------------------------------
const AES_BLOCK_LENGTH: usize = 16;

#[inline(always)]
unsafe fn softaes_block_load(inp: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: crate::common::load32_le(inp.add(0)),
        w1: crate::common::load32_le(inp.add(4)),
        w2: crate::common::load32_le(inp.add(8)),
        w3: crate::common::load32_le(inp.add(12)),
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
unsafe fn softaes_block_store(out: *mut u8, inp: SoftAesBlock) {
    crate::common::store32_le(out.add(0), inp.w0);
    crate::common::store32_le(out.add(4), inp.w1);
    crate::common::store32_le(out.add(8), inp.w2);
    crate::common::store32_le(out.add(12), inp.w3);
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

// AES_ENC(A, B) = softaes_block_encrypt(A, B) -> _sodium_softaes_block_encrypt
#[inline(always)]
fn aes_enc(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    softaes::_sodium_softaes_block_encrypt(a, b)
}

// ---------------------------------------------------------------------------
// aegis128l_soft.c: aegis128l_update
// ---------------------------------------------------------------------------
#[inline(always)]
unsafe fn aegis128l_update(state: *mut SoftAesBlock, d1: SoftAesBlock, d2: SoftAesBlock) {
    let tmp = *state.add(7);
    *state.add(7) = aes_enc(*state.add(6), *state.add(7));
    *state.add(6) = aes_enc(*state.add(5), *state.add(6));
    *state.add(5) = aes_enc(*state.add(4), *state.add(5));
    *state.add(4) = aes_enc(*state.add(3), *state.add(4));
    *state.add(3) = aes_enc(*state.add(2), *state.add(3));
    *state.add(2) = aes_enc(*state.add(1), *state.add(2));
    *state.add(1) = aes_enc(*state.add(0), *state.add(1));
    *state.add(0) = aes_enc(tmp, *state.add(0));

    *state.add(0) = softaes_block_xor(*state.add(0), d1);
    *state.add(4) = softaes_block_xor(*state.add(4), d2);
}

// ---------------------------------------------------------------------------
// aegis128l_common.h template (RATE = 32).
// ---------------------------------------------------------------------------
const RATE: usize = 32;

#[inline(always)]
unsafe fn aegis128l_init(key: *const u8, nonce: *const u8, state: *mut SoftAesBlock) {
    static C0_: [u8; AES_BLOCK_LENGTH] = [
        0x00, 0x01, 0x01, 0x02, 0x03, 0x05, 0x08, 0x0d, 0x15, 0x22, 0x37, 0x59, 0x90, 0xe9, 0x79,
        0x62,
    ];
    static C1_: [u8; AES_BLOCK_LENGTH] = [
        0xdb, 0x3d, 0x18, 0x55, 0x6d, 0xc2, 0x2f, 0xf1, 0x20, 0x11, 0x31, 0x42, 0x73, 0xb5, 0x28,
        0xdd,
    ];

    let c0 = softaes_block_load(C0_.as_ptr());
    let c1 = softaes_block_load(C1_.as_ptr());
    let k: SoftAesBlock;
    let n: SoftAesBlock;
    let mut i: i32;

    k = softaes_block_load(key);
    n = softaes_block_load(nonce);

    *state.add(0) = softaes_block_xor(k, n);
    *state.add(1) = c1;
    *state.add(2) = c0;
    *state.add(3) = c1;
    *state.add(4) = softaes_block_xor(k, n);
    *state.add(5) = softaes_block_xor(k, c0);
    *state.add(6) = softaes_block_xor(k, c1);
    *state.add(7) = softaes_block_xor(k, c0);
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
    state: *mut SoftAesBlock,
) -> i32 {
    let mut tmp: SoftAesBlock;
    let mut i: i32;

    tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, *state.add(2));

    i = 0;
    while i < 7 {
        aegis128l_update(state, tmp, tmp);
        i += 1;
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
        memset(mac, 0, maclen);
        return -1;
    }
    0
}

#[inline(always)]
unsafe fn aegis128l_absorb(src: *const u8, state: *mut SoftAesBlock) {
    let msg0 = softaes_block_load(src);
    let msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_absorb2(src: *const u8, state: *mut SoftAesBlock) {
    let msg0 = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    let msg1 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    let msg2 = softaes_block_load(src.add(2 * AES_BLOCK_LENGTH));
    let msg3 = softaes_block_load(src.add(3 * AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
    aegis128l_update(state, msg2, msg3);
}

#[inline(always)]
unsafe fn aegis128l_enc(dst: *mut u8, src: *const u8, state: *mut SoftAesBlock) {
    let msg0 = softaes_block_load(src);
    let msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    let mut tmp0: SoftAesBlock;
    let mut tmp1: SoftAesBlock;
    tmp0 = softaes_block_xor(msg0, *state.add(6));
    tmp0 = softaes_block_xor(tmp0, *state.add(1));
    tmp1 = softaes_block_xor(msg1, *state.add(5));
    tmp1 = softaes_block_xor(tmp1, *state.add(2));
    tmp0 = softaes_block_xor(tmp0, softaes_block_and(*state.add(2), *state.add(3)));
    tmp1 = softaes_block_xor(tmp1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(dst, tmp0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), tmp1);

    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_dec(dst: *mut u8, src: *const u8, state: *mut SoftAesBlock) {
    let mut msg0 = softaes_block_load(src);
    let mut msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, *state.add(6));
    msg0 = softaes_block_xor(msg0, *state.add(1));
    msg1 = softaes_block_xor(msg1, *state.add(5));
    msg1 = softaes_block_xor(msg1, *state.add(2));
    msg0 = softaes_block_xor(msg0, softaes_block_and(*state.add(2), *state.add(3)));
    msg1 = softaes_block_xor(msg1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(dst, msg0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), msg1);

    aegis128l_update(state, msg0, msg1);
}

#[inline(always)]
unsafe fn aegis128l_declast(dst: *mut u8, src: *const u8, len: usize, state: *mut SoftAesBlock) {
    let mut pad = [0u8; RATE];
    let mut msg0: SoftAesBlock;
    let mut msg1: SoftAesBlock;

    memset(pad.as_mut_ptr(), 0, pad.len());
    memcpy(pad.as_mut_ptr(), src, len);

    msg0 = softaes_block_load(pad.as_ptr());
    msg1 = softaes_block_load(pad.as_ptr().add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, *state.add(6));
    msg0 = softaes_block_xor(msg0, *state.add(1));
    msg1 = softaes_block_xor(msg1, *state.add(5));
    msg1 = softaes_block_xor(msg1, *state.add(2));
    msg0 = softaes_block_xor(msg0, softaes_block_and(*state.add(2), *state.add(3)));
    msg1 = softaes_block_xor(msg1, softaes_block_and(*state.add(6), *state.add(7)));
    softaes_block_store(pad.as_mut_ptr(), msg0);
    softaes_block_store(pad.as_mut_ptr().add(AES_BLOCK_LENGTH), msg1);

    memset(pad.as_mut_ptr().add(len), 0, pad.len() - len);
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
) -> core::ffi::c_int {
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 8];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];
    let mut i: usize;

    aegis128l_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), state.as_mut_ptr());
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis128l_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    i = 0;
    while i + RATE <= mlen {
        aegis128l_enc(c.add(i), m.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if mlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), m.add(i), mlen % RATE);
        aegis128l_enc(dst.as_mut_ptr(), src.as_ptr(), state.as_mut_ptr());
        memcpy(c.add(i), dst.as_ptr(), mlen % RATE);
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
) -> core::ffi::c_int {
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 8];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];
    let mut computed_mac = [0u8; 32];
    let mlen = clen;
    let mut i: usize;
    let mut ret: i32;

    aegis128l_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), state.as_mut_ptr());
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis128l_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(m.add(i), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(dst.as_mut_ptr(), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis128l_declast(m.add(i), c.add(i), mlen % RATE, state.as_mut_ptr());
        } else {
            aegis128l_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, state.as_mut_ptr());
        }
    }

    ret = -1;
    if aegis128l_mac(
        computed_mac.as_mut_ptr(),
        maclen,
        adlen as u64,
        mlen as u64,
        state.as_mut_ptr(),
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
    core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
    0
}

// ---------------------------------------------------------------------------
// exported DATA symbol: aegis128l_soft_implementation
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub static aegis128l_soft_implementation: aegis128l_implementation = aegis128l_implementation {
    encrypt_detached,
    decrypt_detached,
};

// ---------------------------------------------------------------------------
// aead_aegis128l.c
// ---------------------------------------------------------------------------
static mut implementation: *const aegis128l_implementation = &aegis128l_soft_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_keybytes() -> usize {
    crypto_aead_aegis128l_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_nsecbytes() -> usize {
    crypto_aead_aegis128l_NSECBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_npubbytes() -> usize {
    crypto_aead_aegis128l_NPUBBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_abytes() -> usize {
    crypto_aead_aegis128l_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_messagebytes_max() -> usize {
    crypto_aead_aegis128l_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_aead_aegis128l_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt(
    c: *mut u8,
    clen_p: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    let mut clen: core::ffi::c_ulonglong = 0;
    let ret: core::ffi::c_int;

    if mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX as core::ffi::c_ulonglong {
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
            clen = mlen + crypto_aead_aegis128l_ABYTES as core::ffi::c_ulonglong;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_decrypt(
    m: *mut u8,
    mlen_p: *mut core::ffi::c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    let mut mlen: core::ffi::c_ulonglong = 0;
    let mut ret: core::ffi::c_int = -1;

    if clen >= crypto_aead_aegis128l_ABYTES as core::ffi::c_ulonglong {
        ret = crypto_aead_aegis128l_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_aegis128l_ABYTES as core::ffi::c_ulonglong,
            c.add((clen - crypto_aead_aegis128l_ABYTES as core::ffi::c_ulonglong) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_aegis128l_ABYTES as core::ffi::c_ulonglong;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    let maclen: usize = crypto_aead_aegis128l_ABYTES;

    let _ = nsec;
    if !maclen_p.is_null() {
        *maclen_p = maclen as core::ffi::c_ulonglong;
    }
    if mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
        || adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
    {
        sodium_misuse();
    }
    ((*implementation).encrypt_detached)(
        c,
        mac,
        maclen,
        m,
        mlen as usize,
        ad,
        adlen as usize,
        npub,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: core::ffi::c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    let maclen: usize = crypto_aead_aegis128l_ABYTES;

    let _ = nsec;
    if clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
        || adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
    {
        return -1;
    }
    ((*implementation).decrypt_detached)(
        m,
        c,
        clen as usize,
        mac,
        maclen,
        ad,
        adlen as usize,
        npub,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_aead_aegis128l_pick_best_implementation() -> core::ffi::c_int {
    implementation = &aegis128l_soft_implementation;
    0
}
