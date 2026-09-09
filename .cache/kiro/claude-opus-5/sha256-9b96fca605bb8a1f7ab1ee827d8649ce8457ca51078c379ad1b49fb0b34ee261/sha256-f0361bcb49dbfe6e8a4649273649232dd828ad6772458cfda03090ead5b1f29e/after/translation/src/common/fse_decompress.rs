//! Translation of `common/fse_decompress.c`.
#![allow(dead_code)]

use super::bits::ZSTD_highbit32;
use super::bitstream::{
    BIT_initDStream, BIT_reloadDStream, BIT_DStream_overflow, BIT_DStream_t,
    BIT_DStream_unfinished,
};
use super::error_private::*;
use super::fse::{
    FSE_decodeSymbol, FSE_decodeSymbolFast,
    FSE_initDState, FSE_BUILD_DTABLE_WKSP_SIZE, FSE_DECOMPRESS_WKSP_SIZE, FSE_DTABLE_SIZE,
    FSE_DState_t, FSE_DTable, FSE_DTableHeader, FSE_MAX_SYMBOL_VALUE, FSE_MAX_TABLELOG,
    FSE_TABLESTEP, FSE_decode_t,
};
use super::entropy_common::{FSE_isError, FSE_readNCount_bmi2};
use super::mem::{
    size_t, MEM_write64, ZSTD_memcpy, BYTE, S16, U16, U32, U64,
};
use core::ffi::{c_int, c_uint, c_void};

pub unsafe fn FSE_buildDTable_internal(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let tdPtr: *mut c_void = dt.add(1) as *mut c_void; /* because *dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode: *mut FSE_decode_t = tdPtr as *mut FSE_decode_t;
    let symbolNext: *mut U16 = workSpace as *mut U16;
    let spread: *mut BYTE = symbolNext.add((maxSymbolValue + 1) as size_t) as *mut BYTE;

    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1 << tableLog;
    let mut highThreshold: U32 = tableSize - 1;

    /* Sanity Checks */
    if FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE {
        return ERROR(ZSTD_error_maxSymbolValue_tooLarge);
    }
    if tableLog > FSE_MAX_TABLELOG {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
            tableLog: 0,
            fastMode: 0,
        };
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as size_t) == -1 {
                    (*tableDecode.add(highThreshold as size_t)).symbol = s as BYTE;
                    highThreshold -= 1;
                    *symbolNext.add(s as size_t) = 1;
                } else {
                    if *normalizedCounter.add(s as size_t) >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    *symbolNext.add(s as size_t) = *normalizedCounter.add(s as size_t) as U16;
                }
                s += 1;
            }
        }
        ZSTD_memcpy(
            dt as *mut BYTE,
            &DTableH as *const FSE_DTableHeader as *const BYTE,
            core::mem::size_of::<FSE_DTableHeader>() as size_t,
        );
    }

    /* Spread symbols */
    if highThreshold == tableSize - 1 {
        let tableMask: size_t = (tableSize - 1) as size_t;
        let step: size_t = FSE_TABLESTEP(tableSize) as size_t;
        /* First lay down the symbols in order.
         * We use a uint64_t to lay down 8 bytes at a time. This reduces branch
         * misses since small blocks generally have small table logs, so nearly
         * all symbols have counts <= 8. We ensure we have 8 bytes at the end of
         * our buffer to handle the over-write.
         */
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: size_t = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let n: c_int = *normalizedCounter.add(s as size_t) as c_int;
                MEM_write64(spread.add(pos), sv);
                let mut i: c_int = 8;
                while i < n {
                    MEM_write64(spread.add(pos + i as size_t), sv);
                    i += 8;
                }
                pos += n as size_t;
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        /* Now we spread those positions across the table.
         * The benefit of doing it in two stages is that we avoid the
         * variable size inner loop, which caused lots of branch misses.
         * Now we can run through all the positions without any branch misses.
         * We unroll the loop twice, since that is what empirically worked best.
         */
        {
            let mut position: size_t = 0;
            let mut s: size_t = 0;
            let unroll: size_t = 2;
            while s < tableSize as size_t {
                let mut u: size_t = 0;
                while u < unroll {
                    let uPosition: size_t = (position + (u * step)) & tableMask;
                    (*tableDecode.add(uPosition)).symbol = *spread.add(s + u);
                    u += 1;
                }
                position = (position + (unroll * step)) & tableMask;
                s += unroll;
            }
        }
    } else {
        let tableMask: U32 = tableSize - 1;
        let step: U32 = FSE_TABLESTEP(tableSize);
        let mut position: U32 = 0;
        let mut s: U32 = 0;
        while s < maxSV1 {
            let mut i: c_int = 0;
            while i < *normalizedCounter.add(s as size_t) as c_int {
                (*tableDecode.add(position as size_t)).symbol = s as BYTE;
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask; /* lowprob area */
                }
                i += 1;
            }
            s += 1;
        }
        if position != 0 {
            return ERROR(ZSTD_error_GENERIC);
        } /* position must reach all cells once, otherwise normalizedCounter is incorrect */
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.add(u as size_t)).symbol;
            let nextState: U32 = *symbolNext.add(symbol as size_t) as U32;
            *symbolNext.add(symbol as size_t) += 1;
            (*tableDecode.add(u as size_t)).nbBits =
                (tableLog - ZSTD_highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as size_t)).newState =
                ((nextState << (*tableDecode.add(u as size_t)).nbBits) - tableSize) as U16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    dt: *mut FSE_DTable,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    FSE_buildDTable_internal(dt, normalizedCounter, maxSymbolValue, tableLog, workSpace, wkspSize)
}

/*-*******************************************************
*  Decompression (Byte symbols)
*********************************************************/

pub unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    dt: *const FSE_DTable,
    fast: c_uint,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.add(maxDstSize);
    let olimit: *mut BYTE = omax.sub(3);

    let mut bitD: BIT_DStream_t = core::mem::zeroed();
    let mut state1: FSE_DState_t = core::mem::zeroed();
    let mut state2: FSE_DState_t = core::mem::zeroed();

    /* Init */
    {
        let err: size_t = BIT_initDStream(&mut bitD, cSrc as *const u8, cSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* 4 symbols per loop */
    while (BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) & (op < olimit) {
        *op.add(0) = fse_getsymbol(fast, &mut state1, &mut bitD);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of_val(&bitD.bitContainer) * 8) as U32 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = fse_getsymbol(fast, &mut state2, &mut bitD);

        if FSE_MAX_TABLELOG * 4 + 7 > (core::mem::size_of_val(&bitD.bitContainer) * 8) as U32 {
            /* This test must be static */
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = fse_getsymbol(fast, &mut state1, &mut bitD);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of_val(&bitD.bitContainer) * 8) as U32 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = fse_getsymbol(fast, &mut state2, &mut bitD);

        op = op.add(4);
    }

    /* tail */
    /* note : BIT_reloadDStream(&bitD) >= FSE_DStream_partiallyFilled; Ends at exactly BIT_DStream_completed */
    loop {
        if op > omax.sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = fse_getsymbol(fast, &mut state1, &mut bitD);
        op = op.add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = fse_getsymbol(fast, &mut state2, &mut bitD);
            op = op.add(1);
            break;
        }

        if op > omax.sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = fse_getsymbol(fast, &mut state2, &mut bitD);
        op = op.add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = fse_getsymbol(fast, &mut state1, &mut bitD);
            op = op.add(1);
            break;
        }
    }

    op.offset_from(ostart) as size_t
}

#[inline(always)]
unsafe fn fse_getsymbol(
    fast: c_uint,
    statePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    if fast != 0 {
        FSE_decodeSymbolFast(statePtr, bitD)
    } else {
        FSE_decodeSymbol(statePtr, bitD)
    }
}

#[repr(C)]
pub struct FSE_DecompressWksp {
    pub ncount: [S16; (FSE_MAX_SYMBOL_VALUE + 1) as usize],
}

pub unsafe fn FSE_decompress_wksp_body(
    dst: *mut c_void,
    dstCapacity: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
    maxLog: c_uint,
    mut workSpace: *mut c_void,
    mut wkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSE_MAX_SYMBOL_VALUE;
    let wksp: *mut FSE_DecompressWksp = workSpace as *mut FSE_DecompressWksp;
    let dtablePos: size_t =
        core::mem::size_of::<FSE_DecompressWksp>() as size_t / core::mem::size_of::<FSE_DTable>() as size_t;
    let dtable: *mut FSE_DTable = (workSpace as *mut FSE_DTable).add(dtablePos);

    if wkspSize < core::mem::size_of::<FSE_DecompressWksp>() as size_t {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* normal FSE decoding mode */
    {
        let NCountLength: size_t = FSE_readNCount_bmi2(
            (*wksp).ncount.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
            bmi2,
        );
        if FSE_isError(NCountLength) != 0 {
            return NCountLength;
        }
        if tableLog > maxLog {
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        ip = ip.add(NCountLength);
        cSrcSize -= NCountLength;
    }

    if FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    workSpace = (workSpace as *mut BYTE)
        .add(core::mem::size_of::<FSE_DecompressWksp>() as size_t + FSE_DTABLE_SIZE(tableLog))
        as *mut c_void;
    wkspSize -= core::mem::size_of::<FSE_DecompressWksp>() as size_t + FSE_DTABLE_SIZE(tableLog);

    {
        let err: size_t = FSE_buildDTable_internal(
            dtable,
            (*wksp).ncount.as_ptr(),
            maxSymbolValue,
            tableLog,
            workSpace,
            wkspSize,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }

    {
        let ptr: *const c_void = dtable as *const c_void;
        let DTableH: *const FSE_DTableHeader = ptr as *const FSE_DTableHeader;
        let fastMode: U32 = (*DTableH).fastMode as U32;

        /* select fast mode (static) */
        if fastMode != 0 {
            return FSE_decompress_usingDTable_generic(
                dst,
                dstCapacity,
                ip as *const c_void,
                cSrcSize,
                dtable,
                1,
            );
        }
        FSE_decompress_usingDTable_generic(
            dst,
            dstCapacity,
            ip as *const c_void,
            cSrcSize,
            dtable,
            0,
        )
    }
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub unsafe fn FSE_decompress_wksp_body_default(
    dst: *mut c_void,
    dstCapacity: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    maxLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    FSE_decompress_wksp_body(dst, dstCapacity, cSrc, cSrcSize, maxLog, workSpace, wkspSize, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    dst: *mut c_void,
    dstCapacity: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    maxLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    let _ = bmi2;
    FSE_decompress_wksp_body_default(dst, dstCapacity, cSrc, cSrcSize, maxLog, workSpace, wkspSize)
}
