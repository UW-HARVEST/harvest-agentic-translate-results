//! Translation of c_src/src/lz4hc.c (LZ4 HC v1.10.0)
//! Assumptions: x86_64 Linux/gcc, little-endian, LZ4HC_HEAPMODE == 1.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::lz4::{
    free, malloc, memcpy_n, memmove_n, memset_n, pdiff, read16, read32, read64, writeLE16,
    LZ4_compressBound, LZ4_count, LZ4_wildCopy8, LASTLITERALS, LZ4_DISTANCE_MAX, LZ4_MAX_INPUT_SIZE,
    LZ4_MINLENGTH, MFLIMIT, MINMATCH, ML_BITS, ML_MASK, RUN_MASK,
};

pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;

pub const LZ4HC_DICTIONARY_LOGSIZE: u32 = 16;
pub const LZ4HC_MAXD: usize = 1 << LZ4HC_DICTIONARY_LOGSIZE;
pub const LZ4HC_HASH_LOG: u32 = 15;
pub const LZ4HC_HASHTABLESIZE: usize = 1 << LZ4HC_HASH_LOG;

pub const LZ4_STREAMHC_MINSIZE: usize = 262200;

pub const OPTIMAL_ML: c_int = ((ML_MASK - 1) + MINMATCH as u32) as c_int; /* 18 */
pub const LZ4_OPT_NUM: usize = 1 << 12;

pub const LZ4HC_HASHSIZE: usize = 4;
pub const LZ4MID_HASHSIZE: usize = 8;
pub const LZ4MID_HASHLOG: u32 = LZ4HC_HASH_LOG - 1;
pub const LZ4MID_HASHTABLESIZE: usize = 1 << LZ4MID_HASHLOG;

/* limitedOutput_directive */
const NOT_LIMITED: c_int = 0;
const LIMITED_OUTPUT: c_int = 1;
const FILL_OUTPUT: c_int = 2;

/* dictCtx_directive */
const NO_DICT_CTX: c_int = 0;
const USING_DICT_CTX_HC: c_int = 1;

/* lz4hc_strat_e */
const LZ4MID: c_int = 0;
const LZ4HC: c_int = 1;
const LZ4OPT: c_int = 2;

/* repeat_state_e */
const REP_UNTESTED: c_int = 0;
const REP_NOT: c_int = 1;
const REP_CONFIRMED: c_int = 2;

/* HCfavor_e */
const FAVOR_COMPRESSION_RATIO: c_int = 0;
const FAVOR_DECOMPRESSION_SPEED: c_int = 1;

#[repr(C)]
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
pub struct LZ4_streamHC_t {
    pub minStateSize: [u8; LZ4_STREAMHC_MINSIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4HC_match_t {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

const NOMATCH: LZ4HC_match_t = LZ4HC_match_t { off: 0, len: 0, back: 0 };

#[derive(Clone, Copy)]
struct cParams_t {
    strat: c_int,
    nbSearches: c_int,
    targetLength: u32,
}

static K_CL_TABLE: [cParams_t; (LZ4HC_CLEVEL_MAX + 1) as usize] = [
    cParams_t { strat: LZ4MID, nbSearches: 2, targetLength: 16 },
    cParams_t { strat: LZ4MID, nbSearches: 2, targetLength: 16 },
    cParams_t { strat: LZ4MID, nbSearches: 2, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 4, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 8, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 16, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 32, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 64, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 128, targetLength: 16 },
    cParams_t { strat: LZ4HC, nbSearches: 256, targetLength: 16 },
    cParams_t { strat: LZ4OPT, nbSearches: 96, targetLength: 64 },
    cParams_t { strat: LZ4OPT, nbSearches: 512, targetLength: 128 },
    cParams_t { strat: LZ4OPT, nbSearches: 16384, targetLength: LZ4_OPT_NUM as u32 },
];

fn LZ4HC_getCLevelParams(mut c_level: c_int) -> cParams_t {
    if c_level < 1 {
        c_level = LZ4HC_CLEVEL_DEFAULT;
    }
    if LZ4HC_CLEVEL_MAX < c_level {
        c_level = LZ4HC_CLEVEL_MAX;
    }
    K_CL_TABLE[c_level as usize]
}

/* ---- hashing ---- */

#[inline(always)]
unsafe fn LZ4HC_hashPtr(p: *const u8) -> u32 {
    (read32(p).wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - LZ4HC_HASH_LOG)
}

#[inline(always)]
fn LZ4MID_hash4(v: u32) -> u32 {
    v.wrapping_mul(2654435761u32) >> (32 - LZ4MID_HASHLOG)
}
#[inline(always)]
unsafe fn LZ4MID_hash4Ptr(p: *const u8) -> u32 {
    LZ4MID_hash4(read32(p))
}
#[inline(always)]
fn LZ4MID_hash7(v: u64) -> u32 {
    (((v << (64 - 56)).wrapping_mul(58295818150454627u64)) >> (64 - LZ4MID_HASHLOG)) as u32
}
#[inline(always)]
unsafe fn LZ4_readLE64(p: *const u8) -> u64 {
    read64(p)
}
#[inline(always)]
unsafe fn LZ4MID_hash8Ptr(p: *const u8) -> u32 {
    LZ4MID_hash7(LZ4_readLE64(p))
}

#[inline(always)]
fn LZ4HC_NbCommonBytes32(val: u32) -> u32 {
    // little-endian: __builtin_clz(val) >> 3
    val.leading_zeros() >> 3
}

/// @return : negative value, nb of common bytes before ip/match
#[inline(always)]
unsafe fn LZ4HC_countBack(
    ip: *const u8,
    m: *const u8,
    i_min: *const u8,
    m_min: *const u8,
) -> c_int {
    let mut back: c_int = 0;
    let a = pdiff(i_min, ip);
    let b = pdiff(m_min, m);
    let min: c_int = (if a > b { a } else { b }) as c_int;

    while (back - min) > 3 {
        let v = read32(ip.wrapping_offset(back as isize - 4))
            ^ read32(m.wrapping_offset(back as isize - 4));
        if v != 0 {
            return back - LZ4HC_NbCommonBytes32(v) as c_int;
        } else {
            back -= 4;
        }
    }
    while back > min
        && *ip.wrapping_offset(back as isize - 1) == *m.wrapping_offset(back as isize - 1)
    {
        back -= 1;
    }
    back
}

#[inline(always)]
unsafe fn deltanext_get(chain_table: *const u16, pos: u32) -> u32 {
    *chain_table.add((pos & 0xFFFF) as usize) as u32
}
#[inline(always)]
unsafe fn deltanext_set(chain_table: *mut u16, pos: u32, v: u16) {
    *chain_table.add((pos & 0xFFFF) as usize) = v;
}

/* ---- init ---- */

unsafe fn LZ4HC_clearTables(hc4: *mut LZ4HC_CCtx_internal) {
    memset_n(
        (*hc4).hashTable.as_mut_ptr() as *mut u8,
        0,
        LZ4HC_HASHTABLESIZE * 4,
    );
    memset_n(
        (*hc4).chainTable.as_mut_ptr() as *mut u8,
        0xFF,
        LZ4HC_MAXD * 2,
    );
}

unsafe fn LZ4HC_init_internal(hc4: *mut LZ4HC_CCtx_internal, start: *const u8) {
    let buffer_size = pdiff((*hc4).end, (*hc4).prefixStart) as usize;
    let mut new_starting_offset = buffer_size.wrapping_add((*hc4).dictLimit as usize);
    if new_starting_offset > (1usize << 30) {
        LZ4HC_clearTables(hc4);
        new_starting_offset = 0;
    }
    new_starting_offset += 64 * 1024;
    (*hc4).nextToUpdate = new_starting_offset as u32;
    (*hc4).prefixStart = start;
    (*hc4).end = start;
    (*hc4).dictStart = start;
    (*hc4).dictLimit = new_starting_offset as u32;
    (*hc4).lowLimit = new_starting_offset as u32;
}

/* ---- encode ---- */

#[inline(always)]
unsafe fn LZ4HC_encodeSequence(
    _ip: &mut *const u8,
    _op: &mut *mut u8,
    _anchor: &mut *const u8,
    match_length: c_int,
    offset: c_int,
    limit: c_int,
    oend: *mut u8,
) -> c_int {
    let mut length: usize;
    let token: *mut u8 = *_op;
    *_op = (*_op).wrapping_add(1);

    /* Encode Literal length */
    length = pdiff(*_ip, *_anchor) as usize;
    if limit != 0
        && (*_op)
            .wrapping_add(length / 255)
            .wrapping_add(length)
            .wrapping_add(2 + 1 + LASTLITERALS)
            > oend
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
        *token = ((length as u32) << ML_BITS) as u8;
    }

    /* Copy Literals */
    LZ4_wildCopy8(*_op, *_anchor, (*_op).wrapping_add(length));
    *_op = (*_op).wrapping_add(length);

    /* Encode Offset */
    writeLE16(*_op, offset as u16);
    *_op = (*_op).wrapping_add(2);

    /* Encode MatchLength */
    length = (match_length as usize).wrapping_sub(MINMATCH);
    if limit != 0
        && (*_op)
            .wrapping_add(length / 255)
            .wrapping_add(1 + LASTLITERALS)
            > oend
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
        *token = (*token).wrapping_add(length as u8);
    }

    /* Prepare next loop */
    *_ip = (*_ip).wrapping_add(match_length as usize);
    *_anchor = *_ip;

    0
}

/* ---- ext dict search ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4HC_searchExtDict(
    ip: *const u8,
    ip_index: u32,
    i_low_limit: *const u8,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
    mut current_best_ml: c_int,
    mut nb_attempts: c_int,
) -> LZ4HC_match_t {
    let l_dict_end_index =
        (pdiff((*dict_ctx).end, (*dict_ctx).prefixStart) as usize) + (*dict_ctx).dictLimit as usize;
    let mut l_dict_match_index: u32 = (*dict_ctx).hashTable[LZ4HC_hashPtr(ip) as usize];
    let mut match_index: u32 = l_dict_match_index
        .wrapping_add(g_dict_end_index)
        .wrapping_sub(l_dict_end_index as u32);
    let mut offset: c_int = 0;
    let mut s_back: c_int = 0;

    loop {
        if !(ip_index.wrapping_sub(match_index) <= LZ4_DISTANCE_MAX) {
            break;
        }
        let old = nb_attempts;
        nb_attempts -= 1;
        if old == 0 {
            break;
        }

        let match_ptr = (*dict_ctx)
            .prefixStart
            .wrapping_sub((*dict_ctx).dictLimit as usize)
            .wrapping_add(l_dict_match_index as usize);

        if read32(match_ptr) == read32(ip) {
            let mut mlt: c_int;
            let mut back: c_int;
            let mut v_limit =
                ip.wrapping_add(l_dict_end_index.wrapping_sub(l_dict_match_index as usize));
            if v_limit > i_high_limit {
                v_limit = i_high_limit;
            }
            mlt = LZ4_count(
                ip.wrapping_add(MINMATCH),
                match_ptr.wrapping_add(MINMATCH),
                v_limit,
            ) as c_int
                + MINMATCH as c_int;
            back = if ip > i_low_limit {
                LZ4HC_countBack(ip, match_ptr, i_low_limit, (*dict_ctx).prefixStart)
            } else {
                0
            };
            mlt -= back;
            if mlt > current_best_ml {
                current_best_ml = mlt;
                offset = ip_index.wrapping_sub(match_index) as c_int;
                s_back = back;
            }
        }

        {
            let next_offset = deltanext_get((*dict_ctx).chainTable.as_ptr(), l_dict_match_index);
            l_dict_match_index = l_dict_match_index.wrapping_sub(next_offset);
            match_index = match_index.wrapping_sub(next_offset);
        }
    }

    LZ4HC_match_t {
        len: current_best_ml,
        off: offset,
        back: s_back,
    }
}

unsafe fn LZ4MID_searchHCDict(
    ip: *const u8,
    ip_index: u32,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
) -> LZ4HC_match_t {
    LZ4HC_searchExtDict(
        ip,
        ip_index,
        ip,
        i_high_limit,
        dict_ctx,
        g_dict_end_index,
        MINMATCH as c_int - 1,
        2,
    )
}

unsafe fn LZ4MID_searchExtDict(
    ip: *const u8,
    ip_index: u32,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
) -> LZ4HC_match_t {
    let l_dict_end_index =
        (pdiff((*dict_ctx).end, (*dict_ctx).prefixStart) as usize) + (*dict_ctx).dictLimit as usize;
    let hash4_table = (*dict_ctx).hashTable.as_ptr();
    let hash8_table = hash4_table.add(LZ4MID_HASHTABLESIZE);

    /* search long match first */
    {
        let l8 = *hash8_table.add(LZ4MID_hash8Ptr(ip) as usize);
        let m8_index = l8
            .wrapping_add(g_dict_end_index)
            .wrapping_sub(l_dict_end_index as u32);
        if ip_index.wrapping_sub(m8_index) <= LZ4_DISTANCE_MAX {
            let match_ptr = (*dict_ctx)
                .prefixStart
                .wrapping_sub((*dict_ctx).dictLimit as usize)
                .wrapping_add(l8 as usize);
            let a = l_dict_end_index.wrapping_sub(l8 as usize);
            let b = pdiff(i_high_limit, ip) as usize;
            let safe_len = if a < b { a } else { b };
            let mlt = LZ4_count(ip, match_ptr, ip.wrapping_add(safe_len)) as c_int;
            if mlt >= MINMATCH as c_int {
                return LZ4HC_match_t {
                    len: mlt,
                    off: ip_index.wrapping_sub(m8_index) as c_int,
                    back: 0,
                };
            }
        }
    }

    /* search for short match second */
    {
        let l4 = *hash4_table.add(LZ4MID_hash4Ptr(ip) as usize);
        let m4_index = l4
            .wrapping_add(g_dict_end_index)
            .wrapping_sub(l_dict_end_index as u32);
        if ip_index.wrapping_sub(m4_index) <= LZ4_DISTANCE_MAX {
            let match_ptr = (*dict_ctx)
                .prefixStart
                .wrapping_sub((*dict_ctx).dictLimit as usize)
                .wrapping_add(l4 as usize);
            let a = l_dict_end_index.wrapping_sub(l4 as usize);
            let b = pdiff(i_high_limit, ip) as usize;
            let safe_len = if a < b { a } else { b };
            let mlt = LZ4_count(ip, match_ptr, ip.wrapping_add(safe_len)) as c_int;
            if mlt >= MINMATCH as c_int {
                return LZ4HC_match_t {
                    len: mlt,
                    off: ip_index.wrapping_sub(m4_index) as c_int,
                    back: 0,
                };
            }
        }
    }

    NOMATCH
}

/* ---- Mid compression (level 2) ---- */

#[inline(always)]
unsafe fn LZ4MID_addPosition(h_table: *mut u32, h_value: u32, index: u32) {
    *h_table.add(h_value as usize) = index;
}

unsafe fn LZ4MID_fillHTable(cctx: *mut LZ4HC_CCtx_internal, dict: *const c_void, size: usize) {
    let hash4_table = (*cctx).hashTable.as_mut_ptr();
    let hash8_table = hash4_table.add(LZ4MID_HASHTABLESIZE);
    let prefix_ptr = dict as *const u8;
    let prefix_idx = (*cctx).dictLimit;
    let target = prefix_idx
        .wrapping_add(size as u32)
        .wrapping_sub(LZ4MID_HASHSIZE as u32);
    let mut idx = (*cctx).nextToUpdate;
    if size <= LZ4MID_HASHSIZE {
        return;
    }

    while idx < target {
        let p4 = prefix_ptr.wrapping_add(idx as usize).wrapping_sub(prefix_idx as usize);
        LZ4MID_addPosition(hash4_table, LZ4MID_hash4Ptr(p4), idx);
        let p8 = prefix_ptr
            .wrapping_add(idx as usize)
            .wrapping_add(1)
            .wrapping_sub(prefix_idx as usize);
        LZ4MID_addPosition(hash8_table, LZ4MID_hash8Ptr(p8), idx + 1);
        idx = idx.wrapping_add(3);
    }

    idx = if size > 32 * 1024 + LZ4MID_HASHSIZE {
        target.wrapping_sub(32 * 1024)
    } else {
        (*cctx).nextToUpdate
    };
    while idx < target {
        let p8 = prefix_ptr.wrapping_add(idx as usize).wrapping_sub(prefix_idx as usize);
        LZ4MID_addPosition(hash8_table, LZ4MID_hash8Ptr(p8), idx);
        idx = idx.wrapping_add(1);
    }

    (*cctx).nextToUpdate = target;
}

/* 0 = none, 1 = LZ4MID_searchExtDict, 2 = LZ4MID_searchHCDict */
unsafe fn select_searchDict_function(dict_ctx: *const LZ4HC_CCtx_internal) -> c_int {
    if dict_ctx.is_null() {
        return 0;
    }
    if LZ4HC_getCLevelParams((*dict_ctx).compressionLevel as c_int).strat == LZ4MID {
        return 1;
    }
    2
}

unsafe fn LZ4MID_compress(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    max_output_size: c_int,
    limit: c_int,
    dict: c_int,
) -> c_int {
    let hash4_table = (*ctx).hashTable.as_mut_ptr();
    let hash8_table = hash4_table.add(LZ4MID_HASHTABLESIZE);
    let mut ip: *const u8 = src as *const u8;
    let mut anchor: *const u8 = ip;
    let iend: *const u8 = ip.wrapping_add(*src_size_ptr as usize);
    let mflimit: *const u8 = iend.wrapping_sub(MFLIMIT);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);
    let ilimit: *const u8 = iend.wrapping_sub(LZ4MID_HASHSIZE);
    let mut op: *mut u8 = dst as *mut u8;
    let mut oend: *mut u8 = op.wrapping_add(max_output_size as usize);

    let prefix_ptr: *const u8 = (*ctx).prefixStart;
    let prefix_idx: u32 = (*ctx).dictLimit;
    let ilimit_idx: u32 = (pdiff(ilimit, prefix_ptr) as u32).wrapping_add(prefix_idx);
    let dict_start: *const u8 = (*ctx).dictStart;
    let dict_idx: u32 = (*ctx).lowLimit;
    let g_dict_end_index: u32 = (*ctx).lowLimit;
    let search_into_dict: c_int = if dict == USING_DICT_CTX_HC {
        select_searchDict_function((*ctx).dictCtx)
    } else {
        0
    };
    let mut match_length: u32 = 0;
    let mut match_distance: u32 = 0;

    if *src_size_ptr < 0 {
        return 0;
    }
    if max_output_size < 0 {
        return 0;
    }
    if *src_size_ptr > LZ4_MAX_INPUT_SIZE as c_int {
        return 0;
    }
    if limit == FILL_OUTPUT {
        oend = oend.wrapping_sub(LASTLITERALS);
    }

    let mut overflow = false;

    if *src_size_ptr >= LZ4_MINLENGTH {
        'main: while ip <= mflimit {
            let ip_index: u32 = (pdiff(ip, prefix_ptr) as u32).wrapping_add(prefix_idx);

            let found = 'find: {
                /* search long match */
                {
                    let h8 = LZ4MID_hash8Ptr(ip);
                    let pos8 = *hash8_table.add(h8 as usize);
                    LZ4MID_addPosition(hash8_table, h8, ip_index);
                    if ip_index.wrapping_sub(pos8) <= LZ4_DISTANCE_MAX {
                        if pos8 >= prefix_idx {
                            let match_ptr = prefix_ptr
                                .wrapping_add(pos8 as usize)
                                .wrapping_sub(prefix_idx as usize);
                            match_length = LZ4_count(ip, match_ptr, matchlimit);
                            if match_length >= MINMATCH as u32 {
                                match_distance = ip_index.wrapping_sub(pos8);
                                break 'find true;
                            }
                        } else if pos8 >= dict_idx {
                            let match_ptr =
                                dict_start.wrapping_add(pos8.wrapping_sub(dict_idx) as usize);
                            let a = prefix_idx.wrapping_sub(pos8) as usize;
                            let b = pdiff(matchlimit, ip) as usize;
                            let safe_len = if a < b { a } else { b };
                            match_length =
                                LZ4_count(ip, match_ptr, ip.wrapping_add(safe_len));
                            if match_length >= MINMATCH as u32 {
                                match_distance = ip_index.wrapping_sub(pos8);
                                break 'find true;
                            }
                        }
                    }
                }
                /* search short match */
                {
                    let h4 = LZ4MID_hash4Ptr(ip);
                    let pos4 = *hash4_table.add(h4 as usize);
                    LZ4MID_addPosition(hash4_table, h4, ip_index);
                    if ip_index.wrapping_sub(pos4) <= LZ4_DISTANCE_MAX {
                        if pos4 >= prefix_idx {
                            let match_ptr = prefix_ptr
                                .wrapping_add(pos4.wrapping_sub(prefix_idx) as usize);
                            match_length = LZ4_count(ip, match_ptr, matchlimit);
                            if match_length >= MINMATCH as u32 {
                                let h8 = LZ4MID_hash8Ptr(ip.wrapping_add(1));
                                let pos8 = *hash8_table.add(h8 as usize);
                                let m2_distance = ip_index.wrapping_add(1).wrapping_sub(pos8);
                                match_distance = ip_index.wrapping_sub(pos4);
                                if m2_distance <= LZ4_DISTANCE_MAX
                                    && pos8 >= prefix_idx
                                    && ip < mflimit
                                {
                                    let m2_ptr = prefix_ptr
                                        .wrapping_add(pos8.wrapping_sub(prefix_idx) as usize);
                                    let ml2 =
                                        LZ4_count(ip.wrapping_add(1), m2_ptr, matchlimit);
                                    if ml2 > match_length {
                                        LZ4MID_addPosition(
                                            hash8_table,
                                            h8,
                                            ip_index.wrapping_add(1),
                                        );
                                        ip = ip.wrapping_add(1);
                                        match_length = ml2;
                                        match_distance = m2_distance;
                                    }
                                }
                                break 'find true;
                            }
                        } else if pos4 >= dict_idx {
                            let match_ptr =
                                dict_start.wrapping_add(pos4.wrapping_sub(dict_idx) as usize);
                            let a = prefix_idx.wrapping_sub(pos4) as usize;
                            let b = pdiff(matchlimit, ip) as usize;
                            let safe_len = if a < b { a } else { b };
                            match_length =
                                LZ4_count(ip, match_ptr, ip.wrapping_add(safe_len));
                            if match_length >= MINMATCH as u32 {
                                match_distance = ip_index.wrapping_sub(pos4);
                                break 'find true;
                            }
                        }
                    }
                }
                /* no match found in prefix */
                if dict == USING_DICT_CTX_HC
                    && ip_index.wrapping_sub(g_dict_end_index) < LZ4_DISTANCE_MAX - 8
                {
                    let d_match = if search_into_dict == 1 {
                        LZ4MID_searchExtDict(
                            ip,
                            ip_index,
                            matchlimit,
                            (*ctx).dictCtx,
                            g_dict_end_index,
                        )
                    } else {
                        LZ4MID_searchHCDict(
                            ip,
                            ip_index,
                            matchlimit,
                            (*ctx).dictCtx,
                            g_dict_end_index,
                        )
                    };
                    if d_match.len >= MINMATCH as c_int {
                        match_length = d_match.len as u32;
                        match_distance = d_match.off as u32;
                        break 'find true;
                    }
                }
                false
            };

            if !found {
                ip = ip.wrapping_add(1 + ((pdiff(ip, anchor) >> 9) as usize));
                continue 'main;
            }

            /* _lz4mid_encode_sequence: catch back */
            while ((ip > anchor) & ((pdiff(ip, prefix_ptr) as u32) > match_distance))
                && *ip.wrapping_sub(1)
                    == *ip.wrapping_offset(-(match_distance as isize) - 1)
            {
                ip = ip.wrapping_sub(1);
                match_length += 1;
            }

            /* fill table with beginning of match */
            LZ4MID_addPosition(
                hash8_table,
                LZ4MID_hash8Ptr(ip.wrapping_add(1)),
                ip_index.wrapping_add(1),
            );
            LZ4MID_addPosition(
                hash8_table,
                LZ4MID_hash8Ptr(ip.wrapping_add(2)),
                ip_index.wrapping_add(2),
            );
            LZ4MID_addPosition(
                hash4_table,
                LZ4MID_hash4Ptr(ip.wrapping_add(1)),
                ip_index.wrapping_add(1),
            );

            /* encode */
            {
                let saved_op = op;
                if LZ4HC_encodeSequence(
                    &mut ip,
                    &mut op,
                    &mut anchor,
                    match_length as c_int,
                    match_distance as c_int,
                    limit,
                    oend,
                ) != 0
                {
                    op = saved_op;
                    overflow = true;
                    break 'main;
                }
            }

            /* fill table with end of match */
            {
                let end_match_idx = (pdiff(ip, prefix_ptr) as u32).wrapping_add(prefix_idx);
                let pos_m2 = end_match_idx.wrapping_sub(2);
                if pos_m2 < ilimit_idx {
                    if pdiff(ip, prefix_ptr) > 5 {
                        LZ4MID_addPosition(
                            hash8_table,
                            LZ4MID_hash8Ptr(ip.wrapping_sub(5)),
                            end_match_idx.wrapping_sub(5),
                        );
                    }
                    LZ4MID_addPosition(
                        hash8_table,
                        LZ4MID_hash8Ptr(ip.wrapping_sub(3)),
                        end_match_idx.wrapping_sub(3),
                    );
                    LZ4MID_addPosition(
                        hash8_table,
                        LZ4MID_hash8Ptr(ip.wrapping_sub(2)),
                        end_match_idx.wrapping_sub(2),
                    );
                    LZ4MID_addPosition(
                        hash4_table,
                        LZ4MID_hash4Ptr(ip.wrapping_sub(2)),
                        end_match_idx.wrapping_sub(2),
                    );
                    LZ4MID_addPosition(
                        hash4_table,
                        LZ4MID_hash4Ptr(ip.wrapping_sub(1)),
                        end_match_idx.wrapping_sub(1),
                    );
                }
            }
        }
    }

    if overflow {
        if limit == FILL_OUTPUT {
            let ll = pdiff(ip, anchor) as usize;
            let ll_addbytes = (ll + 240) / 255;
            let ll_total_cost = 1 + ll_addbytes + ll;
            let max_lit_pos = oend.wrapping_sub(3);
            if op.wrapping_add(ll_total_cost) <= max_lit_pos {
                let bytes_left_for_ml =
                    pdiff(max_lit_pos, op.wrapping_add(ll_total_cost)) as usize;
                let max_ml_size =
                    MINMATCH + (ML_MASK as usize - 1) + (bytes_left_for_ml * 255);
                if match_length as usize > max_ml_size {
                    match_length = max_ml_size as u32;
                }
                if (pdiff(oend.wrapping_add(LASTLITERALS), op.wrapping_add(ll_total_cost + 2))
                    - 1
                    + match_length as isize)
                    >= MFLIMIT as isize
                {
                    LZ4HC_encodeSequence(
                        &mut ip,
                        &mut op,
                        &mut anchor,
                        match_length as c_int,
                        match_distance as c_int,
                        NOT_LIMITED,
                        oend,
                    );
                }
            }
            /* fall through to last literals */
        } else {
            return 0;
        }
    }

    /* _lz4mid_last_literals */
    {
        let mut last_run_size = pdiff(iend, anchor) as usize;
        let mut ll_add = (last_run_size + 255 - RUN_MASK as usize) / 255;
        let total_size = 1 + ll_add + last_run_size;
        if limit == FILL_OUTPUT {
            oend = oend.wrapping_add(LASTLITERALS);
        }
        if limit != NOT_LIMITED && op.wrapping_add(total_size) > oend {
            if limit == LIMITED_OUTPUT {
                return 0;
            }
            last_run_size = (pdiff(oend, op) as usize) - 1;
            ll_add = (last_run_size + 256 - RUN_MASK as usize) / 256;
            last_run_size -= ll_add;
        }
        ip = anchor.wrapping_add(last_run_size);

        if last_run_size >= RUN_MASK as usize {
            let mut accumulator = last_run_size - RUN_MASK as usize;
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
            *op = ((last_run_size as u32) << ML_BITS) as u8;
            op = op.wrapping_add(1);
        }
        memcpy_n(op, anchor, last_run_size);
        op = op.wrapping_add(last_run_size);
    }

    *src_size_ptr = pdiff(ip, src as *const u8) as c_int;
    pdiff(op, dst as *const u8) as c_int
}

/* ---- HC compression: search ---- */

unsafe fn LZ4HC_Insert(hc4: *mut LZ4HC_CCtx_internal, ip: *const u8) {
    let chain_table = (*hc4).chainTable.as_mut_ptr();
    let hash_table = (*hc4).hashTable.as_mut_ptr();
    let prefix_ptr = (*hc4).prefixStart;
    let prefix_idx = (*hc4).dictLimit;
    let target = (pdiff(ip, prefix_ptr) as u32).wrapping_add(prefix_idx);
    let mut idx = (*hc4).nextToUpdate;

    while idx < target {
        let h = LZ4HC_hashPtr(
            prefix_ptr
                .wrapping_add(idx as usize)
                .wrapping_sub(prefix_idx as usize),
        );
        let mut delta = idx.wrapping_sub(*hash_table.add(h as usize)) as usize;
        if delta > LZ4_DISTANCE_MAX as usize {
            delta = LZ4_DISTANCE_MAX as usize;
        }
        deltanext_set(chain_table, idx, delta as u16);
        *hash_table.add(h as usize) = idx;
        idx = idx.wrapping_add(1);
    }

    (*hc4).nextToUpdate = target;
}

#[inline(always)]
fn LZ4HC_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}

fn LZ4HC_rotatePattern(rotate: usize, pattern: u32) -> u32 {
    let bits_to_rotate = (rotate & 3) << 3;
    if bits_to_rotate == 0 {
        return pattern;
    }
    LZ4HC_rotl32(pattern, bits_to_rotate as u32)
}

unsafe fn LZ4HC_countPattern(ip0: *const u8, i_end: *const u8, pattern32: u32) -> u32 {
    let i_start = ip0;
    let mut ip = ip0;
    let pattern: u64 = (pattern32 as u64) + ((pattern32 as u64) << 32);

    while ip < i_end.wrapping_sub(7) {
        let diff = read64(ip) ^ pattern;
        if diff == 0 {
            ip = ip.wrapping_add(8);
            continue;
        }
        ip = ip.wrapping_add((diff.trailing_zeros() >> 3) as usize);
        return pdiff(ip, i_start) as u32;
    }

    {
        let mut pattern_byte = pattern;
        while ip < i_end && *ip == (pattern_byte as u8) {
            ip = ip.wrapping_add(1);
            pattern_byte >>= 8;
        }
    }

    pdiff(ip, i_start) as u32
}

unsafe fn LZ4HC_reverseCountPattern(ip0: *const u8, i_low: *const u8, pattern: u32) -> u32 {
    let i_start = ip0;
    let mut ip = ip0;

    while ip >= i_low.wrapping_add(4) {
        if read32(ip.wrapping_sub(4)) != pattern {
            break;
        }
        ip = ip.wrapping_sub(4);
    }
    {
        let pbytes = pattern.to_ne_bytes();
        let mut i: isize = 3;
        while ip > i_low {
            if i < 0 {
                // C reads past the 4-byte object; cannot happen for valid patterns
                break;
            }
            if *ip.wrapping_sub(1) != pbytes[i as usize] {
                break;
            }
            ip = ip.wrapping_sub(1);
            i -= 1;
        }
    }
    pdiff(i_start, ip) as u32
}

fn LZ4HC_protectDictEnd(dict_limit: u32, match_index: u32) -> bool {
    dict_limit.wrapping_sub(1).wrapping_sub(match_index) >= 3
}

unsafe fn LZ4HC_InsertAndGetWiderMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    i_low_limit: *const u8,
    i_high_limit: *const u8,
    mut longest: c_int,
    max_nb_attempts: c_int,
    pattern_analysis: c_int,
    chain_swap: c_int,
    dict: c_int,
    favor_dec_speed: c_int,
) -> LZ4HC_match_t {
    let chain_table = (*hc4).chainTable.as_mut_ptr();
    let hash_table = (*hc4).hashTable.as_mut_ptr();
    let dict_ctx = (*hc4).dictCtx;
    let prefix_ptr = (*hc4).prefixStart;
    let prefix_idx = (*hc4).dictLimit;
    let ip_index = (pdiff(ip, prefix_ptr) as u32).wrapping_add(prefix_idx);
    let within_start_distance =
        (*hc4).lowLimit.wrapping_add(LZ4_DISTANCE_MAX + 1) > ip_index;
    let lowest_match_index = if within_start_distance {
        (*hc4).lowLimit
    } else {
        ip_index.wrapping_sub(LZ4_DISTANCE_MAX)
    };
    let dict_start = (*hc4).dictStart;
    let dict_idx = (*hc4).lowLimit;
    let dict_end = dict_start
        .wrapping_add(prefix_idx as usize)
        .wrapping_sub(dict_idx as usize);
    let look_back_length = pdiff(ip, i_low_limit) as c_int;
    let mut nb_attempts = max_nb_attempts;
    let mut match_chain_pos: u32 = 0;
    let pattern: u32 = read32(ip);
    let mut match_index: u32;
    let mut repeat: c_int = REP_UNTESTED;
    let mut src_pattern_length: usize = 0;
    let mut offset: c_int = 0;
    let mut s_back: c_int = 0;

    LZ4HC_Insert(hc4, ip);
    match_index = *hash_table.add(LZ4HC_hashPtr(ip) as usize);

    while match_index >= lowest_match_index && nb_attempts > 0 {
        let mut match_length: c_int = 0;
        nb_attempts -= 1;
        if favor_dec_speed != 0 && ip_index.wrapping_sub(match_index) < 8 {
            /* skip */
        } else if match_index >= prefix_idx {
            let match_ptr = prefix_ptr.wrapping_add(match_index.wrapping_sub(prefix_idx) as usize);
            if read16(i_low_limit.wrapping_offset(longest as isize - 1))
                == read16(
                    match_ptr
                        .wrapping_offset(-(look_back_length as isize))
                        .wrapping_offset(longest as isize - 1),
                )
            {
                if read32(match_ptr) == pattern {
                    let back = if look_back_length != 0 {
                        LZ4HC_countBack(ip, match_ptr, i_low_limit, prefix_ptr)
                    } else {
                        0
                    };
                    match_length = MINMATCH as c_int
                        + LZ4_count(
                            ip.wrapping_add(MINMATCH),
                            match_ptr.wrapping_add(MINMATCH),
                            i_high_limit,
                        ) as c_int;
                    match_length -= back;
                    if match_length > longest {
                        longest = match_length;
                        offset = ip_index.wrapping_sub(match_index) as c_int;
                        s_back = back;
                    }
                }
            }
        } else {
            let match_ptr = dict_start.wrapping_add(match_index.wrapping_sub(dict_idx) as usize);
            if match_index <= prefix_idx.wrapping_sub(4) && read32(match_ptr) == pattern {
                let mut back;
                let mut v_limit = ip.wrapping_add(prefix_idx.wrapping_sub(match_index) as usize);
                if v_limit > i_high_limit {
                    v_limit = i_high_limit;
                }
                match_length = LZ4_count(
                    ip.wrapping_add(MINMATCH),
                    match_ptr.wrapping_add(MINMATCH),
                    v_limit,
                ) as c_int
                    + MINMATCH as c_int;
                if ip.wrapping_offset(match_length as isize) == v_limit && v_limit < i_high_limit {
                    match_length += LZ4_count(
                        ip.wrapping_offset(match_length as isize),
                        prefix_ptr,
                        i_high_limit,
                    ) as c_int;
                }
                back = if look_back_length != 0 {
                    LZ4HC_countBack(ip, match_ptr, i_low_limit, dict_start)
                } else {
                    0
                };
                match_length -= back;
                if match_length > longest {
                    longest = match_length;
                    offset = ip_index.wrapping_sub(match_index) as c_int;
                    s_back = back;
                }
            }
        }

        if chain_swap != 0 && match_length == longest {
            if match_index.wrapping_add(longest as u32) <= ip_index {
                let k_trigger = 4;
                let mut distance_to_next_match: u32 = 1;
                let end = longest - MINMATCH as c_int + 1;
                let mut step: c_int = 1;
                let mut accel: c_int = 1 << k_trigger;
                let mut pos: c_int = 0;
                while pos < end {
                    let candidate_dist =
                        deltanext_get(chain_table, match_index.wrapping_add(pos as u32));
                    step = accel >> k_trigger;
                    accel += 1;
                    if candidate_dist > distance_to_next_match {
                        distance_to_next_match = candidate_dist;
                        match_chain_pos = pos as u32;
                        accel = 1 << k_trigger;
                    }
                    pos += step;
                }
                if distance_to_next_match > 1 {
                    if distance_to_next_match > match_index {
                        break;
                    }
                    match_index = match_index.wrapping_sub(distance_to_next_match);
                    continue;
                }
            }
        }

        {
            let dist_next_match = deltanext_get(chain_table, match_index);
            if pattern_analysis != 0 && dist_next_match == 1 && match_chain_pos == 0 {
                let match_candidate_idx = match_index.wrapping_sub(1);
                if repeat == REP_UNTESTED {
                    if ((pattern & 0xFFFF) == (pattern >> 16))
                        & ((pattern & 0xFF) == (pattern >> 24))
                    {
                        repeat = REP_CONFIRMED;
                        src_pattern_length =
                            LZ4HC_countPattern(ip.wrapping_add(4), i_high_limit, pattern) as usize
                                + 4;
                    } else {
                        repeat = REP_NOT;
                    }
                }
                if repeat == REP_CONFIRMED
                    && match_candidate_idx >= lowest_match_index
                    && LZ4HC_protectDictEnd(prefix_idx, match_candidate_idx)
                {
                    let ext_dict = match_candidate_idx < prefix_idx;
                    let match_ptr = if ext_dict {
                        dict_start.wrapping_add(match_candidate_idx.wrapping_sub(dict_idx) as usize)
                    } else {
                        prefix_ptr
                            .wrapping_add(match_candidate_idx.wrapping_sub(prefix_idx) as usize)
                    };
                    if read32(match_ptr) == pattern {
                        let i_limit = if ext_dict { dict_end } else { i_high_limit };
                        let mut forward_pattern_length =
                            LZ4HC_countPattern(match_ptr.wrapping_add(4), i_limit, pattern)
                                as usize
                                + 4;
                        if ext_dict
                            && match_ptr.wrapping_add(forward_pattern_length) == i_limit
                        {
                            let rotated_pattern =
                                LZ4HC_rotatePattern(forward_pattern_length, pattern);
                            forward_pattern_length +=
                                LZ4HC_countPattern(prefix_ptr, i_high_limit, rotated_pattern)
                                    as usize;
                        }
                        {
                            let lowest_match_ptr = if ext_dict { dict_start } else { prefix_ptr };
                            let mut back_length =
                                LZ4HC_reverseCountPattern(match_ptr, lowest_match_ptr, pattern)
                                    as usize;
                            let current_segment_length: usize;
                            if !ext_dict
                                && match_ptr.wrapping_sub(back_length) == prefix_ptr
                                && dict_idx < prefix_idx
                            {
                                let rotated_pattern = LZ4HC_rotatePattern(
                                    (0u32.wrapping_sub(back_length as u32)) as usize,
                                    pattern,
                                );
                                back_length += LZ4HC_reverseCountPattern(
                                    dict_end,
                                    dict_start,
                                    rotated_pattern,
                                ) as usize;
                            }
                            /* Limit backLength not go further than lowestMatchIndex */
                            {
                                let a = match_candidate_idx.wrapping_sub(back_length as u32);
                                let mx = if a > lowest_match_index { a } else { lowest_match_index };
                                back_length =
                                    match_candidate_idx.wrapping_sub(mx) as usize;
                            }
                            current_segment_length = back_length + forward_pattern_length;

                            if current_segment_length >= src_pattern_length
                                && forward_pattern_length <= src_pattern_length
                            {
                                let new_match_index = match_candidate_idx
                                    .wrapping_add(forward_pattern_length as u32)
                                    .wrapping_sub(src_pattern_length as u32);
                                if LZ4HC_protectDictEnd(prefix_idx, new_match_index) {
                                    match_index = new_match_index;
                                } else {
                                    match_index = prefix_idx;
                                }
                            } else {
                                let new_match_index =
                                    match_candidate_idx.wrapping_sub(back_length as u32);
                                if !LZ4HC_protectDictEnd(prefix_idx, new_match_index) {
                                    match_index = prefix_idx;
                                } else {
                                    match_index = new_match_index;
                                    if look_back_length == 0 {
                                        let max_ml = if current_segment_length < src_pattern_length
                                        {
                                            current_segment_length
                                        } else {
                                            src_pattern_length
                                        };
                                        if (longest as usize) < max_ml {
                                            if (pdiff(ip, prefix_ptr) as usize)
                                                .wrapping_add(prefix_idx as usize)
                                                .wrapping_sub(match_index as usize)
                                                > LZ4_DISTANCE_MAX as usize
                                            {
                                                break;
                                            }
                                            longest = max_ml as c_int;
                                            offset = ip_index.wrapping_sub(match_index) as c_int;
                                        }
                                        {
                                            let dist_to_next_pattern =
                                                deltanext_get(chain_table, match_index);
                                            if dist_to_next_pattern > match_index {
                                                break;
                                            }
                                            match_index =
                                                match_index.wrapping_sub(dist_to_next_pattern);
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
        match_index = match_index.wrapping_sub(deltanext_get(
            chain_table,
            match_index.wrapping_add(match_chain_pos),
        ));
    }

    if dict == USING_DICT_CTX_HC && nb_attempts > 0 && within_start_distance {
        let dict_end_offset = (pdiff((*dict_ctx).end, (*dict_ctx).prefixStart) as usize)
            + (*dict_ctx).dictLimit as usize;
        let mut dict_match_index = (*dict_ctx).hashTable[LZ4HC_hashPtr(ip) as usize];
        match_index = dict_match_index
            .wrapping_add(lowest_match_index)
            .wrapping_sub(dict_end_offset as u32);
        loop {
            if !(ip_index.wrapping_sub(match_index) <= LZ4_DISTANCE_MAX) {
                break;
            }
            let old = nb_attempts;
            nb_attempts -= 1;
            if old == 0 {
                break;
            }
            let match_ptr = (*dict_ctx)
                .prefixStart
                .wrapping_sub((*dict_ctx).dictLimit as usize)
                .wrapping_add(dict_match_index as usize);

            if read32(match_ptr) == pattern {
                let mut mlt: c_int;
                let back: c_int;
                let mut v_limit = ip
                    .wrapping_add(dict_end_offset.wrapping_sub(dict_match_index as usize));
                if v_limit > i_high_limit {
                    v_limit = i_high_limit;
                }
                mlt = LZ4_count(
                    ip.wrapping_add(MINMATCH),
                    match_ptr.wrapping_add(MINMATCH),
                    v_limit,
                ) as c_int
                    + MINMATCH as c_int;
                back = if look_back_length != 0 {
                    LZ4HC_countBack(ip, match_ptr, i_low_limit, (*dict_ctx).prefixStart)
                } else {
                    0
                };
                mlt -= back;
                if mlt > longest {
                    longest = mlt;
                    offset = ip_index.wrapping_sub(match_index) as c_int;
                    s_back = back;
                }
            }

            {
                let next_offset = deltanext_get((*dict_ctx).chainTable.as_ptr(), dict_match_index);
                dict_match_index = dict_match_index.wrapping_sub(next_offset);
                match_index = match_index.wrapping_sub(next_offset);
            }
        }
    }

    LZ4HC_match_t {
        len: longest,
        off: offset,
        back: s_back,
    }
}

#[inline(always)]
unsafe fn LZ4HC_InsertAndFindBestMatch(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    i_limit: *const u8,
    max_nb_attempts: c_int,
    pattern_analysis: c_int,
    dict: c_int,
) -> LZ4HC_match_t {
    LZ4HC_InsertAndGetWiderMatch(
        hc4,
        ip,
        ip,
        i_limit,
        MINMATCH as c_int - 1,
        max_nb_attempts,
        pattern_analysis,
        0,
        dict,
        FAVOR_COMPRESSION_RATIO,
    )
}

/* ---- hash chain compressor ---- */

unsafe fn LZ4HC_compress_hashChain(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dest: *mut c_char,
    src_size_ptr: *mut c_int,
    max_output_size: c_int,
    max_nb_attempts: c_int,
    limit: c_int,
    dict: c_int,
) -> c_int {
    let input_size = *src_size_ptr;
    let pattern_analysis: c_int = if max_nb_attempts > 128 { 1 } else { 0 };

    let mut ip: *const u8 = source as *const u8;
    let mut anchor: *const u8 = ip;
    let iend: *const u8 = ip.wrapping_add(input_size as usize);
    let mflimit: *const u8 = iend.wrapping_sub(MFLIMIT);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);

    let mut optr: *mut u8 = dest as *mut u8;
    let mut op: *mut u8 = dest as *mut u8;
    let mut oend: *mut u8 = op.wrapping_add(max_output_size as usize);

    let mut start0: *const u8;
    let mut start2: *const u8 = ptr::null();
    let mut start3: *const u8 = ptr::null();
    let mut m0 = NOMATCH;
    let mut m1 = NOMATCH;
    let mut m2 = NOMATCH;
    let mut m3 = NOMATCH;

    *src_size_ptr = 0;
    if limit == FILL_OUTPUT {
        oend = oend.wrapping_sub(LASTLITERALS);
    }

    let mut overflow = false;

    if input_size >= LZ4_MINLENGTH {
        'main: while ip <= mflimit {
            m1 = LZ4HC_InsertAndFindBestMatch(
                ctx,
                ip,
                matchlimit,
                max_nb_attempts,
                pattern_analysis,
                dict,
            );
            if m1.len < MINMATCH as c_int {
                ip = ip.wrapping_add(1);
                continue 'main;
            }

            start0 = ip;
            m0 = m1;

            let mut entry_s3 = false;
            'sm: loop {
                if !entry_s3 {
                    /* _Search2 */
                    if ip.wrapping_offset(m1.len as isize) <= mflimit {
                        start2 = ip.wrapping_offset(m1.len as isize - 2);
                        m2 = LZ4HC_InsertAndGetWiderMatch(
                            ctx,
                            start2,
                            ip,
                            matchlimit,
                            m1.len,
                            max_nb_attempts,
                            pattern_analysis,
                            0,
                            dict,
                            FAVOR_COMPRESSION_RATIO,
                        );
                        start2 = start2.wrapping_offset(m2.back as isize);
                    } else {
                        m2 = NOMATCH;
                    }

                    if m2.len <= m1.len {
                        optr = op;
                        if LZ4HC_encodeSequence(
                            &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                        ) != 0
                        {
                            overflow = true;
                            break 'main;
                        }
                        continue 'main;
                    }

                    if start0 < ip {
                        if start2 < ip.wrapping_offset(m0.len as isize) {
                            ip = start0;
                            m1 = m0;
                        }
                    }

                    if pdiff(start2, ip) < 3 {
                        ip = start2;
                        m1 = m2;
                        entry_s3 = false;
                        continue 'sm;
                    }
                }
                entry_s3 = false;

                /* _Search3 */
                if pdiff(start2, ip) < OPTIMAL_ML as isize {
                    let correction: c_int;
                    let mut new_ml = m1.len;
                    if new_ml > OPTIMAL_ML {
                        new_ml = OPTIMAL_ML;
                    }
                    if ip.wrapping_offset(new_ml as isize)
                        > start2.wrapping_offset(m2.len as isize - MINMATCH as isize)
                    {
                        new_ml = pdiff(start2, ip) as c_int + m2.len - MINMATCH as c_int;
                    }
                    correction = new_ml - pdiff(start2, ip) as c_int;
                    if correction > 0 {
                        start2 = start2.wrapping_offset(correction as isize);
                        m2.len -= correction;
                    }
                }

                if start2.wrapping_offset(m2.len as isize) <= mflimit {
                    start3 = start2.wrapping_offset(m2.len as isize - 3);
                    m3 = LZ4HC_InsertAndGetWiderMatch(
                        ctx,
                        start3,
                        start2,
                        matchlimit,
                        m2.len,
                        max_nb_attempts,
                        pattern_analysis,
                        0,
                        dict,
                        FAVOR_COMPRESSION_RATIO,
                    );
                    start3 = start3.wrapping_offset(m3.back as isize);
                } else {
                    m3 = NOMATCH;
                }

                if m3.len <= m2.len {
                    /* No better match => encode ML1 and ML2 */
                    if start2 < ip.wrapping_offset(m1.len as isize) {
                        m1.len = pdiff(start2, ip) as c_int;
                    }
                    optr = op;
                    if LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                    ) != 0
                    {
                        overflow = true;
                        break 'main;
                    }
                    ip = start2;
                    optr = op;
                    if LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m2.len, m2.off, limit, oend,
                    ) != 0
                    {
                        m1 = m2;
                        overflow = true;
                        break 'main;
                    }
                    continue 'main;
                }

                if start3 < ip.wrapping_offset(m1.len as isize + 3) {
                    if start3 >= ip.wrapping_offset(m1.len as isize) {
                        if start2 < ip.wrapping_offset(m1.len as isize) {
                            let correction =
                                pdiff(ip.wrapping_offset(m1.len as isize), start2) as c_int;
                            start2 = start2.wrapping_offset(correction as isize);
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
                            break 'main;
                        }
                        ip = start3;
                        m1 = m3;

                        start0 = start2;
                        m0 = m2;
                        entry_s3 = false;
                        continue 'sm;
                    }

                    start2 = start3;
                    m2 = m3;
                    entry_s3 = true;
                    continue 'sm;
                }

                /* 3 ascending matches: write the first one */
                if start2 < ip.wrapping_offset(m1.len as isize) {
                    if pdiff(start2, ip) < OPTIMAL_ML as isize {
                        let correction: c_int;
                        if m1.len > OPTIMAL_ML {
                            m1.len = OPTIMAL_ML;
                        }
                        if ip.wrapping_offset(m1.len as isize)
                            > start2.wrapping_offset(m2.len as isize - MINMATCH as isize)
                        {
                            m1.len = pdiff(start2, ip) as c_int + m2.len - MINMATCH as c_int;
                        }
                        correction = m1.len - pdiff(start2, ip) as c_int;
                        if correction > 0 {
                            start2 = start2.wrapping_offset(correction as isize);
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
                    overflow = true;
                    break 'main;
                }

                ip = start2;
                m1 = m2;
                start2 = start3;
                m2 = m3;
                entry_s3 = true;
                continue 'sm;
            }
        }
    }

    if overflow {
        if limit == FILL_OUTPUT {
            let ll = pdiff(ip, anchor) as usize;
            let ll_addbytes = (ll + 240) / 255;
            let ll_total_cost = 1 + ll_addbytes + ll;
            let max_lit_pos = oend.wrapping_sub(3);
            op = optr;
            if op.wrapping_add(ll_total_cost) <= max_lit_pos {
                let bytes_left_for_ml =
                    pdiff(max_lit_pos, op.wrapping_add(ll_total_cost)) as usize;
                let max_ml_size = MINMATCH + (ML_MASK as usize - 1) + (bytes_left_for_ml * 255);
                if m1.len as usize > max_ml_size {
                    m1.len = max_ml_size as c_int;
                }
                if pdiff(oend.wrapping_add(LASTLITERALS), op.wrapping_add(ll_total_cost + 2))
                    - 1
                    + m1.len as isize
                    >= MFLIMIT as isize
                {
                    LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, NOT_LIMITED, oend,
                    );
                }
            }
        } else {
            return 0;
        }
    }

    /* _last_literals */
    {
        let mut last_run_size = pdiff(iend, anchor) as usize;
        let mut ll_add = (last_run_size + 255 - RUN_MASK as usize) / 255;
        let total_size = 1 + ll_add + last_run_size;
        if limit == FILL_OUTPUT {
            oend = oend.wrapping_add(LASTLITERALS);
        }
        if limit != NOT_LIMITED && op.wrapping_add(total_size) > oend {
            if limit == LIMITED_OUTPUT {
                return 0;
            }
            last_run_size = (pdiff(oend, op) as usize) - 1;
            ll_add = (last_run_size + 256 - RUN_MASK as usize) / 256;
            last_run_size -= ll_add;
        }
        ip = anchor.wrapping_add(last_run_size);

        if last_run_size >= RUN_MASK as usize {
            let mut accumulator = last_run_size - RUN_MASK as usize;
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
            *op = ((last_run_size as u32) << ML_BITS) as u8;
            op = op.wrapping_add(1);
        }
        memcpy_n(op, anchor, last_run_size);
        op = op.wrapping_add(last_run_size);
    }

    *src_size_ptr = pdiff(ip, source as *const u8) as c_int;
    pdiff(op, dest as *const u8) as c_int
}

/* ---- optimal parser ---- */

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
    if mlen >= (ML_MASK as c_int + MINMATCH as c_int) {
        price += 1 + ((mlen - (ML_MASK as c_int + MINMATCH as c_int)) / 255);
    }
    price
}

#[inline(always)]
unsafe fn LZ4HC_FindLongerMatch(
    ctx: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    i_high_limit: *const u8,
    min_len: c_int,
    nb_searches: c_int,
    dict: c_int,
    favor_dec_speed: c_int,
) -> LZ4HC_match_t {
    let mut md = LZ4HC_InsertAndGetWiderMatch(
        ctx,
        ip,
        ip,
        i_high_limit,
        min_len,
        nb_searches,
        1,
        1,
        dict,
        favor_dec_speed,
    );
    if md.len <= min_len {
        return NOMATCH;
    }
    if favor_dec_speed != 0 {
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
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    nb_searches: c_int,
    mut sufficient_len: usize,
    limit: c_int,
    full_update: c_int,
    dict: c_int,
    favor_dec_speed: c_int,
) -> c_int {
    let mut retval: c_int = 0;
    let opt = malloc(core::mem::size_of::<LZ4HC_optimal_t>() * (LZ4_OPT_NUM + TRAILING_LITERALS))
        as *mut LZ4HC_optimal_t;

    let mut ip: *const u8 = source as *const u8;
    let mut anchor: *const u8 = ip;
    let iend: *const u8 = ip.wrapping_add(*src_size_ptr as usize);
    let mflimit: *const u8 = iend.wrapping_sub(MFLIMIT);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);
    let mut op: *mut u8 = dst as *mut u8;
    let mut op_saved: *mut u8 = dst as *mut u8;
    let mut oend: *mut u8 = op.wrapping_add(dst_capacity as usize);
    let mut ovml: c_int = MINMATCH as c_int;
    let mut ovoff: c_int = 0;

    if opt.is_null() {
        return retval;
    }

    *src_size_ptr = 0;
    if limit == FILL_OUTPUT {
        oend = oend.wrapping_sub(LASTLITERALS);
    }
    if sufficient_len >= LZ4_OPT_NUM {
        sufficient_len = LZ4_OPT_NUM - 1;
    }

    let mut overflow = false;

    'main: while ip <= mflimit {
        let llen: c_int = pdiff(ip, anchor) as c_int;
        let mut best_mlen: c_int;
        let mut best_off: c_int;
        let mut cur: c_int;
        let mut last_match_pos: c_int = 0;

        let first_match = LZ4HC_FindLongerMatch(
            ctx,
            ip,
            matchlimit,
            MINMATCH as c_int - 1,
            nb_searches,
            dict,
            favor_dec_speed,
        );
        if first_match.len == 0 {
            ip = ip.wrapping_add(1);
            continue 'main;
        }

        if first_match.len as usize > sufficient_len {
            let first_ml = first_match.len;
            op_saved = op;
            if LZ4HC_encodeSequence(
                &mut ip,
                &mut op,
                &mut anchor,
                first_ml,
                first_match.off,
                limit,
                oend,
            ) != 0
            {
                ovml = first_ml;
                ovoff = first_match.off;
                overflow = true;
                break 'main;
            }
            continue 'main;
        }

        /* set prices for first positions (literals) */
        for r_pos in 0..MINMATCH as isize {
            let cost = LZ4HC_literalsPrice(llen + r_pos as c_int);
            let o = opt.offset(r_pos);
            (*o).mlen = 1;
            (*o).off = 0;
            (*o).litlen = llen + r_pos as c_int;
            (*o).price = cost;
        }
        /* set prices using initial match */
        {
            let match_ml = first_match.len;
            let offset = first_match.off;
            let mut mlen = MINMATCH as c_int;
            while mlen <= match_ml {
                let cost = LZ4HC_sequencePrice(llen, mlen);
                let o = opt.offset(mlen as isize);
                (*o).mlen = mlen;
                (*o).off = offset;
                (*o).litlen = llen;
                (*o).price = cost;
                mlen += 1;
            }
        }
        last_match_pos = first_match.len;
        for add_lit in 1..=TRAILING_LITERALS as c_int {
            let o = opt.offset((last_match_pos + add_lit) as isize);
            (*o).mlen = 1;
            (*o).off = 0;
            (*o).litlen = add_lit;
            (*o).price = (*opt.offset(last_match_pos as isize)).price
                + LZ4HC_literalsPrice(add_lit);
        }

        /* check further positions */
        let mut jumped = false;
        best_mlen = 0;
        best_off = 0;
        cur = 1;
        while cur < last_match_pos {
            let cur_ptr = ip.wrapping_offset(cur as isize);

            if cur_ptr > mflimit {
                break;
            }
            if full_update != 0 {
                if (*opt.offset((cur + 1) as isize)).price <= (*opt.offset(cur as isize)).price
                    && (*opt.offset((cur + MINMATCH as c_int) as isize)).price
                        < (*opt.offset(cur as isize)).price + 3
                {
                    cur += 1;
                    continue;
                }
            } else {
                if (*opt.offset((cur + 1) as isize)).price <= (*opt.offset(cur as isize)).price {
                    cur += 1;
                    continue;
                }
            }

            let new_match = if full_update != 0 {
                LZ4HC_FindLongerMatch(
                    ctx,
                    cur_ptr,
                    matchlimit,
                    MINMATCH as c_int - 1,
                    nb_searches,
                    dict,
                    favor_dec_speed,
                )
            } else {
                LZ4HC_FindLongerMatch(
                    ctx,
                    cur_ptr,
                    matchlimit,
                    last_match_pos - cur,
                    nb_searches,
                    dict,
                    favor_dec_speed,
                )
            };
            if new_match.len == 0 {
                cur += 1;
                continue;
            }

            if (new_match.len as usize > sufficient_len)
                || (new_match.len + cur >= LZ4_OPT_NUM as c_int)
            {
                best_mlen = new_match.len;
                best_off = new_match.off;
                last_match_pos = cur + 1;
                jumped = true;
                break;
            }

            /* before match : set price with literals at beginning */
            {
                let base_litlen = (*opt.offset(cur as isize)).litlen;
                let mut litlen = 1;
                while litlen < MINMATCH as c_int {
                    let price = (*opt.offset(cur as isize)).price
                        - LZ4HC_literalsPrice(base_litlen)
                        + LZ4HC_literalsPrice(base_litlen + litlen);
                    let pos = cur + litlen;
                    if price < (*opt.offset(pos as isize)).price {
                        let o = opt.offset(pos as isize);
                        (*o).mlen = 1;
                        (*o).off = 0;
                        (*o).litlen = base_litlen + litlen;
                        (*o).price = price;
                    }
                    litlen += 1;
                }
            }

            /* set prices using match at position = cur */
            {
                let match_ml = new_match.len;
                let mut ml = MINMATCH as c_int;
                while ml <= match_ml {
                    let pos = cur + ml;
                    let offset = new_match.off;
                    let price: c_int;
                    let ll: c_int;
                    if (*opt.offset(cur as isize)).mlen == 1 {
                        ll = (*opt.offset(cur as isize)).litlen;
                        price = (if cur > ll {
                            (*opt.offset((cur - ll) as isize)).price
                        } else {
                            0
                        }) + LZ4HC_sequencePrice(ll, ml);
                    } else {
                        ll = 0;
                        price = (*opt.offset(cur as isize)).price + LZ4HC_sequencePrice(0, ml);
                    }

                    if pos > last_match_pos + TRAILING_LITERALS as c_int
                        || price <= (*opt.offset(pos as isize)).price - favor_dec_speed
                    {
                        if ml == match_ml && last_match_pos < pos {
                            last_match_pos = pos;
                        }
                        let o = opt.offset(pos as isize);
                        (*o).mlen = ml;
                        (*o).off = offset;
                        (*o).litlen = ll;
                        (*o).price = price;
                    }
                    ml += 1;
                }
            }
            /* complete following positions with literals */
            for add_lit in 1..=TRAILING_LITERALS as c_int {
                let o = opt.offset((last_match_pos + add_lit) as isize);
                (*o).mlen = 1;
                (*o).off = 0;
                (*o).litlen = add_lit;
                (*o).price = (*opt.offset(last_match_pos as isize)).price
                    + LZ4HC_literalsPrice(add_lit);
            }

            cur += 1;
        }

        if !jumped {
            best_mlen = (*opt.offset(last_match_pos as isize)).mlen;
            best_off = (*opt.offset(last_match_pos as isize)).off;
            cur = last_match_pos - best_mlen;
        }

        /* encode: */
        {
            let mut candidate_pos = cur;
            let mut selected_match_length = best_mlen;
            let mut selected_offset = best_off;
            loop {
                let next_match_length = (*opt.offset(candidate_pos as isize)).mlen;
                let next_offset = (*opt.offset(candidate_pos as isize)).off;
                (*opt.offset(candidate_pos as isize)).mlen = selected_match_length;
                (*opt.offset(candidate_pos as isize)).off = selected_offset;
                selected_match_length = next_match_length;
                selected_offset = next_offset;
                if next_match_length > candidate_pos {
                    break;
                }
                candidate_pos -= next_match_length;
            }
        }

        /* encode all recorded sequences in order */
        {
            let mut r_pos: c_int = 0;
            while r_pos < last_match_pos {
                let ml = (*opt.offset(r_pos as isize)).mlen;
                let offset = (*opt.offset(r_pos as isize)).off;
                if ml == 1 {
                    ip = ip.wrapping_add(1);
                    r_pos += 1;
                    continue;
                }
                r_pos += ml;
                op_saved = op;
                if LZ4HC_encodeSequence(&mut ip, &mut op, &mut anchor, ml, offset, limit, oend)
                    != 0
                {
                    ovml = ml;
                    ovoff = offset;
                    overflow = true;
                    break 'main;
                }
            }
        }
    }

    if overflow {
        if limit == FILL_OUTPUT {
            let ll = pdiff(ip, anchor) as usize;
            let ll_addbytes = (ll + 240) / 255;
            let ll_total_cost = 1 + ll_addbytes + ll;
            let max_lit_pos = oend.wrapping_sub(3);
            op = op_saved;
            if op.wrapping_add(ll_total_cost) <= max_lit_pos {
                let bytes_left_for_ml =
                    pdiff(max_lit_pos, op.wrapping_add(ll_total_cost)) as usize;
                let max_ml_size = MINMATCH + (ML_MASK as usize - 1) + (bytes_left_for_ml * 255);
                if ovml as usize > max_ml_size {
                    ovml = max_ml_size as c_int;
                }
                if pdiff(oend.wrapping_add(LASTLITERALS), op.wrapping_add(ll_total_cost + 2))
                    - 1
                    + ovml as isize
                    >= MFLIMIT as isize
                {
                    LZ4HC_encodeSequence(
                        &mut ip, &mut op, &mut anchor, ovml, ovoff, NOT_LIMITED, oend,
                    );
                }
            }
        } else {
            free(opt as *mut c_void);
            return 0;
        }
    }

    /* _last_literals */
    {
        let mut last_run_size = pdiff(iend, anchor) as usize;
        let mut ll_add = (last_run_size + 255 - RUN_MASK as usize) / 255;
        let total_size = 1 + ll_add + last_run_size;
        if limit == FILL_OUTPUT {
            oend = oend.wrapping_add(LASTLITERALS);
        }
        if limit != NOT_LIMITED && op.wrapping_add(total_size) > oend {
            if limit == LIMITED_OUTPUT {
                free(opt as *mut c_void);
                return 0;
            }
            last_run_size = (pdiff(oend, op) as usize) - 1;
            ll_add = (last_run_size + 256 - RUN_MASK as usize) / 256;
            last_run_size -= ll_add;
        }
        ip = anchor.wrapping_add(last_run_size);

        if last_run_size >= RUN_MASK as usize {
            let mut accumulator = last_run_size - RUN_MASK as usize;
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
            *op = ((last_run_size as u32) << ML_BITS) as u8;
            op = op.wrapping_add(1);
        }
        memcpy_n(op, anchor, last_run_size);
        op = op.wrapping_add(last_run_size);
    }

    *src_size_ptr = pdiff(ip, source as *const u8) as c_int;
    retval = pdiff(op, dst as *const u8) as c_int;

    free(opt as *mut c_void);
    retval
}

/* ---- generic entry points ---- */

unsafe fn LZ4HC_compress_generic_internal(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: c_int,
    dict: c_int,
) -> c_int {
    if limit == FILL_OUTPUT && dst_capacity < 1 {
        return 0;
    }
    if (*src_size_ptr as u32) > LZ4_MAX_INPUT_SIZE {
        return 0;
    }

    (*ctx).end = (*ctx).end.wrapping_add(*src_size_ptr as usize);
    {
        let c_param = LZ4HC_getCLevelParams(c_level);
        let favor = if (*ctx).favorDecSpeed != 0 {
            FAVOR_DECOMPRESSION_SPEED
        } else {
            FAVOR_COMPRESSION_RATIO
        };
        let result: c_int;

        if c_param.strat == LZ4MID {
            result = LZ4MID_compress(ctx, src, dst, src_size_ptr, dst_capacity, limit, dict);
        } else if c_param.strat == LZ4HC {
            result = LZ4HC_compress_hashChain(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_param.nbSearches,
                limit,
                dict,
            );
        } else {
            result = LZ4HC_compress_optimal(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_param.nbSearches,
                c_param.targetLength as usize,
                limit,
                if c_level >= LZ4HC_CLEVEL_MAX { 1 } else { 0 },
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
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: c_int,
) -> c_int {
    LZ4HC_compress_generic_internal(
        ctx,
        src,
        dst,
        src_size_ptr,
        dst_capacity,
        c_level,
        limit,
        NO_DICT_CTX,
    )
}

unsafe fn isStateCompatible(
    ctx1: *const LZ4HC_CCtx_internal,
    ctx2: *const LZ4HC_CCtx_internal,
) -> c_int {
    let is_mid1 =
        (LZ4HC_getCLevelParams((*ctx1).compressionLevel as c_int).strat == LZ4MID) as c_int;
    let is_mid2 =
        (LZ4HC_getCLevelParams((*ctx2).compressionLevel as c_int).strat == LZ4MID) as c_int;
    ((is_mid1 ^ is_mid2) == 0) as c_int
}

unsafe fn LZ4HC_compress_generic_dictCtx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: c_int,
) -> c_int {
    let position = (pdiff((*ctx).end, (*ctx).prefixStart) as usize)
        + ((*ctx).dictLimit.wrapping_sub((*ctx).lowLimit) as usize);
    if position >= 64 * 1024 {
        (*ctx).dictCtx = ptr::null();
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, src_size_ptr, dst_capacity, c_level, limit)
    } else if position == 0
        && *src_size_ptr > 4 * 1024
        && isStateCompatible(ctx, (*ctx).dictCtx) != 0
    {
        ptr::copy_nonoverlapping(
            (*ctx).dictCtx as *const u8,
            ctx as *mut u8,
            core::mem::size_of::<LZ4HC_CCtx_internal>(),
        );
        LZ4HC_setExternalDict(ctx, src as *const u8);
        (*ctx).compressionLevel = c_level as i16;
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, src_size_ptr, dst_capacity, c_level, limit)
    } else {
        LZ4HC_compress_generic_internal(
            ctx,
            src,
            dst,
            src_size_ptr,
            dst_capacity,
            c_level,
            limit,
            USING_DICT_CTX_HC,
        )
    }
}

unsafe fn LZ4HC_compress_generic(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: c_int,
) -> c_int {
    if (*ctx).dictCtx.is_null() {
        LZ4HC_compress_generic_noDictCtx(ctx, src, dst, src_size_ptr, dst_capacity, c_level, limit)
    } else {
        LZ4HC_compress_generic_dictCtx(ctx, src, dst, src_size_ptr, dst_capacity, c_level, limit)
    }
}

/* ---- public API ---- */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

fn LZ4_streamHC_t_alignment() -> usize {
    core::mem::align_of::<LZ4_streamHC_t>()
}

fn is_aligned(p: *const c_void, alignment: usize) -> bool {
    ((p as usize) & (alignment - 1)) == 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC_fastReset(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut src_size: c_int,
    dst_capacity: c_int,
    compression_level: c_int,
) -> c_int {
    let ctx = state as *mut LZ4HC_CCtx_internal;
    if !is_aligned(state, LZ4_streamHC_t_alignment()) {
        return 0;
    }
    LZ4_resetStreamHC_fast(state as *mut LZ4_streamHC_t, compression_level);
    LZ4HC_init_internal(ctx, src as *const u8);
    if dst_capacity < LZ4_compressBound(src_size) {
        LZ4HC_compress_generic(
            ctx,
            src,
            dst,
            &mut src_size,
            dst_capacity,
            compression_level,
            LIMITED_OUTPUT,
        )
    } else {
        LZ4HC_compress_generic(
            ctx,
            src,
            dst,
            &mut src_size,
            dst_capacity,
            compression_level,
            NOT_LIMITED,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    compression_level: c_int,
) -> c_int {
    let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
    if ctx.is_null() {
        return 0;
    }
    LZ4_compress_HC_extStateHC_fastReset(state, src, dst, src_size, dst_capacity, compression_level)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    compression_level: c_int,
) -> c_int {
    let state_ptr = malloc(core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
    if state_ptr.is_null() {
        return 0;
    }
    let c_size = LZ4_compress_HC_extStateHC(
        state_ptr as *mut c_void,
        src,
        dst,
        src_size,
        dst_capacity,
        compression_level,
    );
    free(state_ptr as *mut c_void);
    c_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_destSize(
    state: *mut c_void,
    source: *const c_char,
    dest: *mut c_char,
    source_size_ptr: *mut c_int,
    target_dest_size: c_int,
    c_level: c_int,
) -> c_int {
    let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
    if ctx.is_null() {
        return 0;
    }
    LZ4HC_init_internal(ctx as *mut LZ4HC_CCtx_internal, source as *const u8);
    LZ4_setCompressionLevel(ctx, c_level);
    LZ4HC_compress_generic(
        ctx as *mut LZ4HC_CCtx_internal,
        source,
        dest,
        source_size_ptr,
        target_dest_size,
        c_level,
        FILL_OUTPUT,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamHC() -> *mut LZ4_streamHC_t {
    let state =
        crate::lz4::calloc(1, core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
    if state.is_null() {
        return ptr::null_mut();
    }
    LZ4_setCompressionLevel(state, LZ4HC_CLEVEL_DEFAULT);
    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamHC(p: *mut LZ4_streamHC_t) -> c_int {
    if p.is_null() {
        return 0;
    }
    free(p as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStreamHC(
    buffer: *mut c_void,
    size: usize,
) -> *mut LZ4_streamHC_t {
    let s = buffer as *mut LZ4_streamHC_t;
    if buffer.is_null() {
        return ptr::null_mut();
    }
    if size < core::mem::size_of::<LZ4_streamHC_t>() {
        return ptr::null_mut();
    }
    if !is_aligned(buffer, LZ4_streamHC_t_alignment()) {
        return ptr::null_mut();
    }
    memset_n(
        buffer as *mut u8,
        0,
        core::mem::size_of::<LZ4HC_CCtx_internal>(),
    );
    LZ4_setCompressionLevel(s, LZ4HC_CLEVEL_DEFAULT);
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC(p: *mut LZ4_streamHC_t, compression_level: c_int) {
    LZ4_initStreamHC(p as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
    LZ4_setCompressionLevel(p, compression_level);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC_fast(
    p: *mut LZ4_streamHC_t,
    compression_level: c_int,
) {
    let s = p as *mut LZ4HC_CCtx_internal;
    if (*s).dirty != 0 {
        LZ4_initStreamHC(p as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
    } else {
        (*s).dictLimit = (*s)
            .dictLimit
            .wrapping_add(pdiff((*s).end, (*s).prefixStart) as u32);
        (*s).prefixStart = ptr::null();
        (*s).end = ptr::null();
        (*s).dictCtx = ptr::null();
    }
    LZ4_setCompressionLevel(p, compression_level);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setCompressionLevel(
    p: *mut LZ4_streamHC_t,
    mut compression_level: c_int,
) {
    if compression_level < 1 {
        compression_level = LZ4HC_CLEVEL_DEFAULT;
    }
    if compression_level > LZ4HC_CLEVEL_MAX {
        compression_level = LZ4HC_CLEVEL_MAX;
    }
    (*(p as *mut LZ4HC_CCtx_internal)).compressionLevel = compression_level as i16;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_favorDecompressionSpeed(p: *mut LZ4_streamHC_t, favor: c_int) {
    (*(p as *mut LZ4HC_CCtx_internal)).favorDecSpeed = (favor != 0) as i8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictHC(
    p: *mut LZ4_streamHC_t,
    mut dictionary: *const c_char,
    mut dict_size: c_int,
) -> c_int {
    let ctx_ptr = p as *mut LZ4HC_CCtx_internal;
    let cp: cParams_t;
    if dict_size > 64 * 1024 {
        dictionary = dictionary.wrapping_add(dict_size as usize - 64 * 1024);
        dict_size = 64 * 1024;
    }
    {
        let c_level = (*ctx_ptr).compressionLevel as c_int;
        LZ4_initStreamHC(p as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
        LZ4_setCompressionLevel(p, c_level);
        cp = LZ4HC_getCLevelParams(c_level);
    }
    LZ4HC_init_internal(ctx_ptr, dictionary as *const u8);
    (*ctx_ptr).end = (dictionary as *const u8).wrapping_add(dict_size as usize);
    if cp.strat == LZ4MID {
        LZ4MID_fillHTable(ctx_ptr, dictionary as *const c_void, dict_size as usize);
    } else {
        if dict_size >= LZ4HC_HASHSIZE as c_int {
            LZ4HC_Insert(ctx_ptr, (*ctx_ptr).end.wrapping_sub(3));
        }
    }
    dict_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_HC_dictionary(
    working_stream: *mut LZ4_streamHC_t,
    dictionary_stream: *const LZ4_streamHC_t,
) {
    (*(working_stream as *mut LZ4HC_CCtx_internal)).dictCtx = if !dictionary_stream.is_null() {
        dictionary_stream as *const LZ4HC_CCtx_internal
    } else {
        ptr::null()
    };
}

unsafe fn LZ4HC_setExternalDict(ctx_ptr: *mut LZ4HC_CCtx_internal, new_block: *const u8) {
    if (*ctx_ptr).end >= (*ctx_ptr).prefixStart.wrapping_add(4)
        && LZ4HC_getCLevelParams((*ctx_ptr).compressionLevel as c_int).strat != LZ4MID
    {
        LZ4HC_Insert(ctx_ptr, (*ctx_ptr).end.wrapping_sub(3));
    }

    (*ctx_ptr).lowLimit = (*ctx_ptr).dictLimit;
    (*ctx_ptr).dictStart = (*ctx_ptr).prefixStart;
    (*ctx_ptr).dictLimit = (*ctx_ptr)
        .dictLimit
        .wrapping_add(pdiff((*ctx_ptr).end, (*ctx_ptr).prefixStart) as u32);
    (*ctx_ptr).prefixStart = new_block;
    (*ctx_ptr).end = new_block;
    (*ctx_ptr).nextToUpdate = (*ctx_ptr).dictLimit;

    (*ctx_ptr).dictCtx = ptr::null();
}

unsafe fn LZ4_compressHC_continue_generic(
    p: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    limit: c_int,
) -> c_int {
    let ctx_ptr = p as *mut LZ4HC_CCtx_internal;
    if (*ctx_ptr).prefixStart.is_null() {
        LZ4HC_init_internal(ctx_ptr, src as *const u8);
    }

    /* Check overflow */
    if (pdiff((*ctx_ptr).end, (*ctx_ptr).prefixStart) as usize)
        .wrapping_add((*ctx_ptr).dictLimit as usize)
        > (2usize << 30)
    {
        let mut dict_size = pdiff((*ctx_ptr).end, (*ctx_ptr).prefixStart) as usize;
        if dict_size > 64 * 1024 {
            dict_size = 64 * 1024;
        }
        LZ4_loadDictHC(
            p,
            ((*ctx_ptr).end as *const c_char).wrapping_sub(dict_size),
            dict_size as c_int,
        );
    }

    /* Check if blocks follow each other */
    if src as *const u8 != (*ctx_ptr).end {
        LZ4HC_setExternalDict(ctx_ptr, src as *const u8);
    }

    /* Check overlapping input/dictionary space */
    {
        let mut source_end = (src as *const u8).wrapping_add(*src_size_ptr as usize);
        let dict_begin = (*ctx_ptr).dictStart;
        let dict_end = (*ctx_ptr)
            .dictStart
            .wrapping_add((*ctx_ptr).dictLimit.wrapping_sub((*ctx_ptr).lowLimit) as usize);
        if source_end > dict_begin && (src as *const u8) < dict_end {
            if source_end > dict_end {
                source_end = dict_end;
            }
            (*ctx_ptr).lowLimit = (*ctx_ptr)
                .lowLimit
                .wrapping_add(pdiff(source_end, (*ctx_ptr).dictStart) as u32);
            (*ctx_ptr).dictStart = (*ctx_ptr)
                .dictStart
                .wrapping_add(pdiff(source_end, (*ctx_ptr).dictStart) as u32 as usize);
            if (*ctx_ptr).dictLimit.wrapping_sub((*ctx_ptr).lowLimit) < LZ4HC_HASHSIZE as u32 {
                (*ctx_ptr).lowLimit = (*ctx_ptr).dictLimit;
                (*ctx_ptr).dictStart = (*ctx_ptr).prefixStart;
            }
        }
    }

    LZ4HC_compress_generic(
        ctx_ptr,
        src,
        dst,
        src_size_ptr,
        dst_capacity,
        (*ctx_ptr).compressionLevel as c_int,
        limit,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue(
    p: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    mut src_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    if dst_capacity < LZ4_compressBound(src_size) {
        LZ4_compressHC_continue_generic(p, src, dst, &mut src_size, dst_capacity, LIMITED_OUTPUT)
    } else {
        LZ4_compressHC_continue_generic(p, src, dst, &mut src_size, dst_capacity, NOT_LIMITED)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue_destSize(
    p: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dest_size: c_int,
) -> c_int {
    LZ4_compressHC_continue_generic(p, src, dst, src_size_ptr, target_dest_size, FILL_OUTPUT)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDictHC(
    p: *mut LZ4_streamHC_t,
    safe_buffer: *mut c_char,
    mut dict_size: c_int,
) -> c_int {
    let stream_ptr = p as *mut LZ4HC_CCtx_internal;
    let prefix_size = pdiff((*stream_ptr).end, (*stream_ptr).prefixStart) as c_int;
    if dict_size > 64 * 1024 {
        dict_size = 64 * 1024;
    }
    if dict_size < 4 {
        dict_size = 0;
    }
    if dict_size > prefix_size {
        dict_size = prefix_size;
    }
    if dict_size > 0 {
        memmove_n(
            safe_buffer as *mut u8,
            (*stream_ptr).end.wrapping_sub(dict_size as usize),
            dict_size as usize,
        );
    }
    {
        let end_index = (pdiff((*stream_ptr).end, (*stream_ptr).prefixStart) as u32)
            .wrapping_add((*stream_ptr).dictLimit);
        (*stream_ptr).end = if safe_buffer.is_null() {
            ptr::null()
        } else {
            (safe_buffer as *const u8).wrapping_add(dict_size as usize)
        };
        (*stream_ptr).prefixStart = safe_buffer as *const u8;
        (*stream_ptr).dictLimit = end_index.wrapping_sub(dict_size as u32);
        (*stream_ptr).lowLimit = end_index.wrapping_sub(dict_size as u32);
        (*stream_ptr).dictStart = (*stream_ptr).prefixStart;
        if (*stream_ptr).nextToUpdate < (*stream_ptr).dictLimit {
            (*stream_ptr).nextToUpdate = (*stream_ptr).dictLimit;
        }
    }
    dict_size
}

/* ---- deprecated wrappers ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, src_size, LZ4_compressBound(src_size), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, src_size, max_dst_size, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    c_level: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, src_size, LZ4_compressBound(src_size), c_level)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
    c_level: c_int,
) -> c_int {
    LZ4_compress_HC(src, dst, src_size, max_dst_size, c_level)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, src_size, LZ4_compressBound(src_size), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, src_size, max_dst_size, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    c_level: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, src_size, LZ4_compressBound(src_size), c_level)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
    c_level: c_int,
) -> c_int {
    LZ4_compress_HC_extStateHC(state, src, dst, src_size, max_dst_size, c_level)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    LZ4_compress_HC_continue(ctx, src, dst, src_size, LZ4_compressBound(src_size))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
) -> c_int {
    LZ4_compress_HC_continue(ctx, src, dst, src_size, max_dst_size)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamStateHC(
    state: *mut c_void,
    input_buffer: *mut c_char,
) -> c_int {
    let hc4 = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
    if hc4.is_null() {
        return 1;
    }
    LZ4HC_init_internal(hc4 as *mut LZ4HC_CCtx_internal, input_buffer as *const u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createHC(input_buffer: *const c_char) -> *mut c_void {
    let hc4 = LZ4_createStreamHC();
    if hc4.is_null() {
        return ptr::null_mut();
    }
    LZ4HC_init_internal(hc4 as *mut LZ4HC_CCtx_internal, input_buffer as *const u8);
    hc4 as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeHC(data: *mut c_void) -> c_int {
    if data.is_null() {
        return 0;
    }
    free(data);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_continue(
    data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut src_size: c_int,
    c_level: c_int,
) -> c_int {
    LZ4HC_compress_generic(
        data as *mut LZ4HC_CCtx_internal,
        src,
        dst,
        &mut src_size,
        0,
        c_level,
        NOT_LIMITED,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_continue(
    data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    mut src_size: c_int,
    dst_capacity: c_int,
    c_level: c_int,
) -> c_int {
    LZ4HC_compress_generic(
        data as *mut LZ4HC_CCtx_internal,
        src,
        dst,
        &mut src_size,
        dst_capacity,
        c_level,
        LIMITED_OUTPUT,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBufferHC(data: *mut c_void) -> *mut c_char {
    let s = data as *mut LZ4HC_CCtx_internal;
    let buffer_start = (*s)
        .prefixStart
        .wrapping_sub((*s).dictLimit as usize)
        .wrapping_add((*s).lowLimit as usize);
    LZ4_resetStreamHC_fast(data as *mut LZ4_streamHC_t, (*s).compressionLevel as c_int);
    buffer_start as *mut c_char
}
