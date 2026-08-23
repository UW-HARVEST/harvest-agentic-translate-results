//! `crypto_verify/verify.c`
//!
//! `HAVE_EMMINTRIN_H`/`__SSE2__` are not part of the reference build, so the
//! portable `crypto_verify_n()` is used.

use core::ffi::c_int;
use core::ptr;

pub const CRYPTO_VERIFY_16_BYTES: usize = 16;
pub const CRYPTO_VERIFY_32_BYTES: usize = 32;
pub const CRYPTO_VERIFY_64_BYTES: usize = 64;

static mut OPTBLOCKER_U16: u16 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_16_bytes() -> usize {
    CRYPTO_VERIFY_16_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_32_bytes() -> usize {
    CRYPTO_VERIFY_32_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_64_bytes() -> usize {
    CRYPTO_VERIFY_64_BYTES
}

#[inline]
unsafe fn crypto_verify_n(x: *const u8, y: *const u8, n: c_int) -> c_int {
    let mut d: u16 = 0;
    for i in 0..n as usize {
        d |= (unsafe { ptr::read_volatile(x.add(i)) } ^ unsafe { ptr::read_volatile(y.add(i)) })
            as u16;
    }
    d = d.wrapping_sub(1);
    d = ((d >> 13) ^ unsafe { ptr::read_volatile(&raw const OPTBLOCKER_U16) }) >> 2;

    (d as c_int) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int {
    unsafe { crypto_verify_n(x, y, CRYPTO_VERIFY_16_BYTES as c_int) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int {
    unsafe { crypto_verify_n(x, y, CRYPTO_VERIFY_32_BYTES as c_int) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int {
    unsafe { crypto_verify_n(x, y, CRYPTO_VERIFY_64_BYTES as c_int) }
}
