//! Translation of `crypto_sign/ed25519/`:
//!   - `sign_ed25519.c`
//!   - `ref10/keypair.c`
//!   - `ref10/sign.c`
//!   - `ref10/open.c`
//! plus headers `ref10/sign_ed25519_ref10.h` and
//! `include/sodium/crypto_sign_ed25519.h`.

use core::ffi::{c_int, c_void};

use crate::common::{memmove, memset, SODIUM_SIZE_MAX};
use crate::crypto_core::ed25519_ref10::fe::{
    fe25519_1, fe25519_add, fe25519_mul, fe25519_sub, _sodium_fe25519_invert,
    _sodium_fe25519_tobytes,
};
use crate::crypto_core::ed25519_ref10::ge::{
    _sodium_ge25519_double_scalarmult_vartime, _sodium_ge25519_frombytes,
    _sodium_ge25519_frombytes_negate_vartime, _sodium_ge25519_has_small_order,
    _sodium_ge25519_is_canonical, _sodium_ge25519_is_on_main_subgroup,
    _sodium_ge25519_p2_to_p3, _sodium_ge25519_p3_sub, _sodium_ge25519_p3_tobytes,
    _sodium_ge25519_scalarmult_base,
};
use crate::crypto_core::ed25519_ref10::sc::{
    _sodium_sc25519_is_canonical, _sodium_sc25519_muladd, _sodium_sc25519_reduce,
};
use crate::crypto_core::ed25519_ref10::{ge25519_p2, ge25519_p3};
use crate::crypto_hash::sha512::{
    crypto_hash_sha512, crypto_hash_sha512_final, crypto_hash_sha512_init,
    crypto_hash_sha512_state, crypto_hash_sha512_update, crypto_hash_sha512_BYTES,
};
use crate::crypto_scalarmult::curve25519::crypto_scalarmult_curve25519_BYTES;
use crate::sodium_utils::sodium_memzero;

/* ---- from include/sodium/crypto_sign_ed25519.h ---- */

pub const crypto_sign_ed25519_BYTES: usize = 64;
pub const crypto_sign_ed25519_SEEDBYTES: usize = 32;
pub const crypto_sign_ed25519_PUBLICKEYBYTES: usize = 32;
pub const crypto_sign_ed25519_SECRETKEYBYTES: usize = 32 + 32;
pub const crypto_sign_ed25519_MESSAGEBYTES_MAX: usize =
    SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES;

/// `typedef struct crypto_sign_ed25519ph_state { crypto_hash_sha512_state hs; }`
#[repr(C)]
pub struct crypto_sign_ed25519ph_state {
    pub hs: crypto_hash_sha512_state,
}

/* ---- from sign_ed25519.c ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519ph_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_ed25519ph_state>()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_bytes() -> usize {
    crypto_sign_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_seedbytes() -> usize {
    crypto_sign_ed25519_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_publickeybytes() -> usize {
    crypto_sign_ed25519_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_secretkeybytes() -> usize {
    crypto_sign_ed25519_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_messagebytes_max() -> usize {
    crypto_sign_ed25519_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_seed(
    seed: *mut u8,
    sk: *const u8,
) -> c_int {
    memmove(seed, sk, crypto_sign_ed25519_SEEDBYTES);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_pk(
    pk: *mut u8,
    sk: *const u8,
) -> c_int {
    memmove(
        pk,
        sk.add(crypto_sign_ed25519_SEEDBYTES),
        crypto_sign_ed25519_PUBLICKEYBYTES,
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_init(
    state: *mut crypto_sign_ed25519ph_state,
) -> c_int {
    crypto_hash_sha512_init(&mut (*state).hs);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_update(
    state: *mut crypto_sign_ed25519ph_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    crypto_hash_sha512_update(&mut (*state).hs, m, mlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_create(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    let mut ph: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());

    _crypto_sign_ed25519_detached(
        sig,
        siglen_p,
        ph.as_ptr(),
        core::mem::size_of_val(&ph) as u64,
        sk,
        1,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_verify(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    let mut ph: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());

    _crypto_sign_ed25519_verify_detached(
        sig,
        ph.as_ptr(),
        core::mem::size_of_val(&ph) as u64,
        pk,
        1,
    )
}

/* ---- from ref10/keypair.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut A: ge25519_p3 = core::mem::zeroed();

    crypto_hash_sha512(sk, seed, 32);
    *sk.add(0) &= 248;
    *sk.add(31) &= 127;
    *sk.add(31) |= 64;

    _sodium_ge25519_scalarmult_base(&mut A, sk);
    _sodium_ge25519_p3_tobytes(pk, &A);

    memmove(sk, seed, 32);
    memmove(sk.add(32), pk, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    let mut seed: [u8; 32] = [0; 32];
    let ret: c_int;

    crate::randombytes::randombytes_buf(
        seed.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&seed),
    );
    ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(
        seed.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&seed),
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_pk_to_curve25519(
    curve25519_pk: *mut u8,
    ed25519_pk: *const u8,
) -> c_int {
    let mut A: ge25519_p3 = core::mem::zeroed();
    let mut x: [i32; 10] = [0; 10];
    let mut one_minus_y: [i32; 10] = [0; 10];

    if _sodium_ge25519_frombytes_negate_vartime(&mut A, ed25519_pk) != 0
        || _sodium_ge25519_has_small_order(&A) != 0
        || _sodium_ge25519_is_on_main_subgroup(&A) == 0
    {
        return -1;
    }
    fe25519_1(one_minus_y.as_mut_ptr());
    /* assumes A.Z=1 */
    fe25519_sub(
        one_minus_y.as_mut_ptr(),
        one_minus_y.as_ptr(),
        A.Y.as_ptr(),
    );
    fe25519_1(x.as_mut_ptr());
    fe25519_add(x.as_mut_ptr(), x.as_ptr(), A.Y.as_ptr());
    _sodium_fe25519_invert(one_minus_y.as_mut_ptr(), one_minus_y.as_ptr());
    fe25519_mul(x.as_mut_ptr(), x.as_ptr(), one_minus_y.as_ptr());
    _sodium_fe25519_tobytes(curve25519_pk, x.as_ptr());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_curve25519(
    curve25519_sk: *mut u8,
    ed25519_sk: *const u8,
) -> c_int {
    let mut h: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512(h.as_mut_ptr(), ed25519_sk, 32);
    h[0] &= 248;
    h[31] &= 127;
    h[31] |= 64;
    crate::common::memcpy(
        curve25519_sk,
        h.as_ptr(),
        crypto_scalarmult_curve25519_BYTES,
    );
    sodium_memzero(h.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&h));

    0
}

/* ---- from ref10/sign.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_ref10_hinit(
    hs: *mut crypto_hash_sha512_state,
    prehashed: c_int,
) {
    static DOM2PREFIX: [u8; 32 + 2] = [
        b'S', b'i', b'g', b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'n', b'o',
        b' ', b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'c', b'o', b'l', b'l',
        b'i', b's', b'i', b'o', b'n', b's', 1, 0,
    ];

    crypto_hash_sha512_init(hs);
    if prehashed != 0 {
        crypto_hash_sha512_update(
            hs,
            DOM2PREFIX.as_ptr(),
            core::mem::size_of_val(&DOM2PREFIX) as u64,
        );
    }
}

#[inline]
unsafe fn _crypto_sign_ed25519_clamp(k: *mut u8) {
    *k.add(0) &= 248;
    *k.add(31) &= 127;
    *k.add(31) |= 64;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs: crypto_hash_sha512_state = core::mem::zeroed();
    let mut az: [u8; 64] = [0; 64];
    let mut nonce: [u8; 64] = [0; 64];
    let mut hram: [u8; 64] = [0; 64];
    let mut R: ge25519_p3 = core::mem::zeroed();

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);

    crypto_hash_sha512(az.as_mut_ptr(), sk, 32);
    crypto_hash_sha512_update(&mut hs, az.as_ptr().add(32), 32);

    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, nonce.as_mut_ptr());

    memmove(sig.add(32), sk.add(32), 32);

    _sodium_sc25519_reduce(nonce.as_mut_ptr());
    _sodium_ge25519_scalarmult_base(&mut R, nonce.as_ptr());
    _sodium_ge25519_p3_tobytes(sig, &R);

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 64);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, hram.as_mut_ptr());

    _sodium_sc25519_reduce(hram.as_mut_ptr());
    _crypto_sign_ed25519_clamp(az.as_mut_ptr());
    _sodium_sc25519_muladd(sig.add(32), hram.as_ptr(), az.as_ptr(), nonce.as_ptr());

    sodium_memzero(az.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&az));
    sodium_memzero(
        nonce.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&nonce),
    );

    if !siglen_p.is_null() {
        *siglen_p = 64u64;
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

    memmove(sm.add(crypto_sign_ed25519_BYTES), m, mlen as usize);
    /* LCOV_EXCL_START */
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
        memset(sm, 0, (mlen as usize).wrapping_add(crypto_sign_ed25519_BYTES));
        return -1;
    }
    /* LCOV_EXCL_STOP */

    if !smlen_p.is_null() {
        *smlen_p = mlen.wrapping_add(siglen);
    }
    0
}

/* ---- from ref10/open.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs: crypto_hash_sha512_state = core::mem::zeroed();
    let mut h: [u8; 64] = [0; 64];
    let mut check: ge25519_p3 = core::mem::zeroed();
    let mut expected_r: ge25519_p3 = core::mem::zeroed();
    let mut A: ge25519_p3 = core::mem::zeroed();
    let mut sb_ah: ge25519_p3 = core::mem::zeroed();
    let mut sb_ah_p2: ge25519_p2 = core::mem::zeroed();

    core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire); /* ACQUIRE_FENCE */
    if (*sig.add(63) & 240) != 0 && _sodium_sc25519_is_canonical(sig.add(32)) == 0 {
        return -1;
    }
    if _sodium_ge25519_is_canonical(pk) == 0 {
        return -1;
    }
    if _sodium_ge25519_frombytes_negate_vartime(&mut A, pk) != 0
        || _sodium_ge25519_has_small_order(&A) != 0
    {
        return -1;
    }
    if _sodium_ge25519_frombytes(&mut expected_r, sig) != 0
        || _sodium_ge25519_has_small_order(&expected_r) != 0
    {
        return -1;
    }
    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 32);
    crypto_hash_sha512_update(&mut hs, pk, 32);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, h.as_mut_ptr());
    _sodium_sc25519_reduce(h.as_mut_ptr());

    _sodium_ge25519_double_scalarmult_vartime(&mut sb_ah_p2, h.as_ptr(), &A, sig.add(32));
    _sodium_ge25519_p2_to_p3(&mut sb_ah, &sb_ah_p2);
    _sodium_ge25519_p3_sub(&mut check, &expected_r, &sb_ah);

    _sodium_ge25519_has_small_order(&check) - 1
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

    if smlen < 64 || smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX as u64 {
        return open_badsig(mlen_p);
    }
    mlen = smlen - 64;
    if crypto_sign_ed25519_verify_detached(sm, sm.add(64), mlen, pk) != 0 {
        if !m.is_null() {
            memset(m, 0, mlen as usize);
        }
        return open_badsig(mlen_p);
    }
    if !mlen_p.is_null() {
        *mlen_p = mlen;
    }
    if !m.is_null() {
        memmove(m, sm.add(64), mlen as usize);
    }
    0
}

#[inline]
unsafe fn open_badsig(mlen_p: *mut u64) -> c_int {
    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    -1
}
