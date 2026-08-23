//! crypto_scalarmult dispatch (crypto_scalarmult.c, scalarmult_curve25519.c,
//! scalarmult_ed25519_ref10.c, scalarmult_ristretto255_ref10.c).
use crate::ed25519::ge25519;
use crate::ed25519::x25519::{crypto_scalarmult_curve25519_ref10_implementation,
    Curve25519Implementation};
use core::ffi::{c_char, c_int};

extern "C" {
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

static mut IMPLEMENTATION: *const Curve25519Implementation =
    &crypto_scalarmult_curve25519_ref10_implementation;

/* ---- crypto_scalarmult_curve25519.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let imp = &*IMPLEMENTATION;
    if (imp.mult)(q, n, p) != 0 {
        return -1;
    }
    let mut d: u8 = 0;
    for i in 0..32 {
        d |= *q.add(i);
    }
    -(1 & (((d as i32) - 1) >> 8))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int {
    (crypto_scalarmult_curve25519_ref10_implementation.mult_base)(q, n)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_curve25519_bytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_curve25519_scalarbytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_scalarmult_curve25519_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &crypto_scalarmult_curve25519_ref10_implementation;
    0
}

/* ---- crypto_scalarmult.c ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_primitive() -> *const c_char {
    b"curve25519\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    crypto_scalarmult_curve25519(q, n, p)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_bytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_scalarbytes() -> usize {
    32
}

/* ---- scalarmult_ed25519_ref10.c ---- */

fn ed25519_is_inf(s: &[u8]) -> i32 {
    let mut c = s[0] ^ 0x01;
    for i in 1..31 {
        c |= s[i];
    }
    c |= s[31] & 0x7f;
    ((((c as u32).wrapping_sub(1)) >> 8) & 1) as i32
}

fn ed25519_clamp(k: &mut [u8]) {
    k[0] &= 248;
    k[31] |= 64;
}

unsafe fn scalarmult_ed25519_inner(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
    clamp: i32,
) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    if ge25519::is_canonical(psl) == 0 {
        return -1;
    }
    let (bp, r) = ge25519::frombytes(psl);
    if r != 0 || ge25519::has_small_order(&bp) != 0 || ge25519::is_on_main_subgroup(&bp) == 0 {
        return -1;
    }
    let mut t = [0u8; 32];
    for i in 0..32 {
        t[i] = *n.add(i);
    }
    if clamp != 0 {
        ed25519_clamp(&mut t);
    }
    t[31] &= 127;

    let big_q = ge25519::scalarmult(&t, &bp);
    let out = ge25519::p3_tobytes(&big_q);
    core::ptr::copy_nonoverlapping(out.as_ptr(), q, 32);
    let qsl = core::slice::from_raw_parts(q, 32);
    if ed25519_is_inf(qsl) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

unsafe fn scalarmult_ed25519_base_inner(q: *mut u8, n: *const u8, clamp: i32) -> c_int {
    let mut t = [0u8; 32];
    for i in 0..32 {
        t[i] = *n.add(i);
    }
    if clamp != 0 {
        ed25519_clamp(&mut t);
    }
    t[31] &= 127;

    let big_q = ge25519::scalarmult_base(&t);
    let out = ge25519::p3_tobytes(&big_q);
    core::ptr::copy_nonoverlapping(out.as_ptr(), q, 32);
    let qsl = core::slice::from_raw_parts(q, 32);
    if ed25519_is_inf(qsl) != 0 || sodium_is_zero(n, 32) != 0 {
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
    scalarmult_ed25519_inner(q, n, p, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_noclamp(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    scalarmult_ed25519_inner(q, n, p, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base(q: *mut u8, n: *const u8) -> c_int {
    scalarmult_ed25519_base_inner(q, n, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base_noclamp(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    scalarmult_ed25519_base_inner(q, n, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ed25519_bytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ed25519_scalarbytes() -> usize {
    32
}

/* ---- scalarmult_ristretto255_ref10.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    let (bp, r) = crate::ed25519::ristretto255::frombytes(psl);
    if r != 0 {
        return -1;
    }
    let mut t = [0u8; 32];
    for i in 0..32 {
        t[i] = *n.add(i);
    }
    t[31] &= 127;
    let big_q = ge25519::scalarmult(&t, &bp);
    let out = crate::ed25519::ristretto255::p3_tobytes(&big_q);
    core::ptr::copy_nonoverlapping(out.as_ptr(), q, 32);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_base(q: *mut u8, n: *const u8) -> c_int {
    let mut t = [0u8; 32];
    for i in 0..32 {
        t[i] = *n.add(i);
    }
    t[31] &= 127;
    let big_q = ge25519::scalarmult_base(&t);
    let out = crate::ed25519::ristretto255::p3_tobytes(&big_q);
    core::ptr::copy_nonoverlapping(out.as_ptr(), q, 32);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ristretto255_bytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ristretto255_scalarbytes() -> usize {
    32
}
