//! Translation of `common/mem.h` — unaligned memory access helpers.
//!
//! The C code selects `MEM_FORCE_MEMORY_ACCESS==1` under GCC, i.e. direct
//! unaligned loads/stores. Rust's `read_unaligned`/`write_unaligned` are the
//! exact equivalent.
#![allow(dead_code)]

pub type BYTE = u8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

/// `MEM_32bits()`
#[inline(always)]
pub fn mem_32bits() -> bool {
    core::mem::size_of::<usize>() == 4
}

/// `MEM_64bits()`
#[inline(always)]
pub fn mem_64bits() -> bool {
    core::mem::size_of::<usize>() == 8
}

/// `MEM_isLittleEndian()`
#[inline(always)]
pub fn mem_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}

#[inline(always)]
pub unsafe fn mem_read16(ptr: *const u8) -> U16 {
    (ptr as *const U16).read_unaligned()
}

#[inline(always)]
pub unsafe fn mem_read32(ptr: *const u8) -> U32 {
    (ptr as *const U32).read_unaligned()
}

#[inline(always)]
pub unsafe fn mem_read64(ptr: *const u8) -> U64 {
    (ptr as *const U64).read_unaligned()
}

#[inline(always)]
pub unsafe fn mem_read_st(ptr: *const u8) -> usize {
    (ptr as *const usize).read_unaligned()
}

#[inline(always)]
pub unsafe fn mem_write16(ptr: *mut u8, value: U16) {
    (ptr as *mut U16).write_unaligned(value)
}

#[inline(always)]
pub unsafe fn mem_write32(ptr: *mut u8, value: U32) {
    (ptr as *mut U32).write_unaligned(value)
}

#[inline(always)]
pub unsafe fn mem_write64(ptr: *mut u8, value: U64) {
    (ptr as *mut U64).write_unaligned(value)
}

#[inline(always)]
pub unsafe fn mem_write_st(ptr: *mut u8, value: usize) {
    (ptr as *mut usize).write_unaligned(value)
}

#[inline(always)]
pub fn mem_swap32(v: U32) -> U32 {
    v.swap_bytes()
}

#[inline(always)]
pub fn mem_swap64(v: U64) -> U64 {
    v.swap_bytes()
}

#[inline(always)]
pub fn mem_swap_st(v: usize) -> usize {
    v.swap_bytes()
}

/* =========== little endian r/w =========== */

#[inline(always)]
pub unsafe fn mem_read_le16(ptr: *const u8) -> U16 {
    U16::from_le(mem_read16(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_le16(ptr: *mut u8, val: U16) {
    mem_write16(ptr, val.to_le())
}

/// `MEM_readLE24()`
#[inline(always)]
pub unsafe fn mem_read_le24(ptr: *const u8) -> U32 {
    mem_read_le16(ptr) as U32 + ((*ptr.add(2) as U32) << 16)
}

/// `MEM_writeLE24()`
#[inline(always)]
pub unsafe fn mem_write_le24(ptr: *mut u8, val: U32) {
    mem_write_le16(ptr, (val & 0xFFFF) as U16);
    *ptr.add(2) = (val >> 16) as u8;
}

#[inline(always)]
pub unsafe fn mem_read_le32(ptr: *const u8) -> U32 {
    U32::from_le(mem_read32(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_le32(ptr: *mut u8, val: U32) {
    mem_write32(ptr, val.to_le())
}

#[inline(always)]
pub unsafe fn mem_read_le64(ptr: *const u8) -> U64 {
    U64::from_le(mem_read64(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_le64(ptr: *mut u8, val: U64) {
    mem_write64(ptr, val.to_le())
}

#[inline(always)]
pub unsafe fn mem_read_lest(ptr: *const u8) -> usize {
    usize::from_le(mem_read_st(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_lest(ptr: *mut u8, val: usize) {
    mem_write_st(ptr, val.to_le())
}

/* =========== big endian r/w =========== */

#[inline(always)]
pub unsafe fn mem_read_be32(ptr: *const u8) -> U32 {
    U32::from_be(mem_read32(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_be32(ptr: *mut u8, val: U32) {
    mem_write32(ptr, val.to_be())
}

#[inline(always)]
pub unsafe fn mem_read_be64(ptr: *const u8) -> U64 {
    U64::from_be(mem_read64(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_be64(ptr: *mut u8, val: U64) {
    mem_write64(ptr, val.to_be())
}

#[inline(always)]
pub unsafe fn mem_read_best(ptr: *const u8) -> usize {
    usize::from_be(mem_read_st(ptr))
}

#[inline(always)]
pub unsafe fn mem_write_best(ptr: *mut u8, val: usize) {
    mem_write_st(ptr, val.to_be())
}
