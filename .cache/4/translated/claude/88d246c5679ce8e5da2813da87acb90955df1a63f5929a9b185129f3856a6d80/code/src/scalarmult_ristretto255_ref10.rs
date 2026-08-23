//! Translation of
//! `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c`.
//!
//! The `ge25519_*` / `ristretto255_*` helpers live in
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` and are reached through the
//! linker under their `private/quirks.h` names.

use core::ffi::{c_int, c_uint};

/* `typedef int32_t fe25519[10];` (HAVE_TI_MODE is unset) */
type Fe = [i32; 10];

/* `ge25519_p3` from `private/ed25519_ref10.h` */
#[repr(C)]
#[derive(Copy, Clone)]
struct ge25519_p3 {
    X: Fe,
    Y: Fe,
    Z: Fe,
    T: Fe,
}

impl ge25519_p3 {
    const fn new() -> Self {
        ge25519_p3 { X: [0; 10], Y: [0; 10], Z: [0; 10], T: [0; 10] }
    }
}

extern "C" {
    /* crypto_core/ed25519/ref10/ed25519_ref10.c */
    fn _sodium_ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    fn _sodium_ge25519_scalarmult(
        h: *mut ge25519_p3,
        a: *const u8,
        p: *const ge25519_p3,
    );
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    /* sodium/utils.c */
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

/* crypto_scalarmult_ristretto255.h */
const crypto_scalarmult_ristretto255_BYTES: usize = 32;
const crypto_scalarmult_ristretto255_SCALARBYTES: usize = 32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let t: *mut u8 = q;
    let mut Q = ge25519_p3::new();
    let mut P = ge25519_p3::new();
    let Qp: *mut ge25519_p3 = &mut Q;
    let Pp: *mut ge25519_p3 = &mut P;
    let mut i: c_uint;

    if _sodium_ristretto255_frombytes(Pp, p) != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(31) &= 127;
    _sodium_ge25519_scalarmult(Qp, t, Pp);
    _sodium_ristretto255_p3_tobytes(q, Qp);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    let t: *mut u8 = q;
    let mut Q = ge25519_p3::new();
    let Qp: *mut ge25519_p3 = &mut Q;
    let mut i: c_uint;

    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(31) &= 127;
    _sodium_ge25519_scalarmult_base(Qp, t);
    _sodium_ristretto255_p3_tobytes(q, Qp);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_bytes() -> usize {
    crypto_scalarmult_ristretto255_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_scalarbytes() -> usize {
    crypto_scalarmult_ristretto255_SCALARBYTES
}
