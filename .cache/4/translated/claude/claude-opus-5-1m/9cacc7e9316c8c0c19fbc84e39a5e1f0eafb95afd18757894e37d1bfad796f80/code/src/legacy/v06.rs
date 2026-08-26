//! Transliteration of the second half of `legacy/zstd_v06.c` : C lines 2619..4110.
//!
//! This covers the "Common functions of Zstd compression library"
//! (`ZSTDv06_isError` / `ZBUFFv06_isError` error management), the whole
//! `zstd_decompress` layer (`ZSTDv06_DCtx`, frame/block/literals/sequences
//! decoding, the bufferless streaming API) and the buffered `ZBUFFv06_*`
//! streaming decompression layer.
//!
//! Lines 1..2618 (mem / bitstream / FSEv06 / HUFv06) live in the sibling
//! module `v06_ent`, which is glob-imported below.
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
    unused_comparisons
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::error_private::*;
use crate::legacy::v06_ent::*;
use crate::mem::{calloc, free, malloc, memcpy, memmove, memset, qsort};

/*-*************************************
*  Constants (zstd_v06.h)
***************************************/
pub const ZSTDv06_MAGICNUMBER: U32 = 0xFD2FB526; /* v0.6 */

/*-************************
*  Advanced Streaming API
***************************/
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTDv06_frameParams {
    pub frameContentSize: c_ulonglong,
    pub windowLog: c_uint,
}

/*-****************************************
*  ZSTD Error Management
******************************************/
/* ! ZSTDv06_isError() :
*   tells if a return value is an error code */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv06_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* ! ZSTDv06_getErrorName() :
*   provides error code string from function result (useful for debugging) */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv06_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/* **************************************************************
*  ZBUFF Error Management
****************************************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv06_isError(errorCode: usize) -> c_uint {
    ERR_isError(errorCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv06_getErrorName(errorCode: usize) -> *const c_char {
    ERR_getErrorName(errorCode)
}

/*_*******************************************************
*  Memory operations
**********************************************************/
pub unsafe fn ZSTDv06_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

/*-*************************************************************
*   Context management
***************************************************************/
pub type ZSTDv06_dStage = c_int;
pub const ZSTDds_getFrameHeaderSize: ZSTDv06_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTDv06_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTDv06_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTDv06_dStage = 3;

#[repr(C)]
pub struct ZSTDv06_DCtx {
    pub LLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(LLFSELog)],
    pub OffTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(OffFSELog)],
    pub MLTable: [FSEv06_DTable; FSEv06_DTABLE_SIZE_U32(MLFSELog)],
    pub hufTableX4: [c_uint; HUFv06_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    pub previousDstEnd: *const c_void,
    pub base: *const c_void,
    pub vBase: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub headerSize: usize,
    pub fParams: ZSTDv06_frameParams,
    pub bType: blockType_t, /* used in ZSTDv06_decompressContinue(), to transfer blockType between header decoding and block decoding stages */
    pub stage: ZSTDv06_dStage,
    pub flagRepeatTable: U32,
    pub litPtr: *const BYTE,
    pub litSize: usize,
    pub litBuffer: [BYTE; ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
} /* typedef'd to ZSTDv06_DCtx within "zstd_static.h" */

#[unsafe(no_mangle)]
pub extern "C" fn ZSTDv06_sizeofDCtx() -> usize {
    core::mem::size_of::<ZSTDv06_DCtx>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBegin(dctx: *mut ZSTDv06_DCtx) -> usize {
    (*dctx).expected = ZSTDv06_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = core::ptr::null();
    (*dctx).base = core::ptr::null();
    (*dctx).vBase = core::ptr::null();
    (*dctx).dictEnd = core::ptr::null();
    (*dctx).hufTableX4[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG;
    (*dctx).flagRepeatTable = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_createDCtx() -> *mut ZSTDv06_DCtx {
    let dctx: *mut ZSTDv06_DCtx =
        malloc(core::mem::size_of::<ZSTDv06_DCtx>()) as *mut ZSTDv06_DCtx;
    if dctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDv06_decompressBegin(dctx);
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_freeDCtx(dctx: *mut ZSTDv06_DCtx) -> usize {
    free(dctx as *mut c_void);
    0 /* reserved as a potential error code in the future */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_copyDCtx(
    dstDCtx: *mut ZSTDv06_DCtx,
    srcDCtx: *const ZSTDv06_DCtx,
) {
    memcpy(
        dstDCtx as *mut c_void,
        srcDCtx as *const c_void,
        core::mem::size_of::<ZSTDv06_DCtx>()
            - (ZSTDv06_BLOCKSIZE_MAX + WILDCOPY_OVERLENGTH + ZSTDv06_frameHeaderSize_max),
    ); /* no need to copy workspace */
}

/*-*************************************************************
*   Decompression section
***************************************************************/

/** ZSTDv06_frameHeaderSize() :
*   srcSize must be >= ZSTDv06_frameHeaderSize_min.
*   @return : size of the Frame Header */
pub unsafe fn ZSTDv06_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    {
        let fcsId: U32 = (*(src as *const BYTE).wrapping_add(4) as U32) >> 6;
        return ZSTDv06_frameHeaderSize_min.wrapping_add(ZSTDv06_fcs_fieldSize[fcsId as usize]);
    }
}

/** ZSTDv06_getFrameParams() :
*   decode Frame Header, or provide expected `srcSize`.
*   @return : 0, `fparamsPtr` is correctly filled,
*            >0, `srcSize` is too small, result is expected `srcSize`,
*             or an error code, which can be tested using ZSTDv06_isError() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_getFrameParams(
    fparamsPtr: *mut ZSTDv06_frameParams,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ip: *const BYTE = src as *const BYTE;

    if srcSize < ZSTDv06_frameHeaderSize_min {
        return ZSTDv06_frameHeaderSize_min;
    }
    if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
        return ERROR(ZSTD_error_prefix_unknown);
    }

    /* ensure there is enough `srcSize` to fully read/decode frame header */
    {
        let fhsize: usize = ZSTDv06_frameHeaderSize(src, srcSize);
        if srcSize < fhsize {
            return fhsize;
        }
    }

    memset(
        fparamsPtr as *mut c_void,
        0,
        core::mem::size_of::<ZSTDv06_frameParams>(),
    );
    {
        let frameDesc: BYTE = *ip.wrapping_add(4);
        (*fparamsPtr).windowLog =
            ((frameDesc & 0xF) as U32).wrapping_add(ZSTDv06_WINDOWLOG_ABSOLUTEMIN);
        if (frameDesc & 0x20) != 0 {
            return ERROR(ZSTD_error_frameParameter_unsupported); /* reserved 1 bit */
        }
        match frameDesc >> 6
        /* fcsId */
        {
            1 => {
                (*fparamsPtr).frameContentSize = *ip.wrapping_add(5) as c_ulonglong;
            }
            2 => {
                (*fparamsPtr).frameContentSize =
                    ((MEM_readLE16(ip.wrapping_add(5) as *const c_void) as c_int) + 256)
                        as c_ulonglong;
            }
            3 => {
                (*fparamsPtr).frameContentSize =
                    MEM_readLE64(ip.wrapping_add(5) as *const c_void) as c_ulonglong;
            }
            /* default (impossible) and case 0 */
            _ => {
                (*fparamsPtr).frameContentSize = 0;
            }
        }
    }
    0
}

/** ZSTDv06_decodeFrameHeader() :
*   `srcSize` must be the size provided by ZSTDv06_frameHeaderSize().
*   @return : 0 if success, or an error code, which can be tested using ZSTDv06_isError() */
pub unsafe fn ZSTDv06_decodeFrameHeader(
    zc: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let result: usize = ZSTDv06_getFrameParams(&mut (*zc).fParams, src, srcSize);
    if (MEM_32bits() != 0) && ((*zc).fParams.windowLog > 25) {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    result
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}

/* ! ZSTDv06_getcBlockSize() :
*   Provides the size of compressed block from block header `src` */
pub unsafe fn ZSTDv06_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    let in_: *const BYTE = src as *const BYTE;
    let cSize: U32;

    if srcSize < ZSTDv06_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    (*bpPtr).blockType = ((*in_) >> 6) as blockType_t;
    cSize = (*in_.wrapping_add(2) as U32)
        .wrapping_add((*in_.wrapping_add(1) as U32) << 8)
        .wrapping_add(((*in_.wrapping_add(0) as U32) & 7) << 16);
    (*bpPtr).origSize = if (*bpPtr).blockType == bt_rle { cSize } else { 0 };

    if (*bpPtr).blockType == bt_end {
        return 0;
    }
    if (*bpPtr).blockType == bt_rle {
        return 1;
    }
    cSize as usize
}

pub unsafe fn ZSTDv06_copyRawBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if dst.is_null() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    memcpy(dst, src, srcSize);
    srcSize
}

/* ! ZSTDv06_decodeLiteralsBlock() :
    @return : nb of bytes read from src (< srcSize ) */
pub unsafe fn ZSTDv06_decodeLiteralsBlock(
    dctx: *mut ZSTDv06_DCtx,
    src: *const c_void,
    srcSize: usize, /* note : srcSize < BLOCKSIZE */
) -> usize {
    let istart: *const BYTE = src as *const BYTE;

    /* any compressed block with literals segment must be at least this size */
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    match (*istart.wrapping_add(0) as U32) >> 6 {
        IS_HUF => {
            let litSize: usize;
            let litCSize: usize;
            let mut singleStream: usize = 0;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */
            }
            match lhSize {
                2 => {
                    /* 2 - 2 - 14 - 14 */
                    lhSize = 4;
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 10)
                        + ((*istart.wrapping_add(1) as usize) << 2)
                        + ((*istart.wrapping_add(2) as usize) >> 6);
                    litCSize = (((*istart.wrapping_add(2) as usize) & 63) << 8)
                        + (*istart.wrapping_add(3) as usize);
                }
                3 => {
                    /* 2 - 2 - 18 - 18 */
                    lhSize = 5;
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 14)
                        + ((*istart.wrapping_add(1) as usize) << 6)
                        + ((*istart.wrapping_add(2) as usize) >> 2);
                    litCSize = (((*istart.wrapping_add(2) as usize) & 3) << 16)
                        + ((*istart.wrapping_add(3) as usize) << 8)
                        + (*istart.wrapping_add(4) as usize);
                }
                /* case 0: case 1: default: note : default is impossible, since lhSize into [0..3] */
                _ => {
                    /* 2 - 2 - 10 - 10 */
                    lhSize = 3;
                    singleStream = (*istart.wrapping_add(0) & 16) as usize;
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 6)
                        + ((*istart.wrapping_add(1) as usize) >> 2);
                    litCSize = (((*istart.wrapping_add(1) as usize) & 3) << 8)
                        + (*istart.wrapping_add(2) as usize);
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if litCSize.wrapping_add(lhSize as usize) > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            {
                let hufResult: usize = if singleStream != 0 {
                    HUFv06_decompress1X2(
                        (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                        litSize,
                        istart.wrapping_add(lhSize as usize) as *const c_void,
                        litCSize,
                    )
                } else {
                    HUFv06_decompress(
                        (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                        litSize,
                        istart.wrapping_add(lhSize as usize) as *const c_void,
                        litCSize,
                    )
                };
                if ERR_isError(hufResult) != 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }

            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize.wrapping_add(lhSize as usize)
        }
        IS_PCH => {
            let litSize: usize;
            let litCSize: usize;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            if lhSize != 1 {
                /* only case supported for now : small litSize, single stream */
                return ERROR(ZSTD_error_corruption_detected);
            }
            if (*dctx).flagRepeatTable == 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }

            /* 2 - 2 - 10 - 10 */
            lhSize = 3;
            litSize = (((*istart.wrapping_add(0) as usize) & 15) << 6)
                + ((*istart.wrapping_add(1) as usize) >> 2);
            litCSize = (((*istart.wrapping_add(1) as usize) & 3) << 8)
                + (*istart.wrapping_add(2) as usize);
            if litCSize.wrapping_add(lhSize as usize) > srcSize {
                return ERROR(ZSTD_error_corruption_detected);
            }

            {
                let errorCode: usize = HUFv06_decompress1X4_usingDTable(
                    (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                    litSize,
                    istart.wrapping_add(lhSize as usize) as *const c_void,
                    litCSize,
                    (*dctx).hufTableX4.as_ptr(),
                );
                if ERR_isError(errorCode) != 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
            }
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            memset(
                (*dctx).litBuffer.as_mut_ptr().wrapping_add((*dctx).litSize) as *mut c_void,
                0,
                WILDCOPY_OVERLENGTH,
            );
            litCSize.wrapping_add(lhSize as usize)
        }
        IS_RAW => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            match lhSize {
                2 => {
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 8)
                        + (*istart.wrapping_add(1) as usize);
                }
                3 => {
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 16)
                        + ((*istart.wrapping_add(1) as usize) << 8)
                        + (*istart.wrapping_add(2) as usize);
                }
                /* case 0: case 1: default: note : default is impossible, since lhSize into [0..3] */
                _ => {
                    lhSize = 1;
                    litSize = (*istart.wrapping_add(0) as usize) & 31;
                }
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
            (lhSize as usize).wrapping_add(litSize)
        }
        IS_RLE => {
            let litSize: usize;
            let mut lhSize: U32 = ((*istart.wrapping_add(0) as U32) >> 4) & 3;
            match lhSize {
                2 => {
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 8)
                        + (*istart.wrapping_add(1) as usize);
                }
                3 => {
                    litSize = (((*istart.wrapping_add(0) as usize) & 15) << 16)
                        + ((*istart.wrapping_add(1) as usize) << 8)
                        + (*istart.wrapping_add(2) as usize);
                    if srcSize < 4 {
                        return ERROR(ZSTD_error_corruption_detected); /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */
                    }
                }
                /* case 0: case 1: default: note : default is impossible, since lhSize into [0..3] */
                _ => {
                    lhSize = 1;
                    litSize = (*istart.wrapping_add(0) as usize) & 31;
                }
            }
            if litSize > ZSTDv06_BLOCKSIZE_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            memset(
                (*dctx).litBuffer.as_mut_ptr() as *mut c_void,
                *istart.wrapping_add(lhSize as usize) as c_int,
                litSize.wrapping_add(WILDCOPY_OVERLENGTH),
            );
            (*dctx).litPtr = (*dctx).litBuffer.as_ptr();
            (*dctx).litSize = litSize;
            (lhSize as usize).wrapping_add(1)
        }
        _ => ERROR(ZSTD_error_corruption_detected), /* impossible */
    }
}

/* ! ZSTDv06_buildSeqTable() :
    @return : nb bytes read from src,
              or an error code if it fails, testable with ZSTDv06_isError()
*/
pub unsafe fn ZSTDv06_buildSeqTable(
    DTable: *mut FSEv06_DTable,
    type_: U32,
    mut max: U32,
    maxLog: U32,
    src: *const c_void,
    srcSize: usize,
    defaultNorm: *const S16,
    defaultLog: U32,
    flagRepeatTable: U32,
) -> usize {
    match type_ {
        FSEv06_ENCODING_RLE => {
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if (*(src as *const BYTE) as U32) > max {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv06_buildDTable_rle(DTable, *(src as *const BYTE)); /* if *src > max, data is corrupted */
            1
        }
        FSEv06_ENCODING_RAW => {
            FSEv06_buildDTable(DTable, defaultNorm, max, defaultLog);
            0
        }
        FSEv06_ENCODING_STATIC => {
            if flagRepeatTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            0
        }
        /* default (impossible) and FSEv06_ENCODING_DYNAMIC */
        _ => {
            let mut tableLog: U32 = 0;
            let mut norm: [S16; (MaxSeq + 1) as usize] = [0; (MaxSeq + 1) as usize];
            let headerSize: usize = FSEv06_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut tableLog,
                src,
                srcSize,
            );
            if ERR_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if tableLog > maxLog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            FSEv06_buildDTable(DTable, norm.as_ptr(), max, tableLog);
            headerSize
        }
    }
}

pub unsafe fn ZSTDv06_decodeSeqHeaders(
    nbSeqPtr: *mut c_int,
    DTableLL: *mut FSEv06_DTable,
    DTableML: *mut FSEv06_DTable,
    DTableOffb: *mut FSEv06_DTable,
    flagRepeatTable: U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let mut ip: *const BYTE = istart;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    {
        let mut nbSeq: c_int = *ip as c_int;
        ip = ip.wrapping_add(1);
        if nbSeq == 0 {
            *nbSeqPtr = 0;
            return 1;
        }
        if nbSeq > 0x7F {
            if nbSeq == 0xFF {
                if ip.wrapping_add(2) > iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = (MEM_readLE16(ip as *const c_void) as c_int)
                    .wrapping_add(LONGNBSEQ as c_int);
                ip = ip.wrapping_add(2);
            } else {
                if ip >= iend {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
                nbSeq = ((nbSeq - 0x80) << 8) + (*ip as c_int);
                ip = ip.wrapping_add(1);
            }
        }
        *nbSeqPtr = nbSeq;
    }

    /* FSE table descriptors */
    if ip.wrapping_add(4) > iend {
        return ERROR(ZSTD_error_srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */
    }
    {
        let LLtype: U32 = (*ip as U32) >> 6;
        let Offtype: U32 = ((*ip as U32) >> 4) & 3;
        let MLtype: U32 = ((*ip as U32) >> 2) & 3;
        ip = ip.wrapping_add(1);

        /* Build DTables */
        {
            let bhSize: usize = ZSTDv06_buildSeqTable(
                DTableLL,
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
        {
            let bhSize: usize = ZSTDv06_buildSeqTable(
                DTableOffb,
                Offtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
        {
            let bhSize: usize = ZSTDv06_buildSeqTable(
                DTableML,
                MLtype,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                flagRepeatTable,
            );
            if ERR_isError(bhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.wrapping_add(bhSize);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seq_t {
    pub litLength: usize,
    pub matchLength: usize,
    pub offset: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seqState_t {
    pub DStream: BITv06_DStream_t,
    pub stateLL: FSEv06_DState_t,
    pub stateOffb: FSEv06_DState_t,
    pub stateML: FSEv06_DState_t,
    pub prevOffset: [usize; ZSTDv06_REP_INIT as usize],
}

/* local `static const U32 LL_base[MaxLL+1]` of ZSTDv06_decodeSequence() */
pub static LL_base: [U32; (MaxLL + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

/* local `static const U32 ML_base[MaxML+1]` of ZSTDv06_decodeSequence() */
pub static ML_base: [U32; (MaxML + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 34, 36, 38, 40, 44, 48, 56, 64, 80, 96, 0x80, 0x100, 0x200, 0x400,
    0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

/* local `static const U32 OF_base[MaxOff+1]` of ZSTDv06_decodeSequence() */
pub static OF_base: [U32; (MaxOff + 1) as usize] = [
    0,
    1,
    3,
    7,
    0xF,
    0x1F,
    0x3F,
    0x7F,
    0xFF,
    0x1FF,
    0x3FF,
    0x7FF,
    0xFFF,
    0x1FFF,
    0x3FFF,
    0x7FFF,
    0xFFFF,
    0x1FFFF,
    0x3FFFF,
    0x7FFFF,
    0xFFFFF,
    0x1FFFFF,
    0x3FFFFF,
    0x7FFFFF,
    0xFFFFFF,
    0x1FFFFFF,
    0x3FFFFFF,
    /*fake*/ 1,
    1,
];

pub unsafe fn ZSTDv06_decodeSequence(seq: *mut seq_t, seqState: *mut seqState_t) {
    /* Literal length */
    let llCode: U32 = FSEv06_peekSymbol(&(*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv06_peekSymbol(&(*seqState).stateML) as U32;
    let ofCode: U32 = FSEv06_peekSymbol(&(*seqState).stateOffb) as U32; /* <= maxOff, by table construction */

    let llBits: U32 = LL_bits[llCode as usize];
    let mlBits: U32 = ML_bits[mlCode as usize];
    let ofBits: U32 = ofCode;
    let totalBits: U32 = llBits.wrapping_add(mlBits).wrapping_add(ofBits);

    /* sequence */
    {
        let mut offset: usize;
        if ofCode == 0 {
            offset = 0;
        } else {
            offset = (OF_base[ofCode as usize] as usize)
                .wrapping_add(BITv06_readBits(&mut (*seqState).DStream, ofBits)); /* <=  26 bits */
            if MEM_32bits() != 0 {
                BITv06_reloadDStream(&mut (*seqState).DStream);
            }
        }

        if offset < ZSTDv06_REP_NUM as usize {
            if llCode == 0 && offset <= 1 {
                offset = 1usize.wrapping_sub(offset);
            }

            if offset != 0 {
                let temp: usize = (*seqState).prevOffset[offset];
                if offset != 1 {
                    (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
                }
                (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
                offset = temp;
                (*seqState).prevOffset[0] = offset;
            } else {
                offset = (*seqState).prevOffset[0];
            }
        } else {
            offset = offset.wrapping_sub(ZSTDv06_REP_MOVE as usize);
            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
            (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
            (*seqState).prevOffset[0] = offset;
        }
        (*seq).offset = offset;
    }

    (*seq).matchLength = (ML_base[mlCode as usize].wrapping_add(MINMATCH) as usize).wrapping_add(
        if mlCode > 31 {
            BITv06_readBits(&mut (*seqState).DStream, mlBits)
        } else {
            0
        },
    ); /* <=  16 bits */
    if MEM_32bits() != 0 && (mlBits.wrapping_add(llBits) > 24) {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    (*seq).litLength = (LL_base[llCode as usize] as usize).wrapping_add(if llCode > 15 {
        BITv06_readBits(&mut (*seqState).DStream, llBits)
    } else {
        0
    }); /* <=  16 bits */
    if MEM_32bits() != 0
        || (totalBits > (64u32 - 7 - (LLFSELog + MLFSELog + OffFSELog)))
    {
        BITv06_reloadDStream(&mut (*seqState).DStream);
    }

    /* ANS state update */
    FSEv06_updateState(&mut (*seqState).stateLL, &mut (*seqState).DStream); /* <=  9 bits */
    FSEv06_updateState(&mut (*seqState).stateML, &mut (*seqState).DStream); /* <=  9 bits */
    if MEM_32bits() != 0 {
        BITv06_reloadDStream(&mut (*seqState).DStream); /* <= 18 bits */
    }
    FSEv06_updateState(&mut (*seqState).stateOffb, &mut (*seqState).DStream); /* <=  8 bits */
}

/* local `static const U32 dec32table[]` of ZSTDv06_execSequence() : added */
pub static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
/* local `static const int dec64table[]` of ZSTDv06_execSequence() : subtracted */
pub static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];

pub unsafe fn ZSTDv06_execSequence(
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
    let iLitEnd: *const BYTE = (*litPtr).wrapping_add(sequence.litLength);
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
    if iLitEnd > litLimit {
        return ERROR(ZSTD_error_corruption_detected); /* overRead beyond lit buffer */
    }

    /* copy Literals */
    ZSTDv06_wildcopy(
        op as *mut c_void,
        *litPtr as *const c_void,
        sequence.litLength as isize,
    ); /* note : oLitEnd <= oend-8 : no risk of overwrite beyond oend */
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

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
        r#match = r#match.wrapping_add(dec32table[sequence.offset] as usize);
        ZSTDv06_copy4(
            op.wrapping_add(4) as *mut c_void,
            r#match as *const c_void,
        );
        r#match = r#match.wrapping_offset(0isize.wrapping_sub(sub2 as isize));
    } else {
        ZSTDv06_copy8(op as *mut c_void, r#match as *const c_void);
    }
    op = op.wrapping_add(8);
    r#match = r#match.wrapping_add(8);

    if oMatchEnd > oend.wrapping_sub((16 - MINMATCH) as usize) {
        if op < oend_8 {
            ZSTDv06_wildcopy(
                op as *mut c_void,
                r#match as *const c_void,
                (oend_8 as isize).wrapping_sub(op as isize),
            );
            r#match = r#match
                .wrapping_offset((oend_8 as isize).wrapping_sub(op as isize));
            op = oend_8;
        }
        while op < oMatchEnd {
            *op = *r#match;
            op = op.wrapping_add(1);
            r#match = r#match.wrapping_add(1);
        }
    } else {
        ZSTDv06_wildcopy(
            op as *mut c_void,
            r#match as *const c_void,
            (sequence.matchLength as isize).wrapping_sub(8),
        ); /* works even if matchLength < 8 */
    }
    sequenceLength
}

pub unsafe fn ZSTDv06_decompressSequences(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    mut seqSize: usize,
) -> usize {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(maxDstSize);
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.wrapping_add((*dctx).litSize);
    let DTableLL: *mut FSEv06_DTable = (*dctx).LLTable.as_mut_ptr();
    let DTableML: *mut FSEv06_DTable = (*dctx).MLTable.as_mut_ptr();
    let DTableOffb: *mut FSEv06_DTable = (*dctx).OffTable.as_mut_ptr();
    let base: *const BYTE = (*dctx).base as *const BYTE;
    let vBase: *const BYTE = (*dctx).vBase as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: c_int = 0;

    /* Build Decoding Tables */
    {
        let seqHSize: usize = ZSTDv06_decodeSeqHeaders(
            &mut nbSeq,
            DTableLL,
            DTableML,
            DTableOffb,
            (*dctx).flagRepeatTable,
            ip as *const c_void,
            seqSize,
        );
        if ERR_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.wrapping_add(seqHSize);
        (*dctx).flagRepeatTable = 0;
    }

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequence: seq_t = seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        };
        let mut seqState: seqState_t = seqState_t {
            DStream: BITv06_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: core::ptr::null(),
                start: core::ptr::null(),
            },
            stateLL: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateOffb: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            stateML: FSEv06_DState_t {
                state: 0,
                table: core::ptr::null(),
            },
            prevOffset: [0; ZSTDv06_REP_INIT as usize],
        };

        memset(
            &mut sequence as *mut seq_t as *mut c_void,
            0,
            core::mem::size_of::<seq_t>(),
        );
        sequence.offset = REPCODE_STARTVALUE as usize;
        {
            let mut i: U32 = 0;
            while i < ZSTDv06_REP_INIT {
                seqState.prevOffset[i as usize] = REPCODE_STARTVALUE as usize;
                i = i.wrapping_add(1);
            }
        }
        {
            let errorCode: usize = BITv06_initDStream(
                &mut seqState.DStream,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
        FSEv06_initDState(&mut seqState.stateLL, &mut seqState.DStream, DTableLL);
        FSEv06_initDState(&mut seqState.stateOffb, &mut seqState.DStream, DTableOffb);
        FSEv06_initDState(&mut seqState.stateML, &mut seqState.DStream, DTableML);

        while (BITv06_reloadDStream(&mut seqState.DStream) <= BITv06_DStream_completed)
            && nbSeq != 0
        {
            nbSeq -= 1;
            ZSTDv06_decodeSequence(&mut sequence, &mut seqState);

            {
                let oneSeqSize: usize = ZSTDv06_execSequence(
                    op,
                    oend,
                    sequence,
                    &mut litPtr,
                    litEnd,
                    base,
                    vBase,
                    dictEnd,
                );
                if ERR_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.wrapping_add(oneSeqSize);
            }
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
            memcpy(
                op as *mut c_void,
                litPtr as *const c_void,
                lastLLSize,
            );
            op = op.wrapping_add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTDv06_checkContinuity(dctx: *mut ZSTDv06_DCtx, dst: *const c_void) {
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

pub unsafe fn ZSTDv06_decompressBlock_internal(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip: *const BYTE = src as *const BYTE;

    if srcSize >= ZSTDv06_BLOCKSIZE_MAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals sub-block */
    {
        let litCSize: usize = ZSTDv06_decodeLiteralsBlock(dctx, src, srcSize);
        if ERR_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.wrapping_add(litCSize);
        srcSize = srcSize.wrapping_sub(litCSize);
    }
    ZSTDv06_decompressSequences(dctx, dst, dstCapacity, ip as *const c_void, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBlock(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv06_checkContinuity(dctx, dst as *const c_void);
    ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize)
}

/* ! ZSTDv06_decompressFrame() :
*   `dctx` must be properly initialized */
pub unsafe fn ZSTDv06_decompressFrame(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut remainingSize: usize = srcSize;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    /* check */
    if srcSize < ZSTDv06_frameHeaderSize_min.wrapping_add(ZSTDv06_blockHeaderSize) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Frame Header */
    {
        let frameHeaderSize: usize =
            ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
        if ERR_isError(frameHeaderSize) != 0 {
            return frameHeaderSize;
        }
        if srcSize < frameHeaderSize.wrapping_add(ZSTDv06_blockHeaderSize) {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize = remainingSize.wrapping_sub(frameHeaderSize);
    }

    /* Loop on each block */
    loop {
        let mut decodedSize: usize = 0;
        let cBlockSize: usize = ZSTDv06_getcBlockSize(
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
            &mut blockProperties,
        );
        if ERR_isError(cBlockSize) != 0 {
            return cBlockSize;
        }

        ip = ip.wrapping_add(ZSTDv06_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv06_blockHeaderSize);
        if cBlockSize > remainingSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }

        match blockProperties.blockType {
            bt_compressed => {
                decodedSize = ZSTDv06_decompressBlock_internal(
                    dctx,
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_raw => {
                decodedSize = ZSTDv06_copyRawBlock(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const c_void,
                    cBlockSize,
                );
            }
            bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet supported */
            }
            bt_end => {
                /* end of frame */
                if remainingSize != 0 {
                    return ERROR(ZSTD_error_srcSize_wrong);
                }
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }
        }
        if cBlockSize == 0 {
            break; /* bt_end */
        }

        if ERR_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.wrapping_add(decodedSize);
        ip = ip.wrapping_add(cBlockSize);
        remainingSize = remainingSize.wrapping_sub(cBlockSize);
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress_usingPreparedDCtx(
    dctx: *mut ZSTDv06_DCtx,
    refDCtx: *const ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv06_copyDCtx(dctx, refDCtx);
    ZSTDv06_checkContinuity(dctx, dst as *const c_void);
    ZSTDv06_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress_usingDict(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTDv06_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv06_checkContinuity(dctx, dst as *const c_void);
    ZSTDv06_decompressFrame(dctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressDCtx(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTDv06_decompress_usingDict(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        core::ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* ZSTDv06_HEAPMODE == 1 */
    let regenSize: usize;
    let dctx: *mut ZSTDv06_DCtx = ZSTDv06_createDCtx();
    if dctx.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    regenSize = ZSTDv06_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv06_freeDCtx(dctx);
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
pub unsafe extern "C" fn ZSTDv06_findFrameSizeInfoLegacy(
    src: *const c_void,
    srcSize: usize,
    cSize: *mut usize,
    dBound: *mut c_ulonglong,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: usize = srcSize;
    let mut nbBlocks: usize = 0;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };

    /* Frame Header */
    {
        let frameHeaderSize: usize = ZSTDv06_frameHeaderSize(src, srcSize);
        if ERR_isError(frameHeaderSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
            return;
        }
        if MEM_readLE32(src) != ZSTDv06_MAGICNUMBER {
            ZSTD_errorFrameSizeInfoLegacy(
                cSize,
                dBound,
                ERROR(ZSTD_error_prefix_unknown),
            );
            return;
        }
        if srcSize < frameHeaderSize.wrapping_add(ZSTDv06_blockHeaderSize) {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(ZSTD_error_srcSize_wrong));
            return;
        }
        ip = ip.wrapping_add(frameHeaderSize);
        remainingSize = remainingSize.wrapping_sub(frameHeaderSize);
    }

    /* Loop on each block */
    loop {
        let cBlockSize: usize = ZSTDv06_getcBlockSize(
            ip as *const c_void,
            remainingSize,
            &mut blockProperties,
        );
        if ERR_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }

        ip = ip.wrapping_add(ZSTDv06_blockHeaderSize);
        remainingSize = remainingSize.wrapping_sub(ZSTDv06_blockHeaderSize);
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
    *dBound = nbBlocks.wrapping_mul(ZSTDv06_BLOCKSIZE_MAX) as c_ulonglong;
}

/*_******************************
*  Streaming Decompression API
********************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_nextSrcSizeToDecompress(dctx: *mut ZSTDv06_DCtx) -> usize {
    (*dctx).expected
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressContinue(
    dctx: *mut ZSTDv06_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* Sanity check */
    if srcSize != (*dctx).expected {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dstCapacity != 0 {
        ZSTDv06_checkContinuity(dctx, dst as *const c_void);
    }

    /* Decompress : frame header; part 1 */
    let stage: ZSTDv06_dStage = (*dctx).stage;
    if stage == ZSTDds_getFrameHeaderSize || stage == ZSTDds_decodeFrameHeader {
        if stage == ZSTDds_getFrameHeaderSize {
            if srcSize != ZSTDv06_frameHeaderSize_min {
                return ERROR(ZSTD_error_srcSize_wrong); /* impossible */
            }
            (*dctx).headerSize = ZSTDv06_frameHeaderSize(src, ZSTDv06_frameHeaderSize_min);
            if ERR_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            memcpy(
                (*dctx).headerBuffer.as_mut_ptr() as *mut c_void,
                src,
                ZSTDv06_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv06_frameHeaderSize_min {
                (*dctx).expected = (*dctx)
                    .headerSize
                    .wrapping_sub(ZSTDv06_frameHeaderSize_min);
                (*dctx).stage = ZSTDds_decodeFrameHeader;
                return 0;
            }
            (*dctx).expected = 0; /* not necessary to copy more */
            /* fall-through */
        }
        /* case ZSTDds_decodeFrameHeader: */
        {
            let result: usize;
            memcpy(
                (*dctx)
                    .headerBuffer
                    .as_mut_ptr()
                    .wrapping_add(ZSTDv06_frameHeaderSize_min) as *mut c_void,
                src,
                (*dctx).expected,
            );
            result = ZSTDv06_decodeFrameHeader(
                dctx,
                (*dctx).headerBuffer.as_ptr() as *const c_void,
                (*dctx).headerSize,
            );
            if ERR_isError(result) != 0 {
                return result;
            }
            (*dctx).expected = ZSTDv06_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            return 0;
        }
    } else if stage == ZSTDds_decodeBlockHeader {
        let mut bp: blockProperties_t = blockProperties_t {
            blockType: bt_compressed,
            origSize: 0,
        };
        let cBlockSize: usize =
            ZSTDv06_getcBlockSize(src, ZSTDv06_blockHeaderSize, &mut bp);
        if ERR_isError(cBlockSize) != 0 {
            return cBlockSize;
        }
        if bp.blockType == bt_end {
            (*dctx).expected = 0;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
        } else {
            (*dctx).expected = cBlockSize;
            (*dctx).bType = bp.blockType;
            (*dctx).stage = ZSTDds_decompressBlock;
        }
        return 0;
    } else if stage == ZSTDds_decompressBlock {
        let rSize: usize;
        match (*dctx).bType {
            bt_compressed => {
                rSize =
                    ZSTDv06_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
            }
            bt_raw => {
                rSize = ZSTDv06_copyRawBlock(dst, dstCapacity, src, srcSize);
            }
            bt_rle => {
                return ERROR(ZSTD_error_GENERIC); /* not yet handled */
            }
            bt_end => {
                /* should never happen (filtered at phase 1) */
                rSize = 0;
            }
            _ => {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }
        }
        (*dctx).stage = ZSTDds_decodeBlockHeader;
        (*dctx).expected = ZSTDv06_blockHeaderSize;
        if ERR_isError(rSize) != 0 {
            return rSize;
        }
        (*dctx).previousDstEnd = (dst as *mut c_char).wrapping_add(rSize) as *const c_void;
        return rSize;
    } else {
        return ERROR(ZSTD_error_GENERIC); /* impossible */
    }
}

pub unsafe fn ZSTDv06_refDictContent(
    dctx: *mut ZSTDv06_DCtx,
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

pub unsafe fn ZSTDv06_loadEntropy(
    dctx: *mut ZSTDv06_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let hSize: usize;
    let offcodeHeaderSize: usize;
    let matchlengthHeaderSize: usize;
    let litlengthHeaderSize: usize;

    hSize = HUFv06_readDTableX4((*dctx).hufTableX4.as_mut_ptr(), dict, dictSize);
    if ERR_isError(hSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    dict = (dict as *const c_char).wrapping_add(hSize) as *const c_void;
    dictSize = dictSize.wrapping_sub(hSize);

    {
        let mut offcodeNCount: [i16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
        let mut offcodeMaxValue: c_uint = MaxOff;
        let mut offcodeLog: c_uint = 0;
        offcodeHeaderSize = FSEv06_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dict,
            dictSize,
        );
        if ERR_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode: usize = FSEv06_buildDTable(
                (*dctx).OffTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                offcodeMaxValue,
                offcodeLog,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dict = (dict as *const c_char).wrapping_add(offcodeHeaderSize) as *const c_void;
        dictSize = dictSize.wrapping_sub(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [i16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        matchlengthHeaderSize = FSEv06_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dict,
            dictSize,
        );
        if ERR_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode: usize = FSEv06_buildDTable(
                (*dctx).MLTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
        dict = (dict as *const c_char).wrapping_add(matchlengthHeaderSize) as *const c_void;
        dictSize = dictSize.wrapping_sub(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [i16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        litlengthHeaderSize = FSEv06_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dict,
            dictSize,
        );
        if ERR_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        {
            let errorCode: usize = FSEv06_buildDTable(
                (*dctx).LLTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
            );
            if ERR_isError(errorCode) != 0 {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
        }
    }

    (*dctx).flagRepeatTable = 1;
    hSize
        .wrapping_add(offcodeHeaderSize)
        .wrapping_add(matchlengthHeaderSize)
        .wrapping_add(litlengthHeaderSize)
}

pub unsafe fn ZSTDv06_decompress_insertDictionary(
    dctx: *mut ZSTDv06_DCtx,
    mut dict: *const c_void,
    mut dictSize: usize,
) -> usize {
    let eSize: usize;
    let magic: U32 = MEM_readLE32(dict);
    if magic != ZSTDv06_DICT_MAGIC {
        /* pure content mode */
        ZSTDv06_refDictContent(dctx, dict, dictSize);
        return 0;
    }
    /* load entropy tables */
    dict = (dict as *const c_char).wrapping_add(4) as *const c_void;
    dictSize = dictSize.wrapping_sub(4);
    eSize = ZSTDv06_loadEntropy(dctx, dict, dictSize);
    if ERR_isError(eSize) != 0 {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }

    /* reference dictionary content */
    dict = (dict as *const c_char).wrapping_add(eSize) as *const c_void;
    dictSize = dictSize.wrapping_sub(eSize);
    ZSTDv06_refDictContent(dctx, dict, dictSize);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv06_decompressBegin_usingDict(
    dctx: *mut ZSTDv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    {
        let errorCode: usize = ZSTDv06_decompressBegin(dctx);
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    }

    if !dict.is_null() && dictSize != 0 {
        let errorCode: usize = ZSTDv06_decompress_insertDictionary(dctx, dict, dictSize);
        if ERR_isError(errorCode) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
    }

    0
}

/*-***************************************************************************
*  Buffered version of Zstd compression library : streaming decompression
* ***************************************************************************/
pub type ZBUFFv06_dStage = c_int;
pub const ZBUFFds_init: ZBUFFv06_dStage = 0;
pub const ZBUFFds_loadHeader: ZBUFFv06_dStage = 1;
pub const ZBUFFds_read: ZBUFFv06_dStage = 2;
pub const ZBUFFds_load: ZBUFFv06_dStage = 3;
pub const ZBUFFds_flush: ZBUFFv06_dStage = 4;

/* *** Resource management *** */
#[repr(C)]
pub struct ZBUFFv06_DCtx {
    pub zd: *mut ZSTDv06_DCtx,
    pub fParams: ZSTDv06_frameParams,
    pub stage: ZBUFFv06_dStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub blockSize: usize,
    pub headerBuffer: [BYTE; ZSTDv06_FRAMEHEADERSIZE_MAX],
    pub lhSize: usize,
} /* typedef'd to ZBUFFv06_DCtx within "zstd_buffered.h" */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_createDCtx() -> *mut ZBUFFv06_DCtx {
    let zbd: *mut ZBUFFv06_DCtx =
        malloc(core::mem::size_of::<ZBUFFv06_DCtx>()) as *mut ZBUFFv06_DCtx;
    if zbd.is_null() {
        return core::ptr::null_mut();
    }
    memset(
        zbd as *mut c_void,
        0,
        core::mem::size_of::<ZBUFFv06_DCtx>(),
    );
    (*zbd).zd = ZSTDv06_createDCtx();
    if (*zbd).zd.is_null() {
        ZBUFFv06_freeDCtx(zbd); /* avoid leaking the context */
        return core::ptr::null_mut();
    }
    (*zbd).stage = ZBUFFds_init;
    zbd
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_freeDCtx(zbd: *mut ZBUFFv06_DCtx) -> usize {
    if zbd.is_null() {
        return 0; /* support free on null */
    }
    ZSTDv06_freeDCtx((*zbd).zd);
    free((*zbd).inBuff as *mut c_void);
    free((*zbd).outBuff as *mut c_void);
    free(zbd as *mut c_void);
    0
}

/* *** Initialization *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressInitDictionary(
    zbd: *mut ZBUFFv06_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).outEnd = 0;
    (*zbd).outStart = 0;
    (*zbd).inPos = 0;
    (*zbd).lhSize = 0;
    ZSTDv06_decompressBegin_usingDict((*zbd).zd, dict, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressInit(zbd: *mut ZBUFFv06_DCtx) -> usize {
    ZBUFFv06_decompressInitDictionary(zbd, core::ptr::null(), 0)
}

pub unsafe fn ZBUFFv06_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length: usize = if dstCapacity < srcSize {
        dstCapacity
    } else {
        srcSize
    };
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

/* *** Decompression *** */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv06_decompressContinue(
    zbd: *mut ZBUFFv06_DCtx,
    dst: *mut c_void,
    dstCapacityPtr: *mut usize,
    src: *const c_void,
    srcSizePtr: *mut usize,
) -> usize {
    let istart: *const c_char = src as *const c_char;
    let iend: *const c_char = istart.wrapping_add(*srcSizePtr);
    let mut ip: *const c_char = istart;
    let ostart: *mut c_char = dst as *mut c_char;
    let oend: *mut c_char = ostart.wrapping_add(*dstCapacityPtr);
    let mut op: *mut c_char = ostart;
    let mut notDone: U32 = 1;

    while notDone != 0 {
        /* `loop { ... }` emulates the C `switch` : `break` == leave the switch */
        loop {
            let mut cur: ZBUFFv06_dStage = (*zbd).stage;

            if cur == ZBUFFds_init {
                return ERROR(ZSTD_error_init_missing);
            }
            if cur != ZBUFFds_loadHeader
                && cur != ZBUFFds_read
                && cur != ZBUFFds_load
                && cur != ZBUFFds_flush
            {
                return ERROR(ZSTD_error_GENERIC); /* impossible */
            }

            if cur == ZBUFFds_loadHeader {
                {
                    let hSize: usize = ZSTDv06_getFrameParams(
                        &mut (*zbd).fParams,
                        (*zbd).headerBuffer.as_ptr() as *const c_void,
                        (*zbd).lhSize,
                    );
                    if hSize != 0 {
                        let toLoad: usize = hSize.wrapping_sub((*zbd).lhSize); /* if hSize!=0, hSize > zbd->lhSize */
                        if ERR_isError(hSize) != 0 {
                            return hSize;
                        }
                        if toLoad > (iend as usize).wrapping_sub(ip as usize) {
                            /* not enough input to load full header */
                            if !ip.is_null() {
                                memcpy(
                                    (*zbd)
                                        .headerBuffer
                                        .as_mut_ptr()
                                        .wrapping_add((*zbd).lhSize) as *mut c_void,
                                    ip as *const c_void,
                                    (iend as usize).wrapping_sub(ip as usize),
                                );
                            }
                            (*zbd).lhSize = (*zbd)
                                .lhSize
                                .wrapping_add((iend as usize).wrapping_sub(ip as usize));
                            *dstCapacityPtr = 0;
                            return (hSize.wrapping_sub((*zbd).lhSize))
                                .wrapping_add(ZSTDv06_blockHeaderSize); /* remaining header bytes + next block header */
                        }
                        memcpy(
                            (*zbd).headerBuffer.as_mut_ptr().wrapping_add((*zbd).lhSize)
                                as *mut c_void,
                            ip as *const c_void,
                            toLoad,
                        );
                        (*zbd).lhSize = hSize;
                        ip = ip.wrapping_add(toLoad);
                        break;
                    }
                }

                /* Consume header */
                {
                    let h1Size: usize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd); /* == ZSTDv06_frameHeaderSize_min */
                    let h1Result: usize = ZSTDv06_decompressContinue(
                        (*zbd).zd,
                        core::ptr::null_mut(),
                        0,
                        (*zbd).headerBuffer.as_ptr() as *const c_void,
                        h1Size,
                    );
                    if ERR_isError(h1Result) != 0 {
                        return h1Result;
                    }
                    if h1Size < (*zbd).lhSize {
                        /* long header */
                        let h2Size: usize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                        let h2Result: usize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            core::ptr::null_mut(),
                            0,
                            (*zbd).headerBuffer.as_ptr().wrapping_add(h1Size)
                                as *const c_void,
                            h2Size,
                        );
                        if ERR_isError(h2Result) != 0 {
                            return h2Result;
                        }
                    }
                }

                /* Frame header instruct buffer sizes */
                {
                    let blockSize: usize = {
                        let a: c_int = 1i32.wrapping_shl((*zbd).fParams.windowLog);
                        let b: c_int = ZSTDv06_BLOCKSIZE_MAX as c_int;
                        (if a < b { a } else { b }) as usize
                    };
                    (*zbd).blockSize = blockSize;
                    if (*zbd).inBuffSize < blockSize {
                        free((*zbd).inBuff as *mut c_void);
                        (*zbd).inBuffSize = blockSize;
                        (*zbd).inBuff = malloc(blockSize) as *mut c_char;
                        if (*zbd).inBuff.is_null() {
                            return ERROR(ZSTD_error_memory_allocation);
                        }
                    }
                    {
                        let neededOutSize: usize = (1usize
                            .wrapping_shl((*zbd).fParams.windowLog))
                        .wrapping_add(blockSize)
                        .wrapping_add(WILDCOPY_OVERLENGTH * 2);
                        if (*zbd).outBuffSize < neededOutSize {
                            free((*zbd).outBuff as *mut c_void);
                            (*zbd).outBuffSize = neededOutSize;
                            (*zbd).outBuff = malloc(neededOutSize) as *mut c_char;
                            if (*zbd).outBuff.is_null() {
                                return ERROR(ZSTD_error_memory_allocation);
                            }
                        }
                    }
                }
                (*zbd).stage = ZBUFFds_read;
                cur = ZBUFFds_read;
                /* fall-through */
            }

            if cur == ZBUFFds_read {
                {
                    let neededInSize: usize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    if neededInSize == 0 {
                        /* end of frame */
                        (*zbd).stage = ZBUFFds_init;
                        notDone = 0;
                        break;
                    }
                    if (iend as usize).wrapping_sub(ip as usize) >= neededInSize {
                        /* decode directly from src */
                        let decodedSize: usize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize.wrapping_sub((*zbd).outStart),
                            ip as *const c_void,
                            neededInSize,
                        );
                        if ERR_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        ip = ip.wrapping_add(neededInSize);
                        if decodedSize == 0 {
                            break; /* this was just a header */
                        }
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize);
                        (*zbd).stage = ZBUFFds_flush;
                        break;
                    }
                    if ip == iend {
                        notDone = 0;
                        break;
                    } /* no more input */
                    (*zbd).stage = ZBUFFds_load;
                }
                cur = ZBUFFds_load;
                /* fall-through */
            }

            if cur == ZBUFFds_load {
                {
                    let neededInSize: usize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
                    let toLoad: usize = neededInSize.wrapping_sub((*zbd).inPos); /* should always be <= remaining space within inBuff */
                    let mut loadedSize: usize;
                    if toLoad > (*zbd).inBuffSize.wrapping_sub((*zbd).inPos) {
                        return ERROR(ZSTD_error_corruption_detected); /* should never happen */
                    }
                    loadedSize = ZBUFFv06_limitCopy(
                        (*zbd).inBuff.wrapping_add((*zbd).inPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        (iend as usize).wrapping_sub(ip as usize),
                    );
                    ip = ip.wrapping_add(loadedSize);
                    (*zbd).inPos = (*zbd).inPos.wrapping_add(loadedSize);
                    if loadedSize < toLoad {
                        notDone = 0;
                        break;
                    } /* not enough input, wait for more */

                    /* decode loaded input */
                    {
                        let decodedSize: usize = ZSTDv06_decompressContinue(
                            (*zbd).zd,
                            (*zbd).outBuff.wrapping_add((*zbd).outStart) as *mut c_void,
                            (*zbd).outBuffSize.wrapping_sub((*zbd).outStart),
                            (*zbd).inBuff as *const c_void,
                            neededInSize,
                        );
                        if ERR_isError(decodedSize) != 0 {
                            return decodedSize;
                        }
                        (*zbd).inPos = 0; /* input is consumed */
                        if decodedSize == 0 {
                            (*zbd).stage = ZBUFFds_read;
                            break;
                        } /* this was just a header */
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize);
                        (*zbd).stage = ZBUFFds_flush;
                        /* break; */ /* ZBUFFds_flush follows */
                    }
                }
                cur = ZBUFFds_flush;
                /* fall-through */
            }

            /* case ZBUFFds_flush: */
            {
                let toFlushSize: usize = (*zbd).outEnd.wrapping_sub((*zbd).outStart);
                let flushedSize: usize = ZBUFFv06_limitCopy(
                    op as *mut c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    (*zbd).outBuff.wrapping_add((*zbd).outStart) as *const c_void,
                    toFlushSize,
                );
                op = op.wrapping_add(flushedSize);
                (*zbd).outStart = (*zbd).outStart.wrapping_add(flushedSize);
                if flushedSize == toFlushSize {
                    (*zbd).stage = ZBUFFds_read;
                    if (*zbd).outStart.wrapping_add((*zbd).blockSize) > (*zbd).outBuffSize {
                        (*zbd).outStart = 0;
                        (*zbd).outEnd = 0;
                    }
                    break;
                }
                /* cannot flush everything */
                notDone = 0;
                break;
            }
        }
    }

    /* result */
    *srcSizePtr = (ip as usize).wrapping_sub(istart as usize);
    *dstCapacityPtr = (op as usize).wrapping_sub(ostart as usize);
    {
        let mut nextSrcSizeHint: usize = ZSTDv06_nextSrcSizeToDecompress((*zbd).zd);
        if nextSrcSizeHint > ZSTDv06_blockHeaderSize {
            nextSrcSizeHint = nextSrcSizeHint.wrapping_add(ZSTDv06_blockHeaderSize);
            /* get following block header too */
        }
        nextSrcSizeHint = nextSrcSizeHint.wrapping_sub((*zbd).inPos); /* already loaded*/
        return nextSrcSizeHint;
    }
}

/* *************************************
*  Tool functions
***************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv06_recommendedDInSize() -> usize {
    ZSTDv06_BLOCKSIZE_MAX + ZSTDv06_blockHeaderSize /* block header size*/
}

#[unsafe(no_mangle)]
pub extern "C" fn ZBUFFv06_recommendedDOutSize() -> usize {
    ZSTDv06_BLOCKSIZE_MAX
}
