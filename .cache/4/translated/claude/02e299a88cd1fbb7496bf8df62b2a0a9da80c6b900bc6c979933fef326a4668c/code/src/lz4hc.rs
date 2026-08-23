//! Translation of `c_src/src/lz4hc.c`

use crate::common::*;
use crate::lz4::*;
use core::ffi::{c_char, c_int, c_void};

/*===   Constants   ===*/
pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;

pub const LZ4HC_DICTIONARY_LOGSIZE: u32 = 16;
pub const LZ4HC_MAXD: usize = 1usize << LZ4HC_DICTIONARY_LOGSIZE;
pub const LZ4HC_HASH_LOG: u32 = 15;
pub const LZ4HC_HASHTABLESIZE: usize = 1usize << LZ4HC_HASH_LOG;

pub const OPTIMAL_ML: c_int = ((ML_MASK - 1) + MINMATCH as u32) as c_int;
pub const LZ4_OPT_NUM: usize = 1usize << 12;

pub const LZ4HC_HASHSIZE: usize = 4;
pub const LZ4MID_HASHSIZE: usize = 8;
pub const LZ4MID_HASHLOG: u32 = LZ4HC_HASH_LOG - 1;
pub const LZ4MID_HASHTABLESIZE: usize = 1usize << LZ4MID_HASHLOG;

/// `sizeof(LZ4_streamHC_t)` == `LZ4_STREAMHC_MINSIZE`
pub const SIZEOF_LZ4_STREAMHC_T: usize = 262200;
/// `sizeof(LZ4HC_CCtx_internal)`
pub const SIZEOF_LZ4HC_CCTX_INTERNAL: usize = 262192;

#[repr(C)]
pub struct LZ4HC_CCtx_internal {
    pub hashTable: [u32; LZ4HC_HASHTABLESIZE],
    pub chainTable: [u16; LZ4HC_MAXD],
    /// next block here to continue on current prefix
    pub end: *const u8,
    /// Indexes relative to this position
    pub prefixStart: *const u8,
    /// alternate reference for extDict
    pub dictStart: *const u8,
    /// below that point, need extDict
    pub dictLimit: u32,
    /// below that point, no more history
    pub lowLimit: u32,
    /// index from which to continue dictionary update
    pub nextToUpdate: u32,
    pub compressionLevel: i16,
    pub favorDecSpeed: i8,
    pub dirty: i8,
    pub dictCtx: *const LZ4HC_CCtx_internal,
}

pub type LZ4_streamHC_t = LZ4HC_CCtx_internal;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4HC_match_t {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

/*===   Enums   ===*/
const noDictCtx: i32 = 0;
const usingDictCtxHc: i32 = 1;

/* lz4hc_strat_e */
const lz4mid: i32 = 0;
const lz4hc: i32 = 1;
const lz4opt: i32 = 2;

/* HCfavor_e */
const favorCompressionRatio: i32 = 0;
const favorDecompressionSpeed: i32 = 1;

/* repeat_state_e */
const rep_untested: i32 = 0;
const rep_not: i32 = 1;
const rep_confirmed: i32 = 2;

#[derive(Clone, Copy)]
struct cParams_t {
    strat: i32,
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

fn LZ4HC_getCLevelParams(cLevel: c_int) -> cParams_t {
    let mut cLevel = cLevel;
    if cLevel < 1 {
        cLevel = LZ4HC_CLEVEL_DEFAULT;
    }
    cLevel = if LZ4HC_CLEVEL_MAX < cLevel {
        LZ4HC_CLEVEL_MAX
    } else {
        cLevel
    };
    k_clTable[cLevel as usize]
}

/*===   Hashing   ===*/
#[inline(always)]
fn HASH_FUNCTION(i: u32) -> u32 {
    (i.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - LZ4HC_HASH_LOG)
}

#[inline(always)]
unsafe fn LZ4HC_hashPtr(ptr: *const u8) -> u32 {
    HASH_FUNCTION(LZ4_read32(ptr))
}

#[inline(always)]
fn LZ4MID_hash4(v: u32) -> u32 {
    (v.wrapping_mul(2654435761u32)) >> (32 - LZ4MID_HASHLOG)
}

#[inline(always)]
unsafe fn LZ4MID_hash4Ptr(ptr: *const u8) -> u32 {
    LZ4MID_hash4(LZ4_read32(ptr))
}

#[inline(always)]
fn LZ4MID_hash7(v: u64) -> u32 {
    (((v << (64 - 56)).wrapping_mul(58295818150454627u64)) >> (64 - LZ4MID_HASHLOG)) as u32
}

#[inline(always)]
unsafe fn LZ4MID_hash8Ptr(ptr: *const u8) -> u32 {
    LZ4MID_hash7(LZ4_readLE64(ptr))
}

/*===   Count match length   ===*/
#[inline(always)]
fn LZ4HC_NbCommonBytes32(val: u32) -> u32 {
    if LZ4_isLittleEndian() {
        val.leading_zeros() >> 3
    } else {
        val.trailing_zeros() >> 3
    }
}

/// `LZ4HC_countBack()` : @return : negative value, nb of common bytes before ip/match
#[inline(always)]
unsafe fn LZ4HC_countBack(
    ip: *const u8,
    match_: *const u8,
    iMin: *const u8,
    mMin: *const u8,
) -> c_int {
    let mut back: c_int = 0;
    let a = (iMin as isize) - (ip as isize);
    let b = (mMin as isize) - (match_ as isize);
    let min: c_int = (if a > b { a } else { b }) as c_int;

    while (back - min) > 3 {
        let v = LZ4_read32(ip.wrapping_offset((back - 4) as isize))
            ^ LZ4_read32(match_.wrapping_offset((back - 4) as isize));
        if v != 0 {
            return back - (LZ4HC_NbCommonBytes32(v) as c_int);
        } else {
            back -= 4;
        }
    }
    while (back > min)
        && (*ip.wrapping_offset((back - 1) as isize) == *match_.wrapping_offset((back - 1) as isize))
    {
        back -= 1;
    }
    back
}

/*===   Chain table updates   ===*/
#[inline(always)]
unsafe fn DELTANEXTU16_get(table: *const u16, pos: u32) -> u32 {
    *table.add((pos as u16) as usize) as u32
}

#[inline(always)]
unsafe fn DELTANEXTU16_set(table: *mut u16, pos: u32, v: u16) {
    *table.add((pos as u16) as usize) = v;
}

/**************************************
*  Init
**************************************/
unsafe fn LZ4HC_clearTables(hc4: *mut LZ4HC_CCtx_internal) {
    mem_init(
        (*hc4).hashTable.as_mut_ptr() as *mut u8,
        0,
        LZ4HC_HASHTABLESIZE * 4,
    );
    mem_init(
        (*hc4).chainTable.as_mut_ptr() as *mut u8,
        0xFF,
        LZ4HC_MAXD * 2,
    );
}

unsafe fn LZ4HC_init_internal(hc4: *mut LZ4HC_CCtx_internal, start: *const u8) {
    let bufferSize: usize = (*hc4).end as usize - (*hc4).prefixStart as usize;
    let mut newStartingOffset: usize = bufferSize + (*hc4).dictLimit as usize;
    if newStartingOffset > (1usize << 30) {
        LZ4HC_clearTables(hc4);
        newStartingOffset = 0;
    }
    newStartingOffset += 64 * 1024;
    (*hc4).nextToUpdate = newStartingOffset as u32;
    (*hc4).prefixStart = start;
    (*hc4).end = start;
    (*hc4).dictStart = start;
    (*hc4).dictLimit = newStartingOffset as u32;
    (*hc4).lowLimit = newStartingOffset as u32;
}

/**************************************
*  Encode
**************************************/
/// `LZ4HC_encodeSequence()` : @return : 0 if ok, 1 if buffer issue detected
#[inline(always)]
unsafe fn LZ4HC_encodeSequence(
    _ip: &mut *const u8,
    _op: &mut *mut u8,
    _anchor: &mut *const u8,
    matchLength: c_int,
    offset: c_int,
    limit: i32,
    oend: *mut u8,
) -> c_int {
    let mut length: usize;
    let token: *mut u8 = *_op;
    *_op = (*_op).wrapping_add(1);

    /* Encode Literal length */
    length = (*_ip) as usize - (*_anchor) as usize;
    /* Check output limit */
    if limit != notLimited
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
            *_op = (*_op).wrapping_add(1);
            len -= 255;
        }
        **_op = len as u8;
        *_op = (*_op).wrapping_add(1);
    } else {
        *token = ((length << ML_BITS) & 0xFF) as u8;
    }

    /* Copy Literals */
    LZ4_wildCopy8(*_op, *_anchor, (*_op).wrapping_add(length));
    *_op = (*_op).wrapping_add(length);

    /* Encode Offset */
    LZ4_writeLE16(*_op, offset as u16);
    *_op = (*_op).wrapping_add(2);

    /* Encode MatchLength */
    length = (matchLength as usize) - MINMATCH;
    if limit != notLimited
        && ((*_op)
            .wrapping_add(length / 255)
            .wrapping_add(1 + LASTLITERALS)
            > oend)
    {
        return 1;
    }
    if length >= ML_MASK as usize {
        *token = (*token).wrapping_add(ML_MASK as u8);
        length -= ML_MASK as usize;
        while length >= 510 {
            **_op = 255;
            *_op = (*_op).wrapping_add(1);
            **_op = 255;
            *_op = (*_op).wrapping_add(1);
            length -= 510;
        }
        if length >= 255 {
            length -= 255;
            **_op = 255;
            *_op = (*_op).wrapping_add(1);
        }
        **_op = length as u8;
        *_op = (*_op).wrapping_add(1);
    } else {
        *token = (*token).wrapping_add((length & 0xFF) as u8);
    }

    /* Prepare next loop */
    *_ip = (*_ip).wrapping_add(matchLength as usize);
    *_anchor = *_ip;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4HC_searchExtDict(
    ip: *const u8,
    ipIndex: u32,
    iLowLimit: *const u8,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
    currentBestML: c_int,
    nbAttempts: c_int,
) -> LZ4HC_match_t {
    let mut nbAttempts = nbAttempts;
    let mut currentBestML = currentBestML;
    let lDictEndIndex: usize = ((*dictCtx).end as usize - (*dictCtx).prefixStart as usize)
        + (*dictCtx).dictLimit as usize;
    let mut lDictMatchIndex: u32 = (*dictCtx).hashTable[LZ4HC_hashPtr(ip) as usize];
    let mut matchIndex: u32 = lDictMatchIndex
        .wrapping_add(gDictEndIndex)
        .wrapping_sub(lDictEndIndex as u32);
    let mut offset: c_int = 0;
    let mut sBack: c_int = 0;

    while ipIndex.wrapping_sub(matchIndex) <= LZ4_DISTANCE_MAX && {
        let t = nbAttempts != 0;
        nbAttempts = nbAttempts.wrapping_sub(1);
        t
    } {
        let matchPtr: *const u8 = (*dictCtx)
            .prefixStart
            .wrapping_sub((*dictCtx).dictLimit as usize)
            .wrapping_add(lDictMatchIndex as usize);

        if LZ4_read32(matchPtr) == LZ4_read32(ip) {
            let mut mlt: c_int;
            let back: c_int;
            let mut vLimit: *const u8 =
                ip.wrapping_add(lDictEndIndex - lDictMatchIndex as usize);
            if vLimit > iHighLimit {
                vLimit = iHighLimit;
            }
            mlt = LZ4_count(
                ip.wrapping_add(MINMATCH),
                matchPtr.wrapping_add(MINMATCH),
                vLimit,
            ) as c_int
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
            let nextOffset: u32 =
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

unsafe fn LZ4MID_searchExtDict(
    ip: *const u8,
    ipIndex: u32,
    iHighLimit: *const u8,
    dictCtx: *const LZ4HC_CCtx_internal,
    gDictEndIndex: u32,
) -> LZ4HC_match_t {
    let lDictEndIndex: usize = ((*dictCtx).end as usize - (*dictCtx).prefixStart as usize)
        + (*dictCtx).dictLimit as usize;
    let hash4Table: *const u32 = (*dictCtx).hashTable.as_ptr();
    let hash8Table: *const u32 = hash4Table.add(LZ4MID_HASHTABLESIZE);

    /* search long match first */
    {
        let l8DictMatchIndex: u32 = *hash8Table.add(LZ4MID_hash8Ptr(ip) as usize);
        let m8Index: u32 = l8DictMatchIndex
            .wrapping_add(gDictEndIndex)
            .wrapping_sub(lDictEndIndex as u32);
        if ipIndex.wrapping_sub(m8Index) <= LZ4_DISTANCE_MAX {
            let matchPtr: *const u8 = (*dictCtx)
                .prefixStart
                .wrapping_sub((*dictCtx).dictLimit as usize)
                .wrapping_add(l8DictMatchIndex as usize);
            let safeLen: usize = {
                let a = lDictEndIndex.wrapping_sub(l8DictMatchIndex as usize);
                let b = iHighLimit as usize - ip as usize;
                if a < b {
                    a
                } else {
                    b
                }
            };
            let mlt = LZ4_count(ip, matchPtr, ip.wrapping_add(safeLen)) as c_int;
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
        let l4DictMatchIndex: u32 = *hash4Table.add(LZ4MID_hash4Ptr(ip) as usize);
        let m4Index: u32 = l4DictMatchIndex
            .wrapping_add(gDictEndIndex)
            .wrapping_sub(lDictEndIndex as u32);
        if ipIndex.wrapping_sub(m4Index) <= LZ4_DISTANCE_MAX {
            let matchPtr: *const u8 = (*dictCtx)
                .prefixStart
                .wrapping_sub((*dictCtx).dictLimit as usize)
                .wrapping_add(l4DictMatchIndex as usize);
            let safeLen: usize = {
                let a = lDictEndIndex.wrapping_sub(l4DictMatchIndex as usize);
                let b = iHighLimit as usize - ip as usize;
                if a < b {
                    a
                } else {
                    b
                }
            };
            let mlt = LZ4_count(ip, matchPtr, ip.wrapping_add(safeLen)) as c_int;
            if mlt >= MINMATCH as c_int {
                return LZ4HC_match_t {
                    len: mlt,
                    off: ipIndex.wrapping_sub(m4Index) as c_int,
                    back: 0,
                };
            }
        }
    }

    LZ4HC_match_t {
        off: 0,
        len: 0,
        back: 0,
    }
}

/**************************************
*  Mid Compression (level 2)
**************************************/
#[inline(always)]
unsafe fn LZ4MID_addPosition(hTable: *mut u32, hValue: u32, index: u32) {
    *hTable.add(hValue as usize) = index;
}

unsafe fn LZ4MID_fillHTable(cctx: *mut LZ4HC_CCtx_internal, dict: *const c_void, size: usize) {
    let hash4Table: *mut u32 = (*cctx).hashTable.as_mut_ptr();
    let hash8Table: *mut u32 = hash4Table.add(LZ4MID_HASHTABLESIZE);
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
        LZ4MID_addPosition(
            hash4Table,
            LZ4MID_hash4Ptr(
                prefixPtr
                    .wrapping_add(idx as usize)
                    .wrapping_sub(prefixIdx as usize),
            ),
            idx,
        );
        LZ4MID_addPosition(
            hash8Table,
            LZ4MID_hash8Ptr(
                prefixPtr
                    .wrapping_add(idx as usize)
                    .wrapping_add(1)
                    .wrapping_sub(prefixIdx as usize),
            ),
            idx + 1,
        );
        idx = idx.wrapping_add(3);
    }

    idx = if size > 32 * 1024 + LZ4MID_HASHSIZE {
        target.wrapping_sub(32 * 1024)
    } else {
        (*cctx).nextToUpdate
    };
    while idx < target {
        LZ4MID_addPosition(
            hash8Table,
            LZ4MID_hash8Ptr(
                prefixPtr
                    .wrapping_add(idx as usize)
                    .wrapping_sub(prefixIdx as usize),
            ),
            idx,
        );
        idx = idx.wrapping_add(1);
    }

    (*cctx).nextToUpdate = target;
}

unsafe fn select_searchDict_function(
    dictCtx: *const LZ4HC_CCtx_internal,
) -> Option<LZ4MID_searchIntoDict_f> {
    if dictCtx.is_null() {
        return None;
    }
    if LZ4HC_getCLevelParams((*dictCtx).compressionLevel as c_int).strat == lz4mid {
        return Some(LZ4MID_searchExtDict as LZ4MID_searchIntoDict_f);
    }
    Some(LZ4MID_searchHCDict as LZ4MID_searchIntoDict_f)
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
    let hash4Table: *mut u32 = (*ctx).hashTable.as_mut_ptr();
    let hash8Table: *mut u32 = hash4Table.add(LZ4MID_HASHTABLESIZE);
    let mut ip: *const u8 = src as *const u8;
    let mut anchor: *const u8 = ip;
    let iend: *const u8 = ip.wrapping_add(*srcSizePtr as usize);
    let mflimit: *const u8 = iend.wrapping_sub(MFLIMIT);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);
    let ilimit: *const u8 = iend.wrapping_sub(LZ4MID_HASHSIZE);
    let mut op: *mut u8 = dst as *mut u8;
    let mut oend: *mut u8 = op.wrapping_add(maxOutputSize as usize);

    let prefixPtr: *const u8 = (*ctx).prefixStart;
    let prefixIdx: u32 = (*ctx).dictLimit;
    let ilimitIdx: u32 = ((ilimit as usize - prefixPtr as usize) as u32).wrapping_add(prefixIdx);
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
    if *srcSizePtr > LZ4_MAX_INPUT_SIZE {
        return 0;
    }
    if limit == fillOutput {
        oend = oend.wrapping_sub(LASTLITERALS);
    }

    let mut overflow = false;

    'body: {
        if *srcSizePtr < LZ4_minLength {
            break 'body; /* goto _lz4mid_last_literals */
        }

        /* main loop */
        'main: while ip <= mflimit {
            let ipIndex: u32 = ((ip as usize - prefixPtr as usize) as u32).wrapping_add(prefixIdx);
            let mut found = false;

            /* search long match */
            {
                let h8: u32 = LZ4MID_hash8Ptr(ip);
                let pos8: u32 = *hash8Table.add(h8 as usize);
                LZ4MID_addPosition(hash8Table, h8, ipIndex);
                if ipIndex.wrapping_sub(pos8) <= LZ4_DISTANCE_MAX {
                    if pos8 >= prefixIdx {
                        let matchPtr: *const u8 = prefixPtr
                            .wrapping_add(pos8 as usize)
                            .wrapping_sub(prefixIdx as usize);
                        matchLength = LZ4_count(ip, matchPtr, matchlimit);
                        if matchLength >= MINMATCH as u32 {
                            matchDistance = ipIndex.wrapping_sub(pos8);
                            found = true;
                        }
                    } else {
                        if pos8 >= dictIdx {
                            let matchPtr: *const u8 =
                                dictStart.wrapping_add((pos8.wrapping_sub(dictIdx)) as usize);
                            let safeLen: usize = {
                                let a = prefixIdx.wrapping_sub(pos8) as usize;
                                let b = matchlimit as usize - ip as usize;
                                if a < b {
                                    a
                                } else {
                                    b
                                }
                            };
                            matchLength = LZ4_count(ip, matchPtr, ip.wrapping_add(safeLen));
                            if matchLength >= MINMATCH as u32 {
                                matchDistance = ipIndex.wrapping_sub(pos8);
                                found = true;
                            }
                        }
                    }
                }
            }

            /* search short match */
            if !found {
                let h4: u32 = LZ4MID_hash4Ptr(ip);
                let pos4: u32 = *hash4Table.add(h4 as usize);
                LZ4MID_addPosition(hash4Table, h4, ipIndex);
                if ipIndex.wrapping_sub(pos4) <= LZ4_DISTANCE_MAX {
                    if pos4 >= prefixIdx {
                        let matchPtr: *const u8 = prefixPtr
                            .wrapping_add((pos4.wrapping_sub(prefixIdx)) as usize);
                        matchLength = LZ4_count(ip, matchPtr, matchlimit);
                        if matchLength >= MINMATCH as u32 {
                            /* short match found, let's just check ip+1 for longer */
                            let h8: u32 = LZ4MID_hash8Ptr(ip.wrapping_add(1));
                            let pos8: u32 = *hash8Table.add(h8 as usize);
                            let m2Distance: u32 = ipIndex.wrapping_add(1).wrapping_sub(pos8);
                            matchDistance = ipIndex.wrapping_sub(pos4);
                            if m2Distance <= LZ4_DISTANCE_MAX
                                && pos8 >= prefixIdx
                                && (ip < mflimit)
                            {
                                let m2Ptr: *const u8 = prefixPtr
                                    .wrapping_add((pos8.wrapping_sub(prefixIdx)) as usize);
                                let ml2 = LZ4_count(ip.wrapping_add(1), m2Ptr, matchlimit);
                                if ml2 > matchLength {
                                    LZ4MID_addPosition(hash8Table, h8, ipIndex + 1);
                                    ip = ip.wrapping_add(1);
                                    matchLength = ml2;
                                    matchDistance = m2Distance;
                                }
                            }
                            found = true;
                        }
                    } else {
                        if pos4 >= dictIdx {
                            let matchPtr: *const u8 =
                                dictStart.wrapping_add((pos4.wrapping_sub(dictIdx)) as usize);
                            let safeLen: usize = {
                                let a = prefixIdx.wrapping_sub(pos4) as usize;
                                let b = matchlimit as usize - ip as usize;
                                if a < b {
                                    a
                                } else {
                                    b
                                }
                            };
                            matchLength = LZ4_count(ip, matchPtr, ip.wrapping_add(safeLen));
                            if matchLength >= MINMATCH as u32 {
                                matchDistance = ipIndex.wrapping_sub(pos4);
                                found = true;
                            }
                        }
                    }
                }
            }

            /* no match found in prefix */
            if !found {
                if (dict == usingDictCtxHc)
                    && (ipIndex.wrapping_sub(gDictEndIndex) < LZ4_DISTANCE_MAX - 8)
                {
                    let f = searchIntoDict.unwrap();
                    let dMatch = f(ip, ipIndex, matchlimit, (*ctx).dictCtx, gDictEndIndex);
                    if dMatch.len >= MINMATCH as c_int {
                        matchLength = dMatch.len as u32;
                        matchDistance = dMatch.off as u32;
                        found = true;
                    }
                }
            }

            if !found {
                /* no match found */
                ip = ip
                    .wrapping_add(1)
                    .wrapping_add(((ip as usize - anchor as usize) >> 9) as usize);
                continue 'main;
            }

            /* _lz4mid_encode_sequence: */
            /* catch back */
            while ((ip > anchor)
                & (((ip as usize - prefixPtr as usize) as u32) > matchDistance))
                && (*ip.wrapping_sub(1)
                    == *ip.wrapping_offset(-(matchDistance as isize) - 1))
            {
                ip = ip.wrapping_sub(1);
                matchLength += 1;
            }

            /* fill table with beginning of match */
            LZ4MID_addPosition(
                hash8Table,
                LZ4MID_hash8Ptr(ip.wrapping_add(1)),
                ipIndex.wrapping_add(1),
            );
            LZ4MID_addPosition(
                hash8Table,
                LZ4MID_hash8Ptr(ip.wrapping_add(2)),
                ipIndex.wrapping_add(2),
            );
            LZ4MID_addPosition(
                hash4Table,
                LZ4MID_hash4Ptr(ip.wrapping_add(1)),
                ipIndex.wrapping_add(1),
            );

            /* encode */
            {
                let saved_op: *mut u8 = op;
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
                    overflow = true;
                    break 'body; /* goto _lz4mid_dest_overflow */
                }
            }

            /* fill table with end of match */
            {
                let endMatchIdx: u32 =
                    ((ip as usize - prefixPtr as usize) as u32).wrapping_add(prefixIdx);
                let pos_m2: u32 = endMatchIdx.wrapping_sub(2);
                if pos_m2 < ilimitIdx {
                    if (ip as usize - prefixPtr as usize) > 5 {
                        LZ4MID_addPosition(
                            hash8Table,
                            LZ4MID_hash8Ptr(ip.wrapping_sub(5)),
                            endMatchIdx.wrapping_sub(5),
                        );
                    }
                    LZ4MID_addPosition(
                        hash8Table,
                        LZ4MID_hash8Ptr(ip.wrapping_sub(3)),
                        endMatchIdx.wrapping_sub(3),
                    );
                    LZ4MID_addPosition(
                        hash8Table,
                        LZ4MID_hash8Ptr(ip.wrapping_sub(2)),
                        endMatchIdx.wrapping_sub(2),
                    );
                    LZ4MID_addPosition(
                        hash4Table,
                        LZ4MID_hash4Ptr(ip.wrapping_sub(2)),
                        endMatchIdx.wrapping_sub(2),
                    );
                    LZ4MID_addPosition(
                        hash4Table,
                        LZ4MID_hash4Ptr(ip.wrapping_sub(1)),
                        endMatchIdx.wrapping_sub(1),
                    );
                }
            }
        }
    }

    if overflow {
        /* _lz4mid_dest_overflow: */
        if limit == fillOutput {
            let ll: usize = ip as usize - anchor as usize;
            let ll_addbytes: usize = (ll + 240) / 255;
            let ll_totalCost: usize = 1 + ll_addbytes + ll;
            let maxLitPos: *mut u8 = oend.wrapping_sub(3);
            if op.wrapping_add(ll_totalCost) <= maxLitPos {
                let bytesLeftForMl: usize =
                    maxLitPos as usize - (op.wrapping_add(ll_totalCost)) as usize;
                let maxMlSize: usize =
                    MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                if (matchLength as usize) > maxMlSize {
                    matchLength = maxMlSize as u32;
                }
                if ((oend.wrapping_add(LASTLITERALS)) as usize)
                    .wrapping_sub((op.wrapping_add(ll_totalCost + 2)) as usize)
                    .wrapping_sub(1)
                    .wrapping_add(matchLength as usize)
                    >= MFLIMIT
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
        /* fall through to last literals */
        } else {
            /* compression failed */
            return 0;
        }
    }

    /* _lz4mid_last_literals: Encode Last Literals */
    {
        let mut lastRunSize: usize = iend as usize - anchor as usize;
        let mut llAdd: usize = (lastRunSize + 255 - RUN_MASK as usize) / 255;
        let totalSize: usize = 1 + llAdd + lastRunSize;
        if limit == fillOutput {
            oend = oend.wrapping_add(LASTLITERALS);
        }
        if limit != notLimited && (op.wrapping_add(totalSize) > oend) {
            if limit == limitedOutput {
                return 0;
            }
            lastRunSize = (oend as usize - op as usize) - 1;
            llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
            lastRunSize -= llAdd;
        }
        ip = anchor.wrapping_add(lastRunSize);

        if lastRunSize >= RUN_MASK as usize {
            let mut accumulator = lastRunSize - RUN_MASK as usize;
            *op = (RUN_MASK << ML_BITS) as u8;
            op = op.wrapping_add(1);
            while accumulator >= 255 {
                *op = 255;
                op = op.wrapping_add(1);
                accumulator -= 255;
            }
            *op = accumulator as u8;
            op = op.wrapping_add(1);
        } else {
            *op = ((lastRunSize << ML_BITS) & 0xFF) as u8;
            op = op.wrapping_add(1);
        }
        mem_copy(op, anchor, lastRunSize);
        op = op.wrapping_add(lastRunSize);
    }

    /* End */
    *srcSizePtr = (ip as usize - src as usize) as c_int;
    (op as usize - dst as usize) as c_int
}

/**************************************
*  HC Compression - Search
**************************************/

/// Update chains up to ip (excluded)
#[inline(always)]
unsafe fn LZ4HC_Insert(hc4: *mut LZ4HC_CCtx_internal, ip: *const u8) {
    let chainTable: *mut u16 = (*hc4).chainTable.as_mut_ptr();
    let hashTable: *mut u32 = (*hc4).hashTable.as_mut_ptr();
    let prefixPtr: *const u8 = (*hc4).prefixStart;
    let prefixIdx: u32 = (*hc4).dictLimit;
    let target: u32 = ((ip as usize - prefixPtr as usize) as u32).wrapping_add(prefixIdx);
    let mut idx: u32 = (*hc4).nextToUpdate;

    while idx < target {
        let h = LZ4HC_hashPtr(
            prefixPtr
                .wrapping_add(idx as usize)
                .wrapping_sub(prefixIdx as usize),
        );
        let mut delta: usize = (idx.wrapping_sub(*hashTable.add(h as usize))) as usize;
        if delta > LZ4_DISTANCE_MAX as usize {
            delta = LZ4_DISTANCE_MAX as usize;
        }
        DELTANEXTU16_set(chainTable, idx, delta as u16);
        *hashTable.add(h as usize) = idx;
        idx = idx.wrapping_add(1);
    }

    (*hc4).nextToUpdate = target;
}

#[inline(always)]
fn LZ4HC_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}

fn LZ4HC_rotatePattern(rotate: usize, pattern: u32) -> u32 {
    let bitsToRotate: usize = (rotate & (core::mem::size_of::<u32>() - 1)) << 3;
    if bitsToRotate == 0 {
        return pattern;
    }
    LZ4HC_rotl32(pattern, bitsToRotate as u32)
}

unsafe fn LZ4HC_countPattern(ip: *const u8, iEnd: *const u8, pattern32: u32) -> u32 {
    let iStart = ip;
    let mut ip = ip;
    let pattern: RegT = if core::mem::size_of::<RegT>() == 8 {
        (pattern32 as RegT) + (((pattern32 as RegT)) << (core::mem::size_of::<RegT>() * 4))
    } else {
        pattern32 as RegT
    };

    while ip < iEnd.wrapping_sub(core::mem::size_of::<RegT>() - 1) {
        let diff = LZ4_read_ARCH(ip) ^ pattern;
        if diff == 0 {
            ip = ip.wrapping_add(core::mem::size_of::<RegT>());
            continue;
        }
        ip = ip.wrapping_add(LZ4_NbCommonBytes(diff) as usize);
        return (ip as usize - iStart as usize) as u32;
    }

    if LZ4_isLittleEndian() {
        let mut patternByte: RegT = pattern;
        while (ip < iEnd) && (*ip == (patternByte as u8)) {
            ip = ip.wrapping_add(1);
            patternByte >>= 8;
        }
    } else {
        let mut bitOffset: u32 = (core::mem::size_of::<RegT>() * 8) as u32 - 8;
        while ip < iEnd {
            let byte = (pattern >> bitOffset) as u8;
            if *ip != byte {
                break;
            }
            ip = ip.wrapping_add(1);
            bitOffset -= 8;
        }
    }

    (ip as usize - iStart as usize) as u32
}

unsafe fn LZ4HC_reverseCountPattern(ip: *const u8, iLow: *const u8, pattern: u32) -> u32 {
    let iStart = ip;
    let mut ip = ip;

    while ip >= iLow.wrapping_add(4) {
        if LZ4_read32(ip.wrapping_sub(4)) != pattern {
            break;
        }
        ip = ip.wrapping_sub(4);
    }
    {
        let patternBytes = pattern.to_ne_bytes();
        let mut bytePtr: isize = 3;
        while ip > iLow {
            if *ip.wrapping_sub(1) != patternBytes[bytePtr as usize] {
                break;
            }
            ip = ip.wrapping_sub(1);
            bytePtr -= 1;
            if bytePtr < 0 {
                /* mirrors reading out of bounds in the original code; cannot
                 * happen because a 4-byte pattern stops the loop earlier */
                break;
            }
        }
    }
    (iStart as usize - ip as usize) as u32
}

/// Checks if the match is in the last 3 bytes of the dictionary
fn LZ4HC_protectDictEnd(dictLimit: u32, matchIndex: u32) -> bool {
    (dictLimit.wrapping_sub(1).wrapping_sub(matchIndex)) >= 3
}

#[inline(always)]
fn MAXu32(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

#[inline(always)]
fn MINuz(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

unsafe fn LZ4HC_InsertAndGetWiderMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iLowLimit: *const u8,
    iHighLimit: *const u8,
    longest_in: c_int,
    maxNbAttempts: c_int,
    patternAnalysis: c_int,
    chainSwap: c_int,
    dict: i32,
    favorDecSpeed: i32,
) -> LZ4HC_match_t {
    let mut longest = longest_in;
    let chainTable: *mut u16 = (*hc4).chainTable.as_mut_ptr();
    let hashTable: *mut u32 = (*hc4).hashTable.as_mut_ptr();
    let dictCtx: *const LZ4HC_CCtx_internal = (*hc4).dictCtx;
    let prefixPtr: *const u8 = (*hc4).prefixStart;
    let prefixIdx: u32 = (*hc4).dictLimit;
    let ipIndex: u32 = ((ip as usize - prefixPtr as usize) as u32).wrapping_add(prefixIdx);
    let withinStartDistance: bool =
        (*hc4).lowLimit.wrapping_add(LZ4_DISTANCE_MAX + 1) > ipIndex;
    let lowestMatchIndex: u32 = if withinStartDistance {
        (*hc4).lowLimit
    } else {
        ipIndex.wrapping_sub(LZ4_DISTANCE_MAX)
    };
    let dictStart: *const u8 = (*hc4).dictStart;
    let dictIdx: u32 = (*hc4).lowLimit;
    let dictEnd: *const u8 = dictStart.wrapping_add(prefixIdx.wrapping_sub(dictIdx) as usize);
    let lookBackLength: c_int = (ip as usize - iLowLimit as usize) as c_int;
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
    matchIndex = *hashTable.add(LZ4HC_hashPtr(ip) as usize);

    while (matchIndex >= lowestMatchIndex) && (nbAttempts > 0) {
        let mut matchLength: c_int = 0;
        nbAttempts -= 1;
        if (favorDecSpeed != 0) && (ipIndex.wrapping_sub(matchIndex) < 8) {
            /* do nothing */
        } else if matchIndex >= prefixIdx {
            /* within current Prefix */
            let matchPtr: *const u8 =
                prefixPtr.wrapping_add((matchIndex.wrapping_sub(prefixIdx)) as usize);
            if LZ4_read16(iLowLimit.wrapping_offset((longest - 1) as isize))
                == LZ4_read16(
                    matchPtr
                        .wrapping_offset(-(lookBackLength as isize))
                        .wrapping_offset((longest - 1) as isize),
                )
            {
                if LZ4_read32(matchPtr) == pattern {
                    let back: c_int = if lookBackLength != 0 {
                        LZ4HC_countBack(ip, matchPtr, iLowLimit, prefixPtr)
                    } else {
                        0
                    };
                    matchLength = MINMATCH as c_int
                        + LZ4_count(
                            ip.wrapping_add(MINMATCH),
                            matchPtr.wrapping_add(MINMATCH),
                            iHighLimit,
                        ) as c_int;
                    matchLength -= back;
                    if matchLength > longest {
                        longest = matchLength;
                        offset = ipIndex.wrapping_sub(matchIndex) as c_int;
                        sBack = back;
                    }
                }
            }
        } else {
            /* lowestMatchIndex <= matchIndex < dictLimit : within Ext Dict */
            let matchPtr: *const u8 =
                dictStart.wrapping_add((matchIndex.wrapping_sub(dictIdx)) as usize);
            if (matchIndex <= prefixIdx.wrapping_sub(4)) && (LZ4_read32(matchPtr) == pattern) {
                let mut back: c_int = 0;
                let mut vLimit: *const u8 =
                    ip.wrapping_add(prefixIdx.wrapping_sub(matchIndex) as usize);
                if vLimit > iHighLimit {
                    vLimit = iHighLimit;
                }
                matchLength = LZ4_count(
                    ip.wrapping_add(MINMATCH),
                    matchPtr.wrapping_add(MINMATCH),
                    vLimit,
                ) as c_int
                    + MINMATCH as c_int;
                if (ip.wrapping_add(matchLength as usize) == vLimit) && (vLimit < iHighLimit) {
                    matchLength += LZ4_count(
                        ip.wrapping_add(matchLength as usize),
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

        if (chainSwap != 0) && (matchLength == longest) {
            if matchIndex.wrapping_add(longest as u32) <= ipIndex {
                let kTrigger: c_int = 4;
                let mut distanceToNextMatch: u32 = 1;
                let end: c_int = longest - MINMATCH as c_int + 1;
                let mut step: c_int = 1;
                let mut accel: c_int = 1 << kTrigger;
                let mut pos: c_int = 0;
                while pos < end {
                    let candidateDist: u32 =
                        DELTANEXTU16_get(chainTable, matchIndex.wrapping_add(pos as u32));
                    step = {
                        let v = accel;
                        accel = accel.wrapping_add(1);
                        v >> kTrigger
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
                        break;
                    }
                    matchIndex = matchIndex.wrapping_sub(distanceToNextMatch);
                    continue;
                }
            }
        }

        {
            let distNextMatch: u32 = DELTANEXTU16_get(chainTable, matchIndex);
            if (patternAnalysis != 0) && distNextMatch == 1 && matchChainPos == 0 {
                let matchCandidateIdx: u32 = matchIndex.wrapping_sub(1);
                /* may be a repeated pattern */
                if repeat == rep_untested {
                    if ((pattern & 0xFFFF) == (pattern >> 16))
                        & ((pattern & 0xFF) == (pattern >> 24))
                    {
                        repeat = rep_confirmed;
                        srcPatternLength = LZ4HC_countPattern(
                            ip.wrapping_add(core::mem::size_of::<u32>()),
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
                    && LZ4HC_protectDictEnd(prefixIdx, matchCandidateIdx)
                {
                    let extDict: bool = matchCandidateIdx < prefixIdx;
                    let matchPtr: *const u8 = if extDict {
                        dictStart.wrapping_add((matchCandidateIdx.wrapping_sub(dictIdx)) as usize)
                    } else {
                        prefixPtr.wrapping_add((matchCandidateIdx.wrapping_sub(prefixIdx)) as usize)
                    };
                    if LZ4_read32(matchPtr) == pattern {
                        /* good candidate */
                        let iLimit: *const u8 = if extDict { dictEnd } else { iHighLimit };
                        let mut forwardPatternLength: usize = LZ4HC_countPattern(
                            matchPtr.wrapping_add(core::mem::size_of::<u32>()),
                            iLimit,
                            pattern,
                        ) as usize
                            + core::mem::size_of::<u32>();
                        if extDict && matchPtr.wrapping_add(forwardPatternLength) == iLimit {
                            let rotatedPattern =
                                LZ4HC_rotatePattern(forwardPatternLength, pattern);
                            forwardPatternLength +=
                                LZ4HC_countPattern(prefixPtr, iHighLimit, rotatedPattern) as usize;
                        }
                        {
                            let lowestMatchPtr: *const u8 =
                                if extDict { dictStart } else { prefixPtr };
                            let mut backLength: usize =
                                LZ4HC_reverseCountPattern(matchPtr, lowestMatchPtr, pattern)
                                    as usize;
                            let currentSegmentLength: usize;
                            if !extDict
                                && matchPtr.wrapping_sub(backLength) == prefixPtr
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
                                let newMatchIndex: u32 = matchCandidateIdx
                                    .wrapping_add(forwardPatternLength as u32)
                                    .wrapping_sub(srcPatternLength as u32);
                                if LZ4HC_protectDictEnd(prefixIdx, newMatchIndex) {
                                    matchIndex = newMatchIndex;
                                } else {
                                    matchIndex = prefixIdx;
                                }
                            } else {
                                let newMatchIndex: u32 =
                                    matchCandidateIdx.wrapping_sub(backLength as u32);
                                if !LZ4HC_protectDictEnd(prefixIdx, newMatchIndex) {
                                    matchIndex = prefixIdx;
                                } else {
                                    matchIndex = newMatchIndex;
                                    if lookBackLength == 0 {
                                        let maxML: usize =
                                            MINuz(currentSegmentLength, srcPatternLength);
                                        if (longest as usize) < maxML {
                                            if (ip as usize - prefixPtr as usize)
                                                + prefixIdx as usize
                                                - matchIndex as usize
                                                > LZ4_DISTANCE_MAX as usize
                                            {
                                                break;
                                            }
                                            longest = maxML as c_int;
                                            offset = ipIndex.wrapping_sub(matchIndex) as c_int;
                                        }
                                        {
                                            let distToNextPattern: u32 =
                                                DELTANEXTU16_get(chainTable, matchIndex);
                                            if distToNextPattern > matchIndex {
                                                break;
                                            }
                                            matchIndex =
                                                matchIndex.wrapping_sub(distToNextPattern);
                                        }
                                    }
                                }
                            }
                        }
                        continue;
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
        let dictEndOffset: usize = ((*dictCtx).end as usize - (*dictCtx).prefixStart as usize)
            + (*dictCtx).dictLimit as usize;
        let mut dictMatchIndex: u32 = (*dictCtx).hashTable[LZ4HC_hashPtr(ip) as usize];
        matchIndex = dictMatchIndex
            .wrapping_add(lowestMatchIndex)
            .wrapping_sub(dictEndOffset as u32);
        while ipIndex.wrapping_sub(matchIndex) <= LZ4_DISTANCE_MAX && {
            let t = nbAttempts != 0;
            nbAttempts = nbAttempts.wrapping_sub(1);
            t
        } {
            let matchPtr: *const u8 = (*dictCtx)
                .prefixStart
                .wrapping_sub((*dictCtx).dictLimit as usize)
                .wrapping_add(dictMatchIndex as usize);

            if LZ4_read32(matchPtr) == pattern {
                let mut mlt: c_int;
                let back: c_int;
                let mut vLimit: *const u8 =
                    ip.wrapping_add(dictEndOffset - dictMatchIndex as usize);
                if vLimit > iHighLimit {
                    vLimit = iHighLimit;
                }
                mlt = LZ4_count(
                    ip.wrapping_add(MINMATCH),
                    matchPtr.wrapping_add(MINMATCH),
                    vLimit,
                ) as c_int
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
                let nextOffset: u32 =
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

#[inline(always)]
unsafe fn LZ4HC_InsertAndFindBestMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    iLimit: *const u8,
    maxNbAttempts: c_int,
    patternAnalysis: c_int,
    dict: i32,
) -> LZ4HC_match_t {
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
    let inputSize: c_int = *srcSizePtr;
    let patternAnalysis: c_int = (maxNbAttempts > 128) as c_int;

    let mut ip: *const u8 = source as *const u8;
    let mut anchor: *const u8 = ip;
    let iend: *const u8 = ip.wrapping_add(inputSize as usize);
    let mflimit: *const u8 = iend.wrapping_sub(MFLIMIT);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);

    let mut optr: *mut u8 = dest as *mut u8;
    let mut op: *mut u8 = dest as *mut u8;
    let mut oend: *mut u8 = op.wrapping_add(maxOutputSize as usize);

    let mut start0: *const u8 = core::ptr::null();
    let mut start2: *const u8 = core::ptr::null();
    let mut start3: *const u8 = core::ptr::null();
    let mut m0 = LZ4HC_match_t { off: 0, len: 0, back: 0 };
    let mut m1 = LZ4HC_match_t { off: 0, len: 0, back: 0 };
    let mut m2 = LZ4HC_match_t { off: 0, len: 0, back: 0 };
    let mut m3 = LZ4HC_match_t { off: 0, len: 0, back: 0 };
    let nomatch = LZ4HC_match_t { off: 0, len: 0, back: 0 };

    /* init */
    *srcSizePtr = 0;
    if limit == fillOutput {
        oend = oend.wrapping_sub(LASTLITERALS);
    }

    let mut overflow = false;

    'body: {
        if inputSize < LZ4_minLength {
            break 'body; /* goto _last_literals */
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
                ip = ip.wrapping_add(1);
                continue 'main;
            }

            /* saved, in case we would skip too much */
            start0 = ip;
            m0 = m1;

            'search2: loop {
                /* _Search2: */
                if ip.wrapping_add(m1.len as usize) <= mflimit {
                    start2 = ip.wrapping_add(m1.len as usize).wrapping_sub(2);
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
                    start2 = start2.wrapping_offset(m2.back as isize);
                } else {
                    m2 = nomatch;
                }

                if m2.len <= m1.len {
                    /* No better match => encode ML1 immediately */
                    optr = op;
                    if LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                    ) != 0
                    {
                        overflow = true;
                        break 'body;
                    }
                    continue 'main;
                }

                if start0 < ip {
                    if start2 < ip.wrapping_add(m0.len as usize) {
                        ip = start0;
                        m1 = m0;
                    }
                }

                /* Here, start0==ip */
                if ((start2 as usize - ip as usize) as c_int) < 3 {
                    ip = start2;
                    m1 = m2;
                    continue 'search2;
                }

                'search3: loop {
                    /* _Search3: */
                    if ((start2 as usize - ip as usize) as c_int) < OPTIMAL_ML {
                        let correction: c_int;
                        let mut new_ml: c_int = m1.len;
                        if new_ml > OPTIMAL_ML {
                            new_ml = OPTIMAL_ML;
                        }
                        if ip.wrapping_add(new_ml as usize)
                            > start2
                                .wrapping_add(m2.len as usize)
                                .wrapping_sub(MINMATCH)
                        {
                            new_ml = ((start2 as usize - ip as usize) as c_int) + m2.len
                                - MINMATCH as c_int;
                        }
                        correction = new_ml - ((start2 as usize - ip as usize) as c_int);
                        if correction > 0 {
                            start2 = start2.wrapping_add(correction as usize);
                            m2.len -= correction;
                        }
                    }

                    if start2.wrapping_add(m2.len as usize) <= mflimit {
                        start3 = start2.wrapping_add(m2.len as usize).wrapping_sub(3);
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
                        start3 = start3.wrapping_offset(m3.back as isize);
                    } else {
                        m3 = nomatch;
                    }

                    if m3.len <= m2.len {
                        /* No better match => encode ML1 and ML2 */
                        if start2 < ip.wrapping_add(m1.len as usize) {
                            m1.len = (start2 as usize - ip as usize) as c_int;
                        }
                        optr = op;
                        if LZ4HC_encodeSequence(
                            &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                        ) != 0
                        {
                            overflow = true;
                            break 'body;
                        }
                        ip = start2;
                        optr = op;
                        if LZ4HC_encodeSequence(
                            &mut ip, &mut op, &mut anchor, m2.len, m2.off, limit, oend,
                        ) != 0
                        {
                            m1 = m2;
                            overflow = true;
                            break 'body;
                        }
                        continue 'main;
                    }

                    if start3 < ip.wrapping_add(m1.len as usize).wrapping_add(3) {
                        /* Not enough space for match 2 : remove it */
                        if start3 >= ip.wrapping_add(m1.len as usize) {
                            if start2 < ip.wrapping_add(m1.len as usize) {
                                let correction: c_int = (ip.wrapping_add(m1.len as usize) as usize
                                    - start2 as usize)
                                    as c_int;
                                start2 = start2.wrapping_add(correction as usize);
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
                                overflow = true;
                                break 'body;
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

                    /*
                     * OK, now we have 3 ascending matches;
                     * let's write the first one ML1.
                     */
                    if start2 < ip.wrapping_add(m1.len as usize) {
                        if ((start2 as usize - ip as usize) as c_int) < OPTIMAL_ML {
                            let correction: c_int;
                            if m1.len > OPTIMAL_ML {
                                m1.len = OPTIMAL_ML;
                            }
                            if ip.wrapping_add(m1.len as usize)
                                > start2
                                    .wrapping_add(m2.len as usize)
                                    .wrapping_sub(MINMATCH)
                            {
                                m1.len = ((start2 as usize - ip as usize) as c_int) + m2.len
                                    - MINMATCH as c_int;
                            }
                            correction = m1.len - ((start2 as usize - ip as usize) as c_int);
                            if correction > 0 {
                                start2 = start2.wrapping_add(correction as usize);
                                m2.len -= correction;
                            }
                        } else {
                            m1.len = (start2 as usize - ip as usize) as c_int;
                        }
                    }
                    optr = op;
                    if LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                    ) != 0
                    {
                        overflow = true;
                        break 'body;
                    }

                    /* ML2 becomes ML1 */
                    ip = start2;
                    m1 = m2;

                    /* ML3 becomes ML2 */
                    start2 = start3;
                    m2 = m3;

                    /* let's find a new ML3 */
                    continue 'search3;
                }
            }
        }
    }

    if overflow {
        /* _dest_overflow: */
        if limit == fillOutput {
            let ll: usize = ip as usize - anchor as usize;
            let ll_addbytes: usize = (ll + 240) / 255;
            let ll_totalCost: usize = 1 + ll_addbytes + ll;
            let maxLitPos: *mut u8 = oend.wrapping_sub(3);
            op = optr;
            if op.wrapping_add(ll_totalCost) <= maxLitPos {
                let bytesLeftForMl: usize =
                    maxLitPos as usize - (op.wrapping_add(ll_totalCost)) as usize;
                let maxMlSize: usize = MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                if (m1.len as usize) > maxMlSize {
                    m1.len = maxMlSize as c_int;
                }
                if ((oend.wrapping_add(LASTLITERALS)) as usize)
                    .wrapping_sub((op.wrapping_add(ll_totalCost + 2)) as usize)
                    .wrapping_sub(1)
                    .wrapping_add(m1.len as usize)
                    >= MFLIMIT
                {
                    LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, notLimited, oend,
                    );
                }
            }
        /* fall through to _last_literals */
        } else {
            return 0;
        }
    }

    /* _last_literals: */
    {
        let mut lastRunSize: usize = iend as usize - anchor as usize;
        let mut llAdd: usize = (lastRunSize + 255 - RUN_MASK as usize) / 255;
        let totalSize: usize = 1 + llAdd + lastRunSize;
        if limit == fillOutput {
            oend = oend.wrapping_add(LASTLITERALS);
        }
        if limit != notLimited && (op.wrapping_add(totalSize) > oend) {
            if limit == limitedOutput {
                return 0;
            }
            lastRunSize = (oend as usize - op as usize) - 1;
            llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
            lastRunSize -= llAdd;
        }
        ip = anchor.wrapping_add(lastRunSize);

        if lastRunSize >= RUN_MASK as usize {
            let mut accumulator = lastRunSize - RUN_MASK as usize;
            *op = (RUN_MASK << ML_BITS) as u8;
            op = op.wrapping_add(1);
            while accumulator >= 255 {
                *op = 255;
                op = op.wrapping_add(1);
                accumulator -= 255;
            }
            *op = accumulator as u8;
            op = op.wrapping_add(1);
        } else {
            *op = ((lastRunSize << ML_BITS) & 0xFF) as u8;
            op = op.wrapping_add(1);
        }
        mem_copy(op, anchor, lastRunSize);
        op = op.wrapping_add(lastRunSize);
    }

    /* End */
    *srcSizePtr = (ip as usize - source as usize) as c_int;
    (op as usize - dest as usize) as c_int
}

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
    if limit == fillOutput && dstCapacity < 1 {
        return 0;
    }
    if (*srcSizePtr as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        return 0;
    }

    (*ctx).end = (*ctx).end.wrapping_add(*srcSizePtr as usize);
    {
        let cParam = LZ4HC_getCLevelParams(cLevel);
        let favor: i32 = if (*ctx).favorDecSpeed != 0 {
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

unsafe fn LZ4HC_compress_generic_noDictCtx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    cLevel: c_int,
    limit: i32,
) -> c_int {
    LZ4HC_compress_generic_internal(
        ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit, noDictCtx,
    )
}

unsafe fn isStateCompatible(
    ctx1: *const LZ4HC_CCtx_internal,
    ctx2: *const LZ4HC_CCtx_internal,
) -> c_int {
    let isMid1 =
        (LZ4HC_getCLevelParams((*ctx1).compressionLevel as c_int).strat == lz4mid) as c_int;
    let isMid2 =
        (LZ4HC_getCLevelParams((*ctx2).compressionLevel as c_int).strat == lz4mid) as c_int;
    ((isMid1 ^ isMid2) == 0) as c_int
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
    let position: usize = ((*ctx).end as usize - (*ctx).prefixStart as usize)
        + ((*ctx).dictLimit.wrapping_sub((*ctx).lowLimit)) as usize;
    if position >= 64 * 1024 {
        (*ctx).dictCtx = core::ptr::null();
        return LZ4HC_compress_generic_noDictCtx(
            ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit,
        );
    } else if position == 0
        && *srcSizePtr > 4 * 1024
        && isStateCompatible(ctx, (*ctx).dictCtx) != 0
    {
        mem_copy(
            ctx as *mut u8,
            (*ctx).dictCtx as *const u8,
            SIZEOF_LZ4HC_CCTX_INTERNAL,
        );
        LZ4HC_setExternalDict(ctx, src as *const u8);
        (*ctx).compressionLevel = cLevel as i16;
        return LZ4HC_compress_generic_noDictCtx(
            ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit,
        );
    } else {
        return LZ4HC_compress_generic_internal(
            ctx,
            src,
            dst,
            srcSizePtr,
            dstCapacity,
            cLevel,
            limit,
            usingDictCtxHc,
        );
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
    if (*ctx).dictCtx.is_null() {
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
    } else {
        LZ4HC_compress_generic_dictCtx(ctx, src, dst, srcSizePtr, dstCapacity, cLevel, limit)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_sizeofStateHC() -> c_int {
    SIZEOF_LZ4_STREAMHC_T as c_int
}

fn LZ4_streamHC_t_alignment() -> usize {
    8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC_fastReset(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    compressionLevel: c_int,
) -> c_int {
    let ctx = state as *mut LZ4HC_CCtx_internal;
    if !LZ4_isAligned(state as *const u8, LZ4_streamHC_t_alignment()) {
        return 0;
    }
    LZ4_resetStreamHC_fast(state as *mut LZ4_streamHC_t, compressionLevel);
    LZ4HC_init_internal(ctx, src as *const u8);
    let mut srcSize = srcSize;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    compressionLevel: c_int,
) -> c_int {
    let ctx = LZ4_initStreamHC(state, SIZEOF_LZ4_STREAMHC_T);
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
    /* LZ4HC_HEAPMODE == 1 */
    let statePtr = malloc(SIZEOF_LZ4_STREAMHC_T) as *mut LZ4_streamHC_t;
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
    free(statePtr as *mut c_void);
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
    let ctx = LZ4_initStreamHC(state, SIZEOF_LZ4_STREAMHC_T);
    if ctx.is_null() {
        return 0;
    }
    LZ4HC_init_internal(ctx as *mut LZ4HC_CCtx_internal, source as *const u8);
    LZ4_setCompressionLevel(ctx, cLevel);
    LZ4HC_compress_generic(
        ctx as *mut LZ4HC_CCtx_internal,
        source,
        dest,
        sourceSizePtr,
        targetDestSize,
        cLevel,
        fillOutput,
    )
}

/**************************************
*  Streaming Functions
**************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamHC() -> *mut LZ4_streamHC_t {
    let state = calloc(1, SIZEOF_LZ4_STREAMHC_T) as *mut LZ4_streamHC_t;
    if state.is_null() {
        return core::ptr::null_mut();
    }
    LZ4_setCompressionLevel(state, LZ4HC_CLEVEL_DEFAULT);
    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamHC(LZ4_streamHCPtr: *mut LZ4_streamHC_t) -> c_int {
    if LZ4_streamHCPtr.is_null() {
        return 0;
    }
    free(LZ4_streamHCPtr as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStreamHC(
    buffer: *mut c_void,
    size: usize,
) -> *mut LZ4_streamHC_t {
    let LZ4_streamHCPtr = buffer as *mut LZ4_streamHC_t;
    if buffer.is_null() {
        return core::ptr::null_mut();
    }
    if size < SIZEOF_LZ4_STREAMHC_T {
        return core::ptr::null_mut();
    }
    if !LZ4_isAligned(buffer as *const u8, LZ4_streamHC_t_alignment()) {
        return core::ptr::null_mut();
    }
    mem_init(
        LZ4_streamHCPtr as *mut u8,
        0,
        SIZEOF_LZ4HC_CCTX_INTERNAL,
    );
    LZ4_setCompressionLevel(LZ4_streamHCPtr, LZ4HC_CLEVEL_DEFAULT);
    LZ4_streamHCPtr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    compressionLevel: c_int,
) {
    LZ4_initStreamHC(LZ4_streamHCPtr as *mut c_void, SIZEOF_LZ4_STREAMHC_T);
    LZ4_setCompressionLevel(LZ4_streamHCPtr, compressionLevel);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC_fast(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    compressionLevel: c_int,
) {
    let s = LZ4_streamHCPtr as *mut LZ4HC_CCtx_internal;
    if (*s).dirty != 0 {
        LZ4_initStreamHC(LZ4_streamHCPtr as *mut c_void, SIZEOF_LZ4_STREAMHC_T);
    } else {
        (*s).dictLimit = (*s)
            .dictLimit
            .wrapping_add(((*s).end as usize - (*s).prefixStart as usize) as u32);
        (*s).prefixStart = core::ptr::null();
        (*s).end = core::ptr::null();
        (*s).dictCtx = core::ptr::null();
    }
    LZ4_setCompressionLevel(LZ4_streamHCPtr, compressionLevel);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setCompressionLevel(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    compressionLevel: c_int,
) {
    let mut compressionLevel = compressionLevel;
    if compressionLevel < 1 {
        compressionLevel = LZ4HC_CLEVEL_DEFAULT;
    }
    if compressionLevel > LZ4HC_CLEVEL_MAX {
        compressionLevel = LZ4HC_CLEVEL_MAX;
    }
    (*(LZ4_streamHCPtr as *mut LZ4HC_CCtx_internal)).compressionLevel = compressionLevel as i16;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_favorDecompressionSpeed(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    favor: c_int,
) {
    (*(LZ4_streamHCPtr as *mut LZ4HC_CCtx_internal)).favorDecSpeed = (favor != 0) as i8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    let ctxPtr = LZ4_streamHCPtr as *mut LZ4HC_CCtx_internal;
    let cp: cParams_t;
    let mut dictionary = dictionary;
    let mut dictSize = dictSize;
    if dictSize > 64 * 1024 {
        dictionary = dictionary.wrapping_add(dictSize as usize - 64 * 1024);
        dictSize = 64 * 1024;
    }
    /* need a full initialization */
    {
        let cLevel: c_int = (*ctxPtr).compressionLevel as c_int;
        LZ4_initStreamHC(LZ4_streamHCPtr as *mut c_void, SIZEOF_LZ4_STREAMHC_T);
        LZ4_setCompressionLevel(LZ4_streamHCPtr, cLevel);
        cp = LZ4HC_getCLevelParams(cLevel);
    }
    LZ4HC_init_internal(ctxPtr, dictionary as *const u8);
    (*ctxPtr).end = (dictionary as *const u8).wrapping_add(dictSize as usize);
    if cp.strat == lz4mid {
        LZ4MID_fillHTable(ctxPtr, dictionary as *const c_void, dictSize as usize);
    } else {
        if dictSize >= LZ4HC_HASHSIZE as c_int {
            LZ4HC_Insert(ctxPtr, (*ctxPtr).end.wrapping_sub(3));
        }
    }
    dictSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_HC_dictionary(
    working_stream: *mut LZ4_streamHC_t,
    dictionary_stream: *const LZ4_streamHC_t,
) {
    (*(working_stream as *mut LZ4HC_CCtx_internal)).dictCtx = if !dictionary_stream.is_null() {
        dictionary_stream as *const LZ4HC_CCtx_internal
    } else {
        core::ptr::null()
    };
}

/* compression */

unsafe fn LZ4HC_setExternalDict(ctxPtr: *mut LZ4HC_CCtx_internal, newBlock: *const u8) {
    if ((*ctxPtr).end >= (*ctxPtr).prefixStart.wrapping_add(4))
        && (LZ4HC_getCLevelParams((*ctxPtr).compressionLevel as c_int).strat != lz4mid)
    {
        LZ4HC_Insert(ctxPtr, (*ctxPtr).end.wrapping_sub(3));
    }

    (*ctxPtr).lowLimit = (*ctxPtr).dictLimit;
    (*ctxPtr).dictStart = (*ctxPtr).prefixStart;
    (*ctxPtr).dictLimit = (*ctxPtr)
        .dictLimit
        .wrapping_add(((*ctxPtr).end as usize - (*ctxPtr).prefixStart as usize) as u32);
    (*ctxPtr).prefixStart = newBlock;
    (*ctxPtr).end = newBlock;
    (*ctxPtr).nextToUpdate = (*ctxPtr).dictLimit;

    (*ctxPtr).dictCtx = core::ptr::null();
}

unsafe fn LZ4_compressHC_continue_generic(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    dstCapacity: c_int,
    limit: i32,
) -> c_int {
    let ctxPtr = LZ4_streamHCPtr as *mut LZ4HC_CCtx_internal;
    /* auto-init if forgotten */
    if (*ctxPtr).prefixStart.is_null() {
        LZ4HC_init_internal(ctxPtr, src as *const u8);
    }

    /* Check overflow */
    if ((*ctxPtr).end as usize - (*ctxPtr).prefixStart as usize) + (*ctxPtr).dictLimit as usize
        > (2usize << 30)
    {
        let mut dictSize: usize = (*ctxPtr).end as usize - (*ctxPtr).prefixStart as usize;
        if dictSize > 64 * 1024 {
            dictSize = 64 * 1024;
        }
        LZ4_loadDictHC(
            LZ4_streamHCPtr,
            ((*ctxPtr).end as *const c_char).wrapping_sub(dictSize),
            dictSize as c_int,
        );
    }

    /* Check if blocks follow each other */
    if (src as *const u8) != (*ctxPtr).end {
        LZ4HC_setExternalDict(ctxPtr, src as *const u8);
    }

    /* Check overlapping input/dictionary space */
    {
        let mut sourceEnd: *const u8 = (src as *const u8).wrapping_add(*srcSizePtr as usize);
        let dictBegin: *const u8 = (*ctxPtr).dictStart;
        let dictEnd: *const u8 = (*ctxPtr)
            .dictStart
            .wrapping_add(((*ctxPtr).dictLimit.wrapping_sub((*ctxPtr).lowLimit)) as usize);
        if (sourceEnd > dictBegin) && ((src as *const u8) < dictEnd) {
            if sourceEnd > dictEnd {
                sourceEnd = dictEnd;
            }
            (*ctxPtr).lowLimit = (*ctxPtr)
                .lowLimit
                .wrapping_add((sourceEnd as usize - (*ctxPtr).dictStart as usize) as u32);
            (*ctxPtr).dictStart = (*ctxPtr)
                .dictStart
                .wrapping_add(sourceEnd as usize - (*ctxPtr).dictStart as usize);
            /* invalidate dictionary if it's too small */
            if (*ctxPtr).dictLimit.wrapping_sub((*ctxPtr).lowLimit) < LZ4HC_HASHSIZE as u32 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    let mut srcSize = srcSize;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue_destSize(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDestSize: c_int,
) -> c_int {
    LZ4_compressHC_continue_generic(
        LZ4_streamHCPtr,
        src,
        dst,
        srcSizePtr,
        targetDestSize,
        fillOutput,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDictHC(
    LZ4_streamHCPtr: *mut LZ4_streamHC_t,
    safeBuffer: *mut c_char,
    dictSize: c_int,
) -> c_int {
    let streamPtr = LZ4_streamHCPtr as *mut LZ4HC_CCtx_internal;
    let prefixSize: c_int =
        ((*streamPtr).end as usize - (*streamPtr).prefixStart as usize) as c_int;
    let mut dictSize = dictSize;
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
        mem_move(
            safeBuffer as *mut u8,
            (*streamPtr).end.wrapping_sub(dictSize as usize),
            dictSize as usize,
        );
    }
    {
        let endIndex: u32 = (((*streamPtr).end as usize - (*streamPtr).prefixStart as usize)
            as u32)
            .wrapping_add((*streamPtr).dictLimit);
        (*streamPtr).end = if safeBuffer.is_null() {
            core::ptr::null()
        } else {
            (safeBuffer as *const u8).wrapping_add(dictSize as usize)
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

/// price in bytes
#[inline(always)]
fn LZ4HC_literalsPrice(litlen: c_int) -> c_int {
    let mut price = litlen;
    if litlen >= RUN_MASK as c_int {
        price += 1 + ((litlen - RUN_MASK as c_int) / 255);
    }
    price
}

/// requires mlen >= MINMATCH
#[inline(always)]
fn LZ4HC_sequencePrice(litlen: c_int, mlen: c_int) -> c_int {
    let mut price = 1 + 2;
    price += LZ4HC_literalsPrice(litlen);
    if mlen >= (ML_MASK as c_int + MINMATCH as c_int) {
        price += 1 + ((mlen - (ML_MASK as c_int + MINMATCH as c_int)) / 255);
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
    if favorDecSpeed != 0 {
        if (md.len > 18) & (md.len <= 36) {
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
    sufficient_len_in: usize,
    limit: i32,
    fullUpdate: c_int,
    dict: i32,
    favorDecSpeed: i32,
) -> c_int {
    let mut retval: c_int = 0;
    let mut sufficient_len = sufficient_len_in;

    /* LZ4HC_HEAPMODE == 1 : allocate the table */
    let optAlloc = malloc(
        core::mem::size_of::<LZ4HC_optimal_t>() * (LZ4_OPT_NUM + TRAILING_LITERALS),
    ) as *mut LZ4HC_optimal_t;

    let mut ip: *const u8 = source as *const u8;
    let mut anchor: *const u8 = ip;
    let iend: *const u8 = ip.wrapping_add(*srcSizePtr as usize);
    let mflimit: *const u8 = iend.wrapping_sub(MFLIMIT);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);
    let mut op: *mut u8 = dst as *mut u8;
    let mut opSaved: *mut u8 = dst as *mut u8;
    let mut oend: *mut u8 = op.wrapping_add(dstCapacity as usize);
    let mut ovml: c_int = MINMATCH as c_int;
    let mut ovoff: c_int = 0;

    if optAlloc.is_null() {
        return retval;
    }
    let opt = optAlloc;

    *srcSizePtr = 0;
    if limit == fillOutput {
        oend = oend.wrapping_sub(LASTLITERALS);
    }
    if sufficient_len >= LZ4_OPT_NUM {
        sufficient_len = LZ4_OPT_NUM - 1;
    }

    let mut overflow = false;

    'body: {
        /* Main Loop */
        'main: while ip <= mflimit {
            let llen: c_int = (ip as usize - anchor as usize) as c_int;
            let mut best_mlen: c_int;
            let mut best_off: c_int;
            let mut cur: c_int;
            let mut last_match_pos: c_int = 0;
            let mut goto_encode = false;

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
                ip = ip.wrapping_add(1);
                continue 'main;
            }

            if (firstMatch.len as usize) > sufficient_len {
                /* good enough solution : immediate encoding */
                let firstML: c_int = firstMatch.len;
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
                    overflow = true;
                    break 'body;
                }
                continue 'main;
            }

            /* set prices for first positions (literals) */
            {
                let mut rPos: c_int = 0;
                while rPos < MINMATCH as c_int {
                    let cost = LZ4HC_literalsPrice(llen + rPos);
                    let e = opt.add(rPos as usize);
                    (*e).mlen = 1;
                    (*e).off = 0;
                    (*e).litlen = llen + rPos;
                    (*e).price = cost;
                    rPos += 1;
                }
            }
            /* set prices using initial match */
            {
                let matchML: c_int = firstMatch.len;
                let offset: c_int = firstMatch.off;
                let mut mlen: c_int = MINMATCH as c_int;
                while mlen <= matchML {
                    let cost = LZ4HC_sequencePrice(llen, mlen);
                    let e = opt.add(mlen as usize);
                    (*e).mlen = mlen;
                    (*e).off = offset;
                    (*e).litlen = llen;
                    (*e).price = cost;
                    mlen += 1;
                }
            }
            last_match_pos = firstMatch.len;
            {
                let mut addLit: c_int = 1;
                while addLit <= TRAILING_LITERALS as c_int {
                    let e = opt.add((last_match_pos + addLit) as usize);
                    (*e).mlen = 1;
                    (*e).off = 0;
                    (*e).litlen = addLit;
                    (*e).price =
                        (*opt.add(last_match_pos as usize)).price + LZ4HC_literalsPrice(addLit);
                    addLit += 1;
                }
            }

            /* check further positions */
            best_mlen = 0;
            best_off = 0;
            cur = 1;
            while cur < last_match_pos {
                let curPtr: *const u8 = ip.wrapping_add(cur as usize);
                let newMatch: LZ4HC_match_t;

                if curPtr > mflimit {
                    break;
                }
                if fullUpdate != 0 {
                    if ((*opt.add((cur + 1) as usize)).price <= (*opt.add(cur as usize)).price)
                        && ((*opt.add((cur + MINMATCH as c_int) as usize)).price
                            < (*opt.add(cur as usize)).price + 3)
                    {
                        cur += 1;
                        continue;
                    }
                } else {
                    if (*opt.add((cur + 1) as usize)).price <= (*opt.add(cur as usize)).price {
                        cur += 1;
                        continue;
                    }
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
                    || ((newMatch.len + cur) as usize >= LZ4_OPT_NUM)
                {
                    /* immediate encoding */
                    best_mlen = newMatch.len;
                    best_off = newMatch.off;
                    last_match_pos = cur + 1;
                    goto_encode = true;
                    break;
                }

                /* before match : set price with literals at beginning */
                {
                    let baseLitlen: c_int = (*opt.add(cur as usize)).litlen;
                    let mut litlen: c_int = 1;
                    while litlen < MINMATCH as c_int {
                        let price = (*opt.add(cur as usize)).price
                            - LZ4HC_literalsPrice(baseLitlen)
                            + LZ4HC_literalsPrice(baseLitlen + litlen);
                        let pos = cur + litlen;
                        if price < (*opt.add(pos as usize)).price {
                            let e = opt.add(pos as usize);
                            (*e).mlen = 1;
                            (*e).off = 0;
                            (*e).litlen = baseLitlen + litlen;
                            (*e).price = price;
                        }
                        litlen += 1;
                    }
                }

                /* set prices using match at position = cur */
                {
                    let matchML: c_int = newMatch.len;
                    let mut ml: c_int = MINMATCH as c_int;

                    while ml <= matchML {
                        let pos = cur + ml;
                        let offset = newMatch.off;
                        let price: c_int;
                        let ll: c_int;
                        if (*opt.add(cur as usize)).mlen == 1 {
                            ll = (*opt.add(cur as usize)).litlen;
                            price = (if cur > ll {
                                (*opt.add((cur - ll) as usize)).price
                            } else {
                                0
                            }) + LZ4HC_sequencePrice(ll, ml);
                        } else {
                            ll = 0;
                            price = (*opt.add(cur as usize)).price + LZ4HC_sequencePrice(0, ml);
                        }

                        if pos > last_match_pos + TRAILING_LITERALS as c_int
                            || price <= (*opt.add(pos as usize)).price - (favorDecSpeed as c_int)
                        {
                            if (ml == matchML) && (last_match_pos < pos) {
                                last_match_pos = pos;
                            }
                            let e = opt.add(pos as usize);
                            (*e).mlen = ml;
                            (*e).off = offset;
                            (*e).litlen = ll;
                            (*e).price = price;
                        }
                        ml += 1;
                    }
                }
                /* complete following positions with literals */
                {
                    let mut addLit: c_int = 1;
                    while addLit <= TRAILING_LITERALS as c_int {
                        let e = opt.add((last_match_pos + addLit) as usize);
                        (*e).mlen = 1;
                        (*e).off = 0;
                        (*e).litlen = addLit;
                        (*e).price = (*opt.add(last_match_pos as usize)).price
                            + LZ4HC_literalsPrice(addLit);
                        addLit += 1;
                    }
                }
                cur += 1;
            }

            if !goto_encode {
                best_mlen = (*opt.add(last_match_pos as usize)).mlen;
                best_off = (*opt.add(last_match_pos as usize)).off;
                cur = last_match_pos - best_mlen;
            }

            /* encode: cur, last_match_pos, best_mlen, best_off must be set */
            {
                let mut candidate_pos: c_int = cur;
                let mut selected_matchLength: c_int = best_mlen;
                let mut selected_offset: c_int = best_off;
                loop {
                    let next_matchLength: c_int = (*opt.add(candidate_pos as usize)).mlen;
                    let next_offset: c_int = (*opt.add(candidate_pos as usize)).off;
                    (*opt.add(candidate_pos as usize)).mlen = selected_matchLength;
                    (*opt.add(candidate_pos as usize)).off = selected_offset;
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
                    let ml: c_int = (*opt.add(rPos as usize)).mlen;
                    let offset: c_int = (*opt.add(rPos as usize)).off;
                    if ml == 1 {
                        ip = ip.wrapping_add(1);
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
                        overflow = true;
                        break 'body;
                    }
                }
            }
        }
    }

    if overflow {
        /* _dest_overflow: */
        if limit == fillOutput {
            let ll: usize = ip as usize - anchor as usize;
            let ll_addbytes: usize = (ll + 240) / 255;
            let ll_totalCost: usize = 1 + ll_addbytes + ll;
            let maxLitPos: *mut u8 = oend.wrapping_sub(3);
            op = opSaved;
            if op.wrapping_add(ll_totalCost) <= maxLitPos {
                let bytesLeftForMl: usize =
                    maxLitPos as usize - (op.wrapping_add(ll_totalCost)) as usize;
                let maxMlSize: usize = MINMATCH + (ML_MASK as usize - 1) + (bytesLeftForMl * 255);
                if (ovml as usize) > maxMlSize {
                    ovml = maxMlSize as c_int;
                }
                if ((oend.wrapping_add(LASTLITERALS)) as usize)
                    .wrapping_sub((op.wrapping_add(ll_totalCost + 2)) as usize)
                    .wrapping_sub(1)
                    .wrapping_add(ovml as usize)
                    >= MFLIMIT
                {
                    LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, ovml, ovoff, notLimited, oend,
                    );
                }
            }
        /* fall through to _last_literals */
        } else {
            free(optAlloc as *mut c_void);
            return 0;
        }
    }

    /* _last_literals: */
    {
        let mut lastRunSize: usize = iend as usize - anchor as usize;
        let mut llAdd: usize = (lastRunSize + 255 - RUN_MASK as usize) / 255;
        let totalSize: usize = 1 + llAdd + lastRunSize;
        if limit == fillOutput {
            oend = oend.wrapping_add(LASTLITERALS);
        }
        if limit != notLimited && (op.wrapping_add(totalSize) > oend) {
            if limit == limitedOutput {
                retval = 0;
                free(optAlloc as *mut c_void);
                return retval;
            }
            lastRunSize = (oend as usize - op as usize) - 1;
            llAdd = (lastRunSize + 256 - RUN_MASK as usize) / 256;
            lastRunSize -= llAdd;
        }
        ip = anchor.wrapping_add(lastRunSize);

        if lastRunSize >= RUN_MASK as usize {
            let mut accumulator = lastRunSize - RUN_MASK as usize;
            *op = (RUN_MASK << ML_BITS) as u8;
            op = op.wrapping_add(1);
            while accumulator >= 255 {
                *op = 255;
                op = op.wrapping_add(1);
                accumulator -= 255;
            }
            *op = accumulator as u8;
            op = op.wrapping_add(1);
        } else {
            *op = ((lastRunSize << ML_BITS) & 0xFF) as u8;
            op = op.wrapping_add(1);
        }
        mem_copy(op, anchor, lastRunSize);
        op = op.wrapping_add(lastRunSize);
    }

    /* End */
    *srcSizePtr = (ip as usize - source as usize) as c_int;
    retval = (op as usize - dst as usize) as c_int;

    free(optAlloc as *mut c_void);
    retval
}

/***************************************************
*  Deprecated Functions
***************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, LZ4_compressBound(srcSize), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, maxDstSize, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, LZ4_compressBound(srcSize), cLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
    cLevel: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, srcSize, maxDstSize, cLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, LZ4_compressBound(srcSize), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDstSize, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, LZ4_compressBound(srcSize), cLevel)
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
    LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDstSize, cLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    LZ4_compress_HC_continue(ctx, src, dst, srcSize, LZ4_compressBound(srcSize))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    maxDstSize: c_int,
) -> c_int {
    LZ4_compress_HC_continue(ctx, src, dst, srcSize, maxDstSize)
}

/* Deprecated streaming functions */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_sizeofStreamStateHC() -> c_int {
    SIZEOF_LZ4_STREAMHC_T as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamStateHC(
    state: *mut c_void,
    inputBuffer: *mut c_char,
) -> c_int {
    let hc4 = LZ4_initStreamHC(state, SIZEOF_LZ4_STREAMHC_T);
    if hc4.is_null() {
        return 1;
    }
    LZ4HC_init_internal(hc4 as *mut LZ4HC_CCtx_internal, inputBuffer as *const u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createHC(inputBuffer: *const c_char) -> *mut c_void {
    let hc4 = LZ4_createStreamHC();
    if hc4.is_null() {
        return core::ptr::null_mut();
    }
    LZ4HC_init_internal(hc4 as *mut LZ4HC_CCtx_internal, inputBuffer as *const u8);
    hc4 as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeHC(LZ4HC_Data: *mut c_void) -> c_int {
    if LZ4HC_Data.is_null() {
        return 0;
    }
    free(LZ4HC_Data);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_continue(
    LZ4HC_Data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    cLevel: c_int,
) -> c_int {
    let mut srcSize = srcSize;
    LZ4HC_compress_generic(
        LZ4HC_Data as *mut LZ4HC_CCtx_internal,
        src,
        dst,
        &mut srcSize,
        0,
        cLevel,
        notLimited,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_continue(
    LZ4HC_Data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    cLevel: c_int,
) -> c_int {
    let mut srcSize = srcSize;
    LZ4HC_compress_generic(
        LZ4HC_Data as *mut LZ4HC_CCtx_internal,
        src,
        dst,
        &mut srcSize,
        dstCapacity,
        cLevel,
        limitedOutput,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBufferHC(LZ4HC_Data: *mut c_void) -> *mut c_char {
    let s = LZ4HC_Data as *mut LZ4HC_CCtx_internal;
    let bufferStart: *const u8 = (*s)
        .prefixStart
        .wrapping_sub((*s).dictLimit as usize)
        .wrapping_add((*s).lowLimit as usize);
    LZ4_resetStreamHC_fast(
        LZ4HC_Data as *mut LZ4_streamHC_t,
        (*s).compressionLevel as c_int,
    );
    bufferStart as *mut c_char
}
