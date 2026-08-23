//! Translation of `common/mem.h`
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
    cfg!(target_endian = "little") as u32
}

/* === Native unaligned read/write === */

#[inline(always)]
pub unsafe fn MEM_read16(ptr: *const c_void) -> U16 {
    (ptr as *const U16).read_unaligned()
}

#[inline(always)]
pub unsafe fn MEM_read32(ptr: *const c_void) -> U32 {
    (ptr as *const U32).read_unaligned()
}

#[inline(always)]
pub unsafe fn MEM_read64(ptr: *const c_void) -> U64 {
    (ptr as *const U64).read_unaligned()
}

#[inline(always)]
pub unsafe fn MEM_readST(ptr: *const c_void) -> usize {
    (ptr as *const usize).read_unaligned()
}

#[inline(always)]
pub unsafe fn MEM_write16(memPtr: *mut c_void, value: U16) {
    (memPtr as *mut U16).write_unaligned(value)
}

#[inline(always)]
pub unsafe fn MEM_write32(memPtr: *mut c_void, value: U32) {
    (memPtr as *mut U32).write_unaligned(value)
}

#[inline(always)]
pub unsafe fn MEM_write64(memPtr: *mut c_void, value: U64) {
    (memPtr as *mut U64).write_unaligned(value)
}

/* === Byteswap === */

#[inline(always)]
pub fn MEM_swap32_fallback(input: U32) -> U32 {
    ((input << 24) & 0xff000000)
        | ((input << 8) & 0x00ff0000)
        | ((input >> 8) & 0x0000ff00)
        | ((input >> 24) & 0x000000ff)
}

#[inline(always)]
pub fn MEM_swap32(input: U32) -> U32 {
    input.swap_bytes()
}

#[inline(always)]
pub fn MEM_swap64_fallback(input: U64) -> U64 {
    input.swap_bytes()
}

#[inline(always)]
pub fn MEM_swap64(input: U64) -> U64 {
    input.swap_bytes()
}

#[inline(always)]
pub fn MEM_swapST(input: usize) -> usize {
    if MEM_32bits() != 0 {
        MEM_swap32(input as U32) as usize
    } else {
        MEM_swap64(input as U64) as usize
    }
}

/* === Little endian r/w === */

#[inline(always)]
pub unsafe fn MEM_readLE16(memPtr: *const c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        MEM_read16(memPtr)
    } else {
        let p = memPtr as *const BYTE;
        (*p.add(0) as U16).wrapping_add((*p.add(1) as U16) << 8)
    }
}

#[inline(always)]
pub unsafe fn MEM_writeLE16(memPtr: *mut c_void, val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let p = memPtr as *mut BYTE;
        *p.add(0) = val as BYTE;
        *p.add(1) = (val >> 8) as BYTE;
    }
}

#[inline(always)]
pub unsafe fn MEM_readLE24(memPtr: *const c_void) -> U32 {
    (MEM_readLE16(memPtr) as U32).wrapping_add((*(memPtr as *const BYTE).add(2) as U32) << 16)
}

#[inline(always)]
pub unsafe fn MEM_writeLE24(memPtr: *mut c_void, val: U32) {
    MEM_writeLE16(memPtr, val as U16);
    *(memPtr as *mut BYTE).add(2) = (val >> 16) as BYTE;
}

#[inline(always)]
pub unsafe fn MEM_readLE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_read32(memPtr)
    } else {
        MEM_swap32(MEM_read32(memPtr))
    }
}

#[inline(always)]
pub unsafe fn MEM_writeLE32(memPtr: *mut c_void, val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, val32);
    } else {
        MEM_write32(memPtr, MEM_swap32(val32));
    }
}

#[inline(always)]
pub unsafe fn MEM_readLE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_read64(memPtr)
    } else {
        MEM_swap64(MEM_read64(memPtr))
    }
}

#[inline(always)]
pub unsafe fn MEM_writeLE64(memPtr: *mut c_void, val64: U64) {
    if MEM_isLittleEndian() != 0 {
        MEM_write64(memPtr, val64);
    } else {
        MEM_write64(memPtr, MEM_swap64(val64));
    }
}

#[inline(always)]
pub unsafe fn MEM_readLEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readLE32(memPtr) as usize
    } else {
        MEM_readLE64(memPtr) as usize
    }
}

#[inline(always)]
pub unsafe fn MEM_writeLEST(memPtr: *mut c_void, val: usize) {
    if MEM_32bits() != 0 {
        MEM_writeLE32(memPtr, val as U32);
    } else {
        MEM_writeLE64(memPtr, val as U64);
    }
}

/* === Big endian r/w === */

#[inline(always)]
pub unsafe fn MEM_readBE32(memPtr: *const c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        MEM_swap32(MEM_read32(memPtr))
    } else {
        MEM_read32(memPtr)
    }
}

#[inline(always)]
pub unsafe fn MEM_writeBE32(memPtr: *mut c_void, val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, MEM_swap32(val32));
    } else {
        MEM_write32(memPtr, val32);
    }
}

#[inline(always)]
pub unsafe fn MEM_readBE64(memPtr: *const c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        MEM_swap64(MEM_read64(memPtr))
    } else {
        MEM_read64(memPtr)
    }
}

#[inline(always)]
pub unsafe fn MEM_writeBE64(memPtr: *mut c_void, val64: U64) {
    if MEM_isLittleEndian() != 0 {
        MEM_write64(memPtr, MEM_swap64(val64));
    } else {
        MEM_write64(memPtr, val64);
    }
}

#[inline(always)]
pub unsafe fn MEM_readBEST(memPtr: *const c_void) -> usize {
    if MEM_32bits() != 0 {
        MEM_readBE32(memPtr) as usize
    } else {
        MEM_readBE64(memPtr) as usize
    }
}

#[inline(always)]
pub unsafe fn MEM_writeBEST(memPtr: *mut c_void, val: usize) {
    if MEM_32bits() != 0 {
        MEM_writeBE32(memPtr, val as U32);
    } else {
        MEM_writeBE64(memPtr, val as U64);
    }
}
