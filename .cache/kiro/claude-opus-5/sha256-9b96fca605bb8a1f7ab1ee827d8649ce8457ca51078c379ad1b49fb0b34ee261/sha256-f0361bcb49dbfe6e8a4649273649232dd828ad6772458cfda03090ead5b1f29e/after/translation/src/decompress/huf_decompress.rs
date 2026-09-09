//! Rust transliteration of `c_src/src/decompress/huf_decompress.c`.
//!
//! Build configuration: DYNAMIC_BMI2=0, no x86-64 BMI2 asm
//! (ZSTD_ENABLE_ASM_X86_64_BMI2 == 0), DEBUGLEVEL 0.
//! HUF_ENABLE_FAST_DECODE == 1 (HUF_DISABLE_FAST_DECODE not defined).

use core::ffi::{c_int, c_uint, c_void};

use crate::common::bits::*;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;

use crate::common::entropy_common::HUF_readStats_wksp;

/* **************************************************************
*  Constants
****************************************************************/

const HUF_DECODER_FAST_TABLELOG: U32 = 11;

/* HUF_ENABLE_FAST_DECODE : HUF_DISABLE_FAST_DECODE is not defined in this build. */
const HUF_ENABLE_FAST_DECODE: c_int = 1;

/* **************************************************************
*  Error Management
****************************************************************/
#[inline(always)]
unsafe fn HUF_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

/* **************************************************************
*  compiler.h helper : ZSTD_maybeNullPtrAdd
****************************************************************/
#[inline(always)]
unsafe fn ZSTD_maybeNullPtrAdd(ptr: *mut BYTE, add: isize) -> *mut BYTE {
    if add > 0 {
        ptr.offset(add)
    } else {
        ptr
    }
}

/* **************************************************************
*  BMI2 Variant Wrappers
****************************************************************/
type HUF_DecompressUsingDTableFn = unsafe fn(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t;

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/
#[repr(C)]
#[derive(Copy, Clone)]
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
        &mut dtd as *mut DTableDesc as *mut u8,
        table as *const u8,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

pub unsafe fn HUF_initFastDStream(ip: *const BYTE) -> size_t {
    let lastByte: BYTE = *ip.add(7);
    let bitsConsumed: size_t = if lastByte != 0 {
        (8 - ZSTD_highbit32(lastByte as U32)) as size_t
    } else {
        0
    };
    let value: size_t = MEM_readLEST(ip) | 1;
    value << bitsConsumed
}

/**
 * The input/output arguments to the Huffman fast decoding loop.
 */
#[repr(C)]
pub struct HUF_DecompressFastArgs {
    pub ip: [*const BYTE; 4],
    pub op: [*mut BYTE; 4],
    pub bits: [U64; 4],
    pub dt: *const c_void,
    pub ilowest: *const BYTE,
    pub oend: *mut BYTE,
    pub iend: [*const BYTE; 4],
}

pub type HUF_DecompressFastLoopFn = unsafe fn(*mut HUF_DecompressFastArgs);

/**
 * Initializes args for the fast decoding loop.
 * @returns 1 on success
 *          0 if the fallback implementation should be used.
 *          Or an error code on failure.
 */
pub unsafe fn HUF_DecompressFastArgs_init(
    args: *mut HUF_DecompressFastArgs,
    dst: *mut c_void,
    dstSize: size_t,
    src: *const c_void,
    srcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    let dt: *const c_void = DTable.add(1) as *const c_void;
    let dtLog: U32 = HUF_getDTableDesc(DTable).tableLog as U32;

    let istart: *const BYTE = src as *const BYTE;

    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);

    /* The fast decoding loop assumes 64-bit little-endian. */
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

    /* Must have at least 8 bytes per stream ... */
    if dtLog != HUF_DECODER_FAST_TABLELOG {
        return 0;
    }

    /* Read the jump table. */
    {
        let length1: size_t = MEM_readLE16(istart) as size_t;
        let length2: size_t = MEM_readLE16(istart.add(2)) as size_t;
        let length3: size_t = MEM_readLE16(istart.add(4)) as size_t;
        let length4: size_t = srcSize.wrapping_sub(length1 + length2 + length3 + 6);
        (*args).iend[0] = istart.add(6); /* jumpTable */
        (*args).iend[1] = (*args).iend[0].add(length1);
        (*args).iend[2] = (*args).iend[1].add(length2);
        (*args).iend[3] = (*args).iend[2].add(length3);

        /* HUF_initFastDStream() requires this ... */
        if length1 < 8 || length2 < 8 || length3 < 8 || length4 < 8 {
            return 0;
        }
        if length4 > srcSize {
            return ERROR(ZSTD_error_corruption_detected);
        } /* overflow */
    }
    /* ip[] contains the position that is currently loaded into bits[]. */
    (*args).ip[0] = (*args).iend[1].sub(core::mem::size_of::<U64>());
    (*args).ip[1] = (*args).iend[2].sub(core::mem::size_of::<U64>());
    (*args).ip[2] = (*args).iend[3].sub(core::mem::size_of::<U64>());
    (*args).ip[3] = (src as *const BYTE)
        .add(srcSize)
        .sub(core::mem::size_of::<U64>());

    /* op[] contains the output pointers. */
    (*args).op[0] = dst as *mut BYTE;
    (*args).op[1] = (*args).op[0].add((dstSize + 3) / 4);
    (*args).op[2] = (*args).op[1].add((dstSize + 3) / 4);
    (*args).op[3] = (*args).op[2].add((dstSize + 3) / 4);

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
) -> size_t {
    let s = stream as usize;
    /* Validate that we haven't overwritten. */
    if (*args).op[s] > segmentEnd {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Validate that we haven't read beyond iend[]. */
    if (*args).ip[s] < (*args).iend[s].sub(8) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* Construct the BIT_DStream_t. */
    (*bit).bitContainer = MEM_readLEST((*args).ip[s]);
    (*bit).bitsConsumed = ZSTD_countTrailingZeros64((*args).bits[s]);
    (*bit).start = (*args).ilowest as *const u8;
    (*bit).limitPtr = (*bit).start.add(core::mem::size_of::<size_t>());
    (*bit).ptr = (*args).ip[s] as *const u8;

    0
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/
#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_DEltX1 {
    pub nbBits: BYTE,
    pub byte: BYTE,
}

/**
 * Packs 4 HUF_DEltX1 structs into a U64.
 */
pub unsafe fn HUF_DEltX1_set4(symbol: BYTE, nbBits: BYTE) -> U64 {
    let mut D4: U64;
    if MEM_isLittleEndian() != 0 {
        D4 = (((symbol as c_int) << 8) + (nbBits as c_int)) as U64;
    } else {
        D4 = ((symbol as c_int) + ((nbBits as c_int) << 8)) as U64;
    }
    D4 = D4.wrapping_mul(0x0001000100010001u64);
    D4
}

/**
 * Increase the tableLog to targetTableLog and rescales the stats.
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
        let scale: U32 = targetTableLog - tableLog;
        let mut s: U32;
        /* Increase the weight for all non-zero probability symbols by scale. */
        s = 0;
        while s < nbSymbols {
            let hw = *huffWeight.add(s as usize);
            *huffWeight.add(s as usize) =
                hw.wrapping_add(if hw == 0 { 0 } else { scale as BYTE });
            s += 1;
        }
        /* Update rankVal to reflect the new weights. */
        s = targetTableLog;
        while s > scale {
            *rankVal.add(s as usize) = *rankVal.add((s - scale) as usize);
            s -= 1;
        }
        s = scale;
        while s > 0 {
            *rankVal.add(s as usize) = 0;
            s -= 1;
        }
    }
    targetTableLog
}

#[repr(C)]
pub struct HUF_ReadDTableX1_Workspace {
    pub rankVal: [U32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize],
    pub rankStart: [U32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize],
    pub statsWksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32 as usize],
    pub symbols: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    pub huffWeight: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX1_wksp(
    DTable: *mut HUF_DTable,
    src: *const c_void,
    srcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    let mut tableLog: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let iSize: size_t;
    let dtPtr: *mut c_void = DTable.add(1) as *mut c_void;
    let dt: *mut HUF_DEltX1 = dtPtr as *mut HUF_DEltX1;
    let wksp: *mut HUF_ReadDTableX1_Workspace = workSpace as *mut HUF_ReadDTableX1_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX1_Workspace>() > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUF_readStats_wksp(
        (*wksp).huffWeight.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as size_t,
        (*wksp).rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
        (*wksp).statsWksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).statsWksp),
        flags,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* Table header */
    {
        let mut dtd: DTableDesc = HUF_getDTableDesc(DTable);
        let maxTableLog: U32 = dtd.maxTableLog as U32 + 1;
        let targetTableLog: U32 = MIN(maxTableLog, HUF_DECODER_FAST_TABLELOG);
        tableLog = HUF_rescaleStats(
            (*wksp).huffWeight.as_mut_ptr(),
            (*wksp).rankVal.as_mut_ptr(),
            nbSymbols,
            tableLog,
            targetTableLog,
        );
        if tableLog > (dtd.maxTableLog as U32 + 1) {
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as BYTE;
        ZSTD_memcpy(
            DTable as *mut u8,
            &dtd as *const DTableDesc as *const u8,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Compute symbols and rankStart given rankVal. */
    {
        let mut n: c_int;
        let mut nextRankStart: U32 = 0;
        let unroll: c_int = 4;
        let nLimit: c_int = nbSymbols as c_int - unroll + 1;
        n = 0;
        while n < (tableLog as c_int + 1) {
            let curr: U32 = nextRankStart;
            nextRankStart += (*wksp).rankVal[n as usize];
            (*wksp).rankStart[n as usize] = curr;
            n += 1;
        }
        n = 0;
        while n < nLimit {
            let mut u: c_int = 0;
            while u < unroll {
                let w: size_t = (*wksp).huffWeight[(n + u) as usize] as size_t;
                let idx = (*wksp).rankStart[w as usize];
                (*wksp).symbols[idx as usize] = (n + u) as BYTE;
                (*wksp).rankStart[w as usize] = idx + 1;
                u += 1;
            }
            n += unroll;
        }
        while n < nbSymbols as c_int {
            let w: size_t = (*wksp).huffWeight[n as usize] as size_t;
            let idx = (*wksp).rankStart[w as usize];
            (*wksp).symbols[idx as usize] = n as BYTE;
            (*wksp).rankStart[w as usize] = idx + 1;
            n += 1;
        }
    }

    /* fill DTable */
    {
        let mut w: U32;
        let mut symbol: c_int = (*wksp).rankVal[0] as c_int;
        let mut rankStart: c_int = 0;
        w = 1;
        while w < tableLog + 1 {
            let symbolCount: c_int = (*wksp).rankVal[w as usize] as c_int;
            let length: c_int = (1 << w) >> 1;
            let mut uStart: c_int = rankStart;
            let nbBits: BYTE = (tableLog + 1 - w) as BYTE;
            let mut s: c_int;
            let mut u: c_int;
            match length {
                1 => {
                    s = 0;
                    while s < symbolCount {
                        let D = HUF_DEltX1 {
                            byte: (*wksp).symbols[(symbol + s) as usize],
                            nbBits,
                        };
                        *dt.offset(uStart as isize) = D;
                        uStart += 1;
                        s += 1;
                    }
                }
                2 => {
                    s = 0;
                    while s < symbolCount {
                        let D = HUF_DEltX1 {
                            byte: (*wksp).symbols[(symbol + s) as usize],
                            nbBits,
                        };
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
                            HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
                        MEM_write64(dt.offset(uStart as isize) as *mut u8, D4);
                        uStart += 4;
                        s += 1;
                    }
                }
                8 => {
                    s = 0;
                    while s < symbolCount {
                        let D4: U64 =
                            HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
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
                            HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
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
            w += 1;
        }
    }
    iSize
}

#[inline(always)]
pub unsafe fn HUF_decodeSymbolX1(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> BYTE {
    let val: size_t = BIT_lookBitsFast(Dstream, dtLog) as size_t; /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val)).byte;
    BIT_skipBits(Dstream, (*dt.add(val)).nbBits as U32);
    c
}

/* HUF_DECODE_SYMBOLX1_0 : *ptr++ = HUF_decodeSymbolX1(...) */
macro_rules! HUF_DECODE_SYMBOLX1_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        *$ptr = HUF_decodeSymbolX1($DStreamPtr, $dt, $dtLog);
        $ptr = $ptr.add(1);
    }};
}

/* HUF_DECODE_SYMBOLX1_1 : if (MEM_64bits() || HUF_TABLELOG_MAX<=12) */
macro_rules! HUF_DECODE_SYMBOLX1_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUF_TABLELOG_MAX <= 12) {
            HUF_DECODE_SYMBOLX1_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}

/* HUF_DECODE_SYMBOLX1_2 : if (MEM_64bits()) */
macro_rules! HUF_DECODE_SYMBOLX1_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            HUF_DECODE_SYMBOLX1_0!($ptr, $DStreamPtr, $dt, $dtLog);
        }
    }};
}

#[inline]
pub unsafe fn HUF_decodeStreamX1(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;

    /* up to 4 symbols at a time */
    if (pEnd as isize - p as isize) > 3 {
        while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
            & ((p as *const BYTE) < pEnd.sub(3) as *const BYTE) as c_int)
            != 0
        {
            HUF_DECODE_SYMBOLX1_2!(p, bitDPtr, dt, dtLog);
            HUF_DECODE_SYMBOLX1_1!(p, bitDPtr, dt, dtLog);
            HUF_DECODE_SYMBOLX1_2!(p, bitDPtr, dt, dtLog);
            HUF_DECODE_SYMBOLX1_0!(p, bitDPtr, dt, dtLog);
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }

    /* [0-3] symbols remaining */
    if MEM_32bits() != 0 {
        while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
            & ((p as *const BYTE) < pEnd as *const BYTE) as c_int)
            != 0
        {
            HUF_DECODE_SYMBOLX1_0!(p, bitDPtr, dt, dtLog);
        }
    }

    /* no more data to retrieve from bitstream, no need to reload */
    while (p as *const BYTE) < pEnd as *const BYTE {
        HUF_DECODE_SYMBOLX1_0!(p, bitDPtr, dt, dtLog);
    }

    (pEnd as isize - pStart as isize) as size_t
}

#[inline(always)]
pub unsafe fn HUF_decompress1X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    let op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(op, dstSize as isize);
    let dtPtr: *const c_void = DTable.add(1) as *const c_void;
    let dt: *const HUF_DEltX1 = dtPtr as *const HUF_DEltX1;
    let mut bitD: BIT_DStream_t = new_dstream();
    let dtd: DTableDesc = HUF_getDTableDesc(DTable);
    let dtLog: U32 = dtd.tableLog as U32;

    {
        let e = BIT_initDStream(&mut bitD, cSrc as *const u8, cSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    HUF_decodeStreamX1(op, &mut bitD, oend, dt, dtLog);

    if BIT_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    dstSize
}

/* HUF_decompress4X1_usingDTable_internal_body(): @dstSize >= 6 */
#[inline(always)]
pub unsafe fn HUF_decompress4X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    /* Check */
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if dstSize < 6 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.add(dstSize);
        let olimit: *mut BYTE = oend.sub(3);
        let dtPtr: *const c_void = DTable.add(1) as *const c_void;
        let dt: *const HUF_DEltX1 = dtPtr as *const HUF_DEltX1;

        let mut bitD1: BIT_DStream_t = new_dstream();
        let mut bitD2: BIT_DStream_t = new_dstream();
        let mut bitD3: BIT_DStream_t = new_dstream();
        let mut bitD4: BIT_DStream_t = new_dstream();
        let length1: size_t = MEM_readLE16(istart) as size_t;
        let length2: size_t = MEM_readLE16(istart.add(2)) as size_t;
        let length3: size_t = MEM_readLE16(istart.add(4)) as size_t;
        let length4: size_t = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1: *const BYTE = istart.add(6); /* jumpTable */
        let istart2: *const BYTE = istart1.add(length1);
        let istart3: *const BYTE = istart2.add(length2);
        let istart4: *const BYTE = istart3.add(length3);
        let segmentSize: size_t = (dstSize + 3) / 4;
        let opStart2: *mut BYTE = ostart.add(segmentSize);
        let opStart3: *mut BYTE = opStart2.add(segmentSize);
        let opStart4: *mut BYTE = opStart3.add(segmentSize);
        let mut op1: *mut BYTE = ostart;
        let mut op2: *mut BYTE = opStart2;
        let mut op3: *mut BYTE = opStart3;
        let mut op4: *mut BYTE = opStart4;
        let dtd: DTableDesc = HUF_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;
        let mut endSignal: U32 = 1;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if opStart4 > oend {
            return ERROR(ZSTD_error_corruption_detected);
        }
        {
            let e = BIT_initDStream(&mut bitD1, istart1, length1);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD2, istart2, length2);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD3, istart3, length3);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD4, istart4, length4);
            if ERR_isError(e) != 0 {
                return e;
            }
        }

        /* up to 16 symbols per loop (4 symbols per stream) in 64-bit mode */
        if (oend as isize - op4 as isize) as size_t >= core::mem::size_of::<size_t>() {
            while (endSignal & ((op4 as *const BYTE) < olimit as *const BYTE) as U32) != 0 {
                HUF_DECODE_SYMBOLX1_2!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX1_1!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_1!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_1!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_1!(op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_2!(op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0!(op4, &mut bitD4, dt, dtLog);
                endSignal &=
                    (BIT_reloadDStreamFast(&mut bitD1) == BIT_DStream_unfinished) as U32;
                endSignal &=
                    (BIT_reloadDStreamFast(&mut bitD2) == BIT_DStream_unfinished) as U32;
                endSignal &=
                    (BIT_reloadDStreamFast(&mut bitD3) == BIT_DStream_unfinished) as U32;
                endSignal &=
                    (BIT_reloadDStreamFast(&mut bitD4) == BIT_DStream_unfinished) as U32;
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
        HUF_decodeStreamX1(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX1(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX1(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX1(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        {
            let endCheck: U32 = BIT_endOfDStream(&bitD1)
                & BIT_endOfDStream(&bitD2)
                & BIT_endOfDStream(&bitD3)
                & BIT_endOfDStream(&bitD4);
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
        dstSize
    }
}

/* HUF_NEED_BMI2_FUNCTION == 0 (DYNAMIC_BMI2==0) : no _bmi2 variant compiled. */

pub unsafe fn HUF_decompress4X1_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    HUF_decompress4X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 : no asm loop declared. */

pub unsafe fn HUF_decompress4X1_usingDTable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable: *const U16 = (*args).dt as *const U16;
    let oend: *mut BYTE = (*args).oend;
    let ilowest: *const BYTE = (*args).ilowest;

    /* Copy the arguments to local variables */
    ZSTD_memcpy(
        bits.as_mut_ptr() as *mut u8,
        (*args).bits.as_ptr() as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        ip.as_mut_ptr() as *mut u8,
        (*args).ip.as_ptr() as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        op.as_mut_ptr() as *mut u8,
        (*args).op.as_ptr() as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );

    'outer: loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            /* Each iteration produces 5 output symbols per stream */
            let oiters: size_t = ((oend as isize - op[3] as isize) as size_t) / 5;
            /* Each iteration consumes up to 11 bits * 5 = 55 bits < 7 bytes per stream. */
            let iiters: size_t = ((ip[0] as isize - ilowest as isize) as size_t) / 7;
            let iters: size_t = MIN(oiters, iiters);
            let symbols: size_t = iters * 5;

            olimit = op[3].add(symbols);

            /* Exit fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break 'outer;
            }

            /* Exit the decoding loop if any input pointer has crossed the previous one. */
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'outer;
                }
                stream += 1;
            }
        }

        macro_rules! HUF_4X1_DECODE_SYMBOL {
            ($stream:expr, $symbol:expr) => {{
                let index: c_int = (bits[$stream] >> 53) as c_int;
                let entry: c_int = *dtable.offset(index as isize) as c_int;
                bits[$stream] <<= (entry & 0x3F) as U64;
                *op[$stream].add($symbol) = ((entry >> 8) & 0xFF) as BYTE;
            }};
        }

        macro_rules! HUF_4X1_RELOAD_STREAM {
            ($stream:expr) => {{
                let ctz: c_int = ZSTD_countTrailingZeros64(bits[$stream]) as c_int;
                let nbBits: c_int = ctz & 7;
                let nbBytes: c_int = ctz >> 3;
                op[$stream] = op[$stream].add(5);
                ip[$stream] = ip[$stream].offset(-(nbBytes as isize));
                bits[$stream] = MEM_read64(ip[$stream]) | 1;
                bits[$stream] <<= nbBits as U64;
            }};
        }

        /* Manually unroll the loop. */
        loop {
            /* Decode 5 symbols in each of the 4 streams */
            HUF_4X1_DECODE_SYMBOL!(0, 0);
            HUF_4X1_DECODE_SYMBOL!(1, 0);
            HUF_4X1_DECODE_SYMBOL!(2, 0);
            HUF_4X1_DECODE_SYMBOL!(3, 0);
            HUF_4X1_DECODE_SYMBOL!(0, 1);
            HUF_4X1_DECODE_SYMBOL!(1, 1);
            HUF_4X1_DECODE_SYMBOL!(2, 1);
            HUF_4X1_DECODE_SYMBOL!(3, 1);
            HUF_4X1_DECODE_SYMBOL!(0, 2);
            HUF_4X1_DECODE_SYMBOL!(1, 2);
            HUF_4X1_DECODE_SYMBOL!(2, 2);
            HUF_4X1_DECODE_SYMBOL!(3, 2);
            HUF_4X1_DECODE_SYMBOL!(0, 3);
            HUF_4X1_DECODE_SYMBOL!(1, 3);
            HUF_4X1_DECODE_SYMBOL!(2, 3);
            HUF_4X1_DECODE_SYMBOL!(3, 3);
            HUF_4X1_DECODE_SYMBOL!(0, 4);
            HUF_4X1_DECODE_SYMBOL!(1, 4);
            HUF_4X1_DECODE_SYMBOL!(2, 4);
            HUF_4X1_DECODE_SYMBOL!(3, 4);

            /* Reload each of the 4 the bitstreams */
            HUF_4X1_RELOAD_STREAM!(0);
            HUF_4X1_RELOAD_STREAM!(1);
            HUF_4X1_RELOAD_STREAM!(2);
            HUF_4X1_RELOAD_STREAM!(3);

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* Save the final values back to args. */
    ZSTD_memcpy(
        (*args).bits.as_mut_ptr() as *mut u8,
        bits.as_ptr() as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        (*args).ip.as_mut_ptr() as *mut u8,
        ip.as_ptr() as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        (*args).op.as_mut_ptr() as *mut u8,
        op.as_ptr() as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );
}

pub unsafe fn HUF_decompress4X1_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> size_t {
    let dt: *const c_void = DTable.add(1) as *const c_void;
    let _ilowest: *const BYTE = cSrc as *const BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);
    let mut args: HUF_DecompressFastArgs = new_fast_args();
    {
        let ret: size_t =
            HUF_DecompressFastArgs_init(&mut args, dst, dstSize, cSrc, cSrcSize, DTable);
        if ERR_isError(ret) != 0 {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(&mut args);

    /* finish bit streams one by one. */
    {
        let segmentSize: size_t = (dstSize + 3) / 4;
        let mut segmentEnd: *mut BYTE = dst as *mut BYTE;
        let mut i: c_int = 0;
        while i < 4 {
            let mut bit: BIT_DStream_t = new_dstream();
            if segmentSize <= (oend as isize - segmentEnd as isize) as size_t {
                segmentEnd = segmentEnd.add(segmentSize);
            } else {
                segmentEnd = oend;
            }
            {
                let e = HUF_initRemainingDStream(&mut bit, &args, i, segmentEnd);
                if ERR_isError(e) != 0 {
                    return e;
                }
            }
            args.op[i as usize] = args.op[i as usize].add(HUF_decodeStreamX1(
                args.op[i as usize],
                &mut bit,
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

/* HUF_DGEN(HUF_decompress1X1_usingDTable_internal) with DYNAMIC_BMI2==0 */
pub unsafe fn HUF_decompress1X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> size_t {
    let _ = flags;
    HUF_decompress1X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

pub unsafe fn HUF_decompress4X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> size_t {
    let fallbackFn: HUF_DecompressUsingDTableFn = HUF_decompress4X1_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn = HUF_decompress4X1_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 : no dispatch overrides. */

    if HUF_ENABLE_FAST_DECODE != 0 && (flags & HUF_flags_disableFast as c_int) == 0 {
        let ret: size_t =
            HUF_decompress4X1_usingDTable_internal_fast(dst, dstSize, cSrc, cSrcSize, DTable, loopFn);
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

pub unsafe fn HUF_decompress4X1_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: size_t = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X1_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx, flags)
}

/* *************************/
/* double-symbols decoding */
/* *************************/

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HUF_DEltX2 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
}

pub type rankValCol_t = [U32; (HUF_TABLELOG_MAX + 1) as usize];
pub type rankVal_t = [rankValCol_t; HUF_TABLELOG_MAX as usize];

/**
 * Constructs a HUF_DEltX2 in a U32.
 */
pub unsafe fn HUF_buildDEltX2U32(symbol: U32, nbBits: U32, baseSeq: U32, level: c_int) -> U32 {
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
        &mut DElt as *mut HUF_DEltX2 as *mut u8,
        &val as *const U32 as *const u8,
        core::mem::size_of::<U32>(),
    );
    DElt
}

/**
 * Constructs 2 HUF_DEltX2s and packs them into a U64.
 */
pub unsafe fn HUF_buildDEltX2U64(symbol: U32, nbBits: U32, baseSeq: U16, level: c_int) -> U64 {
    let DElt: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq as U32, level);
    (DElt as U64).wrapping_add((DElt as U64) << 32)
}

/**
 * Fills the DTable rank with all the symbols from [begin, end).
 */
pub unsafe fn HUF_fillDTableX2ForWeight(
    mut DTableRank: *mut HUF_DEltX2,
    begin: *const sortedSymbol_t,
    end: *const sortedSymbol_t,
    nbBits: U32,
    tableLog: U32,
    baseSeq: U16,
    level: c_int,
) {
    let length: U32 = 1u32 << ((tableLog - nbBits) & 0x1F);
    let mut ptr: *const sortedSymbol_t;
    match length {
        1 => {
            ptr = begin;
            while ptr != end {
                let DElt: HUF_DEltX2 =
                    HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level);
                *DTableRank = DElt;
                DTableRank = DTableRank.add(1);
                ptr = ptr.add(1);
            }
        }
        2 => {
            ptr = begin;
            while ptr != end {
                let DElt: HUF_DEltX2 =
                    HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level);
                *DTableRank.add(0) = DElt;
                *DTableRank.add(1) = DElt;
                DTableRank = DTableRank.add(2);
                ptr = ptr.add(1);
            }
        }
        4 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 =
                    HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                ZSTD_memcpy(
                    DTableRank.add(0) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(2) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                DTableRank = DTableRank.add(4);
                ptr = ptr.add(1);
            }
        }
        8 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 =
                    HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                ZSTD_memcpy(
                    DTableRank.add(0) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(2) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(4) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(6) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                DTableRank = DTableRank.add(8);
                ptr = ptr.add(1);
            }
        }
        _ => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 =
                    HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                let DTableRankEnd: *mut HUF_DEltX2 = DTableRank.add(length as usize);
                while DTableRank != DTableRankEnd {
                    ZSTD_memcpy(
                        DTableRank.add(0) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.add(2) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.add(4) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.add(6) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    DTableRank = DTableRank.add(8);
                }
                ptr = ptr.add(1);
            }
        }
    }
}

/* HUF_fillDTableX2Level2() */
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
    /* Fill skipped values. */
    if minWeight > 1 {
        let length: U32 = 1u32 << ((targetLog - consumedBits) & 0x1F);
        let DEltX2: U64 = HUF_buildDEltX2U64(baseSeq as U32, consumedBits, 0, 1);
        let skipSize: c_int = *rankVal.add(minWeight as usize) as c_int;
        match length {
            2 => {
                ZSTD_memcpy(
                    DTable as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
            }
            4 => {
                ZSTD_memcpy(
                    DTable.add(0) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTable.add(2) as *mut u8,
                    &DEltX2 as *const U64 as *const u8,
                    core::mem::size_of::<U64>(),
                );
            }
            _ => {
                let mut i: c_int = 0;
                while i < skipSize {
                    ZSTD_memcpy(
                        DTable.offset((i + 0) as isize) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset((i + 2) as isize) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset((i + 4) as isize) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset((i + 6) as isize) as *mut u8,
                        &DEltX2 as *const U64 as *const u8,
                        core::mem::size_of::<U64>(),
                    );
                    i += 8;
                }
            }
        }
    }

    /* Fill each of the second level symbols by weight. */
    {
        let mut w: c_int = minWeight;
        while w < maxWeight1 {
            let begin: c_int = *rankStart.add(w as usize) as c_int;
            let end: c_int = *rankStart.add((w + 1) as usize) as c_int;
            let nbBits: U32 = nbBitsBaseline - w as U32;
            let totalBits: U32 = nbBits + consumedBits;
            HUF_fillDTableX2ForWeight(
                DTable.offset(*rankVal.add(w as usize) as isize),
                sortedSymbols.offset(begin as isize),
                sortedSymbols.offset(end as isize),
                totalBits,
                targetLog,
                baseSeq,
                2,
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
    let rankVal: *mut U32 = (*rankValOrigin.add(0)).as_mut_ptr();
    let scaleLog: c_int = nbBitsBaseline as c_int - targetLog as c_int; /* scaleLog <= 1 */
    let minBits: U32 = nbBitsBaseline - maxWeight;
    let mut w: c_int;
    let wEnd: c_int = maxWeight as c_int + 1;

    /* Fill DTable in order of weight. */
    w = 1;
    while w < wEnd {
        let begin: c_int = *rankStart.add(w as usize) as c_int;
        let end: c_int = *rankStart.add((w + 1) as usize) as c_int;
        let nbBits: U32 = nbBitsBaseline - w as U32;

        if targetLog as c_int - nbBits as c_int >= minBits as c_int {
            /* Enough room for a second symbol. */
            let mut start: c_int = *rankVal.add(w as usize) as c_int;
            let length: U32 = 1u32 << ((targetLog - nbBits) & 0x1F);
            let mut minWeight: c_int = nbBits as c_int + scaleLog;
            let mut s: c_int;
            if minWeight < 1 {
                minWeight = 1;
            }
            s = begin;
            while s != end {
                HUF_fillDTableX2Level2(
                    DTable.offset(start as isize),
                    targetLog,
                    nbBits,
                    (*rankValOrigin.add(nbBits as usize)).as_ptr(),
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
                0,
                1,
            );
        }
        w += 1;
    }
}

#[repr(C)]
pub struct HUF_ReadDTableX2_Workspace {
    pub rankVal: [rankValCol_t; HUF_TABLELOG_MAX as usize],
    pub rankStats: [U32; (HUF_TABLELOG_MAX + 1) as usize],
    pub rankStart0: [U32; (HUF_TABLELOG_MAX + 3) as usize],
    pub sortedSymbol: [sortedSymbol_t; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    pub weightList: [BYTE; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    pub calleeWksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32 as usize],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX2_wksp(
    DTable: *mut HUF_DTable,
    src: *const c_void,
    srcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    let mut tableLog: U32 = 0;
    let mut maxW: U32;
    let mut nbSymbols: U32 = 0;
    let mut dtd: DTableDesc = HUF_getDTableDesc(DTable);
    let mut maxTableLog: U32 = dtd.maxTableLog as U32;
    let iSize: size_t;
    let dtPtr: *mut c_void = DTable.add(1) as *mut c_void;
    let dt: *mut HUF_DEltX2 = dtPtr as *mut HUF_DEltX2;
    let rankStart: *mut U32;

    let wksp: *mut HUF_ReadDTableX2_Workspace = workSpace as *mut HUF_ReadDTableX2_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX2_Workspace>() > wkspSize {
        return ERROR(ZSTD_error_GENERIC);
    }

    rankStart = (*wksp).rankStart0.as_mut_ptr().add(1);
    ZSTD_memset(
        (*wksp).rankStats.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of_val(&(*wksp).rankStats),
    );
    ZSTD_memset(
        (*wksp).rankStart0.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of_val(&(*wksp).rankStart0),
    );

    if maxTableLog > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    iSize = HUF_readStats_wksp(
        (*wksp).weightList.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as size_t,
        (*wksp).rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
        (*wksp).calleeWksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).calleeWksp),
        flags,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > maxTableLog {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    if tableLog <= HUF_DECODER_FAST_TABLELOG && maxTableLog > HUF_DECODER_FAST_TABLELOG {
        maxTableLog = HUF_DECODER_FAST_TABLELOG;
    }

    /* find maxWeight */
    maxW = tableLog;
    while (*wksp).rankStats[maxW as usize] == 0 {
        maxW -= 1;
    }

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w < maxW + 1 {
            let curr: U32 = nextRankStart;
            nextRankStart += (*wksp).rankStats[w as usize];
            *rankStart.add(w as usize) = curr;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart; /* put all 0w symbols at the end of sorted list */
        *rankStart.add((maxW + 1) as usize) = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = (*wksp).weightList[s as usize] as U32;
            let r: U32 = *rankStart.add(w as usize);
            *rankStart.add(w as usize) = r + 1;
            (*wksp).sortedSymbol[r as usize].symbol = s as BYTE;
            s += 1;
        }
        *rankStart.add(0) = 0; /* forget 0w symbols; beginning of weight(1) */
    }

    /* Build rankVal */
    {
        let rankVal0: *mut U32 = (*wksp).rankVal[0].as_mut_ptr();
        {
            let rescale: c_int = (maxTableLog as c_int - tableLog as c_int) - 1;
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let curr: U32 = nextRankVal;
                nextRankVal = nextRankVal
                    .wrapping_add((*wksp).rankStats[w as usize] << (w as c_int + rescale));
                *rankVal0.add(w as usize) = curr;
                w += 1;
            }
        }
        {
            let minBits: U32 = tableLog + 1 - maxW;
            let mut consumed: U32 = minBits;
            while consumed < maxTableLog - minBits + 1 {
                let rankValPtr: *mut U32 = (*wksp).rankVal[consumed as usize].as_mut_ptr();
                let mut w: U32 = 1;
                while w < maxW + 1 {
                    *rankValPtr.add(w as usize) = *rankVal0.add(w as usize) >> consumed;
                    w += 1;
                }
                consumed += 1;
            }
        }
    }

    HUF_fillDTableX2(
        dt,
        maxTableLog,
        (*wksp).sortedSymbol.as_ptr(),
        (*wksp).rankStart0.as_ptr(),
        (*wksp).rankVal.as_mut_ptr(),
        maxW,
        tableLog + 1,
    );

    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1;
    ZSTD_memcpy(
        DTable as *mut u8,
        &dtd as *const DTableDesc as *const u8,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

#[inline(always)]
pub unsafe fn HUF_decodeSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val: size_t = BIT_lookBitsFast(DStream, dtLog) as size_t; /* note : dtLog >= 1 */
    ZSTD_memcpy(
        op as *mut u8,
        &(*dt.add(val)).sequence as *const U16 as *const u8,
        2,
    );
    BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    (*dt.add(val)).length as U32
}

#[inline(always)]
pub unsafe fn HUF_decodeLastSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val: size_t = BIT_lookBitsFast(DStream, dtLog) as size_t; /* note : dtLog >= 1 */
    ZSTD_memcpy(
        op as *mut u8,
        &(*dt.add(val)).sequence as *const U16 as *const u8,
        1,
    );
    if (*dt.add(val)).length == 1 {
        BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
    } else {
        if (*DStream).bitsConsumed
            < (core::mem::size_of_val(&(*DStream).bitContainer) * 8) as U32
        {
            BIT_skipBits(DStream, (*dt.add(val)).nbBits as U32);
            if (*DStream).bitsConsumed
                > (core::mem::size_of_val(&(*DStream).bitContainer) * 8) as U32
            {
                /* ugly hack; works only because it's the last symbol. */
                (*DStream).bitsConsumed =
                    (core::mem::size_of_val(&(*DStream).bitContainer) * 8) as U32;
            }
        }
    }
    1
}

/* HUF_DECODE_SYMBOLX2_0 : ptr += HUF_decodeSymbolX2(ptr, ...) */
macro_rules! HUF_DECODE_SYMBOLX2_0 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        $ptr = $ptr.add(HUF_decodeSymbolX2($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize);
    }};
}

macro_rules! HUF_DECODE_SYMBOLX2_1 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 || (HUF_TABLELOG_MAX <= 12) {
            $ptr = $ptr
                .add(HUF_decodeSymbolX2($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize);
        }
    }};
}

macro_rules! HUF_DECODE_SYMBOLX2_2 {
    ($ptr:expr, $DStreamPtr:expr, $dt:expr, $dtLog:expr) => {{
        if MEM_64bits() != 0 {
            $ptr = $ptr
                .add(HUF_decodeSymbolX2($ptr as *mut c_void, $DStreamPtr, $dt, $dtLog) as usize);
        }
    }};
}

#[inline]
pub unsafe fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;

    /* up to 8 symbols at a time */
    if (pEnd as isize - p as isize) as size_t
        >= core::mem::size_of_val(&(*bitDPtr).bitContainer)
    {
        if dtLog <= 11 && MEM_64bits() != 0 {
            /* up to 10 symbols at a time */
            while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
                & ((p as *const BYTE) < pEnd.sub(9) as *const BYTE) as c_int)
                != 0
            {
                HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
            }
        } else {
            /* up to 8 symbols at a time */
            while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
                & ((p as *const BYTE)
                    < pEnd.sub(core::mem::size_of_val(&(*bitDPtr).bitContainer) - 1)
                        as *const BYTE) as c_int)
                != 0
            {
                HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_1!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
            }
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }

    /* closer to end : up to 2 symbols at a time */
    if (pEnd as isize - p as isize) as size_t >= 2 {
        while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
            & ((p as *const BYTE) <= pEnd.sub(2) as *const BYTE) as c_int)
            != 0
        {
            HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
        }

        while (p as *const BYTE) <= pEnd.sub(2) as *const BYTE {
            HUF_DECODE_SYMBOLX2_0!(p, bitDPtr, dt, dtLog);
        }
    }

    if (p as *const BYTE) < pEnd as *const BYTE {
        p = p.add(HUF_decodeLastSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    (p as isize - pStart as isize) as size_t
}

#[inline(always)]
pub unsafe fn HUF_decompress1X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    let mut bitD: BIT_DStream_t = new_dstream();

    /* Init */
    {
        let e = BIT_initDStream(&mut bitD, cSrc as *const u8, cSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    /* decode */
    {
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(ostart, dstSize as isize);
        let dtPtr: *const c_void = DTable.add(1) as *const c_void;
        let dt: *const HUF_DEltX2 = dtPtr as *const HUF_DEltX2;
        let dtd: DTableDesc = HUF_getDTableDesc(DTable);
        HUF_decodeStreamX2(ostart, &mut bitD, oend, dt, dtd.tableLog as U32);
    }

    /* check */
    if BIT_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
    dstSize
}

/* HUF_decompress4X2_usingDTable_internal_body(): @dstSize >= 6 */
#[inline(always)]
pub unsafe fn HUF_decompress4X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    if cSrcSize < 10 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if dstSize < 6 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart: *const BYTE = cSrc as *const BYTE;
        let ostart: *mut BYTE = dst as *mut BYTE;
        let oend: *mut BYTE = ostart.add(dstSize);
        let olimit: *mut BYTE = oend.sub(core::mem::size_of::<size_t>() - 1);
        let dtPtr: *const c_void = DTable.add(1) as *const c_void;
        let dt: *const HUF_DEltX2 = dtPtr as *const HUF_DEltX2;

        let mut bitD1: BIT_DStream_t = new_dstream();
        let mut bitD2: BIT_DStream_t = new_dstream();
        let mut bitD3: BIT_DStream_t = new_dstream();
        let mut bitD4: BIT_DStream_t = new_dstream();
        let length1: size_t = MEM_readLE16(istart) as size_t;
        let length2: size_t = MEM_readLE16(istart.add(2)) as size_t;
        let length3: size_t = MEM_readLE16(istart.add(4)) as size_t;
        let length4: size_t = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1: *const BYTE = istart.add(6); /* jumpTable */
        let istart2: *const BYTE = istart1.add(length1);
        let istart3: *const BYTE = istart2.add(length2);
        let istart4: *const BYTE = istart3.add(length3);
        let segmentSize: size_t = (dstSize + 3) / 4;
        let opStart2: *mut BYTE = ostart.add(segmentSize);
        let opStart3: *mut BYTE = opStart2.add(segmentSize);
        let opStart4: *mut BYTE = opStart3.add(segmentSize);
        let mut op1: *mut BYTE = ostart;
        let mut op2: *mut BYTE = opStart2;
        let mut op3: *mut BYTE = opStart3;
        let mut op4: *mut BYTE = opStart4;
        let mut endSignal: U32 = 1;
        let dtd: DTableDesc = HUF_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if opStart4 > oend {
            return ERROR(ZSTD_error_corruption_detected);
        }
        {
            let e = BIT_initDStream(&mut bitD1, istart1, length1);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD2, istart2, length2);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD3, istart3, length3);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD4, istart4, length4);
            if ERR_isError(e) != 0 {
                return e;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        /* Non-clang path (the C build here is gcc-style; but we reproduce the
         * portable #else branch which is semantically identical). */
        if (oend as isize - op4 as isize) as size_t >= core::mem::size_of::<size_t>() {
            while (endSignal & ((op4 as *const BYTE) < olimit as *const BYTE) as U32) != 0 {
                HUF_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX2_1!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_1!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_1!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_1!(op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_2!(op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0!(op4, &mut bitD4, dt, dtLog);
                endSignal = ((BIT_reloadDStreamFast(&mut bitD1) == BIT_DStream_unfinished) as U32
                    & (BIT_reloadDStreamFast(&mut bitD2) == BIT_DStream_unfinished) as U32
                    & (BIT_reloadDStreamFast(&mut bitD3) == BIT_DStream_unfinished) as U32
                    & (BIT_reloadDStreamFast(&mut bitD4) == BIT_DStream_unfinished) as U32)
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
        /* note : op4 already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        {
            let endCheck: U32 = BIT_endOfDStream(&bitD1)
                & BIT_endOfDStream(&bitD2)
                & BIT_endOfDStream(&bitD3)
                & BIT_endOfDStream(&bitD4);
            if endCheck == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }

        /* decoded size */
        dstSize
    }
}

/* HUF_NEED_BMI2_FUNCTION == 0 : no _bmi2 variant. */

pub unsafe fn HUF_decompress4X2_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
) -> size_t {
    HUF_decompress4X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 : no asm loop declared. */

pub unsafe fn HUF_decompress4X2_usingDTable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let mut oend: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable: *const HUF_DEltX2 = (*args).dt as *const HUF_DEltX2;
    let ilowest: *const BYTE = (*args).ilowest;

    /* Copy the arguments to local registers. */
    ZSTD_memcpy(
        bits.as_mut_ptr() as *mut u8,
        (*args).bits.as_ptr() as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        ip.as_mut_ptr() as *mut u8,
        (*args).ip.as_ptr() as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        op.as_mut_ptr() as *mut u8,
        (*args).op.as_ptr() as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );

    oend[0] = op[1];
    oend[1] = op[2];
    oend[2] = op[3];
    oend[3] = (*args).oend;

    'outer: loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            let mut iters: size_t = ((ip[0] as isize - ilowest as isize) as size_t) / 7;
            stream = 0;
            while stream < 4 {
                let oiters: size_t =
                    ((oend[stream as usize] as isize - op[stream as usize] as isize) as size_t)
                        / 10;
                iters = MIN(iters, oiters);
                stream += 1;
            }

            olimit = op[3].add(iters * 5);

            /* Exit the fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break 'outer;
            }

            /* Exit the decoding loop if any input pointer has crossed the previous one. */
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'outer;
                }
                stream += 1;
            }
        }

        macro_rules! HUF_4X2_DECODE_SYMBOL {
            ($stream:expr, $decode3:expr) => {{
                if ($decode3 != 0) || ($stream != 3) {
                    let index: c_int = (bits[$stream] >> 53) as c_int;
                    let entry: HUF_DEltX2 = *dtable.offset(index as isize);
                    MEM_write16(op[$stream], entry.sequence);
                    bits[$stream] <<= (entry.nbBits & 0x3F) as U64;
                    op[$stream] = op[$stream].add(entry.length as usize);
                }
            }};
        }

        macro_rules! HUF_4X2_RELOAD_STREAM {
            ($stream:expr) => {{
                HUF_4X2_DECODE_SYMBOL!(3, 1);
                {
                    let ctz: c_int = ZSTD_countTrailingZeros64(bits[$stream]) as c_int;
                    let nbBits: c_int = ctz & 7;
                    let nbBytes: c_int = ctz >> 3;
                    ip[$stream] = ip[$stream].offset(-(nbBytes as isize));
                    bits[$stream] = MEM_read64(ip[$stream]) | 1;
                    bits[$stream] <<= nbBits as U64;
                }
            }};
        }

        /* Manually unroll the loop. */
        loop {
            /* Decode 5 symbols from each of the first 3 streams. */
            HUF_4X2_DECODE_SYMBOL!(0, 0);
            HUF_4X2_DECODE_SYMBOL!(1, 0);
            HUF_4X2_DECODE_SYMBOL!(2, 0);
            HUF_4X2_DECODE_SYMBOL!(3, 0);
            HUF_4X2_DECODE_SYMBOL!(0, 0);
            HUF_4X2_DECODE_SYMBOL!(1, 0);
            HUF_4X2_DECODE_SYMBOL!(2, 0);
            HUF_4X2_DECODE_SYMBOL!(3, 0);
            HUF_4X2_DECODE_SYMBOL!(0, 0);
            HUF_4X2_DECODE_SYMBOL!(1, 0);
            HUF_4X2_DECODE_SYMBOL!(2, 0);
            HUF_4X2_DECODE_SYMBOL!(3, 0);
            HUF_4X2_DECODE_SYMBOL!(0, 0);
            HUF_4X2_DECODE_SYMBOL!(1, 0);
            HUF_4X2_DECODE_SYMBOL!(2, 0);
            HUF_4X2_DECODE_SYMBOL!(3, 0);
            HUF_4X2_DECODE_SYMBOL!(0, 0);
            HUF_4X2_DECODE_SYMBOL!(1, 0);
            HUF_4X2_DECODE_SYMBOL!(2, 0);
            HUF_4X2_DECODE_SYMBOL!(3, 0);

            /* Decode one symbol from the final stream */
            HUF_4X2_DECODE_SYMBOL!(3, 1);

            /* Decode 4 symbols from the final stream & reload bitstreams. */
            HUF_4X2_RELOAD_STREAM!(0);
            HUF_4X2_RELOAD_STREAM!(1);
            HUF_4X2_RELOAD_STREAM!(2);
            HUF_4X2_RELOAD_STREAM!(3);

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* Save the final values back to args. */
    ZSTD_memcpy(
        (*args).bits.as_mut_ptr() as *mut u8,
        bits.as_ptr() as *const u8,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        (*args).ip.as_mut_ptr() as *mut u8,
        ip.as_ptr() as *const u8,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        (*args).op.as_mut_ptr() as *mut u8,
        op.as_ptr() as *const u8,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );
}

pub unsafe fn HUF_decompress4X2_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> size_t {
    let dt: *const c_void = DTable.add(1) as *const c_void;
    let _ilowest: *const BYTE = cSrc as *const BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);
    let mut args: HUF_DecompressFastArgs = new_fast_args();
    {
        let ret: size_t =
            HUF_DecompressFastArgs_init(&mut args, dst, dstSize, cSrc, cSrcSize, DTable);
        if ERR_isError(ret) != 0 {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(&mut args);

    /* finish bitStreams one by one */
    {
        let segmentSize: size_t = (dstSize + 3) / 4;
        let mut segmentEnd: *mut BYTE = dst as *mut BYTE;
        let mut i: c_int = 0;
        while i < 4 {
            let mut bit: BIT_DStream_t = new_dstream();
            if segmentSize <= (oend as isize - segmentEnd as isize) as size_t {
                segmentEnd = segmentEnd.add(segmentSize);
            } else {
                segmentEnd = oend;
            }
            {
                let e = HUF_initRemainingDStream(&mut bit, &args, i, segmentEnd);
                if ERR_isError(e) != 0 {
                    return e;
                }
            }
            args.op[i as usize] = args.op[i as usize].add(HUF_decodeStreamX2(
                args.op[i as usize],
                &mut bit,
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
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> size_t {
    let fallbackFn: HUF_DecompressUsingDTableFn = HUF_decompress4X2_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn = HUF_decompress4X2_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 : no dispatch overrides. */

    if HUF_ENABLE_FAST_DECODE != 0 && (flags & HUF_flags_disableFast as c_int) == 0 {
        let ret: size_t =
            HUF_decompress4X2_usingDTable_internal_fast(dst, dstSize, cSrc, cSrcSize, DTable, loopFn);
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* HUF_DGEN(HUF_decompress1X2_usingDTable_internal) with DYNAMIC_BMI2==0 */
pub unsafe fn HUF_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> size_t {
    let _ = flags;
    HUF_decompress1X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X2_DCtx_wksp(
    DCtx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: size_t = HUF_readDTableX2_wksp(DCtx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress1X2_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, DCtx, flags)
}

pub unsafe fn HUF_decompress4X2_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: size_t = HUF_readDTableX2_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X2_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx, flags)
}

/* ***********************************/
/* Universal decompression selectors */
/* ***********************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

static algoTime: [[algo_time_t; 2]; 16] = [
    [
        algo_time_t { tableTime: 0, decode256Time: 0 },
        algo_time_t { tableTime: 1, decode256Time: 1 },
    ], /* Q==0 : impossible */
    [
        algo_time_t { tableTime: 0, decode256Time: 0 },
        algo_time_t { tableTime: 1, decode256Time: 1 },
    ], /* Q==1 : impossible */
    [
        algo_time_t { tableTime: 150, decode256Time: 216 },
        algo_time_t { tableTime: 381, decode256Time: 119 },
    ], /* Q == 2 */
    [
        algo_time_t { tableTime: 170, decode256Time: 205 },
        algo_time_t { tableTime: 514, decode256Time: 112 },
    ], /* Q == 3 */
    [
        algo_time_t { tableTime: 177, decode256Time: 199 },
        algo_time_t { tableTime: 539, decode256Time: 110 },
    ], /* Q == 4 */
    [
        algo_time_t { tableTime: 197, decode256Time: 194 },
        algo_time_t { tableTime: 644, decode256Time: 107 },
    ], /* Q == 5 */
    [
        algo_time_t { tableTime: 221, decode256Time: 192 },
        algo_time_t { tableTime: 735, decode256Time: 107 },
    ], /* Q == 6 */
    [
        algo_time_t { tableTime: 256, decode256Time: 189 },
        algo_time_t { tableTime: 881, decode256Time: 106 },
    ], /* Q == 7 */
    [
        algo_time_t { tableTime: 359, decode256Time: 188 },
        algo_time_t { tableTime: 1167, decode256Time: 109 },
    ], /* Q == 8 */
    [
        algo_time_t { tableTime: 582, decode256Time: 187 },
        algo_time_t { tableTime: 1570, decode256Time: 114 },
    ], /* Q == 9 */
    [
        algo_time_t { tableTime: 688, decode256Time: 187 },
        algo_time_t { tableTime: 1712, decode256Time: 122 },
    ], /* Q ==10 */
    [
        algo_time_t { tableTime: 825, decode256Time: 186 },
        algo_time_t { tableTime: 1965, decode256Time: 136 },
    ], /* Q ==11 */
    [
        algo_time_t { tableTime: 976, decode256Time: 185 },
        algo_time_t { tableTime: 2131, decode256Time: 150 },
    ], /* Q ==12 */
    [
        algo_time_t { tableTime: 1180, decode256Time: 186 },
        algo_time_t { tableTime: 2070, decode256Time: 175 },
    ], /* Q ==13 */
    [
        algo_time_t { tableTime: 1377, decode256Time: 185 },
        algo_time_t { tableTime: 1731, decode256Time: 202 },
    ], /* Q ==14 */
    [
        algo_time_t { tableTime: 1412, decode256Time: 185 },
        algo_time_t { tableTime: 1695, decode256Time: 202 },
    ], /* Q ==15 */
];

/** HUF_selectDecoder() : */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_selectDecoder(dstSize: size_t, cSrcSize: size_t) -> U32 {
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
        DTime1 = DTime1.wrapping_add(DTime1 >> 5);
        (DTime1 < DTime0) as U32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    /* validation checks */
    if dstSize == 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if cSrcSize > dstSize {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if cSrcSize == dstSize {
        ZSTD_memcpy(dst as *mut u8, cSrc as *const u8, dstSize);
        return dstSize;
    } /* not compressed */
    if cSrcSize == 1 {
        ZSTD_memset(dst as *mut u8, *(cSrc as *const BYTE) as c_int, dstSize);
        return dstSize;
    } /* RLE */

    {
        let algoNb: U32 = HUF_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUF_decompress1X2_DCtx_wksp(dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags)
        } else {
            HUF_decompress1X1_DCtx_wksp(dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_usingDTable(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> size_t {
    let dtd: DTableDesc = HUF_getDTableDesc(DTable);
    if dtd.tableType != 0 {
        HUF_decompress1X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    } else {
        HUF_decompress1X1_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X1_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: size_t,
    cSrc: *const c_void,
    mut cSrcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;

    let hSize: size_t = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress1X1_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx, flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_usingDTable(
    dst: *mut c_void,
    maxDstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> size_t {
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
    dstSize: size_t,
    cSrc: *const c_void,
    cSrcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
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
            HUF_decompress4X2_DCtx_wksp(dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags)
        } else {
            HUF_decompress4X1_DCtx_wksp(dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags)
        }
    }
}

/* ---- small local constructors (replace C zero-init / uninitialized locals) ---- */

#[inline(always)]
fn new_dstream() -> BIT_DStream_t {
    BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: core::ptr::null(),
        start: core::ptr::null(),
        limitPtr: core::ptr::null(),
    }
}

#[inline(always)]
fn new_fast_args() -> HUF_DecompressFastArgs {
    HUF_DecompressFastArgs {
        ip: [core::ptr::null(); 4],
        op: [core::ptr::null_mut(); 4],
        bits: [0; 4],
        dt: core::ptr::null(),
        ilowest: core::ptr::null(),
        oend: core::ptr::null_mut(),
        iend: [core::ptr::null(); 4],
    }
}
