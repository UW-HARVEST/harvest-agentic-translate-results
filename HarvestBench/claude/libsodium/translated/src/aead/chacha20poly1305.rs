//! Translated from crypto_aead/chacha20poly1305/aead_chacha20poly1305.c
use crate::primitives::cutil::store64_le;
use crate::primitives::poly1305::crypto_onetimeauth_poly1305_state;
use core::ffi::c_void;

extern "C" {
    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32;
    fn crypto_stream_chacha20_ietf_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        input: *const u8,
        inlen: u64,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> i32;
    fn crypto_verify_16(x: *const u8, y: *const u8) -> i32;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const ABYTES: u64 = 16;
const KEYBYTES: usize = 32;
const NPUBBYTES: usize = 8;
const IETF_NPUBBYTES: usize = 12;
const NSECBYTES: usize = 0;
// (SODIUM_SIZE_MAX - ABYTES)
const MESSAGEBYTES_MAX: u64 = u64::MAX - ABYTES;
// min(SODIUM_SIZE_MAX - ABYTES, 64*(2^32-1))
const IETF_MESSAGEBYTES_MAX: u64 = 64 * ((1u64 << 32) - 1);

const _PAD0: [u8; 16] = [0; 16];
const STREAM_POLY1305_CHUNK: u64 = 131072;

// ------------- original (non-ietf) -------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_encrypt_detached(
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
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];

    crypto_stream_chacha20(block0.as_mut_ptr(), 64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    {
        let mut off: u64 = 0;
        let mut ic: u64 = 1;
        while off < mlen {
            let mut cl = mlen - off;
            if cl > STREAM_POLY1305_CHUNK {
                cl = STREAM_POLY1305_CHUNK;
            }
            crypto_stream_chacha20_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                npub,
                ic,
                k,
            );
            crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
            off += cl;
            ic += cl / 64;
        }
    }
    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    if !maclen_p.is_null() {
        *maclen_p = ABYTES;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_encrypt(
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
    let ret = crypto_aead_chacha20poly1305_encrypt_detached(
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
            clen = mlen + ABYTES;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];

    crypto_stream_chacha20_ietf(block0.as_mut_ptr(), 64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    crypto_onetimeauth_poly1305_update(&mut state, _PAD0.as_ptr(), (0x10 - adlen) & 0xf);

    {
        let mut off: u64 = 0;
        let mut ic: u32 = 1;
        while off < mlen {
            let mut cl = mlen - off;
            if cl > STREAM_POLY1305_CHUNK {
                cl = STREAM_POLY1305_CHUNK;
            }
            crypto_stream_chacha20_ietf_xor_ic(
                c.add(off as usize),
                m.add(off as usize),
                cl,
                npub,
                ic,
                k,
            );
            crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
            off += cl;
            ic = ic.wrapping_add((cl / 64) as u32);
        }
    }
    crypto_onetimeauth_poly1305_update(&mut state, _PAD0.as_ptr(), (0x10 - mlen) & 0xf);

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    if !maclen_p.is_null() {
        *maclen_p = ABYTES;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_encrypt(
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
    if mlen > IETF_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    let ret = crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
            clen = mlen + ABYTES;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_decrypt_detached(
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
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut computed_mac = [0u8; 16];

    crypto_stream_chacha20(block0.as_mut_ptr(), 64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    let mlen = clen;
    crypto_onetimeauth_poly1305_update(&mut state, c, mlen);
    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    crypto_onetimeauth_poly1305_final(&mut state, computed_mac.as_mut_ptr());
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    let ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(computed_mac.as_mut_ptr() as *mut c_void, 16);
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        core::ptr::write_bytes(m, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_xor_ic(m, c, mlen, npub, 1, k);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_decrypt(
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
    if clen >= ABYTES {
        ret = crypto_aead_chacha20poly1305_decrypt_detached(
            m,
            nsec,
            c,
            clen - ABYTES,
            c.add((clen - ABYTES) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - ABYTES;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_decrypt_detached(
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
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut computed_mac = [0u8; 16];

    crypto_stream_chacha20_ietf(block0.as_mut_ptr(), 64, npub, k);
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, 64);

    crypto_onetimeauth_poly1305_update(&mut state, ad, adlen);
    crypto_onetimeauth_poly1305_update(&mut state, _PAD0.as_ptr(), (0x10 - adlen) & 0xf);

    let mlen = clen;
    crypto_onetimeauth_poly1305_update(&mut state, c, mlen);
    crypto_onetimeauth_poly1305_update(&mut state, _PAD0.as_ptr(), (0x10 - mlen) & 0xf);

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(&mut state, slen.as_ptr(), 8);

    crypto_onetimeauth_poly1305_final(&mut state, computed_mac.as_mut_ptr());
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    let ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(computed_mac.as_mut_ptr() as *mut c_void, 16);
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        core::ptr::write_bytes(m, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, npub, 1, k);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_decrypt(
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
    if clen >= ABYTES {
        ret = crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            m,
            nsec,
            c,
            clen - ABYTES,
            c.add((clen - ABYTES) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - ABYTES;
        }
        *mlen_p = mlen;
    }
    ret
}

// ------------- constant accessors -------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_npubbytes() -> usize {
    IETF_NPUBBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_nsecbytes() -> usize {
    NSECBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_abytes() -> usize {
    ABYTES as usize
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_ietf_messagebytes_max() -> usize {
    IETF_MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_npubbytes() -> usize {
    NPUBBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_nsecbytes() -> usize {
    NSECBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_abytes() -> usize {
    ABYTES as usize
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_chacha20poly1305_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}
