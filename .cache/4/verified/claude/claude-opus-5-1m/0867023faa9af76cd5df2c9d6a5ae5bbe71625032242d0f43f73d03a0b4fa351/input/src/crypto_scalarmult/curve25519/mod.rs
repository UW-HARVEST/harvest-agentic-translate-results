pub mod ref10;

// ---------------------------------------------------------------------------
// Translation of `crypto_scalarmult/curve25519/scalarmult_curve25519.c`
//
// `HAVE_AVX_ASM` is undefined in the reference build, so the sandy2x backend is
// not compiled and `ref10` is the only available implementation.
// ---------------------------------------------------------------------------

use core::ffi::c_int;
use core::ptr;

pub const crypto_scalarmult_curve25519_BYTES: usize = 32;
pub const crypto_scalarmult_curve25519_SCALARBYTES: usize = 32;

/// `typedef struct crypto_scalarmult_curve25519_implementation` from
/// `crypto_scalarmult/curve25519/scalarmult_curve25519.h`.
#[repr(C)]
pub struct crypto_scalarmult_curve25519_implementation {
    pub mult: Option<unsafe extern "C" fn(q: *mut u8, n: *const u8, p: *const u8) -> c_int>,
    pub mult_base: Option<unsafe extern "C" fn(q: *mut u8, n: *const u8) -> c_int>,
}

/// `static const crypto_scalarmult_curve25519_implementation *implementation =
///      &crypto_scalarmult_curve25519_ref10_implementation;`
static mut implementation: *const crypto_scalarmult_curve25519_implementation =
    &raw const ref10::crypto_scalarmult_curve25519_ref10_implementation;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let mut i: usize;
    /* `volatile unsigned char d = 0;` */
    let mut d: u8 = 0;
    unsafe { ptr::write_volatile(&raw mut d, 0u8) };

    if unsafe { ((*implementation).mult.unwrap())(q, n, p) } != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    i = 0;
    while i < crypto_scalarmult_curve25519_BYTES {
        let prev = unsafe { ptr::read_volatile(&raw const d) };
        unsafe { ptr::write_volatile(&raw mut d, prev | *q.add(i)) };
        i += 1;
    }
    let dv = unsafe { ptr::read_volatile(&raw const d) };

    (0 as c_int).wrapping_sub(1 & (((dv as c_int).wrapping_sub(1)) >> 8))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int {
    unsafe {
        ((*(&raw const ref10::crypto_scalarmult_curve25519_ref10_implementation))
            .mult_base
            .unwrap())(q, n)
    }
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
pub unsafe extern "C" fn _crypto_scalarmult_curve25519_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &raw const ref10::crypto_scalarmult_curve25519_ref10_implementation;
    }

    /* `#ifdef HAVE_AVX_ASM` block is not compiled. */
    0
}
