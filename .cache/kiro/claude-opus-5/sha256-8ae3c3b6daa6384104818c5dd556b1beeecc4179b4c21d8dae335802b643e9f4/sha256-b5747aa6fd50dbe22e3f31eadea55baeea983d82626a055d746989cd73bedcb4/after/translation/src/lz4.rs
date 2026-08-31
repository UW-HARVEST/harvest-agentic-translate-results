//! Translation of `lz4.c` (LZ4 1.10.0).
//!
//! Compiled with `LZ4_HEAPMODE=0`, default `LZ4_MEMORY_USAGE` (14),
//! `LZ4_DISTANCE_MAX` 65535, on a little-endian 64-bit target
//! (so `reg_t` == `U64`, `STEPSIZE` == 8, `LZ4_FAST_DEC_LOOP` == 1).

use core::ffi::{c_char, c_int, c_void};

use crate::util::*;

/* ===== version ===== */

pub const LZ4_VERSION_MAJOR: c_int = 1;
pub const LZ4_VERSION_MINOR: c_int = 10;
pub const LZ4_VERSION_RELEASE: c_int = 0;
pub const LZ4_VERSION_NUMBER: c_int =
    LZ4_VERSION_MAJOR * 100 * 100 + LZ4_VERSION_MINOR * 100 + LZ4_VERSION_RELEASE;
static LZ4_VERSION_STRING_BYTES: &[u8] = b"1.10.0\0";

/* ===== tuning ===== */

pub const LZ4_ACCELERATION_DEFAULT: c_int = 1;
pub const LZ4_ACCELERATION_MAX: c_int = 65537;

pub const LZ4_MEMORY_USAGE: usize = 14;
pub const LZ4_HASHLOG: usize = LZ4_MEMORY_USAGE - 2;
pub const LZ4_HASHTABLESIZE: usize = 1 << LZ4_MEMORY_USAGE;
pub const LZ4_HASH_SIZE_U32: usize = 1 << LZ4_HASHLOG;

pub const LZ4_MAX_INPUT_SIZE: c_int = 0x7E00_0000;

/* ===== common constants ===== */

pub const MINMATCH: usize = 4;
pub const WILDCOPYLENGTH: usize = 8;
pub const LASTLITERALS: usize = 5;
pub const MFLIMIT: usize = 12;
pub const MATCH_SAFEGUARD_DISTANCE: usize = (2 * WILDCOPYLENGTH) - MINMATCH;
pub const FASTLOOP_SAFE_DISTANCE: usize = 64;
pub const LZ4_MIN_LENGTH: c_int = (MFLIMIT + 1) as c_int;

pub const LZ4_DISTANCE_ABSOLUTE_MAX: u32 = 65535;
pub const LZ4_DISTANCE_MAX: u32 = 65535;

pub const ML_BITS: u32 = 4;
pub const ML_MASK: u32 = (1 << ML_BITS) - 1;
pub const RUN_BITS: u32 = 8 - ML_BITS;
pub const RUN_MASK: u32 = (1 << RUN_BITS) - 1;

pub const LZ4_64K_LIMIT: c_int = (64 * 1024) + (MFLIMIT as c_int - 1);
const LZ4_SKIP_TRIGGER: u32 = 6;

pub const STEPSIZE: usize = 8;
pub const HASH_UNIT: usize = 8;

#[inline]
pub const fn lz4_compress_bound(isize_: c_int) -> c_int {
    if (isize_ as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        0
    } else {
        isize_ + (isize_ / 255) + 16
    }
}

/* ===== directives ===== */

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LimitedOutput {
    NotLimited = 0,
    Limited = 1,
    FillOutput = 2,
}
use LimitedOutput::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TableType {
    ClearedTable = 0,
    ByPtr = 1,
    ByU32 = 2,
    ByU16 = 3,
}
use TableType::*;

impl TableType {
    #[inline]
    fn from_u32(v: u32) -> TableType {
        match v {
            0 => ClearedTable,
            1 => ByPtr,
            2 => ByU32,
            _ => ByU16,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DictDirective {
    NoDict = 0,
    WithPrefix64k = 1,
    UsingExtDict = 2,
    UsingDictCtx = 3,
}
use DictDirective::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DictIssue {
    NoDictIssue = 0,
    DictSmall = 1,
}
use DictIssue::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EarlyEnd {
    DecodeFullBlock = 0,
    PartialDecode = 1,
}
use EarlyEnd::*;

/* ===== state structures ===== */

#[repr(C)]
pub struct LZ4StreamInternal {
    pub hash_table: [u32; LZ4_HASH_SIZE_U32],
    pub dictionary: *const u8,
    pub dict_ctx: *const LZ4StreamInternal,
    pub current_offset: u32,
    pub table_type: u32,
    pub dict_size: u32,
}

pub const LZ4_STREAM_MINSIZE: usize = (1usize << LZ4_MEMORY_USAGE) + 32;

#[repr(C)]
pub union LZ4Stream {
    pub min_state_size: [u8; LZ4_STREAM_MINSIZE],
    pub internal_donotuse: core::mem::ManuallyDrop<LZ4StreamInternal>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4StreamDecodeInternal {
    pub external_dict: *const u8,
    pub prefix_end: *const u8,
    pub ext_dict_size: usize,
    pub prefix_size: usize,
}

pub const LZ4_STREAMDECODE_MINSIZE: usize = 32;

#[repr(C)]
pub union LZ4StreamDecode {
    pub min_state_size: [u8; LZ4_STREAMDECODE_MINSIZE],
    pub internal_donotuse: LZ4StreamDecodeInternal,
}

const _: () = assert!(core::mem::size_of::<LZ4StreamInternal>() == LZ4_STREAM_MINSIZE);
const _: () = assert!(core::mem::size_of::<LZ4Stream>() == LZ4_STREAM_MINSIZE);
const _: () = assert!(core::mem::size_of::<LZ4StreamDecode>() == LZ4_STREAMDECODE_MINSIZE);

#[inline]
pub fn lz4_is_aligned(ptr: *const c_void, alignment: usize) -> bool {
    (ptr as usize & (alignment - 1)) == 0
}

#[inline]
fn lz4_stream_t_alignment() -> usize {
    core::mem::align_of::<LZ4Stream>()
}

/* ===== wild copies ===== */

/// `LZ4_wildCopy8` : can overwrite up to 8 bytes beyond `dst_end`.
#[inline(always)]
pub unsafe fn wild_copy8(dst: *mut u8, src: *const u8, dst_end: *mut u8) {
    unsafe {
        let mut d = dst;
        let mut s = src;
        loop {
            copy8(d, s);
            d = d.wrapping_add(8);
            s = s.wrapping_add(8);
            if d >= dst_end {
                break;
            }
        }
    }
}

/// `LZ4_wildCopy32` : two 16-byte copies per iteration.
#[inline(always)]
pub unsafe fn wild_copy32(dst: *mut u8, src: *const u8, dst_end: *mut u8) {
    unsafe {
        let mut d = dst;
        let mut s = src;
        loop {
            copy16(d, s);
            copy16(d.wrapping_add(16), s.wrapping_add(16));
            d = d.wrapping_add(32);
            s = s.wrapping_add(32);
            if d >= dst_end {
                break;
            }
        }
    }
}

pub static INC32TABLE: [u32; 8] = [0, 1, 2, 1, 0, 4, 4, 4];
pub static DEC64TABLE: [i32; 8] = [0, 0, 0, -1, -4, 1, 2, 3];

unsafe fn lz4_memcpy_using_offset_base(
    dst_ptr: *mut u8,
    src_ptr: *const u8,
    dst_end: *mut u8,
    offset: usize,
) {
    unsafe {
        let mut d = dst_ptr;
        let mut s = src_ptr;
        if offset < 8 {
            write32(d, 0);
            *d = *s;
            *d.add(1) = *s.add(1);
            *d.add(2) = *s.add(2);
            *d.add(3) = *s.add(3);
            s = s.wrapping_add(INC32TABLE[offset] as usize);
            copy4(d.add(4), s);
            s = s.wrapping_offset(-(DEC64TABLE[offset] as isize));
            d = d.add(8);
        } else {
            copy8(d, s);
            d = d.add(8);
            s = s.add(8);
        }
        wild_copy8(d, s, dst_end);
    }
}

unsafe fn lz4_memcpy_using_offset(
    dst_ptr: *mut u8,
    src_ptr: *const u8,
    dst_end: *mut u8,
    offset: usize,
) {
    unsafe {
        let mut v: [u8; 8] = [0; 8];
        let mut dst_ptr = dst_ptr;

        match offset {
            1 => {
                mem_init(v.as_mut_ptr(), *src_ptr, 8);
            }
            2 => {
                copy2(v.as_mut_ptr(), src_ptr);
                copy2(v.as_mut_ptr().add(2), src_ptr);
                copy4(v.as_mut_ptr().add(4), v.as_ptr());
            }
            4 => {
                copy4(v.as_mut_ptr(), src_ptr);
                copy4(v.as_mut_ptr().add(4), src_ptr);
            }
            _ => {
                lz4_memcpy_using_offset_base(dst_ptr, src_ptr, dst_end, offset);
                return;
            }
        }

        copy8(dst_ptr, v.as_ptr());
        dst_ptr = dst_ptr.add(8);
        while dst_ptr < dst_end {
            copy8(dst_ptr, v.as_ptr());
            dst_ptr = dst_ptr.add(8);
        }
    }
}

/* ===== common functions ===== */

/// `LZ4_NbCommonBytes` for little-endian 64-bit.
#[inline(always)]
pub fn lz4_nb_common_bytes(val: u64) -> u32 {
    val.trailing_zeros() >> 3
}

#[inline(always)]
pub unsafe fn lz4_count(p_in: *const u8, p_match: *const u8, p_in_limit: *const u8) -> u32 {
    unsafe {
        let p_start = p_in;
        let mut p_in = p_in;
        let mut p_match = p_match;

        if p_in < p_in_limit.wrapping_sub(STEPSIZE - 1) {
            let diff = read_arch(p_match) ^ read_arch(p_in);
            if diff == 0 {
                p_in = p_in.add(STEPSIZE);
                p_match = p_match.add(STEPSIZE);
            } else {
                return lz4_nb_common_bytes(diff);
            }
        }

        while p_in < p_in_limit.wrapping_sub(STEPSIZE - 1) {
            let diff = read_arch(p_match) ^ read_arch(p_in);
            if diff == 0 {
                p_in = p_in.add(STEPSIZE);
                p_match = p_match.add(STEPSIZE);
                continue;
            }
            p_in = p_in.add(lz4_nb_common_bytes(diff) as usize);
            return (p_in as usize - p_start as usize) as u32;
        }

        if p_in < p_in_limit.wrapping_sub(3) && read32(p_match) == read32(p_in) {
            p_in = p_in.add(4);
            p_match = p_match.add(4);
        }
        if p_in < p_in_limit.wrapping_sub(1) && read16(p_match) == read16(p_in) {
            p_in = p_in.add(2);
            p_match = p_match.add(2);
        }
        if p_in < p_in_limit && *p_match == *p_in {
            p_in = p_in.add(1);
        }
        (p_in as usize - p_start as usize) as u32
    }
}

/* ===== hashing ===== */

#[inline(always)]
fn lz4_hash4(sequence: u32, table_type: TableType) -> u32 {
    if table_type == ByU16 {
        sequence.wrapping_mul(2654435761) >> ((MINMATCH * 8) - (LZ4_HASHLOG + 1))
    } else {
        sequence.wrapping_mul(2654435761) >> ((MINMATCH * 8) - LZ4_HASHLOG)
    }
}

#[inline(always)]
fn lz4_hash5(sequence: u64, table_type: TableType) -> u32 {
    let hash_log = if table_type == ByU16 {
        LZ4_HASHLOG + 1
    } else {
        LZ4_HASHLOG
    };
    const PRIME5BYTES: u64 = 889523592379;
    ((sequence << 24).wrapping_mul(PRIME5BYTES) >> (64 - hash_log)) as u32
}

#[inline(always)]
unsafe fn lz4_hash_position(p: *const u8, table_type: TableType) -> u32 {
    unsafe {
        if table_type != ByU16 {
            return lz4_hash5(read_arch(p), table_type);
        }
        lz4_hash4(read32(p), table_type)
    }
}

#[inline(always)]
unsafe fn lz4_clear_hash(h: u32, table_base: *mut u32, table_type: TableType) {
    unsafe {
        match table_type {
            ByPtr => {
                let t = table_base as *mut *const u8;
                *t.add(h as usize) = core::ptr::null();
            }
            ByU32 => {
                *table_base.add(h as usize) = 0;
            }
            ByU16 => {
                let t = table_base as *mut u16;
                *t.add(h as usize) = 0;
            }
            ClearedTable => {}
        }
    }
}

#[inline(always)]
unsafe fn lz4_put_index_on_hash(idx: u32, h: u32, table_base: *mut u32, table_type: TableType) {
    unsafe {
        match table_type {
            ByU32 => {
                *table_base.add(h as usize) = idx;
            }
            ByU16 => {
                let t = table_base as *mut u16;
                *t.add(h as usize) = idx as u16;
            }
            _ => {}
        }
    }
}

#[inline(always)]
unsafe fn lz4_put_position_on_hash(p: *const u8, h: u32, table_base: *mut u32) {
    unsafe {
        let t = table_base as *mut *const u8;
        *t.add(h as usize) = p;
    }
}

#[inline(always)]
unsafe fn lz4_put_position(p: *const u8, table_base: *mut u32, table_type: TableType) {
    unsafe {
        let h = lz4_hash_position(p, table_type);
        lz4_put_position_on_hash(p, h, table_base);
    }
}

#[inline(always)]
unsafe fn lz4_get_index_on_hash(h: u32, table_base: *const u32, table_type: TableType) -> u32 {
    unsafe {
        if table_type == ByU32 {
            return *table_base.add(h as usize);
        }
        if table_type == ByU16 {
            let t = table_base as *const u16;
            return *t.add(h as usize) as u32;
        }
        0
    }
}

#[inline(always)]
unsafe fn lz4_get_position_on_hash(h: u32, table_base: *const u32) -> *const u8 {
    unsafe {
        let t = table_base as *const *const u8;
        *t.add(h as usize)
    }
}

#[inline(always)]
unsafe fn lz4_get_position(
    p: *const u8,
    table_base: *const u32,
    table_type: TableType,
) -> *const u8 {
    unsafe {
        let h = lz4_hash_position(p, table_type);
        lz4_get_position_on_hash(h, table_base)
    }
}

unsafe fn lz4_prepare_table(cctx: *mut LZ4StreamInternal, input_size: c_int, table_type: TableType) {
    unsafe {
        let c = &mut *cctx;
        if TableType::from_u32(c.table_type) != ClearedTable {
            if TableType::from_u32(c.table_type) != table_type
                || (table_type == ByU16
                    && c.current_offset.wrapping_add(input_size as u32) >= 0xFFFF)
                || (table_type == ByU32 && c.current_offset > (1u32 << 30))
                || table_type == ByPtr
                || input_size >= 4 * 1024
            {
                mem_init(c.hash_table.as_mut_ptr() as *mut u8, 0, LZ4_HASHTABLESIZE);
                c.current_offset = 0;
                c.table_type = ClearedTable as u32;
            }
        }

        if c.current_offset != 0 && table_type == ByU32 {
            c.current_offset = c.current_offset.wrapping_add(64 * 1024);
        }

        c.dict_ctx = core::ptr::null();
        c.dictionary = core::ptr::null();
        c.dict_size = 0;
    }
}

/* ===== compression ===== */

unsafe fn lz4_compress_generic_validated(
    cctx: *mut LZ4StreamInternal,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    input_consumed: *mut c_int,
    max_output_size: c_int,
    output_directive: LimitedOutput,
    table_type: TableType,
    dict_directive: DictDirective,
    dict_issue: DictIssue,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let cctx_r = &mut *cctx;
        let mut ip = source as *const u8;

        let start_index: u32 = cctx_r.current_offset;
        let base = (source as *const u8).wrapping_sub(start_index as usize);
        let mut low_limit: *const u8;

        let dict_ctx = cctx_r.dict_ctx;
        let dictionary: *const u8 = if dict_directive == UsingDictCtx {
            (*dict_ctx).dictionary
        } else {
            cctx_r.dictionary
        };
        let dict_size: u32 = if dict_directive == UsingDictCtx {
            (*dict_ctx).dict_size
        } else {
            cctx_r.dict_size
        };
        let dict_delta: u32 = if dict_directive == UsingDictCtx {
            start_index.wrapping_sub((*dict_ctx).current_offset)
        } else {
            0
        };

        let maybe_ext_mem = dict_directive == UsingExtDict || dict_directive == UsingDictCtx;
        let prefix_idx_limit: u32 = start_index.wrapping_sub(dict_size);
        let dict_end: *const u8 = if !dictionary.is_null() {
            dictionary.wrapping_add(dict_size as usize)
        } else {
            dictionary
        };
        let mut anchor = source as *const u8;
        let iend = ip.wrapping_add(input_size as usize);
        let mflimit_plus_one = iend.wrapping_sub(MFLIMIT).wrapping_add(1);
        let matchlimit = iend.wrapping_sub(LASTLITERALS);

        let dict_base: *const u8 = if dictionary.is_null() {
            core::ptr::null()
        } else if dict_directive == UsingDictCtx {
            dictionary
                .wrapping_add(dict_size as usize)
                .wrapping_sub((*dict_ctx).current_offset as usize)
        } else {
            dictionary
                .wrapping_add(dict_size as usize)
                .wrapping_sub(start_index as usize)
        };

        let mut op = dest as *mut u8;
        let olimit = op.wrapping_add(max_output_size as usize);

        let mut offset: u32 = 0;
        let mut forward_h: u32;

        if output_directive == FillOutput && max_output_size < 1 {
            return 0;
        }

        low_limit = (source as *const u8).wrapping_sub(if dict_directive == WithPrefix64k {
            dict_size as usize
        } else {
            0
        });

        /* Update context state */
        if dict_directive == UsingDictCtx {
            cctx_r.dict_ctx = core::ptr::null();
            cctx_r.dict_size = input_size as u32;
        } else {
            cctx_r.dict_size = cctx_r.dict_size.wrapping_add(input_size as u32);
        }
        cctx_r.current_offset = cctx_r.current_offset.wrapping_add(input_size as u32);
        cctx_r.table_type = table_type as u32;

        let table = cctx_r.hash_table.as_mut_ptr();

        let mut token: *mut u8 = core::ptr::null_mut();
        let mut r#match: *const u8 = core::ptr::null();
        let mut filled_ip: *const u8;

        /* `goto _last_literals` when input is too small */
        let small_input = input_size < LZ4_MIN_LENGTH;

        if !small_input {
            /* First Byte */
            {
                let h = lz4_hash_position(ip, table_type);
                if table_type == ByPtr {
                    lz4_put_position_on_hash(ip, h, table);
                } else {
                    lz4_put_index_on_hash(start_index, h, table, table_type);
                }
            }
            ip = ip.add(1);
            forward_h = lz4_hash_position(ip, table_type);

            /* Main Loop */
            'main_loop: loop {
                /* --- Find a match --- */
                if table_type == ByPtr {
                    let mut forward_ip = ip;
                    let mut step: isize = 1;
                    let mut search_match_nb: i32 = acceleration << LZ4_SKIP_TRIGGER;
                    loop {
                        let h = forward_h;
                        ip = forward_ip;
                        forward_ip = forward_ip.wrapping_offset(step);
                        step = (search_match_nb >> LZ4_SKIP_TRIGGER) as isize;
                        search_match_nb += 1;

                        if forward_ip > mflimit_plus_one {
                            break 'main_loop;
                        }

                        r#match = lz4_get_position_on_hash(h, table);
                        forward_h = lz4_hash_position(forward_ip, table_type);
                        lz4_put_position_on_hash(ip, h, table);

                        if !(r#match.wrapping_add(LZ4_DISTANCE_MAX as usize) < ip
                            || read32(r#match) != read32(ip))
                        {
                            break;
                        }
                    }
                } else {
                    let mut forward_ip = ip;
                    let mut step: isize = 1;
                    let mut search_match_nb: i32 = acceleration << LZ4_SKIP_TRIGGER;
                    loop {
                        let h = forward_h;
                        let current = (forward_ip as usize - base as usize) as u32;
                        let mut match_index = lz4_get_index_on_hash(h, table, table_type);
                        ip = forward_ip;
                        forward_ip = forward_ip.wrapping_offset(step);
                        step = (search_match_nb >> LZ4_SKIP_TRIGGER) as isize;
                        search_match_nb += 1;

                        if forward_ip > mflimit_plus_one {
                            break 'main_loop;
                        }

                        if dict_directive == UsingDictCtx {
                            if match_index < start_index {
                                match_index =
                                    lz4_get_index_on_hash(h, (*dict_ctx).hash_table.as_ptr(), ByU32);
                                r#match = dict_base.wrapping_add(match_index as usize);
                                match_index = match_index.wrapping_add(dict_delta);
                                low_limit = dictionary;
                            } else {
                                r#match = base.wrapping_add(match_index as usize);
                                low_limit = source as *const u8;
                            }
                        } else if dict_directive == UsingExtDict {
                            if match_index < start_index {
                                r#match = dict_base.wrapping_add(match_index as usize);
                                low_limit = dictionary;
                            } else {
                                r#match = base.wrapping_add(match_index as usize);
                                low_limit = source as *const u8;
                            }
                        } else {
                            r#match = base.wrapping_add(match_index as usize);
                        }
                        forward_h = lz4_hash_position(forward_ip, table_type);
                        lz4_put_index_on_hash(current, h, table, table_type);

                        if dict_issue == DictSmall && match_index < prefix_idx_limit {
                            continue;
                        }
                        if (table_type != ByU16 || LZ4_DISTANCE_MAX < LZ4_DISTANCE_ABSOLUTE_MAX)
                            && match_index.wrapping_add(LZ4_DISTANCE_MAX) < current
                        {
                            continue;
                        }

                        if read32(r#match) == read32(ip) {
                            if maybe_ext_mem {
                                offset = current.wrapping_sub(match_index);
                            }
                            break;
                        }
                    }
                }

                /* --- Catch up --- */
                filled_ip = ip;
                if r#match > low_limit && *ip.wrapping_sub(1) == *r#match.wrapping_sub(1) {
                    loop {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        if !(((ip > anchor) & (r#match > low_limit))
                            && *ip.wrapping_sub(1) == *r#match.wrapping_sub(1))
                        {
                            break;
                        }
                    }
                }

                /* --- Encode Literals --- */
                {
                    let lit_length = (ip as usize - anchor as usize) as u32;
                    token = op;
                    op = op.wrapping_add(1);
                    if output_directive == Limited
                        && op
                            .wrapping_add(lit_length as usize)
                            .wrapping_add(2 + 1 + LASTLITERALS)
                            .wrapping_add((lit_length / 255) as usize)
                            > olimit
                    {
                        return 0;
                    }
                    if output_directive == FillOutput
                        && op
                            .wrapping_add(((lit_length + 240) / 255) as usize)
                            .wrapping_add(lit_length as usize)
                            .wrapping_add(2 + 1 + MFLIMIT - MINMATCH)
                            > olimit
                    {
                        op = op.wrapping_sub(1);
                        break 'main_loop;
                    }
                    if lit_length >= RUN_MASK {
                        let mut len = lit_length - RUN_MASK;
                        *token = (RUN_MASK << ML_BITS) as u8;
                        while len >= 255 {
                            *op = 255;
                            op = op.add(1);
                            len -= 255;
                        }
                        *op = len as u8;
                        op = op.add(1);
                    } else {
                        *token = (lit_length << ML_BITS) as u8;
                    }

                    /* Copy Literals */
                    wild_copy8(op, anchor, op.wrapping_add(lit_length as usize));
                    op = op.wrapping_add(lit_length as usize);
                }

                /* --- _next_match --- */
                loop {
                    if output_directive == FillOutput
                        && op.wrapping_add(2 + 1 + MFLIMIT - MINMATCH) > olimit
                    {
                        op = token;
                        break 'main_loop;
                    }

                    /* Encode Offset */
                    if maybe_ext_mem {
                        write_le16(op, offset as u16);
                        op = op.wrapping_add(2);
                    } else {
                        write_le16(op, (ip as usize - r#match as usize) as u16);
                        op = op.wrapping_add(2);
                    }

                    /* Encode MatchLength */
                    {
                        let mut match_code: u32;

                        if (dict_directive == UsingExtDict || dict_directive == UsingDictCtx)
                            && low_limit == dictionary
                        {
                            let mut limit = ip
                                .wrapping_add(dict_end as usize - r#match as usize);
                            if limit > matchlimit {
                                limit = matchlimit;
                            }
                            match_code = lz4_count(
                                ip.wrapping_add(MINMATCH),
                                r#match.wrapping_add(MINMATCH),
                                limit,
                            );
                            ip = ip.wrapping_add(match_code as usize + MINMATCH);
                            if ip == limit {
                                let more = lz4_count(limit, source as *const u8, matchlimit);
                                match_code += more;
                                ip = ip.wrapping_add(more as usize);
                            }
                        } else {
                            match_code = lz4_count(
                                ip.wrapping_add(MINMATCH),
                                r#match.wrapping_add(MINMATCH),
                                matchlimit,
                            );
                            ip = ip.wrapping_add(match_code as usize + MINMATCH);
                        }

                        if output_directive != NotLimited
                            && op
                                .wrapping_add(1 + LASTLITERALS)
                                .wrapping_add(((match_code + 240) / 255) as usize)
                                > olimit
                        {
                            if output_directive == FillOutput {
                                let new_match_code: u32 = 15u32.wrapping_sub(1).wrapping_add(
                                    ((olimit as usize - op as usize) as u32)
                                        .wrapping_sub(1)
                                        .wrapping_sub(LASTLITERALS as u32)
                                        .wrapping_mul(255),
                                );
                                ip = ip.wrapping_sub(
                                    match_code.wrapping_sub(new_match_code) as usize,
                                );
                                match_code = new_match_code;
                                if ip <= filled_ip {
                                    let mut ptr = ip;
                                    while ptr <= filled_ip {
                                        let h = lz4_hash_position(ptr, table_type);
                                        lz4_clear_hash(h, table, table_type);
                                        ptr = ptr.add(1);
                                    }
                                }
                            } else {
                                return 0;
                            }
                        }
                        if match_code >= ML_MASK {
                            *token = (*token).wrapping_add(ML_MASK as u8);
                            match_code -= ML_MASK;
                            write32(op, 0xFFFF_FFFF);
                            while match_code >= 4 * 255 {
                                op = op.wrapping_add(4);
                                write32(op, 0xFFFF_FFFF);
                                match_code -= 4 * 255;
                            }
                            op = op.wrapping_add((match_code / 255) as usize);
                            *op = (match_code % 255) as u8;
                            op = op.wrapping_add(1);
                        } else {
                            *token = (*token).wrapping_add(match_code as u8);
                        }
                    }

                    anchor = ip;

                    /* Test end of chunk */
                    if ip >= mflimit_plus_one {
                        break 'main_loop;
                    }

                    /* Fill table */
                    {
                        let h = lz4_hash_position(ip.wrapping_sub(2), table_type);
                        if table_type == ByPtr {
                            lz4_put_position_on_hash(ip.wrapping_sub(2), h, table);
                        } else {
                            let idx = (ip.wrapping_sub(2) as usize - base as usize) as u32;
                            lz4_put_index_on_hash(idx, h, table, table_type);
                        }
                    }

                    /* Test next position */
                    if table_type == ByPtr {
                        r#match = lz4_get_position(ip, table, table_type);
                        lz4_put_position(ip, table, table_type);
                        if r#match.wrapping_add(LZ4_DISTANCE_MAX as usize) >= ip
                            && read32(r#match) == read32(ip)
                        {
                            token = op;
                            op = op.wrapping_add(1);
                            *token = 0;
                            continue;
                        }
                    } else {
                        let h = lz4_hash_position(ip, table_type);
                        let current = (ip as usize - base as usize) as u32;
                        let mut match_index = lz4_get_index_on_hash(h, table, table_type);
                        if dict_directive == UsingDictCtx {
                            if match_index < start_index {
                                match_index =
                                    lz4_get_index_on_hash(h, (*dict_ctx).hash_table.as_ptr(), ByU32);
                                r#match = dict_base.wrapping_add(match_index as usize);
                                low_limit = dictionary;
                                match_index = match_index.wrapping_add(dict_delta);
                            } else {
                                r#match = base.wrapping_add(match_index as usize);
                                low_limit = source as *const u8;
                            }
                        } else if dict_directive == UsingExtDict {
                            if match_index < start_index {
                                r#match = dict_base.wrapping_add(match_index as usize);
                                low_limit = dictionary;
                            } else {
                                r#match = base.wrapping_add(match_index as usize);
                                low_limit = source as *const u8;
                            }
                        } else {
                            r#match = base.wrapping_add(match_index as usize);
                        }
                        lz4_put_index_on_hash(current, h, table, table_type);
                        let cond_a = if dict_issue == DictSmall {
                            match_index >= prefix_idx_limit
                        } else {
                            true
                        };
                        let cond_b = if table_type == ByU16
                            && LZ4_DISTANCE_MAX == LZ4_DISTANCE_ABSOLUTE_MAX
                        {
                            true
                        } else {
                            match_index.wrapping_add(LZ4_DISTANCE_MAX) >= current
                        };
                        if cond_a && cond_b && read32(r#match) == read32(ip) {
                            token = op;
                            op = op.wrapping_add(1);
                            *token = 0;
                            if maybe_ext_mem {
                                offset = current.wrapping_sub(match_index);
                            }
                            continue;
                        }
                    }

                    /* Prepare next loop */
                    ip = ip.wrapping_add(1);
                    forward_h = lz4_hash_position(ip, table_type);
                    break;
                }
            }
        }

        /* --- _last_literals --- */
        {
            let mut last_run = iend as usize - anchor as usize;
            if output_directive != NotLimited
                && op
                    .wrapping_add(last_run)
                    .wrapping_add(1)
                    .wrapping_add((last_run + 255 - RUN_MASK as usize) / 255)
                    > olimit
            {
                if output_directive == FillOutput {
                    last_run = (olimit as usize - op as usize) - 1;
                    last_run -= (last_run + 256 - RUN_MASK as usize) / 256;
                } else {
                    return 0;
                }
            }
            if last_run >= RUN_MASK as usize {
                let mut accumulator = last_run - RUN_MASK as usize;
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
                *op = ((last_run as u32) << ML_BITS) as u8;
                op = op.add(1);
            }
            mem_copy(op, anchor, last_run);
            ip = anchor.wrapping_add(last_run);
            op = op.wrapping_add(last_run);
        }

        if output_directive == FillOutput {
            *input_consumed = (ip as usize - source as usize) as c_int;
        }
        (op as usize - dest as usize) as c_int
    }
}

unsafe fn lz4_compress_generic(
    cctx: *mut LZ4StreamInternal,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    input_consumed: *mut c_int,
    dst_capacity: c_int,
    output_directive: LimitedOutput,
    table_type: TableType,
    dict_directive: DictDirective,
    dict_issue: DictIssue,
    acceleration: c_int,
) -> c_int {
    unsafe {
        if (src_size as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
            return 0;
        }
        if src_size == 0 {
            if output_directive != NotLimited && dst_capacity <= 0 {
                return 0;
            }
            *dst = 0;
            if output_directive == FillOutput {
                *input_consumed = 0;
            }
            return 1;
        }

        lz4_compress_generic_validated(
            cctx,
            src,
            dst,
            src_size,
            input_consumed,
            dst_capacity,
            output_directive,
            table_type,
            dict_directive,
            dict_issue,
            acceleration,
        )
    }
}

/* ===== simple / ext-state compression API ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_versionNumber() -> c_int {
    LZ4_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_versionString() -> *const c_char {
    LZ4_VERSION_STRING_BYTES.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_compressBound(isize_: c_int) -> c_int {
    lz4_compress_bound(isize_)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofState() -> c_int {
    core::mem::size_of::<LZ4Stream>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_extState(
    state: *mut c_void,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    max_output_size: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let ctx =
            LZ4_initStream(state, core::mem::size_of::<LZ4Stream>()) as *mut LZ4StreamInternal;
        let mut acceleration = acceleration;
        if acceleration < 1 {
            acceleration = LZ4_ACCELERATION_DEFAULT;
        }
        if acceleration > LZ4_ACCELERATION_MAX {
            acceleration = LZ4_ACCELERATION_MAX;
        }
        if max_output_size >= lz4_compress_bound(input_size) {
            let table_type = if input_size < LZ4_64K_LIMIT { ByU16 } else { ByU32 };
            lz4_compress_generic(
                ctx,
                source,
                dest,
                input_size,
                core::ptr::null_mut(),
                0,
                NotLimited,
                table_type,
                NoDict,
                NoDictIssue,
                acceleration,
            )
        } else {
            let table_type = if input_size < LZ4_64K_LIMIT { ByU16 } else { ByU32 };
            lz4_compress_generic(
                ctx,
                source,
                dest,
                input_size,
                core::ptr::null_mut(),
                max_output_size,
                Limited,
                table_type,
                NoDict,
                NoDictIssue,
                acceleration,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_extState_fastReset(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let ctx = state as *mut LZ4StreamInternal;
        let mut acceleration = acceleration;
        if acceleration < 1 {
            acceleration = LZ4_ACCELERATION_DEFAULT;
        }
        if acceleration > LZ4_ACCELERATION_MAX {
            acceleration = LZ4_ACCELERATION_MAX;
        }

        let output_directive = if dst_capacity >= lz4_compress_bound(src_size) {
            NotLimited
        } else {
            Limited
        };
        let max_output = if output_directive == NotLimited {
            0
        } else {
            dst_capacity
        };

        if src_size < LZ4_64K_LIMIT {
            let table_type = ByU16;
            lz4_prepare_table(ctx, src_size, table_type);
            let dict_issue = if (*ctx).current_offset != 0 {
                DictSmall
            } else {
                NoDictIssue
            };
            lz4_compress_generic(
                ctx,
                src,
                dst,
                src_size,
                core::ptr::null_mut(),
                max_output,
                output_directive,
                table_type,
                NoDict,
                dict_issue,
                acceleration,
            )
        } else {
            let table_type = ByU32;
            lz4_prepare_table(ctx, src_size, table_type);
            lz4_compress_generic(
                ctx,
                src,
                dst,
                src_size,
                core::ptr::null_mut(),
                max_output,
                output_directive,
                table_type,
                NoDict,
                NoDictIssue,
                acceleration,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast(
    src: *const c_char,
    dest: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let mut ctx = core::mem::MaybeUninit::<LZ4Stream>::uninit();
        LZ4_compress_fast_extState(
            ctx.as_mut_ptr() as *mut c_void,
            src,
            dest,
            src_size,
            dst_capacity,
            acceleration,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_default(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    unsafe { LZ4_compress_fast(src, dst, src_size, dst_capacity, 1) }
}

unsafe fn lz4_compress_dest_size_ext_state_internal(
    state: *mut LZ4Stream,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dst_size: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
        LZ4_initStream(state as *mut c_void, core::mem::size_of::<LZ4Stream>());

        if target_dst_size >= lz4_compress_bound(*src_size_ptr) {
            LZ4_compress_fast_extState(
                state as *mut c_void,
                src,
                dst,
                *src_size_ptr,
                target_dst_size,
                acceleration,
            )
        } else {
            let addr_mode = if *src_size_ptr < LZ4_64K_LIMIT { ByU16 } else { ByU32 };
            lz4_compress_generic(
                state as *mut LZ4StreamInternal,
                src,
                dst,
                *src_size_ptr,
                src_size_ptr,
                target_dst_size,
                FillOutput,
                addr_mode,
                NoDict,
                NoDictIssue,
                acceleration,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_destSize_extState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dst_size: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let r = lz4_compress_dest_size_ext_state_internal(
            state as *mut LZ4Stream,
            src,
            dst,
            src_size_ptr,
            target_dst_size,
            acceleration,
        );
        LZ4_initStream(state, core::mem::size_of::<LZ4Stream>());
        r
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_destSize(
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dst_size: c_int,
) -> c_int {
    unsafe {
        let mut ctx_body = core::mem::MaybeUninit::<LZ4Stream>::uninit();
        lz4_compress_dest_size_ext_state_internal(
            ctx_body.as_mut_ptr(),
            src,
            dst,
            src_size_ptr,
            target_dst_size,
            1,
        )
    }
}

/* ===== streaming compression ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_createStream() -> *mut LZ4Stream {
    unsafe {
        let lz4s = malloc(core::mem::size_of::<LZ4Stream>()) as *mut LZ4Stream;
        if lz4s.is_null() {
            return core::ptr::null_mut();
        }
        LZ4_initStream(lz4s as *mut c_void, core::mem::size_of::<LZ4Stream>());
        lz4s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStream(buffer: *mut c_void, size: usize) -> *mut LZ4Stream {
    unsafe {
        if buffer.is_null() {
            return core::ptr::null_mut();
        }
        if size < core::mem::size_of::<LZ4Stream>() {
            return core::ptr::null_mut();
        }
        if !lz4_is_aligned(buffer, lz4_stream_t_alignment()) {
            return core::ptr::null_mut();
        }
        mem_init(
            buffer as *mut u8,
            0,
            core::mem::size_of::<LZ4StreamInternal>(),
        );
        buffer as *mut LZ4Stream
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream(lz4_stream: *mut LZ4Stream) {
    unsafe {
        mem_init(
            lz4_stream as *mut u8,
            0,
            core::mem::size_of::<LZ4StreamInternal>(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream_fast(ctx: *mut LZ4Stream) {
    unsafe { lz4_prepare_table(ctx as *mut LZ4StreamInternal, 0, ByU32) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStream(lz4_stream: *mut LZ4Stream) -> c_int {
    unsafe {
        if lz4_stream.is_null() {
            return 0;
        }
        free(lz4_stream as *mut c_void);
        0
    }
}

const LD_FAST: u32 = 0;
const LD_SLOW: u32 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict_internal(
    lz4_dict: *mut LZ4Stream,
    dictionary: *const c_char,
    dict_size: c_int,
    ld: u32,
) -> c_int {
    unsafe {
        let dict = lz4_dict as *mut LZ4StreamInternal;
        let table_type = ByU32;
        let mut p = dictionary as *const u8;
        let dict_end = p.wrapping_add(dict_size as usize);
        let mut idx32: u32;

        LZ4_resetStream(lz4_dict);

        (*dict).current_offset = (*dict).current_offset.wrapping_add(64 * 1024);

        if dict_size < HASH_UNIT as c_int {
            return 0;
        }

        if (dict_end as isize - p as isize) > 64 * 1024 {
            p = dict_end.wrapping_sub(64 * 1024);
        }
        (*dict).dictionary = p;
        (*dict).dict_size = (dict_end as usize - p as usize) as u32;
        (*dict).table_type = table_type as u32;
        idx32 = (*dict).current_offset.wrapping_sub((*dict).dict_size);

        let table = (*dict).hash_table.as_mut_ptr();

        while p <= dict_end.wrapping_sub(HASH_UNIT) {
            let h = lz4_hash_position(p, table_type);
            lz4_put_index_on_hash(idx32, h, table, table_type);
            p = p.wrapping_add(3);
            idx32 = idx32.wrapping_add(3);
        }

        if ld == LD_SLOW {
            p = (*dict).dictionary;
            idx32 = (*dict).current_offset.wrapping_sub((*dict).dict_size);
            while p <= dict_end.wrapping_sub(HASH_UNIT) {
                let h = lz4_hash_position(p, table_type);
                let limit = (*dict).current_offset.wrapping_sub(64 * 1024);
                if lz4_get_index_on_hash(h, table, table_type) <= limit {
                    lz4_put_index_on_hash(idx32, h, table, table_type);
                }
                p = p.wrapping_add(1);
                idx32 = idx32.wrapping_add(1);
            }
        }

        (*dict).dict_size as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict(
    lz4_dict: *mut LZ4Stream,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe { LZ4_loadDict_internal(lz4_dict, dictionary, dict_size, LD_FAST) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictSlow(
    lz4_dict: *mut LZ4Stream,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe { LZ4_loadDict_internal(lz4_dict, dictionary, dict_size, LD_SLOW) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_dictionary(
    working_stream: *mut LZ4Stream,
    dictionary_stream: *const LZ4Stream,
) {
    unsafe {
        let mut dict_ctx: *const LZ4StreamInternal = if dictionary_stream.is_null() {
            core::ptr::null()
        } else {
            dictionary_stream as *const LZ4StreamInternal
        };

        if !dict_ctx.is_null() {
            let ws = working_stream as *mut LZ4StreamInternal;
            if (*ws).current_offset == 0 {
                (*ws).current_offset = 64 * 1024;
            }
            if (*dict_ctx).dict_size == 0 {
                dict_ctx = core::ptr::null();
            }
        }
        (*(working_stream as *mut LZ4StreamInternal)).dict_ctx = dict_ctx;
    }
}

unsafe fn lz4_renorm_dict_t(lz4_dict: *mut LZ4StreamInternal, next_size: c_int) {
    unsafe {
        let d = &mut *lz4_dict;
        if (d.current_offset as u64) + (next_size as u32 as u64) > 0x8000_0000 {
            let delta = d.current_offset.wrapping_sub(64 * 1024);
            let dict_end = d.dictionary.wrapping_add(d.dict_size as usize);
            for i in 0..LZ4_HASH_SIZE_U32 {
                if d.hash_table[i] < delta {
                    d.hash_table[i] = 0;
                } else {
                    d.hash_table[i] -= delta;
                }
            }
            d.current_offset = 64 * 1024;
            if d.dict_size > 64 * 1024 {
                d.dict_size = 64 * 1024;
            }
            d.dictionary = dict_end.wrapping_sub(d.dict_size as usize);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_continue(
    lz4_stream: *mut LZ4Stream,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    max_output_size: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let table_type = ByU32;
        let stream_ptr = lz4_stream as *mut LZ4StreamInternal;
        let mut dict_end: *const c_char = if (*stream_ptr).dict_size != 0 {
            ((*stream_ptr).dictionary as *const c_char)
                .wrapping_add((*stream_ptr).dict_size as usize)
        } else {
            core::ptr::null()
        };

        lz4_renorm_dict_t(stream_ptr, input_size);
        let mut acceleration = acceleration;
        if acceleration < 1 {
            acceleration = LZ4_ACCELERATION_DEFAULT;
        }
        if acceleration > LZ4_ACCELERATION_MAX {
            acceleration = LZ4_ACCELERATION_MAX;
        }

        /* invalidate tiny dictionaries */
        if (*stream_ptr).dict_size < 4
            && dict_end != source
            && input_size > 0
            && (*stream_ptr).dict_ctx.is_null()
        {
            (*stream_ptr).dict_size = 0;
            (*stream_ptr).dictionary = source as *const u8;
            dict_end = source;
        }

        /* Check overlapping input/dictionary space */
        {
            let source_end = source.wrapping_add(input_size as usize);
            if source_end > (*stream_ptr).dictionary as *const c_char && source_end < dict_end {
                (*stream_ptr).dict_size = (dict_end as usize - source_end as usize) as u32;
                if (*stream_ptr).dict_size > 64 * 1024 {
                    (*stream_ptr).dict_size = 64 * 1024;
                }
                if (*stream_ptr).dict_size < 4 {
                    (*stream_ptr).dict_size = 0;
                }
                (*stream_ptr).dictionary =
                    (dict_end as *const u8).wrapping_sub((*stream_ptr).dict_size as usize);
            }
        }

        /* prefix mode : source data follows dictionary */
        if dict_end == source {
            let dict_issue = if (*stream_ptr).dict_size < 64 * 1024
                && (*stream_ptr).dict_size < (*stream_ptr).current_offset
            {
                DictSmall
            } else {
                NoDictIssue
            };
            return lz4_compress_generic(
                stream_ptr,
                source,
                dest,
                input_size,
                core::ptr::null_mut(),
                max_output_size,
                Limited,
                table_type,
                WithPrefix64k,
                dict_issue,
                acceleration,
            );
        }

        /* external dictionary mode */
        {
            let result;
            if !(*stream_ptr).dict_ctx.is_null() {
                if input_size > 4 * 1024 {
                    core::ptr::copy_nonoverlapping((*stream_ptr).dict_ctx, stream_ptr, 1);
                    result = lz4_compress_generic(
                        stream_ptr,
                        source,
                        dest,
                        input_size,
                        core::ptr::null_mut(),
                        max_output_size,
                        Limited,
                        table_type,
                        UsingExtDict,
                        NoDictIssue,
                        acceleration,
                    );
                } else {
                    result = lz4_compress_generic(
                        stream_ptr,
                        source,
                        dest,
                        input_size,
                        core::ptr::null_mut(),
                        max_output_size,
                        Limited,
                        table_type,
                        UsingDictCtx,
                        NoDictIssue,
                        acceleration,
                    );
                }
            } else {
                let dict_issue = if (*stream_ptr).dict_size < 64 * 1024
                    && (*stream_ptr).dict_size < (*stream_ptr).current_offset
                {
                    DictSmall
                } else {
                    NoDictIssue
                };
                result = lz4_compress_generic(
                    stream_ptr,
                    source,
                    dest,
                    input_size,
                    core::ptr::null_mut(),
                    max_output_size,
                    Limited,
                    table_type,
                    UsingExtDict,
                    dict_issue,
                    acceleration,
                );
            }
            (*stream_ptr).dictionary = source as *const u8;
            (*stream_ptr).dict_size = input_size as u32;
            result
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_forceExtDict(
    lz4_dict: *mut LZ4Stream,
    source: *const c_char,
    dest: *mut c_char,
    src_size: c_int,
) -> c_int {
    unsafe {
        let stream_ptr = lz4_dict as *mut LZ4StreamInternal;

        lz4_renorm_dict_t(stream_ptr, src_size);

        let dict_issue = if (*stream_ptr).dict_size < 64 * 1024
            && (*stream_ptr).dict_size < (*stream_ptr).current_offset
        {
            DictSmall
        } else {
            NoDictIssue
        };
        let result = lz4_compress_generic(
            stream_ptr,
            source,
            dest,
            src_size,
            core::ptr::null_mut(),
            0,
            NotLimited,
            ByU32,
            UsingExtDict,
            dict_issue,
            1,
        );

        (*stream_ptr).dictionary = source as *const u8;
        (*stream_ptr).dict_size = src_size as u32;

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDict(
    lz4_dict: *mut LZ4Stream,
    safe_buffer: *mut c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        let dict = lz4_dict as *mut LZ4StreamInternal;
        let mut dict_size = dict_size;

        if (dict_size as u32) > 64 * 1024 {
            dict_size = 64 * 1024;
        }
        if (dict_size as u32) > (*dict).dict_size {
            dict_size = (*dict).dict_size as c_int;
        }

        if dict_size > 0 {
            let previous_dict_end = (*dict).dictionary.wrapping_add((*dict).dict_size as usize);
            mem_move(
                safe_buffer as *mut u8,
                previous_dict_end.wrapping_sub(dict_size as usize),
                dict_size as usize,
            );
        }

        (*dict).dictionary = safe_buffer as *const u8;
        (*dict).dict_size = dict_size as u32;

        dict_size
    }
}

/* ===== decompression ===== */

#[inline]
fn min_usize(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

unsafe fn read_long_length_no_check(pp: &mut *const u8) -> usize {
    unsafe {
        let mut l: usize = 0;
        loop {
            let b = **pp as usize;
            *pp = (*pp).add(1);
            l = l.wrapping_add(b);
            if b != 255 {
                break;
            }
        }
        l
    }
}

unsafe fn lz4_decompress_unsafe_generic(
    istart: *const u8,
    ostart: *mut u8,
    decompressed_size: c_int,
    prefix_size: usize,
    dict_start: *const u8,
    dict_size: usize,
) -> c_int {
    unsafe {
        let mut ip = istart;
        let mut op = ostart;
        let oend = ostart.wrapping_add(decompressed_size as usize);
        let prefix_start = ostart.wrapping_sub(prefix_size);

        loop {
            let token = *ip as u32;
            ip = ip.add(1);

            /* literals */
            {
                let mut ll = (token >> ML_BITS) as usize;
                if ll == 15 {
                    ll = ll.wrapping_add(read_long_length_no_check(&mut ip));
                }
                if (oend as usize - op as usize) < ll {
                    return -1;
                }
                mem_move(op, ip, ll);
                op = op.wrapping_add(ll);
                ip = ip.wrapping_add(ll);
                if (oend as usize - op as usize) < MFLIMIT {
                    if op == oend {
                        break;
                    }
                    return -1;
                }
            }

            /* match */
            {
                let mut ml = (token & 15) as usize;
                let offset = read_le16(ip) as usize;
                ip = ip.add(2);

                if ml == 15 {
                    ml = ml.wrapping_add(read_long_length_no_check(&mut ip));
                }
                ml += MINMATCH;

                if (oend as usize - op as usize) < ml {
                    return -1;
                }

                {
                    let mut r#match = op.wrapping_sub(offset) as *const u8;

                    if offset > (op as usize - prefix_start as usize) + dict_size {
                        return -1;
                    }

                    if offset > (op as usize - prefix_start as usize) {
                        let dict_end = dict_start.wrapping_add(dict_size);
                        let ext_match = dict_end
                            .wrapping_sub(offset - (op as usize - prefix_start as usize));
                        let extml = dict_end as usize - ext_match as usize;
                        if extml > ml {
                            mem_move(op, ext_match, ml);
                            op = op.wrapping_add(ml);
                            ml = 0;
                        } else {
                            mem_move(op, ext_match, extml);
                            op = op.wrapping_add(extml);
                            ml -= extml;
                        }
                        r#match = prefix_start;
                    }

                    for u in 0..ml {
                        *op.add(u) = *r#match.add(u);
                    }
                }
                op = op.wrapping_add(ml);
                if (oend as usize - op as usize) < LASTLITERALS {
                    return -1;
                }
            }
        }
        (ip as usize - istart as usize) as c_int
    }
}

const RVL_ERROR: usize = usize::MAX;

unsafe fn read_variable_length(ip: &mut *const u8, ilimit: *const u8, initial_check: bool) -> usize {
    unsafe {
        let mut length: usize = 0;
        if initial_check && *ip >= ilimit {
            return RVL_ERROR;
        }
        let mut s = **ip as usize;
        *ip = (*ip).add(1);
        length = length.wrapping_add(s);
        if *ip > ilimit {
            return RVL_ERROR;
        }
        if s != 255 {
            return length;
        }
        loop {
            s = **ip as usize;
            *ip = (*ip).add(1);
            length = length.wrapping_add(s);
            if *ip > ilimit {
                return RVL_ERROR;
            }
            if s == 255 {
                continue;
            }
            break;
        }
        length
    }
}

unsafe fn lz4_decompress_generic(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    output_size: c_int,
    partial_decoding: EarlyEnd,
    dict: DictDirective,
    low_prefix: *const u8,
    dict_start: *const u8,
    dict_size: usize,
) -> c_int {
    unsafe {
        if src.is_null() || output_size < 0 {
            return -1;
        }

        let mut ip = src as *const u8;
        let iend = ip.wrapping_add(src_size as usize);

        let mut op = dst as *mut u8;
        let oend = op.wrapping_add(output_size as usize);
        let mut cpy: *mut u8;

        let dict_end: *const u8 = if dict_start.is_null() {
            core::ptr::null()
        } else {
            dict_start.wrapping_add(dict_size)
        };

        let check_offset = dict_size < 64 * 1024;

        let shortiend = iend.wrapping_sub(14).wrapping_sub(2);
        let shortoend = oend.wrapping_sub(14).wrapping_sub(18);

        let mut r#match: *const u8 = core::ptr::null();
        let mut offset: usize = 0;
        let mut token: u32 = 0;
        let mut length: usize = 0;

        let partial = partial_decoding == PartialDecode;

        macro_rules! output_error {
            () => {
                return -((ip as isize - src as isize) as c_int) - 1;
            };
        }

        if output_size == 0 {
            if partial {
                return 0;
            }
            return if src_size == 1 && *ip == 0 { 0 } else { -1 };
        }
        if src_size == 0 {
            return -1;
        }

        /* entry point into the safe loop: 0 = top, 1 = safe_literal_copy,
         * 2 = _copy_match, 3 = safe_match_copy */
        let mut entry: u32 = 0;

        if (oend as usize - op as usize) >= FASTLOOP_SAFE_DISTANCE {
            'fast: loop {
                token = *ip as u32;
                ip = ip.add(1);
                length = (token >> ML_BITS) as usize;

                /* decode literal length */
                if length == RUN_MASK as usize {
                    let addl =
                        read_variable_length(&mut ip, iend.wrapping_sub(RUN_MASK as usize), true);
                    if addl == RVL_ERROR {
                        output_error!();
                    }
                    length = length.wrapping_add(addl);
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                    if (ip as usize).wrapping_add(length) < (ip as usize) {
                        output_error!();
                    }

                    if op.wrapping_add(length) > oend.wrapping_sub(32)
                        || ip.wrapping_add(length) > iend.wrapping_sub(32)
                    {
                        entry = 1;
                        break 'fast;
                    }
                    wild_copy32(op, ip, op.wrapping_add(length));
                    ip = ip.wrapping_add(length);
                    op = op.wrapping_add(length);
                } else if ip <= iend.wrapping_sub(16 + 1) {
                    copy16(op, ip);
                    ip = ip.wrapping_add(length);
                    op = op.wrapping_add(length);
                } else {
                    entry = 1;
                    break 'fast;
                }

                /* get offset */
                offset = read_le16(ip) as usize;
                ip = ip.add(2);
                r#match = op.wrapping_sub(offset);

                /* get matchlength */
                length = (token & ML_MASK) as usize;

                if length == ML_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip,
                        iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                        false,
                    );
                    if addl == RVL_ERROR {
                        output_error!();
                    }
                    length = length.wrapping_add(addl);
                    length = length.wrapping_add(MINMATCH);
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                    if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                        entry = 3;
                        break 'fast;
                    }
                } else {
                    length += MINMATCH;
                    if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                        entry = 3;
                        break 'fast;
                    }

                    if dict == WithPrefix64k || r#match >= low_prefix {
                        if offset >= 8 {
                            copy8(op, r#match);
                            copy8(op.add(8), r#match.add(8));
                            copy2(op.add(16), r#match.add(16));
                            op = op.wrapping_add(length);
                            continue 'fast;
                        }
                    }
                }

                if check_offset && r#match.wrapping_add(dict_size) < low_prefix {
                    output_error!();
                }

                /* match starting within external dictionary */
                if dict == UsingExtDict && r#match < low_prefix {
                    if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                        if partial {
                            length = min_usize(length, oend as usize - op as usize);
                        } else {
                            output_error!();
                        }
                    }

                    if length <= (low_prefix as usize - r#match as usize) {
                        mem_move(
                            op,
                            dict_end.wrapping_sub(low_prefix as usize - r#match as usize),
                            length,
                        );
                        op = op.wrapping_add(length);
                    } else {
                        let copy_size = low_prefix as usize - r#match as usize;
                        let rest_size = length - copy_size;
                        mem_copy(op, dict_end.wrapping_sub(copy_size), copy_size);
                        op = op.wrapping_add(copy_size);
                        if rest_size > (op as usize - low_prefix as usize) {
                            let end_of_match = op.wrapping_add(rest_size);
                            let mut copy_from = low_prefix;
                            while op < end_of_match {
                                *op = *copy_from;
                                op = op.add(1);
                                copy_from = copy_from.add(1);
                            }
                        } else {
                            mem_copy(op, low_prefix, rest_size);
                            op = op.wrapping_add(rest_size);
                        }
                    }
                    continue 'fast;
                }

                /* copy match within block */
                cpy = op.wrapping_add(length);

                if offset < 16 {
                    lz4_memcpy_using_offset(op, r#match, cpy, offset);
                } else {
                    wild_copy32(op, r#match, cpy);
                }

                op = cpy;
            }
        }

        /* safe_decode */
        'safe: loop {
            let mut e = entry;
            entry = 0;

            if e == 0 {
                token = *ip as u32;
                ip = ip.add(1);
                length = (token >> ML_BITS) as usize;

                if length != RUN_MASK as usize && ((ip < shortiend) & (op <= shortoend)) {
                    /* Copy the literals */
                    copy16(op, ip);
                    op = op.wrapping_add(length);
                    ip = ip.wrapping_add(length);

                    length = (token & ML_MASK) as usize;
                    offset = read_le16(ip) as usize;
                    ip = ip.add(2);
                    r#match = op.wrapping_sub(offset);

                    if length != ML_MASK as usize
                        && offset >= 8
                        && (dict == WithPrefix64k || r#match >= low_prefix)
                    {
                        copy8(op, r#match);
                        copy8(op.add(8), r#match.add(8));
                        copy2(op.add(16), r#match.add(16));
                        op = op.wrapping_add(length + MINMATCH);
                        continue 'safe;
                    }

                    e = 2; /* goto _copy_match */
                } else {
                    /* decode literal length */
                    if length == RUN_MASK as usize {
                        let addl = read_variable_length(
                            &mut ip,
                            iend.wrapping_sub(RUN_MASK as usize),
                            true,
                        );
                        if addl == RVL_ERROR {
                            output_error!();
                        }
                        length = length.wrapping_add(addl);
                        if (op as usize).wrapping_add(length) < (op as usize) {
                            output_error!();
                        }
                        if (ip as usize).wrapping_add(length) < (ip as usize) {
                            output_error!();
                        }
                    }
                    e = 1;
                }
            }

            if e <= 1 {
                /* safe_literal_copy */
                cpy = op.wrapping_add(length);

                if cpy > oend.wrapping_sub(MFLIMIT)
                    || ip.wrapping_add(length) > iend.wrapping_sub(2 + 1 + LASTLITERALS)
                {
                    if partial {
                        if ip.wrapping_add(length) > iend {
                            length = iend as usize - ip as usize;
                            cpy = op.wrapping_add(length);
                        }
                        if cpy > oend {
                            cpy = oend;
                            length = oend as usize - op as usize;
                        }
                    } else {
                        if ip.wrapping_add(length) != iend || cpy > oend {
                            output_error!();
                        }
                    }
                    mem_move(op, ip, length);
                    ip = ip.wrapping_add(length);
                    op = op.wrapping_add(length);
                    if !partial || cpy == oend || ip >= iend.wrapping_sub(2) {
                        break 'safe;
                    }
                } else {
                    wild_copy8(op, ip, cpy);
                    ip = ip.wrapping_add(length);
                    op = cpy;
                }

                /* get offset */
                offset = read_le16(ip) as usize;
                ip = ip.add(2);
                r#match = op.wrapping_sub(offset);

                /* get matchlength */
                length = (token & ML_MASK) as usize;
                e = 2;
            }

            if e <= 2 {
                /* _copy_match */
                if length == ML_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip,
                        iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                        false,
                    );
                    if addl == RVL_ERROR {
                        output_error!();
                    }
                    length = length.wrapping_add(addl);
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                }
                length = length.wrapping_add(MINMATCH);
            }

            /* safe_match_copy */
            if check_offset && r#match.wrapping_add(dict_size) < low_prefix {
                output_error!();
            }

            if dict == UsingExtDict && r#match < low_prefix {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partial {
                        length = min_usize(length, oend as usize - op as usize);
                    } else {
                        output_error!();
                    }
                }

                if length <= (low_prefix as usize - r#match as usize) {
                    mem_move(
                        op,
                        dict_end.wrapping_sub(low_prefix as usize - r#match as usize),
                        length,
                    );
                    op = op.wrapping_add(length);
                } else {
                    let copy_size = low_prefix as usize - r#match as usize;
                    let rest_size = length - copy_size;
                    mem_copy(op, dict_end.wrapping_sub(copy_size), copy_size);
                    op = op.wrapping_add(copy_size);
                    if rest_size > (op as usize - low_prefix as usize) {
                        let end_of_match = op.wrapping_add(rest_size);
                        let mut copy_from = low_prefix;
                        while op < end_of_match {
                            *op = *copy_from;
                            op = op.add(1);
                            copy_from = copy_from.add(1);
                        }
                    } else {
                        mem_copy(op, low_prefix, rest_size);
                        op = op.wrapping_add(rest_size);
                    }
                }
                continue 'safe;
            }

            /* copy match within block */
            cpy = op.wrapping_add(length);

            if partial && cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let mlen = min_usize(length, oend as usize - op as usize);
                let match_end = r#match.wrapping_add(mlen);
                let copy_end = op.wrapping_add(mlen);
                if match_end as usize > op as usize {
                    while op < copy_end {
                        *op = *r#match;
                        op = op.add(1);
                        r#match = r#match.add(1);
                    }
                } else {
                    mem_copy(op, r#match, mlen);
                }
                op = copy_end;
                if op == oend {
                    break 'safe;
                }
                continue 'safe;
            }

            if offset < 8 {
                write32(op, 0);
                *op = *r#match;
                *op.add(1) = *r#match.add(1);
                *op.add(2) = *r#match.add(2);
                *op.add(3) = *r#match.add(3);
                r#match = r#match.wrapping_add(INC32TABLE[offset] as usize);
                copy4(op.add(4), r#match);
                r#match = r#match.wrapping_offset(-(DEC64TABLE[offset] as isize));
            } else {
                copy8(op, r#match);
                r#match = r#match.wrapping_add(8);
            }
            op = op.wrapping_add(8);

            if cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let o_copy_limit = oend.wrapping_sub(WILDCOPYLENGTH - 1);
                if cpy > oend.wrapping_sub(LASTLITERALS) {
                    output_error!();
                }
                if op < o_copy_limit {
                    wild_copy8(op, r#match, o_copy_limit);
                    r#match = r#match.wrapping_add(o_copy_limit as usize - op as usize);
                    op = o_copy_limit;
                }
                while op < cpy {
                    *op = *r#match;
                    op = op.add(1);
                    r#match = r#match.add(1);
                }
            } else {
                copy8(op, r#match);
                if length > 16 {
                    wild_copy8(op.add(8), r#match.add(8), cpy);
                }
            }
            op = cpy;
        }

        (op as usize - dst as usize) as c_int
    }
}

/* ===== decoding API ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_decompressed_size: c_int,
) -> c_int {
    unsafe {
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            max_decompressed_size,
            DecodeFullBlock,
            NoDict,
            dest as *const u8,
            core::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial(
    src: *const c_char,
    dst: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    unsafe {
        let dst_capacity = min_c_int(target_output_size, dst_capacity);
        lz4_decompress_generic(
            src,
            dst,
            compressed_size,
            dst_capacity,
            PartialDecode,
            NoDict,
            dst as *const u8,
            core::ptr::null(),
            0,
        )
    }
}

#[inline]
fn min_c_int(a: c_int, b: c_int) -> c_int {
    if a < b { a } else { b }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
) -> c_int {
    unsafe {
        lz4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            original_size,
            0,
            core::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
) -> c_int {
    unsafe {
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            max_output_size,
            DecodeFullBlock,
            WithPrefix64k,
            (dest as *const u8).wrapping_sub(64 * 1024),
            core::ptr::null(),
            0,
        )
    }
}

unsafe fn lz4_decompress_safe_partial_with_prefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    unsafe {
        let dst_capacity = min_c_int(target_output_size, dst_capacity);
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            dst_capacity,
            PartialDecode,
            WithPrefix64k,
            (dest as *const u8).wrapping_sub(64 * 1024),
            core::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
) -> c_int {
    unsafe {
        lz4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            original_size,
            64 * 1024,
            core::ptr::null(),
            0,
        )
    }
}

unsafe fn lz4_decompress_safe_with_small_prefix(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
    prefix_size: usize,
) -> c_int {
    unsafe {
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            max_output_size,
            DecodeFullBlock,
            NoDict,
            (dest as *const u8).wrapping_sub(prefix_size),
            core::ptr::null(),
            0,
        )
    }
}

unsafe fn lz4_decompress_safe_partial_with_small_prefix(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    dst_capacity: c_int,
    prefix_size: usize,
) -> c_int {
    unsafe {
        let dst_capacity = min_c_int(target_output_size, dst_capacity);
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            dst_capacity,
            PartialDecode,
            NoDict,
            (dest as *const u8).wrapping_sub(prefix_size),
            core::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_forceExtDict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
    dict_start: *const c_void,
    dict_size: usize,
) -> c_int {
    unsafe {
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            max_output_size,
            DecodeFullBlock,
            UsingExtDict,
            dest as *const u8,
            dict_start as *const u8,
            dict_size,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial_forceExtDict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    dst_capacity: c_int,
    dict_start: *const c_void,
    dict_size: usize,
) -> c_int {
    unsafe {
        let dst_capacity = min_c_int(target_output_size, dst_capacity);
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            dst_capacity,
            PartialDecode,
            UsingExtDict,
            dest as *const u8,
            dict_start as *const u8,
            dict_size,
        )
    }
}

unsafe fn lz4_decompress_fast_ext_dict(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
    dict_start: *const c_void,
    dict_size: usize,
) -> c_int {
    unsafe {
        lz4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            original_size,
            0,
            dict_start as *const u8,
            dict_size,
        )
    }
}

unsafe fn lz4_decompress_safe_double_dict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
    prefix_size: usize,
    dict_start: *const c_void,
    dict_size: usize,
) -> c_int {
    unsafe {
        lz4_decompress_generic(
            source,
            dest,
            compressed_size,
            max_output_size,
            DecodeFullBlock,
            UsingExtDict,
            (dest as *const u8).wrapping_sub(prefix_size),
            dict_start as *const u8,
            dict_size,
        )
    }
}

/* ===== streaming decompression ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_createStreamDecode() -> *mut LZ4StreamDecode {
    unsafe { alloc_and_zero(core::mem::size_of::<LZ4StreamDecode>()) as *mut LZ4StreamDecode }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamDecode(lz4_stream: *mut LZ4StreamDecode) -> c_int {
    unsafe {
        if lz4_stream.is_null() {
            return 0;
        }
        free(lz4_stream as *mut c_void);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setStreamDecode(
    lz4_stream_decode: *mut LZ4StreamDecode,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        let lz4sd = lz4_stream_decode as *mut LZ4StreamDecodeInternal;
        (*lz4sd).prefix_size = dict_size as usize;
        if dict_size != 0 {
            (*lz4sd).prefix_end = (dictionary as *const u8).wrapping_add(dict_size as usize);
        } else {
            (*lz4sd).prefix_end = dictionary as *const u8;
        }
        (*lz4sd).external_dict = core::ptr::null();
        (*lz4sd).ext_dict_size = 0;
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_decoderRingBufferSize(max_block_size: c_int) -> c_int {
    let mut max_block_size = max_block_size;
    if max_block_size < 0 {
        return 0;
    }
    if max_block_size > LZ4_MAX_INPUT_SIZE {
        return 0;
    }
    if max_block_size < 16 {
        max_block_size = 16;
    }
    65536 + 14 + max_block_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_continue(
    lz4_stream_decode: *mut LZ4StreamDecode,
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
) -> c_int {
    unsafe {
        let lz4sd = lz4_stream_decode as *mut LZ4StreamDecodeInternal;
        let result;

        if (*lz4sd).prefix_size == 0 {
            result = LZ4_decompress_safe(source, dest, compressed_size, max_output_size);
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefix_size = result as usize;
            (*lz4sd).prefix_end = (dest as *const u8).wrapping_add(result as usize);
        } else if (*lz4sd).prefix_end == dest as *const u8 {
            if (*lz4sd).prefix_size >= 64 * 1024 - 1 {
                result =
                    LZ4_decompress_safe_withPrefix64k(source, dest, compressed_size, max_output_size);
            } else if (*lz4sd).ext_dict_size == 0 {
                result = lz4_decompress_safe_with_small_prefix(
                    source,
                    dest,
                    compressed_size,
                    max_output_size,
                    (*lz4sd).prefix_size,
                );
            } else {
                result = lz4_decompress_safe_double_dict(
                    source,
                    dest,
                    compressed_size,
                    max_output_size,
                    (*lz4sd).prefix_size,
                    (*lz4sd).external_dict as *const c_void,
                    (*lz4sd).ext_dict_size,
                );
            }
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefix_size += result as usize;
            (*lz4sd).prefix_end = (*lz4sd).prefix_end.wrapping_add(result as usize);
        } else {
            (*lz4sd).ext_dict_size = (*lz4sd).prefix_size;
            (*lz4sd).external_dict = (*lz4sd).prefix_end.wrapping_sub((*lz4sd).ext_dict_size);
            result = LZ4_decompress_safe_forceExtDict(
                source,
                dest,
                compressed_size,
                max_output_size,
                (*lz4sd).external_dict as *const c_void,
                (*lz4sd).ext_dict_size,
            );
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefix_size = result as usize;
            (*lz4sd).prefix_end = (dest as *const u8).wrapping_add(result as usize);
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_continue(
    lz4_stream_decode: *mut LZ4StreamDecode,
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
) -> c_int {
    unsafe {
        let lz4sd = lz4_stream_decode as *mut LZ4StreamDecodeInternal;
        let result;

        if (*lz4sd).prefix_size == 0 {
            result = LZ4_decompress_fast(source, dest, original_size);
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefix_size = original_size as usize;
            (*lz4sd).prefix_end = (dest as *const u8).wrapping_add(original_size as usize);
        } else if (*lz4sd).prefix_end == dest as *const u8 {
            result = lz4_decompress_unsafe_generic(
                source as *const u8,
                dest as *mut u8,
                original_size,
                (*lz4sd).prefix_size,
                (*lz4sd).external_dict,
                (*lz4sd).ext_dict_size,
            );
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefix_size += original_size as usize;
            (*lz4sd).prefix_end = (*lz4sd).prefix_end.wrapping_add(original_size as usize);
        } else {
            (*lz4sd).ext_dict_size = (*lz4sd).prefix_size;
            (*lz4sd).external_dict = (*lz4sd).prefix_end.wrapping_sub((*lz4sd).ext_dict_size);
            result = lz4_decompress_fast_ext_dict(
                source,
                dest,
                original_size,
                (*lz4sd).external_dict as *const c_void,
                (*lz4sd).ext_dict_size,
            );
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefix_size = original_size as usize;
            (*lz4sd).prefix_end = (dest as *const u8).wrapping_add(original_size as usize);
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
    dict_start: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        if dict_size == 0 {
            return LZ4_decompress_safe(source, dest, compressed_size, max_output_size);
        }
        if dict_start.wrapping_add(dict_size as usize) == dest as *const c_char {
            if dict_size >= 64 * 1024 - 1 {
                return LZ4_decompress_safe_withPrefix64k(
                    source,
                    dest,
                    compressed_size,
                    max_output_size,
                );
            }
            return lz4_decompress_safe_with_small_prefix(
                source,
                dest,
                compressed_size,
                max_output_size,
                dict_size as usize,
            );
        }
        LZ4_decompress_safe_forceExtDict(
            source,
            dest,
            compressed_size,
            max_output_size,
            dict_start as *const c_void,
            dict_size as usize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    dst_capacity: c_int,
    dict_start: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        if dict_size == 0 {
            return LZ4_decompress_safe_partial(
                source,
                dest,
                compressed_size,
                target_output_size,
                dst_capacity,
            );
        }
        if dict_start.wrapping_add(dict_size as usize) == dest as *const c_char {
            if dict_size >= 64 * 1024 - 1 {
                return lz4_decompress_safe_partial_with_prefix64k(
                    source,
                    dest,
                    compressed_size,
                    target_output_size,
                    dst_capacity,
                );
            }
            return lz4_decompress_safe_partial_with_small_prefix(
                source,
                dest,
                compressed_size,
                target_output_size,
                dst_capacity,
                dict_size as usize,
            );
        }
        LZ4_decompress_safe_partial_forceExtDict(
            source,
            dest,
            compressed_size,
            target_output_size,
            dst_capacity,
            dict_start as *const c_void,
            dict_size as usize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
    dict_start: *const c_char,
    dict_size: c_int,
) -> c_int {
    unsafe {
        if dict_size == 0 || dict_start.wrapping_add(dict_size as usize) == dest as *const c_char {
            return lz4_decompress_unsafe_generic(
                source as *const u8,
                dest as *mut u8,
                original_size,
                dict_size as usize,
                core::ptr::null(),
                0,
            );
        }
        lz4_decompress_fast_ext_dict(
            source,
            dest,
            original_size,
            dict_start as *const c_void,
            dict_size as usize,
        )
    }
}

/* ===== obsolete functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput(
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    max_output_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_default(source, dest, input_size, max_output_size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress(
    src: *const c_char,
    dest: *mut c_char,
    src_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_default(src, dest, src_size, lz4_compress_bound(src_size)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_size: c_int,
) -> c_int {
    unsafe { LZ4_compress_fast_extState(state, src, dst, src_size, dst_size, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_fast_extState(state, src, dst, src_size, lz4_compress_bound(src_size), 1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_continue(
    lz4_stream: *mut LZ4Stream,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    unsafe { LZ4_compress_fast_continue(lz4_stream, src, dst, src_size, dst_capacity, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_continue(
    lz4_stream: *mut LZ4Stream,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_fast_continue(
            lz4_stream,
            source,
            dest,
            input_size,
            lz4_compress_bound(input_size),
            1,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress(
    source: *const c_char,
    dest: *mut c_char,
    output_size: c_int,
) -> c_int {
    unsafe { LZ4_decompress_fast(source, dest, output_size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress_unknownOutputSize(
    source: *const c_char,
    dest: *mut c_char,
    isize_: c_int,
    max_output_size: c_int,
) -> c_int {
    unsafe { LZ4_decompress_safe(source, dest, isize_, max_output_size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamState() -> c_int {
    core::mem::size_of::<LZ4Stream>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamState(
    state: *mut c_void,
    _input_buffer: *mut c_char,
) -> c_int {
    unsafe {
        LZ4_resetStream(state as *mut LZ4Stream);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_create(_input_buffer: *mut c_char) -> *mut c_void {
    LZ4_createStream() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char {
    unsafe { (*(state as *mut LZ4StreamInternal)).dictionary as *mut c_char }
}
