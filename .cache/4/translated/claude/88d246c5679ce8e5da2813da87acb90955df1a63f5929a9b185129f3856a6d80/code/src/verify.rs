//! Translation of `c_src/libsodium/crypto_verify/verify.c`.
//!
//! Neither `HAVE_EMMINTRIN_H` nor `__SSE2__` is defined in the reference build,
//! so the `#else` (portable) `crypto_verify_n()` is compiled: the one using a
//! `volatile uint16_t d` accumulator plus the private
//! `static volatile uint16_t optblocker_u16` optimisation barrier.
//! `HAVE_INLINE_ASM` is not defined either, so the `__asm__ __volatile__`
//! barrier between the loop and `d--` is absent.

use core::ffi::c_int;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU16, Ordering};

/* #define crypto_verify_16_BYTES 16U */
const crypto_verify_16_BYTES: usize = 16;
/* #define crypto_verify_32_BYTES 32U */
const crypto_verify_32_BYTES: usize = 32;
/* #define crypto_verify_64_BYTES 64U */
const crypto_verify_64_BYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_16_bytes() -> usize {
    crypto_verify_16_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_32_bytes() -> usize {
    crypto_verify_32_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_64_bytes() -> usize {
    crypto_verify_64_BYTES
}

/// `static volatile uint16_t optblocker_u16;`
///
/// Private (non-exported) symbol, permanently zero, read on every call so that
/// the compiler cannot fold the final shift chain into a branch.
static optblocker_u16: AtomicU16 = AtomicU16::new(0);

/// `static inline int crypto_verify_n(const unsigned char *x_,
///                                   const unsigned char *y_, const int n)`
#[inline]
unsafe fn crypto_verify_n(x_: *const u8, y_: *const u8, n: c_int) -> c_int {
    /*
     * const volatile unsigned char *volatile x = ... x_;
     * const volatile unsigned char *volatile y = ... y_;
     */
    let x: *const u8 = x_;
    let y: *const u8 = y_;

    /* volatile uint16_t d = 0U; */
    let mut d_storage: u16 = 0;
    let d: *mut u16 = &mut d_storage;

    let mut i: c_int = 0;
    while i < n {
        /* d |= x[i] ^ y[i]; */
        let xi = read_volatile(x.offset(i as isize));
        let yi = read_volatile(y.offset(i as isize));
        write_volatile(d, read_volatile(d) | ((xi ^ yi) as u16));
        i += 1;
    }

    /* HAVE_INLINE_ASM undefined: no `__asm__ __volatile__("" : "+r"(d) :);` */

    /* d--;  (wraps: 0 -> 0xFFFF) */
    write_volatile(d, read_volatile(d).wrapping_sub(1));

    /*
     * d = ((d >> 13) ^ optblocker_u16) >> 2;
     *
     * Both operands are promoted to `int` in C; every intermediate value here is
     * in 0..=7, so computing in `u32` and truncating back to `u16` is exact.
     */
    let t: u32 = ((read_volatile(d) as u32) >> 13) ^ (optblocker_u16.load(Ordering::Relaxed) as u32);
    write_volatile(d, (t >> 2) as u16);

    /* return (int) d - 1; */
    (read_volatile(d) as c_int) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, crypto_verify_16_BYTES as c_int)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, crypto_verify_32_BYTES as c_int)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, crypto_verify_64_BYTES as c_int)
}
