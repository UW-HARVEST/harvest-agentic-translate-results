//! Translation of `lz4hc.c` (LZ4 v1.10.0), built with `LZ4HC_HEAPMODE=1`.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use crate::common::*;
use crate::lz4::*;
use core::ffi::{c_char, c_int, c_void};

pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;

pub const OPTIMAL_ML: c_int = ((ML_MASK - 1) + MINMATCH as u32) as c_int;
pub const LZ4_OPT_NUM: usize = 1 << 12;

/* dictCtx_directive */
pub const noDictCtx: i32 = 0;
pub const usingDictCtxHc: i32 = 1;

/* lz4hc_strat_e */
pub const lz4mid: i32 = 0;
pub const lz4hc: i32 = 1;
pub const lz4opt: i32 = 2;

/* HCfavor_e */
pub const favorCompressionRatio: i32 = 0;
pub const favorDecompressionSpeed: i32 = 1;

#[derive(Clone, Copy)]
pub struct cParams_t {
    pub strat: i32,
    pub nbSearches: c_int,
    pub targetLength: u32,
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

pub fn LZ4HC_getCLevelParams(mut cLevel: c_int) -> cParams_t {
    if cLevel < 1 {
        cLevel = LZ4HC_CLEVEL_DEFAULT;
    }
    if LZ4HC_CLEVEL_MAX < cLevel {
        cLevel = LZ4HC_CLEVEL_MAX;
    }
    k_clTable[cLevel as usize]
}

/* ===== Hashing ===== */
pub const LZ4HC_HASHSIZE: c_int = 4;

#[inline(always)]
fn HASH_FUNCTION(i: u32) -> u32 {
    i.wrapping_mul(2654435761u32) >> ((MINMATCH as u32 * 8) - LZ4HC_HASH_LOG)
}
#[inline(always)]
unsafe fn LZ4HC_hashPtr(ptr: *const u8) -> u32 {
    unsafe { HASH_FUNCTION(LZ4_read32(ptr)) }
}

pub const LZ4MID_HASHSIZE: usize = 8;
pub const LZ4MID_HASHLOG: u32 = LZ4HC_HASH_LOG - 1;
pub const LZ4MID_HASHTABLESIZE: usize = 1usize << LZ4MID_HASHLOG;

#[inline(always)]
fn LZ4MID_hash4(v: u32) -> u32 {
    v.wrapping_mul(2654435761u32) >> (32 - LZ4MID_HASHLOG)
}
#[inline(always)]
unsafe fn LZ4MID_hash4Ptr(ptr: *const u8) -> u32 {
    unsafe { LZ4MID_hash4(LZ4_read32(ptr)) }
}
#[inline(always)]
fn LZ4MID_hash7(v: u64) -> u32 {
    (((v << (64 - 56)).wrapping_mul(58295818150454627u64)) >> (64 - LZ4MID_HASHLOG)) as u32
}
#[inline(always)]
unsafe fn LZ4MID_hash8Ptr(ptr: *const u8) -> u32 {
    unsafe { LZ4MID_hash7(LZ4_readLE64(ptr)) }
}

/* ===== Count match length ===== */
/// LE: `__builtin_clz(val) >> 3`
#[inline(always)]
fn LZ4HC_NbCommonBytes32(val: u32) -> u32 {
    val.leading_zeros() >> 3
}

#[inline(always)]
unsafe fn LZ4HC_countBack(
    ip: *const u8,
    r#match: *const u8,
    iMin: *const u8,
    mMin: *const u8,
) -> c_int {
    unsafe {
        let mut back: c_int = 0;
        let a = pdiff(iMin, ip);
        let b = pdiff(mMin, r#match);
        let min: c_int = (if a > b { a } else { b }) as c_int;

        while (back - min) > 3 {
            let v = LZ4_read32(coff(ip, (back - 4) as isize))
                ^ LZ4_read32(coff(r#match, (back - 4) as isize));
            if v != 0 {
                return back - (LZ4HC_NbCommonBytes32(v) as c_int);
            } else {
                back -= 4;
            }
        }
        while (back > min)
            && (*coff(ip, (back - 1) as isize) == *coff(r#match, (back - 1) as isize))
        {
            back -= 1;
        }
        back
    }
}

/* ===== Chain table updates ===== */
#[inline(always)]
unsafe fn DELTANEXTU16_get(table: *const u16, pos: u32) -> u32 {
    unsafe { *table.wrapping_add((pos as u16) as usize) as u32 }
}
#[inline(always)]
unsafe fn DELTANEXTU16_set(table: *mut u16, pos: u32, v: u16) {
    unsafe {
        *table.wrapping_add((pos as u16) as usize) = v;
    }
}

/* ===== Init ===== */
unsafe fn LZ4HC_clearTables(hc4: *mut LZ4HC_CCtx_internal) {
    unsafe {
        MEM_INIT(
            (*hc4).hashTable.as_mut_ptr() as *mut u8,
            0,
            core::mem::size_of::<[u32; LZ4HC_HASHTABLESIZE]>(),
        );
        MEM_INIT(
            (*hc4).chainTable.as_mut_ptr() as *mut u8,
            0xFF,
            core::mem::size_of::<[u16; LZ4HC_MAXD]>(),
        );
    }
}

unsafe fn LZ4HC_init_internal(hc4: *mut LZ4HC_CCtx_internal, start: *const u8) {
    unsafe {
        let bufferSize = pdiff((*hc4).end, (*hc4).prefixStart) as usize;
        let mut newStartingOffset = bufferSize + (*hc4).dictLimit as usize;
        if newStartingOffset > (1usize << 30) {
            LZ4HC_clearTables(hc4);
            newStartingOffset = 0;
        }
        newStartingOffset += 64 * KB;
        (*hc4).nextToUpdate = newStartingOffset as u32;
        (*hc4).prefixStart = start;
        (*hc4).end = start;
        (*hc4).dictStart = start;
        (*hc4).dictLimit = newStartingOffset as u32;
        (*hc4).lowLimit = newStartingOffset as u32;
    }
}

/* ===== Encode ===== */
/// @return : 0 if ok, 1 if buffer issue detected
#[inline(always)]
unsafe fn LZ4HC_encodeSequence(
    _ip: *mut *const u8,
    _op: *mut *mut u8,
    _anchor: *mut *const u8,
    matchLength: c_int,
    offset: c_int,
    limit: i32,
    oend: *mut u8,
) -> c_int {
    unsafe {
        let mut length: usize;
        let token: *mut u8 = *_op;
        *_op = madd(*_op, 1);

        /* Encode Literal length */
        length = pdiff(*_ip, *_anchor) as usize;
        if limit != notLimited
            && (madd(*_op, (length / 255) + length + (2 + 1 + LASTLITERALS)) > oend)
        {
            return 1;
        }
        if length >= RUN_MASK as usize {
            let mut len = length - RUN_MASK as usize;
            *token = (RUN_MASK << ML_BITS) as u8;
            while len >= 255 {
                **_op = 255;
                *_op = madd(*_op, 1);
                len -= 255;
            }
            **_op = len as u8;
            *_op = madd(*_op, 1);
        } else {
            *token = ((length as u32) << ML_BITS) as u8;
        }

        /* Copy Literals */
        LZ4_wildCopy8(*_op, *_anchor, madd(*_op, length));
        *_op = madd(*_op, length);

        /* Encode Offset */
        LZ4_writeLE16(*_op, offset as u16);
        *_op = madd(*_op, 2);

        /* Encode MatchLength */
        length = (matchLength as usize) - MINMATCH;
        if limit != notLimited && (madd(*_op, (length / 255) + (1 + LASTLITERALS)) > oend) {
            return 1;
        }
        if length >= ML_MASK as usize {
            *token = (*token).wrapping_add(ML_MASK as u8);
            length -= ML_MASK as usize;
            while length >= 510 {
                **_op = 255;
                *_op = madd(*_op, 1);
                **_op = 255;
                *_op = madd(*_op, 1);
                length -= 510;
            }
            if length >= 255 {
                length -= 255;
                **_op = 255;
                *_op = madd(*_op, 1);
            }
            **_op = length as u8;
            *_op = madd(*_op, 1);
        } else {
            *token = (*token).wrapping_add(length as u8);
        }

        /* Prepare next loop */
        *_ip = cadd(*_ip, matchLength as usize);
        *_anchor = *_ip;

        0
    }
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
    unsafe {
        let lDictEndIndex: usize =
            (pdiff((*dictCtx).end, (*dictCtx).prefixStart) as usize) + (*dictCtx).dictLimit as usize;
        let mut lDictMatchIndex: u32 =
            (*dictCtx).hashTable[LZ4HC_hashPtr(ip) as usize];
        let mut matchIndex: u32 = lDictMatchIndex
            .wrapping_add(gDictEndIndex)
            .wrapping_sub(lDictEndIndex as u32);
        let mut offset: c_int = 0;
        let mut sBack: c_int = 0;

        while ipIndex.wrapping_sub(matchIndex) <= LZ4_DISTANCE_MAX && {
            let t = nbAttempts;
            nbAttempts -= 1;
            t != 0
        } {
            let matchPtr: *const u8 = cadd(
                csub((*dictCtx).prefixStart, (*dictCtx).dictLimit as usize),
                lDictMatchIndex as usize,
            );

            if LZ4_read32(matchPtr) == LZ4_read32(ip) {
                let mut mlt: c_int;
                let mut back: c_int;
                let mut vLimit = cadd(ip, lDictEndIndex - lDictMatchIndex as usize);
                if vLimit > iHighLimit {
                    vLimit = iHighLimit;
                }
                mlt = (LZ4_count(cadd(ip, MINMATCH), cadd(matchPtr, MINMATCH), vLimit) as c_int)
                    + MINMATCH as c_int;
                back = if ip > iLowLimit {
                    LZ4HC_countBack(ip, matchPtr, iLowLimit, (*dictCtx).prefixStart)
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

            {
                let nextOffset =
                    DELTANEXTU16_get((*dictCtx).chainTable.as_ptr(), lDictMatchIndex);
                lDictMatchIndex = lDictMatchIndex.wrapping_sub(nextOffset);
                matchIndex = matchIndex.wrapping_sub(nextOffset);
            }
        }

        LZ4HC_match_t {
            len: currentBestML,
            off: offset,
            back: sBack,
        }
    }
}

type LZ4MID_searchIntoDict_f = unsafe fn(
    ip: *const u8,
    ipIndex: u32,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
) -> LZ4HC_match_t;

unsafe fn LZ4MID_searchHCDict(
    ip: *const u8,
    ipIndex: u32,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
) -> LZ4HC_match_t {
    unsafe {
        LZ4HC_searchExtDict(
            ip,
            ipIndex,
            ip,
            iHighLimit,
            dictCtx,
            gDictEndIndex,
            MINMATCH as c_int - 1,
            2,
        )
    }
}

unsafe fn LZ4MID_searchExtDict(
    ip: *const u8,
    ipIndex: u32,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
) -> LZ4HC_match_t {
    unsafe {
        let lDictEndIndex: usize =
            (pdiff((*dictCtx).end, (*dictCtx).prefixStart) as usize) + (*dictCtx).dictLimit as usize;
        let hash4Table: *const u32 = (*dictCtx).hashTable.as_ptr();
        let hash8Table: *const u32 = hash4Table.wrapping_add(LZ4MID_HASHTABLESIZE);

        /* search long match first */
        {
            let l8DictMatchIndex = *hash8Table.wrapping_add(LZ4MID_hash8Ptr(ip) as usize);
            let m8Index = l8DictMatchIndex
                .wrapping_add(gDictEndIndex)
                .wrapping_sub(lDictEndIndex as u32);
            if ipIndex.wrapping_sub(m8Index) <= LZ4_DISTANCE_MAX {
                let matchPtr = cadd(
                    csub((*dictCtx).prefixStart, (*dictCtx).dictLimit as usize),
                    l8DictMatchIndex as usize,
                );
                let safeLen = MINu2(
                    lDictEndIndex - l8DictMatchIndex as usize,
                    pdiff(iHighLimit, ip) as usize,
                );
                let mlt = LZ4_count(ip, matchPtr, cadd(ip, safeLen)) as c_int;
                if mlt >= MINMATCH as c_int {
                    return LZ4HC_match_t {
                        len: mlt,
                        off: ipIndex.wrapping_sub(m8Index) as c_int,
                        back: 0,
                    };
                }
            }
        }

        /* search for short match second */
        {
            let l4DictMatchIndex = *hash4Table.wrapping_add(LZ4MID_hash4Ptr(ip) as usize);
            let m4Index = l4DictMatchIndex
                .wrapping_add(gDictEndIndex)
                .wrapping_sub(lDictEndIndex as u32);
            if ipIndex.wrapping_sub(m4Index) <= LZ4_DISTANCE_MAX {
                let matchPtr = cadd(
                    csub((*dictCtx).prefixStart, (*dictCtx).dictLimit as usize),
                    l4DictMatchIndex as usize,
                );
                let safeLen = MINu2(
                    lDictEndIndex - l4DictMatchIndex as usize,
                    pdiff(iHighLimit, ip) as usize,
                );
                let mlt = LZ4_count(ip, matchPtr, cadd(ip, safeLen)) as c_int;
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
}

#[inline(always)]
pub fn MINu2(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
#[inline(always)]
pub fn MINi2(a: c_int, b: c_int) -> c_int {
    if a < b { a } else { b }
}
#[inline(always)]
pub fn MAXi2(a: c_int, b: c_int) -> c_int {
    if a > b { a } else { b }
}
#[inline(always)]
pub fn MAXu32(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

/* ===== Mid Compression (level 2) ===== */
#[inline(always)]
unsafe fn LZ4MID_addPosition(hTable: *mut u32, hValue: u32, index: u32) {
    unsafe {
        *hTable.wrapping_add(hValue as usize) = index;
    }
}

/// Fill hash tables with references into dictionary (only exploitable by LZ4MID)
unsafe fn LZ4MID_fillHTable(cctx: *mut LZ4HC_CCtx_internal, dict: *const c_void, size: usize) {
    unsafe {
        let hash4Table: *mut u32 = (*cctx).hashTable.as_mut_ptr();
        let hash8Table: *mut u32 = hash4Table.wrapping_add(LZ4MID_HASHTABLESIZE);
        let prefixPtr: *const u8 = dict as *const u8;
        let prefixIdx: u32 = (*cctx).dictLimit;
        let target: u32 = prefixIdx
            .wrapping_add(size as u32)
            .wrapping_sub(LZ4MID_HASHSIZE as u32);
        let mut idx: u32 = (*cctx).nextToUpdate;
        if size <= LZ4MID_HASHSIZE {
            return;
        }

        while idx < target {
            let p4 = csub(cadd(prefixPtr, idx as usize), prefixIdx as usize);
            LZ4MID_addPosition(hash4Table, LZ4MID_hash4Ptr(p4), idx);
            let p8 = csub(cadd(prefixPtr, idx as usize + 1), prefixIdx as usize);
            LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(p8), idx + 1);
            idx += 3;
        }

        idx = if size > 32 * KB + LZ4MID_HASHSIZE {
            target.wrapping_sub(32 * KB as u32)
        } else {
            (*cctx).nextToUpdate
        };
        while idx < target {
            let p8 = csub(cadd(prefixPtr, idx as usize), prefixIdx as usize);
            LZ4MID_addPosition(hash8Table, LZ4MID_hash8Ptr(p8), idx);
            idx += 1;
        }

        (*cctx).nextToUpdate = target;
    }
}

fn select_searchDict_function(
    dictCtx: *const LZ4HC_CCtx_internal,
) -> Option<LZ4MID_searchIntoDict_f> {
    if dictCtx.is_null() {
        return None;
    }
    unsafe {
        if LZ4HC_getCLevelParams((*dictCtx).compressionLevel as c_int).strat == lz4mid {
            Some(LZ4MID_searchExtDict as LZ4MID_searchIntoDict_f)
        } else {
            Some(LZ4MID_searchHCDict as LZ4MID_searchIntoDict_f)
        }
    }
}

unsafe fn LZ4MID_compress(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    maxOutputSize: c_int,
    limit: i32,
    dict: i32,
) -> c_int {
    unsafe {
        let hash4Table: *mut u32 = (*ctx).hashTable.as_mut_ptr();
        let hash8Table: *mut u32 = hash4Table.wrapping_add(LZ4MID_HASHTABLESIZE);
        let mut ip: *const u8 = src as *const u8;
        let mut anchor: *const u8 = ip;
        let iend: *const u8 = cadd(ip, *srcSizePtr as usize);
        let mflimit: *const u8 = csub(iend, MFLIMIT);
        let matchlimit: *const u8 = csub(iend, LASTLITERALS);
        let ilimit: *const u8 = csub(iend, LZ4MID_HASHSIZE);
        let mut op: *mut u8 = dst as *mut u8;
        let mut oend: *mut u8 = madd(op, maxOutputSize as usize);

        let prefixPtr: *const u8 = (*ctx).prefixStart;
        let prefixIdx: u32 = (*ctx).dictLimit;
        let ilimitIdx: u32 = (pdiff(ilimit, prefixPtr) as u32).wrapping_add(prefixIdx);
        let dictStart: *const u8 = (*ctx).dictStart;
        let dictIdx: u32 = (*ctx).lowLimit;
        let gDictEndIndex: u32 = (*ctx).lowLimit;
        let searchIntoDict: Option<LZ4MID_searchIntoDict_f> = if dict == usingDictCtxHc {
            select_searchDict_function((*ctx).dictCtx)
        } else {
            None
        };
        let mut matchLength: u32 = 0;
        let mut matchDistance: u32 = 0;

        /* input sanitization */
        if *srcSizePtr < 0 {
            return 0;
        }
        if maxOutputSize < 0 {
            return 0;
        }
        if *srcSizePtr > LZ4_MAX_INPUT_SIZE as c_int {
            return 0;
        }
        if limit == fillOutput {
            oend = msub(oend, LASTLITERALS);
        }

        'lastlit: {
            if *srcSizePtr < LZ4_minLength {
                break 'lastlit;
            }

            /* main loop */
            while ip <= mflimit {
                let ipIndex: u32 = (pdiff(ip, prefixPtr) as u32).wrapping_add(prefixIdx);
                let mut do_encode = false;

                'search: {
                    /* search long match */
                    {
                        let h8 = LZ4MID_hash8Ptr(ip);
                        let pos8 = *hash8Table.wrapping_add(h8 as usize);
                        LZ4MID_addPosition(hash8Table, h8, ipIndex);
                        if ipIndex.wrapping_sub(pos8) <= LZ4_DISTANCE_MAX {
                            if pos8 >= prefixIdx {
                                let matchPtr =
                                    csub(cadd(prefixPtr, pos8 as usize), prefixIdx as usize);
                                matchLength = LZ4_count(ip, matchPtr, matchlimit);
                                if matchLength >= MINMATCH as u32 {
                                    matchDistance = ipIndex.wrapping_sub(pos8);
                                    do_encode = true;
                                    break 'search;
                                }
                            } else if pos8 >= dictIdx {
                                let matchPtr = cadd(dictStart, (pos8 - dictIdx) as usize);
                                let safeLen = MINu2(
                                    (prefixIdx - pos8) as usize,
                                    pdiff(matchlimit, ip) as usize,
                                );
                                matchLength = LZ4_count(ip, matchPtr, cadd(ip, safeLen));
                                if matchLength >= MINMATCH as u32 {
                                    matchDistance = ipIndex.wrapping_sub(pos8);
                                    do_encode = true;
                                    break 'search;
                                }
                            }
                        }
                    }
                    /* search short match */
                    {
                        let h4 = LZ4MID_hash4Ptr(ip);
                        let pos4 = *hash4Table.wrapping_add(h4 as usize);
                        LZ4MID_addPosition(hash4Table, h4, ipIndex);
                        if ipIndex.wrapping_sub(pos4) <= LZ4_DISTANCE_MAX {
                            if pos4 >= prefixIdx {
                                let matchPtr = cadd(prefixPtr, (pos4 - prefixIdx) as usize);
                                matchLength = LZ4_count(ip, matchPtr, matchlimit);
                                if matchLength >= MINMATCH as u32 {
                                    let h8 = LZ4MID_hash8Ptr(cadd(ip, 1));
                                    let pos8 = *hash8Table.wrapping_add(h8 as usize);
                                    let m2Distance = ipIndex.wrapping_add(1).wrapping_sub(pos8);
                                    matchDistance = ipIndex.wrapping_sub(pos4);
                                    if m2Distance <= LZ4_DISTANCE_MAX
                                        && pos8 >= prefixIdx
                                        && ip < mflimit
                                    {
                                        let m2Ptr = cadd(prefixPtr, (pos8 - prefixIdx) as usize);
                                        let ml2 = LZ4_count(cadd(ip, 1), m2Ptr, matchlimit);
                                        if ml2 > matchLength {
                                            LZ4MID_addPosition(
                                                hash8Table,
                                                h8,
                                                ipIndex.wrapping_add(1),
                                            );
                                            ip = cadd(ip, 1);
                                            matchLength = ml2;
                                            matchDistance = m2Distance;
                                        }
                                    }
                                    do_encode = true;
                                    break 'search;
                                }
                            } else if pos4 >= dictIdx {
                                let matchPtr = cadd(dictStart, (pos4 - dictIdx) as usize);
                                let safeLen = MINu2(
                                    (prefixIdx - pos4) as usize,
                                    pdiff(matchlimit, ip) as usize,
                                );
                                matchLength = LZ4_count(ip, matchPtr, cadd(ip, safeLen));
                                if matchLength >= MINMATCH as u32 {
                                    matchDistance = ipIndex.wrapping_sub(pos4);
                                    do_encode = true;
                                    break 'search;
                                }
                            }
                        }
                    }
                    /* no match found in prefix */
                    if (dict == usingDictCtxHc)
                        && (ipIndex.wrapping_sub(gDictEndIndex) < LZ4_DISTANCE_MAX - 8)
                    {
                        let f = searchIntoDict.unwrap();
                        let dMatch = f(ip, ipIndex, matchlimit, (*ctx).dictCtx, gDictEndIndex);
                        if dMatch.len >= MINMATCH as c_int {
                            matchLength = dMatch.len as u32;
                            matchDistance = dMatch.off as u32;
                            do_encode = true;
                            break 'search;
                        }
                    }
                }

                if !do_encode {
                    ip = coff(ip, 1 + (pdiff(ip, anchor) >> 9));
                    continue;
                }

                /* _lz4mid_encode_sequence: */
                /* catch back */
                while ((ip > anchor) && ((pdiff(ip, prefixPtr) as u32) > matchDistance))
                    && (*csub(ip, 1) == *csub(ip, matchDistance as usize + 1))
                {
                    ip = csub(ip, 1);
                    matchLength += 1;
                }

                /* fill table with beginning of match */
                LZ4MID_addPosition(
                    hash8Table,
                    LZ4MID_hash8Ptr(cadd(ip, 1)),
                    ipIndex.wrapping_add(1),
                );
                LZ4MID_addPosition(
                    hash8Table,
                    LZ4MID_hash8Ptr(cadd(ip, 2)),
                    ipIndex.wrapping_add(2),
                );
                LZ4MID_addPosition(
                    hash4Table,
                    LZ4MID_hash4Ptr(cadd(ip, 1)),
                    ipIndex.wrapping_add(1),
                );

                /* encode */
                {
                    let saved_op = op;
                    if LZ4HC_encodeSequence(
                        &mut ip,
                        &mut op,
                        &mut anchor,
                        matchLength as c_int,
                        matchDistance as c_int,
                        limit,
                        oend,
                    ) != 0
                    {
                        op = saved_op;
                        /* _lz4mid_dest_overflow: */
                        if limit == fillOutput {
                            let ll = pdiff(ip, anchor) as usize;
                            let ll_addbytes = (ll + 240) / 255;
                            let ll_totalCost = 1 + ll_addbytes + ll;
                            let maxLitPos = msub(oend, 3);
                            if madd(op, ll_totalCost) <= maxLitPos {
                                let bytesLeftForMl =
                                    pdiff(maxLitPos, madd(op, ll_totalCost)) as usize;
                                let maxMlSize =
                                    MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                                if (matchLength as usize) > maxMlSize {
                                    matchLength = maxMlSize as u32;
                                }
                                if pdiff(madd(oend, LASTLITERALS), madd(op, ll_totalCost + 2)) - 1
                                    + matchLength as isize
                                    >= MFLIMIT as isize
                                {
                                    LZ4HC_encodeSequence(
                                        &mut ip,
                                        &mut op,
                                        &mut anchor,
                                        matchLength as c_int,
                                        matchDistance as c_int,
                                        notLimited,
                                        oend,
                                    );
                                }
                            }
                            break 'lastlit;
                        }
                        return 0;
                    }
                }

                /* fill table with end of match */
                {
                    let endMatchIdx: u32 = (pdiff(ip, prefixPtr) as u32).wrapping_add(prefixIdx);
                    let pos_m2 = endMatchIdx.wrapping_sub(2);
                    if pos_m2 < ilimitIdx {
                        if pdiff(ip, prefixPtr) > 5 {
                            LZ4MID_addPosition(
                                hash8Table,
                                LZ4MID_hash8Ptr(csub(ip, 5)),
                                endMatchIdx.wrapping_sub(5),
                            );
                        }
                        LZ4MID_addPosition(
                            hash8Table,
                            LZ4MID_hash8Ptr(csub(ip, 3)),
                            endMatchIdx.wrapping_sub(3),
                        );
                        LZ4MID_addPosition(
                            hash8Table,
                            LZ4MID_hash8Ptr(csub(ip, 2)),
                            endMatchIdx.wrapping_sub(2),
                        );
                        LZ4MID_addPosition(
                            hash4Table,
                            LZ4MID_hash4Ptr(csub(ip, 2)),
                            endMatchIdx.wrapping_sub(2),
                        );
                        LZ4MID_addPosition(
                            hash4Table,
                            LZ4MID_hash4Ptr(csub(ip, 1)),
                            endMatchIdx.wrapping_sub(1),
                        );
                    }
                }
            }
        }

        /* _lz4mid_last_literals: */
        {
            let mut lastRunSize: usize = pdiff(iend, anchor) as usize;
            let mut llAdd: usize = (lastRunSize + 255 - RUN_MASK as usize) / 255;
            let totalSize: usize = 1 + llAdd + lastRunSize;
            if limit == fillOutput {
                oend = madd(oend, LASTLITERALS);
            }
            if limit != notLimited && (madd(op, totalSize) > oend) {
                if limit == limitedOutput {
                    return 0;
                }
                lastRunSize = (pdiff(oend, op) as usize) - 1;
                llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
                lastRunSize -= llAdd;
            }
            ip = cadd(anchor, lastRunSize);

            if lastRunSize >= RUN_MASK as usize {
                let mut accumulator = lastRunSize - RUN_MASK as usize;
                *op = (RUN_MASK << ML_BITS) as u8;
                op = madd(op, 1);
                while accumulator >= 255 {
                    *op = 255;
                    op = madd(op, 1);
                    accumulator -= 255;
                }
                *op = accumulator as u8;
                op = madd(op, 1);
            } else {
                *op = ((lastRunSize as u32) << ML_BITS) as u8;
                op = madd(op, 1);
            }
            LZ4_memcpy(op, anchor, lastRunSize);
            op = madd(op, lastRunSize);
        }

        *srcSizePtr = pdiff(ip, src as *const u8) as c_int;
        pdiff(op, dst as *const u8) as c_int
    }
}

/* ===== HC Compression - Search ===== */

/// Update chains up to ip (excluded)
#[inline(always)]
unsafe fn LZ4HC_Insert(hc4: *mut LZ4HC_CCtx_internal, ip: *const u8) {
    unsafe {
        let chainTable: *mut u16 = (*hc4).chainTable.as_mut_ptr();
        let hashTable: *mut u32 = (*hc4).hashTable.as_mut_ptr();
        let prefixPtr: *const u8 = (*hc4).prefixStart;
        let prefixIdx: u32 = (*hc4).dictLimit;
        let target: u32 = (pdiff(ip, prefixPtr) as u32).wrapping_add(prefixIdx);
        let mut idx: u32 = (*hc4).nextToUpdate;

        while idx < target {
            let h = LZ4HC_hashPtr(csub(cadd(prefixPtr, idx as usize), prefixIdx as usize));
            let mut delta: usize = (idx.wrapping_sub(*hashTable.wrapping_add(h as usize))) as usize;
            if delta > LZ4_DISTANCE_MAX as usize {
                delta = LZ4_DISTANCE_MAX as usize;
            }
            DELTANEXTU16_set(chainTable, idx, delta as u16);
            *hashTable.wrapping_add(h as usize) = idx;
            idx += 1;
        }

        (*hc4).nextToUpdate = target;
    }
}

#[inline(always)]
fn LZ4HC_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}

fn LZ4HC_rotatePattern(rotate: usize, pattern: u32) -> u32 {
    let bitsToRotate = (rotate & (core::mem::size_of::<u32>() - 1)) << 3;
    if bitsToRotate == 0 {
        return pattern;
    }
    LZ4HC_rotl32(pattern, bitsToRotate as u32)
}

unsafe fn LZ4HC_countPattern(ip0: *const u8, iEnd: *const u8, pattern32: u32) -> u32 {
    unsafe {
        let iStart = ip0;
        let mut ip = ip0;
        let pattern: RegT = (pattern32 as RegT) + ((pattern32 as RegT) << 32);

        while ip < csub(iEnd, core::mem::size_of::<RegT>() - 1) {
            let diff = LZ4_read_ARCH(ip) ^ pattern;
            if diff == 0 {
                ip = cadd(ip, core::mem::size_of::<RegT>());
                continue;
            }
            ip = cadd(ip, LZ4_NbCommonBytes(diff) as usize);
            return pdiff(ip, iStart) as u32;
        }

        let mut patternByte: RegT = pattern;
        while (ip < iEnd) && (*ip == (patternByte as u8)) {
            ip = cadd(ip, 1);
            patternByte >>= 8;
        }

        pdiff(ip, iStart) as u32
    }
}

unsafe fn LZ4HC_reverseCountPattern(ip0: *const u8, iLow: *const u8, pattern: u32) -> u32 {
    unsafe {
        let iStart = ip0;
        let mut ip = ip0;

        while ip >= cadd(iLow, 4) {
            if LZ4_read32(csub(ip, 4)) != pattern {
                break;
            }
            ip = csub(ip, 4);
        }
        {
            let pat_bytes = &pattern as *const u32 as *const u8;
            let mut bytePtr = pat_bytes.wrapping_add(3);
            while ip > iLow {
                if *csub(ip, 1) != *bytePtr {
                    break;
                }
                ip = csub(ip, 1);
                bytePtr = bytePtr.wrapping_sub(1);
            }
        }
        pdiff(iStart, ip) as u32
    }
}

fn LZ4HC_protectDictEnd(dictLimit: u32, matchIndex: u32) -> c_int {
    ((dictLimit.wrapping_sub(1).wrapping_sub(matchIndex)) >= 3) as c_int
}

const rep_untested: i32 = 0;
const rep_not: i32 = 1;
const rep_confirmed: i32 = 2;

#[allow(clippy::too_many_arguments)]
unsafe fn LZ4HC_InsertAndGetWiderMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iLowLimit: *const u8,
    iHighLimit: *const u8,
    mut longest: c_int,
    maxNbAttempts: c_int,
    patternAnalysis: c_int,
    chainSwap: c_int,
    dict: i32,
    favorDecSpeed: i32,
) -> LZ4HC_match_t {
    unsafe {
        let chainTable: *mut u16 = (*hc4).chainTable.as_mut_ptr();
        let hashTable: *mut u32 = (*hc4).hashTable.as_mut_ptr();
        let dictCtx: *const LZ4HC_CCtx_internal = (*hc4).dictCtx;
        let prefixPtr: *const u8 = (*hc4).prefixStart;
        let prefixIdx: u32 = (*hc4).dictLimit;
        let ipIndex: u32 = (pdiff(ip, prefixPtr) as u32).wrapping_add(prefixIdx);
        let withinStartDistance: bool =
            (*hc4).lowLimit.wrapping_add(LZ4_DISTANCE_MAX + 1) > ipIndex;
        let lowestMatchIndex: u32 = if withinStartDistance {
            (*hc4).lowLimit
        } else {
            ipIndex.wrapping_sub(LZ4_DISTANCE_MAX)
        };
        let dictStart: *const u8 = (*hc4).dictStart;
        let dictIdx: u32 = (*hc4).lowLimit;
        let dictEnd: *const u8 = cadd(dictStart, prefixIdx.wrapping_sub(dictIdx) as usize);
        let lookBackLength: c_int = pdiff(ip, iLowLimit) as c_int;
        let mut nbAttempts: c_int = maxNbAttempts;
        let mut matchChainPos: u32 = 0;
        let pattern: u32 = LZ4_read32(ip);
        let mut matchIndex: u32;
        let mut repeat: i32 = rep_untested;
        let mut srcPatternLength: usize = 0;
        let mut offset: c_int = 0;
        let mut sBack: c_int = 0;

        /* First Match */
        LZ4HC_Insert(hc4, ip);
        matchIndex = *hashTable.wrapping_add(LZ4HC_hashPtr(ip) as usize);

        'outer: while (matchIndex >= lowestMatchIndex) && (nbAttempts > 0) {
            let mut matchLength: c_int = 0;
            nbAttempts -= 1;
            if favorDecSpeed != 0 && (ipIndex.wrapping_sub(matchIndex) < 8) {
                /* do nothing */
            } else if matchIndex >= prefixIdx {
                let matchPtr = cadd(prefixPtr, matchIndex.wrapping_sub(prefixIdx) as usize);
                if LZ4_read16(coff(iLowLimit, (longest - 1) as isize))
                    == LZ4_read16(coff(matchPtr, (-lookBackLength + longest - 1) as isize))
                {
                    if LZ4_read32(matchPtr) == pattern {
                        let back = if lookBackLength != 0 {
                            LZ4HC_countBack(ip, matchPtr, iLowLimit, prefixPtr)
                        } else {
                            0
                        };
                        matchLength = MINMATCH as c_int
                            + (LZ4_count(cadd(ip, MINMATCH), cadd(matchPtr, MINMATCH), iHighLimit)
                                as c_int);
                        matchLength -= back;
                        if matchLength > longest {
                            longest = matchLength;
                            offset = ipIndex.wrapping_sub(matchIndex) as c_int;
                            sBack = back;
                        }
                    }
                }
            } else {
                let matchPtr = cadd(dictStart, matchIndex.wrapping_sub(dictIdx) as usize);
                if (matchIndex <= prefixIdx.wrapping_sub(4)) && (LZ4_read32(matchPtr) == pattern) {
                    let mut back;
                    let mut vLimit = cadd(ip, prefixIdx.wrapping_sub(matchIndex) as usize);
                    if vLimit > iHighLimit {
                        vLimit = iHighLimit;
                    }
                    matchLength = (LZ4_count(cadd(ip, MINMATCH), cadd(matchPtr, MINMATCH), vLimit)
                        as c_int)
                        + MINMATCH as c_int;
                    if (cadd(ip, matchLength as usize) == vLimit) && (vLimit < iHighLimit) {
                        matchLength += LZ4_count(
                            cadd(ip, matchLength as usize),
                            prefixPtr,
                            iHighLimit,
                        ) as c_int;
                    }
                    back = if lookBackLength != 0 {
                        LZ4HC_countBack(ip, matchPtr, iLowLimit, dictStart)
                    } else {
                        0
                    };
                    matchLength -= back;
                    if matchLength > longest {
                        longest = matchLength;
                        offset = ipIndex.wrapping_sub(matchIndex) as c_int;
                        sBack = back;
                    }
                }
            }

            if chainSwap != 0 && matchLength == longest {
                if matchIndex.wrapping_add(longest as u32) <= ipIndex {
                    let kTrigger: c_int = 4;
                    let mut distanceToNextMatch: u32 = 1;
                    let end: c_int = longest - MINMATCH as c_int + 1;
                    let mut step: c_int = 1;
                    let mut accel: c_int = 1 << kTrigger;
                    let mut pos: c_int = 0;
                    while pos < end {
                        let candidateDist =
                            DELTANEXTU16_get(chainTable, matchIndex.wrapping_add(pos as u32));
                        step = {
                            let s = accel >> kTrigger;
                            accel += 1;
                            s
                        };
                        if candidateDist > distanceToNextMatch {
                            distanceToNextMatch = candidateDist;
                            matchChainPos = pos as u32;
                            accel = 1 << kTrigger;
                        }
                        pos += step;
                    }
                    if distanceToNextMatch > 1 {
                        if distanceToNextMatch > matchIndex {
                            break 'outer;
                        }
                        matchIndex = matchIndex.wrapping_sub(distanceToNextMatch);
                        continue 'outer;
                    }
                }
            }

            {
                let distNextMatch = DELTANEXTU16_get(chainTable, matchIndex);
                if patternAnalysis != 0 && distNextMatch == 1 && matchChainPos == 0 {
                    let matchCandidateIdx = matchIndex.wrapping_sub(1);
                    if repeat == rep_untested {
                        if ((pattern & 0xFFFF) == (pattern >> 16))
                            && ((pattern & 0xFF) == (pattern >> 24))
                        {
                            repeat = rep_confirmed;
                            srcPatternLength = LZ4HC_countPattern(
                                cadd(ip, core::mem::size_of::<u32>()),
                                iHighLimit,
                                pattern,
                            ) as usize
                                + core::mem::size_of::<u32>();
                        } else {
                            repeat = rep_not;
                        }
                    }
                    if (repeat == rep_confirmed)
                        && (matchCandidateIdx >= lowestMatchIndex)
                        && LZ4HC_protectDictEnd(prefixIdx, matchCandidateIdx) != 0
                    {
                        let extDict = matchCandidateIdx < prefixIdx;
                        let matchPtr = if extDict {
                            cadd(dictStart, matchCandidateIdx.wrapping_sub(dictIdx) as usize)
                        } else {
                            cadd(prefixPtr, matchCandidateIdx.wrapping_sub(prefixIdx) as usize)
                        };
                        if LZ4_read32(matchPtr) == pattern {
                            let iLimit = if extDict { dictEnd } else { iHighLimit };
                            let mut forwardPatternLength = LZ4HC_countPattern(
                                cadd(matchPtr, core::mem::size_of::<u32>()),
                                iLimit,
                                pattern,
                            ) as usize
                                + core::mem::size_of::<u32>();
                            if extDict && cadd(matchPtr, forwardPatternLength) == iLimit {
                                let rotatedPattern =
                                    LZ4HC_rotatePattern(forwardPatternLength, pattern);
                                forwardPatternLength +=
                                    LZ4HC_countPattern(prefixPtr, iHighLimit, rotatedPattern)
                                        as usize;
                            }
                            {
                                let lowestMatchPtr = if extDict { dictStart } else { prefixPtr };
                                let mut backLength =
                                    LZ4HC_reverseCountPattern(matchPtr, lowestMatchPtr, pattern)
                                        as usize;
                                let currentSegmentLength: usize;
                                if !extDict
                                    && csub(matchPtr, backLength) == prefixPtr
                                    && dictIdx < prefixIdx
                                {
                                    let rotatedPattern = LZ4HC_rotatePattern(
                                        (0u32.wrapping_sub(backLength as u32)) as usize,
                                        pattern,
                                    );
                                    backLength += LZ4HC_reverseCountPattern(
                                        dictEnd,
                                        dictStart,
                                        rotatedPattern,
                                    ) as usize;
                                }
                                /* Limit backLength not go further than lowestMatchIndex */
                                backLength = matchCandidateIdx.wrapping_sub(MAXu32(
                                    matchCandidateIdx.wrapping_sub(backLength as u32),
                                    lowestMatchIndex,
                                )) as usize;
                                currentSegmentLength = backLength + forwardPatternLength;

                                if (currentSegmentLength >= srcPatternLength)
                                    && (forwardPatternLength <= srcPatternLength)
                                {
                                    let newMatchIndex = matchCandidateIdx
                                        .wrapping_add(forwardPatternLength as u32)
                                        .wrapping_sub(srcPatternLength as u32);
                                    if LZ4HC_protectDictEnd(prefixIdx, newMatchIndex) != 0 {
                                        matchIndex = newMatchIndex;
                                    } else {
                                        matchIndex = prefixIdx;
                                    }
                                } else {
                                    let newMatchIndex =
                                        matchCandidateIdx.wrapping_sub(backLength as u32);
                                    if LZ4HC_protectDictEnd(prefixIdx, newMatchIndex) == 0 {
                                        matchIndex = prefixIdx;
                                    } else {
                                        matchIndex = newMatchIndex;
                                        if lookBackLength == 0 {
                                            let maxML =
                                                MINu2(currentSegmentLength, srcPatternLength);
                                            if (longest as usize) < maxML {
                                                if (pdiff(ip, prefixPtr) as usize
                                                    + prefixIdx as usize
                                                    - matchIndex as usize)
                                                    > LZ4_DISTANCE_MAX as usize
                                                {
                                                    break 'outer;
                                                }
                                                longest = maxML as c_int;
                                                offset =
                                                    ipIndex.wrapping_sub(matchIndex) as c_int;
                                            }
                                            {
                                                let distToNextPattern =
                                                    DELTANEXTU16_get(chainTable, matchIndex);
                                                if distToNextPattern > matchIndex {
                                                    break 'outer;
                                                }
                                                matchIndex =
                                                    matchIndex.wrapping_sub(distToNextPattern);
                                            }
                                        }
                                    }
                                }
                            }
                            continue 'outer;
                        }
                    }
                }
            }

            /* follow current chain */
            matchIndex = matchIndex.wrapping_sub(DELTANEXTU16_get(
                chainTable,
                matchIndex.wrapping_add(matchChainPos),
            ));
        }

        if dict == usingDictCtxHc && nbAttempts > 0 && withinStartDistance {
            let dictEndOffset: usize = (pdiff((*dictCtx).end, (*dictCtx).prefixStart) as usize)
                + (*dictCtx).dictLimit as usize;
            let mut dictMatchIndex: u32 = (*dictCtx).hashTable[LZ4HC_hashPtr(ip) as usize];
            matchIndex = dictMatchIndex
                .wrapping_add(lowestMatchIndex)
                .wrapping_sub(dictEndOffset as u32);
            while ipIndex.wrapping_sub(matchIndex) <= LZ4_DISTANCE_MAX && {
                let t = nbAttempts;
                nbAttempts -= 1;
                t != 0
            } {
                let matchPtr = cadd(
                    csub((*dictCtx).prefixStart, (*dictCtx).dictLimit as usize),
                    dictMatchIndex as usize,
                );

                if LZ4_read32(matchPtr) == pattern {
                    let mut mlt: c_int;
                    let back: c_int;
                    let mut vLimit = cadd(ip, dictEndOffset - dictMatchIndex as usize);
                    if vLimit > iHighLimit {
                        vLimit = iHighLimit;
                    }
                    mlt = (LZ4_count(cadd(ip, MINMATCH), cadd(matchPtr, MINMATCH), vLimit)
                        as c_int)
                        + MINMATCH as c_int;
                    back = if lookBackLength != 0 {
                        LZ4HC_countBack(ip, matchPtr, iLowLimit, (*dictCtx).prefixStart)
                    } else {
                        0
                    };
                    mlt -= back;
                    if mlt > longest {
                        longest = mlt;
                        offset = ipIndex.wrapping_sub(matchIndex) as c_int;
                        sBack = back;
                    }
                }

                {
                    let nextOffset =
                        DELTANEXTU16_get((*dictCtx).chainTable.as_ptr(), dictMatchIndex);
                    dictMatchIndex = dictMatchIndex.wrapping_sub(nextOffset);
                    matchIndex = matchIndex.wrapping_sub(nextOffset);
                }
            }
        }

        LZ4HC_match_t {
            len: longest,
            off: offset,
            back: sBack,
        }
    }
}

#[inline(always)]
unsafe fn LZ4HC_InsertAndFindBestMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iLimit: *const u8,
    maxNbAttempts: c_int,
    patternAnalysis: c_int,
    dict: i32,
) -> LZ4HC_match_t {
    unsafe {
        LZ4HC_InsertAndGetWiderMatch(
            hc4,
            ip,
            ip,
            iLimit,
            MINMATCH as c_int - 1,
            maxNbAttempts,
            patternAnalysis,
            0,
            dict,
            favorCompressionRatio,
        )
    }
}

/// Shared tail of the `_dest_overflow` handlers (fillOutput mode only).
unsafe fn LZ4HC_finishOverflow(
    ip: &mut *const u8,
    op: &mut *mut u8,
    anchor: &mut *const u8,
    optr: *mut u8,
    oend: *mut u8,
    ml: &mut c_int,
    moff: c_int,
) {
    unsafe {
        let ll = pdiff(*ip, *anchor) as usize;
        let ll_addbytes = (ll + 240) / 255;
        let ll_totalCost = 1 + ll_addbytes + ll;
        let maxLitPos = msub(oend, 3);
        *op = optr;
        if madd(*op, ll_totalCost) <= maxLitPos {
            let bytesLeftForMl = pdiff(maxLitPos, madd(*op, ll_totalCost)) as usize;
            let maxMlSize = MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
            if (*ml as usize) > maxMlSize {
                *ml = maxMlSize as c_int;
            }
            if pdiff(madd(oend, LASTLITERALS), madd(*op, ll_totalCost + 2)) - 1 + *ml as isize
                >= MFLIMIT as isize
            {
                LZ4HC_encodeSequence(ip, op, anchor, *ml, moff, notLimited, oend);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn LZ4HC_compress_hashChain(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dest: *mut c_char,
    srcSizePtr: *mut c_int,
    maxOutputSize: c_int,
    maxNbAttempts: c_int,
    limit: i32,
    dict: i32,
) -> c_int {
    unsafe {
        let inputSize = *srcSizePtr;
        let patternAnalysis: c_int = (maxNbAttempts > 128) as c_int;

        let mut ip: *const u8 = source as *const u8;
        let mut anchor: *const u8 = ip;
        let iend: *const u8 = cadd(ip, inputSize as usize);
        let mflimit: *const u8 = csub(iend, MFLIMIT);
        let matchlimit: *const u8 = csub(iend, LASTLITERALS);

        let mut optr: *mut u8 = dest as *mut u8;
        let mut op: *mut u8 = dest as *mut u8;
        let mut oend: *mut u8 = madd(op, maxOutputSize as usize);

        let mut start0: *const u8;
        let mut start2: *const u8 = core::ptr::null();
        let mut start3: *const u8 = core::ptr::null();
        let nomatch = LZ4HC_match_t { off: 0, len: 0, back: 0 };
        let mut m0 = nomatch;
        let mut m1 = nomatch;
        let mut m2 = nomatch;
        let mut m3 = nomatch;

        *srcSizePtr = 0;
        if limit == fillOutput {
            oend = msub(oend, LASTLITERALS);
        }

        macro_rules! dest_overflow {
            () => {
                if limit == fillOutput {
                    LZ4HC_finishOverflow(
                        &mut ip, &mut op, &mut anchor, optr, oend, &mut m1.len, m1.off,
                    );
                    true
                } else {
                    false
                }
            };
        }

        'lastlit: {
            if inputSize < LZ4_minLength {
                break 'lastlit;
            }

            /* Main Loop */
            'main: while ip <= mflimit {
                m1 = LZ4HC_InsertAndFindBestMatch(
                    ctx,
                    ip,
                    matchlimit,
                    maxNbAttempts,
                    patternAnalysis,
                    dict,
                );
                if m1.len < MINMATCH as c_int {
                    ip = cadd(ip, 1);
                    continue 'main;
                }

                start0 = ip;
                m0 = m1;

                let mut label: u32 = 2;
                'inner: loop {
                    if label == 2 {
                        /* _Search2: */
                        if cadd(ip, m1.len as usize) <= mflimit {
                            start2 = csub(cadd(ip, m1.len as usize), 2);
                            m2 = LZ4HC_InsertAndGetWiderMatch(
                                ctx,
                                start2,
                                ip,
                                matchlimit,
                                m1.len,
                                maxNbAttempts,
                                patternAnalysis,
                                0,
                                dict,
                                favorCompressionRatio,
                            );
                            start2 = coff(start2, m2.back as isize);
                        } else {
                            m2 = nomatch;
                        }

                        if m2.len <= m1.len {
                            optr = op;
                            if LZ4HC_encodeSequence(
                                &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                            ) != 0
                            {
                                if dest_overflow!() { break 'lastlit; } else { return 0; }
                            }
                            continue 'main;
                        }

                        if start0 < ip {
                            if start2 < cadd(ip, m0.len as usize) {
                                ip = start0;
                                m1 = m0;
                            }
                        }

                        if pdiff(start2, ip) < 3 {
                            ip = start2;
                            m1 = m2;
                            label = 2;
                            continue 'inner;
                        }
                        label = 3;
                    }

                    /* _Search3: */
                    if pdiff(start2, ip) < OPTIMAL_ML as isize {
                        let correction: c_int;
                        let mut new_ml = m1.len;
                        if new_ml > OPTIMAL_ML {
                            new_ml = OPTIMAL_ML;
                        }
                        if cadd(ip, new_ml as usize)
                            > csub(cadd(start2, m2.len as usize), MINMATCH)
                        {
                            new_ml = (pdiff(start2, ip) as c_int) + m2.len - MINMATCH as c_int;
                        }
                        correction = new_ml - (pdiff(start2, ip) as c_int);
                        if correction > 0 {
                            start2 = coff(start2, correction as isize);
                            m2.len -= correction;
                        }
                    }

                    if cadd(start2, m2.len as usize) <= mflimit {
                        start3 = csub(cadd(start2, m2.len as usize), 3);
                        m3 = LZ4HC_InsertAndGetWiderMatch(
                            ctx,
                            start3,
                            start2,
                            matchlimit,
                            m2.len,
                            maxNbAttempts,
                            patternAnalysis,
                            0,
                            dict,
                            favorCompressionRatio,
                        );
                        start3 = coff(start3, m3.back as isize);
                    } else {
                        m3 = nomatch;
                    }

                    if m3.len <= m2.len {
                        if start2 < cadd(ip, m1.len as usize) {
                            m1.len = pdiff(start2, ip) as c_int;
                        }
                        optr = op;
                        if LZ4HC_encodeSequence(
                            &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                        ) != 0
                        {
                            if dest_overflow!() { break 'lastlit; } else { return 0; }
                        }
                        ip = start2;
                        optr = op;
                        if LZ4HC_encodeSequence(
                            &mut ip, &mut op, &mut anchor, m2.len, m2.off, limit, oend,
                        ) != 0
                        {
                            m1 = m2;
                            if dest_overflow!() { break 'lastlit; } else { return 0; }
                        }
                        continue 'main;
                    }

                    if start3 < cadd(cadd(ip, m1.len as usize), 3) {
                        if start3 >= cadd(ip, m1.len as usize) {
                            if start2 < cadd(ip, m1.len as usize) {
                                let correction =
                                    pdiff(cadd(ip, m1.len as usize), start2) as c_int;
                                start2 = coff(start2, correction as isize);
                                m2.len -= correction;
                                if m2.len < MINMATCH as c_int {
                                    start2 = start3;
                                    m2 = m3;
                                }
                            }

                            optr = op;
                            if LZ4HC_encodeSequence(
                                &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                            ) != 0
                            {
                                if dest_overflow!() { break 'lastlit; } else { return 0; }
                            }
                            ip = start3;
                            m1 = m3;

                            start0 = start2;
                            m0 = m2;
                            label = 2;
                            continue 'inner;
                        }

                        start2 = start3;
                        m2 = m3;
                        label = 3;
                        continue 'inner;
                    }

                    /* 3 ascending matches : write the first one */
                    if start2 < cadd(ip, m1.len as usize) {
                        if pdiff(start2, ip) < OPTIMAL_ML as isize {
                            let correction: c_int;
                            if m1.len > OPTIMAL_ML {
                                m1.len = OPTIMAL_ML;
                            }
                            if cadd(ip, m1.len as usize)
                                > csub(cadd(start2, m2.len as usize), MINMATCH)
                            {
                                m1.len =
                                    (pdiff(start2, ip) as c_int) + m2.len - MINMATCH as c_int;
                            }
                            correction = m1.len - (pdiff(start2, ip) as c_int);
                            if correction > 0 {
                                start2 = coff(start2, correction as isize);
                                m2.len -= correction;
                            }
                        } else {
                            m1.len = pdiff(start2, ip) as c_int;
                        }
                    }
                    optr = op;
                    if LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                    ) != 0
                    {
                        if dest_overflow!() { break 'lastlit; } else { return 0; }
                    }

                    ip = start2;
                    m1 = m2;

                    start2 = start3;
                    m2 = m3;

                    label = 3;
                    continue 'inner;
                }
            }
        }

        /* _last_literals: */
        {
            let mut lastRunSize: usize = pdiff(iend, anchor) as usize;
            let mut llAdd: usize = (lastRunSize + 255 - RUN_MASK as usize) / 255;
            let totalSize: usize = 1 + llAdd + lastRunSize;
            if limit == fillOutput {
                oend = madd(oend, LASTLITERALS);
            }
            if limit != notLimited && (madd(op, totalSize) > oend) {
                if limit == limitedOutput {
                    return 0;
                }
                lastRunSize = (pdiff(oend, op) as usize) - 1;
                llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
                lastRunSize -= llAdd;
            }
            ip = cadd(anchor, lastRunSize);

            if lastRunSize >= RUN_MASK as usize {
                let mut accumulator = lastRunSize - RUN_MASK as usize;
                *op = (RUN_MASK << ML_BITS) as u8;
                op = madd(op, 1);
                while accumulator >= 255 {
                    *op = 255;
                    op = madd(op, 1);
                    accumulator -= 255;
                }
                *op = accumulator as u8;
                op = madd(op, 1);
            } else {
                *op = ((lastRunSize as u32) << ML_BITS) as u8;
                op = madd(op, 1);
            }
            LZ4_memcpy(op, anchor, lastRunSize);
            op = madd(op, lastRunSize);
        }

        *srcSizePtr = pdiff(ip, source as *const u8) as c_int;
        pdiff(op, dest as *const u8) as c_int
    }
}

/* ===== generic compression dispatch ===== */

unsafe fn LZ4HC_compress_generic_internal(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: i32,
    dict: i32,
) -> c_int {
    unsafe {
        if limit == fillOutput && dstCapacity < 1 {
            return 0;
        }
        if (*srcSizePtr as u32) > LZ4_MAX_INPUT_SIZE {
            return 0;
        }

        (*ctx).end = cadd((*ctx).end, *srcSizePtr as usize);
        {
            let cParam = LZ4HC_getCLevelParams(cLevel);
            let favor = if (*ctx).favorDecSpeed != 0 {
                favorDecompressionSpeed
            } else {
                favorCompressionRatio
            };
            let result: c_int;

            if cParam.strat == lz4mid {
                result = LZ4MID_compress(ctx, src, dst, srcSizePtr, dstCapacity, limit, dict);
            } else if cParam.strat == lz4hc {
                result = LZ4HC_compress_hashChain(
                    ctx,
                    src,
                    dst,
                    srcSizePtr,
                    dstCapacity,
                    cParam.nbSearches,
                    limit,
                    dict,
                );
            } else {
                result = LZ4HC_compress_optimal(
                    ctx,
                    src,
                    dst,
                    srcSizePtr,
                    dstCapacity,
                    cParam.nbSearches,
                    cParam.targetLength as usize,
                    limit,
                    (cLevel >= LZ4HC_CLEVEL_MAX) as c_int,
                    dict,
                    favor,
                );
            }
            if result <= 0 {
                (*ctx).dirty = 1;
            }
            result
        }
    }
}

unsafe fn LZ4HC_compress_generic_noDictCtx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: i32,
) -> c_int {
    unsafe {
        LZ4HC_compress_generic_internal(
            ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit, noDictCtx,
        )
    }
}

unsafe fn isStateCompatible(
    ctx1: *const LZ4HC_CCtx_internal,
    ctx2: *const LZ4HC_CCtx_internal,
) -> c_int {
    unsafe {
        let isMid1 = (LZ4HC_getCLevelParams((*ctx1).compressionLevel as c_int).strat == lz4mid)
            as c_int;
        let isMid2 = (LZ4HC_getCLevelParams((*ctx2).compressionLevel as c_int).strat == lz4mid)
            as c_int;
        (!((isMid1 ^ isMid2) != 0)) as c_int
    }
}

unsafe fn LZ4HC_compress_generic_dictCtx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: i32,
) -> c_int {
    unsafe {
        let position = (pdiff((*ctx).end, (*ctx).prefixStart) as usize)
            + ((*ctx).dictLimit.wrapping_sub((*ctx).lowLimit)) as usize;
        if position >= 64 * KB {
            (*ctx).dictCtx = core::ptr::null();
            LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
        } else if position == 0
            && *srcSizePtr > (4 * KB) as c_int
            && isStateCompatible(ctx, (*ctx).dictCtx) != 0
        {
            LZ4_memcpy(
                ctx as *mut u8,
                (*ctx).dictCtx as *const u8,
                core::mem::size_of::<LZ4HC_CCtx_internal>(),
            );
            LZ4HC_setExternalDict(ctx, src as *const u8);
            (*ctx).compressionLevel = cLevel as i16;
            LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
        } else {
            LZ4HC_compress_generic_internal(
                ctx,
                src,
                dst,
                srcSizePtr,
                dstCapacity,
                cLevel,
                limit,
                usingDictCtxHc,
            )
        }
    }
}

unsafe fn LZ4HC_compress_generic(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: i32,
) -> c_int {
    unsafe {
        if (*ctx).dictCtx.is_null() {
            LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
        } else {
            LZ4HC_compress_generic_dictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

fn LZ4_streamHC_t_alignment() -> usize {
    core::mem::align_of::<LZ4_streamHC_t>()
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
    unsafe {
        let ctx: *mut LZ4HC_CCtx_internal = state as *mut LZ4HC_CCtx_internal;
        if LZ4_isAligned(state as *const u8, LZ4_streamHC_t_alignment()) == 0 {
            return 0;
        }
        LZ4_resetStreamHC_fast(state as *mut LZ4_streamHC_t, compressionLevel);
        LZ4HC_init_internal(ctx, src as *const u8);
        if dstCapacity < LZ4_compressBound(srcSize) {
            LZ4HC_compress_generic(
                ctx,
                src,
                dst,
                &mut srcSize,
                dstCapacity,
                compressionLevel,
                limitedOutput,
            )
        } else {
            LZ4HC_compress_generic(
                ctx,
                src,
                dst,
                &mut srcSize,
                dstCapacity,
                compressionLevel,
                notLimited,
            )
        }
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
    unsafe {
        let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
        if ctx.is_null() {
            return 0;
        }
        LZ4_compress_HC_extStateHC_fastReset(
            state,
            src,
            dst,
            srcSize,
            dstCapacity,
            compressionLevel,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    compressionLevel: c_int,
) -> c_int {
    unsafe {
        let statePtr = ALLOC(core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
        if statePtr.is_null() {
            return 0;
        }
        let cSize = LZ4_compress_HC_extStateHC(
            statePtr as *mut c_void,
            src,
            dst,
            srcSize,
            dstCapacity,
            compressionLevel,
        );
        FREEMEM(statePtr as *mut c_void);
        cSize
    }
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
    unsafe {
        let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
        if ctx.is_null() {
            return 0;
        }
        LZ4HC_init_internal(&mut (*ctx).internal_donotuse, source as *const u8);
        LZ4_setCompressionLevel(ctx, cLevel);
        LZ4HC_compress_generic(
            &mut (*ctx).internal_donotuse,
            source,
            dest,
            sourceSizePtr,
            targetDestSize,
            cLevel,
            fillOutput,
        )
    }
}

/* ===== Streaming Functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamHC() -> *mut LZ4_streamHC_t {
    unsafe {
        let state = ALLOC_AND_ZERO(core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
        if state.is_null() {
            return core::ptr::null_mut();
        }
        LZ4_setCompressionLevel(state, LZ4HC_CLEVEL_DEFAULT);
        state
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamHC(LZ4_streamHCPtr: *mut LZ4_streamHC_t) -> c_int {
    unsafe {
        if LZ4_streamHCPtr.is_null() {
            return 0;
        }
        FREEMEM(LZ4_streamHCPtr as *mut c_void);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStreamHC(
    buffer: *mut c_void,
    size: usize,
) -> *mut LZ4_streamHC_t {
    unsafe {
        let LZ4_streamHCPtr = buffer as *mut LZ4_streamHC_t;
        if buffer.is_null() {
            return core::ptr::null_mut();
        }
        if size < core::mem::size_of::<LZ4_streamHC_t>() {
            return core::ptr::null_mut();
        }
        if LZ4_isAligned(buffer as *const u8, LZ4_streamHC_t_alignment()) == 0 {
            return core::ptr::null_mut();
        }
        MEM_INIT(
            &mut (*LZ4_streamHCPtr).internal_donotuse as *mut LZ4HC_CCtx_internal as *mut u8,
            0,
            core::mem::size_of::<LZ4HC_CCtx_internal>(),
        );
        LZ4_setCompressionLevel(LZ4_streamHCPtr, LZ4HC_CLEVEL_DEFAULT);
        LZ4_streamHCPtr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    compressionLevel: c_int,
) {
    unsafe {
        LZ4_initStreamHC(
            LZ4_streamHCPtr as *mut c_void,
            core::mem::size_of::<LZ4_streamHC_t>(),
        );
        LZ4_setCompressionLevel(LZ4_streamHCPtr, compressionLevel);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC_fast(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    compressionLevel: c_int,
) {
    unsafe {
        let s: *mut LZ4HC_CCtx_internal = &mut (*LZ4_streamHCPtr).internal_donotuse;
        if (*s).dirty != 0 {
            LZ4_initStreamHC(
                LZ4_streamHCPtr as *mut c_void,
                core::mem::size_of::<LZ4_streamHC_t>(),
            );
        } else {
            (*s).dictLimit = (*s)
                .dictLimit
                .wrapping_add(pdiff((*s).end, (*s).prefixStart) as u32);
            (*s).prefixStart = core::ptr::null();
            (*s).end = core::ptr::null();
            (*s).dictCtx = core::ptr::null();
        }
        LZ4_setCompressionLevel(LZ4_streamHCPtr, compressionLevel);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setCompressionLevel(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    mut compressionLevel: c_int,
) {
    unsafe {
        if compressionLevel < 1 {
            compressionLevel = LZ4HC_CLEVEL_DEFAULT;
        }
        if compressionLevel > LZ4HC_CLEVEL_MAX {
            compressionLevel = LZ4HC_CLEVEL_MAX;
        }
        (*LZ4_streamHCPtr).internal_donotuse.compressionLevel = compressionLevel as i16;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_favorDecompressionSpeed(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    favor: c_int,
) {
    unsafe {
        (*LZ4_streamHCPtr).internal_donotuse.favorDecSpeed = (favor != 0) as i8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    mut dictionary: *const c_char,
    mut dictSize: c_int,
) -> c_int {
    unsafe {
        let ctxPtr: *mut LZ4HC_CCtx_internal = &mut (*LZ4_streamHCPtr).internal_donotuse;
        let cp: cParams_t;
        if dictSize > (64 * KB) as c_int {
            dictionary = cadd(dictionary as *const u8, dictSize as usize - 64 * KB) as *const c_char;
            dictSize = (64 * KB) as c_int;
        }
        {
            let cLevel = (*ctxPtr).compressionLevel as c_int;
            LZ4_initStreamHC(
                LZ4_streamHCPtr as *mut c_void,
                core::mem::size_of::<LZ4_streamHC_t>(),
            );
            LZ4_setCompressionLevel(LZ4_streamHCPtr, cLevel);
            cp = LZ4HC_getCLevelParams(cLevel);
        }
        LZ4HC_init_internal(ctxPtr, dictionary as *const u8);
        (*ctxPtr).end = cadd(dictionary as *const u8, dictSize as usize);
        if cp.strat == lz4mid {
            LZ4MID_fillHTable(ctxPtr, dictionary as *const c_void, dictSize as usize);
        } else if dictSize >= LZ4HC_HASHSIZE {
            LZ4HC_Insert(ctxPtr, csub((*ctxPtr).end, 3));
        }
        dictSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_HC_dictionary(
    working_stream: *mut LZ4_streamHC_t,
    dictionary_stream: *const LZ4_streamHC_t,
) {
    unsafe {
        (*working_stream).internal_donotuse.dictCtx = if !dictionary_stream.is_null() {
            &(*dictionary_stream).internal_donotuse
        } else {
            core::ptr::null()
        };
    }
}

unsafe fn LZ4HC_setExternalDict(ctxPtr: *mut LZ4HC_CCtx_internal, newBlock: *const u8) {
    unsafe {
        if ((*ctxPtr).end >= cadd((*ctxPtr).prefixStart, 4))
            && (LZ4HC_getCLevelParams((*ctxPtr).compressionLevel as c_int).strat != lz4mid)
        {
            LZ4HC_Insert(ctxPtr, csub((*ctxPtr).end, 3));
        }

        (*ctxPtr).lowLimit = (*ctxPtr).dictLimit;
        (*ctxPtr).dictStart = (*ctxPtr).prefixStart;
        (*ctxPtr).dictLimit = (*ctxPtr)
            .dictLimit
            .wrapping_add(pdiff((*ctxPtr).end, (*ctxPtr).prefixStart) as u32);
        (*ctxPtr).prefixStart = newBlock;
        (*ctxPtr).end = newBlock;
        (*ctxPtr).nextToUpdate = (*ctxPtr).dictLimit;

        (*ctxPtr).dictCtx = core::ptr::null();
    }
}

unsafe fn LZ4_compressHC_continue_generic(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    limit: i32,
) -> c_int {
    unsafe {
        let ctxPtr: *mut LZ4HC_CCtx_internal = &mut (*LZ4_streamHCPtr).internal_donotuse;
        /* auto-init if forgotten */
        if (*ctxPtr).prefixStart.is_null() {
            LZ4HC_init_internal(ctxPtr, src as *const u8);
        }

        /* Check overflow */
        if (pdiff((*ctxPtr).end, (*ctxPtr).prefixStart) as usize) + (*ctxPtr).dictLimit as usize
            > (2usize << 30)
        {
            let mut dictSize = pdiff((*ctxPtr).end, (*ctxPtr).prefixStart) as usize;
            if dictSize > 64 * KB {
                dictSize = 64 * KB;
            }
            LZ4_loadDictHC(
                LZ4_streamHCPtr,
                csub((*ctxPtr).end, dictSize) as *const c_char,
                dictSize as c_int,
            );
        }

        /* Check if blocks follow each other */
        if (src as *const u8) != (*ctxPtr).end {
            LZ4HC_setExternalDict(ctxPtr, src as *const u8);
        }

        /* Check overlapping input/dictionary space */
        {
            let mut sourceEnd: *const u8 = cadd(src as *const u8, *srcSizePtr as usize);
            let dictBegin: *const u8 = (*ctxPtr).dictStart;
            let dictEnd: *const u8 = cadd(
                (*ctxPtr).dictStart,
                (*ctxPtr).dictLimit.wrapping_sub((*ctxPtr).lowLimit) as usize,
            );
            if (sourceEnd > dictBegin) && ((src as *const u8) < dictEnd) {
                if sourceEnd > dictEnd {
                    sourceEnd = dictEnd;
                }
                (*ctxPtr).lowLimit = (*ctxPtr)
                    .lowLimit
                    .wrapping_add(pdiff(sourceEnd, (*ctxPtr).dictStart) as u32);
                (*ctxPtr).dictStart =
                    cadd((*ctxPtr).dictStart, pdiff(sourceEnd, (*ctxPtr).dictStart) as usize);
                if (*ctxPtr).dictLimit.wrapping_sub((*ctxPtr).lowLimit)
                    < LZ4HC_HASHSIZE as u32
                {
                    (*ctxPtr).lowLimit = (*ctxPtr).dictLimit;
                    (*ctxPtr).dictStart = (*ctxPtr).prefixStart;
                }
            }
        }

        LZ4HC_compress_generic(
            ctxPtr,
            src,
            dst,
            srcSizePtr,
            dstCapacity,
            (*ctxPtr).compressionLevel as c_int,
            limit,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    mut srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    unsafe {
        if dstCapacity < LZ4_compressBound(srcSize) {
            LZ4_compressHC_continue_generic(
                LZ4_streamHCPtr,
                src,
                dst,
                &mut srcSize,
                dstCapacity,
                limitedOutput,
            )
        } else {
            LZ4_compressHC_continue_generic(
                LZ4_streamHCPtr,
                src,
                dst,
                &mut srcSize,
                dstCapacity,
                notLimited,
            )
        }
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
    unsafe {
        LZ4_compressHC_continue_generic(
            LZ4_streamHCPtr,
            src,
            dst,
            srcSizePtr,
            targetDestSize,
            fillOutput,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDictHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    safeBuffer: *mut c_char,
    mut dictSize: c_int,
) -> c_int {
    unsafe {
        let streamPtr: *mut LZ4HC_CCtx_internal = &mut (*LZ4_streamHCPtr).internal_donotuse;
        let prefixSize = pdiff((*streamPtr).end, (*streamPtr).prefixStart) as c_int;
        if dictSize > (64 * KB) as c_int {
            dictSize = (64 * KB) as c_int;
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
                csub((*streamPtr).end, dictSize as usize),
                dictSize as usize,
            );
        }
        {
            let endIndex: u32 = (pdiff((*streamPtr).end, (*streamPtr).prefixStart) as u32)
                .wrapping_add((*streamPtr).dictLimit);
            (*streamPtr).end = if safeBuffer.is_null() {
                core::ptr::null()
            } else {
                cadd(safeBuffer as *const u8, dictSize as usize)
            };
            (*streamPtr).prefixStart = safeBuffer as *const u8;
            (*streamPtr).dictLimit = endIndex.wrapping_sub(dictSize as u32);
            (*streamPtr).lowLimit = endIndex.wrapping_sub(dictSize as u32);
            (*streamPtr).dictStart = (*streamPtr).prefixStart;
            if (*streamPtr).nextToUpdate < (*streamPtr).dictLimit {
                (*streamPtr).nextToUpdate = (*streamPtr).dictLimit;
            }
        }
        dictSize
    }
}

/* ================================================
 *  LZ4 Optimal parser
 * ===============================================*/

#[repr(C)]
#[derive(Clone, Copy)]
struct LZ4HC_optimal_t {
    price: c_int,
    off: c_int,
    mlen: c_int,
    litlen: c_int,
}

#[inline(always)]
fn LZ4HC_literalsPrice(litlen: c_int) -> c_int {
    let mut price = litlen;
    if litlen >= RUN_MASK as c_int {
        price += 1 + ((litlen - RUN_MASK as c_int) / 255);
    }
    price
}

#[inline(always)]
fn LZ4HC_sequencePrice(litlen: c_int, mlen: c_int) -> c_int {
    let mut price = 1 + 2;
    price += LZ4HC_literalsPrice(litlen);
    if mlen >= (ML_MASK + MINMATCH as u32) as c_int {
        price += 1 + ((mlen - (ML_MASK + MINMATCH as u32) as c_int) / 255);
    }
    price
}

#[inline(always)]
unsafe fn LZ4HC_FindLongerMatch(
    ctx: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iHighLimit: *const u8,
    minLen: c_int,
    nbSearches: c_int,
    dict: i32,
    favorDecSpeed: i32,
) -> LZ4HC_match_t {
    unsafe {
        let match0 = LZ4HC_match_t { off: 0, len: 0, back: 0 };
        let mut md = LZ4HC_InsertAndGetWiderMatch(
            ctx,
            ip,
            ip,
            iHighLimit,
            minLen,
            nbSearches,
            1,
            1,
            dict,
            favorDecSpeed,
        );
        if md.len <= minLen {
            return match0;
        }
        if favorDecSpeed != 0 && (md.len > 18) && (md.len <= 36) {
            md.len = 18;
        }
        md
    }
}

const TRAILING_LITERALS: usize = 3;

#[allow(clippy::too_many_arguments)]
unsafe fn LZ4HC_compress_optimal(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    nbSearches: c_int,
    mut sufficient_len: usize,
    limit: i32,
    fullUpdate: c_int,
    dict: i32,
    favorDecSpeed: i32,
) -> c_int {
    unsafe {
        let mut retval: c_int = 0;
        let opt = ALLOC(
            core::mem::size_of::<LZ4HC_optimal_t>() * (LZ4_OPT_NUM + TRAILING_LITERALS),
        ) as *mut LZ4HC_optimal_t;

        let mut ip: *const u8 = source as *const u8;
        let mut anchor: *const u8 = ip;
        let iend: *const u8 = cadd(ip, *srcSizePtr as usize);
        let mflimit: *const u8 = csub(iend, MFLIMIT);
        let matchlimit: *const u8 = csub(iend, LASTLITERALS);
        let mut op: *mut u8 = dst as *mut u8;
        let mut opSaved: *mut u8 = dst as *mut u8;
        let mut oend: *mut u8 = madd(op, dstCapacity as usize);
        let mut ovml: c_int = MINMATCH as c_int;
        let mut ovoff: c_int = 0;

        if opt.is_null() {
            return retval;
        }

        *srcSizePtr = 0;
        if limit == fillOutput {
            oend = msub(oend, LASTLITERALS);
        }
        if sufficient_len >= LZ4_OPT_NUM {
            sufficient_len = LZ4_OPT_NUM - 1;
        }

        'lastlit: {
            /* Main Loop */
            'main: while ip <= mflimit {
                let llen: c_int = pdiff(ip, anchor) as c_int;
                let mut best_mlen: c_int;
                let mut best_off: c_int;
                let mut cur: c_int;
                let mut last_match_pos: c_int = 0;

                let firstMatch = LZ4HC_FindLongerMatch(
                    ctx,
                    ip,
                    matchlimit,
                    MINMATCH as c_int - 1,
                    nbSearches,
                    dict,
                    favorDecSpeed,
                );
                if firstMatch.len == 0 {
                    ip = cadd(ip, 1);
                    continue 'main;
                }

                if (firstMatch.len as usize) > sufficient_len {
                    let firstML = firstMatch.len;
                    opSaved = op;
                    if LZ4HC_encodeSequence(
                        &mut ip,
                        &mut op,
                        &mut anchor,
                        firstML,
                        firstMatch.off,
                        limit,
                        oend,
                    ) != 0
                    {
                        ovml = firstML;
                        ovoff = firstMatch.off;
                        /* _dest_overflow */
                        if limit == fillOutput {
                            let ll = pdiff(ip, anchor) as usize;
                            let ll_addbytes = (ll + 240) / 255;
                            let ll_totalCost = 1 + ll_addbytes + ll;
                            let maxLitPos = msub(oend, 3);
                            op = opSaved;
                            if madd(op, ll_totalCost) <= maxLitPos {
                                let bytesLeftForMl =
                                    pdiff(maxLitPos, madd(op, ll_totalCost)) as usize;
                                let maxMlSize =
                                    MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                                if (ovml as usize) > maxMlSize {
                                    ovml = maxMlSize as c_int;
                                }
                                if pdiff(madd(oend, LASTLITERALS), madd(op, ll_totalCost + 2)) - 1
                                    + ovml as isize
                                    >= MFLIMIT as isize
                                {
                                    LZ4HC_encodeSequence(
                                        &mut ip, &mut op, &mut anchor, ovml, ovoff, notLimited,
                                        oend,
                                    );
                                }
                            }
                            break 'lastlit;
                        }
                        retval = 0;
                        FREEMEM(opt as *mut c_void);
                        return retval;
                    }
                    continue 'main;
                }

                /* set prices for first positions (literals) */
                for rPos in 0..(MINMATCH as c_int) {
                    let cost = LZ4HC_literalsPrice(llen + rPos);
                    let o = opt.wrapping_add(rPos as usize);
                    (*o).mlen = 1;
                    (*o).off = 0;
                    (*o).litlen = llen + rPos;
                    (*o).price = cost;
                }
                /* set prices using initial match */
                {
                    let matchML = firstMatch.len;
                    let offset = firstMatch.off;
                    let mut mlen = MINMATCH as c_int;
                    while mlen <= matchML {
                        let cost = LZ4HC_sequencePrice(llen, mlen);
                        let o = opt.wrapping_add(mlen as usize);
                        (*o).mlen = mlen;
                        (*o).off = offset;
                        (*o).litlen = llen;
                        (*o).price = cost;
                        mlen += 1;
                    }
                }
                last_match_pos = firstMatch.len;
                {
                    let mut addLit = 1usize;
                    while addLit <= TRAILING_LITERALS {
                        let o = opt.wrapping_add(last_match_pos as usize + addLit);
                        (*o).mlen = 1;
                        (*o).off = 0;
                        (*o).litlen = addLit as c_int;
                        (*o).price = (*opt.wrapping_add(last_match_pos as usize)).price
                            + LZ4HC_literalsPrice(addLit as c_int);
                        addLit += 1;
                    }
                }

                /* check further positions */
                let mut goto_encode = false;
                cur = 1;
                best_mlen = 0;
                best_off = 0;
                while cur < last_match_pos {
                    let curPtr = cadd(ip, cur as usize);
                    let newMatch: LZ4HC_match_t;

                    if curPtr > mflimit {
                        break;
                    }
                    if fullUpdate != 0 {
                        if ((*opt.wrapping_add(cur as usize + 1)).price
                            <= (*opt.wrapping_add(cur as usize)).price)
                            && ((*opt.wrapping_add(cur as usize + MINMATCH)).price
                                < (*opt.wrapping_add(cur as usize)).price + 3)
                        {
                            cur += 1;
                            continue;
                        }
                    } else if (*opt.wrapping_add(cur as usize + 1)).price
                        <= (*opt.wrapping_add(cur as usize)).price
                    {
                        cur += 1;
                        continue;
                    }

                    if fullUpdate != 0 {
                        newMatch = LZ4HC_FindLongerMatch(
                            ctx,
                            curPtr,
                            matchlimit,
                            MINMATCH as c_int - 1,
                            nbSearches,
                            dict,
                            favorDecSpeed,
                        );
                    } else {
                        newMatch = LZ4HC_FindLongerMatch(
                            ctx,
                            curPtr,
                            matchlimit,
                            last_match_pos - cur,
                            nbSearches,
                            dict,
                            favorDecSpeed,
                        );
                    }
                    if newMatch.len == 0 {
                        cur += 1;
                        continue;
                    }

                    if ((newMatch.len as usize) > sufficient_len)
                        || (newMatch.len + cur >= LZ4_OPT_NUM as c_int)
                    {
                        best_mlen = newMatch.len;
                        best_off = newMatch.off;
                        last_match_pos = cur + 1;
                        goto_encode = true;
                        break;
                    }

                    /* before match : set price with literals at beginning */
                    {
                        let baseLitlen = (*opt.wrapping_add(cur as usize)).litlen;
                        let mut litlen = 1;
                        while litlen < MINMATCH as c_int {
                            let price = (*opt.wrapping_add(cur as usize)).price
                                - LZ4HC_literalsPrice(baseLitlen)
                                + LZ4HC_literalsPrice(baseLitlen + litlen);
                            let pos = cur + litlen;
                            if price < (*opt.wrapping_add(pos as usize)).price {
                                let o = opt.wrapping_add(pos as usize);
                                (*o).mlen = 1;
                                (*o).off = 0;
                                (*o).litlen = baseLitlen + litlen;
                                (*o).price = price;
                            }
                            litlen += 1;
                        }
                    }

                    /* set prices using match at position = cur */
                    {
                        let matchML = newMatch.len;
                        let mut ml = MINMATCH as c_int;

                        while ml <= matchML {
                            let pos = cur + ml;
                            let offset = newMatch.off;
                            let price: c_int;
                            let ll: c_int;
                            if (*opt.wrapping_add(cur as usize)).mlen == 1 {
                                ll = (*opt.wrapping_add(cur as usize)).litlen;
                                price = (if cur > ll {
                                    (*opt.wrapping_add((cur - ll) as usize)).price
                                } else {
                                    0
                                }) + LZ4HC_sequencePrice(ll, ml);
                            } else {
                                ll = 0;
                                price = (*opt.wrapping_add(cur as usize)).price
                                    + LZ4HC_sequencePrice(0, ml);
                            }

                            if pos > last_match_pos + TRAILING_LITERALS as c_int
                                || price <= (*opt.wrapping_add(pos as usize)).price - favorDecSpeed
                            {
                                if (ml == matchML) && (last_match_pos < pos) {
                                    last_match_pos = pos;
                                }
                                let o = opt.wrapping_add(pos as usize);
                                (*o).mlen = ml;
                                (*o).off = offset;
                                (*o).litlen = ll;
                                (*o).price = price;
                            }
                            ml += 1;
                        }
                    }
                    /* complete following positions with literals */
                    {
                        let mut addLit = 1usize;
                        while addLit <= TRAILING_LITERALS {
                            let o = opt.wrapping_add(last_match_pos as usize + addLit);
                            (*o).mlen = 1;
                            (*o).off = 0;
                            (*o).litlen = addLit as c_int;
                            (*o).price = (*opt.wrapping_add(last_match_pos as usize)).price
                                + LZ4HC_literalsPrice(addLit as c_int);
                            addLit += 1;
                        }
                    }
                    cur += 1;
                }

                if !goto_encode {
                    best_mlen = (*opt.wrapping_add(last_match_pos as usize)).mlen;
                    best_off = (*opt.wrapping_add(last_match_pos as usize)).off;
                    cur = last_match_pos - best_mlen;
                }

                /* encode: */
                {
                    let mut candidate_pos = cur;
                    let mut selected_matchLength = best_mlen;
                    let mut selected_offset = best_off;
                    loop {
                        let next_matchLength = (*opt.wrapping_add(candidate_pos as usize)).mlen;
                        let next_offset = (*opt.wrapping_add(candidate_pos as usize)).off;
                        (*opt.wrapping_add(candidate_pos as usize)).mlen = selected_matchLength;
                        (*opt.wrapping_add(candidate_pos as usize)).off = selected_offset;
                        selected_matchLength = next_matchLength;
                        selected_offset = next_offset;
                        if next_matchLength > candidate_pos {
                            break;
                        }
                        candidate_pos -= next_matchLength;
                    }
                }

                /* encode all recorded sequences in order */
                {
                    let mut rPos: c_int = 0;
                    while rPos < last_match_pos {
                        let ml = (*opt.wrapping_add(rPos as usize)).mlen;
                        let offset = (*opt.wrapping_add(rPos as usize)).off;
                        if ml == 1 {
                            ip = cadd(ip, 1);
                            rPos += 1;
                            continue;
                        }
                        rPos += ml;
                        opSaved = op;
                        if LZ4HC_encodeSequence(
                            &mut ip, &mut op, &mut anchor, ml, offset, limit, oend,
                        ) != 0
                        {
                            ovml = ml;
                            ovoff = offset;
                            /* _dest_overflow */
                            if limit == fillOutput {
                                let ll = pdiff(ip, anchor) as usize;
                                let ll_addbytes = (ll + 240) / 255;
                                let ll_totalCost = 1 + ll_addbytes + ll;
                                let maxLitPos = msub(oend, 3);
                                op = opSaved;
                                if madd(op, ll_totalCost) <= maxLitPos {
                                    let bytesLeftForMl =
                                        pdiff(maxLitPos, madd(op, ll_totalCost)) as usize;
                                    let maxMlSize = MINMATCH
                                        + (ML_MASK as usize - 1)
                                        + (bytesLeftForMl * 255);
                                    if (ovml as usize) > maxMlSize {
                                        ovml = maxMlSize as c_int;
                                    }
                                    if pdiff(
                                        madd(oend, LASTLITERALS),
                                        madd(op, ll_totalCost + 2),
                                    ) - 1
                                        + ovml as isize
                                        >= MFLIMIT as isize
                                    {
                                        LZ4HC_encodeSequence(
                                            &mut ip, &mut op, &mut anchor, ovml, ovoff,
                                            notLimited, oend,
                                        );
                                    }
                                }
                                break 'lastlit;
                            }
                            retval = 0;
                            FREEMEM(opt as *mut c_void);
                            return retval;
                        }
                    }
                }
            }
        }

        /* _last_literals: */
        {
            let mut lastRunSize: usize = pdiff(iend, anchor) as usize;
            let mut llAdd: usize = (lastRunSize + 255 - RUN_MASK as usize) / 255;
            let totalSize: usize = 1 + llAdd + lastRunSize;
            if limit == fillOutput {
                oend = madd(oend, LASTLITERALS);
            }
            if limit != notLimited && (madd(op, totalSize) > oend) {
                if limit == limitedOutput {
                    retval = 0;
                    FREEMEM(opt as *mut c_void);
                    return retval;
                }
                lastRunSize = (pdiff(oend, op) as usize) - 1;
                llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
                lastRunSize -= llAdd;
            }
            ip = cadd(anchor, lastRunSize);

            if lastRunSize >= RUN_MASK as usize {
                let mut accumulator = lastRunSize - RUN_MASK as usize;
                *op = (RUN_MASK << ML_BITS) as u8;
                op = madd(op, 1);
                while accumulator >= 255 {
                    *op = 255;
                    op = madd(op, 1);
                    accumulator -= 255;
                }
                *op = accumulator as u8;
                op = madd(op, 1);
            } else {
                *op = ((lastRunSize as u32) << ML_BITS) as u8;
                op = madd(op, 1);
            }
            LZ4_memcpy(op, anchor, lastRunSize);
            op = madd(op, lastRunSize);
        }

        *srcSizePtr = pdiff(ip, source as *const u8) as c_int;
        retval = pdiff(op, dst as *const u8) as c_int;

        FREEMEM(opt as *mut c_void);
        retval
    }
}

/* ===== Deprecated Functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, srcSize, LZ4_compressBound(srcSize), 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, srcSize, maxDstSize, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, srcSize, LZ4_compressBound(srcSize), cLevel) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
    cLevel: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, srcSize, maxDstSize, cLevel) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_HC_extStateHC(state, src, dst, srcSize, LZ4_compressBound(srcSize), 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDstSize, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_HC_extStateHC(state, src, dst, srcSize, LZ4_compressBound(srcSize), cLevel)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
    cLevel: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDstSize, cLevel) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_continue(ctx, src, dst, srcSize, LZ4_compressBound(srcSize)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_continue(ctx, src, dst, srcSize, maxDstSize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamStateHC(
    state: *mut c_void,
    inputBuffer: *mut c_char,
) -> c_int {
    unsafe {
        let hc4 = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
        if hc4.is_null() {
            return 1;
        }
        LZ4HC_init_internal(&mut (*hc4).internal_donotuse, inputBuffer as *const u8);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createHC(inputBuffer: *const c_char) -> *mut c_void {
    unsafe {
        let hc4 = LZ4_createStreamHC();
        if hc4.is_null() {
            return core::ptr::null_mut();
        }
        LZ4HC_init_internal(&mut (*hc4).internal_donotuse, inputBuffer as *const u8);
        hc4 as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeHC(LZ4HC_Data: *mut c_void) -> c_int {
    unsafe {
        if LZ4HC_Data.is_null() {
            return 0;
        }
        FREEMEM(LZ4HC_Data);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_continue(
    LZ4HC_Data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    unsafe {
        LZ4HC_compress_generic(
            &mut (*(LZ4HC_Data as *mut LZ4_streamHC_t)).internal_donotuse,
            src,
            dst,
            &mut srcSize,
            0,
            cLevel,
            notLimited,
        )
    }
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
    unsafe {
        LZ4HC_compress_generic(
            &mut (*(LZ4HC_Data as *mut LZ4_streamHC_t)).internal_donotuse,
            src,
            dst,
            &mut srcSize,
            dstCapacity,
            cLevel,
            limitedOutput,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBufferHC(LZ4HC_Data: *mut c_void) -> *mut c_char {
    unsafe {
        let s: *mut LZ4HC_CCtx_internal =
            &mut (*(LZ4HC_Data as *mut LZ4_streamHC_t)).internal_donotuse;
        let bufferStart = cadd(
            csub((*s).prefixStart, (*s).dictLimit as usize),
            (*s).lowLimit as usize,
        );
        LZ4_resetStreamHC_fast(
            LZ4HC_Data as *mut LZ4_streamHC_t,
            (*s).compressionLevel as c_int,
        );
        bufferStart as *mut c_char
    }
}
