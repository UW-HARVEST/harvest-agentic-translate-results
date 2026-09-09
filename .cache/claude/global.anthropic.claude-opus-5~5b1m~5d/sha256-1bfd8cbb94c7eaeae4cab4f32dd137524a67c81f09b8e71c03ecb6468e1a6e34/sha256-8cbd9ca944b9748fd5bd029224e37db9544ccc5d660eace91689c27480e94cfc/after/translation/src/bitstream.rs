//! Translation of `common/bitstream.h`
#![allow(dead_code)]

use core::ffi::{c_char, c_void};

use crate::bits::ZSTD_highbit32;
use crate::cmem::*;
use crate::error_private::*;

pub type BitContainerType = usize;

pub const STREAM_ACCUMULATOR_MIN_32: u32 = 25;
pub const STREAM_ACCUMULATOR_MIN_64: u32 = 57;

#[inline(always)]
pub fn STREAM_ACCUMULATOR_MIN() -> U32 {
    if MEM_32bits() != 0 {
        STREAM_ACCUMULATOR_MIN_32
    } else {
        STREAM_ACCUMULATOR_MIN_64
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BIT_CStream_t {
    pub bitContainer: BitContainerType,
    pub bitPos: u32,
    pub startPtr: *mut c_char,
    pub ptr: *mut c_char,
    pub endPtr: *mut c_char,
}

impl Default for BIT_CStream_t {
    fn default() -> Self {
        BIT_CStream_t {
            bitContainer: 0,
            bitPos: 0,
            startPtr: core::ptr::null_mut(),
            ptr: core::ptr::null_mut(),
            endPtr: core::ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BIT_DStream_t {
    pub bitContainer: BitContainerType,
    pub bitsConsumed: u32,
    pub ptr: *const c_char,
    pub start: *const c_char,
    pub limitPtr: *const c_char,
}

impl Default for BIT_DStream_t {
    fn default() -> Self {
        BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: core::ptr::null(),
            start: core::ptr::null(),
            limitPtr: core::ptr::null(),
        }
    }
}

pub type BIT_DStream_status = u32;
pub const BIT_DStream_unfinished: BIT_DStream_status = 0;
pub const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
pub const BIT_DStream_completed: BIT_DStream_status = 2;
pub const BIT_DStream_overflow: BIT_DStream_status = 3;

pub static BIT_mask: [u32; 32] = [
    0, 1, 3, 7, 0xF, 0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF,
    0xFFFF, 0x1FFFF, 0x3FFFF, 0x7FFFF, 0xFFFFF, 0x1FFFFF, 0x3FFFFF, 0x7FFFFF, 0xFFFFFF, 0x1FFFFFF,
    0x3FFFFFF, 0x7FFFFFF, 0xFFFFFFF, 0x1FFFFFFF, 0x3FFFFFFF, 0x7FFFFFFF,
];

pub const BIT_MASK_SIZE: usize = 32;

const CONTAINER_BITS: u32 = (core::mem::size_of::<BitContainerType>() * 8) as u32;

/* ===== encoding ===== */

#[inline(always)]
pub unsafe fn BIT_initCStream(
    bitC: *mut BIT_CStream_t,
    startPtr: *mut c_void,
    dstCapacity: usize,
) -> usize {
    (*bitC).bitContainer = 0;
    (*bitC).bitPos = 0;
    (*bitC).startPtr = startPtr as *mut c_char;
    (*bitC).ptr = (*bitC).startPtr;
    /* `dstCapacity` may be smaller than sizeof(bitContainer), in which case the C
     * code forms an out-of-range pointer before returning the error below. Use
     * wrapping address arithmetic so this stays well-defined in Rust. */
    (*bitC).endPtr = ((*bitC).startPtr as usize)
        .wrapping_add(dstCapacity)
        .wrapping_sub(core::mem::size_of::<BitContainerType>()) as *mut c_char;
    if dstCapacity <= core::mem::size_of::<BitContainerType>() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    0
}

#[inline(always)]
pub fn BIT_getLowerBits(bitContainer: BitContainerType, nbBits: U32) -> BitContainerType {
    bitContainer & BIT_mask[nbBits as usize] as BitContainerType
}

#[inline(always)]
pub unsafe fn BIT_addBits(bitC: *mut BIT_CStream_t, value: BitContainerType, nbBits: u32) {
    (*bitC).bitContainer |= BIT_getLowerBits(value, nbBits) << (*bitC).bitPos;
    (*bitC).bitPos += nbBits;
}

#[inline(always)]
pub unsafe fn BIT_addBitsFast(bitC: *mut BIT_CStream_t, value: BitContainerType, nbBits: u32) {
    (*bitC).bitContainer |= value << (*bitC).bitPos;
    (*bitC).bitPos += nbBits;
}

#[inline(always)]
pub unsafe fn BIT_flushBitsFast(bitC: *mut BIT_CStream_t) {
    let nbBytes = ((*bitC).bitPos >> 3) as usize;
    MEM_writeLEST((*bitC).ptr as *mut c_void, (*bitC).bitContainer);
    (*bitC).ptr = (*bitC).ptr.add(nbBytes);
    (*bitC).bitPos &= 7;
    (*bitC).bitContainer >>= nbBytes * 8;
}

#[inline(always)]
pub unsafe fn BIT_flushBits(bitC: *mut BIT_CStream_t) {
    let nbBytes = ((*bitC).bitPos >> 3) as usize;
    MEM_writeLEST((*bitC).ptr as *mut c_void, (*bitC).bitContainer);
    (*bitC).ptr = (*bitC).ptr.add(nbBytes);
    if (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
    (*bitC).bitPos &= 7;
    (*bitC).bitContainer >>= nbBytes * 8;
}

#[inline(always)]
pub unsafe fn BIT_closeCStream(bitC: *mut BIT_CStream_t) -> usize {
    BIT_addBitsFast(bitC, 1, 1);
    BIT_flushBits(bitC);
    if (*bitC).ptr >= (*bitC).endPtr {
        return 0;
    }
    ((*bitC).ptr as usize - (*bitC).startPtr as usize) + ((*bitC).bitPos > 0) as usize
}

/* ===== decoding ===== */

#[inline(always)]
pub unsafe fn BIT_initDStream(
    bitD: *mut BIT_DStream_t,
    srcBuffer: *const c_void,
    srcSize: usize,
) -> usize {
    const SZ: usize = core::mem::size_of::<BitContainerType>();
    if srcSize < 1 {
        ZSTD_memset(bitD as *mut c_void, 0, core::mem::size_of::<BIT_DStream_t>());
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bitD).start = srcBuffer as *const c_char;
    (*bitD).limitPtr = (*bitD).start.add(SZ);

    if srcSize >= SZ {
        (*bitD).ptr = (srcBuffer as *const c_char).add(srcSize - SZ);
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        let lastByte = *(srcBuffer as *const BYTE).add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8 - ZSTD_highbit32(lastByte as U32)
        } else {
            0
        };
        if lastByte == 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
    } else {
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as BitContainerType;
        let sb = srcBuffer as *const BYTE;
        match srcSize {
            7 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*sb.add(6) as BitContainerType) << (SZ * 8 - 16),
                );
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*sb.add(5) as BitContainerType) << (SZ * 8 - 24),
                );
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*sb.add(4) as BitContainerType) << (SZ * 8 - 32),
                );
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(3) as BitContainerType) << 24);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(2) as BitContainerType) << 16);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(1) as BitContainerType) << 8);
            }
            6 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*sb.add(5) as BitContainerType) << (SZ * 8 - 24),
                );
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*sb.add(4) as BitContainerType) << (SZ * 8 - 32),
                );
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(3) as BitContainerType) << 24);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(2) as BitContainerType) << 16);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(1) as BitContainerType) << 8);
            }
            5 => {
                (*bitD).bitContainer = (*bitD).bitContainer.wrapping_add(
                    (*sb.add(4) as BitContainerType) << (SZ * 8 - 32),
                );
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(3) as BitContainerType) << 24);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(2) as BitContainerType) << 16);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(1) as BitContainerType) << 8);
            }
            4 => {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(3) as BitContainerType) << 24);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(2) as BitContainerType) << 16);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(1) as BitContainerType) << 8);
            }
            3 => {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(2) as BitContainerType) << 16);
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(1) as BitContainerType) << 8);
            }
            2 => {
                (*bitD).bitContainer = (*bitD)
                    .bitContainer
                    .wrapping_add((*sb.add(1) as BitContainerType) << 8);
            }
            _ => {}
        }
        let lastByte = *sb.add(srcSize - 1);
        (*bitD).bitsConsumed = if lastByte != 0 {
            8 - ZSTD_highbit32(lastByte as U32)
        } else {
            0
        };
        if lastByte == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        (*bitD).bitsConsumed += ((SZ - srcSize) * 8) as u32;
    }

    srcSize
}

#[inline(always)]
pub fn BIT_getUpperBits(bitContainer: BitContainerType, start: U32) -> BitContainerType {
    bitContainer >> start
}

#[inline(always)]
pub fn BIT_getMiddleBits(
    bitContainer: BitContainerType,
    start: U32,
    nbBits: U32,
) -> BitContainerType {
    let regMask: U32 = CONTAINER_BITS - 1;
    (bitContainer >> (start & regMask)) & (((1u64 << nbBits) - 1) as BitContainerType)
}

#[inline(always)]
pub unsafe fn BIT_lookBits(bitD: *const BIT_DStream_t, nbBits: U32) -> BitContainerType {
    BIT_getMiddleBits(
        (*bitD).bitContainer,
        CONTAINER_BITS
            .wrapping_sub((*bitD).bitsConsumed)
            .wrapping_sub(nbBits),
        nbBits,
    )
}

#[inline(always)]
pub unsafe fn BIT_lookBitsFast(bitD: *const BIT_DStream_t, nbBits: U32) -> BitContainerType {
    let regMask: U32 = CONTAINER_BITS - 1;
    ((*bitD).bitContainer << ((*bitD).bitsConsumed & regMask))
        >> (((regMask + 1).wrapping_sub(nbBits)) & regMask)
}

#[inline(always)]
pub unsafe fn BIT_skipBits(bitD: *mut BIT_DStream_t, nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(nbBits);
}

#[inline(always)]
pub unsafe fn BIT_readBits(bitD: *mut BIT_DStream_t, nbBits: u32) -> BitContainerType {
    let value = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

#[inline(always)]
pub unsafe fn BIT_readBitsFast(bitD: *mut BIT_DStream_t, nbBits: u32) -> BitContainerType {
    let value = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    value
}

#[inline(always)]
pub unsafe fn BIT_reloadDStream_internal(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    (*bitD).ptr = (*bitD).ptr.sub(((*bitD).bitsConsumed >> 3) as usize);
    (*bitD).bitsConsumed &= 7;
    (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
    BIT_DStream_unfinished
}

#[inline(always)]
pub unsafe fn BIT_reloadDStreamFast(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).ptr < (*bitD).limitPtr {
        return BIT_DStream_overflow;
    }
    BIT_reloadDStream_internal(bitD)
}

static BIT_ZERO_FILLED: BitContainerType = 0;

#[inline(always)]
pub unsafe fn BIT_reloadDStream(bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed > CONTAINER_BITS {
        (*bitD).ptr = &BIT_ZERO_FILLED as *const BitContainerType as *const c_char;
        return BIT_DStream_overflow;
    }

    if (*bitD).ptr >= (*bitD).limitPtr {
        return BIT_reloadDStream_internal(bitD);
    }
    if (*bitD).ptr == (*bitD).start {
        if (*bitD).bitsConsumed < CONTAINER_BITS {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    {
        let mut nbBytes: U32 = (*bitD).bitsConsumed >> 3;
        let mut result: BIT_DStream_status = BIT_DStream_unfinished;
        if ((*bitD).ptr as usize).wrapping_sub(nbBytes as usize) < (*bitD).start as usize {
            nbBytes = ((*bitD).ptr as usize - (*bitD).start as usize) as U32;
            result = BIT_DStream_endOfBuffer;
        }
        (*bitD).ptr = (*bitD).ptr.sub(nbBytes as usize);
        (*bitD).bitsConsumed -= nbBytes * 8;
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const c_void);
        result
    }
}

#[inline(always)]
pub unsafe fn BIT_endOfDStream(DStream: *const BIT_DStream_t) -> u32 {
    (((*DStream).ptr == (*DStream).start) && ((*DStream).bitsConsumed == CONTAINER_BITS)) as u32
}
