//! crypto_sign ed25519: keypair.c, sign.c, open.c, sign_ed25519.c, crypto_sign.c.
use crate::ed25519::fe25519::*;
use crate::ed25519::ge25519;
use crate::ed25519::sc25519;
use crate::ed25519::sha512::{
    crypto_hash_sha512_final, crypto_hash_sha512_init, crypto_hash_sha512_update,
    crypto_hash_sha512_state,
};
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, input: *const u8, inlen: u64) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

const ED25519_BYTES: usize = 64;
const ED25519_SEEDBYTES: usize = 32;
const ED25519_PUBLICKEYBYTES: usize = 32;
const ED25519_SECRETKEYBYTES: usize = 64;
const CURVE25519_BYTES: usize = 32;

#[repr(C)]
pub struct crypto_sign_ed25519ph_state {
    pub hs: crypto_hash_sha512_state,
}

fn new_hs() -> crypto_hash_sha512_state {
    crypto_hash_sha512_state {
        state: [0; 8],
        count: [0; 2],
        buf: [0; 128],
    }
}

/* ---- keypair.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_hash_sha512(sk, seed, 32);
    *sk.add(0) &= 248;
    *sk.add(31) &= 127;
    *sk.add(31) |= 64;

    let sk_sl = core::slice::from_raw_parts(sk, 32);
    let a = ge25519::scalarmult_base(sk_sl);
    let pkb = ge25519::p3_tobytes(&a);
    core::ptr::copy_nonoverlapping(pkb.as_ptr(), pk, 32);

    core::ptr::copy(seed, sk, 32);
    core::ptr::copy(pk, sk.add(32), 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; 32];
    randombytes_buf(seed.as_mut_ptr() as *mut c_void, seed.len());
    let ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_pk_to_curve25519(
    curve25519_pk: *mut u8,
    ed25519_pk: *const u8,
) -> c_int {
    let pksl = core::slice::from_raw_parts(ed25519_pk, 32);
    let (a, r) = ge25519::frombytes_negate_vartime(pksl);
    if r != 0 || ge25519::has_small_order(&a) != 0 || ge25519::is_on_main_subgroup(&a) == 0 {
        return -1;
    }
    let mut one_minus_y = fe_1();
    one_minus_y = fe_sub(one_minus_y, a.y);
    let mut x = fe_1();
    x = fe_add(x, a.y);
    one_minus_y = fe_invert(one_minus_y);
    x = fe_mul(x, one_minus_y);
    let out = fe_tobytes(x);
    core::ptr::copy_nonoverlapping(out.as_ptr(), curve25519_pk, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_curve25519(
    curve25519_sk: *mut u8,
    ed25519_sk: *const u8,
) -> c_int {
    let mut h = [0u8; 64];
    crypto_hash_sha512(h.as_mut_ptr(), ed25519_sk, 32);
    h[0] &= 248;
    h[31] &= 127;
    h[31] |= 64;
    core::ptr::copy_nonoverlapping(h.as_ptr(), curve25519_sk, CURVE25519_BYTES);
    sodium_memzero(h.as_mut_ptr() as *mut c_void, h.len());
    0
}

/* ---- sign.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_ref10_hinit(
    hs: *mut crypto_hash_sha512_state,
    prehashed: c_int,
) {
    const DOM2PREFIX: [u8; 34] = [
        b'S', b'i', b'g', b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'n', b'o', b' ', b'E',
        b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'c', b'o', b'l', b'l', b'i', b's', b'i', b'o',
        b'n', b's', 1, 0,
    ];
    crypto_hash_sha512_init(hs);
    if prehashed != 0 {
        crypto_hash_sha512_update(hs, DOM2PREFIX.as_ptr(), DOM2PREFIX.len() as u64);
    }
}

fn clamp(k: &mut [u8]) {
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;
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
    let mut hs = new_hs();
    let mut az = [0u8; 64];
    let mut nonce = [0u8; 64];
    let mut hram = [0u8; 64];

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);

    crypto_hash_sha512(az.as_mut_ptr(), sk, 32);
    crypto_hash_sha512_update(&mut hs, az.as_ptr().add(32), 32);

    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, nonce.as_mut_ptr());

    core::ptr::copy(sk.add(32), sig.add(32), 32);

    sc25519::sc_reduce(&mut nonce);
    let r = ge25519::scalarmult_base(&nonce[0..32]);
    let rb = ge25519::p3_tobytes(&r);
    core::ptr::copy_nonoverlapping(rb.as_ptr(), sig, 32);

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 64);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, hram.as_mut_ptr());

    sc25519::sc_reduce(&mut hram);
    clamp(&mut az);
    let out = sc25519::sc_muladd(&hram[0..32], &az[0..32], &nonce[0..32]);
    core::ptr::copy_nonoverlapping(out.as_ptr(), sig.add(32), 32);

    sodium_memzero(az.as_mut_ptr() as *mut c_void, az.len());
    sodium_memzero(nonce.as_mut_ptr() as *mut c_void, nonce.len());

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
    core::ptr::copy(m, sm.add(ED25519_BYTES), mlen as usize);
    let mut siglen: u64 = 0;
    if crypto_sign_ed25519_detached(sm, &mut siglen, sm.add(ED25519_BYTES), mlen, sk) != 0
        || siglen != ED25519_BYTES as u64
    {
        if !smlen_p.is_null() {
            *smlen_p = 0;
        }
        core::ptr::write_bytes(sm, 0, mlen as usize + ED25519_BYTES);
        return -1;
    }
    if !smlen_p.is_null() {
        *smlen_p = mlen + siglen;
    }
    0
}

/* ---- open.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs = new_hs();
    let mut h = [0u8; 64];

    let sigsl = core::slice::from_raw_parts(sig, 64);
    let pksl = core::slice::from_raw_parts(pk, 32);

    if (sigsl[63] & 240) != 0 && sc25519::sc_is_canonical(&sigsl[32..64]) == 0 {
        return -1;
    }
    if ge25519::is_canonical(pksl) == 0 {
        return -1;
    }
    let (a, ar) = ge25519::frombytes_negate_vartime(pksl);
    if ar != 0 || ge25519::has_small_order(&a) != 0 {
        return -1;
    }
    let (expected_r, er) = ge25519::frombytes(sigsl);
    if er != 0 || ge25519::has_small_order(&expected_r) != 0 {
        return -1;
    }
    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 32);
    crypto_hash_sha512_update(&mut hs, pk, 32);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, h.as_mut_ptr());
    sc25519::sc_reduce(&mut h);

    let sb_ah_p2 = ge25519::double_scalarmult_vartime(&h[0..32], &a, &sigsl[32..64]);
    let sb_ah = ge25519::p2_to_p3(&sb_ah_p2);
    let check = ge25519::p3_sub(&expected_r, &sb_ah);

    ge25519::has_small_order(&check) - 1
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
    // crypto_sign_ed25519_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX - 64
    if smlen < 64 || smlen - 64 > (usize::MAX as u64) - 64 {
        if !mlen_p.is_null() {
            *mlen_p = 0;
        }
        return -1;
    }
    let mlen = smlen - 64;
    if crypto_sign_ed25519_verify_detached(sm, sm.add(64), mlen, pk) != 0 {
        if !m.is_null() {
            core::ptr::write_bytes(m, 0, mlen as usize);
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
        core::ptr::copy(sm.add(64), m, mlen as usize);
    }
    0
}

/* ---- sign_ed25519.c ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519ph_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_ed25519ph_state>()
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_bytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_seedbytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_publickeybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_secretkeybytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_ed25519_messagebytes_max() -> usize {
    usize::MAX - 64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_seed(seed: *mut u8, sk: *const u8) -> c_int {
    core::ptr::copy(sk, seed, ED25519_SEEDBYTES);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_pk(pk: *mut u8, sk: *const u8) -> c_int {
    core::ptr::copy(sk.add(ED25519_SEEDBYTES), pk, ED25519_PUBLICKEYBYTES);
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
    let mut ph = [0u8; 64];
    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());
    _crypto_sign_ed25519_detached(sig, siglen_p, ph.as_ptr(), ph.len() as u64, sk, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_verify(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    let mut ph = [0u8; 64];
    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());
    _crypto_sign_ed25519_verify_detached(sig, ph.as_ptr(), ph.len() as u64, pk, 1)
}

/* ---- crypto_sign.c (generic dispatch to ed25519) ---- */

pub type crypto_sign_state = crypto_sign_ed25519ph_state;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_state>()
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_messagebytes_max() -> usize {
    usize::MAX - 64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_primitive() -> *const c_char {
    b"ed25519\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_sign_ed25519_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_sign_ed25519_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519(sm, smlen_p, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_open(m, mlen_p, sm, smlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_verify_detached(sig, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_init(state: *mut crypto_sign_state) -> c_int {
    crypto_sign_ed25519ph_init(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_update(
    state: *mut crypto_sign_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    crypto_sign_ed25519ph_update(state, m, mlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_create(
    state: *mut crypto_sign_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519ph_final_create(state, sig, siglen_p, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_verify(
    state: *mut crypto_sign_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519ph_final_verify(state, sig, pk)
}
