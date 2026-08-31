//! Translation of `lz4hc.c` (LZ4 1.10.0), built with the default
//! `LZ4HC_HEAPMODE == 1`.

use core::ffi::{c_char, c_int, c_void};

use crate::lz4::LimitedOutput::*;
use crate::lz4::*;
use crate::util::*;

/* ===== constants ===== */

pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;

pub const LZ4HC_DICTIONARY_LOGSIZE: usize = 16;
pub const LZ4HC_MAXD: usize = 1 << LZ4HC_DICTIONARY_LOGSIZE;

pub const LZ4HC_HASH_LOG: usize = 15;
pub const LZ4HC_HASHTABLESIZE: usize = 1 << LZ4HC_HASH_LOG;

const OPTIMAL_ML: c_int = ((ML_MASK - 1) + MINMATCH as u32) as c_int;
const LZ4_OPT_NUM: usize = 1 << 12;

const LZ4HC_HASHSIZE: usize = 4;
const LZ4MID_HASHSIZE: usize = 8;
const LZ4MID_HASHLOG: usize = LZ4HC_HASH_LOG - 1;
const LZ4MID_HASHTABLESIZE: usize = 1 << LZ4MID_HASHLOG;

/* ===== state ===== */

#[repr(C)]
pub struct LZ4HC_CCtx_internal {
    pub hash_table: [u32; LZ4HC_HASHTABLESIZE],
    pub chain_table: [u16; LZ4HC_MAXD],
    pub end: *const u8,
    pub prefix_start: *const u8,
    pub dict_start: *const u8,
    pub dict_limit: u32,
    pub low_limit: u32,
    pub next_to_update: u32,
    pub compression_level: i16,
    pub favor_dec_speed: i8,
    pub dirty: i8,
    pub dict_ctx: *const LZ4HC_CCtx_internal,
}

pub const LZ4_STREAMHC_MINSIZE: usize = 262200;

#[repr(C)]
pub union LZ4_streamHC_t {
    pub min_state_size: [u8; LZ4_STREAMHC_MINSIZE],
    pub internal_donotuse: core::mem::ManuallyDrop<LZ4HC_CCtx_internal>,
}

const _: () = assert!(core::mem::size_of::<LZ4HC_CCtx_internal>() == 262192);
const _: () = assert!(core::mem::size_of::<LZ4_streamHC_t>() == LZ4_STREAMHC_MINSIZE);

/* ===== levels ===== */

#[derive(Clone, Copy, PartialEq, Eq)]
enum Strat {
    Lz4mid,
    Lz4hc,
    Lz4opt,
}
use Strat::*;

#[derive(Clone, Copy)]
struct CParams {
    strat: Strat,
    nb_searches: c_int,
    target_length: u32,
}

static K_CL_TABLE: [CParams; (LZ4HC_CLEVEL_MAX + 1) as usize] = [
    CParams { strat: Lz4mid, nb_searches: 2, target_length: 16 },
    CParams { strat: Lz4mid, nb_searches: 2, target_length: 16 },
    CParams { strat: Lz4mid, nb_searches: 2, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 4, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 8, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 16, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 32, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 64, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 128, target_length: 16 },
    CParams { strat: Lz4hc, nb_searches: 256, target_length: 16 },
    CParams { strat: Lz4opt, nb_searches: 96, target_length: 64 },
    CParams { strat: Lz4opt, nb_searches: 512, target_length: 128 },
    CParams { strat: Lz4opt, nb_searches: 16384, target_length: LZ4_OPT_NUM as u32 },
];

fn get_clevel_params(c_level: c_int) -> CParams {
    let mut c_level = c_level;
    if c_level < 1 {
        c_level = LZ4HC_CLEVEL_DEFAULT;
    }
    if c_level > LZ4HC_CLEVEL_MAX {
        c_level = LZ4HC_CLEVEL_MAX;
    }
    K_CL_TABLE[c_level as usize]
}

/* ===== hashing ===== */

#[inline(always)]
fn hash_function(i: u32) -> u32 {
    i.wrapping_mul(2654435761) >> ((MINMATCH * 8) - LZ4HC_HASH_LOG)
}

#[inline(always)]
unsafe fn hc_hash_ptr(p: *const u8) -> u32 {
    unsafe { hash_function(read32(p)) }
}

#[inline(always)]
fn mid_hash4(v: u32) -> u32 {
    v.wrapping_mul(2654435761) >> (32 - LZ4MID_HASHLOG)
}

#[inline(always)]
unsafe fn mid_hash4_ptr(p: *const u8) -> u32 {
    unsafe { mid_hash4(read32(p)) }
}

#[inline(always)]
fn mid_hash7(v: u64) -> u32 {
    ((v << (64 - 56)).wrapping_mul(58295818150454627) >> (64 - LZ4MID_HASHLOG)) as u32
}

#[inline(always)]
unsafe fn mid_hash8_ptr(p: *const u8) -> u32 {
    unsafe { mid_hash7(u64::from_le(read64(p))) }
}

/* ===== match length helpers ===== */

/// `LZ4HC_NbCommonBytes32` on a little-endian target.
#[inline(always)]
fn hc_nb_common_bytes32(val: u32) -> u32 {
    val.leading_zeros() >> 3
}

/// `LZ4HC_countBack` : returns a negative count of common bytes before ip/match.
#[inline(always)]
unsafe fn hc_count_back(
    ip: *const u8,
    r#match: *const u8,
    i_min: *const u8,
    m_min: *const u8,
) -> c_int {
    unsafe {
        let mut back: c_int = 0;
        let a = (i_min as isize) - (ip as isize);
        let b = (m_min as isize) - (r#match as isize);
        let min = (if a > b { a } else { b }) as c_int;

        while (back - min) > 3 {
            let v = read32(ip.wrapping_offset((back - 4) as isize))
                ^ read32(r#match.wrapping_offset((back - 4) as isize));
            if v != 0 {
                return back - hc_nb_common_bytes32(v) as c_int;
            } else {
                back -= 4;
            }
        }
        while back > min
            && *ip.wrapping_offset((back - 1) as isize)
                == *r#match.wrapping_offset((back - 1) as isize)
        {
            back -= 1;
        }
        back
    }
}

/// `DELTANEXTU16(table, pos)`
#[inline(always)]
unsafe fn delta_next_u16(table: *const u16, pos: u32) -> u32 {
    unsafe { *table.add((pos & 0xFFFF) as usize) as u32 }
}

#[inline(always)]
unsafe fn set_delta_next_u16(table: *mut u16, pos: u32, v: u16) {
    unsafe { *table.add((pos & 0xFFFF) as usize) = v }
}

/* ===== init ===== */

unsafe fn hc_clear_tables(hc4: *mut LZ4HC_CCtx_internal) {
    unsafe {
        mem_init(
            (*hc4).hash_table.as_mut_ptr() as *mut u8,
            0,
            LZ4HC_HASHTABLESIZE * 4,
        );
        mem_init(
            (*hc4).chain_table.as_mut_ptr() as *mut u8,
            0xFF,
            LZ4HC_MAXD * 2,
        );
    }
}

unsafe fn hc_init_internal(hc4: *mut LZ4HC_CCtx_internal, start: *const u8) {
    unsafe {
        let buffer_size = (*hc4).end as usize - (*hc4).prefix_start as usize;
        let mut new_starting_offset = buffer_size + (*hc4).dict_limit as usize;
        if new_starting_offset > (1usize << 30) {
            hc_clear_tables(hc4);
            new_starting_offset = 0;
        }
        new_starting_offset += 64 * 1024;
        (*hc4).next_to_update = new_starting_offset as u32;
        (*hc4).prefix_start = start;
        (*hc4).end = start;
        (*hc4).dict_start = start;
        (*hc4).dict_limit = new_starting_offset as u32;
        (*hc4).low_limit = new_starting_offset as u32;
    }
}

/* ===== encode ===== */

/// `LZ4HC_encodeSequence` : returns 0 on success, 1 if the output budget is
/// exhausted.
unsafe fn hc_encode_sequence(
    p_ip: &mut *const u8,
    p_op: &mut *mut u8,
    p_anchor: &mut *const u8,
    match_length: c_int,
    offset: c_int,
    limit: LimitedOutput,
    oend: *mut u8,
) -> c_int {
    unsafe {
        let token = *p_op;
        *p_op = (*p_op).wrapping_add(1);

        /* Encode Literal length */
        let mut length: usize = *p_ip as usize - *p_anchor as usize;
        if limit != NotLimited
            && (*p_op)
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
                **p_op = 255;
                *p_op = (*p_op).add(1);
                len -= 255;
            }
            **p_op = len as u8;
            *p_op = (*p_op).add(1);
        } else {
            *token = ((length as u32) << ML_BITS) as u8;
        }

        /* Copy Literals */
        wild_copy8(*p_op, *p_anchor, (*p_op).wrapping_add(length));
        *p_op = (*p_op).wrapping_add(length);

        /* Encode Offset */
        write_le16(*p_op, offset as u16);
        *p_op = (*p_op).wrapping_add(2);

        /* Encode MatchLength */
        length = match_length as usize - MINMATCH;
        if limit != NotLimited
            && (*p_op)
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
                **p_op = 255;
                *p_op = (*p_op).add(1);
                **p_op = 255;
                *p_op = (*p_op).add(1);
                length -= 510;
            }
            if length >= 255 {
                length -= 255;
                **p_op = 255;
                *p_op = (*p_op).add(1);
            }
            **p_op = length as u8;
            *p_op = (*p_op).add(1);
        } else {
            *token = (*token).wrapping_add(length as u8);
        }

        /* Prepare next loop */
        *p_ip = (*p_ip).wrapping_add(match_length as usize);
        *p_anchor = *p_ip;

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

const NOMATCH: LZ4HC_match_t = LZ4HC_match_t { off: 0, len: 0, back: 0 };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4HC_searchExtDict(
    ip: *const u8,
    ip_index: u32,
    i_low_limit: *const u8,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
    current_best_ml: c_int,
    nb_attempts: c_int,
) -> LZ4HC_match_t {
    unsafe {
        let mut current_best_ml = current_best_ml;
        let mut nb_attempts = nb_attempts;
        let l_dict_end_index = ((*dict_ctx).end as usize - (*dict_ctx).prefix_start as usize)
            + (*dict_ctx).dict_limit as usize;
        let mut l_dict_match_index = (*dict_ctx).hash_table[hc_hash_ptr(ip) as usize];
        let mut match_index = l_dict_match_index
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
                .prefix_start
                .wrapping_sub((*dict_ctx).dict_limit as usize)
                .wrapping_add(l_dict_match_index as usize);

            if read32(match_ptr) == read32(ip) {
                let mut v_limit =
                    ip.wrapping_add(l_dict_end_index - l_dict_match_index as usize);
                if v_limit > i_high_limit {
                    v_limit = i_high_limit;
                }
                let mut mlt = lz4_count(
                    ip.wrapping_add(MINMATCH),
                    match_ptr.wrapping_add(MINMATCH),
                    v_limit,
                ) as c_int
                    + MINMATCH as c_int;
                let back = if ip > i_low_limit {
                    hc_count_back(ip, match_ptr, i_low_limit, (*dict_ctx).prefix_start)
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

            let next_offset = delta_next_u16((*dict_ctx).chain_table.as_ptr(), l_dict_match_index);
            l_dict_match_index = l_dict_match_index.wrapping_sub(next_offset);
            match_index = match_index.wrapping_sub(next_offset);
        }

        LZ4HC_match_t { off: offset, len: current_best_ml, back: s_back }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchDictKind {
    None,
    ExtDict,
    HcDict,
}

unsafe fn mid_search_hc_dict(
    ip: *const u8,
    ip_index: u32,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
) -> LZ4HC_match_t {
    unsafe {
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
}

unsafe fn mid_search_ext_dict(
    ip: *const u8,
    ip_index: u32,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
) -> LZ4HC_match_t {
    unsafe {
        let l_dict_end_index = ((*dict_ctx).end as usize - (*dict_ctx).prefix_start as usize)
            + (*dict_ctx).dict_limit as usize;
        let hash4_table = (*dict_ctx).hash_table.as_ptr();
        let hash8_table = hash4_table.add(LZ4MID_HASHTABLESIZE);

        /* search long match first */
        {
            let l8 = *hash8_table.add(mid_hash8_ptr(ip) as usize);
            let m8_index = l8
                .wrapping_add(g_dict_end_index)
                .wrapping_sub(l_dict_end_index as u32);
            if ip_index.wrapping_sub(m8_index) <= LZ4_DISTANCE_MAX {
                let match_ptr = (*dict_ctx)
                    .prefix_start
                    .wrapping_sub((*dict_ctx).dict_limit as usize)
                    .wrapping_add(l8 as usize);
                let a = l_dict_end_index.wrapping_sub(l8 as usize);
                let b = i_high_limit as usize - ip as usize;
                let safe_len = if a < b { a } else { b };
                let mlt = lz4_count(ip, match_ptr, ip.wrapping_add(safe_len)) as c_int;
                if mlt >= MINMATCH as c_int {
                    return LZ4HC_match_t {
                        off: ip_index.wrapping_sub(m8_index) as c_int,
                        len: mlt,
                        back: 0,
                    };
                }
            }
        }

        /* search for short match second */
        {
            let l4 = *hash4_table.add(mid_hash4_ptr(ip) as usize);
            let m4_index = l4
                .wrapping_add(g_dict_end_index)
                .wrapping_sub(l_dict_end_index as u32);
            if ip_index.wrapping_sub(m4_index) <= LZ4_DISTANCE_MAX {
                let match_ptr = (*dict_ctx)
                    .prefix_start
                    .wrapping_sub((*dict_ctx).dict_limit as usize)
                    .wrapping_add(l4 as usize);
                let a = l_dict_end_index.wrapping_sub(l4 as usize);
                let b = i_high_limit as usize - ip as usize;
                let safe_len = if a < b { a } else { b };
                let mlt = lz4_count(ip, match_ptr, ip.wrapping_add(safe_len)) as c_int;
                if mlt >= MINMATCH as c_int {
                    return LZ4HC_match_t {
                        off: ip_index.wrapping_sub(m4_index) as c_int,
                        len: mlt,
                        back: 0,
                    };
                }
            }
        }

        NOMATCH
    }
}

#[inline(always)]
unsafe fn mid_search_into_dict(
    kind: SearchDictKind,
    ip: *const u8,
    ip_index: u32,
    i_high_limit: *const u8,
    dict_ctx: *const LZ4HC_CCtx_internal,
    g_dict_end_index: u32,
) -> LZ4HC_match_t {
    unsafe {
        match kind {
            SearchDictKind::ExtDict => {
                mid_search_ext_dict(ip, ip_index, i_high_limit, dict_ctx, g_dict_end_index)
            }
            SearchDictKind::HcDict => {
                mid_search_hc_dict(ip, ip_index, i_high_limit, dict_ctx, g_dict_end_index)
            }
            SearchDictKind::None => NOMATCH,
        }
    }
}

fn select_search_dict_function(dict_ctx: *const LZ4HC_CCtx_internal) -> SearchDictKind {
    if dict_ctx.is_null() {
        return SearchDictKind::None;
    }
    if get_clevel_params(unsafe { (*dict_ctx).compression_level } as c_int).strat == Lz4mid {
        SearchDictKind::ExtDict
    } else {
        SearchDictKind::HcDict
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DictCtxDirective {
    NoDictCtx,
    UsingDictCtxHc,
}
use DictCtxDirective::*;

/* ===== mid compression (level 2) ===== */

#[inline(always)]
unsafe fn mid_add_position(h_table: *mut u32, h_value: u32, index: u32) {
    unsafe { *h_table.add(h_value as usize) = index }
}

unsafe fn mid_fill_htable(cctx: *mut LZ4HC_CCtx_internal, dict: *const c_void, size: usize) {
    unsafe {
        let hash4_table = (*cctx).hash_table.as_mut_ptr();
        let hash8_table = hash4_table.add(LZ4MID_HASHTABLESIZE);
        let prefix_ptr = dict as *const u8;
        let prefix_idx = (*cctx).dict_limit;
        let target = prefix_idx
            .wrapping_add(size as u32)
            .wrapping_sub(LZ4MID_HASHSIZE as u32);
        let mut idx = (*cctx).next_to_update;
        if size <= LZ4MID_HASHSIZE {
            return;
        }

        while idx < target {
            let p4 = prefix_ptr
                .wrapping_add(idx as usize)
                .wrapping_sub(prefix_idx as usize);
            mid_add_position(hash4_table, mid_hash4_ptr(p4), idx);
            let p8 = prefix_ptr
                .wrapping_add(idx as usize + 1)
                .wrapping_sub(prefix_idx as usize);
            mid_add_position(hash8_table, mid_hash8_ptr(p8), idx + 1);
            idx = idx.wrapping_add(3);
        }

        idx = if size > 32 * 1024 + LZ4MID_HASHSIZE {
            target.wrapping_sub(32 * 1024)
        } else {
            (*cctx).next_to_update
        };
        while idx < target {
            let p8 = prefix_ptr
                .wrapping_add(idx as usize)
                .wrapping_sub(prefix_idx as usize);
            mid_add_position(hash8_table, mid_hash8_ptr(p8), idx);
            idx = idx.wrapping_add(1);
        }

        (*cctx).next_to_update = target;
    }
}

unsafe fn mid_compress(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    max_output_size: c_int,
    limit: LimitedOutput,
    dict: DictCtxDirective,
) -> c_int {
    unsafe {
        let hash4_table = (*ctx).hash_table.as_mut_ptr();
        let hash8_table = hash4_table.add(LZ4MID_HASHTABLESIZE);
        let mut ip = src as *const u8;
        let mut anchor = ip;
        let iend = ip.wrapping_add(*src_size_ptr as usize);
        let mflimit = iend.wrapping_sub(MFLIMIT);
        let matchlimit = iend.wrapping_sub(LASTLITERALS);
        let ilimit = iend.wrapping_sub(LZ4MID_HASHSIZE);
        let mut op = dst as *mut u8;
        let mut oend = op.wrapping_add(max_output_size as usize);

        let prefix_ptr = (*ctx).prefix_start;
        let prefix_idx = (*ctx).dict_limit;
        let ilimit_idx = (ilimit as usize - prefix_ptr as usize) as u32 + prefix_idx;
        let dict_start = (*ctx).dict_start;
        let dict_idx = (*ctx).low_limit;
        let g_dict_end_index = (*ctx).low_limit;
        let search_kind = if dict == UsingDictCtxHc {
            select_search_dict_function((*ctx).dict_ctx)
        } else {
            SearchDictKind::None
        };
        let mut match_length: u32;
        let mut match_distance: u32;

        /* input sanitization */
        if *src_size_ptr < 0 {
            return 0;
        }
        if max_output_size < 0 {
            return 0;
        }
        if *src_size_ptr > LZ4_MAX_INPUT_SIZE {
            return 0;
        }
        if limit == FillOutput {
            oend = oend.wrapping_sub(LASTLITERALS);
        }

        /* set when leaving the main loop through the dest-overflow path */
        let mut overflow = false;

        if *src_size_ptr >= LZ4_MIN_LENGTH {
            'main: while ip <= mflimit {
                let ip_index = (ip as usize - prefix_ptr as usize) as u32 + prefix_idx;
                let mut found = false;
                match_length = 0;
                match_distance = 0;

                /* search long match */
                {
                    let h8 = mid_hash8_ptr(ip);
                    let pos8 = *hash8_table.add(h8 as usize);
                    mid_add_position(hash8_table, h8, ip_index);
                    if ip_index.wrapping_sub(pos8) <= LZ4_DISTANCE_MAX {
                        if pos8 >= prefix_idx {
                            let match_ptr = prefix_ptr
                                .wrapping_add(pos8 as usize)
                                .wrapping_sub(prefix_idx as usize);
                            match_length = lz4_count(ip, match_ptr, matchlimit);
                            if match_length >= MINMATCH as u32 {
                                match_distance = ip_index.wrapping_sub(pos8);
                                found = true;
                            }
                        } else if pos8 >= dict_idx {
                            let match_ptr =
                                dict_start.wrapping_add(pos8.wrapping_sub(dict_idx) as usize);
                            let a = prefix_idx.wrapping_sub(pos8) as usize;
                            let b = matchlimit as usize - ip as usize;
                            let safe_len = if a < b { a } else { b };
                            match_length = lz4_count(ip, match_ptr, ip.wrapping_add(safe_len));
                            if match_length >= MINMATCH as u32 {
                                match_distance = ip_index.wrapping_sub(pos8);
                                found = true;
                            }
                        }
                    }
                }

                /* search short match */
                if !found {
                    let h4 = mid_hash4_ptr(ip);
                    let pos4 = *hash4_table.add(h4 as usize);
                    mid_add_position(hash4_table, h4, ip_index);
                    if ip_index.wrapping_sub(pos4) <= LZ4_DISTANCE_MAX {
                        if pos4 >= prefix_idx {
                            let match_ptr = prefix_ptr
                                .wrapping_add(pos4.wrapping_sub(prefix_idx) as usize);
                            match_length = lz4_count(ip, match_ptr, matchlimit);
                            if match_length >= MINMATCH as u32 {
                                /* short match found, check ip+1 for a longer one */
                                let h8 = mid_hash8_ptr(ip.wrapping_add(1));
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
                                        lz4_count(ip.wrapping_add(1), m2_ptr, matchlimit);
                                    if ml2 > match_length {
                                        mid_add_position(
                                            hash8_table,
                                            h8,
                                            ip_index.wrapping_add(1),
                                        );
                                        ip = ip.wrapping_add(1);
                                        match_length = ml2;
                                        match_distance = m2_distance;
                                    }
                                }
                                found = true;
                            }
                        } else if pos4 >= dict_idx {
                            let match_ptr =
                                dict_start.wrapping_add(pos4.wrapping_sub(dict_idx) as usize);
                            let a = prefix_idx.wrapping_sub(pos4) as usize;
                            let b = matchlimit as usize - ip as usize;
                            let safe_len = if a < b { a } else { b };
                            match_length = lz4_count(ip, match_ptr, ip.wrapping_add(safe_len));
                            if match_length >= MINMATCH as u32 {
                                match_distance = ip_index.wrapping_sub(pos4);
                                found = true;
                            }
                        }
                    }
                }

                /* no match found in prefix : try the external dictionary */
                if !found
                    && dict == UsingDictCtxHc
                    && ip_index.wrapping_sub(g_dict_end_index) < LZ4_DISTANCE_MAX - 8
                {
                    let d_match = mid_search_into_dict(
                        search_kind,
                        ip,
                        ip_index,
                        matchlimit,
                        (*ctx).dict_ctx,
                        g_dict_end_index,
                    );
                    if d_match.len >= MINMATCH as c_int {
                        match_length = d_match.len as u32;
                        match_distance = d_match.off as u32;
                        found = true;
                    }
                }

                if !found {
                    /* skip faster over incompressible data */
                    let step = 1 + (((ip as isize - anchor as isize) >> 9) as usize);
                    ip = ip.wrapping_add(step);
                    continue 'main;
                }

                /* _lz4mid_encode_sequence */
                /* catch back */
                while ((ip > anchor)
                    & (((ip as usize - prefix_ptr as usize) as u32) > match_distance))
                    && *ip.wrapping_sub(1)
                        == *ip.wrapping_offset(-(match_distance as isize) - 1)
                {
                    ip = ip.wrapping_sub(1);
                    match_length += 1;
                }

                /* fill table with beginning of match */
                mid_add_position(
                    hash8_table,
                    mid_hash8_ptr(ip.wrapping_add(1)),
                    ip_index.wrapping_add(1),
                );
                mid_add_position(
                    hash8_table,
                    mid_hash8_ptr(ip.wrapping_add(2)),
                    ip_index.wrapping_add(2),
                );
                mid_add_position(
                    hash4_table,
                    mid_hash4_ptr(ip.wrapping_add(1)),
                    ip_index.wrapping_add(1),
                );

                /* encode */
                {
                    let saved_op = op;
                    if hc_encode_sequence(
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
                        /* _lz4mid_dest_overflow */
                        if limit == FillOutput {
                            let ll = ip as usize - anchor as usize;
                            let ll_addbytes = (ll + 240) / 255;
                            let ll_total_cost = 1 + ll_addbytes + ll;
                            let max_lit_pos = oend.wrapping_sub(3);
                            if op.wrapping_add(ll_total_cost) <= max_lit_pos {
                                let bytes_left_for_ml = max_lit_pos as usize
                                    - (op.wrapping_add(ll_total_cost) as usize);
                                let max_ml_size = MINMATCH
                                    + (ML_MASK as usize - 1)
                                    + (bytes_left_for_ml * 255);
                                if match_length as usize > max_ml_size {
                                    match_length = max_ml_size as u32;
                                }
                                let lhs = (oend.wrapping_add(LASTLITERALS) as isize)
                                    - (op.wrapping_add(ll_total_cost + 2) as isize)
                                    - 1
                                    + match_length as isize;
                                if lhs >= MFLIMIT as isize {
                                    hc_encode_sequence(
                                        &mut ip,
                                        &mut op,
                                        &mut anchor,
                                        match_length as c_int,
                                        match_distance as c_int,
                                        NotLimited,
                                        oend,
                                    );
                                }
                            }
                            overflow = true;
                            break 'main;
                        }
                        /* compression failed */
                        return 0;
                    }
                }

                /* fill table with end of match */
                {
                    let end_match_idx = (ip as usize - prefix_ptr as usize) as u32 + prefix_idx;
                    let pos_m2 = end_match_idx.wrapping_sub(2);
                    if pos_m2 < ilimit_idx {
                        if (ip as isize - prefix_ptr as isize) > 5 {
                            mid_add_position(
                                hash8_table,
                                mid_hash8_ptr(ip.wrapping_sub(5)),
                                end_match_idx.wrapping_sub(5),
                            );
                        }
                        mid_add_position(
                            hash8_table,
                            mid_hash8_ptr(ip.wrapping_sub(3)),
                            end_match_idx.wrapping_sub(3),
                        );
                        mid_add_position(
                            hash8_table,
                            mid_hash8_ptr(ip.wrapping_sub(2)),
                            end_match_idx.wrapping_sub(2),
                        );
                        mid_add_position(
                            hash4_table,
                            mid_hash4_ptr(ip.wrapping_sub(2)),
                            end_match_idx.wrapping_sub(2),
                        );
                        mid_add_position(
                            hash4_table,
                            mid_hash4_ptr(ip.wrapping_sub(1)),
                            end_match_idx.wrapping_sub(1),
                        );
                    }
                }
            }
        }
        let _ = overflow;

        /* _lz4mid_last_literals */
        {
            let mut last_run_size = iend as usize - anchor as usize;
            let mut ll_add = (last_run_size + 255 - RUN_MASK as usize) / 255;
            let total_size = 1 + ll_add + last_run_size;
            if limit == FillOutput {
                oend = oend.wrapping_add(LASTLITERALS);
            }
            if limit != NotLimited && op.wrapping_add(total_size) > oend {
                if limit == Limited {
                    return 0;
                }
                last_run_size = (oend as usize - op as usize) - 1;
                ll_add = (last_run_size + 256 - RUN_MASK as usize) / 256;
                last_run_size -= ll_add;
            }
            ip = anchor.wrapping_add(last_run_size);

            if last_run_size >= RUN_MASK as usize {
                let mut accumulator = last_run_size - RUN_MASK as usize;
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
                *op = ((last_run_size as u32) << ML_BITS) as u8;
                op = op.add(1);
            }
            mem_copy(op, anchor, last_run_size);
            op = op.wrapping_add(last_run_size);
        }

        *src_size_ptr = (ip as usize - src as usize) as c_int;
        (op as usize - dst as usize) as c_int
    }
}

/* ===== HC compression : search ===== */

/// Update chains up to `ip` (excluded).
#[inline(always)]
unsafe fn hc_insert(hc4: *mut LZ4HC_CCtx_internal, ip: *const u8) {
    unsafe {
        let chain_table = (*hc4).chain_table.as_mut_ptr();
        let hash_table = (*hc4).hash_table.as_mut_ptr();
        let prefix_ptr = (*hc4).prefix_start;
        let prefix_idx = (*hc4).dict_limit;
        let target = (ip as usize - prefix_ptr as usize) as u32 + prefix_idx;
        let mut idx = (*hc4).next_to_update;

        while idx < target {
            let h = hc_hash_ptr(
                prefix_ptr
                    .wrapping_add(idx as usize)
                    .wrapping_sub(prefix_idx as usize),
            );
            let mut delta = idx.wrapping_sub(*hash_table.add(h as usize)) as usize;
            if delta > LZ4_DISTANCE_MAX as usize {
                delta = LZ4_DISTANCE_MAX as usize;
            }
            set_delta_next_u16(chain_table, idx, delta as u16);
            *hash_table.add(h as usize) = idx;
            idx += 1;
        }

        (*hc4).next_to_update = target;
    }
}

fn hc_rotate_pattern(rotate: usize, pattern: u32) -> u32 {
    let bits_to_rotate = (rotate & (core::mem::size_of::<u32>() - 1)) << 3;
    if bits_to_rotate == 0 {
        return pattern;
    }
    pattern.rotate_left(bits_to_rotate as u32)
}

unsafe fn hc_count_pattern(ip: *const u8, i_end: *const u8, pattern32: u32) -> u32 {
    unsafe {
        let i_start = ip;
        let mut ip = ip;
        let pattern: u64 = (pattern32 as u64).wrapping_add((pattern32 as u64) << 32);

        while ip < i_end.wrapping_sub(7) {
            let diff = read_arch(ip) ^ pattern;
            if diff == 0 {
                ip = ip.wrapping_add(8);
                continue;
            }
            ip = ip.wrapping_add(lz4_nb_common_bytes(diff) as usize);
            return (ip as usize - i_start as usize) as u32;
        }

        let mut pattern_byte = pattern;
        while ip < i_end && *ip == (pattern_byte as u8) {
            ip = ip.wrapping_add(1);
            pattern_byte >>= 8;
        }

        (ip as usize - i_start as usize) as u32
    }
}

unsafe fn hc_reverse_count_pattern(ip: *const u8, i_low: *const u8, pattern: u32) -> u32 {
    unsafe {
        let i_start = ip;
        let mut ip = ip;

        while ip >= i_low.wrapping_add(4) {
            if read32(ip.wrapping_sub(4)) != pattern {
                break;
            }
            ip = ip.wrapping_sub(4);
        }
        {
            /* walks backwards from byte 3 of `pattern` in memory order */
            let bytes = pattern.to_ne_bytes();
            let mut bi: isize = 3;
            while ip > i_low {
                if bi < 0 {
                    break;
                }
                if *ip.wrapping_sub(1) != bytes[bi as usize] {
                    break;
                }
                ip = ip.wrapping_sub(1);
                bi -= 1;
            }
        }
        (i_start as usize - ip as usize) as u32
    }
}

/// True when the match index is far enough from the end of the dictionary that
/// reading MINMATCH bytes cannot overflow.
#[inline(always)]
fn hc_protect_dict_end(dict_limit: u32, match_index: u32) -> bool {
    dict_limit.wrapping_sub(1).wrapping_sub(match_index) >= 3
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RepeatState {
    Untested,
    Not,
    Confirmed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HCfavor {
    FavorCompressionRatio,
    FavorDecompressionSpeed,
}

unsafe fn hc_insert_and_get_wider_match(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    i_low_limit: *const u8,
    i_high_limit: *const u8,
    longest_in: c_int,
    max_nb_attempts: c_int,
    pattern_analysis: bool,
    chain_swap: bool,
    dict: DictCtxDirective,
    favor_dec_speed: HCfavor,
) -> LZ4HC_match_t {
    unsafe {
        let mut longest = longest_in;
        let chain_table = (*hc4).chain_table.as_mut_ptr();
        let hash_table = (*hc4).hash_table.as_ptr();
        let dict_ctx = (*hc4).dict_ctx;
        let prefix_ptr = (*hc4).prefix_start;
        let prefix_idx = (*hc4).dict_limit;
        let ip_index = (ip as usize - prefix_ptr as usize) as u32 + prefix_idx;
        let within_start_distance =
            (*hc4).low_limit.wrapping_add(LZ4_DISTANCE_MAX + 1) > ip_index;
        let lowest_match_index = if within_start_distance {
            (*hc4).low_limit
        } else {
            ip_index.wrapping_sub(LZ4_DISTANCE_MAX)
        };
        let dict_start = (*hc4).dict_start;
        let dict_idx = (*hc4).low_limit;
        let dict_end = dict_start
            .wrapping_add(prefix_idx as usize)
            .wrapping_sub(dict_idx as usize);
        let look_back_length = (ip as isize - i_low_limit as isize) as c_int;
        let mut nb_attempts = max_nb_attempts;
        let mut match_chain_pos: u32 = 0;
        let pattern = read32(ip);
        let mut match_index: u32;
        let mut repeat = RepeatState::Untested;
        let mut src_pattern_length: usize = 0;
        let mut offset: c_int = 0;
        let mut s_back: c_int = 0;

        let favor_dec = favor_dec_speed == HCfavor::FavorDecompressionSpeed;

        /* First Match */
        hc_insert(hc4, ip);
        match_index = *hash_table.add(hc_hash_ptr(ip) as usize);

        'chain: while match_index >= lowest_match_index && nb_attempts > 0 {
            let mut match_length: c_int = 0;
            nb_attempts -= 1;

            if favor_dec && ip_index.wrapping_sub(match_index) < 8 {
                /* favorDecSpeed intentionally skips matches with offset < 8 */
            } else if match_index >= prefix_idx {
                /* within current Prefix */
                let match_ptr =
                    prefix_ptr.wrapping_add(match_index.wrapping_sub(prefix_idx) as usize);
                if read16(i_low_limit.wrapping_offset((longest - 1) as isize))
                    == read16(
                        match_ptr
                            .wrapping_offset(-(look_back_length as isize))
                            .wrapping_offset((longest - 1) as isize),
                    )
                {
                    if read32(match_ptr) == pattern {
                        let back = if look_back_length != 0 {
                            hc_count_back(ip, match_ptr, i_low_limit, prefix_ptr)
                        } else {
                            0
                        };
                        match_length = MINMATCH as c_int
                            + lz4_count(
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
                /* lowestMatchIndex <= matchIndex < dictLimit : within Ext Dict */
                let match_ptr =
                    dict_start.wrapping_add(match_index.wrapping_sub(dict_idx) as usize);
                if match_index <= prefix_idx.wrapping_sub(4) && read32(match_ptr) == pattern {
                    let mut v_limit =
                        ip.wrapping_add(prefix_idx.wrapping_sub(match_index) as usize);
                    if v_limit > i_high_limit {
                        v_limit = i_high_limit;
                    }
                    match_length = lz4_count(
                        ip.wrapping_add(MINMATCH),
                        match_ptr.wrapping_add(MINMATCH),
                        v_limit,
                    ) as c_int
                        + MINMATCH as c_int;
                    if ip.wrapping_add(match_length as usize) == v_limit && v_limit < i_high_limit
                    {
                        match_length += lz4_count(
                            ip.wrapping_add(match_length as usize),
                            prefix_ptr,
                            i_high_limit,
                        ) as c_int;
                    }
                    let back = if look_back_length != 0 {
                        hc_count_back(ip, match_ptr, i_low_limit, dict_start)
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

            if chain_swap && match_length == longest {
                /* better match => select a better chain */
                if match_index.wrapping_add(longest as u32) <= ip_index {
                    const K_TRIGGER: i32 = 4;
                    let mut distance_to_next_match: u32 = 1;
                    let end = longest - MINMATCH as c_int + 1;
                    let mut step: c_int = 1;
                    let mut accel: i32 = 1 << K_TRIGGER;
                    let mut pos: c_int = 0;
                    while pos < end {
                        let candidate_dist =
                            delta_next_u16(chain_table, match_index.wrapping_add(pos as u32));
                        step = accel >> K_TRIGGER;
                        accel += 1;
                        if candidate_dist > distance_to_next_match {
                            distance_to_next_match = candidate_dist;
                            match_chain_pos = pos as u32;
                            accel = 1 << K_TRIGGER;
                        }
                        pos += step;
                    }
                    if distance_to_next_match > 1 {
                        if distance_to_next_match > match_index {
                            break 'chain;
                        }
                        match_index = match_index.wrapping_sub(distance_to_next_match);
                        continue 'chain;
                    }
                }
            }

            {
                let dist_next_match = delta_next_u16(chain_table, match_index);
                if pattern_analysis && dist_next_match == 1 && match_chain_pos == 0 {
                    let match_candidate_idx = match_index.wrapping_sub(1);
                    /* may be a repeated pattern */
                    if repeat == RepeatState::Untested {
                        if ((pattern & 0xFFFF) == (pattern >> 16))
                            && ((pattern & 0xFF) == (pattern >> 24))
                        {
                            repeat = RepeatState::Confirmed;
                            src_pattern_length = hc_count_pattern(
                                ip.wrapping_add(4),
                                i_high_limit,
                                pattern,
                            ) as usize
                                + 4;
                        } else {
                            repeat = RepeatState::Not;
                        }
                    }
                    if repeat == RepeatState::Confirmed
                        && match_candidate_idx >= lowest_match_index
                        && hc_protect_dict_end(prefix_idx, match_candidate_idx)
                    {
                        let ext_dict = match_candidate_idx < prefix_idx;
                        let match_ptr = if ext_dict {
                            dict_start
                                .wrapping_add(match_candidate_idx.wrapping_sub(dict_idx) as usize)
                        } else {
                            prefix_ptr.wrapping_add(
                                match_candidate_idx.wrapping_sub(prefix_idx) as usize,
                            )
                        };
                        if read32(match_ptr) == pattern {
                            /* good candidate */
                            let i_limit = if ext_dict { dict_end } else { i_high_limit };
                            let mut forward_pattern_length =
                                hc_count_pattern(match_ptr.wrapping_add(4), i_limit, pattern)
                                    as usize
                                    + 4;
                            if ext_dict
                                && match_ptr.wrapping_add(forward_pattern_length) == i_limit
                            {
                                let rotated =
                                    hc_rotate_pattern(forward_pattern_length, pattern);
                                forward_pattern_length +=
                                    hc_count_pattern(prefix_ptr, i_high_limit, rotated) as usize;
                            }
                            {
                                let lowest_match_ptr =
                                    if ext_dict { dict_start } else { prefix_ptr };
                                let mut back_length = hc_reverse_count_pattern(
                                    match_ptr,
                                    lowest_match_ptr,
                                    pattern,
                                ) as usize;
                                if !ext_dict
                                    && match_ptr.wrapping_sub(back_length) == prefix_ptr
                                    && dict_idx < prefix_idx
                                {
                                    let rotated = hc_rotate_pattern(
                                        (-(back_length as i32)) as u32 as usize,
                                        pattern,
                                    );
                                    back_length += hc_reverse_count_pattern(
                                        dict_end,
                                        dict_start,
                                        rotated,
                                    ) as usize;
                                }
                                /* limit backLength so it doesn't go further than
                                 * lowestMatchIndex */
                                let cand = match_candidate_idx.wrapping_sub(back_length as u32);
                                let m = if cand > lowest_match_index {
                                    cand
                                } else {
                                    lowest_match_index
                                };
                                back_length = match_candidate_idx.wrapping_sub(m) as usize;
                                let current_segment_length =
                                    back_length + forward_pattern_length;

                                if current_segment_length >= src_pattern_length
                                    && forward_pattern_length <= src_pattern_length
                                {
                                    let new_match_index = match_candidate_idx
                                        .wrapping_add(forward_pattern_length as u32)
                                        .wrapping_sub(src_pattern_length as u32);
                                    if hc_protect_dict_end(prefix_idx, new_match_index) {
                                        match_index = new_match_index;
                                    } else {
                                        match_index = prefix_idx;
                                    }
                                } else {
                                    let new_match_index =
                                        match_candidate_idx.wrapping_sub(back_length as u32);
                                    if !hc_protect_dict_end(prefix_idx, new_match_index) {
                                        match_index = prefix_idx;
                                    } else {
                                        match_index = new_match_index;
                                        if look_back_length == 0 {
                                            let max_ml = if current_segment_length
                                                < src_pattern_length
                                            {
                                                current_segment_length
                                            } else {
                                                src_pattern_length
                                            };
                                            if (longest as usize) < max_ml {
                                                if ((ip as usize - prefix_ptr as usize)
                                                    as u32)
                                                    .wrapping_add(prefix_idx)
                                                    .wrapping_sub(match_index)
                                                    > LZ4_DISTANCE_MAX
                                                {
                                                    break 'chain;
                                                }
                                                longest = max_ml as c_int;
                                                offset =
                                                    ip_index.wrapping_sub(match_index) as c_int;
                                            }
                                            {
                                                let dist_to_next_pattern =
                                                    delta_next_u16(chain_table, match_index);
                                                if dist_to_next_pattern > match_index {
                                                    break 'chain;
                                                }
                                                match_index = match_index
                                                    .wrapping_sub(dist_to_next_pattern);
                                            }
                                        }
                                    }
                                }
                            }
                            continue 'chain;
                        }
                    }
                }
            }

            /* follow current chain */
            match_index = match_index.wrapping_sub(delta_next_u16(
                chain_table,
                match_index.wrapping_add(match_chain_pos),
            ));
        }

        if dict == UsingDictCtxHc && nb_attempts > 0 && within_start_distance {
            let dict_end_offset = ((*dict_ctx).end as usize
                - (*dict_ctx).prefix_start as usize)
                + (*dict_ctx).dict_limit as usize;
            let mut dict_match_index = (*dict_ctx).hash_table[hc_hash_ptr(ip) as usize];
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
                    .prefix_start
                    .wrapping_sub((*dict_ctx).dict_limit as usize)
                    .wrapping_add(dict_match_index as usize);

                if read32(match_ptr) == pattern {
                    let mut v_limit =
                        ip.wrapping_add(dict_end_offset - dict_match_index as usize);
                    if v_limit > i_high_limit {
                        v_limit = i_high_limit;
                    }
                    let mut mlt = lz4_count(
                        ip.wrapping_add(MINMATCH),
                        match_ptr.wrapping_add(MINMATCH),
                        v_limit,
                    ) as c_int
                        + MINMATCH as c_int;
                    let back = if look_back_length != 0 {
                        hc_count_back(ip, match_ptr, i_low_limit, (*dict_ctx).prefix_start)
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

                let next_offset =
                    delta_next_u16((*dict_ctx).chain_table.as_ptr(), dict_match_index);
                dict_match_index = dict_match_index.wrapping_sub(next_offset);
                match_index = match_index.wrapping_sub(next_offset);
            }
        }

        LZ4HC_match_t { off: offset, len: longest, back: s_back }
    }
}

#[inline(always)]
unsafe fn hc_insert_and_find_best_match(
    hc4: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    i_limit: *const u8,
    max_nb_attempts: c_int,
    pattern_analysis: bool,
    dict: DictCtxDirective,
) -> LZ4HC_match_t {
    unsafe {
        hc_insert_and_get_wider_match(
            hc4,
            ip,
            ip,
            i_limit,
            MINMATCH as c_int - 1,
            max_nb_attempts,
            pattern_analysis,
            false,
            dict,
            HCfavor::FavorCompressionRatio,
        )
    }
}

unsafe fn hc_compress_hash_chain(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dest: *mut c_char,
    src_size_ptr: *mut c_int,
    max_output_size: c_int,
    max_nb_attempts: c_int,
    limit: LimitedOutput,
    dict: DictCtxDirective,
) -> c_int {
    unsafe {
        let input_size = *src_size_ptr;
        let pattern_analysis = max_nb_attempts > 128; /* levels 9+ */

        let mut ip = source as *const u8;
        let mut anchor = ip;
        let iend = ip.wrapping_add(input_size as usize);
        let mflimit = iend.wrapping_sub(MFLIMIT);
        let matchlimit = iend.wrapping_sub(LASTLITERALS);

        let mut optr = dest as *mut u8;
        let mut op = dest as *mut u8;
        let mut oend = op.wrapping_add(max_output_size as usize);

        let mut start0: *const u8;
        let mut start2: *const u8 = core::ptr::null();
        let mut start3: *const u8 = core::ptr::null();
        let mut m0 = NOMATCH;
        let mut m1 = NOMATCH;
        let mut m2 = NOMATCH;
        let mut m3 = NOMATCH;

        *src_size_ptr = 0;
        if limit == FillOutput {
            oend = oend.wrapping_sub(LASTLITERALS);
        }

        let mut overflow = false;

        if input_size >= LZ4_MIN_LENGTH {
            /* 0 => _Search2, 1 => _Search3 */
            'main: while ip <= mflimit {
                m1 = hc_insert_and_find_best_match(
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

                /* saved, in case we would skip too much */
                start0 = ip;
                m0 = m1;

                let mut state: u32 = 0;
                loop {
                    if state == 0 {
                        /* _Search2 */
                        if ip.wrapping_add(m1.len as usize) <= mflimit {
                            start2 = ip.wrapping_add(m1.len as usize).wrapping_sub(2);
                            m2 = hc_insert_and_get_wider_match(
                                ctx,
                                start2,
                                ip,
                                matchlimit,
                                m1.len,
                                max_nb_attempts,
                                pattern_analysis,
                                false,
                                dict,
                                HCfavor::FavorCompressionRatio,
                            );
                            start2 = start2.wrapping_offset(m2.back as isize);
                        } else {
                            m2 = NOMATCH;
                        }

                        if m2.len <= m1.len {
                            /* No better match => encode ML1 immediately */
                            optr = op;
                            if hc_encode_sequence(
                                &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                            ) != 0
                            {
                                overflow = true;
                                break 'main;
                            }
                            continue 'main;
                        }

                        if start0 < ip {
                            /* first match was skipped at least once */
                            if start2 < ip.wrapping_add(m0.len as usize) {
                                ip = start0;
                                m1 = m0;
                            }
                        }

                        /* Here, start0==ip */
                        if (start2 as isize - ip as isize) < 3 {
                            /* First Match too small : removed */
                            ip = start2;
                            m1 = m2;
                            state = 0;
                            continue;
                        }
                        state = 1;
                    }

                    /* _Search3 */
                    if (start2 as isize - ip as isize) < OPTIMAL_ML as isize {
                        let mut new_ml = m1.len;
                        if new_ml > OPTIMAL_ML {
                            new_ml = OPTIMAL_ML;
                        }
                        if ip.wrapping_add(new_ml as usize)
                            > start2
                                .wrapping_add(m2.len as usize)
                                .wrapping_sub(MINMATCH)
                        {
                            new_ml = (start2 as isize - ip as isize) as c_int + m2.len
                                - MINMATCH as c_int;
                        }
                        let correction = new_ml - (start2 as isize - ip as isize) as c_int;
                        if correction > 0 {
                            start2 = start2.wrapping_offset(correction as isize);
                            m2.len -= correction;
                        }
                    }

                    if start2.wrapping_add(m2.len as usize) <= mflimit {
                        start3 = start2.wrapping_add(m2.len as usize).wrapping_sub(3);
                        m3 = hc_insert_and_get_wider_match(
                            ctx,
                            start3,
                            start2,
                            matchlimit,
                            m2.len,
                            max_nb_attempts,
                            pattern_analysis,
                            false,
                            dict,
                            HCfavor::FavorCompressionRatio,
                        );
                        start3 = start3.wrapping_offset(m3.back as isize);
                    } else {
                        m3 = NOMATCH;
                    }

                    if m3.len <= m2.len {
                        /* No better match => encode ML1 and ML2 */
                        if start2 < ip.wrapping_add(m1.len as usize) {
                            m1.len = (start2 as isize - ip as isize) as c_int;
                        }
                        optr = op;
                        if hc_encode_sequence(
                            &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                        ) != 0
                        {
                            overflow = true;
                            break 'main;
                        }
                        ip = start2;
                        optr = op;
                        if hc_encode_sequence(
                            &mut ip, &mut op, &mut anchor, m2.len, m2.off, limit, oend,
                        ) != 0
                        {
                            m1 = m2;
                            overflow = true;
                            break 'main;
                        }
                        continue 'main;
                    }

                    if start3 < ip.wrapping_add(m1.len as usize + 3) {
                        /* Not enough space for match 2 : remove it */
                        if start3 >= ip.wrapping_add(m1.len as usize) {
                            /* can write Seq1 immediately */
                            if start2 < ip.wrapping_add(m1.len as usize) {
                                let correction = (ip.wrapping_add(m1.len as usize) as isize
                                    - start2 as isize) as c_int;
                                start2 = start2.wrapping_offset(correction as isize);
                                m2.len -= correction;
                                if m2.len < MINMATCH as c_int {
                                    start2 = start3;
                                    m2 = m3;
                                }
                            }

                            optr = op;
                            if hc_encode_sequence(
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
                            state = 0;
                            continue;
                        }

                        start2 = start3;
                        m2 = m3;
                        state = 1;
                        continue;
                    }

                    /* OK, now we have 3 ascending matches; write the first one */
                    if start2 < ip.wrapping_add(m1.len as usize) {
                        if (start2 as isize - ip as isize) < OPTIMAL_ML as isize {
                            if m1.len > OPTIMAL_ML {
                                m1.len = OPTIMAL_ML;
                            }
                            if ip.wrapping_add(m1.len as usize)
                                > start2
                                    .wrapping_add(m2.len as usize)
                                    .wrapping_sub(MINMATCH)
                            {
                                m1.len = (start2 as isize - ip as isize) as c_int + m2.len
                                    - MINMATCH as c_int;
                            }
                            let correction = m1.len - (start2 as isize - ip as isize) as c_int;
                            if correction > 0 {
                                start2 = start2.wrapping_offset(correction as isize);
                                m2.len -= correction;
                            }
                        } else {
                            m1.len = (start2 as isize - ip as isize) as c_int;
                        }
                    }
                    optr = op;
                    if hc_encode_sequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, limit, oend,
                    ) != 0
                    {
                        overflow = true;
                        break 'main;
                    }

                    /* ML2 becomes ML1 */
                    ip = start2;
                    m1 = m2;

                    /* ML3 becomes ML2 */
                    start2 = start3;
                    m2 = m3;

                    state = 1;
                }
            }
        }

        if overflow {
            /* _dest_overflow */
            if limit != FillOutput {
                return 0;
            }
            let ll = ip as usize - anchor as usize;
            let ll_addbytes = (ll + 240) / 255;
            let ll_total_cost = 1 + ll_addbytes + ll;
            let max_lit_pos = oend.wrapping_sub(3);
            op = optr;
            if op.wrapping_add(ll_total_cost) <= max_lit_pos {
                let bytes_left_for_ml =
                    max_lit_pos as usize - (op.wrapping_add(ll_total_cost) as usize);
                let max_ml_size =
                    MINMATCH + (ML_MASK as usize - 1) + (bytes_left_for_ml * 255);
                if m1.len as usize > max_ml_size {
                    m1.len = max_ml_size as c_int;
                }
                let lhs = (oend.wrapping_add(LASTLITERALS) as isize)
                    - (op.wrapping_add(ll_total_cost + 2) as isize)
                    - 1
                    + m1.len as isize;
                if lhs >= MFLIMIT as isize {
                    hc_encode_sequence(
                        &mut ip, &mut op, &mut anchor, m1.len, m1.off, NotLimited, oend,
                    );
                }
            }
        }

        /* _last_literals */
        {
            let mut last_run_size = iend as usize - anchor as usize;
            let mut ll_add = (last_run_size + 255 - RUN_MASK as usize) / 255;
            let total_size = 1 + ll_add + last_run_size;
            if limit == FillOutput {
                oend = oend.wrapping_add(LASTLITERALS);
            }
            if limit != NotLimited && op.wrapping_add(total_size) > oend {
                if limit == Limited {
                    return 0;
                }
                last_run_size = (oend as usize - op as usize) - 1;
                ll_add = (last_run_size + 256 - RUN_MASK as usize) / 256;
                last_run_size -= ll_add;
            }
            ip = anchor.wrapping_add(last_run_size);

            if last_run_size >= RUN_MASK as usize {
                let mut accumulator = last_run_size - RUN_MASK as usize;
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
                *op = ((last_run_size as u32) << ML_BITS) as u8;
                op = op.add(1);
            }
            mem_copy(op, anchor, last_run_size);
            op = op.wrapping_add(last_run_size);
        }

        *src_size_ptr = (ip as usize - source as usize) as c_int;
        (op as usize - dest as usize) as c_int
    }
}

/* ===== LZ4 optimal parser (levels 10..12) ===== */

#[derive(Clone, Copy, Default)]
struct HCOptimal {
    price: c_int,
    off: c_int,
    mlen: c_int,
    litlen: c_int,
}

const TRAILING_LITERALS: usize = 3;

#[inline(always)]
fn hc_literals_price(litlen: c_int) -> c_int {
    let mut price = litlen;
    if litlen >= RUN_MASK as c_int {
        price += 1 + ((litlen - RUN_MASK as c_int) / 255);
    }
    price
}

#[inline(always)]
fn hc_sequence_price(litlen: c_int, mlen: c_int) -> c_int {
    let mut price = 1 + 2; /* token + 16-bit offset */
    price += hc_literals_price(litlen);
    if mlen >= (ML_MASK as c_int + MINMATCH as c_int) {
        price += 1 + ((mlen - (ML_MASK as c_int + MINMATCH as c_int)) / 255);
    }
    price
}

#[inline(always)]
unsafe fn hc_find_longer_match(
    ctx: *mut LZ4HC_CCtx_internal,
    ip: *const u8,
    i_high_limit: *const u8,
    min_len: c_int,
    nb_searches: c_int,
    dict: DictCtxDirective,
    favor_dec_speed: HCfavor,
) -> LZ4HC_match_t {
    unsafe {
        let mut md = hc_insert_and_get_wider_match(
            ctx,
            ip,
            ip,
            i_high_limit,
            min_len,
            nb_searches,
            true,
            true,
            dict,
            favor_dec_speed,
        );
        if md.len <= min_len {
            return NOMATCH;
        }
        if favor_dec_speed == HCfavor::FavorDecompressionSpeed {
            if (md.len > 18) && (md.len <= 36) {
                md.len = 18;
            }
        }
        md
    }
}

unsafe fn hc_compress_optimal(
    ctx: *mut LZ4HC_CCtx_internal,
    source: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    nb_searches: c_int,
    sufficient_len_in: usize,
    limit: LimitedOutput,
    full_update: bool,
    dict: DictCtxDirective,
    favor_dec_speed: HCfavor,
) -> c_int {
    unsafe {
        let mut sufficient_len = sufficient_len_in;

        /* LZ4HC_HEAPMODE == 1 : the table lives on the heap */
        let mut opt_vec: Vec<HCOptimal> = vec![HCOptimal::default(); LZ4_OPT_NUM + TRAILING_LITERALS];
        let opt = opt_vec.as_mut_ptr();

        let mut ip = source as *const u8;
        let mut anchor = ip;
        let iend = ip.wrapping_add(*src_size_ptr as usize);
        let mflimit = iend.wrapping_sub(MFLIMIT);
        let matchlimit = iend.wrapping_sub(LASTLITERALS);
        let mut op = dst as *mut u8;
        let mut op_saved = dst as *mut u8;
        let mut oend = op.wrapping_add(dst_capacity as usize);
        let mut ovml: c_int = MINMATCH as c_int;
        let mut ovoff: c_int = 0;

        *src_size_ptr = 0;
        if limit == FillOutput {
            oend = oend.wrapping_sub(LASTLITERALS);
        }
        if sufficient_len >= LZ4_OPT_NUM {
            sufficient_len = LZ4_OPT_NUM - 1;
        }

        let mut overflow = false;

        'main: while ip <= mflimit {
            let llen = (ip as isize - anchor as isize) as c_int;
            let mut best_mlen: c_int;
            let mut best_off: c_int;
            let mut cur: c_int;
            let mut last_match_pos: c_int;

            let first_match = hc_find_longer_match(
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
                /* good enough solution : immediate encoding */
                let first_ml = first_match.len;
                op_saved = op;
                if hc_encode_sequence(
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
            for r_pos in 0..MINMATCH {
                let cost = hc_literals_price(llen + r_pos as c_int);
                let e = &mut *opt.add(r_pos);
                e.mlen = 1;
                e.off = 0;
                e.litlen = llen + r_pos as c_int;
                e.price = cost;
            }
            /* set prices using initial match */
            {
                let match_ml = first_match.len;
                let offset = first_match.off;
                let mut mlen = MINMATCH as c_int;
                while mlen <= match_ml {
                    let cost = hc_sequence_price(llen, mlen);
                    let e = &mut *opt.add(mlen as usize);
                    e.mlen = mlen;
                    e.off = offset;
                    e.litlen = llen;
                    e.price = cost;
                    mlen += 1;
                }
            }
            last_match_pos = first_match.len;
            for add_lit in 1..=TRAILING_LITERALS {
                let base = (*opt.add(last_match_pos as usize)).price;
                let e = &mut *opt.add(last_match_pos as usize + add_lit);
                e.mlen = 1;
                e.off = 0;
                e.litlen = add_lit as c_int;
                e.price = base + hc_literals_price(add_lit as c_int);
            }

            /* check further positions */
            let mut jumped_to_encode = false;
            best_mlen = 0;
            best_off = 0;
            cur = 1;
            'cur_loop: while cur < last_match_pos {
                let cur_ptr = ip.wrapping_add(cur as usize);

                if cur_ptr > mflimit {
                    break 'cur_loop;
                }
                if full_update {
                    /* not useful to search here if next position has same (or
                     * lower) cost */
                    if (*opt.add(cur as usize + 1)).price <= (*opt.add(cur as usize)).price
                        && (*opt.add(cur as usize + MINMATCH)).price
                            < (*opt.add(cur as usize)).price + 3
                    {
                        cur += 1;
                        continue 'cur_loop;
                    }
                } else {
                    if (*opt.add(cur as usize + 1)).price <= (*opt.add(cur as usize)).price {
                        cur += 1;
                        continue 'cur_loop;
                    }
                }

                let new_match = if full_update {
                    hc_find_longer_match(
                        ctx,
                        cur_ptr,
                        matchlimit,
                        MINMATCH as c_int - 1,
                        nb_searches,
                        dict,
                        favor_dec_speed,
                    )
                } else {
                    hc_find_longer_match(
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
                    continue 'cur_loop;
                }

                if (new_match.len as usize > sufficient_len)
                    || (new_match.len + cur >= LZ4_OPT_NUM as c_int)
                {
                    /* immediate encoding */
                    best_mlen = new_match.len;
                    best_off = new_match.off;
                    last_match_pos = cur + 1;
                    jumped_to_encode = true;
                    break 'cur_loop;
                }

                /* before match : set price with literals at beginning */
                {
                    let base_litlen = (*opt.add(cur as usize)).litlen;
                    let mut litlen: c_int = 1;
                    while litlen < MINMATCH as c_int {
                        let price = (*opt.add(cur as usize)).price
                            - hc_literals_price(base_litlen)
                            + hc_literals_price(base_litlen + litlen);
                        let pos = cur + litlen;
                        if price < (*opt.add(pos as usize)).price {
                            let e = &mut *opt.add(pos as usize);
                            e.mlen = 1;
                            e.off = 0;
                            e.litlen = base_litlen + litlen;
                            e.price = price;
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
                        let price;
                        let ll;
                        if (*opt.add(cur as usize)).mlen == 1 {
                            ll = (*opt.add(cur as usize)).litlen;
                            price = (if cur > ll {
                                (*opt.add((cur - ll) as usize)).price
                            } else {
                                0
                            }) + hc_sequence_price(ll, ml);
                        } else {
                            ll = 0;
                            price = (*opt.add(cur as usize)).price + hc_sequence_price(0, ml);
                        }

                        let favor_bonus = if favor_dec_speed == HCfavor::FavorDecompressionSpeed {
                            1
                        } else {
                            0
                        };
                        if pos > last_match_pos + TRAILING_LITERALS as c_int
                            || price <= (*opt.add(pos as usize)).price - favor_bonus
                        {
                            if (ml == match_ml) && (last_match_pos < pos) {
                                last_match_pos = pos;
                            }
                            let e = &mut *opt.add(pos as usize);
                            e.mlen = ml;
                            e.off = offset;
                            e.litlen = ll;
                            e.price = price;
                        }
                        ml += 1;
                    }
                }
                /* complete following positions with literals */
                for add_lit in 1..=TRAILING_LITERALS {
                    let base = (*opt.add(last_match_pos as usize)).price;
                    let e = &mut *opt.add(last_match_pos as usize + add_lit);
                    e.mlen = 1;
                    e.off = 0;
                    e.litlen = add_lit as c_int;
                    e.price = base + hc_literals_price(add_lit as c_int);
                }

                cur += 1;
            }

            if !jumped_to_encode {
                best_mlen = (*opt.add(last_match_pos as usize)).mlen;
                best_off = (*opt.add(last_match_pos as usize)).off;
                cur = last_match_pos - best_mlen;
            }

            /* encode: cur, last_match_pos, best_mlen, best_off must be set */
            {
                let mut candidate_pos = cur;
                let mut selected_match_length = best_mlen;
                let mut selected_offset = best_off;
                loop {
                    let next_match_length = (*opt.add(candidate_pos as usize)).mlen;
                    let next_offset = (*opt.add(candidate_pos as usize)).off;
                    {
                        let e = &mut *opt.add(candidate_pos as usize);
                        e.mlen = selected_match_length;
                        e.off = selected_offset;
                    }
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
                    let ml = (*opt.add(r_pos as usize)).mlen;
                    let offset = (*opt.add(r_pos as usize)).off;
                    if ml == 1 {
                        ip = ip.wrapping_add(1);
                        r_pos += 1;
                        continue;
                    }
                    r_pos += ml;
                    op_saved = op;
                    if hc_encode_sequence(&mut ip, &mut op, &mut anchor, ml, offset, limit, oend)
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
            /* _dest_overflow */
            if limit != FillOutput {
                return 0;
            }
            let ll = ip as usize - anchor as usize;
            let ll_addbytes = (ll + 240) / 255;
            let ll_total_cost = 1 + ll_addbytes + ll;
            let max_lit_pos = oend.wrapping_sub(3);
            op = op_saved;
            if op.wrapping_add(ll_total_cost) <= max_lit_pos {
                let bytes_left_for_ml =
                    max_lit_pos as usize - (op.wrapping_add(ll_total_cost) as usize);
                let max_ml_size =
                    MINMATCH + (ML_MASK as usize - 1) + (bytes_left_for_ml * 255);
                if ovml as usize > max_ml_size {
                    ovml = max_ml_size as c_int;
                }
                let lhs = (oend.wrapping_add(LASTLITERALS) as isize)
                    - (op.wrapping_add(ll_total_cost + 2) as isize)
                    - 1
                    + ovml as isize;
                if lhs >= MFLIMIT as isize {
                    hc_encode_sequence(
                        &mut ip, &mut op, &mut anchor, ovml, ovoff, NotLimited, oend,
                    );
                }
            }
        }

        /* _last_literals */
        {
            let mut last_run_size = iend as usize - anchor as usize;
            let mut ll_add = (last_run_size + 255 - RUN_MASK as usize) / 255;
            let total_size = 1 + ll_add + last_run_size;
            if limit == FillOutput {
                oend = oend.wrapping_add(LASTLITERALS);
            }
            if limit != NotLimited && op.wrapping_add(total_size) > oend {
                if limit == Limited {
                    return 0;
                }
                last_run_size = (oend as usize - op as usize) - 1;
                ll_add = (last_run_size + 256 - RUN_MASK as usize) / 256;
                last_run_size -= ll_add;
            }
            ip = anchor.wrapping_add(last_run_size);

            if last_run_size >= RUN_MASK as usize {
                let mut accumulator = last_run_size - RUN_MASK as usize;
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
                *op = ((last_run_size as u32) << ML_BITS) as u8;
                op = op.add(1);
            }
            mem_copy(op, anchor, last_run_size);
            op = op.wrapping_add(last_run_size);
        }

        *src_size_ptr = (ip as usize - source as usize) as c_int;
        let retval = (op as usize - dst as usize) as c_int;
        drop(opt_vec);
        retval
    }
}

/* ===== generic dispatch ===== */

unsafe fn hc_compress_generic_internal(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: LimitedOutput,
    dict: DictCtxDirective,
) -> c_int {
    unsafe {
        if limit == FillOutput && dst_capacity < 1 {
            return 0;
        }
        if (*src_size_ptr as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
            return 0;
        }

        (*ctx).end = (*ctx).end.wrapping_add(*src_size_ptr as usize);
        let c_param = get_clevel_params(c_level);
        let favor = if (*ctx).favor_dec_speed != 0 {
            HCfavor::FavorDecompressionSpeed
        } else {
            HCfavor::FavorCompressionRatio
        };
        let result;

        if c_param.strat == Lz4mid {
            result = mid_compress(ctx, src, dst, src_size_ptr, dst_capacity, limit, dict);
        } else if c_param.strat == Lz4hc {
            result = hc_compress_hash_chain(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_param.nb_searches,
                limit,
                dict,
            );
        } else {
            result = hc_compress_optimal(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_param.nb_searches,
                c_param.target_length as usize,
                limit,
                c_level >= LZ4HC_CLEVEL_MAX,
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

unsafe fn hc_compress_generic_no_dict_ctx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: LimitedOutput,
) -> c_int {
    unsafe {
        hc_compress_generic_internal(
            ctx,
            src,
            dst,
            src_size_ptr,
            dst_capacity,
            c_level,
            limit,
            NoDictCtx,
        )
    }
}

fn is_state_compatible(ctx1: *const LZ4HC_CCtx_internal, ctx2: *const LZ4HC_CCtx_internal) -> bool {
    let is_mid1 =
        get_clevel_params(unsafe { (*ctx1).compression_level } as c_int).strat == Lz4mid;
    let is_mid2 =
        get_clevel_params(unsafe { (*ctx2).compression_level } as c_int).strat == Lz4mid;
    !(is_mid1 ^ is_mid2)
}

unsafe fn hc_compress_generic_dict_ctx(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: LimitedOutput,
) -> c_int {
    unsafe {
        let position = ((*ctx).end as usize - (*ctx).prefix_start as usize)
            + ((*ctx).dict_limit.wrapping_sub((*ctx).low_limit)) as usize;
        if position >= 64 * 1024 {
            (*ctx).dict_ctx = core::ptr::null();
            hc_compress_generic_no_dict_ctx(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_level,
                limit,
            )
        } else if position == 0
            && *src_size_ptr > 4 * 1024
            && is_state_compatible(ctx, (*ctx).dict_ctx)
        {
            core::ptr::copy_nonoverlapping((*ctx).dict_ctx, ctx, 1);
            hc_set_external_dict(ctx, src as *const u8);
            (*ctx).compression_level = c_level as i16;
            hc_compress_generic_no_dict_ctx(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_level,
                limit,
            )
        } else {
            hc_compress_generic_internal(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_level,
                limit,
                UsingDictCtxHc,
            )
        }
    }
}

unsafe fn hc_compress_generic(
    ctx: *mut LZ4HC_CCtx_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    c_level: c_int,
    limit: LimitedOutput,
) -> c_int {
    unsafe {
        if (*ctx).dict_ctx.is_null() {
            hc_compress_generic_no_dict_ctx(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_level,
                limit,
            )
        } else {
            hc_compress_generic_dict_ctx(
                ctx,
                src,
                dst,
                src_size_ptr,
                dst_capacity,
                c_level,
                limit,
            )
        }
    }
}

/* ===== public API ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStateHC() -> c_int {
    core::mem::size_of::<LZ4_streamHC_t>() as c_int
}

fn stream_hc_alignment() -> usize {
    core::mem::align_of::<LZ4_streamHC_t>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_extStateHC_fastReset(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    compression_level: c_int,
) -> c_int {
    unsafe {
        let ctx = state as *mut LZ4HC_CCtx_internal;
        if !lz4_is_aligned(state, stream_hc_alignment()) {
            return 0;
        }
        LZ4_resetStreamHC_fast(state as *mut LZ4_streamHC_t, compression_level);
        hc_init_internal(ctx, src as *const u8);
        let mut src_size = src_size;
        if dst_capacity < lz4_compress_bound(src_size) {
            hc_compress_generic(
                ctx,
                src,
                dst,
                &mut src_size,
                dst_capacity,
                compression_level,
                Limited,
            )
        } else {
            hc_compress_generic(
                ctx,
                src,
                dst,
                &mut src_size,
                dst_capacity,
                compression_level,
                NotLimited,
            )
        }
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
    unsafe {
        let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
        if ctx.is_null() {
            return 0;
        }
        LZ4_compress_HC_extStateHC_fastReset(
            state,
            src,
            dst,
            src_size,
            dst_capacity,
            compression_level,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    compression_level: c_int,
) -> c_int {
    unsafe {
        /* LZ4HC_HEAPMODE == 1 */
        let state_ptr = malloc(core::mem::size_of::<LZ4_streamHC_t>());
        if state_ptr.is_null() {
            return 0;
        }
        let c_size = LZ4_compress_HC_extStateHC(
            state_ptr,
            src,
            dst,
            src_size,
            dst_capacity,
            compression_level,
        );
        free(state_ptr);
        c_size
    }
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
    unsafe {
        let ctx = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
        if ctx.is_null() {
            return 0;
        }
        hc_init_internal(ctx as *mut LZ4HC_CCtx_internal, source as *const u8);
        LZ4_setCompressionLevel(ctx, c_level);
        hc_compress_generic(
            ctx as *mut LZ4HC_CCtx_internal,
            source,
            dest,
            source_size_ptr,
            target_dest_size,
            c_level,
            FillOutput,
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_createStreamHC() -> *mut LZ4_streamHC_t {
    unsafe {
        let state = alloc_and_zero(core::mem::size_of::<LZ4_streamHC_t>()) as *mut LZ4_streamHC_t;
        if state.is_null() {
            return core::ptr::null_mut();
        }
        LZ4_setCompressionLevel(state, LZ4HC_CLEVEL_DEFAULT);
        state
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamHC(ptr: *mut LZ4_streamHC_t) -> c_int {
    unsafe {
        if ptr.is_null() {
            return 0;
        }
        free(ptr as *mut c_void);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStreamHC(
    buffer: *mut c_void,
    size: usize,
) -> *mut LZ4_streamHC_t {
    unsafe {
        if buffer.is_null() {
            return core::ptr::null_mut();
        }
        if size < core::mem::size_of::<LZ4_streamHC_t>() {
            return core::ptr::null_mut();
        }
        if !lz4_is_aligned(buffer, stream_hc_alignment()) {
            return core::ptr::null_mut();
        }
        mem_init(
            buffer as *mut u8,
            0,
            core::mem::size_of::<LZ4HC_CCtx_internal>(),
        );
        let s = buffer as *mut LZ4_streamHC_t;
        LZ4_setCompressionLevel(s, LZ4HC_CLEVEL_DEFAULT);
        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC(
    ptr: *mut LZ4_streamHC_t,
    compression_level: c_int,
) {
    unsafe {
        LZ4_initStreamHC(ptr as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
        LZ4_setCompressionLevel(ptr, compression_level);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamHC_fast(
    ptr: *mut LZ4_streamHC_t,
    compression_level: c_int,
) {
    unsafe {
        let s = ptr as *mut LZ4HC_CCtx_internal;
        if (*s).dirty != 0 {
            LZ4_initStreamHC(ptr as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
        } else {
            (*s).dict_limit = (*s)
                .dict_limit
                .wrapping_add(((*s).end as usize - (*s).prefix_start as usize) as u32);
            (*s).prefix_start = core::ptr::null();
            (*s).end = core::ptr::null();
            (*s).dict_ctx = core::ptr::null();
        }
        LZ4_setCompressionLevel(ptr, compression_level);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setCompressionLevel(
    ptr: *mut LZ4_streamHC_t,
    compression_level: c_int,
) {
    unsafe {
        let mut compression_level = compression_level;
        if compression_level < 1 {
            compression_level = LZ4HC_CLEVEL_DEFAULT;
        }
        if compression_level > LZ4HC_CLEVEL_MAX {
            compression_level = LZ4HC_CLEVEL_MAX;
        }
        (*(ptr as *mut LZ4HC_CCtx_internal)).compression_level = compression_level as i16;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_favorDecompressionSpeed(ptr: *mut LZ4_streamHC_t, favor: c_int) {
    unsafe {
        (*(ptr as *mut LZ4HC_CCtx_internal)).favor_dec_speed = (favor != 0) as i8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictHC(
    ptr: *mut LZ4_streamHC_t,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        let ctx_ptr = ptr as *mut LZ4HC_CCtx_internal;
        let mut dictionary = dictionary;
        let mut dict_size = dict_size;
        if dict_size > 64 * 1024 {
            dictionary = dictionary.wrapping_add(dict_size as usize - 64 * 1024);
            dict_size = 64 * 1024;
        }
        let cp;
        {
            let c_level = (*ctx_ptr).compression_level as c_int;
            LZ4_initStreamHC(ptr as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
            LZ4_setCompressionLevel(ptr, c_level);
            cp = get_clevel_params(c_level);
        }
        hc_init_internal(ctx_ptr, dictionary as *const u8);
        (*ctx_ptr).end = (dictionary as *const u8).wrapping_add(dict_size as usize);
        if cp.strat == Lz4mid {
            mid_fill_htable(ctx_ptr, dictionary as *const c_void, dict_size as usize);
        } else {
            if dict_size >= LZ4HC_HASHSIZE as c_int {
                hc_insert(ctx_ptr, (*ctx_ptr).end.wrapping_sub(3));
            }
        }
        dict_size
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_HC_dictionary(
    working_stream: *mut LZ4_streamHC_t,
    dictionary_stream: *const LZ4_streamHC_t,
) {
    unsafe {
        (*(working_stream as *mut LZ4HC_CCtx_internal)).dict_ctx = if dictionary_stream.is_null() {
            core::ptr::null()
        } else {
            dictionary_stream as *const LZ4HC_CCtx_internal
        };
    }
}

unsafe fn hc_set_external_dict(ctx_ptr: *mut LZ4HC_CCtx_internal, new_block: *const u8) {
    unsafe {
        if (*ctx_ptr).end >= (*ctx_ptr).prefix_start.wrapping_add(4)
            && get_clevel_params((*ctx_ptr).compression_level as c_int).strat != Lz4mid
        {
            hc_insert(ctx_ptr, (*ctx_ptr).end.wrapping_sub(3));
        }

        (*ctx_ptr).low_limit = (*ctx_ptr).dict_limit;
        (*ctx_ptr).dict_start = (*ctx_ptr).prefix_start;
        (*ctx_ptr).dict_limit = (*ctx_ptr)
            .dict_limit
            .wrapping_add(((*ctx_ptr).end as usize - (*ctx_ptr).prefix_start as usize) as u32);
        (*ctx_ptr).prefix_start = new_block;
        (*ctx_ptr).end = new_block;
        (*ctx_ptr).next_to_update = (*ctx_ptr).dict_limit;

        (*ctx_ptr).dict_ctx = core::ptr::null();
    }
}

unsafe fn compress_hc_continue_generic(
    ptr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    dst_capacity: c_int,
    limit: LimitedOutput,
) -> c_int {
    unsafe {
        let ctx_ptr = ptr as *mut LZ4HC_CCtx_internal;
        /* auto-init if forgotten */
        if (*ctx_ptr).prefix_start.is_null() {
            hc_init_internal(ctx_ptr, src as *const u8);
        }

        /* Check overflow */
        if ((*ctx_ptr).end as usize - (*ctx_ptr).prefix_start as usize)
            + (*ctx_ptr).dict_limit as usize
            > (2usize << 30)
        {
            let mut dict_size = (*ctx_ptr).end as usize - (*ctx_ptr).prefix_start as usize;
            if dict_size > 64 * 1024 {
                dict_size = 64 * 1024;
            }
            LZ4_loadDictHC(
                ptr,
                ((*ctx_ptr).end as *const c_char).wrapping_sub(dict_size),
                dict_size as c_int,
            );
        }

        /* Check if blocks follow each other */
        if src as *const u8 != (*ctx_ptr).end {
            hc_set_external_dict(ctx_ptr, src as *const u8);
        }

        /* Check overlapping input/dictionary space */
        {
            let mut source_end = (src as *const u8).wrapping_add(*src_size_ptr as usize);
            let dict_begin = (*ctx_ptr).dict_start;
            let dict_end = (*ctx_ptr)
                .dict_start
                .wrapping_add((*ctx_ptr).dict_limit.wrapping_sub((*ctx_ptr).low_limit) as usize);
            if source_end > dict_begin && (src as *const u8) < dict_end {
                if source_end > dict_end {
                    source_end = dict_end;
                }
                (*ctx_ptr).low_limit = (*ctx_ptr)
                    .low_limit
                    .wrapping_add((source_end as usize - (*ctx_ptr).dict_start as usize) as u32);
                (*ctx_ptr).dict_start = (*ctx_ptr)
                    .dict_start
                    .wrapping_add(source_end as usize - (*ctx_ptr).dict_start as usize);
                /* invalidate the dictionary if it's too small */
                if (*ctx_ptr).dict_limit.wrapping_sub((*ctx_ptr).low_limit)
                    < LZ4HC_HASHSIZE as u32
                {
                    (*ctx_ptr).low_limit = (*ctx_ptr).dict_limit;
                    (*ctx_ptr).dict_start = (*ctx_ptr).prefix_start;
                }
            }
        }

        hc_compress_generic(
            ctx_ptr,
            src,
            dst,
            src_size_ptr,
            dst_capacity,
            (*ctx_ptr).compression_level as c_int,
            limit,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue(
    ptr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    unsafe {
        let mut src_size = src_size;
        if dst_capacity < lz4_compress_bound(src_size) {
            compress_hc_continue_generic(ptr, src, dst, &mut src_size, dst_capacity, Limited)
        } else {
            compress_hc_continue_generic(ptr, src, dst, &mut src_size, dst_capacity, NotLimited)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_HC_continue_destSize(
    ptr: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dest_size: c_int,
) -> c_int {
    unsafe {
        compress_hc_continue_generic(ptr, src, dst, src_size_ptr, target_dest_size, FillOutput)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDictHC(
    ptr: *mut LZ4_streamHC_t,
    safe_buffer: *mut c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        let stream_ptr = ptr as *mut LZ4HC_CCtx_internal;
        let prefix_size =
            ((*stream_ptr).end as isize - (*stream_ptr).prefix_start as isize) as c_int;
        let mut dict_size = dict_size;
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
            mem_move(
                safe_buffer as *mut u8,
                (*stream_ptr).end.wrapping_sub(dict_size as usize),
                dict_size as usize,
            );
        }
        {
            let end_index = (((*stream_ptr).end as usize
                - (*stream_ptr).prefix_start as usize) as u32)
                .wrapping_add((*stream_ptr).dict_limit);
            (*stream_ptr).end = if safe_buffer.is_null() {
                core::ptr::null()
            } else {
                (safe_buffer as *const u8).wrapping_add(dict_size as usize)
            };
            (*stream_ptr).prefix_start = safe_buffer as *const u8;
            (*stream_ptr).dict_limit = end_index.wrapping_sub(dict_size as u32);
            (*stream_ptr).low_limit = end_index.wrapping_sub(dict_size as u32);
            (*stream_ptr).dict_start = (*stream_ptr).prefix_start;
            if (*stream_ptr).next_to_update < (*stream_ptr).dict_limit {
                (*stream_ptr).next_to_update = (*stream_ptr).dict_limit;
            }
        }
        dict_size
    }
}

/* ===== deprecated functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, src_size, lz4_compress_bound(src_size), 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, src_size, max_dst_size, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    c_level: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, src_size, lz4_compress_bound(src_size), c_level) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
    c_level: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC(src, dst, src_size, max_dst_size, c_level) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_HC_extStateHC(state, src, dst, src_size, lz4_compress_bound(src_size), 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_extStateHC(state, src, dst, src_size, max_dst_size, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_withStateHC(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    c_level: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_HC_extStateHC(
            state,
            src,
            dst,
            src_size,
            lz4_compress_bound(src_size),
            c_level,
        )
    }
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
    unsafe { LZ4_compress_HC_extStateHC(state, src, dst, src_size, max_dst_size, c_level) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_continue(ctx, src, dst, src_size, lz4_compress_bound(src_size)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC_limitedOutput_continue(
    ctx: *mut LZ4_streamHC_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    max_dst_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_HC_continue(ctx, src, dst, src_size, max_dst_size) }
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
    unsafe {
        let hc4 = LZ4_initStreamHC(state, core::mem::size_of::<LZ4_streamHC_t>());
        if hc4.is_null() {
            return 1;
        }
        hc_init_internal(hc4 as *mut LZ4HC_CCtx_internal, input_buffer as *const u8);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createHC(input_buffer: *const c_char) -> *mut c_void {
    unsafe {
        let hc4 = LZ4_createStreamHC();
        if hc4.is_null() {
            return core::ptr::null_mut();
        }
        hc_init_internal(hc4 as *mut LZ4HC_CCtx_internal, input_buffer as *const u8);
        hc4 as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeHC(data: *mut c_void) -> c_int {
    unsafe {
        if data.is_null() {
            return 0;
        }
        free(data);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_continue(
    data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    c_level: c_int,
) -> c_int {
    unsafe {
        let mut src_size = src_size;
        hc_compress_generic(
            data as *mut LZ4HC_CCtx_internal,
            src,
            dst,
            &mut src_size,
            0,
            c_level,
            NotLimited,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressHC2_limitedOutput_continue(
    data: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    c_level: c_int,
) -> c_int {
    unsafe {
        let mut src_size = src_size;
        hc_compress_generic(
            data as *mut LZ4HC_CCtx_internal,
            src,
            dst,
            &mut src_size,
            dst_capacity,
            c_level,
            Limited,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBufferHC(data: *mut c_void) -> *mut c_char {
    unsafe {
        let s = data as *mut LZ4HC_CCtx_internal;
        let buffer_start = (*s)
            .prefix_start
            .wrapping_sub((*s).dict_limit as usize)
            .wrapping_add((*s).low_limit as usize);
        LZ4_resetStreamHC_fast(data as *mut LZ4_streamHC_t, (*s).compression_level as c_int);
        buffer_start as *mut c_char
    }
}
