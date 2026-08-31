//! Translation of c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c

use core::ffi::{c_int, c_void};

use crate::common::store64_le;

// crypto_onetimeauth_poly1305_state: public 256-byte aligned struct.
#[repr(C, align(16))]
struct CryptoOnetimeauthPoly1305State {
    opaque: [u8; 256],
}

const CRYPTO_AEAD_CHACHA20POLY1305_IETF_ABYTES: usize = 16;
const CRYPTO_AEAD_CHACHA20POLY1305_IETF_NPUBBYTES: usize = 12;

const CRYPTO_AEAD_XCHACHA20POLY1305_IETF_KEYBYTES: usize = 32;
const CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NSECBYTES: usize = 0;
const CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES: usize = 24;
const CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES: usize = 16;
// (SODIUM_SIZE_MAX - crypto_aead_xchacha20poly1305_ietf_ABYTES)
const CRYPTO_AEAD_XCHACHA20POLY1305_IETF_MESSAGEBYTES_MAX: u64 =
    (usize::MAX as u64).wrapping_sub(CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES as u64);

const CRYPTO_CORE_HCHACHA20_OUTPUTBYTES: usize = 32;
const CRYPTO_CORE_HCHACHA20_INPUTBYTES: usize = 16;

const STREAM_POLY1305_CHUNK: u64 = 131072;

// static const unsigned char _pad0[16] = { 0 };
static _PAD0: [u8; 16] = [0; 16];

extern "C" {
    fn crypto_stream_chacha20_ietf_ext(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_ietf_ext_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int;
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut CryptoOnetimeauthPoly1305State,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut CryptoOnetimeauthPoly1305State,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut CryptoOnetimeauthPoly1305State,
        out: *mut u8,
    ) -> c_int;
    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

unsafe fn _encrypt_detached(
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
    let mut state = core::mem::MaybeUninit::<CryptoOnetimeauthPoly1305State>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];

    let _ = nsec;
    crypto_stream_chacha20_ietf_ext(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(state, ad, adlen);
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(adlen)) & 0xf);

    {
        let mut off: u64 = 0;
        let mut ic: u32 = 1;
        // The ietf_ext counter is 32-bit and overflows into the IV past
        // ~256 GiB, which a chunk restart cannot reproduce, so oversized
        // messages make a single pass instead.
        let chunk: u64 = if mlen <= 64u64 * (0xffffffffu64 - 1u64) {
            STREAM_POLY1305_CHUNK
        } else {
            mlen
        };

        // COMPILER_ASSERT(STREAM_POLY1305_CHUNK % 64U == 0U);
        while off < mlen {
            let mut cl: u64 = mlen - off;
            if cl > chunk {
                cl = chunk;
            }
            crypto_stream_chacha20_ietf_ext_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                npub,
                ic,
                k,
            );
            crypto_onetimeauth_poly1305_update(state, c.add(off as usize), cl);
            off += cl;
            ic = ic.wrapping_add((cl / 64) as u32);
        }
    }
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(mlen)) & 0xf);

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(state, mac);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<CryptoOnetimeauthPoly1305State>(),
    );

    if !maclen_p.is_null() {
        *maclen_p = CRYPTO_AEAD_CHACHA20POLY1305_IETF_ABYTES as u64;
    }
    0
}

unsafe fn _decrypt_detached(
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
    let mut state = core::mem::MaybeUninit::<CryptoOnetimeauthPoly1305State>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut computed_mac = [0u8; CRYPTO_AEAD_CHACHA20POLY1305_IETF_ABYTES];
    let mlen: u64;
    let ret: c_int;

    let _ = nsec;
    crypto_stream_chacha20_ietf_ext(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(state, ad, adlen);
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(adlen)) & 0xf);

    mlen = clen;
    crypto_onetimeauth_poly1305_update(state, c, mlen);
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(mlen)) & 0xf);

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(state, computed_mac.as_mut_ptr());
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<CryptoOnetimeauthPoly1305State>(),
    );

    // COMPILER_ASSERT(sizeof computed_mac == 16U);
    ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(computed_mac.as_mut_ptr() as *mut c_void, computed_mac.len());
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        memset(m as *mut c_void, 0, mlen as usize);
        return -1;
    }
    // ACQUIRE_FENCE;
    crypto_stream_chacha20_ietf_ext_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
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
    let mut k2 = [0u8; CRYPTO_CORE_HCHACHA20_OUTPUTBYTES];
    let mut npub2 = [0u8; CRYPTO_AEAD_CHACHA20POLY1305_IETF_NPUBBYTES];
    let ret: c_int;

    crypto_core_hchacha20(k2.as_mut_ptr(), npub, k, core::ptr::null());
    memcpy(
        npub2.as_mut_ptr().add(4) as *mut c_void,
        npub.add(CRYPTO_CORE_HCHACHA20_INPUTBYTES) as *const c_void,
        CRYPTO_AEAD_CHACHA20POLY1305_IETF_NPUBBYTES - 4,
    );
    ret = _encrypt_detached(
        c,
        mac,
        maclen_p,
        m,
        mlen,
        ad,
        adlen,
        nsec,
        npub2.as_ptr(),
        k2.as_ptr(),
    );
    sodium_memzero(k2.as_mut_ptr() as *mut c_void, CRYPTO_CORE_HCHACHA20_OUTPUTBYTES);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_encrypt(
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

    if mlen > CRYPTO_AEAD_XCHACHA20POLY1305_IETF_MESSAGEBYTES_MAX {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ret = crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
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
            clen = mlen + CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES as u64;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
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
    let mut k2 = [0u8; CRYPTO_CORE_HCHACHA20_OUTPUTBYTES];
    let mut npub2 = [0u8; CRYPTO_AEAD_CHACHA20POLY1305_IETF_NPUBBYTES];
    let ret: c_int;

    crypto_core_hchacha20(k2.as_mut_ptr(), npub, k, core::ptr::null());
    memcpy(
        npub2.as_mut_ptr().add(4) as *mut c_void,
        npub.add(CRYPTO_CORE_HCHACHA20_INPUTBYTES) as *const c_void,
        CRYPTO_AEAD_CHACHA20POLY1305_IETF_NPUBBYTES - 4,
    );
    ret = _decrypt_detached(m, nsec, c, clen, mac, ad, adlen, npub2.as_ptr(), k2.as_ptr());
    sodium_memzero(k2.as_mut_ptr() as *mut c_void, CRYPTO_CORE_HCHACHA20_OUTPUTBYTES);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_decrypt(
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

    if clen >= CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES as u64 {
        ret = crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
            m,
            nsec,
            c,
            clen - CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES as u64,
            c.add((clen - CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES as u64) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES as u64;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_keybytes() -> usize {
    CRYPTO_AEAD_XCHACHA20POLY1305_IETF_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_npubbytes() -> usize {
    CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_nsecbytes() -> usize {
    CRYPTO_AEAD_XCHACHA20POLY1305_IETF_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_abytes() -> usize {
    CRYPTO_AEAD_XCHACHA20POLY1305_IETF_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_messagebytes_max() -> usize {
    CRYPTO_AEAD_XCHACHA20POLY1305_IETF_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_xchacha20poly1305_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_AEAD_XCHACHA20POLY1305_IETF_KEYBYTES);
}
