//! Translation of `compress/huf_compress.c` — Huffman encoder.
//!
//! Literal, semantics-preserving transliteration. Build configuration:
//! `DYNAMIC_BMI2=0`, no assembly, `DEBUGLEVEL 0` (asserts / DEBUGLOG dropped).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]

use core::ffi::{c_int, c_uint, c_void};

use crate::common::bits::*;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;

use crate::compress::hist::{HIST_count_simple, HIST_count_wksp, HIST_WKSP_SIZE_U32};

// FSE compressor functions (implemented by another agent in fse_compress.rs).
use crate::compress::fse_compress::{
    FSE_optimalTableLog, FSE_optimalTableLog_internal, FSE_normalizeCount, FSE_writeNCount,
    FSE_buildCTable_wksp, FSE_compress_usingCTable,
};

// HUF_readStats lives in entropy_common (implemented by another agent).
use crate::common::entropy_common::HUF_readStats;

/* **************************************************************
*  Required declarations
****************************************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct nodeElt {
    pub count: U32,
    pub parent: U16,
    pub byte: BYTE,
    pub nbBits: BYTE,
}

impl Default for nodeElt {
    fn default() -> Self {
        nodeElt { count: 0, parent: 0, byte: 0, nbBits: 0 }
    }
}

/* *******************************************************
*  HUF : Huffman block compression
*********************************************************/
pub const HUF_WORKSPACE_MAX_ALIGNMENT: size_t = 8;

pub unsafe fn HUF_alignUpWorkspace(
    workspace: *mut c_void,
    workspaceSizePtr: *mut size_t,
    align: size_t,
) -> *mut c_void {
    let mask: size_t = align - 1;
    let rem: size_t = (workspace as size_t) & mask;
    let add: size_t = (align - rem) & mask;
    let aligned: *mut BYTE = (workspace as *mut BYTE).wrapping_add(add);
    if *workspaceSizePtr >= add {
        *workspaceSizePtr -= add;
        aligned as *mut c_void
    } else {
        *workspaceSizePtr = 0;
        core::ptr::null_mut()
    }
}

/* HUF_compressWeights() :
 * Same as FSE_compress(), but dedicated to huff0's weights compression.
 */
pub const MAX_FSE_TABLELOG_FOR_HUFF_HEADER: u32 = 6;

#[repr(C)]
pub struct HUF_CompressWeightsWksp {
    pub CTable: [FSE_CTable;
        FSE_CTABLE_SIZE_U32(MAX_FSE_TABLELOG_FOR_HUFF_HEADER, HUF_TABLELOG_MAX) as usize],
    pub scratchBuffer: [U32;
        FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(HUF_TABLELOG_MAX, MAX_FSE_TABLELOG_FOR_HUFF_HEADER)
            as usize],
    pub count: [c_uint; (HUF_TABLELOG_MAX + 1) as usize],
    pub norm: [S16; (HUF_TABLELOG_MAX + 1) as usize],
}

pub unsafe fn HUF_compressWeights(
    dst: *mut c_void,
    dstSize: size_t,
    weightTable: *const c_void,
    wtSize: size_t,
    workspace: *mut c_void,
    mut workspaceSize: size_t,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);

    let mut maxSymbolValue: c_uint = HUF_TABLELOG_MAX;
    let mut tableLog: U32 = MAX_FSE_TABLELOG_FOR_HUFF_HEADER;
    let wksp: *mut HUF_CompressWeightsWksp = HUF_alignUpWorkspace(
        workspace,
        &mut workspaceSize,
        core::mem::align_of::<U32>() as size_t,
    ) as *mut HUF_CompressWeightsWksp;

    if workspaceSize < core::mem::size_of::<HUF_CompressWeightsWksp>() as size_t {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* init conditions */
    if wtSize <= 1 {
        return 0;
    } /* Not compressible */

    /* Scan input and build symbol stats */
    {
        let maxCount: c_uint = HIST_count_simple(
            (*wksp).count.as_mut_ptr(),
            &mut maxSymbolValue,
            weightTable,
            wtSize,
        ); /* never fails */
        if maxCount as size_t == wtSize {
            return 1;
        } /* only a single symbol in src : rle */
        if maxCount == 1 {
            return 0;
        } /* each symbol present maximum once => not compressible */
    }

    tableLog = FSE_optimalTableLog(tableLog, wtSize, maxSymbolValue);
    {
        let err = FSE_normalizeCount(
            (*wksp).norm.as_mut_ptr(),
            tableLog,
            (*wksp).count.as_ptr(),
            wtSize,
            maxSymbolValue,
            /* useLowProbCount */ 0,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }

    /* Write table description header */
    {
        let hSize = FSE_writeNCount(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            (*wksp).norm.as_ptr(),
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
        let err = FSE_buildCTable_wksp(
            (*wksp).CTable.as_mut_ptr(),
            (*wksp).norm.as_ptr(),
            maxSymbolValue,
            tableLog,
            (*wksp).scratchBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*wksp).scratchBuffer) as size_t,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let cSize = FSE_compress_usingCTable(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            weightTable,
            wtSize,
            (*wksp).CTable.as_ptr(),
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 {
            return 0;
        } /* not enough space for compressed data */
        op = op.wrapping_add(cSize);
    }

    (op as size_t).wrapping_sub(ostart as size_t)
}

pub unsafe fn HUF_getNbBits(elt: HUF_CElt) -> size_t {
    elt & 0xFF
}

pub unsafe fn HUF_getNbBitsFast(elt: HUF_CElt) -> size_t {
    elt
}

pub unsafe fn HUF_getValue(elt: HUF_CElt) -> size_t {
    elt & !(0xFF as size_t)
}

pub unsafe fn HUF_getValueFast(elt: HUF_CElt) -> size_t {
    elt
}

pub unsafe fn HUF_setNbBits(elt: *mut HUF_CElt, nbBits: size_t) {
    *elt = nbBits;
}

pub unsafe fn HUF_setValue(elt: *mut HUF_CElt, value: size_t) {
    let nbBits: size_t = HUF_getNbBits(*elt);
    if nbBits > 0 {
        *elt |= value << (core::mem::size_of::<HUF_CElt>() * 8 - nbBits as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTableHeader(ctable: *const HUF_CElt) -> HUF_CTableHeader {
    let mut header: HUF_CTableHeader = HUF_CTableHeader::default();
    ZSTD_memcpy(
        &mut header as *mut HUF_CTableHeader as *mut u8,
        ctable as *const u8,
        core::mem::size_of::<HUF_CTableHeader>() as size_t,
    );
    header
}

pub unsafe fn HUF_writeCTableHeader(ctable: *mut HUF_CElt, tableLog: U32, maxSymbolValue: U32) {
    let mut header: HUF_CTableHeader = HUF_CTableHeader::default();
    ZSTD_memset(
        &mut header as *mut HUF_CTableHeader as *mut u8,
        0,
        core::mem::size_of::<HUF_CTableHeader>() as size_t,
    );
    header.tableLog = tableLog as BYTE;
    header.maxSymbolValue = maxSymbolValue as BYTE;
    ZSTD_memcpy(
        ctable as *mut u8,
        &header as *const HUF_CTableHeader as *const u8,
        core::mem::size_of::<HUF_CTableHeader>() as size_t,
    );
}

#[repr(C)]
pub struct HUF_WriteCTableWksp {
    pub wksp: HUF_CompressWeightsWksp,
    pub bitsToWeight: [BYTE; (HUF_TABLELOG_MAX + 1) as usize],
    pub huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_writeCTable_wksp(
    dst: *mut c_void,
    maxDstSize: size_t,
    CTable: *const HUF_CElt,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    workspace: *mut c_void,
    mut workspaceSize: size_t,
) -> size_t {
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let op: *mut BYTE = dst as *mut BYTE;
    let mut n: U32;
    let wksp: *mut HUF_WriteCTableWksp = HUF_alignUpWorkspace(
        workspace,
        &mut workspaceSize,
        core::mem::align_of::<U32>() as size_t,
    ) as *mut HUF_WriteCTableWksp;

    /* check conditions */
    if workspaceSize < core::mem::size_of::<HUF_WriteCTableWksp>() as size_t {
        return ERROR(ZSTD_error_GENERIC);
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }

    /* convert to weight */
    (*wksp).bitsToWeight[0] = 0;
    n = 1;
    while n < huffLog + 1 {
        (*wksp).bitsToWeight[n as usize] = (huffLog + 1 - n) as BYTE;
        n += 1;
    }
    n = 0;
    while n < maxSymbolValue {
        (*wksp).huffWeight[n as usize] =
            (*wksp).bitsToWeight[HUF_getNbBits(*ct.wrapping_add(n as usize)) as usize];
        n += 1;
    }

    /* attempt weights compression by FSE */
    if maxDstSize < 1 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let hSize = HUF_compressWeights(
            op.wrapping_add(1) as *mut c_void,
            maxDstSize - 1,
            (*wksp).huffWeight.as_ptr() as *const c_void,
            maxSymbolValue as size_t,
            &mut (*wksp).wksp as *mut HUF_CompressWeightsWksp as *mut c_void,
            core::mem::size_of_val(&(*wksp).wksp) as size_t,
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        if ((hSize > 1) as c_int & (hSize < (maxSymbolValue / 2) as size_t) as c_int) != 0 {
            /* FSE compressed */
            *op.wrapping_add(0) = hSize as BYTE;
            return hSize + 1;
        }
    }

    /* write raw values as 4-bits (max : 15) */
    if maxSymbolValue > (256 - 128) {
        return ERROR(ZSTD_error_GENERIC);
    } /* should not happen : likely means source cannot be compressed */
    if ((maxSymbolValue + 1) / 2) as size_t + 1 > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    } /* not enough space within dst buffer */
    *op.wrapping_add(0) = (128 /*special case*/ + (maxSymbolValue - 1)) as BYTE;
    (*wksp).huffWeight[maxSymbolValue as usize] = 0; /* to be sure it doesn't cause msan issue in final combination */
    n = 0;
    while n < maxSymbolValue {
        *op.wrapping_add((n / 2) as usize + 1) = ((((*wksp).huffWeight[n as usize] as U32) << 4)
            + (*wksp).huffWeight[(n + 1) as usize] as U32)
            as BYTE;
        n += 2;
    }
    (((maxSymbolValue + 1) / 2) as size_t) + 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTable(
    CTable: *mut HUF_CElt,
    maxSymbolValuePtr: *mut c_uint,
    src: *const c_void,
    srcSize: size_t,
    hasZeroWeights: *mut c_uint,
) -> size_t {
    let mut huffWeight: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize] =
        [0; (HUF_SYMBOLVALUE_MAX + 1) as usize];
    let mut rankVal: [U32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize] =
        [0; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize];
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let ct: *mut HUF_CElt = CTable.wrapping_add(1);

    /* get symbol weights */
    let readSize = HUF_readStats(
        huffWeight.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as size_t,
        rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
    );
    if ERR_isError(readSize) != 0 {
        return readSize;
    }
    *hasZeroWeights = (rankVal[0] > 0) as c_uint;

    /* check result */
    if tableLog > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    if nbSymbols > *maxSymbolValuePtr + 1 {
        return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
    }

    *maxSymbolValuePtr = nbSymbols - 1;

    HUF_writeCTableHeader(CTable, tableLog, *maxSymbolValuePtr);

    /* Prepare base value per rank */
    {
        let mut nextRankStart: U32 = 0;
        let mut n: U32 = 1;
        while n <= tableLog {
            let curr: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(rankVal[n as usize] << (n - 1));
            rankVal[n as usize] = curr;
            n += 1;
        }
    }

    /* fill nbBits */
    {
        let mut n: U32 = 0;
        while n < nbSymbols {
            let w: U32 = huffWeight[n as usize] as U32;
            HUF_setNbBits(
                ct.wrapping_add(n as usize),
                (((tableLog + 1 - w) as BYTE) & (0u32.wrapping_sub((w != 0) as u32) as BYTE))
                    as size_t,
            );
            n += 1;
        }
    }

    /* fill val */
    {
        let mut nbPerRank: [U16; (HUF_TABLELOG_MAX + 2) as usize] =
            [0; (HUF_TABLELOG_MAX + 2) as usize]; /* support w=0=>n=tableLog+1 */
        let mut valPerRank: [U16; (HUF_TABLELOG_MAX + 2) as usize] =
            [0; (HUF_TABLELOG_MAX + 2) as usize];
        {
            let mut n: U32 = 0;
            while n < nbSymbols {
                nbPerRank[HUF_getNbBits(*ct.wrapping_add(n as usize)) as usize] += 1;
                n += 1;
            }
        }
        /* determine stating value per rank */
        valPerRank[(tableLog + 1) as usize] = 0; /* for w==0 */
        {
            let mut min: U16 = 0;
            let mut n: U32 = tableLog;
            while n > 0 {
                /* start at n=tablelog <-> w=1 */
                valPerRank[n as usize] = min; /* get starting value within each rank */
                min = min.wrapping_add(nbPerRank[n as usize]);
                min >>= 1;
                n -= 1;
            }
        }
        /* assign value within rank, symbol order */
        {
            let mut n: U32 = 0;
            while n < nbSymbols {
                let r = HUF_getNbBits(*ct.wrapping_add(n as usize)) as usize;
                HUF_setValue(ct.wrapping_add(n as usize), valPerRank[r] as size_t);
                valPerRank[r] = valPerRank[r].wrapping_add(1);
                n += 1;
            }
        }
    }

    readSize
}

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
 */
pub unsafe fn HUF_setMaxHeight(huffNode: *mut nodeElt, lastNonNull: U32, targetNbBits: U32) -> U32 {
    let largestBits: U32 = (*huffNode.wrapping_add(lastNonNull as usize)).nbBits as U32;
    /* early exit : no elt > targetNbBits, so the tree is already valid. */
    if largestBits <= targetNbBits {
        return largestBits;
    }

    /* there are several too large elements (at least >= 2) */
    {
        let mut totalCost: c_int = 0;
        let baseCost: U32 = 1u32 << (largestBits - targetNbBits);
        let mut n: c_int = lastNonNull as c_int;

        /* Adjust any ranks > targetNbBits to targetNbBits. */
        while (*huffNode.wrapping_add(n as usize)).nbBits as U32 > targetNbBits {
            totalCost += (baseCost
                - (1u32 << (largestBits - (*huffNode.wrapping_add(n as usize)).nbBits as U32)))
                as c_int;
            (*huffNode.wrapping_add(n as usize)).nbBits = targetNbBits as BYTE;
            n -= 1;
        }
        /* n stops at huffNode[n].nbBits <= targetNbBits */
        /* n end at index of smallest symbol using < targetNbBits */
        while (*huffNode.wrapping_add(n as usize)).nbBits as U32 == targetNbBits {
            n -= 1;
        }

        /* renorm totalCost from 2^largestBits to 2^targetNbBits */
        totalCost >>= (largestBits - targetNbBits);

        /* repay normalized cost */
        {
            let noSymbol: U32 = 0xF0F0F0F0;
            let mut rankLast: [U32; (HUF_TABLELOG_MAX + 2) as usize] =
                [0; (HUF_TABLELOG_MAX + 2) as usize];

            /* Get pos of last (smallest = lowest cum. count) symbol per rank */
            ZSTD_memset(
                rankLast.as_mut_ptr() as *mut u8,
                0xF0,
                core::mem::size_of_val(&rankLast) as size_t,
            );
            {
                let mut currentNbBits: U32 = targetNbBits;
                let mut pos: c_int = n;
                while pos >= 0 {
                    if (*huffNode.wrapping_add(pos as usize)).nbBits as U32 >= currentNbBits {
                        pos -= 1;
                        continue;
                    }
                    currentNbBits = (*huffNode.wrapping_add(pos as usize)).nbBits as U32; /* < targetNbBits */
                    rankLast[(targetNbBits - currentNbBits) as usize] = pos as U32;
                    pos -= 1;
                }
            }

            while totalCost > 0 {
                let mut nBitsToDecrease: U32 = ZSTD_highbit32(totalCost as U32) + 1;
                while nBitsToDecrease > 1 {
                    let highPos: U32 = rankLast[nBitsToDecrease as usize];
                    let lowPos: U32 = rankLast[(nBitsToDecrease - 1) as usize];
                    if highPos == noSymbol {
                        nBitsToDecrease -= 1;
                        continue;
                    }
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
                    nBitsToDecrease -= 1;
                }
                /* HUF_MAX_TABLELOG test just to please gcc 5+ */
                while (nBitsToDecrease <= HUF_TABLELOG_MAX)
                    && (rankLast[nBitsToDecrease as usize] == noSymbol)
                {
                    nBitsToDecrease += 1;
                }
                /* Increase the number of bits to gain back half the rank cost. */
                totalCost -= 1 << (nBitsToDecrease - 1);
                (*huffNode.wrapping_add(rankLast[nBitsToDecrease as usize] as usize)).nbBits += 1;

                /* Fix up the new rank. */
                if rankLast[(nBitsToDecrease - 1) as usize] == noSymbol {
                    rankLast[(nBitsToDecrease - 1) as usize] = rankLast[nBitsToDecrease as usize];
                }
                /* Fix up the old rank. */
                if rankLast[nBitsToDecrease as usize] == 0 {
                    /* special case, reached largest symbol */
                    rankLast[nBitsToDecrease as usize] = noSymbol;
                } else {
                    rankLast[nBitsToDecrease as usize] -= 1;
                    if (*huffNode.wrapping_add(rankLast[nBitsToDecrease as usize] as usize)).nbBits
                        as U32
                        != targetNbBits - nBitsToDecrease
                    {
                        rankLast[nBitsToDecrease as usize] = noSymbol; /* this rank is now empty */
                    }
                }
            } /* while (totalCost > 0) */

            /* If we've removed too much weight, then we have to add it back. */
            while totalCost < 0 {
                /* Sometimes, cost correction overshoot */
                if rankLast[1] == noSymbol {
                    while (*huffNode.wrapping_add(n as usize)).nbBits as U32 == targetNbBits {
                        n -= 1;
                    }
                    (*huffNode.wrapping_add((n + 1) as usize)).nbBits -= 1;
                    rankLast[1] = (n + 1) as U32;
                    totalCost += 1;
                    continue;
                }
                (*huffNode.wrapping_add((rankLast[1] + 1) as usize)).nbBits -= 1;
                rankLast[1] += 1;
                totalCost += 1;
            }
        } /* repay normalized cost */
    } /* there are several too large elements (at least >= 2) */

    targetNbBits
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct rankPos {
    pub base: U16,
    pub curr: U16,
}

pub type huffNodeTable = [nodeElt; 2 * (HUF_SYMBOLVALUE_MAX as usize + 1)];

/* Number of buckets available for HUF_sort() */
pub const RANK_POSITION_TABLE_SIZE: usize = 192;

#[repr(C)]
pub struct HUF_buildCTable_wksp_tables {
    pub huffNodeTbl: huffNodeTable,
    pub rankPosition: [rankPos; RANK_POSITION_TABLE_SIZE],
}

pub const RANK_POSITION_MAX_COUNT_LOG: u32 = 32;
/* == 158 */
pub const RANK_POSITION_LOG_BUCKETS_BEGIN: u32 =
    (RANK_POSITION_TABLE_SIZE as u32 - 1) - RANK_POSITION_MAX_COUNT_LOG - 1;

/* == 166 */
#[inline(always)]
pub fn RANK_POSITION_DISTINCT_COUNT_CUTOFF() -> u32 {
    RANK_POSITION_LOG_BUCKETS_BEGIN + ZSTD_highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN)
}

pub unsafe fn HUF_getIndex(count: U32) -> U32 {
    if count < RANK_POSITION_DISTINCT_COUNT_CUTOFF() {
        count
    } else {
        ZSTD_highbit32(count) + RANK_POSITION_LOG_BUCKETS_BEGIN
    }
}

/* Helper swap function for HUF_quickSortPartition() */
pub unsafe fn HUF_swapNodes(a: *mut nodeElt, b: *mut nodeElt) {
    let tmp: nodeElt = *a;
    *a = *b;
    *b = tmp;
}

/* Returns 0 if the huffNode array is not sorted by descending count */
pub unsafe fn HUF_isSorted(huffNode: *mut nodeElt, maxSymbolValue1: U32) -> c_int {
    let mut i: U32 = 1;
    while i < maxSymbolValue1 {
        if (*huffNode.wrapping_add(i as usize)).count
            > (*huffNode.wrapping_add((i - 1) as usize)).count
        {
            return 0;
        }
        i += 1;
    }
    1
}

/* Insertion sort by descending order */
pub unsafe fn HUF_insertionSort(huffNode: *mut nodeElt, low: c_int, high: c_int) {
    let mut i: c_int;
    let size: c_int = high - low + 1;
    let huffNode: *mut nodeElt = huffNode.wrapping_add(low as usize);
    i = 1;
    while i < size {
        let key: nodeElt = *huffNode.wrapping_add(i as usize);
        let mut j: c_int = i - 1;
        while j >= 0 && (*huffNode.wrapping_add(j as usize)).count < key.count {
            *huffNode.wrapping_add((j + 1) as usize) = *huffNode.wrapping_add(j as usize);
            j -= 1;
        }
        *huffNode.wrapping_add((j + 1) as usize) = key;
        i += 1;
    }
}

/* Pivot helper function for quicksort. */
pub unsafe fn HUF_quickSortPartition(arr: *mut nodeElt, low: c_int, high: c_int) -> c_int {
    let pivot: U32 = (*arr.wrapping_add(high as usize)).count;
    let mut i: c_int = low - 1;
    let mut j: c_int = low;
    while j < high {
        if (*arr.wrapping_add(j as usize)).count > pivot {
            i += 1;
            HUF_swapNodes(arr.wrapping_add(i as usize), arr.wrapping_add(j as usize));
        }
        j += 1;
    }
    HUF_swapNodes(arr.wrapping_add((i + 1) as usize), arr.wrapping_add(high as usize));
    i + 1
}

/* Classic quicksort by descending with partially iterative calls */
pub unsafe fn HUF_simpleQuickSort(arr: *mut nodeElt, mut low: c_int, mut high: c_int) {
    let kInsertionSortThreshold: c_int = 8;
    if high - low < kInsertionSortThreshold {
        HUF_insertionSort(arr, low, high);
        return;
    }
    while low < high {
        let idx: c_int = HUF_quickSortPartition(arr, low, high);
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
 */
pub unsafe fn HUF_sort(
    huffNode: *mut nodeElt,
    count: *const c_uint,
    maxSymbolValue: U32,
    rankPosition: *mut rankPos,
) {
    let mut n: U32;
    let maxSymbolValue1: U32 = maxSymbolValue + 1;

    ZSTD_memset(
        rankPosition as *mut u8,
        0,
        (core::mem::size_of::<rankPos>() * RANK_POSITION_TABLE_SIZE) as size_t,
    );
    n = 0;
    while n < maxSymbolValue1 {
        let lowerRank: U32 = HUF_getIndex(*count.wrapping_add(n as usize));
        (*rankPosition.wrapping_add(lowerRank as usize)).base += 1;
        n += 1;
    }

    /* Set up the rankPosition table */
    n = (RANK_POSITION_TABLE_SIZE as U32) - 1;
    while n > 0 {
        (*rankPosition.wrapping_add((n - 1) as usize)).base = (*rankPosition
            .wrapping_add((n - 1) as usize))
        .base
        .wrapping_add((*rankPosition.wrapping_add(n as usize)).base);
        (*rankPosition.wrapping_add((n - 1) as usize)).curr =
            (*rankPosition.wrapping_add((n - 1) as usize)).base;
        n -= 1;
    }

    /* Insert each symbol into their appropriate bucket. */
    n = 0;
    while n < maxSymbolValue1 {
        let c: U32 = *count.wrapping_add(n as usize);
        let r: U32 = HUF_getIndex(c) + 1;
        let pos: U32 = (*rankPosition.wrapping_add(r as usize)).curr as U32;
        (*rankPosition.wrapping_add(r as usize)).curr =
            (*rankPosition.wrapping_add(r as usize)).curr.wrapping_add(1);
        (*huffNode.wrapping_add(pos as usize)).count = c;
        (*huffNode.wrapping_add(pos as usize)).byte = n as BYTE;
        n += 1;
    }

    /* Sort each bucket. */
    n = RANK_POSITION_DISTINCT_COUNT_CUTOFF();
    while n < (RANK_POSITION_TABLE_SIZE as U32) - 1 {
        let bucketSize: c_int = (*rankPosition.wrapping_add(n as usize)).curr as c_int
            - (*rankPosition.wrapping_add(n as usize)).base as c_int;
        let bucketStartIdx: U32 = (*rankPosition.wrapping_add(n as usize)).base as U32;
        if bucketSize > 1 {
            HUF_simpleQuickSort(huffNode.wrapping_add(bucketStartIdx as usize), 0, bucketSize - 1);
        }
        n += 1;
    }
}

/** HUF_buildCTable_wksp() */
pub const STARTNODE: c_int = (HUF_SYMBOLVALUE_MAX + 1) as c_int;

/* HUF_buildTree() */
pub unsafe fn HUF_buildTree(huffNode: *mut nodeElt, maxSymbolValue: U32) -> c_int {
    let huffNode0: *mut nodeElt = huffNode.wrapping_sub(1);
    let mut nonNullRank: c_int;
    let mut lowS: c_int;
    let mut lowN: c_int;
    let mut nodeNb: c_int = STARTNODE;
    let mut n: c_int;
    let nodeRoot: c_int;
    /* init for parents */
    nonNullRank = maxSymbolValue as c_int;
    while (*huffNode.wrapping_add(nonNullRank as usize)).count == 0 {
        nonNullRank -= 1;
    }
    lowS = nonNullRank;
    nodeRoot = nodeNb + lowS - 1;
    lowN = nodeNb;
    (*huffNode.wrapping_add(nodeNb as usize)).count = (*huffNode.wrapping_add(lowS as usize))
        .count
        .wrapping_add((*huffNode.wrapping_add((lowS - 1) as usize)).count);
    (*huffNode.wrapping_add(lowS as usize)).parent = nodeNb as U16;
    (*huffNode.wrapping_add((lowS - 1) as usize)).parent = nodeNb as U16;
    nodeNb += 1;
    lowS -= 2;
    n = nodeNb;
    while n <= nodeRoot {
        (*huffNode.wrapping_add(n as usize)).count = 1u32 << 30;
        n += 1;
    }
    (*huffNode0.wrapping_add(0)).count = 1u32 << 31; /* fake entry, strong barrier */

    /* create parents */
    while nodeNb <= nodeRoot {
        let n1: c_int = if (*huffNode.wrapping_add(lowS as usize)).count
            < (*huffNode.wrapping_add(lowN as usize)).count
        {
            let t = lowS;
            lowS -= 1;
            t
        } else {
            let t = lowN;
            lowN += 1;
            t
        };
        let n2: c_int = if (*huffNode.wrapping_add(lowS as usize)).count
            < (*huffNode.wrapping_add(lowN as usize)).count
        {
            let t = lowS;
            lowS -= 1;
            t
        } else {
            let t = lowN;
            lowN += 1;
            t
        };
        (*huffNode.wrapping_add(nodeNb as usize)).count = (*huffNode.wrapping_add(n1 as usize))
            .count
            .wrapping_add((*huffNode.wrapping_add(n2 as usize)).count);
        (*huffNode.wrapping_add(n1 as usize)).parent = nodeNb as U16;
        (*huffNode.wrapping_add(n2 as usize)).parent = nodeNb as U16;
        nodeNb += 1;
    }

    /* distribute weights (unlimited tree height) */
    (*huffNode.wrapping_add(nodeRoot as usize)).nbBits = 0;
    n = nodeRoot - 1;
    while n >= STARTNODE {
        (*huffNode.wrapping_add(n as usize)).nbBits =
            (*huffNode.wrapping_add((*huffNode.wrapping_add(n as usize)).parent as usize)).nbBits
                + 1;
        n -= 1;
    }
    n = 0;
    while n <= nonNullRank {
        (*huffNode.wrapping_add(n as usize)).nbBits =
            (*huffNode.wrapping_add((*huffNode.wrapping_add(n as usize)).parent as usize)).nbBits
                + 1;
        n += 1;
    }

    nonNullRank
}

/**
 * HUF_buildCTableFromTree():
 */
pub unsafe fn HUF_buildCTableFromTree(
    CTable: *mut HUF_CElt,
    huffNode: *const nodeElt,
    nonNullRank: c_int,
    maxSymbolValue: U32,
    maxNbBits: U32,
) {
    let ct: *mut HUF_CElt = CTable.wrapping_add(1);
    /* fill result into ctable (val, nbBits) */
    let mut n: c_int;
    let mut nbPerRank: [U16; (HUF_TABLELOG_MAX + 1) as usize] = [0; (HUF_TABLELOG_MAX + 1) as usize];
    let mut valPerRank: [U16; (HUF_TABLELOG_MAX + 1) as usize] =
        [0; (HUF_TABLELOG_MAX + 1) as usize];
    let alphabetSize: c_int = (maxSymbolValue + 1) as c_int;
    n = 0;
    while n <= nonNullRank {
        nbPerRank[(*huffNode.wrapping_add(n as usize)).nbBits as usize] += 1;
        n += 1;
    }
    /* determine starting value per rank */
    {
        let mut min: U16 = 0;
        n = maxNbBits as c_int;
        while n > 0 {
            valPerRank[n as usize] = min; /* get starting value within each rank */
            min = min.wrapping_add(nbPerRank[n as usize]);
            min >>= 1;
            n -= 1;
        }
    }
    n = 0;
    while n < alphabetSize {
        HUF_setNbBits(
            ct.wrapping_add((*huffNode.wrapping_add(n as usize)).byte as usize),
            (*huffNode.wrapping_add(n as usize)).nbBits as size_t,
        ); /* push nbBits per symbol, symbol order */
        n += 1;
    }
    n = 0;
    while n < alphabetSize {
        let r = HUF_getNbBits(*ct.wrapping_add(n as usize)) as usize;
        HUF_setValue(ct.wrapping_add(n as usize), valPerRank[r] as size_t); /* assign value within rank, symbol order */
        valPerRank[r] = valPerRank[r].wrapping_add(1);
        n += 1;
    }

    HUF_writeCTableHeader(CTable, maxNbBits, maxSymbolValue);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_buildCTable_wksp(
    CTable: *mut HUF_CElt,
    count: *const c_uint,
    maxSymbolValue: U32,
    mut maxNbBits: U32,
    workSpace: *mut c_void,
    mut wkspSize: size_t,
) -> size_t {
    let wksp_tables: *mut HUF_buildCTable_wksp_tables = HUF_alignUpWorkspace(
        workSpace,
        &mut wkspSize,
        core::mem::align_of::<U32>() as size_t,
    ) as *mut HUF_buildCTable_wksp_tables;
    let huffNode0: *mut nodeElt = (*wksp_tables).huffNodeTbl.as_mut_ptr();
    let huffNode: *mut nodeElt = huffNode0.wrapping_add(1);
    let nonNullRank: c_int;

    /* safety checks */
    if wkspSize < core::mem::size_of::<HUF_buildCTable_wksp_tables>() as size_t {
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
        core::mem::size_of::<huffNodeTable>() as size_t,
    );

    /* sort, decreasing order */
    HUF_sort(
        huffNode,
        count,
        maxSymbolValue,
        (*wksp_tables).rankPosition.as_mut_ptr(),
    );

    /* build tree */
    nonNullRank = HUF_buildTree(huffNode, maxSymbolValue);

    /* determine and enforce maxTableLog */
    maxNbBits = HUF_setMaxHeight(huffNode, nonNullRank as U32, maxNbBits);
    if maxNbBits > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_GENERIC);
    } /* check fit into table */

    HUF_buildCTableFromTree(CTable, huffNode, nonNullRank, maxSymbolValue, maxNbBits);

    maxNbBits as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_estimateCompressedSize(
    CTable: *const HUF_CElt,
    count: *const c_uint,
    maxSymbolValue: c_uint,
) -> size_t {
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let mut nbBits: size_t = 0;
    let mut s: c_int = 0;
    while s <= maxSymbolValue as c_int {
        nbBits = nbBits.wrapping_add(
            HUF_getNbBits(*ct.wrapping_add(s as usize))
                .wrapping_mul(*count.wrapping_add(s as usize) as size_t),
        );
        s += 1;
    }
    nbBits >> 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_validateCTable(
    CTable: *const HUF_CElt,
    count: *const c_uint,
    maxSymbolValue: c_uint,
) -> c_int {
    let header: HUF_CTableHeader = HUF_readCTableHeader(CTable);
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let mut bad: c_int = 0;
    let mut s: c_int;

    if (header.maxSymbolValue as c_uint) < maxSymbolValue {
        return 0;
    }

    s = 0;
    while s <= maxSymbolValue as c_int {
        bad |= (*count.wrapping_add(s as usize) != 0) as c_int
            & (HUF_getNbBits(*ct.wrapping_add(s as usize)) == 0) as c_int;
        s += 1;
    }
    (bad == 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compressBound(size: size_t) -> size_t {
    HUF_COMPRESSBOUND(size)
}

/** HUF_CStream_t */
pub const HUF_BITS_IN_CONTAINER: size_t = (core::mem::size_of::<size_t>() * 8) as size_t;

#[repr(C)]
pub struct HUF_CStream_t {
    pub bitContainer: [size_t; 2],
    pub bitPos: [size_t; 2],
    pub startPtr: *mut BYTE,
    pub ptr: *mut BYTE,
    pub endPtr: *mut BYTE,
}

pub unsafe fn HUF_initCStream(
    bitC: *mut HUF_CStream_t,
    startPtr: *mut c_void,
    dstCapacity: size_t,
) -> size_t {
    ZSTD_memset(
        bitC as *mut u8,
        0,
        core::mem::size_of::<HUF_CStream_t>() as size_t,
    );
    (*bitC).startPtr = startPtr as *mut BYTE;
    (*bitC).ptr = (*bitC).startPtr;
    (*bitC).endPtr = (*bitC)
        .startPtr
        .wrapping_add(dstCapacity)
        .wrapping_sub(core::mem::size_of::<size_t>());
    if dstCapacity <= core::mem::size_of::<size_t>() as size_t {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    0
}

pub unsafe fn HUF_addBits(bitC: *mut HUF_CStream_t, elt: HUF_CElt, idx: c_int, kFast: c_int) {
    (*bitC).bitContainer[idx as usize] >>= HUF_getNbBits(elt);
    (*bitC).bitContainer[idx as usize] |= if kFast != 0 {
        HUF_getValueFast(elt)
    } else {
        HUF_getValue(elt)
    };
    (*bitC).bitPos[idx as usize] =
        (*bitC).bitPos[idx as usize].wrapping_add(HUF_getNbBitsFast(elt));
}

pub unsafe fn HUF_zeroIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[1] = 0;
    (*bitC).bitPos[1] = 0;
}

/*  HUF_mergeIndex1() */
pub unsafe fn HUF_mergeIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[0] >>= (*bitC).bitPos[1] & 0xFF;
    (*bitC).bitContainer[0] |= (*bitC).bitContainer[1];
    (*bitC).bitPos[0] = (*bitC).bitPos[0].wrapping_add((*bitC).bitPos[1]);
}

/*  HUF_flushBits() */
pub unsafe fn HUF_flushBits(bitC: *mut HUF_CStream_t, kFast: c_int) {
    let nbBits: size_t = (*bitC).bitPos[0] & 0xFF;
    let nbBytes: size_t = nbBits >> 3;
    let bitContainer: size_t = (*bitC).bitContainer[0] >> (HUF_BITS_IN_CONTAINER - nbBits);
    (*bitC).bitPos[0] &= 7;
    MEM_writeLEST((*bitC).ptr, bitContainer);
    (*bitC).ptr = (*bitC).ptr.wrapping_add(nbBytes);
    if kFast == 0 && (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
}

/*  HUF_endMark() */
pub unsafe fn HUF_endMark() -> HUF_CElt {
    let mut endMark: HUF_CElt = 0;
    HUF_setNbBits(&mut endMark, 1);
    HUF_setValue(&mut endMark, 1);
    endMark
}

/*  HUF_closeCStream() */
pub unsafe fn HUF_closeCStream(bitC: *mut HUF_CStream_t) -> size_t {
    HUF_addBits(bitC, HUF_endMark(), /* idx */ 0, /* kFast */ 0);
    HUF_flushBits(bitC, /* kFast */ 0);
    {
        let nbBits: size_t = (*bitC).bitPos[0] & 0xFF;
        if (*bitC).ptr >= (*bitC).endPtr {
            return 0;
        } /* overflow detected */
        (((*bitC).ptr as size_t).wrapping_sub((*bitC).startPtr as size_t))
            + (nbBits > 0) as size_t
    }
}

pub unsafe fn HUF_encodeSymbol(
    bitCPtr: *mut HUF_CStream_t,
    symbol: U32,
    CTable: *const HUF_CElt,
    idx: c_int,
    fast: c_int,
) {
    HUF_addBits(bitCPtr, *CTable.wrapping_add(symbol as usize), idx, fast);
}

pub unsafe fn HUF_compress1X_usingCTable_internal_body_loop(
    bitC: *mut HUF_CStream_t,
    ip: *const BYTE,
    srcSize: size_t,
    ct: *const HUF_CElt,
    kUnroll: c_int,
    kFastFlush: c_int,
    kLastFast: c_int,
) {
    /* Join to kUnroll */
    let mut n: c_int = srcSize as c_int;
    let mut rem: c_int = n % kUnroll;
    if rem > 0 {
        while rem > 0 {
            n -= 1;
            HUF_encodeSymbol(bitC, *ip.wrapping_add(n as usize) as U32, ct, 0, /* fast */ 0);
            rem -= 1;
        }
        HUF_flushBits(bitC, kFastFlush);
    }

    /* Join to 2 * kUnroll */
    if n % (2 * kUnroll) != 0 {
        let mut u: c_int = 1;
        while u < kUnroll {
            HUF_encodeSymbol(bitC, *ip.wrapping_add((n - u) as usize) as U32, ct, 0, 1);
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.wrapping_add((n - kUnroll) as usize) as U32,
            ct,
            0,
            kLastFast,
        );
        HUF_flushBits(bitC, kFastFlush);
        n -= kUnroll;
    }

    while n > 0 {
        /* Encode kUnroll symbols into the bitstream @ index 0. */
        let mut u: c_int = 1;
        while u < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.wrapping_add((n - u) as usize) as U32,
                ct,
                /* idx */ 0,
                /* fast */ 1,
            );
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.wrapping_add((n - kUnroll) as usize) as U32,
            ct,
            /* idx */ 0,
            /* fast */ kLastFast,
        );
        HUF_flushBits(bitC, kFastFlush);
        /* Encode kUnroll symbols into the bitstream @ index 1. */
        HUF_zeroIndex1(bitC);
        let mut u: c_int = 1;
        while u < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.wrapping_add((n - kUnroll - u) as usize) as U32,
                ct,
                /* idx */ 1,
                /* fast */ 1,
            );
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.wrapping_add((n - kUnroll - kUnroll) as usize) as U32,
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

pub unsafe fn HUF_tightCompressBound(srcSize: size_t, tableLog: size_t) -> size_t {
    ((srcSize * tableLog) >> 3) + 8
}

pub unsafe fn HUF_compress1X_usingCTable_internal_body(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    CTable: *const HUF_CElt,
) -> size_t {
    let tableLog: U32 = HUF_readCTableHeader(CTable).tableLog as U32;
    let ct: *const HUF_CElt = CTable.wrapping_add(1);
    let ip: *const BYTE = src as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut bitC: HUF_CStream_t = core::mem::zeroed();

    /* init */
    if dstSize < 8 {
        return 0;
    } /* not enough space to compress */
    {
        let op: *mut BYTE = ostart;
        let initErr = HUF_initCStream(
            &mut bitC,
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
        );
        if ERR_isError(initErr) != 0 {
            return 0;
        }
    }

    if dstSize < HUF_tightCompressBound(srcSize, tableLog as size_t) || tableLog > 11 {
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
                        &mut bitC, ip, srcSize, ct, 2, 1, 0,
                    );
                }
                10 | 9 | 8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 2, 1, 1,
                    );
                }
                _ => {
                    /* case 7, default */
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 3, 1, 1,
                    );
                }
            }
        } else {
            match tableLog {
                11 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 5, 1, 0,
                    );
                }
                10 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 5, 1, 1,
                    );
                }
                9 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 6, 1, 0,
                    );
                }
                8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 7, 1, 0,
                    );
                }
                7 => {
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 8, 1, 0,
                    );
                }
                _ => {
                    /* case 6, default */
                    HUF_compress1X_usingCTable_internal_body_loop(
                        &mut bitC, ip, srcSize, ct, 9, 1, 1,
                    );
                }
            }
        }
    }

    HUF_closeCStream(&mut bitC)
}

/* DYNAMIC_BMI2 == 0 : single body, flags threaded but unused. */
pub unsafe fn HUF_compress1X_usingCTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> size_t {
    let _ = flags;
    HUF_compress1X_usingCTable_internal_body(dst, dstSize, src, srcSize, CTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_usingCTable(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> size_t {
    HUF_compress1X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

pub unsafe fn HUF_compress4X_usingCTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> size_t {
    let segmentSize: size_t = (srcSize + 3) / 4; /* first 3 segments */
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut op: *mut BYTE = ostart;

    if dstSize < 6 + 1 + 1 + 1 + 8 {
        return 0;
    } /* minimum space to compress successfully */
    if srcSize < 12 {
        return 0;
    } /* no saving possible : too small input */
    op = op.wrapping_add(6); /* jumpTable */

    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            ip as *const c_void,
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
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            ip as *const c_void,
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
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            ip as *const c_void,
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
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            ip as *const c_void,
            (iend as size_t).wrapping_sub(ip as size_t),
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

    (op as size_t).wrapping_sub(ostart as size_t)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_usingCTable(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> size_t {
    HUF_compress4X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

pub type HUF_nbStreams_e = c_uint;
pub const HUF_singleStream: HUF_nbStreams_e = 0;
pub const HUF_fourStreams: HUF_nbStreams_e = 1;

pub unsafe fn HUF_compressCTable_internal(
    ostart: *mut BYTE,
    mut op: *mut BYTE,
    oend: *mut BYTE,
    src: *const c_void,
    srcSize: size_t,
    nbStreams: HUF_nbStreams_e,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> size_t {
    let cSize: size_t = if nbStreams == HUF_singleStream {
        HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            src,
            srcSize,
            CTable,
            flags,
        )
    } else {
        HUF_compress4X_usingCTable_internal(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            src,
            srcSize,
            CTable,
            flags,
        )
    };
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    if cSize == 0 {
        return 0;
    } /* uncompressible */
    op = op.wrapping_add(cSize);
    /* check compressibility */
    if (op as size_t).wrapping_sub(ostart as size_t) >= srcSize - 1 {
        return 0;
    }
    (op as size_t).wrapping_sub(ostart as size_t)
}

#[repr(C)]
pub union HUF_compress_tables_t_wksps {
    pub buildCTable_wksp: core::mem::ManuallyDrop<HUF_buildCTable_wksp_tables>,
    pub writeCTable_wksp: core::mem::ManuallyDrop<HUF_WriteCTableWksp>,
    pub hist_wksp: [U32; HIST_WKSP_SIZE_U32],
}

#[repr(C)]
pub struct HUF_compress_tables_t {
    pub count: [c_uint; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    pub CTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX) as usize],
    pub wksps: HUF_compress_tables_t_wksps,
}

pub const SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE: size_t = 4096;
pub const SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO: size_t = 10; /* Must be >= 2 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_cardinality(count: *const c_uint, maxSymbolValue: c_uint) -> c_uint {
    let mut cardinality: c_uint = 0;
    let mut i: c_uint = 0;
    while i < maxSymbolValue + 1 {
        if *count.wrapping_add(i as usize) != 0 {
            cardinality += 1;
        }
        i += 1;
    }
    cardinality
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_minTableLog(symbolCardinality: c_uint) -> c_uint {
    let minBitsSymbols: U32 = ZSTD_highbit32(symbolCardinality) + 1;
    minBitsSymbols
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_optimalTableLog(
    maxTableLog: c_uint,
    srcSize: size_t,
    maxSymbolValue: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
    table: *mut HUF_CElt,
    count: *const c_uint,
    flags: c_int,
) -> c_uint {
    if (flags & HUF_flags_optimalDepth as c_int) == 0 {
        /* cheap evaluation, based on FSE */
        return FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 1);
    }

    {
        let dst: *mut BYTE = (workSpace as *mut BYTE)
            .wrapping_add(core::mem::size_of::<HUF_WriteCTableWksp>());
        let dstSize: size_t = wkspSize - core::mem::size_of::<HUF_WriteCTableWksp>() as size_t;
        let mut hSize: size_t;
        let mut newSize: size_t;
        let symbolCardinality: c_uint = HUF_cardinality(count, maxSymbolValue);
        let minTableLog: c_uint = HUF_minTableLog(symbolCardinality);
        let mut optSize: size_t = (!(0 as size_t)) - 1;
        let mut optLog: c_uint = maxTableLog;
        let mut optLogGuess: c_uint;

        /* Search until size increases */
        optLogGuess = minTableLog;
        while optLogGuess <= maxTableLog {
            {
                let maxBits: size_t = HUF_buildCTable_wksp(
                    table,
                    count,
                    maxSymbolValue,
                    optLogGuess,
                    workSpace,
                    wkspSize,
                );
                if ERR_isError(maxBits) != 0 {
                    optLogGuess += 1;
                    continue;
                }

                if (maxBits as c_uint) < optLogGuess && optLogGuess > minTableLog {
                    break;
                }

                hSize = HUF_writeCTable_wksp(
                    dst as *mut c_void,
                    dstSize,
                    table,
                    maxSymbolValue,
                    maxBits as U32,
                    workSpace,
                    wkspSize,
                );
            }

            if ERR_isError(hSize) != 0 {
                optLogGuess += 1;
                continue;
            }

            newSize = HUF_estimateCompressedSize(table, count, maxSymbolValue) + hSize;

            if newSize > optSize + 1 {
                break;
            }

            if newSize < optSize {
                optSize = newSize;
                optLog = optLogGuess;
            }

            optLogGuess += 1;
        }
        optLog
    }
}

/* HUF_compress_internal() */
pub unsafe fn HUF_compress_internal(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    mut maxSymbolValue: c_uint,
    mut huffLog: c_uint,
    nbStreams: HUF_nbStreams_e,
    workSpace: *mut c_void,
    mut wkspSize: size_t,
    oldHufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: c_int,
) -> size_t {
    let table: *mut HUF_compress_tables_t = HUF_alignUpWorkspace(
        workSpace,
        &mut wkspSize,
        core::mem::align_of::<size_t>() as size_t,
    ) as *mut HUF_compress_tables_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut op: *mut BYTE = ostart;

    /* checks & inits */
    if wkspSize < core::mem::size_of::<HUF_compress_tables_t>() as size_t {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    if srcSize == 0 {
        return 0;
    } /* Uncompressed */
    if dstSize == 0 {
        return 0;
    } /* cannot fit anything within dst budget */
    if srcSize > HUF_BLOCKSIZE_MAX {
        return ERROR(ZSTD_error_srcSize_wrong);
    } /* current block size limit */
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
    if (flags & HUF_flags_preferRepeat as c_int) != 0
        && !repeat.is_null()
        && *repeat == HUF_repeat_valid
    {
        return HUF_compressCTable_internal(
            ostart, op, oend, src, srcSize, nbStreams, oldHufTable, flags,
        );
    }

    /* If uncompressible data is suspected, do a smaller sampling first */
    if (flags & HUF_flags_suspectUncompressible as c_int) != 0
        && srcSize >= (SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE * SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO)
    {
        let mut largestTotal: size_t = 0;
        {
            let mut maxSymbolValueBegin: c_uint = maxSymbolValue;
            let largestBegin: size_t = HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut maxSymbolValueBegin,
                src as *const BYTE as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as size_t;
            largestTotal += largestBegin;
        }
        {
            let mut maxSymbolValueEnd: c_uint = maxSymbolValue;
            let largestEnd: size_t = HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut maxSymbolValueEnd,
                (src as *const BYTE)
                    .wrapping_add(srcSize)
                    .wrapping_sub(SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE)
                    as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as size_t;
            largestTotal += largestEnd;
        }
        if largestTotal <= ((2 * SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE) >> 7) + 4 {
            return 0;
        } /* heuristic : probably not compressible enough */
    }

    /* Scan input and build symbol stats */
    {
        let largest: size_t = HIST_count_wksp(
            (*table).count.as_mut_ptr(),
            &mut maxSymbolValue,
            src as *const BYTE as *const c_void,
            srcSize,
            (*table).wksps.hist_wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.hist_wksp) as size_t,
        );
        if ERR_isError(largest) != 0 {
            return largest;
        }
        if largest == srcSize {
            *ostart = *(src as *const BYTE);
            return 1;
        } /* single symbol, rle */
        if largest <= (srcSize >> 7) + 4 {
            return 0;
        } /* heuristic : probably not compressible enough */
    }

    /* Check validity of previous table */
    if !repeat.is_null()
        && *repeat == HUF_repeat_check
        && HUF_validateCTable(oldHufTable, (*table).count.as_ptr(), maxSymbolValue) == 0
    {
        *repeat = HUF_repeat_none;
    }
    /* Heuristic : use existing table for small inputs */
    if (flags & HUF_flags_preferRepeat as c_int) != 0
        && !repeat.is_null()
        && *repeat != HUF_repeat_none
    {
        return HUF_compressCTable_internal(
            ostart, op, oend, src, srcSize, nbStreams, oldHufTable, flags,
        );
    }

    /* Build Huffman Tree */
    huffLog = HUF_optimalTableLog(
        huffLog,
        srcSize,
        maxSymbolValue,
        &mut (*table).wksps as *mut HUF_compress_tables_t_wksps as *mut c_void,
        core::mem::size_of_val(&(*table).wksps) as size_t,
        (*table).CTable.as_mut_ptr(),
        (*table).count.as_ptr(),
        flags,
    );
    {
        let maxBits: size_t = HUF_buildCTable_wksp(
            (*table).CTable.as_mut_ptr(),
            (*table).count.as_ptr(),
            maxSymbolValue,
            huffLog,
            &mut (*table).wksps.buildCTable_wksp as *mut core::mem::ManuallyDrop<
                HUF_buildCTable_wksp_tables,
            > as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.buildCTable_wksp) as size_t,
        );
        if ERR_isError(maxBits) != 0 {
            return maxBits;
        }
        huffLog = maxBits as U32;
    }

    /* Write table description header */
    {
        let hSize: size_t = HUF_writeCTable_wksp(
            op as *mut c_void,
            dstSize,
            (*table).CTable.as_ptr(),
            maxSymbolValue,
            huffLog,
            &mut (*table).wksps.writeCTable_wksp
                as *mut core::mem::ManuallyDrop<HUF_WriteCTableWksp>
                as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.writeCTable_wksp) as size_t,
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        /* Check if using previous huffman table is beneficial */
        if !repeat.is_null() && *repeat != HUF_repeat_none {
            let oldSize: size_t =
                HUF_estimateCompressedSize(oldHufTable, (*table).count.as_ptr(), maxSymbolValue);
            let newSize: size_t = HUF_estimateCompressedSize(
                (*table).CTable.as_ptr(),
                (*table).count.as_ptr(),
                maxSymbolValue,
            );
            if oldSize <= hSize + newSize || hSize + 12 >= srcSize {
                return HUF_compressCTable_internal(
                    ostart, op, oend, src, srcSize, nbStreams, oldHufTable, flags,
                );
            }
        }

        /* Use the new huffman table */
        if hSize + 12 >= srcSize {
            return 0;
        }
        op = op.wrapping_add(hSize);
        if !repeat.is_null() {
            *repeat = HUF_repeat_none;
        }
        if !oldHufTable.is_null() {
            ZSTD_memcpy(
                oldHufTable as *mut u8,
                (*table).CTable.as_ptr() as *const u8,
                core::mem::size_of_val(&(*table).CTable) as size_t,
            ); /* Save new table */
        }
    }
    HUF_compressCTable_internal(
        ostart,
        op,
        oend,
        src,
        srcSize,
        nbStreams,
        (*table).CTable.as_ptr(),
        flags,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_repeat(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: c_int,
) -> size_t {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_repeat(
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: c_int,
) -> size_t {
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
