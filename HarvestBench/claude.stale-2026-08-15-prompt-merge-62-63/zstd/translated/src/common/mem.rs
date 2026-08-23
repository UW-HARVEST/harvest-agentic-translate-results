//! Translation of common/mem.h — memory I/O helpers.
//! Target is little-endian 64-bit (matches the C build environment).
#![allow(dead_code)]

pub type Byte = u8;
pub type U8 = u8;
pub type S8 = i8;
pub type U16 = u16;
pub type S16 = i16;
pub type U32 = u32;
pub type S32 = i32;
pub type U64 = u64;
pub type S64 = i64;

#[inline]
pub fn mem_32bits() -> u32 {
    (core::mem::size_of::<usize>() == 4) as u32
}
#[inline]
pub fn mem_64bits() -> u32 {
    (core::mem::size_of::<usize>() == 8) as u32
}
#[inline]
pub fn mem_is_little_endian() -> u32 {
    cfg!(target_endian = "little") as u32
}

#[inline]
pub unsafe fn mem_read16(ptr: *const core::ffi::c_void) -> U16 {
    let mut v: U16 = 0;
    core::ptr::copy_nonoverlapping(ptr as *const u8, &mut v as *mut U16 as *mut u8, 2);
    v
}
#[inline]
pub unsafe fn mem_read32(ptr: *const core::ffi::c_void) -> U32 {
    let mut v: U32 = 0;
    core::ptr::copy_nonoverlapping(ptr as *const u8, &mut v as *mut U32 as *mut u8, 4);
    v
}
#[inline]
pub unsafe fn mem_read64(ptr: *const core::ffi::c_void) -> U64 {
    let mut v: U64 = 0;
    core::ptr::copy_nonoverlapping(ptr as *const u8, &mut v as *mut U64 as *mut u8, 8);
    v
}
#[inline]
pub unsafe fn mem_read_st(ptr: *const core::ffi::c_void) -> usize {
    let mut v: usize = 0;
    core::ptr::copy_nonoverlapping(
        ptr as *const u8,
        &mut v as *mut usize as *mut u8,
        core::mem::size_of::<usize>(),
    );
    v
}
#[inline]
pub unsafe fn mem_write16(ptr: *mut core::ffi::c_void, value: U16) {
    core::ptr::copy_nonoverlapping(&value as *const U16 as *const u8, ptr as *mut u8, 2);
}
#[inline]
pub unsafe fn mem_write32(ptr: *mut core::ffi::c_void, value: U32) {
    core::ptr::copy_nonoverlapping(&value as *const U32 as *const u8, ptr as *mut u8, 4);
}
#[inline]
pub unsafe fn mem_write64(ptr: *mut core::ffi::c_void, value: U64) {
    core::ptr::copy_nonoverlapping(&value as *const U64 as *const u8, ptr as *mut u8, 8);
}

#[inline]
pub fn mem_swap32(x: U32) -> U32 {
    x.swap_bytes()
}
#[inline]
pub fn mem_swap64(x: U64) -> U64 {
    x.swap_bytes()
}
#[inline]
pub fn mem_swap_st(x: usize) -> usize {
    x.swap_bytes()
}

#[inline]
pub unsafe fn mem_read_le16(ptr: *const core::ffi::c_void) -> U16 {
    if mem_is_little_endian() != 0 {
        mem_read16(ptr)
    } else {
        let p = ptr as *const u8;
        (*p as U16) + ((*p.add(1) as U16) << 8)
    }
}
#[inline]
pub unsafe fn mem_write_le16(ptr: *mut core::ffi::c_void, val: U16) {
    if mem_is_little_endian() != 0 {
        mem_write16(ptr, val);
    } else {
        let p = ptr as *mut u8;
        *p = val as u8;
        *p.add(1) = (val >> 8) as u8;
    }
}
#[inline]
pub unsafe fn mem_read_le24(ptr: *const core::ffi::c_void) -> U32 {
    (mem_read_le16(ptr) as U32) + ((*((ptr as *const u8).add(2)) as U32) << 16)
}
#[inline]
pub unsafe fn mem_write_le24(ptr: *mut core::ffi::c_void, val: U32) {
    mem_write_le16(ptr, val as U16);
    *((ptr as *mut u8).add(2)) = (val >> 16) as u8;
}
#[inline]
pub unsafe fn mem_read_le32(ptr: *const core::ffi::c_void) -> U32 {
    if mem_is_little_endian() != 0 {
        mem_read32(ptr)
    } else {
        mem_swap32(mem_read32(ptr))
    }
}
#[inline]
pub unsafe fn mem_write_le32(ptr: *mut core::ffi::c_void, val: U32) {
    if mem_is_little_endian() != 0 {
        mem_write32(ptr, val);
    } else {
        mem_write32(ptr, mem_swap32(val));
    }
}
#[inline]
pub unsafe fn mem_read_le64(ptr: *const core::ffi::c_void) -> U64 {
    if mem_is_little_endian() != 0 {
        mem_read64(ptr)
    } else {
        mem_swap64(mem_read64(ptr))
    }
}
#[inline]
pub unsafe fn mem_write_le64(ptr: *mut core::ffi::c_void, val: U64) {
    if mem_is_little_endian() != 0 {
        mem_write64(ptr, val);
    } else {
        mem_write64(ptr, mem_swap64(val));
    }
}
#[inline]
pub unsafe fn mem_read_le_st(ptr: *const core::ffi::c_void) -> usize {
    if mem_32bits() != 0 {
        mem_read_le32(ptr) as usize
    } else {
        mem_read_le64(ptr) as usize
    }
}
#[inline]
pub unsafe fn mem_write_le_st(ptr: *mut core::ffi::c_void, val: usize) {
    if mem_32bits() != 0 {
        mem_write_le32(ptr, val as U32);
    } else {
        mem_write_le64(ptr, val as U64);
    }
}
#[inline]
pub unsafe fn mem_read_be32(ptr: *const core::ffi::c_void) -> U32 {
    if mem_is_little_endian() != 0 {
        mem_swap32(mem_read32(ptr))
    } else {
        mem_read32(ptr)
    }
}
#[inline]
pub unsafe fn mem_write_be32(ptr: *mut core::ffi::c_void, val: U32) {
    if mem_is_little_endian() != 0 {
        mem_write32(ptr, mem_swap32(val));
    } else {
        mem_write32(ptr, val);
    }
}
#[inline]
pub unsafe fn mem_read_be64(ptr: *const core::ffi::c_void) -> U64 {
    if mem_is_little_endian() != 0 {
        mem_swap64(mem_read64(ptr))
    } else {
        mem_read64(ptr)
    }
}
#[inline]
pub unsafe fn mem_write_be64(ptr: *mut core::ffi::c_void, val: U64) {
    if mem_is_little_endian() != 0 {
        mem_write64(ptr, mem_swap64(val));
    } else {
        mem_write64(ptr, val);
    }
}
#[inline]
pub unsafe fn mem_read_be_st(ptr: *const core::ffi::c_void) -> usize {
    if mem_32bits() != 0 {
        mem_read_be32(ptr) as usize
    } else {
        mem_read_be64(ptr) as usize
    }
}
#[inline]
pub unsafe fn mem_write_be_st(ptr: *mut core::ffi::c_void, val: usize) {
    if mem_32bits() != 0 {
        mem_write_be32(ptr, val as U32);
    } else {
        mem_write_be64(ptr, val as U64);
    }
}
