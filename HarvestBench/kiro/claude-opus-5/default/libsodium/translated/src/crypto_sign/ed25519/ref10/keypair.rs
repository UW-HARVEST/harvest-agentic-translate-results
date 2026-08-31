//! Translation of c_src/libsodium/crypto_sign/ed25519/ref10/keypair.c

use core::ffi::{c_int, c_void};

use crate::fe25519::{fe25519, fe25519_1, fe25519_add, fe25519_sub, fe25519_mul, ge25519_p3};

// crypto_hash_sha512_BYTES
const crypto_hash_sha512_BYTES: usize = 64;
// crypto_scalarmult_curve25519_BYTES
const crypto_scalarmult_curve25519_BYTES: usize = 32;

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    fn _sodium_ge25519_frombytes_negate_vartime(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    fn _sodium_fe25519_invert(out: *mut i32, z: *const i32);
    fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32);
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);

    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut a = core::mem::MaybeUninit::<ge25519_p3>::uninit();

    crypto_hash_sha512(sk, seed, 32);
    *sk.add(0) &= 248;
    *sk.add(31) &= 127;
    *sk.add(31) |= 64;

    _sodium_ge25519_scalarmult_base(a.as_mut_ptr(), sk);
    _sodium_ge25519_p3_tobytes(pk, a.as_ptr());

    memmove(sk as *mut c_void, seed as *const c_void, 32);
    memmove(sk.add(32) as *mut c_void, pk as *const c_void, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    let mut seed: [u8; 32] = [0; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 32]>());
    ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 32]>());

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_pk_to_curve25519(
    curve25519_pk: *mut u8,
    ed25519_pk: *const u8,
) -> c_int {
    let mut a = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut x: fe25519 = [0; 10];
    let mut one_minus_y: fe25519 = [0; 10];

    if _sodium_ge25519_frombytes_negate_vartime(a.as_mut_ptr(), ed25519_pk) != 0
        || _sodium_ge25519_has_small_order(a.as_ptr()) != 0
        || _sodium_ge25519_is_on_main_subgroup(a.as_ptr()) == 0
    {
        return -1;
    }
    fe25519_1(one_minus_y.as_mut_ptr());
    // assumes A.Z=1
    fe25519_sub(
        one_minus_y.as_mut_ptr(),
        one_minus_y.as_ptr(),
        core::ptr::addr_of!((*a.as_ptr()).Y) as *const i32,
    );
    fe25519_1(x.as_mut_ptr());
    fe25519_add(
        x.as_mut_ptr(),
        x.as_ptr(),
        core::ptr::addr_of!((*a.as_ptr()).Y) as *const i32,
    );
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
    memcpy(
        curve25519_sk as *mut c_void,
        h.as_ptr() as *const c_void,
        crypto_scalarmult_curve25519_BYTES,
    );
    sodium_memzero(h.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; crypto_hash_sha512_BYTES]>());

    0
}
