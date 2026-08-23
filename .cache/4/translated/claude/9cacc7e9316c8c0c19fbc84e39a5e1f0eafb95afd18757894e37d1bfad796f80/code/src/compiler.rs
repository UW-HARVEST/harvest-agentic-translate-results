//! Translation of common/compiler.h helpers
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

/// ZSTD_isPower2
#[inline(always)]
pub fn ZSTD_isPower2(u: usize) -> core::ffi::c_int {
    ((u & (u.wrapping_sub(1))) == 0) as core::ffi::c_int
}

#[inline(always)]
pub fn ZSTD_wrappedPtrDiff(lhs: *const u8, rhs: *const u8) -> isize {
    (lhs as isize).wrapping_sub(rhs as isize)
}

#[inline(always)]
pub fn ZSTD_wrappedPtrAdd(ptr: *const u8, add: isize) -> *const u8 {
    ptr.wrapping_offset(add)
}

#[inline(always)]
pub fn ZSTD_wrappedPtrSub(ptr: *const u8, sub: isize) -> *const u8 {
    ptr.wrapping_offset(-sub)
}

#[inline(always)]
pub fn ZSTD_maybeNullPtrAdd(ptr: *mut u8, add: isize) -> *mut u8 {
    if add > 0 {
        ptr.wrapping_offset(add)
    } else {
        ptr
    }
}

#[inline(always)]
pub fn ZSTD_maybeNullPtrAddConst(ptr: *const u8, add: isize) -> *const u8 {
    if add > 0 {
        ptr.wrapping_offset(add)
    } else {
        ptr
    }
}

/* PREFETCH_L1 / PREFETCH_L2 are hints only: no-ops here. */
#[inline(always)]
pub fn PREFETCH_L1(_ptr: *const u8) {}
#[inline(always)]
pub fn PREFETCH_L2(_ptr: *const u8) {}
#[inline(always)]
pub fn PREFETCH_AREA(_p: *const u8, _s: usize) {}
