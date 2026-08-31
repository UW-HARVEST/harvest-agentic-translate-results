//! Translation of `compress/huf_compress.c`.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::zstd_highbit32;
use crate::error::*;
use crate::fse::*;
use crate::huf::*;
use crate::mem::*;

/* ==========================================================================
 * `HUF_CTableHeader` — defined in `common/huf.h` (not present in `crate::huf`),
 * so it is reproduced here exactly.
 *   typedef struct {
 *       BYTE tableLog;
 *       BYTE maxSymbolValue;
 *       BYTE unused[sizeof(size_t) - 2];
 *   } HUF_CTableHeader;
 * ========================================================================== */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_CTableHeader {
    pub tableLog: BYTE,
    pub maxSymbolValue: BYTE,
    pub unused: [BYTE; core::mem::size_of::<usize>() - 2],
}

/* ==========================================================================
 * FSE workspace-sizing macros used by this file (from `common/fse.h`).
 * ========================================================================== */

/// `FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue)`
const fn fse_ctable_size_u32_local(max_table_log: u32, max_symbol_value: u32) -> usize {
    (1 + (1u32 << (max_table_log - 1)) + ((max_symbol_value + 1) * 2)) as usize
}

/// `FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)`
const fn fse_build_ctable_workspace_size_u32(max_symbol_value: u32, table_log: u32) -> usize {
    (((max_symbol_value + 2) as u64 + (1u64 << table_log)) / 2
        + (core::mem::size_of::<U64>() / core::mem::size_of::<U32>()) as u64) as usize
}

/// `HIST_WKSP_SIZE_U32` (from `compress/hist.h`)
const HIST_WKSP_SIZE_U32: usize = 1024;

/* ==========================================================================
 * Required declarations
 * ========================================================================== */

/// `nodeElt` — the Huffman tree node.
#[repr(C)]
#[derive(Clone, Copy)]
struct nodeElt {
    count: U32,
    parent: U16,
    byte: BYTE,
    nbBits: BYTE,
}

impl Default for nodeElt {
    fn default() -> Self {
        nodeElt {
            count: 0,
            parent: 0,
            byte: 0,
            nbBits: 0,
        }
    }
}

/* ==========================================================================
 * HUF : Huffman block compression
 * ========================================================================== */

const HUF_WORKSPACE_MAX_ALIGNMENT: usize = 8;

/// `HUF_alignUpWorkspace()`
unsafe fn HUF_alignUpWorkspace(
    workspace: *mut c_void,
    workspace_size_ptr: *mut usize,
    align: usize,
) -> *mut c_void {
    let mask = align - 1;
    let rem = (workspace as usize) & mask;
    let add = (align - rem) & mask;
    let aligned = (workspace as *mut u8).add(add);
    /* assert((align & (align - 1)) == 0);  pow2 */
    /* assert(align <= HUF_WORKSPACE_MAX_ALIGNMENT); */
    if *workspace_size_ptr >= add {
        *workspace_size_ptr -= add;
        aligned as *mut c_void
    } else {
        *workspace_size_ptr = 0;
        core::ptr::null_mut()
    }
}

const MAX_FSE_TABLELOG_FOR_HUFF_HEADER: u32 = 6;

/// `HUF_CompressWeightsWksp`
#[repr(C)]
struct HUF_CompressWeightsWksp {
    CTable: [FSE_CTable;
        fse_ctable_size_u32_local(MAX_FSE_TABLELOG_FOR_HUFF_HEADER, HUF_TABLELOG_MAX)],
    scratchBuffer: [U32;
        fse_build_ctable_workspace_size_u32(HUF_TABLELOG_MAX, MAX_FSE_TABLELOG_FOR_HUFF_HEADER)],
    count: [c_uint; HUF_TABLELOG_MAX as usize + 1],
    norm: [S16; HUF_TABLELOG_MAX as usize + 1],
}

/// `HUF_compressWeights()`
unsafe fn HUF_compressWeights(
    dst: *mut c_void,
    dst_size: usize,
    weight_table: *const c_void,
    wt_size: usize,
    workspace: *mut c_void,
    mut workspace_size: usize,
) -> usize {
    let ostart = dst as *mut u8;
    let mut op = ostart;
    let oend = ostart.add(dst_size);

    let mut max_symbol_value: c_uint = HUF_TABLELOG_MAX;
    let mut table_log: U32 = MAX_FSE_TABLELOG_FOR_HUFF_HEADER;
    let wksp = HUF_alignUpWorkspace(
        workspace,
        &mut workspace_size,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_CompressWeightsWksp;

    if workspace_size < core::mem::size_of::<HUF_CompressWeightsWksp>() {
        return err_code(ZSTD_error_GENERIC);
    }

    /* init conditions */
    if wt_size <= 1 {
        return 0; /* Not compressible */
    }

    /* Scan input and build symbol stats */
    {
        let max_count = crate::fse_compress::HIST_count_simple(
            (*wksp).count.as_mut_ptr(),
            &mut max_symbol_value,
            weight_table,
            wt_size,
        ); /* never fails */
        if max_count as usize == wt_size {
            return 1; /* only a single symbol in src : rle */
        }
        if max_count == 1 {
            return 0; /* each symbol present maximum once => not compressible */
        }
    }

    table_log =
        crate::fse_compress::FSE_optimalTableLog(table_log, wt_size, max_symbol_value);
    {
        let e = crate::fse_compress::FSE_normalizeCount(
            (*wksp).norm.as_mut_ptr(),
            table_log,
            (*wksp).count.as_ptr(),
            wt_size,
            max_symbol_value,
            0, /* useLowProbCount */
        );
        if err_is_error(e) {
            return e;
        }
    }

    /* Write table description header */
    {
        let h_size = crate::fse_compress::FSE_writeNCount(
            op as *mut c_void,
            oend as usize - op as usize,
            (*wksp).norm.as_ptr(),
            max_symbol_value,
            table_log,
        );
        if err_is_error(h_size) {
            return h_size;
        }
        op = op.add(h_size);
    }

    /* Compress */
    {
        let e = crate::fse_compress::FSE_buildCTable_wksp(
            (*wksp).CTable.as_mut_ptr(),
            (*wksp).norm.as_ptr(),
            max_symbol_value,
            table_log,
            (*wksp).scratchBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*wksp).scratchBuffer),
        );
        if err_is_error(e) {
            return e;
        }
    }
    {
        let c_size = crate::fse_compress::FSE_compress_usingCTable(
            op as *mut c_void,
            oend as usize - op as usize,
            weight_table,
            wt_size,
            (*wksp).CTable.as_ptr(),
        );
        if err_is_error(c_size) {
            return c_size;
        }
        if c_size == 0 {
            return 0; /* not enough space for compressed data */
        }
        op = op.add(c_size);
    }

    op as usize - ostart as usize
}

/// `HUF_getNbBits()`
fn HUF_getNbBits(elt: HUF_CElt) -> usize {
    elt & 0xFF
}

/// `HUF_getNbBitsFast()`
fn HUF_getNbBitsFast(elt: HUF_CElt) -> usize {
    elt
}

/// `HUF_getValue()`
fn HUF_getValue(elt: HUF_CElt) -> usize {
    elt & !(0xFFusize)
}

/// `HUF_getValueFast()`
fn HUF_getValueFast(elt: HUF_CElt) -> usize {
    elt
}

/// `HUF_setNbBits()`
fn HUF_setNbBits(elt: *mut HUF_CElt, nb_bits: usize) {
    /* assert(nbBits <= HUF_TABLELOG_ABSOLUTEMAX); */
    unsafe {
        *elt = nb_bits;
    }
}

/// `HUF_setValue()`
fn HUF_setValue(elt: *mut HUF_CElt, value: usize) {
    unsafe {
        let nb_bits = HUF_getNbBits(*elt);
        if nb_bits > 0 {
            /* assert((value >> nbBits) == 0); */
            *elt |= value << (core::mem::size_of::<HUF_CElt>() * 8 - nb_bits);
        }
    }
}

/// `HUF_readCTableHeader()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTableHeader(ctable: *const HUF_CElt) -> HUF_CTableHeader {
    let mut header = HUF_CTableHeader {
        tableLog: 0,
        maxSymbolValue: 0,
        unused: [0; core::mem::size_of::<usize>() - 2],
    };
    core::ptr::copy_nonoverlapping(
        ctable as *const u8,
        &mut header as *mut HUF_CTableHeader as *mut u8,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
    header
}

/// `HUF_writeCTableHeader()`
unsafe fn HUF_writeCTableHeader(ctable: *mut HUF_CElt, table_log: U32, max_symbol_value: U32) {
    let mut header = HUF_CTableHeader {
        tableLog: 0,
        maxSymbolValue: 0,
        unused: [0; core::mem::size_of::<usize>() - 2],
    };
    core::ptr::write_bytes(&mut header as *mut HUF_CTableHeader as *mut u8, 0, core::mem::size_of::<HUF_CTableHeader>());
    /* assert(tableLog < 256); */
    header.tableLog = table_log as BYTE;
    /* assert(maxSymbolValue < 256); */
    header.maxSymbolValue = max_symbol_value as BYTE;
    core::ptr::copy_nonoverlapping(
        &header as *const HUF_CTableHeader as *const u8,
        ctable as *mut u8,
        core::mem::size_of::<HUF_CTableHeader>(),
    );
}

/// `HUF_WriteCTableWksp`
#[repr(C)]
struct HUF_WriteCTableWksp {
    wksp: HUF_CompressWeightsWksp,
    bitsToWeight: [BYTE; HUF_TABLELOG_MAX as usize + 1], /* precomputed conversion table */
    huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize],
}

/// `HUF_writeCTable_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_writeCTable_wksp(
    dst: *mut c_void,
    max_dst_size: usize,
    CTable: *const HUF_CElt,
    max_symbol_value: c_uint,
    huff_log: c_uint,
    workspace: *mut c_void,
    mut workspace_size: usize,
) -> usize {
    let ct = CTable.add(1);
    let op = dst as *mut u8;
    let mut n: U32;
    let wksp = HUF_alignUpWorkspace(
        workspace,
        &mut workspace_size,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_WriteCTableWksp;

    /* assert(HUF_readCTableHeader(CTable).maxSymbolValue == maxSymbolValue); */
    /* assert(HUF_readCTableHeader(CTable).tableLog == huffLog); */

    /* check conditions */
    if workspace_size < core::mem::size_of::<HUF_WriteCTableWksp>() {
        return err_code(ZSTD_error_GENERIC);
    }
    if max_symbol_value > HUF_SYMBOLVALUE_MAX {
        return err_code(ZSTD_error_maxSymbolValue_tooLarge);
    }

    /* convert to weight */
    (*wksp).bitsToWeight[0] = 0;
    n = 1;
    while n < huff_log + 1 {
        (*wksp).bitsToWeight[n as usize] = (huff_log + 1 - n) as BYTE;
        n += 1;
    }
    n = 0;
    while n < max_symbol_value {
        (*wksp).huffWeight[n as usize] =
            (*wksp).bitsToWeight[HUF_getNbBits(*ct.add(n as usize))];
        n += 1;
    }

    /* attempt weights compression by FSE */
    if max_dst_size < 1 {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    {
        let h_size = HUF_compressWeights(
            op.add(1) as *mut c_void,
            max_dst_size - 1,
            (*wksp).huffWeight.as_ptr() as *const c_void,
            max_symbol_value as usize,
            &mut (*wksp).wksp as *mut HUF_CompressWeightsWksp as *mut c_void,
            core::mem::size_of_val(&(*wksp).wksp),
        );
        if err_is_error(h_size) {
            return h_size;
        }
        if (h_size > 1) & (h_size < max_symbol_value as usize / 2) {
            /* FSE compressed */
            *op.add(0) = h_size as BYTE;
            return h_size + 1;
        }
    }

    /* write raw values as 4-bits (max : 15) */
    if max_symbol_value > (256 - 128) {
        return err_code(ZSTD_error_GENERIC); /* should not happen : likely means source cannot be compressed */
    }
    if ((max_symbol_value + 1) / 2) as usize + 1 > max_dst_size {
        return err_code(ZSTD_error_dstSize_tooSmall); /* not enough space within dst buffer */
    }
    *op.add(0) = (128 /*special case*/ + (max_symbol_value - 1)) as BYTE;
    (*wksp).huffWeight[max_symbol_value as usize] = 0; /* to be sure it doesn't cause msan issue in final combination */
    n = 0;
    while n < max_symbol_value {
        *op.add((n / 2) as usize + 1) = (((*wksp).huffWeight[n as usize] as u32) << 4)
            .wrapping_add((*wksp).huffWeight[n as usize + 1] as u32) as BYTE;
        n += 2;
    }
    ((max_symbol_value + 1) / 2) as usize + 1
}

/// `HUF_readCTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTable(
    CTable: *mut HUF_CElt,
    max_symbol_value_ptr: *mut c_uint,
    src: *const c_void,
    src_size: usize,
    has_zero_weights: *mut c_uint,
) -> usize {
    let mut huff_weight = [0u8; HUF_SYMBOLVALUE_MAX as usize + 1];
    let mut rank_val = [0u32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1];
    let mut table_log: U32 = 0;
    let mut nb_symbols: U32 = 0;
    let ct = CTable.add(1);

    /* get symbol weights */
    let read_size = crate::entropy_common::HUF_readStats(
        huff_weight.as_mut_ptr(),
        HUF_SYMBOLVALUE_MAX as usize + 1,
        rank_val.as_mut_ptr(),
        &mut nb_symbols,
        &mut table_log,
        src,
        src_size,
    );
    if err_is_error(read_size) {
        return read_size;
    }
    *has_zero_weights = (rank_val[0] > 0) as c_uint;

    /* check result */
    if table_log > HUF_TABLELOG_MAX {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }
    if nb_symbols > *max_symbol_value_ptr + 1 {
        return err_code(ZSTD_error_maxSymbolValue_tooSmall);
    }

    *max_symbol_value_ptr = nb_symbols - 1;

    HUF_writeCTableHeader(CTable, table_log, *max_symbol_value_ptr);

    /* Prepare base value per rank */
    {
        let mut next_rank_start: U32 = 0;
        let mut n: U32 = 1;
        while n <= table_log {
            let curr = next_rank_start;
            next_rank_start += rank_val[n as usize] << (n - 1);
            rank_val[n as usize] = curr;
            n += 1;
        }
    }

    /* fill nbBits */
    {
        let mut n: U32 = 0;
        while n < nb_symbols {
            let w = huff_weight[n as usize] as U32;
            HUF_setNbBits(
                ct.add(n as usize),
                (((table_log + 1 - w) as BYTE) & (0u8.wrapping_sub((w != 0) as u8))) as usize,
            );
            n += 1;
        }
    }

    /* fill val */
    {
        let mut nb_per_rank = [0u16; HUF_TABLELOG_MAX as usize + 2]; /* support w=0=>n=tableLog+1 */
        let mut val_per_rank = [0u16; HUF_TABLELOG_MAX as usize + 2];
        {
            let mut n: U32 = 0;
            while n < nb_symbols {
                nb_per_rank[HUF_getNbBits(*ct.add(n as usize))] += 1;
                n += 1;
            }
        }
        /* determine stating value per rank */
        val_per_rank[table_log as usize + 1] = 0; /* for w==0 */
        {
            let mut min: U16 = 0;
            let mut n: U32 = table_log;
            while n > 0 {
                /* start at n=tablelog <-> w=1 */
                val_per_rank[n as usize] = min; /* get starting value within each rank */
                min += nb_per_rank[n as usize];
                min >>= 1;
                n -= 1;
            }
        }
        /* assign value within rank, symbol order */
        {
            let mut n: U32 = 0;
            while n < nb_symbols {
                let nb = HUF_getNbBits(*ct.add(n as usize));
                HUF_setValue(ct.add(n as usize), val_per_rank[nb] as usize);
                val_per_rank[nb] += 1;
                n += 1;
            }
        }
    }

    read_size
}

/// `HUF_getNbBitsFromCTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getNbBitsFromCTable(CTable: *const HUF_CElt, symbol_value: U32) -> U32 {
    let ct = CTable.add(1);
    /* assert(symbolValue <= HUF_SYMBOLVALUE_MAX); */
    if symbol_value > HUF_readCTableHeader(CTable).maxSymbolValue as U32 {
        return 0;
    }
    HUF_getNbBits(*ct.add(symbol_value as usize)) as U32
}

/// `HUF_setMaxHeight()`
unsafe fn HUF_setMaxHeight(huff_node: *mut nodeElt, last_non_null: U32, target_nb_bits: U32) -> U32 {
    let largest_bits = (*huff_node.add(last_non_null as usize)).nbBits as U32;
    /* early exit : no elt > targetNbBits, so the tree is already valid. */
    if largest_bits <= target_nb_bits {
        return largest_bits;
    }

    /* there are several too large elements (at least >= 2) */
    {
        let mut total_cost: c_int = 0;
        let base_cost: U32 = 1 << (largest_bits - target_nb_bits);
        let mut n: c_int = last_non_null as c_int;

        /* Adjust any ranks > targetNbBits to targetNbBits. */
        while (*huff_node.add(n as usize)).nbBits as U32 > target_nb_bits {
            total_cost += (base_cost
                - (1u32 << (largest_bits - (*huff_node.add(n as usize)).nbBits as U32)))
                as c_int;
            (*huff_node.add(n as usize)).nbBits = target_nb_bits as BYTE;
            n -= 1;
        }
        /* n stops at huffNode[n].nbBits <= targetNbBits */
        /* n end at index of smallest symbol using < targetNbBits */
        while (*huff_node.add(n as usize)).nbBits as U32 == target_nb_bits {
            n -= 1;
        }

        /* renorm totalCost from 2^largestBits to 2^targetNbBits */
        total_cost >>= largest_bits - target_nb_bits;
        /* assert(totalCost > 0); */

        /* repay normalized cost */
        {
            let no_symbol: U32 = 0xF0F0F0F0;
            let mut rank_last = [0u32; HUF_TABLELOG_MAX as usize + 2];

            /* Get pos of last (smallest = lowest cum. count) symbol per rank */
            core::ptr::write_bytes(rank_last.as_mut_ptr() as *mut u8, 0xF0, core::mem::size_of_val(&rank_last));
            {
                let mut current_nb_bits = target_nb_bits;
                let mut pos: c_int = n;
                while pos >= 0 {
                    if (*huff_node.add(pos as usize)).nbBits as U32 >= current_nb_bits {
                        pos -= 1;
                        continue;
                    }
                    current_nb_bits = (*huff_node.add(pos as usize)).nbBits as U32; /* < targetNbBits */
                    rank_last[(target_nb_bits - current_nb_bits) as usize] = pos as U32;
                    pos -= 1;
                }
            }

            while total_cost > 0 {
                /* Try to reduce the next power of 2 above totalCost */
                let mut n_bits_to_decrease = zstd_highbit32(total_cost as U32) + 1;
                while n_bits_to_decrease > 1 {
                    let high_pos = rank_last[n_bits_to_decrease as usize];
                    let low_pos = rank_last[n_bits_to_decrease as usize - 1];
                    if high_pos == no_symbol {
                        n_bits_to_decrease -= 1;
                        continue;
                    }
                    if low_pos == no_symbol {
                        break;
                    }
                    {
                        let high_total = (*huff_node.add(high_pos as usize)).count;
                        let low_total = 2u32.wrapping_mul((*huff_node.add(low_pos as usize)).count);
                        if high_total <= low_total {
                            break;
                        }
                    }
                    n_bits_to_decrease -= 1;
                }
                /* assert(rankLast[nBitsToDecrease] != noSymbol || nBitsToDecrease == 1); */
                /* HUF_MAX_TABLELOG test just to please gcc 5+; but it should not be necessary */
                while (n_bits_to_decrease <= HUF_TABLELOG_MAX)
                    && (rank_last[n_bits_to_decrease as usize] == no_symbol)
                {
                    n_bits_to_decrease += 1;
                }
                /* assert(rankLast[nBitsToDecrease] != noSymbol); */
                /* Increase the number of bits to gain back half the rank cost. */
                total_cost -= 1 << (n_bits_to_decrease - 1);
                (*huff_node.add(rank_last[n_bits_to_decrease as usize] as usize)).nbBits += 1;

                /* Fix up the new rank. */
                if rank_last[n_bits_to_decrease as usize - 1] == no_symbol {
                    rank_last[n_bits_to_decrease as usize - 1] =
                        rank_last[n_bits_to_decrease as usize];
                }
                /* Fix up the old rank. */
                if rank_last[n_bits_to_decrease as usize] == 0 {
                    /* special case, reached largest symbol */
                    rank_last[n_bits_to_decrease as usize] = no_symbol;
                } else {
                    rank_last[n_bits_to_decrease as usize] -= 1;
                    if (*huff_node.add(rank_last[n_bits_to_decrease as usize] as usize)).nbBits
                        as U32
                        != target_nb_bits - n_bits_to_decrease
                    {
                        rank_last[n_bits_to_decrease as usize] = no_symbol; /* this rank is now empty */
                    }
                }
            } /* while (totalCost > 0) */

            /* If we've removed too much weight, then we have to add it back. */
            while total_cost < 0 {
                /* Sometimes, cost correction overshoot */
                /* special case : no rank 1 symbol (using targetNbBits-1); */
                if rank_last[1] == no_symbol {
                    while (*huff_node.add(n as usize)).nbBits as U32 == target_nb_bits {
                        n -= 1;
                    }
                    (*huff_node.add((n + 1) as usize)).nbBits -= 1;
                    /* assert(n >= 0); */
                    rank_last[1] = (n + 1) as U32;
                    total_cost += 1;
                    continue;
                }
                (*huff_node.add((rank_last[1] + 1) as usize)).nbBits -= 1;
                rank_last[1] += 1;
                total_cost += 1;
            }
        } /* repay normalized cost */
    } /* there are several too large elements (at least >= 2) */

    target_nb_bits
}

/// `rankPos`
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct rankPos {
    base: U16,
    curr: U16,
}

/* typedef nodeElt huffNodeTable[2 * (HUF_SYMBOLVALUE_MAX + 1)]; */
const HUFNODETABLE_LEN: usize = 2 * (HUF_SYMBOLVALUE_MAX as usize + 1);

/// Number of buckets available for `HUF_sort()`
const RANK_POSITION_TABLE_SIZE: usize = 192;

/// `HUF_buildCTable_wksp_tables`
#[repr(C)]
struct HUF_buildCTable_wksp_tables {
    huffNodeTbl: [nodeElt; HUFNODETABLE_LEN],
    rankPosition: [rankPos; RANK_POSITION_TABLE_SIZE],
}

const RANK_POSITION_MAX_COUNT_LOG: u32 = 32;
/// `RANK_POSITION_LOG_BUCKETS_BEGIN` == 158
const RANK_POSITION_LOG_BUCKETS_BEGIN: u32 =
    (RANK_POSITION_TABLE_SIZE as u32 - 1) - RANK_POSITION_MAX_COUNT_LOG - 1;
/// `RANK_POSITION_DISTINCT_COUNT_CUTOFF` == 166
const RANK_POSITION_DISTINCT_COUNT_CUTOFF: u32 =
    RANK_POSITION_LOG_BUCKETS_BEGIN + highbit32_const(RANK_POSITION_LOG_BUCKETS_BEGIN);

/// `ZSTD_highbit32()` evaluated at compile time for constants.
const fn highbit32_const(val: u32) -> u32 {
    31 - val.leading_zeros()
}

/// `HUF_getIndex()`
fn HUF_getIndex(count: U32) -> U32 {
    if count < RANK_POSITION_DISTINCT_COUNT_CUTOFF {
        count
    } else {
        zstd_highbit32(count) + RANK_POSITION_LOG_BUCKETS_BEGIN
    }
}

/// `HUF_swapNodes()`
unsafe fn HUF_swapNodes(a: *mut nodeElt, b: *mut nodeElt) {
    let tmp = *a;
    *a = *b;
    *b = tmp;
}

/// `HUF_isSorted()` — returns 0 if `huffNode` is not sorted by descending count.
unsafe fn HUF_isSorted(huff_node: *const nodeElt, max_symbol_value1: U32) -> c_int {
    let mut i: U32 = 1;
    while i < max_symbol_value1 {
        if (*huff_node.add(i as usize)).count > (*huff_node.add(i as usize - 1)).count {
            return 0;
        }
        i += 1;
    }
    1
}

/// `HUF_insertionSort()` — insertion sort by descending order.
unsafe fn HUF_insertionSort(huff_node: *mut nodeElt, low: c_int, high: c_int) {
    let size = high - low + 1;
    let huff_node = huff_node.offset(low as isize);
    let mut i: c_int = 1;
    while i < size {
        let key = *huff_node.offset(i as isize);
        let mut j: c_int = i - 1;
        while j >= 0 && (*huff_node.offset(j as isize)).count < key.count {
            *huff_node.offset(j as isize + 1) = *huff_node.offset(j as isize);
            j -= 1;
        }
        *huff_node.offset(j as isize + 1) = key;
        i += 1;
    }
}

/// `HUF_quickSortPartition()`
unsafe fn HUF_quickSortPartition(arr: *mut nodeElt, low: c_int, high: c_int) -> c_int {
    let pivot = (*arr.offset(high as isize)).count;
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

/// `HUF_simpleQuickSort()`
unsafe fn HUF_simpleQuickSort(arr: *mut nodeElt, mut low: c_int, mut high: c_int) {
    let k_insertion_sort_threshold: c_int = 8;
    if high - low < k_insertion_sort_threshold {
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

/// `HUF_sort()`
unsafe fn HUF_sort(
    huff_node: *mut nodeElt,
    count: *const c_uint,
    max_symbol_value: U32,
    rank_position: *mut rankPos,
) {
    let mut n: U32;
    let max_symbol_value1 = max_symbol_value + 1;

    /* Compute base and set curr to base. */
    core::ptr::write_bytes(
        rank_position,
        0,
        RANK_POSITION_TABLE_SIZE,
    );
    n = 0;
    while n < max_symbol_value1 {
        let lower_rank = HUF_getIndex(*count.add(n as usize));
        (*rank_position.add(lower_rank as usize)).base += 1;
        n += 1;
    }

    /* Set up the rankPosition table */
    n = RANK_POSITION_TABLE_SIZE as U32 - 1;
    while n > 0 {
        let add = (*rank_position.add(n as usize)).base;
        (*rank_position.add(n as usize - 1)).base += add;
        (*rank_position.add(n as usize - 1)).curr = (*rank_position.add(n as usize - 1)).base;
        n -= 1;
    }

    /* Insert each symbol into their appropriate bucket, setting up rankPosition table. */
    n = 0;
    while n < max_symbol_value1 {
        let c = *count.add(n as usize);
        let r = HUF_getIndex(c) + 1;
        let pos = (*rank_position.add(r as usize)).curr;
        (*rank_position.add(r as usize)).curr += 1;
        (*huff_node.add(pos as usize)).count = c;
        (*huff_node.add(pos as usize)).byte = n as BYTE;
        n += 1;
    }

    /* Sort each bucket. */
    n = RANK_POSITION_DISTINCT_COUNT_CUTOFF;
    while n < RANK_POSITION_TABLE_SIZE as U32 - 1 {
        let bucket_size =
            (*rank_position.add(n as usize)).curr as c_int - (*rank_position.add(n as usize)).base as c_int;
        let bucket_start_idx = (*rank_position.add(n as usize)).base;
        if bucket_size > 1 {
            HUF_simpleQuickSort(huff_node.add(bucket_start_idx as usize), 0, bucket_size - 1);
        }
        n += 1;
    }

    /* assert(HUF_isSorted(huffNode, maxSymbolValue1)); */
    let _ = HUF_isSorted;
}

const STARTNODE: c_int = HUF_SYMBOLVALUE_MAX as c_int + 1;

/// `HUF_buildTree()`
unsafe fn HUF_buildTree(huff_node: *mut nodeElt, max_symbol_value: U32) -> c_int {
    let huff_node0 = huff_node.offset(-1);
    let mut non_null_rank: c_int;
    let mut low_s: c_int;
    let mut low_n: c_int;
    let mut node_nb: c_int = STARTNODE;
    let mut n: c_int;
    let node_root: c_int;

    /* init for parents */
    non_null_rank = max_symbol_value as c_int;
    while (*huff_node.offset(non_null_rank as isize)).count == 0 {
        non_null_rank -= 1;
    }
    low_s = non_null_rank;
    node_root = node_nb + low_s - 1;
    low_n = node_nb;
    (*huff_node.offset(node_nb as isize)).count = (*huff_node.offset(low_s as isize)).count
        + (*huff_node.offset(low_s as isize - 1)).count;
    (*huff_node.offset(low_s as isize)).parent = node_nb as U16;
    (*huff_node.offset(low_s as isize - 1)).parent = node_nb as U16;
    node_nb += 1;
    low_s -= 2;
    n = node_nb;
    while n <= node_root {
        (*huff_node.offset(n as isize)).count = 1u32 << 30;
        n += 1;
    }
    (*huff_node0.offset(0)).count = 1u32 << 31; /* fake entry, strong barrier */

    /* create parents */
    while node_nb <= node_root {
        let n1 = if (*huff_node.offset(low_s as isize)).count
            < (*huff_node.offset(low_n as isize)).count
        {
            let t = low_s;
            low_s -= 1;
            t
        } else {
            let t = low_n;
            low_n += 1;
            t
        };
        let n2 = if (*huff_node.offset(low_s as isize)).count
            < (*huff_node.offset(low_n as isize)).count
        {
            let t = low_s;
            low_s -= 1;
            t
        } else {
            let t = low_n;
            low_n += 1;
            t
        };
        (*huff_node.offset(node_nb as isize)).count = (*huff_node.offset(n1 as isize)).count
            + (*huff_node.offset(n2 as isize)).count;
        (*huff_node.offset(n1 as isize)).parent = node_nb as U16;
        (*huff_node.offset(n2 as isize)).parent = node_nb as U16;
        node_nb += 1;
    }

    /* distribute weights (unlimited tree height) */
    (*huff_node.offset(node_root as isize)).nbBits = 0;
    n = node_root - 1;
    while n >= STARTNODE {
        (*huff_node.offset(n as isize)).nbBits =
            (*huff_node.offset((*huff_node.offset(n as isize)).parent as isize)).nbBits + 1;
        n -= 1;
    }
    n = 0;
    while n <= non_null_rank {
        (*huff_node.offset(n as isize)).nbBits =
            (*huff_node.offset((*huff_node.offset(n as isize)).parent as isize)).nbBits + 1;
        n += 1;
    }

    non_null_rank
}

/// `HUF_buildCTableFromTree()`
unsafe fn HUF_buildCTableFromTree(
    CTable: *mut HUF_CElt,
    huff_node: *const nodeElt,
    non_null_rank: c_int,
    max_symbol_value: U32,
    max_nb_bits: U32,
) {
    let ct = CTable.add(1);
    /* fill result into ctable (val, nbBits) */
    let mut n: c_int;
    let mut nb_per_rank = [0u16; HUF_TABLELOG_MAX as usize + 1];
    let mut val_per_rank = [0u16; HUF_TABLELOG_MAX as usize + 1];
    let alphabet_size = (max_symbol_value + 1) as c_int;
    n = 0;
    while n <= non_null_rank {
        nb_per_rank[(*huff_node.offset(n as isize)).nbBits as usize] += 1;
        n += 1;
    }
    /* determine starting value per rank */
    {
        let mut min: U16 = 0;
        n = max_nb_bits as c_int;
        while n > 0 {
            val_per_rank[n as usize] = min; /* get starting value within each rank */
            min += nb_per_rank[n as usize];
            min >>= 1;
            n -= 1;
        }
    }
    n = 0;
    while n < alphabet_size {
        HUF_setNbBits(
            ct.add((*huff_node.offset(n as isize)).byte as usize),
            (*huff_node.offset(n as isize)).nbBits as usize,
        ); /* push nbBits per symbol, symbol order */
        n += 1;
    }
    n = 0;
    while n < alphabet_size {
        let nb = HUF_getNbBits(*ct.add(n as usize));
        HUF_setValue(ct.add(n as usize), val_per_rank[nb] as usize); /* assign value within rank, symbol order */
        val_per_rank[nb] += 1;
        n += 1;
    }

    HUF_writeCTableHeader(CTable, max_nb_bits, max_symbol_value);
}

/// `HUF_buildCTable_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_buildCTable_wksp(
    CTable: *mut HUF_CElt,
    count: *const c_uint,
    max_symbol_value: U32,
    mut max_nb_bits: U32,
    work_space: *mut c_void,
    mut wksp_size: usize,
) -> usize {
    let wksp_tables = HUF_alignUpWorkspace(
        work_space,
        &mut wksp_size,
        core::mem::align_of::<U32>(),
    ) as *mut HUF_buildCTable_wksp_tables;
    let huff_node0 = (*wksp_tables).huffNodeTbl.as_mut_ptr();
    let huff_node = huff_node0.add(1);
    let non_null_rank: c_int;

    /* safety checks */
    if wksp_size < core::mem::size_of::<HUF_buildCTable_wksp_tables>() {
        return err_code(ZSTD_error_workSpace_tooSmall);
    }
    if max_nb_bits == 0 {
        max_nb_bits = HUF_TABLELOG_DEFAULT;
    }
    if max_symbol_value > HUF_SYMBOLVALUE_MAX {
        return err_code(ZSTD_error_maxSymbolValue_tooLarge);
    }
    core::ptr::write_bytes(
        huff_node0 as *mut u8,
        0,
        core::mem::size_of::<[nodeElt; HUFNODETABLE_LEN]>(),
    );

    /* sort, decreasing order */
    HUF_sort(huff_node, count, max_symbol_value, (*wksp_tables).rankPosition.as_mut_ptr());

    /* build tree */
    non_null_rank = HUF_buildTree(huff_node, max_symbol_value);

    /* determine and enforce maxTableLog */
    max_nb_bits = HUF_setMaxHeight(huff_node, non_null_rank as U32, max_nb_bits);
    if max_nb_bits > HUF_TABLELOG_MAX {
        return err_code(ZSTD_error_GENERIC); /* check fit into table */
    }

    HUF_buildCTableFromTree(CTable, huff_node, non_null_rank, max_symbol_value, max_nb_bits);

    max_nb_bits as usize
}

/// `HUF_estimateCompressedSize()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_estimateCompressedSize(
    CTable: *const HUF_CElt,
    count: *const c_uint,
    max_symbol_value: c_uint,
) -> usize {
    let ct = CTable.add(1);
    let mut nb_bits: usize = 0;
    let mut s: c_int = 0;
    while s <= max_symbol_value as c_int {
        nb_bits += HUF_getNbBits(*ct.add(s as usize)) * *count.add(s as usize) as usize;
        s += 1;
    }
    nb_bits >> 3
}

/// `HUF_validateCTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_validateCTable(
    CTable: *const HUF_CElt,
    count: *const c_uint,
    max_symbol_value: c_uint,
) -> c_int {
    let header = HUF_readCTableHeader(CTable);
    let ct = CTable.add(1);
    let mut bad: c_int = 0;
    let mut s: c_int;

    /* assert(header.tableLog <= HUF_TABLELOG_ABSOLUTEMAX); */

    if (header.maxSymbolValue as c_uint) < max_symbol_value {
        return 0;
    }

    s = 0;
    while s <= max_symbol_value as c_int {
        bad |= ((*count.add(s as usize) != 0) as c_int)
            & ((HUF_getNbBits(*ct.add(s as usize)) == 0) as c_int);
        s += 1;
    }
    (bad == 0) as c_int
}

/// `HUF_compressBound()`
#[unsafe(no_mangle)]
pub extern "C" fn HUF_compressBound(size: usize) -> usize {
    huf_compressbound(size)
}

/* ==========================================================================
 * HUF_CStream_t
 * ========================================================================== */

const HUF_BITS_IN_CONTAINER: usize = core::mem::size_of::<usize>() * 8;

/// `HUF_CStream_t`
#[repr(C)]
struct HUF_CStream_t {
    bitContainer: [usize; 2],
    bitPos: [usize; 2],
    startPtr: *mut BYTE,
    ptr: *mut BYTE,
    endPtr: *mut BYTE,
}

/// `HUF_initCStream()`
unsafe fn HUF_initCStream(
    bit_c: *mut HUF_CStream_t,
    start_ptr: *mut c_void,
    dst_capacity: usize,
) -> usize {
    core::ptr::write_bytes(bit_c as *mut u8, 0, core::mem::size_of::<HUF_CStream_t>());
    (*bit_c).startPtr = start_ptr as *mut BYTE;
    (*bit_c).ptr = (*bit_c).startPtr;
    (*bit_c).endPtr = (*bit_c)
        .startPtr
        .add(dst_capacity.wrapping_sub(core::mem::size_of::<usize>()));
    if dst_capacity <= core::mem::size_of::<usize>() {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    0
}

/// `HUF_addBits()`
#[inline(always)]
unsafe fn HUF_addBits(bit_c: *mut HUF_CStream_t, elt: HUF_CElt, idx: c_int, k_fast: c_int) {
    /* assert(idx <= 1); */
    /* assert(HUF_getNbBits(elt) <= HUF_TABLELOG_ABSOLUTEMAX); */
    (*bit_c).bitContainer[idx as usize] >>= HUF_getNbBits(elt);
    (*bit_c).bitContainer[idx as usize] |= if k_fast != 0 {
        HUF_getValueFast(elt)
    } else {
        HUF_getValue(elt)
    };
    (*bit_c).bitPos[idx as usize] += HUF_getNbBitsFast(elt);
    /* assert((bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER); */
}

/// `HUF_zeroIndex1()`
#[inline(always)]
unsafe fn HUF_zeroIndex1(bit_c: *mut HUF_CStream_t) {
    (*bit_c).bitContainer[1] = 0;
    (*bit_c).bitPos[1] = 0;
}

/// `HUF_mergeIndex1()`
#[inline(always)]
unsafe fn HUF_mergeIndex1(bit_c: *mut HUF_CStream_t) {
    /* assert((bitC->bitPos[1] & 0xFF) < HUF_BITS_IN_CONTAINER); */
    (*bit_c).bitContainer[0] >>= (*bit_c).bitPos[1] & 0xFF;
    (*bit_c).bitContainer[0] |= (*bit_c).bitContainer[1];
    (*bit_c).bitPos[0] += (*bit_c).bitPos[1];
    /* assert((bitC->bitPos[0] & 0xFF) <= HUF_BITS_IN_CONTAINER); */
}

/// `HUF_flushBits()`
#[inline(always)]
unsafe fn HUF_flushBits(bit_c: *mut HUF_CStream_t, k_fast: c_int) {
    /* The upper bits of bitPos are noisy, so we must mask by 0xFF. */
    let nb_bits = (*bit_c).bitPos[0] & 0xFF;
    let nb_bytes = nb_bits >> 3;
    /* The top nbBits bits of bitContainer are the ones we need. */
    let bit_container = (*bit_c).bitContainer[0] >> (HUF_BITS_IN_CONTAINER - nb_bits);
    /* Mask bitPos to account for the bytes we consumed. */
    (*bit_c).bitPos[0] &= 7;
    /* assert(nbBits > 0); */
    mem_write_lest((*bit_c).ptr, bit_container);
    (*bit_c).ptr = (*bit_c).ptr.add(nb_bytes);
    if k_fast == 0 && (*bit_c).ptr > (*bit_c).endPtr {
        (*bit_c).ptr = (*bit_c).endPtr;
    }
}

/// `HUF_endMark()`
fn HUF_endMark() -> HUF_CElt {
    let mut end_mark: HUF_CElt = 0;
    HUF_setNbBits(&mut end_mark, 1);
    HUF_setValue(&mut end_mark, 1);
    end_mark
}

/// `HUF_closeCStream()`
unsafe fn HUF_closeCStream(bit_c: *mut HUF_CStream_t) -> usize {
    HUF_addBits(bit_c, HUF_endMark(), 0 /* idx */, 0 /* kFast */);
    HUF_flushBits(bit_c, 0 /* kFast */);
    {
        let nb_bits = (*bit_c).bitPos[0] & 0xFF;
        if (*bit_c).ptr >= (*bit_c).endPtr {
            return 0; /* overflow detected */
        }
        ((*bit_c).ptr as usize - (*bit_c).startPtr as usize) + (nb_bits > 0) as usize
    }
}

/// `HUF_encodeSymbol()`
#[inline(always)]
unsafe fn HUF_encodeSymbol(
    bit_c_ptr: *mut HUF_CStream_t,
    symbol: U32,
    CTable: *const HUF_CElt,
    idx: c_int,
    fast: c_int,
) {
    HUF_addBits(bit_c_ptr, *CTable.add(symbol as usize), idx, fast);
}

/// `HUF_compress1X_usingCTable_internal_body_loop()`
#[inline(always)]
unsafe fn HUF_compress1X_usingCTable_internal_body_loop(
    bit_c: *mut HUF_CStream_t,
    ip: *const BYTE,
    src_size: usize,
    ct: *const HUF_CElt,
    k_unroll: c_int,
    k_fast_flush: c_int,
    k_last_fast: c_int,
) {
    /* Join to kUnroll */
    let mut n: c_int = src_size as c_int;
    let mut rem = n % k_unroll;
    if rem > 0 {
        while rem > 0 {
            n -= 1;
            HUF_encodeSymbol(bit_c, *ip.offset(n as isize) as U32, ct, 0, 0 /* fast */);
            rem -= 1;
        }
        HUF_flushBits(bit_c, k_fast_flush);
    }
    /* assert(n % kUnroll == 0); */

    /* Join to 2 * kUnroll */
    if n % (2 * k_unroll) != 0 {
        let mut u: c_int = 1;
        while u < k_unroll {
            HUF_encodeSymbol(bit_c, *ip.offset((n - u) as isize) as U32, ct, 0, 1);
            u += 1;
        }
        HUF_encodeSymbol(bit_c, *ip.offset((n - k_unroll) as isize) as U32, ct, 0, k_last_fast);
        HUF_flushBits(bit_c, k_fast_flush);
        n -= k_unroll;
    }
    /* assert(n % (2 * kUnroll) == 0); */

    while n > 0 {
        /* Encode kUnroll symbols into the bitstream @ index 0. */
        let mut u: c_int = 1;
        while u < k_unroll {
            HUF_encodeSymbol(bit_c, *ip.offset((n - u) as isize) as U32, ct, 0 /* idx */, 1 /* fast */);
            u += 1;
        }
        HUF_encodeSymbol(
            bit_c,
            *ip.offset((n - k_unroll) as isize) as U32,
            ct,
            0, /* idx */
            k_last_fast, /* fast */
        );
        HUF_flushBits(bit_c, k_fast_flush);
        /* Encode kUnroll symbols into the bitstream @ index 1. */
        HUF_zeroIndex1(bit_c);
        let mut u: c_int = 1;
        while u < k_unroll {
            HUF_encodeSymbol(
                bit_c,
                *ip.offset((n - k_unroll - u) as isize) as U32,
                ct,
                1, /* idx */
                1, /* fast */
            );
            u += 1;
        }
        HUF_encodeSymbol(
            bit_c,
            *ip.offset((n - k_unroll - k_unroll) as isize) as U32,
            ct,
            1, /* idx */
            k_last_fast, /* fast */
        );
        /* Merge bitstream @ index 1 into the bitstream @ index 0 */
        HUF_mergeIndex1(bit_c);
        HUF_flushBits(bit_c, k_fast_flush);

        n -= 2 * k_unroll;
    }
    /* assert(n == 0); */
}

/// `HUF_tightCompressBound()`
fn HUF_tightCompressBound(src_size: usize, table_log: usize) -> usize {
    ((src_size * table_log) >> 3) + 8
}

/// `HUF_compress1X_usingCTable_internal_body()`
#[inline(always)]
unsafe fn HUF_compress1X_usingCTable_internal_body(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    CTable: *const HUF_CElt,
) -> usize {
    let table_log = HUF_readCTableHeader(CTable).tableLog as U32;
    let ct = CTable.add(1);
    let ip = src as *const BYTE;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dst_size);
    let mut bit_c: HUF_CStream_t = core::mem::zeroed();

    /* init */
    if dst_size < 8 {
        return 0; /* not enough space to compress */
    }
    {
        let op = ostart;
        let init_err = HUF_initCStream(&mut bit_c, op as *mut c_void, oend as usize - op as usize);
        if err_is_error(init_err) {
            return 0;
        }
    }

    if dst_size < HUF_tightCompressBound(src_size, table_log as usize) || table_log > 11 {
        HUF_compress1X_usingCTable_internal_body_loop(
            &mut bit_c,
            ip,
            src_size,
            ct,
            if mem_32bits() { 2 } else { 4 }, /* kUnroll */
            0,                                /* kFast */
            0,                                /* kLastFast */
        );
    } else {
        if mem_32bits() {
            match table_log {
                11 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 2, 1, 0);
                }
                10 | 9 | 8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 2, 1, 1);
                }
                _ /* 7, default */ => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 3, 1, 1);
                }
            }
        } else {
            match table_log {
                11 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 5, 1, 0);
                }
                10 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 5, 1, 1);
                }
                9 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 6, 1, 0);
                }
                8 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 7, 1, 0);
                }
                7 => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 8, 1, 0);
                }
                _ /* 6, default */ => {
                    HUF_compress1X_usingCTable_internal_body_loop(&mut bit_c, ip, src_size, ct, 9, 1, 1);
                }
            }
        }
    }
    /* assert(bitC.ptr <= bitC.endPtr); */

    HUF_closeCStream(&mut bit_c)
}

/* DYNAMIC_BMI2 == 0: the `#else` branch is compiled. `flags` is ignored and the
 * body is called directly. */

/// `HUF_compress1X_usingCTable_internal()`
unsafe fn HUF_compress1X_usingCTable_internal(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    CTable: *const HUF_CElt,
    _flags: c_int,
) -> usize {
    HUF_compress1X_usingCTable_internal_body(dst, dst_size, src, src_size, CTable)
}

/// `HUF_compress1X_usingCTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_usingCTable(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    HUF_compress1X_usingCTable_internal(dst, dst_size, src, src_size, CTable, flags)
}

/// `HUF_compress4X_usingCTable_internal()`
unsafe fn HUF_compress4X_usingCTable_internal(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    let segment_size = (src_size + 3) / 4; /* first 3 segments */
    let mut ip = src as *const BYTE;
    let iend = ip.add(src_size);
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dst_size);
    let mut op = ostart;

    if dst_size < 6 + 1 + 1 + 1 + 8 {
        return 0; /* minimum space to compress successfully */
    }
    if src_size < 12 {
        return 0; /* no saving possible : too small input */
    }
    op = op.add(6); /* jumpTable */

    {
        let c_size = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            segment_size,
            CTable,
            flags,
        );
        if err_is_error(c_size) {
            return c_size;
        }
        if c_size == 0 || c_size > 65535 {
            return 0;
        }
        mem_write_le16(ostart, c_size as U16);
        op = op.add(c_size);
    }

    ip = ip.add(segment_size);
    {
        let c_size = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            segment_size,
            CTable,
            flags,
        );
        if err_is_error(c_size) {
            return c_size;
        }
        if c_size == 0 || c_size > 65535 {
            return 0;
        }
        mem_write_le16(ostart.add(2), c_size as U16);
        op = op.add(c_size);
    }

    ip = ip.add(segment_size);
    {
        let c_size = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            segment_size,
            CTable,
            flags,
        );
        if err_is_error(c_size) {
            return c_size;
        }
        if c_size == 0 || c_size > 65535 {
            return 0;
        }
        mem_write_le16(ostart.add(4), c_size as U16);
        op = op.add(c_size);
    }

    ip = ip.add(segment_size);
    {
        let c_size = HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            ip as *const c_void,
            iend as usize - ip as usize,
            CTable,
            flags,
        );
        if err_is_error(c_size) {
            return c_size;
        }
        if c_size == 0 || c_size > 65535 {
            return 0;
        }
        op = op.add(c_size);
    }

    op as usize - ostart as usize
}

/// `HUF_compress4X_usingCTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_usingCTable(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    HUF_compress4X_usingCTable_internal(dst, dst_size, src, src_size, CTable, flags)
}

/// `HUF_nbStreams_e`
#[derive(Clone, Copy, PartialEq, Eq)]
enum HUF_nbStreams_e {
    HUF_singleStream,
    HUF_fourStreams,
}

/// `HUF_compressCTable_internal()`
unsafe fn HUF_compressCTable_internal(
    ostart: *mut BYTE,
    mut op: *mut BYTE,
    oend: *mut BYTE,
    src: *const c_void,
    src_size: usize,
    nb_streams: HUF_nbStreams_e,
    CTable: *const HUF_CElt,
    flags: c_int,
) -> usize {
    let c_size = if nb_streams == HUF_nbStreams_e::HUF_singleStream {
        HUF_compress1X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            src,
            src_size,
            CTable,
            flags,
        )
    } else {
        HUF_compress4X_usingCTable_internal(
            op as *mut c_void,
            oend as usize - op as usize,
            src,
            src_size,
            CTable,
            flags,
        )
    };
    if err_is_error(c_size) {
        return c_size;
    }
    if c_size == 0 {
        return 0; /* uncompressible */
    }
    op = op.add(c_size);
    /* check compressibility */
    /* assert(op >= ostart); */
    if (op as usize - ostart as usize) >= src_size - 1 {
        return 0;
    }
    op as usize - ostart as usize
}

/// `HUF_compress_tables_t`
#[repr(C)]
struct HUF_compress_tables_t {
    count: [c_uint; HUF_SYMBOLVALUE_MAX as usize + 1],
    CTable: [HUF_CElt; huf_ctable_size_st(HUF_SYMBOLVALUE_MAX as usize)],
    wksps: HUF_compress_tables_t_wksps,
}

/// The `union` in `HUF_compress_tables_t`.
#[repr(C)]
union HUF_compress_tables_t_wksps {
    buildCTable_wksp: core::mem::ManuallyDrop<HUF_buildCTable_wksp_tables>,
    writeCTable_wksp: core::mem::ManuallyDrop<HUF_WriteCTableWksp>,
    hist_wksp: [U32; HIST_WKSP_SIZE_U32],
}

const SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE: usize = 4096;
const SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO: usize = 10; /* Must be >= 2 */

/// `HUF_cardinality()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_cardinality(count: *const c_uint, max_symbol_value: c_uint) -> c_uint {
    let mut cardinality: c_uint = 0;
    let mut i: c_uint = 0;
    while i < max_symbol_value + 1 {
        if *count.add(i as usize) != 0 {
            cardinality += 1;
        }
        i += 1;
    }
    cardinality
}

/// `HUF_minTableLog()`
#[unsafe(no_mangle)]
pub extern "C" fn HUF_minTableLog(symbol_cardinality: c_uint) -> c_uint {
    let min_bits_symbols = zstd_highbit32(symbol_cardinality) + 1;
    min_bits_symbols
}

/// `HUF_optimalTableLog()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_optimalTableLog(
    max_table_log: c_uint,
    src_size: usize,
    max_symbol_value: c_uint,
    work_space: *mut c_void,
    wksp_size: usize,
    table: *mut HUF_CElt,
    count: *const c_uint,
    flags: c_int,
) -> c_uint {
    /* assert(srcSize > 1); */
    /* assert(wkspSize >= sizeof(HUF_buildCTable_wksp_tables)); */

    if (flags & HUF_flags_optimalDepth) == 0 {
        /* cheap evaluation, based on FSE */
        return crate::fse_compress::FSE_optimalTableLog_internal(
            max_table_log,
            src_size,
            max_symbol_value,
            1,
        );
    }

    {
        let dst = (work_space as *mut BYTE).add(core::mem::size_of::<HUF_WriteCTableWksp>());
        let dst_size = wksp_size - core::mem::size_of::<HUF_WriteCTableWksp>();
        let mut h_size: usize;
        let mut new_size: usize;
        let symbol_cardinality = HUF_cardinality(count, max_symbol_value);
        let min_table_log = HUF_minTableLog(symbol_cardinality);
        let mut opt_size: usize = (!0usize) - 1;
        let mut opt_log: c_uint = max_table_log;
        let mut opt_log_guess: c_uint;

        /* Search until size increases */
        opt_log_guess = min_table_log;
        while opt_log_guess <= max_table_log {
            {
                let max_bits = HUF_buildCTable_wksp(
                    table,
                    count,
                    max_symbol_value,
                    opt_log_guess,
                    work_space,
                    wksp_size,
                );
                if err_is_error(max_bits) {
                    opt_log_guess += 1;
                    continue;
                }

                if (max_bits as c_uint) < opt_log_guess && opt_log_guess > min_table_log {
                    break;
                }

                h_size = HUF_writeCTable_wksp(
                    dst as *mut c_void,
                    dst_size,
                    table,
                    max_symbol_value,
                    max_bits as U32,
                    work_space,
                    wksp_size,
                );
            }

            if err_is_error(h_size) {
                opt_log_guess += 1;
                continue;
            }

            new_size = HUF_estimateCompressedSize(table, count, max_symbol_value) + h_size;

            if new_size > opt_size + 1 {
                break;
            }

            if new_size < opt_size {
                opt_size = new_size;
                opt_log = opt_log_guess;
            }

            opt_log_guess += 1;
        }
        /* assert(optLog <= HUF_TABLELOG_MAX); */
        opt_log
    }
}

/// `HUF_compress_internal()`
unsafe fn HUF_compress_internal(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    mut max_symbol_value: c_uint,
    mut huff_log: c_uint,
    nb_streams: HUF_nbStreams_e,
    work_space: *mut c_void,
    mut wksp_size: usize,
    old_huf_table: *mut HUF_CElt,
    repeat: *mut u32,
    flags: c_int,
) -> usize {
    let table = HUF_alignUpWorkspace(
        work_space,
        &mut wksp_size,
        core::mem::align_of::<usize>(),
    ) as *mut HUF_compress_tables_t;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dst_size);
    let mut op = ostart;

    /* checks & inits */
    if wksp_size < core::mem::size_of::<HUF_compress_tables_t>() {
        return err_code(ZSTD_error_workSpace_tooSmall);
    }
    if src_size == 0 {
        return 0; /* Uncompressed */
    }
    if dst_size == 0 {
        return 0; /* cannot fit anything within dst budget */
    }
    if src_size > HUF_BLOCKSIZE_MAX {
        return err_code(ZSTD_error_srcSize_wrong); /* current block size limit */
    }
    if huff_log > HUF_TABLELOG_MAX {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }
    if max_symbol_value > HUF_SYMBOLVALUE_MAX {
        return err_code(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if max_symbol_value == 0 {
        max_symbol_value = HUF_SYMBOLVALUE_MAX;
    }
    if huff_log == 0 {
        huff_log = HUF_TABLELOG_DEFAULT;
    }

    /* Heuristic : If old table is valid, use it for small inputs */
    if (flags & HUF_flags_preferRepeat) != 0
        && !repeat.is_null()
        && *repeat == HUF_repeat_valid
    {
        return HUF_compressCTable_internal(
            ostart, op, oend, src, src_size, nb_streams, old_huf_table, flags,
        );
    }

    /* If uncompressible data is suspected, do a smaller sampling first */
    if (flags & HUF_flags_suspectUncompressible) != 0
        && src_size >= (SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE * SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO)
    {
        let mut largest_total: usize = 0;
        {
            let mut max_symbol_value_begin = max_symbol_value;
            let largest_begin = crate::fse_compress::HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut max_symbol_value_begin,
                src as *const BYTE as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            );
            /* HIST_count_simple returns unsigned; CHECK_V_F treats it as size_t */
            largest_total += largest_begin as usize;
        }
        {
            let mut max_symbol_value_end = max_symbol_value;
            let largest_end = crate::fse_compress::HIST_count_simple(
                (*table).count.as_mut_ptr(),
                &mut max_symbol_value_end,
                (src as *const BYTE)
                    .add(src_size - SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE)
                    as *const c_void,
                SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE,
            );
            largest_total += largest_end as usize;
        }
        if largest_total <= ((2 * SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE) >> 7) + 4 {
            return 0; /* heuristic : probably not compressible enough */
        }
    }

    /* Scan input and build symbol stats */
    {
        let largest = crate::fse_compress::HIST_count_wksp(
            (*table).count.as_mut_ptr(),
            &mut max_symbol_value,
            src as *const BYTE as *const c_void,
            src_size,
            (*table).wksps.hist_wksp.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.hist_wksp),
        );
        if err_is_error(largest) {
            return largest;
        }
        if largest == src_size {
            *ostart = *(src as *const BYTE).add(0);
            return 1; /* single symbol, rle */
        }
        if largest <= (src_size >> 7) + 4 {
            return 0; /* heuristic : probably not compressible enough */
        }
    }

    /* Check validity of previous table */
    if !repeat.is_null()
        && *repeat == HUF_repeat_check
        && HUF_validateCTable(old_huf_table, (*table).count.as_ptr(), max_symbol_value) == 0
    {
        *repeat = HUF_repeat_none;
    }
    /* Heuristic : use existing table for small inputs */
    if (flags & HUF_flags_preferRepeat) != 0 && !repeat.is_null() && *repeat != HUF_repeat_none {
        return HUF_compressCTable_internal(
            ostart, op, oend, src, src_size, nb_streams, old_huf_table, flags,
        );
    }

    /* Build Huffman Tree */
    huff_log = HUF_optimalTableLog(
        huff_log,
        src_size,
        max_symbol_value,
        &mut (*table).wksps as *mut HUF_compress_tables_t_wksps as *mut c_void,
        core::mem::size_of_val(&(*table).wksps),
        (*table).CTable.as_mut_ptr(),
        (*table).count.as_ptr(),
        flags,
    );
    {
        let max_bits = HUF_buildCTable_wksp(
            (*table).CTable.as_mut_ptr(),
            (*table).count.as_ptr(),
            max_symbol_value,
            huff_log,
            &mut (*table).wksps.buildCTable_wksp as *mut core::mem::ManuallyDrop<
                HUF_buildCTable_wksp_tables,
            > as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.buildCTable_wksp),
        );
        if err_is_error(max_bits) {
            return max_bits;
        }
        huff_log = max_bits as c_uint;
    }

    /* Write table description header */
    {
        let h_size = HUF_writeCTable_wksp(
            op as *mut c_void,
            dst_size,
            (*table).CTable.as_ptr(),
            max_symbol_value,
            huff_log,
            &mut (*table).wksps.writeCTable_wksp as *mut core::mem::ManuallyDrop<
                HUF_WriteCTableWksp,
            > as *mut c_void,
            core::mem::size_of_val(&(*table).wksps.writeCTable_wksp),
        );
        if err_is_error(h_size) {
            return h_size;
        }
        /* Check if using previous huffman table is beneficial */
        if !repeat.is_null() && *repeat != HUF_repeat_none {
            let old_size =
                HUF_estimateCompressedSize(old_huf_table, (*table).count.as_ptr(), max_symbol_value);
            let new_size = HUF_estimateCompressedSize(
                (*table).CTable.as_ptr(),
                (*table).count.as_ptr(),
                max_symbol_value,
            );
            if old_size <= h_size + new_size || h_size + 12 >= src_size {
                return HUF_compressCTable_internal(
                    ostart, op, oend, src, src_size, nb_streams, old_huf_table, flags,
                );
            }
        }

        /* Use the new huffman table */
        if h_size + 12usize >= src_size {
            return 0;
        }
        op = op.add(h_size);
        if !repeat.is_null() {
            *repeat = HUF_repeat_none;
        }
        if !old_huf_table.is_null() {
            core::ptr::copy_nonoverlapping(
                (*table).CTable.as_ptr(),
                old_huf_table,
                (*table).CTable.len(),
            ); /* Save new table */
        }
    }
    HUF_compressCTable_internal(
        ostart,
        op,
        oend,
        src,
        src_size,
        nb_streams,
        (*table).CTable.as_ptr(),
        flags,
    )
}

/// `HUF_compress1X_repeat()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_repeat(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    max_symbol_value: c_uint,
    huff_log: c_uint,
    work_space: *mut c_void,
    wksp_size: usize,
    huf_table: *mut HUF_CElt,
    repeat: *mut u32,
    flags: c_int,
) -> usize {
    HUF_compress_internal(
        dst,
        dst_size,
        src,
        src_size,
        max_symbol_value,
        huff_log,
        HUF_nbStreams_e::HUF_singleStream,
        work_space,
        wksp_size,
        huf_table,
        repeat,
        flags,
    )
}

/// `HUF_compress4X_repeat()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_repeat(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    max_symbol_value: c_uint,
    huff_log: c_uint,
    work_space: *mut c_void,
    wksp_size: usize,
    huf_table: *mut HUF_CElt,
    repeat: *mut u32,
    flags: c_int,
) -> usize {
    HUF_compress_internal(
        dst,
        dst_size,
        src,
        src_size,
        max_symbol_value,
        huff_log,
        HUF_nbStreams_e::HUF_fourStreams,
        work_space,
        wksp_size,
        huf_table,
        repeat,
        flags,
    )
}
