//! Translation of `common/fse_decompress.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::zstd_highbit32;
use crate::bitstream::*;
use crate::entropy_common::FSE_readNCount_bmi2;
use crate::error::*;
use crate::fse::*;
use crate::mem::*;

/// `FSE_buildDTable_internal()`
pub unsafe fn fse_build_dtable_internal(
    dt: *mut FSE_DTable,
    normalized_counter: *const i16,
    max_symbol_value: u32,
    table_log: u32,
    work_space: *mut c_void,
    wksp_size: usize,
) -> usize {
    let table_decode = dt.add(1) as *mut FSE_decode_t;
    let symbol_next = work_space as *mut U16;
    let spread = symbol_next.add(max_symbol_value as usize + 1) as *mut u8;

    let max_sv1 = max_symbol_value + 1;
    let table_size: U32 = 1 << table_log;
    let mut high_threshold = table_size - 1;

    /* Sanity Checks */
    if fse_build_dtable_wksp_size(table_log, max_symbol_value) > wksp_size {
        return err_code(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if max_symbol_value > FSE_MAX_SYMBOL_VALUE {
        return err_code(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if table_log > FSE_MAX_TABLELOG {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    {
        let mut dtable_h = FSE_DTableHeader {
            tableLog: table_log as U16,
            fastMode: 1,
        };
        let large_limit: S16 = (1i32 << (table_log - 1)) as S16;
        for s in 0..max_sv1 {
            let nc = *normalized_counter.add(s as usize);
            if nc == -1 {
                (*table_decode.add(high_threshold as usize)).symbol = s as u8;
                high_threshold -= 1;
                *symbol_next.add(s as usize) = 1;
            } else {
                if nc >= large_limit {
                    dtable_h.fastMode = 0;
                }
                *symbol_next.add(s as usize) = nc as U16;
            }
        }
        core::ptr::copy_nonoverlapping(
            &dtable_h as *const FSE_DTableHeader as *const u8,
            dt as *mut u8,
            core::mem::size_of::<FSE_DTableHeader>(),
        );
    }

    /* Spread symbols */
    if high_threshold == table_size - 1 {
        let table_mask = (table_size - 1) as usize;
        let step = fse_tablestep(table_size as usize);
        /* First lay down the symbols in order, 8 bytes at a time. */
        {
            let add: U64 = 0x0101010101010101;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            for s in 0..max_sv1 {
                let n = *normalized_counter.add(s as usize) as i32;
                mem_write64(spread.add(pos), sv);
                let mut i = 8i32;
                while i < n {
                    mem_write64(spread.add(pos + i as usize), sv);
                    i += 8;
                }
                pos += n as usize;
                sv = sv.wrapping_add(add);
            }
        }
        /* Now spread those positions across the table. */
        {
            let mut position: usize = 0;
            let unroll: usize = 2;
            let mut s: usize = 0;
            while s < table_size as usize {
                for u in 0..unroll {
                    let u_position = (position + (u * step)) & table_mask;
                    (*table_decode.add(u_position)).symbol = *spread.add(s + u);
                }
                position = (position + (unroll * step)) & table_mask;
                s += unroll;
            }
        }
    } else {
        let table_mask = table_size - 1;
        let step = fse_tablestep(table_size as usize) as U32;
        let mut position: U32 = 0;
        for s in 0..max_sv1 {
            let nc = *normalized_counter.add(s as usize) as i32;
            let mut i = 0i32;
            while i < nc {
                (*table_decode.add(position as usize)).symbol = s as u8;
                position = (position + step) & table_mask;
                while position > high_threshold {
                    position = (position + step) & table_mask; /* lowprob area */
                }
                i += 1;
            }
        }
        if position != 0 {
            return err_code(ZSTD_error_GENERIC);
        }
    }

    /* Build Decoding table */
    for u in 0..table_size {
        let symbol = (*table_decode.add(u as usize)).symbol;
        let next_state = *symbol_next.add(symbol as usize);
        *symbol_next.add(symbol as usize) = next_state + 1;
        let nb_bits = (table_log - zstd_highbit32(next_state as U32)) as u8;
        (*table_decode.add(u as usize)).nbBits = nb_bits;
        (*table_decode.add(u as usize)).newState =
            (((next_state as U32) << nb_bits).wrapping_sub(table_size)) as u16;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    dt: *mut FSE_DTable,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    table_log: c_uint,
    work_space: *mut c_void,
    wksp_size: usize,
) -> usize {
    fse_build_dtable_internal(
        dt,
        normalized_counter,
        max_symbol_value,
        table_log,
        work_space,
        wksp_size,
    )
}

/// `FSE_decompress_usingDTable_generic()`
///
/// On a 64-bit target both `FSE_MAX_TABLELOG*2+7 > 64` and
/// `FSE_MAX_TABLELOG*4+7 > 64` are false, so the two conditional reloads inside
/// the 4-symbol loop are compiled out; that is reflected below.
unsafe fn fse_decompress_using_dtable_generic(
    dst: *mut u8,
    max_dst_size: usize,
    c_src: *const u8,
    c_src_size: usize,
    dt: *const FSE_DTable,
    fast: bool,
) -> usize {
    let ostart = dst;
    let mut op = ostart;
    let omax = op.add(max_dst_size);
    let olimit = omax.sub(3);

    let mut bit_d = BIT_DStream_t::default();
    let mut state1 = FSE_DState_t::default();
    let mut state2 = FSE_DState_t::default();

    /* Init */
    {
        let e = bit_init_dstream(&mut bit_d, c_src, c_src_size);
        if err_is_error(e) {
            return e;
        }
    }

    fse_init_dstate(&mut state1, &mut bit_d, dt);
    fse_init_dstate(&mut state2, &mut bit_d, dt);

    if bit_reload_dstream(&mut bit_d) == BIT_DStream_status::overflow {
        return err_code(ZSTD_error_corruption_detected);
    }

    macro_rules! get_symbol {
        ($state:expr) => {
            if fast {
                fse_decode_symbol_fast($state, &mut bit_d)
            } else {
                fse_decode_symbol($state, &mut bit_d)
            }
        };
    }

    /* 4 symbols per loop */
    while (bit_reload_dstream(&mut bit_d) == BIT_DStream_status::unfinished) & (op < olimit) {
        *op.add(0) = get_symbol!(&mut state1);
        *op.add(1) = get_symbol!(&mut state2);
        *op.add(2) = get_symbol!(&mut state1);
        *op.add(3) = get_symbol!(&mut state2);
        op = op.add(4);
    }

    /* tail */
    loop {
        if op > omax.sub(2) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        *op = get_symbol!(&mut state1);
        op = op.add(1);
        if bit_reload_dstream(&mut bit_d) == BIT_DStream_status::overflow {
            *op = get_symbol!(&mut state2);
            op = op.add(1);
            break;
        }

        if op > omax.sub(2) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        *op = get_symbol!(&mut state2);
        op = op.add(1);
        if bit_reload_dstream(&mut bit_d) == BIT_DStream_status::overflow {
            *op = get_symbol!(&mut state1);
            op = op.add(1);
            break;
        }
    }

    op as usize - ostart as usize
}

/// `FSE_DecompressWksp`
#[repr(C)]
struct FSE_DecompressWksp {
    ncount: [i16; FSE_MAX_SYMBOL_VALUE as usize + 1],
}

/// `FSE_decompress_wksp_body()`
unsafe fn fse_decompress_wksp_body(
    dst: *mut c_void,
    dst_capacity: usize,
    c_src: *const c_void,
    mut c_src_size: usize,
    max_log: c_uint,
    mut work_space: *mut c_void,
    mut wksp_size: usize,
    bmi2: c_int,
) -> usize {
    let istart = c_src as *const u8;
    let mut ip = istart;
    let mut table_log: c_uint = 0;
    let mut max_symbol_value: c_uint = FSE_MAX_SYMBOL_VALUE;
    let wksp = work_space as *mut FSE_DecompressWksp;
    let dtable_pos =
        core::mem::size_of::<FSE_DecompressWksp>() / core::mem::size_of::<FSE_DTable>();
    let dtable = (work_space as *mut FSE_DTable).add(dtable_pos);

    if wksp_size < core::mem::size_of::<FSE_DecompressWksp>() {
        return err_code(ZSTD_error_GENERIC);
    }

    /* normal FSE decoding mode */
    {
        let ncount_length = FSE_readNCount_bmi2(
            (*wksp).ncount.as_mut_ptr(),
            &mut max_symbol_value,
            &mut table_log,
            istart as *const c_void,
            c_src_size,
            bmi2,
        );
        if err_is_error(ncount_length) {
            return ncount_length;
        }
        if table_log > max_log {
            return err_code(ZSTD_error_tableLog_tooLarge);
        }
        ip = ip.add(ncount_length);
        c_src_size -= ncount_length;
    }

    if fse_decompress_wksp_size(table_log, max_symbol_value) > wksp_size {
        return err_code(ZSTD_error_tableLog_tooLarge);
    }
    let consumed = core::mem::size_of::<FSE_DecompressWksp>() + fse_dtable_size(table_log);
    work_space = (work_space as *mut u8).add(consumed) as *mut c_void;
    wksp_size -= consumed;

    {
        let e = fse_build_dtable_internal(
            dtable,
            (*wksp).ncount.as_ptr(),
            max_symbol_value,
            table_log,
            work_space,
            wksp_size,
        );
        if err_is_error(e) {
            return e;
        }
    }

    {
        let dtable_h = &*(dtable as *const FSE_DTableHeader);
        let fast_mode = dtable_h.fastMode;

        if fast_mode != 0 {
            return fse_decompress_using_dtable_generic(
                dst as *mut u8,
                dst_capacity,
                ip,
                c_src_size,
                dtable,
                true,
            );
        }
        fse_decompress_using_dtable_generic(
            dst as *mut u8,
            dst_capacity,
            ip,
            c_src_size,
            dtable,
            false,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    dst: *mut c_void,
    dst_capacity: usize,
    c_src: *const c_void,
    c_src_size: usize,
    max_log: c_uint,
    work_space: *mut c_void,
    wksp_size: usize,
    _bmi2: c_int,
) -> usize {
    fse_decompress_wksp_body(
        dst,
        dst_capacity,
        c_src,
        c_src_size,
        max_log,
        work_space,
        wksp_size,
        0,
    )
}
