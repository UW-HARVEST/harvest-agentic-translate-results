//! Translation of `include/sodium/private/common.h` helpers plus libc glue.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_long, c_void};

// ---------------------------------------------------------------------------
// libc glue (no external crates available)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(n: usize, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn syscall(num: c_long, ...) -> c_long;
    pub fn __errno_location() -> *mut c_int;
}

pub const EPERM: c_int = 1;
pub const ENOENT: c_int = 2;
pub const EINTR: c_int = 4;
pub const EIO: c_int = 5;
pub const EAGAIN: c_int = 11;
pub const ENOMEM: c_int = 12;
pub const EINVAL: c_int = 22;
pub const ERANGE: c_int = 34;
pub const ENOSYS: c_int = 38;

#[inline]
pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v };
}

#[inline]
pub fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

pub const SIZE_MAX: usize = usize::MAX;
/// `SODIUM_SIZE_MAX` == min(UINT64_MAX, SIZE_MAX)
pub const SODIUM_SIZE_MAX: u64 = if (u64::MAX as u128) < (usize::MAX as u128) {
    u64::MAX
} else {
    usize::MAX as u64
};

// ---------------------------------------------------------------------------
// rotations
// ---------------------------------------------------------------------------

#[inline(always)]
pub const fn rotl32(x: u32, b: i32) -> u32 {
    (x << b) | (x >> (32 - b))
}

#[inline(always)]
pub const fn rotl64(x: u64, b: i32) -> u64 {
    (x << b) | (x >> (64 - b))
}

#[inline(always)]
pub const fn rotr32(x: u32, b: i32) -> u32 {
    (x >> b) | (x << (32 - b))
}

#[inline(always)]
pub const fn rotr64(x: u64, b: i32) -> u64 {
    (x >> b) | (x << (64 - b))
}

// ---------------------------------------------------------------------------
// load / store
// ---------------------------------------------------------------------------

#[inline(always)]
pub unsafe fn load64_le(src: *const u8) -> u64 {
    let mut w = 0u64;
    for i in 0..8 {
        w |= (unsafe { *src.add(i) } as u64) << (8 * i);
    }
    w
}

#[inline(always)]
pub unsafe fn store64_le(dst: *mut u8, w: u64) {
    for i in 0..8 {
        unsafe { *dst.add(i) = (w >> (8 * i)) as u8 };
    }
}

#[inline(always)]
pub unsafe fn load32_le(src: *const u8) -> u32 {
    let mut w = 0u32;
    for i in 0..4 {
        w |= (unsafe { *src.add(i) } as u32) << (8 * i);
    }
    w
}

#[inline(always)]
pub unsafe fn store32_le(dst: *mut u8, w: u32) {
    for i in 0..4 {
        unsafe { *dst.add(i) = (w >> (8 * i)) as u8 };
    }
}

#[inline(always)]
pub unsafe fn load64_be(src: *const u8) -> u64 {
    let mut w = 0u64;
    for i in 0..8 {
        w |= (unsafe { *src.add(i) } as u64) << (8 * (7 - i));
    }
    w
}

#[inline(always)]
pub unsafe fn store64_be(dst: *mut u8, w: u64) {
    for i in 0..8 {
        unsafe { *dst.add(i) = (w >> (8 * (7 - i))) as u8 };
    }
}

#[inline(always)]
pub unsafe fn load32_be(src: *const u8) -> u32 {
    let mut w = 0u32;
    for i in 0..4 {
        w |= (unsafe { *src.add(i) } as u32) << (8 * (3 - i));
    }
    w
}

#[inline(always)]
pub unsafe fn store32_be(dst: *mut u8, w: u32) {
    for i in 0..4 {
        unsafe { *dst.add(i) = (w >> (8 * (3 - i))) as u8 };
    }
}

#[inline(always)]
pub unsafe fn xor_buf(out: *mut u8, inp: *const u8, n: usize) {
    for i in 0..n {
        unsafe { *out.add(i) ^= *inp.add(i) };
    }
}

// safe-slice helpers -------------------------------------------------------

#[inline(always)]
pub fn ld32_le(s: &[u8]) -> u32 {
    u32::from_le_bytes([s[0], s[1], s[2], s[3]])
}

#[inline(always)]
pub fn ld64_le(s: &[u8]) -> u64 {
    u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
}

#[inline(always)]
pub fn ld32_be(s: &[u8]) -> u32 {
    u32::from_be_bytes([s[0], s[1], s[2], s[3]])
}

#[inline(always)]
pub fn ld64_be(s: &[u8]) -> u64 {
    u64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
}

#[inline(always)]
pub fn st32_le(d: &mut [u8], w: u32) {
    d[..4].copy_from_slice(&w.to_le_bytes());
}

#[inline(always)]
pub fn st64_le(d: &mut [u8], w: u64) {
    d[..8].copy_from_slice(&w.to_le_bytes());
}

#[inline(always)]
pub fn st32_be(d: &mut [u8], w: u32) {
    d[..4].copy_from_slice(&w.to_be_bytes());
}

#[inline(always)]
pub fn st64_be(d: &mut [u8], w: u64) {
    d[..8].copy_from_slice(&w.to_be_bytes());
}

/// `memcpy` for raw pointers.
#[inline(always)]
pub unsafe fn memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        unsafe { core::ptr::copy_nonoverlapping(src, dst, n) };
    }
}

/// `memmove` for raw pointers.
#[inline(always)]
pub unsafe fn memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        unsafe { core::ptr::copy(src, dst, n) };
    }
}

/// `memset` for raw pointers.
#[inline(always)]
pub unsafe fn memset(dst: *mut u8, v: u8, n: usize) {
    if n != 0 {
        unsafe { core::ptr::write_bytes(dst, v, n) };
    }
}
