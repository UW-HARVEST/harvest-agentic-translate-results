//! Translation of `crypto_aead/aegis256/aead_aegis256.c` plus the soft
//! implementation (`aegis256_soft.c` + `aegis256_common.h` template) and the
//! vtable declared in `aegis256/implementations.h`.
//!
//! Build config facts: no `HAVE_*` macros -> only the soft implementation
//! exists, and `_crypto_aead_aegis256_pick_best_implementation()` always
//! selects it. The AES-block macros expand to the softaes helpers.

use crate::common::{memcpy, memset};
use crate::crypto_core::softaes::{self, SoftAesBlock};
use crate::crypto_verify::{crypto_verify_16, crypto_verify_32};
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;

// ---- constants from include/sodium/crypto_aead_aegis256.h ----
pub const crypto_aead_aegis256_KEYBYTES: usize = 32;
pub const crypto_aead_aegis256_NSECBYTES: usize = 0;
pub const crypto_aead_aegis256_NPUBBYTES: usize = 32;
pub const crypto_aead_aegis256_ABYTES: usize = 32;
// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, (1ULL << 61) - 1)
pub const crypto_aead_aegis256_MESSAGEBYTES_MAX: usize = {
    let a = usize::MAX - crypto_aead_aegis256_ABYTES;
    let b = ((1u64 << 61) - 1) as usize;
    if a < b {
        a
    } else {
        b
    }
};

// ---------------------------------------------------------------------------
// implementations.h : the vtable type.
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct aegis256_implementation {
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

unsafe impl Sync for aegis256_implementation {}

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

#[inline(always)]
fn aes_enc(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    softaes::_sodium_softaes_block_encrypt(a, b)
}

// ---------------------------------------------------------------------------
// aegis256_soft.c: aegis256_update
// ---------------------------------------------------------------------------
#[inline(always)]
unsafe fn aegis256_update(state: *mut SoftAesBlock, d: SoftAesBlock) {
    let tmp = *state.add(5);
    *state.add(5) = aes_enc(*state.add(4), *state.add(5));
    *state.add(4) = aes_enc(*state.add(3), *state.add(4));
    *state.add(3) = aes_enc(*state.add(2), *state.add(3));
    *state.add(2) = aes_enc(*state.add(1), *state.add(2));
    *state.add(1) = aes_enc(*state.add(0), *state.add(1));
    *state.add(0) = softaes_block_xor(aes_enc(tmp, *state.add(0)), d);
}

// ---------------------------------------------------------------------------
// aegis256_common.h template (RATE = 16).
// ---------------------------------------------------------------------------
const RATE: usize = 16;

#[inline(always)]
unsafe fn aegis256_init(key: *const u8, nonce: *const u8, state: *mut SoftAesBlock) {
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
    let k0 = softaes_block_load(key);
    let k1 = softaes_block_load(key.add(AES_BLOCK_LENGTH));
    let n0 = softaes_block_load(nonce);
    let n1 = softaes_block_load(nonce.add(AES_BLOCK_LENGTH));
    let k0_n0 = softaes_block_xor(k0, n0);
    let k1_n1 = softaes_block_xor(k1, n1);
    let mut i: i32;

    *state.add(0) = k0_n0;
    *state.add(1) = k1_n1;
    *state.add(2) = c1;
    *state.add(3) = c0;
    *state.add(4) = softaes_block_xor(k0, c0);
    *state.add(5) = softaes_block_xor(k1, c1);
    i = 0;
    while i < 4 {
        aegis256_update(state, k0);
        aegis256_update(state, k1);
        aegis256_update(state, k0_n0);
        aegis256_update(state, k1_n1);
        i += 1;
    }
}

#[inline(always)]
unsafe fn aegis256_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: *mut SoftAesBlock,
) -> i32 {
    let mut tmp: SoftAesBlock;
    let mut i: i32;

    tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, *state.add(3));

    i = 0;
    while i < 7 {
        aegis256_update(state, tmp);
        i += 1;
    }

    if maclen == 16 {
        tmp = softaes_block_xor(*state.add(5), *state.add(4));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(3), *state.add(2)));
        tmp = softaes_block_xor(tmp, softaes_block_xor(*state.add(1), *state.add(0)));
        softaes_block_store(mac, tmp);
    } else if maclen == 32 {
        tmp = softaes_block_xor(
            softaes_block_xor(*state.add(2), *state.add(1)),
            *state.add(0),
        );
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(
            softaes_block_xor(*state.add(5), *state.add(4)),
            *state.add(3),
        );
        softaes_block_store(mac.add(16), tmp);
    } else {
        memset(mac, 0, maclen);
        return -1;
    }
    0
}

#[inline(always)]
unsafe fn aegis256_absorb(src: *const u8, state: *mut SoftAesBlock) {
    let msg = softaes_block_load(src);
    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_absorb2(src: *const u8, state: *mut SoftAesBlock) {
    let msg = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    let msg2 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    aegis256_update(state, msg);
    aegis256_update(state, msg2);
}

#[inline(always)]
unsafe fn aegis256_enc(dst: *mut u8, src: *const u8, state: *mut SoftAesBlock) {
    let msg = softaes_block_load(src);
    let mut tmp: SoftAesBlock;
    tmp = softaes_block_xor(msg, *state.add(5));
    tmp = softaes_block_xor(tmp, *state.add(4));
    tmp = softaes_block_xor(tmp, *state.add(1));
    tmp = softaes_block_xor(tmp, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(dst, tmp);

    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_dec(dst: *mut u8, src: *const u8, state: *mut SoftAesBlock) {
    let mut msg = softaes_block_load(src);
    msg = softaes_block_xor(msg, *state.add(5));
    msg = softaes_block_xor(msg, *state.add(4));
    msg = softaes_block_xor(msg, *state.add(1));
    msg = softaes_block_xor(msg, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(dst, msg);

    aegis256_update(state, msg);
}

#[inline(always)]
unsafe fn aegis256_declast(dst: *mut u8, src: *const u8, len: usize, state: *mut SoftAesBlock) {
    let mut pad = [0u8; RATE];
    let mut msg: SoftAesBlock;

    memset(pad.as_mut_ptr(), 0, pad.len());
    memcpy(pad.as_mut_ptr(), src, len);

    msg = softaes_block_load(pad.as_ptr());
    msg = softaes_block_xor(msg, *state.add(5));
    msg = softaes_block_xor(msg, *state.add(4));
    msg = softaes_block_xor(msg, *state.add(1));
    msg = softaes_block_xor(msg, softaes_block_and(*state.add(2), *state.add(3)));
    softaes_block_store(pad.as_mut_ptr(), msg);

    memset(pad.as_mut_ptr().add(len), 0, pad.len() - len);
    memcpy(dst, pad.as_ptr(), len);

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
) -> core::ffi::c_int {
    let mut state = [SoftAesBlock {
        w0: 0,
        w1: 0,
        w2: 0,
        w3: 0,
    }; 6];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];
    let mut i: usize;

    aegis256_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), state.as_mut_ptr());
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis256_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    i = 0;
    while i + RATE <= mlen {
        aegis256_enc(c.add(i), m.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if mlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), m.add(i), mlen % RATE);
        aegis256_enc(dst.as_mut_ptr(), src.as_ptr(), state.as_mut_ptr());
        memcpy(c.add(i), dst.as_ptr(), mlen % RATE);
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
) -> core::ffi::c_int {
    let mut state = [SoftAesBlock {
        w0: 0,
        w1: 0,
        w2: 0,
        w3: 0,
    }; 6];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];
    let mut computed_mac = [0u8; 32];
    let mlen = clen;
    let mut i: usize;
    let mut ret: i32;

    aegis256_init(k, npub, state.as_mut_ptr());

    i = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), state.as_mut_ptr());
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), state.as_mut_ptr());
        i += RATE;
    }
    if adlen % RATE != 0 {
        memset(src.as_mut_ptr(), 0, RATE);
        memcpy(src.as_mut_ptr(), ad.add(i), adlen % RATE);
        aegis256_absorb(src.as_ptr(), state.as_mut_ptr());
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(m.add(i), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(dst.as_mut_ptr(), c.add(i), state.as_mut_ptr());
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis256_declast(m.add(i), c.add(i), mlen % RATE, state.as_mut_ptr());
        } else {
            aegis256_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, state.as_mut_ptr());
        }
    }

    ret = -1;
    if aegis256_mac(
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
// exported DATA symbol: aegis256_soft_implementation
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub static aegis256_soft_implementation: aegis256_implementation = aegis256_implementation {
    encrypt_detached,
    decrypt_detached,
};

// ---------------------------------------------------------------------------
// aead_aegis256.c
// ---------------------------------------------------------------------------
static mut implementation: *const aegis256_implementation = &aegis256_soft_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_keybytes() -> usize {
    crypto_aead_aegis256_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_nsecbytes() -> usize {
    crypto_aead_aegis256_NSECBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_npubbytes() -> usize {
    crypto_aead_aegis256_NPUBBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_abytes() -> usize {
    crypto_aead_aegis256_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_messagebytes_max() -> usize {
    crypto_aead_aegis256_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_aead_aegis256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt(
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

    if mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX as core::ffi::c_ulonglong {
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
            clen = mlen + crypto_aead_aegis256_ABYTES as core::ffi::c_ulonglong;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt(
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

    if clen >= crypto_aead_aegis256_ABYTES as core::ffi::c_ulonglong {
        ret = crypto_aead_aegis256_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_aegis256_ABYTES as core::ffi::c_ulonglong,
            c.add((clen - crypto_aead_aegis256_ABYTES as core::ffi::c_ulonglong) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_aegis256_ABYTES as core::ffi::c_ulonglong;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt_detached(
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
    let maclen: usize = crypto_aead_aegis256_ABYTES;

    let _ = nsec;
    if !maclen_p.is_null() {
        *maclen_p = maclen as core::ffi::c_ulonglong;
    }
    if mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
        || adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
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
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt_detached(
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
    let maclen: usize = crypto_aead_aegis256_ABYTES;

    let _ = nsec;
    if clen > crypto_aead_aegis256_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
        || adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX as core::ffi::c_ulonglong
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
pub unsafe extern "C" fn _crypto_aead_aegis256_pick_best_implementation() -> core::ffi::c_int {
    implementation = &aegis256_soft_implementation;
    0
}
