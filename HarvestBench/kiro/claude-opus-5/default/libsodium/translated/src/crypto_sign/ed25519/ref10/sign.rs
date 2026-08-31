//! Translation of c_src/libsodium/crypto_sign/ed25519/ref10/sign.c

use core::ffi::{c_int, c_void};

// crypto_sign_ed25519_BYTES
const crypto_sign_ed25519_BYTES: usize = 64;

// Local repr(C) copy of crypto_hash_sha512_state (rule 4).
#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

extern "C" {
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(
        state: *mut crypto_hash_sha512_state,
        out: *mut u8,
    ) -> c_int;
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;

    fn _sodium_sc25519_reduce(s: *mut u8);
    fn _sodium_sc25519_muladd(s: *mut u8, a: *const u8, b: *const u8, c: *const u8);
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);

    fn sodium_memzero(pnt: *mut c_void, len: usize);

    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

use crate::fe25519::ge25519_p3;

// _crypto_sign_ed25519_ref10_hinit is not renamed by quirks.h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_ref10_hinit(
    hs: *mut crypto_hash_sha512_state,
    prehashed: c_int,
) {
    static DOM2PREFIX: [u8; 32 + 2] = [
        b'S', b'i', b'g', b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ',
        b'n', b'o', b' ',
        b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ',
        b'c', b'o', b'l', b'l', b'i', b's', b'i', b'o', b'n', b's', 1, 0,
    ];

    crypto_hash_sha512_init(hs);
    if prehashed != 0 {
        crypto_hash_sha512_update(
            hs,
            DOM2PREFIX.as_ptr(),
            core::mem::size_of::<[u8; 32 + 2]>() as u64,
        );
    }
}

#[inline]
unsafe fn _crypto_sign_ed25519_clamp(k: *mut u8) {
    *k.add(0) &= 248;
    *k.add(31) &= 127;
    *k.add(31) |= 64;
}

// ED25519_NONDETERMINISTIC undefined: _crypto_sign_ed25519_synthetic_r_hv is
// not compiled and the deterministic nonce branch is used.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs = core::mem::MaybeUninit::<crypto_hash_sha512_state>::uninit();
    let hs = hs.as_mut_ptr();
    let mut az: [u8; 64] = [0; 64];
    let mut nonce: [u8; 64] = [0; 64];
    let mut hram: [u8; 64] = [0; 64];
    let mut r = core::mem::MaybeUninit::<ge25519_p3>::uninit();

    _crypto_sign_ed25519_ref10_hinit(hs, prehashed);

    crypto_hash_sha512(az.as_mut_ptr(), sk, 32);
    // ED25519_NONDETERMINISTIC undefined:
    crypto_hash_sha512_update(hs, az.as_ptr().add(32), 32);

    crypto_hash_sha512_update(hs, m, mlen);
    crypto_hash_sha512_final(hs, nonce.as_mut_ptr());

    memmove(
        sig.add(32) as *mut c_void,
        sk.add(32) as *const c_void,
        32,
    );

    _sodium_sc25519_reduce(nonce.as_mut_ptr());
    _sodium_ge25519_scalarmult_base(r.as_mut_ptr(), nonce.as_ptr());
    _sodium_ge25519_p3_tobytes(sig, r.as_ptr());

    _crypto_sign_ed25519_ref10_hinit(hs, prehashed);
    crypto_hash_sha512_update(hs, sig, 64);
    crypto_hash_sha512_update(hs, m, mlen);
    crypto_hash_sha512_final(hs, hram.as_mut_ptr());

    _sodium_sc25519_reduce(hram.as_mut_ptr());
    _crypto_sign_ed25519_clamp(az.as_mut_ptr());
    _sodium_sc25519_muladd(sig.add(32), hram.as_ptr(), az.as_ptr(), nonce.as_ptr());

    sodium_memzero(az.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 64]>());
    sodium_memzero(nonce.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 64]>());

    if !siglen_p.is_null() {
        *siglen_p = 64;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519(
    sm: *mut u8,
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let mut siglen: u64 = 0;

    memmove(
        sm.add(crypto_sign_ed25519_BYTES) as *mut c_void,
        m as *const c_void,
        mlen as usize,
    );
    // LCOV_EXCL_START
    if crypto_sign_ed25519_detached(
        sm,
        &mut siglen,
        sm.add(crypto_sign_ed25519_BYTES),
        mlen,
        sk,
    ) != 0
        || siglen != crypto_sign_ed25519_BYTES as u64
    {
        if !smlen_p.is_null() {
            *smlen_p = 0;
        }
        memset(sm as *mut c_void, 0, (mlen + crypto_sign_ed25519_BYTES as u64) as usize);
        return -1;
    }
    // LCOV_EXCL_STOP

    if !smlen_p.is_null() {
        *smlen_p = mlen + siglen;
    }
    0
}
