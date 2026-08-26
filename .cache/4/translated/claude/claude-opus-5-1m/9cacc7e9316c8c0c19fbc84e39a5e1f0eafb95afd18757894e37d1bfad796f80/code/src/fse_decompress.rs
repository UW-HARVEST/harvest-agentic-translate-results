//! Transliteration of common/fse_decompress.c
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

use crate::bits::*;
use crate::bitstream::*;
use crate::error_private::*;
use crate::fse::*;
use crate::mem::*;

/* **************************************************************
*  Error Management
****************************************************************/
/* #define FSE_isError ERR_isError */
#[inline(always)]
pub fn FSE_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* `FSE_DECODE_TYPE` == `FSE_decode_t`, `FSE_FUNCTION_TYPE` == `BYTE` (fse.h) */

/* Defined by common/entropy_common.c (`crate::entropy_common`); declared here as an
 * extern so that this translation unit only depends on the C-visible symbol. */
extern "C" {
    fn FSE_readNCount_bmi2(
        normalizedCounter: *mut core::ffi::c_short,
        maxSVPtr: *mut core::ffi::c_uint,
        tableLogPtr: *mut core::ffi::c_uint,
        headerBuffer: *const core::ffi::c_void,
        hbSize: usize,
        bmi2: core::ffi::c_int,
    ) -> usize;
}

pub unsafe fn FSE_buildDTable_internal(
    dt: *mut FSE_DTable,
    normalizedCounter: *const core::ffi::c_short,
    maxSymbolValue: core::ffi::c_uint,
    tableLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    let tdPtr: *mut core::ffi::c_void = dt.add(1) as *mut core::ffi::c_void;
    let tableDecode: *mut FSE_decode_t = tdPtr as *mut FSE_decode_t;
    let symbolNext: *mut U16 = workSpace as *mut U16;
    let spread: *mut BYTE = symbolNext.wrapping_add(maxSymbolValue as usize).wrapping_add(1) as *mut BYTE;

    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32.wrapping_shl(tableLog);
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

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
        let mut DTableH: FSE_DTableHeader = FSE_DTableHeader::default();
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = 1i32.wrapping_shl(tableLog.wrapping_sub(1)) as S16;
            let mut s: U32;
            s = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as usize) == -1 {
                    (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
                    highThreshold = highThreshold.wrapping_sub(1);
                    *symbolNext.add(s as usize) = 1;
                } else {
                    if *normalizedCounter.add(s as usize) >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    *symbolNext.add(s as usize) = *normalizedCounter.add(s as usize) as U16;
                }
                s = s.wrapping_add(1);
            }
        }
        ZSTD_memcpy(
            dt as *mut BYTE,
            &DTableH as *const FSE_DTableHeader as *const BYTE,
            core::mem::size_of::<FSE_DTableHeader>(),
        );
    }

    /* Spread symbols */
    if highThreshold == tableSize.wrapping_sub(1) {
        let tableMask: usize = tableSize.wrapping_sub(1) as usize;
        let step: usize = FSE_TABLESTEP(tableSize) as usize;
        /* First lay down the symbols in order.
         * We use a uint64_t to lay down 8 bytes at a time. This reduces branch
         * misses since small blocks generally have small table logs, so nearly
         * all symbols have counts <= 8. We ensure we have 8 bytes at the end of
         * our buffer to handle the over-write.
         */
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32;
            s = 0;
            while s < maxSV1 {
                let mut i: core::ffi::c_int;
                let n: core::ffi::c_int = *normalizedCounter.add(s as usize) as core::ffi::c_int;
                MEM_write64(spread.wrapping_add(pos), sv);
                i = 8;
                while i < n {
                    MEM_write64(spread.wrapping_add(pos).wrapping_add(i as usize), sv);
                    i += 8;
                }
                pos = pos.wrapping_add(n as usize);
                s = s.wrapping_add(1);
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
            let mut position: usize = 0;
            let mut s: usize;
            let unroll: usize = 2;
            s = 0;
            while s < tableSize as usize {
                let mut u: usize;
                u = 0;
                while u < unroll {
                    let uPosition: usize =
                        (position.wrapping_add(u.wrapping_mul(step))) & tableMask;
                    (*tableDecode.add(uPosition)).symbol = *spread.wrapping_add(s + u);
                    u += 1;
                }
                position = (position.wrapping_add(unroll.wrapping_mul(step))) & tableMask;
                s += unroll;
            }
        }
    } else {
        let tableMask: U32 = tableSize.wrapping_sub(1);
        let step: U32 = FSE_TABLESTEP(tableSize);
        let mut s: U32;
        let mut position: U32 = 0;
        s = 0;
        while s < maxSV1 {
            let mut i: core::ffi::c_int;
            i = 0;
            while i < *normalizedCounter.add(s as usize) as core::ffi::c_int {
                (*tableDecode.add(position as usize)).symbol = s as BYTE;
                position = (position.wrapping_add(step)) & tableMask;
                while position > highThreshold {
                    position = (position.wrapping_add(step)) & tableMask;
                } /* lowprob area */
                i += 1;
            }
            s = s.wrapping_add(1);
        }
        if position != 0 {
            return ERROR(ZSTD_error_GENERIC);
        } /* position must reach all cells once, otherwise normalizedCounter is incorrect */
    }

    /* Build Decoding table */
    {
        let mut u: U32;
        u = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.add(u as usize)).symbol as BYTE;
            let nextState: U32 = *symbolNext.add(symbol as usize) as U32;
            *symbolNext.add(symbol as usize) =
                (*symbolNext.add(symbol as usize)).wrapping_add(1);
            (*tableDecode.add(u as usize)).nbBits =
                tableLog.wrapping_sub(ZSTD_highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as usize)).newState = nextState
                .wrapping_shl((*tableDecode.add(u as usize)).nbBits as U32)
                .wrapping_sub(tableSize) as U16;
            u = u.wrapping_add(1);
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    dt: *mut FSE_DTable,
    normalizedCounter: *const core::ffi::c_short,
    maxSymbolValue: core::ffi::c_uint,
    tableLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    FSE_buildDTable_internal(
        dt,
        normalizedCounter,
        maxSymbolValue,
        tableLog,
        workSpace,
        wkspSize,
    )
}

/* #ifndef FSE_COMMONDEFS_ONLY  (not defined for this build) */

/*-*******************************************************
*  Decompression (Byte symbols)
*********************************************************/

#[inline(always)]
pub unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut core::ffi::c_void,
    maxDstSize: usize,
    cSrc: *const core::ffi::c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: core::ffi::c_uint,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.wrapping_add(maxDstSize);
    let olimit: *mut BYTE = omax.wrapping_sub(3);

    let mut bitD: BIT_DStream_t = BIT_DStream_t::default();
    let mut state1: FSE_DState_t = FSE_DState_t::default();
    let mut state2: FSE_DState_t = FSE_DState_t::default();

    /* Init */
    {
        let err_code = BIT_initDStream(&mut bitD, cSrc as *const BYTE, cSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* #define FSE_GETSYMBOL(statePtr) fast ? FSE_decodeSymbolFast(statePtr, &bitD) : FSE_decodeSymbol(statePtr, &bitD) */
    macro_rules! FSE_GETSYMBOL {
        ($statePtr:expr) => {
            if fast != 0 {
                FSE_decodeSymbolFast($statePtr, &mut bitD)
            } else {
                FSE_decodeSymbol($statePtr, &mut bitD)
            }
        };
    }

    /* 4 symbols per loop */
    loop {
        let cond: core::ffi::c_int = ((BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished)
            as core::ffi::c_int)
            & ((op < olimit) as core::ffi::c_int);
        if cond == 0 {
            break;
        }

        *op.add(0) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<BitContainerType>() * 8) as U32 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL!(&mut state2);

        if FSE_MAX_TABLELOG * 4 + 7 > (core::mem::size_of::<BitContainerType>() * 8) as U32 {
            /* This test must be static */
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.wrapping_add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL!(&mut state1);

        if FSE_MAX_TABLELOG * 2 + 7 > (core::mem::size_of::<BitContainerType>() * 8) as U32 {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL!(&mut state2);

        op = op.wrapping_add(4);
    }

    /* tail */
    /* note : BIT_reloadDStream(&bitD) >= FSE_DStream_partiallyFilled; Ends at exactly BIT_DStream_completed */
    loop {
        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = FSE_GETSYMBOL!(&mut state1);
        op = op.wrapping_add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = FSE_GETSYMBOL!(&mut state2);
            op = op.wrapping_add(1);
            break;
        }

        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = FSE_GETSYMBOL!(&mut state2);
        op = op.wrapping_add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = FSE_GETSYMBOL!(&mut state1);
            op = op.wrapping_add(1);
            break;
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_DecompressWksp {
    pub ncount: [core::ffi::c_short; FSE_MAX_SYMBOL_VALUE as usize + 1],
}

#[inline(always)]
pub unsafe fn FSE_decompress_wksp_body(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    cSrc: *const core::ffi::c_void,
    mut cSrcSize: usize,
    maxLog: core::ffi::c_uint,
    mut workSpace: *mut core::ffi::c_void,
    mut wkspSize: usize,
    bmi2: core::ffi::c_int,
) -> usize {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut tableLog: core::ffi::c_uint = 0;
    let mut maxSymbolValue: core::ffi::c_uint = FSE_MAX_SYMBOL_VALUE;
    let wksp: *mut FSE_DecompressWksp = workSpace as *mut FSE_DecompressWksp;
    let dtablePos: usize =
        core::mem::size_of::<FSE_DecompressWksp>() / core::mem::size_of::<FSE_DTable>();
    let dtable: *mut FSE_DTable = (workSpace as *mut FSE_DTable).wrapping_add(dtablePos);

    if wkspSize < core::mem::size_of::<FSE_DecompressWksp>() {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* correct offset to dtable depends on this property */

    /* normal FSE decoding mode */
    {
        let NCountLength: usize = FSE_readNCount_bmi2(
            (*wksp).ncount.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const core::ffi::c_void,
            cSrcSize,
            bmi2,
        );
        if FSE_isError(NCountLength) != 0 {
            return NCountLength;
        }
        if tableLog > maxLog {
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        ip = ip.wrapping_add(NCountLength);
        cSrcSize = cSrcSize.wrapping_sub(NCountLength);
    }

    if FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    workSpace = (workSpace as *mut BYTE)
        .wrapping_add(core::mem::size_of::<FSE_DecompressWksp>())
        .wrapping_add(FSE_DTABLE_SIZE(tableLog)) as *mut core::ffi::c_void;
    wkspSize = wkspSize
        .wrapping_sub(core::mem::size_of::<FSE_DecompressWksp>() + FSE_DTABLE_SIZE(tableLog));

    {
        let err_code = FSE_buildDTable_internal(
            dtable,
            (*wksp).ncount.as_ptr(),
            maxSymbolValue,
            tableLog,
            workSpace,
            wkspSize,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    {
        let ptr: *const core::ffi::c_void = dtable as *const core::ffi::c_void;
        let DTableH: *const FSE_DTableHeader = ptr as *const FSE_DTableHeader;
        let fastMode: U32 = (*DTableH).fastMode as U32;

        /* select fast mode (static) */
        if fastMode != 0 {
            return FSE_decompress_usingDTable_generic(
                dst,
                dstCapacity,
                ip as *const core::ffi::c_void,
                cSrcSize,
                dtable,
                1,
            );
        }
        FSE_decompress_usingDTable_generic(
            dst,
            dstCapacity,
            ip as *const core::ffi::c_void,
            cSrcSize,
            dtable,
            0,
        )
    }
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub unsafe fn FSE_decompress_wksp_body_default(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    cSrc: *const core::ffi::c_void,
    cSrcSize: usize,
    maxLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    FSE_decompress_wksp_body(
        dst,
        dstCapacity,
        cSrc,
        cSrcSize,
        maxLog,
        workSpace,
        wkspSize,
        0,
    )
}

/* #if DYNAMIC_BMI2 -> FSE_decompress_wksp_body_bmi2() is not compiled (DYNAMIC_BMI2 == 0) */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    cSrc: *const core::ffi::c_void,
    cSrcSize: usize,
    maxLog: core::ffi::c_uint,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
    bmi2: core::ffi::c_int,
) -> usize {
    /* #if DYNAMIC_BMI2 ... #endif  (DYNAMIC_BMI2 == 0) */
    let _ = bmi2;
    FSE_decompress_wksp_body_default(dst, dstCapacity, cSrc, cSrcSize, maxLog, workSpace, wkspSize)
}
