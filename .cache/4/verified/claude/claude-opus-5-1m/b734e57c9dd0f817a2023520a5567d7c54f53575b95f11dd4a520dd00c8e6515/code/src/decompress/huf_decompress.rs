//! Translation of `decompress/huf_decompress.c`
//!
//! Build configuration for this port:
//!   * `DYNAMIC_BMI2 == 0`  -> only the `_default` (non-bmi2) bodies exist.
//!   * `HUF_ASM_X86_64_BMI2 == 0` / `ZSTD_ENABLE_ASM_X86_64_BMI2 == 0` -> no asm loops.
//!   * neither `HUF_FORCE_DECOMPRESS_X1` nor `HUF_FORCE_DECOMPRESS_X2` defined ->
//!     both decoders are compiled in and `HUF_selectDecoder` is used.
//!   * `DEBUGLEVEL == 0` -> asserts / DEBUGLOG dropped.
#![allow(dead_code)]

use crate::common::bits::*;
use crate::common::bitstream::*;
use crate::common::entropy_common::HUF_readStats_wksp;
use crate::common::error_private::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_internal::MIN;
use crate::libc::{ZSTD_memcpy, ZSTD_memset};
use core::ffi::{c_char, c_int, c_void};

/* **************************************************************
 *  Constants
 ****************************************************************/

const HUF_DECODER_FAST_TABLELOG: U32 = 11;

/* `HUF_DISABLE_FAST_DECODE` is not defined */
const HUF_ENABLE_FAST_DECODE: c_int = 1;

/* **************************************************************
 *  compiler.h helpers used by this file
 ****************************************************************/

/// `ZSTD_maybeNullPtrAdd()` : `ptr + add`, except that `NULL + 0 == NULL`.
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
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize;

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct DTableDesc {
    maxTableLog: BYTE,
    tableType: BYTE,
    tableLog: BYTE,
    reserved: BYTE,
}

unsafe fn HUF_getDTableDesc(table: *const HUF_DTable) -> DTableDesc {
    let mut dtd = DTableDesc::default();
    ZSTD_memcpy(
        &mut dtd as *mut DTableDesc as *mut c_void,
        table as *const c_void,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

unsafe fn HUF_initFastDStream(ip: *const BYTE) -> usize {
    let lastByte: BYTE = *ip.add(7);
    let bitsConsumed: usize = if lastByte != 0 {
        (8 - ZSTD_highbit32(lastByte as U32)) as usize
    } else {
        0
    };
    let value: usize = MEM_readLEST(ip as *const c_void) | 1;
    value << bitsConsumed
}

/**
 * The input/output arguments to the Huffman fast decoding loop:
 *
 * ip [in/out] - The input pointers, must be updated to reflect what is consumed.
 * op [in/out] - The output pointers, must be updated to reflect what is written.
 * bits [in/out] - The bitstream containers, must be updated to reflect the current state.
 * dt [in] - The decoding table.
 * ilowest [in] - The beginning of the valid range of the input. Decoders may read
 *                down to this pointer. It may be below iend[0].
 * oend [in] - The end of the output stream. op[3] must not cross oend.
 * iend [in] - The end of each input stream. ip[i] may cross iend[i],
 *             as long as it is above ilowest, but that indicates corruption.
 */
#[repr(C)]
#[derive(Copy, Clone)]
struct HUF_DecompressFastArgs {
    ip: [*const BYTE; 4],
    op: [*mut BYTE; 4],
    bits: [U64; 4],
    dt: *const c_void,
    ilowest: *const BYTE,
    oend: *mut BYTE,
    iend: [*const BYTE; 4],
}

impl Default for HUF_DecompressFastArgs {
    fn default() -> Self {
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
}

type HUF_DecompressFastLoopFn = unsafe fn(args: *mut HUF_DecompressFastArgs);

/**
 * Initializes args for the fast decoding loop.
 * @returns 1 on success
 *          0 if the fallback implementation should be used.
 *          Or an error code on failure.
 */
unsafe fn HUF_DecompressFastArgs_init(
    args: *mut HUF_DecompressFastArgs,
    dst: *mut c_void,
    dstSize: usize,
    src: *const c_void,
    srcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let dt = DTable.add(1) as *const c_void;
    let dtLog: U32 = HUF_getDTableDesc(DTable).tableLog as U32;

    let istart = src as *const BYTE;

    let oend = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);

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
     * If table log is not correct at this point, fallback to the old decoder.
     * On small inputs we don't have enough data to trigger the fast loop, so use
     * the old decoder.
     */
    if dtLog != HUF_DECODER_FAST_TABLELOG {
        return 0;
    }

    /* Read the jump table. */
    {
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize = srcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
        (*args).iend[0] = istart.add(6); /* jumpTable */
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

    /* bits[] is the bit container.
     * It is read from the MSB down to the LSB.
     * It is shifted left as it is read, and zeros are
     * shifted in. After the lowest valid bit a 1 is
     * set, so that CountTrailingZeros(bits[]) can be used
     * to count how many bits we've consumed.
     */
    (*args).bits[0] = HUF_initFastDStream((*args).ip[0]) as U64;
    (*args).bits[1] = HUF_initFastDStream((*args).ip[1]) as U64;
    (*args).bits[2] = HUF_initFastDStream((*args).ip[2]) as U64;
    (*args).bits[3] = HUF_initFastDStream((*args).ip[3]) as U64;

    /* The decoders must be sure to never read beyond ilowest.
     * This is lower than iend[0], but allowing decoders to read
     * down to ilowest can allow an extra iteration or two in the
     * fast loop.
     */
    (*args).ilowest = istart;

    (*args).oend = oend;
    (*args).dt = dt;

    1
}

unsafe fn HUF_initRemainingDStream(
    bit: *mut BIT_DStream_t,
    args: *const HUF_DecompressFastArgs,
    stream: c_int,
    segmentEnd: *mut BYTE,
) -> usize {
    let s = stream as usize;
    /* Validate that we haven't overwritten. */
    if (*args).op[s] > segmentEnd {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Validate that we haven't read beyond iend[].
     * Note that ip[] may be < iend[] because the MSB is
     * the next bit to read, and we may have consumed 100%
     * of the stream, so down to iend[i] - 8 is valid.
     */
    if (*args).ip[s] < (*args).iend[s].wrapping_sub(8) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* Construct the BIT_DStream_t. */
    (*bit).bitContainer = MEM_readLEST((*args).ip[s] as *const c_void);
    (*bit).bitsConsumed = ZSTD_countTrailingZeros64((*args).bits[s]);
    (*bit).start = (*args).ilowest as *const c_char;
    (*bit).limitPtr = (*bit).start.add(core::mem::size_of::<usize>());
    (*bit).ptr = (*args).ip[s] as *const c_char;

    0
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct HUF_DEltX1 {
    nbBits: BYTE,
    byte: BYTE,
}

/**
 * Packs 4 HUF_DEltX1 structs into a U64. This is used to lay down 4 entries at
 * a time.
 */
unsafe fn HUF_DEltX1_set4(symbol: BYTE, nbBits: BYTE) -> U64 {
    let mut D4: U64;
    if MEM_isLittleEndian() != 0 {
        D4 = (((symbol as U32) << 8).wrapping_add(nbBits as U32)) as U64;
    } else {
        D4 = ((symbol as U32).wrapping_add((nbBits as U32) << 8)) as U64;
    }
    D4 = D4.wrapping_mul(0x0001000100010001u64);
    D4
}

/**
 * Increase the tableLog to targetTableLog and rescales the stats.
 * If tableLog > targetTableLog this is a no-op.
 * @returns New tableLog
 */
unsafe fn HUF_rescaleStats(
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
        /* Increase the weight for all non-zero probability symbols by scale. */
        let mut s: U32 = 0;
        while s < nbSymbols {
            let add: BYTE = if *huffWeight.add(s as usize) == 0 {
                0
            } else {
                scale as BYTE
            };
            *huffWeight.add(s as usize) = (*huffWeight.add(s as usize)).wrapping_add(add);
            s += 1;
        }
        /* Update rankVal to reflect the new weights.
         * All weights except 0 get moved to weight + scale.
         * Weights [1, scale] are empty.
         */
        let mut s: U32 = targetTableLog;
        while s > scale {
            *rankVal.add(s as usize) = *rankVal.add((s - scale) as usize);
            s -= 1;
        }
        let mut s: U32 = scale;
        while s > 0 {
            *rankVal.add(s as usize) = 0;
            s -= 1;
        }
    }
    targetTableLog
}

#[repr(C)]
struct HUF_ReadDTableX1_Workspace {
    rankVal: [U32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1],
    rankStart: [U32; HUF_TABLELOG_ABSOLUTEMAX as usize + 1],
    statsWksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
    symbols: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1],
    huffWeight: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1],
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
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUF_DEltX1;
    let wksp = workSpace as *mut HUF_ReadDTableX1_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX1_Workspace>() > wkspSize {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }

    /* ZSTD_memset(huffWeight, 0, sizeof(huffWeight)); is not necessary */

    iSize = HUF_readStats_wksp(
        (*wksp).huffWeight.as_mut_ptr(),
        HUF_SYMBOLVALUE_MAX as usize + 1,
        (*wksp).rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
        (*wksp).statsWksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).statsWksp),
        flags,
    );
    if ERR_isError(iSize) != 0 {
        return iSize;
    }

    /* Table header */
    {
        let mut dtd = HUF_getDTableDesc(DTable);
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
            /* DTable too small, Huffman tree cannot fit in */
            return ERROR(ZSTD_error_tableLog_tooLarge);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as BYTE;
        ZSTD_memcpy(
            DTable as *mut c_void,
            &dtd as *const DTableDesc as *const c_void,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Compute symbols and rankStart given rankVal:
     *
     * rankVal already contains the number of values of each weight.
     *
     * symbols contains the symbols ordered by weight. First are the rankVal[0]
     * weight 0 symbols, followed by the rankVal[1] weight 1 symbols, and so on.
     * symbols[0] is filled (but unused) to avoid a branch.
     *
     * rankStart contains the offset where each rank belongs in the DTable.
     * rankStart[0] is not filled because there are no entries in the table for
     * weight 0.
     */
    {
        let mut n: c_int;
        let mut nextRankStart: U32 = 0;
        let unroll: c_int = 4;
        let nLimit: c_int = nbSymbols as c_int - unroll + 1;
        n = 0;
        while n < tableLog as c_int + 1 {
            let curr: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add((*wksp).rankVal[n as usize]);
            (*wksp).rankStart[n as usize] = curr;
            n += 1;
        }
        n = 0;
        while n < nLimit {
            let mut u: c_int = 0;
            while u < unroll {
                let w: usize = (*wksp).huffWeight[(n + u) as usize] as usize;
                let idx = (*wksp).rankStart[w];
                (*wksp).rankStart[w] = idx.wrapping_add(1);
                (*wksp).symbols[idx as usize] = (n + u) as BYTE;
                u += 1;
            }
            n += unroll;
        }
        while n < nbSymbols as c_int {
            let w: usize = (*wksp).huffWeight[n as usize] as usize;
            let idx = (*wksp).rankStart[w];
            (*wksp).rankStart[w] = idx.wrapping_add(1);
            (*wksp).symbols[idx as usize] = n as BYTE;
            n += 1;
        }
    }

    /* fill DTable
     * We fill all entries of each weight in order.
     * That way length is a constant for each iteration of the outer loop.
     * We can switch based on the length to a different inner loop which is
     * optimized for that particular case.
     */
    {
        let mut w: U32;
        let mut symbol: c_int = (*wksp).rankVal[0] as c_int;
        let mut rankStart: c_int = 0;
        w = 1;
        while w < tableLog + 1 {
            let symbolCount: c_int = (*wksp).rankVal[w as usize] as c_int;
            let length: c_int = (1i32 << w) >> 1;
            let mut uStart: c_int = rankStart;
            let nbBits: BYTE = (tableLog + 1 - w) as BYTE;
            let mut s: c_int;
            let mut u: c_int;
            match length {
                1 => {
                    s = 0;
                    while s < symbolCount {
                        let mut D = HUF_DEltX1::default();
                        D.byte = (*wksp).symbols[(symbol + s) as usize];
                        D.nbBits = nbBits;
                        *dt.offset(uStart as isize) = D;
                        uStart += 1;
                        s += 1;
                    }
                }
                2 => {
                    s = 0;
                    while s < symbolCount {
                        let mut D = HUF_DEltX1::default();
                        D.byte = (*wksp).symbols[(symbol + s) as usize];
                        D.nbBits = nbBits;
                        *dt.offset(uStart as isize + 0) = D;
                        *dt.offset(uStart as isize + 1) = D;
                        uStart += 2;
                        s += 1;
                    }
                }
                4 => {
                    s = 0;
                    while s < symbolCount {
                        let D4: U64 =
                            HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
                        MEM_write64(dt.offset(uStart as isize) as *mut c_void, D4);
                        uStart += 4;
                        s += 1;
                    }
                }
                8 => {
                    s = 0;
                    while s < symbolCount {
                        let D4: U64 =
                            HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
                        MEM_write64(dt.offset(uStart as isize) as *mut c_void, D4);
                        MEM_write64(dt.offset(uStart as isize + 4) as *mut c_void, D4);
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
                            let base = uStart as isize + u as isize;
                            MEM_write64(dt.offset(base + 0) as *mut c_void, D4);
                            MEM_write64(dt.offset(base + 4) as *mut c_void, D4);
                            MEM_write64(dt.offset(base + 8) as *mut c_void, D4);
                            MEM_write64(dt.offset(base + 12) as *mut c_void, D4);
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
unsafe fn HUF_decodeSymbolX1(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> BYTE {
    let val = BIT_lookBitsFast(Dstream, dtLog); /* note : dtLog >= 1 */
    let c: BYTE = (*dt.add(val as usize)).byte;
    BIT_skipBits(Dstream, (*dt.add(val as usize)).nbBits as U32);
    c
}

/* HUF_DECODE_SYMBOLX1_0 / _1 / _2 :
 * `_1` is guarded by `MEM_64bits() || (HUF_TABLELOG_MAX<=12)` and `_2` by
 * `MEM_64bits()`; both are always true for this build, so all three are the
 * same unconditional decode step. */
#[inline(always)]
unsafe fn HUF_DECODE_SYMBOLX1_0(
    ptr: &mut *mut BYTE,
    DStreamPtr: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) {
    **ptr = HUF_decodeSymbolX1(DStreamPtr, dt, dtLog);
    *ptr = (*ptr).add(1);
}

#[inline(always)]
unsafe fn HUF_decodeStreamX1(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    if pEnd.offset_from(p) > 3 {
        while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
            & ((p < pEnd.wrapping_sub(3)) as c_int))
            != 0
        {
            HUF_DECODE_SYMBOLX1_0(&mut p, bitDPtr, dt, dtLog);
            HUF_DECODE_SYMBOLX1_0(&mut p, bitDPtr, dt, dtLog);
            HUF_DECODE_SYMBOLX1_0(&mut p, bitDPtr, dt, dtLog);
            HUF_DECODE_SYMBOLX1_0(&mut p, bitDPtr, dt, dtLog);
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }

    /* [0-3] symbols remaining : the `MEM_32bits()` reload loop is compiled out */

    /* no more data to retrieve from bitstream, no need to reload */
    while p < pEnd {
        HUF_DECODE_SYMBOLX1_0(&mut p, bitDPtr, dt, dtLog);
    }

    pEnd.offset_from(pStart) as usize
}

unsafe fn HUF_decompress1X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let op = dst as *mut BYTE;
    let oend = ZSTD_maybeNullPtrAdd(op, dstSize as isize);
    let dtPtr = DTable.add(1) as *const c_void;
    let dt = dtPtr as *const HUF_DEltX1;
    let mut bitD = BIT_DStream_t::default();
    let dtd = HUF_getDTableDesc(DTable);
    let dtLog: U32 = dtd.tableLog as U32;

    {
        let e = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
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

/* HUF_decompress4X1_usingDTable_internal_body():
 * Conditions :
 * @dstSize >= 6
 */
unsafe fn HUF_decompress4X1_usingDTable_internal_body(
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
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let olimit = oend.wrapping_sub(3);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUF_DEltX1;

        /* Init */
        let mut bitD1 = BIT_DStream_t::default();
        let mut bitD2 = BIT_DStream_t::default();
        let mut bitD3 = BIT_DStream_t::default();
        let mut bitD4 = BIT_DStream_t::default();
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let dtd = HUF_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;
        let mut endSignal: U32 = 1;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        if opStart4 > oend {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let e = BIT_initDStream(&mut bitD1, istart1 as *const c_void, length1);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD2, istart2 as *const c_void, length2);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD3, istart3 as *const c_void, length3);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD4, istart4 as *const c_void, length4);
            if ERR_isError(e) != 0 {
                return e;
            }
        }

        /* up to 16 symbols per loop (4 symbols per stream) in 64-bit mode */
        if oend.offset_from(op4) as usize >= core::mem::size_of::<usize>() {
            while (endSignal & ((op4 < olimit) as U32)) != 0 {
                HUF_DECODE_SYMBOLX1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX1_0(&mut op4, &mut bitD4, dt, dtLog);
                endSignal &= (BIT_reloadDStreamFast(&mut bitD1) == BIT_DStream_unfinished) as U32;
                endSignal &= (BIT_reloadDStreamFast(&mut bitD2) == BIT_DStream_unfinished) as U32;
                endSignal &= (BIT_reloadDStreamFast(&mut bitD3) == BIT_DStream_unfinished) as U32;
                endSignal &= (BIT_reloadDStreamFast(&mut bitD4) == BIT_DStream_unfinished) as U32;
            }
        }

        /* check corruption
         * note : should not be necessary : op# advance in lock step, and we
         *        control op4. */
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

/* HUF_NEED_BMI2_FUNCTION == 0, so `..._internal_bmi2` is not compiled. */

unsafe fn HUF_decompress4X1_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    HUF_decompress4X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* ZSTD_ENABLE_ASM_X86_64_BMI2 == 0, so
 * `HUF_decompress4X1_usingDTable_internal_fast_asm_loop` is not declared. */

#[inline(always)]
unsafe fn HUF_4X1_DECODE_SYMBOL(
    bits: &mut [U64; 4],
    op: &[*mut BYTE; 4],
    dtable: *const U16,
    stream: usize,
    symbol: usize,
) {
    let index: c_int = (bits[stream] >> 53) as c_int;
    let entry: c_int = *dtable.add(index as usize) as c_int;
    bits[stream] = bits[stream].wrapping_shl((entry & 0x3F) as u32);
    *op[stream].add(symbol) = ((entry >> 8) & 0xFF) as BYTE;
}

#[inline(always)]
unsafe fn HUF_4X1_RELOAD_STREAM(
    bits: &mut [U64; 4],
    ip: &mut [*const BYTE; 4],
    op: &mut [*mut BYTE; 4],
    stream: usize,
) {
    let ctz: c_int = ZSTD_countTrailingZeros64(bits[stream]) as c_int;
    let nbBits: c_int = ctz & 7;
    let nbBytes: c_int = ctz >> 3;
    op[stream] = op[stream].add(5);
    ip[stream] = ip[stream].sub(nbBytes as usize);
    bits[stream] = MEM_read64(ip[stream] as *const c_void) | 1;
    bits[stream] = bits[stream].wrapping_shl(nbBits as u32);
}

unsafe fn HUF_decompress4X1_usingDTable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable = (*args).dt as *const U16;
    let oend = (*args).oend;
    let ilowest = (*args).ilowest;

    /* Copy the arguments to local variables */
    ZSTD_memcpy(
        bits.as_mut_ptr() as *mut c_void,
        (*args).bits.as_ptr() as *const c_void,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        ip.as_mut_ptr() as *mut c_void,
        (*args).ip.as_ptr() as *const c_void,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        op.as_mut_ptr() as *mut c_void,
        (*args).op.as_ptr() as *const c_void,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );

    loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            /* Each iteration produces 5 output symbols per stream */
            let oiters: usize = (oend.offset_from(op[3]) as usize) / 5;
            /* Each iteration consumes up to 11 bits * 5 = 55 bits < 7 bytes
             * per stream.
             */
            let iiters: usize = (ip[0].offset_from(ilowest) as usize) / 7;
            /* We can safely run iters iterations before running bounds checks */
            let iters: usize = MIN(oiters, iiters);
            let symbols: usize = iters * 5;

            /* We can simply check that op[3] < olimit, instead of checking all
             * of our bounds, since we can't hit the other bounds until we've run
             * iters iterations, which only happens when op[3] == olimit.
             */
            olimit = op[3].add(symbols);

            /* Exit fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break;
            }

            /* Exit the decoding loop if any input pointer has crossed the
             * previous one. This indicates corruption, and a precondition
             * to our loop is that ip[i] >= ip[0].
             */
            let mut crossed = false;
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    crossed = true;
                    break;
                }
                stream += 1;
            }
            if crossed {
                break; /* goto _out */
            }
        }

        /* Manually unrolled in the C source; the semantics are identical. */
        loop {
            /* Decode 5 symbols in each of the 4 streams */
            let mut symbol: usize = 0;
            while symbol < 5 {
                let mut st: usize = 0;
                while st < 4 {
                    HUF_4X1_DECODE_SYMBOL(&mut bits, &op, dtable, st, symbol);
                    st += 1;
                }
                symbol += 1;
            }

            /* Reload each of the 4 the bitstreams */
            let mut st: usize = 0;
            while st < 4 {
                HUF_4X1_RELOAD_STREAM(&mut bits, &mut ip, &mut op, st);
                st += 1;
            }

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* _out: save the final values of each of the state variables back to args. */
    ZSTD_memcpy(
        (*args).bits.as_mut_ptr() as *mut c_void,
        bits.as_ptr() as *const c_void,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        (*args).ip.as_mut_ptr() as *mut c_void,
        ip.as_ptr() as *const c_void,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        (*args).op.as_mut_ptr() as *mut c_void,
        op.as_ptr() as *const c_void,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );
}

/**
 * @returns @p dstSize on success (>= 6)
 *          0 if the fallback implementation should be used
 *          An error if an error occurred
 */
unsafe fn HUF_decompress4X1_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt = DTable.add(1) as *const c_void;
    let _ilowest = cSrc as *const BYTE;
    let oend = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);
    let mut args = HUF_DecompressFastArgs::default();
    {
        let ret = HUF_DecompressFastArgs_init(&mut args, dst, dstSize, cSrc, cSrcSize, DTable);
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
        let segmentSize: usize = (dstSize + 3) / 4;
        let mut segmentEnd = dst as *mut BYTE;
        let mut i: c_int = 0;
        while i < 4 {
            let mut bit = BIT_DStream_t::default();
            if segmentSize <= oend.offset_from(segmentEnd) as usize {
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
            /* Decompress and validate that we've produced exactly the expected length. */
            let produced = HUF_decodeStreamX1(
                args.op[i as usize],
                &mut bit,
                segmentEnd,
                dt as *const HUF_DEltX1,
                HUF_DECODER_FAST_TABLELOG,
            );
            args.op[i as usize] = args.op[i as usize].add(produced);
            if args.op[i as usize] != segmentEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            i += 1;
        }
    }

    /* decoded size */
    dstSize
}

/* HUF_DGEN(HUF_decompress1X1_usingDTable_internal) with DYNAMIC_BMI2 == 0 */
unsafe fn HUF_decompress1X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    _flags: c_int,
) -> usize {
    HUF_decompress1X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

unsafe fn HUF_decompress4X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let fallbackFn: HUF_DecompressUsingDTableFn = HUF_decompress4X1_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn = HUF_decompress4X1_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 : nothing to select */

    if HUF_ENABLE_FAST_DECODE != 0 && (flags & HUF_flags_disableFast) == 0 {
        let ret =
            HUF_decompress4X1_usingDTable_internal_fast(dst, dstSize, cSrc, cSrcSize, DTable, loopFn);
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

unsafe fn HUF_decompress4X1_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip = cSrc as *const BYTE;

    let hSize = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
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

/* *************************/
/* double-symbols decoding */
/* *************************/

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct HUF_DEltX2 {
    sequence: U16,
    nbBits: BYTE,
    length: BYTE,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct sortedSymbol_t {
    symbol: BYTE,
}

type rankValCol_t = [U32; HUF_TABLELOG_MAX as usize + 1];
type rankVal_t = [rankValCol_t; HUF_TABLELOG_MAX as usize];

/**
 * Constructs a HUF_DEltX2 in a U32.
 */
unsafe fn HUF_buildDEltX2U32(symbol: U32, nbBits: U32, baseSeq: U32, level: c_int) -> U32 {
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
unsafe fn HUF_buildDEltX2(symbol: U32, nbBits: U32, baseSeq: U32, level: c_int) -> HUF_DEltX2 {
    let mut DElt = HUF_DEltX2::default();
    let val: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq, level);
    ZSTD_memcpy(
        &mut DElt as *mut HUF_DEltX2 as *mut c_void,
        &val as *const U32 as *const c_void,
        core::mem::size_of::<U32>(),
    );
    DElt
}

/**
 * Constructs 2 HUF_DEltX2s and packs them into a U64.
 */
unsafe fn HUF_buildDEltX2U64(symbol: U32, nbBits: U32, baseSeq: U16, level: c_int) -> U64 {
    let DElt: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq as U32, level);
    (DElt as U64).wrapping_add((DElt as U64) << 32)
}

/**
 * Fills the DTable rank with all the symbols from [begin, end) that are each
 * nbBits long.
 */
unsafe fn HUF_fillDTableX2ForWeight(
    mut DTableRank: *mut HUF_DEltX2,
    begin: *const sortedSymbol_t,
    end: *const sortedSymbol_t,
    nbBits: U32,
    tableLog: U32,
    baseSeq: U16,
    level: c_int,
) {
    let length: U32 = 1u32.wrapping_shl(tableLog.wrapping_sub(nbBits) & 0x1F);
    let mut ptr: *const sortedSymbol_t;
    match length {
        1 => {
            ptr = begin;
            while ptr != end {
                let DElt = HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level);
                *DTableRank = DElt;
                DTableRank = DTableRank.add(1);
                ptr = ptr.add(1);
            }
        }
        2 => {
            ptr = begin;
            while ptr != end {
                let DElt = HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level);
                *DTableRank.add(0) = DElt;
                *DTableRank.add(1) = DElt;
                DTableRank = DTableRank.add(2);
                ptr = ptr.add(1);
            }
        }
        4 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 = HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                ZSTD_memcpy(
                    DTableRank.add(0) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(2) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                DTableRank = DTableRank.add(4);
                ptr = ptr.add(1);
            }
        }
        8 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 = HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                ZSTD_memcpy(
                    DTableRank.add(0) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(2) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(4) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTableRank.add(6) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                DTableRank = DTableRank.add(8);
                ptr = ptr.add(1);
            }
        }
        _ => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 = HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level);
                let DTableRankEnd = DTableRank.add(length as usize);
                while DTableRank != DTableRankEnd {
                    ZSTD_memcpy(
                        DTableRank.add(0) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.add(2) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.add(4) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTableRank.add(6) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    DTableRank = DTableRank.add(8);
                }
                ptr = ptr.add(1);
            }
        }
    }
}

/* HUF_fillDTableX2Level2() :
 * `rankValOrigin` must be a table of at least (HUF_TABLELOG_MAX + 1) U32 */
unsafe fn HUF_fillDTableX2Level2(
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
    /* Fill skipped values (all positions up to rankVal[minWeight]).
     * These are positions only get a single symbol because the combined weight
     * is too large.
     */
    if minWeight > 1 {
        let length: U32 = 1u32.wrapping_shl(targetLog.wrapping_sub(consumedBits) & 0x1F);
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
                    DTable as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
            }
            4 => {
                ZSTD_memcpy(
                    DTable.add(0) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
                ZSTD_memcpy(
                    DTable.add(2) as *mut c_void,
                    &DEltX2 as *const U64 as *const c_void,
                    core::mem::size_of::<U64>(),
                );
            }
            _ => {
                let mut i: c_int = 0;
                while i < skipSize {
                    ZSTD_memcpy(
                        DTable.offset(i as isize + 0) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset(i as isize + 2) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset(i as isize + 4) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
                        core::mem::size_of::<U64>(),
                    );
                    ZSTD_memcpy(
                        DTable.offset(i as isize + 6) as *mut c_void,
                        &DEltX2 as *const U64 as *const c_void,
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

unsafe fn HUF_fillDTableX2(
    DTable: *mut HUF_DEltX2,
    targetLog: U32,
    sortedList: *const sortedSymbol_t,
    rankStart: *const U32,
    rankValOrigin: *mut rankValCol_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let rankVal = rankValOrigin as *const U32;
    /* note : targetLog >= srcLog, hence scaleLog <= 1 */
    let scaleLog: c_int = nbBitsBaseline.wrapping_sub(targetLog) as c_int;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut w: c_int;
    let wEnd: c_int = maxWeight as c_int + 1;

    /* Fill DTable in order of weight. */
    w = 1;
    while w < wEnd {
        let begin: c_int = *rankStart.add(w as usize) as c_int;
        let end: c_int = *rankStart.add((w + 1) as usize) as c_int;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(w as U32);

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* Enough room for a second symbol. */
            let mut start: c_int = *rankVal.add(w as usize) as c_int;
            let length: U32 = 1u32.wrapping_shl(targetLog.wrapping_sub(nbBits) & 0x1F);
            let mut minWeight: c_int = nbBits as c_int + scaleLog;
            let mut s: c_int;
            if minWeight < 1 {
                minWeight = 1;
            }
            /* Fill the DTable for every symbol of weight w.
             * These symbols get at least 1 second symbol.
             */
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
struct HUF_ReadDTableX2_Workspace {
    rankVal: rankVal_t,
    rankStats: [U32; HUF_TABLELOG_MAX as usize + 1],
    rankStart0: [U32; HUF_TABLELOG_MAX as usize + 3],
    sortedSymbol: [sortedSymbol_t; HUF_SYMBOLVALUE_MAX as usize + 1],
    weightList: [BYTE; HUF_SYMBOLVALUE_MAX as usize + 1],
    calleeWksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
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
    let mut dtd = HUF_getDTableDesc(DTable);
    let mut maxTableLog: U32 = dtd.maxTableLog as U32;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void; /* force compiler to avoid strict-aliasing */
    let dt = dtPtr as *mut HUF_DEltX2;
    let rankStart: *mut U32;

    let wksp = workSpace as *mut HUF_ReadDTableX2_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX2_Workspace>() > wkspSize {
        return ERROR(ZSTD_error_GENERIC);
    }

    rankStart = (*wksp).rankStart0.as_mut_ptr().add(1);
    ZSTD_memset(
        (*wksp).rankStats.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of_val(&(*wksp).rankStats),
    );
    ZSTD_memset(
        (*wksp).rankStart0.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of_val(&(*wksp).rankStart0),
    );

    if maxTableLog > HUF_TABLELOG_MAX {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    /* ZSTD_memset(weightList, 0, sizeof(weightList)); is not necessary */

    iSize = HUF_readStats_wksp(
        (*wksp).weightList.as_mut_ptr(),
        HUF_SYMBOLVALUE_MAX as usize + 1,
        (*wksp).rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
        (*wksp).calleeWksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).calleeWksp),
        flags,
    );
    if ERR_isError(iSize) != 0 {
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
    while *(*wksp).rankStats.as_ptr().add(maxW as usize) == 0 {
        maxW = maxW.wrapping_sub(1);
    } /* necessarily finds a solution before 0 */

    /* Get start index of each weight */
    {
        let mut w: U32;
        let mut nextRankStart: U32 = 0;
        w = 1;
        while w < maxW + 1 {
            let curr: U32 = nextRankStart;
            nextRankStart = nextRankStart.wrapping_add((*wksp).rankStats[w as usize]);
            *rankStart.add(w as usize) = curr;
            w += 1;
        }
        /* put all 0w symbols at the end of sorted list */
        *rankStart.add(0) = nextRankStart;
        *rankStart.add((maxW + 1) as usize) = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: U32 = 0;
        while s < nbSymbols {
            let w: U32 = (*wksp).weightList[s as usize] as U32;
            let r: U32 = *rankStart.add(w as usize);
            *rankStart.add(w as usize) = r.wrapping_add(1);
            (*wksp).sortedSymbol[r as usize].symbol = s as BYTE;
            s += 1;
        }
        /* forget 0w symbols; this is beginning of weight(1) */
        *rankStart.add(0) = 0;
    }

    /* Build rankVal */
    {
        let rankVal0 = (*wksp).rankVal[0].as_mut_ptr();
        {
            /* tableLog <= maxTableLog */
            let rescale: c_int = maxTableLog.wrapping_sub(tableLog).wrapping_sub(1) as c_int;
            let mut nextRankVal: U32 = 0;
            let mut w: U32 = 1;
            while w < maxW + 1 {
                let curr: U32 = nextRankVal;
                nextRankVal = nextRankVal.wrapping_add(
                    (*wksp).rankStats[w as usize].wrapping_shl(w.wrapping_add(rescale as U32)),
                );
                *rankVal0.add(w as usize) = curr;
                w += 1;
            }
        }
        {
            let minBits: U32 = tableLog + 1 - maxW;
            let mut consumed: U32 = minBits;
            while consumed < maxTableLog.wrapping_sub(minBits).wrapping_add(1) {
                let rankValPtr =
                (*wksp).rankVal.as_mut_ptr().add(consumed as usize) as *mut U32;
                let mut w: U32 = 1;
                while w < maxW + 1 {
                    *rankValPtr.add(w as usize) =
                        (*rankVal0.add(w as usize)).wrapping_shr(consumed);
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
        DTable as *mut c_void,
        &dtd as *const DTableDesc as *const c_void,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

#[inline(always)]
unsafe fn HUF_decodeSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    let e = dt.add(val as usize);
    ZSTD_memcpy(op, core::ptr::addr_of!((*e).sequence) as *const c_void, 2);
    BIT_skipBits(DStream, (*e).nbBits as U32);
    (*e).length as U32
}

#[inline(always)]
unsafe fn HUF_decodeLastSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val = BIT_lookBitsFast(DStream, dtLog); /* note : dtLog >= 1 */
    let e = dt.add(val as usize);
    ZSTD_memcpy(op, core::ptr::addr_of!((*e).sequence) as *const c_void, 1);
    if (*e).length == 1 {
        BIT_skipBits(DStream, (*e).nbBits as U32);
    } else {
        if ((*DStream).bitsConsumed as usize)
            < core::mem::size_of::<BitContainerType>() * 8
        {
            BIT_skipBits(DStream, (*e).nbBits as U32);
            if (*DStream).bitsConsumed as usize > core::mem::size_of::<BitContainerType>() * 8 {
                /* ugly hack; works only because it's the last symbol. Note :
                 * can't easily extract nbBits from just this symbol */
                (*DStream).bitsConsumed =
                    (core::mem::size_of::<BitContainerType>() * 8) as u32;
            }
        }
    }
    1
}

/* HUF_DECODE_SYMBOLX2_0 / _1 / _2 : `_1` and `_2` are guarded by conditions
 * that are always true for this build, so all three are identical. */
#[inline(always)]
unsafe fn HUF_DECODE_SYMBOLX2_0(
    ptr: &mut *mut BYTE,
    DStreamPtr: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) {
    let n = HUF_decodeSymbolX2(*ptr as *mut c_void, DStreamPtr, dt, dtLog);
    *ptr = (*ptr).add(n as usize);
}

#[inline(always)]
unsafe fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    if pEnd.offset_from(p) as usize >= core::mem::size_of::<BitContainerType>() {
        if dtLog <= 11 && MEM_64bits() != 0 {
            /* up to 10 symbols at a time */
            while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
                & ((p < pEnd.wrapping_sub(9)) as c_int))
                != 0
            {
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
            }
        } else {
            /* up to 8 symbols at a time */
            while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
                & ((p < pEnd
                    .wrapping_sub(core::mem::size_of::<BitContainerType>() - 1))
                    as c_int))
                != 0
            {
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
            }
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }

    /* closer to end : up to 2 symbols at a time */
    if pEnd.offset_from(p) as usize >= 2 {
        while ((BIT_reloadDStream(bitDPtr) == BIT_DStream_unfinished) as c_int
            & ((p <= pEnd.wrapping_sub(2)) as c_int))
            != 0
        {
            HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
        }

        while p <= pEnd.wrapping_sub(2) {
            /* no need to reload : reached the end of DStream */
            HUF_DECODE_SYMBOLX2_0(&mut p, bitDPtr, dt, dtLog);
        }
    }

    if p < pEnd {
        let n = HUF_decodeLastSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog);
        p = p.add(n as usize);
    }

    p.offset_from(pStart) as usize
}

unsafe fn HUF_decompress1X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let mut bitD = BIT_DStream_t::default();

    /* Init */
    {
        let e = BIT_initDStream(&mut bitD, cSrc, cSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }

    /* decode */
    {
        let ostart = dst as *mut BYTE;
        let oend = ZSTD_maybeNullPtrAdd(ostart, dstSize as isize);
        let dtPtr = DTable.add(1) as *const c_void; /* force compiler to not use strict-aliasing */
        let dt = dtPtr as *const HUF_DEltX2;
        let dtd = HUF_getDTableDesc(DTable);
        HUF_decodeStreamX2(ostart, &mut bitD, oend, dt, dtd.tableLog as U32);
    }

    /* check */
    if BIT_endOfDStream(&bitD) == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* decoded size */
    dstSize
}

/* HUF_decompress4X2_usingDTable_internal_body():
 * Conditions:
 * @dstSize >= 6
 */
unsafe fn HUF_decompress4X2_usingDTable_internal_body(
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
        let istart = cSrc as *const BYTE;
        let ostart = dst as *mut BYTE;
        let oend = ostart.wrapping_add(dstSize);
        let olimit = oend.wrapping_sub(core::mem::size_of::<usize>() - 1);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUF_DEltX2;

        /* Init */
        let mut bitD1 = BIT_DStream_t::default();
        let mut bitD2 = BIT_DStream_t::default();
        let mut bitD3 = BIT_DStream_t::default();
        let mut bitD4 = BIT_DStream_t::default();
        let length1: usize = MEM_readLE16(istart as *const c_void) as usize;
        let length2: usize = MEM_readLE16(istart.add(2) as *const c_void) as usize;
        let length3: usize = MEM_readLE16(istart.add(4) as *const c_void) as usize;
        let length4: usize = cSrcSize.wrapping_sub(
            length1
                .wrapping_add(length2)
                .wrapping_add(length3)
                .wrapping_add(6),
        );
        let istart1 = istart.add(6); /* jumpTable */
        let istart2 = istart1.wrapping_add(length1);
        let istart3 = istart2.wrapping_add(length2);
        let istart4 = istart3.wrapping_add(length3);
        let segmentSize: usize = (dstSize + 3) / 4;
        let opStart2 = ostart.wrapping_add(segmentSize);
        let opStart3 = opStart2.wrapping_add(segmentSize);
        let opStart4 = opStart3.wrapping_add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: U32 = 1;
        let dtd = HUF_getDTableDesc(DTable);
        let dtLog: U32 = dtd.tableLog as U32;

        if length4 > cSrcSize {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        if opStart4 > oend {
            return ERROR(ZSTD_error_corruption_detected); /* overflow */
        }
        {
            let e = BIT_initDStream(&mut bitD1, istart1 as *const c_void, length1);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD2, istart2 as *const c_void, length2);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD3, istart3 as *const c_void, length3);
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        {
            let e = BIT_initDStream(&mut bitD4, istart4 as *const c_void, length4);
            if ERR_isError(e) != 0 {
                return e;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        if oend.offset_from(op4) as usize >= core::mem::size_of::<usize>() {
            while (endSignal & ((op4 < olimit) as U32)) != 0 {
                /* gcc build : the non-clang branch */
                HUF_DECODE_SYMBOLX2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_DECODE_SYMBOLX2_0(&mut op4, &mut bitD4, dt, dtLog);
                let r1 = (BIT_reloadDStreamFast(&mut bitD1) == BIT_DStream_unfinished) as U32;
                let r2 = (BIT_reloadDStreamFast(&mut bitD2) == BIT_DStream_unfinished) as U32;
                let r3 = (BIT_reloadDStreamFast(&mut bitD3) == BIT_DStream_unfinished) as U32;
                let r4 = (BIT_reloadDStreamFast(&mut bitD4) == BIT_DStream_unfinished) as U32;
                endSignal = r1 & r2 & r3 & r4;
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

/* HUF_NEED_BMI2_FUNCTION == 0, so `..._internal_bmi2` is not compiled. */

unsafe fn HUF_decompress4X2_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    HUF_decompress4X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* ZSTD_ENABLE_ASM_X86_64_BMI2 == 0, so
 * `HUF_decompress4X2_usingDTable_internal_fast_asm_loop` is not declared. */

/* HUF_4X2_DECODE_SYMBOL(_stream, _decode3) : the body only runs when
 * `_decode3 != 0 || _stream != 3`. */
#[inline(always)]
unsafe fn HUF_4X2_DECODE_SYMBOL(
    bits: &mut [U64; 4],
    op: &mut [*mut BYTE; 4],
    dtable: *const HUF_DEltX2,
    stream: usize,
    decode3: c_int,
) {
    if decode3 != 0 || stream != 3 {
        let index: c_int = (bits[stream] >> 53) as c_int;
        let entry: HUF_DEltX2 = *dtable.add(index as usize);
        MEM_write16(op[stream] as *mut c_void, entry.sequence);
        bits[stream] = bits[stream].wrapping_shl((entry.nbBits as c_int & 0x3F) as u32);
        op[stream] = op[stream].add(entry.length as usize);
    }
}

#[inline(always)]
unsafe fn HUF_4X2_RELOAD_STREAM(
    bits: &mut [U64; 4],
    ip: &mut [*const BYTE; 4],
    op: &mut [*mut BYTE; 4],
    dtable: *const HUF_DEltX2,
    stream: usize,
) {
    HUF_4X2_DECODE_SYMBOL(bits, op, dtable, 3, 1);
    {
        let ctz: c_int = ZSTD_countTrailingZeros64(bits[stream]) as c_int;
        let nbBits: c_int = ctz & 7;
        let nbBytes: c_int = ctz >> 3;
        ip[stream] = ip[stream].sub(nbBytes as usize);
        bits[stream] = MEM_read64(ip[stream] as *const c_void) | 1;
        bits[stream] = bits[stream].wrapping_shl(nbBits as u32);
    }
}

unsafe fn HUF_decompress4X2_usingDTable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [core::ptr::null(); 4];
    let mut op: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let mut oend: [*mut BYTE; 4] = [core::ptr::null_mut(); 4];
    let dtable = (*args).dt as *const HUF_DEltX2;
    let ilowest = (*args).ilowest;

    /* Copy the arguments to local registers. */
    ZSTD_memcpy(
        bits.as_mut_ptr() as *mut c_void,
        (*args).bits.as_ptr() as *const c_void,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        ip.as_mut_ptr() as *mut c_void,
        (*args).ip.as_ptr() as *const c_void,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        op.as_mut_ptr() as *mut c_void,
        (*args).op.as_ptr() as *const c_void,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );

    oend[0] = op[1];
    oend[1] = op[2];
    oend[2] = op[3];
    oend[3] = (*args).oend;

    loop {
        let olimit: *mut BYTE;
        let mut stream: c_int;

        /* Compute olimit */
        {
            /* Each loop does 5 table lookups for each of the 4 streams.
             * Each table lookup consumes up to 11 bits of input, and produces
             * up to 2 bytes of output.
             */
            /* We can consume up to 7 bytes of input per iteration per stream.
             * We also know that each input pointer is >= ip[0]. So we can run
             * iters loops before running out of input.
             */
            let mut iters: usize = (ip[0].offset_from(ilowest) as usize) / 7;
            /* Each iteration can produce up to 10 bytes of output per stream.
             * Each output stream my advance at different rates. So take the
             * minimum number of safe iterations among all the output streams.
             */
            stream = 0;
            while stream < 4 {
                let oiters: usize =
                    (oend[stream as usize].offset_from(op[stream as usize]) as usize) / 10;
                iters = MIN(iters, oiters);
                stream += 1;
            }

            /* Each iteration produces at least 5 output symbols. So until
             * op[3] crosses olimit, we know we haven't executed iters
             * iterations yet. This saves us maintaining an iters counter,
             * at the expense of computing the remaining # of iterations
             * more frequently.
             */
            olimit = op[3].add(iters * 5);

            /* Exit the fast decoding loop once we reach the end. */
            if op[3] == olimit {
                break;
            }

            /* Exit the decoding loop if any input pointer has crossed the
             * previous one. This indicates corruption, and a precondition
             * to our loop is that ip[i] >= ip[0].
             */
            let mut crossed = false;
            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    crossed = true;
                    break;
                }
                stream += 1;
            }
            if crossed {
                break; /* goto _out */
            }
        }

        /* Manually unrolled in the C source; the semantics are identical. */
        loop {
            /* Decode 5 symbols from each of the first 3 streams.
             * The final stream will be decoded during the reload phase
             * to reduce register pressure.
             */
            let mut round: usize = 0;
            while round < 5 {
                let mut st: usize = 0;
                while st < 4 {
                    HUF_4X2_DECODE_SYMBOL(&mut bits, &mut op, dtable, st, 0);
                    st += 1;
                }
                round += 1;
            }

            /* Decode one symbol from the final stream */
            HUF_4X2_DECODE_SYMBOL(&mut bits, &mut op, dtable, 3, 1);

            /* Decode 4 symbols from the final stream & reload bitstreams.
             * The final stream is reloaded last, meaning that all 5 symbols
             * are decoded from the final stream before it is reloaded.
             */
            let mut st: usize = 0;
            while st < 4 {
                HUF_4X2_RELOAD_STREAM(&mut bits, &mut ip, &mut op, dtable, st);
                st += 1;
            }

            if !(op[3] < olimit) {
                break;
            }
        }
    }

    /* _out: save the final values of each of the state variables back to args. */
    ZSTD_memcpy(
        (*args).bits.as_mut_ptr() as *mut c_void,
        bits.as_ptr() as *const c_void,
        core::mem::size_of::<[U64; 4]>(),
    );
    ZSTD_memcpy(
        (*args).ip.as_mut_ptr() as *mut c_void,
        ip.as_ptr() as *const c_void,
        core::mem::size_of::<[*const BYTE; 4]>(),
    );
    ZSTD_memcpy(
        (*args).op.as_mut_ptr() as *mut c_void,
        op.as_ptr() as *const c_void,
        core::mem::size_of::<[*mut BYTE; 4]>(),
    );
}

unsafe fn HUF_decompress4X2_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt = DTable.add(1) as *const c_void;
    let _ilowest = cSrc as *const BYTE;
    let oend = ZSTD_maybeNullPtrAdd(dst as *mut BYTE, dstSize as isize);
    let mut args = HUF_DecompressFastArgs::default();
    {
        let ret = HUF_DecompressFastArgs_init(&mut args, dst, dstSize, cSrc, cSrcSize, DTable);
        if ERR_isError(ret) != 0 {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(&mut args);

    /* note : op4 already verified within main loop */

    /* finish bitStreams one by one */
    {
        let segmentSize: usize = (dstSize + 3) / 4;
        let mut segmentEnd = dst as *mut BYTE;
        let mut i: c_int = 0;
        while i < 4 {
            let mut bit = BIT_DStream_t::default();
            if segmentSize <= oend.offset_from(segmentEnd) as usize {
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
            let produced = HUF_decodeStreamX2(
                args.op[i as usize],
                &mut bit,
                segmentEnd,
                dt as *const HUF_DEltX2,
                HUF_DECODER_FAST_TABLELOG,
            );
            args.op[i as usize] = args.op[i as usize].add(produced);
            if args.op[i as usize] != segmentEnd {
                return ERROR(ZSTD_error_corruption_detected);
            }
            i += 1;
        }
    }

    /* decoded size */
    dstSize
}

unsafe fn HUF_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let fallbackFn: HUF_DecompressUsingDTableFn = HUF_decompress4X2_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn = HUF_decompress4X2_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2 == 0 and ZSTD_ENABLE_ASM_X86_64_BMI2 == 0 : nothing to select */

    if HUF_ENABLE_FAST_DECODE != 0 && (flags & HUF_flags_disableFast) == 0 {
        let ret = HUF_decompress4X2_usingDTable_internal_fast(
            dst, dstSize, cSrc, cSrcSize, DTable, loopFn,
        );
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* HUF_DGEN(HUF_decompress1X2_usingDTable_internal) with DYNAMIC_BMI2 == 0 */
unsafe fn HUF_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    _flags: c_int,
) -> usize {
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
    let mut ip = cSrc as *const BYTE;

    let hSize = HUF_readDTableX2_wksp(DCtx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
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

unsafe fn HUF_decompress4X2_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip = cSrc as *const BYTE;

    let hSize = HUF_readDTableX2_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
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

/* ***********************************/
/* Universal decompression selectors */
/* ***********************************/

#[repr(C)]
#[derive(Copy, Clone)]
struct algo_time_t {
    tableTime: U32,
    decode256Time: U32,
}

const fn AT(tableTime: U32, decode256Time: U32) -> algo_time_t {
    algo_time_t {
        tableTime,
        decode256Time,
    }
}

static algoTime: [[algo_time_t; 2]; 16] = [
    /* single, double */
    [AT(0, 0), AT(1, 1)],       /* Q==0 : impossible */
    [AT(0, 0), AT(1, 1)],       /* Q==1 : impossible */
    [AT(150, 216), AT(381, 119)], /* Q == 2 : 12-18% */
    [AT(170, 205), AT(514, 112)], /* Q == 3 : 18-25% */
    [AT(177, 199), AT(539, 110)], /* Q == 4 : 25-32% */
    [AT(197, 194), AT(644, 107)], /* Q == 5 : 32-38% */
    [AT(221, 192), AT(735, 107)], /* Q == 6 : 38-44% */
    [AT(256, 189), AT(881, 106)], /* Q == 7 : 44-50% */
    [AT(359, 188), AT(1167, 109)], /* Q == 8 : 50-56% */
    [AT(582, 187), AT(1570, 114)], /* Q == 9 : 56-62% */
    [AT(688, 187), AT(1712, 122)], /* Q ==10 : 62-69% */
    [AT(825, 186), AT(1965, 136)], /* Q ==11 : 69-75% */
    [AT(976, 185), AT(2131, 150)], /* Q ==12 : 75-81% */
    [AT(1180, 186), AT(2070, 175)], /* Q ==13 : 81-87% */
    [AT(1377, 185), AT(1731, 202)], /* Q ==14 : 87-93% */
    [AT(1412, 185), AT(1695, 202)], /* Q ==15 : 93-99% */
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
        /* Q < 16 */
        let Q: U32 = if cSrcSize >= dstSize {
            15
        } else {
            (cSrcSize.wrapping_mul(16) / dstSize) as U32
        };
        let D256: U32 = (dstSize >> 8) as U32;
        let DTime0: U32 = algoTime[Q as usize][0].tableTime.wrapping_add(
            algoTime[Q as usize][0].decode256Time.wrapping_mul(D256),
        );
        let mut DTime1: U32 = algoTime[Q as usize][1].tableTime.wrapping_add(
            algoTime[Q as usize][1].decode256Time.wrapping_mul(D256),
        );
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
        ZSTD_memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        /* RLE */
        ZSTD_memset(dst, *(cSrc as *const BYTE) as c_int, dstSize);
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
    let dtd = HUF_getDTableDesc(DTable);
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
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let mut ip = cSrc as *const BYTE;

    let hSize = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    ip = ip.add(hSize);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: c_int,
) -> usize {
    let dtd = HUF_getDTableDesc(DTable);
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
