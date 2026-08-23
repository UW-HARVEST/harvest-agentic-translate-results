//! Translation of compress/huf_compress.c
//!
//! Huffman encoder, part of New Generation Entropy library
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use core::ptr::{addr_of, addr_of_mut};

use crate::bits::*;
use crate::error_private::*;
use crate::fse::*;
use crate::hist::HIST_WKSP_SIZE_U32;
use crate::huf::*;
use crate::mem::*;

/* **************************************************************
*  Error Management
****************************************************************/
/* #define HUF_isError ERR_isError */
#[inline(always)]
pub fn HUF_isError(code: usize) -> core::ffi::c_uint {
    ERR_isError(code)
}

/* **************************************************************
*  Required declarations
****************************************************************/
/*
 * typedef struct nodeElt_s {
 *     U32 count;
 *     U16 parent;
 *     BYTE byte;
 *     BYTE nbBits;
 * } nodeElt;
 */
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct nodeElt {
    pub count: U32,
    pub parent: U16,
    pub byte: BYTE,
    pub nbBits: BYTE,
}

/* **************************************************************
*  Debug Traces
****************************************************************/
/* DEBUGLEVEL == 0 : showU32() / showCTableBits() / showHNodeSymbols() /
 * showHNodeBits() are not compiled in. */

/* *******************************************************
*  HUF : Huffman block compression
*********************************************************/
/* #define HUF_WORKSPACE_MAX_ALIGNMENT 8 */
pub const HUF_WORKSPACE_MAX_ALIGNMENT: usize = 8;

pub unsafe fn HUF_alignUpWorkspace(
    workspace: *mut core::ffi::c_void,
    workspaceSizePtr: *mut usize,
    align: usize,
) -> *mut core::ffi::c_void {
    let mask: usize = align.wrapping_sub(1);
    let rem: usize = (workspace as usize) & mask;
    let add: usize = (align.wrapping_sub(rem)) & mask;
    let aligned: *mut BYTE = (workspace as *mut BYTE).wrapping_add(add);
    if *workspaceSizePtr >= add {
        *workspaceSizePtr = (*workspaceSizePtr).wrapping_sub(add);
        return aligned as *mut core::ffi::c_void;
    } else {
        *workspaceSizePtr = 0;
        return core::ptr::null_mut();
    }
}

/* HUF_compressWeights() :
 * Same as FSE_compress(), but dedicated to huff0's weights compression.
 * The use case needs much less stack memory.
 * Note : all elements within weightTable are supposed to be <= HUF_TABLELOG_MAX.
 */
/* #define MAX_FSE_TABLELOG_FOR_HUFF_HEADER 6 */
pub const MAX_FSE_TABLELOG_FOR_HUFF_HEADER: U32 = 6;

pub const HUF_CW_CTABLE_LEN: usize =
    FSE_CTABLE_SIZE_U32(MAX_FSE_TABLELOG_FOR_HUFF_HEADER, HUF_TABLELOG_MAX);
pub const HUF_CW_SCRATCH_LEN: usize =
    FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(HUF_TABLELOG_MAX, MAX_FSE_TABLELOG_FOR_HUFF_HEADER);

/*
 * typedef struct {
 *     FSE_CTable CTable[FSE_CTABLE_SIZE_U32(MAX_FSE_TABLELOG_FOR_HUFF_HEADER, HUF_TABLELOG_MAX)];
 *     U32 scratchBuffer[FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(HUF_TABLELOG_MAX, MAX_FSE_TABLELOG_FOR_HUFF_HEADER)];
 *     unsigned count[HUF_TABLELOG_MAX+1];
 *     S16 norm[HUF_TABLELOG_MAX+1];
 * } HUF_CompressWeightsWksp;
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_CompressWeightsWksp {
    pub CTable: [FSE_CTable; HUF_CW_CTABLE_LEN],
    pub scratchBuffer: [U32; HUF_CW_SCRATCH_LEN],
    pub count: [core::ffi::c_uint; HUF_TABLELOG_MAX as usize + 1],
    pub norm: [S16; HUF_TABLELOG_MAX as usize + 1],
}

pub unsafe fn HUF_compressWeights(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    weightTable: *const core::ffi::c_void,
    wtSize: usize,
    workspace: *mut core::ffi::c_void,
    mut workspaceSize: usize,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);

    let mut maxSymbolValue: core::ffi::c_uint = HUF_TABLELOG_MAX;
    let mut tableLog: U32 = MAX_FSE_TABLELOG_FOR_HUFF_HEADER;
    let wksp: *mut HUF_CompressWeightsWksp = HUF_alignUpWorkspace(
        workspace,
        &mut workspaceSize,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_CompressWeightsWksp;

    if workspaceSize < core::mem::size_of::<HUF_CompressWeightsWksp>() {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* init conditions */
    if wtSize <= 1 {
        return 0; /* Not compressible */
    }

    /* Scan input and build symbol stats */
    {
        let maxCount: core::ffi::c_uint = crate::hist::HIST_count_simple(
            addr_of_mut!((*wksp).count) as *mut core::ffi::c_uint,
            &mut maxSymbolValue,
            weightTable,
            wtSize,
        ); /* never fails */
        if maxCount as usize == wtSize {
            return 1; /* only a single symbol in src : rle */
        }
        if maxCount == 1 {
            return 0; /* each symbol present maximum once => not compressible */
        }
    }

    tableLog = crate::fse_compress::FSE_optimalTableLog(tableLog, wtSize, maxSymbolValue);
    {
        let err_code = crate::fse_compress::FSE_normalizeCount(
            addr_of_mut!((*wksp).norm) as *mut S16,
            tableLog,
            addr_of!((*wksp).count) as *const core::ffi::c_uint,
            wtSize,
            maxSymbolValue,
            /* useLowProbCount */ 0,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    /* Write table description header */
    {
        let hSize = crate::fse_compress::FSE_writeNCount(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            addr_of!((*wksp).norm) as *const S16,
            maxSymbolValue,
            tableLog,
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        op = op.wrapping_add(hSize);
    }

    /* Compress */
    {
        let err_code = crate::fse_compress::FSE_buildCTable_wksp(
            addr_of_mut!((*wksp).CTable) as *mut FSE_CTable,
            addr_of!((*wksp).norm) as *const S16,
            maxSymbolValue,
            tableLog,
            addr_of_mut!((*wksp).scratchBuffer) as *mut core::ffi::c_void,
            core::mem::size_of::<[U32; HUF_CW_SCRATCH_LEN]>(),
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let cSize = crate::fse_compress::FSE_compress_usingCTable(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            weightTable,
            wtSize,
            addr_of!((*wksp).CTable) as *const FSE_CTable,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 {
            return 0; /* not enough space for compressed data */
        }
        op = op.wrapping_add(cSize);
    }

    (op as usize).wrapping_sub(ostart as usize)
}

pub fn HUF_getNbBits(elt: HUF_CElt) -> usize {
    elt & 0xFF
}

pub fn HUF_getNbBitsFast(elt: HUF_CElt) -> usize {
    elt
}

pub fn HUF_getValue(elt: HUF_CElt) -> usize {
    elt & !(0xFFusize)
}

pub fn HUF_getValueFast(elt: HUF_CElt) -> usize {
    elt
}

pub unsafe fn HUF_setNbBits(elt: *mut HUF_CElt, nbBits: usize) {
    *elt = nbBits;
}

pub unsafe fn HUF_setValue(elt: *mut HUF_CElt, value: usize) {
    let nbBits: usize = HUF_getNbBits(*elt);
    if nbBits > 0 {
        *elt |= value << (core::mem::size_of::<HUF_CElt>() * 8 - nbBits);
    }
}

/// `HUF_CTableHeader HUF_readCTableHeader(HUF_CElt const* ctable)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTableHeader(ctable: *const HUF_CElt) -> HUF_CTableHeader {
    let mut header = core::mem::MaybeUninit::<HUF_CTableHeader>::uninit();
    ZSTD_memcpy(
        header.as_mut_ptr() as *mut u8,
        ctable as *const u8,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
    header.assume_init()
}

pub unsafe fn HUF_writeCTableHeader(ctable: *mut HUF_CElt, tableLog: U32, maxSymbolValue: U32) {
    let mut header = core::mem::MaybeUninit::<HUF_CTableHeader>::uninit();
    ZSTD_memset(
        header.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
    (*header.as_mut_ptr()).tableLog = tableLog as BYTE;
    (*header.as_mut_ptr()).maxSymbolValue = maxSymbolValue as BYTE;
    ZSTD_memcpy(
        ctable as *mut u8,
        header.as_ptr() as *const u8,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
}

/*
 * typedef struct {
 *     HUF_CompressWeightsWksp wksp;
 *     BYTE bitsToWeight[HUF_TABLELOG_MAX + 1];
 *     BYTE huffWeight[HUF_SYMBOLVALUE_MAX];
 * } HUF_WriteCTableWksp;
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_WriteCTableWksp {
    pub wksp: HUF_CompressWeightsWksp,
    pub bitsToWeight: [BYTE; HUF_TABLELOG_MAX as usize + 1], /* precomputed conversion table */
    pub huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize],
}

/// `size_t HUF_writeCTable_wksp(void* dst, size_t maxDstSize, const HUF_CElt* CTable,
///                              unsigned maxSymbolValue, unsigned huffLog,
///                              void* workspace, size_t workspaceSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_writeCTable_wksp(
    dst: *mut core::ffi::c_void,
    maxDstSize: usize,
    CTable: *const HUF_CElt,
    maxSymbolValue: core::ffi::c_uint,
    huffLog: core::ffi::c_uint,
    workspace: *mut core::ffi::c_void,
    mut workspaceSize: usize,
) -> usize {
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let op: *mut BYTE = dst as *mut BYTE;
    let mut n: U32 = 0;
    let wksp: *mut HUF_WriteCTableWksp = HUF_alignUpWorkspace(
        workspace,
        &mut workspaceSize,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_WriteCTableWksp;

    /* check conditions */
    if workspaceSize < core::mem::size_of::<HUF_WriteCTableWksp>() {
        return ERROR(ZSTD_error_GENERIC);
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }

    let bitsToWeight: *mut BYTE = addr_of_mut!((*wksp).bitsToWeight) as *mut BYTE;
    let huffWeight: *mut BYTE = addr_of_mut!((*wksp).huffWeight) as *mut BYTE;

    /* convert to weight */
    *bitsToWeight.add(0) = 0;
    n = 1;
    while n < huffLog.wrapping_add(1) {
        *bitsToWeight.add(n as usize) = huffLog.wrapping_add(1).wrapping_sub(n) as BYTE;
        n = n.wrapping_add(1);
    }
    n = 0;
    while n < maxSymbolValue {
        *huffWeight.add(n as usize) =
            *bitsToWeight.add(HUF_getNbBits(*ct.wrapping_add(n as usize)));
        n = n.wrapping_add(1);
    }

    /* attempt weights compression by FSE */
    if maxDstSize < 1 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let hSize = HUF_compressWeights(
            op.wrapping_add(1) as *mut core::ffi::c_void,
            maxDstSize.wrapping_sub(1),
            huffWeight as *const core::ffi::c_void,
            maxSymbolValue as usize,
            addr_of_mut!((*wksp).wksp) as *mut core::ffi::c_void,
            core::mem::size_of::<HUF_CompressWeightsWksp>(),
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        if (((hSize > 1) as core::ffi::c_int) & ((hSize < (maxSymbolValue / 2) as usize) as core::ffi::c_int))
            != 0
        {
            /* FSE compressed */
            *op.add(0) = hSize as BYTE;
            return hSize.wrapping_add(1);
        }
    }

    /* write raw values as 4-bits (max : 15) */
    if maxSymbolValue > (256 - 128) {
        return ERROR(ZSTD_error_GENERIC); /* should not happen : likely means source cannot be compressed */
    }
    if ((maxSymbolValue.wrapping_add(1) / 2).wrapping_add(1)) as usize > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* not enough space within dst buffer */
    }
    *op.add(0) = (128u32.wrapping_add(maxSymbolValue.wrapping_sub(1))) as BYTE;
    *huffWeight.add(maxSymbolValue as usize) = 0; /* to be sure it doesn't cause msan issue in final combination */
    n = 0;
    while n < maxSymbolValue {
        *op.wrapping_add(((n / 2) as usize).wrapping_add(1)) = (((*huffWeight.add(n as usize)
            as U32)
            << 4)
            .wrapping_add(*huffWeight.add(n.wrapping_add(1) as usize) as U32))
            as BYTE;
        n = n.wrapping_add(2);
    }
    ((maxSymbolValue.wrapping_add(1) / 2).wrapping_add(1)) as usize
}

/// `size_t HUF_readCTable (HUF_CElt* CTable, unsigned* maxSymbolValuePtr,
///                         const void* src, size_t srcSize, unsigned* hasZeroWeights)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTable(
    CTable: *mut HUF_CElt,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    src: *const core::ffi::c_void,
    srcSize: usize,
    hasZeroWeights: *mut core::ffi::c_uint,
) -> usize {
    /* init not required, even though some static analyzer may complain */
    let mut huffWeight_arr: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1] =
        [0; HUF_SYMBOLVALUE_MAX as usize + 1];
    /* large enough for values from 0 to 16 */
    let mut rankVal_arr: [U32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1] =
        [0; HUF_TABLELOG_ABSOLUTEMAX as usize + 1];
    let huffWeight: *mut BYTE = huffWeight_arr.as_mut_ptr();
    let rankVal: *mut U32 = rankVal_arr.as_mut_ptr();
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let ct: *mut HUF_CElt = CTable.wrapping_add(1);

    /* get symbol weights */
    let readSize = crate::entropy_common::HUF_readStats(
        huffWeight,
        HUF_SYMBOLVALUE_MAX as usize + 1,
        rankVal,
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if ERR_isError(readSize) != 0 {
        return readSize;
    }
    *hasZeroWeights = (*rankVal.add(0) > 0) as core::ffi::c_uint;

    /* check result */
    if tableLog > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    if nbSymbols > (*maxSymbolValuePtr).wrapping_add(1) {
        return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
    }

    *maxSymbolValuePtr = nbSymbols.wrapping_sub(1);

    HUF_writeCTableHeader(CTable, tableLog, *maxSymbolValuePtr);

    /* Prepare base value per rank */
    {
        let mut n: U32;
        let mut nextRankStart: U32 = 0;
        n = 1;
        while n <= tableLog {
            let curr: U32 = nextRankStart;
            nextRankStart =
                nextRankStart.wrapping_add(*rankVal.add(n as usize) << (n.wrapping_sub(1)));
            *rankVal.add(n as usize) = curr;
            n = n.wrapping_add(1);
        }
    }

    /* fill nbBits */
    {
        let mut n: U32 = 0;
        while n < nbSymbols {
            let w: U32 = *huffWeight.add(n as usize) as U32;
            HUF_setNbBits(
                ct.wrapping_add(n as usize),
                ((tableLog.wrapping_add(1).wrapping_sub(w) as BYTE)
                    & (0u8.wrapping_sub((w != 0) as u8))) as usize,
            );
            n = n.wrapping_add(1);
        }
    }

    /* fill val */
    {
        /* support w=0=>n=tableLog+1 */
        let mut nbPerRank_arr: [U16; HUF_TABLELOG_MAX as usize + 2] =
            [0; HUF_TABLELOG_MAX as usize + 2];
        let mut valPerRank_arr: [U16; HUF_TABLELOG_MAX as usize + 2] =
            [0; HUF_TABLELOG_MAX as usize + 2];
        let nbPerRank: *mut U16 = nbPerRank_arr.as_mut_ptr();
        let valPerRank: *mut U16 = valPerRank_arr.as_mut_ptr();
        {
            let mut n: U32 = 0;
            while n < nbSymbols {
                let p = nbPerRank.add(HUF_getNbBits(*ct.wrapping_add(n as usize)));
                *p = (*p).wrapping_add(1);
                n = n.wrapping_add(1);
            }
        }
        /* determine stating value per rank */
        *valPerRank.add(tableLog.wrapping_add(1) as usize) = 0; /* for w==0 */
        {
            let mut min: U16 = 0;
            let mut n: U32 = tableLog;
            while n > 0 {
                /* start at n=tablelog <-> w=1 */
                *valPerRank.add(n as usize) = min; /* get starting value within each rank */
                min = min.wrapping_add(*nbPerRank.add(n as usize));
                min >>= 1;
                n = n.wrapping_sub(1);
            }
        }
        /* assign value within rank, symbol order */
        {
            let mut n: U32 = 0;
            while n < nbSymbols {
                let p = valPerRank.add(HUF_getNbBits(*ct.wrapping_add(n as usize)));
                let v: U16 = *p;
                *p = v.wrapping_add(1);
                HUF_setValue(ct.wrapping_add(n as usize), v as usize);
                n = n.wrapping_add(1);
            }
        }
    }

    readSize
}

/// `U32 HUF_getNbBitsFromCTable(HUF_CElt const* CTable, U32 symbolValue)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getNbBitsFromCTable(CTable: *const HUF_CElt, symbolValue: U32) -> U32 {
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    if symbolValue > HUF_readCTableHeader(CTable).maxSymbolValue as U32 {
        return 0;
    }
    HUF_getNbBits(*ct.wrapping_add(symbolValue as usize)) as U32
}

/**
 * HUF_setMaxHeight():
 * Try to enforce @targetNbBits on the Huffman tree described in @huffNode.
 */
pub unsafe fn HUF_setMaxHeight(huffNode: *mut nodeElt, lastNonNull: U32, targetNbBits: U32) -> U32 {
    let largestBits: U32 = (*huffNode.wrapping_add(lastNonNull as usize)).nbBits as U32;
    /* early exit : no elt > targetNbBits, so the tree is already valid. */
    if largestBits <= targetNbBits {
        return largestBits;
    }

    /* there are several too large elements (at least >= 2) */
    {
        let mut totalCost: core::ffi::c_int = 0;
        let baseCost: U32 = 1u32 << (largestBits.wrapping_sub(targetNbBits));
        let mut n: core::ffi::c_int = lastNonNull as core::ffi::c_int;

        /* Adjust any ranks > targetNbBits to targetNbBits.
         * Compute totalCost, which is how far the sum of the ranks is
         * we are over 2^largestBits after adjust the offending ranks.
         */
        while (*huffNode.offset(n as isize)).nbBits as U32 > targetNbBits {
            totalCost = totalCost.wrapping_add(
                baseCost.wrapping_sub(
                    1u32 << (largestBits
                        .wrapping_sub((*huffNode.offset(n as isize)).nbBits as U32)),
                ) as core::ffi::c_int,
            );
            (*huffNode.offset(n as isize)).nbBits = targetNbBits as BYTE;
            n -= 1;
        }
        /* n stops at huffNode[n].nbBits <= targetNbBits */
        /* n end at index of smallest symbol using < targetNbBits */
        while (*huffNode.offset(n as isize)).nbBits as U32 == targetNbBits {
            n -= 1;
        }

        /* renorm totalCost from 2^largestBits to 2^targetNbBits
         * note : totalCost is necessarily a multiple of baseCost */
        totalCost >>= (largestBits.wrapping_sub(targetNbBits));

        /* repay normalized cost */
        {
            let noSymbol: U32 = 0xF0F0F0F0;
            let mut rankLast_arr: [U32; HUF_TABLELOG_MAX as usize + 2] =
                [0; HUF_TABLELOG_MAX as usize + 2];
            let rankLast: *mut U32 = rankLast_arr.as_mut_ptr();

            /* Get pos of last (smallest = lowest cum. count) symbol per rank */
            ZSTD_memset(
                rankLast as *mut u8,
                0xF0,
                core::mem::size_of::<[U32; HUF_TABLELOG_MAX as usize + 2]>(),
            );
            {
                let mut currentNbBits: U32 = targetNbBits;
                let mut pos: core::ffi::c_int;
                pos = n;
                while pos >= 0 {
                    if (*huffNode.offset(pos as isize)).nbBits as U32 >= currentNbBits {
                        pos -= 1;
                        continue;
                    }
                    currentNbBits = (*huffNode.offset(pos as isize)).nbBits as U32; /* < targetNbBits */
                    *rankLast.add(targetNbBits.wrapping_sub(currentNbBits) as usize) = pos as U32;
                    pos -= 1;
                }
            }

            while totalCost > 0 {
                /* Try to reduce the next power of 2 above totalCost because we
                 * gain back half the rank.
                 */
                let mut nBitsToDecrease: U32 = ZSTD_highbit32(totalCost as U32).wrapping_add(1);
                while nBitsToDecrease > 1 {
                    let highPos: U32 = *rankLast.add(nBitsToDecrease as usize);
                    let lowPos: U32 = *rankLast.add(nBitsToDecrease.wrapping_sub(1) as usize);
                    if highPos == noSymbol {
                        nBitsToDecrease = nBitsToDecrease.wrapping_sub(1);
                        continue;
                    }
                    /* Decrease highPos if no symbols of lowPos or if it is
                     * not cheaper to remove 2 lowPos than highPos.
                     */
                    if lowPos == noSymbol {
                        break;
                    }
                    {
                        let highTotal: U32 = (*huffNode.wrapping_add(highPos as usize)).count;
                        let lowTotal: U32 =
                            2u32.wrapping_mul((*huffNode.wrapping_add(lowPos as usize)).count);
                        if highTotal <= lowTotal {
                            break;
                        }
                    }
                    nBitsToDecrease = nBitsToDecrease.wrapping_sub(1);
                }
                /* HUF_MAX_TABLELOG test just to please gcc 5+; but it should not be necessary */
                while (nBitsToDecrease <= HUF_TABLELOG_MAX)
                    && (*rankLast.add(nBitsToDecrease as usize) == noSymbol)
                {
                    nBitsToDecrease = nBitsToDecrease.wrapping_add(1);
                }
                /* Increase the number of bits to gain back half the rank cost. */
                totalCost -= 1i32 << (nBitsToDecrease.wrapping_sub(1));
                {
                    let p = huffNode
                        .wrapping_add(*rankLast.add(nBitsToDecrease as usize) as usize);
                    (*p).nbBits = (*p).nbBits.wrapping_add(1);
                }

                /* Fix up the new rank.
                 * If the new rank was empty, this symbol is now its smallest.
                 * Otherwise, this symbol will be the largest in the new rank so no adjustment.
                 */
                if *rankLast.add(nBitsToDecrease.wrapping_sub(1) as usize) == noSymbol {
                    *rankLast.add(nBitsToDecrease.wrapping_sub(1) as usize) =
                        *rankLast.add(nBitsToDecrease as usize);
                }
                /* Fix up the old rank. */
                if *rankLast.add(nBitsToDecrease as usize) == 0 {
                    /* special case, reached largest symbol */
                    *rankLast.add(nBitsToDecrease as usize) = noSymbol;
                } else {
                    *rankLast.add(nBitsToDecrease as usize) =
                        (*rankLast.add(nBitsToDecrease as usize)).wrapping_sub(1);
                    if (*huffNode.wrapping_add(*rankLast.add(nBitsToDecrease as usize) as usize))
                        .nbBits as U32
                        != targetNbBits.wrapping_sub(nBitsToDecrease)
                    {
                        *rankLast.add(nBitsToDecrease as usize) = noSymbol;
                        /* this rank is now empty */
                    }
                }
            } /* while (totalCost > 0) */

            /* If we've removed too much weight, then we have to add it back.
             * To avoid overshooting again, we only adjust the smallest rank.
             */
            while totalCost < 0 {
                /* Sometimes, cost correction overshoot */
                /* special case : no rank 1 symbol (using targetNbBits-1);
                 * let's create one from largest rank 0 (using targetNbBits).
                 */
                if *rankLast.add(1) == noSymbol {
                    while (*huffNode.offset(n as isize)).nbBits as U32 == targetNbBits {
                        n -= 1;
                    }
                    {
                        let p = huffNode.offset((n + 1) as isize);
                        (*p).nbBits = (*p).nbBits.wrapping_sub(1);
                    }
                    *rankLast.add(1) = (n + 1) as U32;
                    totalCost += 1;
                    continue;
                }
                {
                    let p = huffNode.wrapping_add((*rankLast.add(1)).wrapping_add(1) as usize);
                    (*p).nbBits = (*p).nbBits.wrapping_sub(1);
                }
                *rankLast.add(1) = (*rankLast.add(1)).wrapping_add(1);
                totalCost += 1;
            }
        } /* repay normalized cost */
    } /* there are several too large elements (at least >= 2) */

    targetNbBits
}

/*
 * typedef struct {
 *     U16 base;
 *     U16 curr;
 * } rankPos;
 */
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct rankPos {
    pub base: U16,
    pub curr: U16,
}

/* typedef nodeElt huffNodeTable[2 * (HUF_SYMBOLVALUE_MAX + 1)]; */
pub type huffNodeTable = [nodeElt; 2 * (HUF_SYMBOLVALUE_MAX as usize + 1)];

/* Number of buckets available for HUF_sort() */
/* #define RANK_POSITION_TABLE_SIZE 192 */
pub const RANK_POSITION_TABLE_SIZE: usize = 192;

/*
 * typedef struct {
 *   huffNodeTable huffNodeTbl;
 *   rankPos rankPosition[RANK_POSITION_TABLE_SIZE];
 * } HUF_buildCTable_wksp_tables;
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_buildCTable_wksp_tables {
    pub huffNodeTbl: huffNodeTable,
    pub rankPosition: [rankPos; RANK_POSITION_TABLE_SIZE],
}

/* #define RANK_POSITION_MAX_COUNT_LOG 32 */
pub const RANK_POSITION_MAX_COUNT_LOG: U32 = 32;
/* #define RANK_POSITION_LOG_BUCKETS_BEGIN ((RANK_POSITION_TABLE_SIZE - 1) - RANK_POSITION_MAX_COUNT_LOG - 1) */
pub const RANK_POSITION_LOG_BUCKETS_BEGIN: U32 =
    ((RANK_POSITION_TABLE_SIZE as U32 - 1) - RANK_POSITION_MAX_COUNT_LOG - 1); /* == 158 */
/* #define RANK_POSITION_DISTINCT_COUNT_CUTOFF (RANK_POSITION_LOG_BUCKETS_BEGIN + ZSTD_highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN)) */
#[inline(always)]
pub fn RANK_POSITION_DISTINCT_COUNT_CUTOFF() -> U32 {
    RANK_POSITION_LOG_BUCKETS_BEGIN.wrapping_add(ZSTD_highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN))
}

/* Return the appropriate bucket index for a given count. See definition of
 * RANK_POSITION_DISTINCT_COUNT_CUTOFF for explanation of bucketing strategy.
 */
pub fn HUF_getIndex(count: U32) -> U32 {
    if count < RANK_POSITION_DISTINCT_COUNT_CUTOFF() {
        count
    } else {
        ZSTD_highbit32(count).wrapping_add(RANK_POSITION_LOG_BUCKETS_BEGIN)
    }
}

/* Helper swap function for HUF_quickSortPartition() */
pub unsafe fn HUF_swapNodes(a: *mut nodeElt, b: *mut nodeElt) {
    let tmp: nodeElt = *a;
    *a = *b;
    *b = tmp;
}

/* Returns 0 if the huffNode array is not sorted by descending count */
pub unsafe fn HUF_isSorted(huffNode: *mut nodeElt, maxSymbolValue1: U32) -> core::ffi::c_int {
    let mut i: U32;
    i = 1;
    while i < maxSymbolValue1 {
        if (*huffNode.add(i as usize)).count > (*huffNode.add((i - 1) as usize)).count {
            return 0;
        }
        i = i.wrapping_add(1);
    }
    1
}

/* Insertion sort by descending order */
#[inline]
pub unsafe fn HUF_insertionSort(
    mut huffNode: *mut nodeElt,
    low: core::ffi::c_int,
    high: core::ffi::c_int,
) {
    let mut i: core::ffi::c_int;
    let size: core::ffi::c_int = high - low + 1;
    huffNode = huffNode.offset(low as isize);
    i = 1;
    while i < size {
        let key: nodeElt = *huffNode.offset(i as isize);
        let mut j: core::ffi::c_int = i - 1;
        while j >= 0 && (*huffNode.offset(j as isize)).count < key.count {
            *huffNode.offset((j + 1) as isize) = *huffNode.offset(j as isize);
            j -= 1;
        }
        *huffNode.offset((j + 1) as isize) = key;
        i += 1;
    }
}

/* Pivot helper function for quicksort. */
pub unsafe fn HUF_quickSortPartition(
    arr: *mut nodeElt,
    low: core::ffi::c_int,
    high: core::ffi::c_int,
) -> core::ffi::c_int {
    /* Simply select rightmost element as pivot. "Better" selectors like
     * median-of-three don't experimentally appear to have any benefit.
     */
    let pivot: U32 = (*arr.offset(high as isize)).count;
    let mut i: core::ffi::c_int = low - 1;
    let mut j: core::ffi::c_int = low;
    while j < high {
        if (*arr.offset(j as isize)).count > pivot {
            i += 1;
            HUF_swapNodes(arr.offset(i as isize), arr.offset(j as isize));
        }
        j += 1;
    }
    HUF_swapNodes(arr.offset((i + 1) as isize), arr.offset(high as isize));
    i + 1
}

/* Classic quicksort by descending with partially iterative calls
 * to reduce worst case callstack size.
 */
pub unsafe fn HUF_simpleQuickSort(
    arr: *mut nodeElt,
    mut low: core::ffi::c_int,
    mut high: core::ffi::c_int,
) {
    let kInsertionSortThreshold: core::ffi::c_int = 8;
    if high - low < kInsertionSortThreshold {
        HUF_insertionSort(arr, low, high);
        return;
    }
    while low < high {
        let idx: core::ffi::c_int = HUF_quickSortPartition(arr, low, high);
        if idx - low < high - idx {
            HUF_simpleQuickSort(arr, low, idx - 1);
            low = idx + 1;
        } else {
            HUF_simpleQuickSort(arr, idx + 1, high);
            high = idx - 1;
        }
    }
}

/**
 * HUF_sort():
 * Sorts the symbols [0, maxSymbolValue] by count[symbol] in decreasing order.
 */
pub unsafe fn HUF_sort(
    huffNode: *mut nodeElt,
    count: *const core::ffi::c_uint,
    maxSymbolValue: U32,
    rankPosition: *mut rankPos,
) {
    let mut n: U32;
    let maxSymbolValue1: U32 = maxSymbolValue.wrapping_add(1);

    /* Compute base and set curr to base. */
    ZSTD_memset(
        rankPosition as *mut u8,
        0,
        core::mem::size_of::<rankPos>() * RANK_POSITION_TABLE_SIZE,
    );
    n = 0;
    while n < maxSymbolValue1 {
        let lowerRank: U32 = HUF_getIndex(*count.add(n as usize));
        let p = rankPosition.add(lowerRank as usize);
        (*p).base = (*p).base.wrapping_add(1);
        n = n.wrapping_add(1);
    }

    /* Set up the rankPosition table */
    n = RANK_POSITION_TABLE_SIZE as U32 - 1;
    while n > 0 {
        let pprev = rankPosition.add((n - 1) as usize);
        let pcur = rankPosition.add(n as usize);
        (*pprev).base = (*pprev).base.wrapping_add((*pcur).base);
        (*pprev).curr = (*pprev).base;
        n = n.wrapping_sub(1);
    }

    /* Insert each symbol into their appropriate bucket, setting up rankPosition table. */
    n = 0;
    while n < maxSymbolValue1 {
        let c: U32 = *count.add(n as usize);
        let r: U32 = HUF_getIndex(c).wrapping_add(1);
        let p = rankPosition.add(r as usize);
        let pos: U32 = (*p).curr as U32;
        (*p).curr = (*p).curr.wrapping_add(1);
        (*huffNode.add(pos as usize)).count = c;
        (*huffNode.add(pos as usize)).byte = n as BYTE;
        n = n.wrapping_add(1);
    }

    /* Sort each bucket. */
    n = RANK_POSITION_DISTINCT_COUNT_CUTOFF();
    while n < RANK_POSITION_TABLE_SIZE as U32 - 1 {
        let p = rankPosition.add(n as usize);
        let bucketSize: core::ffi::c_int =
            ((*p).curr as core::ffi::c_int) - ((*p).base as core::ffi::c_int);
        let bucketStartIdx: U32 = (*p).base as U32;
        if bucketSize > 1 {
            HUF_simpleQuickSort(huffNode.add(bucketStartIdx as usize), 0, bucketSize - 1);
        }
        n = n.wrapping_add(1);
    }
}

/** HUF_buildCTable_wksp() :
 *  Same as HUF_buildCTable(), but using externally allocated scratch buffer.
 */
/* #define STARTNODE (HUF_SYMBOLVALUE_MAX+1) */
pub const STARTNODE: core::ffi::c_int = HUF_SYMBOLVALUE_MAX as core::ffi::c_int + 1;

/* HUF_buildTree():
 * Takes the huffNode array sorted by HUF_sort() and builds an unlimited-depth Huffman tree.
 */
pub unsafe fn HUF_buildTree(huffNode: *mut nodeElt, maxSymbolValue: U32) -> core::ffi::c_int {
    let huffNode0: *mut nodeElt = huffNode.wrapping_sub(1);
    let mut nonNullRank: core::ffi::c_int;
    let mut lowS: core::ffi::c_int;
    let mut lowN: core::ffi::c_int;
    let mut nodeNb: core::ffi::c_int = STARTNODE;
    let mut n: core::ffi::c_int;
    let nodeRoot: core::ffi::c_int;

    /* init for parents */
    nonNullRank = maxSymbolValue as core::ffi::c_int;
    while (*huffNode.offset(nonNullRank as isize)).count == 0 {
        nonNullRank -= 1;
    }
    lowS = nonNullRank;
    nodeRoot = nodeNb + lowS - 1;
    lowN = nodeNb;
    (*huffNode.offset(nodeNb as isize)).count = (*huffNode.offset(lowS as isize))
        .count
        .wrapping_add((*huffNode.offset((lowS - 1) as isize)).count);
    (*huffNode.offset((lowS - 1) as isize)).parent = nodeNb as U16;
    (*huffNode.offset(lowS as isize)).parent = nodeNb as U16;
    nodeNb += 1;
    lowS -= 2;
    n = nodeNb;
    while n <= nodeRoot {
        (*huffNode.offset(n as isize)).count = 1u32 << 30;
        n += 1;
    }
    (*huffNode0.offset(0)).count = 1u32 << 31; /* fake entry, strong barrier */

    /* create parents */
    while nodeNb <= nodeRoot {
        let n1: core::ffi::c_int =
            if (*huffNode.offset(lowS as isize)).count < (*huffNode.offset(lowN as isize)).count {
                let t = lowS;
                lowS -= 1;
                t
            } else {
                let t = lowN;
                lowN += 1;
                t
            };
        let n2: core::ffi::c_int =
            if (*huffNode.offset(lowS as isize)).count < (*huffNode.offset(lowN as isize)).count {
                let t = lowS;
                lowS -= 1;
                t
            } else {
                let t = lowN;
                lowN += 1;
                t
            };
        (*huffNode.offset(nodeNb as isize)).count = (*huffNode.offset(n1 as isize))
            .count
            .wrapping_add((*huffNode.offset(n2 as isize)).count);
        (*huffNode.offset(n2 as isize)).parent = nodeNb as U16;
        (*huffNode.offset(n1 as isize)).parent = nodeNb as U16;
        nodeNb += 1;
    }

    /* distribute weights (unlimited tree height) */
    (*huffNode.offset(nodeRoot as isize)).nbBits = 0;
    n = nodeRoot - 1;
    while n >= STARTNODE {
        (*huffNode.offset(n as isize)).nbBits = (*huffNode
            .offset((*huffNode.offset(n as isize)).parent as isize))
        .nbBits
        .wrapping_add(1);
        n -= 1;
    }
    n = 0;
    while n <= nonNullRank {
        (*huffNode.offset(n as isize)).nbBits = (*huffNode
            .offset((*huffNode.offset(n as isize)).parent as isize))
        .nbBits
        .wrapping_add(1);
        n += 1;
    }

    nonNullRank
}

/**
 * HUF_buildCTableFromTree():
 * Build the CTable given the Huffman tree in huffNode.
 */
pub unsafe fn HUF_buildCTableFromTree(
    CTable: *mut HUF_CElt,
    huffNode: *const nodeElt,
    nonNullRank: core::ffi::c_int,
    maxSymbolValue: U32,
    maxNbBits: U32,
) {
    let ct: *mut HUF_CElt = CTable.wrapping_add(1);
    /* fill result into ctable (val, nbBits) */
    let mut n: core::ffi::c_int;
    let mut nbPerRank_arr: [U16; HUF_TABLELOG_MAX as usize + 1] =
        [0; HUF_TABLELOG_MAX as usize + 1];
    let mut valPerRank_arr: [U16; HUF_TABLELOG_MAX as usize + 1] =
        [0; HUF_TABLELOG_MAX as usize + 1];
    let nbPerRank: *mut U16 = nbPerRank_arr.as_mut_ptr();
    let valPerRank: *mut U16 = valPerRank_arr.as_mut_ptr();
    let alphabetSize: core::ffi::c_int = maxSymbolValue.wrapping_add(1) as core::ffi::c_int;
    n = 0;
    while n <= nonNullRank {
        let p = nbPerRank.offset((*huffNode.offset(n as isize)).nbBits as isize);
        *p = (*p).wrapping_add(1);
        n += 1;
    }
    /* determine starting value per rank */
    {
        let mut min: U16 = 0;
        n = maxNbBits as core::ffi::c_int;
        while n > 0 {
            *valPerRank.offset(n as isize) = min; /* get starting value within each rank */
            min = min.wrapping_add(*nbPerRank.offset(n as isize));
            min >>= 1;
            n -= 1;
        }
    }
    n = 0;
    while n < alphabetSize {
        /* push nbBits per symbol, symbol order */
        HUF_setNbBits(
            ct.offset((*huffNode.offset(n as isize)).byte as isize),
            (*huffNode.offset(n as isize)).nbBits as usize,
        );
        n += 1;
    }
    n = 0;
    while n < alphabetSize {
        /* assign value within rank, symbol order */
        let p = valPerRank.add(HUF_getNbBits(*ct.offset(n as isize)));
        let v: U16 = *p;
        *p = v.wrapping_add(1);
        HUF_setValue(ct.offset(n as isize), v as usize);
        n += 1;
    }

    HUF_writeCTableHeader(CTable, maxNbBits, maxSymbolValue);
}

/// `size_t HUF_buildCTable_wksp(HUF_CElt* CTable, const unsigned* count, U32 maxSymbolValue,
///                              U32 maxNbBits, void* workSpace, size_t wkspSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_buildCTable_wksp(
    CTable: *mut HUF_CElt,
    count: *const core::ffi::c_uint,
    maxSymbolValue: U32,
    mut maxNbBits: U32,
    workSpace: *mut core::ffi::c_void,
    mut wkspSize: usize,
) -> usize {
    let wksp_tables: *mut HUF_buildCTable_wksp_tables = HUF_alignUpWorkspace(
        workSpace,
        &mut wkspSize,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_buildCTable_wksp_tables;
    let huffNode0: *mut nodeElt = addr_of_mut!((*wksp_tables).huffNodeTbl) as *mut nodeElt;
    let huffNode: *mut nodeElt = huffNode0.wrapping_add(1);
    let nonNullRank: core::ffi::c_int;

    /* safety checks */
    if wkspSize < core::mem::size_of::<HUF_buildCTable_wksp_tables>() {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    if maxNbBits == 0 {
        maxNbBits = HUF_TABLELOG_DEFAULT;
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    ZSTD_memset(
        huffNode0 as *mut u8,
        0,
        core::mem::size_of::<huffNodeTable>(),
    );

    /* sort, decreasing order */
    HUF_sort(
        huffNode,
        count,
        maxSymbolValue,
        addr_of_mut!((*wksp_tables).rankPosition) as *mut rankPos,
    );

    /* build tree */
    nonNullRank = HUF_buildTree(huffNode, maxSymbolValue);

    /* determine and enforce maxTableLog */
    maxNbBits = HUF_setMaxHeight(huffNode, nonNullRank as U32, maxNbBits);
    if maxNbBits > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_GENERIC); /* check fit into table */
    }

    HUF_buildCTableFromTree(CTable, huffNode, nonNullRank, maxSymbolValue, maxNbBits);

    maxNbBits as usize
}

/// `size_t HUF_estimateCompressedSize(const HUF_CElt* CTable, const unsigned* count,
///                                    unsigned maxSymbolValue)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_estimateCompressedSize(
    CTable: *const HUF_CElt,
    count: *const core::ffi::c_uint,
    maxSymbolValue: core::ffi::c_uint,
) -> usize {
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let mut nbBits: usize = 0;
    let mut s: core::ffi::c_int;
    s = 0;
    while s <= maxSymbolValue as core::ffi::c_int {
        nbBits = nbBits.wrapping_add(
            HUF_getNbBits(*ct.offset(s as isize))
                .wrapping_mul(*count.offset(s as isize) as usize),
        );
        s += 1;
    }
    nbBits >> 3
}

/// `int HUF_validateCTable(const HUF_CElt* CTable, const unsigned* count, unsigned maxSymbolValue)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_validateCTable(
    CTable: *const HUF_CElt,
    count: *const core::ffi::c_uint,
    maxSymbolValue: core::ffi::c_uint,
) -> core::ffi::c_int {
    let header: HUF_CTableHeader = HUF_readCTableHeader(CTable);
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let mut bad: core::ffi::c_int = 0;
    let mut s: core::ffi::c_int;

    if (header.maxSymbolValue as core::ffi::c_uint) < maxSymbolValue {
        return 0;
    }

    s = 0;
    while s <= maxSymbolValue as core::ffi::c_int {
        bad |= ((*count.offset(s as isize) != 0) as core::ffi::c_int)
            & ((HUF_getNbBits(*ct.offset(s as isize)) == 0) as core::ffi::c_int);
        s += 1;
    }
    (bad == 0) as core::ffi::c_int
}

/// `size_t HUF_compressBound(size_t size) { return HUF_COMPRESSBOUND(size); }`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compressBound(size: usize) -> usize {
    HUF_COMPRESSBOUND(size)
}

/** HUF_CStream_t:
 * Huffman uses its own BIT_CStream_t implementation.
 */
/* #define HUF_BITS_IN_CONTAINER (sizeof(size_t) * 8) */
pub const HUF_BITS_IN_CONTAINER: usize = core::mem::size_of::<usize>() * 8;

/*
 * typedef struct {
 *     size_t bitContainer[2];
 *     size_t bitPos[2];
 *
 *     BYTE* startPtr;
 *     BYTE* ptr;
 *     BYTE* endPtr;
 * } HUF_CStream_t;
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_CStream_t {
    pub bitContainer: [usize; 2],
    pub bitPos: [usize; 2],
    pub startPtr: *mut BYTE,
    pub ptr: *mut BYTE,
    pub endPtr: *mut BYTE,
}

impl Default for HUF_CStream_t {
    fn default() -> Self {
        HUF_CStream_t {
            bitContainer: [0, 0],
            bitPos: [0, 0],
            startPtr: core::ptr::null_mut(),
            ptr: core::ptr::null_mut(),
            endPtr: core::ptr::null_mut(),
        }
    }
}

/* HUF_initCStream():
 * Initializes the bitstream.
 * @returns 0 or an error code.
 */
pub unsafe fn HUF_initCStream(
    bitC: *mut HUF_CStream_t,
    startPtr: *mut core::ffi::c_void,
    dstCapacity: usize,
) -> usize {
    ZSTD_memset(
        bitC as *mut u8,
        0,
        core::mem::size_of::<HUF_CStream_t>(),
    );
    (*bitC).startPtr = startPtr as *mut BYTE;
    (*bitC).ptr = (*bitC).startPtr;
    (*bitC).endPtr = (*bitC)
        .startPtr
        .wrapping_add(dstCapacity)
        .wrapping_sub(core::mem::size_of::<usize>());
    if dstCapacity <= core::mem::size_of::<usize>() {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    0
}

/* HUF_addBits():
 * Adds the symbol stored in HUF_CElt elt to the bitstream.
 */
#[inline(always)]
pub unsafe fn HUF_addBits(
    bitC: *mut HUF_CStream_t,
    elt: HUF_CElt,
    idx: core::ffi::c_int,
    kFast: core::ffi::c_int,
) {
    /* This is efficient on x86-64 with BMI2 because shrx
     * only reads the low 6 bits of the register.
     */
    (*bitC).bitContainer[idx as usize] >>= HUF_getNbBits(elt);
    (*bitC).bitContainer[idx as usize] |= if kFast != 0 {
        HUF_getValueFast(elt)
    } else {
        HUF_getValue(elt)
    };
    /* We only read the low 8 bits of bitC->bitPos[idx] so it
     * doesn't matter that the high bits have noise from the value.
     */
    (*bitC).bitPos[idx as usize] =
        (*bitC).bitPos[idx as usize].wrapping_add(HUF_getNbBitsFast(elt));
}

#[inline(always)]
pub unsafe fn HUF_zeroIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[1] = 0;
    (*bitC).bitPos[1] = 0;
}

/* HUF_mergeIndex1() :
 * Merges the bit container @ index 1 into the bit container @ index 0
 * and zeros the bit container @ index 1.
 */
#[inline(always)]
pub unsafe fn HUF_mergeIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[0] >>= ((*bitC).bitPos[1] & 0xFF);
    (*bitC).bitContainer[0] |= (*bitC).bitContainer[1];
    (*bitC).bitPos[0] = (*bitC).bitPos[0].wrapping_add((*bitC).bitPos[1]);
}

/* HUF_flushBits() :
 * Flushes the bits in the bit container @ index 0.
 */
#[inline(always)]
pub unsafe fn HUF_flushBits(bitC: *mut HUF_CStream_t, kFast: core::ffi::c_int) {
    /* The upper bits of bitPos are noisy, so we must mask by 0xFF. */
    let nbBits: usize = (*bitC).bitPos[0] & 0xFF;
    let nbBytes: usize = nbBits >> 3;
    /* The top nbBits bits of bitContainer are the ones we need. */
    let bitContainer: usize = (*bitC).bitContainer[0] >> (HUF_BITS_IN_CONTAINER - nbBits);
    /* Mask bitPos to account for the bytes we consumed. */
    (*bitC).bitPos[0] &= 7;
    MEM_writeLEST((*bitC).ptr, bitContainer);
    (*bitC).ptr = (*bitC).ptr.wrapping_add(nbBytes);
    if kFast == 0 && (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
    /* bitContainer doesn't need to be modified because the leftover
     * bits are already the top bitPos bits.
     */
}

/* HUF_endMark()
 * @returns The Huffman stream end mark: A 1-bit value = 1.
 */
pub unsafe fn HUF_endMark() -> HUF_CElt {
    let mut endMark: HUF_CElt = 0;
    HUF_setNbBits(&mut endMark, 1);
    HUF_setValue(&mut endMark, 1);
    endMark
}

/* HUF_closeCStream() :
 *  @return Size of CStream, in bytes,
 *          or 0 if it could not fit into dstBuffer */
pub unsafe fn HUF_closeCStream(bitC: *mut HUF_CStream_t) -> usize {
    HUF_addBits(bitC, HUF_endMark(), /* idx */ 0, /* kFast */ 0);
    HUF_flushBits(bitC, /* kFast */ 0);
    {
        let nbBits: usize = (*bitC).bitPos[0] & 0xFF;
        if (*bitC).ptr >= (*bitC).endPtr {
            return 0; /* overflow detected */
        }
        return ((*bitC).ptr as usize).wrapping_sub((*bitC).startPtr as usize)
            + (nbBits > 0) as usize;
    }
}

#[inline(always)]
pub unsafe fn HUF_encodeSymbol(
    bitCPtr: *mut HUF_CStream_t,
    symbol: U32,
    CTable: *const HUF_CElt,
    idx: core::ffi::c_int,
    fast: core::ffi::c_int,
) {
    HUF_addBits(bitCPtr, *CTable.wrapping_add(symbol as usize), idx, fast);
}

#[inline(always)]
pub unsafe fn HUF_compress1X_usingCTable_internal_body_loop(
    bitC: *mut HUF_CStream_t,
    ip: *const BYTE,
    srcSize: usize,
    ct: *const HUF_CElt,
    kUnroll: core::ffi::c_int,
    kFastFlush: core::ffi::c_int,
    kLastFast: core::ffi::c_int,
) {
    /* Join to kUnroll */
    let mut n: core::ffi::c_int = srcSize as core::ffi::c_int;
    let mut rem: core::ffi::c_int = n % kUnroll;
    if rem > 0 {
        while rem > 0 {
            n -= 1;
            HUF_encodeSymbol(bitC, *ip.offset(n as isize) as U32, ct, 0, /* fast */ 0);
            rem -= 1;
        }
        HUF_flushBits(bitC, kFastFlush);
    }

    /* Join to 2 * kUnroll */
    if (n % (2 * kUnroll)) != 0 {
        let mut u: core::ffi::c_int;
        u = 1;
        while u < kUnroll {
            HUF_encodeSymbol(bitC, *ip.offset((n - u) as isize) as U32, ct, 0, 1);
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.offset((n - kUnroll) as isize) as U32,
            ct,
            0,
            kLastFast,
        );
        HUF_flushBits(bitC, kFastFlush);
        n -= kUnroll;
    }

    while n > 0 {
        /* Encode kUnroll symbols into the bitstream @ index 0. */
        let mut u: core::ffi::c_int;
        u = 1;
        while u < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.offset((n - u) as isize) as U32,
                ct,
                /* idx */ 0,
                /* fast */ 1,
            );
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.offset((n - kUnroll) as isize) as U32,
            ct,
            /* idx */ 0,
            /* fast */ kLastFast,
        );
        HUF_flushBits(bitC, kFastFlush);
        /* Encode kUnroll symbols into the bitstream @ index 1. */
        HUF_zeroIndex1(bitC);
        u = 1;
        while u < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.offset((n - kUnroll - u) as isize) as U32,
                ct,
                /* idx */ 1,
                /* fast */ 1,
            );
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.offset((n - kUnroll - kUnroll) as isize) as U32,
            ct,
            /* idx */ 1,
            /* fast */ kLastFast,
        );
        /* Merge bitstream @ index 1 into the bitstream @ index 0 */
        HUF_mergeIndex1(bitC);
        HUF_flushBits(bitC, kFastFlush);
        n -= 2 * kUnroll;
    }
}

/**
 * Returns a tight upper bound on the output space needed by Huffman
 * with 8 bytes buffer to handle over-writes.
 */
pub fn HUF_tightCompressBound(srcSize: usize, tableLog: usize) -> usize {
    ((srcSize.wrapping_mul(tableLog)) >> 3).wrapping_add(8)
}

#[inline(always)]
pub unsafe fn HUF_compress1X_usingCTable_internal_body(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
) -> usize {
    let tableLog: U32 = HUF_readCTableHeader(CTable).tableLog as U32;
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let ip: *const BYTE = src as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut bitC: HUF_CStream_t = HUF_CStream_t::default();

    /* init */
    if dstSize < 8 {
        return 0; /* not enough space to compress */
    }
    {
        let op: *mut BYTE = ostart;
        let initErr = HUF_initCStream(
            &mut bitC,
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
        );
        if HUF_isError(initErr) != 0 {
            return 0;
        }
    }

    if dstSize < HUF_tightCompressBound(srcSize, tableLog as usize) || tableLog > 11 {
        HUF_compress1X_usingCTable_internal_body_loop(
            &mut bitC,
            ip,
            srcSize,
            ct,
            /* kUnroll */ if MEM_32bits() != 0 { 2 } else { 4 },
            /* kFast */ 0,
            /* kLastFast */ 0,
        );
    } else {
        if MEM_32bits() != 0 {
            match tableLog {
                11 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 2,
                        /* kFastFlush */ 1, /* kLastFast */ 0,
                    );
                }
                10 | 9 | 8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 2,
                        /* kFastFlush */ 1, /* kLastFast */ 1,
                    );
                }
                /* case 7: ZSTD_FALLTHROUGH; default: */
                _ => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 3,
                        /* kFastFlush */ 1, /* kLastFast */ 1,
                    );
                }
            }
        } else {
            match tableLog {
                11 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 5,
                        /* kFastFlush */ 1, /* kLastFast */ 0,
                    );
                }
                10 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 5,
                        /* kFastFlush */ 1, /* kLastFast */ 1,
                    );
                }
                9 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 6,
                        /* kFastFlush */ 1, /* kLastFast */ 0,
                    );
                }
                8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 7,
                        /* kFastFlush */ 1, /* kLastFast */ 0,
                    );
                }
                7 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 8,
                        /* kFastFlush */ 1, /* kLastFast */ 0,
                    );
                }
                /* case 6: ZSTD_FALLTHROUGH; default: */
                _ => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, /* kUnroll */ 9,
                        /* kFastFlush */ 1, /* kLastFast */ 1,
                    );
                }
            }
        }
    }

    HUF_closeCStream(&mut bitC)
}

/* DYNAMIC_BMI2 == 0 : only the `#else` branch is compiled. */
pub unsafe fn HUF_compress1X_usingCTable_internal(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: core::ffi::c_int,
) -> usize {
    HUF_compress1X_usingCTable_internal_body(dst, dstSize, src, srcSize, CTable)
}

/// `size_t HUF_compress1X_usingCTable(void* dst, size_t dstSize, const void* src, size_t srcSize,
///                                    const HUF_CElt* CTable, int flags)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_usingCTable(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: core::ffi::c_int,
) -> usize {
    HUF_compress1X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

pub unsafe fn HUF_compress4X_usingCTable_internal(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: core::ffi::c_int,
) -> usize {
    let segmentSize: usize = (srcSize.wrapping_add(3)) / 4; /* first 3 segments */
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut op: *mut BYTE = ostart;

    if dstSize < 6 + 1 + 1 + 1 + 8 {
        return 0; /* minimum space to compress successfully */
    }
    if srcSize < 12 {
        return 0; /* no saving possible : too small input */
    }
    op = op.wrapping_add(6); /* jumpTable */

    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            ip as *const core::ffi::c_void,
            segmentSize,
            CTable,
            flags,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        MEM_writeLE16(ostart, cSize as U16);
        op = op.wrapping_add(cSize);
    }

    ip = ip.wrapping_add(segmentSize);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            ip as *const core::ffi::c_void,
            segmentSize,
            CTable,
            flags,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        MEM_writeLE16(ostart.wrapping_add(2), cSize as U16);
        op = op.wrapping_add(cSize);
    }

    ip = ip.wrapping_add(segmentSize);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            ip as *const core::ffi::c_void,
            segmentSize,
            CTable,
            flags,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        MEM_writeLE16(ostart.wrapping_add(4), cSize as U16);
        op = op.wrapping_add(cSize);
    }

    ip = ip.wrapping_add(segmentSize);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            ip as *const core::ffi::c_void,
            (iend as usize).wrapping_sub(ip as usize),
            CTable,
            flags,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        op = op.wrapping_add(cSize);
    }

    (op as usize).wrapping_sub(ostart as usize)
}

/// `size_t HUF_compress4X_usingCTable(void* dst, size_t dstSize, const void* src, size_t srcSize,
///                                    const HUF_CElt* CTable, int flags)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_usingCTable(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: core::ffi::c_int,
) -> usize {
    HUF_compress4X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

/* typedef enum { HUF_singleStream, HUF_fourStreams } HUF_nbStreams_e; */
pub type HUF_nbStreams_e = core::ffi::c_int;
pub const HUF_singleStream: HUF_nbStreams_e = 0;
pub const HUF_fourStreams: HUF_nbStreams_e = 1;

pub unsafe fn HUF_compressCTable_internal(
    ostart: *mut BYTE,
    mut op: *mut BYTE,
    oend: *mut BYTE,
    src: *const core::ffi::c_void,
    srcSize: usize,
    nbStreams: HUF_nbStreams_e,
    CTable: *const HUF_CElt,
    flags: core::ffi::c_int,
) -> usize {
    let cSize: usize = if nbStreams == HUF_singleStream {
        HUF_compress1X_usingCTable_internal(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            src,
            srcSize,
            CTable,
            flags,
        )
    } else {
        HUF_compress4X_usingCTable_internal(
            op as *mut core::ffi::c_void,
            (oend as usize).wrapping_sub(op as usize),
            src,
            srcSize,
            CTable,
            flags,
        )
    };
    if HUF_isError(cSize) != 0 {
        return cSize;
    }
    if cSize == 0 {
        return 0; /* uncompressible */
    }
    op = op.wrapping_add(cSize);
    /* check compressibility */
    if ((op as usize).wrapping_sub(ostart as usize)) >= srcSize.wrapping_sub(1) {
        return 0;
    }
    (op as usize).wrapping_sub(ostart as usize)
}

/*
 * typedef struct {
 *     unsigned count[HUF_SYMBOLVALUE_MAX + 1];
 *     HUF_CElt CTable[HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX)];
 *     union {
 *         HUF_buildCTable_wksp_tables buildCTable_wksp;
 *         HUF_WriteCTableWksp writeCTable_wksp;
 *         U32 hist_wksp[HIST_WKSP_SIZE_U32];
 *     } wksps;
 * } HUF_compress_tables_t;
 */
#[repr(C)]
pub union HUF_compress_tables_t_wksps {
    pub buildCTable_wksp: HUF_buildCTable_wksp_tables,
    pub writeCTable_wksp: HUF_WriteCTableWksp,
    pub hist_wksp: [U32; HIST_WKSP_SIZE_U32],
}

#[repr(C)]
pub struct HUF_compress_tables_t {
    pub count: [core::ffi::c_uint; HUF_SYMBOLVALUE_MAX as usize + 1],
    pub CTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX as usize)],
    pub wksps: HUF_compress_tables_t_wksps,
}

/* #define SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE 4096 */
pub const SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE: usize = 4096;
/* #define SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO 10 */
pub const SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO: usize = 10;

/// `unsigned HUF_cardinality(const unsigned* count, unsigned maxSymbolValue)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_cardinality(
    count: *const core::ffi::c_uint,
    maxSymbolValue: core::ffi::c_uint,
) -> core::ffi::c_uint {
    let mut cardinality: core::ffi::c_uint = 0;
    let mut i: core::ffi::c_uint;

    i = 0;
    while i < maxSymbolValue.wrapping_add(1) {
        if *count.add(i as usize) != 0 {
            cardinality = cardinality.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }

    cardinality
}

/// `unsigned HUF_minTableLog(unsigned symbolCardinality)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_minTableLog(
    symbolCardinality: core::ffi::c_uint,
) -> core::ffi::c_uint {
    /* `symbolCardinality == 0` makes the C `ZSTD_highbit32()` evaluate
     * `31 - __builtin_clz(0)`, which is undefined behaviour. At this site the
     * reference build folds it to a bare `bsr`, whose destination is left
     * untouched for a zero source, and observably yields 0 (so the function
     * returns 1). Reproduced verbatim; this input is unreachable from the
     * public compression path, which only reaches here with cardinality >= 2. */
    let minBitsSymbols: U32 = if symbolCardinality == 0 {
        1
    } else {
        ZSTD_highbit32(symbolCardinality).wrapping_add(1)
    };
    minBitsSymbols
}

/// `unsigned HUF_optimalTableLog(unsigned maxTableLog, size_t srcSize, unsigned maxSymbolValue,
///                               void* workSpace, size_t wkspSize, HUF_CElt* table,
///                               const unsigned* count, int flags)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_optimalTableLog(
    maxTableLog: core::ffi::c_uint,
    srcSize: usize,
    maxSymbolValue: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
    table: *mut HUF_CElt,
    count: *const core::ffi::c_uint,
    flags: core::ffi::c_int,
) -> core::ffi::c_uint {
    if (flags & HUF_flags_optimalDepth) == 0 {
        /* cheap evaluation, based on FSE */
        return crate::fse_compress::FSE_optimalTableLog_internal(
            maxTableLog,
            srcSize,
            maxSymbolValue,
            1,
        );
    }

    {
        let dst: *mut BYTE = (workSpace as *mut BYTE)
            .wrapping_add(core::mem::size_of::<HUF_WriteCTableWksp>());
        let dstSize: usize = wkspSize.wrapping_sub(core::mem::size_of::<HUF_WriteCTableWksp>());
        let mut hSize: usize = 0;
        let mut newSize: usize = 0;
        let symbolCardinality: core::ffi::c_uint = HUF_cardinality(count, maxSymbolValue);
        let minTableLog: core::ffi::c_uint = HUF_minTableLog(symbolCardinality);
        let mut optSize: usize = ((!0usize) - 1);
        let mut optLog: core::ffi::c_uint = maxTableLog;
        let mut optLogGuess: core::ffi::c_uint;

        /* Search until size increases */
        optLogGuess = minTableLog;
        while optLogGuess <= maxTableLog {
            {
                let maxBits: usize = HUF_buildCTable_wksp(
                    table,
                    count,
                    maxSymbolValue,
                    optLogGuess,
                    workSpace,
                    wkspSize,
                );
                if ERR_isError(maxBits) != 0 {
                    optLogGuess = optLogGuess.wrapping_add(1);
                    continue;
                }

                if maxBits < optLogGuess as usize && optLogGuess > minTableLog {
                    break;
                }

                hSize = HUF_writeCTable_wksp(
                    dst as *mut core::ffi::c_void,
                    dstSize,
                    table,
                    maxSymbolValue,
                    maxBits as U32,
                    workSpace,
                    wkspSize,
                );
            }

            if ERR_isError(hSize) != 0 {
                optLogGuess = optLogGuess.wrapping_add(1);
                continue;
            }

            newSize = HUF_estimateCompressedSize(table, count, maxSymbolValue).wrapping_add(hSize);

            if newSize > optSize.wrapping_add(1) {
                break;
            }

            if newSize < optSize {
                optSize = newSize;
                optLog = optLogGuess;
            }
            optLogGuess = optLogGuess.wrapping_add(1);
        }
        return optLog;
    }
}

/* HUF_compress_internal() :
 * `workSpace_align4` must be aligned on 4-bytes boundaries,
 * and occupies the same space as a table of HUF_WORKSPACE_SIZE_U64 unsigned */
pub unsafe fn HUF_compress_internal(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    mut maxSymbolValue: core::ffi::c_uint,
    mut huffLog: core::ffi::c_uint,
    nbStreams: HUF_nbStreams_e,
    workSpace: *mut core::ffi::c_void,
    mut wkspSize: usize,
    oldHufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: core::ffi::c_int,
) -> usize {
    let table: *mut HUF_compress_tables_t = HUF_alignUpWorkspace(
        workSpace,
        &mut wkspSize,
        core::mem::align_of::<usize>(),
    ) as *mut HUF_compress_tables_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut op: *mut BYTE = ostart;

    /* checks & inits */
    if wkspSize < core::mem::size_of::<HUF_compress_tables_t>() {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    if srcSize == 0 {
        return 0; /* Uncompressed */
    }
    if dstSize == 0 {
        return 0; /* cannot fit anything within dst budget */
    }
    if srcSize > HUF_BLOCKSIZE_MAX {
        return ERROR(ZSTD_error_srcSize_wrong); /* current block size limit */
    }
    if huffLog > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if maxSymbolValue == 0 {
        maxSymbolValue = HUF_SYMBOLVALUE_MAX;
    }
    if huffLog == 0 {
        huffLog = HUF_TABLELOG_DEFAULT;
    }

    /* Heuristic : If old table is valid, use it for small inputs */
    if (flags & HUF_flags_preferRepeat) != 0 && !repeat.is_null() && *repeat == HUF_repeat_valid {
        return HUF_compressCTable_internal(
            ostart,
            op,
            oend,
            src,
            srcSize,
            nbStreams,
            oldHufTable,
            flags,
        );
    }

    /* If uncompressible data is suspected, do a smaller sampling first */
    if (flags & HUF_flags_suspectUncompressible) != 0
        && srcSize >= (SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE * SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO)
    {
        let mut largestTotal: usize = 0;
        {
            let mut maxSymbolValueBegin: core::ffi::c_uint = maxSymbolValue;
            let largestBegin: usize = crate::hist::HIST_count_simple(
                addr_of_mut!((*table).count) as *mut core::ffi::c_uint,
                &mut maxSymbolValueBegin,
                src as *const BYTE as *const core::ffi::c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as usize;
            if ERR_isError(largestBegin) != 0 {
                return largestBegin;
            }
            largestTotal = largestTotal.wrapping_add(largestBegin);
        }
        {
            let mut maxSymbolValueEnd: core::ffi::c_uint = maxSymbolValue;
            let largestEnd: usize = crate::hist::HIST_count_simple(
                addr_of_mut!((*table).count) as *mut core::ffi::c_uint,
                &mut maxSymbolValueEnd,
                (src as *const BYTE)
                    .wrapping_add(srcSize)
                    .wrapping_sub(SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE)
                    as *const core::ffi::c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as usize;
            if ERR_isError(largestEnd) != 0 {
                return largestEnd;
            }
            largestTotal = largestTotal.wrapping_add(largestEnd);
        }
        if largestTotal <= (((2 * SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE) >> 7) + 4) {
            return 0; /* heuristic : probably not compressible enough */
        }
    }

    /* Scan input and build symbol stats */
    {
        let largest: usize = crate::hist::HIST_count_wksp(
            addr_of_mut!((*table).count) as *mut core::ffi::c_uint,
            &mut maxSymbolValue,
            src as *const BYTE as *const core::ffi::c_void,
            srcSize,
            addr_of_mut!((*table).wksps.hist_wksp) as *mut core::ffi::c_void,
            core::mem::size_of::<[U32; HIST_WKSP_SIZE_U32]>(),
        );
        if ERR_isError(largest) != 0 {
            return largest;
        }
        if largest == srcSize {
            /* single symbol, rle */
            *ostart = *(src as *const BYTE);
            return 1;
        }
        if largest <= (srcSize >> 7) + 4 {
            return 0; /* heuristic : probably not compressible enough */
        }
    }

    /* Check validity of previous table */
    if !repeat.is_null()
        && *repeat == HUF_repeat_check
        && HUF_validateCTable(
            oldHufTable,
            addr_of!((*table).count) as *const core::ffi::c_uint,
            maxSymbolValue,
        ) == 0
    {
        *repeat = HUF_repeat_none;
    }
    /* Heuristic : use existing table for small inputs */
    if (flags & HUF_flags_preferRepeat) != 0 && !repeat.is_null() && *repeat != HUF_repeat_none {
        return HUF_compressCTable_internal(
            ostart,
            op,
            oend,
            src,
            srcSize,
            nbStreams,
            oldHufTable,
            flags,
        );
    }

    /* Build Huffman Tree */
    huffLog = HUF_optimalTableLog(
        huffLog,
        srcSize,
        maxSymbolValue,
        addr_of_mut!((*table).wksps) as *mut core::ffi::c_void,
        core::mem::size_of::<HUF_compress_tables_t_wksps>(),
        addr_of_mut!((*table).CTable) as *mut HUF_CElt,
        addr_of!((*table).count) as *const core::ffi::c_uint,
        flags,
    );
    {
        let maxBits: usize = HUF_buildCTable_wksp(
            addr_of_mut!((*table).CTable) as *mut HUF_CElt,
            addr_of!((*table).count) as *const core::ffi::c_uint,
            maxSymbolValue,
            huffLog,
            addr_of_mut!((*table).wksps.buildCTable_wksp) as *mut core::ffi::c_void,
            core::mem::size_of::<HUF_buildCTable_wksp_tables>(),
        );
        {
            let err_code = maxBits;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        huffLog = maxBits as U32;
    }

    /* Write table description header */
    {
        let hSize: usize = HUF_writeCTable_wksp(
            op as *mut core::ffi::c_void,
            dstSize,
            addr_of!((*table).CTable) as *const HUF_CElt,
            maxSymbolValue,
            huffLog,
            addr_of_mut!((*table).wksps.writeCTable_wksp) as *mut core::ffi::c_void,
            core::mem::size_of::<HUF_WriteCTableWksp>(),
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        /* Check if using previous huffman table is beneficial */
        if !repeat.is_null() && *repeat != HUF_repeat_none {
            let oldSize: usize = HUF_estimateCompressedSize(
                oldHufTable,
                addr_of!((*table).count) as *const core::ffi::c_uint,
                maxSymbolValue,
            );
            let newSize: usize = HUF_estimateCompressedSize(
                addr_of!((*table).CTable) as *const HUF_CElt,
                addr_of!((*table).count) as *const core::ffi::c_uint,
                maxSymbolValue,
            );
            if oldSize <= hSize.wrapping_add(newSize) || hSize.wrapping_add(12) >= srcSize {
                return HUF_compressCTable_internal(
                    ostart,
                    op,
                    oend,
                    src,
                    srcSize,
                    nbStreams,
                    oldHufTable,
                    flags,
                );
            }
        }

        /* Use the new huffman table */
        if hSize.wrapping_add(12usize) >= srcSize {
            return 0;
        }
        op = op.wrapping_add(hSize);
        if !repeat.is_null() {
            *repeat = HUF_repeat_none;
        }
        if !oldHufTable.is_null() {
            /* Save new table */
            ZSTD_memcpy(
                oldHufTable as *mut u8,
                addr_of!((*table).CTable) as *const u8,
                core::mem::size_of::<[HUF_CElt; HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX as usize)]>(),
            );
        }
    }
    HUF_compressCTable_internal(
        ostart,
        op,
        oend,
        src,
        srcSize,
        nbStreams,
        addr_of!((*table).CTable) as *const HUF_CElt,
        flags,
    )
}

/// `size_t HUF_compress1X_repeat(void* dst, size_t dstSize, const void* src, size_t srcSize,
///                               unsigned maxSymbolValue, unsigned huffLog,
///                               void* workSpace, size_t wkspSize,
///                               HUF_CElt* hufTable, HUF_repeat* repeat, int flags)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_repeat(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    maxSymbolValue: core::ffi::c_uint,
    huffLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: core::ffi::c_int,
) -> usize {
    HUF_compress_internal(
        dst,
        dstSize,
        src,
        srcSize,
        maxSymbolValue,
        huffLog,
        HUF_singleStream,
        workSpace,
        wkspSize,
        hufTable,
        repeat,
        flags,
    )
}

/* HUF_compress4X_repeat():
 * compress input using 4 streams.
 * consider skipping quickly
 * reuse an existing huffman compression table */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_repeat(
    dst: *mut core::ffi::c_void,
    dstSize: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    maxSymbolValue: core::ffi::c_uint,
    huffLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: core::ffi::c_int,
) -> usize {
    HUF_compress_internal(
        dst,
        dstSize,
        src,
        srcSize,
        maxSymbolValue,
        huffLog,
        HUF_fourStreams,
        workSpace,
        wkspSize,
        hufTable,
        repeat,
        flags,
    )
}
