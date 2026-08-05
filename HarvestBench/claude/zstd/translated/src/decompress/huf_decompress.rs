//! Translation of c_src/src/decompress/huf_decompress.c
//! huff0 huffman decoder, part of Finite State Entropy library.
//!
//! Build configuration assumptions:
//!   DYNAMIC_BMI2 = 0
//!   ZSTD_ENABLE_ASM_X86_64_BMI2 = 0  (no assembly; C default fast-loop path)
//!   HUF_ENABLE_FAST_DECODE = 1
//! Target: little-endian 64-bit.

#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::c_void;

use crate::common::bits::highbit32;
use crate::common::bits::count_trailing_zeros64;
use crate::common::bitstream::{
    bit_end_of_dstream, bit_init_dstream, bit_look_bits_fast, bit_reload_dstream,
    bit_reload_dstream_fast, bit_skip_bits, BIT_DStream_t, BIT_DStream_unfinished,
};
use crate::common::error::{code, err_is_error, error};
use crate::common::huf_common::{
    HUF_flags_disableFast, HUF_SYMBOLVALUE_MAX, HUF_TABLELOG_ABSOLUTEMAX, HUF_TABLELOG_MAX,
    HUF_readStats_wksp,
};
use crate::common::mem::{
    mem_32bits, mem_is_little_endian, mem_read64, mem_read_le16, mem_read_le_st, mem_write16,
    mem_write64,
};

pub type HUF_DTable = u32;

const HUF_DECODER_FAST_TABLELOG: u32 = 11;

/* HUF_ENABLE_FAST_DECODE (no HUF_DISABLE_FAST_DECODE) */
const HUF_ENABLE_FAST_DECODE: bool = true;

/* HUF_READ_STATS_WORKSPACE_SIZE_U32 = FSE_DECOMPRESS_WKSP_SIZE_U32(6, HUF_TABLELOG_MAX-1)
 * = FSE_DTABLE_SIZE_U32(6) + 1 + FSE_BUILD_DTABLE_WKSP_SIZE_U32(6,11) + (255+1)/2 + 1
 * = 65 + 1 + 24 + 128 + 1 = 219 */
const HUF_READ_STATS_WORKSPACE_SIZE_U32: usize = 219;

/* HUF_DECOMPRESS_WORKSPACE_SIZE = (2 << 10) + (1 << 9) = 2560 */
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);

/* ZSTD_maybeNullPtrAdd (compiler.h) */
#[inline]
unsafe fn ZSTD_maybeNullPtrAdd(ptr: *mut u8, add: isize) -> *mut u8 {
    if add > 0 {
        ptr.offset(add)
    } else {
        ptr
    }
}

#[inline]
fn MIN(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

/*-***************************/
/*  generic DTableDesc       */
/*-***************************/
#[repr(C)]
#[derive(Clone, Copy)]
struct DTableDesc {
    maxTableLog: u8,
    tableType: u8,
    tableLog: u8,
    reserved: u8,
}

#[inline]
unsafe fn HUF_getDTableDesc(table: *const HUF_DTable) -> DTableDesc {
    let mut dtd = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    core::ptr::copy_nonoverlapping(
        table as *const u8,
        &mut dtd as *mut DTableDesc as *mut u8,
        core::mem::size_of::<DTableDesc>(),
    );
    dtd
}

#[inline]
unsafe fn HUF_initFastDStream(ip: *const u8) -> usize {
    let lastByte = *ip.add(7);
    let bitsConsumed: usize = if lastByte != 0 {
        (8 - highbit32(lastByte as u32)) as usize
    } else {
        0
    };
    let value = mem_read_le_st(ip as *const c_void) | 1;
    value << bitsConsumed
}

/**
 * The input/output arguments to the Huffman fast decoding loop.
 */
#[repr(C)]
struct HUF_DecompressFastArgs {
    ip: [*const u8; 4],
    op: [*mut u8; 4],
    bits: [u64; 4],
    dt: *const c_void,
    ilowest: *const u8,
    oend: *mut u8,
    iend: [*const u8; 4],
}

type HUF_DecompressFastLoopFn = unsafe fn(*mut HUF_DecompressFastArgs);

type HUF_DecompressUsingDTableFn = unsafe fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const HUF_DTable,
) -> usize;

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
    let dtLog = HUF_getDTableDesc(DTable).tableLog as u32;

    let istart = src as *const u8;

    let oend = ZSTD_maybeNullPtrAdd(dst as *mut u8, dstSize as isize);

    /* The fast decoding loop assumes 64-bit little-endian. */
    if mem_is_little_endian() == 0 || mem_32bits() != 0 {
        return 0;
    }

    /* Avoid nullptr addition */
    if dstSize == 0 {
        return 0;
    }

    /* strict minimum : jump table + 1 byte per stream */
    if srcSize < 10 {
        return error(code::CORRUPTION_DETECTED);
    }

    if dtLog != HUF_DECODER_FAST_TABLELOG {
        return 0;
    }

    /* Read the jump table. */
    {
        let length1 = mem_read_le16(istart as *const c_void) as usize;
        let length2 = mem_read_le16(istart.add(2) as *const c_void) as usize;
        let length3 = mem_read_le16(istart.add(4) as *const c_void) as usize;
        let length4 = srcSize.wrapping_sub(length1 + length2 + length3 + 6);
        (*args).iend[0] = istart.add(6); /* jumpTable */
        (*args).iend[1] = (*args).iend[0].add(length1);
        (*args).iend[2] = (*args).iend[1].add(length2);
        (*args).iend[3] = (*args).iend[2].add(length3);

        if length1 < 8 || length2 < 8 || length3 < 8 || length4 < 8 {
            return 0;
        }
        if length4 > srcSize {
            return error(code::CORRUPTION_DETECTED); /* overflow */
        }
    }
    /* ip[] contains the position that is currently loaded into bits[]. */
    (*args).ip[0] = (*args).iend[1].sub(core::mem::size_of::<u64>());
    (*args).ip[1] = (*args).iend[2].sub(core::mem::size_of::<u64>());
    (*args).ip[2] = (*args).iend[3].sub(core::mem::size_of::<u64>());
    (*args).ip[3] = (src as *const u8)
        .add(srcSize)
        .sub(core::mem::size_of::<u64>());

    /* op[] contains the output pointers. */
    (*args).op[0] = dst as *mut u8;
    (*args).op[1] = (*args).op[0].add((dstSize + 3) / 4);
    (*args).op[2] = (*args).op[1].add((dstSize + 3) / 4);
    (*args).op[3] = (*args).op[2].add((dstSize + 3) / 4);

    /* No point to call the ASM loop for tiny outputs. */
    if (*args).op[3] >= oend {
        return 0;
    }

    /* bits[] is the bit container. */
    (*args).bits[0] = HUF_initFastDStream((*args).ip[0]) as u64;
    (*args).bits[1] = HUF_initFastDStream((*args).ip[1]) as u64;
    (*args).bits[2] = HUF_initFastDStream((*args).ip[2]) as u64;
    (*args).bits[3] = HUF_initFastDStream((*args).ip[3]) as u64;

    (*args).ilowest = istart;

    (*args).oend = oend;
    (*args).dt = dt;

    1
}

unsafe fn HUF_initRemainingDStream(
    bit: *mut BIT_DStream_t,
    args: *const HUF_DecompressFastArgs,
    stream: i32,
    segmentEnd: *mut u8,
) -> usize {
    let s = stream as usize;
    /* Validate that we haven't overwritten. */
    if (*args).op[s] > segmentEnd {
        return error(code::CORRUPTION_DETECTED);
    }
    /* Validate that we haven't read beyond iend[]. */
    if (*args).ip[s] < (*args).iend[s].sub(8) {
        return error(code::CORRUPTION_DETECTED);
    }

    /* Construct the BIT_DStream_t. */
    (*bit).bitContainer = mem_read_le_st((*args).ip[s] as *const c_void);
    (*bit).bitsConsumed = count_trailing_zeros64((*args).bits[s]);
    (*bit).start = (*args).ilowest;
    (*bit).limitPtr = (*bit).start.add(core::mem::size_of::<usize>());
    (*bit).ptr = (*args).ip[s];

    0
}

/*-***************************/
/*  single-symbol decoding   */
/*-***************************/
#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX1 {
    nbBits: u8,
    byte: u8,
}

/**
 * Packs 4 HUF_DEltX1 structs into a U64.
 */
#[inline]
fn HUF_DEltX1_set4(symbol: u8, nbBits: u8) -> u64 {
    let mut D4: u64;
    if mem_is_little_endian() != 0 {
        D4 = (((symbol as u32) << 8).wrapping_add(nbBits as u32)) as u64;
    } else {
        D4 = ((symbol as u32).wrapping_add((nbBits as u32) << 8)) as u64;
    }
    D4 = D4.wrapping_mul(0x0001000100010001u64);
    D4
}

/**
 * Increase the tableLog to targetTableLog and rescales the stats.
 */
unsafe fn HUF_rescaleStats(
    huffWeight: *mut u8,
    rankVal: *mut u32,
    nbSymbols: u32,
    tableLog: u32,
    targetTableLog: u32,
) -> u32 {
    if tableLog > targetTableLog {
        return tableLog;
    }
    if tableLog < targetTableLog {
        let scale = targetTableLog - tableLog;
        let mut s: u32;
        s = 0;
        while s < nbSymbols {
            let hw = *huffWeight.add(s as usize);
            *huffWeight.add(s as usize) =
                hw.wrapping_add(if hw == 0 { 0u8 } else { scale as u8 });
            s += 1;
        }
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
struct HUF_ReadDTableX1_Workspace {
    rankVal: [u32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize],
    rankStart: [u32; (HUF_TABLELOG_ABSOLUTEMAX + 1) as usize],
    statsWksp: [u32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
    symbols: [u8; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    huffWeight: [u8; (HUF_SYMBOLVALUE_MAX + 1) as usize],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX1_wksp(
    DTable: *mut HUF_DTable,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: i32,
) -> usize {
    let mut tableLog: u32 = 0;
    let mut nbSymbols: u32 = 0;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUF_DEltX1;
    let wksp = workSpace as *mut HUF_ReadDTableX1_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX1_Workspace>() > wkspSize {
        return error(code::TABLELOG_TOOLARGE);
    }

    iSize = HUF_readStats_wksp(
        (*wksp).huffWeight.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as usize,
        (*wksp).rankVal.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
        (*wksp).statsWksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).statsWksp),
        flags,
    );
    if err_is_error(iSize) != 0 {
        return iSize;
    }

    /* Table header */
    {
        let mut dtd = HUF_getDTableDesc(DTable);
        let maxTableLog = dtd.maxTableLog as u32 + 1;
        let targetTableLog = MIN(maxTableLog as usize, HUF_DECODER_FAST_TABLELOG as usize) as u32;
        tableLog = HUF_rescaleStats(
            (*wksp).huffWeight.as_mut_ptr(),
            (*wksp).rankVal.as_mut_ptr(),
            nbSymbols,
            tableLog,
            targetTableLog,
        );
        if tableLog > (dtd.maxTableLog as u32 + 1) {
            return error(code::TABLELOG_TOOLARGE);
        }
        dtd.tableType = 0;
        dtd.tableLog = tableLog as u8;
        core::ptr::copy_nonoverlapping(
            &dtd as *const DTableDesc as *const u8,
            DTable as *mut u8,
            core::mem::size_of::<DTableDesc>(),
        );
    }

    /* Compute symbols and rankStart given rankVal */
    {
        let mut n: i32;
        let mut nextRankStart: u32 = 0;
        let unroll = 4i32;
        let nLimit = nbSymbols as i32 - unroll + 1;
        n = 0;
        while n < tableLog as i32 + 1 {
            let curr = nextRankStart;
            nextRankStart += (*wksp).rankVal[n as usize];
            (*wksp).rankStart[n as usize] = curr;
            n += 1;
        }
        n = 0;
        while n < nLimit {
            let mut u = 0i32;
            while u < unroll {
                let w = (*wksp).huffWeight[(n + u) as usize] as usize;
                let idx = (*wksp).rankStart[w];
                (*wksp).rankStart[w] += 1;
                (*wksp).symbols[idx as usize] = (n + u) as u8;
                u += 1;
            }
            n += unroll;
        }
        while n < nbSymbols as i32 {
            let w = (*wksp).huffWeight[n as usize] as usize;
            let idx = (*wksp).rankStart[w];
            (*wksp).rankStart[w] += 1;
            (*wksp).symbols[idx as usize] = n as u8;
            n += 1;
        }
    }

    /* fill DTable */
    {
        let mut w: u32;
        let mut symbol = (*wksp).rankVal[0] as i32;
        let mut rankStart = 0i32;
        w = 1;
        while w < tableLog + 1 {
            let symbolCount = (*wksp).rankVal[w as usize] as i32;
            let length = (1i32 << w) >> 1;
            let mut uStart = rankStart;
            let nbBits = (tableLog + 1 - w) as u8;
            let mut s: i32;
            let mut u: i32;
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
                        let D4 = HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
                        mem_write64(dt.offset(uStart as isize) as *mut c_void, D4);
                        uStart += 4;
                        s += 1;
                    }
                }
                8 => {
                    s = 0;
                    while s < symbolCount {
                        let D4 = HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
                        mem_write64(dt.offset(uStart as isize) as *mut c_void, D4);
                        mem_write64(dt.offset((uStart + 4) as isize) as *mut c_void, D4);
                        uStart += 8;
                        s += 1;
                    }
                }
                _ => {
                    s = 0;
                    while s < symbolCount {
                        let D4 = HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits);
                        u = 0;
                        while u < length {
                            mem_write64(dt.offset((uStart + u + 0) as isize) as *mut c_void, D4);
                            mem_write64(dt.offset((uStart + u + 4) as isize) as *mut c_void, D4);
                            mem_write64(dt.offset((uStart + u + 8) as isize) as *mut c_void, D4);
                            mem_write64(dt.offset((uStart + u + 12) as isize) as *mut c_void, D4);
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

#[inline]
unsafe fn HUF_decodeSymbolX1(
    Dstream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dtLog: u32,
) -> u8 {
    let val = bit_look_bits_fast(Dstream, dtLog);
    let c = (*dt.add(val)).byte;
    bit_skip_bits(Dstream, (*dt.add(val)).nbBits as u32);
    c
}

/* On 64-bit with HUF_TABLELOG_MAX<=12, all of _0/_1/_2 decode. */
#[inline]
unsafe fn HUF_decode_symbolx1_0(
    ptr: &mut *mut u8,
    ds: *mut BIT_DStream_t,
    dt: *const HUF_DEltX1,
    dtLog: u32,
) {
    **ptr = HUF_decodeSymbolX1(ds, dt, dtLog);
    *ptr = (*ptr).add(1);
}

unsafe fn HUF_decodeStreamX1(
    mut p: *mut u8,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut u8,
    dt: *const HUF_DEltX1,
    dtLog: u32,
) -> usize {
    let pStart = p;

    /* up to 4 symbols at a time */
    if (pEnd as isize - p as isize) > 3 {
        while ((bit_reload_dstream(bitDPtr) == BIT_DStream_unfinished) as i32
            & ((p < pEnd.sub(3)) as i32))
            != 0
        {
            HUF_decode_symbolx1_0(&mut p, bitDPtr, dt, dtLog);
            HUF_decode_symbolx1_0(&mut p, bitDPtr, dt, dtLog);
            HUF_decode_symbolx1_0(&mut p, bitDPtr, dt, dtLog);
            HUF_decode_symbolx1_0(&mut p, bitDPtr, dt, dtLog);
        }
    } else {
        bit_reload_dstream(bitDPtr);
    }

    /* MEM_32bits() is false: skip the 32-bit block */

    /* no more data to retrieve from bitstream, no need to reload */
    while p < pEnd {
        HUF_decode_symbolx1_0(&mut p, bitDPtr, dt, dtLog);
    }

    (pEnd as usize - pStart as usize)
}

unsafe fn HUF_decompress1X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let op = dst as *mut u8;
    let oend = ZSTD_maybeNullPtrAdd(op, dstSize as isize);
    let dtPtr = DTable.add(1) as *const c_void;
    let dt = dtPtr as *const HUF_DEltX1;
    let mut bitD = core::mem::zeroed::<BIT_DStream_t>();
    let dtd = HUF_getDTableDesc(DTable);
    let dtLog = dtd.tableLog as u32;

    {
        let e = bit_init_dstream(&mut bitD, cSrc, cSrcSize);
        if err_is_error(e) != 0 {
            return e;
        }
    }

    HUF_decodeStreamX1(op, &mut bitD, oend, dt, dtLog);

    if bit_end_of_dstream(&bitD) == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    dstSize
}

/* HUF_decompress4X1_usingDTable_internal_body(): @dstSize >= 6 */
unsafe fn HUF_decompress4X1_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    if cSrcSize < 10 {
        return error(code::CORRUPTION_DETECTED);
    }
    if dstSize < 6 {
        return error(code::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const u8;
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);
        let olimit = oend.sub(3);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUF_DEltX1;

        let mut bitD1 = core::mem::zeroed::<BIT_DStream_t>();
        let mut bitD2 = core::mem::zeroed::<BIT_DStream_t>();
        let mut bitD3 = core::mem::zeroed::<BIT_DStream_t>();
        let mut bitD4 = core::mem::zeroed::<BIT_DStream_t>();
        let length1 = mem_read_le16(istart as *const c_void) as usize;
        let length2 = mem_read_le16(istart.add(2) as *const c_void) as usize;
        let length3 = mem_read_le16(istart.add(4) as *const c_void) as usize;
        let length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1 = istart.add(6);
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize = (dstSize + 3) / 4;
        let opStart2 = ostart.add(segmentSize);
        let opStart3 = opStart2.add(segmentSize);
        let opStart4 = opStart3.add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let dtd = HUF_getDTableDesc(DTable);
        let dtLog = dtd.tableLog as u32;
        let mut endSignal: u32 = 1;

        if length4 > cSrcSize {
            return error(code::CORRUPTION_DETECTED);
        }
        if opStart4 > oend {
            return error(code::CORRUPTION_DETECTED);
        }
        {
            let e = bit_init_dstream(&mut bitD1, istart1 as *const c_void, length1);
            if err_is_error(e) != 0 {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bitD2, istart2 as *const c_void, length2);
            if err_is_error(e) != 0 {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bitD3, istart3 as *const c_void, length3);
            if err_is_error(e) != 0 {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bitD4, istart4 as *const c_void, length4);
            if err_is_error(e) != 0 {
                return e;
            }
        }

        /* up to 16 symbols per loop (4 symbols per stream) in 64-bit mode */
        if (oend as usize - op4 as usize) >= core::mem::size_of::<usize>() {
            while (endSignal & ((op4 < olimit) as u32)) != 0 {
                HUF_decode_symbolx1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx1_0(&mut op4, &mut bitD4, dt, dtLog);
                endSignal &= (bit_reload_dstream_fast(&mut bitD1) == BIT_DStream_unfinished) as u32;
                endSignal &= (bit_reload_dstream_fast(&mut bitD2) == BIT_DStream_unfinished) as u32;
                endSignal &= (bit_reload_dstream_fast(&mut bitD3) == BIT_DStream_unfinished) as u32;
                endSignal &= (bit_reload_dstream_fast(&mut bitD4) == BIT_DStream_unfinished) as u32;
            }
        }

        /* check corruption */
        if op1 > opStart2 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op2 > opStart3 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op3 > opStart4 {
            return error(code::CORRUPTION_DETECTED);
        }
        /* note : op4 supposed already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX1(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX1(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX1(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX1(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        {
            let endCheck = bit_end_of_dstream(&bitD1)
                & bit_end_of_dstream(&bitD2)
                & bit_end_of_dstream(&bitD3)
                & bit_end_of_dstream(&bitD4);
            if endCheck == 0 {
                return error(code::CORRUPTION_DETECTED);
            }
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X1_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    HUF_decompress4X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

unsafe fn HUF_decompress4X1_usingDTable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [u64; 4] = [0; 4];
    let mut ip: [*const u8; 4] = [core::ptr::null(); 4];
    let mut op: [*mut u8; 4] = [core::ptr::null_mut(); 4];
    let dtable = (*args).dt as *const u16;
    let oend = (*args).oend;
    let ilowest = (*args).ilowest;

    core::ptr::copy_nonoverlapping((*args).bits.as_ptr(), bits.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).ip.as_ptr(), ip.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).op.as_ptr(), op.as_mut_ptr(), 4);

    'outer: loop {
        let olimit: *mut u8;
        let mut stream: i32;

        /* Compute olimit */
        {
            let oiters = (oend as usize - op[3] as usize) / 5;
            let iiters = (ip[0] as usize - ilowest as usize) / 7;
            let iters = MIN(oiters, iiters);
            let symbols = iters * 5;

            olimit = op[3].add(symbols);

            if op[3] == olimit {
                break 'outer;
            }

            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'outer;
                }
                stream += 1;
            }
        }

        macro_rules! decode_symbol {
            ($stream:expr, $symbol:expr) => {{
                let index = (bits[$stream] >> 53) as usize;
                let entry = *dtable.add(index) as i32;
                bits[$stream] <<= (entry & 0x3F) as u32;
                *op[$stream].add($symbol) = ((entry >> 8) & 0xFF) as u8;
            }};
        }
        macro_rules! reload_stream {
            ($stream:expr) => {{
                let ctz = count_trailing_zeros64(bits[$stream]) as i32;
                let nbBits = ctz & 7;
                let nbBytes = ctz >> 3;
                op[$stream] = op[$stream].add(5);
                ip[$stream] = ip[$stream].sub(nbBytes as usize);
                bits[$stream] = mem_read64(ip[$stream] as *const c_void) | 1;
                bits[$stream] <<= nbBits as u32;
            }};
        }

        loop {
            decode_symbol!(0, 0);
            decode_symbol!(1, 0);
            decode_symbol!(2, 0);
            decode_symbol!(3, 0);
            decode_symbol!(0, 1);
            decode_symbol!(1, 1);
            decode_symbol!(2, 1);
            decode_symbol!(3, 1);
            decode_symbol!(0, 2);
            decode_symbol!(1, 2);
            decode_symbol!(2, 2);
            decode_symbol!(3, 2);
            decode_symbol!(0, 3);
            decode_symbol!(1, 3);
            decode_symbol!(2, 3);
            decode_symbol!(3, 3);
            decode_symbol!(0, 4);
            decode_symbol!(1, 4);
            decode_symbol!(2, 4);
            decode_symbol!(3, 4);

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

unsafe fn HUF_decompress4X1_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt = DTable.add(1) as *const c_void;
    let _ilowest = cSrc as *const u8;
    let oend = ZSTD_maybeNullPtrAdd(dst as *mut u8, dstSize as isize);
    let mut args: HUF_DecompressFastArgs = core::mem::zeroed();
    {
        let ret = HUF_DecompressFastArgs_init(&mut args, dst, dstSize, cSrc, cSrcSize, DTable);
        if err_is_error(ret) != 0 {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(&mut args);

    /* finish bit streams one by one. */
    {
        let segmentSize = (dstSize + 3) / 4;
        let mut segmentEnd = dst as *mut u8;
        let mut i = 0i32;
        while i < 4 {
            let mut bit = core::mem::zeroed::<BIT_DStream_t>();
            if segmentSize <= (oend as usize - segmentEnd as usize) {
                segmentEnd = segmentEnd.add(segmentSize);
            } else {
                segmentEnd = oend;
            }
            let e = HUF_initRemainingDStream(&mut bit, &args, i, segmentEnd);
            if err_is_error(e) != 0 {
                return e;
            }
            args.op[i as usize] = args.op[i as usize].add(HUF_decodeStreamX1(
                args.op[i as usize],
                &mut bit,
                segmentEnd,
                dt as *const HUF_DEltX1,
                HUF_DECODER_FAST_TABLELOG,
            ));
            if args.op[i as usize] != segmentEnd {
                return error(code::CORRUPTION_DETECTED);
            }
            i += 1;
        }
    }

    dstSize
}

/* HUF_DGEN(HUF_decompress1X1_usingDTable_internal) : DYNAMIC_BMI2=0 variant */
unsafe fn HUF_decompress1X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    _flags: i32,
) -> usize {
    HUF_decompress1X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

unsafe fn HUF_decompress4X1_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: i32,
) -> usize {
    let fallbackFn: HUF_DecompressUsingDTableFn =
        HUF_decompress4X1_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn =
        HUF_decompress4X1_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2=0, ZSTD_ENABLE_ASM_X86_64_BMI2=0 */
    if HUF_ENABLE_FAST_DECODE && (flags & HUF_flags_disableFast) == 0 {
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
    flags: i32,
) -> usize {
    let mut ip = cSrc as *const u8;

    let hSize = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if err_is_error(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X1_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx, flags)
}

/* *************************/
/* double-symbols decoding */
/* *************************/
#[repr(C)]
#[derive(Clone, Copy)]
struct HUF_DEltX2 {
    sequence: u16,
    nbBits: u8,
    length: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct sortedSymbol_t {
    symbol: u8,
}

/* rankValCol_t = U32[HUF_TABLELOG_MAX+1]; rankVal_t = rankValCol_t[HUF_TABLELOG_MAX] */
const RANKVALCOL_LEN: usize = (HUF_TABLELOG_MAX + 1) as usize;
const RANKVAL_ROWS: usize = HUF_TABLELOG_MAX as usize;

/**
 * Constructs a HUF_DEltX2 in a U32.
 */
#[inline]
fn HUF_buildDEltX2U32(symbol: u32, nbBits: u32, baseSeq: u32, level: i32) -> u32 {
    let seq: u32;
    if mem_is_little_endian() != 0 {
        seq = if level == 1 {
            symbol
        } else {
            baseSeq.wrapping_add(symbol << 8)
        };
        seq.wrapping_add(nbBits << 16).wrapping_add((level as u32) << 24)
    } else {
        seq = if level == 1 {
            symbol << 8
        } else {
            (baseSeq << 8).wrapping_add(symbol)
        };
        (seq << 16).wrapping_add(nbBits << 8).wrapping_add(level as u32)
    }
}

/**
 * Constructs a HUF_DEltX2.
 */
#[inline]
unsafe fn HUF_buildDEltX2(symbol: u32, nbBits: u32, baseSeq: u32, level: i32) -> HUF_DEltX2 {
    let mut DElt = HUF_DEltX2 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let val = HUF_buildDEltX2U32(symbol, nbBits, baseSeq, level);
    core::ptr::copy_nonoverlapping(
        &val as *const u32 as *const u8,
        &mut DElt as *mut HUF_DEltX2 as *mut u8,
        core::mem::size_of::<u32>(),
    );
    DElt
}

/**
 * Constructs 2 HUF_DEltX2s and packs them into a U64.
 */
#[inline]
fn HUF_buildDEltX2U64(symbol: u32, nbBits: u32, baseSeq: u16, level: i32) -> u64 {
    let DElt = HUF_buildDEltX2U32(symbol, nbBits, baseSeq as u32, level);
    (DElt as u64).wrapping_add((DElt as u64) << 32)
}

/**
 * Fills the DTable rank with all the symbols from [begin, end) that are each
 * nbBits long.
 */
unsafe fn HUF_fillDTableX2ForWeight(
    mut DTableRank: *mut HUF_DEltX2,
    begin: *const sortedSymbol_t,
    end: *const sortedSymbol_t,
    nbBits: u32,
    tableLog: u32,
    baseSeq: u16,
    level: i32,
) {
    let length: u32 = 1u32 << ((tableLog.wrapping_sub(nbBits)) & 0x1F);
    let mut ptr: *const sortedSymbol_t;
    match length {
        1 => {
            ptr = begin;
            while ptr != end {
                let DElt = HUF_buildDEltX2((*ptr).symbol as u32, nbBits, baseSeq as u32, level);
                *DTableRank = DElt;
                DTableRank = DTableRank.add(1);
                ptr = ptr.add(1);
            }
        }
        2 => {
            ptr = begin;
            while ptr != end {
                let DElt = HUF_buildDEltX2((*ptr).symbol as u32, nbBits, baseSeq as u32, level);
                *DTableRank.add(0) = DElt;
                *DTableRank.add(1) = DElt;
                DTableRank = DTableRank.add(2);
                ptr = ptr.add(1);
            }
        }
        4 => {
            ptr = begin;
            while ptr != end {
                let DEltX2 = HUF_buildDEltX2U64((*ptr).symbol as u32, nbBits, baseSeq, level);
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTableRank.add(0) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTableRank.add(2) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                DTableRank = DTableRank.add(4);
                ptr = ptr.add(1);
            }
        }
        8 => {
            ptr = begin;
            while ptr != end {
                let DEltX2 = HUF_buildDEltX2U64((*ptr).symbol as u32, nbBits, baseSeq, level);
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTableRank.add(0) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTableRank.add(2) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTableRank.add(4) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTableRank.add(6) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                DTableRank = DTableRank.add(8);
                ptr = ptr.add(1);
            }
        }
        _ => {
            ptr = begin;
            while ptr != end {
                let DEltX2 = HUF_buildDEltX2U64((*ptr).symbol as u32, nbBits, baseSeq, level);
                let DTableRankEnd = DTableRank.add(length as usize);
                while DTableRank != DTableRankEnd {
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTableRank.add(0) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTableRank.add(2) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTableRank.add(4) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTableRank.add(6) as *mut u8,
                        core::mem::size_of::<u64>(),
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
    targetLog: u32,
    consumedBits: u32,
    rankVal: *const u32,
    minWeight: i32,
    maxWeight1: i32,
    sortedSymbols: *const sortedSymbol_t,
    rankStart: *const u32,
    nbBitsBaseline: u32,
    baseSeq: u16,
) {
    if minWeight > 1 {
        let length: u32 = 1u32 << ((targetLog.wrapping_sub(consumedBits)) & 0x1F);
        let DEltX2 = HUF_buildDEltX2U64(baseSeq as u32, consumedBits, 0, 1);
        let skipSize = *rankVal.add(minWeight as usize) as i32;
        match length {
            2 => {
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTable as *mut u8,
                    core::mem::size_of::<u64>(),
                );
            }
            4 => {
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTable.add(0) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                core::ptr::copy_nonoverlapping(
                    &DEltX2 as *const u64 as *const u8,
                    DTable.add(2) as *mut u8,
                    core::mem::size_of::<u64>(),
                );
            }
            _ => {
                let mut i = 0i32;
                while i < skipSize {
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTable.offset((i + 0) as isize) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTable.offset((i + 2) as isize) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTable.offset((i + 4) as isize) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    core::ptr::copy_nonoverlapping(
                        &DEltX2 as *const u64 as *const u8,
                        DTable.offset((i + 6) as isize) as *mut u8,
                        core::mem::size_of::<u64>(),
                    );
                    i += 8;
                }
            }
        }
    }

    /* Fill each of the second level symbols by weight. */
    {
        let mut w = minWeight;
        while w < maxWeight1 {
            let begin = *rankStart.add(w as usize) as i32;
            let end = *rankStart.add((w + 1) as usize) as i32;
            let nbBits = nbBitsBaseline.wrapping_sub(w as u32);
            let totalBits = nbBits.wrapping_add(consumedBits);
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

unsafe fn HUF_fillDTableX2(
    DTable: *mut HUF_DEltX2,
    targetLog: u32,
    sortedList: *const sortedSymbol_t,
    rankStart: *const u32,
    rankValOrigin: *mut [u32; RANKVALCOL_LEN],
    maxWeight: u32,
    nbBitsBaseline: u32,
) {
    let rankVal = (*rankValOrigin.add(0)).as_mut_ptr();
    let scaleLog = nbBitsBaseline as i32 - targetLog as i32;
    let minBits = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut w: i32;
    let wEnd = maxWeight as i32 + 1;

    w = 1;
    while w < wEnd {
        let begin = *rankStart.add(w as usize) as i32;
        let end = *rankStart.add((w + 1) as usize) as i32;
        let nbBits = nbBitsBaseline.wrapping_sub(w as u32);

        if targetLog.wrapping_sub(nbBits) >= minBits {
            /* Enough room for a second symbol. */
            let mut start = *rankVal.add(w as usize) as i32;
            let length: u32 = 1u32 << ((targetLog.wrapping_sub(nbBits)) & 0x1F);
            let mut minWeight = nbBits as i32 + scaleLog;
            let mut s: i32;
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
                    (*sortedList.offset(s as isize)).symbol as u16,
                );
                start += length as i32;
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
struct HUF_ReadDTableX2_Workspace {
    rankVal: [[u32; RANKVALCOL_LEN]; RANKVAL_ROWS],
    rankStats: [u32; (HUF_TABLELOG_MAX + 1) as usize],
    rankStart0: [u32; (HUF_TABLELOG_MAX + 3) as usize],
    sortedSymbol: [sortedSymbol_t; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    weightList: [u8; (HUF_SYMBOLVALUE_MAX + 1) as usize],
    calleeWksp: [u32; HUF_READ_STATS_WORKSPACE_SIZE_U32],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX2_wksp(
    DTable: *mut HUF_DTable,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: i32,
) -> usize {
    let mut tableLog: u32 = 0;
    let mut maxW: u32;
    let mut nbSymbols: u32 = 0;
    let mut dtd = HUF_getDTableDesc(DTable);
    let mut maxTableLog = dtd.maxTableLog as u32;
    let iSize: usize;
    let dtPtr = DTable.add(1) as *mut c_void;
    let dt = dtPtr as *mut HUF_DEltX2;
    let rankStart: *mut u32;

    let wksp = workSpace as *mut HUF_ReadDTableX2_Workspace;

    if core::mem::size_of::<HUF_ReadDTableX2_Workspace>() > wkspSize {
        return error(code::GENERIC);
    }

    rankStart = (*wksp).rankStart0.as_mut_ptr().add(1);
    core::ptr::write_bytes(
        (*wksp).rankStats.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of_val(&(*wksp).rankStats),
    );
    core::ptr::write_bytes(
        (*wksp).rankStart0.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of_val(&(*wksp).rankStart0),
    );

    if maxTableLog > HUF_TABLELOG_MAX {
        return error(code::TABLELOG_TOOLARGE);
    }

    iSize = HUF_readStats_wksp(
        (*wksp).weightList.as_mut_ptr(),
        (HUF_SYMBOLVALUE_MAX + 1) as usize,
        (*wksp).rankStats.as_mut_ptr(),
        &mut nbSymbols,
        &mut tableLog,
        src,
        srcSize,
        (*wksp).calleeWksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&(*wksp).calleeWksp),
        flags,
    );
    if err_is_error(iSize) != 0 {
        return iSize;
    }

    /* check result */
    if tableLog > maxTableLog {
        return error(code::TABLELOG_TOOLARGE);
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
        let mut w: u32;
        let mut nextRankStart: u32 = 0;
        w = 1;
        while w < maxW + 1 {
            let curr = nextRankStart;
            nextRankStart += (*wksp).rankStats[w as usize];
            *rankStart.add(w as usize) = curr;
            w += 1;
        }
        *rankStart.add(0) = nextRankStart;
        *rankStart.add((maxW + 1) as usize) = nextRankStart;
    }

    /* sort symbols by weight */
    {
        let mut s: u32 = 0;
        while s < nbSymbols {
            let w = (*wksp).weightList[s as usize] as u32;
            let r = *rankStart.add(w as usize);
            *rankStart.add(w as usize) += 1;
            (*wksp).sortedSymbol[r as usize].symbol = s as u8;
            s += 1;
        }
        *rankStart.add(0) = 0;
    }

    /* Build rankVal */
    {
        let rankVal0 = (*wksp).rankVal[0].as_mut_ptr();
        {
            let rescale = (maxTableLog as i32 - tableLog as i32) - 1;
            let mut nextRankVal: u32 = 0;
            let mut w: u32 = 1;
            while w < maxW + 1 {
                let curr = nextRankVal;
                nextRankVal = nextRankVal
                    .wrapping_add((*wksp).rankStats[w as usize] << ((w as i32 + rescale) as u32));
                *rankVal0.add(w as usize) = curr;
                w += 1;
            }
        }
        {
            let minBits = tableLog + 1 - maxW;
            let mut consumed = minBits;
            while consumed < maxTableLog - minBits + 1 {
                let rankValPtr = (*wksp).rankVal[consumed as usize].as_mut_ptr();
                let mut w: u32 = 1;
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

    dtd.tableLog = maxTableLog as u8;
    dtd.tableType = 1;
    core::ptr::copy_nonoverlapping(
        &dtd as *const DTableDesc as *const u8,
        DTable as *mut u8,
        core::mem::size_of::<DTableDesc>(),
    );
    iSize
}

#[inline]
unsafe fn HUF_decodeSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: u32,
) -> u32 {
    let val = bit_look_bits_fast(DStream, dtLog);
    core::ptr::copy_nonoverlapping(
        &(*dt.add(val)).sequence as *const u16 as *const u8,
        op as *mut u8,
        2,
    );
    bit_skip_bits(DStream, (*dt.add(val)).nbBits as u32);
    (*dt.add(val)).length as u32
}

#[inline]
unsafe fn HUF_decodeLastSymbolX2(
    op: *mut c_void,
    DStream: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: u32,
) -> u32 {
    let val = bit_look_bits_fast(DStream, dtLog);
    core::ptr::copy_nonoverlapping(
        &(*dt.add(val)).sequence as *const u16 as *const u8,
        op as *mut u8,
        1,
    );
    if (*dt.add(val)).length == 1 {
        bit_skip_bits(DStream, (*dt.add(val)).nbBits as u32);
    } else {
        let bcBits = (core::mem::size_of_val(&(*DStream).bitContainer) * 8) as u32;
        if (*DStream).bitsConsumed < bcBits {
            bit_skip_bits(DStream, (*dt.add(val)).nbBits as u32);
            if (*DStream).bitsConsumed > bcBits {
                (*DStream).bitsConsumed = bcBits;
            }
        }
    }
    1
}

/* On 64-bit, all of _0/_1/_2 decode a symbol. */
#[inline]
unsafe fn HUF_decode_symbolx2_0(
    ptr: &mut *mut u8,
    ds: *mut BIT_DStream_t,
    dt: *const HUF_DEltX2,
    dtLog: u32,
) {
    let adv = HUF_decodeSymbolX2(*ptr as *mut c_void, ds, dt, dtLog);
    *ptr = (*ptr).add(adv as usize);
}

unsafe fn HUF_decodeStreamX2(
    mut p: *mut u8,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut u8,
    dt: *const HUF_DEltX2,
    dtLog: u32,
) -> usize {
    let pStart = p;

    /* up to 8 symbols at a time */
    if (pEnd as usize - p as usize) >= core::mem::size_of_val(&(*bitDPtr).bitContainer) {
        if dtLog <= 11 {
            /* MEM_64bits() true */
            /* up to 10 symbols at a time */
            while ((bit_reload_dstream(bitDPtr) == BIT_DStream_unfinished) as i32
                & ((p < pEnd.sub(9)) as i32))
                != 0
            {
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
            }
        } else {
            /* up to 8 symbols at a time */
            while ((bit_reload_dstream(bitDPtr) == BIT_DStream_unfinished) as i32
                & ((p < pEnd.sub(core::mem::size_of_val(&(*bitDPtr).bitContainer) - 1)) as i32))
                != 0
            {
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
                HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
            }
        }
    } else {
        bit_reload_dstream(bitDPtr);
    }

    /* closer to end : up to 2 symbols at a time */
    if (pEnd as usize - p as usize) >= 2 {
        while ((bit_reload_dstream(bitDPtr) == BIT_DStream_unfinished) as i32
            & ((p <= pEnd.sub(2)) as i32))
            != 0
        {
            HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
        }

        while p <= pEnd.sub(2) {
            HUF_decode_symbolx2_0(&mut p, bitDPtr, dt, dtLog);
        }
    }

    if p < pEnd {
        p = p.add(HUF_decodeLastSymbolX2(p as *mut c_void, bitDPtr, dt, dtLog) as usize);
    }

    (p as usize - pStart as usize)
}

unsafe fn HUF_decompress1X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    let mut bitD = core::mem::zeroed::<BIT_DStream_t>();

    {
        let e = bit_init_dstream(&mut bitD, cSrc, cSrcSize);
        if err_is_error(e) != 0 {
            return e;
        }
    }

    {
        let ostart = dst as *mut u8;
        let oend = ZSTD_maybeNullPtrAdd(ostart, dstSize as isize);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUF_DEltX2;
        let dtd = HUF_getDTableDesc(DTable);
        HUF_decodeStreamX2(ostart, &mut bitD, oend, dt, dtd.tableLog as u32);
    }

    if bit_end_of_dstream(&bitD) == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    dstSize
}

/* HUF_decompress4X2_usingDTable_internal_body(): @dstSize >= 6 */
unsafe fn HUF_decompress4X2_usingDTable_internal_body(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    if cSrcSize < 10 {
        return error(code::CORRUPTION_DETECTED);
    }
    if dstSize < 6 {
        return error(code::CORRUPTION_DETECTED);
    }

    {
        let istart = cSrc as *const u8;
        let ostart = dst as *mut u8;
        let oend = ostart.add(dstSize);
        let olimit = oend.sub(core::mem::size_of::<usize>() - 1);
        let dtPtr = DTable.add(1) as *const c_void;
        let dt = dtPtr as *const HUF_DEltX2;

        let mut bitD1 = core::mem::zeroed::<BIT_DStream_t>();
        let mut bitD2 = core::mem::zeroed::<BIT_DStream_t>();
        let mut bitD3 = core::mem::zeroed::<BIT_DStream_t>();
        let mut bitD4 = core::mem::zeroed::<BIT_DStream_t>();
        let length1 = mem_read_le16(istart as *const c_void) as usize;
        let length2 = mem_read_le16(istart.add(2) as *const c_void) as usize;
        let length3 = mem_read_le16(istart.add(4) as *const c_void) as usize;
        let length4 = cSrcSize.wrapping_sub(length1 + length2 + length3 + 6);
        let istart1 = istart.add(6);
        let istart2 = istart1.add(length1);
        let istart3 = istart2.add(length2);
        let istart4 = istart3.add(length3);
        let segmentSize = (dstSize + 3) / 4;
        let opStart2 = ostart.add(segmentSize);
        let opStart3 = opStart2.add(segmentSize);
        let opStart4 = opStart3.add(segmentSize);
        let mut op1 = ostart;
        let mut op2 = opStart2;
        let mut op3 = opStart3;
        let mut op4 = opStart4;
        let mut endSignal: u32 = 1;
        let dtd = HUF_getDTableDesc(DTable);
        let dtLog = dtd.tableLog as u32;

        if length4 > cSrcSize {
            return error(code::CORRUPTION_DETECTED);
        }
        if opStart4 > oend {
            return error(code::CORRUPTION_DETECTED);
        }
        {
            let e = bit_init_dstream(&mut bitD1, istart1 as *const c_void, length1);
            if err_is_error(e) != 0 {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bitD2, istart2 as *const c_void, length2);
            if err_is_error(e) != 0 {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bitD3, istart3 as *const c_void, length3);
            if err_is_error(e) != 0 {
                return e;
            }
        }
        {
            let e = bit_init_dstream(&mut bitD4, istart4 as *const c_void, length4);
            if err_is_error(e) != 0 {
                return e;
            }
        }

        /* 16-32 symbols per loop (4-8 symbols per stream) */
        if (oend as usize - op4 as usize) >= core::mem::size_of::<usize>() {
            while (endSignal & ((op4 < olimit) as u32)) != 0 {
                /* non-clang generic path */
                HUF_decode_symbolx2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op4, &mut bitD4, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op1, &mut bitD1, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op2, &mut bitD2, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op3, &mut bitD3, dt, dtLog);
                HUF_decode_symbolx2_0(&mut op4, &mut bitD4, dt, dtLog);
                endSignal = ((bit_reload_dstream_fast(&mut bitD1) == BIT_DStream_unfinished) as u32
                    & (bit_reload_dstream_fast(&mut bitD2) == BIT_DStream_unfinished) as u32
                    & (bit_reload_dstream_fast(&mut bitD3) == BIT_DStream_unfinished) as u32
                    & (bit_reload_dstream_fast(&mut bitD4) == BIT_DStream_unfinished) as u32);
            }
        }

        /* check corruption */
        if op1 > opStart2 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op2 > opStart3 {
            return error(code::CORRUPTION_DETECTED);
        }
        if op3 > opStart4 {
            return error(code::CORRUPTION_DETECTED);
        }
        /* note : op4 already verified within main loop */

        /* finish bitStreams one by one */
        HUF_decodeStreamX2(op1, &mut bitD1, opStart2, dt, dtLog);
        HUF_decodeStreamX2(op2, &mut bitD2, opStart3, dt, dtLog);
        HUF_decodeStreamX2(op3, &mut bitD3, opStart4, dt, dtLog);
        HUF_decodeStreamX2(op4, &mut bitD4, oend, dt, dtLog);

        /* check */
        {
            let endCheck = bit_end_of_dstream(&bitD1)
                & bit_end_of_dstream(&bitD2)
                & bit_end_of_dstream(&bitD3)
                & bit_end_of_dstream(&bitD4);
            if endCheck == 0 {
                return error(code::CORRUPTION_DETECTED);
            }
        }

        dstSize
    }
}

unsafe fn HUF_decompress4X2_usingDTable_internal_default(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
) -> usize {
    HUF_decompress4X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable)
}

unsafe fn HUF_decompress4X2_usingDTable_internal_fast_c_loop(args: *mut HUF_DecompressFastArgs) {
    let mut bits: [u64; 4] = [0; 4];
    let mut ip: [*const u8; 4] = [core::ptr::null(); 4];
    let mut op: [*mut u8; 4] = [core::ptr::null_mut(); 4];
    let mut oend: [*mut u8; 4] = [core::ptr::null_mut(); 4];
    let dtable = (*args).dt as *const HUF_DEltX2;
    let ilowest = (*args).ilowest;

    core::ptr::copy_nonoverlapping((*args).bits.as_ptr(), bits.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).ip.as_ptr(), ip.as_mut_ptr(), 4);
    core::ptr::copy_nonoverlapping((*args).op.as_ptr(), op.as_mut_ptr(), 4);

    oend[0] = op[1];
    oend[1] = op[2];
    oend[2] = op[3];
    oend[3] = (*args).oend;

    /* HUF_4X2_DECODE_SYMBOL(_stream, _decode3) */
    macro_rules! decode_symbol {
        ($stream:expr, $decode3:expr) => {{
            if ($decode3 != 0) || ($stream != 3) {
                let index = (bits[$stream] >> 53) as usize;
                let entry = *dtable.add(index);
                mem_write16(op[$stream] as *mut c_void, entry.sequence);
                bits[$stream] <<= (entry.nbBits & 0x3F) as u32;
                op[$stream] = op[$stream].add(entry.length as usize);
            }
        }};
    }
    /* HUF_4X2_RELOAD_STREAM(_stream) */
    macro_rules! reload_stream {
        ($stream:expr) => {{
            decode_symbol!(3, 1);
            {
                let ctz = count_trailing_zeros64(bits[$stream]) as i32;
                let nbBits = ctz & 7;
                let nbBytes = ctz >> 3;
                ip[$stream] = ip[$stream].sub(nbBytes as usize);
                bits[$stream] = mem_read64(ip[$stream] as *const c_void) | 1;
                bits[$stream] <<= nbBits as u32;
            }
        }};
    }

    'outer: loop {
        let olimit: *mut u8;
        let mut stream: i32;

        /* Compute olimit */
        {
            let mut iters = (ip[0] as usize - ilowest as usize) / 7;
            stream = 0;
            while stream < 4 {
                let oiters = (oend[stream as usize] as usize - op[stream as usize] as usize) / 10;
                iters = MIN(iters, oiters);
                stream += 1;
            }

            olimit = op[3].add(iters * 5);

            if op[3] == olimit {
                break 'outer;
            }

            stream = 1;
            while stream < 4 {
                if ip[stream as usize] < ip[(stream - 1) as usize] {
                    break 'outer;
                }
                stream += 1;
            }
        }

        loop {
            /* Decode 5 symbols from each of the first 3 streams (decode3=0). */
            decode_symbol!(0, 0);
            decode_symbol!(1, 0);
            decode_symbol!(2, 0);
            decode_symbol!(3, 0);
            decode_symbol!(0, 0);
            decode_symbol!(1, 0);
            decode_symbol!(2, 0);
            decode_symbol!(3, 0);
            decode_symbol!(0, 0);
            decode_symbol!(1, 0);
            decode_symbol!(2, 0);
            decode_symbol!(3, 0);
            decode_symbol!(0, 0);
            decode_symbol!(1, 0);
            decode_symbol!(2, 0);
            decode_symbol!(3, 0);
            decode_symbol!(0, 0);
            decode_symbol!(1, 0);
            decode_symbol!(2, 0);
            decode_symbol!(3, 0);

            /* Decode one symbol from the final stream */
            decode_symbol!(3, 1);

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

unsafe fn HUF_decompress4X2_usingDTable_internal_fast(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    loopFn: HUF_DecompressFastLoopFn,
) -> usize {
    let dt = DTable.add(1) as *const c_void;
    let _ilowest = cSrc as *const u8;
    let oend = ZSTD_maybeNullPtrAdd(dst as *mut u8, dstSize as isize);
    let mut args: HUF_DecompressFastArgs = core::mem::zeroed();
    {
        let ret = HUF_DecompressFastArgs_init(&mut args, dst, dstSize, cSrc, cSrcSize, DTable);
        if err_is_error(ret) != 0 {
            return ret;
        }
        if ret == 0 {
            return 0;
        }
    }

    loopFn(&mut args);

    /* finish bitStreams one by one */
    {
        let segmentSize = (dstSize + 3) / 4;
        let mut segmentEnd = dst as *mut u8;
        let mut i = 0i32;
        while i < 4 {
            let mut bit = core::mem::zeroed::<BIT_DStream_t>();
            if segmentSize <= (oend as usize - segmentEnd as usize) {
                segmentEnd = segmentEnd.add(segmentSize);
            } else {
                segmentEnd = oend;
            }
            let e = HUF_initRemainingDStream(&mut bit, &args, i, segmentEnd);
            if err_is_error(e) != 0 {
                return e;
            }
            args.op[i as usize] = args.op[i as usize].add(HUF_decodeStreamX2(
                args.op[i as usize],
                &mut bit,
                segmentEnd,
                dt as *const HUF_DEltX2,
                HUF_DECODER_FAST_TABLELOG,
            ));
            if args.op[i as usize] != segmentEnd {
                return error(code::CORRUPTION_DETECTED);
            }
            i += 1;
        }
    }

    dstSize
}

unsafe fn HUF_decompress4X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: i32,
) -> usize {
    let fallbackFn: HUF_DecompressUsingDTableFn =
        HUF_decompress4X2_usingDTable_internal_default;
    let loopFn: HUF_DecompressFastLoopFn =
        HUF_decompress4X2_usingDTable_internal_fast_c_loop;

    /* DYNAMIC_BMI2=0, ZSTD_ENABLE_ASM_X86_64_BMI2=0 */
    if HUF_ENABLE_FAST_DECODE && (flags & HUF_flags_disableFast) == 0 {
        let ret =
            HUF_decompress4X2_usingDTable_internal_fast(dst, dstSize, cSrc, cSrcSize, DTable, loopFn);
        if ret != 0 {
            return ret;
        }
    }
    fallbackFn(dst, dstSize, cSrc, cSrcSize, DTable)
}

/* HUF_DGEN(HUF_decompress1X2_usingDTable_internal) : DYNAMIC_BMI2=0 variant */
unsafe fn HUF_decompress1X2_usingDTable_internal(
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    _flags: i32,
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
    flags: i32,
) -> usize {
    let mut ip = cSrc as *const u8;

    let hSize = HUF_readDTableX2_wksp(DCtx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if err_is_error(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress1X2_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, DCtx, flags)
}

unsafe fn HUF_decompress4X2_DCtx_wksp(
    dctx: *mut HUF_DTable,
    dst: *mut c_void,
    dstSize: usize,
    cSrc: *const c_void,
    mut cSrcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: i32,
) -> usize {
    let mut ip = cSrc as *const u8;

    let hSize = HUF_readDTableX2_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if err_is_error(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress4X2_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx, flags)
}

/* ***********************************/
/* Universal decompression selectors */
/* ***********************************/

#[repr(C)]
#[derive(Clone, Copy)]
struct algo_time_t {
    tableTime: u32,
    decode256Time: u32,
}

static algoTime: [[algo_time_t; 2]; 16] = [
    /* single, double */
    [algo_time_t { tableTime: 0, decode256Time: 0 }, algo_time_t { tableTime: 1, decode256Time: 1 }], /* Q==0 : impossible */
    [algo_time_t { tableTime: 0, decode256Time: 0 }, algo_time_t { tableTime: 1, decode256Time: 1 }], /* Q==1 : impossible */
    [algo_time_t { tableTime: 150, decode256Time: 216 }, algo_time_t { tableTime: 381, decode256Time: 119 }], /* Q == 2 */
    [algo_time_t { tableTime: 170, decode256Time: 205 }, algo_time_t { tableTime: 514, decode256Time: 112 }], /* Q == 3 */
    [algo_time_t { tableTime: 177, decode256Time: 199 }, algo_time_t { tableTime: 539, decode256Time: 110 }], /* Q == 4 */
    [algo_time_t { tableTime: 197, decode256Time: 194 }, algo_time_t { tableTime: 644, decode256Time: 107 }], /* Q == 5 */
    [algo_time_t { tableTime: 221, decode256Time: 192 }, algo_time_t { tableTime: 735, decode256Time: 107 }], /* Q == 6 */
    [algo_time_t { tableTime: 256, decode256Time: 189 }, algo_time_t { tableTime: 881, decode256Time: 106 }], /* Q == 7 */
    [algo_time_t { tableTime: 359, decode256Time: 188 }, algo_time_t { tableTime: 1167, decode256Time: 109 }], /* Q == 8 */
    [algo_time_t { tableTime: 582, decode256Time: 187 }, algo_time_t { tableTime: 1570, decode256Time: 114 }], /* Q == 9 */
    [algo_time_t { tableTime: 688, decode256Time: 187 }, algo_time_t { tableTime: 1712, decode256Time: 122 }], /* Q ==10 */
    [algo_time_t { tableTime: 825, decode256Time: 186 }, algo_time_t { tableTime: 1965, decode256Time: 136 }], /* Q ==11 */
    [algo_time_t { tableTime: 976, decode256Time: 185 }, algo_time_t { tableTime: 2131, decode256Time: 150 }], /* Q ==12 */
    [algo_time_t { tableTime: 1180, decode256Time: 186 }, algo_time_t { tableTime: 2070, decode256Time: 175 }], /* Q ==13 */
    [algo_time_t { tableTime: 1377, decode256Time: 185 }, algo_time_t { tableTime: 1731, decode256Time: 202 }], /* Q ==14 */
    [algo_time_t { tableTime: 1412, decode256Time: 185 }, algo_time_t { tableTime: 1695, decode256Time: 202 }], /* Q ==15 */
];

/** HUF_selectDecoder() :
 * @return : 0==HUF_decompress4X1, 1==HUF_decompress4X2 . */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_selectDecoder(dstSize: usize, cSrcSize: usize) -> u32 {
    /* decoder timing evaluation */
    let Q: u32 = if cSrcSize >= dstSize {
        15
    } else {
        (cSrcSize * 16 / dstSize) as u32
    };
    let D256 = (dstSize >> 8) as u32;
    let DTime0 = algoTime[Q as usize][0]
        .tableTime
        .wrapping_add(algoTime[Q as usize][0].decode256Time.wrapping_mul(D256));
    let mut DTime1 = algoTime[Q as usize][1]
        .tableTime
        .wrapping_add(algoTime[Q as usize][1].decode256Time.wrapping_mul(D256));
    DTime1 = DTime1.wrapping_add(DTime1 >> 5);
    (DTime1 < DTime0) as u32
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
    flags: i32,
) -> usize {
    /* validation checks */
    if dstSize == 0 {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if cSrcSize > dstSize {
        return error(code::CORRUPTION_DETECTED);
    }
    if cSrcSize == dstSize {
        core::ptr::copy_nonoverlapping(cSrc as *const u8, dst as *mut u8, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 {
        core::ptr::write_bytes(dst as *mut u8, *(cSrc as *const u8), dstSize);
        return dstSize;
    }

    {
        let algoNb = HUF_selectDecoder(dstSize, cSrcSize);
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
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: i32,
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
    flags: i32,
) -> usize {
    let mut ip = cSrc as *const u8;

    let hSize = HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if err_is_error(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return error(code::SRCSIZE_WRONG);
    }
    ip = ip.add(hSize);
    cSrcSize -= hSize;

    HUF_decompress1X1_usingDTable_internal(dst, dstSize, ip as *const c_void, cSrcSize, dctx, flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_usingDTable(
    dst: *mut c_void,
    maxDstSize: usize,
    cSrc: *const c_void,
    cSrcSize: usize,
    DTable: *const HUF_DTable,
    flags: i32,
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
    flags: i32,
) -> usize {
    /* validation checks */
    if dstSize == 0 {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if cSrcSize == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    {
        let algoNb = HUF_selectDecoder(dstSize, cSrcSize);
        if algoNb != 0 {
            HUF_decompress4X2_DCtx_wksp(dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags)
        } else {
            HUF_decompress4X1_DCtx_wksp(dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags)
        }
    }
}










