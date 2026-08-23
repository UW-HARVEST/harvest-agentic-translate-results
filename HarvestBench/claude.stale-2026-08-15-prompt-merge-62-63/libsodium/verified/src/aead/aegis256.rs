//! Translated from crypto_aead/aegis256/{aead_aegis256.c, aegis256_soft.c,
//! aegis256_common.h}. SOFT implementation only.
use crate::aead::softaes::*;
use core::ffi::c_void;

extern "C" {
    fn crypto_verify_16(x: *const u8, y: *const u8) -> i32;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> i32;
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const KEYBYTES: usize = 32;
const NSECBYTES: usize = 0;
const NPUBBYTES: usize = 32;
const ABYTES: usize = 32;
const MESSAGEBYTES_MAX: u64 = (1u64 << 61) - 1;

const AES_BLOCK_LENGTH: usize = 16;
const RATE: usize = 16;

type AesBlock = SoftAesBlock;

#[inline(always)]
unsafe fn aegis256_update(state: &mut [AesBlock; 6], d: AesBlock) {
    let tmp = state[5];
    state[5] = softaes_block_encrypt(state[4], state[5]);
    state[4] = softaes_block_encrypt(state[3], state[4]);
    state[3] = softaes_block_encrypt(state[2], state[3]);
    state[2] = softaes_block_encrypt(state[1], state[2]);
    state[1] = softaes_block_encrypt(state[0], state[1]);
    state[0] = softaes_block_xor(softaes_block_encrypt(tmp, state[0]), d);
}

const C0: [u8; 16] = [
    0x00, 0x01, 0x01, 0x02, 0x03, 0x05, 0x08, 0x0d, 0x15, 0x22, 0x37, 0x59, 0x90, 0xe9, 0x79, 0x62,
];
const C1: [u8; 16] = [
    0xdb, 0x3d, 0x18, 0x55, 0x6d, 0xc2, 0x2f, 0xf1, 0x20, 0x11, 0x31, 0x42, 0x73, 0xb5, 0x28, 0xdd,
];

#[inline]
unsafe fn aegis256_init(key: *const u8, nonce: *const u8, state: &mut [AesBlock; 6]) {
    let c0 = softaes_block_load(C0.as_ptr());
    let c1 = softaes_block_load(C1.as_ptr());
    let k0 = softaes_block_load(key);
    let k1 = softaes_block_load(key.add(AES_BLOCK_LENGTH));
    let n0 = softaes_block_load(nonce);
    let n1 = softaes_block_load(nonce.add(AES_BLOCK_LENGTH));
    let k0_n0 = softaes_block_xor(k0, n0);
    let k1_n1 = softaes_block_xor(k1, n1);

    state[0] = k0_n0;
    state[1] = k1_n1;
    state[2] = c1;
    state[3] = c0;
    state[4] = softaes_block_xor(k0, c0);
    state[5] = softaes_block_xor(k1, c1);
    for _ in 0..4 {
        aegis256_update(state, k0);
        aegis256_update(state, k1);
        aegis256_update(state, k0_n0);
        aegis256_update(state, k1_n1);
    }
}

#[inline]
unsafe fn aegis256_mac(
    mac: *mut u8,
    maclen: usize,
    adlen: u64,
    mlen: u64,
    state: &mut [AesBlock; 6],
) -> i32 {
    let mut tmp = softaes_block_load64x2(mlen << 3, adlen << 3);
    tmp = softaes_block_xor(tmp, state[3]);

    for _ in 0..7 {
        aegis256_update(state, tmp);
    }

    if maclen == 16 {
        tmp = softaes_block_xor(state[5], state[4]);
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[3], state[2]));
        tmp = softaes_block_xor(tmp, softaes_block_xor(state[1], state[0]));
        softaes_block_store(mac, tmp);
    } else if maclen == 32 {
        tmp = softaes_block_xor(softaes_block_xor(state[2], state[1]), state[0]);
        softaes_block_store(mac, tmp);
        tmp = softaes_block_xor(softaes_block_xor(state[5], state[4]), state[3]);
        softaes_block_store(mac.add(16), tmp);
    } else {
        core::ptr::write_bytes(mac, 0, maclen);
        return -1;
    }
    0
}

#[inline]
unsafe fn aegis256_absorb(src: *const u8, state: &mut [AesBlock; 6]) {
    let msg = softaes_block_load(src);
    aegis256_update(state, msg);
}

#[inline]
unsafe fn aegis256_absorb2(src: *const u8, state: &mut [AesBlock; 6]) {
    let msg = softaes_block_load(src.add(0 * AES_BLOCK_LENGTH));
    let msg2 = softaes_block_load(src.add(1 * AES_BLOCK_LENGTH));
    aegis256_update(state, msg);
    aegis256_update(state, msg2);
}

#[inline]
unsafe fn aegis256_enc(dst: *mut u8, src: *const u8, state: &mut [AesBlock; 6]) {
    let msg = softaes_block_load(src);
    let mut tmp = softaes_block_xor(msg, state[5]);
    tmp = softaes_block_xor(tmp, state[4]);
    tmp = softaes_block_xor(tmp, state[1]);
    tmp = softaes_block_xor(tmp, softaes_block_and(state[2], state[3]));
    softaes_block_store(dst, tmp);

    aegis256_update(state, msg);
}

#[inline]
unsafe fn aegis256_dec(dst: *mut u8, src: *const u8, state: &mut [AesBlock; 6]) {
    let mut msg = softaes_block_load(src);
    msg = softaes_block_xor(msg, state[5]);
    msg = softaes_block_xor(msg, state[4]);
    msg = softaes_block_xor(msg, state[1]);
    msg = softaes_block_xor(msg, softaes_block_and(state[2], state[3]));
    softaes_block_store(dst, msg);

    aegis256_update(state, msg);
}

#[inline]
unsafe fn aegis256_declast(dst: *mut u8, src: *const u8, len: usize, state: &mut [AesBlock; 6]) {
    let mut pad = [0u8; RATE];
    core::ptr::copy_nonoverlapping(src, pad.as_mut_ptr(), len);

    let mut msg = softaes_block_load(pad.as_ptr());
    msg = softaes_block_xor(msg, state[5]);
    msg = softaes_block_xor(msg, state[4]);
    msg = softaes_block_xor(msg, state[1]);
    msg = softaes_block_xor(msg, softaes_block_and(state[2], state[3]));
    softaes_block_store(pad.as_mut_ptr(), msg);

    core::ptr::write_bytes(pad.as_mut_ptr().add(len), 0, RATE - len);
    core::ptr::copy_nonoverlapping(pad.as_ptr(), dst, len);

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
) -> i32 {
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 6];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];

    aegis256_init(k, npub, &mut state);

    let mut i: usize = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), &mut state);
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        core::ptr::write_bytes(src.as_mut_ptr(), 0, RATE);
        core::ptr::copy_nonoverlapping(ad.add(i), src.as_mut_ptr(), adlen % RATE);
        aegis256_absorb(src.as_ptr(), &mut state);
    }
    i = 0;
    while i + RATE <= mlen {
        aegis256_enc(c.add(i), m.add(i), &mut state);
        i += RATE;
    }
    if mlen % RATE != 0 {
        core::ptr::write_bytes(src.as_mut_ptr(), 0, RATE);
        core::ptr::copy_nonoverlapping(m.add(i), src.as_mut_ptr(), mlen % RATE);
        aegis256_enc(dst.as_mut_ptr(), src.as_ptr(), &mut state);
        core::ptr::copy_nonoverlapping(dst.as_ptr(), c.add(i), mlen % RATE);
    }

    aegis256_mac(mac, maclen, adlen as u64, mlen as u64, &mut state)
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
    let mut state = [SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 }; 6];
    let mut src = [0u8; RATE];
    let mut dst = [0u8; RATE];
    let mut computed_mac = [0u8; 32];
    let mlen = clen;

    aegis256_init(k, npub, &mut state);

    let mut i: usize = 0;
    while i + 2 * RATE <= adlen {
        aegis256_absorb2(ad.add(i), &mut state);
        i += 2 * RATE;
    }
    while i + RATE <= adlen {
        aegis256_absorb(ad.add(i), &mut state);
        i += RATE;
    }
    if adlen % RATE != 0 {
        core::ptr::write_bytes(src.as_mut_ptr(), 0, RATE);
        core::ptr::copy_nonoverlapping(ad.add(i), src.as_mut_ptr(), adlen % RATE);
        aegis256_absorb(src.as_ptr(), &mut state);
    }
    if !m.is_null() {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(m.add(i), c.add(i), &mut state);
            i += RATE;
        }
    } else {
        i = 0;
        while i + RATE <= mlen {
            aegis256_dec(dst.as_mut_ptr(), c.add(i), &mut state);
            i += RATE;
        }
    }
    if mlen % RATE != 0 {
        if !m.is_null() {
            aegis256_declast(m.add(i), c.add(i), mlen % RATE, &mut state);
        } else {
            aegis256_declast(dst.as_mut_ptr(), c.add(i), mlen % RATE, &mut state);
        }
    }

    let mut ret: i32 = -1;
    if aegis256_mac(computed_mac.as_mut_ptr(), maclen, adlen as u64, mlen as u64, &mut state) == 0 {
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
pub struct aegis256_implementation {
    pub encrypt_detached: EncryptDetachedFn,
    pub decrypt_detached: DecryptDetachedFn,
}

unsafe impl Sync for aegis256_implementation {}

#[unsafe(no_mangle)]
pub static aegis256_soft_implementation: aegis256_implementation = aegis256_implementation {
    encrypt_detached,
    decrypt_detached,
};

static mut IMPLEMENTATION: *const aegis256_implementation = &aegis256_soft_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_nsecbytes() -> usize {
    NSECBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_npubbytes() -> usize {
    NPUBBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_abytes() -> usize {
    ABYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aegis256_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

#[unsafe(no_mangle)]
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
) -> i32 {
    let mut clen: u64 = 0;
    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    let ret = crypto_aead_aegis256_encrypt_detached(
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
) -> i32 {
    let mut mlen: u64 = 0;
    let mut ret: i32 = -1;
    if clen >= ABYTES as u64 {
        ret = crypto_aead_aegis256_decrypt_detached(
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
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt_detached(
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
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt_detached(
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
pub unsafe extern "C" fn _crypto_aead_aegis256_pick_best_implementation() -> i32 {
    IMPLEMENTATION = &aegis256_soft_implementation;
    0
}
