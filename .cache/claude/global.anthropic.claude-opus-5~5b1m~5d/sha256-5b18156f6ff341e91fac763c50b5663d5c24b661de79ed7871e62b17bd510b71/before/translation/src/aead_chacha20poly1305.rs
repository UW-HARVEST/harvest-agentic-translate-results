use core::ffi::{c_int, c_void};

use crate::common::{store64_le, SODIUM_SIZE_MAX};

const CRYPTO_STREAM_CHUNK: u64 = 131072;
const IETF_MESSAGEBYTES_MAX_LIMIT: u64 = 64u64 * ((1u64 << 32) - 1u64);

#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

extern "C" {
    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_ietf_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int;

    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int;

    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);

    fn memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

static _PAD0: [u8; 16] = [0u8; 16];

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_encrypt_detached(
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
    let mut state = core::mem::MaybeUninit::<crypto_onetimeauth_poly1305_state>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];

    let _ = nsec;
    crypto_stream_chacha20(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(state, ad, adlen);
    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    {
        let mut off: u64 = 0;
        let mut ic: u64 = 1;

        while off < mlen {
            let mut cl = mlen - off;
            if cl > CRYPTO_STREAM_CHUNK {
                cl = CRYPTO_STREAM_CHUNK;
            }
            crypto_stream_chacha20_xor_ic(c.add(off as usize), m.add(off as usize), cl, npub, ic, k);
            crypto_onetimeauth_poly1305_update(state, c.add(off as usize), cl);
            off += cl;
            ic += cl / 64;
        }
    }
    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(state, mac);
    sodium_memzero(state as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    if !maclen_p.is_null() {
        *maclen_p = 16;
    }
    0
}

#[no_mangle]
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
) -> c_int {
    let mut clen: u64 = 0;

    if mlen > SODIUM_SIZE_MAX - 16 {
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
            clen = mlen + 16;
        }
        *clen_p = clen;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
    let mut state = core::mem::MaybeUninit::<crypto_onetimeauth_poly1305_state>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];

    let _ = nsec;
    crypto_stream_chacha20_ietf(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(state, ad, adlen);
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(adlen)) & 0xf);

    {
        let mut off: u64 = 0;
        let mut ic: u32 = 1;

        while off < mlen {
            let mut cl = mlen - off;
            if cl > CRYPTO_STREAM_CHUNK {
                cl = CRYPTO_STREAM_CHUNK;
            }
            crypto_stream_chacha20_ietf_xor_ic(
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
    sodium_memzero(state as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    if !maclen_p.is_null() {
        *maclen_p = 16;
    }
    0
}

#[no_mangle]
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
) -> c_int {
    let mut clen: u64 = 0;

    let limit_a = SODIUM_SIZE_MAX - 16;
    let limit = if limit_a < IETF_MESSAGEBYTES_MAX_LIMIT {
        limit_a
    } else {
        IETF_MESSAGEBYTES_MAX_LIMIT
    };
    if mlen > limit {
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
            clen = mlen + 16;
        }
        *clen_p = clen;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_decrypt_detached(
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
    let mut state = core::mem::MaybeUninit::<crypto_onetimeauth_poly1305_state>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut computed_mac = [0u8; 16];

    let _ = nsec;
    crypto_stream_chacha20(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(state, ad, adlen);
    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    let mlen: u64 = clen;
    crypto_onetimeauth_poly1305_update(state, c, mlen);
    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(state, computed_mac.as_mut_ptr());
    sodium_memzero(state as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    let ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(computed_mac.as_mut_ptr() as *mut c_void, computed_mac.len());
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        memset(m as *mut c_void, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[no_mangle]
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
) -> c_int {
    let mut mlen: u64 = 0;
    let mut ret: c_int = -1;

    if clen >= 16 {
        ret = crypto_aead_chacha20poly1305_decrypt_detached(
            m,
            nsec,
            c,
            clen - 16,
            c.add((clen - 16) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - 16;
        }
        *mlen_p = mlen;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_decrypt_detached(
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
    let mut state = core::mem::MaybeUninit::<crypto_onetimeauth_poly1305_state>::uninit();
    let state = state.as_mut_ptr();
    let mut block0 = [0u8; 64];
    let mut slen = [0u8; 8];
    let mut computed_mac = [0u8; 16];

    let _ = nsec;
    crypto_stream_chacha20_ietf(block0.as_mut_ptr(), block0.len() as u64, npub, k);
    crypto_onetimeauth_poly1305_init(state, block0.as_ptr());
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, block0.len());

    crypto_onetimeauth_poly1305_update(state, ad, adlen);
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(adlen)) & 0xf);

    let mlen: u64 = clen;
    crypto_onetimeauth_poly1305_update(state, c, mlen);
    crypto_onetimeauth_poly1305_update(state, _PAD0.as_ptr(), (0x10u64.wrapping_sub(mlen)) & 0xf);

    store64_le(slen.as_mut_ptr(), adlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    store64_le(slen.as_mut_ptr(), mlen);
    crypto_onetimeauth_poly1305_update(state, slen.as_ptr(), slen.len() as u64);

    crypto_onetimeauth_poly1305_final(state, computed_mac.as_mut_ptr());
    sodium_memzero(state as *mut c_void, core::mem::size_of::<crypto_onetimeauth_poly1305_state>());

    let ret = crypto_verify_16(computed_mac.as_ptr(), mac);
    sodium_memzero(computed_mac.as_mut_ptr() as *mut c_void, computed_mac.len());
    if m.is_null() {
        return ret;
    }
    if ret != 0 {
        memset(m as *mut c_void, 0, mlen as usize);
        return -1;
    }
    crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, npub, 1, k);

    0
}

#[no_mangle]
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
) -> c_int {
    let mut mlen: u64 = 0;
    let mut ret: c_int = -1;

    if clen >= 16 {
        ret = crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            m,
            nsec,
            c,
            clen - 16,
            c.add((clen - 16) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - 16;
        }
        *mlen_p = mlen;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_npubbytes() -> usize {
    12
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_nsecbytes() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_abytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_messagebytes_max() -> usize {
    let limit_a = SODIUM_SIZE_MAX - 16;
    let limit = if limit_a < IETF_MESSAGEBYTES_MAX_LIMIT {
        limit_a
    } else {
        IETF_MESSAGEBYTES_MAX_LIMIT
    };
    limit as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_npubbytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_nsecbytes() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_abytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_aead_chacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
