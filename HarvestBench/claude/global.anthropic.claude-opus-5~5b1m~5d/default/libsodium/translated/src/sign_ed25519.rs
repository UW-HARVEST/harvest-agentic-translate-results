//! Translated from:
//!  - `c_src/libsodium/crypto_sign/crypto_sign.c`
//!  - `c_src/libsodium/crypto_sign/ed25519/sign_ed25519.c`
//!  - `c_src/libsodium/crypto_sign/ed25519/ref10/keypair.c`
//!  - `c_src/libsodium/crypto_sign/ed25519/ref10/sign.c`
//!  - `c_src/libsodium/crypto_sign/ed25519/ref10/open.c`
//!
//! `ED25519_COMPAT` is not defined in the reference build, so `open.c` takes
//! the `#else` branch (canonical `s` and canonical `pk` checks, no
//! `crypto_verify_32` call).

use core::ffi::{c_char, c_int, c_void};

use crate::ed25519_ref10_fe::*;
use crate::types::{crypto_hash_sha512_state, fe25519, ge25519_p2, ge25519_p3};

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    #[link_name = "_sodium_ge25519_p3_tobytes"]
    fn ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_frombytes_negate_vartime"]
    fn ge25519_frombytes_negate_vartime(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_frombytes"]
    fn ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_p2_to_p3"]
    fn ge25519_p2_to_p3(r: *mut ge25519_p3, p: *const ge25519_p2);
    #[link_name = "_sodium_ge25519_p3_sub"]
    fn ge25519_p3_sub(r: *mut ge25519_p3, p: *const ge25519_p3, q: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_scalarmult_base"]
    fn ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    #[link_name = "_sodium_ge25519_double_scalarmult_vartime"]
    fn ge25519_double_scalarmult_vartime(
        r: *mut ge25519_p2,
        a: *const u8,
        p: *const ge25519_p3,
        b: *const u8,
    );
    #[link_name = "_sodium_ge25519_is_canonical"]
    fn ge25519_is_canonical(s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_is_on_main_subgroup"]
    fn ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_has_small_order"]
    fn ge25519_has_small_order(p: *const ge25519_p3) -> c_int;

    #[link_name = "_sodium_sc25519_reduce"]
    fn sc25519_reduce(s: *mut u8);
    #[link_name = "_sodium_sc25519_muladd"]
    fn sc25519_muladd(s: *mut u8, a: *const u8, b: *const u8, c: *const u8);
    #[link_name = "_sodium_sc25519_is_canonical"]
    fn sc25519_is_canonical(s: *const u8) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);

    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

/// `SODIUM_SIZE_MAX` (see `crate::common`), used to compute
/// `crypto_sign*_messagebytes_max()`.
const SODIUM_SIZE_MAX: u64 = crate::common::SODIUM_SIZE_MAX;

// ---------------------------------------------------------------------------
// crypto_sign_ed25519.h / crypto_sign.h types
// ---------------------------------------------------------------------------

/// `typedef struct crypto_sign_ed25519ph_state { crypto_hash_sha512_state hs; } crypto_sign_ed25519ph_state;`
#[repr(C)]
pub struct crypto_sign_ed25519ph_state {
    pub hs: crypto_hash_sha512_state,
}

/// `typedef crypto_sign_ed25519ph_state crypto_sign_state;`
pub type crypto_sign_state = crypto_sign_ed25519ph_state;

// ---------------------------------------------------------------------------
// crypto_sign/ed25519/ref10/sign_ed25519_ref10.h
// ---------------------------------------------------------------------------

/// `void _crypto_sign_ed25519_ref10_hinit(crypto_hash_sha512_state *hs, int prehashed)`
///
/// This name is not rewritten by `quirks.h` — the C source already spells it
/// with a leading underscore, so the exported symbol keeps the same name.
#[no_mangle]
pub unsafe extern "C" fn _crypto_sign_ed25519_ref10_hinit(
    hs: *mut crypto_hash_sha512_state,
    prehashed: c_int,
) {
    static DOM2PREFIX: [u8; 34] = [
        b'S', b'i', b'g', b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'n', b'o', b' ', b'E',
        b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'c', b'o', b'l', b'l', b'i', b's', b'i', b'o',
        b'n', b's', 1, 0,
    ];

    crypto_hash_sha512_init(hs);
    if prehashed != 0 {
        crypto_hash_sha512_update(hs, DOM2PREFIX.as_ptr(), DOM2PREFIX.len() as u64);
    }
}

/// `static inline void _crypto_sign_ed25519_clamp(unsigned char k[32])`
#[inline]
unsafe fn crypto_sign_ed25519_clamp(k: *mut u8) {
    *k &= 248;
    *k.add(31) &= 127;
    *k.add(31) |= 64;
}

/// `int _crypto_sign_ed25519_detached(unsigned char *sig, unsigned long long *siglen_p, const unsigned char *m, unsigned long long mlen, const unsigned char *sk, int prehashed)`
#[no_mangle]
pub unsafe extern "C" fn _crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs: crypto_hash_sha512_state = core::mem::zeroed();
    let mut az = [0u8; 64];
    let mut nonce = [0u8; 64];
    let mut hram = [0u8; 64];
    let mut r: ge25519_p3 = core::mem::zeroed();

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);

    crypto_hash_sha512(az.as_mut_ptr(), sk, 32);

    crypto_hash_sha512_update(&mut hs, az.as_ptr().add(32), 32);

    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, nonce.as_mut_ptr());

    memmove(
        sig.add(32) as *mut c_void,
        sk.add(32) as *const c_void,
        32,
    );

    sc25519_reduce(nonce.as_mut_ptr());
    ge25519_scalarmult_base(&mut r, nonce.as_ptr());
    ge25519_p3_tobytes(sig, &r);

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 64);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, hram.as_mut_ptr());

    sc25519_reduce(hram.as_mut_ptr());
    crypto_sign_ed25519_clamp(az.as_mut_ptr());
    sc25519_muladd(sig.add(32), hram.as_ptr(), az.as_ptr(), nonce.as_ptr());

    sodium_memzero(az.as_mut_ptr() as *mut c_void, az.len());
    sodium_memzero(nonce.as_mut_ptr() as *mut c_void, nonce.len());

    if !siglen_p.is_null() {
        *siglen_p = 64;
    }
    0
}

/// `int crypto_sign_ed25519_detached(...)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk, 0)
}

/// `int crypto_sign_ed25519(unsigned char *sm, unsigned long long *smlen_p, const unsigned char *m, unsigned long long mlen, const unsigned char *sk)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519(
    sm: *mut u8,
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let mut siglen: u64 = 0;

    memmove(
        sm.add(64) as *mut c_void,
        m as *const c_void,
        mlen as usize,
    );

    if crypto_sign_ed25519_detached(sm, &mut siglen, sm.add(64), mlen, sk) != 0 || siglen != 64 {
        if !smlen_p.is_null() {
            *smlen_p = 0;
        }
        memset(sm as *mut c_void, 0, (mlen + 64) as usize);
        return -1;
    }

    if !smlen_p.is_null() {
        *smlen_p = mlen + siglen;
    }
    0
}

// ---------------------------------------------------------------------------
// crypto_sign/ed25519/ref10/open.c
// ---------------------------------------------------------------------------

/// `int _crypto_sign_ed25519_verify_detached(const unsigned char *sig, const unsigned char *m, unsigned long long mlen, const unsigned char *pk, int prehashed)`
#[no_mangle]
pub unsafe extern "C" fn _crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs: crypto_hash_sha512_state = core::mem::zeroed();
    let mut h = [0u8; 64];
    let mut check: ge25519_p3 = core::mem::zeroed();
    let mut expected_r: ge25519_p3 = core::mem::zeroed();
    let mut a: ge25519_p3 = core::mem::zeroed();
    let mut sb_ah: ge25519_p3 = core::mem::zeroed();
    let mut sb_ah_p2: ge25519_p2 = core::mem::zeroed();

    // ACQUIRE_FENCE is a no-op (HAVE_ATOMIC_OPS / HAVE_PTHREAD not defined).

    if (*sig.add(63) & 240) != 0 && sc25519_is_canonical(sig.add(32)) == 0 {
        return -1;
    }
    if ge25519_is_canonical(pk) == 0 {
        return -1;
    }

    if ge25519_frombytes_negate_vartime(&mut a, pk) != 0 || ge25519_has_small_order(&a) != 0 {
        return -1;
    }
    if ge25519_frombytes(&mut expected_r, sig) != 0
        || ge25519_has_small_order(&expected_r) != 0
    {
        return -1;
    }
    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 32);
    crypto_hash_sha512_update(&mut hs, pk, 32);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, h.as_mut_ptr());
    sc25519_reduce(h.as_mut_ptr());

    ge25519_double_scalarmult_vartime(&mut sb_ah_p2, h.as_ptr(), &a, sig.add(32));
    ge25519_p2_to_p3(&mut sb_ah, &sb_ah_p2);
    ge25519_p3_sub(&mut check, &expected_r, &sb_ah);

    ge25519_has_small_order(&check) - 1
}

/// `int crypto_sign_ed25519_verify_detached(...)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_verify_detached(sig, m, mlen, pk, 0)
}

/// `int crypto_sign_ed25519_open(unsigned char *m, unsigned long long *mlen_p, const unsigned char *sm, unsigned long long smlen, const unsigned char *pk)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let mlen: u64;

    if smlen < 64 || smlen - 64 > (SODIUM_SIZE_MAX - 64) {
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
        if !mlen_p.is_null() {
            *mlen_p = 0;
        }
        return -1;
    }
    if !mlen_p.is_null() {
        *mlen_p = mlen;
    }
    if !m.is_null() {
        memmove(
            m as *mut c_void,
            sm.add(64) as *const c_void,
            mlen as usize,
        );
    }
    0
}

// ---------------------------------------------------------------------------
// crypto_sign/ed25519/ref10/keypair.c
// ---------------------------------------------------------------------------

/// `int crypto_sign_ed25519_seed_keypair(unsigned char *pk, unsigned char *sk, const unsigned char *seed)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut a: ge25519_p3 = core::mem::zeroed();

    crypto_hash_sha512(sk, seed, 32);
    *sk &= 248;
    *sk.add(31) &= 127;
    *sk.add(31) |= 64;

    ge25519_scalarmult_base(&mut a, sk);
    ge25519_p3_tobytes(pk, &a);

    memmove(sk as *mut c_void, seed as *const c_void, 32);
    memmove(sk.add(32) as *mut c_void, pk as *const c_void, 32);

    0
}

/// `int crypto_sign_ed25519_keypair(unsigned char *pk, unsigned char *sk)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, seed.len());
    ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());

    ret
}

/// `int crypto_sign_ed25519_pk_to_curve25519(unsigned char *curve25519_pk, const unsigned char *ed25519_pk)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_pk_to_curve25519(
    curve25519_pk: *mut u8,
    ed25519_pk: *const u8,
) -> c_int {
    let mut a: ge25519_p3 = core::mem::zeroed();
    let mut x: fe25519 = [0i32; 10];
    let mut one_minus_y: fe25519 = [0i32; 10];

    if ge25519_frombytes_negate_vartime(&mut a, ed25519_pk) != 0
        || ge25519_has_small_order(&a) != 0
        || ge25519_is_on_main_subgroup(&a) == 0
    {
        return -1;
    }
    fe25519_1(&mut one_minus_y);

    let one_minus_y_copy = one_minus_y;
    fe25519_sub(&mut one_minus_y, &one_minus_y_copy, &a.Y);
    fe25519_1(&mut x);
    let x_copy = x;
    fe25519_add(&mut x, &x_copy, &a.Y);
    let one_minus_y_copy = one_minus_y;
    fe25519_invert(&mut one_minus_y, &one_minus_y_copy);
    fe25519_mul_ip(&mut x, &one_minus_y);
    fe25519_tobytes(curve25519_pk, &x);

    0
}

/// `int crypto_sign_ed25519_sk_to_curve25519(unsigned char *curve25519_sk, const unsigned char *ed25519_sk)`
#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_curve25519(
    curve25519_sk: *mut u8,
    ed25519_sk: *const u8,
) -> c_int {
    let mut h = [0u8; 64];

    crypto_hash_sha512(h.as_mut_ptr(), ed25519_sk, 32);
    h[0] &= 248;
    h[31] &= 127;
    h[31] |= 64;
    memmove(
        curve25519_sk as *mut c_void,
        h.as_ptr() as *const c_void,
        32,
    );
    sodium_memzero(h.as_mut_ptr() as *mut c_void, h.len());

    0
}

// ---------------------------------------------------------------------------
// crypto_sign/ed25519/sign_ed25519.c
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519ph_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_ed25519ph_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_bytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_publickeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_secretkeybytes() -> usize {
    32 + 32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 64) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_seed(seed: *mut u8, sk: *const u8) -> c_int {
    memmove(seed as *mut c_void, sk as *const c_void, 32);
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_pk(pk: *mut u8, sk: *const u8) -> c_int {
    memmove(pk as *mut c_void, sk.add(32) as *const c_void, 32);
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519ph_init(
    state: *mut crypto_sign_ed25519ph_state,
) -> c_int {
    crypto_hash_sha512_init(&mut (*state).hs);
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519ph_update(
    state: *mut crypto_sign_ed25519ph_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    crypto_hash_sha512_update(&mut (*state).hs, m, mlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_create(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    let mut ph = [0u8; 64];

    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());

    _crypto_sign_ed25519_detached(sig, siglen_p, ph.as_ptr(), ph.len() as u64, sk, 1)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_verify(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    let mut ph = [0u8; 64];

    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());

    _crypto_sign_ed25519_verify_detached(sig, ph.as_ptr(), ph.len() as u64, pk, 1)
}

// ---------------------------------------------------------------------------
// crypto_sign/crypto_sign.c
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_bytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> usize {
    32 + 32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 64) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_primitive() -> *const c_char {
    b"ed25519\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_sign_ed25519_seed_keypair(pk, sk, seed)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_sign_ed25519_keypair(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519(sm, smlen_p, m, mlen, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_open(m, mlen_p, sm, smlen, pk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_verify_detached(sig, m, mlen, pk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_init(state: *mut crypto_sign_state) -> c_int {
    crypto_sign_ed25519ph_init(state)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_update(
    state: *mut crypto_sign_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    crypto_sign_ed25519ph_update(state, m, mlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_final_create(
    state: *mut crypto_sign_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519ph_final_create(state, sig, siglen_p, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_final_verify(
    state: *mut crypto_sign_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519ph_final_verify(state, sig, pk)
}
