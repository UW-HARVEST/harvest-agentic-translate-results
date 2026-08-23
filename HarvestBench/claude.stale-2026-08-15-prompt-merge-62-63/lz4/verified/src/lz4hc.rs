// Translation of lz4hc.c (LZ4 HC v1.10.0). Target: x86_64 little-endian. LZ4HC_HEAPMODE=1.
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::lz4::{
    limitedOutput_directive, LZ4_compressBound, LZ4_count, LZ4_isLittleEndian, LZ4_memcpy,
    LZ4_memmove, LZ4_read16, LZ4_read32, LZ4_read_ARCH, LZ4_wildCopy8, LZ4_writeLE16,
    LZ4_NbCommonBytes, LASTLITERALS, LZ4_DISTANCE_MAX, LZ4_MAX_INPUT_SIZE, LZ4_minLength, MFLIMIT,
    MINMATCH, ML_BITS, ML_MASK, RUN_MASK,
};
use limitedOutput_directive::*;

const LZ4HC_CLEVEL_MIN: c_int = 2;
const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
const LZ4HC_CLEVEL_MAX: c_int = 12;

const LZ4HC_DICTIONARY_LOGSIZE: usize = 16;
const LZ4HC_MAXD: usize = 1 << LZ4HC_DICTIONARY_LOGSIZE;
const LZ4HC_HASH_LOG: usize = 15;
const LZ4HC_HASHTABLESIZE: usize = 1 << LZ4HC_HASH_LOG;

const LZ4_STREAMHC_MINSIZE: usize = 262200;

const OPTIMAL_ML: c_int = ((ML_MASK - 1) + MINMATCH as u32) as c_int;
const LZ4_OPT_NUM: usize = 1 << 12;

const LZ4HC_HASHSIZE: usize = 4;
const LZ4MID_HASHLOG: usize = LZ4HC_HASH_LOG - 1;
const LZ4MID_HASHTABLESIZE: usize = 1 << LZ4MID_HASHLOG;
const LZ4MID_HASHSIZE: usize = 8;

const LZ4_DISTANCE_MAX_US: usize = LZ4_DISTANCE_MAX as usize;

#[inline]
fn MIN(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
#[inline]
fn MAXi(a: c_int, b: c_int) -> c_int {
    if a > b { a } else { b }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4HC_CCtx_internal {
    pub hashTable: [u32; LZ4HC_HASHTABLESIZE],
    pub chainTable: [u16; LZ4HC_MAXD],
    pub end: *const u8,
    pub prefixStart: *const u8,
    pub dictStart: *const u8,
    pub dictLimit: u32,
    pub lowLimit: u32,
    pub nextToUpdate: u32,
    pub compressionLevel: i16,
    pub favorDecSpeed: i8,
    pub dirty: i8,
    pub dictCtx: *const LZ4HC_CCtx_internal,
}

#[repr(C, align(8))]
pub union LZ4_streamHC_t {
    pub minStateSize: [u8; LZ4_STREAMHC_MINSIZE],
    pub internal_donotuse: LZ4HC_CCtx_internal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum dictCtx_directive {
    noDictCtx,
    usingDictCtxHc,
}
use dictCtx_directive::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum lz4hc_strat_e {
    lz4mid,
    lz4hc,
    lz4opt,
}
use lz4hc_strat_e::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum repeat_state_e {
    rep_untested,
    rep_not,
    rep_confirmed,
}
use repeat_state_e::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HCfavor_e {
    favorCompressionRatio = 0,
    favorDecompressionSpeed = 1,
}
use HCfavor_e::*;

#[derive(Clone, Copy)]
struct cParams_t {
    strat: lz4hc_strat_e,
    nbSearches: c_int,
    targetLength: u32,
}

static k_clTable: [cParams_t; (LZ4HC_CLEVEL_MAX + 1) as usize] = [
    cParams_t { strat: lz4mid, nbSearches: 2, targetLength: 16 },
    cParams_t { strat: lz4mid, nbSearches: 2, targetLength: 16 },
    cParams_t { strat: lz4mid, nbSearches: 2, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 4, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 8, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 16, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 32, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 64, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 128, targetLength: 16 },
    cParams_t { strat: lz4hc, nbSearches: 256, targetLength: 16 },
    cParams_t { strat: lz4opt, nbSearches: 96, targetLength: 64 },
    cParams_t { strat: lz4opt, nbSearches: 512, targetLength: 128 },
    cParams_t { strat: lz4opt, nbSearches: 16384, targetLength: LZ4_OPT_NUM as u32 },
];

fn LZ4HC_getCLevelParams(mut cLevel: c_int) -> cParams_t {
    if cLevel < 1 {
        cLevel = LZ4HC_CLEVEL_DEFAULT;
    }
    cLevel = MIN(LZ4HC_CLEVEL_MAX as usize, cLevel as usize) as c_int;
    k_clTable[cLevel as usize]
}

#[inline]
fn HASH_FUNCTION(i: u32) -> u32 {
    (i.wrapping_mul(2654435761)) >> ((MINMATCH * 8) - LZ4HC_HASH_LOG)
}
#[inline]
unsafe fn LZ4HC_hashPtr(ptr: *const u8) -> u32 {
    HASH_FUNCTION(LZ4_read32(ptr))
}
#[inline]
unsafe fn LZ4_read64(memPtr: *const u8) -> u64 {
    (memPtr as *const u64).read_unaligned()
}
#[inline]
fn LZ4MID_hash4(v: u32) -> u32 {
    v.wrapping_mul(2654435761) >> (32 - LZ4MID_HASHLOG)
}
#[inline]
unsafe fn LZ4MID_hash4Ptr(ptr: *const u8) -> u32 {
    LZ4MID_hash4(LZ4_read32(ptr))
}
#[inline]
fn LZ4MID_hash7(v: u64) -> u32 {
    ((v << (64 - 56)).wrapping_mul(58295818150454627) >> (64 - LZ4MID_HASHLOG)) as u32
}
#[inline]
unsafe fn LZ4_readLE64(memPtr: *const u8) -> u64 {
    if LZ4_isLittleEndian() {
        LZ4_read64(memPtr)
    } else {
        0
    }
}
#[inline]
unsafe fn LZ4MID_hash8Ptr(ptr: *const u8) -> u32 {
    LZ4MID_hash7(LZ4_readLE64(ptr))
}

#[inline]
fn LZ4HC_NbCommonBytes32(val: u32) -> u32 {
    // little-endian: __builtin_clz(val) >> 3
    val.leading_zeros() >> 3
}

#[inline]
unsafe fn LZ4HC_countBack(
    ip: *const u8,
    match_: *const u8,
    iMin: *const u8,
    mMin: *const u8,
) -> c_int {
    let mut back: c_int = 0;
    let min = MAXi(
        (iMin as isize - ip as isize) as c_int,
        (mMin as isize - match_ as isize) as c_int,
    );
    while (back - min) > 3 {
        let v = LZ4_read32(ip.offset(back as isize - 4))
            ^ LZ4_read32(match_.offset(back as isize - 4));
        if v != 0 {
            return back - LZ4HC_NbCommonBytes32(v) as c_int;
        } else {
            back -= 4;
        }
    }
    while (back > min) && (*ip.offset(back as isize - 1) == *match_.offset(back as isize - 1)) {
        back -= 1;
    }
    back
}

#[inline]
unsafe fn DELTANEXTU16(table: *const u16, pos: u32) -> u16 {
    *table.add((pos as u16) as usize)
}
#[inline]
unsafe fn SET_DELTANEXTU16(table: *mut u16, pos: u32, v: u16) {
    *table.add((pos as u16) as usize) = v;
}

unsafe fn LZ4HC_clearTables(hc4: *mut LZ4HC_CCtx_internal) {
    let hc4 = &mut *hc4;
    ptr::write_bytes(hc4.hashTable.as_mut_ptr() as *mut u8, 0, core::mem::size_of_val(&hc4.hashTable));
    ptr::write_bytes(hc4.chainTable.as_mut_ptr() as *mut u8, 0xFF, core::mem::size_of_val(&hc4.chainTable));
}

unsafe fn LZ4HC_init_internal(hc4: *mut LZ4HC_CCtx_internal, start: *const u8) {
    let hc4r = &mut *hc4;
    let bufferSize = (hc4r.end as usize).wrapping_sub(hc4r.prefixStart as usize);
    let mut newStartingOffset = bufferSize + hc4r.dictLimit as usize;
    if newStartingOffset > (1usize << 30) {
        LZ4HC_clearTables(hc4);
        newStartingOffset = 0;
    }
    newStartingOffset += 64 * 1024;
    hc4r.nextToUpdate = newStartingOffset as u32;
    hc4r.prefixStart = start;
    hc4r.end = start;
    hc4r.dictStart = start;
    hc4r.dictLimit = newStartingOffset as u32;
    hc4r.lowLimit = newStartingOffset as u32;
}

#[inline]
unsafe fn LZ4HC_encodeSequence(
    _ip: &mut *const u8,
    _op: &mut *mut u8,
    _anchor: &mut *const u8,
    matchLength: c_int,
    offset: c_int,
    limit: limitedOutput_directive,
    oend: *mut u8,
) -> c_int {
    let mut length: usize;
    let token = *_op;
    *_op = (*_op).add(1);

    length = (*_ip as usize).wrapping_sub(*_anchor as usize);
    if (limit != notLimited)
        && ((*_op)
            .wrapping_add(length / 255)
            .wrapping_add(length)
            .wrapping_add(2 + 1 + LASTLITERALS)
            > oend)
    {
        return 1;
    }
    if length >= RUN_MASK as usize {
        let mut len = length - RUN_MASK as usize;
        *token = (RUN_MASK << ML_BITS) as u8;
        while len >= 255 {
            **_op = 255;
            *_op = (*_op).add(1);
            len -= 255;
        }
        **_op = len as u8;
        *_op = (*_op).add(1);
    } else {
        *token = ((length as u32) << ML_BITS) as u8;
    }

    LZ4_wildCopy8(*_op, *_anchor, (*_op).wrapping_add(length));
    *_op = (*_op).add(length);

    LZ4_writeLE16(*_op, offset as u16);
    *_op = (*_op).add(2);

    length = (matchLength - MINMATCH as c_int) as usize;
    if (limit != notLimited)
        && ((*_op).wrapping_add(length / 255).wrapping_add(1 + LASTLITERALS) > oend)
    {
        return 1;
    }
    if length >= ML_MASK as usize {
        *token += ML_MASK as u8;
        length -= ML_MASK as usize;
        while length >= 510 {
            **_op = 255;
            *_op = (*_op).add(1);
            **_op = 255;
            *_op = (*_op).add(1);
            length -= 510;
        }
        if length >= 255 {
            length -= 255;
            **_op = 255;
            *_op = (*_op).add(1);
        }
        **_op = length as u8;
        *_op = (*_op).add(1);
    } else {
        *token += length as u8;
    }

    *_ip = (*_ip).add(matchLength as usize);
    *_anchor = *_ip;

    0
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4HC_match_t {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4HC_searchExtDict(
    ip: *const u8,
    ipIndex: u32,
    iLowLimit: *const u8,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
    mut currentBestML: c_int,
    mut nbAttempts: c_int,
) -> LZ4HC_match_t {
    let dc = &*dictCtx;
    let lDictEndIndex = (dc.end as usize).wrapping_sub(dc.prefixStart as usize) + dc.dictLimit as usize;
    let mut lDictMatchIndex = dc.hashTable[LZ4HC_hashPtr(ip) as usize];
    let mut matchIndex = lDictMatchIndex
        .wrapping_add(gDictEndIndex)
        .wrapping_sub(lDictEndIndex as u32);
    let mut offset: c_int = 0;
    let mut sBack: c_int = 0;

    while ipIndex.wrapping_sub(matchIndex) <= LZ4_DISTANCE_MAX && {
        let cont = nbAttempts != 0;
        nbAttempts -= 1;
        cont
    } {
        let matchPtr = dc
            .prefixStart
            .wrapping_sub(dc.dictLimit as usize)
            .wrapping_add(lDictMatchIndex as usize);

        if LZ4_read32(matchPtr) == LZ4_read32(ip) {
            let mut vLimit = ip.wrapping_add(lDictEndIndex - lDictMatchIndex as usize);
            if vLimit > iHighLimit {
                vLimit = iHighLimit;
            }
            let mut mlt =
                LZ4_count(ip.add(MINMATCH), matchPtr.add(MINMATCH), vLimit) as c_int + MINMATCH as c_int;
            let back = if ip > iLowLimit {
                LZ4HC_countBack(ip, matchPtr, iLowLimit, dc.prefixStart)
            } else {
                0
            };
            mlt -= back;
            if mlt > currentBestML {
                currentBestML = mlt;
                offset = ipIndex.wrapping_sub(matchIndex) as c_int;
                sBack = back;
            }
        }

        let nextOffset = DELTANEXTU16(dc.chainTable.as_ptr(), lDictMatchIndex);
        lDictMatchIndex = lDictMatchIndex.wrapping_sub(nextOffset as u32);
        matchIndex = matchIndex.wrapping_sub(nextOffset as u32);
    }

    LZ4HC_match_t {
        len: currentBestML,
        off: offset,
        back: sBack,
    }
}

type LZ4MID_searchIntoDict_f = unsafe fn(
    *const u8,
    u32,
    *const u8,
    *const LZ4HC_CCtx_internal,
    u32,
) -> LZ4HC_match_t;

unsafe fn LZ4MID_searchHCDict(
    ip: *const u8,
    ipIndex: u32,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
) -> LZ4HC_match_t {
    LZ4HC_searchExtDict(ip, ipIndex, ip, iHighLimit, dictCtx, gDictEndIndex, MINMATCH as c_int - 1, 2)
}

unsafe fn LZ4MID_searchExtDict(
    ip: *const u8,
    ipIndex: u32,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
) -> LZ4HC_match_t {
    let dc = &*dictCtx;
    let lDictEndIndex =
        (dc.end as usize).wrapping_sub(dc.prefixStart as usize) + dc.dictLimit as usize;
    let hash4Table = dc.hashTable.as_ptr();
    let hash8Table = hash4Table.add(LZ4MID_HASHTABLESIZE);

    {
        let l8DictMatchIndex = *hash8Table.add(LZ4MID_hash8Ptr(ip) as usize);
        let m8Index = l8DictMatchIndex
            .wrapping_add(gDictEndIndex)
            .wrapping_sub(lDictEndIndex as u32);
        if ipIndex.wrapping_sub(m8Index) <= LZ4_DISTANCE_MAX {
            let matchPtr = dc
                .prefixStart
                .wrapping_sub(dc.dictLimit as usize)
                .wrapping_add(l8DictMatchIndex as usize);
            let safeLen = MIN(
                lDictEndIndex - l8DictMatchIndex as usize,
                (iHighLimit as usize) - (ip as usize),
            );
            let mlt = LZ4_count(ip, matchPtr, ip.add(safeLen)) as c_int;
            if mlt >= MINMATCH as c_int {
                return LZ4HC_match_t {
                    len: mlt,
                    off: ipIndex.wrapping_sub(m8Index) as c_int,
                    back: 0,
                };
            }
        }
    }

    {
        let l4DictMatchIndex = *hash4Table.add(LZ4MID_hash4Ptr(ip) as usize);
        let m4Index = l4DictMatchIndex
            .wrapping_add(gDictEndIndex)
            .wrapping_sub(lDictEndIndex as u32);
        if ipIndex.wrapping_sub(m4Index) <= LZ4_DISTANCE_MAX {
            let matchPtr = dc
                .prefixStart
                .wrapping_sub(dc.dictLimit as usize)
                .wrapping_add(l4DictMatchIndex as usize);
            let safeLen = MIN(
                lDictEndIndex - l4DictMatchIndex as usize,
                (iHighLimit as usize) - (ip as usize),
            );
            let mlt = LZ4_count(ip, matchPtr, ip.add(safeLen)) as c_int;
            if mlt >= MINMATCH as c_int {
                return LZ4HC_match_t {
                    len: mlt,
                    off: ipIndex.wrapping_sub(m4Index) as c_int,
                    back: 0,
                };
            }
        }
    }

    LZ4HC_match_t { off: 0, len: 0, back: 0 }
}

#[inline]
unsafe fn LZ4MID_addPosition(hTable: *mut u32, hValue: u32, index: u32) {
    *hTable.add(hValue as usize) = index;
}

unsafe fn LZ4MID_fillHTable(cctx: *mut LZ4HC_CCtx_internal, dict: *const c_void, size: usize) {
    let c = &mut *cctx;
    let hash4Table = c.hashTable.as_mut_ptr();
    let hash8Table = hash4Table.add(LZ4MID_HASHTABLESIZE);
    let prefixPtr = dict as *const u8;
    let prefixIdx = c.dictLimit;
    let target = prefixIdx.wrapping_add(size as u32).wrapping_sub(LZ4MID_HASHSIZE as u32);
    let mut idx = c.nextToUpdate;
    if size <= LZ4MID_HASHSIZE {
        return;
    }

    while idx < target {
        LZ4MID_addPosition(hash4Table, LZ4MID_hash4Ptr(prefixPtr.add((idx - prefixIdx) as usize)), idx);
        LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(prefixPtr.add((idx + 1 - prefixIdx) as usize)), idx + 1);
        idx += 3;
    }

    idx = if size > 32 * 1024 + LZ4MID_HASHSIZE {
        target - 32 * 1024
    } else {
        c.nextToUpdate
    };
    while idx < target {
        LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(prefixPtr.add((idx - prefixIdx) as usize)), idx);
        idx += 1;
    }

    c.nextToUpdate = target;
}

fn select_searchDict_function(dictCtx: *const LZ4HC_CCtx_internal) -> Option<LZ4MID_searchIntoDict_f> {
    if dictCtx.is_null() {
        return None;
    }
    let lvl = unsafe { (*dictCtx).compressionLevel };
    if LZ4HC_getCLevelParams(lvl as c_int).strat == lz4mid {
        Some(LZ4MID_searchExtDict as LZ4MID_searchIntoDict_f)
    } else {
        Some(LZ4MID_searchHCDict as LZ4MID_searchIntoDict_f)
    }
}

/* ===== last-literals helper shared by mid / hashChain / optimal ===== */
unsafe fn hc_last_literals(
    source: *const c_char,
    dest: *mut c_char,
    srcSizePtr: *mut c_int,
    anchor: *const u8,
    mut op: *mut u8,
    iend: *const u8,
    limit: limitedOutput_directive,
    mut oend: *mut u8,
) -> c_int {
    let mut lastRunSize = (iend as usize - anchor as usize) as usize;
    let mut llAdd = (lastRunSize + 255 - RUN_MASK as usize) / 255;
    let totalSize = 1 + llAdd + lastRunSize;
    if limit == fillOutput {
        oend = oend.wrapping_add(LASTLITERALS);
    }
    if (limit != notLimited) && (op.wrapping_add(totalSize) > oend) {
        if limit == limitedOutput {
            return 0;
        }
        lastRunSize = (oend as usize - op as usize) - 1;
        llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
        lastRunSize -= llAdd;
    }
    let ip = anchor.add(lastRunSize);

    if lastRunSize >= RUN_MASK as usize {
        let mut accumulator = lastRunSize - RUN_MASK as usize;
        *op = (RUN_MASK << ML_BITS) as u8;
        op = op.add(1);
        while accumulator >= 255 {
            *op = 255;
            op = op.add(1);
            accumulator -= 255;
        }
        *op = accumulator as u8;
        op = op.add(1);
    } else {
        *op = ((lastRunSize as u32) << ML_BITS) as u8;
        op = op.add(1);
    }
    LZ4_memcpy(op, anchor, lastRunSize);
    op = op.add(lastRunSize);

    *srcSizePtr = (ip as usize - source as usize) as c_int;
    (op as usize - dest as usize) as c_int
}

/* ===== Mid compression (level 2) ===== */
unsafe fn LZ4MID_compress(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    maxOutputSize: c_int,
    limit: limitedOutput_directive,
    dict: dictCtx_directive,
) -> c_int {
    let c = &mut *ctx;
    let hash4Table = c.hashTable.as_mut_ptr();
    let hash8Table = hash4Table.add(LZ4MID_HASHTABLESIZE);
    let mut ip = src as *const u8;
    let mut anchor = ip;
    let iend = ip.add(*srcSizePtr as usize);
    let mflimit = iend.wrapping_sub(MFLIMIT);
    let matchlimit = iend.wrapping_sub(LASTLITERALS);
    let ilimit = iend.wrapping_sub(LZ4MID_HASHSIZE);
    let mut op = dst as *mut u8;
    let mut oend = op.wrapping_add(maxOutputSize as usize);

    let prefixPtr = c.prefixStart;
    let prefixIdx = c.dictLimit;
    /* `ilimit` is `iend - LZ4MID_HASHSIZE`, which is *before* `prefixPtr` when
     * the input is shorter than LZ4MID_HASHSIZE.  The C computes
     * `(U32)(ilimit - prefixPtr) + prefixIdx` with signed pointer subtraction
     * and wrap-around unsigned addition, so mirror that exactly instead of
     * panicking on the underflow. */
    let ilimitIdx = ((ilimit as usize).wrapping_sub(prefixPtr as usize) as u32)
        .wrapping_add(prefixIdx);
    let dictStart = c.dictStart;
    let dictIdx = c.lowLimit;
    let gDictEndIndex = c.lowLimit;
    let searchIntoDict = if dict == usingDictCtxHc {
        select_searchDict_function(c.dictCtx)
    } else {
        None
    };
    let mut matchLength: u32;
    let mut matchDistance: u32;

    if *srcSizePtr < 0 {
        return 0;
    }
    if maxOutputSize < 0 {
        return 0;
    }
    if *srcSizePtr > LZ4_MAX_INPUT_SIZE {
        return 0;
    }
    if limit == fillOutput {
        oend = oend.wrapping_sub(LASTLITERALS);
    }

    // encode+overflow helper: returns Some(final_result) on overflow, else None (continue).
    macro_rules! encode_seq {
        ($ipIndex:expr) => {{
            let ipIndex = $ipIndex;
            // catch back
            while (((ip) > anchor)
                && (((ip as usize - prefixPtr as usize) as u32) > matchDistance))
                && (*ip.offset(-1) == *ip.offset(-(matchDistance as isize) - 1))
            {
                ip = ip.offset(-1);
                matchLength += 1;
            }
            LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(ip.add(1)), ipIndex + 1);
            LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(ip.add(2)), ipIndex + 2);
            LZ4MID_addPosition(hash4Table, LZ4MID_hash4Ptr(ip.add(1)), ipIndex + 1);

            let saved_op = op;
            if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, matchLength as c_int, matchDistance as c_int, limit, oend) != 0 {
                op = saved_op;
                // _lz4mid_dest_overflow
                if limit == fillOutput {
                    let ll = (ip as usize - anchor as usize) as usize;
                    let ll_addbytes = (ll + 240) / 255;
                    let ll_totalCost = 1 + ll_addbytes + ll;
                    let maxLitPos = oend.wrapping_sub(3);
                    if op.wrapping_add(ll_totalCost) <= maxLitPos {
                        let bytesLeftForMl = (maxLitPos as usize) - ((op as usize) + ll_totalCost);
                        let maxMlSize = MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                        if (matchLength as usize) > maxMlSize {
                            matchLength = maxMlSize as u32;
                        }
                        if ((oend as usize + LASTLITERALS) as isize
                            - ((op as usize) + ll_totalCost + 2) as isize
                            - 1
                            + matchLength as isize)
                            >= MFLIMIT as isize
                        {
                            LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, matchLength as c_int, matchDistance as c_int, notLimited, oend);
                        }
                    }
                    return hc_last_literals(src, dst, srcSizePtr, anchor, op, iend, limit, oend);
                }
                return 0;
            }
            {
                let endMatchIdx = ((ip as usize - prefixPtr as usize) as u32) + prefixIdx;
                let pos_m2 = endMatchIdx - 2;
                if pos_m2 < ilimitIdx {
                    if ((ip as usize - prefixPtr as usize) as isize) > 5 {
                        LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(ip.offset(-5)), endMatchIdx - 5);
                    }
                    LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(ip.offset(-3)), endMatchIdx - 3);
                    LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(ip.offset(-2)), endMatchIdx - 2);
                    LZ4MID_addPosition(hash4Table, LZ4MID_hash4Ptr(ip.offset(-2)), endMatchIdx - 2);
                    LZ4MID_addPosition(hash4Table, LZ4MID_hash4Ptr(ip.offset(-1)), endMatchIdx - 1);
                }
            }
        }};
    }

    if *srcSizePtr >= LZ4_minLength {
        'mainloop: while ip <= mflimit {
            let ipIndex = (ip as usize - prefixPtr as usize) as u32 + prefixIdx;

            // search long match
            {
                let h8 = LZ4MID_hash8Ptr(ip);
                let pos8 = *hash8Table.add(h8 as usize);
                LZ4MID_addPosition(hash8Table, h8, ipIndex);
                if ipIndex.wrapping_sub(pos8) <= LZ4_DISTANCE_MAX {
                    if pos8 >= prefixIdx {
                        let matchPtr = prefixPtr.add((pos8 - prefixIdx) as usize);
                        matchLength = LZ4_count(ip, matchPtr, matchlimit);
                        if matchLength >= MINMATCH as u32 {
                            matchDistance = ipIndex.wrapping_sub(pos8);
                            encode_seq!(ipIndex);
                            continue 'mainloop;
                        }
                    } else if pos8 >= dictIdx {
                        let matchPtr = dictStart.add((pos8 - dictIdx) as usize);
                        let safeLen = MIN((prefixIdx - pos8) as usize, (matchlimit as usize) - (ip as usize));
                        matchLength = LZ4_count(ip, matchPtr, ip.add(safeLen));
                        if matchLength >= MINMATCH as u32 {
                            matchDistance = ipIndex.wrapping_sub(pos8);
                            encode_seq!(ipIndex);
                            continue 'mainloop;
                        }
                    }
                }
            }

            // search short match
            {
                let h4 = LZ4MID_hash4Ptr(ip);
                let pos4 = *hash4Table.add(h4 as usize);
                LZ4MID_addPosition(hash4Table, h4, ipIndex);
                if ipIndex.wrapping_sub(pos4) <= LZ4_DISTANCE_MAX {
                    if pos4 >= prefixIdx {
                        let matchPtr = prefixPtr.add((pos4 - prefixIdx) as usize);
                        matchLength = LZ4_count(ip, matchPtr, matchlimit);
                        if matchLength >= MINMATCH as u32 {
                            let h8 = LZ4MID_hash8Ptr(ip.add(1));
                            let pos8 = *hash8Table.add(h8 as usize);
                            let m2Distance = ipIndex + 1 - pos8;
                            matchDistance = ipIndex.wrapping_sub(pos4);
                            if m2Distance <= LZ4_DISTANCE_MAX && pos8 >= prefixIdx && ip < mflimit {
                                let m2Ptr = prefixPtr.add((pos8 - prefixIdx) as usize);
                                let ml2 = LZ4_count(ip.add(1), m2Ptr, matchlimit);
                                if ml2 > matchLength {
                                    LZ4MID_addPosition(hash8Table, h8, ipIndex + 1);
                                    ip = ip.add(1);
                                    matchLength = ml2;
                                    matchDistance = m2Distance;
                                }
                            }
                            encode_seq!(ipIndex);
                            continue 'mainloop;
                        }
                    } else if pos4 >= dictIdx {
                        let matchPtr = dictStart.add((pos4 - dictIdx) as usize);
                        let safeLen = MIN((prefixIdx - pos4) as usize, (matchlimit as usize) - (ip as usize));
                        matchLength = LZ4_count(ip, matchPtr, ip.add(safeLen));
                        if matchLength >= MINMATCH as u32 {
                            matchDistance = ipIndex.wrapping_sub(pos4);
                            encode_seq!(ipIndex);
                            continue 'mainloop;
                        }
                    }
                }
            }

            if (dict == usingDictCtxHc) && (ipIndex.wrapping_sub(gDictEndIndex) < LZ4_DISTANCE_MAX - 8) {
                let dMatch = (searchIntoDict.unwrap())(ip, ipIndex, matchlimit, c.dictCtx, gDictEndIndex);
                if dMatch.len >= MINMATCH as c_int {
                    matchLength = dMatch.len as u32;
                    matchDistance = dMatch.off as u32;
                    encode_seq!(ipIndex);
                    continue 'mainloop;
                }
            }

            ip = ip.add(1 + ((ip as usize - anchor as usize) >> 9));
        }
    }

    hc_last_literals(src, dst, srcSizePtr, anchor, op, iend, limit, oend)
}

/* ===== HC compression search ===== */
unsafe fn LZ4HC_Insert(hc4: *mut LZ4HC_CCtx_internal, ip: *const u8) {
    let hc = &mut *hc4;
    let chainTable = hc.chainTable.as_mut_ptr();
    let hashTable = hc.hashTable.as_mut_ptr();
    let prefixPtr = hc.prefixStart;
    let prefixIdx = hc.dictLimit;
    let target = (ip as usize - prefixPtr as usize) as u32 + prefixIdx;
    let mut idx = hc.nextToUpdate;

    while idx < target {
        let h = LZ4HC_hashPtr(prefixPtr.add((idx - prefixIdx) as usize));
        let mut delta = idx.wrapping_sub(*hashTable.add(h as usize));
        if delta > LZ4_DISTANCE_MAX {
            delta = LZ4_DISTANCE_MAX;
        }
        SET_DELTANEXTU16(chainTable, idx, delta as u16);
        *hashTable.add(h as usize) = idx;
        idx += 1;
    }

    hc.nextToUpdate = target;
}

#[inline]
fn LZ4HC_rotl32(x: u32, r: i32) -> u32 {
    (x << r) | (x >> (32 - r))
}

fn LZ4HC_rotatePattern(rotate: usize, pattern: u32) -> u32 {
    let bitsToRotate = (rotate & (core::mem::size_of::<u32>() - 1)) << 3;
    if bitsToRotate == 0 {
        return pattern;
    }
    LZ4HC_rotl32(pattern, bitsToRotate as i32)
}

unsafe fn LZ4HC_countPattern(ip_in: *const u8, iEnd: *const u8, pattern32: u32) -> u32 {
    let iStart = ip_in;
    let mut ip = ip_in;
    let pattern: usize = (pattern32 as usize) + ((pattern32 as usize) << 32);

    while ip < iEnd.wrapping_sub(8 - 1) {
        let diff = LZ4_read_ARCH(ip) ^ pattern;
        if diff == 0 {
            ip = ip.add(8);
            continue;
        }
        ip = ip.add(LZ4_NbCommonBytes(diff) as usize);
        return (ip as usize - iStart as usize) as u32;
    }

    let mut patternByte = pattern;
    while (ip < iEnd) && (*ip == (patternByte as u8)) {
        ip = ip.add(1);
        patternByte >>= 8;
    }

    (ip as usize - iStart as usize) as u32
}

unsafe fn LZ4HC_reverseCountPattern(ip_in: *const u8, iLow: *const u8, pattern: u32) -> u32 {
    let iStart = ip_in;
    let mut ip = ip_in;

    while ip >= iLow.wrapping_add(4) {
        if LZ4_read32(ip.offset(-4)) != pattern {
            break;
        }
        ip = ip.offset(-4);
    }
    {
        let mut bytePtr = (&pattern as *const u32 as *const u8).add(3);
        while ip > iLow {
            if *ip.offset(-1) != *bytePtr {
                break;
            }
            ip = ip.offset(-1);
            bytePtr = bytePtr.offset(-1);
        }
    }
    (iStart as usize - ip as usize) as u32
}

fn LZ4HC_protectDictEnd(dictLimit: u32, matchIndex: u32) -> bool {
    (dictLimit.wrapping_sub(1).wrapping_sub(matchIndex)) >= 3
}

unsafe fn LZ4HC_InsertAndGetWiderMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iLowLimit: *const u8,
    iHighLimit: *const u8,
    mut longest: c_int,
    maxNbAttempts: c_int,
    patternAnalysis: c_int,
    chainSwap: c_int,
    dict: dictCtx_directive,
    favorDecSpeed: HCfavor_e,
) -> LZ4HC_match_t {
    let hc = &mut *hc4;
    let chainTable = hc.chainTable.as_mut_ptr();
    let hashTable = hc.hashTable.as_mut_ptr();
    let dictCtx = hc.dictCtx;
    let prefixPtr = hc.prefixStart;
    let prefixIdx = hc.dictLimit;
    let ipIndex = (ip as usize - prefixPtr as usize) as u32 + prefixIdx;
    let withinStartDistance = hc.lowLimit + (LZ4_DISTANCE_MAX + 1) > ipIndex;
    let lowestMatchIndex = if withinStartDistance {
        hc.lowLimit
    } else {
        ipIndex - LZ4_DISTANCE_MAX
    };
    let dictStart = hc.dictStart;
    let dictIdx = hc.lowLimit;
    let dictEnd = dictStart.wrapping_add((prefixIdx - dictIdx) as usize);
    let lookBackLength = (ip as isize - iLowLimit as isize) as c_int;
    let mut nbAttempts = maxNbAttempts;
    let mut matchChainPos: u32 = 0;
    let pattern = LZ4_read32(ip);
    let mut matchIndex: u32;
    let mut repeat = rep_untested;
    let mut srcPatternLength: usize = 0;
    let mut offset: c_int = 0;
    let mut sBack: c_int = 0;

    LZ4HC_Insert(hc4, ip);
    matchIndex = *hashTable.add(LZ4HC_hashPtr(ip) as usize);

    'searchloop: while (matchIndex >= lowestMatchIndex) && (nbAttempts > 0) {
        let mut matchLength: c_int = 0;
        nbAttempts -= 1;
        if (favorDecSpeed == favorDecompressionSpeed) && (ipIndex - matchIndex < 8) {
            // skip
        } else if matchIndex >= prefixIdx {
            let matchPtr = prefixPtr.add((matchIndex - prefixIdx) as usize);
            if LZ4_read16(iLowLimit.offset(longest as isize - 1))
                == LZ4_read16(matchPtr.offset(-(lookBackLength as isize) + longest as isize - 1))
            {
                if LZ4_read32(matchPtr) == pattern {
                    let back = if lookBackLength != 0 {
                        LZ4HC_countBack(ip, matchPtr, iLowLimit, prefixPtr)
                    } else {
                        0
                    };
                    matchLength = MINMATCH as c_int
                        + LZ4_count(ip.add(MINMATCH), matchPtr.add(MINMATCH), iHighLimit) as c_int;
                    matchLength -= back;
                    if matchLength > longest {
                        longest = matchLength;
                        offset = (ipIndex - matchIndex) as c_int;
                        sBack = back;
                    }
                }
            }
        } else {
            let matchPtr = dictStart.add((matchIndex - dictIdx) as usize);
            if (matchIndex <= prefixIdx - 4) && (LZ4_read32(matchPtr) == pattern) {
                let mut vLimit = ip.wrapping_add((prefixIdx - matchIndex) as usize);
                if vLimit > iHighLimit {
                    vLimit = iHighLimit;
                }
                matchLength =
                    LZ4_count(ip.add(MINMATCH), matchPtr.add(MINMATCH), vLimit) as c_int + MINMATCH as c_int;
                if (ip.wrapping_add(matchLength as usize) == vLimit) && (vLimit < iHighLimit) {
                    matchLength +=
                        LZ4_count(ip.wrapping_add(matchLength as usize), prefixPtr, iHighLimit) as c_int;
                }
                let back = if lookBackLength != 0 {
                    LZ4HC_countBack(ip, matchPtr, iLowLimit, dictStart)
                } else {
                    0
                };
                matchLength -= back;
                if matchLength > longest {
                    longest = matchLength;
                    offset = (ipIndex - matchIndex) as c_int;
                    sBack = back;
                }
            }
        }

        if (chainSwap != 0) && (matchLength == longest) {
            if matchIndex + longest as u32 <= ipIndex {
                let kTrigger = 4;
                let mut distanceToNextMatch: u32 = 1;
                let end = longest - MINMATCH as c_int + 1;
                let mut step = 1;
                let mut accel = 1 << kTrigger;
                let mut pos = 0;
                while pos < end {
                    let candidateDist = DELTANEXTU16(chainTable, matchIndex + pos as u32) as u32;
                    step = accel >> kTrigger;
                    accel += 1;
                    if candidateDist > distanceToNextMatch {
                        distanceToNextMatch = candidateDist;
                        matchChainPos = pos as u32;
                        accel = 1 << kTrigger;
                    }
                    pos += step;
                }
                if distanceToNextMatch > 1 {
                    if distanceToNextMatch > matchIndex {
                        break 'searchloop;
                    }
                    matchIndex -= distanceToNextMatch;
                    continue 'searchloop;
                }
            }
        }

        {
            let distNextMatch = DELTANEXTU16(chainTable, matchIndex) as u32;
            if (patternAnalysis != 0) && distNextMatch == 1 && matchChainPos == 0 {
                let matchCandidateIdx = matchIndex - 1;
                if repeat == rep_untested {
                    if ((pattern & 0xFFFF) == (pattern >> 16))
                        && ((pattern & 0xFF) == (pattern >> 24))
                    {
                        repeat = rep_confirmed;
                        srcPatternLength = LZ4HC_countPattern(ip.add(4), iHighLimit, pattern) as usize + 4;
                    } else {
                        repeat = rep_not;
                    }
                }
                if (repeat == rep_confirmed)
                    && (matchCandidateIdx >= lowestMatchIndex)
                    && LZ4HC_protectDictEnd(prefixIdx, matchCandidateIdx)
                {
                    let extDict = matchCandidateIdx < prefixIdx;
                    let matchPtr = if extDict {
                        dictStart.add((matchCandidateIdx - dictIdx) as usize)
                    } else {
                        prefixPtr.add((matchCandidateIdx - prefixIdx) as usize)
                    };
                    if LZ4_read32(matchPtr) == pattern {
                        let iLimit = if extDict { dictEnd } else { iHighLimit };
                        let mut forwardPatternLength =
                            LZ4HC_countPattern(matchPtr.add(4), iLimit, pattern) as usize + 4;
                        if extDict && matchPtr.wrapping_add(forwardPatternLength) == iLimit {
                            let rotatedPattern = LZ4HC_rotatePattern(forwardPatternLength, pattern);
                            forwardPatternLength +=
                                LZ4HC_countPattern(prefixPtr, iHighLimit, rotatedPattern) as usize;
                        }
                        {
                            let lowestMatchPtr = if extDict { dictStart } else { prefixPtr };
                            let mut backLength =
                                LZ4HC_reverseCountPattern(matchPtr, lowestMatchPtr, pattern) as usize;
                            let currentSegmentLength: usize;
                            if !extDict
                                && matchPtr.wrapping_sub(backLength) == prefixPtr
                                && dictIdx < prefixIdx
                            {
                                let rotatedPattern = LZ4HC_rotatePattern(
                                    (0u32.wrapping_sub(backLength as u32)) as usize,
                                    pattern,
                                );
                                backLength +=
                                    LZ4HC_reverseCountPattern(dictEnd, dictStart, rotatedPattern) as usize;
                            }
                            backLength = matchCandidateIdx
                                .wrapping_sub(core::cmp::max(
                                    matchCandidateIdx.wrapping_sub(backLength as u32),
                                    lowestMatchIndex,
                                )) as usize;
                            currentSegmentLength = backLength + forwardPatternLength;

                            if (currentSegmentLength >= srcPatternLength)
                                && (forwardPatternLength <= srcPatternLength)
                            {
                                let newMatchIndex = matchCandidateIdx + forwardPatternLength as u32
                                    - srcPatternLength as u32;
                                if LZ4HC_protectDictEnd(prefixIdx, newMatchIndex) {
                                    matchIndex = newMatchIndex;
                                } else {
                                    matchIndex = prefixIdx;
                                }
                            } else {
                                let newMatchIndex = matchCandidateIdx - backLength as u32;
                                if !LZ4HC_protectDictEnd(prefixIdx, newMatchIndex) {
                                    matchIndex = prefixIdx;
                                } else {
                                    matchIndex = newMatchIndex;
                                    if lookBackLength == 0 {
                                        let maxML = MIN(currentSegmentLength, srcPatternLength);
                                        if (longest as usize) < maxML {
                                            if (ip as usize - prefixPtr as usize)
                                                + (prefixIdx as usize)
                                                - (matchIndex as usize)
                                                > LZ4_DISTANCE_MAX_US
                                            {
                                                break 'searchloop;
                                            }
                                            longest = maxML as c_int;
                                            offset = (ipIndex - matchIndex) as c_int;
                                        }
                                        {
                                            let distToNextPattern =
                                                DELTANEXTU16(chainTable, matchIndex) as u32;
                                            if distToNextPattern > matchIndex {
                                                break 'searchloop;
                                            }
                                            matchIndex -= distToNextPattern;
                                        }
                                    }
                                }
                            }
                        }
                        continue 'searchloop;
                    }
                }
            }
        }

        matchIndex -= DELTANEXTU16(chainTable, matchIndex + matchChainPos) as u32;
    }

    if dict == usingDictCtxHc && nbAttempts > 0 && withinStartDistance {
        let dc = &*dictCtx;
        let dictEndOffset = (dc.end as usize - dc.prefixStart as usize) + dc.dictLimit as usize;
        let mut dictMatchIndex = dc.hashTable[LZ4HC_hashPtr(ip) as usize];
        matchIndex = dictMatchIndex
            .wrapping_add(lowestMatchIndex)
            .wrapping_sub(dictEndOffset as u32);
        while ipIndex.wrapping_sub(matchIndex) <= LZ4_DISTANCE_MAX && {
            let cont = nbAttempts != 0;
            nbAttempts -= 1;
            cont
        } {
            let matchPtr = dc
                .prefixStart
                .wrapping_sub(dc.dictLimit as usize)
                .wrapping_add(dictMatchIndex as usize);
            if LZ4_read32(matchPtr) == pattern {
                let mut vLimit = ip.wrapping_add(dictEndOffset - dictMatchIndex as usize);
                if vLimit > iHighLimit {
                    vLimit = iHighLimit;
                }
                let mut mlt = LZ4_count(ip.add(MINMATCH), matchPtr.add(MINMATCH), vLimit) as c_int
                    + MINMATCH as c_int;
                let back = if lookBackLength != 0 {
                    LZ4HC_countBack(ip, matchPtr, iLowLimit, dc.prefixStart)
                } else {
                    0
                };
                mlt -= back;
                if mlt > longest {
                    longest = mlt;
                    offset = (ipIndex - matchIndex) as c_int;
                    sBack = back;
                }
            }
            let nextOffset = DELTANEXTU16(dc.chainTable.as_ptr(), dictMatchIndex) as u32;
            /* U32 wrap-around, exactly as in the C: the `ipIndex - matchIndex`
             * loop guard is what rejects the underflowed value. */
            dictMatchIndex = dictMatchIndex.wrapping_sub(nextOffset);
            matchIndex = matchIndex.wrapping_sub(nextOffset);
        }
    }

    LZ4HC_match_t {
        len: longest,
        off: offset,
        back: sBack,
    }
}

#[inline]
unsafe fn LZ4HC_InsertAndFindBestMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iLimit: *const u8,
    maxNbAttempts: c_int,
    patternAnalysis: c_int,
    dict: dictCtx_directive,
) -> LZ4HC_match_t {
    LZ4HC_InsertAndGetWiderMatch(
        hc4, ip, ip, iLimit, MINMATCH as c_int - 1, maxNbAttempts, patternAnalysis, 0, dict,
        favorCompressionRatio,
    )
}

unsafe fn LZ4HC_compress_hashChain(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dest: *mut c_char,
    srcSizePtr: *mut c_int,
    maxOutputSize: c_int,
    maxNbAttempts: c_int,
    limit: limitedOutput_directive,
    dict: dictCtx_directive,
) -> c_int {
    let inputSize = *srcSizePtr;
    let patternAnalysis = (maxNbAttempts > 128) as c_int;

    let mut ip = source as *const u8;
    let mut anchor = ip;
    let iend = ip.add(inputSize as usize);
    let mflimit = iend.wrapping_sub(MFLIMIT);
    let matchlimit = iend.wrapping_sub(LASTLITERALS);

    let mut optr;
    let mut op = dest as *mut u8;
    let mut oend = op.wrapping_add(maxOutputSize as usize);

    let mut start0: *const u8;
    let mut start2: *const u8 = ptr::null();
    let mut start3: *const u8 = ptr::null();
    let mut m0: LZ4HC_match_t;
    let mut m1: LZ4HC_match_t;
    let mut m2: LZ4HC_match_t;
    let mut m3: LZ4HC_match_t;
    let nomatch = LZ4HC_match_t { off: 0, len: 0, back: 0 };

    *srcSizePtr = 0;
    if limit == fillOutput {
        oend = oend.wrapping_sub(LASTLITERALS);
    }

    // dest_overflow handler (fillOutput): fixup last sequence then last_literals.
    macro_rules! dest_overflow {
        ($mm:expr) => {{
            if limit == fillOutput {
                let mut mm: LZ4HC_match_t = $mm;
                let ll = (ip as usize - anchor as usize) as usize;
                let ll_addbytes = (ll + 240) / 255;
                let ll_totalCost = 1 + ll_addbytes + ll;
                let maxLitPos = oend.wrapping_sub(3);
                op = optr;
                if op.wrapping_add(ll_totalCost) <= maxLitPos {
                    let bytesLeftForMl = (maxLitPos as usize) - ((op as usize) + ll_totalCost);
                    let maxMlSize = MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                    if (mm.len as usize) > maxMlSize {
                        mm.len = maxMlSize as c_int;
                    }
                    if ((oend as usize + LASTLITERALS) as isize
                        - ((op as usize) + ll_totalCost + 2) as isize
                        - 1
                        + mm.len as isize)
                        >= MFLIMIT as isize
                    {
                        LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, mm.len, mm.off, notLimited, oend);
                    }
                }
                return hc_last_literals(source, dest, srcSizePtr, anchor, op, iend, limit, oend);
            }
            return 0;
        }};
    }

    if inputSize >= LZ4_minLength {
        'mainloop: while ip <= mflimit {
            m1 = LZ4HC_InsertAndFindBestMatch(ctx, ip, matchlimit, maxNbAttempts, patternAnalysis, dict);
            if m1.len < MINMATCH as c_int {
                ip = ip.add(1);
                continue 'mainloop;
            }

            start0 = ip;
            m0 = m1;

            'search2: loop {
                if ip.wrapping_add(m1.len as usize) <= mflimit {
                    start2 = ip.wrapping_add(m1.len as usize).wrapping_sub(2);
                    m2 = LZ4HC_InsertAndGetWiderMatch(
                        ctx, start2, ip.wrapping_add(0), matchlimit, m1.len, maxNbAttempts,
                        patternAnalysis, 0, dict, favorCompressionRatio,
                    );
                    start2 = start2.offset(m2.back as isize);
                } else {
                    m2 = nomatch;
                }

                if m2.len <= m1.len {
                    optr = op;
                    if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend) != 0 {
                        dest_overflow!(m1);
                    }
                    continue 'mainloop;
                }

                if start0 < ip {
                    if start2 < ip.wrapping_add(m0.len as usize) {
                        ip = start0;
                        m1 = m0;
                    }
                }

                if (start2 as usize - ip as usize) < 3 {
                    ip = start2;
                    m1 = m2;
                    continue 'search2;
                }

                'search3: loop {
                    if (start2 as usize - ip as usize) < OPTIMAL_ML as usize {
                        let mut new_ml = m1.len;
                        if new_ml > OPTIMAL_ML {
                            new_ml = OPTIMAL_ML;
                        }
                        if ip.wrapping_add(new_ml as usize)
                            > start2.wrapping_add(m2.len as usize).wrapping_sub(MINMATCH)
                        {
                            new_ml = (start2 as usize - ip as usize) as c_int + m2.len - MINMATCH as c_int;
                        }
                        let correction = new_ml - (start2 as usize - ip as usize) as c_int;
                        if correction > 0 {
                            start2 = start2.offset(correction as isize);
                            m2.len -= correction;
                        }
                    }

                    if start2.wrapping_add(m2.len as usize) <= mflimit {
                        start3 = start2.wrapping_add(m2.len as usize).wrapping_sub(3);
                        m3 = LZ4HC_InsertAndGetWiderMatch(
                            ctx, start3, start2, matchlimit, m2.len, maxNbAttempts,
                            patternAnalysis, 0, dict, favorCompressionRatio,
                        );
                        start3 = start3.offset(m3.back as isize);
                    } else {
                        m3 = nomatch;
                    }

                    if m3.len <= m2.len {
                        if start2 < ip.wrapping_add(m1.len as usize) {
                            m1.len = (start2 as usize - ip as usize) as c_int;
                        }
                        optr = op;
                        if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend) != 0 {
                            dest_overflow!(m1);
                        }
                        ip = start2;
                        optr = op;
                        if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, m2.len, m2.off, limit, oend) != 0 {
                            dest_overflow!(m2);
                        }
                        continue 'mainloop;
                    }

                    if start3 < ip.wrapping_add(m1.len as usize + 3) {
                        if start3 >= ip.wrapping_add(m1.len as usize) {
                            if start2 < ip.wrapping_add(m1.len as usize) {
                                let correction =
                                    (ip.wrapping_add(m1.len as usize) as usize - start2 as usize) as c_int;
                                start2 = start2.offset(correction as isize);
                                m2.len -= correction;
                                if m2.len < MINMATCH as c_int {
                                    start2 = start3;
                                    m2 = m3;
                                }
                            }

                            optr = op;
                            if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend) != 0 {
                                dest_overflow!(m1);
                            }
                            ip = start3;
                            m1 = m3;

                            start0 = start2;
                            m0 = m2;
                            continue 'search2;
                        }

                        start2 = start3;
                        m2 = m3;
                        continue 'search3;
                    }

                    if start2 < ip.wrapping_add(m1.len as usize) {
                        if (start2 as usize - ip as usize) < OPTIMAL_ML as usize {
                            if m1.len > OPTIMAL_ML {
                                m1.len = OPTIMAL_ML;
                            }
                            if ip.wrapping_add(m1.len as usize)
                                > start2.wrapping_add(m2.len as usize).wrapping_sub(MINMATCH)
                            {
                                m1.len = (start2 as usize - ip as usize) as c_int + m2.len - MINMATCH as c_int;
                            }
                            let correction = m1.len - (start2 as usize - ip as usize) as c_int;
                            if correction > 0 {
                                start2 = start2.offset(correction as isize);
                                m2.len -= correction;
                            }
                        } else {
                            m1.len = (start2 as usize - ip as usize) as c_int;
                        }
                    }
                    optr = op;
                    if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend) != 0 {
                        dest_overflow!(m1);
                    }

                    ip = start2;
                    m1 = m2;
                    start2 = start3;
                    m2 = m3;
                    continue 'search3;
                }
            }
        }
    }

    hc_last_literals(source, dest, srcSizePtr, anchor, op, iend, limit, oend)
}

/* ===== optimal parser ===== */
#[derive(Clone, Copy)]
struct LZ4HC_optimal_t {
    price: c_int,
    off: c_int,
    mlen: c_int,
    litlen: c_int,
}

#[inline]
fn LZ4HC_literalsPrice(litlen: c_int) -> c_int {
    let mut price = litlen;
    if litlen >= RUN_MASK as c_int {
        price += 1 + ((litlen - RUN_MASK as c_int) / 255);
    }
    price
}

#[inline]
fn LZ4HC_sequencePrice(litlen: c_int, mlen: c_int) -> c_int {
    let mut price = 1 + 2;
    price += LZ4HC_literalsPrice(litlen);
    if mlen >= (ML_MASK as c_int + MINMATCH as c_int) {
        price += 1 + ((mlen - (ML_MASK as c_int + MINMATCH as c_int)) / 255);
    }
    price
}

unsafe fn LZ4HC_FindLongerMatch(
    ctx: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iHighLimit: *const u8,
    minLen: c_int,
    nbSearches: c_int,
    dict: dictCtx_directive,
    favorDecSpeed: HCfavor_e,
) -> LZ4HC_match_t {
    let match0 = LZ4HC_match_t { off: 0, len: 0, back: 0 };
    let mut md = LZ4HC_InsertAndGetWiderMatch(
        ctx, ip, ip, iHighLimit, minLen, nbSearches, 1, 1, dict, favorDecSpeed,
    );
    if md.len <= minLen {
        return match0;
    }
    if favorDecSpeed == favorDecompressionSpeed {
        if (md.len > 18) && (md.len <= 36) {
            md.len = 18;
        }
    }
    md
}

const TRAILING_LITERALS: usize = 3;

unsafe fn LZ4HC_compress_optimal(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    nbSearches: c_int,
    mut sufficient_len: usize,
    limit: limitedOutput_directive,
    fullUpdate: c_int,
    dict: dictCtx_directive,
    favorDecSpeed: HCfavor_e,
) -> c_int {
    let mut retval: c_int = 0;
    let opt = crate::c_malloc(core::mem::size_of::<LZ4HC_optimal_t>() * (LZ4_OPT_NUM + TRAILING_LITERALS))
        as *mut LZ4HC_optimal_t;

    let mut ip = source as *const u8;
    let mut anchor = ip;
    let iend = ip.add(*srcSizePtr as usize);
    let mflimit = iend.wrapping_sub(MFLIMIT);
    let matchlimit = iend.wrapping_sub(LASTLITERALS);
    let mut op = dst as *mut u8;
    let mut opSaved = dst as *mut u8;
    let mut oend = op.wrapping_add(dstCapacity as usize);
    let mut ovml: c_int = MINMATCH as c_int;
    let mut ovoff: c_int = 0;

    if opt.is_null() {
        return retval;
    }

    *srcSizePtr = 0;
    if limit == fillOutput {
        oend = oend.wrapping_sub(LASTLITERALS);
    }
    if sufficient_len >= LZ4_OPT_NUM {
        sufficient_len = LZ4_OPT_NUM - 1;
    }

    let at = |i: c_int| -> *mut LZ4HC_optimal_t { opt.add(i as usize) };

    // _dest_overflow / _last_literals handled inline via macro producing a return
    macro_rules! do_dest_overflow {
        () => {{
            if limit == fillOutput {
                let ll = (ip as usize - anchor as usize) as usize;
                let ll_addbytes = (ll + 240) / 255;
                let ll_totalCost = 1 + ll_addbytes + ll;
                let maxLitPos = oend.wrapping_sub(3);
                op = opSaved;
                if op.wrapping_add(ll_totalCost) <= maxLitPos {
                    let bytesLeftForMl = (maxLitPos as usize) - ((op as usize) + ll_totalCost);
                    let maxMlSize = MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                    if (ovml as usize) > maxMlSize {
                        ovml = maxMlSize as c_int;
                    }
                    if ((oend as usize + LASTLITERALS) as isize
                        - ((op as usize) + ll_totalCost + 2) as isize
                        - 1
                        + ovml as isize)
                        >= MFLIMIT as isize
                    {
                        LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, ovml, ovoff, notLimited, oend);
                    }
                }
                /* NOTE: `oend` must NOT be restored here. The C falls through
                 * to `_last_literals`, which is the single place that performs
                 * `if (limit == fillOutput) oend += LASTLITERALS;` — and
                 * `hc_last_literals` already does exactly that. Restoring it
                 * twice inflates the salvaged final literal run by
                 * LASTLITERALS bytes. */
                retval = hc_last_literals(source, dst, srcSizePtr, anchor, op, iend, limit, oend);
                crate::c_free(opt as *mut u8);
                return retval;
            } else {
                crate::c_free(opt as *mut u8);
                return 0;
            }
        }};
    }

    macro_rules! do_last_literals {
        () => {{
            retval = hc_last_literals(source, dst, srcSizePtr, anchor, op, iend, limit, oend);
            crate::c_free(opt as *mut u8);
            return retval;
        }};
    }

    // encode section (reverse traversal + emit). On overflow -> do_dest_overflow.
    macro_rules! run_encode {
        ($cur:expr, $last_match_pos:expr, $best_mlen:expr, $best_off:expr) => {{
            let mut candidate_pos = $cur;
            let mut selected_matchLength = $best_mlen;
            let mut selected_offset = $best_off;
            loop {
                let next_matchLength = (*at(candidate_pos)).mlen;
                let next_offset = (*at(candidate_pos)).off;
                (*at(candidate_pos)).mlen = selected_matchLength;
                (*at(candidate_pos)).off = selected_offset;
                selected_matchLength = next_matchLength;
                selected_offset = next_offset;
                if next_matchLength > candidate_pos {
                    break;
                }
                candidate_pos -= next_matchLength;
            }
            {
                let mut rPos = 0;
                while rPos < $last_match_pos {
                    let ml = (*at(rPos)).mlen;
                    let offset = (*at(rPos)).off;
                    if ml == 1 {
                        ip = ip.add(1);
                        rPos += 1;
                        continue;
                    }
                    rPos += ml;
                    opSaved = op;
                    if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, ml, offset, limit, oend) != 0 {
                        ovml = ml;
                        ovoff = offset;
                        do_dest_overflow!();
                    }
                }
            }
        }};
    }

    'mainloop: while ip <= mflimit {
        let llen = (ip as usize - anchor as usize) as c_int;
        let mut best_mlen: c_int;
        let mut best_off: c_int;
        let mut cur: c_int;
        let mut last_match_pos: c_int;

        let firstMatch =
            LZ4HC_FindLongerMatch(ctx, ip, matchlimit, MINMATCH as c_int - 1, nbSearches, dict, favorDecSpeed);
        if firstMatch.len == 0 {
            ip = ip.add(1);
            continue 'mainloop;
        }

        if (firstMatch.len as usize) > sufficient_len {
            let firstML = firstMatch.len;
            opSaved = op;
            if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, firstML, firstMatch.off, limit, oend) != 0 {
                ovml = firstML;
                ovoff = firstMatch.off;
                do_dest_overflow!();
            }
            continue 'mainloop;
        }

        // set prices for first positions (literals)
        {
            let mut rPos = 0;
            while rPos < MINMATCH as c_int {
                let cost = LZ4HC_literalsPrice(llen + rPos);
                let o = &mut *at(rPos);
                o.mlen = 1;
                o.off = 0;
                o.litlen = llen + rPos;
                o.price = cost;
                rPos += 1;
            }
        }
        {
            let matchML = firstMatch.len;
            let offset = firstMatch.off;
            let mut mlen = MINMATCH as c_int;
            while mlen <= matchML {
                let cost = LZ4HC_sequencePrice(llen, mlen);
                let o = &mut *at(mlen);
                o.mlen = mlen;
                o.off = offset;
                o.litlen = llen;
                o.price = cost;
                mlen += 1;
            }
        }
        last_match_pos = firstMatch.len;
        {
            let mut addLit = 1;
            while addLit <= TRAILING_LITERALS as c_int {
                let base_price = (*at(last_match_pos)).price;
                let o = &mut *at(last_match_pos + addLit);
                o.mlen = 1;
                o.off = 0;
                o.litlen = addLit;
                o.price = base_price + LZ4HC_literalsPrice(addLit);
                addLit += 1;
            }
        }

        // check further positions; use labeled block to model "goto encode"
        let mut immediate = false;
        best_mlen = 0;
        best_off = 0;
        cur = 1;
        'curloop: while cur < last_match_pos {
            let curPtr = ip.wrapping_add(cur as usize);
            let newMatch: LZ4HC_match_t;

            if curPtr > mflimit {
                break 'curloop;
            }
            if fullUpdate != 0 {
                if ((*at(cur + 1)).price <= (*at(cur)).price)
                    && ((*at(cur + MINMATCH as c_int)).price < (*at(cur)).price + 3)
                {
                    cur += 1;
                    continue 'curloop;
                }
            } else {
                if (*at(cur + 1)).price <= (*at(cur)).price {
                    cur += 1;
                    continue 'curloop;
                }
            }

            if fullUpdate != 0 {
                newMatch = LZ4HC_FindLongerMatch(ctx, curPtr, matchlimit, MINMATCH as c_int - 1, nbSearches, dict, favorDecSpeed);
            } else {
                newMatch = LZ4HC_FindLongerMatch(ctx, curPtr, matchlimit, last_match_pos - cur, nbSearches, dict, favorDecSpeed);
            }
            if newMatch.len == 0 {
                cur += 1;
                continue 'curloop;
            }

            if ((newMatch.len as usize) > sufficient_len)
                || (newMatch.len + cur >= LZ4_OPT_NUM as c_int)
            {
                best_mlen = newMatch.len;
                best_off = newMatch.off;
                last_match_pos = cur + 1;
                immediate = true;
                break 'curloop; // goto encode
            }

            {
                let baseLitlen = (*at(cur)).litlen;
                let mut litlen = 1;
                while litlen < MINMATCH as c_int {
                    let price = (*at(cur)).price - LZ4HC_literalsPrice(baseLitlen)
                        + LZ4HC_literalsPrice(baseLitlen + litlen);
                    let pos = cur + litlen;
                    if price < (*at(pos)).price {
                        let o = &mut *at(pos);
                        o.mlen = 1;
                        o.off = 0;
                        o.litlen = baseLitlen + litlen;
                        o.price = price;
                    }
                    litlen += 1;
                }
            }

            {
                let matchML = newMatch.len;
                let mut ml = MINMATCH as c_int;
                while ml <= matchML {
                    let pos = cur + ml;
                    let offset = newMatch.off;
                    let price;
                    let ll;
                    if (*at(cur)).mlen == 1 {
                        ll = (*at(cur)).litlen;
                        price = (if cur > ll { (*at(cur - ll)).price } else { 0 })
                            + LZ4HC_sequencePrice(ll, ml);
                    } else {
                        ll = 0;
                        price = (*at(cur)).price + LZ4HC_sequencePrice(0, ml);
                    }

                    if pos > last_match_pos + TRAILING_LITERALS as c_int
                        || price <= (*at(pos)).price - (favorDecSpeed as c_int)
                    {
                        if (ml == matchML) && (last_match_pos < pos) {
                            last_match_pos = pos;
                        }
                        let o = &mut *at(pos);
                        o.mlen = ml;
                        o.off = offset;
                        o.litlen = ll;
                        o.price = price;
                    }
                    ml += 1;
                }
            }
            {
                let mut addLit = 1;
                while addLit <= TRAILING_LITERALS as c_int {
                    let base_price = (*at(last_match_pos)).price;
                    let o = &mut *at(last_match_pos + addLit);
                    o.mlen = 1;
                    o.off = 0;
                    o.litlen = addLit;
                    o.price = base_price + LZ4HC_literalsPrice(addLit);
                    addLit += 1;
                }
            }
            cur += 1;
        }

        if !immediate {
            best_mlen = (*at(last_match_pos)).mlen;
            best_off = (*at(last_match_pos)).off;
            cur = last_match_pos - best_mlen;
        }

        // encode:
        run_encode!(cur, last_match_pos, best_mlen, best_off);
        // continue main loop
    }

    do_last_literals!();
}

/* ===== generic dispatch ===== */
unsafe fn LZ4HC_compress_generic_internal(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: limitedOutput_directive,
    dict: dictCtx_directive,
) -> c_int {
    if limit == fillOutput && dstCapacity < 1 {
        return 0;
    }
    if (*srcSizePtr as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        return 0;
    }

    (*ctx).end = (*ctx).end.wrapping_add(*srcSizePtr as usize);
    let cParam = LZ4HC_getCLevelParams(cLevel);
    let favor = if (*ctx).favorDecSpeed != 0 {
        favorDecompressionSpeed
    } else {
        favorCompressionRatio
    };
    let result;

    if cParam.strat == lz4mid {
        result = LZ4MID_compress(ctx, src, dst, srcSizePtr, dstCapacity, limit, dict);
    } else if cParam.strat == lz4hc {
        result = LZ4HC_compress_hashChain(ctx, src, dst, srcSizePtr, dstCapacity, cParam.nbSearches, limit, dict);
    } else {
        result = LZ4HC_compress_optimal(
            ctx, src, dst, srcSizePtr, dstCapacity, cParam.nbSearches, cParam.targetLength as usize,
            limit, (cLevel >= LZ4HC_CLEVEL_MAX) as c_int, dict, favor,
        );
    }
    if result <= 0 {
        (*ctx).dirty = 1;
    }
    result
}

unsafe fn LZ4HC_compress_generic_noDictCtx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: limitedOutput_directive,
) -> c_int {
    LZ4HC_compress_generic_internal(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit, noDictCtx)
}

unsafe fn isStateCompatible(ctx1: *const LZ4HC_CCtx_internal, ctx2: *const LZ4HC_CCtx_internal) -> c_int {
    let isMid1 = LZ4HC_getCLevelParams((*ctx1).compressionLevel as c_int).strat == lz4mid;
    let isMid2 = LZ4HC_getCLevelParams((*ctx2).compressionLevel as c_int).strat == lz4mid;
    (!(isMid1 ^ isMid2)) as c_int
}

unsafe fn LZ4HC_compress_generic_dictCtx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: limitedOutput_directive,
) -> c_int {
    let position = (((*ctx).end as usize) - ((*ctx).prefixStart as usize))
        + (((*ctx).dictLimit - (*ctx).lowLimit) as usize);
    if position >= 64 * 1024 {
        (*ctx).dictCtx = ptr::null();
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
    } else if position == 0 && *srcSizePtr > 4 * 1024 && isStateCompatible(ctx, (*ctx).dictCtx) != 0 {
        LZ4_memcpy(ctx as *mut u8, (*ctx).dictCtx as *const u8, core::mem::size_of::<LZ4HC_CCtx_internal>());
        LZ4HC_setExternalDict(ctx, src as *const u8);
        (*ctx).compressionLevel = cLevel as i16;
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
    } else {
        LZ4HC_compress_generic_internal(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit, usingDictCtxHc)
    }
}

unsafe fn LZ4HC_compress_generic(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: limitedOutput_directive,
) -> c_int {
    if (*ctx).dictCtx.is_null() {
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
    } else {
        LZ4HC_compress_generic_dictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
    }
}

/* ===== public API ===== */
#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

fn LZ4_streamHC_t_alignment() -> usize {
    core::mem::align_of::<LZ4_streamHC_t>()
}

#[inline]
fn crate_isAligned(ptr: *const u8, alignment: usize) -> bool {
    (ptr as usize) & (alignment - 1) == 0
}

#[inline]
unsafe fn hc_internal(s: *mut LZ4_streamHC_t) -> *mut LZ4HC_CCtx_internal {
    s as *mut LZ4HC_CCtx_internal
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC_fastReset(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut srcSize: c_int,
    dstCapacity: c_int,
    compressionLevel: c_int,
) -> c_int {
    let ctx = hc_internal(state as *mut LZ4_streamHC_t);
    if !crate_isAligned(state as *const u8, LZ4_streamHC_t_alignment()) {
        return 0;
    }
    LZ4_resetStreamHC_fast(state as *mut LZ4_streamHC_t, compressionLevel);
    LZ4HC_init_internal(ctx, src as *const u8);
    if dstCapacity < LZ4_compressBound(srcSize) {
        LZ4HC_compress_generic(ctx, src, dst, &mut srcSize, dstCapacity, compressionLevel, limitedOutput)
    } else {
        LZ4HC_compress_generic(ctx, src, dst, &mut srcSize, dstCapacity, compressionLevel, notLimited)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    compressionLevel: c_int,
) -> c_int {
    let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
    if ctx.is_null() {
        return 0;
    }
    LZ4_compress_HC_extStateHC_fastReset(state, src, dst, srcSize, dstCapacity, compressionLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    compressionLevel: c_int,
) -> c_int {
    let statePtr = crate::c_malloc(core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
    if statePtr.is_null() {
        return 0;
    }
    let cSize = LZ4_compress_HC_extStateHC(statePtr as *mut c_void, src, dst, srcSize, dstCapacity, compressionLevel);
    crate::c_free(statePtr as *mut u8);
    cSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_destSize(
    state: *mut c_void,
    source: *const c_char,
    dest: *mut c_char,
    sourceSizePtr: *mut c_int,
    targetDestSize: c_int,
    cLevel: c_int,
) -> c_int {
    let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
    if ctx.is_null() {
        return 0;
    }
    LZ4HC_init_internal(hc_internal(ctx), source as *const u8);
    LZ4_setCompressionLevel(ctx, cLevel);
    LZ4HC_compress_generic(hc_internal(ctx), source, dest, sourceSizePtr, targetDestSize, cLevel, fillOutput)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamHC() -> *mut LZ4_streamHC_t {
    let state = crate::c_calloc(core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
    if state.is_null() {
        return ptr::null_mut();
    }
    LZ4_setCompressionLevel(state, LZ4HC_CLEVEL_DEFAULT);
    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamHC(LZ4_streamHCPtr: *mut LZ4_streamHC_t) -> c_int {
    if LZ4_streamHCPtr.is_null() {
        return 0;
    }
    crate::c_free(LZ4_streamHCPtr as *mut u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStreamHC(buffer: *mut c_void, size: usize) -> *mut LZ4_streamHC_t {
    let LZ4_streamHCPtr = buffer as *mut LZ4_streamHC_t;
    if buffer.is_null() {
        return ptr::null_mut();
    }
    if size < core::mem::size_of::<LZ4_streamHC_t>() {
        return ptr::null_mut();
    }
    if !crate_isAligned(buffer as *const u8, LZ4_streamHC_t_alignment()) {
        return ptr::null_mut();
    }
    let hcstate = hc_internal(LZ4_streamHCPtr);
    ptr::write_bytes(hcstate as *mut u8, 0, core::mem::size_of::<LZ4HC_CCtx_internal>());
    LZ4_setCompressionLevel(LZ4_streamHCPtr, LZ4HC_CLEVEL_DEFAULT);
    LZ4_streamHCPtr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC(LZ4_streamHCPtr: *mut LZ4_streamHC_t, compressionLevel: c_int) {
    LZ4_initStreamHC(LZ4_streamHCPtr as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
    LZ4_setCompressionLevel(LZ4_streamHCPtr, compressionLevel);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC_fast(LZ4_streamHCPtr: *mut LZ4_streamHC_t, compressionLevel: c_int) {
    let s = hc_internal(LZ4_streamHCPtr);
    if (*s).dirty != 0 {
        LZ4_initStreamHC(LZ4_streamHCPtr as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
    } else {
        (*s).dictLimit += ((*s).end as usize - (*s).prefixStart as usize) as u32;
        (*s).prefixStart = ptr::null();
        (*s).end = ptr::null();
        (*s).dictCtx = ptr::null();
    }
    LZ4_setCompressionLevel(LZ4_streamHCPtr, compressionLevel);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setCompressionLevel(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    mut compressionLevel: c_int,
) {
    if compressionLevel < 1 {
        compressionLevel = LZ4HC_CLEVEL_DEFAULT;
    }
    if compressionLevel > LZ4HC_CLEVEL_MAX {
        compressionLevel = LZ4HC_CLEVEL_MAX;
    }
    (*hc_internal(LZ4_streamHCPtr)).compressionLevel = compressionLevel as i16;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_favorDecompressionSpeed(LZ4_streamHCPtr: *mut LZ4_streamHC_t, favor: c_int) {
    (*hc_internal(LZ4_streamHCPtr)).favorDecSpeed = (favor != 0) as i8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    mut dictionary: *const c_char,
    mut dictSize: c_int,
) -> c_int {
    let ctxPtr = hc_internal(LZ4_streamHCPtr);
    let cp;
    if dictSize > 64 * 1024 {
        dictionary = dictionary.add((dictSize as usize) - 64 * 1024);
        dictSize = 64 * 1024;
    }
    {
        let cLevel = (*ctxPtr).compressionLevel as c_int;
        LZ4_initStreamHC(LZ4_streamHCPtr as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
        LZ4_setCompressionLevel(LZ4_streamHCPtr, cLevel);
        cp = LZ4HC_getCLevelParams(cLevel);
    }
    LZ4HC_init_internal(ctxPtr, dictionary as *const u8);
    (*ctxPtr).end = (dictionary as *const u8).add(dictSize as usize);
    if cp.strat == lz4mid {
        LZ4MID_fillHTable(ctxPtr, dictionary as *const c_void, dictSize as usize);
    } else {
        if dictSize >= LZ4HC_HASHSIZE as c_int {
            LZ4HC_Insert(ctxPtr, (*ctxPtr).end.offset(-3));
        }
    }
    dictSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_HC_dictionary(
    working_stream: *mut LZ4_streamHC_t,
    dictionary_stream: *const LZ4_streamHC_t,
) {
    (*hc_internal(working_stream)).dictCtx = if !dictionary_stream.is_null() {
        dictionary_stream as *const LZ4HC_CCtx_internal
    } else {
        ptr::null()
    };
}

unsafe fn LZ4HC_setExternalDict(ctxPtr: *mut LZ4HC_CCtx_internal, newBlock: *const u8) {
    let c = &mut *ctxPtr;
    if (c.end >= c.prefixStart.wrapping_add(4))
        && (LZ4HC_getCLevelParams(c.compressionLevel as c_int).strat != lz4mid)
    {
        LZ4HC_Insert(ctxPtr, c.end.offset(-3));
    }

    c.lowLimit = c.dictLimit;
    c.dictStart = c.prefixStart;
    c.dictLimit += (c.end as usize - c.prefixStart as usize) as u32;
    c.prefixStart = newBlock;
    c.end = newBlock;
    c.nextToUpdate = c.dictLimit;
    c.dictCtx = ptr::null();
}

unsafe fn LZ4_compressHC_continue_generic(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    limit: limitedOutput_directive,
) -> c_int {
    let ctxPtr = hc_internal(LZ4_streamHCPtr);
    if (*ctxPtr).prefixStart.is_null() {
        LZ4HC_init_internal(ctxPtr, src as *const u8);
    }

    if ((*ctxPtr).end as usize - (*ctxPtr).prefixStart as usize) + (*ctxPtr).dictLimit as usize
        > (2usize << 30)
    {
        let mut dictSize = ((*ctxPtr).end as usize - (*ctxPtr).prefixStart as usize) as usize;
        if dictSize > 64 * 1024 {
            dictSize = 64 * 1024;
        }
        LZ4_loadDictHC(
            LZ4_streamHCPtr,
            ((*ctxPtr).end as *const c_char).wrapping_sub(dictSize),
            dictSize as c_int,
        );
    }

    if src as *const u8 != (*ctxPtr).end {
        LZ4HC_setExternalDict(ctxPtr, src as *const u8);
    }

    {
        let mut sourceEnd = (src as *const u8).wrapping_add(*srcSizePtr as usize);
        let dictBegin = (*ctxPtr).dictStart;
        let dictEnd = (*ctxPtr)
            .dictStart
            .wrapping_add(((*ctxPtr).dictLimit - (*ctxPtr).lowLimit) as usize);
        if (sourceEnd > dictBegin) && ((src as *const u8) < dictEnd) {
            if sourceEnd > dictEnd {
                sourceEnd = dictEnd;
            }
            (*ctxPtr).lowLimit += (sourceEnd as usize - (*ctxPtr).dictStart as usize) as u32;
            (*ctxPtr).dictStart = (*ctxPtr)
                .dictStart
                .wrapping_add(sourceEnd as usize - (*ctxPtr).dictStart as usize);
            if (*ctxPtr).dictLimit - (*ctxPtr).lowLimit < LZ4HC_HASHSIZE as u32 {
                (*ctxPtr).lowLimit = (*ctxPtr).dictLimit;
                (*ctxPtr).dictStart = (*ctxPtr).prefixStart;
            }
        }
    }

    LZ4HC_compress_generic(ctxPtr, src, dst, srcSizePtr, dstCapacity, (*ctxPtr).compressionLevel as c_int, limit)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    mut srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    if dstCapacity < LZ4_compressBound(srcSize) {
        LZ4_compressHC_continue_generic(LZ4_streamHCPtr, src, dst, &mut srcSize, dstCapacity, limitedOutput)
    } else {
        LZ4_compressHC_continue_generic(LZ4_streamHCPtr, src, dst, &mut srcSize, dstCapacity, notLimited)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue_destSize(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDestSize: c_int,
) -> c_int {
    LZ4_compressHC_continue_generic(LZ4_streamHCPtr, src, dst, srcSizePtr, targetDestSize, fillOutput)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDictHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    safeBuffer: *mut c_char,
    mut dictSize: c_int,
) -> c_int {
    let streamPtr = hc_internal(LZ4_streamHCPtr);
    let prefixSize = ((*streamPtr).end as usize - (*streamPtr).prefixStart as usize) as c_int;
    if dictSize > 64 * 1024 {
        dictSize = 64 * 1024;
    }
    if dictSize < 4 {
        dictSize = 0;
    }
    if dictSize > prefixSize {
        dictSize = prefixSize;
    }
    if dictSize > 0 {
        LZ4_memmove(
            safeBuffer as *mut u8,
            (*streamPtr).end.wrapping_sub(dictSize as usize),
            dictSize as usize,
        );
    }
    let endIndex = ((*streamPtr).end as usize - (*streamPtr).prefixStart as usize) as u32
        + (*streamPtr).dictLimit;
    (*streamPtr).end = if safeBuffer.is_null() {
        ptr::null()
    } else {
        (safeBuffer as *const u8).add(dictSize as usize)
    };
    (*streamPtr).prefixStart = safeBuffer as *const u8;
    (*streamPtr).dictLimit = endIndex - dictSize as u32;
    (*streamPtr).lowLimit = endIndex - dictSize as u32;
    (*streamPtr).dictStart = (*streamPtr).prefixStart;
    if (*streamPtr).nextToUpdate < (*streamPtr).dictLimit {
        (*streamPtr).nextToUpdate = (*streamPtr).dictLimit;
    }
    dictSize
}

/* ===== deprecated ===== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC(src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, LZ4_compressBound(srcSize), 0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput(src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, maxDstSize, 0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2(src: *const c_char, dst: *mut c_char, srcSize: c_int, cLevel: c_int) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, LZ4_compressBound(srcSize), cLevel)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput(src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int, cLevel: c_int) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, maxDstSize, cLevel)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, LZ4_compressBound(srcSize), 0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDstSize, 0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, cLevel: c_int) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, LZ4_compressBound(srcSize), cLevel)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int, cLevel: c_int) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDstSize, cLevel)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_continue(ctx: *mut LZ4_streamHC_t, src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int {
    LZ4_compress_HC_continue(ctx, src, dst, srcSize, LZ4_compressBound(srcSize))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_continue(ctx: *mut LZ4_streamHC_t, src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int) -> c_int {
    LZ4_compress_HC_continue(ctx, src, dst, srcSize, maxDstSize)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamStateHC(state: *mut c_void, inputBuffer: *mut c_char) -> c_int {
    let hc4 = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
    if hc4.is_null() {
        return 1;
    }
    LZ4HC_init_internal(hc_internal(hc4), inputBuffer as *const u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createHC(inputBuffer: *const c_char) -> *mut c_void {
    let hc4 = LZ4_createStreamHC();
    if hc4.is_null() {
        return ptr::null_mut();
    }
    LZ4HC_init_internal(hc_internal(hc4), inputBuffer as *const u8);
    hc4 as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeHC(LZ4HC_Data: *mut c_void) -> c_int {
    if LZ4HC_Data.is_null() {
        return 0;
    }
    crate::c_free(LZ4HC_Data as *mut u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_continue(
    LZ4HC_Data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    LZ4HC_compress_generic(hc_internal(LZ4HC_Data as *mut LZ4_streamHC_t), src, dst, &mut srcSize, 0, cLevel, notLimited)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_continue(
    LZ4HC_Data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut srcSize: c_int,
    dstCapacity: c_int,
    cLevel: c_int,
) -> c_int {
    LZ4HC_compress_generic(hc_internal(LZ4HC_Data as *mut LZ4_streamHC_t), src, dst, &mut srcSize, dstCapacity, cLevel, limitedOutput)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBufferHC(LZ4HC_Data: *mut c_void) -> *mut c_char {
    let s = hc_internal(LZ4HC_Data as *mut LZ4_streamHC_t);
    let bufferStart = (*s)
        .prefixStart
        .wrapping_sub((*s).dictLimit as usize)
        .wrapping_add((*s).lowLimit as usize);
    LZ4_resetStreamHC_fast(LZ4HC_Data as *mut LZ4_streamHC_t, (*s).compressionLevel as c_int);
    bufferStart as *mut c_char
}

// silence unused import warning for LZ4HC_CLEVEL_MIN
const _USE_CLEVEL_MIN: c_int = LZ4HC_CLEVEL_MIN;
