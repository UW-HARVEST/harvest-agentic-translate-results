//! Translation of `common/zstd_internal.h`, `common/allocations.h` and `common/cpu.h`.
#![allow(dead_code)]

use core::ffi::{c_uint, c_void};

use crate::cmem::*;
use crate::zstd_h::*;

pub const ZSTD_OPT_NUM: usize = 1 << 12;
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

pub type blockType_e = c_uint;
pub const bt_raw: blockType_e = 0;
pub const bt_rle: blockType_e = 1;
pub const bt_compressed: blockType_e = 2;
pub const bt_reserved: blockType_e = 3;

pub const ZSTD_FRAMECHECKSUMSIZE: usize = 4;

pub const MIN_SEQUENCES_SIZE: usize = 1;
pub const MIN_CBLOCK_SIZE: usize = 1 + 1;
pub const MIN_LITERALS_FOR_4_STREAMS: usize = 6;

pub type SymbolEncodingType_e = c_uint;
pub const set_basic: SymbolEncodingType_e = 0;
pub const set_rle: SymbolEncodingType_e = 1;
pub const set_compressed: SymbolEncodingType_e = 2;
pub const set_repeat: SymbolEncodingType_e = 3;

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

pub static LL_bits: [U8; (MaxLL + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
pub static LL_defaultNorm: [S16; (MaxLL + 1) as usize] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
pub const LL_DEFAULTNORMLOG: u32 = 6;
pub const LL_defaultNormLog: U32 = LL_DEFAULTNORMLOG;

pub static ML_bits: [U8; (MaxML + 1) as usize] = [
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

pub const WILDCOPY_OVERLENGTH: usize = 32;
pub const WILDCOPY_VECLEN: isize = 16;

pub type ZSTD_overlap_e = c_uint;
pub const ZSTD_no_overlap: ZSTD_overlap_e = 0;
pub const ZSTD_overlap_src_before_dst: ZSTD_overlap_e = 1;

pub const ZSTD_WORKSPACETOOLARGE_FACTOR: usize = 3;
pub const ZSTD_WORKSPACETOOLARGE_MAXDURATION: u32 = 128;

pub type ZSTD_bufferMode_e = c_uint;
pub const ZSTD_bm_buffered: ZSTD_bufferMode_e = 0;
pub const ZSTD_bm_stable: ZSTD_bufferMode_e = 1;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_frameSizeInfo {
    pub nbBlocks: usize,
    pub compressedSize: usize,
    pub decompressedBound: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct blockProperties_t {
    pub blockType: blockType_e,
    pub lastBlock: U32,
    pub origSize: U32,
}

#[inline(always)]
pub unsafe fn ZSTD_copy8(dst: *mut c_void, src: *const c_void) {
    ZSTD_memcpy(dst, src, 8);
}

#[inline(always)]
pub unsafe fn ZSTD_copy16(dst: *mut c_void, src: *const c_void) {
    let mut buf: [BYTE; 16] = [0; 16];
    ZSTD_memcpy(buf.as_mut_ptr() as *mut c_void, src, 16);
    ZSTD_memcpy(dst, buf.as_ptr() as *const c_void, 16);
}

#[inline(always)]
pub unsafe fn ZSTD_wildcopy(
    dst: *mut c_void,
    src: *const c_void,
    length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff = (dst as *mut BYTE as isize) - (src as *const BYTE as isize);
    let mut ip = src as *const BYTE;
    let mut op = dst as *mut BYTE;
    let oend = op.offset(length);

    if ovtype == ZSTD_overlap_src_before_dst && diff < WILDCOPY_VECLEN {
        loop {
            ZSTD_copy8(op as *mut c_void, ip as *const c_void);
            op = op.add(8);
            ip = ip.add(8);
            if op >= oend {
                break;
            }
        }
    } else {
        ZSTD_copy16(op as *mut c_void, ip as *const c_void);
        if 16 >= length {
            return;
        }
        op = op.add(16);
        ip = ip.add(16);
        loop {
            ZSTD_copy16(op as *mut c_void, ip as *const c_void);
            op = op.add(16);
            ip = ip.add(16);
            ZSTD_copy16(op as *mut c_void, ip as *const c_void);
            op = op.add(16);
            ip = ip.add(16);
            if op >= oend {
                break;
            }
        }
    }
}

#[inline(always)]
pub unsafe fn ZSTD_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length = if dstCapacity < srcSize {
        dstCapacity
    } else {
        srcSize
    };
    if length > 0 {
        ZSTD_memcpy(dst, src, length);
    }
    length
}

/* ===== allocations.h ===== */

#[inline(always)]
pub unsafe fn ZSTD_customMalloc(size: usize, customMem: ZSTD_customMem) -> *mut c_void {
    if let Some(f) = customMem.customAlloc {
        return f(customMem.opaque, size);
    }
    malloc(size)
}

#[inline(always)]
pub unsafe fn ZSTD_customCalloc(size: usize, customMem: ZSTD_customMem) -> *mut c_void {
    if let Some(f) = customMem.customAlloc {
        let ptr = f(customMem.opaque, size);
        ZSTD_memset(ptr, 0, size);
        return ptr;
    }
    calloc(1, size)
}

#[inline(always)]
pub unsafe fn ZSTD_customFree(ptr: *mut c_void, customMem: ZSTD_customMem) {
    if !ptr.is_null() {
        if let Some(f) = customMem.customFree {
            f(customMem.opaque, ptr);
        } else {
            free(ptr);
        }
    }
}

/* ===== cpu.h ===== */

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_cpuid_t {
    pub f1c: U32,
    pub f1d: U32,
    pub f7b: U32,
    pub f7c: U32,
}

/// Translation of `ZSTD_cpuid()` from `common/cpu.h` (GCC x86-64 branch).
pub fn ZSTD_cpuid() -> ZSTD_cpuid_t {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::__cpuid_count;
        let mut r = ZSTD_cpuid_t::default();
        let leaf0 = __cpuid_count(0, 0);
        let n = leaf0.eax;
        if n >= 1 {
            let l1 = __cpuid_count(1, 0);
            r.f1c = l1.ecx;
            r.f1d = l1.edx;
        }
        if n >= 7 {
            let l7 = __cpuid_count(7, 0);
            r.f7b = l7.ebx;
            r.f7c = l7.ecx;
        }
        r
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        ZSTD_cpuid_t::default()
    }
}

#[inline(always)]
pub fn ZSTD_cpuid_bmi1(cpuid: ZSTD_cpuid_t) -> u32 {
    ((cpuid.f7b >> 3) & 1) as u32
}

#[inline(always)]
pub fn ZSTD_cpuid_bmi2(cpuid: ZSTD_cpuid_t) -> u32 {
    ((cpuid.f7b >> 8) & 1) as u32
}

/// `ZSTD_cpuSupportsBmi2()` from `common/zstd_internal.h`.
#[inline(always)]
pub fn ZSTD_cpuSupportsBmi2() -> i32 {
    let cpuid = ZSTD_cpuid();
    ((ZSTD_cpuid_bmi1(cpuid) != 0) && (ZSTD_cpuid_bmi2(cpuid) != 0)) as i32
}
