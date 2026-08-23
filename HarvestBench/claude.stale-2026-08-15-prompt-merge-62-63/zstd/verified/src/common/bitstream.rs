//! Translation of common/bitstream.h — forward-write / backward-read bitstreams.
#![allow(dead_code)]
use super::bits::highbit32;
use super::error::{error, code};
use super::mem::*;
use core::ffi::c_void;

pub const STREAM_ACCUMULATOR_MIN_32: u32 = 25;
pub const STREAM_ACCUMULATOR_MIN_64: u32 = 57;
#[inline]
pub fn stream_accumulator_min() -> u32 {
    if mem_32bits() != 0 {
        STREAM_ACCUMULATOR_MIN_32
    } else {
        STREAM_ACCUMULATOR_MIN_64
    }
}

pub type BitContainerType = usize;

#[repr(C)]
pub struct BIT_CStream_t {
    pub bitContainer: BitContainerType,
    pub bitPos: u32,
    pub startPtr: *mut u8,
    pub ptr: *mut u8,
    pub endPtr: *mut u8,
}

#[repr(C)]
pub struct BIT_DStream_t {
    pub bitContainer: BitContainerType,
    pub bitsConsumed: u32,
    pub ptr: *const u8,
    pub start: *const u8,
    pub limitPtr: *const u8,
}

pub const BIT_DStream_unfinished: u32 = 0;
pub const BIT_DStream_endOfBuffer: u32 = 1;
pub const BIT_DStream_completed: u32 = 2;
pub const BIT_DStream_overflow: u32 = 3;
pub type BIT_DStream_status = u32;

pub static BIT_MASK: [u32; 32] = [
    0, 1, 3, 7, 0xF, 0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF,
    0xFFFF, 0x1FFFF, 0x3FFFF, 0x7FFFF, 0xFFFFF, 0x1FFFFF, 0x3FFFFF, 0x7FFFFF, 0xFFFFFF, 0x1FFFFFF,
    0x3FFFFFF, 0x7FFFFFF, 0xFFFFFFF, 0x1FFFFFFF, 0x3FFFFFFF, 0x7FFFFFFF,
];
pub const BIT_MASK_SIZE: usize = 32;

const SZ: usize = core::mem::size_of::<BitContainerType>();

#[inline]
pub unsafe fn bit_init_cstream(
    bitC: *mut BIT_CStream_t,
    startPtr: *mut c_void,
    dstCapacity: usize,
) -> usize {
    (*bitC).bitContainer = 0;
    (*bitC).bitPos = 0;
    (*bitC).startPtr = startPtr as *mut u8;
    (*bitC).ptr = (*bitC).startPtr;
    (*bitC).endPtr = (*bitC).startPtr.add(dstCapacity - SZ);
    if dstCapacity <= SZ {
        return error(code::DSTSIZE_TOOSMALL);
    }
    0
}

#[inline]
pub fn bit_get_lower_bits(bitContainer: BitContainerType, nbBits: u32) -> BitContainerType {
    bitContainer & (BIT_MASK[nbBits as usize] as BitContainerType)
}

#[inline]
pub unsafe fn bit_add_bits(bitC: *mut BIT_CStream_t, value: BitContainerType, nbBits: u32) {
    (*bitC).bitContainer |= bit_get_lower_bits(value, nbBits) << (*bitC).bitPos;
    (*bitC).bitPos += nbBits;
}

#[inline]
pub unsafe fn bit_add_bits_fast(bitC: *mut BIT_CStream_t, value: BitContainerType, nbBits: u32) {
    (*bitC).bitContainer |= value << (*bitC).bitPos;
    (*bitC).bitPos += nbBits;
}

#[inline]
pub unsafe fn bit_flush_bits_fast(bitC: *mut BIT_CStream_t) {
    let nbBytes = ((*bitC).bitPos >> 3) as usize;
    mem_write_le_st((*bitC).ptr as *mut c_void, (*bitC).bitContainer);
    (*bitC).ptr = (*bitC).ptr.add(nbBytes);
    (*bitC).bitPos &= 7;
    (*bitC).bitContainer >>= nbBytes * 8;
}

#[inline]
pub unsafe fn bit_flush_bits(bitC: *mut BIT_CStream_t) {
    let nbBytes = ((*bitC).bitPos >> 3) as usize;
    mem_write_le_st((*bitC).ptr as *mut c_void, (*bitC).bitContainer);
    (*bitC).ptr = (*bitC).ptr.add(nbBytes);
    if (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
    (*bitC).bitPos &= 7;
    (*bitC).bitContainer >>= nbBytes * 8;
}

#[inline]
pub unsafe fn bit_close_cstream(bitC: *mut BIT_CStream_t) -> usize {
    bit_add_bits_fast(bitC, 1, 1);
    bit_flush_bits(bitC);
    if (*bitC).ptr >= (*bitC).endPtr {
        return 0;
    }
    ((*bitC).ptr as usize - (*bitC).startPtr as usize) + ((*bitC).bitPos > 0) as usize
}

#[inline]
pub unsafe fn bit_init_dstream(
    bitD: *mut BIT_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    if srcSize < 1 {
        core::ptr::write_bytes(bitD as *mut u8, 0, core::mem::size_of::<BIT_DStream_t>());
        return error(code::SRCSIZE_WRONG);
    }
    let src = srcBuffer as *const u8;
    (*bitD).start = src;
    (*bitD).limitPtr = src.add(SZ);

    if srcSize >= SZ {
        (*bitD).ptr = src.add(srcSize - SZ);
        (*bitD).bitContainer = mem_read_le_st((*bitD).ptr as *const c_void);
        let lastByte = *src.add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8 - highbit32(lastByte as u32)
        } else {
            0
        };
        if lastByte == 0 {
            return error(code::GENERIC);
        }
    } else {
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *src as BitContainerType;
        match srcSize {
            7 => {
                (*bitD).bitContainer +=
                    (*src.add(6) as BitContainerType) << (SZ * 8 - 16);
                (*bitD).bitContainer +=
                    (*src.add(5) as BitContainerType) << (SZ * 8 - 24);
                (*bitD).bitContainer +=
                    (*src.add(4) as BitContainerType) << (SZ * 8 - 32);
                (*bitD).bitContainer += (*src.add(3) as BitContainerType) << 24;
                (*bitD).bitContainer += (*src.add(2) as BitContainerType) << 16;
                (*bitD).bitContainer += (*src.add(1) as BitContainerType) << 8;
            }
            6 => {
                (*bitD).bitContainer +=
                    (*src.add(5) as BitContainerType) << (SZ * 8 - 24);
                (*bitD).bitContainer +=
                    (*src.add(4) as BitContainerType) << (SZ * 8 - 32);
                (*bitD).bitContainer += (*src.add(3) as BitContainerType) << 24;
                (*bitD).bitContainer += (*src.add(2) as BitContainerType) << 16;
                (*bitD).bitContainer += (*src.add(1) as BitContainerType) << 8;
            }
            5 => {
                (*bitD).bitContainer +=
                    (*src.add(4) as BitContainerType) << (SZ * 8 - 32);
                (*bitD).bitContainer += (*src.add(3) as BitContainerType) << 24;
                (*bitD).bitContainer += (*src.add(2) as BitContainerType) << 16;
                (*bitD).bitContainer += (*src.add(1) as BitContainerType) << 8;
            }
            4 => {
                (*bitD).bitContainer += (*src.add(3) as BitContainerType) << 24;
                (*bitD).bitContainer += (*src.add(2) as BitContainerType) << 16;
                (*bitD).bitContainer += (*src.add(1) as BitContainerType) << 8;
            }
            3 => {
                (*bitD).bitContainer += (*src.add(2) as BitContainerType) << 16;
                (*bitD).bitContainer += (*src.add(1) as BitContainerType) << 8;
            }
            2 => {
                (*bitD).bitContainer += (*src.add(1) as BitContainerType) << 8;
            }
            _ => {}
        }
        let lastByte = *src.add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8 - highbit32(lastByte as u32)
        } else {
            0
        };
        if lastByte == 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        (*bitD).bitsConsumed += ((SZ - srcSize) * 8) as u32;
    }
    srcSize
}

#[inline]
pub fn bit_get_upper_bits(bitContainer: BitContainerType, start: u32) -> BitContainerType {
    bitContainer >> start
}

#[inline]
pub fn bit_get_middle_bits(
    bitContainer: BitContainerType,
    start: u32,
    nbBits: u32,
) -> BitContainerType {
    let regMask = (SZ * 8 - 1) as u32;
    (bitContainer >> (start & regMask)) & (((1u64 << nbBits) - 1) as BitContainerType)
}

#[inline]
pub unsafe fn bit_look_bits(bitD: *const BIT_DStream_t, nbBits: u32) -> BitContainerType {
    bit_get_middle_bits(
        (*bitD).bitContainer,
        (SZ * 8) as u32 - (*bitD).bitsConsumed - nbBits,
        nbBits,
    )
}

#[inline]
pub unsafe fn bit_look_bits_fast(bitD: *const BIT_DStream_t, nbBits: u32) -> BitContainerType {
    let regMask = (SZ * 8 - 1) as u32;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & regMask))
        >> (((regMask + 1) - nbBits) & regMask)
}

#[inline]
pub unsafe fn bit_skip_bits(bitD: *mut BIT_DStream_t, nbBits: u32) {
    (*bitD).bitsConsumed += nbBits;
}

#[inline]
pub unsafe fn bit_read_bits(bitD: *mut BIT_DStream_t, nbBits: u32) -> BitContainerType {
    let value = bit_look_bits(bitD, nbBits);
    bit_skip_bits(bitD, nbBits);
    value
}

#[inline]
pub unsafe fn bit_read_bits_fast(bitD: *mut BIT_DStream_t, nbBits: u32) -> BitContainerType {
    let value = bit_look_bits_fast(bitD, nbBits);
    bit_skip_bits(bitD, nbBits);
    value
}

#[inline]
pub unsafe fn bit_reload_dstream_internal(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    (*bitD).ptr = (*bitD).ptr.sub(((*bitD).bitsConsumed >> 3) as usize);
    (*bitD).bitsConsumed &= 7;
    (*bitD).bitContainer = mem_read_le_st((*bitD).ptr as *const c_void);
    BIT_DStream_unfinished
}

#[inline]
pub unsafe fn bit_reload_dstream_fast(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).ptr < (*bitD).limitPtr {
        return BIT_DStream_overflow;
    }
    bit_reload_dstream_internal(bitD)
}

static ZERO_FILLED: BitContainerType = 0;

#[inline]
pub unsafe fn bit_reload_dstream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed > (SZ * 8) as u32 {
        (*bitD).ptr = &ZERO_FILLED as *const BitContainerType as *const u8;
        return BIT_DStream_overflow;
    }
    if (*bitD).ptr >= (*bitD).limitPtr {
        return bit_reload_dstream_internal(bitD);
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize) < SZ * 8 {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes = (*bitD).bitsConsumed >> 3;
        let mut result = BIT_DStream_unfinished;
        if ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) < (*bitD).start as usize {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as u32;
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.sub(nbBytes as usize);
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = mem_read_le_st((*bitD).ptr as *const c_void);
        result
    }
}

#[inline]
pub unsafe fn bit_end_of_dstream(bitD: *const BIT_DStream_t) -> u32 {
    (((*bitD).ptr == (*bitD).start) && ((*bitD).bitsConsumed as usize == SZ * 8)) as u32
}
