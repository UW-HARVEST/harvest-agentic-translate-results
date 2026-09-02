//! Translation of `common/mem.h`.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub type BYTE = u8;
pub type U8 = u8;
pub type S8 = i8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;
pub type size_t = usize;

#[inline(always)]
pub fn MEM_32bits() -> u32 {
    (core::mem::size_of::<size_t>() == 4) as u32
}

#[inline(always)]
pub fn MEM_64bits() -> u32 {
    (core::mem::size_of::<size_t>() == 8) as u32
}

#[inline(always)]
pub fn MEM_isLittleEndian() -> u32 {
    if cfg!(target_endian = "little") {
        1
    } else {
        0
    }
}

/* ==== unaligned raw reads / writes ==== */

#[inline(always)]
pub unsafe fn MEM_read16(p: *const u8) -> U16 {
    (p as *const U16).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_read32(p: *const u8) -> U32 {
    (p as *const U32).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_read64(p: *const u8) -> U64 {
    (p as *const U64).read_unaligned()
}
#[inline(always)]
pub unsafe fn MEM_readST(p: *const u8) -> size_t {
    (p as *const size_t).read_unaligned()
}

#[inline(always)]
pub unsafe fn MEM_write16(p: *mut u8, v: U16) {
    (p as *mut U16).write_unaligned(v)
}
#[inline(always)]
pub unsafe fn MEM_write32(p: *mut u8, v: U32) {
    (p as *mut U32).write_unaligned(v)
}
#[inline(always)]
pub unsafe fn MEM_write64(p: *mut u8, v: U64) {
    (p as *mut U64).write_unaligned(v)
}

/* ==== byteswap ==== */

#[inline(always)]
pub fn MEM_swap32(v: U32) -> U32 {
    v.swap_bytes()
}
#[inline(always)]
pub fn MEM_swap64(v: U64) -> U64 {
    v.swap_bytes()
}
#[inline(always)]
pub fn MEM_swapST(v: size_t) -> size_t {
    v.swap_bytes()
}

/* ==== little endian ==== */

#[inline(always)]
pub unsafe fn MEM_readLE16(p: *const u8) -> U16 {
    U16::from_le(MEM_read16(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE16(p: *mut u8, v: U16) {
    MEM_write16(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLE24(p: *const u8) -> U32 {
    MEM_readLE16(p) as U32 + ((*p.add(2) as U32) << 16)
}
#[inline(always)]
pub unsafe fn MEM_writeLE24(p: *mut u8, v: U32) {
    MEM_writeLE16(p, v as U16);
    *p.add(2) = (v >> 16) as u8;
}
#[inline(always)]
pub unsafe fn MEM_readLE32(p: *const u8) -> U32 {
    U32::from_le(MEM_read32(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE32(p: *mut u8, v: U32) {
    MEM_write32(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLE64(p: *const u8) -> U64 {
    U64::from_le(MEM_read64(p))
}
#[inline(always)]
pub unsafe fn MEM_writeLE64(p: *mut u8, v: U64) {
    MEM_write64(p, v.to_le())
}
#[inline(always)]
pub unsafe fn MEM_readLEST(p: *const u8) -> size_t {
    if MEM_32bits() != 0 {
        MEM_readLE32(p) as size_t
    } else {
        MEM_readLE64(p) as size_t
    }
}
#[inline(always)]
pub unsafe fn MEM_writeLEST(p: *mut u8, v: size_t) {
    if MEM_32bits() != 0 {
        MEM_writeLE32(p, v as U32)
    } else {
        MEM_writeLE64(p, v as U64)
    }
}

/* ==== big endian ==== */

#[inline(always)]
pub unsafe fn MEM_readBE32(p: *const u8) -> U32 {
    U32::from_be(MEM_read32(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBE32(p: *mut u8, v: U32) {
    MEM_write32(p, v.to_be())
}
#[inline(always)]
pub unsafe fn MEM_readBE64(p: *const u8) -> U64 {
    U64::from_be(MEM_read64(p))
}
#[inline(always)]
pub unsafe fn MEM_writeBE64(p: *mut u8, v: U64) {
    MEM_write64(p, v.to_be())
}
#[inline(always)]
pub unsafe fn MEM_readBEST(p: *const u8) -> size_t {
    if MEM_32bits() != 0 {
        MEM_readBE32(p) as size_t
    } else {
        MEM_readBE64(p) as size_t
    }
}
#[inline(always)]
pub unsafe fn MEM_writeBEST(p: *mut u8, v: size_t) {
    if MEM_32bits() != 0 {
        MEM_writeBE32(p, v as U32)
    } else {
        MEM_writeBE64(p, v as U64)
    }
}

/* ==== zstd_deps.h shims ==== */

#[inline(always)]
pub unsafe fn ZSTD_memcpy(dst: *mut u8, src: *const u8, n: size_t) {
    if n != 0 {
        core::ptr::copy_nonoverlapping(src, dst, n);
    }
}

#[inline(always)]
pub unsafe fn ZSTD_memmove(dst: *mut u8, src: *const u8, n: size_t) {
    if n != 0 {
        core::ptr::copy(src, dst, n);
    }
}

#[inline(always)]
pub unsafe fn ZSTD_memset(dst: *mut u8, c: i32, n: size_t) {
    if n != 0 {
        core::ptr::write_bytes(dst, c as u8, n);
    }
}
