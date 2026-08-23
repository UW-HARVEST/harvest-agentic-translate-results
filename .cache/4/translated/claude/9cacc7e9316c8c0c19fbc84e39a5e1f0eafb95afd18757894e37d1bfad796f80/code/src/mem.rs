//! Translation of common/mem.h
#![allow(non_snake_case, dead_code)]

pub type BYTE = u8;
pub type U8 = u8;
pub type S8 = i8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

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

#[inline(always)]
pub unsafe fn MEM_read16(p: *const u8) -> u16 {
    core::ptr::read_unaligned(p as *const u16)
}
#[inline(always)]
pub unsafe fn MEM_read32(p: *const u8) -> u32 {
    core::ptr::read_unaligned(p as *const u32)
}
#[inline(always)]
pub unsafe fn MEM_read64(p: *const u8) -> u64 {
    core::ptr::read_unaligned(p as *const u64)
}
#[inline(always)]
pub unsafe fn MEM_readST(p: *const u8) -> usize {
    core::ptr::read_unaligned(p as *const usize)
}

#[inline(always)]
pub unsafe fn MEM_write16(p: *mut u8, v: u16) {
    core::ptr::write_unaligned(p as *mut u16, v)
}
#[inline(always)]
pub unsafe fn MEM_write32(p: *mut u8, v: u32) {
    core::ptr::write_unaligned(p as *mut u32, v)
}
#[inline(always)]
pub unsafe fn MEM_write64(p: *mut u8, v: u64) {
    core::ptr::write_unaligned(p as *mut u64, v)
}

#[inline(always)]
pub fn MEM_swap32(x: u32) -> u32 {
    x.swap_bytes()
}
#[inline(always)]
pub fn MEM_swap64(x: u64) -> u64 {
    x.swap_bytes()
}
#[inline(always)]
pub fn MEM_swapST(x: usize) -> usize {
    x.swap_bytes()
}

#[inline(always)]
pub unsafe fn MEM_readLE16(p: *const u8) -> u16 {
    u16::from_le(MEM_read16(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE16(p: *mut u8, v: u16) {
    MEM_write16(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLE24(p: *const u8) -> u32 {
    MEM_readLE16(p) as u32 + ((*p.add(2) as u32) << 16)
}
#[inline(always)]
pub unsafe fn MEM_writeLE24(p: *mut u8, v: u32) {
    MEM_writeLE16(p, v as u16);
    *p.add(2) = (v >> 16) as u8;
}
#[inline(always)]
pub unsafe fn MEM_readLE32(p: *const u8) -> u32 {
    u32::from_le(MEM_read32(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE32(p: *mut u8, v: u32) {
    MEM_write32(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLE64(p: *const u8) -> u64 {
    u64::from_le(MEM_read64(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE64(p: *mut u8, v: u64) {
    MEM_write64(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLEST(p: *const u8) -> usize {
    usize::from_le(MEM_readST(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLEST(p: *mut u8, v: usize) {
    core::ptr::write_unaligned(p as *mut usize, v.to_le())
}

#[inline(always)]
pub unsafe fn MEM_readBE32(p: *const u8) -> u32 {
    u32::from_be(MEM_read32(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBE32(p: *mut u8, v: u32) {
    MEM_write32(p, v.to_be())
}
#[inline(always)]
pub unsafe fn MEM_readBE64(p: *const u8) -> u64 {
    u64::from_be(MEM_read64(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBE64(p: *mut u8, v: u64) {
    MEM_write64(p, v.to_be())
}
#[inline(always)]
pub unsafe fn MEM_readBEST(p: *const u8) -> usize {
    usize::from_be(MEM_readST(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBEST(p: *mut u8, v: usize) {
    core::ptr::write_unaligned(p as *mut usize, v.to_be())
}

/* ---- libc-ish helpers ---- */

extern "C" {
    pub fn malloc(size: usize) -> *mut core::ffi::c_void;
    pub fn calloc(n: usize, size: usize) -> *mut core::ffi::c_void;
    pub fn free(p: *mut core::ffi::c_void);
    pub fn memcpy(
        dst: *mut core::ffi::c_void,
        src: *const core::ffi::c_void,
        n: usize,
    ) -> *mut core::ffi::c_void;
    pub fn memmove(
        dst: *mut core::ffi::c_void,
        src: *const core::ffi::c_void,
        n: usize,
    ) -> *mut core::ffi::c_void;
    pub fn memset(
        dst: *mut core::ffi::c_void,
        c: core::ffi::c_int,
        n: usize,
    ) -> *mut core::ffi::c_void;
    pub fn memcmp(
        a: *const core::ffi::c_void,
        b: *const core::ffi::c_void,
        n: usize,
    ) -> core::ffi::c_int;
    pub fn qsort(
        base: *mut core::ffi::c_void,
        nmemb: usize,
        size: usize,
        compar: Option<
            unsafe extern "C" fn(*const core::ffi::c_void, *const core::ffi::c_void) -> core::ffi::c_int,
        >,
    );
}

#[inline(always)]
pub unsafe fn ZSTD_memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        memcpy(dst as *mut _, src as *const _, n);
    }
}
#[inline(always)]
pub unsafe fn ZSTD_memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        memmove(dst as *mut _, src as *const _, n);
    }
}
#[inline(always)]
pub unsafe fn ZSTD_memset(dst: *mut u8, c: i32, n: usize) {
    if n != 0 {
        memset(dst as *mut _, c, n);
    }
}
#[inline(always)]
pub unsafe fn ZSTD_memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    memcmp(a as *const _, b as *const _, n)
}
