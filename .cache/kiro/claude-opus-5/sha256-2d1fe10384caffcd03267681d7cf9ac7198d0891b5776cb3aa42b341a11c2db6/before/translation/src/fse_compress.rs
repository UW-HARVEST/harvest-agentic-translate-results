//! Translation of `compress/fse_compress.c` and `compress/hist.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::zstd_highbit32;
use crate::bitstream::*;
use crate::error::*;
use crate::fse::*;
use crate::mem::*;

/* Build config (verified against portability_macros.h / compiler.h):
 *   DYNAMIC_BMI2 == 0  -> no BMI2/asm variants exist in these two files anyway.
 *   NDEBUG             -> C `assert()` is a no-op, so asserts are dropped (kept
 *                         only as `debug_assert!` where harmless).
 *   FSE_MAX_TABLELOG == 12 (FSE_MAX_MEMORY_USAGE==14).
 *   x86_64, little-endian, 64-bit: sizeof(BIT_CStream_t.bitContainer)*8 == 64.
 *   FSE_FUNCTION_TYPE == BYTE (from fse.h line 601), so `tableSymbol` is a
 *   `BYTE*` (u8) in FSE_buildCTable_wksp.
 */

/// `FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)` (fse.h)
#[inline(always)]
const fn fse_build_ctable_workspace_size_u32(max_symbol_value: c_uint, table_log: c_uint) -> usize {
    (((max_symbol_value as u64 + 2) + (1u64 << table_log)) / 2) as usize
        + core::mem::size_of::<U64>() / core::mem::size_of::<U32>()
}

/// `FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog)` (fse.h)
#[inline(always)]
const fn fse_build_ctable_workspace_size(max_symbol_value: c_uint, table_log: c_uint) -> usize {
    core::mem::size_of::<c_uint>()
        * fse_build_ctable_workspace_size_u32(max_symbol_value, table_log)
}

/// `FSE_TABLESTEP(tableSize)` — `((tableSize)>>1) + ((tableSize)>>3) + 3`
#[inline(always)]
const fn fse_tablestep_u32(table_size: U32) -> U32 {
    (table_size >> 1) + (table_size >> 3) + 3
}

/* ===== fse_compress.c ===== */

/* **************************************************************
 *  FSE_buildCTable_wksp
 ****************************************************************/

/// `FSE_buildCTable_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_wksp(
    ct: *mut FSE_CTable,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    table_log: c_uint,
    work_space: *mut c_void,
    wksp_size: usize,
) -> usize {
    let table_size: U32 = 1 << table_log;
    let table_mask: U32 = table_size - 1;
    let ptr = ct as *mut c_void;
    let table_u16 = (ptr as *mut U16).add(2);
    let fsct = (ptr as *mut U32)
        .add(1 /* header */ + if table_log != 0 { (table_size >> 1) as usize } else { 1 });
    let symbol_tt = fsct as *mut FSE_symbolCompressionTransform;
    let step: U32 = fse_tablestep_u32(table_size);
    let max_sv1: U32 = max_symbol_value + 1;

    let cumul = work_space as *mut U16; /* size = maxSV1 */
    /* FSE_FUNCTION_TYPE == BYTE */
    let table_symbol = cumul.add(max_sv1 as usize + 1) as *mut BYTE; /* size = tableSize */

    let mut high_threshold: U32 = table_size - 1;

    debug_assert!(((work_space as usize) & 1) == 0); /* Must be 2 bytes-aligned */
    if fse_build_ctable_workspace_size(max_symbol_value, table_log) > wksp_size {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }
    /* CTable header */
    *table_u16.offset(-2) = table_log as U16;
    *table_u16.offset(-1) = max_symbol_value as U16;
    debug_assert!(table_log < 16); /* required for threshold strategy to work */

    /* symbol start positions */
    {
        *cumul.add(0) = 0;
        let mut u: U32 = 1;
        while u <= max_sv1 {
            if *normalized_counter.add((u - 1) as usize) == -1 {
                /* Low proba symbol */
                *cumul.add(u as usize) = *cumul.add((u - 1) as usize) + 1;
                *table_symbol.add(high_threshold as usize) = (u - 1) as BYTE;
                high_threshold -= 1;
            } else {
                *cumul.add(u as usize) = *cumul.add((u - 1) as usize)
                    + *normalized_counter.add((u - 1) as usize) as U16;
            }
            u += 1;
        }
        *cumul.add(max_sv1 as usize) = (table_size + 1) as U16;
    }

    /* Spread symbols */
    if high_threshold == table_size - 1 {
        /* Case for no low prob count symbols. */
        let spread = table_symbol.add(table_size as usize); /* size = tableSize + 8 */
        {
            let add: U64 = 0x0101010101010101;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < max_sv1 {
                let n: c_int = *normalized_counter.add(s as usize) as c_int;
                mem_write64(spread.add(pos), sv);
                let mut i: c_int = 8;
                while i < n {
                    mem_write64(spread.add(pos + i as usize), sv);
                    i += 8;
                }
                pos += n as usize;
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        /* Spread symbols across the table. */
        {
            let mut position: usize = 0;
            let unroll: usize = 2; /* Experimentally determined optimal unroll */
            let mut s: usize = 0;
            while s < table_size as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let u_position = (position + (u * step as usize)) & table_mask as usize;
                    *table_symbol.add(u_position) = *spread.add(s + u);
                    u += 1;
                }
                position = (position + (unroll * step as usize)) & table_mask as usize;
                s += unroll;
            }
            debug_assert!(position == 0); /* Must have initialized all positions */
        }
    } else {
        let mut position: U32 = 0;
        let mut symbol: U32 = 0;
        while symbol < max_sv1 {
            let freq: c_int = *normalized_counter.add(symbol as usize) as c_int;
            let mut nb_occurrences: c_int = 0;
            while nb_occurrences < freq {
                *table_symbol.add(position as usize) = symbol as BYTE;
                position = (position + step) & table_mask;
                while position > high_threshold {
                    position = (position + step) & table_mask; /* Low proba area */
                }
                nb_occurrences += 1;
            }
            symbol += 1;
        }
        debug_assert!(position == 0); /* Must have initialized all positions */
    }

    /* Build table */
    {
        let mut u: U32 = 0;
        while u < table_size {
            let s = *table_symbol.add(u as usize); /* FSE_FUNCTION_TYPE */
            let idx = *cumul.add(s as usize);
            *cumul.add(s as usize) = idx + 1;
            *table_u16.add(idx as usize) = (table_size + u) as U16;
            u += 1;
        }
    }

    /* Build Symbol Transformation Table */
    {
        let mut total: c_uint = 0;
        let mut s: c_uint = 0;
        while s <= max_symbol_value {
            let nc = *normalized_counter.add(s as usize);
            match nc {
                0 => {
                    /* filling nonetheless, for compatibility with FSE_getMaxNbBits() */
                    (*symbol_tt.add(s as usize)).deltaNbBits =
                        ((table_log + 1) << 16).wrapping_sub(1 << table_log);
                }
                -1 | 1 => {
                    (*symbol_tt.add(s as usize)).deltaNbBits =
                        (table_log << 16).wrapping_sub(1 << table_log);
                    (*symbol_tt.add(s as usize)).deltaFindState = total.wrapping_sub(1) as c_int;
                    total += 1;
                }
                _ => {
                    /* normalizedCounter[s] > 1 */
                    let max_bits_out: U32 =
                        table_log - zstd_highbit32(nc as U32 - 1);
                    let min_state_plus: U32 = (nc as U32) << max_bits_out;
                    (*symbol_tt.add(s as usize)).deltaNbBits =
                        (max_bits_out << 16).wrapping_sub(min_state_plus);
                    (*symbol_tt.add(s as usize)).deltaFindState =
                        total.wrapping_sub(nc as c_uint) as c_int;
                    total += nc as c_uint;
                }
            }
            s += 1;
        }
    }

    0
}

/*-**************************************************************
 *  FSE NCount encoding
 ****************************************************************/

/// `FSE_NCountWriteBound()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_NCountWriteBound(
    max_symbol_value: c_uint,
    table_log: c_uint,
) -> usize {
    let max_header_size: usize = (((max_symbol_value + 1) * table_log
        + 4 /* bitCount initialized at 4 */
        + 2 /* first two symbols may use one additional bit each */) as usize
        / 8)
        + 1 /* round up to whole nb bytes */
        + 2 /* additional two bytes for bitstream flush */;
    if max_symbol_value != 0 {
        max_header_size
    } else {
        FSE_NCOUNTBOUND
    }
}

/// `FSE_writeNCount_generic()`
unsafe fn fse_write_ncount_generic(
    header: *mut c_void,
    header_buffer_size: usize,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    table_log: c_uint,
    write_is_safe: c_uint,
) -> usize {
    let ostart = header as *mut BYTE;
    let mut out = ostart;
    let oend = ostart.add(header_buffer_size);
    let mut nb_bits: c_int;
    let table_size: c_int = 1 << table_log;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bit_stream: U32 = 0;
    let mut bit_count: c_int = 0;
    let mut symbol: c_uint = 0;
    let alphabet_size: c_uint = max_symbol_value + 1;
    let mut previous_is0: c_int = 0;

    /* Table Size */
    bit_stream += (table_log - FSE_MIN_TABLELOG) << bit_count;
    bit_count += 4;

    /* Init */
    remaining = table_size + 1; /* +1 for extra accuracy */
    threshold = table_size;
    nb_bits = table_log as c_int + 1;

    while (symbol < alphabet_size) && (remaining > 1) {
        /* stops at 1 */
        if previous_is0 != 0 {
            let mut start: c_uint = symbol;
            while (symbol < alphabet_size) && (*normalized_counter.add(symbol as usize) == 0) {
                symbol += 1;
            }
            if symbol == alphabet_size {
                break; /* incorrect distribution */
            }
            while symbol >= start + 24 {
                start += 24;
                bit_stream += 0xFFFFu32 << bit_count;
                if (write_is_safe == 0) && (out > oend.sub(2)) {
                    return err_code(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
                }
                *out.add(0) = bit_stream as BYTE;
                *out.add(1) = (bit_stream >> 8) as BYTE;
                out = out.add(2);
                bit_stream >>= 16;
            }
            while symbol >= start + 3 {
                start += 3;
                bit_stream += 3u32 << bit_count;
                bit_count += 2;
            }
            bit_stream += (symbol - start) << bit_count;
            bit_count += 2;
            if bit_count > 16 {
                if (write_is_safe == 0) && (out > oend.sub(2)) {
                    return err_code(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
                }
                *out.add(0) = bit_stream as BYTE;
                *out.add(1) = (bit_stream >> 8) as BYTE;
                out = out.add(2);
                bit_stream >>= 16;
                bit_count -= 16;
            }
        }
        {
            let mut count: c_int = *normalized_counter.add(symbol as usize) as c_int;
            symbol += 1;
            let max: c_int = (2 * threshold - 1) - remaining;
            remaining -= if count < 0 { -count } else { count };
            count += 1; /* +1 for extra accuracy */
            if count >= threshold {
                count += max; /* [0..max[ [max..threshold[ (...) [threshold+max 2*threshold[ */
            }
            bit_stream += (count as U32) << bit_count;
            bit_count += nb_bits;
            bit_count -= (count < max) as c_int;
            previous_is0 = (count == 1) as c_int;
            if remaining < 1 {
                return err_code(ZSTD_error_GENERIC);
            }
            while remaining < threshold {
                nb_bits -= 1;
                threshold >>= 1;
            }
        }
        if bit_count > 16 {
            if (write_is_safe == 0) && (out > oend.sub(2)) {
                return err_code(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
            }
            *out.add(0) = bit_stream as BYTE;
            *out.add(1) = (bit_stream >> 8) as BYTE;
            out = out.add(2);
            bit_stream >>= 16;
            bit_count -= 16;
        }
    }

    if remaining != 1 {
        return err_code(ZSTD_error_GENERIC); /* incorrect normalized distribution */
    }
    debug_assert!(symbol <= alphabet_size);

    /* flush remaining bitStream */
    if (write_is_safe == 0) && (out > oend.sub(2)) {
        return err_code(ZSTD_error_dstSize_tooSmall); /* Buffer overflow */
    }
    *out.add(0) = bit_stream as BYTE;
    *out.add(1) = (bit_stream >> 8) as BYTE;
    out = out.add(((bit_count + 7) / 8) as usize);

    debug_assert!(out >= ostart);
    out as usize - ostart as usize
}

/// `FSE_writeNCount()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_writeNCount(
    buffer: *mut c_void,
    buffer_size: usize,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    table_log: c_uint,
) -> usize {
    if table_log > FSE_MAX_TABLELOG {
        return err_code(ZSTD_error_tableLog_tooLarge); /* Unsupported */
    }
    if table_log < FSE_MIN_TABLELOG {
        return err_code(ZSTD_error_GENERIC); /* Unsupported */
    }

    if buffer_size < FSE_NCountWriteBound(max_symbol_value, table_log) {
        return fse_write_ncount_generic(
            buffer,
            buffer_size,
            normalized_counter,
            max_symbol_value,
            table_log,
            0,
        );
    }

    fse_write_ncount_generic(
        buffer,
        buffer_size,
        normalized_counter,
        max_symbol_value,
        table_log,
        1, /* write in buffer is safe */
    )
}

/*-**************************************************************
 *  FSE Compression Code
 ****************************************************************/

/// `FSE_minTableLog()`
unsafe fn fse_min_table_log(src_size: usize, max_symbol_value: c_uint) -> c_uint {
    let min_bits_src: U32 = zstd_highbit32(src_size as U32) + 1;
    let min_bits_symbols: U32 = zstd_highbit32(max_symbol_value) + 2;
    let min_bits: U32 = if min_bits_src < min_bits_symbols {
        min_bits_src
    } else {
        min_bits_symbols
    };
    min_bits
}

/// `FSE_optimalTableLog_internal()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog_internal(
    max_table_log: c_uint,
    src_size: usize,
    max_symbol_value: c_uint,
    minus: c_uint,
) -> c_uint {
    let max_bits_src: U32 = zstd_highbit32((src_size - 1) as U32) - minus;
    let mut table_log: U32 = max_table_log;
    let min_bits: U32 = fse_min_table_log(src_size, max_symbol_value);
    if table_log == 0 {
        table_log = FSE_DEFAULT_TABLELOG;
    }
    if max_bits_src < table_log {
        table_log = max_bits_src; /* Accuracy can be reduced */
    }
    if min_bits > table_log {
        table_log = min_bits; /* Need a minimum to safely represent all symbol values */
    }
    if table_log < FSE_MIN_TABLELOG {
        table_log = FSE_MIN_TABLELOG;
    }
    if table_log > FSE_MAX_TABLELOG {
        table_log = FSE_MAX_TABLELOG;
    }
    table_log
}

/// `FSE_optimalTableLog()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog(
    max_table_log: c_uint,
    src_size: usize,
    max_symbol_value: c_uint,
) -> c_uint {
    FSE_optimalTableLog_internal(max_table_log, src_size, max_symbol_value, 2)
}

/// `FSE_normalizeM2()` — Secondary normalization method.
unsafe fn fse_normalize_m2(
    norm: *mut i16,
    table_log: U32,
    count: *const c_uint,
    mut total: usize,
    max_symbol_value: U32,
    low_prob_count: i16,
) -> usize {
    const NOT_YET_ASSIGNED: i16 = -2;
    let mut distributed: U32 = 0;
    let mut to_distribute: U32;

    /* Init */
    let low_threshold: U32 = (total >> table_log) as U32;
    let mut low_one: U32 = (((total as u64) * 3) >> (table_log + 1)) as U32;

    let mut s: U32 = 0;
    while s <= max_symbol_value {
        if *count.add(s as usize) == 0 {
            *norm.add(s as usize) = 0;
            s += 1;
            continue;
        }
        if *count.add(s as usize) <= low_threshold {
            *norm.add(s as usize) = low_prob_count;
            distributed += 1;
            total -= *count.add(s as usize) as usize;
            s += 1;
            continue;
        }
        if *count.add(s as usize) <= low_one {
            *norm.add(s as usize) = 1;
            distributed += 1;
            total -= *count.add(s as usize) as usize;
            s += 1;
            continue;
        }

        *norm.add(s as usize) = NOT_YET_ASSIGNED;
        s += 1;
    }
    to_distribute = (1 << table_log) - distributed;

    if to_distribute == 0 {
        return 0;
    }

    if (total / to_distribute as usize) > low_one as usize {
        /* risk of rounding to zero */
        low_one = (((total as u64) * 3) / ((to_distribute as u64) * 2)) as U32;
        let mut s: U32 = 0;
        while s <= max_symbol_value {
            if (*norm.add(s as usize) == NOT_YET_ASSIGNED)
                && (*count.add(s as usize) <= low_one)
            {
                *norm.add(s as usize) = 1;
                distributed += 1;
                total -= *count.add(s as usize) as usize;
                s += 1;
                continue;
            }
            s += 1;
        }
        to_distribute = (1 << table_log) - distributed;
    }

    if distributed == max_symbol_value + 1 {
        /* all values are pretty poor;
        probably incompressible data (should have already been detected);
        find max, then give all remaining points to max */
        let mut max_v: U32 = 0;
        let mut max_c: U32 = 0;
        let mut s: U32 = 0;
        while s <= max_symbol_value {
            if *count.add(s as usize) > max_c {
                max_v = s;
                max_c = *count.add(s as usize);
            }
            s += 1;
        }
        *norm.add(max_v as usize) += to_distribute as i16;
        return 0;
    }

    if total == 0 {
        /* all of the symbols were low enough for the lowOne or lowThreshold */
        let mut s: U32 = 0;
        while to_distribute > 0 {
            if *norm.add(s as usize) > 0 {
                to_distribute -= 1;
                *norm.add(s as usize) += 1;
            }
            s = (s + 1) % (max_symbol_value + 1);
        }
        return 0;
    }

    {
        let v_step_log: U64 = 62 - table_log as U64;
        let mid: U64 = (1u64 << (v_step_log - 1)) - 1;
        let r_step: U64 =
            ((((1u64) << v_step_log) * to_distribute as u64) + mid) / (total as U32) as U64; /* scale on remaining */
        let mut tmp_total: U64 = mid;
        let mut s: U32 = 0;
        while s <= max_symbol_value {
            if *norm.add(s as usize) == NOT_YET_ASSIGNED {
                let end: U64 = tmp_total + (*count.add(s as usize) as U64 * r_step);
                let s_start: U32 = (tmp_total >> v_step_log) as U32;
                let s_end: U32 = (end >> v_step_log) as U32;
                let weight: U32 = s_end - s_start;
                if weight < 1 {
                    return err_code(ZSTD_error_GENERIC);
                }
                *norm.add(s as usize) = weight as i16;
                tmp_total = end;
            }
            s += 1;
        }
    }

    0
}

/// `FSE_normalizeCount()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_normalizeCount(
    normalized_counter: *mut i16,
    mut table_log: c_uint,
    count: *const c_uint,
    total: usize,
    max_symbol_value: c_uint,
    use_low_prob_count: c_uint,
) -> usize {
    /* Sanity checks */
    if table_log == 0 {
        table_log = FSE_DEFAULT_TABLELOG;
    }
    if table_log < FSE_MIN_TABLELOG {
        return err_code(ZSTD_error_GENERIC); /* Unsupported size */
    }
    if table_log > FSE_MAX_TABLELOG {
        return err_code(ZSTD_error_tableLog_tooLarge); /* Unsupported size */
    }
    if table_log < fse_min_table_log(total, max_symbol_value) {
        return err_code(ZSTD_error_GENERIC); /* Too small tableLog, compression potentially impossible */
    }

    {
        static RTB_TABLE: [U32; 8] =
            [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];
        let low_prob_count: i16 = if use_low_prob_count != 0 { -1 } else { 1 };
        let scale: U64 = 62 - table_log as U64;
        let step: U64 = ((1u64) << 62) / (total as U32) as U64; /* <== here, one division ! */
        let v_step: U64 = 1u64 << (scale - 20);
        let mut still_to_distribute: c_int = 1 << table_log;
        let mut largest: c_uint = 0;
        let mut largest_p: i16 = 0;
        let low_threshold: U32 = (total >> table_log) as U32;

        let mut s: c_uint = 0;
        while s <= max_symbol_value {
            if *count.add(s as usize) as usize == total {
                return 0; /* rle special case */
            }
            if *count.add(s as usize) == 0 {
                *normalized_counter.add(s as usize) = 0;
                s += 1;
                continue;
            }
            if *count.add(s as usize) <= low_threshold {
                *normalized_counter.add(s as usize) = low_prob_count;
                still_to_distribute -= 1;
            } else {
                let mut proba: i16 =
                    ((*count.add(s as usize) as U64 * step) >> scale) as i16;
                if proba < 8 {
                    let rest_to_beat: U64 = v_step * RTB_TABLE[proba as usize] as U64;
                    proba += ((*count.add(s as usize) as U64 * step)
                        - ((proba as U64) << scale)
                        > rest_to_beat) as i16;
                }
                if proba > largest_p {
                    largest_p = proba;
                    largest = s;
                }
                *normalized_counter.add(s as usize) = proba;
                still_to_distribute -= proba as c_int;
            }
            s += 1;
        }
        if -still_to_distribute >= (*normalized_counter.add(largest as usize) >> 1) as c_int {
            /* corner case, need another normalization method */
            let error_code = fse_normalize_m2(
                normalized_counter,
                table_log,
                count,
                total,
                max_symbol_value,
                low_prob_count,
            );
            if err_is_error(error_code) {
                return error_code;
            }
        } else {
            *normalized_counter.add(largest as usize) += still_to_distribute as i16;
        }
    }

    table_log as usize
}

/// `FSE_buildCTable_rle()` — fake FSE_CTable, for rle input (always same symbol)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_rle(ct: *mut FSE_CTable, symbol_value: BYTE) -> usize {
    let ptr = ct as *mut c_void;
    let table_u16 = (ptr as *mut U16).add(2);
    let fsct_ptr = (ptr as *mut U32).add(2);
    let symbol_tt = fsct_ptr as *mut FSE_symbolCompressionTransform;

    /* header */
    *table_u16.offset(-2) = 0u16;
    *table_u16.offset(-1) = symbol_value as U16;

    /* Build table */
    *table_u16.add(0) = 0;
    *table_u16.add(1) = 0; /* just in case */

    /* Build Symbol Transformation Table */
    (*symbol_tt.add(symbol_value as usize)).deltaNbBits = 0;
    (*symbol_tt.add(symbol_value as usize)).deltaFindState = 0;

    0
}

/// `FSE_compress_usingCTable_generic()`
unsafe fn fse_compress_using_ctable_generic(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    mut src_size: usize,
    ct: *const FSE_CTable,
    fast: c_uint,
) -> usize {
    let istart = src as *const BYTE;
    let iend = istart.add(src_size);
    let mut ip = iend;

    let mut bit_c = BIT_CStream_t::default();
    let mut c_state1 = FSE_CState_t::default();
    let mut c_state2 = FSE_CState_t::default();

    /* init */
    if src_size <= 2 {
        return 0;
    }
    {
        let init_error = bit_init_cstream(&mut bit_c, dst as *mut u8, dst_size);
        if err_is_error(init_error) {
            return 0; /* not enough space available to write a bitstream */
        }
    }

    /* FSE_FLUSHBITS(s) : fast ? BIT_flushBitsFast(s) : BIT_flushBits(s) */
    macro_rules! flush_bits {
        ($s:expr) => {
            if fast != 0 {
                bit_flush_bits_fast($s)
            } else {
                bit_flush_bits($s)
            }
        };
    }

    if (src_size & 1) != 0 {
        ip = ip.sub(1);
        fse_init_cstate2(&mut c_state1, ct, *ip as U32);
        ip = ip.sub(1);
        fse_init_cstate2(&mut c_state2, ct, *ip as U32);
        ip = ip.sub(1);
        fse_encode_symbol(&mut bit_c, &mut c_state1, *ip as u32);
        flush_bits!(&mut bit_c);
    } else {
        ip = ip.sub(1);
        fse_init_cstate2(&mut c_state2, ct, *ip as U32);
        ip = ip.sub(1);
        fse_init_cstate2(&mut c_state1, ct, *ip as U32);
    }

    /* join to mod 4 */
    src_size -= 2;
    /* sizeof(bitContainer)*8 == 64 > FSE_MAX_TABLELOG*4+7 == 55  -> true */
    if (core::mem::size_of::<BitContainerType>() * 8 > FSE_MAX_TABLELOG as usize * 4 + 7)
        && (src_size & 2) != 0
    {
        /* test bit 2 */
        ip = ip.sub(1);
        fse_encode_symbol(&mut bit_c, &mut c_state2, *ip as u32);
        ip = ip.sub(1);
        fse_encode_symbol(&mut bit_c, &mut c_state1, *ip as u32);
        flush_bits!(&mut bit_c);
    }

    /* 2 or 4 encoding per loop */
    while ip > istart {
        ip = ip.sub(1);
        fse_encode_symbol(&mut bit_c, &mut c_state2, *ip as u32);

        /* sizeof(bitContainer)*8 == 64 < FSE_MAX_TABLELOG*2+7 == 31 -> false */
        if core::mem::size_of::<BitContainerType>() * 8 < FSE_MAX_TABLELOG as usize * 2 + 7 {
            flush_bits!(&mut bit_c);
        }

        ip = ip.sub(1);
        fse_encode_symbol(&mut bit_c, &mut c_state1, *ip as u32);

        /* sizeof(bitContainer)*8 == 64 > FSE_MAX_TABLELOG*4+7 == 55 -> true */
        if core::mem::size_of::<BitContainerType>() * 8 > FSE_MAX_TABLELOG as usize * 4 + 7 {
            ip = ip.sub(1);
            fse_encode_symbol(&mut bit_c, &mut c_state2, *ip as u32);
            ip = ip.sub(1);
            fse_encode_symbol(&mut bit_c, &mut c_state1, *ip as u32);
        }

        flush_bits!(&mut bit_c);
    }

    fse_flush_cstate(&mut bit_c, &c_state2);
    fse_flush_cstate(&mut bit_c, &c_state1);
    bit_close_cstream(&mut bit_c)
}

/// `FSE_compress_usingCTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compress_usingCTable(
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    ct: *const FSE_CTable,
) -> usize {
    let fast: c_uint = (dst_size >= fse_blockbound(src_size)) as c_uint;

    if fast != 0 {
        fse_compress_using_ctable_generic(dst, dst_size, src, src_size, ct, 1)
    } else {
        fse_compress_using_ctable_generic(dst, dst_size, src, src_size, ct, 0)
    }
}

/// `FSE_compressBound()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compressBound(size: usize) -> usize {
    fse_compressbound(size)
}

/* ===== hist.c ===== */

/* --- Error management --- */

/// `HIST_isError()`
#[unsafe(no_mangle)]
pub extern "C" fn HIST_isError(code: usize) -> c_uint {
    err_is_error(code) as c_uint
}

/*-**************************************************************
 *  Histogram functions
 ****************************************************************/

/// `HIST_add()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_add(count: *mut c_uint, src: *const c_void, src_size: usize) {
    let mut ip = src as *const BYTE;
    let end = ip.add(src_size);

    while ip < end {
        let v = *ip;
        ip = ip.add(1);
        *count.add(v as usize) += 1;
    }
}

/// `HIST_count_simple()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_simple(
    count: *mut c_uint,
    max_symbol_value_ptr: *mut c_uint,
    src: *const c_void,
    src_size: usize,
) -> c_uint {
    let mut ip = src as *const BYTE;
    let end = ip.add(src_size);
    let mut max_symbol_value: c_uint = *max_symbol_value_ptr;
    let mut largest_count: c_uint = 0;

    core::ptr::write_bytes(
        count,
        0,
        max_symbol_value as usize + 1,
    );
    if src_size == 0 {
        *max_symbol_value_ptr = 0;
        return 0;
    }

    while ip < end {
        let v = *ip;
        ip = ip.add(1);
        *count.add(v as usize) += 1;
    }

    while *count.add(max_symbol_value as usize) == 0 {
        max_symbol_value -= 1;
    }
    *max_symbol_value_ptr = max_symbol_value;

    {
        let mut s: U32 = 0;
        while s <= max_symbol_value {
            if *count.add(s as usize) > largest_count {
                largest_count = *count.add(s as usize);
            }
            s += 1;
        }
    }

    largest_count
}

/// `HIST_checkInput_e`
#[derive(Clone, Copy, PartialEq, Eq)]
enum HIST_checkInput_e {
    trustInput,
    checkMaxSymbolValue,
}

/// `HIST_count_parallel_wksp()`
///
/// store histogram into 4 intermediate tables, recombined at the end.
unsafe fn hist_count_parallel_wksp(
    count: *mut c_uint,
    max_symbol_value_ptr: *mut c_uint,
    source: *const c_void,
    source_size: usize,
    check: HIST_checkInput_e,
    work_space: *mut U32,
) -> usize {
    let mut ip = source as *const BYTE;
    let iend = ip.add(source_size);
    let count_size: usize = (*max_symbol_value_ptr as usize + 1) * core::mem::size_of::<c_uint>();
    let mut max: c_uint = 0;
    let counting1: *mut U32 = work_space;
    let counting2: *mut U32 = counting1.add(256);
    let counting3: *mut U32 = counting2.add(256);
    let counting4: *mut U32 = counting3.add(256);

    /* safety checks */
    debug_assert!(*max_symbol_value_ptr <= 255);
    if source_size == 0 {
        core::ptr::write_bytes(count as *mut u8, 0, count_size);
        *max_symbol_value_ptr = 0;
        return 0;
    }
    core::ptr::write_bytes(work_space as *mut u8, 0, 4 * 256 * core::mem::size_of::<c_uint>());

    /* by stripes of 16 bytes */
    {
        let mut cached: U32 = mem_read32(ip);
        ip = ip.add(4);
        while ip < iend.sub(15) {
            let mut c: U32 = cached;
            cached = mem_read32(ip);
            ip = ip.add(4);
            *counting1.add((c as BYTE) as usize) += 1;
            *counting2.add(((c >> 8) as BYTE) as usize) += 1;
            *counting3.add(((c >> 16) as BYTE) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = mem_read32(ip);
            ip = ip.add(4);
            *counting1.add((c as BYTE) as usize) += 1;
            *counting2.add(((c >> 8) as BYTE) as usize) += 1;
            *counting3.add(((c >> 16) as BYTE) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = mem_read32(ip);
            ip = ip.add(4);
            *counting1.add((c as BYTE) as usize) += 1;
            *counting2.add(((c >> 8) as BYTE) as usize) += 1;
            *counting3.add(((c >> 16) as BYTE) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = mem_read32(ip);
            ip = ip.add(4);
            *counting1.add((c as BYTE) as usize) += 1;
            *counting2.add(((c >> 8) as BYTE) as usize) += 1;
            *counting3.add(((c >> 16) as BYTE) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
        }
        ip = ip.sub(4);
    }

    /* finish last symbols */
    while ip < iend {
        let v = *ip;
        ip = ip.add(1);
        *counting1.add(v as usize) += 1;
    }

    {
        let mut s: U32 = 0;
        while s < 256 {
            *counting1.add(s as usize) += *counting2.add(s as usize)
                + *counting3.add(s as usize)
                + *counting4.add(s as usize);
            if *counting1.add(s as usize) > max {
                max = *counting1.add(s as usize);
            }
            s += 1;
        }
    }

    {
        let mut max_symbol_value: c_uint = 255;
        while *counting1.add(max_symbol_value as usize) == 0 {
            max_symbol_value -= 1;
        }
        if (check == HIST_checkInput_e::checkMaxSymbolValue)
            && max_symbol_value > *max_symbol_value_ptr
        {
            return err_code(ZSTD_error_maxSymbolValue_tooSmall);
        }
        *max_symbol_value_ptr = max_symbol_value;
        core::ptr::copy(counting1 as *const u8, count as *mut u8, count_size); /* in case count & Counting1 are overlapping */
    }
    max as usize
}

/// `HIST_countFast_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_countFast_wksp(
    count: *mut c_uint,
    max_symbol_value_ptr: *mut c_uint,
    source: *const c_void,
    source_size: usize,
    work_space: *mut c_void,
    work_space_size: usize,
) -> usize {
    if source_size < 1500 {
        /* heuristic threshold */
        return HIST_count_simple(count, max_symbol_value_ptr, source, source_size) as usize;
    }
    if (work_space as usize) & 3 != 0 {
        return err_code(ZSTD_error_GENERIC); /* must be aligned on 4-bytes boundaries */
    }
    if work_space_size < HIST_WKSP_SIZE {
        return err_code(ZSTD_error_workSpace_tooSmall);
    }
    hist_count_parallel_wksp(
        count,
        max_symbol_value_ptr,
        source,
        source_size,
        HIST_checkInput_e::trustInput,
        work_space as *mut U32,
    )
}

/// `HIST_count_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_wksp(
    count: *mut c_uint,
    max_symbol_value_ptr: *mut c_uint,
    source: *const c_void,
    source_size: usize,
    work_space: *mut c_void,
    work_space_size: usize,
) -> usize {
    if (work_space as usize) & 3 != 0 {
        return err_code(ZSTD_error_GENERIC); /* must be aligned on 4-bytes boundaries */
    }
    if work_space_size < HIST_WKSP_SIZE {
        return err_code(ZSTD_error_workSpace_tooSmall);
    }
    if *max_symbol_value_ptr < 255 {
        return hist_count_parallel_wksp(
            count,
            max_symbol_value_ptr,
            source,
            source_size,
            HIST_checkInput_e::checkMaxSymbolValue,
            work_space as *mut U32,
        );
    }
    *max_symbol_value_ptr = 255;
    HIST_countFast_wksp(
        count,
        max_symbol_value_ptr,
        source,
        source_size,
        work_space,
        work_space_size,
    )
}

/* HIST_WKSP_SIZE_U32 / HIST_WKSP_SIZE (hist.h) */
/// `HIST_WKSP_SIZE_U32`
pub const HIST_WKSP_SIZE_U32: usize = 1024;
/// `HIST_WKSP_SIZE`
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * core::mem::size_of::<c_uint>();

/* ZSTD_NO_UNUSED_FUNCTIONS is not defined, so these two are present. */

/// `HIST_countFast()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_countFast(
    count: *mut c_uint,
    max_symbol_value_ptr: *mut c_uint,
    source: *const c_void,
    source_size: usize,
) -> usize {
    let mut tmp_counters = [0u32; HIST_WKSP_SIZE_U32];
    HIST_countFast_wksp(
        count,
        max_symbol_value_ptr,
        source,
        source_size,
        tmp_counters.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmp_counters),
    )
}

/// `HIST_count()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count(
    count: *mut c_uint,
    max_symbol_value_ptr: *mut c_uint,
    src: *const c_void,
    src_size: usize,
) -> usize {
    let mut tmp_counters = [0u32; HIST_WKSP_SIZE_U32];
    HIST_count_wksp(
        count,
        max_symbol_value_ptr,
        src,
        src_size,
        tmp_counters.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmp_counters),
    )
}
