//! Translation of `crypto_verify/verify.c`
//!
//! `HAVE_EMMINTRIN_H`/`__SSE2__` are not both defined in the reference build,
//! so the portable byte-loop implementation is used.

use core::ffi::c_int;

pub const crypto_verify_16_BYTES: usize = 16;
pub const crypto_verify_32_BYTES: usize = 32;
pub const crypto_verify_64_BYTES: usize = 64;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_16_bytes() -> usize {
    crypto_verify_16_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_32_bytes() -> usize {
    crypto_verify_32_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_64_bytes() -> usize {
    crypto_verify_64_BYTES
}

static mut OPTBLOCKER_U16: u16 = 0;

#[inline(always)]
unsafe fn crypto_verify_n(x_: *const u8, y_: *const u8, n: c_int) -> c_int {
    let mut d: u16 = 0;

    for i in 0..n {
        d |= (core::ptr::read_volatile(x_.offset(i as isize))
            ^ core::ptr::read_volatile(y_.offset(i as isize))) as u16;
    }
    d = d.wrapping_sub(1);
    d = ((d >> 13) ^ core::ptr::read_volatile(&raw const OPTBLOCKER_U16)) >> 2;

    (d as c_int) - 1
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
