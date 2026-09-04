//! Translation of `common/fse_decompress.c`
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::ZSTD_highbit32;
use crate::bitstream::*;
use crate::cmem::*;
use crate::entropy_common::FSE_readNCount_bmi2;
use crate::error_private::*;
use crate::fse::*;

/* **************************************************************
*  Templates
****************************************************************/
/*
  designed to be included
  for type-specific functions (template emulation in C)
  Objective is to write these functions only once, for improved maintenance
*/

pub(crate) unsafe fn FSE_buildDTable_internal(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let tdPtr = dt.add(1) as *mut c_void; /* because *dt is unsigned, 32-bits aligned on 32-bits */
    let tableDecode = tdPtr as *mut FSE_decode_t;
    let symbolNext = workSpace as *mut U16;
    let spread = symbolNext.add(maxSymbolValue as usize + 1) as *mut BYTE;

    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32 << tableLog;
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
            let largeLimit: S16 = (1i32).wrapping_shl(tableLog.wrapping_sub(1)) as S16;
            let mut s: U32 = 0;
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
                s += 1;
            }
        }
        ZSTD_memcpy(
            dt as *mut c_void,
            &DTableH as *const FSE_DTableHeader as *const c_void,
            core::mem::size_of::<FSE_DTableHeader>(),
        );
    }

    /* Spread symbols */
    if highThreshold == tableSize - 1 {
        let tableMask: usize = (tableSize - 1) as usize;
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
            let mut s: U32 = 0;
            while s < maxSV1 {
                let mut i: c_int;
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                MEM_write64(spread.add(pos) as *mut c_void, sv);
                i = 8;
                while i < n {
                    MEM_write64(spread.add(pos).offset(i as isize) as *mut c_void, sv);
                    i += 8;
                }
                pos = pos.wrapping_add(n as usize);
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
            let mut position: usize = 0;
            let mut s: usize = 0;
            let unroll: usize = 2;
            while s < tableSize as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let uPosition: usize = (position.wrapping_add(u.wrapping_mul(step))) & tableMask;
                    (*tableDecode.add(uPosition)).symbol = *spread.add(s + u);
                    u += 1;
                }
                position = (position.wrapping_add(unroll.wrapping_mul(step))) & tableMask;
                s += unroll;
            }
        }
    } else {
        let tableMask: U32 = tableSize - 1;
        let step: U32 = FSE_TABLESTEP(tableSize);
        let mut s: U32;
        let mut position: U32 = 0;
        s = 0;
        while s < maxSV1 {
            let mut i: c_int = 0;
            while i < *normalizedCounter.add(s as usize) as c_int {
                (*tableDecode.add(position as usize)).symbol = s as BYTE;
                position = (position.wrapping_add(step)) & tableMask;
                while position > highThreshold {
                    position = (position.wrapping_add(step)) & tableMask;
                } /* lowprob area */
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
            let symbol: BYTE = (*tableDecode.add(u as usize)).symbol as BYTE;
            let nextState: U32 = *symbolNext.add(symbol as usize) as U32;
            *symbolNext.add(symbol as usize) =
                (*symbolNext.add(symbol as usize)).wrapping_add(1);
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog.wrapping_sub(ZSTD_highbit32(nextState))) as BYTE;
            (*tableDecode.add(u as usize)).newState = (nextState
                .wrapping_shl((*tableDecode.add(u as usize)).nbBits as u32)
                .wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    tableLog: c_uint,
    workSpace: *mut c_void,
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

/*-*******************************************************
*  Decompression (Byte symbols)
*********************************************************/

pub(crate) unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: c_uint,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.wrapping_add(maxDstSize);
    let olimit = omax.wrapping_sub(3);

    let mut bitD: BIT_DStream_t = BIT_DStream_t::default();
    let mut state1: FSE_DState_t = FSE_DState_t::default();
    let mut state2: FSE_DState_t = FSE_DState_t::default();

    /* Init */
    {
        let err = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
        return ERROR(ZSTD_error_corruption_detected);
    }

    const CONTAINER_BITS: u32 = (core::mem::size_of::<BitContainerType>() * 8) as u32;

    /* 4 symbols per loop */
    loop {
        let cond = ((BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) as u32)
            & ((op < olimit) as u32);
        if cond == 0 {
            break;
        }

        *op.add(0) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if FSE_MAX_TABLELOG * 2 + 7 > CONTAINER_BITS {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };

        if FSE_MAX_TABLELOG * 4 + 7 > CONTAINER_BITS {
            /* This test must be static */
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };

        if FSE_MAX_TABLELOG * 2 + 7 > CONTAINER_BITS {
            /* This test must be static */
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };

        op = op.add(4);
    }

    /* tail */
    /* note : BIT_reloadDStream(&bitD) >= FSE_DStream_partiallyFilled; Ends at exactly BIT_DStream_completed */
    loop {
        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = if fast != 0 {
            FSE_decodeSymbolFast(&mut state1, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state1, &mut bitD)
        };
        op = op.add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = if fast != 0 {
                FSE_decodeSymbolFast(&mut state2, &mut bitD)
            } else {
                FSE_decodeSymbol(&mut state2, &mut bitD)
            };
            op = op.add(1);
            break;
        }

        if op > omax.wrapping_sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = if fast != 0 {
            FSE_decodeSymbolFast(&mut state2, &mut bitD)
        } else {
            FSE_decodeSymbol(&mut state2, &mut bitD)
        };
        op = op.add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = if fast != 0 {
                FSE_decodeSymbolFast(&mut state1, &mut bitD)
            } else {
                FSE_decodeSymbol(&mut state1, &mut bitD)
            };
            op = op.add(1);
            break;
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FSE_DecompressWksp {
    pub ncount: [i16; FSE_MAX_SYMBOL_VALUE as usize + 1],
}

pub(crate) unsafe fn FSE_decompress_wksp_body(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    maxLog: c_uint,
    mut workSpace: *mut c_void,
    mut wkspSize: usize,
    bmi2: c_int,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut tableLog: c_uint = 0;
    let mut maxSymbolValue: c_uint = FSE_MAX_SYMBOL_VALUE;
    let wksp = workSpace as *mut FSE_DecompressWksp;
    let dtablePos: usize =
        core::mem::size_of::<FSE_DecompressWksp>() / core::mem::size_of::<FSE_DTable>();
    let dtable = (workSpace as *mut FSE_DTable).add(dtablePos);

    if wkspSize < core::mem::size_of::<FSE_DecompressWksp>() {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* correct offset to dtable depends on this property */

    /* normal FSE decoding mode */
    {
        let NCountLength = FSE_readNCount_bmi2(
            (*wksp).ncount.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
            bmi2,
        );
        if ERR_isError(NCountLength) != 0 {
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
    workSpace = (workSpace as *mut BYTE).add(
        core::mem::size_of::<FSE_DecompressWksp>()
            + FSE_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<FSE_DTable>(),
    ) as *mut c_void;
    wkspSize -= core::mem::size_of::<FSE_DecompressWksp>()
        + FSE_DTABLE_SIZE_U32(tableLog) * core::mem::size_of::<FSE_DTable>();

    {
        let err = FSE_buildDTable_internal(
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
        let ptr = dtable as *const c_void;
        let DTableH = ptr as *const FSE_DTableHeader;
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
pub(crate) unsafe fn FSE_decompress_wksp_body_default(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    maxLog: c_uint,
    workSpace: *mut c_void,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    maxLog: c_uint,
    workSpace: *mut c_void,
    wkspSize: usize,
    bmi2: c_int,
) -> usize {
    let _ = bmi2;
    FSE_decompress_wksp_body_default(
        dst,
        dstCapacity,
        cSrc,
        cSrcSize,
        maxLog,
        workSpace,
        wkspSize,
    )
}
