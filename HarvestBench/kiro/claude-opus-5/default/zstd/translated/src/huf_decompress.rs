//! Translation of `decompress/huf_decompress.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::*;
use crate::bitstream::*;
use crate::error::*;
use crate::huf::*;
use crate::mem::*;
use crate::zstd_internal::min_usize;

/* **************************************************************
 *  Constants
 ****************************************************************/

const HUF_DECODER_FAST_TABLELOG: u32 = 11;

/* With the build configuration (DYNAMIC_BMI2=0, no -mbmi2), HUF_ENABLE_FAST_DECODE
 * is 1 (HUF_DISABLE_FAST_DECODE is not defined). */
const HUF_ENABLE_FAST_DECODE: bool = true;

/* **************************************************************
 *  Helpers matching common macros used by this file
 ****************************************************************/

/// `ZSTD_maybeNullPtrAdd(ptr, add)` — `add > 0 ? ptr + add : ptr`.
/// `add` has type `ptrdiff_t` (isize) in C.
#[inline(always)]
unsafe fn zstd_maybe_null_ptr_add(ptr: *mut u8, add: isize) -> *mut u8 {
    if add > 0 {
        ptr.offset(add)
    } else {
        ptr
    }
}

/// `MIN(a, b)`
#[inline(always)]
fn min_u32(a: u32, b: u32) -> u32 {
    if a < b {
        a
    } else {
        b
    }
}

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/

/// `DTableDesc`
#[repr(C)]
#[derive(Clone, Copy)]
struct DTableDesc {
    max_table_log: BYTE,
    table_type: BYTE,
    table_log: BYTE,
    reserved: BYTE,
}

/// `HUF_getDTableDesc()`
#[inline]
unsafe fn huf_get_dtable_desc(table: *const HUF_DTable) -> DTableDesc {
    let mut dtd = DTableDesc {
        max_table_log: 0,
        table_type: 0,
        table_log: 0,
        reserved: 0,
    };
    core::ptr::copy_nonoverlapping(
        table as *const u8,
        &mut dtd as *mut DTableDesc as *mut u8,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

/// `HUF_initFastDStream()`
#[inline]
unsafe fn huf_init_fast_dstream(ip: *const BYTE) -> usize {
    let last_byte = *ip.add(7);
    let bits_consumed = if last_byte != 0 {
        (8 - zstd_highbit32(last_byte as U32)) as usize
    } else {
        0usize
    };
    let value = mem_read_lest(ip) | 1;
    debug_assert!(bits_consumed <= 8);
    debug_assert!(core::mem::size_of::<usize>() == 8);
    value << bits_consumed
}

/// `HUF_DecompressFastArgs`
#[repr(C)]
struct HUF_DecompressFastArgs {
    ip: [*const BYTE; 4],
    op: [*mut BYTE; 4],
    bits: [U64; 4],
    dt: *const c_void,
    ilowest: *const BYTE,
    oend: *mut BYTE,
    iend: [*const BYTE; 4],
}

type HUF_DecompressFastLoopFn = unsafe fn(*mut HUF_DecompressFastArgs);

/// `HUF_DecompressFastArgs_init()`
///
/// @returns 1 on success, 0 if the fallback implementation should be used,
///          or an error code on failure.
unsafe fn huf_decompress_fast_args_init(
    args: *mut HUF_DecompressFastArgs,
    dst: *mut c_void,
    dst_size: usize,
    src: *const c_void,
    src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    let dt = dtable.add(1) as *const c_void;
    let dt_log: U32 = huf_get_dtable_desc(dtable).table_log as U32;

    let istart = src as *const BYTE;

    let oend = zstd_maybe_null_ptr_add(dst as *mut BYTE, dst_size as isize);

    /* The fast decoding loop assumes 64-bit little-endian. */
    if !mem_is_little_endian() || mem_32bits() {
        return 0;
    }

    /* Avoid nullptr addition */
    if dst_size == 0 {
        return 0;
    }
    debug_assert!(!dst.is_null());

    /* strict minimum : jump table + 1 byte per stream */
    if src_size < 10 {
        return err_code(ZSTD_error_corruption_detected);
    }

    if dt_log != HUF_DECODER_FAST_TABLELOG {
        return 0;
    }

    /* Read the jump table. */
    {
        let length1 = mem_read_le16(istart) as usize;
        let length2 = mem_read_le16(istart.add(2)) as usize;
        let length3 = mem_read_le16(istart.add(4)) as usize;
        let length4 = src_size.wrapping_sub(length1 + length2 + length3 + 6);
        (*args).iend[0] = istart.add(6); /* jumpTable */
        (*args).iend[1] = (*args).iend[0].add(length1);
        (*args).iend[2] = (*args).iend[1].add(length2);
        (*args).iend[3] = (*args).iend[2].add(length3);

        if length1 < 8 || length2 < 8 || length3 < 8 || length4 < 8 {
            return 0;
        }
        if length4 > src_size {
            return err_code(ZSTD_error_corruption_detected); /* overflow */
        }
    }
    /* ip[] contains the position that is currently loaded into bits[]. */
    (*args).ip[0] = (*args).iend[1].sub(core::mem::size_of::<U64>());
    (*args).ip[1] = (*args).iend[2].sub(core::mem::size_of::<U64>());
    (*args).ip[2] = (*args).iend[3].sub(core::mem::size_of::<U64>());
    (*args).ip[3] = (src as *const BYTE)
        .add(src_size)
        .sub(core::mem::size_of::<U64>());

    /* op[] contains the output pointers. */
    (*args).op[0] = dst as *mut BYTE;
    (*args).op[1] = (*args).op[0].add((dst_size + 3) / 4);
    (*args).op[2] = (*args).op[1].add((dst_size + 3) / 4);
    (*args).op[3] = (*args).op[2].add((dst_size + 3) / 4);

    /* No point to call the ASM loop for tiny outputs. */
    if (*args).op[3] >= oend {
        return 0;
    }

    (*args).bits[0] = huf_init_fast_dstream((*args).ip[0]) as U64;
    (*args).bits[1] = huf_init_fast_dstream((*args).ip[1]) as U64;
    (*args).bits[2] = huf_init_fast_dstream((*args).ip[2]) as U64;
    (*args).bits[3] = huf_init_fast_dstream((*args).ip[3]) as U64;

    (*args).ilowest = istart;

    (*args).oend = oend;
    (*args).dt = dt;

    1
}

/// `HUF_initRemainingDStream()`
unsafe fn huf_init_remaining_dstream(
    bit: *mut BIT_DStream_t,
    args: *const HUF_DecompressFastArgs,
    stream: c_int,
    segment_end: *mut BYTE,
) -> usize {
    let s = stream as usize;
    /* Validate that we haven't overwritten. */
    if (*args).op[s] > segment_end {
        return err_code(ZSTD_error_corruption_detected);
    }
    /* Validate that we haven't read beyond iend[]. */
    if (*args).ip[s] < (*args).iend[s].sub(8) {
        return err_code(ZSTD_error_corruption_detected);
    }

    /* Construct the BIT_DStream_t. */
    debug_assert!(core::mem::size_of::<usize>() == 8);
    (*bit).bitContainer = mem_read_lest((*args).ip[s]);
    (*bit).bitsConsumed = zstd_count_trailing_zeros64((*args).bits[s]);
    (*bit).start = (*args).ilowest as *const u8;
    (*bit).limitPtr = (*bit).start.add(core::mem::size_of::<usize>());
    (*bit).ptr = (*args).ip[s] as *const u8;

    0
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

/// `HUF_DEltX1`
#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX1 {
    nb_bits: BYTE,
    byte: BYTE,
}

/// `HUF_DEltX1_set4()`
#[inline]
fn huf_delt_x1_set4(symbol: BYTE, nb_bits: BYTE) -> U64 {
    let mut d4: U64;
    if mem_is_little_endian() {
        d4 = (((symbol as c_int) << 8) + nb_bits as c_int) as U64;
    } else {
        d4 = (symbol as c_int + ((nb_bits as c_int) << 8)) as U64;
    }
    debug_assert!(d4 < (1u32 << 16) as U64);
    d4 = d4.wrapping_mul(0x0001000100010001u64);
    d4
}

/// `HUF_rescaleStats()`
unsafe fn huf_rescale_stats(
    huff_weight: *mut BYTE,
    rank_val: *mut U32,
    nb_symbols: U32,
    table_log: U32,
    target_table_log: U32,
) -> U32 {
    if table_log > target_table_log {
        return table_log;
    }
    if table_log < target_table_log {
        let scale = target_table_log - table_log;
        let mut s: U32;
        s = 0;
        while s < nb_symbols {
            let hw = *huff_weight.add(s as usize);
            *huff_weight.add(s as usize) =
                hw.wrapping_add(if hw == 0 { 0 } else { scale as BYTE });
            s += 1;
        }
        s = target_table_log;
        while s > scale {
            *rank_val.add(s as usize) = *rank_val.add((s - scale) as usize);
            s -= 1;
        }
        s = scale;
        while s > 0 {
            *rank_val.add(s as usize) = 0;
            s -= 1;
        }
    }
    target_table_log
}

/// `HUF_ReadDTableX1_Workspace`
#[repr(C)]
struct HUF_ReadDTableX1_Workspace {
    rank_val: [U32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize],
    rank_start: [U32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize],
    stats_wksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
    symbols: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    huff_weight: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize],
}

/// `HUF_readDTableX1_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX1_wksp(
    dtable: *mut HUF_DTable,
    src: *const c_void,
    src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    let mut table_log: U32 = 0;
    let mut nb_symbols: U32 = 0;
    let i_size: usize;
    let dt_ptr = dtable.add(1) as *mut c_void;
    let dt = dt_ptr as *mut HUF_DEltX1;
    let wksp = work_space as *mut HUF_ReadDTableX1_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX1_Workspace>() > wksp_size {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }

    i_size = crate::entropy_common::HUF_readStats_wksp(
        (*wksp).huff_weight.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as usize,
        (*wksp).rank_val.as_mut_ptr(),
        &mut nb_symbols,
        &mut table_log,
        src,
        src_size,
        (*wksp).stats_wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).stats_wksp),
        flags,
    );
    if err_is_error(i_size) {
        return i_size;
    }

    /* Table header */
    {
        let mut dtd = huf_get_dtable_desc(dtable);
        let max_table_log = dtd.max_table_log as U32 + 1;
        let target_table_log = min_u32(max_table_log, HUF_DECODER_FAST_TABLELOG);
        table_log = huf_rescale_stats(
            (*wksp).huff_weight.as_mut_ptr(),
            (*wksp).rank_val.as_mut_ptr(),
            nb_symbols,
            table_log,
            target_table_log,
        );
        if table_log > (dtd.max_table_log as U32 + 1) {
            return err_code(ZSTD_error_tableLog_tooLarge); /* DTable too small */
        }
        dtd.table_type = 0;
        dtd.table_log = table_log as BYTE;
        core::ptr::copy_nonoverlapping(
            &dtd as *const DTableDesc as *const u8,
            dtable as *mut u8,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Compute symbols and rankStart given rankVal */
    {
        let mut n: c_int;
        let mut next_rank_start: U32 = 0;
        let unroll: c_int = 4;
        let n_limit: c_int = nb_symbols as c_int - unroll + 1;
        n = 0;
        while n < table_log as c_int + 1 {
            let curr = next_rank_start;
            next_rank_start += *(*wksp).rank_val.as_ptr().add(n as usize);
            (*wksp).rank_start[n as usize] = curr;
            n += 1;
        }
        n = 0;
        while n < n_limit {
            let mut u: c_int = 0;
            while u < unroll {
                let w = (*wksp).huff_weight[(n + u) as usize] as usize;
                let idx = (*wksp).rank_start[w];
                (*wksp).rank_start[w] = idx + 1;
                (*wksp).symbols[idx as usize] = (n + u) as BYTE;
                u += 1;
            }
            n += unroll;
        }
        while n < nb_symbols as c_int {
            let w = (*wksp).huff_weight[n as usize] as usize;
            let idx = (*wksp).rank_start[w];
            (*wksp).rank_start[w] = idx + 1;
            (*wksp).symbols[idx as usize] = n as BYTE;
            n += 1;
        }
    }

    /* fill DTable */
    {
        let mut w: U32;
        let mut symbol: c_int = (*wksp).rank_val[0] as c_int;
        let mut rank_start: c_int = 0;
        w = 1;
        while w < table_log + 1 {
            let symbol_count: c_int = (*wksp).rank_val[w as usize] as c_int;
            let length: c_int = (1 << w) >> 1;
            let mut u_start: c_int = rank_start;
            let nb_bits: BYTE = (table_log + 1 - w) as BYTE;
            let mut s: c_int;
            match length {
                1 => {
                    s = 0;
                    while s < symbol_count {
                        let d = HUF_DEltX1 {
                            byte: (*wksp).symbols[(symbol + s) as usize],
                            nb_bits,
                        };
                        *dt.offset(u_start as isize) = d;
                        u_start += 1;
                        s += 1;
                    }
                }
                2 => {
                    s = 0;
                    while s < symbol_count {
                        let d = HUF_DEltX1 {
                            byte: (*wksp).symbols[(symbol + s) as usize],
                            nb_bits,
                        };
                        *dt.offset((u_start + 0) as isize) = d;
                        *dt.offset((u_start + 1) as isize) = d;
                        u_start += 2;
                        s += 1;
                    }
                }
                4 => {
                    s = 0;
                    while s < symbol_count {
                        let d4 = huf_delt_x1_set4((*wksp).symbols[(symbol + s) as usize], nb_bits);
                        mem_write64(dt.offset(u_start as isize) as *mut u8, d4);
                        u_start += 4;
                        s += 1;
                    }
                }
                8 => {
                    s = 0;
                    while s < symbol_count {
                        let d4 = huf_delt_x1_set4((*wksp).symbols[(symbol + s) as usize], nb_bits);
                        mem_write64(dt.offset(u_start as isize) as *mut u8, d4);
                        mem_write64(dt.offset((u_start + 4) as isize) as *mut u8, d4);
                        u_start += 8;
                        s += 1;
                    }
                }
                _ => {
                    s = 0;
                    while s < symbol_count {
                        let d4 = huf_delt_x1_set4((*wksp).symbols[(symbol + s) as usize], nb_bits);
                        let mut u: c_int = 0;
                        while u < length {
                            mem_write64(dt.offset((u_start + u + 0) as isize) as *mut u8, d4);
                            mem_write64(dt.offset((u_start + u + 4) as isize) as *mut u8, d4);
                            mem_write64(dt.offset((u_start + u + 8) as isize) as *mut u8, d4);
                            mem_write64(dt.offset((u_start + u + 12) as isize) as *mut u8, d4);
                            u += 16;
                        }
                        u_start += length;
                        s += 1;
                    }
                }
            }
            symbol += symbol_count;
            rank_start += symbol_count * length;
            w += 1;
        }
    }
    i_size
}

/// `HUF_decodeSymbolX1()`
#[inline(always)]
unsafe fn huf_decode_symbol_x1(
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dt_log: U32,
) -> BYTE {
    let val = bit_look_bits_fast(&*d_stream, dt_log); /* note : dtLog >= 1 */
    let c = (*dt.add(val)).byte;
    bit_skip_bits(&mut *d_stream, (*dt.add(val)).nb_bits as u32);
    c
}

/// `HUF_DECODE_SYMBOLX1_0`
#[inline(always)]
unsafe fn huf_decode_symbolx1_0(
    p: &mut *mut BYTE,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dt_log: U32,
) {
    **p = huf_decode_symbol_x1(d_stream, dt, dt_log);
    *p = (*p).add(1);
}

/// `HUF_DECODE_SYMBOLX1_1` — MEM_64bits() is true, so always decodes.
#[inline(always)]
unsafe fn huf_decode_symbolx1_1(
    p: &mut *mut BYTE,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dt_log: U32,
) {
    if mem_64bits() || HUF_TABLELOG_MAX <= 12 {
        huf_decode_symbolx1_0(p, d_stream, dt, dt_log);
    }
}

/// `HUF_DECODE_SYMBOLX1_2` — MEM_64bits() is true, so always decodes.
#[inline(always)]
unsafe fn huf_decode_symbolx1_2(
    p: &mut *mut BYTE,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dt_log: U32,
) {
    if mem_64bits() {
        huf_decode_symbolx1_0(p, d_stream, dt, dt_log);
    }
}

/// `HUF_decodeStreamX1()`
#[inline]
unsafe fn huf_decode_stream_x1(
    mut p: *mut BYTE,
    bit_d_ptr: *mut BIT_DStream_t,
    p_end: *mut BYTE,
    dt: *const HUF_DEltX1,
    dt_log: U32,
) -> usize {
    let p_start = p;

    /* up to 4 symbols at a time */
    if (p_end as isize - p as isize) > 3 {
        while (bit_reload_dstream(&mut *bit_d_ptr) == BIT_DStream_status::unfinished)
            & (p < p_end.sub(3))
        {
            huf_decode_symbolx1_2(&mut p, bit_d_ptr, dt, dt_log);
            huf_decode_symbolx1_1(&mut p, bit_d_ptr, dt, dt_log);
            huf_decode_symbolx1_2(&mut p, bit_d_ptr, dt, dt_log);
            huf_decode_symbolx1_0(&mut p, bit_d_ptr, dt, dt_log);
        }
    } else {
        bit_reload_dstream(&mut *bit_d_ptr);
    }

    /* [0-3] symbols remaining — MEM_32bits() is false, so skipped */
    if mem_32bits() {
        while (bit_reload_dstream(&mut *bit_d_ptr) == BIT_DStream_status::unfinished) & (p < p_end) {
            huf_decode_symbolx1_0(&mut p, bit_d_ptr, dt, dt_log);
        }
    }

    /* no more data to retrieve from bitstream, no need to reload */
    while p < p_end {
        huf_decode_symbolx1_0(&mut p, bit_d_ptr, dt, dt_log);
    }

    (p_end as usize) - (p_start as usize)
}

/// `HUF_decompress1X1_usingDTable_internal_body()`
unsafe fn huf_decompress1_x1_using_dtable_internal_body(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    let op = dst as *mut BYTE;
    let oend = zstd_maybe_null_ptr_add(op, dst_size as isize);
    let dt_ptr = dtable.add(1) as *const c_void;
    let dt = dt_ptr as *const HUF_DEltX1;
    let mut bit_d = BIT_DStream_t::default();
    let dtd = huf_get_dtable_desc(dtable);
    let dt_log = dtd.table_log as U32;

    {
        let e = bit_init_dstream(&mut bit_d, c_src as *const u8, c_src_size);
        if err_is_error(e) {
            return e;
        }
    }

    huf_decode_stream_x1(op, &mut bit_d, oend, dt, dt_log);

    if !bit_end_of_dstream(&bit_d) {
        return err_code(ZSTD_error_corruption_detected);
    }

    dst_size
}

/// `HUF_decompress4X1_usingDTable_internal_body()`
unsafe fn huf_decompress4_x1_using_dtable_internal_body(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    if c_src_size < 10 {
        return err_code(ZSTD_error_corruption_detected); /* strict minimum */
    }
    if dst_size < 6 {
        return err_code(ZSTD_error_corruption_detected); /* stream 4-split doesn't work */
    }

    {
        let istart = c_src as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dst_size);
        let olimit = oend.sub(3);
        let dt_ptr = dtable.add(1) as *const c_void;
        let dt = dt_ptr as *const HUF_DEltX1;

        let mut bit_d1 = BIT_DStream_t::default();
        let mut bit_d2 = BIT_DStream_t::default();
        let mut bit_d3 = BIT_DStream_t::default();
        let mut bit_d4 = BIT_DStream_t::default();
        let length1 = mem_read_le16(istart) as usize;
        let length2 = mem_read_le16(istart.add(2)) as usize;
        let length3 = mem_read_le16(istart.add(4)) as usize;
        let length4 = c_src_size.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segment_size = (dst_size + 3) / 4;
        let op_start2 = ostart.add(segment_size);
        let op_start3 = op_start2.add(segment_size);
        let op_start4 = op_start3.add(segment_size);
        let mut op1 = ostart;
        let mut op2 = op_start2;
        let mut op3 = op_start3;
        let mut op4 = op_start4;
        let dtd = huf_get_dtable_desc(dtable);
        let dt_log = dtd.table_log as U32;
        let mut end_signal: U32 = 1;

        if length4 > c_src_size {
            return err_code(ZSTD_error_corruption_detected); /* overflow */
        }
        if op_start4 > oend {
            return err_code(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let e = bit_init_dstream(&mut bit_d1, istart1, length1);
            if err_is_error(e) {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bit_d2, istart2, length2);
            if err_is_error(e) {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bit_d3, istart3, length3);
            if err_is_error(e) {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bit_d4, istart4, length4);
            if err_is_error(e) {
                return e;
            }
        }

        /* up to 16 symbols per loop in 64-bit mode */
        if (oend as usize - op4 as usize) >= core::mem::size_of::<usize>() {
            while (end_signal != 0) & (op4 < olimit) {
                huf_decode_symbolx1_2(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx1_2(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx1_2(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx1_2(&mut op4, &mut bit_d4, dt, dt_log);
                huf_decode_symbolx1_1(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx1_1(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx1_1(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx1_1(&mut op4, &mut bit_d4, dt, dt_log);
                huf_decode_symbolx1_2(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx1_2(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx1_2(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx1_2(&mut op4, &mut bit_d4, dt, dt_log);
                huf_decode_symbolx1_0(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx1_0(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx1_0(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx1_0(&mut op4, &mut bit_d4, dt, dt_log);
                end_signal &=
                    (bit_reload_dstream_fast(&mut bit_d1) == BIT_DStream_status::unfinished) as U32;
                end_signal &=
                    (bit_reload_dstream_fast(&mut bit_d2) == BIT_DStream_status::unfinished) as U32;
                end_signal &=
                    (bit_reload_dstream_fast(&mut bit_d3) == BIT_DStream_status::unfinished) as U32;
                end_signal &=
                    (bit_reload_dstream_fast(&mut bit_d4) == BIT_DStream_status::unfinished) as U32;
            }
        }

        /* check corruption */
        if op1 > op_start2 {
            return err_code(ZSTD_error_corruption_detected);
        }
        if op2 > op_start3 {
            return err_code(ZSTD_error_corruption_detected);
        }
        if op3 > op_start4 {
            return err_code(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        huf_decode_stream_x1(op1, &mut bit_d1, op_start2, dt, dt_log);
        huf_decode_stream_x1(op2, &mut bit_d2, op_start3, dt, dt_log);
        huf_decode_stream_x1(op3, &mut bit_d3, op_start4, dt, dt_log);
        huf_decode_stream_x1(op4, &mut bit_d4, oend, dt, dt_log);

        /* check */
        {
            let end_check = (bit_end_of_dstream(&bit_d1) as U32)
                & (bit_end_of_dstream(&bit_d2) as U32)
                & (bit_end_of_dstream(&bit_d3) as U32)
                & (bit_end_of_dstream(&bit_d4) as U32);
            if end_check == 0 {
                return err_code(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
        dst_size
    }
}

/// `HUF_decompress4X1_usingDTable_internal_default()`
unsafe fn huf_decompress4_x1_using_dtable_internal_default(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    huf_decompress4_x1_using_dtable_internal_body(dst, dst_size, c_src, c_src_size, dtable)
}

/// `HUF_decompress4X1_usingDTable_internal_fast_c_loop()`
unsafe fn huf_decompress4_x1_using_dtable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable = (*args).dt as *const U16;
    let oend = (*args).oend;
    let ilowest = (*args).ilowest;

    /* Copy the arguments to local variables */
    core::ptr::copy_nonoverlapping((*args).bits.as_ptr(), bits.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).ip.as_ptr(), ip.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).op.as_ptr(), op.as_mut_ptr(), 4);

    debug_assert!(mem_is_little_endian());
    debug_assert!(!mem_32bits());

    'outer: loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            let oiters = (oend as usize - op[3] as usize) / 5;
            let iiters = (ip[0] as usize - ilowest as usize) / 7;
            let iters = min_usize(oiters, iiters);
            let symbols = iters * 5;

            olimit = op[3].add(symbols);

            /* Exit fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break 'outer;
            }

            /* Exit if any input pointer has crossed the previous one. */
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'outer; /* goto _out */
                }
                stream += 1;
            }
        }

        /* Manually unrolled loop. */
        loop {
            macro_rules! decode_symbol {
                ($stream:expr, $symbol:expr) => {{
                    let index = (bits[$stream] >> 53) as c_int;
                    let entry = *dtable.offset(index as isize) as c_int;
                    bits[$stream] <<= entry & 0x3F;
                    *op[$stream].add($symbol) = ((entry >> 8) & 0xFF) as BYTE;
                }};
            }
            macro_rules! reload_stream {
                ($stream:expr) => {{
                    let ctz = zstd_count_trailing_zeros64(bits[$stream]) as c_int;
                    let nb_bits = ctz & 7;
                    let nb_bytes = ctz >> 3;
                    op[$stream] = op[$stream].add(5);
                    ip[$stream] = ip[$stream].offset(-(nb_bytes as isize));
                    bits[$stream] = mem_read64(ip[$stream]) | 1;
                    bits[$stream] <<= nb_bits;
                }};
            }

            /* Decode 5 symbols in each of the 4 streams */
            for sym in 0..5usize {
                decode_symbol!(0, sym);
                decode_symbol!(1, sym);
                decode_symbol!(2, sym);
                decode_symbol!(3, sym);
            }

            /* Reload each of the 4 bitstreams */
            reload_stream!(0);
            reload_stream!(1);
            reload_stream!(2);
            reload_stream!(3);

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* _out: Save the final values back to args. */
    core::ptr::copy_nonoverlapping(bits.as_ptr(), (*args).bits.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping(ip.as_ptr(), (*args).ip.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping(op.as_ptr(), (*args).op.as_mut_ptr(), 4);
}

/// `HUF_decompress4X1_usingDTable_internal_fast()`
unsafe fn huf_decompress4_x1_using_dtable_internal_fast(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    loop_fn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt = dtable.add(1) as *const c_void;
    let _ilowest = c_src as *const BYTE;
    let oend = zstd_maybe_null_ptr_add(dst as *mut BYTE, dst_size as isize);
    let mut args: HUF_DecompressFastArgs = core::mem::zeroed();
    {
        let ret =
            huf_decompress_fast_args_init(&mut args, dst, dst_size, c_src, c_src_size, dtable);
        if err_is_error(ret) {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    debug_assert!(args.ip[0] >= args.ilowest);
    loop_fn(&mut args);

    /* finish bit streams one by one. */
    {
        let segment_size = (dst_size + 3) / 4;
        let mut segment_end = dst as *mut BYTE;
        let mut i: c_int = 0;
        while i < 4 {
            let mut bit = BIT_DStream_t::default();
            if segment_size <= (oend as usize - segment_end as usize) {
                segment_end = segment_end.add(segment_size);
            } else {
                segment_end = oend;
            }
            let e = huf_init_remaining_dstream(&mut bit, &args, i, segment_end);
            if err_is_error(e) {
                return e;
            }
            args.op[i as usize] = args.op[i as usize].add(huf_decode_stream_x1(
                args.op[i as usize],
                &mut bit,
                segment_end,
                dt as *const HUF_DEltX1,
                HUF_DECODER_FAST_TABLELOG,
            ));
            if args.op[i as usize] != segment_end {
                return err_code(ZSTD_error_corruption_detected);
            }
            i += 1;
        }
    }

    dst_size
}

/// `HUF_decompress1X1_usingDTable_internal()` — HUF_DGEN, flags ignored.
unsafe fn huf_decompress1_x1_using_dtable_internal(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    _flags: c_int,
) -> usize {
    huf_decompress1_x1_using_dtable_internal_body(dst, dst_size, c_src, c_src_size, dtable)
}

/// `HUF_decompress4X1_usingDTable_internal()`
unsafe fn huf_decompress4_x1_using_dtable_internal(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let fallback_fn = huf_decompress4_x1_using_dtable_internal_default;
    let loop_fn: HUF_DecompressFastLoopFn = huf_decompress4_x1_using_dtable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ASM disabled: no early branches. */

    if HUF_ENABLE_FAST_DECODE && (flags & HUF_flags_disableFast) == 0 {
        let ret = huf_decompress4_x1_using_dtable_internal_fast(
            dst, dst_size, c_src, c_src_size, dtable, loop_fn,
        );
        if ret != 0 {
            return ret;
        }
    }
    fallback_fn(dst, dst_size, c_src, c_src_size, dtable)
}

/// `HUF_decompress4X1_DCtx_wksp()`
unsafe fn huf_decompress4_x1_dctx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    mut c_src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    let mut ip = c_src as *const BYTE;

    let h_size = HUF_readDTableX1_wksp(dctx, c_src, c_src_size, work_space, wksp_size, flags);
    if err_is_error(h_size) {
        return h_size;
    }
    if h_size >= c_src_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(h_size);
    c_src_size -= h_size;

    huf_decompress4_x1_using_dtable_internal(dst, dst_size, ip as *const c_void, c_src_size, dctx, flags)
}

/* *************************/
/* double-symbols decoding */
/* *************************/

/// `HUF_DEltX2`
#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX2 {
    sequence: U16,
    nb_bits: BYTE,
    length: BYTE,
}

/// `sortedSymbol_t`
#[repr(C)]
#[derive(Clone, Copy)]
struct sortedSymbol_t {
    symbol: BYTE,
}

/// `rankValCol_t`
type rankValCol_t = [U32; (HUF_TABLELOG_MAX + 1) as usize];

/// `HUF_buildDEltX2U32()`
#[inline]
fn huf_build_delt_x2_u32(symbol: U32, nb_bits: U32, base_seq: U32, level: c_int) -> U32 {
    let seq: U32;
    if mem_is_little_endian() {
        seq = if level == 1 {
            symbol
        } else {
            base_seq.wrapping_add(symbol << 8)
        };
        seq.wrapping_add(nb_bits << 16)
            .wrapping_add((level as U32) << 24)
    } else {
        seq = if level == 1 {
            symbol << 8
        } else {
            (base_seq << 8).wrapping_add(symbol)
        };
        (seq << 16)
            .wrapping_add(nb_bits << 8)
            .wrapping_add(level as U32)
    }
}

/// `HUF_buildDEltX2()`
#[inline]
unsafe fn huf_build_delt_x2(symbol: U32, nb_bits: U32, base_seq: U32, level: c_int) -> HUF_DEltX2 {
    let val = huf_build_delt_x2_u32(symbol, nb_bits, base_seq, level);
    let mut d_elt = HUF_DEltX2 {
        sequence: 0,
        nb_bits: 0,
        length: 0,
    };
    core::ptr::copy_nonoverlapping(
        &val as *const U32 as *const u8,
        &mut d_elt as *mut HUF_DEltX2 as *mut u8,
        core::mem::size_of::<U32>(),
    );
    d_elt
}

/// `HUF_buildDEltX2U64()`
#[inline]
fn huf_build_delt_x2_u64(symbol: U32, nb_bits: U32, base_seq: U16, level: c_int) -> U64 {
    let d_elt = huf_build_delt_x2_u32(symbol, nb_bits, base_seq as U32, level);
    (d_elt as U64).wrapping_add((d_elt as U64) << 32)
}

/// `HUF_fillDTableX2ForWeight()`
unsafe fn huf_fill_dtable_x2_for_weight(
    mut d_table_rank: *mut HUF_DEltX2,
    begin: *const sortedSymbol_t,
    end: *const sortedSymbol_t,
    nb_bits: U32,
    table_log: U32,
    base_seq: U16,
    level: c_int,
) {
    let length = 1u32 << ((table_log - nb_bits) & 0x1F);
    let mut ptr: *const sortedSymbol_t;
    debug_assert!(level >= 1 && level <= 2);
    match length {
        1 => {
            ptr = begin;
            while ptr != end {
                let d_elt = huf_build_delt_x2((*ptr).symbol as U32, nb_bits, base_seq as U32, level);
                *d_table_rank = d_elt;
                d_table_rank = d_table_rank.add(1);
                ptr = ptr.add(1);
            }
        }
        2 => {
            ptr = begin;
            while ptr != end {
                let d_elt = huf_build_delt_x2((*ptr).symbol as U32, nb_bits, base_seq as U32, level);
                *d_table_rank.add(0) = d_elt;
                *d_table_rank.add(1) = d_elt;
                d_table_rank = d_table_rank.add(2);
                ptr = ptr.add(1);
            }
        }
        4 => {
            ptr = begin;
            while ptr != end {
                let d_elt_x2 = huf_build_delt_x2_u64((*ptr).symbol as U32, nb_bits, base_seq, level);
                copy_u64_to_delt2(d_table_rank.add(0), d_elt_x2);
                copy_u64_to_delt2(d_table_rank.add(2), d_elt_x2);
                d_table_rank = d_table_rank.add(4);
                ptr = ptr.add(1);
            }
        }
        8 => {
            ptr = begin;
            while ptr != end {
                let d_elt_x2 = huf_build_delt_x2_u64((*ptr).symbol as U32, nb_bits, base_seq, level);
                copy_u64_to_delt2(d_table_rank.add(0), d_elt_x2);
                copy_u64_to_delt2(d_table_rank.add(2), d_elt_x2);
                copy_u64_to_delt2(d_table_rank.add(4), d_elt_x2);
                copy_u64_to_delt2(d_table_rank.add(6), d_elt_x2);
                d_table_rank = d_table_rank.add(8);
                ptr = ptr.add(1);
            }
        }
        _ => {
            ptr = begin;
            while ptr != end {
                let d_elt_x2 = huf_build_delt_x2_u64((*ptr).symbol as U32, nb_bits, base_seq, level);
                let d_table_rank_end = d_table_rank.add(length as usize);
                while d_table_rank != d_table_rank_end {
                    copy_u64_to_delt2(d_table_rank.add(0), d_elt_x2);
                    copy_u64_to_delt2(d_table_rank.add(2), d_elt_x2);
                    copy_u64_to_delt2(d_table_rank.add(4), d_elt_x2);
                    copy_u64_to_delt2(d_table_rank.add(6), d_elt_x2);
                    d_table_rank = d_table_rank.add(8);
                }
                ptr = ptr.add(1);
            }
        }
    }
}

/// Helper reproducing `ZSTD_memcpy(dst, &DEltX2, sizeof(DEltX2))` where
/// DEltX2 is a U64 written over two HUF_DEltX2 entries.
#[inline(always)]
unsafe fn copy_u64_to_delt2(dst: *mut HUF_DEltX2, val: U64) {
    core::ptr::copy_nonoverlapping(
        &val as *const U64 as *const u8,
        dst as *mut u8,
        core::mem::size_of::<U64>(),
    );
}

/// `HUF_fillDTableX2Level2()`
unsafe fn huf_fill_dtable_x2_level2(
    d_table: *mut HUF_DEltX2,
    target_log: U32,
    consumed_bits: U32,
    rank_val: *const U32,
    min_weight: c_int,
    max_weight1: c_int,
    sorted_symbols: *const sortedSymbol_t,
    rank_start: *const U32,
    nb_bits_baseline: U32,
    base_seq: U16,
) {
    /* Fill skipped values. */
    if min_weight > 1 {
        let length = 1u32 << ((target_log - consumed_bits) & 0x1F);
        let d_elt_x2 = huf_build_delt_x2_u64(base_seq as U32, consumed_bits, 0, 1);
        let skip_size = *rank_val.add(min_weight as usize) as c_int;
        debug_assert!(length > 1);
        debug_assert!((skip_size as u32) < length);
        match length {
            2 => {
                debug_assert!(skip_size == 1);
                copy_u64_to_delt2(d_table, d_elt_x2);
            }
            4 => {
                debug_assert!(skip_size <= 4);
                copy_u64_to_delt2(d_table.add(0), d_elt_x2);
                copy_u64_to_delt2(d_table.add(2), d_elt_x2);
            }
            _ => {
                let mut i: c_int = 0;
                while i < skip_size {
                    copy_u64_to_delt2(d_table.offset((i + 0) as isize), d_elt_x2);
                    copy_u64_to_delt2(d_table.offset((i + 2) as isize), d_elt_x2);
                    copy_u64_to_delt2(d_table.offset((i + 4) as isize), d_elt_x2);
                    copy_u64_to_delt2(d_table.offset((i + 6) as isize), d_elt_x2);
                    i += 8;
                }
            }
        }
    }

    /* Fill each of the second level symbols by weight. */
    {
        let mut w: c_int = min_weight;
        while w < max_weight1 {
            let begin = *rank_start.add(w as usize) as c_int;
            let end = *rank_start.add((w + 1) as usize) as c_int;
            let nb_bits = nb_bits_baseline - w as U32;
            let total_bits = nb_bits + consumed_bits;
            huf_fill_dtable_x2_for_weight(
                d_table.offset(*rank_val.add(w as usize) as isize),
                sorted_symbols.offset(begin as isize),
                sorted_symbols.offset(end as isize),
                total_bits,
                target_log,
                base_seq,
                2,
            );
            w += 1;
        }
    }
}

/// `HUF_fillDTableX2()`
unsafe fn huf_fill_dtable_x2(
    d_table: *mut HUF_DEltX2,
    target_log: U32,
    sorted_list: *const sortedSymbol_t,
    rank_start: *const U32,
    rank_val_origin: *mut rankValCol_t,
    max_weight: U32,
    nb_bits_baseline: U32,
) {
    let rank_val = (*rank_val_origin.add(0)).as_ptr();
    let scale_log = nb_bits_baseline as c_int - target_log as c_int;
    let min_bits = nb_bits_baseline - max_weight;
    let mut w: c_int;
    let w_end = max_weight as c_int + 1;

    w = 1;
    while w < w_end {
        let begin = *rank_start.add(w as usize) as c_int;
        let end = *rank_start.add((w + 1) as usize) as c_int;
        let nb_bits = nb_bits_baseline - w as U32;

        if (target_log as c_int - nb_bits as c_int) >= min_bits as c_int {
            /* Enough room for a second symbol. */
            let mut start = *rank_val.add(w as usize) as c_int;
            let length = 1u32 << ((target_log - nb_bits) & 0x1F);
            let mut min_weight = nb_bits as c_int + scale_log;
            let mut s: c_int;
            if min_weight < 1 {
                min_weight = 1;
            }
            s = begin;
            while s != end {
                huf_fill_dtable_x2_level2(
                    d_table.offset(start as isize),
                    target_log,
                    nb_bits,
                    (*rank_val_origin.add(nb_bits as usize)).as_ptr(),
                    min_weight,
                    w_end,
                    sorted_list,
                    rank_start,
                    nb_bits_baseline,
                    (*sorted_list.offset(s as isize)).symbol as U16,
                );
                start += length as c_int;
                s += 1;
            }
        } else {
            /* Only a single symbol. */
            huf_fill_dtable_x2_for_weight(
                d_table.offset(*rank_val.add(w as usize) as isize),
                sorted_list.offset(begin as isize),
                sorted_list.offset(end as isize),
                nb_bits,
                target_log,
                0,
                1,
            );
        }
        w += 1;
    }
}

/// `HUF_ReadDTableX2_Workspace`
#[repr(C)]
struct HUF_ReadDTableX2_Workspace {
    rank_val: [rankValCol_t; HUF_TABLELOG_MAX as usize],
    rank_stats: [U32; (HUF_TABLELOG_MAX + 1) as usize],
    rank_start0: [U32; (HUF_TABLELOG_MAX + 3) as usize],
    sorted_symbol: [sortedSymbol_t; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    weight_list: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    callee_wksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
}

/// `HUF_readDTableX2_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX2_wksp(
    dtable: *mut HUF_DTable,
    src: *const c_void,
    src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    let mut table_log: U32 = 0;
    let mut max_w: U32;
    let mut nb_symbols: U32 = 0;
    let mut dtd = huf_get_dtable_desc(dtable);
    let mut max_table_log = dtd.max_table_log as U32;
    let i_size: usize;
    let dt_ptr = dtable.add(1) as *mut c_void;
    let dt = dt_ptr as *mut HUF_DEltX2;
    let rank_start: *mut U32;

    let wksp = work_space as *mut HUF_ReadDTableX2_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX2_Workspace>() > wksp_size {
        return err_code(ZSTD_error_GENERIC);
    }

    rank_start = (*wksp).rank_start0.as_mut_ptr().add(1);
    core::ptr::write_bytes(
        (*wksp).rank_stats.as_mut_ptr(),
        0,
        (HUF_TABLELOG_MAX + 1) as usize,
    );
    core::ptr::write_bytes(
        (*wksp).rank_start0.as_mut_ptr(),
        0,
        (HUF_TABLELOG_MAX + 3) as usize,
    );

    if max_table_log > HUF_TABLELOG_MAX {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }

    i_size = crate::entropy_common::HUF_readStats_wksp(
        (*wksp).weight_list.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as usize,
        (*wksp).rank_stats.as_mut_ptr(),
        &mut nb_symbols,
        &mut table_log,
        src,
        src_size,
        (*wksp).callee_wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).callee_wksp),
        flags,
    );
    if err_is_error(i_size) {
        return i_size;
    }

    /* check result */
    if table_log > max_table_log {
        return err_code(ZSTD_error_tableLog_tooLarge); /* DTable can't fit code depth */
    }
    if table_log <= HUF_DECODER_FAST_TABLELOG && max_table_log > HUF_DECODER_FAST_TABLELOG {
        max_table_log = HUF_DECODER_FAST_TABLELOG;
    }

    /* find maxWeight */
    max_w = table_log;
    while *(*wksp).rank_stats.as_ptr().add(max_w as usize) == 0 {
        max_w -= 1;
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut next_rank_start: U32 = 0;
        w = 1;
        while w < max_w + 1 {
            let curr = next_rank_start;
            next_rank_start += (*wksp).rank_stats[w as usize];
            *rank_start.add(w as usize) = curr;
            w += 1;
        }
        *rank_start.add(0) = next_rank_start; /* put all 0w symbols at the end */
        *rank_start.add((max_w + 1) as usize) = next_rank_start;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nb_symbols {
            let w = (*wksp).weight_list[s as usize] as U32;
            let r = *rank_start.add(w as usize);
            *rank_start.add(w as usize) = r + 1;
            (*wksp).sorted_symbol[r as usize].symbol = s as BYTE;
            s += 1;
        }
        *rank_start.add(0) = 0; /* forget 0w symbols; beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let rank_val0 = (*wksp).rank_val[0].as_mut_ptr();
        {
            let rescale = (max_table_log as c_int - table_log as c_int) - 1;
            let mut next_rank_val: U32 = 0;
            let mut w: U32 = 1;
            while w < max_w + 1 {
                let curr = next_rank_val;
                next_rank_val += (*wksp).rank_stats[w as usize] << (w as c_int + rescale);
                *rank_val0.add(w as usize) = curr;
                w += 1;
            }
        }
        {
            let min_bits = table_log + 1 - max_w;
            let mut consumed: U32 = min_bits;
            while consumed < max_table_log - min_bits + 1 {
                let rank_val_ptr = (*wksp).rank_val[consumed as usize].as_mut_ptr();
                let mut w: U32 = 1;
                while w < max_w + 1 {
                    *rank_val_ptr.add(w as usize) = *rank_val0.add(w as usize) >> consumed;
                    w += 1;
                }
                consumed += 1;
            }
        }
    }

    huf_fill_dtable_x2(
        dt,
        max_table_log,
        (*wksp).sorted_symbol.as_ptr(),
        (*wksp).rank_start0.as_ptr(),
        (*wksp).rank_val.as_mut_ptr(),
        max_w,
        table_log + 1,
    );

    dtd.table_log = max_table_log as BYTE;
    dtd.table_type = 1;
    core::ptr::copy_nonoverlapping(
        &dtd as *const DTableDesc as *const u8,
        dtable as *mut u8,
        core::mem::size_of::<DTableDesc>(),
    );
    i_size
}

/// `HUF_decodeSymbolX2()`
#[inline(always)]
unsafe fn huf_decode_symbol_x2(
    op: *mut c_void,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dt_log: U32,
) -> U32 {
    let val = bit_look_bits_fast(&*d_stream, dt_log); /* note : dtLog >= 1 */
    core::ptr::copy_nonoverlapping(
        &(*dt.add(val)).sequence as *const U16 as *const u8,
        op as *mut u8,
        2,
    );
    bit_skip_bits(&mut *d_stream, (*dt.add(val)).nb_bits as u32);
    (*dt.add(val)).length as U32
}

/// `HUF_decodeLastSymbolX2()`
#[inline(always)]
unsafe fn huf_decode_last_symbol_x2(
    op: *mut c_void,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dt_log: U32,
) -> U32 {
    let val = bit_look_bits_fast(&*d_stream, dt_log); /* note : dtLog >= 1 */
    core::ptr::copy_nonoverlapping(
        &(*dt.add(val)).sequence as *const U16 as *const u8,
        op as *mut u8,
        1,
    );
    if (*dt.add(val)).length == 1 {
        bit_skip_bits(&mut *d_stream, (*dt.add(val)).nb_bits as u32);
    } else {
        let container_bits = (core::mem::size_of::<BitContainerType>() * 8) as u32;
        if (*d_stream).bitsConsumed < container_bits {
            bit_skip_bits(&mut *d_stream, (*dt.add(val)).nb_bits as u32);
            if (*d_stream).bitsConsumed > container_bits {
                /* ugly hack; works only because it's the last symbol. */
                (*d_stream).bitsConsumed = container_bits;
            }
        }
    }
    1
}

/// `HUF_DECODE_SYMBOLX2_0`
#[inline(always)]
unsafe fn huf_decode_symbolx2_0(
    ptr: &mut *mut BYTE,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dt_log: U32,
) {
    let adv = huf_decode_symbol_x2(*ptr as *mut c_void, d_stream, dt, dt_log);
    *ptr = (*ptr).add(adv as usize);
}

/// `HUF_DECODE_SYMBOLX2_1` — MEM_64bits() is true.
#[inline(always)]
unsafe fn huf_decode_symbolx2_1(
    ptr: &mut *mut BYTE,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dt_log: U32,
) {
    if mem_64bits() || HUF_TABLELOG_MAX <= 12 {
        let adv = huf_decode_symbol_x2(*ptr as *mut c_void, d_stream, dt, dt_log);
        *ptr = (*ptr).add(adv as usize);
    }
}

/// `HUF_DECODE_SYMBOLX2_2` — MEM_64bits() is true.
#[inline(always)]
unsafe fn huf_decode_symbolx2_2(
    ptr: &mut *mut BYTE,
    d_stream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dt_log: U32,
) {
    if mem_64bits() {
        let adv = huf_decode_symbol_x2(*ptr as *mut c_void, d_stream, dt, dt_log);
        *ptr = (*ptr).add(adv as usize);
    }
}

/// `HUF_decodeStreamX2()`
#[inline]
unsafe fn huf_decode_stream_x2(
    mut p: *mut BYTE,
    bit_d_ptr: *mut BIT_DStream_t,
    p_end: *mut BYTE,
    dt: *const HUF_DEltX2,
    dt_log: U32,
) -> usize {
    let p_start = p;
    let container_bytes = core::mem::size_of::<BitContainerType>();

    /* up to 8 symbols at a time */
    if (p_end as usize - p as usize) >= container_bytes {
        if dt_log <= 11 && mem_64bits() {
            /* up to 10 symbols at a time */
            while (bit_reload_dstream(&mut *bit_d_ptr) == BIT_DStream_status::unfinished)
                & (p < p_end.sub(9))
            {
                huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
            }
        } else {
            /* up to 8 symbols at a time */
            while (bit_reload_dstream(&mut *bit_d_ptr) == BIT_DStream_status::unfinished)
                & (p < p_end.sub(container_bytes - 1))
            {
                huf_decode_symbolx2_2(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_1(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_2(&mut p, bit_d_ptr, dt, dt_log);
                huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
            }
        }
    } else {
        bit_reload_dstream(&mut *bit_d_ptr);
    }

    /* closer to end : up to 2 symbols at a time */
    if (p_end as usize - p as usize) >= 2 {
        while (bit_reload_dstream(&mut *bit_d_ptr) == BIT_DStream_status::unfinished)
            & (p <= p_end.sub(2))
        {
            huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
        }

        while p <= p_end.sub(2) {
            huf_decode_symbolx2_0(&mut p, bit_d_ptr, dt, dt_log);
        }
    }

    if p < p_end {
        p = p.add(huf_decode_last_symbol_x2(p as *mut c_void, bit_d_ptr, dt, dt_log) as usize);
    }

    (p as usize) - (p_start as usize)
}

/// `HUF_decompress1X2_usingDTable_internal_body()`
unsafe fn huf_decompress1_x2_using_dtable_internal_body(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    let mut bit_d = BIT_DStream_t::default();

    {
        let e = bit_init_dstream(&mut bit_d, c_src as *const u8, c_src_size);
        if err_is_error(e) {
            return e;
        }
    }

    {
        let ostart = dst as *mut BYTE;
        let oend = zstd_maybe_null_ptr_add(ostart, dst_size as isize);
        let dt_ptr = dtable.add(1) as *const c_void;
        let dt = dt_ptr as *const HUF_DEltX2;
        let dtd = huf_get_dtable_desc(dtable);
        huf_decode_stream_x2(ostart, &mut bit_d, oend, dt, dtd.table_log as U32);
    }

    if !bit_end_of_dstream(&bit_d) {
        return err_code(ZSTD_error_corruption_detected);
    }

    dst_size
}

/// `HUF_decompress4X2_usingDTable_internal_body()`
unsafe fn huf_decompress4_x2_using_dtable_internal_body(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    if c_src_size < 10 {
        return err_code(ZSTD_error_corruption_detected); /* strict minimum */
    }
    if dst_size < 6 {
        return err_code(ZSTD_error_corruption_detected); /* stream 4-split doesn't work */
    }

    {
        let istart = c_src as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.add(dst_size);
        let olimit = oend.sub(core::mem::size_of::<usize>() - 1);
        let dt_ptr = dtable.add(1) as *const c_void;
        let dt = dt_ptr as *const HUF_DEltX2;

        let mut bit_d1 = BIT_DStream_t::default();
        let mut bit_d2 = BIT_DStream_t::default();
        let mut bit_d3 = BIT_DStream_t::default();
        let mut bit_d4 = BIT_DStream_t::default();
        let length1 = mem_read_le16(istart) as usize;
        let length2 = mem_read_le16(istart.add(2)) as usize;
        let length3 = mem_read_le16(istart.add(4)) as usize;
        let length4 = c_src_size.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segment_size = (dst_size + 3) / 4;
        let op_start2 = ostart.add(segment_size);
        let op_start3 = op_start2.add(segment_size);
        let op_start4 = op_start3.add(segment_size);
        let mut op1 = ostart;
        let mut op2 = op_start2;
        let mut op3 = op_start3;
        let mut op4 = op_start4;
        let mut end_signal: U32 = 1;
        let dtd = huf_get_dtable_desc(dtable);
        let dt_log = dtd.table_log as U32;

        if length4 > c_src_size {
            return err_code(ZSTD_error_corruption_detected); /* overflow */
        }
        if op_start4 > oend {
            return err_code(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let e = bit_init_dstream(&mut bit_d1, istart1, length1);
            if err_is_error(e) {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bit_d2, istart2, length2);
            if err_is_error(e) {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bit_d3, istart3, length3);
            if err_is_error(e) {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bit_d4, istart4, length4);
            if err_is_error(e) {
                return e;
            }
        }

        /* 16-32 symbols per loop (non-clang path) */
        if (oend as usize - op4 as usize) >= core::mem::size_of::<usize>() {
            while (end_signal != 0) & (op4 < olimit) {
                huf_decode_symbolx2_2(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx2_2(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx2_2(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx2_2(&mut op4, &mut bit_d4, dt, dt_log);
                huf_decode_symbolx2_1(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx2_1(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx2_1(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx2_1(&mut op4, &mut bit_d4, dt, dt_log);
                huf_decode_symbolx2_2(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx2_2(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx2_2(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx2_2(&mut op4, &mut bit_d4, dt, dt_log);
                huf_decode_symbolx2_0(&mut op1, &mut bit_d1, dt, dt_log);
                huf_decode_symbolx2_0(&mut op2, &mut bit_d2, dt, dt_log);
                huf_decode_symbolx2_0(&mut op3, &mut bit_d3, dt, dt_log);
                huf_decode_symbolx2_0(&mut op4, &mut bit_d4, dt, dt_log);
                end_signal = ((bit_reload_dstream_fast(&mut bit_d1)
                    == BIT_DStream_status::unfinished) as U32
                    & (bit_reload_dstream_fast(&mut bit_d2) == BIT_DStream_status::unfinished)
                        as U32
                    & (bit_reload_dstream_fast(&mut bit_d3) == BIT_DStream_status::unfinished)
                        as U32
                    & (bit_reload_dstream_fast(&mut bit_d4) == BIT_DStream_status::unfinished)
                        as U32) as U32;
            }
        }

        /* check corruption */
        if op1 > op_start2 {
            return err_code(ZSTD_error_corruption_detected);
        }
        if op2 > op_start3 {
            return err_code(ZSTD_error_corruption_detected);
        }
        if op3 > op_start4 {
            return err_code(ZSTD_error_corruption_detected);
        }
        /* note : op4 already verified within main loop */

        /* finish bitStreams one by one */
        huf_decode_stream_x2(op1, &mut bit_d1, op_start2, dt, dt_log);
        huf_decode_stream_x2(op2, &mut bit_d2, op_start3, dt, dt_log);
        huf_decode_stream_x2(op3, &mut bit_d3, op_start4, dt, dt_log);
        huf_decode_stream_x2(op4, &mut bit_d4, oend, dt, dt_log);

        /* check */
        {
            let end_check = (bit_end_of_dstream(&bit_d1) as U32)
                & (bit_end_of_dstream(&bit_d2) as U32)
                & (bit_end_of_dstream(&bit_d3) as U32)
                & (bit_end_of_dstream(&bit_d4) as U32);
            if end_check == 0 {
                return err_code(ZSTD_error_corruption_detected);
            }
        }

        dst_size
    }
}

/// `HUF_decompress4X2_usingDTable_internal_default()`
unsafe fn huf_decompress4_x2_using_dtable_internal_default(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
) -> usize {
    huf_decompress4_x2_using_dtable_internal_body(dst, dst_size, c_src, c_src_size, dtable)
}

/// `HUF_decompress4X2_usingDTable_internal_fast_c_loop()`
unsafe fn huf_decompress4_x2_using_dtable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let mut oend: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable = (*args).dt as *const HUF_DEltX2;
    let ilowest = (*args).ilowest;

    /* Copy the arguments to local registers. */
    core::ptr::copy_nonoverlapping((*args).bits.as_ptr(), bits.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).ip.as_ptr(), ip.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).op.as_ptr(), op.as_mut_ptr(), 4);

    oend[0] = op[1];
    oend[1] = op[2];
    oend[2] = op[3];
    oend[3] = (*args).oend;

    debug_assert!(mem_is_little_endian());
    debug_assert!(!mem_32bits());

    'outer: loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            let mut iters = (ip[0] as usize - ilowest as usize) / 7;
            stream = 0;
            while stream < 4 {
                let oiters = (oend[stream as usize] as usize - op[stream as usize] as usize) / 10;
                iters = min_usize(iters, oiters);
                stream += 1;
            }

            olimit = op[3].add(iters * 5);

            /* Exit the fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break 'outer;
            }

            /* Exit if any input pointer has crossed the previous one. */
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'outer; /* goto _out */
                }
                stream += 1;
            }
        }

        macro_rules! decode_symbol {
            ($stream:expr, $decode3:expr) => {{
                if $decode3 || $stream != 3 {
                    let index = (bits[$stream] >> 53) as c_int;
                    let entry = *dtable.offset(index as isize);
                    mem_write16(op[$stream], entry.sequence);
                    bits[$stream] <<= (entry.nb_bits as c_int) & 0x3F;
                    op[$stream] = op[$stream].add(entry.length as usize);
                }
            }};
        }
        macro_rules! reload_stream {
            ($stream:expr) => {{
                decode_symbol!(3, true);
                {
                    let ctz = zstd_count_trailing_zeros64(bits[$stream]) as c_int;
                    let nb_bits = ctz & 7;
                    let nb_bytes = ctz >> 3;
                    ip[$stream] = ip[$stream].offset(-(nb_bytes as isize));
                    bits[$stream] = mem_read64(ip[$stream]) | 1;
                    bits[$stream] <<= nb_bits;
                }
            }};
        }

        /* Manually unrolled loop. */
        loop {
            /* Decode 5 symbols from each of the first 3 streams. */
            for _ in 0..5 {
                decode_symbol!(0, false);
                decode_symbol!(1, false);
                decode_symbol!(2, false);
                decode_symbol!(3, false);
            }

            /* Decode one symbol from the final stream */
            decode_symbol!(3, true);

            /* Decode 4 symbols from the final stream & reload bitstreams. */
            reload_stream!(0);
            reload_stream!(1);
            reload_stream!(2);
            reload_stream!(3);

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* _out: Save the final values back to args. */
    core::ptr::copy_nonoverlapping(bits.as_ptr(), (*args).bits.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping(ip.as_ptr(), (*args).ip.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping(op.as_ptr(), (*args).op.as_mut_ptr(), 4);
}

/// `HUF_decompress4X2_usingDTable_internal_fast()`
unsafe fn huf_decompress4_x2_using_dtable_internal_fast(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    loop_fn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt = dtable.add(1) as *const c_void;
    let _ilowest = c_src as *const BYTE;
    let oend = zstd_maybe_null_ptr_add(dst as *mut BYTE, dst_size as isize);
    let mut args: HUF_DecompressFastArgs = core::mem::zeroed();
    {
        let ret =
            huf_decompress_fast_args_init(&mut args, dst, dst_size, c_src, c_src_size, dtable);
        if err_is_error(ret) {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    debug_assert!(args.ip[0] >= args.ilowest);
    loop_fn(&mut args);

    /* finish bitStreams one by one */
    {
        let segment_size = (dst_size + 3) / 4;
        let mut segment_end = dst as *mut BYTE;
        let mut i: c_int = 0;
        while i < 4 {
            let mut bit = BIT_DStream_t::default();
            if segment_size <= (oend as usize - segment_end as usize) {
                segment_end = segment_end.add(segment_size);
            } else {
                segment_end = oend;
            }
            let e = huf_init_remaining_dstream(&mut bit, &args, i, segment_end);
            if err_is_error(e) {
                return e;
            }
            args.op[i as usize] = args.op[i as usize].add(huf_decode_stream_x2(
                args.op[i as usize],
                &mut bit,
                segment_end,
                dt as *const HUF_DEltX2,
                HUF_DECODER_FAST_TABLELOG,
            ));
            if args.op[i as usize] != segment_end {
                return err_code(ZSTD_error_corruption_detected);
            }
            i += 1;
        }
    }

    dst_size
}

/// `HUF_decompress4X2_usingDTable_internal()`
unsafe fn huf_decompress4_x2_using_dtable_internal(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let fallback_fn = huf_decompress4_x2_using_dtable_internal_default;
    let loop_fn: HUF_DecompressFastLoopFn = huf_decompress4_x2_using_dtable_internal_fast_c_loop;

    if HUF_ENABLE_FAST_DECODE && (flags & HUF_flags_disableFast) == 0 {
        let ret = huf_decompress4_x2_using_dtable_internal_fast(
            dst, dst_size, c_src, c_src_size, dtable, loop_fn,
        );
        if ret != 0 {
            return ret;
        }
    }
    fallback_fn(dst, dst_size, c_src, c_src_size, dtable)
}

/// `HUF_decompress1X2_usingDTable_internal()` — HUF_DGEN, flags ignored.
unsafe fn huf_decompress1_x2_using_dtable_internal(
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    _flags: c_int,
) -> usize {
    huf_decompress1_x2_using_dtable_internal_body(dst, dst_size, c_src, c_src_size, dtable)
}

/// `HUF_decompress1X2_DCtx_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X2_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    mut c_src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    let mut ip = c_src as *const BYTE;

    let h_size = HUF_readDTableX2_wksp(dctx, c_src, c_src_size, work_space, wksp_size, flags);
    if err_is_error(h_size) {
        return h_size;
    }
    if h_size >= c_src_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(h_size);
    c_src_size -= h_size;

    huf_decompress1_x2_using_dtable_internal(dst, dst_size, ip as *const c_void, c_src_size, dctx, flags)
}

/// `HUF_decompress4X2_DCtx_wksp()`
unsafe fn huf_decompress4_x2_dctx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    mut c_src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    let mut ip = c_src as *const BYTE;

    let h_size = HUF_readDTableX2_wksp(dctx, c_src, c_src_size, work_space, wksp_size, flags);
    if err_is_error(h_size) {
        return h_size;
    }
    if h_size >= c_src_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(h_size);
    c_src_size -= h_size;

    huf_decompress4_x2_using_dtable_internal(dst, dst_size, ip as *const c_void, c_src_size, dctx, flags)
}

/* ***********************************/
/* Universal decompression selectors */
/* ***********************************/

/// `algo_time_t`
#[derive(Clone, Copy)]
struct algo_time_t {
    table_time: U32,
    decode256_time: U32,
}

const fn at(t: U32, d: U32) -> algo_time_t {
    algo_time_t {
        table_time: t,
        decode256_time: d,
    }
}

/// `algoTime[16][2]`
static ALGO_TIME: [[algo_time_t; 2]; 16] = [
    [at(0, 0), at(1, 1)],
    [at(0, 0), at(1, 1)],
    [at(150, 216), at(381, 119)],
    [at(170, 205), at(514, 112)],
    [at(177, 199), at(539, 110)],
    [at(197, 194), at(644, 107)],
    [at(221, 192), at(735, 107)],
    [at(256, 189), at(881, 106)],
    [at(359, 188), at(1167, 109)],
    [at(582, 187), at(1570, 114)],
    [at(688, 187), at(1712, 122)],
    [at(825, 186), at(1965, 136)],
    [at(976, 185), at(2131, 150)],
    [at(1180, 186), at(2070, 175)],
    [at(1377, 185), at(1731, 202)],
    [at(1412, 185), at(1695, 202)],
];

/// `HUF_selectDecoder()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_selectDecoder(dst_size: usize, c_src_size: usize) -> U32 {
    debug_assert!(dst_size > 0);
    debug_assert!(dst_size <= 128 * 1024);

    let q: U32 = if c_src_size >= dst_size {
        15
    } else {
        (c_src_size * 16 / dst_size) as U32
    };
    let d256: U32 = (dst_size >> 8) as U32;
    let d_time0: U32 = ALGO_TIME[q as usize][0]
        .table_time
        .wrapping_add(ALGO_TIME[q as usize][0].decode256_time.wrapping_mul(d256));
    let mut d_time1: U32 = ALGO_TIME[q as usize][1]
        .table_time
        .wrapping_add(ALGO_TIME[q as usize][1].decode256_time.wrapping_mul(d256));
    d_time1 = d_time1.wrapping_add(d_time1 >> 5);
    (d_time1 < d_time0) as U32
}

/// `HUF_decompress1X_DCtx_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    /* validation checks */
    if dst_size == 0 {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    if c_src_size > dst_size {
        return err_code(ZSTD_error_corruption_detected); /* invalid */
    }
    if c_src_size == dst_size {
        core::ptr::copy_nonoverlapping(c_src as *const u8, dst as *mut u8, dst_size);
        return dst_size; /* not compressed */
    }
    if c_src_size == 1 {
        core::ptr::write_bytes(dst as *mut u8, *(c_src as *const BYTE), dst_size);
        return dst_size; /* RLE */
    }

    {
        let algo_nb = HUF_selectDecoder(dst_size, c_src_size);
        if algo_nb != 0 {
            HUF_decompress1X2_DCtx_wksp(
                dctx, dst, dst_size, c_src, c_src_size, work_space, wksp_size, flags,
            )
        } else {
            HUF_decompress1X1_DCtx_wksp(
                dctx, dst, dst_size, c_src, c_src_size, work_space, wksp_size, flags,
            )
        }
    }
}

/// `HUF_decompress1X_usingDTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_usingDTable(
    dst: *mut c_void,
    max_dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let dtd = huf_get_dtable_desc(dtable);
    if dtd.table_type != 0 {
        huf_decompress1_x2_using_dtable_internal(dst, max_dst_size, c_src, c_src_size, dtable, flags)
    } else {
        huf_decompress1_x1_using_dtable_internal(dst, max_dst_size, c_src, c_src_size, dtable, flags)
    }
}

/// `HUF_decompress1X1_DCtx_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X1_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    mut c_src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    let mut ip = c_src as *const BYTE;

    let h_size = HUF_readDTableX1_wksp(dctx, c_src, c_src_size, work_space, wksp_size, flags);
    if err_is_error(h_size) {
        return h_size;
    }
    if h_size >= c_src_size {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(h_size);
    c_src_size -= h_size;

    huf_decompress1_x1_using_dtable_internal(dst, dst_size, ip as *const c_void, c_src_size, dctx, flags)
}

/// `HUF_decompress4X_usingDTable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_usingDTable(
    dst: *mut c_void,
    max_dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    dtable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let dtd = huf_get_dtable_desc(dtable);
    if dtd.table_type != 0 {
        huf_decompress4_x2_using_dtable_internal(dst, max_dst_size, c_src, c_src_size, dtable, flags)
    } else {
        huf_decompress4_x1_using_dtable_internal(dst, max_dst_size, c_src, c_src_size, dtable, flags)
    }
}

/// `HUF_decompress4X_hufOnly_wksp()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_hufOnly_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dst_size: usize,
    c_src: *const c_void,
    c_src_size: usize,
    work_space: *mut c_void,
    wksp_size: usize,
    flags: c_int,
) -> usize {
    /* validation checks */
    if dst_size == 0 {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    if c_src_size == 0 {
        return err_code(ZSTD_error_corruption_detected);
    }

    {
        let algo_nb = HUF_selectDecoder(dst_size, c_src_size);
        if algo_nb != 0 {
            huf_decompress4_x2_dctx_wksp(
                dctx, dst, dst_size, c_src, c_src_size, work_space, wksp_size, flags,
            )
        } else {
            huf_decompress4_x1_dctx_wksp(
                dctx, dst, dst_size, c_src, c_src_size, work_space, wksp_size, flags,
            )
        }
    }
}

/* Keep c_uint referenced to match the C `int flags` / U32 conventions. */
const _: () = {
    let _ = core::mem::size_of::<c_uint>();
};
