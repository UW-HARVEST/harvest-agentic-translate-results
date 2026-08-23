//! Transliteration of the SECOND half of `legacy/zstd_v05.c` (C lines 2494 - 4005):
//! the `ZSTDv05_*` decompressor proper (context management, frame / block /
//! literals / sequences decoding, the bufferless streaming API and the
//! dictionary loader) followed by the buffered `ZBUFFv05_*` streaming
//! decompression layer.
//!
//! C lines 1 - 2493 (the bundled `mem.h`, `zstd_internal.h`, `fse.h` /
//! `bitstream.h` / `fse.c` (FSEv05) and `huff0.h` / `huff0.c` (HUFv05)
//! sections) live in the sibling module `v05_ent`, which is glob-imported
//! below.  Nothing defined there is redefined here.
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens,
    unused_imports,
    unused_comparisons,
    unused_labels
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::error_private::*;
use crate::legacy::v05_ent::*;
use crate::mem::{calloc, free, malloc, memcpy, memmove, memset, qsort};

/*-*************************************
*  Constants (zstd_v05.h)
***************************************/
pub const ZSTDv05_MAGICNUMBER: U32 = 0xFD2FB525; /* v0.5 */

/*-************************
*  Advanced Streaming API  (zstd_v05.h)
***************************/
pub type ZSTDv05_strategy = c_int;
pub const ZSTDv05_fast: ZSTDv05_strategy = 0;
pub const ZSTDv05_greedy: ZSTDv05_strategy = 1;
pub const ZSTDv05_lazy: ZSTDv05_strategy = 2;
pub const ZSTDv05_lazy2: ZSTDv05_strategy = 3;
pub const ZSTDv05_btlazy2: ZSTDv05_strategy = 4;
pub const ZSTDv05_opt: ZSTDv05_strategy = 5;
pub const ZSTDv05_btopt: ZSTDv05_strategy = 6;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTDv05_parameters {
    pub srcSize: U64,
    pub windowLog: U32, /* the only useful information to retrieve */
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub targetLength: U32,
    pub strategy: ZSTDv05_strategy,
}

/* ***************************************************************
*  Tuning parameters
*****************************************************************/
/*
 * HEAPMODE :
 * Select how default decompression function ZSTDv05_decompress() will allocate memory,
 * in memory stack (0), or in memory heap (1, requires malloc())
 */
pub const ZSTDv05_HEAPMODE: c_int = 1;

/*-*************************************
*  Local types
***************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

/* *******************************************************
*  Memory operations
**********************************************************/
pub unsafe fn ZSTDv05_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

/* *************************************
*  Error Management
***************************************/
/* ! ZSTDv05_isError() :
*   tells if a return value is an error code */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv05_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* ! ZSTDv05_getErrorName() :
*   provides error code string (useful for debugging) */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv05_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* *************************************************************
*   Context management
***************************************************************/
pub type ZSTDv05_dStage = c_int;
pub const ZSTDv05ds_getFrameHeaderSize: ZSTDv05_dStage = 0;
pub const ZSTDv05ds_decodeFrameHeader: ZSTDv05_dStage = 1;
pub const ZSTDv05ds_decodeBlockHeader: ZSTDv05_dStage = 2;
pub const ZSTDv05ds_decompressBlock: ZSTDv05_dStage = 3;

#[repr(C)]
pub struct ZSTDv05_DCtx {
    pub LLTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(LLFSEv05Log)],
    pub OffTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(OffFSEv05Log)],
    pub MLTable: [FSEv05_DTable; FSEv05_DTABLE_SIZE_U32(MLFSEv05Log)],
    pub hufTableX4: [c_uint; HUFv05_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub headerSize: usize,
    pub params: ZSTDv05_parameters,
    pub bType: blockType_t, /* used in ZSTDv05_decompressContinue(), to transfer blockType between header decoding and block decoding stages */
    pub stage: ZSTDv05_dStage,
    pub flagStaticTables: U32,
    pub litPtr: *const BYTE,
    pub litSize: usize,
    pub litBuffer: [BYTE; BLOCKSIZE + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv05_frameHeaderSize_max],
} /* typedef'd to ZSTDv05_DCtx within "zstd_static.h" */

#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv05_sizeofDCtx() -> usize {
    core::mem::size_of::<ZSTDv05_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBegin(dctx: *mut ZSTDv05_DCtx) -> usize {
    (*dctx).expected = ZSTDv05_frameHeaderSize_min;
    (*dctx).stage = ZSTDv05ds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTableX4[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG;
    (*dctx).flagStaticTables = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_createDCtx() -> *mut ZSTDv05_DCtx {
    let dctx: *mut ZSTDv05_DCtx =
        malloc(core::mem::size_of::<ZSTDv05_DCtx>()) as *mut ZSTDv05_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv05_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_freeDCtx(dctx: *mut ZSTDv05_DCtx) -> usize {
    free(dctx as *mut c_void);
    0 /* reserved as a potential error code in the future */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_copyDCtx(
    dstDCtx: *mut ZSTDv05_DCtx,
    srcDCtx: *const ZSTDv05_DCtx,
) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv05_DCtx>()
            - (BLOCKSIZE + WILDCOPY_OVERLENGTH + ZSTDv05_frameHeaderSize_max),
    ); /* no need to copy workspace */
}

/* *************************************************************
*   Decompression section
***************************************************************/

/* Frame format description
   Frame Header -  [ Block Header - Block ] - Frame End
   1) Frame Header
      - 4 bytes - Magic Number : ZSTDv05_MAGICNUMBER (defined within zstd_internal.h)
      - 1 byte  - Window Descriptor
   2) Block Header
      - 3 bytes, starting with a 2-bits descriptor
                 Uncompressed, Compressed, Frame End, unused
   3) Block
      See Block Format Description
   4) Frame End
      - 3 bytes, compatible with Block Header
*/

/* * ZSTDv05_decodeFrameHeader_Part1() :
*   decode the 1st part of the Frame Header, which tells Frame Header size.
*   srcSize must be == ZSTDv05_frameHeaderSize_min.
*   @return : the full size of the Frame Header */
pub unsafe fn ZSTDv05_decodeFrameHeader_Part1(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize != ZSTDv05_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    (*zc).headerSize = ZSTDv05_frameHeaderSize_min;
    (*zc).headerSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_getFrameParams(
    params: *mut ZSTDv05_parameters,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let magicNumber: U32;
    if srcSize < ZSTDv05_frameHeaderSize_min {
        return ZSTDv05_frameHeaderSize_max;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTDv05_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }
    memset(
        params as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv05_parameters>(),
    );
    (*params).windowLog =
        ((*(src as *const BYTE).wrapping_add(4) & 15) as U32).wrapping_add(ZSTDv05_WINDOWLOG_ABSOLUTEMIN);
    if (*(src as *const BYTE).wrapping_add(4) >> 4) != 0 {
        return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved bits */
    }
    0
}

/* * ZSTDv05_decodeFrameHeader_Part2() :
*   decode the full Frame Header.
*   srcSize must be the size provided by ZSTDv05_decodeFrameHeader_Part1().
*   @return : 0, or an error code, which can be tested using ZSTDv05_isError() */
pub unsafe fn ZSTDv05_decodeFrameHeader_Part2(
    zc: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result: usize;
    if srcSize != (*zc).headerSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    result = ZSTDv05_getFrameParams(&mut (*zc).params, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).params.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

pub unsafe fn ZSTDv05_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let headerFlags: BYTE;
    let cSize: U32;

    if srcSize < 3 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    headerFlags = *in_;
    cSize = (*in_.wrapping_add(2) as U32)
        .wrapping_add((*in_.wrapping_add(1) as U32) << 8)
        .wrapping_add(((*in_.wrapping_add(0) & 7) as U32) << 16);

    (*bpPtr).blockType = (headerFlags >> 6) as blockType_t;
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

pub unsafe fn ZSTDv05_copyRawBlock(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if dst.is_null() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

/* ! ZSTDv05_decodeLiteralsBlock() :
    @return : nb of bytes read from src (< srcSize ) */
pub unsafe fn ZSTDv05_decodeLiteralsBlock(
    dctx: *mut ZSTDv05_DCtx,
    src: *const c_void,
    srcSize: usize, /* note : srcSize < BLOCKSIZE */
) -> usize {
    let istart: *const BYTE = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    let selector: U32 = (*istart.wrapping_add(0) >> 6) as U32;

    if selector == IS_HUFv05 {
        let mut litSize: usize;
        let litCSize: usize;
        let mut singleStream: usize = 0;
        let mut lhSize: U32 = ((*istart.wrapping_add(0)) >> 4) as U32 & 3;
        if srcSize < 5 {
            return ERROR(ZSTD_error_corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for case 3 */
        }
        if lhSize == 2 {
            /* 2 - 2 - 14 - 14 */
            lhSize = 4;
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 10)
                .wrapping_add((*istart.wrapping_add(1) as usize) << 2)
                .wrapping_add((*istart.wrapping_add(2) >> 6) as usize);
            litCSize = (((*istart.wrapping_add(2) & 63) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(3) as usize);
        } else if lhSize == 3 {
            /* 2 - 2 - 18 - 18 */
            lhSize = 5;
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 14)
                .wrapping_add((*istart.wrapping_add(1) as usize) << 6)
                .wrapping_add((*istart.wrapping_add(2) >> 2) as usize);
            litCSize = (((*istart.wrapping_add(2) & 3) as usize) << 16)
                .wrapping_add((*istart.wrapping_add(3) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(4) as usize);
        } else {
            /* case 0: case 1: default: note : default is impossible, since lhSize into [0..3] */
            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            singleStream = (*istart.wrapping_add(0) & 16) as usize;
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 6)
                .wrapping_add((*istart.wrapping_add(1) >> 2) as usize);
            litCSize = (((*istart.wrapping_add(1) & 3) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(2) as usize);
        }
        if litSize > BLOCKSIZE {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if litCSize.wrapping_add(lhSize as usize) > srcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }

        if HUFv05_isError(if singleStream != 0 {
            HUFv05_decompress1X2(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litCSize,
            )
        } else {
            HUFv05_decompress(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                litSize,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litCSize,
            )
        }) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }

        (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
        (*dctx).litSize = litSize;
        memset(
            (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
            0,
            WILDCOPY_OVERLENGTH,
        );
        return litCSize.wrapping_add(lhSize as usize);
    } else if selector == IS_PCH {
        let errorCode: usize;
        let litSize: usize;
        let litCSize: usize;
        let mut lhSize: U32 = ((*istart.wrapping_add(0)) >> 4) as U32 & 3;
        if lhSize != 1 {
            /* only case supported for now : small litSize, single stream */
            return ERROR(ZSTD_error_corruption_detected);
        }
        if (*dctx).flagStaticTables == 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }

        /* 2 - 2 - 10 - 10 */
        lhSize = 3;
        litSize = (((*istart.wrapping_add(0) & 15) as usize) << 6)
            .wrapping_add((*istart.wrapping_add(1) >> 2) as usize);
        litCSize = (((*istart.wrapping_add(1) & 3) as usize) << 8)
            .wrapping_add(*istart.wrapping_add(2) as usize);
        if litCSize.wrapping_add(lhSize as usize) > srcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }

        errorCode = HUFv05_decompress1X4_usingDTable(
            (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
            litSize,
            istart.wrapping_add(lhSize as usize) as *const c_void,
            litCSize,
            (*dctx).hufTableX4.as_ptr(),
        );
        if HUFv05_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
        (*dctx).litSize = litSize;
        memset(
            (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
            0,
            WILDCOPY_OVERLENGTH,
        );
        return litCSize.wrapping_add(lhSize as usize);
    } else if selector == IS_RAW {
        let litSize: usize;
        let mut lhSize: U32 = ((*istart.wrapping_add(0)) >> 4) as U32 & 3;
        if lhSize == 2 {
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(1) as usize);
        } else if lhSize == 3 {
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 16)
                .wrapping_add((*istart.wrapping_add(1) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(2) as usize);
        } else {
            /* case 0: case 1: default: note : default is impossible, since lhSize into [0..3] */
            lhSize = 1;
            litSize = (*istart.wrapping_add(0) & 31) as usize;
        }

        if (lhSize as usize)
            .wrapping_add(litSize)
            .wrapping_add(WILDCOPY_OVERLENGTH)
            > srcSize
        {
            /* risk reading beyond src buffer with wildcopy */
            if litSize.wrapping_add(lhSize as usize) > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memcpy(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                istart.wrapping_add(lhSize as usize) as *const c_void,
                litSize,
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            return (lhSize as usize).wrapping_add(litSize);
        }
        /* direct reference into compressed stream */
        (*dctx).litPtr = istart.wrapping_add(lhSize as usize);
        (*dctx).litSize = litSize;
        return (lhSize as usize).wrapping_add(litSize);
    } else if selector == IS_RLE {
        let litSize: usize;
        let mut lhSize: U32 = ((*istart.wrapping_add(0)) >> 4) as U32 & 3;
        if lhSize == 2 {
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(1) as usize);
        } else if lhSize == 3 {
            litSize = (((*istart.wrapping_add(0) & 15) as usize) << 16)
                .wrapping_add((*istart.wrapping_add(1) as usize) << 8)
                .wrapping_add(*istart.wrapping_add(2) as usize);
            if srcSize < 4 {
                return ERROR(ZSTD_error_corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */
            }
        } else {
            /* case 0: case 1: default: note : default is impossible, since lhSize into [0..3] */
            lhSize = 1;
            litSize = (*istart.wrapping_add(0) & 31) as usize;
        }
        if litSize > BLOCKSIZE {
            return ERROR(ZSTD_error_corruption_detected);
        }
        memset(
            (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
            *istart.wrapping_add(lhSize as usize) as c_int,
            litSize.wrapping_add(WILDCOPY_OVERLENGTH),
        );
        (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
        (*dctx).litSize = litSize;
        return (lhSize as usize).wrapping_add(1);
    }

    /* default: */
    ERROR(ZSTD_error_corruption_detected) /* impossible */
}

pub unsafe fn ZSTDv05_decodeSeqHeaders(
    nbSeq: *mut c_int,
    dumpsPtr: *mut *const BYTE,
    dumpsLengthPtr: *mut usize,
    DTableLL: *mut FSEv05_DTable,
    DTableML: *mut FSEv05_DTable,
    DTableOffb: *mut FSEv05_DTable,
    src: *const c_void,
    srcSize: usize,
    flagStaticTable: U32,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let LLtype: U32;
    let Offtype: U32;
    let MLtype: U32;
    let mut LLlog: c_uint = 0;
    let mut Offlog: c_uint = 0;
    let mut MLlog: c_uint = 0;
    let mut dumpsLength: usize;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    *nbSeq = *ip as c_int;
    ip = ip.wrapping_add(1);
    if *nbSeq == 0 {
        return 1;
    }
    if *nbSeq >= 128 {
        if ip >= iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        *nbSeq = ((*nbSeq).wrapping_sub(128).wrapping_shl(8)).wrapping_add(*ip as c_int);
        ip = ip.wrapping_add(1);
    }

    if ip >= iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    LLtype = (*ip >> 6) as U32;
    Offtype = ((*ip >> 4) & 3) as U32;
    MLtype = ((*ip >> 2) & 3) as U32;
    if (*ip & 2) != 0 {
        if ip.wrapping_add(3) > iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        dumpsLength = *ip.wrapping_add(2) as usize;
        dumpsLength = dumpsLength.wrapping_add((*ip.wrapping_add(1) as usize) << 8);
        ip = ip.wrapping_add(3);
    } else {
        if ip.wrapping_add(2) > iend {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        dumpsLength = *ip.wrapping_add(1) as usize;
        dumpsLength = dumpsLength.wrapping_add(((*ip.wrapping_add(0) & 1) as usize) << 8);
        ip = ip.wrapping_add(2);
    }
    *dumpsPtr = ip;
    ip = ip.wrapping_add(dumpsLength);
    *dumpsLengthPtr = dumpsLength;

    /* check */
    if ip > iend.wrapping_sub(3) {
        return ERROR(ZSTD_error_srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */
    }

    /* sequences */
    {
        let mut norm: [S16; MaxML as usize + 1] = [0; MaxML as usize + 1]; /* assumption : MaxML >= MaxLL >= MaxOff */
        let mut headerSize: usize;

        /* Build DTables */
        if LLtype == FSEv05_ENCODING_RLE {
            LLlog = 0;
            FSEv05_buildDTable_rle(DTableLL, *ip);
            ip = ip.wrapping_add(1);
        } else if LLtype == FSEv05_ENCODING_RAW {
            LLlog = LLbits;
            FSEv05_buildDTable_raw(DTableLL, LLbits);
        } else if LLtype == FSEv05_ENCODING_STATIC {
            if flagStaticTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        } else {
            /* FSEv05_ENCODING_DYNAMIC, default : impossible */
            let mut max: c_uint = MaxLL;
            headerSize = FSEv05_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut LLlog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
            );
            if FSEv05_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if LLlog > LLFSEv05Log {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(headerSize);
            FSEv05_buildDTable(DTableLL, norm.as_ptr(), max, LLlog);
        }

        if Offtype == FSEv05_ENCODING_RLE {
            Offlog = 0;
            if ip > iend.wrapping_sub(2) {
                return ERROR(ZSTD_error_srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */
            }
            FSEv05_buildDTable_rle(DTableOffb, ((*ip as U32) & MaxOff) as BYTE); /* if *ip > MaxOff, data is corrupted */
            ip = ip.wrapping_add(1);
        } else if Offtype == FSEv05_ENCODING_RAW {
            Offlog = Offbits;
            FSEv05_buildDTable_raw(DTableOffb, Offbits);
        } else if Offtype == FSEv05_ENCODING_STATIC {
            if flagStaticTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        } else {
            /* FSEv05_ENCODING_DYNAMIC, default : impossible */
            let mut max: c_uint = MaxOff;
            headerSize = FSEv05_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut Offlog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
            );
            if FSEv05_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if Offlog > OffFSEv05Log {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(headerSize);
            FSEv05_buildDTable(DTableOffb, norm.as_ptr(), max, Offlog);
        }

        if MLtype == FSEv05_ENCODING_RLE {
            MLlog = 0;
            if ip > iend.wrapping_sub(2) {
                return ERROR(ZSTD_error_srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */
            }
            FSEv05_buildDTable_rle(DTableML, *ip);
            ip = ip.wrapping_add(1);
        } else if MLtype == FSEv05_ENCODING_RAW {
            MLlog = MLbits;
            FSEv05_buildDTable_raw(DTableML, MLbits);
        } else if MLtype == FSEv05_ENCODING_STATIC {
            if flagStaticTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        } else {
            /* FSEv05_ENCODING_DYNAMIC, default : impossible */
            let mut max: c_uint = MaxML;
            headerSize = FSEv05_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut MLlog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
            );
            if FSEv05_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_GENERIC);
            }
            if MLlog > MLFSEv05Log {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(headerSize);
            FSEv05_buildDTable(DTableML, norm.as_ptr(), max, MLlog);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seq_t {
    pub litLength: usize,
    pub matchLength: usize,
    pub offset: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seqState_t {
    pub DStream: BITv05_DStream_t,
    pub stateLL: FSEv05_DState_t,
    pub stateOffb: FSEv05_DState_t,
    pub stateML: FSEv05_DState_t,
    pub prevOffset: usize,
    pub dumps: *const BYTE,
    pub dumpsEnd: *const BYTE,
}

/* local `static const U32 offsetPrefix[MaxOff+1]` of ZSTDv05_decodeSequence() */
pub static offsetPrefix: [U32; MaxOff as usize + 1] = [
    1, /*fake*/
    1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072,
    262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, /*fake*/ 1, 1, 1, 1,
    1,
];

pub unsafe fn ZSTDv05_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    let mut litLength: usize;
    let prevOffset: usize;
    let mut offset: usize;
    let mut matchLength: usize;
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;

    /* Literal length */
    litLength = FSEv05_peakSymbol(&mut (*seqState).stateLL) as usize;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    if litLength == MaxLL as usize {
        let add: U32 = *dumps as U32;
        dumps = dumps.wrapping_add(1);
        if add < 255 {
            litLength = litLength.wrapping_add(add as usize);
        } else if dumps.wrapping_add(2) <= de {
            litLength = MEM_readLE16(dumps as *const c_void) as usize;
            dumps = dumps.wrapping_add(2);
            if ((litLength & 1) != 0) && (dumps < de) {
                litLength = litLength.wrapping_add((*dumps as usize) << 16);
                dumps = dumps.wrapping_add(1);
            }
            litLength >>= 1;
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        } /* late correction, to avoid read overflow (data is now corrupted anyway) */
    }

    /* Offset */
    {
        let offsetCode: U32 = FSEv05_peakSymbol(&mut (*seqState).stateOffb) as U32; /* <= maxOff, by table construction */
        let mut nbBits: U32 = offsetCode.wrapping_sub(1);
        if offsetCode == 0 {
            nbBits = 0; /* cmove */
        }
        offset = (offsetPrefix[offsetCode as usize] as usize)
            .wrapping_add(BITv05_readBits(&mut (*seqState).DStream, nbBits));
        if MEM_32bits() != 0 {
            BITv05_reloadDStream(&mut (*seqState).DStream);
        }
        if offsetCode == 0 {
            offset = prevOffset; /* repcode, cmove */
        }
        if (offsetCode | ((litLength == 0) as U32)) != 0 {
            (*seqState).prevOffset = (*seq).offset; /* cmove */
        }
        FSEv05_decodeSymbol(&mut (*seqState).stateOffb, &mut (*seqState).DStream); /* update */
    }

    /* Literal length update */
    FSEv05_decodeSymbol(&mut (*seqState).stateLL, &mut (*seqState).DStream); /* update */
    if MEM_32bits() != 0 {
        BITv05_reloadDStream(&mut (*seqState).DStream);
    }

    /* MatchLength */
    matchLength = FSEv05_decodeSymbol(&mut (*seqState).stateML, &mut (*seqState).DStream) as usize;
    if matchLength == MaxML as usize {
        let add: U32 = if dumps < de {
            let v = *dumps as U32;
            dumps = dumps.wrapping_add(1);
            v
        } else {
            0
        };
        if add < 255 {
            matchLength = matchLength.wrapping_add(add as usize);
        } else if dumps.wrapping_add(2) <= de {
            matchLength = MEM_readLE16(dumps as *const c_void) as usize;
            dumps = dumps.wrapping_add(2);
            if ((matchLength & 1) != 0) && (dumps < de) {
                matchLength = matchLength.wrapping_add((*dumps as usize) << 16);
                dumps = dumps.wrapping_add(1);
            }
            matchLength >>= 1;
        }
        if dumps >= de {
            dumps = de.wrapping_sub(1);
        } /* late correction, to avoid read overflow (data is now corrupted anyway) */
    }
    matchLength = matchLength.wrapping_add(MINMATCH as usize);

    /* save result */
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}

/* local `static const int dec32table[]` of ZSTDv05_execSequence() : added */
pub static dec32table: [c_int; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
/* local `static const int dec64table[]` of ZSTDv05_execSequence() : subtracted */
pub static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];

pub unsafe fn ZSTDv05_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd: *mut BYTE = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd: *mut BYTE = op.wrapping_add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_8: *mut BYTE = oend.wrapping_sub(8);
    let litEnd: *const BYTE = (*litPtr).wrapping_add(sequence.litLength);
    let mut r#match: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* checks */
    let seqLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);

    if seqLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub(*litPtr as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Now we know there are no overflow in literal nor match lengths, can use pointer checks */
    if oLitEnd > oend_8 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    if oMatchEnd > oend {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* overwrite beyond dst buffer */
    }
    if litEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); /* overRead beyond lit buffer */
    }

    /* copy Literals */
    ZSTDv05_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    ); /* note : oLitEnd <= oend-8 : no risk of overwrite beyond oend */
    op = oLitEnd;
    *litPtr = litEnd; /* update for next sequence */

    /* copy Match */
    if sequence.offset > (oLitEnd as usize).wrapping_sub(base as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(vBase as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        r#match = dictEnd.wrapping_offset(
            0isize.wrapping_sub((base as isize).wrapping_sub(r#match as isize)),
        );
        if r#match.wrapping_add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                r#match as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(r#match as usize);
            memmove(oLitEnd as *mut c_void, r#match as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            r#match = base;
            if op > oend_8 || sequence.matchLength < MINMATCH as usize {
                while op < oMatchEnd {
                    *op = *r#match;
                    op = op.wrapping_add(1);
                    r#match = r#match.wrapping_add(1);
                }
                return sequenceLength;
            }
        }
    }
    /* Requirement: op <= oend_8 */

    /* match within prefix */
    if sequence.offset < 8 {
        /* close range match, overlap */
        let sub2: c_int = dec64table[sequence.offset];
        *op.wrapping_add(0) = *r#match.wrapping_add(0);
        *op.wrapping_add(1) = *r#match.wrapping_add(1);
        *op.wrapping_add(2) = *r#match.wrapping_add(2);
        *op.wrapping_add(3) = *r#match.wrapping_add(3);
        r#match = r#match.wrapping_offset(dec32table[sequence.offset] as isize);
        ZSTDv05_copy4(op.wrapping_add(4) as *mut c_void, r#match as *const c_void);
        r#match = r#match.wrapping_offset(0isize.wrapping_sub(sub2 as isize));
    } else {
        ZSTDv05_copy8(op as *mut c_void, r#match as *const c_void);
    }
    op = op.wrapping_add(8);
    r#match = r#match.wrapping_add(8);

    if oMatchEnd > oend.wrapping_sub((16 - MINMATCH) as usize) {
        if op < oend_8 {
            ZSTDv05_wildcopy(
                op as *mut c_void,
                r#match as *const c_void,
                (oend_8 as isize).wrapping_sub(op as isize),
            );
            r#match =
                r#match.wrapping_offset((oend_8 as isize).wrapping_sub(op as isize));
            op = oend_8;
        }
        while op < oMatchEnd {
            *op = *r#match;
            op = op.wrapping_add(1);
            r#match = r#match.wrapping_add(1);
        }
    } else {
        ZSTDv05_wildcopy(
            op as *mut c_void,
            r#match as *const c_void,
            (sequence.matchLength as isize).wrapping_sub(8),
        ); /* works even if matchLength < 8 */
    }
    sequenceLength
}

pub unsafe fn ZSTDv05_decompressSequences(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
) -> usize {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut errorCode: usize;
    let mut dumpsLength: usize = 0;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.wrapping_add((*dctx).litSize);
    let mut nbSeq: c_int = 0;
    let mut dumps: *const BYTE = core::ptr::null();
    let DTableLL: *mut c_uint = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut c_uint = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut c_uint = (*dctx).OffTable.as_mut_ptr();
    let base: *const BYTE = (*dctx).base as *const BYTE;
    let vBase: *const BYTE = (*dctx).vBase as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

    /* Build Decoding Tables */
    errorCode = ZSTDv05_decodeSeqHeaders(
        &mut nbSeq,
        &mut dumps,
        &mut dumpsLength,
        DTableLL,
        DTableML,
        DTableOffb,
        ip as *const c_void,
        seqSize,
        (*dctx).flagStaticTables,
    );
    if ZSTDv05_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.wrapping_add(errorCode);

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequence: seq_t = core::mem::zeroed();
        let mut seqState: seqState_t = core::mem::zeroed();

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = REPCODE_STARTVALUE as usize;
        seqState.dumps = dumps;
        seqState.dumpsEnd = dumps.wrapping_add(dumpsLength);
        seqState.prevOffset = REPCODE_STARTVALUE as usize;
        errorCode = BITv05_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
        );
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        FSEv05_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv05_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv05_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv05_reloadDStream(&mut seqState.DStream) <= BITv05_DStream_completed)
            && (nbSeq != 0)
        {
            let oneSeqSize: usize;
            nbSeq -= 1;
            ZSTDv05_decodeSequence(&mut sequence, &mut seqState);
            oneSeqSize = ZSTDv05_execSequence(
                op,
                oend,
                sequence,
                &mut litPtr,
                litEnd,
                base,
                vBase,
                dictEnd,
            );
            if ZSTDv05_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
    }

    /* last literal segment */
    {
        let lastLLSize: usize = (litEnd as usize).wrapping_sub(litPtr as usize);
        if litPtr > litEnd {
            return ERROR(ZSTD_error_corruption_detected); /* too many literals already used */
        }
        if op.wrapping_add(lastLLSize) > oend {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if lastLLSize > 0 {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTDv05_checkContinuity(dctx: *mut ZSTDv05_DCtx, dst: *const c_void) {
    if dst != (*dctx).previousDstEnd {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const c_char).wrapping_offset(
            0isize.wrapping_sub(
                ((*dctx).previousDstEnd as *const c_char as isize)
                    .wrapping_sub((*dctx).base as *const c_char as isize),
            ),
        ) as *const c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}

pub unsafe fn ZSTDv05_decompressBlock_internal(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip: *const BYTE = src as *const BYTE;
    let litCSize: usize;

    if srcSize >= BLOCKSIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    litCSize = ZSTDv05_decodeLiteralsBlock(dctx, src, srcSize);
    if ZSTDv05_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.wrapping_add(litCSize);
    srcSize = srcSize.wrapping_sub(litCSize);

    ZSTDv05_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBlock(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv05_checkContinuity(dctx, dst as *const c_void);
    ZSTDv05_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize)
}

/* ! ZSTDv05_decompress_continueDCtx
*   dctx must have been properly initialized */
pub unsafe fn ZSTDv05_decompress_continueDCtx(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut remainingSize: usize = srcSize;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };
    memset(
        &mut blockProperties as *mut blockProperties_t as *mut c_void,
        0,
        core::mem::size_of::<blockProperties_t>(),
    );

    /* Frame Header */
    {
        let mut frameHeaderSize: usize;
        if srcSize < ZSTDv05_frameHeaderSize_min + ZSTDv05_blockHeaderSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        frameHeaderSize =
            ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize.wrapping_add(ZSTDv05_blockHeaderSize) {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize = remainingSize.wrapping_sub(frameHeaderSize);
        frameHeaderSize = ZSTDv05_decodeFrameHeader_Part2(dctx, src, frameHeaderSize);
        if ZSTDv05_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize: usize = ZSTDv05_getcBlockSize(
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
            &mut blockProperties,
        );
        if ZSTDv05_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTDv05_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv05_blockHeaderSize);
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        if blockProperties.blockType == bt_compressed {
            decodedSize = ZSTDv05_decompressBlock_internal(
                dctx,
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_raw {
            decodedSize = ZSTDv05_copyRawBlock(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                ip as *const c_void,
                cBlockSize,
            );
        } else if blockProperties.blockType == bt_rle {
            return ERROR(ZSTD_error_GENERIC); /* not yet supported */
        } else if blockProperties.blockType == bt_end {
            /* end of frame */
            if remainingSize != 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
        } else {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        if cBlockSize == 0 {
            break; /* bt_end */
        }

        if ZSTDv05_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize = remainingSize.wrapping_sub(cBlockSize);
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv05_DCtx,
    refDCtx: *const ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv05_copyDCtx(dctx, refDCtx);
    ZSTDv05_checkContinuity(dctx, dst as *const c_void);
    ZSTDv05_decompress_continueDCtx(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress_usingDict(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTDv05_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv05_checkContinuity(dctx, dst as *const c_void);
    ZSTDv05_decompress_continueDCtx(dctx, dst, maxDstSize, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressDCtx(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv05_decompress_usingDict(
        dctx,
        dst,
        maxDstSize,
        src,
        srcSize,
        core::ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompress(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* ZSTDv05_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx: *mut ZSTDv05_DCtx = ZSTDv05_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv05_decompressDCtx(dctx, dst, maxDstSize, src, srcSize);
    ZSTDv05_freeDCtx(dctx);
    regenSize
}

/* ZSTD_errorFrameSizeInfoLegacy() :
   assumes `cSize` and `dBound` are _not_ NULL */
pub unsafe fn ZSTD_errorFrameSizeInfoLegacy(
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
    ret: usize,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR as c_ulonglong;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: 0,
        origSize: 0,
    };

    /* Frame Header */
    if srcSize < ZSTDv05_frameHeaderSize_min {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
        return;
    }
    if MEM_readLE32(src) != ZSTDv05_MAGICNUMBER {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_prefix_unknown));
        return;
    }
    ip = ip.wrapping_add(ZSTDv05_frameHeaderSize_min);
    remainingSize = remainingSize.wrapping_sub(ZSTDv05_frameHeaderSize_min);

    /* Loop on each block */
    loop {
        let cBlockSize: usize =
            ZSTDv05_getcBlockSize(ip as *const c_void, remainingSize, &mut blockProperties);
        if ZSTDv05_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTDv05_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv05_blockHeaderSize);
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }

        if cBlockSize == 0 {
            break; /* bt_end */
        }

        ip = ip.wrapping_add(cBlockSize);
        remainingSize = remainingSize.wrapping_sub(cBlockSize);
        nbBlocks = nbBlocks.wrapping_add(1);
    }

    *cSize = (ip as usize).wrapping_sub(src as *const BYTE as usize);
    *dBound = nbBlocks.wrapping_mul(BLOCKSIZE) as c_ulonglong;
}

/* ******************************
*  Streaming Decompression API
********************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_nextSrcSizeToDecompress(dctx: *mut ZSTDv05_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressContinue(
    dctx: *mut ZSTDv05_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* Sanity check */
    if srcSize != (*dctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ZSTDv05_checkContinuity(dctx, dst as *const c_void);

    /* Decompress : frame header; part 1 */
    let stage: ZSTDv05_dStage = (*dctx).stage;
    if stage == ZSTDv05ds_getFrameHeaderSize || stage == ZSTDv05ds_decodeFrameHeader {
        if stage == ZSTDv05ds_getFrameHeaderSize {
            /* get frame header size */
            if srcSize != ZSTDv05_frameHeaderSize_min {
                return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
            }
            (*dctx).headerSize =
                ZSTDv05_decodeFrameHeader_Part1(dctx, src, ZSTDv05_frameHeaderSize_min);
            if ZSTDv05_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv05_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv05_frameHeaderSize_min {
                return ERROR(ZSTD_error_GENERIC); /* should never happen */
            }
            (*dctx).expected = 0; /* not necessary to copy more */
            /* fallthrough */
        }
        /* case ZSTDv05ds_decodeFrameHeader: get frame header */
        {
            let result: usize = ZSTDv05_decodeFrameHeader_Part2(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if ZSTDv05_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv05_blockHeaderSize;
            (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
            return 0;
        }
    } else if stage == ZSTDv05ds_decodeBlockHeader {
        /* Decode block header */
        let mut bp: blockProperties_t = blockProperties_t {
            blockType: 0,
            origSize: 0,
        };
        let blockSize: usize = ZSTDv05_getcBlockSize(src, ZSTDv05_blockHeaderSize, &mut bp);
        if ZSTDv05_isError(blockSize) != 0 {
            return blockSize;
        }
        if bp.blockType == bt_end {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDv05ds_getFrameHeaderSize;
        } else {
            (*dctx).expected = blockSize;
            (*dctx).bType = bp.blockType;
            (*dctx).stage = ZSTDv05ds_decompressBlock;
        }
        return 0;
    } else if stage == ZSTDv05ds_decompressBlock {
        /* Decompress : block content */
        let rSize: usize;
        if (*dctx).bType == bt_compressed {
            rSize = ZSTDv05_decompressBlock_internal(dctx, dst, maxDstSize, src, srcSize);
        } else if (*dctx).bType == bt_raw {
            rSize = ZSTDv05_copyRawBlock(dst, maxDstSize, src, srcSize);
        } else if (*dctx).bType == bt_rle {
            return ERROR(ZSTD_error_GENERIC); /* not yet handled */
        } else if (*dctx).bType == bt_end {
            /* should never happen (filtered at phase 1) */
            rSize = 0;
        } else {
            return ERROR(ZSTD_error_GENERIC); /* impossible */
        }
        (*dctx).stage = ZSTDv05ds_decodeBlockHeader;
        (*dctx).expected = ZSTDv05_blockHeaderSize;
        if ZSTDv05_isError(rSize) != 0 {
            return rSize;
        }
        (*dctx).previousDstEnd = (dst as *mut c_char).wrapping_add(rSize) as *const c_void;
        return rSize;
    }

    /* default: */
    ERROR(ZSTD_error_GENERIC) /* impossible */
}

pub unsafe fn ZSTDv05_refDictContent(
    dctx: *mut ZSTDv05_DCtx,
    dict: *const c_void,
    dictSize: usize,
) {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const c_char).wrapping_offset(
        0isize.wrapping_sub(
            ((*dctx).previousDstEnd as *const c_char as isize)
                .wrapping_sub((*dctx).base as *const c_char as isize),
        ),
    ) as *const c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd =
        (dict as *const c_char).wrapping_add(dictSize) as *const c_void;
}

pub unsafe fn ZSTDv05_loadEntropy(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let hSize: usize;
    let offcodeHeaderSize: usize;
    let matchlengthHeaderSize: usize;
    let mut errorCode: usize;
    let litlengthHeaderSize: usize;
    let mut offcodeNCount: [i16; MaxOff as usize + 1] = [0; MaxOff as usize + 1];
    let mut offcodeMaxValue: c_uint = MaxOff;
    let mut offcodeLog: c_uint = 0;
    let mut matchlengthNCount: [i16; MaxML as usize + 1] = [0; MaxML as usize + 1];
    let mut matchlengthMaxValue: c_uint = MaxML;
    let mut matchlengthLog: c_uint = 0;
    let mut litlengthNCount: [i16; MaxLL as usize + 1] = [0; MaxLL as usize + 1];
    let mut litlengthMaxValue: c_uint = MaxLL;
    let mut litlengthLog: c_uint = 0;

    hSize = HUFv05_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if HUFv05_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(hSize) as *const c_void;
    dictSize = dictSize.wrapping_sub(hSize);

    offcodeHeaderSize = FSEv05_readNCount(
        offcodeNCount.as_mut_ptr(),
        &mut offcodeMaxValue,
        &mut offcodeLog,
        dict,
        dictSize,
    );
    if FSEv05_isError(offcodeHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if offcodeLog > OffFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).OffTable.as_mut_ptr(),
        offcodeNCount.as_ptr(),
        offcodeMaxValue,
        offcodeLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(offcodeHeaderSize) as *const c_void;
    dictSize = dictSize.wrapping_sub(offcodeHeaderSize);

    matchlengthHeaderSize = FSEv05_readNCount(
        matchlengthNCount.as_mut_ptr(),
        &mut matchlengthMaxValue,
        &mut matchlengthLog,
        dict,
        dictSize,
    );
    if FSEv05_isError(matchlengthHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if matchlengthLog > MLFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).MLTable.as_mut_ptr(),
        matchlengthNCount.as_ptr(),
        matchlengthMaxValue,
        matchlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(matchlengthHeaderSize) as *const c_void;
    dictSize = dictSize.wrapping_sub(matchlengthHeaderSize);

    litlengthHeaderSize = FSEv05_readNCount(
        litlengthNCount.as_mut_ptr(),
        &mut litlengthMaxValue,
        &mut litlengthLog,
        dict,
        dictSize,
    );
    if litlengthLog > LLFSEv05Log {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    if FSEv05_isError(litlengthHeaderSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    errorCode = FSEv05_buildDTable(
        (*dctx).LLTable.as_mut_ptr(),
        litlengthNCount.as_ptr(),
        litlengthMaxValue,
        litlengthLog,
    );
    if FSEv05_isError(errorCode) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    (*dctx).flagStaticTables = 1;
    hSize
        .wrapping_add(offcodeHeaderSize)
        .wrapping_add(matchlengthHeaderSize)
        .wrapping_add(litlengthHeaderSize)
}

pub unsafe fn ZSTDv05_decompress_insertDictionary(
    dctx: *mut ZSTDv05_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let eSize: usize;
    let magic: U32 = MEM_readLE32(dict);
    if magic != ZSTDv05_DICT_MAGIC {
        /* pure content mode */
        ZSTDv05_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as *const c_char).wrapping_add(4) as *const c_void;
    dictSize = dictSize.wrapping_sub(4);
    eSize = ZSTDv05_loadEntropy(dctx, dict, dictSize);
    if ZSTDv05_isError(eSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    /* reference dictionary content */
    dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
    dictSize = dictSize.wrapping_sub(eSize);
    ZSTDv05_refDictContent(dctx, dict, dictSize);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv05_decompressBegin_usingDict(
    dctx: *mut ZSTDv05_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut errorCode: usize;
    errorCode = ZSTDv05_decompressBegin(dctx);
    if ZSTDv05_isError(errorCode) != 0 {
        return errorCode;
    }

    if !dict.is_null() && dictSize != 0 {
        errorCode = ZSTDv05_decompress_insertDictionary(dctx, dict, dictSize);
        if ZSTDv05_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

/*
    Buffered version of Zstd compression library
    Copyright (C) 2015-2016, Yann Collet.
*/

/* *************************************
*  Constants
***************************************/
pub static ZBUFFv05_blockHeaderSize: usize = 3;

/* *** Compression *** */

pub unsafe fn ZBUFFv05_limitCopy(
    dst: *mut c_void,
    maxDstSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length: usize = if maxDstSize < srcSize {
        maxDstSize
    } else {
        srcSize
    };
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

/* * ***********************************************
*  Streaming decompression
* **************************************************/

pub type ZBUFFv05_dStage = c_int;
pub const ZBUFFv05ds_init: ZBUFFv05_dStage = 0;
pub const ZBUFFv05ds_readHeader: ZBUFFv05_dStage = 1;
pub const ZBUFFv05ds_loadHeader: ZBUFFv05_dStage = 2;
pub const ZBUFFv05ds_decodeHeader: ZBUFFv05_dStage = 3;
pub const ZBUFFv05ds_read: ZBUFFv05_dStage = 4;
pub const ZBUFFv05ds_load: ZBUFFv05_dStage = 5;
pub const ZBUFFv05ds_flush: ZBUFFv05_dStage = 6;

/* *** Resource management *** */

#[repr(C)]
pub struct ZBUFFv05_DCtx {
    pub zc: *mut ZSTDv05_DCtx,
    pub params: ZSTDv05_parameters,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub hPos: usize,
    pub stage: ZBUFFv05_dStage,
    pub headerBuffer: [core::ffi::c_uchar; ZSTDv05_frameHeaderSize_max],
} /* typedef'd to ZBUFFv05_DCtx within "zstd_buffered.h" */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_createDCtx() -> *mut ZBUFFv05_DCtx {
    let zbc: *mut ZBUFFv05_DCtx =
        malloc(core::mem::size_of::<ZBUFFv05_DCtx>()) as *mut ZBUFFv05_DCtx;
    if zbc.is_null() {
        return core::ptr::null_mut();
    }
    memset(
        zbc as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv05_DCtx>(),
    );
    (*zbc).zc = ZSTDv05_createDCtx();
    (*zbc).stage = ZBUFFv05ds_init;
    zbc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_freeDCtx(zbc: *mut ZBUFFv05_DCtx) -> usize {
    if zbc.is_null() {
        return 0; /* support free on null */
    }
    ZSTDv05_freeDCtx((*zbc).zc);
    free((*zbc).inBuff as *mut c_void);
    free((*zbc).outBuff as *mut c_void);
    free(zbc as *mut c_void);
    0
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressInitDictionary(
    zbc: *mut ZBUFFv05_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbc).stage = ZBUFFv05ds_readHeader;
    (*zbc).outEnd = 0;
    (*zbc).outStart = 0;
    (*zbc).inPos = 0;
    (*zbc).hPos = 0;
    ZSTDv05_decompressBegin_usingDict((*zbc).zc, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressInit(zbc: *mut ZBUFFv05_DCtx) -> usize {
    ZBUFFv05_decompressInitDictionary(zbc, core::ptr::null(), 0)
}

/* *** Decompression *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv05_decompressContinue(
    zbc: *mut ZBUFFv05_DCtx,
    dst: *mut c_void,
    maxDstSizePtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart: *const c_char = src as *const c_char;
    let mut ip: *const c_char = istart;
    let iend: *const c_char = istart.wrapping_add(*srcSizePtr);
    let ostart: *mut c_char = dst as *mut c_char;
    let mut op: *mut c_char = ostart;
    let oend: *mut c_char = ostart.wrapping_add(*maxDstSizePtr);
    let mut notDone: U32 = 1;

    while notDone != 0 {
        /* `loop { ... }` emulates the C `switch` : `break` == leave the switch */
        loop {
            let mut cur: ZBUFFv05_dStage = (*zbc).stage;

            if cur == ZBUFFv05ds_init {
                return ERROR(ZSTD_error_init_missing);
            }
            if cur != ZBUFFv05ds_readHeader
                && cur != ZBUFFv05ds_loadHeader
                && cur != ZBUFFv05ds_decodeHeader
                && cur != ZBUFFv05ds_read
                && cur != ZBUFFv05ds_load
                && cur != ZBUFFv05ds_flush
            {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }

            if cur == ZBUFFv05ds_readHeader {
                /* read header from src */
                {
                    let headerSize: usize =
                        ZSTDv05_getFrameParams(&mut (*zbc).params, src, *srcSizePtr);
                    if ZSTDv05_isError(headerSize) != 0 {
                        return headerSize;
                    }
                    if headerSize != 0 {
                        /* not enough input to decode header : tell how many bytes would be necessary */
                        memcpy(
                            (*zbc).headerBuffer.as_mut_ptr().wrapping_add((*zbc).hPos)
                                as *mut c_void,
                            src,
                            *srcSizePtr,
                        );
                        (*zbc).hPos = (*zbc).hPos.wrapping_add(*srcSizePtr);
                        *maxDstSizePtr = 0;
                        (*zbc).stage = ZBUFFv05ds_loadHeader;
                        return headerSize.wrapping_sub((*zbc).hPos);
                    }
                    (*zbc).stage = ZBUFFv05ds_decodeHeader;
                    break;
                }
            }

            if cur == ZBUFFv05ds_loadHeader {
                /* complete header from src */
                {
                    let mut headerSize: usize = ZBUFFv05_limitCopy(
                        (*zbc).headerBuffer.as_mut_ptr().wrapping_add((*zbc).hPos)
                            as *mut c_void,
                        ZSTDv05_frameHeaderSize_max.wrapping_sub((*zbc).hPos),
                        src,
                        *srcSizePtr,
                    );
                    (*zbc).hPos = (*zbc).hPos.wrapping_add(headerSize);
                    ip = ip.wrapping_add(headerSize);
                    headerSize = ZSTDv05_getFrameParams(
                        &mut (*zbc).params,
                        (*zbc).headerBuffer.as_ptr() as *const c_void,
                        (*zbc).hPos,
                    );
                    if ZSTDv05_isError(headerSize) != 0 {
                        return headerSize;
                    }
                    if headerSize != 0 {
                        /* not enough input to decode header : tell how many bytes would be necessary */
                        *maxDstSizePtr = 0;
                        return headerSize.wrapping_sub((*zbc).hPos);
                    }
                    /* zbc->stage = ZBUFFv05ds_decodeHeader; break; */
                    /* useless : stage follows */
                }
                cur = ZBUFFv05ds_decodeHeader;
                /* fall-through */
            }

            if cur == ZBUFFv05ds_decodeHeader {
                /* apply header to create / resize buffers */
                {
                    let neededOutSize: usize = 1usize.wrapping_shl((*zbc).params.windowLog);
                    let neededInSize: usize = BLOCKSIZE; /* a block is never > BLOCKSIZE */
                    if (*zbc).inBuffSize < neededInSize {
                        free((*zbc).inBuff as *mut c_void);
                        (*zbc).inBuffSize = neededInSize;
                        (*zbc).inBuff = malloc(neededInSize) as *mut c_char;
                        if (*zbc).inBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
                        }
                    }
                    if (*zbc).outBuffSize < neededOutSize {
                        free((*zbc).outBuff as *mut c_void);
                        (*zbc).outBuffSize = neededOutSize;
                        (*zbc).outBuff = malloc(neededOutSize) as *mut c_char;
                        if (*zbc).outBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
                        }
                    }
                }
                if (*zbc).hPos != 0 {
                    /* some data already loaded into headerBuffer : transfer into inBuff */
                    memcpy(
                        (*zbc).inBuff as *mut c_void,
                        (*zbc).headerBuffer.as_ptr() as *const c_void,
                        (*zbc).hPos,
                    );
                    (*zbc).inPos = (*zbc).hPos;
                    (*zbc).hPos = 0;
                    (*zbc).stage = ZBUFFv05ds_load;
                    break;
                }
                (*zbc).stage = ZBUFFv05ds_read;
                cur = ZBUFFv05ds_read;
                /* fall-through */
            }

            if cur == ZBUFFv05ds_read {
                {
                    let neededInSize: usize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zbc).stage = ZBUFFv05ds_init;
                        notDone = 0;
                        break;
                    }
                    if (iend as usize).wrapping_sub(ip as usize) >= neededInSize {
                        /* directly decode from src */
                        let decodedSize: usize = ZSTDv05_decompressContinue(
                            (*zbc).zc,
                            (*zbc).outBuff.wrapping_add((*zbc).outStart) as *mut c_void,
                            (*zbc).outBuffSize.wrapping_sub((*zbc).outStart),
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv05_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.wrapping_add(neededInSize);
                        if decodedSize == 0 {
                            break; /* this was just a header */
                        }
                        (*zbc).outEnd = (*zbc).outStart.wrapping_add(decodedSize);
                        (*zbc).stage = ZBUFFv05ds_flush;
                        break;
                    }
                    if ip == iend {
                        notDone = 0;
                        break;
                    } /* no more input */
                    (*zbc).stage = ZBUFFv05ds_load;
                }
                cur = ZBUFFv05ds_load;
                /* fall-through */
            }

            if cur == ZBUFFv05ds_load {
                {
                    let neededInSize: usize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
                    let toLoad: usize = neededInSize.wrapping_sub((*zbc).inPos); /* should always be <= remaining space within inBuff */
                    let loadedSize: usize;
                    if toLoad > (*zbc).inBuffSize.wrapping_sub((*zbc).inPos) {
                        return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                    }
                    loadedSize = ZBUFFv05_limitCopy(
                        (*zbc).inBuff.wrapping_add((*zbc).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        (iend as usize).wrapping_sub(ip as usize),
                    );
                    ip = ip.wrapping_add(loadedSize);
                    (*zbc).inPos = (*zbc).inPos.wrapping_add(loadedSize);
                    if loadedSize < toLoad {
                        notDone = 0;
                        break;
                    } /* not enough input, wait for more */
                    {
                        let decodedSize: usize = ZSTDv05_decompressContinue(
                            (*zbc).zc,
                            (*zbc).outBuff.wrapping_add((*zbc).outStart) as *mut c_void,
                            (*zbc).outBuffSize.wrapping_sub((*zbc).outStart),
                            (*zbc).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ZSTDv05_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbc).inPos = 0; /* input is consumed */
                        if decodedSize == 0 {
                            (*zbc).stage = ZBUFFv05ds_read;
                            break;
                        } /* this was just a header */
                        (*zbc).outEnd = (*zbc).outStart.wrapping_add(decodedSize);
                        (*zbc).stage = ZBUFFv05ds_flush;
                        /* break; */ /* ZBUFFv05ds_flush follows */
                    }
                }
                cur = ZBUFFv05ds_flush;
                /* fall-through */
            }

            /* case ZBUFFv05ds_flush: */
            {
                let toFlushSize: usize = (*zbc).outEnd.wrapping_sub((*zbc).outStart);
                let flushedSize: usize = ZBUFFv05_limitCopy(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    (*zbc).outBuff.wrapping_add((*zbc).outStart) as *const c_void,
                    toFlushSize,
                );
                op = op.wrapping_add(flushedSize);
                (*zbc).outStart = (*zbc).outStart.wrapping_add(flushedSize);
                if flushedSize == toFlushSize {
                    (*zbc).stage = ZBUFFv05ds_read;
                    if (*zbc).outStart.wrapping_add(BLOCKSIZE) > (*zbc).outBuffSize {
                        (*zbc).outStart = 0;
                        (*zbc).outEnd = 0;
                    }
                    break;
                }
                /* cannot flush everything */
                notDone = 0;
                break;
            }
        }
    }

    *srcSizePtr = (ip as usize).wrapping_sub(istart as usize);
    *maxDstSizePtr = (op as usize).wrapping_sub(ostart as usize);

    {
        let mut nextSrcSizeHint: usize = ZSTDv05_nextSrcSizeToDecompress((*zbc).zc);
        if nextSrcSizeHint > ZBUFFv05_blockHeaderSize {
            nextSrcSizeHint = nextSrcSizeHint.wrapping_add(ZBUFFv05_blockHeaderSize);
            /* get next block header too */
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbc).inPos); /* already loaded*/
        return nextSrcSizeHint;
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_recommendedDInSize() -> usize {
    BLOCKSIZE + ZBUFFv05_blockHeaderSize /* block header size*/
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv05_recommendedDOutSize() -> usize {
    BLOCKSIZE
}
