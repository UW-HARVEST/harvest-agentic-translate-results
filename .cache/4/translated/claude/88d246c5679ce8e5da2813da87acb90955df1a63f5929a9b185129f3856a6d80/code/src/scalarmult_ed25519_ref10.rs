//! Translation of `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c`.
//!
//! The `ge25519_*` helpers live in
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
    fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_scalarmult(
        h: *mut ge25519_p3,
        a: *const u8,
        p: *const ge25519_p3,
    );
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    /* sodium/utils.c */
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

/* crypto_scalarmult_ed25519.h */
const crypto_scalarmult_ed25519_BYTES: usize = 32;
const crypto_scalarmult_ed25519_SCALARBYTES: usize = 32;

unsafe fn _crypto_scalarmult_ed25519_is_inf(s: *const u8) -> c_int {
    let mut c: u8;
    let mut i: c_uint;

    c = *s.add(0) ^ 0x01;
    i = 1;
    while i < 31 {
        c |= *s.add(i as usize);
        i += 1;
    }
    c |= *s.add(31) & 0x7f;

    ((((c as c_uint).wrapping_sub(1u32)) >> 8) & 1) as c_int
}

#[inline]
unsafe fn _crypto_scalarmult_ed25519_clamp(k: *mut u8) {
    *k.add(0) &= 248;
    *k.add(31) |= 64;
}

unsafe fn _crypto_scalarmult_ed25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
    clamp: c_int,
) -> c_int {
    let t: *mut u8 = q;
    let mut Q = ge25519_p3::new();
    let mut P = ge25519_p3::new();
    let Qp: *mut ge25519_p3 = &mut Q;
    let Pp: *mut ge25519_p3 = &mut P;
    let mut i: c_uint;

    if _sodium_ge25519_is_canonical(p) == 0
        || _sodium_ge25519_frombytes(Pp, p) != 0
        || _sodium_ge25519_has_small_order(Pp) != 0
        || _sodium_ge25519_is_on_main_subgroup(Pp) == 0
    {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    if clamp != 0 {
        _crypto_scalarmult_ed25519_clamp(t);
    }
    *t.add(31) &= 127;

    _sodium_ge25519_scalarmult(Qp, t, Pp);
    _sodium_ge25519_p3_tobytes(q, Qp);
    if _crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519(q, n, p, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_noclamp(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519(q, n, p, 0)
}

unsafe fn _crypto_scalarmult_ed25519_base(
    q: *mut u8,
    n: *const u8,
    clamp: c_int,
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
    if clamp != 0 {
        _crypto_scalarmult_ed25519_clamp(t);
    }
    *t.add(31) &= 127;

    _sodium_ge25519_scalarmult_base(Qp, t);
    _sodium_ge25519_p3_tobytes(q, Qp);
    if _crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519_base(q, n, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base_noclamp(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519_base(q, n, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_bytes() -> usize {
    crypto_scalarmult_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_scalarbytes() -> usize {
    crypto_scalarmult_ed25519_SCALARBYTES
}
