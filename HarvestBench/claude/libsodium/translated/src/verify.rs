//! Translated from crypto_verify/verify.c (portable non-SSE2 path)
use core::ffi::c_int;

pub const CRYPTO_VERIFY_16_BYTES: usize = 16;
pub const CRYPTO_VERIFY_32_BYTES: usize = 32;
pub const CRYPTO_VERIFY_64_BYTES: usize = 64;

static mut OPTBLOCKER_U16: u16 = 0;

#[inline]
fn crypto_verify_n(x: *const u8, y: *const u8, n: i32) -> c_int {
    let mut d: u16 = 0;
    unsafe {
        for i in 0..n as isize {
            d |= (*x.offset(i) ^ *y.offset(i)) as u16;
        }
        d = d.wrapping_sub(1);
        d = ((d >> 13) ^ core::ptr::read_volatile(&raw const OPTBLOCKER_U16)) >> 2;
    }
    d as c_int - 1
}

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

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, CRYPTO_VERIFY_16_BYTES as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, CRYPTO_VERIFY_32_BYTES as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int {
    crypto_verify_n(x, y, CRYPTO_VERIFY_64_BYTES as i32)
}
