//! Constants and small helpers from common/zstd_internal.h shared across
//! compress/decompress/dictBuilder.
#![allow(dead_code)]
use core::ffi::c_void;

pub const ZSTD_OPT_NUM: usize = 1 << 12;
pub const ZSTD_REP_NUM: usize = 3;
pub static repStartValue: [u32; ZSTD_REP_NUM] = [1, 4, 8];

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

// blockType_e
pub const bt_raw: u32 = 0;
pub const bt_rle: u32 = 1;
pub const bt_compressed: u32 = 2;
pub const bt_reserved: u32 = 3;

pub const ZSTD_FRAMECHECKSUMSIZE: usize = 4;
pub const MIN_SEQUENCES_SIZE: usize = 1;
pub const MIN_CBLOCK_SIZE: usize = 2;
pub const MIN_LITERALS_FOR_4_STREAMS: usize = 6;

// SymbolEncodingType_e
pub const set_basic: u32 = 0;
pub const set_rle: u32 = 1;
pub const set_compressed: u32 = 2;
pub const set_repeat: u32 = 3;

pub const LONGNBSEQ: u32 = 0x7F00;
pub const MINMATCH: u32 = 3;
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

pub static LL_bits: [u8; 36] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
pub static LL_defaultNorm: [i16; 36] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
pub const LL_DEFAULTNORMLOG: u32 = 6;
pub const LL_defaultNormLog: u32 = LL_DEFAULTNORMLOG;

pub static ML_bits: [u8; 53] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
pub static ML_defaultNorm: [i16; 53] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
pub const ML_DEFAULTNORMLOG: u32 = 6;
pub const ML_defaultNormLog: u32 = ML_DEFAULTNORMLOG;

pub static OF_defaultNorm: [i16; 29] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
pub const OF_DEFAULTNORMLOG: u32 = 5;
pub const OF_defaultNormLog: u32 = OF_DEFAULTNORMLOG;

pub const WILDCOPY_OVERLENGTH: usize = 32;
pub const WILDCOPY_VECLEN: usize = 16;

// ZSTD_overlap_e
pub const ZSTD_no_overlap: u32 = 0;
pub const ZSTD_overlap_src_before_dst: u32 = 1;

pub const ZSTD_WORKSPACETOOLARGE_FACTOR: usize = 3;
pub const ZSTD_WORKSPACETOOLARGE_MAXDURATION: usize = 128;

// ZSTD_bufferMode_e
pub const ZSTD_bm_buffered: u32 = 0;
pub const ZSTD_bm_stable: u32 = 1;

#[inline]
pub unsafe fn zstd_copy8(dst: *mut c_void, src: *const c_void) {
    core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, 8);
}
#[inline]
pub unsafe fn zstd_copy16(dst: *mut c_void, src: *const c_void) {
    core::ptr::copy(src as *const u8, dst as *mut u8, 16);
}

#[inline]
pub unsafe fn zstd_wildcopy(
    dst: *mut c_void,
    src: *const c_void,
    length: isize,
    ovtype: u32,
) {
    let diff = (dst as *mut u8 as isize) - (src as *const u8 as isize);
    let mut ip = src as *const u8;
    let mut op = dst as *mut u8;
    let oend = op.offset(length);

    if ovtype == ZSTD_overlap_src_before_dst && diff < WILDCOPY_VECLEN as isize {
        loop {
            zstd_copy8(op as *mut c_void, ip as *const c_void);
            op = op.add(8);
            ip = ip.add(8);
            if op >= oend {
                break;
            }
        }
    } else {
        zstd_copy16(op as *mut c_void, ip as *const c_void);
        if 16 >= length {
            return;
        }
        op = op.add(16);
        ip = ip.add(16);
        loop {
            zstd_copy16(op as *mut c_void, ip as *const c_void);
            op = op.add(16);
            ip = ip.add(16);
            zstd_copy16(op as *mut c_void, ip as *const c_void);
            op = op.add(16);
            ip = ip.add(16);
            if op >= oend {
                break;
            }
        }
    }
}

#[inline]
pub unsafe fn zstd_limit_copy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length = if dstCapacity < srcSize { dstCapacity } else { srcSize };
    if length > 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, length);
    }
    length
}

// blockProperties_t
#[repr(C)]
#[derive(Clone, Copy)]
pub struct blockProperties_t {
    pub blockType: u32,
    pub lastBlock: u32,
    pub origSize: u32,
}

// ZSTD_frameSizeInfo
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameSizeInfo {
    pub nbBlocks: usize,
    pub compressedSize: usize,
    pub decompressedBound: u64,
}

#[inline]
pub fn zstd_cpu_supports_bmi2() -> i32 {
    0
}
