//! Translation of `common/fse_decompress.c`
#![allow(dead_code)]

use super::bits::*;
use super::bitstream::*;
use super::error_private::*;
use super::fse::*;
use super::mem::*;
use crate::libc::*;
use core::ffi::{c_int, c_void};

pub unsafe fn FSE_buildDTable_internal(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let tdPtr = dt.add(1) as *mut c_void;
    let tableDecode = tdPtr as *mut FSE_decode_t;
    let symbolNext = workSpace as *mut U16;
    let spread = symbolNext.add(maxSymbolValue as usize + 1) as *mut BYTE;

    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1u32 << tableLog;
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
        let mut DTableH = FSE_DTableHeader::default();
        DTableH.tableLog = tableLog as U16;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as usize) == -1 {
                    (*tableDecode.add(highThreshold as usize)).symbol = s as BYTE;
                    highThreshold -= 1;
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
            core::ptr::addr_of!(DTableH) as *const c_void,
            core::mem::size_of::<FSE_DTableHeader>(),
        );
    }

    /* Spread symbols */
    if highThreshold == tableSize - 1 {
        let tableMask = (tableSize - 1) as usize;
        let step = FSE_TABLESTEP(tableSize) as usize;
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                MEM_write64(spread.add(pos) as *mut c_void, sv);
                let mut i: c_int = 8;
                while i < n {
                    MEM_write64(spread.add(pos + i as usize) as *mut c_void, sv);
                    i += 8;
                }
                pos += n as usize;
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        {
            let mut position: usize = 0;
            let unroll: usize = 2;
            let mut s: usize = 0;
            while s < tableSize as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let uPosition = (position + (u * step)) & tableMask;
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
            while i < *normalizedCounter.add(s as usize) as c_int {
                (*tableDecode.add(position as usize)).symbol = s as BYTE;
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask;
                }
                i += 1;
            }
            s += 1;
        }
        if position != 0 {
            return ERROR(ZSTD_error_GENERIC);
        }
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: BYTE = (*tableDecode.add(u as usize)).symbol;
            let nextState: U32 = *symbolNext.add(symbol as usize) as U32;
            *symbolNext.add(symbol as usize) = (nextState + 1) as U16;
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog - ZSTD_highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as usize)).newState =
                ((nextState << (*tableDecode.add(u as usize)).nbBits).wrapping_sub(tableSize)) as U16;
            u += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    dt: *mut FSE_DTable,
    normalizedCounter: *const i16,
    maxSymbolValue: u32,
    tableLog: u32,
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

#[inline(always)]
unsafe fn FSE_decompress_usingDTable_generic(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    dt: *const FSE_DTable,
    fast: u32,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;
    let omax = op.add(maxDstSize);
    let olimit = omax.sub(3);

    let mut bitD = BIT_DStream_t::default();
    let mut state1 = FSE_DState_t::default();
    let mut state2 = FSE_DState_t::default();

    /* Init */
    {
        let _var_err__ = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
        if ERR_isError(_var_err__) != 0 {
            return _var_err__;
        }
    }

    FSE_initDState(&mut state1, &mut bitD, dt);
    FSE_initDState(&mut state2, &mut bitD, dt);

    if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
        return ERROR(ZSTD_error_corruption_detected);
    }

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
    let bcBits = core::mem::size_of::<BitContainerType>() * 8;
    while ((BIT_reloadDStream(&mut bitD) == BIT_DStream_unfinished) as u32 & ((op < olimit) as u32))
        != 0
    {
        *op.add(0) = FSE_GETSYMBOL!(&mut state1);

        if (FSE_MAX_TABLELOG * 2 + 7) as usize > bcBits {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(1) = FSE_GETSYMBOL!(&mut state2);

        if (FSE_MAX_TABLELOG * 4 + 7) as usize > bcBits {
            if BIT_reloadDStream(&mut bitD) > BIT_DStream_unfinished {
                op = op.add(2);
                break;
            }
        }

        *op.add(2) = FSE_GETSYMBOL!(&mut state1);

        if (FSE_MAX_TABLELOG * 2 + 7) as usize > bcBits {
            BIT_reloadDStream(&mut bitD);
        }

        *op.add(3) = FSE_GETSYMBOL!(&mut state2);

        op = op.add(4);
    }

    /* tail */
    loop {
        if op > omax.sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = FSE_GETSYMBOL!(&mut state1);
        op = op.add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = FSE_GETSYMBOL!(&mut state2);
            op = op.add(1);
            break;
        }

        if op > omax.sub(2) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        *op = FSE_GETSYMBOL!(&mut state2);
        op = op.add(1);
        if BIT_reloadDStream(&mut bitD) == BIT_DStream_overflow {
            *op = FSE_GETSYMBOL!(&mut state1);
            op = op.add(1);
            break;
        }
    }

    op.offset_from(ostart) as usize
}

#[repr(C)]
pub struct FSE_DecompressWksp {
    pub ncount: [i16; (FSE_MAX_SYMBOL_VALUE + 1) as usize],
}

unsafe fn FSE_decompress_wksp_body(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    maxLog: u32,
    mut workSpace: *mut c_void,
    mut wkspSize: usize,
    bmi2: c_int,
) -> usize {
    let istart = cSrc as *const BYTE;
    let mut ip = istart;
    let mut tableLog: u32 = 0;
    let mut maxSymbolValue: u32 = FSE_MAX_SYMBOL_VALUE;
    let wksp = workSpace as *mut FSE_DecompressWksp;
    let dtablePos =
        core::mem::size_of::<FSE_DecompressWksp>() / core::mem::size_of::<FSE_DTable>();
    let dtable = (workSpace as *mut FSE_DTable).add(dtablePos);

    if wkspSize < core::mem::size_of::<FSE_DecompressWksp>() {
        return ERROR(ZSTD_error_GENERIC);
    }

    /* normal FSE decoding mode */
    {
        let NCountLength = super::entropy_common::FSE_readNCount_bmi2(
            (*wksp).ncount.as_mut_ptr(),
            &mut maxSymbolValue,
            &mut tableLog,
            istart as *const c_void,
            cSrcSize,
            bmi2,
        );
        if super::entropy_common::FSE_isError(NCountLength) != 0 {
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
        .add(core::mem::size_of::<FSE_DecompressWksp>() + FSE_DTABLE_SIZE(tableLog))
        as *mut c_void;
    wkspSize -= core::mem::size_of::<FSE_DecompressWksp>() + FSE_DTABLE_SIZE(tableLog);

    {
        let _var_err__ = FSE_buildDTable_internal(
            dtable,
            (*wksp).ncount.as_ptr(),
            maxSymbolValue,
            tableLog,
            workSpace,
            wkspSize,
        );
        if ERR_isError(_var_err__) != 0 {
            return _var_err__;
        }
    }

    {
        let ptr = dtable as *const c_void;
        let DTableH = ptr as *const FSE_DTableHeader;
        let fastMode = (*DTableH).fastMode as U32;

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

unsafe fn FSE_decompress_wksp_body_default(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    maxLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    FSE_decompress_wksp_body(
        dst, dstCapacity, cSrc, cSrcSize, maxLog, workSpace, wkspSize, 0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    dst: *mut c_void,
    dstCapacity: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    maxLog: u32,
    workSpace: *mut c_void,
    wkspSize: usize,
    _bmi2: c_int,
) -> usize {
    FSE_decompress_wksp_body_default(
        dst, dstCapacity, cSrc, cSrcSize, maxLog, workSpace, wkspSize,
    )
}
