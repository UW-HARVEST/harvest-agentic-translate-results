//! Translation of c_src/libsodium/crypto_sign/ed25519/ref10/open.c

use core::ffi::{c_int, c_void};

use crate::fe25519::{ge25519_p2, ge25519_p3};

// crypto_sign_ed25519_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES
const crypto_sign_ed25519_BYTES: usize = 64;
const SODIUM_SIZE_MAX: usize = usize::MAX;
const crypto_sign_ed25519_MESSAGEBYTES_MAX: usize =
    SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES;

// Local repr(C) copy of crypto_hash_sha512_state (rule 4).
#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

extern "C" {
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(
        state: *mut crypto_hash_sha512_state,
        out: *mut u8,
    ) -> c_int;

    // _crypto_sign_ed25519_ref10_hinit lives in sign.c (not renamed).
    fn _crypto_sign_ed25519_ref10_hinit(
        hs: *mut crypto_hash_sha512_state,
        prehashed: c_int,
    );

    fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_sc25519_reduce(s: *mut u8);
    fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes_negate_vartime(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_double_scalarmult_vartime(
        r: *mut ge25519_p2,
        a: *const u8,
        big_a: *const ge25519_p3,
        b: *const u8,
    );
    fn _sodium_ge25519_p2_to_p3(r: *mut ge25519_p3, p: *const ge25519_p2);
    fn _sodium_ge25519_p3_sub(
        r: *mut ge25519_p3,
        p: *const ge25519_p3,
        q: *const ge25519_p3,
    );

    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs = core::mem::MaybeUninit::<crypto_hash_sha512_state>::uninit();
    let hs = hs.as_mut_ptr();
    let mut h: [u8; 64] = [0; 64];
    let mut check = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut expected_r = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut a = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut sb_ah = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut sb_ah_p2 = core::mem::MaybeUninit::<ge25519_p2>::uninit();

    // ACQUIRE_FENCE; -> nothing
    // ED25519_COMPAT undefined:
    if (*sig.add(63) & 240) != 0 && _sodium_sc25519_is_canonical(sig.add(32)) == 0 {
        return -1;
    }
    if _sodium_ge25519_is_canonical(pk) == 0 {
        return -1;
    }
    if _sodium_ge25519_frombytes_negate_vartime(a.as_mut_ptr(), pk) != 0
        || _sodium_ge25519_has_small_order(a.as_ptr()) != 0
    {
        return -1;
    }
    if _sodium_ge25519_frombytes(expected_r.as_mut_ptr(), sig) != 0
        || _sodium_ge25519_has_small_order(expected_r.as_ptr()) != 0
    {
        return -1;
    }
    _crypto_sign_ed25519_ref10_hinit(hs, prehashed);
    crypto_hash_sha512_update(hs, sig, 32);
    crypto_hash_sha512_update(hs, pk, 32);
    crypto_hash_sha512_update(hs, m, mlen);
    crypto_hash_sha512_final(hs, h.as_mut_ptr());
    _sodium_sc25519_reduce(h.as_mut_ptr());

    _sodium_ge25519_double_scalarmult_vartime(
        sb_ah_p2.as_mut_ptr(),
        h.as_ptr(),
        a.as_ptr(),
        sig.add(32),
    );
    _sodium_ge25519_p2_to_p3(sb_ah.as_mut_ptr(), sb_ah_p2.as_ptr());
    _sodium_ge25519_p3_sub(check.as_mut_ptr(), expected_r.as_ptr(), sb_ah.as_ptr());

    _sodium_ge25519_has_small_order(check.as_ptr()) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_verify_detached(sig, m, mlen, pk, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let mlen: u64;

    if smlen < 64 || (smlen - 64) > crypto_sign_ed25519_MESSAGEBYTES_MAX as u64 {
        // goto badsig
        if !mlen_p.is_null() {
            *mlen_p = 0;
        }
        return -1;
    }
    mlen = smlen - 64;
    if crypto_sign_ed25519_verify_detached(sm, sm.add(64), mlen, pk) != 0 {
        if !m.is_null() {
            memset(m as *mut c_void, 0, mlen as usize);
        }
        // goto badsig
        if !mlen_p.is_null() {
            *mlen_p = 0;
        }
        return -1;
    }
    if !mlen_p.is_null() {
        *mlen_p = mlen;
    }
    if !m.is_null() {
        memmove(m as *mut c_void, sm.add(64) as *const c_void, mlen as usize);
    }
    0
}
