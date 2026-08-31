//! Translation of `common/bitstream.h`.
#![allow(dead_code)]

use crate::bits::zstd_highbit32;
use crate::error::*;
use crate::mem::*;

pub type BitContainerType = usize;

pub const STREAM_ACCUMULATOR_MIN_32: u32 = 25;
pub const STREAM_ACCUMULATOR_MIN_64: u32 = 57;

/// `STREAM_ACCUMULATOR_MIN`
#[inline(always)]
pub fn stream_accumulator_min() -> u32 {
    if mem_32bits() {
        STREAM_ACCUMULATOR_MIN_32
    } else {
        STREAM_ACCUMULATOR_MIN_64
    }
}

const CONTAINER_BYTES: usize = core::mem::size_of::<BitContainerType>();
const CONTAINER_BITS: u32 = (CONTAINER_BYTES * 8) as u32;

/// `BIT_CStream_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BIT_CStream_t {
    pub bitContainer: BitContainerType,
    pub bitPos: u32,
    pub startPtr: *mut u8,
    pub ptr: *mut u8,
    pub endPtr: *mut u8,
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

/// `BIT_DStream_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BIT_DStream_t {
    pub bitContainer: BitContainerType,
    pub bitsConsumed: u32,
    pub ptr: *const u8,
    pub start: *const u8,
    pub limitPtr: *const u8,
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

/// `BIT_DStream_status`
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum BIT_DStream_status {
    unfinished = 0,
    endOfBuffer = 1,
    completed = 2,
    overflow = 3,
}

/// `BIT_mask[]` — up to 31 bits.
pub static BIT_MASK: [u32; 32] = [
    0, 1, 3, 7, 0xF, 0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF, 0x3FFF, 0x7FFF,
    0xFFFF, 0x1FFFF, 0x3FFFF, 0x7FFFF, 0xFFFFF, 0x1FFFFF, 0x3FFFFF, 0x7FFFFF, 0xFFFFFF, 0x1FFFFFF,
    0x3FFFFFF, 0x7FFFFFF, 0xFFFFFFF, 0x1FFFFFFF, 0x3FFFFFFF, 0x7FFFFFFF,
];

/* ================= encoding ================= */

/// `BIT_initCStream()`
#[inline(always)]
pub unsafe fn bit_init_cstream(
    bit_c: &mut BIT_CStream_t,
    start_ptr: *mut u8,
    dst_capacity: usize,
) -> usize {
    bit_c.bitContainer = 0;
    bit_c.bitPos = 0;
    bit_c.startPtr = start_ptr;
    bit_c.ptr = bit_c.startPtr;
    bit_c.endPtr = bit_c.startPtr.add(dst_capacity.wrapping_sub(CONTAINER_BYTES));
    if dst_capacity <= CONTAINER_BYTES {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    0
}

/// `BIT_getLowerBits()`
#[inline(always)]
pub fn bit_get_lower_bits(bit_container: BitContainerType, nb_bits: u32) -> BitContainerType {
    debug_assert!((nb_bits as usize) < BIT_MASK.len());
    bit_container & BIT_MASK[nb_bits as usize] as BitContainerType
}

/// `BIT_addBits()`
#[inline(always)]
pub fn bit_add_bits(bit_c: &mut BIT_CStream_t, value: BitContainerType, nb_bits: u32) {
    debug_assert!((nb_bits as usize) < BIT_MASK.len());
    bit_c.bitContainer |= bit_get_lower_bits(value, nb_bits) << bit_c.bitPos;
    bit_c.bitPos += nb_bits;
}

/// `BIT_addBitsFast()`
#[inline(always)]
pub fn bit_add_bits_fast(bit_c: &mut BIT_CStream_t, value: BitContainerType, nb_bits: u32) {
    debug_assert_eq!(value >> nb_bits, 0);
    bit_c.bitContainer |= value << bit_c.bitPos;
    bit_c.bitPos += nb_bits;
}

/// `BIT_flushBitsFast()`
#[inline(always)]
pub unsafe fn bit_flush_bits_fast(bit_c: &mut BIT_CStream_t) {
    let nb_bytes = (bit_c.bitPos >> 3) as usize;
    mem_write_lest(bit_c.ptr, bit_c.bitContainer);
    bit_c.ptr = bit_c.ptr.add(nb_bytes);
    bit_c.bitPos &= 7;
    bit_c.bitContainer >>= nb_bytes * 8;
}

/// `BIT_flushBits()`
#[inline(always)]
pub unsafe fn bit_flush_bits(bit_c: &mut BIT_CStream_t) {
    let nb_bytes = (bit_c.bitPos >> 3) as usize;
    mem_write_lest(bit_c.ptr, bit_c.bitContainer);
    bit_c.ptr = bit_c.ptr.add(nb_bytes);
    if bit_c.ptr > bit_c.endPtr {
        bit_c.ptr = bit_c.endPtr;
    }
    bit_c.bitPos &= 7;
    bit_c.bitContainer >>= nb_bytes * 8;
}

/// `BIT_closeCStream()`
#[inline(always)]
pub unsafe fn bit_close_cstream(bit_c: &mut BIT_CStream_t) -> usize {
    bit_add_bits_fast(bit_c, 1, 1); /* endMark */
    bit_flush_bits(bit_c);
    if bit_c.ptr >= bit_c.endPtr {
        return 0; /* overflow detected */
    }
    (bit_c.ptr as usize - bit_c.startPtr as usize) + (bit_c.bitPos > 0) as usize
}

/* ================= decoding ================= */

/// `BIT_initDStream()`
#[inline(always)]
pub unsafe fn bit_init_dstream(
    bit_d: &mut BIT_DStream_t,
    src_buffer: *const u8,
    src_size: usize,
) -> usize {
    if src_size < 1 {
        *bit_d = BIT_DStream_t::default();
        return err_code(ZSTD_error_srcSize_wrong);
    }

    bit_d.start = src_buffer;
    bit_d.limitPtr = bit_d.start.add(CONTAINER_BYTES);

    if src_size >= CONTAINER_BYTES {
        /* normal case */
        bit_d.ptr = src_buffer.add(src_size - CONTAINER_BYTES);
        bit_d.bitContainer = mem_read_lest(bit_d.ptr);
        let last_byte = *src_buffer.add(src_size - 1);
        bit_d.bitsConsumed = if last_byte != 0 {
            8 - zstd_highbit32(last_byte as U32)
        } else {
            0
        };
        if last_byte == 0 {
            return err_code(ZSTD_error_GENERIC); /* endMark not present */
        }
    } else {
        bit_d.ptr = bit_d.start;
        bit_d.bitContainer = *bit_d.start as BitContainerType;
        /* fallthrough chain from `case 7` down to `case 2` */
        if src_size >= 7 {
            bit_d.bitContainer = bit_d.bitContainer.wrapping_add(
                (*src_buffer.add(6) as BitContainerType) << (CONTAINER_BITS - 16),
            );
        }
        if src_size >= 6 {
            bit_d.bitContainer = bit_d.bitContainer.wrapping_add(
                (*src_buffer.add(5) as BitContainerType) << (CONTAINER_BITS - 24),
            );
        }
        if src_size >= 5 {
            bit_d.bitContainer = bit_d.bitContainer.wrapping_add(
                (*src_buffer.add(4) as BitContainerType) << (CONTAINER_BITS - 32),
            );
        }
        if src_size >= 4 {
            bit_d.bitContainer = bit_d
                .bitContainer
                .wrapping_add((*src_buffer.add(3) as BitContainerType) << 24);
        }
        if src_size >= 3 {
            bit_d.bitContainer = bit_d
                .bitContainer
                .wrapping_add((*src_buffer.add(2) as BitContainerType) << 16);
        }
        if src_size >= 2 {
            bit_d.bitContainer = bit_d
                .bitContainer
                .wrapping_add((*src_buffer.add(1) as BitContainerType) << 8);
        }
        let last_byte = *src_buffer.add(src_size - 1);
        bit_d.bitsConsumed = if last_byte != 0 {
            8 - zstd_highbit32(last_byte as U32)
        } else {
            0
        };
        if last_byte == 0 {
            return err_code(ZSTD_error_corruption_detected); /* endMark not present */
        }
        bit_d.bitsConsumed += ((CONTAINER_BYTES - src_size) * 8) as u32;
    }

    src_size
}

/// `BIT_getUpperBits()`
#[inline(always)]
pub fn bit_get_upper_bits(bit_container: BitContainerType, start: u32) -> BitContainerType {
    bit_container >> start
}

/// `BIT_getMiddleBits()`
#[inline(always)]
pub fn bit_get_middle_bits(
    bit_container: BitContainerType,
    start: u32,
    nb_bits: u32,
) -> BitContainerType {
    let reg_mask = CONTAINER_BITS - 1;
    /* x86_64 path in the C code */
    (bit_container >> (start & reg_mask)) & (((1u64) << nb_bits) - 1) as BitContainerType
}

/// `BIT_lookBits()`
#[inline(always)]
pub fn bit_look_bits(bit_d: &BIT_DStream_t, nb_bits: u32) -> BitContainerType {
    bit_get_middle_bits(
        bit_d.bitContainer,
        CONTAINER_BITS
            .wrapping_sub(bit_d.bitsConsumed)
            .wrapping_sub(nb_bits),
        nb_bits,
    )
}

/// `BIT_lookBitsFast()`
#[inline(always)]
pub fn bit_look_bits_fast(bit_d: &BIT_DStream_t, nb_bits: u32) -> BitContainerType {
    let reg_mask = CONTAINER_BITS - 1;
    debug_assert!(nb_bits >= 1);
    (bit_d.bitContainer << (bit_d.bitsConsumed & reg_mask))
        >> (((reg_mask + 1) - nb_bits) & reg_mask)
}

/// `BIT_skipBits()`
#[inline(always)]
pub fn bit_skip_bits(bit_d: &mut BIT_DStream_t, nb_bits: u32) {
    bit_d.bitsConsumed += nb_bits;
}

/// `BIT_readBits()`
#[inline(always)]
pub fn bit_read_bits(bit_d: &mut BIT_DStream_t, nb_bits: u32) -> BitContainerType {
    let value = bit_look_bits(bit_d, nb_bits);
    bit_skip_bits(bit_d, nb_bits);
    value
}

/// `BIT_readBitsFast()`
#[inline(always)]
pub fn bit_read_bits_fast(bit_d: &mut BIT_DStream_t, nb_bits: u32) -> BitContainerType {
    let value = bit_look_bits_fast(bit_d, nb_bits);
    debug_assert!(nb_bits >= 1);
    bit_skip_bits(bit_d, nb_bits);
    value
}

/// `BIT_reloadDStream_internal()`
#[inline(always)]
pub unsafe fn bit_reload_dstream_internal(bit_d: &mut BIT_DStream_t) -> BIT_DStream_status {
    debug_assert!(bit_d.bitsConsumed <= CONTAINER_BITS);
    bit_d.ptr = bit_d.ptr.sub((bit_d.bitsConsumed >> 3) as usize);
    bit_d.bitsConsumed &= 7;
    bit_d.bitContainer = mem_read_lest(bit_d.ptr);
    BIT_DStream_status::unfinished
}

/// `BIT_reloadDStreamFast()`
#[inline(always)]
pub unsafe fn bit_reload_dstream_fast(bit_d: &mut BIT_DStream_t) -> BIT_DStream_status {
    if bit_d.ptr < bit_d.limitPtr {
        return BIT_DStream_status::overflow;
    }
    bit_reload_dstream_internal(bit_d)
}

/// The `static const BitContainerType zeroFilled = 0;` used by the overflow path.
static ZERO_FILLED: BitContainerType = 0;

/// `BIT_reloadDStream()`
#[inline(always)]
pub unsafe fn bit_reload_dstream(bit_d: &mut BIT_DStream_t) -> BIT_DStream_status {
    if bit_d.bitsConsumed > CONTAINER_BITS {
        bit_d.ptr = &ZERO_FILLED as *const BitContainerType as *const u8;
        return BIT_DStream_status::overflow;
    }

    if bit_d.ptr >= bit_d.limitPtr {
        return bit_reload_dstream_internal(bit_d);
    }
    if bit_d.ptr == bit_d.start {
        if bit_d.bitsConsumed < CONTAINER_BITS {
            return BIT_DStream_status::endOfBuffer;
        }
        return BIT_DStream_status::completed;
    }
    /* start < ptr < limitPtr => cautious update */
    let mut nb_bytes = bit_d.bitsConsumed >> 3;
    let mut result = BIT_DStream_status::unfinished;
    if (bit_d.ptr as usize) - (nb_bytes as usize) < bit_d.start as usize {
        nb_bytes = (bit_d.ptr as usize - bit_d.start as usize) as u32;
        result = BIT_DStream_status::endOfBuffer;
    }
    bit_d.ptr = bit_d.ptr.sub(nb_bytes as usize);
    bit_d.bitsConsumed -= nb_bytes * 8;
    bit_d.bitContainer = mem_read_lest(bit_d.ptr);
    result
}

/// `BIT_endOfDStream()`
#[inline(always)]
pub fn bit_end_of_dstream(d_stream: &BIT_DStream_t) -> bool {
    d_stream.ptr == d_stream.start && d_stream.bitsConsumed == CONTAINER_BITS
}
