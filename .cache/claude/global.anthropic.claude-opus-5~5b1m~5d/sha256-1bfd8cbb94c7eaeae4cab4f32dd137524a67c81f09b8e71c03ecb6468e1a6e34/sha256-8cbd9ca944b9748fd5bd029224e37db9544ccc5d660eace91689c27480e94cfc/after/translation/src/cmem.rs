//! Translation of `common/mem.h`, `common/zstd_deps.h` helpers and libc glue.
#![allow(dead_code)]

use core::ffi::c_void;

pub type BYTE = u8;
pub type U8 = u8;
pub type S8 = i8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(n: usize, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32;
    pub fn strlen(s: *const core::ffi::c_char) -> usize;
}

/* <stdio.h> / <time.h> glue, used by the dictBuilder `DISPLAY*` macros.
 * On glibc `stderr` is a real `FILE*` object symbol. */
unsafe extern "C" {
    pub static mut stderr: *mut c_void;
    pub fn fprintf(
        stream: *mut c_void,
        fmt: *const core::ffi::c_char,
        ...
    ) -> core::ffi::c_int;
    pub fn fflush(stream: *mut c_void) -> core::ffi::c_int;
    pub fn clock() -> core::ffi::c_long;
}

#[inline(always)]
pub unsafe fn ZSTD_memcpy(dst: *mut c_void, src: *const c_void, n: usize) {
    if n != 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, n);
    }
}

#[inline(always)]
pub unsafe fn ZSTD_memmove(dst: *mut c_void, src: *const c_void, n: usize) {
    if n != 0 {
        core::ptr::copy(src as *const u8, dst as *mut u8, n);
    }
}

#[inline(always)]
pub unsafe fn ZSTD_memset(dst: *mut c_void, c: i32, n: usize) {
    if n != 0 {
        core::ptr::write_bytes(dst as *mut u8, c as u8, n);
    }
}

#[inline(always)]
pub unsafe fn ZSTD_memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    memcmp(a, b, n)
}

/* ===== platform detection ===== */
#[inline(always)]
pub fn MEM_32bits() -> u32 {
    (core::mem::size_of::<usize>() == 4) as u32
}
#[inline(always)]
pub fn MEM_64bits() -> u32 {
    (core::mem::size_of::<usize>() == 8) as u32
}
#[inline(always)]
pub fn MEM_isLittleEndian() -> u32 {
    if cfg!(target_endian = "little") {
        1
    } else {
        0
    }
}

/* ===== native unaligned read/write ===== */
#[inline(always)]
pub unsafe fn MEM_read16(p: *const c_void) -> U16 {
    (p as *const U16).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_read32(p: *const c_void) -> U32 {
    (p as *const U32).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_read64(p: *const c_void) -> U64 {
    (p as *const U64).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_readST(p: *const c_void) -> usize {
    (p as *const usize).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_write16(p: *mut c_void, v: U16) {
    (p as *mut U16).write_unaligned(v)
}
#[inline(always)]
pub unsafe fn MEM_write32(p: *mut c_void, v: U32) {
    (p as *mut U32).write_unaligned(v)
}
#[inline(always)]
pub unsafe fn MEM_write64(p: *mut c_void, v: U64) {
    (p as *mut U64).write_unaligned(v)
}

/* ===== byteswap ===== */
#[inline(always)]
pub fn MEM_swap32(v: U32) -> U32 {
    v.swap_bytes()
}
#[inline(always)]
pub fn MEM_swap64(v: U64) -> U64 {
    v.swap_bytes()
}
#[inline(always)]
pub fn MEM_swapST(v: usize) -> usize {
    v.swap_bytes()
}

/* ===== little-endian ===== */
#[inline(always)]
pub unsafe fn MEM_readLE16(p: *const c_void) -> U16 {
    U16::from_le(MEM_read16(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE16(p: *mut c_void, v: U16) {
    MEM_write16(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLE24(p: *const c_void) -> U32 {
    MEM_readLE16(p) as U32 + ((*(p as *const BYTE).add(2) as U32) << 16)
}
#[inline(always)]
pub unsafe fn MEM_writeLE24(p: *mut c_void, v: U32) {
    MEM_writeLE16(p, v as U16);
    *(p as *mut BYTE).add(2) = (v >> 16) as BYTE;
}
#[inline(always)]
pub unsafe fn MEM_readLE32(p: *const c_void) -> U32 {
    U32::from_le(MEM_read32(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE32(p: *mut c_void, v: U32) {
    MEM_write32(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLE64(p: *const c_void) -> U64 {
    U64::from_le(MEM_read64(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE64(p: *mut c_void, v: U64) {
    MEM_write64(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLEST(p: *const c_void) -> usize {
    usize::from_le(MEM_readST(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLEST(p: *mut c_void, v: usize) {
    (p as *mut usize).write_unaligned(v.to_le())
}

/* ===== big-endian ===== */
#[inline(always)]
pub unsafe fn MEM_readBE32(p: *const c_void) -> U32 {
    U32::from_be(MEM_read32(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBE32(p: *mut c_void, v: U32) {
    MEM_write32(p, v.to_be())
}
#[inline(always)]
pub unsafe fn MEM_readBE64(p: *const c_void) -> U64 {
    U64::from_be(MEM_read64(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBE64(p: *mut c_void, v: U64) {
    MEM_write64(p, v.to_be())
}
#[inline(always)]
pub unsafe fn MEM_readBEST(p: *const c_void) -> usize {
    usize::from_be(MEM_readST(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBEST(p: *mut c_void, v: usize) {
    (p as *mut usize).write_unaligned(v.to_be())
}
