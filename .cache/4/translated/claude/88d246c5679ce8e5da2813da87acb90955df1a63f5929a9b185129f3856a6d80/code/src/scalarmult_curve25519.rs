//! Translation of `crypto_scalarmult/curve25519/scalarmult_curve25519.c`.
//!
//! `HAVE_AVX_ASM` is not defined in the reference build, so the sandy2x
//! implementation does not exist and `ref10` is always selected.

use core::ffi::c_int;

/* `struct crypto_scalarmult_curve25519_implementation` from the private header
 * `crypto_scalarmult/curve25519/scalarmult_curve25519.h` (duplicated here). */
#[repr(C)]
pub struct crypto_scalarmult_curve25519_implementation {
    pub mult:
        unsafe extern "C" fn(q: *mut u8, n: *const u8, p: *const u8) -> c_int,
    pub mult_base: unsafe extern "C" fn(q: *mut u8, n: *const u8) -> c_int,
}

extern "C" {
    /* crypto_scalarmult/curve25519/ref10/x25519_ref10.c */
    static crypto_scalarmult_curve25519_ref10_implementation:
        crypto_scalarmult_curve25519_implementation;
}

/* crypto_scalarmult_curve25519.h */
const crypto_scalarmult_curve25519_BYTES: usize = 32;
const crypto_scalarmult_curve25519_SCALARBYTES: usize = 32;

static mut implementation: *const crypto_scalarmult_curve25519_implementation =
    unsafe { &crypto_scalarmult_curve25519_ref10_implementation };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let mut i: usize;
    /* `volatile unsigned char d = 0;` */
    let mut d: u8 = 0;
    let dp: *mut u8 = &mut d;

    core::ptr::write_volatile(dp, 0);
    if ((*implementation).mult)(q, n, p) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    i = 0;
    while i < crypto_scalarmult_curve25519_BYTES {
        core::ptr::write_volatile(dp, core::ptr::read_volatile(dp) | *q.add(i));
        i += 1;
    }
    -(1 & (((core::ptr::read_volatile(dp) as c_int) - 1) >> 8))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    (crypto_scalarmult_curve25519_ref10_implementation.mult_base)(q, n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_bytes() -> usize {
    crypto_scalarmult_curve25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_scalarbytes() -> usize {
    crypto_scalarmult_curve25519_SCALARBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_scalarmult_curve25519_pick_best_implementation(
) -> c_int {
    implementation = &crypto_scalarmult_curve25519_ref10_implementation;

    /* `#ifdef HAVE_AVX_ASM` — not defined in the reference build, so the
     * `sodium_runtime_has_avx()` / sandy2x branch is preprocessed away. */

    0
}
