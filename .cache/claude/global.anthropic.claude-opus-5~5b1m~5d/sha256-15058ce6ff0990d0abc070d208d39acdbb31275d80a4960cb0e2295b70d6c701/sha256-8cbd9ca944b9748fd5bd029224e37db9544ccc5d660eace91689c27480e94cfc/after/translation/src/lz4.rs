//! Translation of c_src/src/lz4.c (LZ4 v1.10.0)
//!
//! Target assumptions matching the C build on x86_64 Linux/gcc:
//!  - little endian, unaligned access OK
//!  - LZ4_FAST_DEC_LOOP == 1
//!  - reg_t == u64 (STEPSIZE == 8)
//!  - LZ4_HEAPMODE == 0
//!  - sizeof(void*) == 8, so tableType `byPtr` is never selected at runtime
//!  - LZ4_DISTANCE_MAX == LZ4_DISTANCE_ABSOLUTE_MAX == 65535

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(n: usize, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
}

/* ---------------- constants ---------------- */

pub const LZ4_VERSION_MAJOR: c_int = 1;
pub const LZ4_VERSION_MINOR: c_int = 10;
pub const LZ4_VERSION_RELEASE: c_int = 0;
pub const LZ4_VERSION_NUMBER: c_int =
    LZ4_VERSION_MAJOR * 100 * 100 + LZ4_VERSION_MINOR * 100 + LZ4_VERSION_RELEASE;
pub const LZ4_VERSION_STRING: &[u8; 7] = b"1.10.0\0";

pub const LZ4_MEMORY_USAGE: u32 = 14;
pub const LZ4_HASHLOG: u32 = LZ4_MEMORY_USAGE - 2;
pub const LZ4_HASHTABLESIZE: usize = 1usize << LZ4_MEMORY_USAGE;
pub const LZ4_HASH_SIZE_U32: usize = 1usize << LZ4_HASHLOG;
pub const LZ4_STREAM_MINSIZE: usize = (1usize << LZ4_MEMORY_USAGE) + 32;
pub const LZ4_STREAMDECODE_MINSIZE: usize = 32;

pub const LZ4_MAX_INPUT_SIZE: u32 = 0x7E00_0000;
pub const LZ4_ACCELERATION_DEFAULT: c_int = 1;
pub const LZ4_ACCELERATION_MAX: c_int = 65537;

pub const MINMATCH: usize = 4;
pub const WILDCOPYLENGTH: usize = 8;
pub const LASTLITERALS: usize = 5;
pub const MFLIMIT: usize = 12;
pub const MATCH_SAFEGUARD_DISTANCE: usize = (2 * WILDCOPYLENGTH) - MINMATCH;
pub const FASTLOOP_SAFE_DISTANCE: usize = 64;
pub const LZ4_MINLENGTH: c_int = (MFLIMIT + 1) as c_int;

pub const LZ4_DISTANCE_MAX: u32 = 65535;
pub const LZ4_DISTANCE_ABSOLUTE_MAX: u32 = 65535;

pub const ML_BITS: u32 = 4;
pub const ML_MASK: u32 = (1u32 << ML_BITS) - 1;
pub const RUN_BITS: u32 = 8 - ML_BITS;
pub const RUN_MASK: u32 = (1u32 << RUN_BITS) - 1;

pub const LZ4_64KLIMIT: c_int = (64 * 1024) + (MFLIMIT as c_int - 1);
pub const LZ4_SKIP_TRIGGER: u32 = 6;

pub const STEPSIZE: usize = 8;
pub const HASH_UNIT: usize = 8;

pub const KB: usize = 1024;

#[inline]
pub fn LZ4_COMPRESSBOUND(isize_: c_int) -> c_int {
    if (isize_ as u32) > LZ4_MAX_INPUT_SIZE {
        0
    } else {
        isize_ + (isize_ / 255) + 16
    }
}

/* ---------------- types ---------------- */

#[repr(C)]
pub struct LZ4_stream_t_internal {
    pub hashTable: [u32; LZ4_HASH_SIZE_U32],
    pub dictionary: *const u8,
    pub dictCtx: *const LZ4_stream_t_internal,
    pub currentOffset: u32,
    pub tableType: u32,
    pub dictSize: u32,
}

/// Same size/alignment as the C `union LZ4_stream_u` (16416 bytes, align 8).
#[repr(C, align(8))]
pub struct LZ4_stream_t {
    pub minStateSize: [u8; LZ4_STREAM_MINSIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4_streamDecode_t_internal {
    pub externalDict: *const u8,
    pub prefixEnd: *const u8,
    pub extDictSize: usize,
    pub prefixSize: usize,
}

#[repr(C, align(8))]
pub struct LZ4_streamDecode_t {
    pub minStateSize: [u8; LZ4_STREAMDECODE_MINSIZE],
}

pub const NOT_LIMITED: c_int = 0;
pub const LIMITED_OUTPUT: c_int = 1;
pub const FILL_OUTPUT: c_int = 2;

pub const CLEARED_TABLE: u32 = 0;
pub const BY_PTR: u32 = 1;
pub const BY_U32: u32 = 2;
pub const BY_U16: u32 = 3;

pub const NO_DICT: c_int = 0;
pub const WITH_PREFIX_64K: c_int = 1;
pub const USING_EXT_DICT: c_int = 2;
pub const USING_DICT_CTX: c_int = 3;

pub const NO_DICT_ISSUE: c_int = 0;
pub const DICT_SMALL: c_int = 1;

pub const DECODE_FULL_BLOCK: c_int = 0;
pub const PARTIAL_DECODE: c_int = 1;

/* ---------------- low-level memory helpers ---------------- */

#[inline(always)]
pub unsafe fn read16(p: *const u8) -> u16 {
    (p as *const u16).read_unaligned()
}
#[inline(always)]
pub unsafe fn read32(p: *const u8) -> u32 {
    (p as *const u32).read_unaligned()
}
#[inline(always)]
pub unsafe fn read64(p: *const u8) -> u64 {
    (p as *const u64).read_unaligned()
}
#[inline(always)]
pub unsafe fn read_arch(p: *const u8) -> u64 {
    read64(p)
}
#[inline(always)]
pub unsafe fn write16(p: *mut u8, v: u16) {
    (p as *mut u16).write_unaligned(v)
}
#[inline(always)]
pub unsafe fn write32(p: *mut u8, v: u32) {
    (p as *mut u32).write_unaligned(v)
}
#[inline(always)]
pub unsafe fn readLE16(p: *const u8) -> u16 {
    // little-endian host
    read16(p)
}
#[inline(always)]
pub unsafe fn writeLE16(p: *mut u8, v: u16) {
    write16(p, v)
}
#[inline(always)]
pub unsafe fn memcpy_n(dst: *mut u8, src: *const u8, n: usize) {
    ptr::copy_nonoverlapping(src, dst, n)
}
#[inline(always)]
pub unsafe fn memmove_n(dst: *mut u8, src: *const u8, n: usize) {
    ptr::copy(src, dst, n)
}
#[inline(always)]
pub unsafe fn memset_n(dst: *mut u8, v: u8, n: usize) {
    ptr::write_bytes(dst, v, n)
}

#[inline(always)]
pub fn pdiff(a: *const u8, b: *const u8) -> isize {
    (a as isize).wrapping_sub(b as isize)
}

#[inline(always)]
pub unsafe fn LZ4_wildCopy8(dst: *mut u8, src: *const u8, dst_end: *mut u8) {
    let mut d = dst;
    let mut s = src;
    loop {
        memcpy_n(d, s, 8);
        d = d.wrapping_add(8);
        s = s.wrapping_add(8);
        if d >= dst_end {
            break;
        }
    }
}

#[inline(always)]
pub unsafe fn LZ4_wildCopy32(dst: *mut u8, src: *const u8, dst_end: *mut u8) {
    let mut d = dst;
    let mut s = src;
    loop {
        memcpy_n(d, s, 16);
        memcpy_n(d.wrapping_add(16), s.wrapping_add(16), 16);
        d = d.wrapping_add(32);
        s = s.wrapping_add(32);
        if d >= dst_end {
            break;
        }
    }
}

pub static INC32TABLE: [u32; 8] = [0, 1, 2, 1, 0, 4, 4, 4];
pub static DEC64TABLE: [i32; 8] = [0, 0, 0, -1, -4, 1, 2, 3];

#[inline(always)]
unsafe fn LZ4_memcpy_using_offset_base(
    mut dst: *mut u8,
    mut src: *const u8,
    dst_end: *mut u8,
    offset: usize,
) {
    if offset < 8 {
        write32(dst, 0);
        *dst.add(0) = *src.add(0);
        *dst.add(1) = *src.add(1);
        *dst.add(2) = *src.add(2);
        *dst.add(3) = *src.add(3);
        src = src.wrapping_add(INC32TABLE[offset] as usize);
        memcpy_n(dst.wrapping_add(4), src, 4);
        src = src.wrapping_offset(-(DEC64TABLE[offset] as isize));
        dst = dst.wrapping_add(8);
    } else {
        memcpy_n(dst, src, 8);
        dst = dst.wrapping_add(8);
        src = src.wrapping_add(8);
    }
    LZ4_wildCopy8(dst, src, dst_end);
}

#[inline(always)]
unsafe fn LZ4_memcpy_using_offset(
    mut dst: *mut u8,
    src: *const u8,
    dst_end: *mut u8,
    offset: usize,
) {
    let mut v: [u8; 8] = [0; 8];
    match offset {
        1 => {
            memset_n(v.as_mut_ptr(), *src, 8);
        }
        2 => {
            memcpy_n(v.as_mut_ptr(), src, 2);
            memcpy_n(v.as_mut_ptr().add(2), src, 2);
            let tmp = [v[0], v[1], v[2], v[3]];
            memcpy_n(v.as_mut_ptr().add(4), tmp.as_ptr(), 4);
        }
        4 => {
            memcpy_n(v.as_mut_ptr(), src, 4);
            memcpy_n(v.as_mut_ptr().add(4), src, 4);
        }
        _ => {
            LZ4_memcpy_using_offset_base(dst, src, dst_end, offset);
            return;
        }
    }
    memcpy_n(dst, v.as_ptr(), 8);
    dst = dst.wrapping_add(8);
    while dst < dst_end {
        memcpy_n(dst, v.as_ptr(), 8);
        dst = dst.wrapping_add(8);
    }
}

#[inline(always)]
pub fn LZ4_NbCommonBytes(val: u64) -> u32 {
    // little-endian, 64-bit
    (val.trailing_zeros()) >> 3
}

#[inline(always)]
pub unsafe fn LZ4_count(p_in: *const u8, p_match: *const u8, p_in_limit: *const u8) -> u32 {
    let p_start = p_in;
    let mut pin = p_in;
    let mut pm = p_match;

    if pin < p_in_limit.wrapping_sub(STEPSIZE - 1) {
        let diff = read_arch(pm) ^ read_arch(pin);
        if diff == 0 {
            pin = pin.wrapping_add(STEPSIZE);
            pm = pm.wrapping_add(STEPSIZE);
        } else {
            return LZ4_NbCommonBytes(diff);
        }
    }

    while pin < p_in_limit.wrapping_sub(STEPSIZE - 1) {
        let diff = read_arch(pm) ^ read_arch(pin);
        if diff == 0 {
            pin = pin.wrapping_add(STEPSIZE);
            pm = pm.wrapping_add(STEPSIZE);
            continue;
        }
        pin = pin.wrapping_add(LZ4_NbCommonBytes(diff) as usize);
        return pdiff(pin, p_start) as u32;
    }

    if pin < p_in_limit.wrapping_sub(3) && read32(pm) == read32(pin) {
        pin = pin.wrapping_add(4);
        pm = pm.wrapping_add(4);
    }
    if pin < p_in_limit.wrapping_sub(1) && read16(pm) == read16(pin) {
        pin = pin.wrapping_add(2);
        pm = pm.wrapping_add(2);
    }
    if pin < p_in_limit && *pm == *pin {
        pin = pin.wrapping_add(1);
    }
    pdiff(pin, p_start) as u32
}

fn LZ4_isAligned(p: *const c_void, alignment: usize) -> bool {
    ((p as usize) & (alignment - 1)) == 0
}

/* ---------------- hashing ---------------- */

#[inline(always)]
pub fn LZ4_hash4(sequence: u32, table_type: u32) -> u32 {
    if table_type == BY_U16 {
        (sequence.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - (LZ4_HASHLOG + 1))
    } else {
        (sequence.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - LZ4_HASHLOG)
    }
}

#[inline(always)]
pub fn LZ4_hash5(sequence: u64, table_type: u32) -> u32 {
    let hash_log = if table_type == BY_U16 {
        LZ4_HASHLOG + 1
    } else {
        LZ4_HASHLOG
    };
    // little endian
    let prime5bytes: u64 = 889523592379u64;
    (((sequence << 24).wrapping_mul(prime5bytes)) >> (64 - hash_log)) as u32
}

#[inline(always)]
pub unsafe fn LZ4_hashPosition(p: *const u8, table_type: u32) -> u32 {
    if table_type != BY_U16 {
        return LZ4_hash5(read_arch(p), table_type);
    }
    LZ4_hash4(read32(p), table_type)
}

#[inline(always)]
unsafe fn LZ4_clearHash(h: u32, table_base: *mut c_void, table_type: u32) {
    match table_type {
        BY_PTR => {
            let t = table_base as *mut *const u8;
            *t.add(h as usize) = ptr::null();
        }
        BY_U32 => {
            let t = table_base as *mut u32;
            *t.add(h as usize) = 0;
        }
        BY_U16 => {
            let t = table_base as *mut u16;
            *t.add(h as usize) = 0;
        }
        _ => {}
    }
}

#[inline(always)]
unsafe fn LZ4_putIndexOnHash(idx: u32, h: u32, table_base: *mut c_void, table_type: u32) {
    match table_type {
        BY_U32 => {
            let t = table_base as *mut u32;
            *t.add(h as usize) = idx;
        }
        BY_U16 => {
            let t = table_base as *mut u16;
            *t.add(h as usize) = idx as u16;
        }
        _ => {}
    }
}

#[inline(always)]
unsafe fn LZ4_putPositionOnHash(p: *const u8, h: u32, table_base: *mut c_void, _tt: u32) {
    let t = table_base as *mut *const u8;
    *t.add(h as usize) = p;
}

#[inline(always)]
unsafe fn LZ4_putPosition(p: *const u8, table_base: *mut c_void, table_type: u32) {
    let h = LZ4_hashPosition(p, table_type);
    LZ4_putPositionOnHash(p, h, table_base, table_type);
}

#[inline(always)]
unsafe fn LZ4_getIndexOnHash(h: u32, table_base: *const c_void, table_type: u32) -> u32 {
    if table_type == BY_U32 {
        let t = table_base as *const u32;
        return *t.add(h as usize);
    }
    if table_type == BY_U16 {
        let t = table_base as *const u16;
        return *t.add(h as usize) as u32;
    }
    0
}

#[inline(always)]
unsafe fn LZ4_getPositionOnHash(h: u32, table_base: *const c_void, _tt: u32) -> *const u8 {
    let t = table_base as *const *const u8;
    *t.add(h as usize)
}

#[inline(always)]
unsafe fn LZ4_getPosition(p: *const u8, table_base: *const c_void, table_type: u32) -> *const u8 {
    let h = LZ4_hashPosition(p, table_type);
    LZ4_getPositionOnHash(h, table_base, table_type)
}

pub unsafe fn LZ4_prepareTable(
    cctx: *mut LZ4_stream_t_internal,
    input_size: c_int,
    table_type: u32,
) {
    if (*cctx).tableType != CLEARED_TABLE {
        if (*cctx).tableType != table_type
            || (table_type == BY_U16
                && (*cctx).currentOffset.wrapping_add(input_size as u32) >= 0xFFFFu32)
            || (table_type == BY_U32 && (*cctx).currentOffset > (1u32 << 30))
            || table_type == BY_PTR
            || input_size >= 4 * 1024
        {
            memset_n((*cctx).hashTable.as_mut_ptr() as *mut u8, 0, LZ4_HASHTABLESIZE);
            (*cctx).currentOffset = 0;
            (*cctx).tableType = CLEARED_TABLE;
        }
    }

    if (*cctx).currentOffset != 0 && table_type == BY_U32 {
        (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(64 * 1024);
    }

    (*cctx).dictCtx = ptr::null();
    (*cctx).dictionary = ptr::null();
    (*cctx).dictSize = 0;
}

/* ---------------- compression core ---------------- */

unsafe fn LZ4_compress_generic_validated(
    cctx: *mut LZ4_stream_t_internal,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    input_consumed: *mut c_int,
    max_output_size: c_int,
    output_directive: c_int,
    table_type: u32,
    dict_directive: c_int,
    dict_issue: c_int,
    acceleration: c_int,
) -> c_int {
    let result: c_int;
    let mut ip: *const u8 = source as *const u8;

    let start_index: u32 = (*cctx).currentOffset;
    let base: *const u8 = (source as *const u8).wrapping_sub(start_index as usize);
    let mut low_limit: *const u8;

    let dict_ctx: *const LZ4_stream_t_internal = (*cctx).dictCtx;
    let dictionary: *const u8 = if dict_directive == USING_DICT_CTX {
        (*dict_ctx).dictionary
    } else {
        (*cctx).dictionary
    };
    let dict_size: u32 = if dict_directive == USING_DICT_CTX {
        (*dict_ctx).dictSize
    } else {
        (*cctx).dictSize
    };
    let dict_delta: u32 = if dict_directive == USING_DICT_CTX {
        start_index.wrapping_sub((*dict_ctx).currentOffset)
    } else {
        0
    };

    let maybe_ext_mem: bool =
        dict_directive == USING_EXT_DICT || dict_directive == USING_DICT_CTX;
    let prefix_idx_limit: u32 = start_index.wrapping_sub(dict_size);
    let dict_end: *const u8 = if !dictionary.is_null() {
        dictionary.wrapping_add(dict_size as usize)
    } else {
        dictionary
    };
    let mut anchor: *const u8 = source as *const u8;
    let iend: *const u8 = ip.wrapping_add(input_size as usize);
    let mflimit_plus_one: *const u8 = iend.wrapping_sub(MFLIMIT).wrapping_add(1);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);

    let dict_base: *const u8 = if dictionary.is_null() {
        ptr::null()
    } else if dict_directive == USING_DICT_CTX {
        dictionary
            .wrapping_add(dict_size as usize)
            .wrapping_sub((*dict_ctx).currentOffset as usize)
    } else {
        dictionary
            .wrapping_add(dict_size as usize)
            .wrapping_sub(start_index as usize)
    };

    let mut op: *mut u8 = dest as *mut u8;
    let olimit: *mut u8 = op.wrapping_add(max_output_size as usize);

    let mut offset: u32 = 0;
    let mut forward_h: u32;

    if output_directive == FILL_OUTPUT && max_output_size < 1 {
        return 0;
    }

    low_limit = (source as *const u8)
        .wrapping_sub(if dict_directive == WITH_PREFIX_64K { dict_size as usize } else { 0 });

    /* Update context state */
    if dict_directive == USING_DICT_CTX {
        (*cctx).dictCtx = ptr::null();
        (*cctx).dictSize = input_size as u32;
    } else {
        (*cctx).dictSize = (*cctx).dictSize.wrapping_add(input_size as u32);
    }
    (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(input_size as u32);
    (*cctx).tableType = table_type;

    let mut token: *mut u8 = ptr::null_mut();
    let mut m: *const u8;

    // "last literals" state machine emulation of C's gotos
    let mut goto_last_literals = false;

    if input_size < LZ4_MINLENGTH {
        goto_last_literals = true;
    }

    if !goto_last_literals {
        /* First Byte */
        {
            let h = LZ4_hashPosition(ip, table_type);
            if table_type == BY_PTR {
                LZ4_putPositionOnHash(
                    ip,
                    h,
                    (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                    BY_PTR,
                );
            } else {
                LZ4_putIndexOnHash(
                    start_index,
                    h,
                    (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                    table_type,
                );
            }
        }
        ip = ip.wrapping_add(1);
        forward_h = LZ4_hashPosition(ip, table_type);

        /* Main Loop */
        'main_loop: loop {
            let mut filled_ip: *const u8;
            let mut goto_next_match = false;

            /* Find a match */
            m = ptr::null();
            if table_type == BY_PTR {
                let mut forward_ip = ip;
                let mut step: isize = 1;
                let mut search_match_nb: i32 = acceleration << LZ4_SKIP_TRIGGER;
                loop {
                    let h = forward_h;
                    ip = forward_ip;
                    forward_ip = forward_ip.wrapping_offset(step);
                    step = (search_match_nb >> LZ4_SKIP_TRIGGER) as isize;
                    search_match_nb = search_match_nb.wrapping_add(1);

                    if forward_ip > mflimit_plus_one {
                        goto_last_literals = true;
                        break 'main_loop;
                    }

                    m = LZ4_getPositionOnHash(
                        h,
                        (*cctx).hashTable.as_ptr() as *const c_void,
                        table_type,
                    );
                    forward_h = LZ4_hashPosition(forward_ip, table_type);
                    LZ4_putPositionOnHash(
                        ip,
                        h,
                        (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                        table_type,
                    );

                    if !(m.wrapping_add(LZ4_DISTANCE_MAX as usize) < ip
                        || read32(m) != read32(ip))
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
                    let current: u32 = pdiff(forward_ip, base) as u32;
                    let mut match_index: u32 = LZ4_getIndexOnHash(
                        h,
                        (*cctx).hashTable.as_ptr() as *const c_void,
                        table_type,
                    );
                    ip = forward_ip;
                    forward_ip = forward_ip.wrapping_offset(step);
                    step = (search_match_nb >> LZ4_SKIP_TRIGGER) as isize;
                    search_match_nb = search_match_nb.wrapping_add(1);

                    if forward_ip > mflimit_plus_one {
                        goto_last_literals = true;
                        break 'main_loop;
                    }

                    if dict_directive == USING_DICT_CTX {
                        if match_index < start_index {
                            match_index = LZ4_getIndexOnHash(
                                h,
                                (*dict_ctx).hashTable.as_ptr() as *const c_void,
                                BY_U32,
                            );
                            m = dict_base.wrapping_add(match_index as usize);
                            match_index = match_index.wrapping_add(dict_delta);
                            low_limit = dictionary;
                        } else {
                            m = base.wrapping_add(match_index as usize);
                            low_limit = source as *const u8;
                        }
                    } else if dict_directive == USING_EXT_DICT {
                        if match_index < start_index {
                            m = dict_base.wrapping_add(match_index as usize);
                            low_limit = dictionary;
                        } else {
                            m = base.wrapping_add(match_index as usize);
                            low_limit = source as *const u8;
                        }
                    } else {
                        m = base.wrapping_add(match_index as usize);
                    }
                    forward_h = LZ4_hashPosition(forward_ip, table_type);
                    LZ4_putIndexOnHash(
                        current,
                        h,
                        (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                        table_type,
                    );

                    if dict_issue == DICT_SMALL && match_index < prefix_idx_limit {
                        continue;
                    }
                    if table_type != BY_U16
                        && match_index.wrapping_add(LZ4_DISTANCE_MAX) < current
                    {
                        continue;
                    }

                    if read32(m) == read32(ip) {
                        if maybe_ext_mem {
                            offset = current.wrapping_sub(match_index);
                        }
                        break;
                    }
                }
            }

            /* Catch up */
            filled_ip = ip;
            if m > low_limit && *ip.wrapping_sub(1) == *m.wrapping_sub(1) {
                loop {
                    ip = ip.wrapping_sub(1);
                    m = m.wrapping_sub(1);
                    if !(((ip > anchor) & (m > low_limit))
                        && *ip.wrapping_sub(1) == *m.wrapping_sub(1))
                    {
                        break;
                    }
                }
            }

            /* Encode Literals */
            {
                let lit_length: u32 = pdiff(ip, anchor) as u32;
                token = op;
                op = op.wrapping_add(1);
                if output_directive == LIMITED_OUTPUT
                    && op
                        .wrapping_add(lit_length as usize)
                        .wrapping_add(2 + 1 + LASTLITERALS)
                        .wrapping_add((lit_length / 255) as usize)
                        > olimit
                {
                    return 0;
                }
                if output_directive == FILL_OUTPUT
                    && op
                        .wrapping_add(((lit_length + 240) / 255) as usize)
                        .wrapping_add(lit_length as usize)
                        .wrapping_add(2)
                        .wrapping_add(1)
                        .wrapping_add(MFLIMIT - MINMATCH)
                        > olimit
                {
                    op = op.wrapping_sub(1);
                    goto_last_literals = true;
                    break 'main_loop;
                }
                if lit_length >= RUN_MASK {
                    let mut len = lit_length - RUN_MASK;
                    *token = (RUN_MASK << ML_BITS) as u8;
                    while len >= 255 {
                        *op = 255;
                        op = op.wrapping_add(1);
                        len -= 255;
                    }
                    *op = len as u8;
                    op = op.wrapping_add(1);
                } else {
                    *token = ((lit_length << ML_BITS) as u8) as u8;
                }

                /* Copy Literals */
                LZ4_wildCopy8(op, anchor, op.wrapping_add(lit_length as usize));
                op = op.wrapping_add(lit_length as usize);
            }

            // _next_match:
            loop {
                if output_directive == FILL_OUTPUT
                    && op
                        .wrapping_add(2)
                        .wrapping_add(1)
                        .wrapping_add(MFLIMIT - MINMATCH)
                        > olimit
                {
                    op = token;
                    goto_last_literals = true;
                    break 'main_loop;
                }

                /* Encode Offset */
                if maybe_ext_mem {
                    writeLE16(op, offset as u16);
                    op = op.wrapping_add(2);
                } else {
                    writeLE16(op, pdiff(ip, m) as u16);
                    op = op.wrapping_add(2);
                }

                /* Encode MatchLength */
                {
                    let mut match_code: u32;

                    if (dict_directive == USING_EXT_DICT || dict_directive == USING_DICT_CTX)
                        && low_limit == dictionary
                    {
                        let mut limit = ip.wrapping_offset(pdiff(dict_end, m));
                        if limit > matchlimit {
                            limit = matchlimit;
                        }
                        match_code = LZ4_count(
                            ip.wrapping_add(MINMATCH),
                            m.wrapping_add(MINMATCH),
                            limit,
                        );
                        ip = ip.wrapping_add(match_code as usize + MINMATCH);
                        if ip == limit {
                            let more = LZ4_count(limit, source as *const u8, matchlimit);
                            match_code = match_code.wrapping_add(more);
                            ip = ip.wrapping_add(more as usize);
                        }
                    } else {
                        match_code = LZ4_count(
                            ip.wrapping_add(MINMATCH),
                            m.wrapping_add(MINMATCH),
                            matchlimit,
                        );
                        ip = ip.wrapping_add(match_code as usize + MINMATCH);
                    }

                    if output_directive != NOT_LIMITED
                        && op
                            .wrapping_add(1 + LASTLITERALS)
                            .wrapping_add(((match_code + 240) / 255) as usize)
                            > olimit
                    {
                        if output_directive == FILL_OUTPUT {
                            let new_match_code: u32 = 15u32
                                .wrapping_sub(1)
                                .wrapping_add(
                                    ((pdiff(olimit, op) as u32)
                                        .wrapping_sub(1)
                                        .wrapping_sub(LASTLITERALS as u32))
                                        .wrapping_mul(255),
                                );
                            ip = ip.wrapping_sub((match_code - new_match_code) as usize);
                            match_code = new_match_code;
                            if ip <= filled_ip {
                                let mut p = ip;
                                while p <= filled_ip {
                                    let h = LZ4_hashPosition(p, table_type);
                                    LZ4_clearHash(
                                        h,
                                        (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                                        table_type,
                                    );
                                    p = p.wrapping_add(1);
                                }
                            }
                        } else {
                            return 0;
                        }
                    }
                    if match_code >= ML_MASK {
                        *token = (*token).wrapping_add(ML_MASK as u8);
                        match_code -= ML_MASK;
                        write32(op, 0xFFFFFFFF);
                        while match_code >= 4 * 255 {
                            op = op.wrapping_add(4);
                            write32(op, 0xFFFFFFFF);
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
                    let h = LZ4_hashPosition(ip.wrapping_sub(2), table_type);
                    if table_type == BY_PTR {
                        LZ4_putPositionOnHash(
                            ip.wrapping_sub(2),
                            h,
                            (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                            BY_PTR,
                        );
                    } else {
                        let idx = pdiff(ip.wrapping_sub(2), base) as u32;
                        LZ4_putIndexOnHash(
                            idx,
                            h,
                            (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                            table_type,
                        );
                    }
                }

                /* Test next position */
                if table_type == BY_PTR {
                    m = LZ4_getPosition(ip, (*cctx).hashTable.as_ptr() as *const c_void, table_type);
                    LZ4_putPosition(ip, (*cctx).hashTable.as_mut_ptr() as *mut c_void, table_type);
                    if m.wrapping_add(LZ4_DISTANCE_MAX as usize) >= ip && read32(m) == read32(ip) {
                        token = op;
                        op = op.wrapping_add(1);
                        *token = 0;
                        goto_next_match = true;
                    }
                } else {
                    let h = LZ4_hashPosition(ip, table_type);
                    let current: u32 = pdiff(ip, base) as u32;
                    let mut match_index: u32 = LZ4_getIndexOnHash(
                        h,
                        (*cctx).hashTable.as_ptr() as *const c_void,
                        table_type,
                    );
                    if dict_directive == USING_DICT_CTX {
                        if match_index < start_index {
                            match_index = LZ4_getIndexOnHash(
                                h,
                                (*dict_ctx).hashTable.as_ptr() as *const c_void,
                                BY_U32,
                            );
                            m = dict_base.wrapping_add(match_index as usize);
                            low_limit = dictionary;
                            match_index = match_index.wrapping_add(dict_delta);
                        } else {
                            m = base.wrapping_add(match_index as usize);
                            low_limit = source as *const u8;
                        }
                    } else if dict_directive == USING_EXT_DICT {
                        if match_index < start_index {
                            m = dict_base.wrapping_add(match_index as usize);
                            low_limit = dictionary;
                        } else {
                            m = base.wrapping_add(match_index as usize);
                            low_limit = source as *const u8;
                        }
                    } else {
                        m = base.wrapping_add(match_index as usize);
                    }
                    LZ4_putIndexOnHash(
                        current,
                        h,
                        (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                        table_type,
                    );
                    let cond_a = if dict_issue == DICT_SMALL {
                        match_index >= prefix_idx_limit
                    } else {
                        true
                    };
                    let cond_b = if table_type == BY_U16 {
                        true
                    } else {
                        match_index.wrapping_add(LZ4_DISTANCE_MAX) >= current
                    };
                    if cond_a && cond_b && read32(m) == read32(ip) {
                        token = op;
                        op = op.wrapping_add(1);
                        *token = 0;
                        if maybe_ext_mem {
                            offset = current.wrapping_sub(match_index);
                        }
                        goto_next_match = true;
                    }
                }

                if goto_next_match {
                    // loop back to _next_match
                    goto_next_match = false;
                    continue;
                }

                /* Prepare next loop */
                ip = ip.wrapping_add(1);
                forward_h = LZ4_hashPosition(ip, table_type);
                break;
            }
        }
    }

    // _last_literals:
    {
        let mut last_run: usize = pdiff(iend, anchor) as usize;
        if output_directive != NOT_LIMITED
            && op
                .wrapping_add(last_run)
                .wrapping_add(1)
                .wrapping_add((last_run + 255 - RUN_MASK as usize) / 255)
                > olimit
        {
            if output_directive == FILL_OUTPUT {
                last_run = (pdiff(olimit, op) as usize).wrapping_sub(1);
                last_run = last_run.wrapping_sub((last_run + 256 - RUN_MASK as usize) / 256);
            } else {
                return 0;
            }
        }
        if last_run >= RUN_MASK as usize {
            let mut accumulator = last_run - RUN_MASK as usize;
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
            *op = ((last_run as u32) << ML_BITS) as u8;
            op = op.wrapping_add(1);
        }
        memcpy_n(op, anchor, last_run);
        ip = anchor.wrapping_add(last_run);
        op = op.wrapping_add(last_run);
    }

    if output_directive == FILL_OUTPUT {
        *input_consumed = pdiff(ip, source as *const u8) as c_int;
    }
    result = pdiff(op as *const u8, dest as *const u8) as c_int;
    result
}

unsafe fn LZ4_compress_generic(
    cctx: *mut LZ4_stream_t_internal,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    input_consumed: *mut c_int,
    dst_capacity: c_int,
    output_directive: c_int,
    table_type: u32,
    dict_directive: c_int,
    dict_issue: c_int,
    acceleration: c_int,
) -> c_int {
    if (src_size as u32) > LZ4_MAX_INPUT_SIZE {
        return 0;
    }
    if src_size == 0 {
        if output_directive != NOT_LIMITED && dst_capacity <= 0 {
            return 0;
        }
        *dst = 0;
        if output_directive == FILL_OUTPUT {
            *input_consumed = 0;
        }
        return 1;
    }

    LZ4_compress_generic_validated(
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

/* ---------------- public simple API ---------------- */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_versionNumber() -> c_int {
    LZ4_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_versionString() -> *const c_char {
    LZ4_VERSION_STRING.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_compressBound(isize_: c_int) -> c_int {
    LZ4_COMPRESSBOUND(isize_)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofState() -> c_int {
    core::mem::size_of::<LZ4_stream_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_extState(
    state: *mut c_void,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    max_output_size: c_int,
    mut acceleration: c_int,
) -> c_int {
    let s = LZ4_initStream(state, core::mem::size_of::<LZ4_stream_t>());
    let ctx = s as *mut LZ4_stream_t_internal;
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }
    if max_output_size >= LZ4_compressBound(input_size) {
        if input_size < LZ4_64KLIMIT {
            LZ4_compress_generic(
                ctx, source, dest, input_size, ptr::null_mut(), 0, NOT_LIMITED, BY_U16, NO_DICT,
                NO_DICT_ISSUE, acceleration,
            )
        } else {
            LZ4_compress_generic(
                ctx, source, dest, input_size, ptr::null_mut(), 0, NOT_LIMITED, BY_U32, NO_DICT,
                NO_DICT_ISSUE, acceleration,
            )
        }
    } else {
        if input_size < LZ4_64KLIMIT {
            LZ4_compress_generic(
                ctx,
                source,
                dest,
                input_size,
                ptr::null_mut(),
                max_output_size,
                LIMITED_OUTPUT,
                BY_U16,
                NO_DICT,
                NO_DICT_ISSUE,
                acceleration,
            )
        } else {
            LZ4_compress_generic(
                ctx,
                source,
                dest,
                input_size,
                ptr::null_mut(),
                max_output_size,
                LIMITED_OUTPUT,
                BY_U32,
                NO_DICT,
                NO_DICT_ISSUE,
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
    mut acceleration: c_int,
) -> c_int {
    let ctx = state as *mut LZ4_stream_t_internal;
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }

    if dst_capacity >= LZ4_compressBound(src_size) {
        if src_size < LZ4_64KLIMIT {
            let table_type = BY_U16;
            LZ4_prepareTable(ctx, src_size, table_type);
            if (*ctx).currentOffset != 0 {
                LZ4_compress_generic(
                    ctx, src, dst, src_size, ptr::null_mut(), 0, NOT_LIMITED, table_type, NO_DICT,
                    DICT_SMALL, acceleration,
                )
            } else {
                LZ4_compress_generic(
                    ctx, src, dst, src_size, ptr::null_mut(), 0, NOT_LIMITED, table_type, NO_DICT,
                    NO_DICT_ISSUE, acceleration,
                )
            }
        } else {
            let table_type = BY_U32;
            LZ4_prepareTable(ctx, src_size, table_type);
            LZ4_compress_generic(
                ctx, src, dst, src_size, ptr::null_mut(), 0, NOT_LIMITED, table_type, NO_DICT,
                NO_DICT_ISSUE, acceleration,
            )
        }
    } else {
        if src_size < LZ4_64KLIMIT {
            let table_type = BY_U16;
            LZ4_prepareTable(ctx, src_size, table_type);
            if (*ctx).currentOffset != 0 {
                LZ4_compress_generic(
                    ctx,
                    src,
                    dst,
                    src_size,
                    ptr::null_mut(),
                    dst_capacity,
                    LIMITED_OUTPUT,
                    table_type,
                    NO_DICT,
                    DICT_SMALL,
                    acceleration,
                )
            } else {
                LZ4_compress_generic(
                    ctx,
                    src,
                    dst,
                    src_size,
                    ptr::null_mut(),
                    dst_capacity,
                    LIMITED_OUTPUT,
                    table_type,
                    NO_DICT,
                    NO_DICT_ISSUE,
                    acceleration,
                )
            }
        } else {
            let table_type = BY_U32;
            LZ4_prepareTable(ctx, src_size, table_type);
            LZ4_compress_generic(
                ctx,
                src,
                dst,
                src_size,
                ptr::null_mut(),
                dst_capacity,
                LIMITED_OUTPUT,
                table_type,
                NO_DICT,
                NO_DICT_ISSUE,
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
    let mut ctx: LZ4_stream_t = core::mem::zeroed();
    LZ4_compress_fast_extState(
        &mut ctx as *mut LZ4_stream_t as *mut c_void,
        src,
        dest,
        src_size,
        dst_capacity,
        acceleration,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_default(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    LZ4_compress_fast(src, dst, src_size, dst_capacity, 1)
}

unsafe fn LZ4_compress_destSize_extState_internal(
    state: *mut LZ4_stream_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dst_size: c_int,
    acceleration: c_int,
) -> c_int {
    let _s = LZ4_initStream(state as *mut c_void, core::mem::size_of::<LZ4_stream_t>());

    if target_dst_size >= LZ4_compressBound(*src_size_ptr) {
        LZ4_compress_fast_extState(
            state as *mut c_void,
            src,
            dst,
            *src_size_ptr,
            target_dst_size,
            acceleration,
        )
    } else {
        if *src_size_ptr < LZ4_64KLIMIT {
            LZ4_compress_generic(
                state as *mut LZ4_stream_t_internal,
                src,
                dst,
                *src_size_ptr,
                src_size_ptr,
                target_dst_size,
                FILL_OUTPUT,
                BY_U16,
                NO_DICT,
                NO_DICT_ISSUE,
                acceleration,
            )
        } else {
            LZ4_compress_generic(
                state as *mut LZ4_stream_t_internal,
                src,
                dst,
                *src_size_ptr,
                src_size_ptr,
                target_dst_size,
                FILL_OUTPUT,
                BY_U32,
                NO_DICT,
                NO_DICT_ISSUE,
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
    let r = LZ4_compress_destSize_extState_internal(
        state as *mut LZ4_stream_t,
        src,
        dst,
        src_size_ptr,
        target_dst_size,
        acceleration,
    );
    LZ4_initStream(state, core::mem::size_of::<LZ4_stream_t>());
    r
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_destSize(
    src: *const c_char,
    dst: *mut c_char,
    src_size_ptr: *mut c_int,
    target_dst_size: c_int,
) -> c_int {
    let mut ctx_body: LZ4_stream_t = core::mem::zeroed();
    LZ4_compress_destSize_extState_internal(
        &mut ctx_body as *mut LZ4_stream_t,
        src,
        dst,
        src_size_ptr,
        target_dst_size,
        1,
    )
}

/* ---------------- streaming compression ---------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStream() -> *mut LZ4_stream_t {
    let lz4s = malloc(core::mem::size_of::<LZ4_stream_t>()) as *mut LZ4_stream_t;
    if lz4s.is_null() {
        return ptr::null_mut();
    }
    LZ4_initStream(lz4s as *mut c_void, core::mem::size_of::<LZ4_stream_t>());
    lz4s
}

fn LZ4_stream_t_alignment() -> usize {
    core::mem::align_of::<LZ4_stream_t>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStream(buffer: *mut c_void, size: usize) -> *mut LZ4_stream_t {
    if buffer.is_null() {
        return ptr::null_mut();
    }
    if size < core::mem::size_of::<LZ4_stream_t>() {
        return ptr::null_mut();
    }
    if !LZ4_isAligned(buffer, LZ4_stream_t_alignment()) {
        return ptr::null_mut();
    }
    memset_n(
        buffer as *mut u8,
        0,
        core::mem::size_of::<LZ4_stream_t_internal>(),
    );
    buffer as *mut LZ4_stream_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream(lz4_stream: *mut LZ4_stream_t) {
    memset_n(
        lz4_stream as *mut u8,
        0,
        core::mem::size_of::<LZ4_stream_t_internal>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream_fast(ctx: *mut LZ4_stream_t) {
    LZ4_prepareTable(ctx as *mut LZ4_stream_t_internal, 0, BY_U32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStream(lz4_stream: *mut LZ4_stream_t) -> c_int {
    if lz4_stream.is_null() {
        return 0;
    }
    free(lz4_stream as *mut c_void);
    0
}

pub const LD_FAST: c_int = 0;
pub const LD_SLOW: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict_internal(
    lz4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dict_size: c_int,
    ld: c_int,
) -> c_int {
    let dict = lz4_dict as *mut LZ4_stream_t_internal;
    let table_type = BY_U32;
    let mut p: *const u8 = dictionary as *const u8;
    let dict_end: *const u8 = p.wrapping_add(dict_size as usize);
    let mut idx32: u32;

    LZ4_resetStream(lz4_dict);

    (*dict).currentOffset = (*dict).currentOffset.wrapping_add(64 * 1024);

    if dict_size < HASH_UNIT as c_int {
        return 0;
    }

    if pdiff(dict_end, p) > (64 * 1024) {
        p = dict_end.wrapping_sub(64 * 1024);
    }
    (*dict).dictionary = p;
    (*dict).dictSize = pdiff(dict_end, p) as u32;
    (*dict).tableType = table_type;
    idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);

    while p <= dict_end.wrapping_sub(HASH_UNIT) {
        let h = LZ4_hashPosition(p, table_type);
        LZ4_putIndexOnHash(
            idx32,
            h,
            (*dict).hashTable.as_mut_ptr() as *mut c_void,
            table_type,
        );
        p = p.wrapping_add(3);
        idx32 = idx32.wrapping_add(3);
    }

    if ld == LD_SLOW {
        p = (*dict).dictionary;
        idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);
        while p <= dict_end.wrapping_sub(HASH_UNIT) {
            let h = LZ4_hashPosition(p, table_type);
            let limit = (*dict).currentOffset.wrapping_sub(64 * 1024);
            if LZ4_getIndexOnHash(h, (*dict).hashTable.as_ptr() as *const c_void, table_type)
                <= limit
            {
                LZ4_putIndexOnHash(
                    idx32,
                    h,
                    (*dict).hashTable.as_mut_ptr() as *mut c_void,
                    table_type,
                );
            }
            p = p.wrapping_add(1);
            idx32 = idx32.wrapping_add(1);
        }
    }

    (*dict).dictSize as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict(
    lz4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    LZ4_loadDict_internal(lz4_dict, dictionary, dict_size, LD_FAST)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictSlow(
    lz4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    LZ4_loadDict_internal(lz4_dict, dictionary, dict_size, LD_SLOW)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_dictionary(
    working_stream: *mut LZ4_stream_t,
    dictionary_stream: *const LZ4_stream_t,
) {
    let mut dict_ctx: *const LZ4_stream_t_internal = if dictionary_stream.is_null() {
        ptr::null()
    } else {
        dictionary_stream as *const LZ4_stream_t_internal
    };

    if !dict_ctx.is_null() {
        let ws = working_stream as *mut LZ4_stream_t_internal;
        if (*ws).currentOffset == 0 {
            (*ws).currentOffset = 64 * 1024;
        }
        if (*dict_ctx).dictSize == 0 {
            dict_ctx = ptr::null();
        }
    }
    (*(working_stream as *mut LZ4_stream_t_internal)).dictCtx = dict_ctx;
}

unsafe fn LZ4_renormDictT(lz4_dict: *mut LZ4_stream_t_internal, next_size: c_int) {
    if (*lz4_dict).currentOffset.wrapping_add(next_size as u32) > 0x80000000u32 {
        let delta = (*lz4_dict).currentOffset.wrapping_sub(64 * 1024);
        let dict_end = (*lz4_dict).dictionary.wrapping_add((*lz4_dict).dictSize as usize);
        for i in 0..LZ4_HASH_SIZE_U32 {
            if (*lz4_dict).hashTable[i] < delta {
                (*lz4_dict).hashTable[i] = 0;
            } else {
                (*lz4_dict).hashTable[i] -= delta;
            }
        }
        (*lz4_dict).currentOffset = 64 * 1024;
        if (*lz4_dict).dictSize > 64 * 1024 {
            (*lz4_dict).dictSize = 64 * 1024;
        }
        (*lz4_dict).dictionary = dict_end.wrapping_sub((*lz4_dict).dictSize as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_continue(
    lz4_stream: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    max_output_size: c_int,
    mut acceleration: c_int,
) -> c_int {
    let table_type = BY_U32;
    let stream_ptr = lz4_stream as *mut LZ4_stream_t_internal;
    let mut dict_end: *const c_char = if (*stream_ptr).dictSize != 0 {
        ((*stream_ptr).dictionary as *const c_char)
            .wrapping_add((*stream_ptr).dictSize as usize)
    } else {
        ptr::null()
    };

    LZ4_renormDictT(stream_ptr, input_size);
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }

    /* invalidate tiny dictionaries */
    if (*stream_ptr).dictSize < 4
        && dict_end != source
        && input_size > 0
        && (*stream_ptr).dictCtx.is_null()
    {
        (*stream_ptr).dictSize = 0;
        (*stream_ptr).dictionary = source as *const u8;
        dict_end = source;
    }

    /* Check overlapping input/dictionary space */
    {
        let source_end = source.wrapping_add(input_size as usize);
        if source_end > (*stream_ptr).dictionary as *const c_char && source_end < dict_end {
            (*stream_ptr).dictSize = pdiff(dict_end as *const u8, source_end as *const u8) as u32;
            if (*stream_ptr).dictSize > 64 * 1024 {
                (*stream_ptr).dictSize = 64 * 1024;
            }
            if (*stream_ptr).dictSize < 4 {
                (*stream_ptr).dictSize = 0;
            }
            (*stream_ptr).dictionary =
                (dict_end as *const u8).wrapping_sub((*stream_ptr).dictSize as usize);
        }
    }

    /* prefix mode */
    if dict_end == source {
        if (*stream_ptr).dictSize < 64 * 1024 && (*stream_ptr).dictSize < (*stream_ptr).currentOffset
        {
            return LZ4_compress_generic(
                stream_ptr,
                source,
                dest,
                input_size,
                ptr::null_mut(),
                max_output_size,
                LIMITED_OUTPUT,
                table_type,
                WITH_PREFIX_64K,
                DICT_SMALL,
                acceleration,
            );
        } else {
            return LZ4_compress_generic(
                stream_ptr,
                source,
                dest,
                input_size,
                ptr::null_mut(),
                max_output_size,
                LIMITED_OUTPUT,
                table_type,
                WITH_PREFIX_64K,
                NO_DICT_ISSUE,
                acceleration,
            );
        }
    }

    /* external dictionary mode */
    {
        let result: c_int;
        if !(*stream_ptr).dictCtx.is_null() {
            if input_size > 4 * 1024 {
                ptr::copy_nonoverlapping(
                    (*stream_ptr).dictCtx as *const u8,
                    stream_ptr as *mut u8,
                    core::mem::size_of::<LZ4_stream_t_internal>(),
                );
                result = LZ4_compress_generic(
                    stream_ptr,
                    source,
                    dest,
                    input_size,
                    ptr::null_mut(),
                    max_output_size,
                    LIMITED_OUTPUT,
                    table_type,
                    USING_EXT_DICT,
                    NO_DICT_ISSUE,
                    acceleration,
                );
            } else {
                result = LZ4_compress_generic(
                    stream_ptr,
                    source,
                    dest,
                    input_size,
                    ptr::null_mut(),
                    max_output_size,
                    LIMITED_OUTPUT,
                    table_type,
                    USING_DICT_CTX,
                    NO_DICT_ISSUE,
                    acceleration,
                );
            }
        } else {
            if (*stream_ptr).dictSize < 64 * 1024
                && (*stream_ptr).dictSize < (*stream_ptr).currentOffset
            {
                result = LZ4_compress_generic(
                    stream_ptr,
                    source,
                    dest,
                    input_size,
                    ptr::null_mut(),
                    max_output_size,
                    LIMITED_OUTPUT,
                    table_type,
                    USING_EXT_DICT,
                    DICT_SMALL,
                    acceleration,
                );
            } else {
                result = LZ4_compress_generic(
                    stream_ptr,
                    source,
                    dest,
                    input_size,
                    ptr::null_mut(),
                    max_output_size,
                    LIMITED_OUTPUT,
                    table_type,
                    USING_EXT_DICT,
                    NO_DICT_ISSUE,
                    acceleration,
                );
            }
        }
        (*stream_ptr).dictionary = source as *const u8;
        (*stream_ptr).dictSize = input_size as u32;
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_forceExtDict(
    lz4_dict: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    src_size: c_int,
) -> c_int {
    let stream_ptr = lz4_dict as *mut LZ4_stream_t_internal;
    let result: c_int;

    LZ4_renormDictT(stream_ptr, src_size);

    if (*stream_ptr).dictSize < 64 * 1024 && (*stream_ptr).dictSize < (*stream_ptr).currentOffset {
        result = LZ4_compress_generic(
            stream_ptr,
            source,
            dest,
            src_size,
            ptr::null_mut(),
            0,
            NOT_LIMITED,
            BY_U32,
            USING_EXT_DICT,
            DICT_SMALL,
            1,
        );
    } else {
        result = LZ4_compress_generic(
            stream_ptr,
            source,
            dest,
            src_size,
            ptr::null_mut(),
            0,
            NOT_LIMITED,
            BY_U32,
            USING_EXT_DICT,
            NO_DICT_ISSUE,
            1,
        );
    }

    (*stream_ptr).dictionary = source as *const u8;
    (*stream_ptr).dictSize = src_size as u32;

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDict(
    lz4_dict: *mut LZ4_stream_t,
    safe_buffer: *mut c_char,
    mut dict_size: c_int,
) -> c_int {
    let dict = lz4_dict as *mut LZ4_stream_t_internal;

    if (dict_size as u32) > 64 * 1024 {
        dict_size = 64 * 1024;
    }
    if (dict_size as u32) > (*dict).dictSize {
        dict_size = (*dict).dictSize as c_int;
    }

    if dict_size > 0 {
        let previous_dict_end = (*dict).dictionary.wrapping_add((*dict).dictSize as usize);
        memmove_n(
            safe_buffer as *mut u8,
            previous_dict_end.wrapping_sub(dict_size as usize),
            dict_size as usize,
        );
    }

    (*dict).dictionary = safe_buffer as *const u8;
    (*dict).dictSize = dict_size as u32;

    dict_size
}

/* ---------------- decompression ---------------- */

unsafe fn read_long_length_no_check(pp: &mut *const u8) -> usize {
    let mut b: usize;
    let mut l: usize = 0;
    loop {
        b = **pp as usize;
        *pp = (*pp).wrapping_add(1);
        l = l.wrapping_add(b);
        if b != 255 {
            break;
        }
    }
    l
}

unsafe fn LZ4_decompress_unsafe_generic(
    istart: *const u8,
    ostart: *mut u8,
    decompressed_size: c_int,
    prefix_size: usize,
    dict_start: *const u8,
    dict_size: usize,
) -> c_int {
    let mut ip: *const u8 = istart;
    let mut op: *mut u8 = ostart;
    let oend: *mut u8 = ostart.wrapping_add(decompressed_size as usize);
    let prefix_start: *const u8 = ostart.wrapping_sub(prefix_size);

    loop {
        let token = *ip as u32;
        ip = ip.wrapping_add(1);

        /* literals */
        {
            let mut ll: usize = (token >> ML_BITS) as usize;
            if ll == 15 {
                ll = ll.wrapping_add(read_long_length_no_check(&mut ip));
            }
            if (pdiff(oend, op) as usize) < ll {
                return -1;
            }
            memmove_n(op, ip, ll);
            op = op.wrapping_add(ll);
            ip = ip.wrapping_add(ll);
            if (pdiff(oend, op) as usize) < MFLIMIT {
                if op == oend {
                    break;
                }
                return -1;
            }
        }

        /* match */
        {
            let mut ml: usize = (token & 15) as usize;
            let offset: usize = readLE16(ip) as usize;
            ip = ip.wrapping_add(2);

            if ml == 15 {
                ml = ml.wrapping_add(read_long_length_no_check(&mut ip));
            }
            ml += MINMATCH;

            if (pdiff(oend, op) as usize) < ml {
                return -1;
            }

            {
                let mut m: *const u8 = op.wrapping_sub(offset);

                if offset > (pdiff(op, prefix_start) as usize) + dict_size {
                    return -1;
                }

                if offset > pdiff(op, prefix_start) as usize {
                    let dict_end = dict_start.wrapping_add(dict_size);
                    let ext_match = dict_end
                        .wrapping_sub(offset - (pdiff(op, prefix_start) as usize));
                    let extml = pdiff(dict_end, ext_match) as usize;
                    if extml > ml {
                        memmove_n(op, ext_match, ml);
                        op = op.wrapping_add(ml);
                        ml = 0;
                    } else {
                        memmove_n(op, ext_match, extml);
                        op = op.wrapping_add(extml);
                        ml -= extml;
                    }
                    m = prefix_start;
                }

                {
                    let mut u = 0usize;
                    while u < ml {
                        *op.wrapping_add(u) = *m.wrapping_add(u);
                        u += 1;
                    }
                }
            }
            op = op.wrapping_add(ml);
            if (pdiff(oend, op) as usize) < LASTLITERALS {
                return -1;
            }
        }
    }
    pdiff(ip, istart) as c_int
}

const RVL_ERROR: usize = usize::MAX;

#[inline(always)]
unsafe fn read_variable_length(
    ip: &mut *const u8,
    ilimit: *const u8,
    initial_check: bool,
) -> usize {
    let mut s: usize;
    let mut length: usize = 0;
    if initial_check && *ip >= ilimit {
        return RVL_ERROR;
    }
    s = **ip as usize;
    *ip = (*ip).wrapping_add(1);
    length = length.wrapping_add(s);
    if *ip > ilimit {
        return RVL_ERROR;
    }
    if s != 255 {
        return length;
    }
    loop {
        s = **ip as usize;
        *ip = (*ip).wrapping_add(1);
        length = length.wrapping_add(s);
        if *ip > ilimit {
            return RVL_ERROR;
        }
        if s != 255 {
            break;
        }
    }
    length
}

#[derive(Clone, Copy, PartialEq)]
enum Entry {
    Top,
    SafeLiteralCopy,
    CopyMatch,
    SafeMatchCopy,
}

unsafe fn LZ4_decompress_generic(
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    output_size: c_int,
    partial_decoding: c_int,
    dict: c_int,
    low_prefix: *const u8,
    dict_start: *const u8,
    dict_size: usize,
) -> c_int {
    if src.is_null() || output_size < 0 {
        return -1;
    }

    let mut ip: *const u8 = src as *const u8;
    let iend: *const u8 = ip.wrapping_add(src_size as usize);

    let mut op: *mut u8 = dst as *mut u8;
    let oend: *mut u8 = op.wrapping_add(output_size as usize);
    let mut cpy: *mut u8;

    let dict_end: *const u8 = if dict_start.is_null() {
        ptr::null()
    } else {
        dict_start.wrapping_add(dict_size)
    };

    let check_offset: bool = dict_size < (64 * 1024);

    let shortiend: *const u8 = iend.wrapping_sub(14).wrapping_sub(2);
    let shortoend: *mut u8 = oend.wrapping_sub(14).wrapping_sub(18);

    let mut m: *const u8 = ptr::null();
    let mut offset: usize = 0;
    let mut token: u32 = 0;
    let mut length: usize = 0;

    macro_rules! err {
        () => {
            return (-(pdiff(ip, src as *const u8) as i64) - 1) as c_int
        };
    }

    /* Special cases */
    if output_size == 0 {
        if partial_decoding != 0 {
            return 0;
        }
        return if src_size == 1 && *ip == 0 { 0 } else { -1 };
    }
    if src_size == 0 {
        return -1;
    }

    let mut entry = Entry::Top;
    let mut in_fast_loop = pdiff(oend, op) >= FASTLOOP_SAFE_DISTANCE as isize;

    if in_fast_loop {
        'fast: loop {
            token = *ip as u32;
            ip = ip.wrapping_add(1);
            length = (token >> ML_BITS) as usize;

            /* decode literal length */
            if length == RUN_MASK as usize {
                let addl = read_variable_length(
                    &mut ip,
                    iend.wrapping_sub(RUN_MASK as usize),
                    true,
                );
                if addl == RVL_ERROR {
                    err!();
                }
                length = length.wrapping_add(addl);
                if (op as usize).wrapping_add(length) < (op as usize) {
                    err!();
                }
                if (ip as usize).wrapping_add(length) < (ip as usize) {
                    err!();
                }

                if op.wrapping_add(length) > oend.wrapping_sub(32)
                    || ip.wrapping_add(length) > iend.wrapping_sub(32)
                {
                    entry = Entry::SafeLiteralCopy;
                    break 'fast;
                }
                LZ4_wildCopy32(op, ip, op.wrapping_add(length));
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
            } else if ip <= iend.wrapping_sub(16 + 1) {
                memcpy_n(op, ip, 16);
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
            } else {
                entry = Entry::SafeLiteralCopy;
                break 'fast;
            }

            /* get offset */
            offset = readLE16(ip) as usize;
            ip = ip.wrapping_add(2);
            m = op.wrapping_sub(offset);

            /* get matchlength */
            length = (token & ML_MASK) as usize;

            if length == ML_MASK as usize {
                let addl = read_variable_length(
                    &mut ip,
                    iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                    false,
                );
                if addl == RVL_ERROR {
                    err!();
                }
                length = length.wrapping_add(addl);
                length = length.wrapping_add(MINMATCH);
                if (op as usize).wrapping_add(length) < (op as usize) {
                    err!();
                }
                if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                    entry = Entry::SafeMatchCopy;
                    break 'fast;
                }
            } else {
                length = length.wrapping_add(MINMATCH);
                if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                    entry = Entry::SafeMatchCopy;
                    break 'fast;
                }

                if dict == WITH_PREFIX_64K || m >= low_prefix {
                    if offset >= 8 {
                        memcpy_n(op, m, 8);
                        memcpy_n(op.wrapping_add(8), m.wrapping_add(8), 8);
                        memcpy_n(op.wrapping_add(16), m.wrapping_add(16), 2);
                        op = op.wrapping_add(length);
                        continue 'fast;
                    }
                }
            }

            if check_offset && m.wrapping_add(dict_size) < low_prefix {
                err!();
            }
            /* match starting within external dictionary */
            if dict == USING_EXT_DICT && m < low_prefix {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partial_decoding != 0 {
                        let avail = pdiff(oend, op) as usize;
                        if avail < length {
                            length = avail;
                        }
                    } else {
                        err!();
                    }
                }

                if length <= pdiff(low_prefix, m) as usize {
                    memmove_n(
                        op,
                        dict_end.wrapping_sub(pdiff(low_prefix, m) as usize),
                        length,
                    );
                    op = op.wrapping_add(length);
                } else {
                    let copy_size = pdiff(low_prefix, m) as usize;
                    let rest_size = length - copy_size;
                    memcpy_n(op, dict_end.wrapping_sub(copy_size), copy_size);
                    op = op.wrapping_add(copy_size);
                    if rest_size > (pdiff(op, low_prefix) as usize) {
                        let end_of_match = op.wrapping_add(rest_size);
                        let mut copy_from = low_prefix;
                        while op < end_of_match {
                            *op = *copy_from;
                            op = op.wrapping_add(1);
                            copy_from = copy_from.wrapping_add(1);
                        }
                    } else {
                        memcpy_n(op, low_prefix, rest_size);
                        op = op.wrapping_add(rest_size);
                    }
                }
                continue 'fast;
            }

            /* copy match within block */
            cpy = op.wrapping_add(length);

            if offset < 16 {
                LZ4_memcpy_using_offset(op, m, cpy, offset);
            } else {
                LZ4_wildCopy32(op, m, cpy);
            }

            op = cpy;
        }
    }

    /* Main safe loop */
    'safe: loop {
        if entry == Entry::Top {
            token = *ip as u32;
            ip = ip.wrapping_add(1);
            length = (token >> ML_BITS) as usize;

            let mut goto_copy_match = false;
            if length != RUN_MASK as usize && (ip < shortiend) & (op <= shortoend) {
                memcpy_n(op, ip, 16);
                op = op.wrapping_add(length);
                ip = ip.wrapping_add(length);

                length = (token & ML_MASK) as usize;
                offset = readLE16(ip) as usize;
                ip = ip.wrapping_add(2);
                m = op.wrapping_sub(offset);

                if length != ML_MASK as usize
                    && offset >= 8
                    && (dict == WITH_PREFIX_64K || m >= low_prefix)
                {
                    memcpy_n(op, m, 8);
                    memcpy_n(op.wrapping_add(8), m.wrapping_add(8), 8);
                    memcpy_n(op.wrapping_add(16), m.wrapping_add(16), 2);
                    op = op.wrapping_add(length + MINMATCH);
                    entry = Entry::Top;
                    continue 'safe;
                }

                goto_copy_match = true;
            }

            if goto_copy_match {
                entry = Entry::CopyMatch;
            } else {
                /* decode literal length */
                if length == RUN_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip,
                        iend.wrapping_sub(RUN_MASK as usize),
                        true,
                    );
                    if addl == RVL_ERROR {
                        err!();
                    }
                    length = length.wrapping_add(addl);
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        err!();
                    }
                    if (ip as usize).wrapping_add(length) < (ip as usize) {
                        err!();
                    }
                }
                entry = Entry::SafeLiteralCopy;
            }
        }

        if entry == Entry::SafeLiteralCopy {
            /* copy literals */
            cpy = op.wrapping_add(length);

            if cpy > oend.wrapping_sub(MFLIMIT)
                || ip.wrapping_add(length) > iend.wrapping_sub(2 + 1 + LASTLITERALS)
            {
                if partial_decoding != 0 {
                    if ip.wrapping_add(length) > iend {
                        length = pdiff(iend, ip) as usize;
                        cpy = op.wrapping_add(length);
                    }
                    if cpy > oend {
                        cpy = oend;
                        length = pdiff(oend, op) as usize;
                    }
                } else {
                    if ip.wrapping_add(length) != iend || cpy > oend {
                        err!();
                    }
                }
                memmove_n(op, ip, length);
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
                if partial_decoding == 0 || cpy == oend || ip >= iend.wrapping_sub(2) {
                    break 'safe;
                }
            } else {
                LZ4_wildCopy8(op, ip, cpy);
                ip = ip.wrapping_add(length);
                op = cpy;
            }

            /* get offset */
            offset = readLE16(ip) as usize;
            ip = ip.wrapping_add(2);
            m = op.wrapping_sub(offset);

            /* get matchlength */
            length = (token & ML_MASK) as usize;

            entry = Entry::CopyMatch;
        }

        if entry == Entry::CopyMatch {
            if length == ML_MASK as usize {
                let addl = read_variable_length(
                    &mut ip,
                    iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                    false,
                );
                if addl == RVL_ERROR {
                    err!();
                }
                length = length.wrapping_add(addl);
                if (op as usize).wrapping_add(length) < (op as usize) {
                    err!();
                }
            }
            length = length.wrapping_add(MINMATCH);
            entry = Entry::SafeMatchCopy;
        }

        /* safe_match_copy */
        {
            if check_offset && m.wrapping_add(dict_size) < low_prefix {
                err!();
            }
            if dict == USING_EXT_DICT && m < low_prefix {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partial_decoding != 0 {
                        let avail = pdiff(oend, op) as usize;
                        if avail < length {
                            length = avail;
                        }
                    } else {
                        err!();
                    }
                }

                if length <= pdiff(low_prefix, m) as usize {
                    memmove_n(
                        op,
                        dict_end.wrapping_sub(pdiff(low_prefix, m) as usize),
                        length,
                    );
                    op = op.wrapping_add(length);
                } else {
                    let copy_size = pdiff(low_prefix, m) as usize;
                    let rest_size = length - copy_size;
                    memcpy_n(op, dict_end.wrapping_sub(copy_size), copy_size);
                    op = op.wrapping_add(copy_size);
                    if rest_size > (pdiff(op, low_prefix) as usize) {
                        let end_of_match = op.wrapping_add(rest_size);
                        let mut copy_from = low_prefix;
                        while op < end_of_match {
                            *op = *copy_from;
                            op = op.wrapping_add(1);
                            copy_from = copy_from.wrapping_add(1);
                        }
                    } else {
                        memcpy_n(op, low_prefix, rest_size);
                        op = op.wrapping_add(rest_size);
                    }
                }
                entry = Entry::Top;
                continue 'safe;
            }

            /* copy match within block */
            cpy = op.wrapping_add(length);

            if partial_decoding != 0 && cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let avail = pdiff(oend, op) as usize;
                let mlen = if length < avail { length } else { avail };
                let match_end = m.wrapping_add(mlen);
                let copy_end = op.wrapping_add(mlen);
                if match_end > op {
                    while op < copy_end {
                        *op = *m;
                        op = op.wrapping_add(1);
                        m = m.wrapping_add(1);
                    }
                } else {
                    memcpy_n(op, m, mlen);
                }
                op = copy_end;
                if op == oend {
                    break 'safe;
                }
                entry = Entry::Top;
                continue 'safe;
            }

            if offset < 8 {
                write32(op, 0);
                *op.wrapping_add(0) = *m.wrapping_add(0);
                *op.wrapping_add(1) = *m.wrapping_add(1);
                *op.wrapping_add(2) = *m.wrapping_add(2);
                *op.wrapping_add(3) = *m.wrapping_add(3);
                m = m.wrapping_add(INC32TABLE[offset] as usize);
                memcpy_n(op.wrapping_add(4), m, 4);
                m = m.wrapping_offset(-(DEC64TABLE[offset] as isize));
            } else {
                memcpy_n(op, m, 8);
                m = m.wrapping_add(8);
            }
            op = op.wrapping_add(8);

            if cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let o_copy_limit = oend.wrapping_sub(WILDCOPYLENGTH - 1);
                if cpy > oend.wrapping_sub(LASTLITERALS) {
                    err!();
                }
                if op < o_copy_limit {
                    LZ4_wildCopy8(op, m, o_copy_limit);
                    m = m.wrapping_offset(pdiff(o_copy_limit, op));
                    op = o_copy_limit;
                }
                while op < cpy {
                    *op = *m;
                    op = op.wrapping_add(1);
                    m = m.wrapping_add(1);
                }
            } else {
                memcpy_n(op, m, 8);
                if length > 16 {
                    LZ4_wildCopy8(op.wrapping_add(8), m.wrapping_add(8), cpy);
                }
            }
            op = cpy;
        }

        entry = Entry::Top;
    }

    pdiff(op, dst as *const u8) as c_int
}

/* ===== Instantiate the API decoding functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_decompressed_size: c_int,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        max_decompressed_size,
        DECODE_FULL_BLOCK,
        NO_DICT,
        dest as *const u8,
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial(
    src: *const c_char,
    dst: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    mut dst_capacity: c_int,
) -> c_int {
    if target_output_size < dst_capacity {
        dst_capacity = target_output_size;
    }
    LZ4_decompress_generic(
        src,
        dst,
        compressed_size,
        dst_capacity,
        PARTIAL_DECODE,
        NO_DICT,
        dst as *const u8,
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
) -> c_int {
    LZ4_decompress_unsafe_generic(
        source as *const u8,
        dest as *mut u8,
        original_size,
        0,
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        max_output_size,
        DECODE_FULL_BLOCK,
        WITH_PREFIX_64K,
        (dest as *const u8).wrapping_sub(64 * 1024),
        ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_partial_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    mut dst_capacity: c_int,
) -> c_int {
    if target_output_size < dst_capacity {
        dst_capacity = target_output_size;
    }
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        dst_capacity,
        PARTIAL_DECODE,
        WITH_PREFIX_64K,
        (dest as *const u8).wrapping_sub(64 * 1024),
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
) -> c_int {
    LZ4_decompress_unsafe_generic(
        source as *const u8,
        dest as *mut u8,
        original_size,
        64 * 1024,
        ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
    prefix_size: usize,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        max_output_size,
        DECODE_FULL_BLOCK,
        NO_DICT,
        (dest as *const u8).wrapping_sub(prefix_size),
        ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_partial_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    mut dst_capacity: c_int,
    prefix_size: usize,
) -> c_int {
    if target_output_size < dst_capacity {
        dst_capacity = target_output_size;
    }
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        dst_capacity,
        PARTIAL_DECODE,
        NO_DICT,
        (dest as *const u8).wrapping_sub(prefix_size),
        ptr::null(),
        0,
    )
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
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        max_output_size,
        DECODE_FULL_BLOCK,
        USING_EXT_DICT,
        dest as *const u8,
        dict_start as *const u8,
        dict_size,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial_forceExtDict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    target_output_size: c_int,
    mut dst_capacity: c_int,
    dict_start: *const c_void,
    dict_size: usize,
) -> c_int {
    if target_output_size < dst_capacity {
        dst_capacity = target_output_size;
    }
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        dst_capacity,
        PARTIAL_DECODE,
        USING_EXT_DICT,
        dest as *const u8,
        dict_start as *const u8,
        dict_size,
    )
}

unsafe fn LZ4_decompress_fast_extDict(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
    dict_start: *const c_void,
    dict_size: usize,
) -> c_int {
    LZ4_decompress_unsafe_generic(
        source as *const u8,
        dest as *mut u8,
        original_size,
        0,
        dict_start as *const u8,
        dict_size,
    )
}

unsafe fn LZ4_decompress_safe_doubleDict(
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
    prefix_size: usize,
    dict_start: *const u8,
    dict_size: usize,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressed_size,
        max_output_size,
        DECODE_FULL_BLOCK,
        USING_EXT_DICT,
        (dest as *const u8).wrapping_sub(prefix_size),
        dict_start,
        dict_size,
    )
}

/* ===== streaming decompression ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamDecode() -> *mut LZ4_streamDecode_t {
    calloc(1, core::mem::size_of::<LZ4_streamDecode_t>()) as *mut LZ4_streamDecode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamDecode(lz4_stream: *mut LZ4_streamDecode_t) -> c_int {
    if lz4_stream.is_null() {
        return 0;
    }
    free(lz4_stream as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setStreamDecode(
    lz4_stream_decode: *mut LZ4_streamDecode_t,
    dictionary: *const c_char,
    dict_size: c_int,
) -> c_int {
    let lz4sd = lz4_stream_decode as *mut LZ4_streamDecode_t_internal;
    (*lz4sd).prefixSize = dict_size as usize;
    if dict_size != 0 {
        (*lz4sd).prefixEnd = (dictionary as *const u8).wrapping_add(dict_size as usize);
    } else {
        (*lz4sd).prefixEnd = dictionary as *const u8;
    }
    (*lz4sd).externalDict = ptr::null();
    (*lz4sd).extDictSize = 0;
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_decoderRingBufferSize(mut max_block_size: c_int) -> c_int {
    if max_block_size < 0 {
        return 0;
    }
    if max_block_size > LZ4_MAX_INPUT_SIZE as c_int {
        return 0;
    }
    if max_block_size < 16 {
        max_block_size = 16;
    }
    65536 + 14 + max_block_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_continue(
    lz4_stream_decode: *mut LZ4_streamDecode_t,
    source: *const c_char,
    dest: *mut c_char,
    compressed_size: c_int,
    max_output_size: c_int,
) -> c_int {
    let lz4sd = lz4_stream_decode as *mut LZ4_streamDecode_t_internal;
    let result: c_int;

    if (*lz4sd).prefixSize == 0 {
        result = LZ4_decompress_safe(source, dest, compressed_size, max_output_size);
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = result as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(result as usize);
    } else if (*lz4sd).prefixEnd == dest as *const u8 {
        if (*lz4sd).prefixSize >= 64 * 1024 - 1 {
            result =
                LZ4_decompress_safe_withPrefix64k(source, dest, compressed_size, max_output_size);
        } else if (*lz4sd).extDictSize == 0 {
            result = LZ4_decompress_safe_withSmallPrefix(
                source,
                dest,
                compressed_size,
                max_output_size,
                (*lz4sd).prefixSize,
            );
        } else {
            result = LZ4_decompress_safe_doubleDict(
                source,
                dest,
                compressed_size,
                max_output_size,
                (*lz4sd).prefixSize,
                (*lz4sd).externalDict,
                (*lz4sd).extDictSize,
            );
        }
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize += result as usize;
        (*lz4sd).prefixEnd = (*lz4sd).prefixEnd.wrapping_add(result as usize);
    } else {
        (*lz4sd).extDictSize = (*lz4sd).prefixSize;
        (*lz4sd).externalDict = (*lz4sd).prefixEnd.wrapping_sub((*lz4sd).extDictSize);
        result = LZ4_decompress_safe_forceExtDict(
            source,
            dest,
            compressed_size,
            max_output_size,
            (*lz4sd).externalDict as *const c_void,
            (*lz4sd).extDictSize,
        );
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = result as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(result as usize);
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_continue(
    lz4_stream_decode: *mut LZ4_streamDecode_t,
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
) -> c_int {
    let lz4sd = lz4_stream_decode as *mut LZ4_streamDecode_t_internal;
    let result: c_int;

    if (*lz4sd).prefixSize == 0 {
        result = LZ4_decompress_fast(source, dest, original_size);
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = original_size as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(original_size as usize);
    } else if (*lz4sd).prefixEnd == dest as *const u8 {
        result = LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            original_size,
            (*lz4sd).prefixSize,
            (*lz4sd).externalDict,
            (*lz4sd).extDictSize,
        );
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize += original_size as usize;
        (*lz4sd).prefixEnd = (*lz4sd).prefixEnd.wrapping_add(original_size as usize);
    } else {
        (*lz4sd).extDictSize = (*lz4sd).prefixSize;
        (*lz4sd).externalDict = (*lz4sd).prefixEnd.wrapping_sub((*lz4sd).extDictSize);
        result = LZ4_decompress_fast_extDict(
            source,
            dest,
            original_size,
            (*lz4sd).externalDict as *const c_void,
            (*lz4sd).extDictSize,
        );
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = original_size as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(original_size as usize);
    }

    result
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
        return LZ4_decompress_safe_withSmallPrefix(
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
            return LZ4_decompress_safe_partial_withPrefix64k(
                source,
                dest,
                compressed_size,
                target_output_size,
                dst_capacity,
            );
        }
        return LZ4_decompress_safe_partial_withSmallPrefix(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    original_size: c_int,
    dict_start: *const c_char,
    dict_size: c_int,
) -> c_int {
    if dict_size == 0 || dict_start.wrapping_add(dict_size as usize) == dest as *const c_char {
        return LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            original_size,
            dict_size as usize,
            ptr::null(),
            0,
        );
    }
    LZ4_decompress_fast_extDict(
        source,
        dest,
        original_size,
        dict_start as *const c_void,
        dict_size as usize,
    )
}

/* ===== Obsolete Functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput(
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
    max_output_size: c_int,
) -> c_int {
    LZ4_compress_default(source, dest, input_size, max_output_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress(
    src: *const c_char,
    dest: *mut c_char,
    src_size: c_int,
) -> c_int {
    LZ4_compress_default(src, dest, src_size, LZ4_compressBound(src_size))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_size: c_int,
) -> c_int {
    LZ4_compress_fast_extState(state, src, dst, src_size, dst_size, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
) -> c_int {
    LZ4_compress_fast_extState(state, src, dst, src_size, LZ4_compressBound(src_size), 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_continue(
    lz4_stream: *mut LZ4_stream_t,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
) -> c_int {
    LZ4_compress_fast_continue(lz4_stream, src, dst, src_size, dst_capacity, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_continue(
    lz4_stream: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    input_size: c_int,
) -> c_int {
    LZ4_compress_fast_continue(
        lz4_stream,
        source,
        dest,
        input_size,
        LZ4_compressBound(input_size),
        1,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress(
    source: *const c_char,
    dest: *mut c_char,
    output_size: c_int,
) -> c_int {
    LZ4_decompress_fast(source, dest, output_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress_unknownOutputSize(
    source: *const c_char,
    dest: *mut c_char,
    isize_: c_int,
    max_output_size: c_int,
) -> c_int {
    LZ4_decompress_safe(source, dest, isize_, max_output_size)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamState() -> c_int {
    core::mem::size_of::<LZ4_stream_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamState(
    state: *mut c_void,
    _input_buffer: *mut c_char,
) -> c_int {
    LZ4_resetStream(state as *mut LZ4_stream_t);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_create(_input_buffer: *mut c_char) -> *mut c_void {
    LZ4_createStream() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char {
    (*(state as *mut LZ4_stream_t_internal)).dictionary as *mut c_char
}
