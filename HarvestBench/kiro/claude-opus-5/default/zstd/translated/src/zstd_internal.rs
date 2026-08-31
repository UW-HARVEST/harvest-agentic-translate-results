//! Translation of `common/zstd_internal.h` — constants and shared helpers.
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use crate::mem::*;

pub const ZSTD_OPT_NUM: u32 = 1 << 12;
pub const ZSTD_REP_NUM: usize = 3;
pub static repStartValue: [U32; ZSTD_REP_NUM] = [1, 4, 8];

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;
pub const BIT1: u32 = 2;
pub const BIT0: u32 = 1;

pub const ZSTD_WINDOWLOG_ABSOLUTEMIN: u32 = 10;
pub static ZSTD_fcs_fieldSize: [usize; 4] = [0, 2, 4, 8];
pub static ZSTD_did_fieldSize: [usize; 4] = [0, 1, 2, 4];

pub const ZSTD_FRAMEIDSIZE: usize = 4;
pub const ZSTD_BLOCKHEADERSIZE: usize = 3;
pub const ZSTD_blockHeaderSize: usize = ZSTD_BLOCKHEADERSIZE;

/// `blockType_e`
pub const bt_raw: u32 = 0;
pub const bt_rle: u32 = 1;
pub const bt_compressed: u32 = 2;
pub const bt_reserved: u32 = 3;

pub const ZSTD_FRAMECHECKSUMSIZE: usize = 4;

pub const MIN_SEQUENCES_SIZE: usize = 1;
pub const MIN_CBLOCK_SIZE: usize = 2;
pub const MIN_LITERALS_FOR_4_STREAMS: usize = 6;

/// `SymbolEncodingType_e`
pub const set_basic: u32 = 0;
pub const set_rle: u32 = 1;
pub const set_compressed: u32 = 2;
pub const set_repeat: u32 = 3;

pub const LONGNBSEQ: u32 = 0x7F00;

pub const MINMATCH: usize = 3;

pub const Litbits: u32 = 8;
pub const LitHufLog: u32 = 11;
pub const MaxLit: u32 = (1 << Litbits) - 1;
pub const MaxML: u32 = 52;
pub const MaxLL: u32 = 35;
pub const DefaultMaxOff: u32 = 28;
pub const MaxOff: u32 = 31;
pub const MaxSeq: u32 = if MaxLL > MaxML { MaxLL } else { MaxML };
pub const MLFSELog: u32 = 9;
pub const LLFSELog: u32 = 9;
pub const OffFSELog: u32 = 8;
pub const MaxFSELog: u32 = 9;
pub const MaxMLBits: u32 = 16;
pub const MaxLLBits: u32 = 16;

pub const ZSTD_MAX_HUF_HEADER_SIZE: usize = 128;
pub const ZSTD_MAX_FSE_HEADERS_SIZE: usize = (((MaxML + 1) * MLFSELog
    + (MaxLL + 1) * LLFSELog
    + (MaxOff + 1) * OffFSELog
    + 7)
    / 8) as usize;

pub static LL_bits: [u8; (MaxLL + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
pub static LL_defaultNorm: [S16; (MaxLL + 1) as usize] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
pub const LL_DEFAULTNORMLOG: u32 = 6;
pub const LL_defaultNormLog: U32 = LL_DEFAULTNORMLOG;

pub static ML_bits: [u8; (MaxML + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
pub static ML_defaultNorm: [S16; (MaxML + 1) as usize] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
pub const ML_DEFAULTNORMLOG: u32 = 6;
pub const ML_defaultNormLog: U32 = ML_DEFAULTNORMLOG;

pub static OF_defaultNorm: [S16; (DefaultMaxOff + 1) as usize] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
pub const OF_DEFAULTNORMLOG: u32 = 5;
pub const OF_defaultNormLog: U32 = OF_DEFAULTNORMLOG;

/// `ZSTD_copy8()`
#[inline(always)]
pub unsafe fn zstd_copy8(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, 8);
}

/// `ZSTD_copy16()` — GCC path copies through a 16-byte stack buffer.
#[inline(always)]
pub unsafe fn zstd_copy16(dst: *mut u8, src: *const u8) {
    let mut buf = [0u8; 16];
    core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), 16);
    core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, 16);
}

pub const WILDCOPY_OVERLENGTH: usize = 32;
pub const WILDCOPY_VECLEN: isize = 16;

/// `ZSTD_overlap_e`
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ZSTD_overlap_e {
    ZSTD_no_overlap,
    ZSTD_overlap_src_before_dst,
}

/// `ZSTD_wildcopy()`
#[inline(always)]
pub unsafe fn zstd_wildcopy(
    dst: *mut u8,
    src: *const u8,
    length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff = (dst as isize) - (src as isize);
    let mut ip = src;
    let mut op = dst;
    let oend = op.offset(length);

    if ovtype == ZSTD_overlap_e::ZSTD_overlap_src_before_dst && diff < WILDCOPY_VECLEN {
        loop {
            zstd_copy8(op, ip);
            op = op.add(8);
            ip = ip.add(8);
            if op >= oend {
                break;
            }
        }
    } else {
        zstd_copy16(op, ip);
        if 16 >= length {
            return;
        }
        op = op.add(16);
        ip = ip.add(16);
        loop {
            zstd_copy16(op, ip);
            op = op.add(16);
            ip = ip.add(16);
            zstd_copy16(op, ip);
            op = op.add(16);
            ip = ip.add(16);
            if op >= oend {
                break;
            }
        }
    }
}

/// `ZSTD_limitCopy()`
#[inline(always)]
pub unsafe fn zstd_limit_copy(
    dst: *mut u8,
    dst_capacity: usize,
    src: *const u8,
    src_size: usize,
) -> usize {
    let length = if dst_capacity < src_size {
        dst_capacity
    } else {
        src_size
    };
    if length > 0 {
        core::ptr::copy_nonoverlapping(src, dst, length);
    }
    length
}

pub const ZSTD_WORKSPACETOOLARGE_FACTOR: u32 = 3;
pub const ZSTD_WORKSPACETOOLARGE_MAXDURATION: u32 = 128;

/// `ZSTD_bufferMode_e`
pub const ZSTD_bm_buffered: u32 = 0;
pub const ZSTD_bm_stable: u32 = 1;

/// `ZSTD_frameSizeInfo`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_frameSizeInfo {
    pub nbBlocks: usize,
    pub compressedSize: usize,
    pub decompressedBound: u64,
}

/// `blockProperties_t`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct blockProperties_t {
    pub blockType: u32,
    pub lastBlock: U32,
    pub origSize: U32,
}

#[inline(always)]
pub fn min_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
pub fn max_usize(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

#[inline(always)]
pub fn min_u32(a: U32, b: U32) -> U32 {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
pub fn max_u32(a: U32, b: U32) -> U32 {
    if a > b {
        a
    } else {
        b
    }
}
