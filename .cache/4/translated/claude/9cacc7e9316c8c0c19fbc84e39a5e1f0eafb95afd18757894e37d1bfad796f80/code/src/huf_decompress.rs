//! Transliteration of decompress/huf_decompress.c
//!
//! huff0 huffman decoder, part of Finite State Entropy library.
//!
//! Build configuration reproduced here:
//!   * `DYNAMIC_BMI2 == 0`             -> no `_bmi2` variants, `HUF_DGEN` uses the `#else` branch.
//!   * `ZSTD_ENABLE_ASM_X86_64_BMI2 == 0` -> the `_fast_asm_loop` paths are not compiled in.
//!   * `HUF_FORCE_DECOMPRESS_X1` / `HUF_FORCE_DECOMPRESS_X2` are *not* defined -> both decoders exist.
//!   * `HUF_DISABLE_FAST_DECODE` is not defined -> `HUF_ENABLE_FAST_DECODE == 1`.
//!   * `__clang__` is not defined (gcc build) -> the `#else` branch of the 4X2 body loop.
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

use core::ffi::{c_char, c_int, c_void};
use core::ptr::{addr_of, addr_of_mut};

use crate::bits::*;
use crate::bitstream::*;
use crate::compiler::*;
use crate::entropy_common::HUF_readStats_wksp;
use crate::error_private::*;
use crate::huf::*;
use crate::mem::*;
use crate::zstd_internal::MIN;

/* **************************************************************
*  Constants
****************************************************************/

pub const HUF_DECODER_FAST_TABLELOG: U32 = 11;

/* **************************************************************
*  Macros
****************************************************************/

/* #ifdef HUF_DISABLE_FAST_DECODE -> not defined */
pub const HUF_ENABLE_FAST_DECODE: c_int = 1;

/* DYNAMIC_BMI2 == 0 -> HUF_FAST_BMI2_ATTRS is empty, HUF_NEED_BMI2_FUNCTION == 0 */

/* **************************************************************
*  Error Management
****************************************************************/
/* #define HUF_isError ERR_isError */
#[inline(always)]
pub fn HUF_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/* **************************************************************
*  Byte alignment for workSpace management
****************************************************************/
/* HUF_ALIGN / HUF_ALIGN_MASK are unused in this translation unit. */
#[inline(always)]
pub const fn HUF_ALIGN_MASK(x: usize, mask: usize) -> usize {
    (x + mask) & !mask
}
#[inline(always)]
pub const fn HUF_ALIGN(x: usize, a: usize) -> usize {
    HUF_ALIGN_MASK(x, a - 1)
}

/* **************************************************************
*  BMI2 Variant Wrappers
****************************************************************/
pub type HUF_DecompressUsingDTableFn = unsafe fn(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize;

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DTableDesc {
    pub maxTableLog: BYTE,
    pub tableType: BYTE,
    pub tableLog: BYTE,
    pub reserved: BYTE,
}

pub unsafe fn HUF_getDTableDesc(table: *const HUF_DTable) -> DTableDesc {
    let mut dtd: DTableDesc = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    ZSTD_memcpy(
        addr_of_mut!(dtd) as *mut u8,
        table as *const u8,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

pub unsafe fn HUF_initFastDStream(ip: *const BYTE) -> usize {
    let lastByte: BYTE = *ip.add(7);
    let bitsConsumed: usize = if lastByte != 0 {
        (8u32.wrapping_sub(ZSTD_highbit32(lastByte as U32))) as usize
    } else {
        0
    };
    let value: usize = MEM_readLEST(ip) | 1;
    value << bitsConsumed
}

/**
 * The input/output arguments to the Huffman fast decoding loop.
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DecompressFastArgs {
    pub ip: [*const BYTE; 4],
    pub op: [*mut BYTE; 4],
    pub bits: [U64; 4],
    pub dt: *const c_void,
    pub ilowest: *const BYTE,
    pub oend: *mut BYTE,
    pub iend: [*const BYTE; 4],
}

pub type HUF_DecompressFastLoopFn = unsafe fn(args: *mut HUF_DecompressFastArgs);

/**
 * Initializes args for the fast decoding loop.
 * @returns 1 on success
 *          0 if the fallback implementation should be used.
 *          Or an error code on failure.
 */
pub unsafe fn HUF_DecompressFastArgs_init(
    args: *mut HUF_DecompressFastArgs,
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let dt: *const c_void = DTable.add(1) as *const c_void;
    let dtLog: U32 = HUF_getDTableDesc(DTable).tableLog as U32;

    let istart: *const BYTE = src as *const BYTE;

    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);

    /* The fast decoding loop assumes 64-bit little-endian.
     * This condition is false on x32.
     */
    if MEM_isLittleEndian() == 0 || MEM_32bits() != 0 {
        return 0;
    }

    /* Avoid nullptr addition */
    if dstSize == 0 {
        return 0;
    }

    /* strict minimum : jump table + 1 byte per stream */
    if srcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* Must have at least 8 bytes per stream because we don't handle initializing
     * smaller bit containers.
     */
    if dtLog != HUF_DECODER_FAST_TABLELOG {
        return 0;
    }

    /* Read the jump table. */
    {
        let length1: usize = MEM_readLE16(istart) as usize;
        let length2: usize = MEM_readLE16(istart.add(2)) as usize;
        let length3: usize = MEM_readLE16(istart.add(4)) as usize;
        let length4: usize = srcSize.wrapping_sub(length1 + length2 + length3 + 6);
        (*args).iend[0] = istart.wrapping_add(6); /* jumpTable */
        (*args).iend[1] = (*args).iend[0].wrapping_add(length1);
        (*args).iend[2] = (*args).iend[1].wrapping_add(length2);
        (*args).iend[3] = (*args).iend[2].wrapping_add(length3);

        /* HUF_initFastDStream() requires this, and this small of an input
         * won't benefit from the ASM loop anyways.
         */
        if length1 < 8 || length2 < 8 || length3 < 8 || length4 < 8 {
            return 0;
        }
        if length4 > srcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
    }
    /* ip[] contains the position that is currently loaded into bits[]. */
    (*args).ip[0] = (*args).iend[1].wrapping_sub(core::mem::size_of::<U64>());
    (*args).ip[1] = (*args).iend[2].wrapping_sub(core::mem::size_of::<U64>());
    (*args).ip[2] = (*args).iend[3].wrapping_sub(core::mem::size_of::<U64>());
    (*args).ip[3] = (src as *const BYTE)
        .wrapping_add(srcSize)
        .wrapping_sub(core::mem::size_of::<U64>());

    /* op[] contains the output pointers. */
    (*args).op[0] = dst as *mut BYTE;
    (*args).op[1] = (*args).op[0].wrapping_add((dstSize + 3) / 4);
    (*args).op[2] = (*args).op[1].wrapping_add((dstSize + 3) / 4);
    (*args).op[3] = (*args).op[2].wrapping_add((dstSize + 3) / 4);

    /* No point to call the ASM loop for tiny outputs. */
    if (*args).op[3] >= oend {
        return 0;
    }

    /* bits[] is the bit container. */
    (*args).bits[0] = HUF_initFastDStream((*args).ip[0]) as U64;
    (*args).bits[1] = HUF_initFastDStream((*args).ip[1]) as U64;
    (*args).bits[2] = HUF_initFastDStream((*args).ip[2]) as U64;
    (*args).bits[3] = HUF_initFastDStream((*args).ip[3]) as U64;

    /* The decoders must be sure to never read beyond ilowest. */
    (*args).ilowest = istart;

    (*args).oend = oend;
    (*args).dt = dt;

    1
}

pub unsafe fn HUF_initRemainingDStream(
    bit: *mut BIT_DStream_t,
    args: *const HUF_DecompressFastArgs,
    stream: c_int,
    segmentEnd: *mut BYTE,
) -> usize {
    /* Validate that we haven't overwritten. */
    if (*args).op[stream as usize] > segmentEnd {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Validate that we haven't read beyond iend[]. */
    if (*args).ip[stream as usize] < (*args).iend[stream as usize].wrapping_sub(8) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* Construct the BIT_DStream_t. */
    (*bit).bitContainer = MEM_readLEST((*args).ip[stream as usize]);
    (*bit).bitsConsumed = ZSTD_countTrailingZeros64((*args).bits[stream as usize]);
    (*bit).start = (*args).ilowest as *const c_char;
    (*bit).limitPtr = ((*bit).start as *const u8).wrapping_add(core::mem::size_of::<usize>())
        as *const c_char;
    (*bit).ptr = (*args).ip[stream as usize] as *const c_char;

    0
}

/* HUF_4X_FOR_EACH_STREAM / HUF_4X_FOR_EACH_STREAM_WITH_VAR are expanded manually
 * below at each use site. */

/* #ifndef HUF_FORCE_DECOMPRESS_X2 */

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DEltX1 {
    pub nbBits: BYTE,
    pub byte: BYTE,
}

/**
 * Packs 4 HUF_DEltX1 structs into a U64. This is used to lay down 4 entries at
 * a time.
 */
pub fn HUF_DEltX1_set4(symbol: BYTE, nbBits: BYTE) -> U64 {
    let mut D4: U64;
    if MEM_isLittleEndian() != 0 {
        D4 = ((((symbol as c_int) << 8) + (nbBits as c_int)) as U64);
    } else {
        D4 = (((symbol as c_int) + ((nbBits as c_int) << 8)) as U64);
    }
    D4 = D4.wrapping_mul(0x0001000100010001u64);
    D4
}

/**
 * Increase the tableLog to targetTableLog and rescales the stats.
 * If tableLog > targetTableLog this is a no-op.
 * @returns New tableLog
 */
pub unsafe fn HUF_rescaleStats(
    huffWeight: *mut BYTE,
    rankVal: *mut U32,
    nbSymbols: U32,
    tableLog: U32,
    targetTableLog: U32,
) -> U32 {
    if tableLog > targetTableLog {
        return tableLog;
    }
    if tableLog < targetTableLog {
        let scale: U32 = targetTableLog.wrapping_sub(tableLog);
        let mut s: U32;
        /* Increase the weight for all non-zero probability symbols by scale. */
        s = 0;
        while s < nbSymbols {
            *huffWeight.add(s as usize) = (*huffWeight.add(s as usize)).wrapping_add(
                (if *huffWeight.add(s as usize) == 0 {
                    0u32
                } else {
                    scale
                }) as BYTE,
            );
            s = s.wrapping_add(1);
        }
        /* Update rankVal to reflect the new weights. */
        s = targetTableLog;
        while s > scale {
            *rankVal.add(s as usize) = *rankVal.add(s.wrapping_sub(scale) as usize);
            s = s.wrapping_sub(1);
        }
        s = scale;
        while s > 0 {
            *rankVal.add(s as usize) = 0;
            s = s.wrapping_sub(1);
        }
    }
    targetTableLog
}

#[repr(C)]
pub struct HUF_ReadDTableX1_Workspace {
    pub rankVal: [U32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1],
    pub rankStart: [U32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1],
    pub statsWksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
    pub symbols: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1],
    pub huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX1_wksp(
    DTable: *mut HUF_DTable,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let iSize: usize;
    let dtPtr: *mut c_void = DTable.add(1) as *mut c_void;
    let dt: *mut HUF_DEltX1 = dtPtr as *mut HUF_DEltX1;
    let wksp: *mut HUF_ReadDTableX1_Workspace = workSpace as *mut HUF_ReadDTableX1_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX1_Workspace>() > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    let wksp_rankVal: *mut U32 = addr_of_mut!((*wksp).rankVal) as *mut U32;
    let wksp_rankStart: *mut U32 = addr_of_mut!((*wksp).rankStart) as *mut U32;
    let wksp_statsWksp: *mut U32 = addr_of_mut!((*wksp).statsWksp) as *mut U32;
    let wksp_symbols: *mut BYTE = addr_of_mut!((*wksp).symbols) as *mut BYTE;
    let wksp_huffWeight: *mut BYTE = addr_of_mut!((*wksp).huffWeight) as *mut BYTE;

    /* ZSTD_memset(huffWeight, 0, sizeof(huffWeight)); */ /* is not necessary */

    iSize = HUF_readStats_wksp(
        wksp_huffWeight,
        HUF_SYMBOLVALUE_MAX as usize + 1,
        wksp_rankVal,
        addr_of_mut!(nbSymbols),
        addr_of_mut!(tableLog),
        src,
        srcSize,
        wksp_statsWksp as *mut c_void,
        core::mem::size_of::<[U32; HUF_READ_STATS_WORKSPACE_SIZE_U32]>(),
        flags,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* Table header */
    {
        let mut dtd: DTableDesc = HUF_getDTableDesc(DTable);
        let maxTableLog: U32 = (dtd.maxTableLog as U32).wrapping_add(1);
        let targetTableLog: U32 = MIN(maxTableLog, HUF_DECODER_FAST_TABLELOG);
        tableLog = HUF_rescaleStats(
            wksp_huffWeight,
            wksp_rankVal,
            nbSymbols,
            tableLog,
            targetTableLog,
        );
        if tableLog > (dtd.maxTableLog as U32).wrapping_add(1) {
            /* DTable too small, Huffman tree cannot fit in */
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as BYTE;
        ZSTD_memcpy(
            DTable as *mut u8,
            addr_of!(dtd) as *const u8,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Compute symbols and rankStart given rankVal */
    {
        let mut n: c_int;
        let mut nextRankStart: U32 = 0;
        let unroll: c_int = 4;
        let nLimit: c_int = (nbSymbols as c_int) - unroll + 1;
        n = 0;
        while n < (tableLog as c_int) + 1 {
            let curr: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(*wksp_rankVal.add(n as usize));
            *wksp_rankStart.add(n as usize) = curr;
            n += 1;
        }
        n = 0;
        while n < nLimit {
            let mut u: c_int;
            u = 0;
            while u < unroll {
                let w: usize = *wksp_huffWeight.add((n + u) as usize) as usize;
                let r: U32 = *wksp_rankStart.add(w);
                *wksp_rankStart.add(w) = r.wrapping_add(1);
                *wksp_symbols.add(r as usize) = (n + u) as BYTE;
                u += 1;
            }
            n += unroll;
        }
        while n < (nbSymbols as c_int) {
            let w: usize = *wksp_huffWeight.add(n as usize) as usize;
            let r: U32 = *wksp_rankStart.add(w);
            *wksp_rankStart.add(w) = r.wrapping_add(1);
            *wksp_symbols.add(r as usize) = n as BYTE;
            n += 1;
        }
    }

    /* fill DTable */
    {
        let mut w: U32;
        let mut symbol: c_int = *wksp_rankVal.add(0) as c_int;
        let mut rankStart: c_int = 0;
        w = 1;
        while w < tableLog.wrapping_add(1) {
            let symbolCount: c_int = *wksp_rankVal.add(w as usize) as c_int;
            let length: c_int = (1i32 << w) >> 1;
            let mut uStart: c_int = rankStart;
            let nbBits: BYTE = tableLog.wrapping_add(1).wrapping_sub(w) as BYTE;
            let mut s: c_int;
            let mut u: c_int;
            match length {
                1 => {
                    s = 0;
                    while s < symbolCount {
                        let mut D: HUF_DEltX1 = HUF_DEltX1 { nbBits: 0, byte: 0 };
                        D.byte = *wksp_symbols.add((symbol + s) as usize);
                        D.nbBits = nbBits;
                        *dt.offset(uStart as isize) = D;
                        uStart += 1;
                        s += 1;
                    }
                }
                2 => {
                    s = 0;
                    while s < symbolCount {
                        let mut D: HUF_DEltX1 = HUF_DEltX1 { nbBits: 0, byte: 0 };
                        D.byte = *wksp_symbols.add((symbol + s) as usize);
                        D.nbBits = nbBits;
                        *dt.offset((uStart + 0) as isize) = D;
                        *dt.offset((uStart + 1) as isize) = D;
                        uStart += 2;
                        s += 1;
                    }
                }
                4 => {
                    s = 0;
                    while s < symbolCount {
                        let D4: U64 =
                            HUF_DEltX1_set4(*wksp_symbols.add((symbol + s) as usize), nbBits);
                        MEM_write64(dt.offset(uStart as isize) as *mut u8, D4);
                        uStart += 4;
                        s += 1;
                    }
                }
                8 => {
                    s = 0;
                    while s < symbolCount {
                        let D4: U64 =
                            HUF_DEltX1_set4(*wksp_symbols.add((symbol + s) as usize), nbBits);
                        MEM_write64(dt.offset(uStart as isize) as *mut u8, D4);
                        MEM_write64(dt.offset((uStart + 4) as isize) as *mut u8, D4);
                        uStart += 8;
                        s += 1;
                    }
                }
                _ => {
                    s = 0;
                    while s < symbolCount {
                        let D4: U64 =
                            HUF_DEltX1_set4(*wksp_symbols.add((symbol + s) as usize), nbBits);
                        u = 0;
                        while u < length {
                            MEM_write64(dt.offset((uStart + u + 0) as isize) as *mut u8, D4);
                            MEM_write64(dt.offset((uStart + u + 4) as isize) as *mut u8, D4);
                            MEM_write64(dt.offset((uStart + u + 8) as isize) as *mut u8, D4);
                            MEM_write64(dt.offset((uStart + u + 12) as isize) as *mut u8, D4);
                            u += 16;
                        }
                        uStart += length;
                        s += 1;
                    }
                }
            }
            symbol += symbolCount;
            rankStart += symbolCount * length;
            w = w.wrapping_add(1);
        }
    }
    iSize
}

/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decodeSymbolX1(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> BYTE {
    let val: usize = BIT_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

/* HUF_DECODE_SYMBOLX1_0 / _1 / _2 are expanded manually at each use site.
 *   _0 : unconditional
 *   _1 : if (MEM_64bits() || (HUF_TABLELOG_MAX<=12))
 *   _2 : if (MEM_64bits())
 */

/* HINT_INLINE */
pub unsafe fn HUF_decodeStreamX1(
    p_arg: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> usize {
    let mut p: *mut BYTE = p_arg;
    let pStart: *mut BYTE = p;

    /* up to 4 symbols at a time */
    if (pEnd as isize).wrapping_sub(p as isize) > 3 {
        while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
            & (p < pEnd.wrapping_sub(3))
        {
            /* HUF_DECODE_SYMBOLX1_2(p, bitDPtr); */
            if MEM_64bits() != 0 {
                *p = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
                p = p.wrapping_add(1);
            }
            /* HUF_DECODE_SYMBOLX1_1(p, bitDPtr); */
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                *p = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
                p = p.wrapping_add(1);
            }
            /* HUF_DECODE_SYMBOLX1_2(p, bitDPtr); */
            if MEM_64bits() != 0 {
                *p = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
                p = p.wrapping_add(1);
            }
            /* HUF_DECODE_SYMBOLX1_0(p, bitDPtr); */
            *p = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }

    /* [0-3] symbols remaining */
    if MEM_32bits() != 0 {
        while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) & (p < pEnd) {
            /* HUF_DECODE_SYMBOLX1_0(p, bitDPtr); */
            *p = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
            p = p.wrapping_add(1);
        }
    }

    /* no more data to retrieve from bitstream, no need to reload */
    while p < pEnd {
        /* HUF_DECODE_SYMBOLX1_0(p, bitDPtr); */
        *p = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
        p = p.wrapping_add(1);
    }

    (pEnd as isize).wrapping_sub(pStart as isize) as usize
}

/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decompress1X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(op, dstSize as isize);
    let dtPtr: *const c_void = DTable.add(1) as *const c_void;
    let dt: *const HUF_DEltX1 = dtPtr as *const HUF_DEltX1;
    let mut bitD: BIT_DStream_t = BIT_DStream_t::default();
    let dtd: DTableDesc = HUF_getDTableDesc(DTable);
    let dtLog: U32 = dtd.tableLog as U32;

    {
        let err_code = BIT_initDStream(addr_of_mut!(bitD), cSrc as *const BYTE, cSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    HUF_decodeStreamX1(op, addr_of_mut!(bitD), oend, dt, dtLog);

    if BIT_endOfDStream(addr_of!(bitD)) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

/* HUF_decompress4X1_usingDTable_internal_body():
 * Conditions :
 * @dstSize >= 6
 */
/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decompress4X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    /* Check */
    if cSrcSize < 10 {
        /* strict minimum : jump table + 1 byte per stream */
        return ERROR(ZSTD_error_corruption_detected);
    }
    if dstSize < 6 {
        /* stream 4-split doesn't work */
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let olimit: *mut BYTE = oend.wrapping_sub(3);
        let dtPtr: *const c_void = DTable.add(1) as *const c_void;
        let dt: *const HUF_DEltX1 = dtPtr as *const HUF_DEltX1;

        /* Init */
        let mut bitD1: BIT_DStream_t = BIT_DStream_t::default();
        let mut bitD2: BIT_DStream_t = BIT_DStream_t::default();
        let mut bitD3: BIT_DStream_t = BIT_DStream_t::default();
        let mut bitD4: BIT_DStream_t = BIT_DStream_t::default();
        let length1: usize = MEM_readLE16(istart) as usize;
        let length2: usize = MEM_readLE16(istart.add(2)) as usize;
        let length3: usize = MEM_readLE16(istart.add(4)) as usize;
        let length4: usize = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1: *const BYTE = istart.wrapping_add(6); /* jumpTable */
        let istart2: *const BYTE = istart1.wrapping_add(length1);
        let istart3: *const BYTE = istart2.wrapping_add(length2);
        let istart4: *const BYTE = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2: *mut BYTE = ostart.wrapping_add(segmentSize);
        let opStart3: *mut BYTE = opStart2.wrapping_add(segmentSize);
        let opStart4: *mut BYTE = opStart3.wrapping_add(segmentSize);
        let mut op1: *mut BYTE = ostart;
        let mut op2: *mut BYTE = opStart2;
        let mut op3: *mut BYTE = opStart3;
        let mut op4: *mut BYTE = opStart4;
        let dtd: DTableDesc = HUF_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;
        let mut endSignal: U32 = 1;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        if opStart4 > oend {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD1), istart1, length1);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD2), istart2, length2);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD3), istart3, length3);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD4), istart4, length4);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }

        /* up to 16 symbols per loop (4 symbols per stream) in 64-bit mode */
        if ((oend as isize).wrapping_sub(op4 as isize) as usize) >= core::mem::size_of::<usize>() {
            while (endSignal & ((op4 < olimit) as U32)) != 0 {
                /* HUF_DECODE_SYMBOLX1_2(op1..op4) */
                if MEM_64bits() != 0 {
                    *op1 = HUF_decodeSymbolX1(addr_of_mut!(bitD1), dt, dtLog);
                    op1 = op1.wrapping_add(1);
                }
                if MEM_64bits() != 0 {
                    *op2 = HUF_decodeSymbolX1(addr_of_mut!(bitD2), dt, dtLog);
                    op2 = op2.wrapping_add(1);
                }
                if MEM_64bits() != 0 {
                    *op3 = HUF_decodeSymbolX1(addr_of_mut!(bitD3), dt, dtLog);
                    op3 = op3.wrapping_add(1);
                }
                if MEM_64bits() != 0 {
                    *op4 = HUF_decodeSymbolX1(addr_of_mut!(bitD4), dt, dtLog);
                    op4 = op4.wrapping_add(1);
                }
                /* HUF_DECODE_SYMBOLX1_1(op1..op4) */
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    *op1 = HUF_decodeSymbolX1(addr_of_mut!(bitD1), dt, dtLog);
                    op1 = op1.wrapping_add(1);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    *op2 = HUF_decodeSymbolX1(addr_of_mut!(bitD2), dt, dtLog);
                    op2 = op2.wrapping_add(1);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    *op3 = HUF_decodeSymbolX1(addr_of_mut!(bitD3), dt, dtLog);
                    op3 = op3.wrapping_add(1);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    *op4 = HUF_decodeSymbolX1(addr_of_mut!(bitD4), dt, dtLog);
                    op4 = op4.wrapping_add(1);
                }
                /* HUF_DECODE_SYMBOLX1_2(op1..op4) */
                if MEM_64bits() != 0 {
                    *op1 = HUF_decodeSymbolX1(addr_of_mut!(bitD1), dt, dtLog);
                    op1 = op1.wrapping_add(1);
                }
                if MEM_64bits() != 0 {
                    *op2 = HUF_decodeSymbolX1(addr_of_mut!(bitD2), dt, dtLog);
                    op2 = op2.wrapping_add(1);
                }
                if MEM_64bits() != 0 {
                    *op3 = HUF_decodeSymbolX1(addr_of_mut!(bitD3), dt, dtLog);
                    op3 = op3.wrapping_add(1);
                }
                if MEM_64bits() != 0 {
                    *op4 = HUF_decodeSymbolX1(addr_of_mut!(bitD4), dt, dtLog);
                    op4 = op4.wrapping_add(1);
                }
                /* HUF_DECODE_SYMBOLX1_0(op1..op4) */
                *op1 = HUF_decodeSymbolX1(addr_of_mut!(bitD1), dt, dtLog);
                op1 = op1.wrapping_add(1);
                *op2 = HUF_decodeSymbolX1(addr_of_mut!(bitD2), dt, dtLog);
                op2 = op2.wrapping_add(1);
                *op3 = HUF_decodeSymbolX1(addr_of_mut!(bitD3), dt, dtLog);
                op3 = op3.wrapping_add(1);
                *op4 = HUF_decodeSymbolX1(addr_of_mut!(bitD4), dt, dtLog);
                op4 = op4.wrapping_add(1);

                endSignal &= (BIT_reloadDStreamFast(addr_of_mut!(bitD1)) == BIT_DStream_unfinished)
                    as U32;
                endSignal &= (BIT_reloadDStreamFast(addr_of_mut!(bitD2)) == BIT_DStream_unfinished)
                    as U32;
                endSignal &= (BIT_reloadDStreamFast(addr_of_mut!(bitD3)) == BIT_DStream_unfinished)
                    as U32;
                endSignal &= (BIT_reloadDStreamFast(addr_of_mut!(bitD4)) == BIT_DStream_unfinished)
                    as U32;
            }
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX1(op1, addr_of_mut!(bitD1), opStart2, dt, dtLog);
        HUF_decodeStreamX1(op2, addr_of_mut!(bitD2), opStart3, dt, dtLog);
        HUF_decodeStreamX1(op3, addr_of_mut!(bitD3), opStart4, dt, dtLog);
        HUF_decodeStreamX1(op4, addr_of_mut!(bitD4), oend, dt, dtLog);

        /* check */
        {
            let endCheck: U32 = BIT_endOfDStream(addr_of!(bitD1))
                & BIT_endOfDStream(addr_of!(bitD2))
                & BIT_endOfDStream(addr_of!(bitD3))
                & BIT_endOfDStream(addr_of!(bitD4));
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
        return dstSize;
    }
}

/* HUF_NEED_BMI2_FUNCTION == 0 : HUF_decompress4X1_usingDTable_internal_bmi2 is
 * not compiled in. */

pub unsafe fn HUF_decompress4X1_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    HUF_decompress4X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 :
 * HUF_decompress4X1_usingDTable_internal_fast_asm_loop is not declared. */

pub unsafe fn HUF_decompress4X1_usingDTable_internal_fast_c_loop(
    args: *mut HUF_DecompressFastArgs,
) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable: *const U16 = (*args).dt as *const U16;
    let oend: *mut BYTE = (*args).oend;
    let ilowest: *const BYTE = (*args).ilowest;

    /* Copy the arguments to local variables */
    ZSTD_memcpy(
        addr_of_mut!(bits) as *mut u8,
        addr_of!((*args).bits) as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!(ip) as *mut u8,
        addr_of!((*args).ip) as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!(op) as *mut u8,
        addr_of!((*args).op) as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );

    'fastloop: loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            /* Each iteration produces 5 output symbols per stream */
            let oiters: usize = ((oend as isize).wrapping_sub(op[3] as isize) as usize) / 5;
            /* Each iteration consumes up to 11 bits * 5 = 55 bits < 7 bytes per stream. */
            let iiters: usize = ((ip[0] as isize).wrapping_sub(ilowest as isize) as usize) / 7;
            /* We can safely run iters iterations before running bounds checks */
            let iters: usize = MIN(oiters, iiters);
            let symbols: usize = iters * 5;

            olimit = op[3].wrapping_add(symbols);

            /* Exit fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break 'fastloop;
            }

            /* Exit the decoding loop if any input pointer has crossed the
             * previous one. This indicates corruption. (goto _out) */
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'fastloop;
                }
                stream += 1;
            }
        }

        /* Manually unroll the loop because compilers don't consistently
         * unroll the inner loops, which destroys performance.
         */
        loop {
            /* HUF_4X_FOR_EACH_STREAM_WITH_VAR(HUF_4X1_DECODE_SYMBOL, 0); */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[0] <<= (entry & 0x3F) as u32;
                *op[0].add(0) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[1] <<= (entry & 0x3F) as u32;
                *op[1].add(0) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[2] <<= (entry & 0x3F) as u32;
                *op[2].add(0) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[3] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[3] <<= (entry & 0x3F) as u32;
                *op[3].add(0) = ((entry >> 8) & 0xFF) as BYTE;
            }
            /* HUF_4X_FOR_EACH_STREAM_WITH_VAR(HUF_4X1_DECODE_SYMBOL, 1); */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[0] <<= (entry & 0x3F) as u32;
                *op[0].add(1) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[1] <<= (entry & 0x3F) as u32;
                *op[1].add(1) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[2] <<= (entry & 0x3F) as u32;
                *op[2].add(1) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[3] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[3] <<= (entry & 0x3F) as u32;
                *op[3].add(1) = ((entry >> 8) & 0xFF) as BYTE;
            }
            /* HUF_4X_FOR_EACH_STREAM_WITH_VAR(HUF_4X1_DECODE_SYMBOL, 2); */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[0] <<= (entry & 0x3F) as u32;
                *op[0].add(2) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[1] <<= (entry & 0x3F) as u32;
                *op[1].add(2) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[2] <<= (entry & 0x3F) as u32;
                *op[2].add(2) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[3] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[3] <<= (entry & 0x3F) as u32;
                *op[3].add(2) = ((entry >> 8) & 0xFF) as BYTE;
            }
            /* HUF_4X_FOR_EACH_STREAM_WITH_VAR(HUF_4X1_DECODE_SYMBOL, 3); */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[0] <<= (entry & 0x3F) as u32;
                *op[0].add(3) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[1] <<= (entry & 0x3F) as u32;
                *op[1].add(3) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[2] <<= (entry & 0x3F) as u32;
                *op[2].add(3) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[3] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[3] <<= (entry & 0x3F) as u32;
                *op[3].add(3) = ((entry >> 8) & 0xFF) as BYTE;
            }
            /* HUF_4X_FOR_EACH_STREAM_WITH_VAR(HUF_4X1_DECODE_SYMBOL, 4); */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[0] <<= (entry & 0x3F) as u32;
                *op[0].add(4) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[1] <<= (entry & 0x3F) as u32;
                *op[1].add(4) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[2] <<= (entry & 0x3F) as u32;
                *op[2].add(4) = ((entry >> 8) & 0xFF) as BYTE;
            }
            {
                let index: c_int = (bits[3] >> 53) as c_int;
                let entry: c_int = *dtable.add(index as usize) as c_int;
                bits[3] <<= (entry & 0x3F) as u32;
                *op[3].add(4) = ((entry >> 8) & 0xFF) as BYTE;
            }

            /* HUF_4X_FOR_EACH_STREAM(HUF_4X1_RELOAD_STREAM); */
            {
                let ctz: c_int = ZSTD_countTrailingZeros64(bits[0]) as c_int;
                let nbBits: c_int = ctz & 7;
                let nbBytes: c_int = ctz >> 3;
                op[0] = op[0].wrapping_add(5);
                ip[0] = ip[0].wrapping_sub(nbBytes as usize);
                bits[0] = MEM_read64(ip[0]) | 1;
                bits[0] <<= nbBits as u32;
            }
            {
                let ctz: c_int = ZSTD_countTrailingZeros64(bits[1]) as c_int;
                let nbBits: c_int = ctz & 7;
                let nbBytes: c_int = ctz >> 3;
                op[1] = op[1].wrapping_add(5);
                ip[1] = ip[1].wrapping_sub(nbBytes as usize);
                bits[1] = MEM_read64(ip[1]) | 1;
                bits[1] <<= nbBits as u32;
            }
            {
                let ctz: c_int = ZSTD_countTrailingZeros64(bits[2]) as c_int;
                let nbBits: c_int = ctz & 7;
                let nbBytes: c_int = ctz >> 3;
                op[2] = op[2].wrapping_add(5);
                ip[2] = ip[2].wrapping_sub(nbBytes as usize);
                bits[2] = MEM_read64(ip[2]) | 1;
                bits[2] <<= nbBits as u32;
            }
            {
                let ctz: c_int = ZSTD_countTrailingZeros64(bits[3]) as c_int;
                let nbBits: c_int = ctz & 7;
                let nbBytes: c_int = ctz >> 3;
                op[3] = op[3].wrapping_add(5);
                ip[3] = ip[3].wrapping_sub(nbBytes as usize);
                bits[3] = MEM_read64(ip[3]) | 1;
                bits[3] <<= nbBits as u32;
            }

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* _out: */

    /* Save the final values of each of the state variables back to args. */
    ZSTD_memcpy(
        addr_of_mut!((*args).bits) as *mut u8,
        addr_of!(bits) as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!((*args).ip) as *mut u8,
        addr_of!(ip) as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!((*args).op) as *mut u8,
        addr_of!(op) as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );
}

/**
 * @returns @p dstSize on success (>= 6)
 *          0 if the fallback implementation should be used
 *          An error if an error occurred
 */
pub unsafe fn HUF_decompress4X1_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt: *const c_void = DTable.add(1) as *const c_void;
    let ilowest: *const BYTE = cSrc as *const BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);
    let mut args: HUF_DecompressFastArgs = core::mem::zeroed();
    {
        let ret: usize =
            HUF_DecompressFastArgs_init(addr_of_mut!(args), dst, dstSize, cSrc, cSrcSize, DTable);
        {
            let err_code = ret;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(addr_of_mut!(args));

    /* finish bit streams one by one. */
    {
        let segmentSize: usize = (dstSize + 3) / 4;
        let mut segmentEnd: *mut BYTE = dst as *mut BYTE;
        let mut i: c_int;
        i = 0;
        while i < 4 {
            let mut bit: BIT_DStream_t = BIT_DStream_t::default();
            if segmentSize <= ((oend as isize).wrapping_sub(segmentEnd as isize) as usize) {
                segmentEnd = segmentEnd.wrapping_add(segmentSize);
            } else {
                segmentEnd = oend;
            }
            {
                let err_code = HUF_initRemainingDStream(
                    addr_of_mut!(bit),
                    addr_of!(args),
                    i,
                    segmentEnd,
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            /* Decompress and validate that we've produced exactly the expected length. */
            args.op[i as usize] = args.op[i as usize].wrapping_add(HUF_decodeStreamX1(
                args.op[i as usize],
                addr_of_mut!(bit),
                segmentEnd,
                dt as *const HUF_DEltX1,
                HUF_DECODER_FAST_TABLELOG,
            ));
            if args.op[i as usize] != segmentEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            i += 1;
        }
    }

    /* decoded size */
    dstSize
}

/* HUF_DGEN(HUF_decompress1X1_usingDTable_internal) -- DYNAMIC_BMI2 == 0 branch */
pub unsafe fn HUF_decompress1X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let _ = flags;
    HUF_decompress1X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

pub unsafe fn HUF_decompress4X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let fallbackFn: HUF_DecompressUsingDTableFn = HUF_decompress4X1_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn = HUF_decompress4X1_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 :
     * no bmi2 fallback selection, no asm loop selection. */

    if HUF_ENABLE_FAST_DECODE != 0 && (flags & HUF_flags_disableFast) == 0 {
        let ret: usize = HUF_decompress4X1_usingDTable_internal_fast(
            dst, dstSize, cSrc, cSrcSize, DTable, loopFn,
        );
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

pub unsafe fn HUF_decompress4X1_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X1_usingDTable_internal(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        dctx,
        flags,
    )
}

/* #endif  HUF_FORCE_DECOMPRESS_X2 */

/* #ifndef HUF_FORCE_DECOMPRESS_X1 */

/* *************************/
/* double-symbols decoding */
/* *************************/

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HUF_DEltX2 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
}

pub type rankValCol_t = [U32; HUF_TABLELOG_MAX as usize + 1];
pub type rankVal_t = [rankValCol_t; HUF_TABLELOG_MAX as usize];

/**
 * Constructs a HUF_DEltX2 in a U32.
 */
pub fn HUF_buildDEltX2U32(symbol: U32, nbBits: U32, baseSeq: U32, level: c_int) -> U32 {
    let seq: U32;
    if MEM_isLittleEndian() != 0 {
        seq = if level == 1 {
            symbol
        } else {
            baseSeq.wrapping_add(symbol << 8)
        };
        seq.wrapping_add(nbBits << 16)
            .wrapping_add((level as U32) << 24)
    } else {
        seq = if level == 1 {
            symbol << 8
        } else {
            (baseSeq << 8).wrapping_add(symbol)
        };
        (seq << 16)
            .wrapping_add(nbBits << 8)
            .wrapping_add(level as U32)
    }
}

/**
 * Constructs a HUF_DEltX2.
 */
pub unsafe fn HUF_buildDEltX2(symbol: U32, nbBits: U32, baseSeq: U32, level: c_int) -> HUF_DEltX2 {
    let mut DElt: HUF_DEltX2 = HUF_DEltX2 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let val: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq, level);
    ZSTD_memcpy(
        addr_of_mut!(DElt) as *mut u8,
        addr_of!(val) as *const u8,
        core::mem::size_of::<U32>(),
    );
    DElt
}

/**
 * Constructs 2 HUF_DEltX2s and packs them into a U64.
 */
pub fn HUF_buildDEltX2U64(symbol: U32, nbBits: U32, baseSeq: U16, level: c_int) -> U64 {
    let DElt: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq as U32, level);
    (DElt as U64).wrapping_add((DElt as U64) << 32)
}

/**
 * Fills the DTable rank with all the symbols from [begin, end) that are each
 * nbBits long.
 */
pub unsafe fn HUF_fillDTableX2ForWeight(
    DTableRank_arg: *mut HUF_DEltX2,
    begin: *const sortedSymbol_t,
    end: *const sortedSymbol_t,
    nbBits: U32,
    tableLog: U32,
    baseSeq: U16,
    level: c_int,
) {
    let mut DTableRank: *mut HUF_DEltX2 = DTableRank_arg;
    let length: U32 = 1u32 << ((tableLog.wrapping_sub(nbBits)) & 0x1F /* quiet static-analyzer */);
    let mut ptr: *const sortedSymbol_t;
    match length {
        1 => {
            ptr = begin;
            while ptr != end {
                let DElt: HUF_DEltX2 =
                    HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level);
                *DTableRank = DElt;
                DTableRank = DTableRank.wrapping_add(1);
                ptr = ptr.wrapping_add(1);
            }
        }
        2 => {
            ptr = begin;
            while ptr != end {
                let DElt: HUF_DEltX2 =
                    HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level);
                *DTableRank.wrapping_add(0) = DElt;
                *DTableRank.wrapping_add(1) = DElt;
                DTableRank = DTableRank.wrapping_add(2);
                ptr = ptr.wrapping_add(1);
            }
        }
        4 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 = HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                ZSTD_memcpy(
                    DTableRank.wrapping_add(0) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.wrapping_add(2) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                DTableRank = DTableRank.wrapping_add(4);
                ptr = ptr.wrapping_add(1);
            }
        }
        8 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 = HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                ZSTD_memcpy(
                    DTableRank.wrapping_add(0) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.wrapping_add(2) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.wrapping_add(4) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.wrapping_add(6) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                DTableRank = DTableRank.wrapping_add(8);
                ptr = ptr.wrapping_add(1);
            }
        }
        _ => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 = HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                let DTableRankEnd: *mut HUF_DEltX2 = DTableRank.wrapping_add(length as usize);
                while DTableRank != DTableRankEnd {
                    ZSTD_memcpy(
                        DTableRank.wrapping_add(0) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.wrapping_add(2) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.wrapping_add(4) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.wrapping_add(6) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    DTableRank = DTableRank.wrapping_add(8);
                }
                ptr = ptr.wrapping_add(1);
            }
        }
    }
}

/* HUF_fillDTableX2Level2() :
 * `rankValOrigin` must be a table of at least (HUF_TABLELOG_MAX + 1) U32 */
pub unsafe fn HUF_fillDTableX2Level2(
    DTable: *mut HUF_DEltX2,
    targetLog: U32,
    consumedBits: U32,
    rankVal: *const U32,
    minWeight: c_int,
    maxWeight1: c_int,
    sortedSymbols: *const sortedSymbol_t,
    rankStart: *const U32,
    nbBitsBaseline: U32,
    baseSeq: U16,
) {
    /* Fill skipped values (all positions up to rankVal[minWeight]). */
    if minWeight > 1 {
        let length: U32 =
            1u32 << ((targetLog.wrapping_sub(consumedBits)) & 0x1F /* quiet static-analyzer */);
        let DEltX2: U64 = HUF_buildDEltX2U64(
            baseSeq as U32,
            consumedBits,
            /* baseSeq */ 0,
            /* level */ 1,
        );
        let skipSize: c_int = *rankVal.add(minWeight as usize) as c_int;
        match length {
            2 => {
                ZSTD_memcpy(
                    DTable as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
            }
            4 => {
                ZSTD_memcpy(
                    DTable.wrapping_add(0) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTable.wrapping_add(2) as *mut u8,
                    addr_of!(DEltX2) as *const u8,
                    core::mem::size_of::<U64>(),
                );
            }
            _ => {
                let mut i: c_int;
                i = 0;
                while i < skipSize {
                    ZSTD_memcpy(
                        DTable.offset((i + 0) as isize) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset((i + 2) as isize) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset((i + 4) as isize) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset((i + 6) as isize) as *mut u8,
                        addr_of!(DEltX2) as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    i += 8;
                }
            }
        }
    }

    /* Fill each of the second level symbols by weight. */
    {
        let mut w: c_int;
        w = minWeight;
        while w < maxWeight1 {
            let begin: c_int = *rankStart.add(w as usize) as c_int;
            let end: c_int = *rankStart.add((w + 1) as usize) as c_int;
            let nbBits: U32 = nbBitsBaseline.wrapping_sub(w as U32);
            let totalBits: U32 = nbBits.wrapping_add(consumedBits);
            HUF_fillDTableX2ForWeight(
                DTable.offset(*rankVal.add(w as usize) as isize),
                sortedSymbols.offset(begin as isize),
                sortedSymbols.offset(end as isize),
                totalBits,
                targetLog,
                baseSeq,
                /* level */ 2,
            );
            w += 1;
        }
    }
}

pub unsafe fn HUF_fillDTableX2(
    DTable: *mut HUF_DEltX2,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    rankStart: *const U32,
    rankValOrigin: *mut rankValCol_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let rankVal: *mut U32 = rankValOrigin as *mut U32; /* rankValOrigin[0] */
    /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut w: c_int;
    let wEnd: c_int = (maxWeight as c_int) + 1;

    /* Fill DTable in order of weight. */
    w = 1;
    while w < wEnd {
        let begin: c_int = *rankStart.add(w as usize) as c_int;
        let end: c_int = *rankStart.add((w + 1) as usize) as c_int;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(w as U32);

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* Enough room for a second symbol. */
            let mut start: c_int = *rankVal.add(w as usize) as c_int;
            let length: U32 =
                1u32 << ((targetLog.wrapping_sub(nbBits)) & 0x1F /* quiet static-analyzer */);
            let mut minWeight: c_int = (nbBits as c_int) + scaleLog;
            let mut s: c_int;
            if minWeight < 1 {
                minWeight = 1;
            }
            /* Fill the DTable for every symbol of weight w. */
            s = begin;
            while s != end {
                HUF_fillDTableX2Level2(
                    DTable.offset(start as isize),
                    targetLog,
                    nbBits,
                    rankValOrigin.add(nbBits as usize) as *const U32,
                    minWeight,
                    wEnd,
                    sortedList,
                    rankStart,
                    nbBitsBaseline,
                    (*sortedList.offset(s as isize)).symbol as U16,
                );
                start += length as c_int;
                s += 1;
            }
        } else {
            /* Only a single symbol. */
            HUF_fillDTableX2ForWeight(
                DTable.offset(*rankVal.add(w as usize) as isize),
                sortedList.offset(begin as isize),
                sortedList.offset(end as isize),
                nbBits,
                targetLog,
                /* baseSeq */ 0,
                /* level */ 1,
            );
        }
        w += 1;
    }
}

#[repr(C)]
pub struct HUF_ReadDTableX2_Workspace {
    pub rankVal: rankVal_t,
    pub rankStats: [U32; HUF_TABLELOG_MAX as usize + 1],
    pub rankStart0: [U32; HUF_TABLELOG_MAX as usize + 3],
    pub sortedSymbol: [sortedSymbol_t; HUF_SYMBOLVALUE_MAX as usize + 1],
    pub weightList: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1],
    pub calleeWksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX2_wksp(
    DTable: *mut HUF_DTable,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut nbSymbols: U32 = 0;
    let mut dtd: DTableDesc = HUF_getDTableDesc(DTable);
    let mut maxTableLog: U32 = dtd.maxTableLog as U32;
    let iSize: usize;
    let dtPtr: *mut c_void = DTable.add(1) as *mut c_void; /* force compiler to avoid strict-aliasing */
    let dt: *mut HUF_DEltX2 = dtPtr as *mut HUF_DEltX2;
    let rankStart: *mut U32;

    let wksp: *mut HUF_ReadDTableX2_Workspace = workSpace as *mut HUF_ReadDTableX2_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX2_Workspace>() > wkspSize {
        return ERROR(ZSTD_error_GENERIC);
    }

    let wksp_rankVal: *mut rankValCol_t = addr_of_mut!((*wksp).rankVal) as *mut rankValCol_t;
    let wksp_rankStats: *mut U32 = addr_of_mut!((*wksp).rankStats) as *mut U32;
    let wksp_rankStart0: *mut U32 = addr_of_mut!((*wksp).rankStart0) as *mut U32;
    let wksp_sortedSymbol: *mut sortedSymbol_t =
        addr_of_mut!((*wksp).sortedSymbol) as *mut sortedSymbol_t;
    let wksp_weightList: *mut BYTE = addr_of_mut!((*wksp).weightList) as *mut BYTE;
    let wksp_calleeWksp: *mut U32 = addr_of_mut!((*wksp).calleeWksp) as *mut U32;

    rankStart = wksp_rankStart0.add(1);
    ZSTD_memset(
        wksp_rankStats as *mut u8,
        0,
        core::mem::size_of::<[U32; HUF_TABLELOG_MAX as usize + 1]>(),
    );
    ZSTD_memset(
        wksp_rankStart0 as *mut u8,
        0,
        core::mem::size_of::<[U32; HUF_TABLELOG_MAX as usize + 3]>(),
    );

    if maxTableLog > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    /* ZSTD_memset(weightList, 0, sizeof(weightList)); */ /* is not necessary */

    iSize = HUF_readStats_wksp(
        wksp_weightList,
        HUF_SYMBOLVALUE_MAX as usize + 1,
        wksp_rankStats,
        addr_of_mut!(nbSymbols),
        addr_of_mut!(tableLog),
        src,
        srcSize,
        wksp_calleeWksp as *mut c_void,
        core::mem::size_of::<[U32; HUF_READ_STATS_WORKSPACE_SIZE_U32]>(),
        flags,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > maxTableLog {
        return ERROR(ZSTD_error_tableLog_tooLarge); /* DTable can't fit code depth */
    }
    if tableLog <= HUF_DECODER_FAST_TABLELOG && maxTableLog > HUF_DECODER_FAST_TABLELOG {
        maxTableLog = HUF_DECODER_FAST_TABLELOG;
    }

    /* find maxWeight */
    maxW = tableLog;
    while *wksp_rankStats.add(maxW as usize) == 0 {
        maxW = maxW.wrapping_sub(1);
    } /* necessarily finds a solution before 0 */

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w < maxW.wrapping_add(1) {
            let curr: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add(*wksp_rankStats.add(w as usize));
            *rankStart.add(w as usize) = curr;
            w = w.wrapping_add(1);
        }
        *rankStart.add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list*/
        *rankStart.add(maxW.wrapping_add(1) as usize) = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32;
        s = 0;
        while s < nbSymbols {
            let w: U32 = *wksp_weightList.add(s as usize) as U32;
            let r: U32 = *rankStart.add(w as usize);
            *rankStart.add(w as usize) = r.wrapping_add(1);
            (*wksp_sortedSymbol.add(r as usize)).symbol = s as BYTE;
            s = s.wrapping_add(1);
        }
        *rankStart.add(0) = 0; /* forget 0w symbols; this is beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let rankVal0: *mut U32 = wksp_rankVal as *mut U32; /* wksp->rankVal[0] */
        {
            /* tableLog <= maxTableLog */
            let rescale: c_int = (maxTableLog.wrapping_sub(tableLog) as c_int) - 1;
            let mut nextRankVal: U32 = 0;
            let mut w: U32;
            w = 1;
            while w < maxW.wrapping_add(1) {
                let curr: U32 = nextRankVal;
                nextRankVal = nextRankVal.wrapping_add(
                    (*wksp_rankStats.add(w as usize)).wrapping_shl(w.wrapping_add(rescale as U32)),
                );
                *rankVal0.add(w as usize) = curr;
                w = w.wrapping_add(1);
            }
        }
        {
            let minBits: U32 = tableLog.wrapping_add(1).wrapping_sub(maxW);
            let mut consumed: U32;
            consumed = minBits;
            while consumed < maxTableLog.wrapping_sub(minBits).wrapping_add(1) {
                let rankValPtr: *mut U32 = wksp_rankVal.add(consumed as usize) as *mut U32;
                let mut w: U32;
                w = 1;
                while w < maxW.wrapping_add(1) {
                    *rankValPtr.add(w as usize) =
                        (*rankVal0.add(w as usize)).wrapping_shr(consumed);
                    w = w.wrapping_add(1);
                }
                consumed = consumed.wrapping_add(1);
            }
        }
    }

    HUF_fillDTableX2(
        dt,
        maxTableLog,
        wksp_sortedSymbol,
        wksp_rankStart0,
        wksp_rankVal,
        maxW,
        tableLog.wrapping_add(1),
    );

    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1;
    ZSTD_memcpy(
        DTable as *mut u8,
        addr_of!(dtd) as *const u8,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decodeSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val: usize = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(
        op as *mut u8,
        addr_of!((*dt.add(val)).sequence) as *const u8,
        2,
    );
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decodeLastSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val: usize = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    ZSTD_memcpy(
        op as *mut u8,
        addr_of!((*dt.add(val)).sequence) as *const u8,
        1,
    );
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed < (core::mem::size_of::<BitContainerType>() * 8) as u32 {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed > (core::mem::size_of::<BitContainerType>() * 8) as u32 {
                /* ugly hack; works only because it's the last symbol. */
                (*DStream).bitsConsumed =
                    (core::mem::size_of::<BitContainerType>() * 8) as u32;
            }
        }
    }
    1
}

/* HUF_DECODE_SYMBOLX2_0 / _1 / _2 are expanded manually at each use site. */

/* HINT_INLINE */
pub unsafe fn HUF_decodeStreamX2(
    p_arg: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> usize {
    let mut p: *mut BYTE = p_arg;
    let pStart: *mut BYTE = p;

    /* up to 8 symbols at a time */
    if (((pEnd as isize).wrapping_sub(p as isize)) as usize) >= core::mem::size_of::<BitContainerType>()
    {
        if dtLog <= 11 && MEM_64bits() != 0 {
            /* up to 10 symbols at a time */
            while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
                & (p < pEnd.wrapping_sub(9))
            {
                /* HUF_DECODE_SYMBOLX2_0 x5 */
                p = p.wrapping_add(
                    HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                );
                p = p.wrapping_add(
                    HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                );
                p = p.wrapping_add(
                    HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                );
                p = p.wrapping_add(
                    HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                );
                p = p.wrapping_add(
                    HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                );
            }
        } else {
            /* up to 8 symbols at a time */
            while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
                & (p < pEnd
                    .wrapping_sub(core::mem::size_of::<BitContainerType>() - 1))
            {
                /* HUF_DECODE_SYMBOLX2_2 */
                if MEM_64bits() != 0 {
                    p = p.wrapping_add(
                        HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                    );
                }
                /* HUF_DECODE_SYMBOLX2_1 */
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    p = p.wrapping_add(
                        HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                    );
                }
                /* HUF_DECODE_SYMBOLX2_2 */
                if MEM_64bits() != 0 {
                    p = p.wrapping_add(
                        HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                    );
                }
                /* HUF_DECODE_SYMBOLX2_0 */
                p = p.wrapping_add(
                    HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
                );
            }
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }

    /* closer to end : up to 2 symbols at a time */
    if (((pEnd as isize).wrapping_sub(p as isize)) as usize) >= 2 {
        while (BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished)
            & (p <= pEnd.wrapping_sub(2))
        {
            p = p.wrapping_add(
                HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }

        while p <= pEnd.wrapping_sub(2) {
            /* no need to reload : reached the end of DStream */
            p = p.wrapping_add(
                HUF_decodeSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
            );
        }
    }

    if p < pEnd {
        p = p.wrapping_add(
            HUF_decodeLastSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize,
        );
    }

    (p as isize).wrapping_sub(pStart as isize) as usize
}

/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decompress1X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let mut bitD: BIT_DStream_t = BIT_DStream_t::default();

    /* Init */
    {
        let err_code = BIT_initDStream(addr_of_mut!(bitD), cSrc as *const BYTE, cSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    /* decode */
    {
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(ostart, dstSize as isize);
        let dtPtr: *const c_void = DTable.add(1) as *const c_void; /* force compiler to not use strict-aliasing */
        let dt: *const HUF_DEltX2 = dtPtr as *const HUF_DEltX2;
        let dtd: DTableDesc = HUF_getDTableDesc(DTable);
        HUF_decodeStreamX2(ostart, addr_of_mut!(bitD), oend, dt, dtd.tableLog as U32);
    }

    /* check */
    if BIT_endOfDStream(addr_of!(bitD)) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
    dstSize
}

/* HUF_decompress4X2_usingDTable_internal_body():
 * Conditions:
 * @dstSize >= 6
 */
/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_decompress4X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    if cSrcSize < 10 {
        /* strict minimum : jump table + 1 byte per stream */
        return ERROR(ZSTD_error_corruption_detected);
    }
    if dstSize < 6 {
        /* stream 4-split doesn't work */
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.wrapping_add(dstSize);
        let olimit: *mut BYTE = oend.wrapping_sub(core::mem::size_of::<usize>() - 1);
        let dtPtr: *const c_void = DTable.add(1) as *const c_void;
        let dt: *const HUF_DEltX2 = dtPtr as *const HUF_DEltX2;

        /* Init */
        let mut bitD1: BIT_DStream_t = BIT_DStream_t::default();
        let mut bitD2: BIT_DStream_t = BIT_DStream_t::default();
        let mut bitD3: BIT_DStream_t = BIT_DStream_t::default();
        let mut bitD4: BIT_DStream_t = BIT_DStream_t::default();
        let length1: usize = MEM_readLE16(istart) as usize;
        let length2: usize = MEM_readLE16(istart.add(2)) as usize;
        let length3: usize = MEM_readLE16(istart.add(4)) as usize;
        let length4: usize = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1: *const BYTE = istart.wrapping_add(6); /* jumpTable */
        let istart2: *const BYTE = istart1.wrapping_add(length1);
        let istart3: *const BYTE = istart2.wrapping_add(length2);
        let istart4: *const BYTE = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2: *mut BYTE = ostart.wrapping_add(segmentSize);
        let opStart3: *mut BYTE = opStart2.wrapping_add(segmentSize);
        let opStart4: *mut BYTE = opStart3.wrapping_add(segmentSize);
        let mut op1: *mut BYTE = ostart;
        let mut op2: *mut BYTE = opStart2;
        let mut op3: *mut BYTE = opStart3;
        let mut op4: *mut BYTE = opStart4;
        let mut endSignal: U32 = 1;
        let dtd: DTableDesc = HUF_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        if opStart4 > oend {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD1), istart1, length1);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD2), istart2, length2);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD3), istart3, length3);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        {
            let err_code = BIT_initDStream(addr_of_mut!(bitD4), istart4, length4);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        if ((oend as isize).wrapping_sub(op4 as isize) as usize) >= core::mem::size_of::<usize>() {
            while (endSignal & ((op4 < olimit) as U32)) != 0 {
                /* !defined(__clang__) branch */
                /* HUF_DECODE_SYMBOLX2_2(op1..op4) */
                if MEM_64bits() != 0 {
                    op1 = op1.wrapping_add(HUF_decodeSymbolX2(
                        op1 as *mut c_void,
                        addr_of_mut!(bitD1),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 {
                    op2 = op2.wrapping_add(HUF_decodeSymbolX2(
                        op2 as *mut c_void,
                        addr_of_mut!(bitD2),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 {
                    op3 = op3.wrapping_add(HUF_decodeSymbolX2(
                        op3 as *mut c_void,
                        addr_of_mut!(bitD3),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 {
                    op4 = op4.wrapping_add(HUF_decodeSymbolX2(
                        op4 as *mut c_void,
                        addr_of_mut!(bitD4),
                        dt,
                        dtLog,
                    ) as usize);
                }
                /* HUF_DECODE_SYMBOLX2_1(op1..op4) */
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    op1 = op1.wrapping_add(HUF_decodeSymbolX2(
                        op1 as *mut c_void,
                        addr_of_mut!(bitD1),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    op2 = op2.wrapping_add(HUF_decodeSymbolX2(
                        op2 as *mut c_void,
                        addr_of_mut!(bitD2),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    op3 = op3.wrapping_add(HUF_decodeSymbolX2(
                        op3 as *mut c_void,
                        addr_of_mut!(bitD3),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 {
                    op4 = op4.wrapping_add(HUF_decodeSymbolX2(
                        op4 as *mut c_void,
                        addr_of_mut!(bitD4),
                        dt,
                        dtLog,
                    ) as usize);
                }
                /* HUF_DECODE_SYMBOLX2_2(op1..op4) */
                if MEM_64bits() != 0 {
                    op1 = op1.wrapping_add(HUF_decodeSymbolX2(
                        op1 as *mut c_void,
                        addr_of_mut!(bitD1),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 {
                    op2 = op2.wrapping_add(HUF_decodeSymbolX2(
                        op2 as *mut c_void,
                        addr_of_mut!(bitD2),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 {
                    op3 = op3.wrapping_add(HUF_decodeSymbolX2(
                        op3 as *mut c_void,
                        addr_of_mut!(bitD3),
                        dt,
                        dtLog,
                    ) as usize);
                }
                if MEM_64bits() != 0 {
                    op4 = op4.wrapping_add(HUF_decodeSymbolX2(
                        op4 as *mut c_void,
                        addr_of_mut!(bitD4),
                        dt,
                        dtLog,
                    ) as usize);
                }
                /* HUF_DECODE_SYMBOLX2_0(op1..op4) */
                op1 = op1.wrapping_add(HUF_decodeSymbolX2(
                    op1 as *mut c_void,
                    addr_of_mut!(bitD1),
                    dt,
                    dtLog,
                ) as usize);
                op2 = op2.wrapping_add(HUF_decodeSymbolX2(
                    op2 as *mut c_void,
                    addr_of_mut!(bitD2),
                    dt,
                    dtLog,
                ) as usize);
                op3 = op3.wrapping_add(HUF_decodeSymbolX2(
                    op3 as *mut c_void,
                    addr_of_mut!(bitD3),
                    dt,
                    dtLog,
                ) as usize);
                op4 = op4.wrapping_add(HUF_decodeSymbolX2(
                    op4 as *mut c_void,
                    addr_of_mut!(bitD4),
                    dt,
                    dtLog,
                ) as usize);

                endSignal = (((BIT_reloadDStreamFast(addr_of_mut!(bitD1))
                    == BIT_DStream_unfinished) as U32)
                    & ((BIT_reloadDStreamFast(addr_of_mut!(bitD2)) == BIT_DStream_unfinished)
                        as U32)
                    & ((BIT_reloadDStreamFast(addr_of_mut!(bitD3)) == BIT_DStream_unfinished)
                        as U32)
                    & ((BIT_reloadDStreamFast(addr_of_mut!(bitD4)) == BIT_DStream_unfinished)
                        as U32)) as U32;
            }
        }

        /* check corruption */
        if op1 > opStart2 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op2 > opStart3 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if op3 > opStart4 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* note : op4 already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX2(op1, addr_of_mut!(bitD1), opStart2, dt, dtLog);
        HUF_decodeStreamX2(op2, addr_of_mut!(bitD2), opStart3, dt, dtLog);
        HUF_decodeStreamX2(op3, addr_of_mut!(bitD3), opStart4, dt, dtLog);
        HUF_decodeStreamX2(op4, addr_of_mut!(bitD4), oend, dt, dtLog);

        /* check */
        {
            let endCheck: U32 = BIT_endOfDStream(addr_of!(bitD1))
                & BIT_endOfDStream(addr_of!(bitD2))
                & BIT_endOfDStream(addr_of!(bitD3))
                & BIT_endOfDStream(addr_of!(bitD4));
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
        return dstSize;
    }
}

/* HUF_NEED_BMI2_FUNCTION == 0 : HUF_decompress4X2_usingDTable_internal_bmi2 is
 * not compiled in. */

pub unsafe fn HUF_decompress4X2_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    HUF_decompress4X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 :
 * HUF_decompress4X2_usingDTable_internal_fast_asm_loop is not declared. */

pub unsafe fn HUF_decompress4X2_usingDTable_internal_fast_c_loop(
    args: *mut HUF_DecompressFastArgs,
) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let mut oend: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable: *const HUF_DEltX2 = (*args).dt as *const HUF_DEltX2;
    let ilowest: *const BYTE = (*args).ilowest;

    /* Copy the arguments to local registers. */
    ZSTD_memcpy(
        addr_of_mut!(bits) as *mut u8,
        addr_of!((*args).bits) as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!(ip) as *mut u8,
        addr_of!((*args).ip) as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!(op) as *mut u8,
        addr_of!((*args).op) as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );

    oend[0] = op[1];
    oend[1] = op[2];
    oend[2] = op[3];
    oend[3] = (*args).oend;

    'fastloop: loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            /* We can consume up to 7 bytes of input per iteration per stream. */
            let mut iters: usize = ((ip[0] as isize).wrapping_sub(ilowest as isize) as usize) / 7;
            /* Each iteration can produce up to 10 bytes of output per stream. */
            stream = 0;
            while stream < 4 {
                let oiters: usize = ((oend[stream as usize] as isize)
                    .wrapping_sub(op[stream as usize] as isize)
                    as usize)
                    / 10;
                iters = MIN(iters, oiters);
                stream += 1;
            }

            olimit = op[3].wrapping_add(iters * 5);

            /* Exit the fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break 'fastloop;
            }

            /* Exit the decoding loop if any input pointer has crossed the
             * previous one. This indicates corruption. (goto _out) */
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'fastloop;
                }
                stream += 1;
            }
        }

        /* Manually unroll the loop because compilers don't consistently
         * unroll the inner loops, which destroys performance.
         */
        loop {
            /* Decode 5 symbols from each of the first 3 streams.
             * HUF_4X_FOR_EACH_STREAM_WITH_VAR(HUF_4X2_DECODE_SYMBOL, 0) x 5
             * -- with _decode3 == 0, stream 3 is skipped at each of the 5 rounds.
             */
            /* round 1 */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[0], entry.sequence);
                bits[0] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[0] = op[0].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[1], entry.sequence);
                bits[1] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[1] = op[1].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[2], entry.sequence);
                bits[2] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[2] = op[2].wrapping_add(entry.length as usize);
            }
            /* round 2 */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[0], entry.sequence);
                bits[0] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[0] = op[0].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[1], entry.sequence);
                bits[1] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[1] = op[1].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[2], entry.sequence);
                bits[2] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[2] = op[2].wrapping_add(entry.length as usize);
            }
            /* round 3 */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[0], entry.sequence);
                bits[0] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[0] = op[0].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[1], entry.sequence);
                bits[1] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[1] = op[1].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[2], entry.sequence);
                bits[2] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[2] = op[2].wrapping_add(entry.length as usize);
            }
            /* round 4 */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[0], entry.sequence);
                bits[0] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[0] = op[0].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[1], entry.sequence);
                bits[1] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[1] = op[1].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[2], entry.sequence);
                bits[2] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[2] = op[2].wrapping_add(entry.length as usize);
            }
            /* round 5 */
            {
                let index: c_int = (bits[0] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[0], entry.sequence);
                bits[0] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[0] = op[0].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[1] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[1], entry.sequence);
                bits[1] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[1] = op[1].wrapping_add(entry.length as usize);
            }
            {
                let index: c_int = (bits[2] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[2], entry.sequence);
                bits[2] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[2] = op[2].wrapping_add(entry.length as usize);
            }

            /* Decode one symbol from the final stream
             * HUF_4X2_DECODE_SYMBOL(3, 1); */
            {
                let index: c_int = (bits[3] >> 53) as c_int;
                let entry: HUF_DEltX2 = *dtable.add(index as usize);
                MEM_write16(op[3], entry.sequence);
                bits[3] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                op[3] = op[3].wrapping_add(entry.length as usize);
            }

            /* HUF_4X_FOR_EACH_STREAM(HUF_4X2_RELOAD_STREAM); */
            /* HUF_4X2_RELOAD_STREAM(0) */
            {
                /* HUF_4X2_DECODE_SYMBOL(3, 1); */
                {
                    let index: c_int = (bits[3] >> 53) as c_int;
                    let entry: HUF_DEltX2 = *dtable.add(index as usize);
                    MEM_write16(op[3], entry.sequence);
                    bits[3] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                    op[3] = op[3].wrapping_add(entry.length as usize);
                }
                {
                    let ctz: c_int = ZSTD_countTrailingZeros64(bits[0]) as c_int;
                    let nbBits: c_int = ctz & 7;
                    let nbBytes: c_int = ctz >> 3;
                    ip[0] = ip[0].wrapping_sub(nbBytes as usize);
                    bits[0] = MEM_read64(ip[0]) | 1;
                    bits[0] <<= nbBits as u32;
                }
            }
            /* HUF_4X2_RELOAD_STREAM(1) */
            {
                /* HUF_4X2_DECODE_SYMBOL(3, 1); */
                {
                    let index: c_int = (bits[3] >> 53) as c_int;
                    let entry: HUF_DEltX2 = *dtable.add(index as usize);
                    MEM_write16(op[3], entry.sequence);
                    bits[3] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                    op[3] = op[3].wrapping_add(entry.length as usize);
                }
                {
                    let ctz: c_int = ZSTD_countTrailingZeros64(bits[1]) as c_int;
                    let nbBits: c_int = ctz & 7;
                    let nbBytes: c_int = ctz >> 3;
                    ip[1] = ip[1].wrapping_sub(nbBytes as usize);
                    bits[1] = MEM_read64(ip[1]) | 1;
                    bits[1] <<= nbBits as u32;
                }
            }
            /* HUF_4X2_RELOAD_STREAM(2) */
            {
                /* HUF_4X2_DECODE_SYMBOL(3, 1); */
                {
                    let index: c_int = (bits[3] >> 53) as c_int;
                    let entry: HUF_DEltX2 = *dtable.add(index as usize);
                    MEM_write16(op[3], entry.sequence);
                    bits[3] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                    op[3] = op[3].wrapping_add(entry.length as usize);
                }
                {
                    let ctz: c_int = ZSTD_countTrailingZeros64(bits[2]) as c_int;
                    let nbBits: c_int = ctz & 7;
                    let nbBytes: c_int = ctz >> 3;
                    ip[2] = ip[2].wrapping_sub(nbBytes as usize);
                    bits[2] = MEM_read64(ip[2]) | 1;
                    bits[2] <<= nbBits as u32;
                }
            }
            /* HUF_4X2_RELOAD_STREAM(3) */
            {
                /* HUF_4X2_DECODE_SYMBOL(3, 1); */
                {
                    let index: c_int = (bits[3] >> 53) as c_int;
                    let entry: HUF_DEltX2 = *dtable.add(index as usize);
                    MEM_write16(op[3], entry.sequence);
                    bits[3] <<= ((entry.nbBits as c_int) & 0x3F) as u32;
                    op[3] = op[3].wrapping_add(entry.length as usize);
                }
                {
                    let ctz: c_int = ZSTD_countTrailingZeros64(bits[3]) as c_int;
                    let nbBits: c_int = ctz & 7;
                    let nbBytes: c_int = ctz >> 3;
                    ip[3] = ip[3].wrapping_sub(nbBytes as usize);
                    bits[3] = MEM_read64(ip[3]) | 1;
                    bits[3] <<= nbBits as u32;
                }
            }

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* _out: */

    /* Save the final values of each of the state variables back to args. */
    ZSTD_memcpy(
        addr_of_mut!((*args).bits) as *mut u8,
        addr_of!(bits) as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!((*args).ip) as *mut u8,
        addr_of!(ip) as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        addr_of_mut!((*args).op) as *mut u8,
        addr_of!(op) as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );
}

pub unsafe fn HUF_decompress4X2_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt: *const c_void = DTable.add(1) as *const c_void;
    let ilowest: *const BYTE = cSrc as *const BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);
    let mut args: HUF_DecompressFastArgs = core::mem::zeroed();
    {
        let ret: usize =
            HUF_DecompressFastArgs_init(addr_of_mut!(args), dst, dstSize, cSrc, cSrcSize, DTable);
        {
            let err_code = ret;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(addr_of_mut!(args));

    /* note : op4 already verified within main loop */

    /* finish bitStreams one by one */
    {
        let segmentSize: usize = (dstSize + 3) / 4;
        let mut segmentEnd: *mut BYTE = dst as *mut BYTE;
        let mut i: c_int;
        i = 0;
        while i < 4 {
            let mut bit: BIT_DStream_t = BIT_DStream_t::default();
            if segmentSize <= ((oend as isize).wrapping_sub(segmentEnd as isize) as usize) {
                segmentEnd = segmentEnd.wrapping_add(segmentSize);
            } else {
                segmentEnd = oend;
            }
            {
                let err_code = HUF_initRemainingDStream(
                    addr_of_mut!(bit),
                    addr_of!(args),
                    i,
                    segmentEnd,
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            args.op[i as usize] = args.op[i as usize].wrapping_add(HUF_decodeStreamX2(
                args.op[i as usize],
                addr_of_mut!(bit),
                segmentEnd,
                dt as *const HUF_DEltX2,
                HUF_DECODER_FAST_TABLELOG,
            ));
            if args.op[i as usize] != segmentEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            i += 1;
        }
    }

    /* decoded size */
    dstSize
}

pub unsafe fn HUF_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let fallbackFn: HUF_DecompressUsingDTableFn = HUF_decompress4X2_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn = HUF_decompress4X2_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 :
     * no bmi2 fallback selection, no asm loop selection. */

    if HUF_ENABLE_FAST_DECODE != 0 && (flags & HUF_flags_disableFast) == 0 {
        let ret: usize = HUF_decompress4X2_usingDTable_internal_fast(
            dst, dstSize, cSrc, cSrcSize, DTable, loopFn,
        );
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* HUF_DGEN(HUF_decompress1X2_usingDTable_internal) -- DYNAMIC_BMI2 == 0 branch */
pub unsafe fn HUF_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let _ = flags;
    HUF_decompress1X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X2_DCtx_wksp(
    DCtx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUF_readDTableX2_wksp(DCtx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress1X2_usingDTable_internal(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        DCtx,
        flags,
    )
}

pub unsafe fn HUF_decompress4X2_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let mut hSize: usize = HUF_readDTableX2_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X2_usingDTable_internal(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        dctx,
        flags,
    )
}

/* #endif  HUF_FORCE_DECOMPRESS_X1 */

/* ***********************************/
/* Universal decompression selectors */
/* ***********************************/

#[repr(C)]
#[derive(Clone, Copy)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}

macro_rules! at {
    ($t:expr, $d:expr) => {
        algo_time_t {
            tableTime: $t,
            decode256Time: $d,
        }
    };
}

pub static algoTime: [[algo_time_t; 2]; 16] = [
    /* single, double, quad */
    [at!(0, 0), at!(1, 1)],       /* Q==0 : impossible */
    [at!(0, 0), at!(1, 1)],       /* Q==1 : impossible */
    [at!(150, 216), at!(381, 119)], /* Q == 2 : 12-18% */
    [at!(170, 205), at!(514, 112)], /* Q == 3 : 18-25% */
    [at!(177, 199), at!(539, 110)], /* Q == 4 : 25-32% */
    [at!(197, 194), at!(644, 107)], /* Q == 5 : 32-38% */
    [at!(221, 192), at!(735, 107)], /* Q == 6 : 38-44% */
    [at!(256, 189), at!(881, 106)], /* Q == 7 : 44-50% */
    [at!(359, 188), at!(1167, 109)], /* Q == 8 : 50-56% */
    [at!(582, 187), at!(1570, 114)], /* Q == 9 : 56-62% */
    [at!(688, 187), at!(1712, 122)], /* Q ==10 : 62-69% */
    [at!(825, 186), at!(1965, 136)], /* Q ==11 : 69-75% */
    [at!(976, 185), at!(2131, 150)], /* Q ==12 : 75-81% */
    [at!(1180, 186), at!(2070, 175)], /* Q ==13 : 81-87% */
    [at!(1377, 185), at!(1731, 202)], /* Q ==14 : 87-93% */
    [at!(1412, 185), at!(1695, 202)], /* Q ==15 : 93-99% */
];

/** HUF_selectDecoder() :
 *  Tells which decoder is likely to decode faster,
 *  based on a set of pre-computed metrics.
 * @return : 0==HUF_decompress4X1, 1==HUF_decompress4X2 .
 *  Assumption : 0 < dstSize <= 128 KB */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_selectDecoder(dstSize: usize, cSrcSize: usize) -> U32 {
    /* decoder timing evaluation */
    {
        let Q: U32 = if cSrcSize >= dstSize {
            15
        } else {
            (cSrcSize * 16 / dstSize) as U32
        }; /* Q < 16 */
        let D256: U32 = (dstSize >> 8) as U32;
        let DTime0: U32 = algoTime[Q as usize][0]
            .tableTime
            .wrapping_add(algoTime[Q as usize][0].decode256Time.wrapping_mul(D256));
        let mut DTime1: U32 = algoTime[Q as usize][1]
            .tableTime
            .wrapping_add(algoTime[Q as usize][1].decode256Time.wrapping_mul(D256));
        /* small advantage to algorithm using less memory, to reduce cache eviction */
        DTime1 = DTime1.wrapping_add(DTime1 >> 5);
        (DTime1 < DTime0) as U32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected); /* invalid */
    }
    if cSrcSize == dstSize {
        /* not compressed */
        ZSTD_memcpy(dst as *mut u8, cSrc as *const u8, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        /* RLE */
        ZSTD_memset(dst as *mut u8, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    }

    {
        let algoNb: U32 = HUF_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUF_decompress1X2_DCtx_wksp(
                dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
            )
        } else {
            HUF_decompress1X1_DCtx_wksp(
                dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let dtd: DTableDesc = HUF_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUF_decompress1X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    } else {
        HUF_decompress1X1_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    }
}

/* #ifndef HUF_FORCE_DECOMPRESS_X2 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X1_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: usize = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.wrapping_add(hSize);
    cSrcSize -= hSize;

    HUF_decompress1X1_usingDTable_internal(
        dst,
        dstSize,
        ip as *const c_void,
        cSrcSize,
        dctx,
        flags,
    )
}
/* #endif */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let dtd: DTableDesc = HUF_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUF_decompress4X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    } else {
        HUF_decompress4X1_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_hufOnly_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let algoNb: U32 = HUF_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUF_decompress4X2_DCtx_wksp(
                dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
            )
        } else {
            HUF_decompress4X1_DCtx_wksp(
                dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
            )
        }
    }
}
