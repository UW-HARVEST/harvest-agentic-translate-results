// Translation of lz4.c (LZ4 v1.10.0). Target: x86_64 little-endian.
// reg_t = u64 (STEPSIZE=8), sizeof(void*)==8 (byPtr never selected), LZ4_FAST_DEC_LOOP=1.
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

/* ===== Constants ===== */
pub(crate) const MINMATCH: usize = 4;
pub(crate) const WILDCOPYLENGTH: usize = 8;
pub(crate) const LASTLITERALS: usize = 5;
pub(crate) const MFLIMIT: usize = 12;
pub(crate) const MATCH_SAFEGUARD_DISTANCE: usize = (2 * WILDCOPYLENGTH) - MINMATCH;
pub(crate) const FASTLOOP_SAFE_DISTANCE: usize = 64;
pub(crate) const LZ4_minLength: c_int = (MFLIMIT + 1) as c_int;

pub(crate) const LZ4_DISTANCE_MAX: u32 = 65535;
pub(crate) const LZ4_DISTANCE_ABSOLUTE_MAX: u32 = 65535;

pub(crate) const ML_BITS: u32 = 4;
pub(crate) const ML_MASK: u32 = (1 << ML_BITS) - 1;
pub(crate) const RUN_BITS: u32 = 8 - ML_BITS;
pub(crate) const RUN_MASK: u32 = (1 << RUN_BITS) - 1;

pub(crate) const LZ4_MAX_INPUT_SIZE: c_int = 0x7E000000;
const LZ4_ACCELERATION_DEFAULT: c_int = 1;
const LZ4_ACCELERATION_MAX: c_int = 65537;

const LZ4_MEMORY_USAGE: u32 = 14;
const LZ4_HASHLOG: u32 = LZ4_MEMORY_USAGE - 2; // 12
const LZ4_HASHTABLESIZE: usize = 1 << LZ4_MEMORY_USAGE; // bytes for MEM_INIT
const LZ4_HASH_SIZE_U32: usize = 1 << LZ4_HASHLOG; // 4096
const LZ4_STREAM_MINSIZE: usize = (1usize << LZ4_MEMORY_USAGE) + 32; // 16416
const LZ4_STREAMDECODE_MINSIZE: usize = 32;

const LZ4_64Klimit: c_int = (64 * 1024) + (MFLIMIT as c_int - 1);
const LZ4_skipTrigger: u32 = 6;

pub(crate) const LZ4_VERSION_MAJOR: c_int = 1;
pub(crate) const LZ4_VERSION_MINOR: c_int = 10;
pub(crate) const LZ4_VERSION_RELEASE: c_int = 0;
pub(crate) const LZ4_VERSION_NUMBER: c_int =
    LZ4_VERSION_MAJOR * 100 * 100 + LZ4_VERSION_MINOR * 100 + LZ4_VERSION_RELEASE;
static LZ4_VERSION_STRING_BYTES: &[u8] = b"1.10.0\0";

#[inline]
fn LZ4_compressbound(isize_: c_int) -> c_int {
    if (isize_ as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        0
    } else {
        isize_ + (isize_ / 255) + 16
    }
}

/* ===== enums ===== */
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum limitedOutput_directive {
    notLimited = 0,
    limitedOutput = 1,
    fillOutput = 2,
}
use limitedOutput_directive::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum tableType_t {
    clearedTable = 0,
    byPtr = 1,
    byU32 = 2,
    byU16 = 3,
}
use tableType_t::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum dict_directive {
    noDict = 0,
    withPrefix64k,
    usingExtDict,
    usingDictCtx,
}
use dict_directive::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum dictIssue_directive {
    noDictIssue = 0,
    dictSmall,
}
use dictIssue_directive::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum earlyEnd_directive {
    decode_full_block = 0,
    partial_decode = 1,
}
use earlyEnd_directive::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadDict_mode_e {
    _ld_fast,
    _ld_slow,
}

/* ===== structs ===== */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4_stream_t_internal {
    pub hashTable: [u32; LZ4_HASH_SIZE_U32],
    pub dictionary: *const u8,
    pub dictCtx: *const LZ4_stream_t_internal,
    pub currentOffset: u32,
    pub tableType: u32,
    pub dictSize: u32,
}

#[repr(C, align(8))]
pub union LZ4_stream_t {
    pub minStateSize: [u8; LZ4_STREAM_MINSIZE],
    pub internal_donotuse: LZ4_stream_t_internal,
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
pub union LZ4_streamDecode_t {
    pub minStateSize: [u8; LZ4_STREAMDECODE_MINSIZE],
    pub internal_donotuse: LZ4_streamDecode_t_internal,
}

/* ===== memory access helpers (shared with lz4hc) ===== */
#[inline]
pub(crate) unsafe fn LZ4_read16(p: *const u8) -> u16 {
    (p as *const u16).read_unaligned()
}
#[inline]
pub(crate) unsafe fn LZ4_read32(p: *const u8) -> u32 {
    (p as *const u32).read_unaligned()
}
#[inline]
pub(crate) unsafe fn LZ4_read_ARCH(p: *const u8) -> usize {
    (p as *const usize).read_unaligned()
}
#[inline]
pub(crate) unsafe fn LZ4_write16(p: *mut u8, v: u16) {
    (p as *mut u16).write_unaligned(v)
}
#[inline]
pub(crate) unsafe fn LZ4_write32(p: *mut u8, v: u32) {
    (p as *mut u32).write_unaligned(v)
}
#[inline]
pub(crate) fn LZ4_isLittleEndian() -> bool {
    true
}
#[inline]
pub(crate) unsafe fn LZ4_readLE16(p: *const u8) -> u16 {
    LZ4_read16(p)
}
#[inline]
pub(crate) unsafe fn LZ4_writeLE16(p: *mut u8, v: u16) {
    LZ4_write16(p, v)
}

#[inline]
pub(crate) unsafe fn LZ4_memcpy(dst: *mut u8, src: *const u8, n: usize) {
    ptr::copy_nonoverlapping(src, dst, n)
}
#[inline]
pub(crate) unsafe fn LZ4_memmove(dst: *mut u8, src: *const u8, n: usize) {
    ptr::copy(src, dst, n)
}
#[inline]
unsafe fn MEM_INIT(p: *mut u8, v: u8, n: usize) {
    ptr::write_bytes(p, v, n)
}

fn LZ4_isAligned(ptr: *const u8, alignment: usize) -> bool {
    (ptr as usize) & (alignment - 1) == 0
}

#[inline]
pub(crate) unsafe fn LZ4_wildCopy8(dst_ptr: *mut u8, src_ptr: *const u8, dst_end: *mut u8) {
    let mut d = dst_ptr;
    let mut s = src_ptr;
    loop {
        LZ4_memcpy(d, s, 8);
        d = d.add(8);
        s = s.add(8);
        if d >= dst_end {
            break;
        }
    }
}

pub(crate) static inc32table: [u32; 8] = [0, 1, 2, 1, 0, 4, 4, 4];
pub(crate) static dec64table: [i32; 8] = [0, 0, 0, -1, -4, 1, 2, 3];

#[inline]
unsafe fn LZ4_memcpy_using_offset_base(
    dst_ptr: *mut u8,
    src_ptr: *const u8,
    dst_end: *mut u8,
    offset: usize,
) {
    let mut dstPtr = dst_ptr;
    let mut srcPtr = src_ptr;
    if offset < 8 {
        LZ4_write32(dstPtr, 0);
        *dstPtr.add(0) = *srcPtr.add(0);
        *dstPtr.add(1) = *srcPtr.add(1);
        *dstPtr.add(2) = *srcPtr.add(2);
        *dstPtr.add(3) = *srcPtr.add(3);
        srcPtr = srcPtr.add(inc32table[offset] as usize);
        LZ4_memcpy(dstPtr.add(4), srcPtr, 4);
        srcPtr = srcPtr.offset(-(dec64table[offset] as isize));
        dstPtr = dstPtr.add(8);
    } else {
        LZ4_memcpy(dstPtr, srcPtr, 8);
        dstPtr = dstPtr.add(8);
        srcPtr = srcPtr.add(8);
    }
    LZ4_wildCopy8(dstPtr, srcPtr, dst_end);
}

#[inline]
unsafe fn LZ4_wildCopy32(dst_ptr: *mut u8, src_ptr: *const u8, dst_end: *mut u8) {
    let mut d = dst_ptr;
    let mut s = src_ptr;
    loop {
        LZ4_memcpy(d, s, 16);
        LZ4_memcpy(d.add(16), s.add(16), 16);
        d = d.add(32);
        s = s.add(32);
        if d >= dst_end {
            break;
        }
    }
}

#[inline]
unsafe fn LZ4_memcpy_using_offset(
    dst_ptr: *mut u8,
    src_ptr: *const u8,
    dst_end: *mut u8,
    offset: usize,
) {
    let mut v: [u8; 8] = [0; 8];
    let mut dstPtr = dst_ptr;
    match offset {
        1 => {
            MEM_INIT(v.as_mut_ptr(), *src_ptr, 8);
        }
        2 => {
            LZ4_memcpy(v.as_mut_ptr(), src_ptr, 2);
            LZ4_memcpy(v.as_mut_ptr().add(2), src_ptr, 2);
            LZ4_memcpy(v.as_mut_ptr().add(4), v.as_ptr(), 4);
        }
        4 => {
            LZ4_memcpy(v.as_mut_ptr(), src_ptr, 4);
            LZ4_memcpy(v.as_mut_ptr().add(4), src_ptr, 4);
        }
        _ => {
            LZ4_memcpy_using_offset_base(dst_ptr, src_ptr, dst_end, offset);
            return;
        }
    }
    LZ4_memcpy(dstPtr, v.as_ptr(), 8);
    dstPtr = dstPtr.add(8);
    while dstPtr < dst_end {
        LZ4_memcpy(dstPtr, v.as_ptr(), 8);
        dstPtr = dstPtr.add(8);
    }
}

/* ===== common count ===== */
#[inline]
pub(crate) fn LZ4_NbCommonBytes(val: usize) -> u32 {
    // little-endian, 64-bit
    (val.trailing_zeros()) >> 3
}

#[inline]
pub(crate) unsafe fn LZ4_count(pIn: *const u8, pMatch: *const u8, pInLimit: *const u8) -> u32 {
    const STEPSIZE: usize = 8;
    let pStart = pIn;
    let mut pIn = pIn;
    let mut pMatch = pMatch;

    if pIn < pInLimit.wrapping_sub(STEPSIZE - 1) {
        let diff = LZ4_read_ARCH(pMatch) ^ LZ4_read_ARCH(pIn);
        if diff == 0 {
            pIn = pIn.add(STEPSIZE);
            pMatch = pMatch.add(STEPSIZE);
        } else {
            return LZ4_NbCommonBytes(diff);
        }
    }

    while pIn < pInLimit.wrapping_sub(STEPSIZE - 1) {
        let diff = LZ4_read_ARCH(pMatch) ^ LZ4_read_ARCH(pIn);
        if diff == 0 {
            pIn = pIn.add(STEPSIZE);
            pMatch = pMatch.add(STEPSIZE);
            continue;
        }
        pIn = pIn.add(LZ4_NbCommonBytes(diff) as usize);
        return (pIn as usize - pStart as usize) as u32;
    }

    if (pIn < pInLimit.wrapping_sub(3)) && (LZ4_read32(pMatch) == LZ4_read32(pIn)) {
        pIn = pIn.add(4);
        pMatch = pMatch.add(4);
    }
    if (pIn < pInLimit.wrapping_sub(1)) && (LZ4_read16(pMatch) == LZ4_read16(pIn)) {
        pIn = pIn.add(2);
        pMatch = pMatch.add(2);
    }
    if (pIn < pInLimit) && (*pMatch == *pIn) {
        pIn = pIn.add(1);
    }
    (pIn as usize - pStart as usize) as u32
}

/* ===== hashing ===== */
#[inline]
fn LZ4_hash4(sequence: u32, tableType: tableType_t) -> u32 {
    if tableType == byU16 {
        sequence.wrapping_mul(2654435761) >> ((MINMATCH as u32 * 8) - (LZ4_HASHLOG + 1))
    } else {
        sequence.wrapping_mul(2654435761) >> ((MINMATCH as u32 * 8) - LZ4_HASHLOG)
    }
}

#[inline]
fn LZ4_hash5(sequence: u64, tableType: tableType_t) -> u32 {
    let hashLog = if tableType == byU16 {
        LZ4_HASHLOG + 1
    } else {
        LZ4_HASHLOG
    };
    // little-endian
    let prime5bytes: u64 = 889523592379;
    ((sequence << 24).wrapping_mul(prime5bytes) >> (64 - hashLog)) as u32
}

#[inline]
unsafe fn LZ4_hashPosition(p: *const u8, tableType: tableType_t) -> u32 {
    if tableType != byU16 {
        // sizeof(reg_t)==8
        return LZ4_hash5(LZ4_read_ARCH(p) as u64, tableType);
    }
    LZ4_hash4(LZ4_read32(p), tableType)
}

#[inline]
unsafe fn LZ4_clearHash(h: u32, tableBase: *mut u8, tableType: tableType_t) {
    match tableType {
        clearedTable => {}
        byPtr => {
            let ht = tableBase as *mut *const u8;
            *ht.add(h as usize) = ptr::null();
        }
        byU32 => {
            let ht = tableBase as *mut u32;
            *ht.add(h as usize) = 0;
        }
        byU16 => {
            let ht = tableBase as *mut u16;
            *ht.add(h as usize) = 0;
        }
    }
}

#[inline]
unsafe fn LZ4_putIndexOnHash(idx: u32, h: u32, tableBase: *mut u8, tableType: tableType_t) {
    match tableType {
        clearedTable | byPtr => {}
        byU32 => {
            let ht = tableBase as *mut u32;
            *ht.add(h as usize) = idx;
        }
        byU16 => {
            let ht = tableBase as *mut u16;
            *ht.add(h as usize) = idx as u16;
        }
    }
}

#[inline]
unsafe fn LZ4_putPositionOnHash(p: *const u8, h: u32, tableBase: *mut u8, _tableType: tableType_t) {
    let ht = tableBase as *mut *const u8;
    *ht.add(h as usize) = p;
}

#[inline]
unsafe fn LZ4_putPosition(p: *const u8, tableBase: *mut u8, tableType: tableType_t) {
    let h = LZ4_hashPosition(p, tableType);
    LZ4_putPositionOnHash(p, h, tableBase, tableType);
}

#[inline]
unsafe fn LZ4_getIndexOnHash(h: u32, tableBase: *const u8, tableType: tableType_t) -> u32 {
    if tableType == byU32 {
        let ht = tableBase as *const u32;
        return *ht.add(h as usize);
    }
    if tableType == byU16 {
        let ht = tableBase as *const u16;
        return *ht.add(h as usize) as u32;
    }
    0
}

#[inline]
unsafe fn LZ4_getPositionOnHash(h: u32, tableBase: *const u8, _tableType: tableType_t) -> *const u8 {
    let ht = tableBase as *const *const u8;
    *ht.add(h as usize)
}

#[inline]
unsafe fn LZ4_getPosition(p: *const u8, tableBase: *const u8, tableType: tableType_t) -> *const u8 {
    let h = LZ4_hashPosition(p, tableType);
    LZ4_getPositionOnHash(h, tableBase, tableType)
}

/* ===== local utils (exported) ===== */
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
    LZ4_compressbound(isize_)
}
#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofState() -> c_int {
    core::mem::size_of::<LZ4_stream_t>() as c_int
}

#[inline]
unsafe fn ct_internal(s: *mut LZ4_stream_t) -> *mut LZ4_stream_t_internal {
    s as *mut LZ4_stream_t_internal
}

unsafe fn LZ4_prepareTable(cctx: *mut LZ4_stream_t_internal, inputSize: c_int, tableType: tableType_t) {
    let cctx = &mut *cctx;
    if cctx.tableType != (clearedTable as u32) {
        let tt = cctx.tableType;
        let cur_tt = if tt == byU32 as u32 {
            byU32
        } else if tt == byU16 as u32 {
            byU16
        } else if tt == byPtr as u32 {
            byPtr
        } else {
            clearedTable
        };
        if cur_tt != tableType
            || ((tableType == byU16)
                && (cctx.currentOffset.wrapping_add(inputSize as u32) >= 0xFFFF))
            || ((tableType == byU32) && (cctx.currentOffset > (1u32 << 30)))
            || tableType == byPtr
            || inputSize >= 4 * 1024
        {
            MEM_INIT(cctx.hashTable.as_mut_ptr() as *mut u8, 0, LZ4_HASHTABLESIZE);
            cctx.currentOffset = 0;
            cctx.tableType = clearedTable as u32;
        }
    }

    if cctx.currentOffset != 0 && tableType == byU32 {
        cctx.currentOffset = cctx.currentOffset.wrapping_add(64 * 1024);
    }

    cctx.dictCtx = ptr::null();
    cctx.dictionary = ptr::null();
    cctx.dictSize = 0;
}

unsafe fn LZ4_compress_generic_validated(
    cctx: *mut LZ4_stream_t_internal,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    inputConsumed: *mut c_int,
    maxOutputSize: c_int,
    outputDirective: limitedOutput_directive,
    tableType: tableType_t,
    dictDirective: dict_directive,
    dictIssue: dictIssue_directive,
    acceleration: c_int,
) -> c_int {
    let cctxr = &mut *cctx;
    let result: c_int;
    let mut ip = source as *const u8;

    let startIndex = cctxr.currentOffset;
    let base = (source as *const u8).wrapping_sub(startIndex as usize);
    let mut lowLimit: *const u8;

    let dictCtx = cctxr.dictCtx;
    let dictionary: *const u8 = if dictDirective == usingDictCtx {
        (*dictCtx).dictionary
    } else {
        cctxr.dictionary
    };
    let dictSize: u32 = if dictDirective == usingDictCtx {
        (*dictCtx).dictSize
    } else {
        cctxr.dictSize
    };
    let dictDelta: u32 = if dictDirective == usingDictCtx {
        startIndex.wrapping_sub((*dictCtx).currentOffset)
    } else {
        0
    };

    let maybe_extMem = (dictDirective == usingExtDict) || (dictDirective == usingDictCtx);
    let prefixIdxLimit = startIndex.wrapping_sub(dictSize);
    let dictEnd = if dictionary.is_null() {
        dictionary
    } else {
        dictionary.wrapping_add(dictSize as usize)
    };
    let mut anchor = source as *const u8;
    let iend = ip.wrapping_add(inputSize as usize);
    let mflimitPlusOne = iend.wrapping_sub(MFLIMIT).wrapping_add(1);
    let matchlimit = iend.wrapping_sub(LASTLITERALS);

    let dictBase: *const u8 = if dictionary.is_null() {
        ptr::null()
    } else if dictDirective == usingDictCtx {
        dictionary
            .wrapping_add(dictSize as usize)
            .wrapping_sub((*dictCtx).currentOffset as usize)
    } else {
        dictionary
            .wrapping_add(dictSize as usize)
            .wrapping_sub(startIndex as usize)
    };

    let mut op = dest as *mut u8;
    let olimit = op.wrapping_add(maxOutputSize as usize);

    let mut offset: u32 = 0;
    let mut forwardH: u32;

    if outputDirective == fillOutput && maxOutputSize < 1 {
        return 0;
    }

    lowLimit = (source as *const u8)
        .wrapping_sub(if dictDirective == withPrefix64k { dictSize as usize } else { 0 });

    if dictDirective == usingDictCtx {
        cctxr.dictCtx = ptr::null();
        cctxr.dictSize = inputSize as u32;
    } else {
        cctxr.dictSize = cctxr.dictSize.wrapping_add(inputSize as u32);
    }
    cctxr.currentOffset = cctxr.currentOffset.wrapping_add(inputSize as u32);
    cctxr.tableType = tableType as u32;

    let hashTable = cctxr.hashTable.as_mut_ptr() as *mut u8;

    if inputSize < LZ4_minLength {
        // goto _last_literals
        return finalize_last_literals(
            ip, iend, anchor, op, olimit, outputDirective, source, dest, inputConsumed,
        );
    }

    // First Byte
    {
        let h = LZ4_hashPosition(ip, tableType);
        if tableType == byPtr {
            LZ4_putPositionOnHash(ip, h, hashTable, byPtr);
        } else {
            LZ4_putIndexOnHash(startIndex, h, hashTable, tableType);
        }
    }
    ip = ip.add(1);
    forwardH = LZ4_hashPosition(ip, tableType);

    // Main loop
    'main: loop {
        let mut match_: *const u8;
        let mut token: *mut u8;
        let filledIp: *const u8;

        // Find a match
        if tableType == byPtr {
            let mut forwardIp = ip;
            let mut step: isize = 1;
            let mut searchMatchNb = (acceleration as u32) << LZ4_skipTrigger;
            loop {
                let h = forwardH;
                ip = forwardIp;
                forwardIp = forwardIp.wrapping_offset(step);
                step = (searchMatchNb >> LZ4_skipTrigger) as isize;
                searchMatchNb += 1;
                if forwardIp > mflimitPlusOne {
                    return finalize_last_literals(
                        ip, iend, anchor, op, olimit, outputDirective, source, dest, inputConsumed,
                    );
                }
                match_ = LZ4_getPositionOnHash(h, hashTable, tableType);
                forwardH = LZ4_hashPosition(forwardIp, tableType);
                LZ4_putPositionOnHash(ip, h, hashTable, tableType);
                if !((match_.wrapping_add(LZ4_DISTANCE_MAX as usize) < ip)
                    || (LZ4_read32(match_) != LZ4_read32(ip)))
                {
                    break;
                }
            }
        } else {
            let mut forwardIp = ip;
            let mut step: isize = 1;
            let mut searchMatchNb = (acceleration as u32) << LZ4_skipTrigger;
            loop {
                let h = forwardH;
                let current = (forwardIp as usize - base as usize) as u32;
                let mut matchIndex = LZ4_getIndexOnHash(h, hashTable, tableType);
                ip = forwardIp;
                forwardIp = forwardIp.wrapping_offset(step);
                step = (searchMatchNb >> LZ4_skipTrigger) as isize;
                searchMatchNb += 1;
                if forwardIp > mflimitPlusOne {
                    return finalize_last_literals(
                        ip, iend, anchor, op, olimit, outputDirective, source, dest, inputConsumed,
                    );
                }

                if dictDirective == usingDictCtx {
                    if matchIndex < startIndex {
                        matchIndex = LZ4_getIndexOnHash(
                            h,
                            (*dictCtx).hashTable.as_ptr() as *const u8,
                            byU32,
                        );
                        match_ = dictBase.wrapping_add(matchIndex as usize);
                        matchIndex = matchIndex.wrapping_add(dictDelta);
                        lowLimit = dictionary;
                    } else {
                        match_ = base.wrapping_add(matchIndex as usize);
                        lowLimit = source as *const u8;
                    }
                } else if dictDirective == usingExtDict {
                    if matchIndex < startIndex {
                        match_ = dictBase.wrapping_add(matchIndex as usize);
                        lowLimit = dictionary;
                    } else {
                        match_ = base.wrapping_add(matchIndex as usize);
                        lowLimit = source as *const u8;
                    }
                } else {
                    match_ = base.wrapping_add(matchIndex as usize);
                }
                forwardH = LZ4_hashPosition(forwardIp, tableType);
                LZ4_putIndexOnHash(current, h, hashTable, tableType);

                if (dictIssue == dictSmall) && (matchIndex < prefixIdxLimit) {
                    continue;
                }
                if ((tableType != byU16) || (LZ4_DISTANCE_MAX < LZ4_DISTANCE_ABSOLUTE_MAX))
                    && (matchIndex.wrapping_add(LZ4_DISTANCE_MAX) < current)
                {
                    continue;
                }

                if LZ4_read32(match_) == LZ4_read32(ip) {
                    if maybe_extMem {
                        offset = current.wrapping_sub(matchIndex);
                    }
                    break;
                }
            }
        }

        // Catch up
        filledIp = ip;
        if (match_ > lowLimit) && (*ip.offset(-1) == *match_.offset(-1)) {
            loop {
                ip = ip.offset(-1);
                match_ = match_.offset(-1);
                if !(((ip > anchor) && (match_ > lowLimit)) && (*ip.offset(-1) == *match_.offset(-1)))
                {
                    break;
                }
            }
        }

        // Encode Literals
        {
            let litLength = (ip as usize - anchor as usize) as u32;
            token = op;
            op = op.add(1);
            if (outputDirective == limitedOutput)
                && (op
                    .wrapping_add(litLength as usize)
                    .wrapping_add(2 + 1 + LASTLITERALS)
                    .wrapping_add((litLength / 255) as usize)
                    > olimit)
            {
                return 0;
            }
            if (outputDirective == fillOutput)
                && (op
                    .wrapping_add(((litLength + 240) / 255) as usize)
                    .wrapping_add(litLength as usize)
                    .wrapping_add(2)
                    .wrapping_add(1)
                    .wrapping_add(MFLIMIT - MINMATCH)
                    > olimit)
            {
                op = op.offset(-1);
                return finalize_last_literals(
                    ip, iend, anchor, op, olimit, outputDirective, source, dest, inputConsumed,
                );
            }
            if litLength >= RUN_MASK {
                let mut len = litLength - RUN_MASK;
                *token = (RUN_MASK << ML_BITS) as u8;
                while len >= 255 {
                    *op = 255;
                    op = op.add(1);
                    len -= 255;
                }
                *op = len as u8;
                op = op.add(1);
            } else {
                *token = ((litLength) << ML_BITS) as u8;
            }
            LZ4_wildCopy8(op, anchor, op.wrapping_add(litLength as usize));
            op = op.add(litLength as usize);
        }

        // _next_match loop
        loop {
            if (outputDirective == fillOutput)
                && (op
                    .wrapping_add(2)
                    .wrapping_add(1)
                    .wrapping_add(MFLIMIT - MINMATCH)
                    > olimit)
            {
                op = token;
                return finalize_last_literals(
                    ip, iend, anchor, op, olimit, outputDirective, source, dest, inputConsumed,
                );
            }

            // Encode Offset
            if maybe_extMem {
                LZ4_writeLE16(op, offset as u16);
                op = op.add(2);
            } else {
                LZ4_writeLE16(op, (ip as usize - match_ as usize) as u16);
                op = op.add(2);
            }

            // Encode MatchLength
            {
                let mut matchCode: u32;
                if (dictDirective == usingExtDict || dictDirective == usingDictCtx)
                    && (lowLimit == dictionary)
                {
                    // C: const BYTE* limit = ip + (dictEnd-match);
                    // `dictEnd - match` is a signed ptrdiff_t and CAN be
                    // negative (match may sit past dictEnd in dictCtx mode),
                    // so the subtraction must wrap exactly as C's does.
                    let mut limit =
                        ip.wrapping_add((dictEnd as usize).wrapping_sub(match_ as usize));
                    if limit > matchlimit {
                        limit = matchlimit;
                    }
                    matchCode = LZ4_count(ip.add(MINMATCH), match_.add(MINMATCH), limit);
                    ip = ip.add(matchCode as usize + MINMATCH);
                    if ip == limit {
                        let more = LZ4_count(limit, source as *const u8, matchlimit);
                        matchCode += more;
                        ip = ip.add(more as usize);
                    }
                } else {
                    matchCode = LZ4_count(ip.add(MINMATCH), match_.add(MINMATCH), matchlimit);
                    ip = ip.add(matchCode as usize + MINMATCH);
                }

                if (outputDirective != notLimited)
                    && (op
                        .wrapping_add(1 + LASTLITERALS)
                        .wrapping_add(((matchCode + 240) / 255) as usize)
                        > olimit)
                {
                    if outputDirective == fillOutput {
                        let newMatchCode = 15u32 - 1
                            + ((olimit as usize - op as usize) as u32 - 1 - LASTLITERALS as u32)
                                * 255;
                        ip = ip.wrapping_sub((matchCode - newMatchCode) as usize);
                        matchCode = newMatchCode;
                        if ip <= filledIp {
                            let mut p = ip;
                            while p <= filledIp {
                                let h = LZ4_hashPosition(p, tableType);
                                LZ4_clearHash(h, hashTable, tableType);
                                p = p.add(1);
                            }
                        }
                    } else {
                        return 0;
                    }
                }
                if matchCode >= ML_MASK {
                    *token += ML_MASK as u8;
                    matchCode -= ML_MASK;
                    LZ4_write32(op, 0xFFFFFFFF);
                    while matchCode >= 4 * 255 {
                        op = op.add(4);
                        LZ4_write32(op, 0xFFFFFFFF);
                        matchCode -= 4 * 255;
                    }
                    op = op.add((matchCode / 255) as usize);
                    *op = (matchCode % 255) as u8;
                    op = op.add(1);
                } else {
                    *token += matchCode as u8;
                }
            }

            anchor = ip;

            if ip >= mflimitPlusOne {
                break 'main;
            }

            // Fill table
            {
                let h = LZ4_hashPosition(ip.offset(-2), tableType);
                if tableType == byPtr {
                    LZ4_putPositionOnHash(ip.offset(-2), h, hashTable, byPtr);
                } else {
                    let idx = (ip.offset(-2) as usize - base as usize) as u32;
                    LZ4_putIndexOnHash(idx, h, hashTable, tableType);
                }
            }

            // Test next position
            if tableType == byPtr {
                match_ = LZ4_getPosition(ip, hashTable, tableType);
                LZ4_putPosition(ip, hashTable, tableType);
                if (match_.wrapping_add(LZ4_DISTANCE_MAX as usize) >= ip)
                    && (LZ4_read32(match_) == LZ4_read32(ip))
                {
                    token = op;
                    op = op.add(1);
                    *token = 0;
                    continue; // goto _next_match
                }
            } else {
                let h = LZ4_hashPosition(ip, tableType);
                let current = (ip as usize - base as usize) as u32;
                let mut matchIndex = LZ4_getIndexOnHash(h, hashTable, tableType);
                if dictDirective == usingDictCtx {
                    if matchIndex < startIndex {
                        matchIndex = LZ4_getIndexOnHash(
                            h,
                            (*dictCtx).hashTable.as_ptr() as *const u8,
                            byU32,
                        );
                        match_ = dictBase.wrapping_add(matchIndex as usize);
                        lowLimit = dictionary;
                        matchIndex = matchIndex.wrapping_add(dictDelta);
                    } else {
                        match_ = base.wrapping_add(matchIndex as usize);
                        lowLimit = source as *const u8;
                    }
                } else if dictDirective == usingExtDict {
                    if matchIndex < startIndex {
                        match_ = dictBase.wrapping_add(matchIndex as usize);
                        lowLimit = dictionary;
                    } else {
                        match_ = base.wrapping_add(matchIndex as usize);
                        lowLimit = source as *const u8;
                    }
                } else {
                    match_ = base.wrapping_add(matchIndex as usize);
                }
                LZ4_putIndexOnHash(current, h, hashTable, tableType);
                let cond_a = if dictIssue == dictSmall {
                    matchIndex >= prefixIdxLimit
                } else {
                    true
                };
                let cond_b = if (tableType == byU16)
                    && (LZ4_DISTANCE_MAX == LZ4_DISTANCE_ABSOLUTE_MAX)
                {
                    true
                } else {
                    matchIndex.wrapping_add(LZ4_DISTANCE_MAX) >= current
                };
                if cond_a && cond_b && (LZ4_read32(match_) == LZ4_read32(ip)) {
                    token = op;
                    op = op.add(1);
                    *token = 0;
                    if maybe_extMem {
                        offset = current.wrapping_sub(matchIndex);
                    }
                    continue; // goto _next_match
                }
            }

            // Prepare next loop
            ip = ip.add(1);
            forwardH = LZ4_hashPosition(ip, tableType);
            break; // back to main loop top
        }
    }

    // _last_literals
    result = finalize_last_literals(
        ip, iend, anchor, op, olimit, outputDirective, source, dest, inputConsumed,
    );
    result
}

#[inline]
unsafe fn finalize_last_literals(
    ip_in: *const u8,
    iend: *const u8,
    anchor: *const u8,
    op_in: *mut u8,
    olimit: *mut u8,
    outputDirective: limitedOutput_directive,
    source: *const c_char,
    dest: *mut c_char,
    inputConsumed: *mut c_int,
) -> c_int {
    let mut op = op_in;
    let mut ip = ip_in;
    let mut lastRun = (iend as usize - anchor as usize) as usize;
    if (outputDirective != notLimited)
        && (op
            .wrapping_add(lastRun)
            .wrapping_add(1)
            .wrapping_add((lastRun + 255 - RUN_MASK as usize) / 255)
            > olimit)
    {
        if outputDirective == fillOutput {
            lastRun = (olimit as usize - op as usize) - 1;
            lastRun -= (lastRun + 256 - RUN_MASK as usize) / 256;
        } else {
            return 0;
        }
    }
    if lastRun >= RUN_MASK as usize {
        let mut accumulator = lastRun - RUN_MASK as usize;
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
        *op = ((lastRun as u32) << ML_BITS) as u8;
        op = op.add(1);
    }
    LZ4_memcpy(op, anchor, lastRun);
    ip = anchor.add(lastRun);
    op = op.add(lastRun);

    if outputDirective == fillOutput {
        *inputConsumed = (ip as usize - source as usize) as c_int;
    }
    (op as usize - dest as usize) as c_int
}

unsafe fn LZ4_compress_generic(
    cctx: *mut LZ4_stream_t_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    inputConsumed: *mut c_int,
    dstCapacity: c_int,
    outputDirective: limitedOutput_directive,
    tableType: tableType_t,
    dictDirective: dict_directive,
    dictIssue: dictIssue_directive,
    acceleration: c_int,
) -> c_int {
    if (srcSize as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        return 0;
    }
    if srcSize == 0 {
        if outputDirective != notLimited && dstCapacity <= 0 {
            return 0;
        }
        *dst = 0;
        if outputDirective == fillOutput {
            *inputConsumed = 0;
        }
        return 1;
    }

    LZ4_compress_generic_validated(
        cctx,
        src,
        dst,
        srcSize,
        inputConsumed,
        dstCapacity,
        outputDirective,
        tableType,
        dictDirective,
        dictIssue,
        acceleration,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_extState(
    state: *mut c_void,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    maxOutputSize: c_int,
    mut acceleration: c_int,
) -> c_int {
    let ctx = ct_internal(LZ4_initStream(state, core::mem::size_of::<LZ4_stream_t>()));
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }
    if maxOutputSize >= LZ4_compressBound(inputSize) {
        if inputSize < LZ4_64Klimit {
            LZ4_compress_generic(
                ctx, source, dest, inputSize, ptr::null_mut(), 0, notLimited, byU16, noDict,
                noDictIssue, acceleration,
            )
        } else {
            let tableType = byU32;
            LZ4_compress_generic(
                ctx, source, dest, inputSize, ptr::null_mut(), 0, notLimited, tableType, noDict,
                noDictIssue, acceleration,
            )
        }
    } else {
        if inputSize < LZ4_64Klimit {
            LZ4_compress_generic(
                ctx, source, dest, inputSize, ptr::null_mut(), maxOutputSize, limitedOutput, byU16,
                noDict, noDictIssue, acceleration,
            )
        } else {
            let tableType = byU32;
            LZ4_compress_generic(
                ctx, source, dest, inputSize, ptr::null_mut(), maxOutputSize, limitedOutput,
                tableType, noDict, noDictIssue, acceleration,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_extState_fastReset(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    mut acceleration: c_int,
) -> c_int {
    let ctx = ct_internal(state as *mut LZ4_stream_t);
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }

    if dstCapacity >= LZ4_compressBound(srcSize) {
        if srcSize < LZ4_64Klimit {
            let tableType = byU16;
            LZ4_prepareTable(ctx, srcSize, tableType);
            if (*ctx).currentOffset != 0 {
                LZ4_compress_generic(
                    ctx, src, dst, srcSize, ptr::null_mut(), 0, notLimited, tableType, noDict,
                    dictSmall, acceleration,
                )
            } else {
                LZ4_compress_generic(
                    ctx, src, dst, srcSize, ptr::null_mut(), 0, notLimited, tableType, noDict,
                    noDictIssue, acceleration,
                )
            }
        } else {
            let tableType = byU32;
            LZ4_prepareTable(ctx, srcSize, tableType);
            LZ4_compress_generic(
                ctx, src, dst, srcSize, ptr::null_mut(), 0, notLimited, tableType, noDict,
                noDictIssue, acceleration,
            )
        }
    } else {
        if srcSize < LZ4_64Klimit {
            let tableType = byU16;
            LZ4_prepareTable(ctx, srcSize, tableType);
            if (*ctx).currentOffset != 0 {
                LZ4_compress_generic(
                    ctx, src, dst, srcSize, ptr::null_mut(), dstCapacity, limitedOutput, tableType,
                    noDict, dictSmall, acceleration,
                )
            } else {
                LZ4_compress_generic(
                    ctx, src, dst, srcSize, ptr::null_mut(), dstCapacity, limitedOutput, tableType,
                    noDict, noDictIssue, acceleration,
                )
            }
        } else {
            let tableType = byU32;
            LZ4_prepareTable(ctx, srcSize, tableType);
            LZ4_compress_generic(
                ctx, src, dst, srcSize, ptr::null_mut(), dstCapacity, limitedOutput, tableType,
                noDict, noDictIssue, acceleration,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast(
    src: *const c_char,
    dest: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    acceleration: c_int,
) -> c_int {
    // LZ4_HEAPMODE==0 : stack
    let mut ctx: LZ4_stream_t = core::mem::zeroed();
    LZ4_compress_fast_extState(
        &mut ctx as *mut LZ4_stream_t as *mut c_void,
        src,
        dest,
        srcSize,
        dstCapacity,
        acceleration,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_default(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    LZ4_compress_fast(src, dst, srcSize, dstCapacity, 1)
}

unsafe fn LZ4_compress_destSize_extState_internal(
    state: *mut LZ4_stream_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
    acceleration: c_int,
) -> c_int {
    LZ4_initStream(state as *mut c_void, core::mem::size_of::<LZ4_stream_t>());

    if targetDstSize >= LZ4_compressBound(*srcSizePtr) {
        LZ4_compress_fast_extState(
            state as *mut c_void,
            src,
            dst,
            *srcSizePtr,
            targetDstSize,
            acceleration,
        )
    } else {
        if *srcSizePtr < LZ4_64Klimit {
            LZ4_compress_generic(
                ct_internal(state),
                src,
                dst,
                *srcSizePtr,
                srcSizePtr,
                targetDstSize,
                fillOutput,
                byU16,
                noDict,
                noDictIssue,
                acceleration,
            )
        } else {
            let addrMode = byU32;
            LZ4_compress_generic(
                ct_internal(state),
                src,
                dst,
                *srcSizePtr,
                srcSizePtr,
                targetDstSize,
                fillOutput,
                addrMode,
                noDict,
                noDictIssue,
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
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
    acceleration: c_int,
) -> c_int {
    let r = LZ4_compress_destSize_extState_internal(
        state as *mut LZ4_stream_t,
        src,
        dst,
        srcSizePtr,
        targetDstSize,
        acceleration,
    );
    LZ4_initStream(state, core::mem::size_of::<LZ4_stream_t>());
    r
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_destSize(
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
) -> c_int {
    let mut ctxBody: LZ4_stream_t = core::mem::zeroed();
    LZ4_compress_destSize_extState_internal(&mut ctxBody, src, dst, srcSizePtr, targetDstSize, 1)
}

/* ===== Streaming functions ===== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStream() -> *mut LZ4_stream_t {
    let lz4s = crate::c_malloc(core::mem::size_of::<LZ4_stream_t>()) as *mut LZ4_stream_t;
    if lz4s.is_null() {
        return ptr::null_mut();
    }
    LZ4_initStream(lz4s as *mut c_void, core::mem::size_of::<LZ4_stream_t>());
    lz4s
}

fn LZ4_stream_t_alignment() -> usize {
    // #[repr(align(8))]
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
    if !LZ4_isAligned(buffer as *const u8, LZ4_stream_t_alignment()) {
        return ptr::null_mut();
    }
    MEM_INIT(
        buffer as *mut u8,
        0,
        core::mem::size_of::<LZ4_stream_t_internal>(),
    );
    buffer as *mut LZ4_stream_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream(LZ4_stream: *mut LZ4_stream_t) {
    MEM_INIT(
        LZ4_stream as *mut u8,
        0,
        core::mem::size_of::<LZ4_stream_t_internal>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream_fast(ctx: *mut LZ4_stream_t) {
    LZ4_prepareTable(ct_internal(ctx), 0, byU32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStream(LZ4_stream: *mut LZ4_stream_t) -> c_int {
    if LZ4_stream.is_null() {
        return 0;
    }
    crate::c_free(LZ4_stream as *mut u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict_internal(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
    _ld: c_int, // LoadDict_mode_e: 0=fast, 1=slow
) -> c_int {
    const HASH_UNIT: usize = 8; // sizeof(reg_t)
    let dict = ct_internal(LZ4_dict);
    let tableType = byU32;
    let mut p = dictionary as *const u8;
    let dictEnd = p.wrapping_add(dictSize as usize);
    let mut idx32: u32;

    LZ4_resetStream(LZ4_dict);

    (*dict).currentOffset = (*dict).currentOffset.wrapping_add(64 * 1024);

    if dictSize < HASH_UNIT as c_int {
        return 0;
    }

    if (dictEnd as usize - p as usize) > 64 * 1024 {
        p = dictEnd.wrapping_sub(64 * 1024);
    }
    (*dict).dictionary = p;
    (*dict).dictSize = (dictEnd as usize - p as usize) as u32;
    (*dict).tableType = tableType as u32;
    idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);

    let hashTable = (*dict).hashTable.as_mut_ptr() as *mut u8;
    while p <= dictEnd.wrapping_sub(HASH_UNIT) {
        let h = LZ4_hashPosition(p, tableType);
        LZ4_putIndexOnHash(idx32, h, hashTable, tableType);
        p = p.add(3);
        idx32 = idx32.wrapping_add(3);
    }

    if _ld == 1 {
        p = (*dict).dictionary;
        idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);
        while p <= dictEnd.wrapping_sub(HASH_UNIT) {
            let h = LZ4_hashPosition(p, tableType);
            let limit = (*dict).currentOffset.wrapping_sub(64 * 1024);
            if LZ4_getIndexOnHash(h, hashTable, tableType) <= limit {
                LZ4_putIndexOnHash(idx32, h, hashTable, tableType);
            }
            p = p.add(1);
            idx32 = idx32.wrapping_add(1);
        }
    }

    (*dict).dictSize as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    LZ4_loadDict_internal(LZ4_dict, dictionary, dictSize, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictSlow(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    LZ4_loadDict_internal(LZ4_dict, dictionary, dictSize, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_dictionary(
    workingStream: *mut LZ4_stream_t,
    dictionaryStream: *const LZ4_stream_t,
) {
    let mut dictCtx: *const LZ4_stream_t_internal = if dictionaryStream.is_null() {
        ptr::null()
    } else {
        dictionaryStream as *const LZ4_stream_t_internal
    };

    if !dictCtx.is_null() {
        let ws = ct_internal(workingStream);
        if (*ws).currentOffset == 0 {
            (*ws).currentOffset = 64 * 1024;
        }
        if (*dictCtx).dictSize == 0 {
            dictCtx = ptr::null();
        }
    }
    (*ct_internal(workingStream)).dictCtx = dictCtx;
}

unsafe fn LZ4_renormDictT(LZ4_dict: *mut LZ4_stream_t_internal, nextSize: c_int) {
    let d = &mut *LZ4_dict;
    if (d.currentOffset as u64).wrapping_add(nextSize as u64) > 0x80000000 {
        let delta = d.currentOffset.wrapping_sub(64 * 1024);
        let dictEnd = d.dictionary.wrapping_add(d.dictSize as usize);
        for i in 0..LZ4_HASH_SIZE_U32 {
            if d.hashTable[i] < delta {
                d.hashTable[i] = 0;
            } else {
                d.hashTable[i] -= delta;
            }
        }
        d.currentOffset = 64 * 1024;
        if d.dictSize > 64 * 1024 {
            d.dictSize = 64 * 1024;
        }
        d.dictionary = dictEnd.wrapping_sub(d.dictSize as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_fast_continue(
    LZ4_stream: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    maxOutputSize: c_int,
    mut acceleration: c_int,
) -> c_int {
    let tableType = byU32;
    let streamPtr = ct_internal(LZ4_stream);
    let sp = &mut *streamPtr;
    let mut dictEnd: *const u8 = if sp.dictSize != 0 {
        sp.dictionary.wrapping_add(sp.dictSize as usize)
    } else {
        ptr::null()
    };

    LZ4_renormDictT(streamPtr, inputSize);
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }

    if (sp.dictSize < 4)
        && (dictEnd != source as *const u8)
        && (inputSize > 0)
        && (sp.dictCtx.is_null())
    {
        sp.dictSize = 0;
        sp.dictionary = source as *const u8;
        dictEnd = source as *const u8;
    }

    {
        let sourceEnd = (source as *const u8).wrapping_add(inputSize as usize);
        if (sourceEnd > sp.dictionary) && (sourceEnd < dictEnd) {
            sp.dictSize = (dictEnd as usize - sourceEnd as usize) as u32;
            if sp.dictSize > 64 * 1024 {
                sp.dictSize = 64 * 1024;
            }
            if sp.dictSize < 4 {
                sp.dictSize = 0;
            }
            sp.dictionary = dictEnd.wrapping_sub(sp.dictSize as usize);
        }
    }

    if dictEnd == source as *const u8 {
        return if (sp.dictSize < 64 * 1024) && (sp.dictSize < sp.currentOffset) {
            LZ4_compress_generic(
                streamPtr, source, dest, inputSize, ptr::null_mut(), maxOutputSize, limitedOutput,
                tableType, withPrefix64k, dictSmall, acceleration,
            )
        } else {
            LZ4_compress_generic(
                streamPtr, source, dest, inputSize, ptr::null_mut(), maxOutputSize, limitedOutput,
                tableType, withPrefix64k, noDictIssue, acceleration,
            )
        };
    }

    {
        let result: c_int;
        if !sp.dictCtx.is_null() {
            if inputSize > 4 * 1024 {
                LZ4_memcpy(
                    streamPtr as *mut u8,
                    sp.dictCtx as *const u8,
                    core::mem::size_of::<LZ4_stream_t_internal>(),
                );
                result = LZ4_compress_generic(
                    streamPtr, source, dest, inputSize, ptr::null_mut(), maxOutputSize,
                    limitedOutput, tableType, usingExtDict, noDictIssue, acceleration,
                );
            } else {
                result = LZ4_compress_generic(
                    streamPtr, source, dest, inputSize, ptr::null_mut(), maxOutputSize,
                    limitedOutput, tableType, usingDictCtx, noDictIssue, acceleration,
                );
            }
        } else {
            if (sp.dictSize < 64 * 1024) && (sp.dictSize < sp.currentOffset) {
                result = LZ4_compress_generic(
                    streamPtr, source, dest, inputSize, ptr::null_mut(), maxOutputSize,
                    limitedOutput, tableType, usingExtDict, dictSmall, acceleration,
                );
            } else {
                result = LZ4_compress_generic(
                    streamPtr, source, dest, inputSize, ptr::null_mut(), maxOutputSize,
                    limitedOutput, tableType, usingExtDict, noDictIssue, acceleration,
                );
            }
        }
        sp.dictionary = source as *const u8;
        sp.dictSize = inputSize as u32;
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_forceExtDict(
    LZ4_dict: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    srcSize: c_int,
) -> c_int {
    let streamPtr = ct_internal(LZ4_dict);
    let result: c_int;

    LZ4_renormDictT(streamPtr, srcSize);

    if ((*streamPtr).dictSize < 64 * 1024) && ((*streamPtr).dictSize < (*streamPtr).currentOffset) {
        result = LZ4_compress_generic(
            streamPtr, source, dest, srcSize, ptr::null_mut(), 0, notLimited, byU32, usingExtDict,
            dictSmall, 1,
        );
    } else {
        result = LZ4_compress_generic(
            streamPtr, source, dest, srcSize, ptr::null_mut(), 0, notLimited, byU32, usingExtDict,
            noDictIssue, 1,
        );
    }

    (*streamPtr).dictionary = source as *const u8;
    (*streamPtr).dictSize = srcSize as u32;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDict(
    LZ4_dict: *mut LZ4_stream_t,
    safeBuffer: *mut c_char,
    mut dictSize: c_int,
) -> c_int {
    let dict = ct_internal(LZ4_dict);
    if (dictSize as u32) > 64 * 1024 {
        dictSize = 64 * 1024;
    }
    if (dictSize as u32) > (*dict).dictSize {
        dictSize = (*dict).dictSize as c_int;
    }

    if dictSize > 0 {
        let previousDictEnd = (*dict).dictionary.wrapping_add((*dict).dictSize as usize);
        LZ4_memmove(
            safeBuffer as *mut u8,
            previousDictEnd.wrapping_sub(dictSize as usize),
            dictSize as usize,
        );
    }

    (*dict).dictionary = safeBuffer as *const u8;
    (*dict).dictSize = dictSize as u32;
    dictSize
}

/* ===== Decompression ===== */

const rvl_error: usize = usize::MAX;

#[inline]
unsafe fn read_variable_length(
    ip: &mut *const u8,
    ilimit: *const u8,
    initial_check: bool,
) -> usize {
    let mut length: usize = 0;
    let mut s: usize;
    if initial_check && (*ip >= ilimit) {
        return rvl_error;
    }
    s = **ip as usize;
    *ip = (*ip).add(1);
    length += s;
    if *ip > ilimit {
        return rvl_error;
    }
    // sizeof(length)==8, overflow branch skipped
    if s != 255 {
        return length;
    }
    loop {
        s = **ip as usize;
        *ip = (*ip).add(1);
        length += s;
        if *ip > ilimit {
            return rvl_error;
        }
        if s != 255 {
            break;
        }
    }
    length
}

#[inline]
unsafe fn read_long_length_no_check(pp: &mut *const u8) -> usize {
    let mut l: usize = 0;
    let mut b: usize;
    loop {
        b = **pp as usize;
        *pp = (*pp).add(1);
        l += b;
        if b != 255 {
            break;
        }
    }
    l
}

unsafe fn LZ4_decompress_unsafe_generic(
    istart: *const u8,
    ostart: *mut u8,
    decompressedSize: c_int,
    prefixSize: usize,
    dictStart: *const u8,
    dictSize: usize,
) -> c_int {
    let mut ip = istart;
    let mut op = ostart;
    let oend = ostart.wrapping_add(decompressedSize as usize);
    let prefixStart = ostart.wrapping_sub(prefixSize);

    loop {
        let token = *ip as u32;
        ip = ip.add(1);

        // literals
        {
            let mut ll = (token >> ML_BITS) as usize;
            if ll == 15 {
                ll += read_long_length_no_check(&mut ip);
            }
            if (oend as usize - op as usize) < ll {
                return -1;
            }
            LZ4_memmove(op, ip, ll);
            op = op.add(ll);
            ip = ip.add(ll);
            if (oend as usize - op as usize) < MFLIMIT {
                if op == oend {
                    break;
                }
                return -1;
            }
        }

        // match
        {
            let mut ml = (token & 15) as usize;
            let offset = LZ4_readLE16(ip) as usize;
            ip = ip.add(2);
            if ml == 15 {
                ml += read_long_length_no_check(&mut ip);
            }
            ml += MINMATCH;
            if (oend as usize - op as usize) < ml {
                return -1;
            }
            {
                let mut match_ = op.wrapping_sub(offset);
                if offset > (op as usize - prefixStart as usize) + dictSize {
                    return -1;
                }
                if offset > (op as usize - prefixStart as usize) {
                    let dictEnd = dictStart.wrapping_add(dictSize);
                    let extMatch =
                        dictEnd.wrapping_sub(offset - (op as usize - prefixStart as usize));
                    let extml = (dictEnd as usize - extMatch as usize) as usize;
                    if extml > ml {
                        LZ4_memmove(op, extMatch, ml);
                        op = op.add(ml);
                        ml = 0;
                    } else {
                        LZ4_memmove(op, extMatch, extml);
                        op = op.add(extml);
                        ml -= extml;
                    }
                    match_ = prefixStart;
                }
                let mut u = 0usize;
                while u < ml {
                    *op.add(u) = *match_.add(u);
                    u += 1;
                }
            }
            op = op.add(ml);
            if (oend as usize - op as usize) < LASTLITERALS {
                return -1;
            }
        }
    }
    (ip as usize - istart as usize) as c_int
}

unsafe fn LZ4_decompress_generic(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    outputSize: c_int,
    partialDecoding: earlyEnd_directive,
    dict: dict_directive,
    lowPrefix: *const u8,
    dictStart: *const u8,
    dictSize: usize,
) -> c_int {
    if src.is_null() || outputSize < 0 {
        return -1;
    }

    let mut ip = src as *const u8;
    let iend = ip.wrapping_add(srcSize as usize);

    let mut op = dst as *mut u8;
    let oend = op.wrapping_add(outputSize as usize);
    let mut cpy: *mut u8;

    let dictEnd = if dictStart.is_null() {
        ptr::null()
    } else {
        dictStart.wrapping_add(dictSize)
    };

    let checkOffset = dictSize < 64 * 1024;

    let shortiend = iend.wrapping_sub(14).wrapping_sub(2);
    let shortoend = oend.wrapping_sub(14).wrapping_sub(18);

    let mut match_: *const u8 = ptr::null();
    let mut offset: usize = 0;
    let mut token: u32 = 0;
    let mut length: usize = 0;

    if outputSize == 0 {
        if partialDecoding == partial_decode {
            return 0;
        }
        return if (srcSize == 1) && (*ip == 0) { 0 } else { -1 };
    }
    if srcSize == 0 {
        return -1;
    }

    // resume codes: 0 normal top, 1 safe_literal_copy, 2 safe_match_copy
    let mut resume: u8 = 0;

    // ===== FAST LOOP =====
    'fast_wrap: {
        if (oend as usize - op as usize) < FASTLOOP_SAFE_DISTANCE {
            break 'fast_wrap; // goto safe_decode
        }

        loop {
            token = *ip as u32;
            ip = ip.add(1);
            length = (token >> ML_BITS) as usize;

            if length == RUN_MASK as usize {
                let addl =
                    read_variable_length(&mut ip, iend.wrapping_sub(RUN_MASK as usize), true);
                if addl == rvl_error {
                    return output_error(ip, src);
                }
                length += addl;
                if (op as usize).wrapping_add(length) < (op as usize) {
                    return output_error(ip, src);
                }
                if (ip as usize).wrapping_add(length) < (ip as usize) {
                    return output_error(ip, src);
                }
                if (op.wrapping_add(length) > oend.wrapping_sub(32))
                    || (ip.wrapping_add(length) > iend.wrapping_sub(32))
                {
                    resume = 1;
                    break 'fast_wrap; // goto safe_literal_copy
                }
                LZ4_wildCopy32(op, ip, op.wrapping_add(length));
                ip = ip.add(length);
                op = op.add(length);
            } else if ip <= iend.wrapping_sub(16 + 1) {
                LZ4_memcpy(op, ip, 16);
                ip = ip.add(length);
                op = op.add(length);
            } else {
                resume = 1;
                break 'fast_wrap; // goto safe_literal_copy
            }

            offset = LZ4_readLE16(ip) as usize;
            ip = ip.add(2);
            match_ = op.wrapping_sub(offset);

            length = (token & ML_MASK) as usize;

            if length == ML_MASK as usize {
                let addl = read_variable_length(
                    &mut ip,
                    iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                    false,
                );
                if addl == rvl_error {
                    return output_error(ip, src);
                }
                length += addl;
                length += MINMATCH;
                if (op as usize).wrapping_add(length) < (op as usize) {
                    return output_error(ip, src);
                }
                if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                    resume = 2;
                    break 'fast_wrap; // goto safe_match_copy
                }
            } else {
                length += MINMATCH;
                if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                    resume = 2;
                    break 'fast_wrap; // goto safe_match_copy
                }
                if (dict == withPrefix64k) || (match_ >= lowPrefix) {
                    if offset >= 8 {
                        LZ4_memcpy(op, match_, 8);
                        LZ4_memcpy(op.add(8), match_.add(8), 8);
                        LZ4_memcpy(op.add(16), match_.add(16), 2);
                        op = op.add(length);
                        continue;
                    }
                }
            }

            if checkOffset && (match_.wrapping_add(dictSize) < lowPrefix) {
                return output_error(ip, src);
            }
            if (dict == usingExtDict) && (match_ < lowPrefix) {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partialDecoding == partial_decode {
                        length = core::cmp::min(length, oend as usize - op as usize);
                    } else {
                        return output_error(ip, src);
                    }
                }

                if length <= (lowPrefix as usize - match_ as usize) {
                    LZ4_memmove(
                        op,
                        dictEnd.wrapping_sub(lowPrefix as usize - match_ as usize),
                        length,
                    );
                    op = op.add(length);
                } else {
                    let copySize = (lowPrefix as usize - match_ as usize) as usize;
                    let restSize = length - copySize;
                    LZ4_memcpy(op, dictEnd.wrapping_sub(copySize), copySize);
                    op = op.add(copySize);
                    if restSize > (op as usize - lowPrefix as usize) {
                        let endOfMatch = op.wrapping_add(restSize);
                        let mut copyFrom = lowPrefix;
                        while op < endOfMatch {
                            *op = *copyFrom;
                            op = op.add(1);
                            copyFrom = copyFrom.add(1);
                        }
                    } else {
                        LZ4_memcpy(op, lowPrefix, restSize);
                        op = op.add(restSize);
                    }
                }
                continue;
            }

            cpy = op.wrapping_add(length);

            if offset < 16 {
                LZ4_memcpy_using_offset(op, match_, cpy, offset);
            } else {
                LZ4_wildCopy32(op, match_, cpy);
            }
            op = cpy;
        }
    }
    // end fast loop; fall into safe_decode

    // ===== SAFE LOOP =====
    // variables reused: token, length, offset, match_, cpy
    loop {
        let mut go_copy_match = false;
        let mut go_safe_match = false;

        if resume == 2 {
            resume = 0;
            go_safe_match = true;
        } else if resume == 1 {
            resume = 0;
            // enter at safe_literal_copy (from fast loop); token & length are set
            match safe_literal_copy(
                &mut ip, &mut op, iend, oend, &mut length, partialDecoding, dst,
            ) {
                LitResult::OutputError => return output_error(ip, src),
                LitResult::Break => break,
                LitResult::Continue => {
                    // get offset / match / length, then _copy_match
                    offset = LZ4_readLE16(ip) as usize;
                    ip = ip.add(2);
                    match_ = op.wrapping_sub(offset);
                    length = (token & ML_MASK) as usize;
                    go_copy_match = true;
                }
            }
        }

        if !go_safe_match && !go_copy_match {
            // resume == 0 : normal top
            token = *ip as u32;
            ip = ip.add(1);
            length = (token >> ML_BITS) as usize;

            // shortcut
            if (length != RUN_MASK as usize) && ((ip < shortiend) && (op <= shortoend)) {
                LZ4_memcpy(op, ip, 16);
                op = op.add(length);
                ip = ip.add(length);

                length = (token & ML_MASK) as usize;
                offset = LZ4_readLE16(ip) as usize;
                ip = ip.add(2);
                match_ = op.wrapping_sub(offset);

                if (length != ML_MASK as usize)
                    && (offset >= 8)
                    && (dict == withPrefix64k || match_ >= lowPrefix)
                {
                    LZ4_memcpy(op.add(0), match_.add(0), 8);
                    LZ4_memcpy(op.add(8), match_.add(8), 8);
                    LZ4_memcpy(op.add(16), match_.add(16), 2);
                    op = op.add(length + MINMATCH);
                    continue;
                }
                go_copy_match = true; // goto _copy_match
            }

            if !go_copy_match {
                if length == RUN_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip,
                        iend.wrapping_sub(RUN_MASK as usize),
                        true,
                    );
                    if addl == rvl_error {
                        return output_error(ip, src);
                    }
                    length += addl;
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        return output_error(ip, src);
                    }
                    if (ip as usize).wrapping_add(length) < (ip as usize) {
                        return output_error(ip, src);
                    }
                }

                match safe_literal_copy(
                    &mut ip, &mut op, iend, oend, &mut length, partialDecoding, dst,
                ) {
                    LitResult::OutputError => return output_error(ip, src),
                    LitResult::Break => break,
                    LitResult::Continue => {}
                }

                offset = LZ4_readLE16(ip) as usize;
                ip = ip.add(2);
                match_ = op.wrapping_sub(offset);
                length = (token & ML_MASK) as usize;
                // fall into _copy_match
                go_copy_match = true;
            }
        }

        if go_copy_match && !go_safe_match {
            // _copy_match
            if length == ML_MASK as usize {
                let addl = read_variable_length(
                    &mut ip,
                    iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                    false,
                );
                if addl == rvl_error {
                    return output_error(ip, src);
                }
                length += addl;
                if (op as usize).wrapping_add(length) < (op as usize) {
                    return output_error(ip, src);
                }
            }
            length += MINMATCH;
            go_safe_match = true;
        }

        // safe_match_copy
        if go_safe_match {
            if checkOffset && (match_.wrapping_add(dictSize) < lowPrefix) {
                return output_error(ip, src);
            }
            if (dict == usingExtDict) && (match_ < lowPrefix) {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partialDecoding == partial_decode {
                        length = core::cmp::min(length, oend as usize - op as usize);
                    } else {
                        return output_error(ip, src);
                    }
                }
                if length <= (lowPrefix as usize - match_ as usize) {
                    LZ4_memmove(
                        op,
                        dictEnd.wrapping_sub(lowPrefix as usize - match_ as usize),
                        length,
                    );
                    op = op.add(length);
                } else {
                    let copySize = (lowPrefix as usize - match_ as usize) as usize;
                    let restSize = length - copySize;
                    LZ4_memcpy(op, dictEnd.wrapping_sub(copySize), copySize);
                    op = op.add(copySize);
                    if restSize > (op as usize - lowPrefix as usize) {
                        let endOfMatch = op.wrapping_add(restSize);
                        let mut copyFrom = lowPrefix;
                        while op < endOfMatch {
                            *op = *copyFrom;
                            op = op.add(1);
                            copyFrom = copyFrom.add(1);
                        }
                    } else {
                        LZ4_memcpy(op, lowPrefix, restSize);
                        op = op.add(restSize);
                    }
                }
                continue;
            }

            cpy = op.wrapping_add(length);

            if (partialDecoding == partial_decode) && (cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE)) {
                let mlen = core::cmp::min(length, oend as usize - op as usize);
                let matchEnd = match_.wrapping_add(mlen);
                let copyEnd = op.wrapping_add(mlen);
                if matchEnd > op as *const u8 {
                    while op < copyEnd {
                        *op = *match_;
                        op = op.add(1);
                        match_ = match_.add(1);
                    }
                } else {
                    LZ4_memcpy(op, match_, mlen);
                }
                op = copyEnd;
                if op == oend {
                    break;
                }
                continue;
            }

            if offset < 8 {
                LZ4_write32(op, 0);
                *op.add(0) = *match_.add(0);
                *op.add(1) = *match_.add(1);
                *op.add(2) = *match_.add(2);
                *op.add(3) = *match_.add(3);
                match_ = match_.add(inc32table[offset] as usize);
                LZ4_memcpy(op.add(4), match_, 4);
                match_ = match_.offset(-(dec64table[offset] as isize));
            } else {
                LZ4_memcpy(op, match_, 8);
                match_ = match_.add(8);
            }
            op = op.add(8);

            if cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let oCopyLimit = oend.wrapping_sub(WILDCOPYLENGTH - 1);
                if cpy > oend.wrapping_sub(LASTLITERALS) {
                    return output_error(ip, src);
                }
                if op < oCopyLimit {
                    LZ4_wildCopy8(op, match_, oCopyLimit);
                    match_ = match_.add(oCopyLimit as usize - op as usize);
                    op = oCopyLimit;
                }
                while op < cpy {
                    *op = *match_;
                    op = op.add(1);
                    match_ = match_.add(1);
                }
            } else {
                LZ4_memcpy(op, match_, 8);
                if length > 16 {
                    LZ4_wildCopy8(op.add(8), match_.add(8), cpy);
                }
            }
            op = cpy;
        }
    }

    (op as usize - dst as usize) as c_int
}

enum LitResult {
    Continue,
    Break,
    OutputError,
}

#[inline]
unsafe fn safe_literal_copy(
    ip: &mut *const u8,
    op: &mut *mut u8,
    iend: *const u8,
    oend: *mut u8,
    length: &mut usize,
    partialDecoding: earlyEnd_directive,
    _dst: *mut c_char,
) -> LitResult {
    let mut cpy = (*op).wrapping_add(*length);
    if (cpy > oend.wrapping_sub(MFLIMIT))
        || ((*ip).wrapping_add(*length) > iend.wrapping_sub(2 + 1 + LASTLITERALS))
    {
        if partialDecoding == partial_decode {
            if (*ip).wrapping_add(*length) > iend {
                *length = (iend as usize - *ip as usize) as usize;
                cpy = (*op).wrapping_add(*length);
            }
            if cpy > oend {
                cpy = oend;
                *length = (oend as usize - *op as usize) as usize;
            }
        } else {
            if ((*ip).wrapping_add(*length) != iend) || (cpy > oend) {
                return LitResult::OutputError;
            }
        }
        LZ4_memmove(*op, *ip, *length);
        *ip = (*ip).add(*length);
        *op = (*op).add(*length);
        if (partialDecoding != partial_decode) || (cpy == oend) || (*ip >= iend.wrapping_sub(2)) {
            return LitResult::Break;
        }
        // In C this path does NOT continue to match; but the safe loop after
        // this "if" reads offset. Actually after this branch, control continues
        // to read offset/match (does not `continue`). So we return a special
        // marker meaning "proceed to read offset". We use Continue here BUT the
        // caller must know that we already advanced ip/op. However for the
        // non-taken-branch path below (else), we also advance and proceed.
        // To match C exactly we must proceed to offset read in BOTH sub-branches
        // that don't break. Return Continue.
        return LitResult::Continue;
    } else {
        LZ4_wildCopy8(*op, *ip, cpy);
        *ip = (*ip).add(*length);
        *op = cpy;
        LitResult::Continue
    }
}

#[inline]
fn output_error(ip: *const u8, src: *const c_char) -> c_int {
    -(((ip as usize) as isize) - (src as isize)) as c_int - 1
}

/* ===== decoding API ===== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxDecompressedSize: c_int,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        maxDecompressedSize,
        decode_full_block,
        noDict,
        dest as *const u8,
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial(
    src: *const c_char,
    dst: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    mut dstCapacity: c_int,
) -> c_int {
    dstCapacity = core::cmp::min(targetOutputSize, dstCapacity);
    LZ4_decompress_generic(
        src,
        dst,
        compressedSize,
        dstCapacity,
        partial_decode,
        noDict,
        dst as *const u8,
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
) -> c_int {
    LZ4_decompress_unsafe_generic(
        source as *const u8,
        dest as *mut u8,
        originalSize,
        0,
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        maxOutputSize,
        decode_full_block,
        withPrefix64k,
        (dest as *const u8).wrapping_sub(64 * 1024),
        ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_partial_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    mut dstCapacity: c_int,
) -> c_int {
    dstCapacity = core::cmp::min(targetOutputSize, dstCapacity);
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        dstCapacity,
        partial_decode,
        withPrefix64k,
        (dest as *const u8).wrapping_sub(64 * 1024),
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
) -> c_int {
    LZ4_decompress_unsafe_generic(
        source as *const u8,
        dest as *mut u8,
        originalSize,
        64 * 1024,
        ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
    prefixSize: usize,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        maxOutputSize,
        decode_full_block,
        noDict,
        (dest as *const u8).wrapping_sub(prefixSize),
        ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_partial_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    mut dstCapacity: c_int,
    prefixSize: usize,
) -> c_int {
    dstCapacity = core::cmp::min(targetOutputSize, dstCapacity);
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        dstCapacity,
        partial_decode,
        noDict,
        (dest as *const u8).wrapping_sub(prefixSize),
        ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_forceExtDict(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        maxOutputSize,
        decode_full_block,
        usingExtDict,
        dest as *const u8,
        dictStart as *const u8,
        dictSize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial_forceExtDict(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    mut dstCapacity: c_int,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    dstCapacity = core::cmp::min(targetOutputSize, dstCapacity);
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        dstCapacity,
        partial_decode,
        usingExtDict,
        dest as *const u8,
        dictStart as *const u8,
        dictSize,
    )
}

unsafe fn LZ4_decompress_fast_extDict(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    LZ4_decompress_unsafe_generic(
        source as *const u8,
        dest as *mut u8,
        originalSize,
        0,
        dictStart as *const u8,
        dictSize,
    )
}

unsafe fn LZ4_decompress_safe_doubleDict(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
    prefixSize: usize,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        maxOutputSize,
        decode_full_block,
        usingExtDict,
        (dest as *const u8).wrapping_sub(prefixSize),
        dictStart as *const u8,
        dictSize,
    )
}

/* ===== streaming decompression ===== */
#[inline]
unsafe fn sd_internal(s: *mut LZ4_streamDecode_t) -> *mut LZ4_streamDecode_t_internal {
    s as *mut LZ4_streamDecode_t_internal
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamDecode() -> *mut LZ4_streamDecode_t {
    crate::c_calloc(core::mem::size_of::<LZ4_streamDecode_t>()) as *mut LZ4_streamDecode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamDecode(LZ4_stream: *mut LZ4_streamDecode_t) -> c_int {
    if LZ4_stream.is_null() {
        return 0;
    }
    crate::c_free(LZ4_stream as *mut u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setStreamDecode(
    LZ4_streamDecode: *mut LZ4_streamDecode_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    let lz4sd = &mut *sd_internal(LZ4_streamDecode);
    lz4sd.prefixSize = dictSize as usize;
    if dictSize != 0 {
        lz4sd.prefixEnd = (dictionary as *const u8).wrapping_add(dictSize as usize);
    } else {
        lz4sd.prefixEnd = dictionary as *const u8;
    }
    lz4sd.externalDict = ptr::null();
    lz4sd.extDictSize = 0;
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decoderRingBufferSize(mut maxBlockSize: c_int) -> c_int {
    if maxBlockSize < 0 {
        return 0;
    }
    if maxBlockSize > LZ4_MAX_INPUT_SIZE {
        return 0;
    }
    if maxBlockSize < 16 {
        maxBlockSize = 16;
    }
    65536 + 14 + maxBlockSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_continue(
    LZ4_streamDecode: *mut LZ4_streamDecode_t,
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
) -> c_int {
    let lz4sd = &mut *sd_internal(LZ4_streamDecode);
    let result: c_int;

    if lz4sd.prefixSize == 0 {
        result = LZ4_decompress_safe(source, dest, compressedSize, maxOutputSize);
        if result <= 0 {
            return result;
        }
        lz4sd.prefixSize = result as usize;
        lz4sd.prefixEnd = (dest as *mut u8).wrapping_add(result as usize);
    } else if lz4sd.prefixEnd == dest as *const u8 {
        if lz4sd.prefixSize >= 64 * 1024 - 1 {
            result = LZ4_decompress_safe_withPrefix64k(source, dest, compressedSize, maxOutputSize);
        } else if lz4sd.extDictSize == 0 {
            result = LZ4_decompress_safe_withSmallPrefix(
                source,
                dest,
                compressedSize,
                maxOutputSize,
                lz4sd.prefixSize,
            );
        } else {
            result = LZ4_decompress_safe_doubleDict(
                source,
                dest,
                compressedSize,
                maxOutputSize,
                lz4sd.prefixSize,
                lz4sd.externalDict as *const c_void,
                lz4sd.extDictSize,
            );
        }
        if result <= 0 {
            return result;
        }
        lz4sd.prefixSize += result as usize;
        lz4sd.prefixEnd = lz4sd.prefixEnd.wrapping_add(result as usize);
    } else {
        lz4sd.extDictSize = lz4sd.prefixSize;
        lz4sd.externalDict = lz4sd.prefixEnd.wrapping_sub(lz4sd.extDictSize);
        result = LZ4_decompress_safe_forceExtDict(
            source,
            dest,
            compressedSize,
            maxOutputSize,
            lz4sd.externalDict as *const c_void,
            lz4sd.extDictSize,
        );
        if result <= 0 {
            return result;
        }
        lz4sd.prefixSize = result as usize;
        lz4sd.prefixEnd = (dest as *mut u8).wrapping_add(result as usize);
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_continue(
    LZ4_streamDecode: *mut LZ4_streamDecode_t,
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
) -> c_int {
    let lz4sd = &mut *sd_internal(LZ4_streamDecode);
    let result: c_int;

    if lz4sd.prefixSize == 0 {
        result = LZ4_decompress_fast(source, dest, originalSize);
        if result <= 0 {
            return result;
        }
        lz4sd.prefixSize = originalSize as usize;
        lz4sd.prefixEnd = (dest as *mut u8).wrapping_add(originalSize as usize);
    } else if lz4sd.prefixEnd == dest as *const u8 {
        result = LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            originalSize,
            lz4sd.prefixSize,
            lz4sd.externalDict,
            lz4sd.extDictSize,
        );
        if result <= 0 {
            return result;
        }
        lz4sd.prefixSize += originalSize as usize;
        lz4sd.prefixEnd = lz4sd.prefixEnd.wrapping_add(originalSize as usize);
    } else {
        lz4sd.extDictSize = lz4sd.prefixSize;
        lz4sd.externalDict = lz4sd.prefixEnd.wrapping_sub(lz4sd.extDictSize);
        result = LZ4_decompress_fast_extDict(
            source,
            dest,
            originalSize,
            lz4sd.externalDict as *const c_void,
            lz4sd.extDictSize,
        );
        if result <= 0 {
            return result;
        }
        lz4sd.prefixSize = originalSize as usize;
        lz4sd.prefixEnd = (dest as *mut u8).wrapping_add(originalSize as usize);
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
    dictStart: *const c_char,
    dictSize: c_int,
) -> c_int {
    if dictSize == 0 {
        return LZ4_decompress_safe(source, dest, compressedSize, maxOutputSize);
    }
    if (dictStart as *const u8).wrapping_add(dictSize as usize) == dest as *const u8 {
        if dictSize >= 64 * 1024 - 1 {
            return LZ4_decompress_safe_withPrefix64k(source, dest, compressedSize, maxOutputSize);
        }
        return LZ4_decompress_safe_withSmallPrefix(
            source,
            dest,
            compressedSize,
            maxOutputSize,
            dictSize as usize,
        );
    }
    LZ4_decompress_safe_forceExtDict(
        source,
        dest,
        compressedSize,
        maxOutputSize,
        dictStart as *const c_void,
        dictSize as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
    dictStart: *const c_char,
    dictSize: c_int,
) -> c_int {
    if dictSize == 0 {
        return LZ4_decompress_safe_partial(source, dest, compressedSize, targetOutputSize, dstCapacity);
    }
    if (dictStart as *const u8).wrapping_add(dictSize as usize) == dest as *const u8 {
        if dictSize >= 64 * 1024 - 1 {
            return LZ4_decompress_safe_partial_withPrefix64k(
                source,
                dest,
                compressedSize,
                targetOutputSize,
                dstCapacity,
            );
        }
        return LZ4_decompress_safe_partial_withSmallPrefix(
            source,
            dest,
            compressedSize,
            targetOutputSize,
            dstCapacity,
            dictSize as usize,
        );
    }
    LZ4_decompress_safe_partial_forceExtDict(
        source,
        dest,
        compressedSize,
        targetOutputSize,
        dstCapacity,
        dictStart as *const c_void,
        dictSize as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
    dictStart: *const c_char,
    dictSize: c_int,
) -> c_int {
    if dictSize == 0 || (dictStart as *const u8).wrapping_add(dictSize as usize) == dest as *const u8 {
        return LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            originalSize,
            dictSize as usize,
            ptr::null(),
            0,
        );
    }
    LZ4_decompress_fast_extDict(source, dest, originalSize, dictStart as *const c_void, dictSize as usize)
}

/* ===== Obsolete functions ===== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput(
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    maxOutputSize: c_int,
) -> c_int {
    LZ4_compress_default(source, dest, inputSize, maxOutputSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress(
    src: *const c_char,
    dest: *mut c_char,
    srcSize: c_int,
) -> c_int {
    LZ4_compress_default(src, dest, srcSize, LZ4_compressBound(srcSize))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstSize: c_int,
) -> c_int {
    LZ4_compress_fast_extState(state, src, dst, srcSize, dstSize, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    LZ4_compress_fast_extState(state, src, dst, srcSize, LZ4_compressBound(srcSize), 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_continue(
    LZ4_stream: *mut LZ4_stream_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    LZ4_compress_fast_continue(LZ4_stream, src, dst, srcSize, dstCapacity, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_continue(
    LZ4_stream: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
) -> c_int {
    LZ4_compress_fast_continue(LZ4_stream, source, dest, inputSize, LZ4_compressBound(inputSize), 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress(
    source: *const c_char,
    dest: *mut c_char,
    outputSize: c_int,
) -> c_int {
    LZ4_decompress_fast(source, dest, outputSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress_unknownOutputSize(
    source: *const c_char,
    dest: *mut c_char,
    isize_: c_int,
    maxOutputSize: c_int,
) -> c_int {
    LZ4_decompress_safe(source, dest, isize_, maxOutputSize)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamState() -> c_int {
    core::mem::size_of::<LZ4_stream_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamState(state: *mut c_void, _inputBuffer: *mut c_char) -> c_int {
    LZ4_resetStream(state as *mut LZ4_stream_t);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_create(_inputBuffer: *mut c_char) -> *mut c_void {
    LZ4_createStream() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char {
    (*ct_internal(state as *mut LZ4_stream_t)).dictionary as *mut c_char
}
