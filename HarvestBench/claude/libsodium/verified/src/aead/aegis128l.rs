//! Translated from crypto_aead/aegis128l/{aead_aegis128l.c, aegis128l_soft.c,
//! aegis128l_common.h}. SOFT implementation only.
use crate::aead::softaes::*;
use core::ffi::c_void;

extern "C" {
    fn crypto_verify_16(x: *const u8, y: *const u8) -> i32;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> i32;
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const KEYBYTES: usize = 16;
const NSECBYTES: usize = 0;
const NPUBBYTES: usize = 16;
const ABYTES: usize = 32;
// min(SODIUM_SIZE_MAX - ABYTES, (1<<61)-1)
const MESSAGEBYTES_MAX: u64 = (1u64 << 61) - 1;

const AES_BLOCK_LENGTH: usize = 16;
const RATE: usize = 32;

type AesBlock = SoftAesBlock;

#[inline(always)]
unsafe fn aegis128l_update(state: &mut [AesBlock; 8], d1: AesBlock, d2: AesBlock) {
    let tmp = state[7];
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

const C0: [u8; 16] = [
    0x00, 0x01, 0x01, 0x02, 0x03, 0x05, 0x08, 0x0d, 0x15, 0x22, 0x37, 0x59, 0x90, 0xe9, 0x79, 0x62,
];
const C1: [u8; 16] = [
    0xdb, 0x3d, 0x18, 0x55, 0x6d, 0xc2, 0x2f, 0xf1, 0x20, 0x11, 0x31, 0x42, 0x73, 0xb5, 0x28, 0xdd,
];

#[inline]
unsafe fn aegis128l_init(key: *const u8, nonce: *const u8, state: &mut [AesBlock; 8]) {
    let c0 = softaes_block_load(C0.as_ptr());
    let c1 = softaes_block_load(C1.as_ptr());
    let k = softaes_block_load(key);
    let n = softaes_block_load(nonce);

    state[0] = softaes_block_xor(k, n);
    state[1] = c1;
    state[2] = c0;
    state[3] = c1;
    state[4] = softaes_block_xor(k, n);
    state[5] = softaes_block_xor(k, c0);
    state[6] = softaes_block_xor(k, c1);
    state[7] = softaes_block_xor(k, c0);
    for _ in 0..10 {
        aegis128l_update(state, n, k);
    }
}

#[inline]
unsafe fn aegis128l_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: &mut [AesBlock; 8],
) -> i32 {
    let mut tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, state[2]);

    for _ in 0..7 {
        aegis128l_update(state, tmp, tmp);
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
        core::ptr::write_bytes(mac, 0, maclen);
        return -1;
    }
    0
}

#[inline]
unsafe fn aegis128l_absorb(src: *const u8, state: &mut [AesBlock; 8]) {
    let msg0 = softaes_block_load(src);
    let msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
}

#[inline]
unsafe fn aegis128l_absorb2(src: *const u8, state: &mut [AesBlock; 8]) {
    let msg0 = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    let msg1 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    let msg2 = softaes_block_load(src.add(2 * AES_BLOCK_LENGTH));
    let msg3 = softaes_block_load(src.add(3 * AES_BLOCK_LENGTH));
    aegis128l_update(state, msg0, msg1);
    aegis128l_update(state, msg2, msg3);
}

#[inline]
unsafe fn aegis128l_enc(dst: *mut u8, src: *const u8, state: &mut [AesBlock; 8]) {
    let msg0 = softaes_block_load(src);
    let msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
    let mut tmp0 = softaes_block_xor(msg0, state[6]);
    tmp0 = softaes_block_xor(tmp0, state[1]);
    let mut tmp1 = softaes_block_xor(msg1, state[5]);
    tmp1 = softaes_block_xor(tmp1, state[2]);
    tmp0 = softaes_block_xor(tmp0, softaes_block_and(state[2], state[3]));
    tmp1 = softaes_block_xor(tmp1, softaes_block_and(state[6], state[7]));
    softaes_block_store(dst, tmp0);
    softaes_block_store(dst.add(AES_BLOCK_LENGTH), tmp1);

    aegis128l_update(state, msg0, msg1);
}

#[inline]
unsafe fn aegis128l_dec(dst: *mut u8, src: *const u8, state: &mut [AesBlock; 8]) {
    let mut msg0 = softaes_block_load(src);
    let mut msg1 = softaes_block_load(src.add(AES_BLOCK_LENGTH));
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

#[inline]
unsafe fn aegis128l_declast(dst: *mut u8, src: *const u8, len: usize, state: &mut [AesBlock; 8]) {
    let mut pad = [0u8; RATE];
    core::ptr::copy_nonoverlapping(src, pad.as_mut_ptr(), len);

    let mut msg0 = softaes_block_load(pad.as_ptr());
    let mut msg1 = softaes_block_load(pad.as_ptr().add(AES_BLOCK_LENGTH));
    msg0 = softaes_block_xor(msg0, state[6]);
    msg0 = softaes_block_xor(msg0, state[1]);
    msg1 = softaes_block_xor(msg1, state[5]);
    msg1 = softaes_block_xor(msg1, state[2]);
    msg0 = softaes_block_xor(msg0, softaes_block_and(state[2], state[3]));
    msg1 = softaes_block_xor(msg1, softaes_block_and(state[6], state[7]));
    softaes_block_store(pad.as_mut_ptr(), msg0);
    softaes_block_store(pad.as_mut_ptr().add(AES_BLOCK_LENGTH), msg1);

    core::ptr::write_bytes(pad.as_mut_ptr().add(len), 0, RATE - len);
    core::ptr::copy_nonoverlapping(pad.as_ptr(), dst, len);

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
) -> i32 {
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 8];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];

    aegis128l_init(k, npub, &mut state);

    let mut i: usize = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), &mut state);
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        core::ptr::write_bytes(src.as_mut_ptr(), 0, RATE);
        core::ptr::copy_nonoverlapping(ad.add(i), src.as_mut_ptr(), adlen % RATE);
        aegis128l_absorb(src.as_ptr(), &mut state);
    }
    i = 0;
    while i + RATE <= mlen {
        aegis128l_enc(c.add(i), m.add(i), &mut state);
        i += RATE;
    }
    if mlen % RATE != 0 {
        core::ptr::write_bytes(src.as_mut_ptr(), 0, RATE);
        core::ptr::copy_nonoverlapping(m.add(i), src.as_mut_ptr(), mlen % RATE);
        aegis128l_enc(dst.as_mut_ptr(), src.as_ptr(), &mut state);
        core::ptr::copy_nonoverlapping(dst.as_ptr(), c.add(i), mlen % RATE);
    }

    aegis128l_mac(mac, maclen, adlen as u64, mlen as u64, &mut state)
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
) -> i32 {
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 8];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];
    let mut computed_mac = [0u8; 32];
    let mlen = clen;

    aegis128l_init(k, npub, &mut state);

    let mut i: usize = 0;
    while i + RATE * 2 <= adlen {
        aegis128l_absorb2(ad.add(i), &mut state);
        i += RATE * 2;
    }
    while i + RATE <= adlen {
        aegis128l_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        core::ptr::write_bytes(src.as_mut_ptr(), 0, RATE);
        core::ptr::copy_nonoverlapping(ad.add(i), src.as_mut_ptr(), adlen % RATE);
        aegis128l_absorb(src.as_ptr(), &mut state);
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(m.add(i), c.add(i), &mut state);
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis128l_dec(dst.as_mut_ptr(), c.add(i), &mut state);
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis128l_declast(m.add(i), c.add(i), mlen % RATE, &mut state);
        } else {
            aegis128l_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, &mut state);
        }
    }

    let mut ret: i32 = -1;
    if aegis128l_mac(computed_mac.as_mut_ptr(), maclen, adlen as u64, mlen as u64, &mut state) == 0 {
        if maclen == 16 {
            ret = crypto_verify_16(computed_mac.as_ptr(), mac);
        } else if maclen == 32 {
            ret = crypto_verify_32(computed_mac.as_ptr(), mac);
        }
    }
    if ret != 0 {
        if !m.is_null() {
            core::ptr::write_bytes(m, 0, mlen);
        }
        return ret;
    }
    0
}

// ---- implementation struct + dispatch ----

type EncryptDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> i32;
type DecryptDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> i32;

#[repr(C)]
pub struct aegis128l_implementation {
    pub encrypt_detached: EncryptDetachedFn,
    pub decrypt_detached: DecryptDetachedFn,
}

unsafe impl Sync for aegis128l_implementation {}

#[unsafe(no_mangle)]
pub static aegis128l_soft_implementation: aegis128l_implementation = aegis128l_implementation {
    encrypt_detached,
    decrypt_detached,
};

static mut IMPLEMENTATION: *const aegis128l_implementation = &aegis128l_soft_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_nsecbytes() -> usize {
    NSECBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_npubbytes() -> usize {
    NPUBBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_abytes() -> usize {
    ABYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis128l_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

#[unsafe(no_mangle)]
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
) -> i32 {
    let mut clen: u64 = 0;
    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    let ret = crypto_aead_aegis128l_encrypt_detached(
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
            clen = mlen + ABYTES as u64;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
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
) -> i32 {
    let mut mlen: u64 = 0;
    let mut ret: i32 = -1;
    if clen >= ABYTES as u64 {
        ret = crypto_aead_aegis128l_decrypt_detached(
            m,
            nsec,
            c,
            clen - ABYTES as u64,
            c.add((clen - ABYTES as u64) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - ABYTES as u64;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    _nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> i32 {
    let maclen: usize = ABYTES;
    if !maclen_p.is_null() {
        *maclen_p = maclen as u64;
    }
    if mlen > MESSAGEBYTES_MAX || adlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).encrypt_detached)(
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
    _nsec: *mut u8,
    c: *const u8,
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> i32 {
    let maclen: usize = ABYTES;
    if clen > MESSAGEBYTES_MAX || adlen > MESSAGEBYTES_MAX {
        return -1;
    }
    ((*IMPLEMENTATION).decrypt_detached)(
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
pub unsafe extern "C" fn _crypto_aead_aegis128l_pick_best_implementation() -> i32 {
    IMPLEMENTATION = &aegis128l_soft_implementation;
    0
}
