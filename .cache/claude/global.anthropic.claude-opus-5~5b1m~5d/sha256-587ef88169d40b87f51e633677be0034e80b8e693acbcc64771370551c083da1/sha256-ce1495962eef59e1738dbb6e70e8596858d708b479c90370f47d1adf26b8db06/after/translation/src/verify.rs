//! Translation of `crypto_verify/verify.c`.
//!
//! The reference build defines neither `HAVE_EMMINTRIN_H`/`__SSE2__` nor
//! `HAVE_INLINE_ASM`, so this always takes the portable `crypto_verify_n`
//! path (no SSE2, no inline asm barrier).

use core::ffi::c_int;

const CRYPTO_VERIFY_16_BYTES: c_int = 16;
const CRYPTO_VERIFY_32_BYTES: c_int = 32;
const CRYPTO_VERIFY_64_BYTES: c_int = 64;

#[no_mangle]
pub unsafe extern "C" fn crypto_verify_16_bytes() -> usize {
    CRYPTO_VERIFY_16_BYTES as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_verify_32_bytes() -> usize {
    CRYPTO_VERIFY_32_BYTES as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_verify_64_bytes() -> usize {
    CRYPTO_VERIFY_64_BYTES as usize
}

/// `static volatile uint16_t optblocker_u16;` in the C source.
static mut OPTBLOCKER_U16: u16 = 0;

#[inline]
unsafe fn crypto_verify_n(x_: *const u8, y_: *const u8, n: c_int) -> c_int {
    let x = x_;
    let y = y_;
    let mut d: u16 = 0;
    let mut i: c_int = 0;

    while i < n {
        d |= (*x.add(i as usize) ^ *y.add(i as usize)) as u16;
        i += 1;
    }

    d = d.wrapping_sub(1);
    let blocker = core::ptr::read_volatile(&raw const OPTBLOCKER_U16);
    d = ((d >> 13) ^ blocker) >> 2;

    (d as c_int) - 1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, CRYPTO_VERIFY_16_BYTES)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, CRYPTO_VERIFY_32_BYTES)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, CRYPTO_VERIFY_64_BYTES)
}
