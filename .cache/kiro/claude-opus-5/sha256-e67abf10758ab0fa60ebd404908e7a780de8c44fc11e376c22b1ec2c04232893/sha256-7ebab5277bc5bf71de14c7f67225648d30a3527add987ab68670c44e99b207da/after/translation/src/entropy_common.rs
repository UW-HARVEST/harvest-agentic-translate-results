//! Translation of `common/entropy_common.c`.
//!
//! `DYNAMIC_BMI2=0` in the C build, so the `_bmi2` variants forward to the
//! default body; that is reproduced here.
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

use crate::bits::*;
use crate::error::*;
use crate::fse::*;
use crate::fse_decompress::FSE_decompress_wksp_bmi2;
use crate::huf::*;
use crate::mem::*;

/// `FSE_readNCount_body()`
unsafe fn fse_read_ncount_body(
    normalized_counter: *mut i16,
    max_sv_ptr: *mut u32,
    table_log_ptr: *mut u32,
    header_buffer: *const u8,
    hb_size: usize,
) -> usize {
    let istart = header_buffer;
    let iend = istart.add(hb_size);
    let mut ip = istart;
    let mut nb_bits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bit_stream: U32;
    let mut bit_count: c_int;
    let mut charnum: u32 = 0;
    let max_sv1: u32 = *max_sv_ptr + 1;
    let mut previous0 = false;

    if hb_size < 8 {
        /* This function only works when hbSize >= 8 */
        let mut buffer = [0u8; 8];
        core::ptr::copy_nonoverlapping(header_buffer, buffer.as_mut_ptr(), hb_size);
        let count_size = FSE_readNCount(
            normalized_counter,
            max_sv_ptr,
            table_log_ptr,
            buffer.as_ptr() as *const c_void,
            8,
        );
        if err_is_error(count_size) {
            return count_size;
        }
        if count_size > hb_size {
            return err_code(ZSTD_error_corruption_detected);
        }
        return count_size;
    }

    /* init */
    core::ptr::write_bytes(normalized_counter, 0, (*max_sv_ptr + 1) as usize);
    bit_stream = mem_read_le32(ip);
    nb_bits = ((bit_stream & 0xF) + FSE_MIN_TABLELOG) as c_int;
    if nb_bits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }
    bit_stream >>= 4;
    bit_count = 4;
    *table_log_ptr = nb_bits as u32;
    remaining = (1 << nb_bits) + 1;
    threshold = 1 << nb_bits;
    nb_bits += 1;

    loop {
        if previous0 {
            /* Count the number of repeats; 0b11 means "another repeat". */
            let mut repeats = (zstd_count_trailing_zeros32(!bit_stream | 0x80000000) >> 1) as c_int;
            while repeats >= 12 {
                charnum += 3 * 12;
                if ip <= iend.sub(7) {
                    ip = ip.add(3);
                } else {
                    bit_count -= (8 * (iend as isize - 7 - ip as isize)) as c_int;
                    bit_count &= 31;
                    ip = iend.sub(4);
                }
                bit_stream = mem_read_le32(ip) >> bit_count;
                repeats = (zstd_count_trailing_zeros32(!bit_stream | 0x80000000) >> 1) as c_int;
            }
            charnum += 3 * repeats as u32;
            bit_stream >>= 2 * repeats;
            bit_count += 2 * repeats;

            /* Add the final repeat which isn't 0b11. */
            charnum += bit_stream & 3;
            bit_count += 2;

            /* This is an error, but break and return an error at the end. */
            if charnum >= max_sv1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.offset((bit_count >> 3) as isize) <= iend.sub(4)) {
                ip = ip.offset((bit_count >> 3) as isize);
                bit_count &= 7;
            } else {
                bit_count -= (8 * (iend as isize - 4 - ip as isize)) as c_int;
                bit_count &= 31;
                ip = iend.sub(4);
            }
            bit_stream = mem_read_le32(ip) >> bit_count;
        }
        {
            let max: c_int = (2 * threshold - 1) - remaining;
            let mut count: c_int;

            if (bit_stream & (threshold - 1) as U32) < max as U32 {
                count = (bit_stream & (threshold - 1) as U32) as c_int;
                bit_count += nb_bits - 1;
            } else {
                count = (bit_stream & (2 * threshold - 1) as U32) as c_int;
                if count >= threshold {
                    count -= max;
                }
                bit_count += nb_bits;
            }

            count -= 1; /* extra accuracy */
            if count >= 0 {
                remaining -= count;
            } else {
                remaining += count;
            }
            *normalized_counter.add(charnum as usize) = count as i16;
            charnum += 1;
            previous0 = count == 0;

            if remaining < threshold {
                if remaining <= 1 {
                    break;
                }
                nb_bits = zstd_highbit32(remaining as U32) as c_int + 1;
                threshold = 1 << (nb_bits - 1);
            }
            if charnum >= max_sv1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.offset((bit_count >> 3) as isize) <= iend.sub(4)) {
                ip = ip.offset((bit_count >> 3) as isize);
                bit_count &= 7;
            } else {
                bit_count -= (8 * (iend as isize - 4 - ip as isize)) as c_int;
                bit_count &= 31;
                ip = iend.sub(4);
            }
            bit_stream = mem_read_le32(ip) >> bit_count;
        }
    }
    if remaining != 1 {
        return err_code(ZSTD_error_corruption_detected);
    }
    if charnum > max_sv1 {
        return err_code(ZSTD_error_maxSymbolValue_tooSmall);
    }
    if bit_count > 32 {
        return err_code(ZSTD_error_corruption_detected);
    }
    *max_sv_ptr = charnum - 1;

    ip = ip.offset(((bit_count + 7) >> 3) as isize);
    ip as usize - istart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount_bmi2(
    normalized_counter: *mut i16,
    max_sv_ptr: *mut u32,
    table_log_ptr: *mut u32,
    header_buffer: *const c_void,
    hb_size: usize,
    _bmi2: c_int,
) -> usize {
    fse_read_ncount_body(
        normalized_counter,
        max_sv_ptr,
        table_log_ptr,
        header_buffer as *const u8,
        hb_size,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount(
    normalized_counter: *mut i16,
    max_sv_ptr: *mut u32,
    table_log_ptr: *mut u32,
    header_buffer: *const c_void,
    hb_size: usize,
) -> usize {
    FSE_readNCount_bmi2(
        normalized_counter,
        max_sv_ptr,
        table_log_ptr,
        header_buffer,
        hb_size,
        0,
    )
}

/// `HUF_readStats()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats(
    huff_weight: *mut u8,
    hw_size: usize,
    rank_stats: *mut U32,
    nb_symbols_ptr: *mut U32,
    table_log_ptr: *mut U32,
    src: *const c_void,
    src_size: usize,
) -> usize {
    let mut wksp = [0u32; HUF_READ_STATS_WORKSPACE_SIZE_U32];
    HUF_readStats_wksp(
        huff_weight,
        hw_size,
        rank_stats,
        nb_symbols_ptr,
        table_log_ptr,
        src,
        src_size,
        wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&wksp),
        0,
    )
}

/// `HUF_readStats_body()`
unsafe fn huf_read_stats_body(
    huff_weight: *mut u8,
    hw_size: usize,
    rank_stats: *mut U32,
    nb_symbols_ptr: *mut U32,
    table_log_ptr: *mut U32,
    src: *const u8,
    src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    bmi2: c_int,
) -> usize {
    let weight_total: U32;
    let mut ip = src;
    let mut i_size: usize;
    let o_size: usize;

    if src_size == 0 {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    i_size = *ip.add(0) as usize;

    if i_size >= 128 {
        /* special header */
        o_size = i_size - 127;
        i_size = (o_size + 1) / 2;
        if i_size + 1 > src_size {
            return err_code(ZSTD_error_srcSize_wrong);
        }
        if o_size >= hw_size {
            return err_code(ZSTD_error_corruption_detected);
        }
        ip = ip.add(1);
        let mut n = 0usize;
        while n < o_size {
            *huff_weight.add(n) = *ip.add(n / 2) >> 4;
            *huff_weight.add(n + 1) = *ip.add(n / 2) & 15;
            n += 2;
        }
    } else {
        /* header compressed with FSE (normal case) */
        if i_size + 1 > src_size {
            return err_code(ZSTD_error_srcSize_wrong);
        }
        o_size = FSE_decompress_wksp_bmi2(
            huff_weight as *mut c_void,
            hw_size - 1,
            ip.add(1) as *const c_void,
            i_size,
            6,
            work_space,
            wksp_size,
            bmi2,
        );
        if err_is_error(o_size) {
            return o_size;
        }
    }

    /* collect weight stats */
    core::ptr::write_bytes(rank_stats, 0, HUF_TABLELOG_MAX as usize + 1);
    let mut wt: U32 = 0;
    for n in 0..o_size {
        let w = *huff_weight.add(n);
        if w as U32 > HUF_TABLELOG_MAX {
            return err_code(ZSTD_error_corruption_detected);
        }
        *rank_stats.add(w as usize) += 1;
        wt += (1u32 << w) >> 1;
    }
    weight_total = wt;
    if weight_total == 0 {
        return err_code(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    let table_log = zstd_highbit32(weight_total) + 1;
    if table_log > HUF_TABLELOG_MAX {
        return err_code(ZSTD_error_corruption_detected);
    }
    *table_log_ptr = table_log;
    {
        let total = 1u32 << table_log;
        let rest = total - weight_total;
        let verif = 1u32 << zstd_highbit32(rest);
        let last_weight = zstd_highbit32(rest) + 1;
        if verif != rest {
            return err_code(ZSTD_error_corruption_detected);
        }
        *huff_weight.add(o_size) = last_weight as u8;
        *rank_stats.add(last_weight as usize) += 1;
    }

    /* check tree construction validity */
    if (*rank_stats.add(1) < 2) || (*rank_stats.add(1) & 1) != 0 {
        return err_code(ZSTD_error_corruption_detected);
    }

    *nb_symbols_ptr = (o_size + 1) as U32;
    i_size + 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats_wksp(
    huff_weight: *mut u8,
    hw_size: usize,
    rank_stats: *mut U32,
    nb_symbols_ptr: *mut U32,
    table_log_ptr: *mut U32,
    src: *const c_void,
    src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    _flags: c_int,
) -> usize {
    huf_read_stats_body(
        huff_weight,
        hw_size,
        rank_stats,
        nb_symbols_ptr,
        table_log_ptr,
        src as *const u8,
        src_size,
        work_space,
        wksp_size,
        0,
    )
}
