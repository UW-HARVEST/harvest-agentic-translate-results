//! Translation of `compress/huf_compress.c`
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::ZSTD_highbit32;
use crate::bitstream::*;
use crate::cmem::*;
use crate::compress::fse_compress::{
    FSE_buildCTable_wksp, FSE_compress_usingCTable, FSE_normalizeCount, FSE_optimalTableLog,
    FSE_optimalTableLog_internal, FSE_writeNCount,
};
use crate::compress::hist::{HIST_WKSP_SIZE_U32, HIST_count_simple, HIST_count_wksp};
use crate::entropy_common::HUF_readStats;
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;

/* **************************************************************
*  Error Management
****************************************************************/
/* #define HUF_isError ERR_isError */
#[inline(always)]
fn HUF_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

/* **************************************************************
*  Required declarations
****************************************************************/
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct nodeElt {
    pub count: U32,
    pub parent: U16,
    pub byte: BYTE,
    pub nbBits: BYTE,
}

/* *******************************************************
*  HUF : Huffman block compression
*********************************************************/
const HUF_WORKSPACE_MAX_ALIGNMENT: usize = 8;

unsafe fn HUF_alignUpWorkspace(
    workspace: *mut c_void,
    workspaceSizePtr: *mut usize,
    align: usize,
) -> *mut c_void {
    let mask: usize = align - 1;
    let rem: usize = (workspace as usize) & mask;
    let add: usize = (align - rem) & mask;
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
 * The use case needs much less stack memory.
 * Note : all elements within weightTable are supposed to be <= HUF_TABLELOG_MAX.
 */
const MAX_FSE_TABLELOG_FOR_HUFF_HEADER: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_CompressWeightsWksp {
    pub CTable: [FSE_CTable; FSE_CTABLE_SIZE_U32(MAX_FSE_TABLELOG_FOR_HUFF_HEADER, HUF_TABLELOG_MAX)],
    pub scratchBuffer: [U32; FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(
        HUF_TABLELOG_MAX,
        MAX_FSE_TABLELOG_FOR_HUFF_HEADER,
    )],
    pub count: [c_uint; HUF_TABLELOG_MAX as usize + 1],
    pub norm: [S16; HUF_TABLELOG_MAX as usize + 1],
}

unsafe fn HUF_compressWeights(
    dst: *mut c_void,
    dstSize: usize,
    weightTable: *const c_void,
    wtSize: usize,
    workspace: *mut c_void,
    workspaceSize: usize,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);

    let mut maxSymbolValue: c_uint = HUF_TABLELOG_MAX;
    let mut tableLog: U32 = MAX_FSE_TABLELOG_FOR_HUFF_HEADER;
    let mut workspaceSize = workspaceSize;
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
        let maxCount: c_uint = HIST_count_simple(
            (*wksp).count.as_mut_ptr(),
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

    tableLog = FSE_optimalTableLog(tableLog, wtSize, maxSymbolValue);
    {
        let _var_err__ = FSE_normalizeCount(
            (*wksp).norm.as_mut_ptr(),
            tableLog,
            (*wksp).count.as_ptr(),
            wtSize,
            maxSymbolValue,
            /* useLowProbCount */ 0,
        );
        if ERR_isError(_var_err__) != 0 {
            return _var_err__;
        }
    }

    /* Write table description header */
    {
        let hSize = FSE_writeNCount(
            op as *mut c_void,
            (oend as usize) - (op as usize),
            (*wksp).norm.as_ptr(),
            maxSymbolValue,
            tableLog,
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        op = op.add(hSize);
    }

    /* Compress */
    {
        let _var_err__ = FSE_buildCTable_wksp(
            (*wksp).CTable.as_mut_ptr(),
            (*wksp).norm.as_ptr(),
            maxSymbolValue,
            tableLog,
            (*wksp).scratchBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*wksp).scratchBuffer),
        );
        if ERR_isError(_var_err__) != 0 {
            return _var_err__;
        }
    }
    {
        let cSize = FSE_compress_usingCTable(
            op as *mut c_void,
            (oend as usize) - (op as usize),
            weightTable,
            wtSize,
            (*wksp).CTable.as_ptr(),
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 {
            return 0; /* not enough space for compressed data */
        }
        op = op.add(cSize);
    }

    (op as usize) - (ostart as usize)
}

unsafe fn HUF_getNbBits(elt: HUF_CElt) -> usize {
    elt & 0xFF
}

unsafe fn HUF_getNbBitsFast(elt: HUF_CElt) -> usize {
    elt
}

unsafe fn HUF_getValue(elt: HUF_CElt) -> usize {
    elt & !(0xFFusize)
}

unsafe fn HUF_getValueFast(elt: HUF_CElt) -> usize {
    elt
}

unsafe fn HUF_setNbBits(elt: *mut HUF_CElt, nbBits: usize) {
    *elt = nbBits;
}

unsafe fn HUF_setValue(elt: *mut HUF_CElt, value: usize) {
    let nbBits: usize = HUF_getNbBits(*elt);
    if nbBits > 0 {
        *elt |= value << (core::mem::size_of::<HUF_CElt>() * 8 - nbBits);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTableHeader(ctable: *const HUF_CElt) -> HUF_CTableHeader {
    let mut header: HUF_CTableHeader = HUF_CTableHeader::default();
    ZSTD_memcpy(
        &mut header as *mut HUF_CTableHeader as *mut c_void,
        ctable as *const c_void,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
    header
}

unsafe fn HUF_writeCTableHeader(ctable: *mut HUF_CElt, tableLog: U32, maxSymbolValue: U32) {
    let mut header: HUF_CTableHeader = HUF_CTableHeader::default();
    ZSTD_memset(
        &mut header as *mut HUF_CTableHeader as *mut c_void,
        0,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
    header.tableLog = tableLog as BYTE;
    header.maxSymbolValue = maxSymbolValue as BYTE;
    ZSTD_memcpy(
        ctable as *mut c_void,
        &header as *const HUF_CTableHeader as *const c_void,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_WriteCTableWksp {
    pub wksp: HUF_CompressWeightsWksp,
    /* precomputed conversion table */
    pub bitsToWeight: [BYTE; HUF_TABLELOG_MAX as usize + 1],
    pub huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_writeCTable_wksp(
    dst: *mut c_void,
    maxDstSize: usize,
    CTable: *const HUF_CElt,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    workspace: *mut c_void,
    workspaceSize: usize,
) -> usize {
    let ct: *const HUF_CElt = CTable.add(1);
    let op: *mut BYTE = dst as *mut BYTE;
    let mut n: U32;
    let mut workspaceSize = workspaceSize;
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

    /* convert to weight */
    (*wksp).bitsToWeight[0] = 0;
    n = 1;
    while n < huffLog + 1 {
        *(*wksp).bitsToWeight.as_mut_ptr().add(n as usize) = (huffLog + 1 - n) as BYTE;
        n += 1;
    }
    n = 0;
    while n < maxSymbolValue {
        *(*wksp).huffWeight.as_mut_ptr().add(n as usize) = *(*wksp)
            .bitsToWeight
            .as_ptr()
            .add(HUF_getNbBits(*ct.add(n as usize)));
        n += 1;
    }

    /* attempt weights compression by FSE */
    if maxDstSize < 1 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let hSize = HUF_compressWeights(
            op.add(1) as *mut c_void,
            maxDstSize - 1,
            (*wksp).huffWeight.as_ptr() as *const c_void,
            maxSymbolValue as usize,
            &mut (*wksp).wksp as *mut HUF_CompressWeightsWksp as *mut c_void,
            core::mem::size_of::<HUF_CompressWeightsWksp>(),
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        if ((hSize > 1) as c_int & ((hSize < (maxSymbolValue as usize) / 2) as c_int)) != 0 {
            /* FSE compressed */
            *op.add(0) = hSize as BYTE;
            return hSize + 1;
        }
    }

    /* write raw values as 4-bits (max : 15) */
    if maxSymbolValue > (256 - 128) {
        return ERROR(ZSTD_error_GENERIC); /* should not happen : likely means source cannot be compressed */
    }
    if (((maxSymbolValue as usize + 1) / 2) + 1) > maxDstSize {
        return ERROR(ZSTD_error_dstSize_tooSmall); /* not enough space within dst buffer */
    }
    *op.add(0) = (128u32 /*special case*/).wrapping_add(maxSymbolValue.wrapping_sub(1)) as BYTE;
    /* to be sure it doesn't cause msan issue in final combination */
    *(*wksp)
        .huffWeight
        .as_mut_ptr()
        .add(maxSymbolValue as usize) = 0;
    n = 0;
    while n < maxSymbolValue {
        *op.add((n as usize / 2) + 1) = (((*(*wksp).huffWeight.as_ptr().add(n as usize) as c_int)
            << 4)
            + *(*wksp).huffWeight.as_ptr().add(n as usize + 1) as c_int) as BYTE;
        n += 2;
    }
    ((maxSymbolValue as usize + 1) / 2) + 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTable(
    CTable: *mut HUF_CElt,
    maxSymbolValuePtr: *mut c_uint,
    src: *const c_void,
    srcSize: usize,
    hasZeroWeights: *mut c_uint,
) -> usize {
    /* init not required, even though some static analyzer may complain */
    let mut huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1] =
        [0; HUF_SYMBOLVALUE_MAX as usize + 1];
    /* large enough for values from 0 to 16 */
    let mut rankVal: [U32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1] =
        [0; HUF_TABLELOG_ABSOLUTEMAX as usize + 1];
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let ct: *mut HUF_CElt = CTable.add(1);

    /* get symbol weights */
    let readSize = HUF_readStats(
        huffWeight.as_mut_ptr(),
        HUF_SYMBOLVALUE_MAX as usize + 1,
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
    if nbSymbols > (*maxSymbolValuePtr).wrapping_add(1) {
        return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
    }

    *maxSymbolValuePtr = nbSymbols - 1;

    HUF_writeCTableHeader(CTable, tableLog, *maxSymbolValuePtr);

    /* Prepare base value per rank */
    {
        let mut n: U32;
        let mut nextRankStart: U32 = 0;
        n = 1;
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
                ct.add(n as usize),
                (((tableLog.wrapping_add(1).wrapping_sub(w)) as BYTE) as c_int
                    & -((w != 0) as c_int)) as usize,
            );
            n += 1;
        }
    }

    /* fill val */
    {
        /* support w=0=>n=tableLog+1 */
        let mut nbPerRank: [U16; HUF_TABLELOG_MAX as usize + 2] =
            [0; HUF_TABLELOG_MAX as usize + 2];
        let mut valPerRank: [U16; HUF_TABLELOG_MAX as usize + 2] =
            [0; HUF_TABLELOG_MAX as usize + 2];
        {
            let mut n: U32 = 0;
            while n < nbSymbols {
                nbPerRank[HUF_getNbBits(*ct.add(n as usize))] += 1;
                n += 1;
            }
        }
        /* determine stating value per rank */
        valPerRank[tableLog as usize + 1] = 0; /* for w==0 */
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
                let idx = HUF_getNbBits(*ct.add(n as usize));
                let v = valPerRank[idx];
                valPerRank[idx] = v.wrapping_add(1);
                HUF_setValue(ct.add(n as usize), v as usize);
                n += 1;
            }
        }
    }

    readSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getNbBitsFromCTable(
    CTable: *const HUF_CElt,
    symbolValue: U32,
) -> U32 {
    let ct: *const HUF_CElt = CTable.add(1);
    if symbolValue > HUF_readCTableHeader(CTable).maxSymbolValue as U32 {
        return 0;
    }
    HUF_getNbBits(*ct.add(symbolValue as usize)) as U32
}

/**
 * HUF_setMaxHeight():
 * Try to enforce @targetNbBits on the Huffman tree described in @huffNode.
 */
unsafe fn HUF_setMaxHeight(huffNode: *mut nodeElt, lastNonNull: U32, targetNbBits: U32) -> U32 {
    let largestBits: U32 = (*huffNode.add(lastNonNull as usize)).nbBits as U32;
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
        while (*huffNode.offset(n as isize)).nbBits as U32 > targetNbBits {
            totalCost = (totalCost as U32).wrapping_add(baseCost.wrapping_sub(
                1u32 << (largestBits - (*huffNode.offset(n as isize)).nbBits as U32),
            )) as c_int;
            (*huffNode.offset(n as isize)).nbBits = targetNbBits as BYTE;
            n -= 1;
        }
        /* n end at index of smallest symbol using < targetNbBits */
        while (*huffNode.offset(n as isize)).nbBits as U32 == targetNbBits {
            n -= 1;
        }

        /* renorm totalCost from 2^largestBits to 2^targetNbBits
         * note : totalCost is necessarily a multiple of baseCost */
        totalCost >>= largestBits - targetNbBits;

        /* repay normalized cost */
        {
            let noSymbol: U32 = 0xF0F0F0F0;
            let mut rankLast: [U32; HUF_TABLELOG_MAX as usize + 2] =
                [0; HUF_TABLELOG_MAX as usize + 2];

            /* Get pos of last (smallest = lowest cum. count) symbol per rank */
            ZSTD_memset(
                rankLast.as_mut_ptr() as *mut c_void,
                0xF0,
                core::mem::size_of_val(&rankLast),
            );
            {
                let mut currentNbBits: U32 = targetNbBits;
                let mut pos: c_int = n;
                while pos >= 0 {
                    if (*huffNode.offset(pos as isize)).nbBits as U32 >= currentNbBits {
                        pos -= 1;
                        continue;
                    }
                    currentNbBits = (*huffNode.offset(pos as isize)).nbBits as U32; /* < targetNbBits */
                    rankLast[(targetNbBits - currentNbBits) as usize] = pos as U32;
                    pos -= 1;
                }
            }

            while totalCost > 0 {
                /* Try to reduce the next power of 2 above totalCost because we
                 * gain back half the rank.
                 */
                let mut nBitsToDecrease: U32 = ZSTD_highbit32(totalCost as U32) + 1;
                while nBitsToDecrease > 1 {
                    let highPos: U32 = rankLast[nBitsToDecrease as usize];
                    let lowPos: U32 = rankLast[(nBitsToDecrease - 1) as usize];
                    if highPos == noSymbol {
                        nBitsToDecrease -= 1;
                        continue;
                    }
                    /* Decrease highPos if no symbols of lowPos or if it is
                     * not cheaper to remove 2 lowPos than highPos.
                     */
                    if lowPos == noSymbol {
                        break;
                    }
                    {
                        let highTotal: U32 = (*huffNode.add(highPos as usize)).count;
                        let lowTotal: U32 =
                            2u32.wrapping_mul((*huffNode.add(lowPos as usize)).count);
                        if highTotal <= lowTotal {
                            break;
                        }
                    }
                    nBitsToDecrease -= 1;
                }
                /* HUF_MAX_TABLELOG test just to please gcc 5+; but it should not be necessary */
                while (nBitsToDecrease <= HUF_TABLELOG_MAX)
                    && (rankLast[nBitsToDecrease as usize] == noSymbol)
                {
                    nBitsToDecrease += 1;
                }
                /* Increase the number of bits to gain back half the rank cost. */
                totalCost -= 1 << (nBitsToDecrease - 1);
                (*huffNode.add(rankLast[nBitsToDecrease as usize] as usize)).nbBits =
                    (*huffNode.add(rankLast[nBitsToDecrease as usize] as usize))
                        .nbBits
                        .wrapping_add(1);

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
                    if (*huffNode.add(rankLast[nBitsToDecrease as usize] as usize)).nbBits as U32
                        != targetNbBits.wrapping_sub(nBitsToDecrease)
                    {
                        rankLast[nBitsToDecrease as usize] = noSymbol; /* this rank is now empty */
                    }
                }
            } /* while (totalCost > 0) */

            /* If we've removed too much weight, then we have to add it back. */
            while totalCost < 0 {
                /* Sometimes, cost correction overshoot */
                /* special case : no rank 1 symbol (using targetNbBits-1);
                 * let's create one from largest rank 0 (using targetNbBits).
                 */
                if rankLast[1] == noSymbol {
                    while (*huffNode.offset(n as isize)).nbBits as U32 == targetNbBits {
                        n -= 1;
                    }
                    (*huffNode.offset((n + 1) as isize)).nbBits =
                        (*huffNode.offset((n + 1) as isize)).nbBits.wrapping_sub(1);
                    rankLast[1] = (n + 1) as U32;
                    totalCost += 1;
                    continue;
                }
                (*huffNode.add((rankLast[1] + 1) as usize)).nbBits =
                    (*huffNode.add((rankLast[1] + 1) as usize)).nbBits.wrapping_sub(1);
                rankLast[1] += 1;
                totalCost += 1;
            }
        } /* repay normalized cost */
    } /* there are several too large elements (at least >= 2) */

    targetNbBits
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct rankPos {
    pub base: U16,
    pub curr: U16,
}

pub type huffNodeTable = [nodeElt; 2 * (HUF_SYMBOLVALUE_MAX as usize + 1)];

/* Number of buckets available for HUF_sort() */
const RANK_POSITION_TABLE_SIZE: usize = 192;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_buildCTable_wksp_tables {
    pub huffNodeTbl: huffNodeTable,
    pub rankPosition: [rankPos; RANK_POSITION_TABLE_SIZE],
}

/* RANK_POSITION_DISTINCT_COUNT_CUTOFF == Cutoff point in HUF_sort() buckets for
 * which we use log2 bucketing. */
const RANK_POSITION_MAX_COUNT_LOG: u32 = 32;
const RANK_POSITION_LOG_BUCKETS_BEGIN: u32 =
    (RANK_POSITION_TABLE_SIZE as u32 - 1) - RANK_POSITION_MAX_COUNT_LOG - 1 /* == 158 */;

#[inline(always)]
fn RANK_POSITION_DISTINCT_COUNT_CUTOFF() -> u32 {
    RANK_POSITION_LOG_BUCKETS_BEGIN + ZSTD_highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN)
}

/* Return the appropriate bucket index for a given count. */
unsafe fn HUF_getIndex(count: U32) -> U32 {
    if count < RANK_POSITION_DISTINCT_COUNT_CUTOFF() {
        count
    } else {
        ZSTD_highbit32(count) + RANK_POSITION_LOG_BUCKETS_BEGIN
    }
}

/* Helper swap function for HUF_quickSortPartition() */
unsafe fn HUF_swapNodes(a: *mut nodeElt, b: *mut nodeElt) {
    let tmp: nodeElt = *a;
    *a = *b;
    *b = tmp;
}

/* Returns 0 if the huffNode array is not sorted by descending count */
unsafe fn HUF_isSorted(huffNode: *mut nodeElt, maxSymbolValue1: U32) -> c_int {
    let mut i: U32 = 1;
    while i < maxSymbolValue1 {
        if (*huffNode.add(i as usize)).count > (*huffNode.add((i - 1) as usize)).count {
            return 0;
        }
        i += 1;
    }
    1
}

/* Insertion sort by descending order */
unsafe fn HUF_insertionSort(huffNode: *mut nodeElt, low: c_int, high: c_int) {
    let mut i: c_int;
    let size: c_int = high - low + 1;
    let huffNode: *mut nodeElt = huffNode.offset(low as isize);
    i = 1;
    while i < size {
        let key: nodeElt = *huffNode.offset(i as isize);
        let mut j: c_int = i - 1;
        while j >= 0 && (*huffNode.offset(j as isize)).count < key.count {
            *huffNode.offset((j + 1) as isize) = *huffNode.offset(j as isize);
            j -= 1;
        }
        *huffNode.offset((j + 1) as isize) = key;
        i += 1;
    }
}

/* Pivot helper function for quicksort. */
unsafe fn HUF_quickSortPartition(arr: *mut nodeElt, low: c_int, high: c_int) -> c_int {
    /* Simply select rightmost element as pivot. */
    let pivot: U32 = (*arr.offset(high as isize)).count;
    let mut i: c_int = low - 1;
    let mut j: c_int = low;
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
unsafe fn HUF_simpleQuickSort(arr: *mut nodeElt, low: c_int, high: c_int) {
    let kInsertionSortThreshold: c_int = 8;
    let mut low = low;
    let mut high = high;
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
 * Sorts the symbols [0, maxSymbolValue] by count[symbol] in decreasing order.
 */
unsafe fn HUF_sort(
    huffNode: *mut nodeElt,
    count: *const c_uint,
    maxSymbolValue: U32,
    rankPosition: *mut rankPos,
) {
    let mut n: U32;
    let maxSymbolValue1: U32 = maxSymbolValue + 1;

    /* Compute base and set curr to base. */
    ZSTD_memset(
        rankPosition as *mut c_void,
        0,
        core::mem::size_of::<rankPos>() * RANK_POSITION_TABLE_SIZE,
    );
    n = 0;
    while n < maxSymbolValue1 {
        let lowerRank: U32 = HUF_getIndex(*count.add(n as usize));
        (*rankPosition.add(lowerRank as usize)).base += 1;
        n += 1;
    }

    /* Set up the rankPosition table */
    n = RANK_POSITION_TABLE_SIZE as U32 - 1;
    while n > 0 {
        (*rankPosition.add((n - 1) as usize)).base = (*rankPosition.add((n - 1) as usize))
            .base
            .wrapping_add((*rankPosition.add(n as usize)).base);
        (*rankPosition.add((n - 1) as usize)).curr = (*rankPosition.add((n - 1) as usize)).base;
        n -= 1;
    }

    /* Insert each symbol into their appropriate bucket, setting up rankPosition table. */
    n = 0;
    while n < maxSymbolValue1 {
        let c: U32 = *count.add(n as usize);
        let r: U32 = HUF_getIndex(c) + 1;
        let pos: U32 = (*rankPosition.add(r as usize)).curr as U32;
        (*rankPosition.add(r as usize)).curr =
            (*rankPosition.add(r as usize)).curr.wrapping_add(1);
        (*huffNode.add(pos as usize)).count = c;
        (*huffNode.add(pos as usize)).byte = n as BYTE;
        n += 1;
    }

    /* Sort each bucket. */
    n = RANK_POSITION_DISTINCT_COUNT_CUTOFF();
    while n < RANK_POSITION_TABLE_SIZE as U32 - 1 {
        let bucketSize: c_int = (*rankPosition.add(n as usize)).curr as c_int
            - (*rankPosition.add(n as usize)).base as c_int;
        let bucketStartIdx: U32 = (*rankPosition.add(n as usize)).base as U32;
        if bucketSize > 1 {
            HUF_simpleQuickSort(huffNode.add(bucketStartIdx as usize), 0, bucketSize - 1);
        }
        n += 1;
    }
}

/** HUF_buildCTable_wksp() :
 *  Same as HUF_buildCTable(), but using externally allocated scratch buffer.
 */
const STARTNODE: c_int = HUF_SYMBOLVALUE_MAX as c_int + 1;

/* HUF_buildTree():
 * Takes the huffNode array sorted by HUF_sort() and builds an unlimited-depth
 * Huffman tree.
 */
unsafe fn HUF_buildTree(huffNode: *mut nodeElt, maxSymbolValue: U32) -> c_int {
    let huffNode0: *mut nodeElt = huffNode.offset(-1);
    let nonNullRank: c_int;
    let mut lowS: c_int;
    let mut lowN: c_int;
    let mut nodeNb: c_int = STARTNODE;
    let mut n: c_int;
    let nodeRoot: c_int;
    /* init for parents */
    let mut nnr: c_int = maxSymbolValue as c_int;
    while (*huffNode.offset(nnr as isize)).count == 0 {
        nnr -= 1;
    }
    nonNullRank = nnr;
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
        let n1: c_int = if (*huffNode.offset(lowS as isize)).count
            < (*huffNode.offset(lowN as isize)).count
        {
            let t = lowS;
            lowS -= 1;
            t
        } else {
            let t = lowN;
            lowN += 1;
            t
        };
        let n2: c_int = if (*huffNode.offset(lowS as isize)).count
            < (*huffNode.offset(lowN as isize)).count
        {
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
unsafe fn HUF_buildCTableFromTree(
    CTable: *mut HUF_CElt,
    huffNode: *const nodeElt,
    nonNullRank: c_int,
    maxSymbolValue: U32,
    maxNbBits: U32,
) {
    let ct: *mut HUF_CElt = CTable.add(1);
    /* fill result into ctable (val, nbBits) */
    let mut n: c_int;
    let mut nbPerRank: [U16; HUF_TABLELOG_MAX as usize + 1] = [0; HUF_TABLELOG_MAX as usize + 1];
    let mut valPerRank: [U16; HUF_TABLELOG_MAX as usize + 1] = [0; HUF_TABLELOG_MAX as usize + 1];
    let alphabetSize: c_int = (maxSymbolValue + 1) as c_int;
    n = 0;
    while n <= nonNullRank {
        nbPerRank[(*huffNode.offset(n as isize)).nbBits as usize] += 1;
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
        let idx = HUF_getNbBits(*ct.offset(n as isize));
        let v = valPerRank[idx];
        valPerRank[idx] = v.wrapping_add(1);
        HUF_setValue(ct.offset(n as isize), v as usize);
        n += 1;
    }

    HUF_writeCTableHeader(CTable, maxNbBits, maxSymbolValue);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_buildCTable_wksp(
    CTable: *mut HUF_CElt,
    count: *const c_uint,
    maxSymbolValue: U32,
    maxNbBits: U32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let mut wkspSize = wkspSize;
    let mut maxNbBits = maxNbBits;
    let wksp_tables: *mut HUF_buildCTable_wksp_tables = HUF_alignUpWorkspace(
        workSpace,
        &mut wkspSize,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_buildCTable_wksp_tables;
    let huffNode0: *mut nodeElt = (*wksp_tables).huffNodeTbl.as_mut_ptr();
    let huffNode: *mut nodeElt = huffNode0.add(1);
    let nonNullRank: c_int;

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
        huffNode0 as *mut c_void,
        0,
        core::mem::size_of::<huffNodeTable>(),
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
        return ERROR(ZSTD_error_GENERIC); /* check fit into table */
    }

    HUF_buildCTableFromTree(CTable, huffNode, nonNullRank, maxSymbolValue, maxNbBits);

    maxNbBits as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_estimateCompressedSize(
    CTable: *const HUF_CElt,
    count: *const c_uint,
    maxSymbolValue: c_uint,
) -> usize {
    let ct: *const HUF_CElt = CTable.add(1);
    let mut nbBits: usize = 0;
    let mut s: c_int = 0;
    while s <= maxSymbolValue as c_int {
        nbBits = nbBits.wrapping_add(
            HUF_getNbBits(*ct.offset(s as isize)).wrapping_mul(*count.offset(s as isize) as usize),
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
    let ct: *const HUF_CElt = CTable.add(1);
    let mut bad: c_int = 0;
    let mut s: c_int;

    if (header.maxSymbolValue as c_uint) < maxSymbolValue {
        return 0;
    }

    s = 0;
    while s <= maxSymbolValue as c_int {
        bad |= (*count.offset(s as isize) != 0) as c_int
            & (HUF_getNbBits(*ct.offset(s as isize)) == 0) as c_int;
        s += 1;
    }
    (bad == 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compressBound(size: usize) -> usize {
    HUF_COMPRESSBOUND(size)
}

/** HUF_CStream_t:
 * Huffman uses its own BIT_CStream_t implementation.
 */
const HUF_BITS_IN_CONTAINER: usize = core::mem::size_of::<usize>() * 8;

#[repr(C)]
#[derive(Copy, Clone)]
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
            bitContainer: [0; 2],
            bitPos: [0; 2],
            startPtr: core::ptr::null_mut(),
            ptr: core::ptr::null_mut(),
            endPtr: core::ptr::null_mut(),
        }
    }
}

/**! HUF_initCStream():
 * Initializes the bitstream.
 * @returns 0 or an error code.
 */
unsafe fn HUF_initCStream(
    bitC: *mut HUF_CStream_t,
    startPtr: *mut c_void,
    dstCapacity: usize,
) -> usize {
    ZSTD_memset(
        bitC as *mut c_void,
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
unsafe fn HUF_addBits(bitC: *mut HUF_CStream_t, elt: HUF_CElt, idx: c_int, kFast: c_int) {
    /* This is efficient on x86-64 with BMI2 because shrx
     * only reads the low 6 bits of the register.
     */
    (*bitC).bitContainer[idx as usize] =
        (*bitC).bitContainer[idx as usize].wrapping_shr(HUF_getNbBits(elt) as u32);
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
unsafe fn HUF_zeroIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[1] = 0;
    (*bitC).bitPos[1] = 0;
}

/* HUF_mergeIndex1() :
 * Merges the bit container @ index 1 into the bit container @ index 0
 * and zeros the bit container @ index 1.
 */
#[inline(always)]
unsafe fn HUF_mergeIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[0] = (*bitC)
        .bitContainer[0]
        .wrapping_shr(((*bitC).bitPos[1] & 0xFF) as u32);
    (*bitC).bitContainer[0] |= (*bitC).bitContainer[1];
    (*bitC).bitPos[0] = (*bitC).bitPos[0].wrapping_add((*bitC).bitPos[1]);
}

/* HUF_flushBits() :
 * Flushes the bits in the bit container @ index 0.
 */
#[inline(always)]
unsafe fn HUF_flushBits(bitC: *mut HUF_CStream_t, kFast: c_int) {
    /* The upper bits of bitPos are noisy, so we must mask by 0xFF. */
    let nbBits: usize = (*bitC).bitPos[0] & 0xFF;
    let nbBytes: usize = nbBits >> 3;
    /* The top nbBits bits of bitContainer are the ones we need. */
    let bitContainer: usize = (*bitC)
        .bitContainer[0]
        .wrapping_shr((HUF_BITS_IN_CONTAINER - nbBits) as u32);
    /* Mask bitPos to account for the bytes we consumed. */
    (*bitC).bitPos[0] &= 7;
    MEM_writeLEST((*bitC).ptr as *mut c_void, bitContainer);
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
unsafe fn HUF_endMark() -> HUF_CElt {
    let mut endMark: HUF_CElt = 0;
    HUF_setNbBits(&mut endMark, 1);
    HUF_setValue(&mut endMark, 1);
    endMark
}

/* HUF_closeCStream() :
 *  @return Size of CStream, in bytes,
 *          or 0 if it could not fit into dstBuffer */
unsafe fn HUF_closeCStream(bitC: *mut HUF_CStream_t) -> usize {
    HUF_addBits(bitC, HUF_endMark(), /* idx */ 0, /* kFast */ 0);
    HUF_flushBits(bitC, /* kFast */ 0);
    {
        let nbBits: usize = (*bitC).bitPos[0] & 0xFF;
        if (*bitC).ptr >= (*bitC).endPtr {
            return 0; /* overflow detected */
        }
        ((*bitC).ptr as usize - (*bitC).startPtr as usize) + (nbBits > 0) as usize
    }
}

#[inline(always)]
unsafe fn HUF_encodeSymbol(
    bitCPtr: *mut HUF_CStream_t,
    symbol: U32,
    CTable: *const HUF_CElt,
    idx: c_int,
    fast: c_int,
) {
    HUF_addBits(bitCPtr, *CTable.add(symbol as usize), idx, fast);
}

#[inline(always)]
unsafe fn HUF_compress1X_usingCTable_internal_body_loop(
    bitC: *mut HUF_CStream_t,
    ip: *const BYTE,
    srcSize: usize,
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
            HUF_encodeSymbol(bitC, *ip.offset(n as isize) as U32, ct, 0, /* fast */ 0);
            rem -= 1;
        }
        HUF_flushBits(bitC, kFastFlush);
    }

    /* Join to 2 * kUnroll */
    if n % (2 * kUnroll) != 0 {
        let mut u: c_int = 1;
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
        let mut u: c_int = 1;
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
unsafe fn HUF_tightCompressBound(srcSize: usize, tableLog: usize) -> usize {
    ((srcSize.wrapping_mul(tableLog)) >> 3) + 8
}

#[inline(always)]
unsafe fn HUF_compress1X_usingCTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
) -> usize {
    let tableLog: U32 = HUF_readCTableHeader(CTable).tableLog as U32;
    let ct: *const HUF_CElt = CTable.add(1);
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
            op as *mut c_void,
            (oend as usize) - (op as usize),
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
                7 | _ => {
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
                6 | _ => {
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

/* DYNAMIC_BMI2 == 0 */
unsafe fn HUF_compress1X_usingCTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    HUF_compress1X_usingCTable_internal_body(dst, dstSize, src, srcSize, CTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_usingCTable(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    HUF_compress1X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

unsafe fn HUF_compress4X_usingCTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    let segmentSize: usize = (srcSize + 3) / 4; /* first 3 segments */
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
    op = op.add(6); /* jumpTable */

    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as usize) - (op as usize),
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
        MEM_writeLE16(ostart as *mut c_void, cSize as U16);
        op = op.add(cSize);
    }

    ip = ip.add(segmentSize);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as usize) - (op as usize),
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
        MEM_writeLE16(ostart.add(2) as *mut c_void, cSize as U16);
        op = op.add(cSize);
    }

    ip = ip.add(segmentSize);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as usize) - (op as usize),
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
        MEM_writeLE16(ostart.add(4) as *mut c_void, cSize as U16);
        op = op.add(cSize);
    }

    ip = ip.add(segmentSize);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as usize) - (op as usize),
            ip as *const c_void,
            (iend as usize) - (ip as usize),
            CTable,
            flags,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        op = op.add(cSize);
    }

    (op as usize) - (ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_usingCTable(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    HUF_compress4X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

pub type HUF_nbStreams_e = c_uint;
pub const HUF_singleStream: HUF_nbStreams_e = 0;
pub const HUF_fourStreams: HUF_nbStreams_e = 1;

unsafe fn HUF_compressCTable_internal(
    ostart: *mut BYTE,
    op: *mut BYTE,
    oend: *mut BYTE,
    src: *const c_void,
    srcSize: usize,
    nbStreams: HUF_nbStreams_e,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    let mut op = op;
    let cSize: usize = if nbStreams == HUF_singleStream {
        HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            (oend as usize) - (op as usize),
            src,
            srcSize,
            CTable,
            flags,
        )
    } else {
        HUF_compress4X_usingCTable_internal(
            op as *mut c_void,
            (oend as usize) - (op as usize),
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
    op = op.add(cSize);
    /* check compressibility */
    if ((op as usize) - (ostart as usize)) >= srcSize.wrapping_sub(1) {
        return 0;
    }
    (op as usize) - (ostart as usize)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union HUF_compress_tables_t_wksps {
    pub buildCTable_wksp: HUF_buildCTable_wksp_tables,
    pub writeCTable_wksp: HUF_WriteCTableWksp,
    pub hist_wksp: [U32; HIST_WKSP_SIZE_U32 as usize],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_compress_tables_t {
    pub count: [c_uint; HUF_SYMBOLVALUE_MAX as usize + 1],
    pub CTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX as usize)],
    pub wksps: HUF_compress_tables_t_wksps,
}

const SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE: usize = 4096;
const SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO: usize = 10; /* Must be >= 2 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_cardinality(count: *const c_uint, maxSymbolValue: c_uint) -> c_uint {
    let mut cardinality: c_uint = 0;
    let mut i: c_uint = 0;

    while i < maxSymbolValue + 1 {
        if *count.add(i as usize) != 0 {
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
    srcSize: usize,
    maxSymbolValue: c_uint,
    workSpace: *mut c_void,
    wkspSize: usize,
    table: *mut HUF_CElt,
    count: *const c_uint,
    flags: c_int,
) -> c_uint {
    if (flags & HUF_flags_optimalDepth) == 0 {
        /* cheap evaluation, based on FSE */
        return FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 1);
    }

    {
        let dst: *mut BYTE =
            (workSpace as *mut BYTE).wrapping_add(core::mem::size_of::<HUF_WriteCTableWksp>());
        let dstSize: usize = wkspSize - core::mem::size_of::<HUF_WriteCTableWksp>();
        let mut hSize: usize;
        let mut newSize: usize;
        let symbolCardinality: c_uint = HUF_cardinality(count, maxSymbolValue);
        let minTableLog: c_uint = HUF_minTableLog(symbolCardinality);
        let mut optSize: usize = (!0usize) - 1;
        let mut optLog: c_uint = maxTableLog;
        let mut optLogGuess: c_uint;

        /* Search until size increases */
        optLogGuess = minTableLog;
        while optLogGuess <= maxTableLog {
            {
                let maxBits = HUF_buildCTable_wksp(
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

                if maxBits < optLogGuess as usize && optLogGuess > minTableLog {
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

/* HUF_compress_internal() :
 * `workSpace_align4` must be aligned on 4-bytes boundaries,
 * and occupies the same space as a table of HUF_WORKSPACE_SIZE_U64 unsigned */
unsafe fn HUF_compress_internal(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    nbStreams: HUF_nbStreams_e,
    workSpace: *mut c_void,
    wkspSize: usize,
    oldHufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: c_int,
) -> usize {
    let mut wkspSize = wkspSize;
    let mut maxSymbolValue = maxSymbolValue;
    let mut huffLog = huffLog;
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
            ostart, op, oend, src, srcSize, nbStreams, oldHufTable, flags,
        );
    }

    /* If uncompressible data is suspected, do a smaller sampling first */
    if (flags & HUF_flags_suspectUncompressible) != 0
        && srcSize >= (SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE * SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO)
    {
        let mut largestTotal: usize = 0;
        {
            let mut maxSymbolValueBegin: c_uint = maxSymbolValue;
            let largestBegin: usize = HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut maxSymbolValueBegin,
                src as *const BYTE as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as usize;
            if ERR_isError(largestBegin) != 0 {
                return largestBegin;
            }
            largestTotal += largestBegin;
        }
        {
            let mut maxSymbolValueEnd: c_uint = maxSymbolValue;
            let largestEnd: usize = HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut maxSymbolValueEnd,
                (src as *const BYTE)
                    .add(srcSize)
                    .sub(SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE) as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as usize;
            if ERR_isError(largestEnd) != 0 {
                return largestEnd;
            }
            largestTotal += largestEnd;
        }
        if largestTotal <= ((2 * SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE) >> 7) + 4 {
            return 0; /* heuristic : probably not compressible enough */
        }
    }

    /* Scan input and build symbol stats */
    {
        let largest = HIST_count_wksp(
            (*table).count.as_mut_ptr(),
            &mut maxSymbolValue,
            src as *const BYTE as *const c_void,
            srcSize,
            (*table).wksps.hist_wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<[U32; HIST_WKSP_SIZE_U32 as usize]>(),
        );
        if ERR_isError(largest) != 0 {
            return largest;
        }
        if largest == srcSize {
            *ostart = *(src as *const BYTE);
            return 1; /* single symbol, rle */
        }
        if largest <= (srcSize >> 7) + 4 {
            return 0; /* heuristic : probably not compressible enough */
        }
    }

    /* Check validity of previous table */
    if !repeat.is_null()
        && *repeat == HUF_repeat_check
        && HUF_validateCTable(oldHufTable, (*table).count.as_ptr(), maxSymbolValue) == 0
    {
        *repeat = HUF_repeat_none;
    }
    /* Heuristic : use existing table for small inputs */
    if (flags & HUF_flags_preferRepeat) != 0 && !repeat.is_null() && *repeat != HUF_repeat_none {
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
        core::mem::size_of::<HUF_compress_tables_t_wksps>(),
        (*table).CTable.as_mut_ptr(),
        (*table).count.as_ptr(),
        flags,
    );
    {
        let maxBits = HUF_buildCTable_wksp(
            (*table).CTable.as_mut_ptr(),
            (*table).count.as_ptr(),
            maxSymbolValue,
            huffLog,
            &mut (*table).wksps.buildCTable_wksp as *mut HUF_buildCTable_wksp_tables as *mut c_void,
            core::mem::size_of::<HUF_buildCTable_wksp_tables>(),
        );
        if ERR_isError(maxBits) != 0 {
            return maxBits;
        }
        huffLog = maxBits as U32;
    }

    /* Write table description header */
    {
        let hSize = HUF_writeCTable_wksp(
            op as *mut c_void,
            dstSize,
            (*table).CTable.as_ptr(),
            maxSymbolValue,
            huffLog,
            &mut (*table).wksps.writeCTable_wksp as *mut HUF_WriteCTableWksp as *mut c_void,
            core::mem::size_of::<HUF_WriteCTableWksp>(),
        );
        if ERR_isError(hSize) != 0 {
            return hSize;
        }
        /* Check if using previous huffman table is beneficial */
        if !repeat.is_null() && *repeat != HUF_repeat_none {
            let oldSize: usize =
                HUF_estimateCompressedSize(oldHufTable, (*table).count.as_ptr(), maxSymbolValue);
            let newSize: usize = HUF_estimateCompressedSize(
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
        if hSize + 12usize >= srcSize {
            return 0;
        }
        op = op.add(hSize);
        if !repeat.is_null() {
            *repeat = HUF_repeat_none;
        }
        if !oldHufTable.is_null() {
            ZSTD_memcpy(
                oldHufTable as *mut c_void,
                (*table).CTable.as_ptr() as *const c_void,
                core::mem::size_of_val(&(*table).CTable),
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
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: usize,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: c_int,
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
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    maxSymbolValue: c_uint,
    huffLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: usize,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: c_int,
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
