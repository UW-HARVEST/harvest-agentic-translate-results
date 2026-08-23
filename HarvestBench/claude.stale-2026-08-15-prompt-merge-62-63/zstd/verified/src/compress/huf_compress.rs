//! Translation of c_src/src/compress/huf_compress.c
//! Huffman encoder, part of New Generation Entropy library.
//! Build config: DYNAMIC_BMI2=0 (non-bmi2 default paths).
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::c_void;

use crate::common::bits::highbit32;
use crate::common::error::{code, error, err_is_error};
use crate::common::fse::FSE_CTable;
use crate::common::huf_common::{
    HUF_readStats, HUF_BLOCKSIZE_MAX, HUF_SYMBOLVALUE_MAX, HUF_TABLELOG_ABSOLUTEMAX,
    HUF_TABLELOG_DEFAULT, HUF_TABLELOG_MAX, HUF_flags_optimalDepth, HUF_flags_preferRepeat,
    HUF_flags_suspectUncompressible,
};
use crate::common::mem::{mem_32bits, mem_write_le16, mem_write_le_st};
use crate::compress::fse_compress::{
    FSE_buildCTable_wksp, FSE_compress_usingCTable, FSE_normalizeCount, FSE_optimalTableLog,
    FSE_optimalTableLog_internal, FSE_writeNCount,
};
use crate::compress::hist::{HIST_count_simple, HIST_count_wksp, HIST_WKSP_SIZE_U32};

// ---------------------------------------------------------------------------
// Core public types
// ---------------------------------------------------------------------------

/// HUF_CElt : consider it an incomplete type. size_t on this platform.
pub type HUF_CElt = usize;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_CTableHeader {
    pub tableLog: u8,
    pub maxSymbolValue: u8,
    pub unused: [u8; 6],
}

// HUF_repeat enum (C int)
pub const HUF_repeat_none: u32 = 0;
pub const HUF_repeat_check: u32 = 1;
pub const HUF_repeat_valid: u32 = 2;
type HUF_repeat = u32;

// HUF_nbStreams_e
#[derive(Clone, Copy, PartialEq, Eq)]
enum HUF_nbStreams_e {
    HUF_singleStream,
    HUF_fourStreams,
}
use HUF_nbStreams_e::*;

// ---------------------------------------------------------------------------
// Constants derived from huf.h / fse.h
// ---------------------------------------------------------------------------

// HUF_CTABLE_SIZE_ST(maxSymbolValue) = maxSymbolValue + 2
const fn HUF_CTABLE_SIZE_ST(maxSymbolValue: usize) -> usize {
    maxSymbolValue + 2
}

// HUF_CTABLE_WORKSPACE_SIZE_U32 == (4*(255+1)) + 192 == 1216
const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = (4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192;
const HUF_CTABLE_WORKSPACE_SIZE: usize = HUF_CTABLE_WORKSPACE_SIZE_U32 * core::mem::size_of::<u32>();

// FSE_CTABLE_SIZE_U32(6, 12) = 1 + (1<<5) + ((12+1)*2) = 59
const FSE_CTABLE_SIZE_U32_HUFF: usize =
    1 + (1usize << (MAX_FSE_TABLELOG_FOR_HUFF_HEADER - 1)) + ((HUF_TABLELOG_MAX as usize + 1) * 2);
// FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(12, 6) = ((12+2)+(1<<6))/2 + 2 = 41
const FSE_BUILD_CTABLE_WKSP_U32_HUFF: usize = ((HUF_TABLELOG_MAX as usize + 2)
    + (1usize << MAX_FSE_TABLELOG_FOR_HUFF_HEADER))
    / 2
    + core::mem::size_of::<u64>() / core::mem::size_of::<u32>();

const MAX_FSE_TABLELOG_FOR_HUFF_HEADER: usize = 6;

// HUF_COMPRESSBOUND(size) = HUF_CTABLEBOUND + HUF_BLOCKBOUND(size)
//   HUF_CTABLEBOUND = 129 ; HUF_BLOCKBOUND(size) = size + (size >> 8) + 8
const HUF_CTABLEBOUND: usize = 129;
const fn HUF_BLOCKBOUND(size: usize) -> usize {
    size + (size >> 8) + 8
}

// ---------------------------------------------------------------------------
// nodeElt
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct nodeElt {
    count: u32,
    parent: u16,
    byte: u8,
    nbBits: u8,
}

// ---------------------------------------------------------------------------
// Workspace layout structs (must match C layout exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
struct HUF_CompressWeightsWksp {
    CTable: [FSE_CTable; FSE_CTABLE_SIZE_U32_HUFF],
    scratchBuffer: [u32; FSE_BUILD_CTABLE_WKSP_U32_HUFF],
    count: [u32; HUF_TABLELOG_MAX as usize + 1],
    norm: [i16; HUF_TABLELOG_MAX as usize + 1],
}

#[repr(C)]
struct HUF_WriteCTableWksp {
    wksp: HUF_CompressWeightsWksp,
    bitsToWeight: [u8; HUF_TABLELOG_MAX as usize + 1],
    huffWeight: [u8; HUF_SYMBOLVALUE_MAX as usize],
}

// huffNodeTable[2*(HUF_SYMBOLVALUE_MAX+1)]
const HUFNODE_TABLE_SIZE: usize = 2 * (HUF_SYMBOLVALUE_MAX as usize + 1);

#[repr(C)]
#[derive(Clone, Copy)]
struct rankPos {
    base: u16,
    curr: u16,
}

const RANK_POSITION_TABLE_SIZE: usize = 192;

#[repr(C)]
struct HUF_buildCTable_wksp_tables {
    huffNodeTbl: [nodeElt; HUFNODE_TABLE_SIZE],
    rankPosition: [rankPos; RANK_POSITION_TABLE_SIZE],
}

// ---------------------------------------------------------------------------
// HUF_alignUpWorkspace
// ---------------------------------------------------------------------------

const HUF_WORKSPACE_MAX_ALIGNMENT: usize = 8;

unsafe fn HUF_alignUpWorkspace(
    workspace: *mut c_void,
    workspaceSizePtr: *mut usize,
    align: usize,
) -> *mut c_void {
    let mask = align - 1;
    let rem = (workspace as usize) & mask;
    let add = (align - rem) & mask;
    let aligned = (workspace as *mut u8).add(add);
    debug_assert!((align & (align - 1)) == 0);
    debug_assert!(align <= HUF_WORKSPACE_MAX_ALIGNMENT);
    if *workspaceSizePtr >= add {
        *workspaceSizePtr -= add;
        aligned as *mut c_void
    } else {
        *workspaceSizePtr = 0;
        core::ptr::null_mut()
    }
}

// ---------------------------------------------------------------------------
// HUF_compressWeights
// ---------------------------------------------------------------------------

unsafe fn HUF_compressWeights(
    dst: *mut c_void,
    dstSize: usize,
    weightTable: *const c_void,
    wtSize: usize,
    workspace: *mut c_void,
    mut workspaceSize: usize,
) -> usize {
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let oend = ostart.add(dstSize);

    let mut maxSymbolValue: u32 = HUF_TABLELOG_MAX;
    let mut tableLog: u32 = MAX_FSE_TABLELOG_FOR_HUFF_HEADER as u32;
    let wksp = HUF_alignUpWorkspace(workspace, &mut workspaceSize, core::mem::align_of::<u32>())
        as *mut HUF_CompressWeightsWksp;

    if workspaceSize < core::mem::size_of::<HUF_CompressWeightsWksp>() {
        return error(code::GENERIC);
    }

    /* init conditions */
    if wtSize <= 1 {
        return 0; /* Not compressible */
    }

    /* Scan input and build symbol stats */
    {
        let maxCount = HIST_count_simple(
            (*wksp).count.as_mut_ptr(),
            &mut maxSymbolValue,
            weightTable,
            wtSize,
        ) as usize;
        if maxCount == wtSize {
            return 1; /* only a single symbol in src : rle */
        }
        if maxCount == 1 {
            return 0; /* each symbol present maximum once => not compressible */
        }
    }

    tableLog = FSE_optimalTableLog(tableLog, wtSize, maxSymbolValue);
    {
        let err = FSE_normalizeCount(
            (*wksp).norm.as_mut_ptr(),
            tableLog,
            (*wksp).count.as_ptr(),
            wtSize,
            maxSymbolValue,
            0,
        );
        if err_is_error(err) != 0 {
            return err;
        }
    }

    /* Write table description header */
    {
        let hSize = FSE_writeNCount(
            op as *mut c_void,
            oend as usize - op as usize,
            (*wksp).norm.as_ptr(),
            maxSymbolValue,
            tableLog,
        );
        if err_is_error(hSize) != 0 {
            return hSize;
        }
        op = op.add(hSize);
    }

    /* Compress */
    {
        let err = FSE_buildCTable_wksp(
            (*wksp).CTable.as_mut_ptr(),
            (*wksp).norm.as_ptr(),
            maxSymbolValue,
            tableLog,
            (*wksp).scratchBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*wksp).scratchBuffer),
        );
        if err_is_error(err) != 0 {
            return err;
        }
    }
    {
        let cSize = FSE_compress_usingCTable(
            op as *mut c_void,
            oend as usize - op as usize,
            weightTable,
            wtSize,
            (*wksp).CTable.as_ptr(),
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 {
            return 0; /* not enough space for compressed data */
        }
        op = op.add(cSize);
    }

    op as usize - ostart as usize
}

// ---------------------------------------------------------------------------
// CElt bit-packing helpers
// ---------------------------------------------------------------------------

#[inline]
fn HUF_getNbBits(elt: HUF_CElt) -> usize {
    elt & 0xFF
}

#[inline]
fn HUF_getNbBitsFast(elt: HUF_CElt) -> usize {
    elt
}

#[inline]
fn HUF_getValue(elt: HUF_CElt) -> usize {
    elt & !(0xFF as usize)
}

#[inline]
fn HUF_getValueFast(elt: HUF_CElt) -> usize {
    elt
}

#[inline]
unsafe fn HUF_setNbBits(elt: *mut HUF_CElt, nbBits: usize) {
    debug_assert!(nbBits <= HUF_TABLELOG_ABSOLUTEMAX as usize);
    *elt = nbBits;
}

#[inline]
unsafe fn HUF_setValue(elt: *mut HUF_CElt, value: usize) {
    let nbBits = HUF_getNbBits(*elt);
    if nbBits > 0 {
        debug_assert!((value >> nbBits) == 0);
        *elt |= value << (core::mem::size_of::<HUF_CElt>() * 8 - nbBits);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTableHeader(ctable: *const HUF_CElt) -> HUF_CTableHeader {
    let mut header = HUF_CTableHeader {
        tableLog: 0,
        maxSymbolValue: 0,
        unused: [0; 6],
    };
    core::ptr::copy_nonoverlapping(
        ctable as *const u8,
        &mut header as *mut HUF_CTableHeader as *mut u8,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
    header
}

unsafe fn HUF_writeCTableHeader(ctable: *mut HUF_CElt, tableLog: u32, maxSymbolValue: u32) {
    let mut header = HUF_CTableHeader {
        tableLog: 0,
        maxSymbolValue: 0,
        unused: [0; 6],
    };
    debug_assert!(tableLog < 256);
    header.tableLog = tableLog as u8;
    debug_assert!(maxSymbolValue < 256);
    header.maxSymbolValue = maxSymbolValue as u8;
    core::ptr::copy_nonoverlapping(
        &header as *const HUF_CTableHeader as *const u8,
        ctable as *mut u8,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
}

// ---------------------------------------------------------------------------
// HUF_writeCTable_wksp
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_writeCTable_wksp(
    dst: *mut c_void,
    maxDstSize: usize,
    CTable: *const HUF_CElt,
    maxSymbolValue: u32,
    huffLog: u32,
    workspace: *mut c_void,
    mut workspaceSize: usize,
) -> usize {
    let ct = CTable.add(1);
    let op = dst as *mut u8;
    let mut n: u32;
    let wksp = HUF_alignUpWorkspace(workspace, &mut workspaceSize, core::mem::align_of::<u32>())
        as *mut HUF_WriteCTableWksp;

    debug_assert!(HUF_readCTableHeader(CTable).maxSymbolValue as u32 == maxSymbolValue);
    debug_assert!(HUF_readCTableHeader(CTable).tableLog as u32 == huffLog);

    /* check conditions */
    if workspaceSize < core::mem::size_of::<HUF_WriteCTableWksp>() {
        return error(code::GENERIC);
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }

    /* convert to weight */
    (*wksp).bitsToWeight[0] = 0;
    n = 1;
    while n < huffLog + 1 {
        (*wksp).bitsToWeight[n as usize] = (huffLog + 1 - n) as u8;
        n += 1;
    }
    n = 0;
    while n < maxSymbolValue {
        (*wksp).huffWeight[n as usize] =
            (*wksp).bitsToWeight[HUF_getNbBits(*ct.add(n as usize))];
        n += 1;
    }

    /* attempt weights compression by FSE */
    if maxDstSize < 1 {
        return error(code::DSTSIZE_TOOSMALL);
    }
    {
        let hSize = HUF_compressWeights(
            op.add(1) as *mut c_void,
            maxDstSize - 1,
            (*wksp).huffWeight.as_ptr() as *const c_void,
            maxSymbolValue as usize,
            &mut (*wksp).wksp as *mut HUF_CompressWeightsWksp as *mut c_void,
            core::mem::size_of_val(&(*wksp).wksp),
        );
        if err_is_error(hSize) != 0 {
            return hSize;
        }
        if (hSize > 1) & (hSize < maxSymbolValue as usize / 2) {
            /* FSE compressed */
            *op.add(0) = hSize as u8;
            return hSize + 1;
        }
    }

    /* write raw values as 4-bits (max : 15) */
    if maxSymbolValue > (256 - 128) {
        return error(code::GENERIC); /* should not happen : likely means source cannot be compressed */
    }
    if ((maxSymbolValue as usize + 1) / 2) + 1 > maxDstSize {
        return error(code::DSTSIZE_TOOSMALL); /* not enough space within dst buffer */
    }
    *op.add(0) = (128 /*special case*/ + (maxSymbolValue - 1)) as u8;
    (*wksp).huffWeight[maxSymbolValue as usize] = 0; /* to be sure it doesn't cause msan issue in final combination */
    n = 0;
    while n < maxSymbolValue {
        *op.add((n as usize / 2) + 1) = (((*wksp).huffWeight[n as usize] as u32) << 4)
            as u8
            + (*wksp).huffWeight[n as usize + 1];
        n += 2;
    }
    ((maxSymbolValue as usize + 1) / 2) + 1
}

// ---------------------------------------------------------------------------
// HUF_readCTable
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTable(
    CTable: *mut HUF_CElt,
    maxSymbolValuePtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
    hasZeroWeights: *mut u32,
) -> usize {
    let mut huffWeight = [0u8; HUF_SYMBOLVALUE_MAX as usize + 1];
    let mut rankVal = [0u32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1];
    let mut tableLog: u32 = 0;
    let mut nbSymbols: u32 = 0;
    let ct = CTable.add(1);

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
    if err_is_error(readSize) != 0 {
        return readSize;
    }
    *hasZeroWeights = (rankVal[0] > 0) as u32;

    /* check result */
    if tableLog > HUF_TABLELOG_MAX {
        return error(code::TABLELOG_TOOLARGE);
    }
    if nbSymbols > *maxSymbolValuePtr + 1 {
        return error(code::MAXSYMBOLVALUE_TOOSMALL);
    }

    *maxSymbolValuePtr = nbSymbols - 1;

    HUF_writeCTableHeader(CTable, tableLog, *maxSymbolValuePtr);

    /* Prepare base value per rank */
    {
        let mut nextRankStart: u32 = 0;
        let mut n = 1u32;
        while n <= tableLog {
            let curr = nextRankStart;
            nextRankStart += rankVal[n as usize] << (n - 1);
            rankVal[n as usize] = curr;
            n += 1;
        }
    }

    /* fill nbBits */
    {
        let mut n = 0u32;
        while n < nbSymbols {
            let w = huffWeight[n as usize] as u32;
            HUF_setNbBits(
                ct.add(n as usize),
                (((tableLog + 1 - w) as u8) & (0u8.wrapping_sub((w != 0) as u8))) as usize,
            );
            n += 1;
        }
    }

    /* fill val */
    {
        let mut nbPerRank = [0u16; HUF_TABLELOG_MAX as usize + 2];
        let mut valPerRank = [0u16; HUF_TABLELOG_MAX as usize + 2];
        {
            let mut n = 0u32;
            while n < nbSymbols {
                nbPerRank[HUF_getNbBits(*ct.add(n as usize))] += 1;
                n += 1;
            }
        }
        /* determine stating value per rank */
        valPerRank[tableLog as usize + 1] = 0; /* for w==0 */
        {
            let mut min: u16 = 0;
            let mut n = tableLog;
            while n > 0 {
                valPerRank[n as usize] = min;
                min += nbPerRank[n as usize];
                min >>= 1;
                n -= 1;
            }
        }
        /* assign value within rank, symbol order */
        {
            let mut n = 0u32;
            while n < nbSymbols {
                let r = HUF_getNbBits(*ct.add(n as usize));
                HUF_setValue(ct.add(n as usize), valPerRank[r] as usize);
                valPerRank[r] += 1;
                n += 1;
            }
        }
    }

    readSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getNbBitsFromCTable(CTable: *const HUF_CElt, symbolValue: u32) -> u32 {
    let ct = CTable.add(1);
    debug_assert!(symbolValue <= HUF_SYMBOLVALUE_MAX);
    if symbolValue > HUF_readCTableHeader(CTable).maxSymbolValue as u32 {
        return 0;
    }
    HUF_getNbBits(*ct.add(symbolValue as usize)) as u32
}

// ---------------------------------------------------------------------------
// HUF_setMaxHeight
// ---------------------------------------------------------------------------

unsafe fn HUF_setMaxHeight(huffNode: *mut nodeElt, lastNonNull: u32, mut targetNbBits: u32) -> u32 {
    let largestBits = (*huffNode.add(lastNonNull as usize)).nbBits as u32;
    /* early exit : no elt > targetNbBits, so the tree is already valid. */
    if largestBits <= targetNbBits {
        return largestBits;
    }

    /* there are several too large elements (at least >= 2) */
    {
        let mut totalCost: i32 = 0;
        let baseCost: u32 = 1 << (largestBits - targetNbBits);
        let mut n: i32 = lastNonNull as i32;

        while (*huffNode.add(n as usize)).nbBits as u32 > targetNbBits {
            totalCost += (baseCost
                - (1 << (largestBits - (*huffNode.add(n as usize)).nbBits as u32)))
                as i32;
            (*huffNode.add(n as usize)).nbBits = targetNbBits as u8;
            n -= 1;
        }
        debug_assert!((*huffNode.add(n as usize)).nbBits as u32 <= targetNbBits);
        while (*huffNode.add(n as usize)).nbBits as u32 == targetNbBits {
            n -= 1;
        }

        /* renorm totalCost from 2^largestBits to 2^targetNbBits */
        debug_assert!((totalCost as u32 & (baseCost - 1)) == 0);
        totalCost >>= largestBits - targetNbBits;
        debug_assert!(totalCost > 0);

        /* repay normalized cost */
        {
            let noSymbol: u32 = 0xF0F0F0F0;
            let mut rankLast = [0u32; HUF_TABLELOG_MAX as usize + 2];

            /* Get pos of last (smallest = lowest cum. count) symbol per rank */
            core::ptr::write_bytes(rankLast.as_mut_ptr() as *mut u8, 0xF0, core::mem::size_of_val(&rankLast));
            {
                let mut currentNbBits = targetNbBits;
                let mut pos = n;
                while pos >= 0 {
                    if (*huffNode.add(pos as usize)).nbBits as u32 >= currentNbBits {
                        pos -= 1;
                        continue;
                    }
                    currentNbBits = (*huffNode.add(pos as usize)).nbBits as u32; /* < targetNbBits */
                    rankLast[(targetNbBits - currentNbBits) as usize] = pos as u32;
                    pos -= 1;
                }
            }

            while totalCost > 0 {
                let mut nBitsToDecrease = highbit32(totalCost as u32) + 1;
                while nBitsToDecrease > 1 {
                    let highPos = rankLast[nBitsToDecrease as usize];
                    let lowPos = rankLast[(nBitsToDecrease - 1) as usize];
                    if highPos == noSymbol {
                        nBitsToDecrease -= 1;
                        continue;
                    }
                    if lowPos == noSymbol {
                        break;
                    }
                    {
                        let highTotal = (*huffNode.add(highPos as usize)).count;
                        let lowTotal = 2 * (*huffNode.add(lowPos as usize)).count;
                        if highTotal <= lowTotal {
                            break;
                        }
                    }
                    nBitsToDecrease -= 1;
                }
                debug_assert!(rankLast[nBitsToDecrease as usize] != noSymbol || nBitsToDecrease == 1);
                while (nBitsToDecrease <= HUF_TABLELOG_MAX)
                    && (rankLast[nBitsToDecrease as usize] == noSymbol)
                {
                    nBitsToDecrease += 1;
                }
                debug_assert!(rankLast[nBitsToDecrease as usize] != noSymbol);
                /* Increase the number of bits to gain back half the rank cost. */
                totalCost -= 1 << (nBitsToDecrease - 1);
                (*huffNode.add(rankLast[nBitsToDecrease as usize] as usize)).nbBits += 1;

                if rankLast[(nBitsToDecrease - 1) as usize] == noSymbol {
                    rankLast[(nBitsToDecrease - 1) as usize] = rankLast[nBitsToDecrease as usize];
                }
                if rankLast[nBitsToDecrease as usize] == 0 {
                    rankLast[nBitsToDecrease as usize] = noSymbol;
                } else {
                    rankLast[nBitsToDecrease as usize] -= 1;
                    if (*huffNode.add(rankLast[nBitsToDecrease as usize] as usize)).nbBits as u32
                        != targetNbBits - nBitsToDecrease
                    {
                        rankLast[nBitsToDecrease as usize] = noSymbol; /* this rank is now empty */
                    }
                }
            } /* while (totalCost > 0) */

            while totalCost < 0 {
                /* Sometimes, cost correction overshoot */
                if rankLast[1] == noSymbol {
                    while (*huffNode.add(n as usize)).nbBits as u32 == targetNbBits {
                        n -= 1;
                    }
                    (*huffNode.add((n + 1) as usize)).nbBits -= 1;
                    debug_assert!(n >= 0);
                    rankLast[1] = (n + 1) as u32;
                    totalCost += 1;
                    continue;
                }
                (*huffNode.add((rankLast[1] + 1) as usize)).nbBits -= 1;
                rankLast[1] += 1;
                totalCost += 1;
            }
        } /* repay normalized cost */
    } /* there are several too large elements (at least >= 2) */

    targetNbBits
}

// ---------------------------------------------------------------------------
// HUF_sort and helpers
// ---------------------------------------------------------------------------

const RANK_POSITION_MAX_COUNT_LOG: u32 = 32;
// RANK_POSITION_LOG_BUCKETS_BEGIN == (192-1) - 32 - 1 == 158
const RANK_POSITION_LOG_BUCKETS_BEGIN: u32 =
    (RANK_POSITION_TABLE_SIZE as u32 - 1) - RANK_POSITION_MAX_COUNT_LOG - 1;
// RANK_POSITION_DISTINCT_COUNT_CUTOFF == 158 + highbit32(158) == 166
fn rank_position_distinct_count_cutoff() -> u32 {
    RANK_POSITION_LOG_BUCKETS_BEGIN + highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN)
}

fn HUF_getIndex(count: u32) -> u32 {
    if count < rank_position_distinct_count_cutoff() {
        count
    } else {
        highbit32(count) + RANK_POSITION_LOG_BUCKETS_BEGIN
    }
}

unsafe fn HUF_swapNodes(a: *mut nodeElt, b: *mut nodeElt) {
    let tmp = *a;
    *a = *b;
    *b = tmp;
}

unsafe fn HUF_isSorted(huffNode: *const nodeElt, maxSymbolValue1: u32) -> i32 {
    let mut i = 1u32;
    while i < maxSymbolValue1 {
        if (*huffNode.add(i as usize)).count > (*huffNode.add((i - 1) as usize)).count {
            return 0;
        }
        i += 1;
    }
    1
}

/* Insertion sort by descending order */
unsafe fn HUF_insertionSort(huffNode: *mut nodeElt, low: i32, high: i32) {
    let size = high - low + 1;
    let huffNode = huffNode.add(low as usize);
    let mut i = 1i32;
    while i < size {
        let key = *huffNode.add(i as usize);
        let mut j = i - 1;
        while j >= 0 && (*huffNode.add(j as usize)).count < key.count {
            *huffNode.add((j + 1) as usize) = *huffNode.add(j as usize);
            j -= 1;
        }
        *huffNode.add((j + 1) as usize) = key;
        i += 1;
    }
}

unsafe fn HUF_quickSortPartition(arr: *mut nodeElt, low: i32, high: i32) -> i32 {
    let pivot = (*arr.add(high as usize)).count;
    let mut i = low - 1;
    let mut j = low;
    while j < high {
        if (*arr.add(j as usize)).count > pivot {
            i += 1;
            HUF_swapNodes(arr.add(i as usize), arr.add(j as usize));
        }
        j += 1;
    }
    HUF_swapNodes(arr.add((i + 1) as usize), arr.add(high as usize));
    i + 1
}

unsafe fn HUF_simpleQuickSort(arr: *mut nodeElt, mut low: i32, mut high: i32) {
    let kInsertionSortThreshold = 8;
    if high - low < kInsertionSortThreshold {
        HUF_insertionSort(arr, low, high);
        return;
    }
    while low < high {
        let idx = HUF_quickSortPartition(arr, low, high);
        if idx - low < high - idx {
            HUF_simpleQuickSort(arr, low, idx - 1);
            low = idx + 1;
        } else {
            HUF_simpleQuickSort(arr, idx + 1, high);
            high = idx - 1;
        }
    }
}

unsafe fn HUF_sort(
    huffNode: *mut nodeElt,
    count: *const u32,
    maxSymbolValue: u32,
    rankPosition: *mut rankPos,
) {
    let mut n: u32;
    let maxSymbolValue1 = maxSymbolValue + 1;

    core::ptr::write_bytes(rankPosition, 0, RANK_POSITION_TABLE_SIZE);
    n = 0;
    while n < maxSymbolValue1 {
        let lowerRank = HUF_getIndex(*count.add(n as usize));
        debug_assert!(lowerRank < RANK_POSITION_TABLE_SIZE as u32 - 1);
        (*rankPosition.add(lowerRank as usize)).base += 1;
        n += 1;
    }

    debug_assert!((*rankPosition.add(RANK_POSITION_TABLE_SIZE - 1)).base == 0);
    /* Set up the rankPosition table */
    n = RANK_POSITION_TABLE_SIZE as u32 - 1;
    while n > 0 {
        (*rankPosition.add((n - 1) as usize)).base += (*rankPosition.add(n as usize)).base;
        (*rankPosition.add((n - 1) as usize)).curr = (*rankPosition.add((n - 1) as usize)).base;
        n -= 1;
    }

    /* Insert each symbol into their appropriate bucket. */
    n = 0;
    while n < maxSymbolValue1 {
        let c = *count.add(n as usize);
        let r = HUF_getIndex(c) + 1;
        let pos = (*rankPosition.add(r as usize)).curr;
        (*rankPosition.add(r as usize)).curr += 1;
        debug_assert!((pos as u32) < maxSymbolValue1);
        (*huffNode.add(pos as usize)).count = c;
        (*huffNode.add(pos as usize)).byte = n as u8;
        n += 1;
    }

    /* Sort each bucket. */
    n = rank_position_distinct_count_cutoff();
    while n < RANK_POSITION_TABLE_SIZE as u32 - 1 {
        let bucketSize =
            (*rankPosition.add(n as usize)).curr as i32 - (*rankPosition.add(n as usize)).base as i32;
        let bucketStartIdx = (*rankPosition.add(n as usize)).base;
        if bucketSize > 1 {
            debug_assert!((bucketStartIdx as u32) < maxSymbolValue1);
            HUF_simpleQuickSort(huffNode.add(bucketStartIdx as usize), 0, bucketSize - 1);
        }
        n += 1;
    }

    debug_assert!(HUF_isSorted(huffNode, maxSymbolValue1) != 0);
}

// ---------------------------------------------------------------------------
// HUF_buildTree / HUF_buildCTableFromTree / HUF_buildCTable_wksp
// ---------------------------------------------------------------------------

const STARTNODE: i32 = HUF_SYMBOLVALUE_MAX as i32 + 1;

unsafe fn HUF_buildTree(huffNode: *mut nodeElt, maxSymbolValue: u32) -> i32 {
    let huffNode0 = huffNode.offset(-1);
    let mut nonNullRank: i32;
    let mut lowS: i32;
    let mut lowN: i32;
    let mut nodeNb: i32 = STARTNODE;
    let mut n: i32;
    let nodeRoot: i32;
    /* init for parents */
    nonNullRank = maxSymbolValue as i32;
    while (*huffNode.add(nonNullRank as usize)).count == 0 {
        nonNullRank -= 1;
    }
    lowS = nonNullRank;
    nodeRoot = nodeNb + lowS - 1;
    lowN = nodeNb;
    (*huffNode.add(nodeNb as usize)).count =
        (*huffNode.add(lowS as usize)).count + (*huffNode.add((lowS - 1) as usize)).count;
    (*huffNode.add(lowS as usize)).parent = nodeNb as u16;
    (*huffNode.add((lowS - 1) as usize)).parent = nodeNb as u16;
    nodeNb += 1;
    lowS -= 2;
    n = nodeNb;
    while n <= nodeRoot {
        (*huffNode.add(n as usize)).count = 1u32 << 30;
        n += 1;
    }
    (*huffNode0.add(0)).count = 1u32 << 31; /* fake entry, strong barrier */

    /* create parents */
    while nodeNb <= nodeRoot {
        let n1 = if (*huffNode.add(lowS as usize)).count < (*huffNode.add(lowN as usize)).count {
            let t = lowS;
            lowS -= 1;
            t
        } else {
            let t = lowN;
            lowN += 1;
            t
        };
        let n2 = if (*huffNode.add(lowS as usize)).count < (*huffNode.add(lowN as usize)).count {
            let t = lowS;
            lowS -= 1;
            t
        } else {
            let t = lowN;
            lowN += 1;
            t
        };
        (*huffNode.add(nodeNb as usize)).count =
            (*huffNode.add(n1 as usize)).count + (*huffNode.add(n2 as usize)).count;
        (*huffNode.add(n1 as usize)).parent = nodeNb as u16;
        (*huffNode.add(n2 as usize)).parent = nodeNb as u16;
        nodeNb += 1;
    }

    /* distribute weights (unlimited tree height) */
    (*huffNode.add(nodeRoot as usize)).nbBits = 0;
    n = nodeRoot - 1;
    while n >= STARTNODE {
        let parent = (*huffNode.add(n as usize)).parent;
        (*huffNode.add(n as usize)).nbBits = (*huffNode.add(parent as usize)).nbBits + 1;
        n -= 1;
    }
    n = 0;
    while n <= nonNullRank {
        let parent = (*huffNode.add(n as usize)).parent;
        (*huffNode.add(n as usize)).nbBits = (*huffNode.add(parent as usize)).nbBits + 1;
        n += 1;
    }

    nonNullRank
}

unsafe fn HUF_buildCTableFromTree(
    CTable: *mut HUF_CElt,
    huffNode: *const nodeElt,
    nonNullRank: i32,
    maxSymbolValue: u32,
    maxNbBits: u32,
) {
    let ct = CTable.add(1);
    /* fill result into ctable (val, nbBits) */
    let mut n: i32;
    let mut nbPerRank = [0u16; HUF_TABLELOG_MAX as usize + 1];
    let mut valPerRank = [0u16; HUF_TABLELOG_MAX as usize + 1];
    let alphabetSize = (maxSymbolValue + 1) as i32;
    n = 0;
    while n <= nonNullRank {
        nbPerRank[(*huffNode.add(n as usize)).nbBits as usize] += 1;
        n += 1;
    }
    /* determine starting value per rank */
    {
        let mut min: u16 = 0;
        n = maxNbBits as i32;
        while n > 0 {
            valPerRank[n as usize] = min;
            min += nbPerRank[n as usize];
            min >>= 1;
            n -= 1;
        }
    }
    n = 0;
    while n < alphabetSize {
        HUF_setNbBits(
            ct.add((*huffNode.add(n as usize)).byte as usize),
            (*huffNode.add(n as usize)).nbBits as usize,
        );
        n += 1;
    }
    n = 0;
    while n < alphabetSize {
        let r = HUF_getNbBits(*ct.add(n as usize));
        HUF_setValue(ct.add(n as usize), valPerRank[r] as usize);
        valPerRank[r] += 1;
        n += 1;
    }

    HUF_writeCTableHeader(CTable, maxNbBits, maxSymbolValue);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_buildCTable_wksp(
    CTable: *mut HUF_CElt,
    count: *const u32,
    maxSymbolValue: u32,
    mut maxNbBits: u32,
    workSpace: *mut c_void,
    mut wkspSize: usize,
) -> usize {
    let wksp_tables = HUF_alignUpWorkspace(workSpace, &mut wkspSize, core::mem::align_of::<u32>())
        as *mut HUF_buildCTable_wksp_tables;
    let huffNode0 = (*wksp_tables).huffNodeTbl.as_mut_ptr();
    let huffNode = huffNode0.add(1);
    let nonNullRank: i32;

    /* safety checks */
    if wkspSize < core::mem::size_of::<HUF_buildCTable_wksp_tables>() {
        return error(code::WORKSPACE_TOOSMALL);
    }
    if maxNbBits == 0 {
        maxNbBits = HUF_TABLELOG_DEFAULT;
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }
    core::ptr::write_bytes(
        huffNode0,
        0,
        HUFNODE_TABLE_SIZE,
    );

    /* sort, decreasing order */
    HUF_sort(huffNode, count, maxSymbolValue, (*wksp_tables).rankPosition.as_mut_ptr());

    /* build tree */
    nonNullRank = HUF_buildTree(huffNode, maxSymbolValue);

    /* determine and enforce maxTableLog */
    maxNbBits = HUF_setMaxHeight(huffNode, nonNullRank as u32, maxNbBits);
    if maxNbBits > HUF_TABLELOG_MAX {
        return error(code::GENERIC); /* check fit into table */
    }

    HUF_buildCTableFromTree(CTable, huffNode, nonNullRank, maxSymbolValue, maxNbBits);

    maxNbBits as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_estimateCompressedSize(
    CTable: *const HUF_CElt,
    count: *const u32,
    maxSymbolValue: u32,
) -> usize {
    let ct = CTable.add(1);
    let mut nbBits: usize = 0;
    let mut s = 0i32;
    while s <= maxSymbolValue as i32 {
        nbBits += HUF_getNbBits(*ct.add(s as usize)) * *count.add(s as usize) as usize;
        s += 1;
    }
    nbBits >> 3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_validateCTable(
    CTable: *const HUF_CElt,
    count: *const u32,
    maxSymbolValue: u32,
) -> i32 {
    let header = HUF_readCTableHeader(CTable);
    let ct = CTable.add(1);
    let mut bad: i32 = 0;
    let mut s: i32;

    debug_assert!(header.tableLog as u32 <= HUF_TABLELOG_ABSOLUTEMAX);

    if (header.maxSymbolValue as u32) < maxSymbolValue {
        return 0;
    }

    s = 0;
    while s <= maxSymbolValue as i32 {
        bad |= (*count.add(s as usize) != 0) as i32 & (HUF_getNbBits(*ct.add(s as usize)) == 0) as i32;
        s += 1;
    }
    (bad == 0) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_compressBound(size: usize) -> usize {
    HUF_CTABLEBOUND + HUF_BLOCKBOUND(size)
}

// ---------------------------------------------------------------------------
// HUF_CStream_t : Huffman bitstream
// ---------------------------------------------------------------------------

const HUF_BITS_IN_CONTAINER: usize = core::mem::size_of::<usize>() * 8;

#[repr(C)]
struct HUF_CStream_t {
    bitContainer: [usize; 2],
    bitPos: [usize; 2],
    startPtr: *mut u8,
    ptr: *mut u8,
    endPtr: *mut u8,
}

unsafe fn HUF_initCStream(bitC: *mut HUF_CStream_t, startPtr: *mut c_void, dstCapacity: usize) -> usize {
    core::ptr::write_bytes(bitC as *mut u8, 0, core::mem::size_of::<HUF_CStream_t>());
    (*bitC).startPtr = startPtr as *mut u8;
    (*bitC).ptr = (*bitC).startPtr;
    (*bitC).endPtr = (*bitC)
        .startPtr
        .add(dstCapacity - core::mem::size_of::<usize>());
    if dstCapacity <= core::mem::size_of::<usize>() {
        return error(code::DSTSIZE_TOOSMALL);
    }
    0
}

#[inline(always)]
unsafe fn HUF_addBits(bitC: *mut HUF_CStream_t, elt: HUF_CElt, idx: usize, kFast: i32) {
    debug_assert!(idx <= 1);
    debug_assert!(HUF_getNbBits(elt) <= HUF_TABLELOG_ABSOLUTEMAX as usize);
    (*bitC).bitContainer[idx] >>= HUF_getNbBits(elt);
    (*bitC).bitContainer[idx] |= if kFast != 0 {
        HUF_getValueFast(elt)
    } else {
        HUF_getValue(elt)
    };
    (*bitC).bitPos[idx] += HUF_getNbBitsFast(elt);
    debug_assert!(((*bitC).bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);
}

#[inline(always)]
unsafe fn HUF_zeroIndex1(bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[1] = 0;
    (*bitC).bitPos[1] = 0;
}

#[inline(always)]
unsafe fn HUF_mergeIndex1(bitC: *mut HUF_CStream_t) {
    debug_assert!(((*bitC).bitPos[1] & 0xFF) < HUF_BITS_IN_CONTAINER);
    (*bitC).bitContainer[0] >>= (*bitC).bitPos[1] & 0xFF;
    (*bitC).bitContainer[0] |= (*bitC).bitContainer[1];
    (*bitC).bitPos[0] += (*bitC).bitPos[1];
    debug_assert!(((*bitC).bitPos[0] & 0xFF) <= HUF_BITS_IN_CONTAINER);
}

#[inline(always)]
unsafe fn HUF_flushBits(bitC: *mut HUF_CStream_t, kFast: i32) {
    let nbBits = (*bitC).bitPos[0] & 0xFF;
    let nbBytes = nbBits >> 3;
    let bitContainer = (*bitC).bitContainer[0] >> (HUF_BITS_IN_CONTAINER - nbBits);
    (*bitC).bitPos[0] &= 7;
    debug_assert!(nbBits > 0);
    debug_assert!(nbBits <= core::mem::size_of::<usize>() * 8);
    debug_assert!((*bitC).ptr <= (*bitC).endPtr);
    mem_write_le_st((*bitC).ptr as *mut c_void, bitContainer);
    (*bitC).ptr = (*bitC).ptr.add(nbBytes);
    debug_assert!(kFast == 0 || (*bitC).ptr <= (*bitC).endPtr);
    if kFast == 0 && (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
}

unsafe fn HUF_endMark() -> HUF_CElt {
    let mut endMark: HUF_CElt = 0;
    HUF_setNbBits(&mut endMark, 1);
    HUF_setValue(&mut endMark, 1);
    endMark
}

unsafe fn HUF_closeCStream(bitC: *mut HUF_CStream_t) -> usize {
    HUF_addBits(bitC, HUF_endMark(), 0, 0);
    HUF_flushBits(bitC, 0);
    {
        let nbBits = (*bitC).bitPos[0] & 0xFF;
        if (*bitC).ptr >= (*bitC).endPtr {
            return 0; /* overflow detected */
        }
        ((*bitC).ptr as usize - (*bitC).startPtr as usize) + (nbBits > 0) as usize
    }
}

#[inline(always)]
unsafe fn HUF_encodeSymbol(
    bitCPtr: *mut HUF_CStream_t,
    symbol: u32,
    CTable: *const HUF_CElt,
    idx: usize,
    fast: i32,
) {
    HUF_addBits(bitCPtr, *CTable.add(symbol as usize), idx, fast);
}

unsafe fn HUF_compress1X_usingCTable_internal_body_loop(
    bitC: *mut HUF_CStream_t,
    ip: *const u8,
    srcSize: usize,
    ct: *const HUF_CElt,
    kUnroll: i32,
    kFastFlush: i32,
    kLastFast: i32,
) {
    /* Join to kUnroll */
    let mut n = srcSize as i32;
    let mut rem = n % kUnroll;
    if rem > 0 {
        while rem > 0 {
            n -= 1;
            HUF_encodeSymbol(bitC, *ip.add(n as usize) as u32, ct, 0, 0);
            rem -= 1;
        }
        HUF_flushBits(bitC, kFastFlush);
    }
    debug_assert!(n % kUnroll == 0);

    /* Join to 2 * kUnroll */
    if n % (2 * kUnroll) != 0 {
        let mut u = 1;
        while u < kUnroll {
            HUF_encodeSymbol(bitC, *ip.add((n - u) as usize) as u32, ct, 0, 1);
            u += 1;
        }
        HUF_encodeSymbol(bitC, *ip.add((n - kUnroll) as usize) as u32, ct, 0, kLastFast);
        HUF_flushBits(bitC, kFastFlush);
        n -= kUnroll;
    }
    debug_assert!(n % (2 * kUnroll) == 0);

    while n > 0 {
        /* Encode kUnroll symbols into the bitstream @ index 0. */
        let mut u = 1;
        while u < kUnroll {
            HUF_encodeSymbol(bitC, *ip.add((n - u) as usize) as u32, ct, 0, 1);
            u += 1;
        }
        HUF_encodeSymbol(bitC, *ip.add((n - kUnroll) as usize) as u32, ct, 0, kLastFast);
        HUF_flushBits(bitC, kFastFlush);
        /* Encode kUnroll symbols into the bitstream @ index 1. */
        HUF_zeroIndex1(bitC);
        let mut u = 1;
        while u < kUnroll {
            HUF_encodeSymbol(bitC, *ip.add((n - kUnroll - u) as usize) as u32, ct, 1, 1);
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.add((n - kUnroll - kUnroll) as usize) as u32,
            ct,
            1,
            kLastFast,
        );
        /* Merge bitstream @ index 1 into the bitstream @ index 0 */
        HUF_mergeIndex1(bitC);
        HUF_flushBits(bitC, kFastFlush);

        n -= 2 * kUnroll;
    }
    debug_assert!(n == 0);
}

fn HUF_tightCompressBound(srcSize: usize, tableLog: usize) -> usize {
    ((srcSize * tableLog) >> 3) + 8
}

unsafe fn HUF_compress1X_usingCTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
) -> usize {
    let tableLog = HUF_readCTableHeader(CTable).tableLog as u32;
    let ct = CTable.add(1);
    let ip = src as *const u8;
    let ostart = dst as *mut u8;
    let oend = ostart.add(dstSize);
    let mut bitC: HUF_CStream_t = core::mem::zeroed();

    /* init */
    if dstSize < 8 {
        return 0; /* not enough space to compress */
    }
    {
        let op = ostart;
        let initErr = HUF_initCStream(&mut bitC, op as *mut c_void, oend as usize - op as usize);
        if err_is_error(initErr) != 0 {
            return 0;
        }
    }

    if dstSize < HUF_tightCompressBound(srcSize, tableLog as usize) || tableLog > 11 {
        HUF_compress1X_usingCTable_internal_body_loop(
            &mut bitC,
            ip,
            srcSize,
            ct,
            if mem_32bits() != 0 { 2 } else { 4 },
            0,
            0,
        );
    } else {
        if mem_32bits() != 0 {
            match tableLog {
                11 => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 2, 1, 0),
                10 | 9 | 8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 2, 1, 1)
                }
                _ => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 3, 1, 1),
            }
        } else {
            match tableLog {
                11 => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 5, 1, 0),
                10 => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 5, 1, 1),
                9 => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 6, 1, 0),
                8 => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 7, 1, 0),
                7 => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 8, 1, 0),
                _ => HUF_compress1X_usingCTable_internal_body_loop(&mut bitC, ip, srcSize, ct, 9, 1, 1),
            }
        }
    }
    debug_assert!(bitC.ptr <= bitC.endPtr);

    HUF_closeCStream(&mut bitC)
}

// DYNAMIC_BMI2 == 0 : default path only
unsafe fn HUF_compress1X_usingCTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: i32,
) -> usize {
    let _ = flags;
    HUF_compress1X_usingCTable_internal_body(dst, dstSize, src, srcSize, CTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_usingCTable(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: i32,
) -> usize {
    HUF_compress1X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

// ---------------------------------------------------------------------------
// HUF_compress4X_usingCTable
// ---------------------------------------------------------------------------

unsafe fn HUF_compress4X_usingCTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: i32,
) -> usize {
    let segmentSize = (srcSize + 3) / 4; /* first 3 segments */
    let mut ip = src as *const u8;
    let iend = ip.add(srcSize);
    let ostart = dst as *mut u8;
    let oend = ostart.add(dstSize);
    let mut op = ostart;

    if dstSize < 6 + 1 + 1 + 1 + 8 {
        return 0; /* minimum space to compress successfully */
    }
    if srcSize < 12 {
        return 0; /* no saving possible : too small input */
    }
    op = op.add(6); /* jumpTable */

    debug_assert!(op <= oend);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            segmentSize,
            CTable,
            flags,
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        mem_write_le16(ostart as *mut c_void, cSize as u16);
        op = op.add(cSize);
    }

    ip = ip.add(segmentSize);
    debug_assert!(op <= oend);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            segmentSize,
            CTable,
            flags,
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        mem_write_le16(ostart.add(2) as *mut c_void, cSize as u16);
        op = op.add(cSize);
    }

    ip = ip.add(segmentSize);
    debug_assert!(op <= oend);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            segmentSize,
            CTable,
            flags,
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        mem_write_le16(ostart.add(4) as *mut c_void, cSize as u16);
        op = op.add(cSize);
    }

    ip = ip.add(segmentSize);
    debug_assert!(op <= oend);
    debug_assert!(ip <= iend);
    {
        let cSize = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            iend as usize - ip as usize,
            CTable,
            flags,
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }
        if cSize == 0 || cSize > 65535 {
            return 0;
        }
        op = op.add(cSize);
    }

    op as usize - ostart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_usingCTable(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    CTable: *const HUF_CElt,
    flags: i32,
) -> usize {
    HUF_compress4X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags)
}

unsafe fn HUF_compressCTable_internal(
    ostart: *mut u8,
    mut op: *mut u8,
    oend: *mut u8,
    src: *const c_void,
    srcSize: usize,
    nbStreams: HUF_nbStreams_e,
    CTable: *const HUF_CElt,
    flags: i32,
) -> usize {
    let cSize = if nbStreams == HUF_singleStream {
        HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            src,
            srcSize,
            CTable,
            flags,
        )
    } else {
        HUF_compress4X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            src,
            srcSize,
            CTable,
            flags,
        )
    };
    if err_is_error(cSize) != 0 {
        return cSize;
    }
    if cSize == 0 {
        return 0; /* uncompressible */
    }
    op = op.add(cSize);
    /* check compressibility */
    debug_assert!(op >= ostart);
    if (op as usize - ostart as usize) >= srcSize - 1 {
        return 0;
    }
    op as usize - ostart as usize
}

// ---------------------------------------------------------------------------
// HUF_compress_tables_t workspace union
// ---------------------------------------------------------------------------

// union { buildCTable_wksp; writeCTable_wksp; hist_wksp[HIST_WKSP_SIZE_U32] }
const WKSPS_UNION_SIZE: usize = {
    let a = core::mem::size_of::<HUF_buildCTable_wksp_tables>();
    let b = core::mem::size_of::<HUF_WriteCTableWksp>();
    let c = HIST_WKSP_SIZE_U32 * core::mem::size_of::<u32>();
    let ab = if a > b { a } else { b };
    if ab > c {
        ab
    } else {
        c
    }
};

#[repr(C)]
struct HUF_compress_tables_t {
    count: [u32; HUF_SYMBOLVALUE_MAX as usize + 1],
    CTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX as usize)],
    wksps: WkspsUnion,
}

#[repr(C)]
union WkspsUnion {
    buildCTable_wksp: core::mem::ManuallyDrop<HUF_buildCTable_wksp_tables>,
    writeCTable_wksp: core::mem::ManuallyDrop<HUF_WriteCTableWksp>,
    hist_wksp: [u32; HIST_WKSP_SIZE_U32],
}

const SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE: usize = 4096;
const SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO: usize = 10;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_cardinality(count: *const u32, maxSymbolValue: u32) -> u32 {
    let mut cardinality: u32 = 0;
    let mut i: u32 = 0;
    while i < maxSymbolValue + 1 {
        if *count.add(i as usize) != 0 {
            cardinality += 1;
        }
        i += 1;
    }
    cardinality
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_minTableLog(symbolCardinality: u32) -> u32 {
    let minBitsSymbols = highbit32(symbolCardinality) + 1;
    minBitsSymbols
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_optimalTableLog(
    maxTableLog: u32,
    srcSize: usize,
    maxSymbolValue: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
    table: *mut HUF_CElt,
    count: *const u32,
    flags: i32,
) -> u32 {
    debug_assert!(srcSize > 1);
    debug_assert!(wkspSize >= core::mem::size_of::<HUF_buildCTable_wksp_tables>());

    if (flags & HUF_flags_optimalDepth) == 0 {
        /* cheap evaluation, based on FSE */
        return FSE_optimalTableLog_internal(maxTableLog, srcSize, maxSymbolValue, 1);
    }

    {
        let dst = (workSpace as *mut u8).add(core::mem::size_of::<HUF_WriteCTableWksp>());
        let dstSize = wkspSize - core::mem::size_of::<HUF_WriteCTableWksp>();
        let mut hSize: usize;
        let mut newSize: usize;
        let symbolCardinality = HUF_cardinality(count, maxSymbolValue);
        let minTableLog = HUF_minTableLog(symbolCardinality);
        let mut optSize: usize = (!0usize) - 1;
        let mut optLog: u32 = maxTableLog;
        let mut optLogGuess: u32;

        /* Search until size increases */
        optLogGuess = minTableLog;
        while optLogGuess <= maxTableLog {
            {
                let maxBits =
                    HUF_buildCTable_wksp(table, count, maxSymbolValue, optLogGuess, workSpace, wkspSize);
                if err_is_error(maxBits) != 0 {
                    optLogGuess += 1;
                    continue;
                }

                if (maxBits as u32) < optLogGuess && optLogGuess > minTableLog {
                    break;
                }

                hSize = HUF_writeCTable_wksp(
                    dst as *mut c_void,
                    dstSize,
                    table,
                    maxSymbolValue,
                    maxBits as u32,
                    workSpace,
                    wkspSize,
                );
            }

            if err_is_error(hSize) != 0 {
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
        debug_assert!(optLog <= HUF_TABLELOG_MAX);
        optLog
    }
}

// ---------------------------------------------------------------------------
// HUF_compress_internal + public repeat entry points
// ---------------------------------------------------------------------------

unsafe fn HUF_compress_internal(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    mut maxSymbolValue: u32,
    mut huffLog: u32,
    nbStreams: HUF_nbStreams_e,
    workSpace: *mut c_void,
    mut wkspSize: usize,
    oldHufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: i32,
) -> usize {
    let table = HUF_alignUpWorkspace(workSpace, &mut wkspSize, core::mem::align_of::<usize>())
        as *mut HUF_compress_tables_t;
    let ostart = dst as *mut u8;
    let oend = ostart.add(dstSize);
    let mut op = ostart;

    /* checks & inits */
    if wkspSize < core::mem::size_of::<HUF_compress_tables_t>() {
        return error(code::WORKSPACE_TOOSMALL);
    }
    if srcSize == 0 {
        return 0; /* Uncompressed */
    }
    if dstSize == 0 {
        return 0; /* cannot fit anything within dst budget */
    }
    if srcSize > HUF_BLOCKSIZE_MAX {
        return error(code::SRCSIZE_WRONG); /* current block size limit */
    }
    if huffLog > HUF_TABLELOG_MAX {
        return error(code::TABLELOG_TOOLARGE);
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX {
        return error(code::MAXSYMBOLVALUE_TOOLARGE);
    }
    if maxSymbolValue == 0 {
        maxSymbolValue = HUF_SYMBOLVALUE_MAX;
    }
    if huffLog == 0 {
        huffLog = HUF_TABLELOG_DEFAULT;
    }

    /* Heuristic : If old table is valid, use it for small inputs */
    if (flags & HUF_flags_preferRepeat) != 0
        && !repeat.is_null()
        && *repeat == HUF_repeat_valid
    {
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
            let mut maxSymbolValueBegin = maxSymbolValue;
            let largestBegin = HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut maxSymbolValueBegin,
                src,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as usize;
            largestTotal += largestBegin;
        }
        {
            let mut maxSymbolValueEnd = maxSymbolValue;
            let largestEnd = HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut maxSymbolValueEnd,
                (src as *const u8).add(srcSize - SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE)
                    as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            ) as usize;
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
            src,
            srcSize,
            (*table).wksps.hist_wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.hist_wksp),
        );
        if err_is_error(largest) != 0 {
            return largest;
        }
        if largest == srcSize {
            *ostart = *(src as *const u8);
            return 1;
        } /* single symbol, rle */
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
        &mut (*table).wksps as *mut WkspsUnion as *mut c_void,
        core::mem::size_of_val(&(*table).wksps),
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
            &mut *(*table).wksps.buildCTable_wksp as *mut HUF_buildCTable_wksp_tables
                as *mut c_void,
            core::mem::size_of::<HUF_buildCTable_wksp_tables>(),
        );
        if err_is_error(maxBits) != 0 {
            return maxBits;
        }
        huffLog = maxBits as u32;
    }

    /* Write table description header */
    {
        let hSize = HUF_writeCTable_wksp(
            op as *mut c_void,
            dstSize,
            (*table).CTable.as_ptr(),
            maxSymbolValue,
            huffLog,
            &mut *(*table).wksps.writeCTable_wksp as *mut HUF_WriteCTableWksp as *mut c_void,
            core::mem::size_of::<HUF_WriteCTableWksp>(),
        );
        if err_is_error(hSize) != 0 {
            return hSize;
        }
        /* Check if using previous huffman table is beneficial */
        if !repeat.is_null() && *repeat != HUF_repeat_none {
            let oldSize =
                HUF_estimateCompressedSize(oldHufTable, (*table).count.as_ptr(), maxSymbolValue);
            let newSize = HUF_estimateCompressedSize(
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
            core::ptr::copy_nonoverlapping(
                (*table).CTable.as_ptr(),
                oldHufTable,
                (*table).CTable.len(),
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
    maxSymbolValue: u32,
    huffLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: i32,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_repeat(
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    maxSymbolValue: u32,
    huffLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
    hufTable: *mut HUF_CElt,
    repeat: *mut HUF_repeat,
    flags: i32,
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






