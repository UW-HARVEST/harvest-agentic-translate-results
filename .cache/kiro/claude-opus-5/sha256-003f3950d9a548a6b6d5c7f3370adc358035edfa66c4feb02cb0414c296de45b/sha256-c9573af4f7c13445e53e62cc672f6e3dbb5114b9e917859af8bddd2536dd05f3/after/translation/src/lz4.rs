//! Translation of `lz4.c` (LZ4 v1.10.0), built with `LZ4_HEAPMODE=0`.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use crate::common::*;
use core::ffi::{c_char, c_int, c_void};

pub const LZ4_VERSION_MAJOR: c_int = 1;
pub const LZ4_VERSION_MINOR: c_int = 10;
pub const LZ4_VERSION_RELEASE: c_int = 0;
pub const LZ4_VERSION_NUMBER: c_int =
    LZ4_VERSION_MAJOR * 100 * 100 + LZ4_VERSION_MINOR * 100 + LZ4_VERSION_RELEASE;
static LZ4_VERSION_STRING_C: &[u8] = b"1.10.0\0";

pub const LZ4_ACCELERATION_DEFAULT: c_int = 1;
pub const LZ4_ACCELERATION_MAX: c_int = 65537;

pub const LZ4_64Klimit: c_int = (64 * KB + (MFLIMIT - 1)) as c_int;
pub const LZ4_skipTrigger: u32 = 6;

pub const HASH_UNIT: usize = 8; /* sizeof(reg_t) */

/* ===== allocation ===== */
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}
#[inline(always)]
pub unsafe fn ALLOC(s: usize) -> *mut c_void {
    unsafe { malloc(s) }
}
#[inline(always)]
pub unsafe fn ALLOC_AND_ZERO(s: usize) -> *mut c_void {
    unsafe { calloc(1, s) }
}
#[inline(always)]
pub unsafe fn FREEMEM(p: *mut c_void) {
    unsafe { free(p) }
}

#[inline]
pub fn LZ4_COMPRESSBOUND(isize_: c_int) -> c_int {
    if (isize_ as u32) > LZ4_MAX_INPUT_SIZE {
        0
    } else {
        isize_ + (isize_ / 255) + 16
    }
}

/* ===== Local Utils ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_versionNumber() -> c_int {
    LZ4_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_versionString() -> *const c_char {
    LZ4_VERSION_STRING_C.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_compressBound(isize_: c_int) -> c_int {
    LZ4_COMPRESSBOUND(isize_)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofState() -> c_int {
    core::mem::size_of::<LZ4_stream_t>() as c_int
}

/* ===== Hashing ===== */

#[inline(always)]
fn LZ4_hash4(sequence: u32, tableType: u32) -> u32 {
    if tableType == byU16 {
        sequence.wrapping_mul(2654435761u32) >> ((MINMATCH as u32 * 8) - (LZ4_HASHLOG + 1))
    } else {
        sequence.wrapping_mul(2654435761u32) >> ((MINMATCH as u32 * 8) - LZ4_HASHLOG)
    }
}

#[inline(always)]
fn LZ4_hash5(sequence: u64, tableType: u32) -> u32 {
    let hashLog = if tableType == byU16 {
        LZ4_HASHLOG + 1
    } else {
        LZ4_HASHLOG
    };
    let prime5bytes: u64 = 889523592379u64;
    ((sequence << 24).wrapping_mul(prime5bytes) >> (64 - hashLog)) as u32
}

#[inline(always)]
unsafe fn LZ4_hashPosition(p: *const u8, tableType: u32) -> u32 {
    unsafe {
        if tableType != byU16 {
            return LZ4_hash5(LZ4_read_ARCH(p), tableType);
        }
        LZ4_hash4(LZ4_read32(p), tableType)
    }
}

#[inline(always)]
unsafe fn LZ4_clearHash(h: u32, tableBase: *mut u32, tableType: u32) {
    unsafe {
        match tableType {
            byPtr => {
                let hashTable = tableBase as *mut *const u8;
                *hashTable.wrapping_add(h as usize) = core::ptr::null();
            }
            byU32 => {
                *tableBase.wrapping_add(h as usize) = 0;
            }
            byU16 => {
                let hashTable = tableBase as *mut u16;
                *hashTable.wrapping_add(h as usize) = 0;
            }
            _ => {}
        }
    }
}

#[inline(always)]
unsafe fn LZ4_putIndexOnHash(idx: u32, h: u32, tableBase: *mut u32, tableType: u32) {
    unsafe {
        match tableType {
            byU32 => {
                *tableBase.wrapping_add(h as usize) = idx;
            }
            byU16 => {
                let hashTable = tableBase as *mut u16;
                *hashTable.wrapping_add(h as usize) = idx as u16;
            }
            _ => {}
        }
    }
}

#[inline(always)]
unsafe fn LZ4_putPositionOnHash(p: *const u8, h: u32, tableBase: *mut u32, _tableType: u32) {
    unsafe {
        let hashTable = tableBase as *mut *const u8;
        *hashTable.wrapping_add(h as usize) = p;
    }
}

#[inline(always)]
unsafe fn LZ4_putPosition(p: *const u8, tableBase: *mut u32, tableType: u32) {
    unsafe {
        let h = LZ4_hashPosition(p, tableType);
        LZ4_putPositionOnHash(p, h, tableBase, tableType);
    }
}

#[inline(always)]
unsafe fn LZ4_getIndexOnHash(h: u32, tableBase: *const u32, tableType: u32) -> u32 {
    unsafe {
        if tableType == byU32 {
            return *tableBase.wrapping_add(h as usize);
        }
        if tableType == byU16 {
            let hashTable = tableBase as *const u16;
            return *hashTable.wrapping_add(h as usize) as u32;
        }
        0
    }
}

#[inline(always)]
unsafe fn LZ4_getPositionOnHash(h: u32, tableBase: *const u32, _tableType: u32) -> *const u8 {
    unsafe {
        let hashTable = tableBase as *const *const u8;
        *hashTable.wrapping_add(h as usize)
    }
}

#[inline(always)]
unsafe fn LZ4_getPosition(p: *const u8, tableBase: *const u32, tableType: u32) -> *const u8 {
    unsafe {
        let h = LZ4_hashPosition(p, tableType);
        LZ4_getPositionOnHash(h, tableBase, tableType)
    }
}

#[inline(always)]
unsafe fn LZ4_prepareTable(cctx: *mut LZ4_stream_t_internal, inputSize: c_int, tableType: u32) {
    unsafe {
        if (*cctx).tableType != clearedTable {
            if (*cctx).tableType != tableType
                || (tableType == byU16
                    && (*cctx).currentOffset.wrapping_add(inputSize as u32) >= 0xFFFFu32)
                || (tableType == byU32 && (*cctx).currentOffset > (1u32 << 30))
                || tableType == byPtr
                || inputSize >= (4 * KB) as c_int
            {
                MEM_INIT(
                    (*cctx).hashTable.as_mut_ptr() as *mut u8,
                    0,
                    LZ4_HASHTABLESIZE,
                );
                (*cctx).currentOffset = 0;
                (*cctx).tableType = clearedTable;
            }
        }

        if (*cctx).currentOffset != 0 && tableType == byU32 {
            (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(64 * KB as u32);
        }

        (*cctx).dictCtx = core::ptr::null();
        (*cctx).dictionary = core::ptr::null();
        (*cctx).dictSize = 0;
    }
}

/* ===== Compression ===== */

unsafe fn LZ4_compress_generic_validated(
    cctx: *mut LZ4_stream_t_internal,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    inputConsumed: *mut c_int,
    maxOutputSize: c_int,
    outputDirective: i32,
    tableType: u32,
    dictDirective: i32,
    dictIssue: i32,
    acceleration: c_int,
) -> c_int {
    unsafe {
        let result: c_int;
        let mut ip: *const u8 = source as *const u8;

        let startIndex: u32 = (*cctx).currentOffset;
        let base: *const u8 = csub(source as *const u8, startIndex as usize);
        let mut lowLimit: *const u8;

        let dictCtx: *const LZ4_stream_t_internal = (*cctx).dictCtx;
        let dictionary: *const u8 = if dictDirective == usingDictCtx {
            (*dictCtx).dictionary
        } else {
            (*cctx).dictionary
        };
        let dictSize: u32 = if dictDirective == usingDictCtx {
            (*dictCtx).dictSize
        } else {
            (*cctx).dictSize
        };
        let dictDelta: u32 = if dictDirective == usingDictCtx {
            startIndex.wrapping_sub((*dictCtx).currentOffset)
        } else {
            0
        };

        let maybe_extMem: bool =
            (dictDirective == usingExtDict) || (dictDirective == usingDictCtx);
        let prefixIdxLimit: u32 = startIndex.wrapping_sub(dictSize);
        let dictEnd: *const u8 = if !dictionary.is_null() {
            cadd(dictionary, dictSize as usize)
        } else {
            dictionary
        };
        let mut anchor: *const u8 = source as *const u8;
        let iend: *const u8 = cadd(ip, inputSize as usize);
        let mflimitPlusOne: *const u8 = csub(cadd(iend, 1), MFLIMIT);
        let matchlimit: *const u8 = csub(iend, LASTLITERALS);

        let dictBase: *const u8 = if dictionary.is_null() {
            core::ptr::null()
        } else if dictDirective == usingDictCtx {
            csub(
                cadd(dictionary, dictSize as usize),
                (*dictCtx).currentOffset as usize,
            )
        } else {
            csub(cadd(dictionary, dictSize as usize), startIndex as usize)
        };

        let mut op: *mut u8 = dest as *mut u8;
        let olimit: *mut u8 = madd(op, maxOutputSize as usize);

        let mut offset: u32 = 0;
        let mut forwardH: u32;

        if outputDirective == fillOutput && maxOutputSize < 1 {
            return 0;
        }

        lowLimit = csub(
            source as *const u8,
            if dictDirective == withPrefix64k {
                dictSize as usize
            } else {
                0
            },
        );

        /* Update context state */
        if dictDirective == usingDictCtx {
            (*cctx).dictCtx = core::ptr::null();
            (*cctx).dictSize = inputSize as u32;
        } else {
            (*cctx).dictSize = (*cctx).dictSize.wrapping_add(inputSize as u32);
        }
        (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(inputSize as u32);
        (*cctx).tableType = tableType;

        let mut r#match: *const u8 = core::ptr::null();
        let mut token: *mut u8 = core::ptr::null_mut();
        let mut filledIp: *const u8 = core::ptr::null();

        'lastlit: {
            if inputSize < LZ4_minLength {
                break 'lastlit;
            }

            /* First Byte */
            {
                let h = LZ4_hashPosition(ip, tableType);
                if tableType == byPtr {
                    LZ4_putPositionOnHash(ip, h, (*cctx).hashTable.as_mut_ptr(), byPtr);
                } else {
                    LZ4_putIndexOnHash(startIndex, h, (*cctx).hashTable.as_mut_ptr(), tableType);
                }
            }
            ip = cadd(ip, 1);
            forwardH = LZ4_hashPosition(ip, tableType);

            let mut at_next_match = false;
            'main: loop {
                if !at_next_match {
                    /* Find a match */
                    if tableType == byPtr {
                        let mut forwardIp = ip;
                        let mut step: c_int = 1;
                        let mut searchMatchNb: c_int = acceleration << LZ4_skipTrigger;
                        loop {
                            let h = forwardH;
                            ip = forwardIp;
                            forwardIp = coff(forwardIp, step as isize);
                            step = {
                                let s = searchMatchNb >> LZ4_skipTrigger;
                                searchMatchNb += 1;
                                s
                            };

                            if forwardIp > mflimitPlusOne {
                                break 'lastlit;
                            }

                            r#match = LZ4_getPositionOnHash(
                                h,
                                (*cctx).hashTable.as_ptr(),
                                tableType,
                            );
                            forwardH = LZ4_hashPosition(forwardIp, tableType);
                            LZ4_putPositionOnHash(
                                ip,
                                h,
                                (*cctx).hashTable.as_mut_ptr(),
                                tableType,
                            );

                            if !((cadd(r#match, LZ4_DISTANCE_MAX as usize) < ip)
                                || (LZ4_read32(r#match) != LZ4_read32(ip)))
                            {
                                break;
                            }
                        }
                    } else {
                        let mut forwardIp = ip;
                        let mut step: c_int = 1;
                        let mut searchMatchNb: c_int = acceleration << LZ4_skipTrigger;
                        loop {
                            let h = forwardH;
                            let current: u32 = pdiff(forwardIp, base) as u32;
                            let mut matchIndex =
                                LZ4_getIndexOnHash(h, (*cctx).hashTable.as_ptr(), tableType);
                            ip = forwardIp;
                            forwardIp = coff(forwardIp, step as isize);
                            step = {
                                let s = searchMatchNb >> LZ4_skipTrigger;
                                searchMatchNb += 1;
                                s
                            };

                            if forwardIp > mflimitPlusOne {
                                break 'lastlit;
                            }

                            if dictDirective == usingDictCtx {
                                if matchIndex < startIndex {
                                    matchIndex = LZ4_getIndexOnHash(
                                        h,
                                        (*dictCtx).hashTable.as_ptr(),
                                        byU32,
                                    );
                                    r#match = cadd(dictBase, matchIndex as usize);
                                    matchIndex = matchIndex.wrapping_add(dictDelta);
                                    lowLimit = dictionary;
                                } else {
                                    r#match = cadd(base, matchIndex as usize);
                                    lowLimit = source as *const u8;
                                }
                            } else if dictDirective == usingExtDict {
                                if matchIndex < startIndex {
                                    r#match = cadd(dictBase, matchIndex as usize);
                                    lowLimit = dictionary;
                                } else {
                                    r#match = cadd(base, matchIndex as usize);
                                    lowLimit = source as *const u8;
                                }
                            } else {
                                r#match = cadd(base, matchIndex as usize);
                            }
                            forwardH = LZ4_hashPosition(forwardIp, tableType);
                            LZ4_putIndexOnHash(
                                current,
                                h,
                                (*cctx).hashTable.as_mut_ptr(),
                                tableType,
                            );

                            if (dictIssue == dictSmall) && (matchIndex < prefixIdxLimit) {
                                continue;
                            }
                            if ((tableType != byU16)
                                || (LZ4_DISTANCE_MAX < LZ4_DISTANCE_ABSOLUTE_MAX))
                                && (matchIndex.wrapping_add(LZ4_DISTANCE_MAX) < current)
                            {
                                continue;
                            }

                            if LZ4_read32(r#match) == LZ4_read32(ip) {
                                if maybe_extMem {
                                    offset = current.wrapping_sub(matchIndex);
                                }
                                break;
                            }
                        }
                    }

                    /* Catch up */
                    filledIp = ip;
                    if (r#match > lowLimit) && (*csub(ip, 1) == *csub(r#match, 1)) {
                        loop {
                            ip = csub(ip, 1);
                            r#match = csub(r#match, 1);
                            if !(((ip > anchor) && (r#match > lowLimit))
                                && (*csub(ip, 1) == *csub(r#match, 1)))
                            {
                                break;
                            }
                        }
                    }

                    /* Encode Literals */
                    {
                        let litLength: u32 = pdiff(ip, anchor) as u32;
                        token = op;
                        op = madd(op, 1);
                        if (outputDirective == limitedOutput)
                            && (madd(
                                op,
                                litLength as usize + (2 + 1 + LASTLITERALS) + (litLength / 255) as usize,
                            ) > olimit)
                        {
                            return 0;
                        }
                        if (outputDirective == fillOutput)
                            && (madd(
                                op,
                                ((litLength + 240) / 255) as usize
                                    + litLength as usize
                                    + 2
                                    + 1
                                    + MFLIMIT
                                    - MINMATCH,
                            ) > olimit)
                        {
                            op = msub(op, 1);
                            break 'lastlit;
                        }
                        if litLength >= RUN_MASK {
                            let mut len = litLength - RUN_MASK;
                            *token = (RUN_MASK << ML_BITS) as u8;
                            while len >= 255 {
                                *op = 255;
                                op = madd(op, 1);
                                len -= 255;
                            }
                            *op = len as u8;
                            op = madd(op, 1);
                        } else {
                            *token = ((litLength << ML_BITS) as u8) as u8;
                        }

                        /* Copy Literals */
                        LZ4_wildCopy8(op, anchor, madd(op, litLength as usize));
                        op = madd(op, litLength as usize);
                    }
                }
                at_next_match = false;

                /* _next_match: */
                if (outputDirective == fillOutput)
                    && (madd(op, 2 + 1 + MFLIMIT - MINMATCH) > olimit)
                {
                    op = token;
                    break 'lastlit;
                }

                /* Encode Offset */
                if maybe_extMem {
                    LZ4_writeLE16(op, offset as u16);
                    op = madd(op, 2);
                } else {
                    LZ4_writeLE16(op, pdiff(ip, r#match) as u16);
                    op = madd(op, 2);
                }

                /* Encode MatchLength */
                {
                    let mut matchCode: u32;

                    if (dictDirective == usingExtDict || dictDirective == usingDictCtx)
                        && (lowLimit == dictionary)
                    {
                        let mut limit = coff(ip, pdiff(dictEnd, r#match));
                        if limit > matchlimit {
                            limit = matchlimit;
                        }
                        matchCode =
                            LZ4_count(cadd(ip, MINMATCH), cadd(r#match, MINMATCH), limit);
                        ip = cadd(ip, matchCode as usize + MINMATCH);
                        if ip == limit {
                            let more = LZ4_count(limit, source as *const u8, matchlimit);
                            matchCode += more;
                            ip = cadd(ip, more as usize);
                        }
                    } else {
                        matchCode =
                            LZ4_count(cadd(ip, MINMATCH), cadd(r#match, MINMATCH), matchlimit);
                        ip = cadd(ip, matchCode as usize + MINMATCH);
                    }

                    if (outputDirective != notLimited)
                        && (madd(op, (1 + LASTLITERALS) + ((matchCode + 240) / 255) as usize)
                            > olimit)
                    {
                        if outputDirective == fillOutput {
                            let newMatchCode: u32 = (15u32 - 1).wrapping_add(
                                (pdiff(olimit, op) as u32)
                                    .wrapping_sub(1)
                                    .wrapping_sub(LASTLITERALS as u32)
                                    .wrapping_mul(255),
                            );
                            ip = csub(ip, matchCode.wrapping_sub(newMatchCode) as usize);
                            matchCode = newMatchCode;
                            if ip <= filledIp {
                                let mut ptr = ip;
                                while ptr <= filledIp {
                                    let h = LZ4_hashPosition(ptr, tableType);
                                    LZ4_clearHash(h, (*cctx).hashTable.as_mut_ptr(), tableType);
                                    ptr = cadd(ptr, 1);
                                }
                            }
                        } else {
                            return 0;
                        }
                    }
                    if matchCode >= ML_MASK {
                        *token = (*token).wrapping_add(ML_MASK as u8);
                        matchCode -= ML_MASK;
                        LZ4_write32(op, 0xFFFFFFFF);
                        while matchCode >= 4 * 255 {
                            op = madd(op, 4);
                            LZ4_write32(op, 0xFFFFFFFF);
                            matchCode -= 4 * 255;
                        }
                        op = madd(op, (matchCode / 255) as usize);
                        *op = (matchCode % 255) as u8;
                        op = madd(op, 1);
                    } else {
                        *token = (*token).wrapping_add(matchCode as u8);
                    }
                }

                anchor = ip;

                /* Test end of chunk */
                if ip >= mflimitPlusOne {
                    break 'main;
                }

                /* Fill table */
                {
                    let h = LZ4_hashPosition(csub(ip, 2), tableType);
                    if tableType == byPtr {
                        LZ4_putPositionOnHash(
                            csub(ip, 2),
                            h,
                            (*cctx).hashTable.as_mut_ptr(),
                            byPtr,
                        );
                    } else {
                        let idx: u32 = pdiff(csub(ip, 2), base) as u32;
                        LZ4_putIndexOnHash(idx, h, (*cctx).hashTable.as_mut_ptr(), tableType);
                    }
                }

                /* Test next position */
                if tableType == byPtr {
                    r#match = LZ4_getPosition(ip, (*cctx).hashTable.as_ptr(), tableType);
                    LZ4_putPosition(ip, (*cctx).hashTable.as_mut_ptr(), tableType);
                    if (cadd(r#match, LZ4_DISTANCE_MAX as usize) >= ip)
                        && (LZ4_read32(r#match) == LZ4_read32(ip))
                    {
                        token = op;
                        op = madd(op, 1);
                        *token = 0;
                        at_next_match = true;
                        continue 'main;
                    }
                } else {
                    let h = LZ4_hashPosition(ip, tableType);
                    let current: u32 = pdiff(ip, base) as u32;
                    let mut matchIndex =
                        LZ4_getIndexOnHash(h, (*cctx).hashTable.as_ptr(), tableType);
                    if dictDirective == usingDictCtx {
                        if matchIndex < startIndex {
                            matchIndex =
                                LZ4_getIndexOnHash(h, (*dictCtx).hashTable.as_ptr(), byU32);
                            r#match = cadd(dictBase, matchIndex as usize);
                            lowLimit = dictionary;
                            matchIndex = matchIndex.wrapping_add(dictDelta);
                        } else {
                            r#match = cadd(base, matchIndex as usize);
                            lowLimit = source as *const u8;
                        }
                    } else if dictDirective == usingExtDict {
                        if matchIndex < startIndex {
                            r#match = cadd(dictBase, matchIndex as usize);
                            lowLimit = dictionary;
                        } else {
                            r#match = cadd(base, matchIndex as usize);
                            lowLimit = source as *const u8;
                        }
                    } else {
                        r#match = cadd(base, matchIndex as usize);
                    }
                    LZ4_putIndexOnHash(current, h, (*cctx).hashTable.as_mut_ptr(), tableType);
                    let cond1 = if dictIssue == dictSmall {
                        matchIndex >= prefixIdxLimit
                    } else {
                        true
                    };
                    let cond2 = if (tableType == byU16)
                        && (LZ4_DISTANCE_MAX == LZ4_DISTANCE_ABSOLUTE_MAX)
                    {
                        true
                    } else {
                        matchIndex.wrapping_add(LZ4_DISTANCE_MAX) >= current
                    };
                    if cond1 && cond2 && (LZ4_read32(r#match) == LZ4_read32(ip)) {
                        token = op;
                        op = madd(op, 1);
                        *token = 0;
                        if maybe_extMem {
                            offset = current.wrapping_sub(matchIndex);
                        }
                        at_next_match = true;
                        continue 'main;
                    }
                }

                /* Prepare next loop */
                ip = cadd(ip, 1);
                forwardH = LZ4_hashPosition(ip, tableType);
            }
        }

        /* _last_literals: */
        {
            let mut lastRun: usize = pdiff(iend, anchor) as usize;
            if (outputDirective != notLimited)
                && (madd(op, lastRun + 1 + ((lastRun + 255 - RUN_MASK as usize) / 255)) > olimit)
            {
                if outputDirective == fillOutput {
                    lastRun = (pdiff(olimit, op) as usize) - 1;
                    lastRun -= (lastRun + 256 - RUN_MASK as usize) / 256;
                } else {
                    return 0;
                }
            }
            if lastRun >= RUN_MASK as usize {
                let mut accumulator = lastRun - RUN_MASK as usize;
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
                *op = ((lastRun as u32) << ML_BITS) as u8;
                op = madd(op, 1);
            }
            LZ4_memcpy(op, anchor, lastRun);
            ip = cadd(anchor, lastRun);
            op = madd(op, lastRun);
        }

        if outputDirective == fillOutput {
            *inputConsumed = pdiff(ip, source as *const u8) as c_int;
        }
        result = pdiff(op as *const u8, dest as *const u8) as c_int;
        result
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn LZ4_compress_generic(
    cctx: *mut LZ4_stream_t_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    inputConsumed: *mut c_int,
    dstCapacity: c_int,
    outputDirective: i32,
    tableType: u32,
    dictDirective: i32,
    dictIssue: i32,
    acceleration: c_int,
) -> c_int {
    unsafe {
        if (srcSize as u32) > LZ4_MAX_INPUT_SIZE {
            return 0;
        }
        if srcSize == 0 {
            if outputDirective != notLimited && dstCapacity <= 0 {
                return 0;
            }
            *(dst as *mut u8) = 0;
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
    unsafe {
        let ctx = LZ4_initStream(state, core::mem::size_of::<LZ4_stream_t>());
        if acceleration < 1 {
            acceleration = LZ4_ACCELERATION_DEFAULT;
        }
        if acceleration > LZ4_ACCELERATION_MAX {
            acceleration = LZ4_ACCELERATION_MAX;
        }
        if maxOutputSize >= LZ4_compressBound(inputSize) {
            if inputSize < LZ4_64Klimit {
                LZ4_compress_generic(
                    ctx,
                    source,
                    dest,
                    inputSize,
                    core::ptr::null_mut(),
                    0,
                    notLimited,
                    byU16,
                    noDict,
                    noDictIssue,
                    acceleration,
                )
            } else {
                let tableType = byU32;
                LZ4_compress_generic(
                    ctx,
                    source,
                    dest,
                    inputSize,
                    core::ptr::null_mut(),
                    0,
                    notLimited,
                    tableType,
                    noDict,
                    noDictIssue,
                    acceleration,
                )
            }
        } else if inputSize < LZ4_64Klimit {
            LZ4_compress_generic(
                ctx,
                source,
                dest,
                inputSize,
                core::ptr::null_mut(),
                maxOutputSize,
                limitedOutput,
                byU16,
                noDict,
                noDictIssue,
                acceleration,
            )
        } else {
            let tableType = byU32;
            LZ4_compress_generic(
                ctx,
                source,
                dest,
                inputSize,
                core::ptr::null_mut(),
                maxOutputSize,
                limitedOutput,
                tableType,
                noDict,
                noDictIssue,
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
    srcSize: c_int,
    dstCapacity: c_int,
    mut acceleration: c_int,
) -> c_int {
    unsafe {
        let ctx = state as *mut LZ4_stream_t_internal;
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
                        ctx,
                        src,
                        dst,
                        srcSize,
                        core::ptr::null_mut(),
                        0,
                        notLimited,
                        tableType,
                        noDict,
                        dictSmall,
                        acceleration,
                    )
                } else {
                    LZ4_compress_generic(
                        ctx,
                        src,
                        dst,
                        srcSize,
                        core::ptr::null_mut(),
                        0,
                        notLimited,
                        tableType,
                        noDict,
                        noDictIssue,
                        acceleration,
                    )
                }
            } else {
                let tableType = byU32;
                LZ4_prepareTable(ctx, srcSize, tableType);
                LZ4_compress_generic(
                    ctx,
                    src,
                    dst,
                    srcSize,
                    core::ptr::null_mut(),
                    0,
                    notLimited,
                    tableType,
                    noDict,
                    noDictIssue,
                    acceleration,
                )
            }
        } else if srcSize < LZ4_64Klimit {
            let tableType = byU16;
            LZ4_prepareTable(ctx, srcSize, tableType);
            if (*ctx).currentOffset != 0 {
                LZ4_compress_generic(
                    ctx,
                    src,
                    dst,
                    srcSize,
                    core::ptr::null_mut(),
                    dstCapacity,
                    limitedOutput,
                    tableType,
                    noDict,
                    dictSmall,
                    acceleration,
                )
            } else {
                LZ4_compress_generic(
                    ctx,
                    src,
                    dst,
                    srcSize,
                    core::ptr::null_mut(),
                    dstCapacity,
                    limitedOutput,
                    tableType,
                    noDict,
                    noDictIssue,
                    acceleration,
                )
            }
        } else {
            let tableType = byU32;
            LZ4_prepareTable(ctx, srcSize, tableType);
            LZ4_compress_generic(
                ctx,
                src,
                dst,
                srcSize,
                core::ptr::null_mut(),
                dstCapacity,
                limitedOutput,
                tableType,
                noDict,
                noDictIssue,
                acceleration,
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
    unsafe {
        let mut ctx = core::mem::MaybeUninit::<LZ4_stream_t>::uninit();
        LZ4_compress_fast_extState(
            ctx.as_mut_ptr() as *mut c_void,
            src,
            dest,
            srcSize,
            dstCapacity,
            acceleration,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_default(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    unsafe { LZ4_compress_fast(src, dst, srcSize, dstCapacity, 1) }
}

unsafe fn LZ4_compress_destSize_extState_internal(
    state: *mut LZ4_stream_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
    acceleration: c_int,
) -> c_int {
    unsafe {
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
        } else if *srcSizePtr < LZ4_64Klimit {
            LZ4_compress_generic(
                state,
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
                state,
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
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_destSize(
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
) -> c_int {
    unsafe {
        let mut ctxBody = core::mem::MaybeUninit::<LZ4_stream_t>::uninit();
        LZ4_compress_destSize_extState_internal(
            ctxBody.as_mut_ptr(),
            src,
            dst,
            srcSizePtr,
            targetDstSize,
            1,
        )
    }
}

/* ===== Streaming ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStream() -> *mut LZ4_stream_t {
    unsafe {
        let lz4s = ALLOC(core::mem::size_of::<LZ4_stream_t>()) as *mut LZ4_stream_t;
        if lz4s.is_null() {
            return core::ptr::null_mut();
        }
        LZ4_initStream(lz4s as *mut c_void, core::mem::size_of::<LZ4_stream_t>());
        lz4s
    }
}

fn LZ4_stream_t_alignment() -> usize {
    core::mem::align_of::<LZ4_stream_t>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStream(buffer: *mut c_void, size: usize) -> *mut LZ4_stream_t {
    unsafe {
        if buffer.is_null() {
            return core::ptr::null_mut();
        }
        if size < core::mem::size_of::<LZ4_stream_t>() {
            return core::ptr::null_mut();
        }
        if LZ4_isAligned(buffer as *const u8, LZ4_stream_t_alignment()) == 0 {
            return core::ptr::null_mut();
        }
        MEM_INIT(
            buffer as *mut u8,
            0,
            core::mem::size_of::<LZ4_stream_t_internal>(),
        );
        buffer as *mut LZ4_stream_t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream(LZ4_stream: *mut LZ4_stream_t) {
    unsafe {
        MEM_INIT(
            LZ4_stream as *mut u8,
            0,
            core::mem::size_of::<LZ4_stream_t_internal>(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream_fast(ctx: *mut LZ4_stream_t) {
    unsafe {
        LZ4_prepareTable(ctx, 0, byU32);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStream(LZ4_stream: *mut LZ4_stream_t) -> c_int {
    unsafe {
        if LZ4_stream.is_null() {
            return 0;
        }
        FREEMEM(LZ4_stream as *mut c_void);
        0
    }
}

const _ld_fast: i32 = 0;
const _ld_slow: i32 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict_internal(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
    _ld: i32,
) -> c_int {
    unsafe {
        let dict: *mut LZ4_stream_t_internal = LZ4_dict;
        let tableType = byU32;
        let mut p: *const u8 = dictionary as *const u8;
        let dictEnd: *const u8 = cadd(p, dictSize as usize);
        let mut idx32: u32;

        LZ4_resetStream(LZ4_dict);

        (*dict).currentOffset = (*dict).currentOffset.wrapping_add(64 * KB as u32);

        if dictSize < HASH_UNIT as c_int {
            return 0;
        }

        if pdiff(dictEnd, p) > (64 * KB) as isize {
            p = csub(dictEnd, 64 * KB);
        }
        (*dict).dictionary = p;
        (*dict).dictSize = pdiff(dictEnd, p) as u32;
        (*dict).tableType = tableType;
        idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);

        while p <= csub(dictEnd, HASH_UNIT) {
            let h = LZ4_hashPosition(p, tableType);
            LZ4_putIndexOnHash(idx32, h, (*dict).hashTable.as_mut_ptr(), tableType);
            p = cadd(p, 3);
            idx32 = idx32.wrapping_add(3);
        }

        if _ld == _ld_slow {
            p = (*dict).dictionary;
            idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);
            while p <= csub(dictEnd, HASH_UNIT) {
                let h = LZ4_hashPosition(p, tableType);
                let limit = (*dict).currentOffset.wrapping_sub(64 * KB as u32);
                if LZ4_getIndexOnHash(h, (*dict).hashTable.as_ptr(), tableType) <= limit {
                    LZ4_putIndexOnHash(idx32, h, (*dict).hashTable.as_mut_ptr(), tableType);
                }
                p = cadd(p, 1);
                idx32 = idx32.wrapping_add(1);
            }
        }

        (*dict).dictSize as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    unsafe { LZ4_loadDict_internal(LZ4_dict, dictionary, dictSize, _ld_fast) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictSlow(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    unsafe { LZ4_loadDict_internal(LZ4_dict, dictionary, dictSize, _ld_slow) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_dictionary(
    workingStream: *mut LZ4_stream_t,
    dictionaryStream: *const LZ4_stream_t,
) {
    unsafe {
        let mut dictCtx: *const LZ4_stream_t_internal = if dictionaryStream.is_null() {
            core::ptr::null()
        } else {
            dictionaryStream
        };

        if !dictCtx.is_null() {
            if (*workingStream).currentOffset == 0 {
                (*workingStream).currentOffset = 64 * KB as u32;
            }
            if (*dictCtx).dictSize == 0 {
                dictCtx = core::ptr::null();
            }
        }
        (*workingStream).dictCtx = dictCtx;
    }
}

unsafe fn LZ4_renormDictT(LZ4_dict: *mut LZ4_stream_t_internal, nextSize: c_int) {
    unsafe {
        if (*LZ4_dict).currentOffset.wrapping_add(nextSize as u32) > 0x80000000u32 {
            let delta = (*LZ4_dict).currentOffset.wrapping_sub(64 * KB as u32);
            let dictEnd = cadd((*LZ4_dict).dictionary, (*LZ4_dict).dictSize as usize);
            for i in 0..LZ4_HASH_SIZE_U32 {
                if (*LZ4_dict).hashTable[i] < delta {
                    (*LZ4_dict).hashTable[i] = 0;
                } else {
                    (*LZ4_dict).hashTable[i] -= delta;
                }
            }
            (*LZ4_dict).currentOffset = 64 * KB as u32;
            if (*LZ4_dict).dictSize > 64 * KB as u32 {
                (*LZ4_dict).dictSize = 64 * KB as u32;
            }
            (*LZ4_dict).dictionary = csub(dictEnd, (*LZ4_dict).dictSize as usize);
        }
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
    unsafe {
        let tableType = byU32;
        let streamPtr: *mut LZ4_stream_t_internal = LZ4_stream;
        let mut dictEnd: *const c_char = if (*streamPtr).dictSize != 0 {
            cadd((*streamPtr).dictionary, (*streamPtr).dictSize as usize) as *const c_char
        } else {
            core::ptr::null()
        };

        LZ4_renormDictT(streamPtr, inputSize);
        if acceleration < 1 {
            acceleration = LZ4_ACCELERATION_DEFAULT;
        }
        if acceleration > LZ4_ACCELERATION_MAX {
            acceleration = LZ4_ACCELERATION_MAX;
        }

        /* invalidate tiny dictionaries */
        if ((*streamPtr).dictSize < 4)
            && (dictEnd != source)
            && (inputSize > 0)
            && (*streamPtr).dictCtx.is_null()
        {
            (*streamPtr).dictSize = 0;
            (*streamPtr).dictionary = source as *const u8;
            dictEnd = source;
        }

        /* Check overlapping input/dictionary space */
        {
            let sourceEnd: *const c_char = cadd(source as *const u8, inputSize as usize) as *const c_char;
            if (sourceEnd > (*streamPtr).dictionary as *const c_char) && (sourceEnd < dictEnd) {
                (*streamPtr).dictSize = pdiff(dictEnd as *const u8, sourceEnd as *const u8) as u32;
                if (*streamPtr).dictSize > 64 * KB as u32 {
                    (*streamPtr).dictSize = 64 * KB as u32;
                }
                if (*streamPtr).dictSize < 4 {
                    (*streamPtr).dictSize = 0;
                }
                (*streamPtr).dictionary =
                    csub(dictEnd as *const u8, (*streamPtr).dictSize as usize);
            }
        }

        /* prefix mode : source data follows dictionary */
        if dictEnd == source {
            if ((*streamPtr).dictSize < 64 * KB as u32)
                && ((*streamPtr).dictSize < (*streamPtr).currentOffset)
            {
                return LZ4_compress_generic(
                    streamPtr,
                    source,
                    dest,
                    inputSize,
                    core::ptr::null_mut(),
                    maxOutputSize,
                    limitedOutput,
                    tableType,
                    withPrefix64k,
                    dictSmall,
                    acceleration,
                );
            } else {
                return LZ4_compress_generic(
                    streamPtr,
                    source,
                    dest,
                    inputSize,
                    core::ptr::null_mut(),
                    maxOutputSize,
                    limitedOutput,
                    tableType,
                    withPrefix64k,
                    noDictIssue,
                    acceleration,
                );
            }
        }

        /* external dictionary mode */
        {
            let result: c_int;
            if !(*streamPtr).dictCtx.is_null() {
                if inputSize > (4 * KB) as c_int {
                    LZ4_memcpy(
                        streamPtr as *mut u8,
                        (*streamPtr).dictCtx as *const u8,
                        core::mem::size_of::<LZ4_stream_t_internal>(),
                    );
                    result = LZ4_compress_generic(
                        streamPtr,
                        source,
                        dest,
                        inputSize,
                        core::ptr::null_mut(),
                        maxOutputSize,
                        limitedOutput,
                        tableType,
                        usingExtDict,
                        noDictIssue,
                        acceleration,
                    );
                } else {
                    result = LZ4_compress_generic(
                        streamPtr,
                        source,
                        dest,
                        inputSize,
                        core::ptr::null_mut(),
                        maxOutputSize,
                        limitedOutput,
                        tableType,
                        usingDictCtx,
                        noDictIssue,
                        acceleration,
                    );
                }
            } else if ((*streamPtr).dictSize < 64 * KB as u32)
                && ((*streamPtr).dictSize < (*streamPtr).currentOffset)
            {
                result = LZ4_compress_generic(
                    streamPtr,
                    source,
                    dest,
                    inputSize,
                    core::ptr::null_mut(),
                    maxOutputSize,
                    limitedOutput,
                    tableType,
                    usingExtDict,
                    dictSmall,
                    acceleration,
                );
            } else {
                result = LZ4_compress_generic(
                    streamPtr,
                    source,
                    dest,
                    inputSize,
                    core::ptr::null_mut(),
                    maxOutputSize,
                    limitedOutput,
                    tableType,
                    usingExtDict,
                    noDictIssue,
                    acceleration,
                );
            }
            (*streamPtr).dictionary = source as *const u8;
            (*streamPtr).dictSize = inputSize as u32;
            result
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_forceExtDict(
    LZ4_dict: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    srcSize: c_int,
) -> c_int {
    unsafe {
        let streamPtr: *mut LZ4_stream_t_internal = LZ4_dict;
        let result: c_int;

        LZ4_renormDictT(streamPtr, srcSize);

        if ((*streamPtr).dictSize < 64 * KB as u32)
            && ((*streamPtr).dictSize < (*streamPtr).currentOffset)
        {
            result = LZ4_compress_generic(
                streamPtr,
                source,
                dest,
                srcSize,
                core::ptr::null_mut(),
                0,
                notLimited,
                byU32,
                usingExtDict,
                dictSmall,
                1,
            );
        } else {
            result = LZ4_compress_generic(
                streamPtr,
                source,
                dest,
                srcSize,
                core::ptr::null_mut(),
                0,
                notLimited,
                byU32,
                usingExtDict,
                noDictIssue,
                1,
            );
        }

        (*streamPtr).dictionary = source as *const u8;
        (*streamPtr).dictSize = srcSize as u32;

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDict(
    LZ4_dict: *mut LZ4_stream_t,
    safeBuffer: *mut c_char,
    mut dictSize: c_int,
) -> c_int {
    unsafe {
        let dict: *mut LZ4_stream_t_internal = LZ4_dict;

        if (dictSize as u32) > 64 * KB as u32 {
            dictSize = (64 * KB) as c_int;
        }
        if (dictSize as u32) > (*dict).dictSize {
            dictSize = (*dict).dictSize as c_int;
        }

        if dictSize > 0 {
            let previousDictEnd = cadd((*dict).dictionary, (*dict).dictSize as usize);
            LZ4_memmove(
                safeBuffer as *mut u8,
                csub(previousDictEnd, dictSize as usize),
                dictSize as usize,
            );
        }

        (*dict).dictionary = safeBuffer as *const u8;
        (*dict).dictSize = dictSize as u32;

        dictSize
    }
}

/* ===== Decompression ===== */

#[inline(always)]
fn MINi(a: c_int, b: c_int) -> c_int {
    if a < b { a } else { b }
}
#[inline(always)]
fn MINu(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

unsafe fn read_long_length_no_check(pp: *mut *const u8) -> usize {
    unsafe {
        let mut b: usize;
        let mut l: usize = 0;
        loop {
            b = **pp as usize;
            *pp = cadd(*pp, 1);
            l += b;
            if b != 255 {
                break;
            }
        }
        l
    }
}

unsafe fn LZ4_decompress_unsafe_generic(
    istart: *const u8,
    ostart: *mut u8,
    decompressedSize: c_int,
    prefixSize: usize,
    dictStart: *const u8,
    dictSize: usize,
) -> c_int {
    unsafe {
        let ip0 = istart;
        let mut ip = istart;
        let mut op: *mut u8 = ostart;
        let oend: *mut u8 = madd(ostart, decompressedSize as usize);
        let prefixStart: *const u8 = msub(ostart, prefixSize);

        loop {
            /* start new sequence */
            let token = *ip as u32;
            ip = cadd(ip, 1);

            /* literals */
            {
                let mut ll: usize = (token >> ML_BITS) as usize;
                if ll == 15 {
                    ll += read_long_length_no_check(&mut ip as *mut *const u8);
                }
                if (pdiff(oend as *const u8, op as *const u8) as usize) < ll {
                    return -1;
                }
                LZ4_memmove(op, ip, ll);
                op = madd(op, ll);
                ip = cadd(ip, ll);
                if (pdiff(oend as *const u8, op as *const u8) as usize) < MFLIMIT {
                    if op == oend {
                        break;
                    }
                    return -1;
                }
            }

            /* match */
            {
                let mut ml: usize = (token & 15) as usize;
                let offset: usize = LZ4_readLE16(ip) as usize;
                ip = cadd(ip, 2);

                if ml == 15 {
                    ml += read_long_length_no_check(&mut ip as *mut *const u8);
                }
                ml += MINMATCH;

                if (pdiff(oend as *const u8, op as *const u8) as usize) < ml {
                    return -1;
                }

                {
                    let mut r#match: *const u8 = msub(op, offset);

                    if offset > (pdiff(op as *const u8, prefixStart) as usize) + dictSize {
                        return -1;
                    }

                    if offset > (pdiff(op as *const u8, prefixStart) as usize) {
                        let dictEnd = cadd(dictStart, dictSize);
                        let extMatch = csub(
                            dictEnd,
                            offset - (pdiff(op as *const u8, prefixStart) as usize),
                        );
                        let extml = pdiff(dictEnd, extMatch) as usize;
                        if extml > ml {
                            LZ4_memmove(op, extMatch, ml);
                            op = madd(op, ml);
                            ml = 0;
                        } else {
                            LZ4_memmove(op, extMatch, extml);
                            op = madd(op, extml);
                            ml -= extml;
                        }
                        r#match = prefixStart;
                    }

                    let mut u = 0usize;
                    while u < ml {
                        *op.wrapping_add(u) = *r#match.wrapping_add(u);
                        u += 1;
                    }
                }
                op = madd(op, ml);
                if (pdiff(oend as *const u8, op as *const u8) as usize) < LASTLITERALS {
                    return -1;
                }
            }
        }
        pdiff(ip, ip0) as c_int
    }
}

const rvl_error: usize = usize::MAX;

#[inline(always)]
unsafe fn read_variable_length(
    ip: *mut *const u8,
    ilimit: *const u8,
    initial_check: i32,
) -> usize {
    unsafe {
        let mut s: usize;
        let mut length: usize = 0;
        if initial_check != 0 && (*ip >= ilimit) {
            return rvl_error;
        }
        s = **ip as usize;
        *ip = cadd(*ip, 1);
        length += s;
        if *ip > ilimit {
            return rvl_error;
        }
        if s != 255 {
            return length;
        }
        loop {
            s = **ip as usize;
            *ip = cadd(*ip, 1);
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
}

const ST_TOP: u32 = 0;
const ST_LITERAL_COPY: u32 = 1;
const ST_COPY_MATCH: u32 = 2;
const ST_MATCH_COPY: u32 = 3;

unsafe fn LZ4_decompress_generic(
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    outputSize: c_int,
    partialDecoding: i32,
    dict: i32,
    lowPrefix: *const u8,
    dictStart: *const u8,
    dictSize: usize,
) -> c_int {
    unsafe {
        if src.is_null() || outputSize < 0 {
            return -1;
        }

        let mut ip: *const u8 = src as *const u8;
        let iend: *const u8 = cadd(ip, srcSize as usize);

        let mut op: *mut u8 = dst as *mut u8;
        let oend: *mut u8 = madd(op, outputSize as usize);
        let mut cpy: *mut u8;

        let dictEnd: *const u8 = if dictStart.is_null() {
            core::ptr::null()
        } else {
            cadd(dictStart, dictSize)
        };

        let checkOffset: bool = dictSize < (64 * KB);

        let shortiend: *const u8 = csub(iend, 14 + 2);
        let shortoend: *mut u8 = msub(oend, 14 + 18);

        let mut r#match: *const u8 = core::ptr::null();
        let mut offset: usize = 0;
        let mut token: u32 = 0;
        let mut length: usize = 0;

        macro_rules! output_error {
            () => {
                return (-(pdiff(ip, src as *const u8) as c_int)) - 1;
            };
        }

        /* Special cases */
        if outputSize == 0 {
            if partialDecoding != 0 {
                return 0;
            }
            return if (srcSize == 1) && (*ip == 0) { 0 } else { -1 };
        }
        if srcSize == 0 {
            return -1;
        }

        let mut state: u32 = ST_TOP;

        /* ---- Fast decode loop ---- */
        if pdiff(oend as *const u8, op as *const u8) >= FASTLOOP_SAFE_DISTANCE {
            'fast: loop {
                token = *ip as u32;
                ip = cadd(ip, 1);
                length = (token >> ML_BITS) as usize;

                /* decode literal length */
                if length == RUN_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip as *mut *const u8,
                        csub(iend, RUN_MASK as usize),
                        1,
                    );
                    if addl == rvl_error {
                        output_error!();
                    }
                    length += addl;
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                    if (ip as usize).wrapping_add(length) < (ip as usize) {
                        output_error!();
                    }

                    if (madd(op, length) > msub(oend, 32)) || (cadd(ip, length) > csub(iend, 32)) {
                        state = ST_LITERAL_COPY;
                        break 'fast;
                    }
                    LZ4_wildCopy32(op, ip, madd(op, length));
                    ip = cadd(ip, length);
                    op = madd(op, length);
                } else if ip <= csub(iend, 16 + 1) {
                    LZ4_memcpy(op, ip, 16);
                    ip = cadd(ip, length);
                    op = madd(op, length);
                } else {
                    state = ST_LITERAL_COPY;
                    break 'fast;
                }

                /* get offset */
                offset = LZ4_readLE16(ip) as usize;
                ip = cadd(ip, 2);
                r#match = msub(op, offset);

                /* get matchlength */
                length = (token & ML_MASK) as usize;

                if length == ML_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip as *mut *const u8,
                        cadd(csub(iend, LASTLITERALS), 1),
                        0,
                    );
                    if addl == rvl_error {
                        output_error!();
                    }
                    length += addl;
                    length += MINMATCH;
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                    if madd(op, length) >= moff(oend, -FASTLOOP_SAFE_DISTANCE) {
                        state = ST_MATCH_COPY;
                        break 'fast;
                    }
                } else {
                    length += MINMATCH;
                    if madd(op, length) >= moff(oend, -FASTLOOP_SAFE_DISTANCE) {
                        state = ST_MATCH_COPY;
                        break 'fast;
                    }

                    if (dict == withPrefix64k) || (r#match >= lowPrefix) {
                        if offset >= 8 {
                            LZ4_memcpy(op, r#match, 8);
                            LZ4_memcpy(madd(op, 8), cadd(r#match, 8), 8);
                            LZ4_memcpy(madd(op, 16), cadd(r#match, 16), 2);
                            op = madd(op, length);
                            continue 'fast;
                        }
                    }
                }

                if checkOffset && (cadd(r#match, dictSize) < lowPrefix) {
                    output_error!();
                }
                /* match starting within external dictionary */
                if (dict == usingExtDict) && (r#match < lowPrefix) {
                    if madd(op, length) > msub(oend, LASTLITERALS) {
                        if partialDecoding != 0 {
                            length = MINu(length, pdiff(oend as *const u8, op as *const u8) as usize);
                        } else {
                            output_error!();
                        }
                    }

                    if length <= (pdiff(lowPrefix, r#match) as usize) {
                        LZ4_memmove(op, csub(dictEnd, pdiff(lowPrefix, r#match) as usize), length);
                        op = madd(op, length);
                    } else {
                        let copySize = pdiff(lowPrefix, r#match) as usize;
                        let restSize = length - copySize;
                        LZ4_memcpy(op, csub(dictEnd, copySize), copySize);
                        op = madd(op, copySize);
                        if restSize > (pdiff(op as *const u8, lowPrefix) as usize) {
                            let endOfMatch = madd(op, restSize);
                            let mut copyFrom = lowPrefix;
                            while op < endOfMatch {
                                *op = *copyFrom;
                                op = madd(op, 1);
                                copyFrom = cadd(copyFrom, 1);
                            }
                        } else {
                            LZ4_memcpy(op, lowPrefix, restSize);
                            op = madd(op, restSize);
                        }
                    }
                    continue 'fast;
                }

                /* copy match within block */
                cpy = madd(op, length);

                if offset < 16 {
                    LZ4_memcpy_using_offset(op, r#match, cpy, offset);
                } else {
                    LZ4_wildCopy32(op, r#match, cpy);
                }

                op = cpy;
            }
        }

        /* ---- Safe decode loop ---- */
        'safe: loop {
            if state == ST_TOP {
                token = *ip as u32;
                ip = cadd(ip, 1);
                length = (token >> ML_BITS) as usize;

                let mut jumped_to_copy_match = false;
                if (length != RUN_MASK as usize) && ((ip < shortiend) && (op <= shortoend)) {
                    /* Copy the literals */
                    LZ4_memcpy(op, ip, 16);
                    op = madd(op, length);
                    ip = cadd(ip, length);

                    length = (token & ML_MASK) as usize;
                    offset = LZ4_readLE16(ip) as usize;
                    ip = cadd(ip, 2);
                    r#match = msub(op, offset);

                    if (length != ML_MASK as usize)
                        && (offset >= 8)
                        && (dict == withPrefix64k || r#match >= lowPrefix)
                    {
                        LZ4_memcpy(madd(op, 0), cadd(r#match, 0), 8);
                        LZ4_memcpy(madd(op, 8), cadd(r#match, 8), 8);
                        LZ4_memcpy(madd(op, 16), cadd(r#match, 16), 2);
                        op = madd(op, length + MINMATCH);
                        continue 'safe;
                    }

                    jumped_to_copy_match = true;
                }

                if jumped_to_copy_match {
                    state = ST_COPY_MATCH;
                } else {
                    /* decode literal length */
                    if length == RUN_MASK as usize {
                        let addl = read_variable_length(
                            &mut ip as *mut *const u8,
                            csub(iend, RUN_MASK as usize),
                            1,
                        );
                        if addl == rvl_error {
                            output_error!();
                        }
                        length += addl;
                        if (op as usize).wrapping_add(length) < (op as usize) {
                            output_error!();
                        }
                        if (ip as usize).wrapping_add(length) < (ip as usize) {
                            output_error!();
                        }
                    }
                    state = ST_LITERAL_COPY;
                }
            }

            if state == ST_LITERAL_COPY {
                /* safe_literal_copy: */
                cpy = madd(op, length);

                if (cpy > msub(oend, MFLIMIT))
                    || (cadd(ip, length) > csub(iend, 2 + 1 + LASTLITERALS))
                {
                    if partialDecoding != 0 {
                        if cadd(ip, length) > iend {
                            length = pdiff(iend, ip) as usize;
                            cpy = madd(op, length);
                        }
                        if cpy > oend {
                            cpy = oend;
                            length = pdiff(oend as *const u8, op as *const u8) as usize;
                        }
                    } else if (cadd(ip, length) != iend) || (cpy > oend) {
                        output_error!();
                    }
                    LZ4_memmove(op, ip, length);
                    ip = cadd(ip, length);
                    op = madd(op, length);
                    if partialDecoding == 0 || (cpy == oend) || (ip >= csub(iend, 2)) {
                        break 'safe;
                    }
                } else {
                    LZ4_wildCopy8(op, ip, cpy);
                    ip = cadd(ip, length);
                    op = cpy;
                }

                /* get offset */
                offset = LZ4_readLE16(ip) as usize;
                ip = cadd(ip, 2);
                r#match = msub(op, offset);

                /* get matchlength */
                length = (token & ML_MASK) as usize;

                state = ST_COPY_MATCH;
            }

            if state == ST_COPY_MATCH {
                /* _copy_match: */
                if length == ML_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip as *mut *const u8,
                        cadd(csub(iend, LASTLITERALS), 1),
                        0,
                    );
                    if addl == rvl_error {
                        output_error!();
                    }
                    length += addl;
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                }
                length += MINMATCH;
                state = ST_MATCH_COPY;
            }

            /* safe_match_copy: */
            {
                if checkOffset && (cadd(r#match, dictSize) < lowPrefix) {
                    output_error!();
                }
                /* match starting within external dictionary */
                if (dict == usingExtDict) && (r#match < lowPrefix) {
                    if madd(op, length) > msub(oend, LASTLITERALS) {
                        if partialDecoding != 0 {
                            length =
                                MINu(length, pdiff(oend as *const u8, op as *const u8) as usize);
                        } else {
                            output_error!();
                        }
                    }

                    if length <= (pdiff(lowPrefix, r#match) as usize) {
                        LZ4_memmove(
                            op,
                            csub(dictEnd, pdiff(lowPrefix, r#match) as usize),
                            length,
                        );
                        op = madd(op, length);
                    } else {
                        let copySize = pdiff(lowPrefix, r#match) as usize;
                        let restSize = length - copySize;
                        LZ4_memcpy(op, csub(dictEnd, copySize), copySize);
                        op = madd(op, copySize);
                        if restSize > (pdiff(op as *const u8, lowPrefix) as usize) {
                            let endOfMatch = madd(op, restSize);
                            let mut copyFrom = lowPrefix;
                            while op < endOfMatch {
                                *op = *copyFrom;
                                op = madd(op, 1);
                                copyFrom = cadd(copyFrom, 1);
                            }
                        } else {
                            LZ4_memcpy(op, lowPrefix, restSize);
                            op = madd(op, restSize);
                        }
                    }
                    state = ST_TOP;
                    continue 'safe;
                }

                /* copy match within block */
                cpy = madd(op, length);

                if partialDecoding != 0 && (cpy > msub(oend, MATCH_SAFEGUARD_DISTANCE)) {
                    let mlen = MINu(length, pdiff(oend as *const u8, op as *const u8) as usize);
                    let matchEnd = cadd(r#match, mlen);
                    let copyEnd = madd(op, mlen);
                    if matchEnd > op as *const u8 {
                        while op < copyEnd {
                            *op = *r#match;
                            op = madd(op, 1);
                            r#match = cadd(r#match, 1);
                        }
                    } else {
                        LZ4_memcpy(op, r#match, mlen);
                    }
                    op = copyEnd;
                    if op == oend {
                        break 'safe;
                    }
                    state = ST_TOP;
                    continue 'safe;
                }

                if offset < 8 {
                    LZ4_write32(op, 0);
                    *op.wrapping_add(0) = *r#match.wrapping_add(0);
                    *op.wrapping_add(1) = *r#match.wrapping_add(1);
                    *op.wrapping_add(2) = *r#match.wrapping_add(2);
                    *op.wrapping_add(3) = *r#match.wrapping_add(3);
                    r#match = cadd(r#match, inc32table[offset] as usize);
                    LZ4_memcpy(madd(op, 4), r#match, 4);
                    r#match = coff(r#match, -(dec64table[offset] as isize));
                } else {
                    LZ4_memcpy(op, r#match, 8);
                    r#match = cadd(r#match, 8);
                }
                op = madd(op, 8);

                if cpy > msub(oend, MATCH_SAFEGUARD_DISTANCE) {
                    let oCopyLimit = msub(oend, WILDCOPYLENGTH - 1);
                    if cpy > msub(oend, LASTLITERALS) {
                        output_error!();
                    }
                    if op < oCopyLimit {
                        LZ4_wildCopy8(op, r#match, oCopyLimit);
                        r#match = coff(r#match, pdiff(oCopyLimit as *const u8, op as *const u8));
                        op = oCopyLimit;
                    }
                    while op < cpy {
                        *op = *r#match;
                        op = madd(op, 1);
                        r#match = cadd(r#match, 1);
                    }
                } else {
                    LZ4_memcpy(op, r#match, 8);
                    if length > 16 {
                        LZ4_wildCopy8(madd(op, 8), cadd(r#match, 8), cpy);
                    }
                }
                op = cpy;
            }
            state = ST_TOP;
        }

        pdiff(op as *const u8, dst as *const u8) as c_int
    }
}

/* ===== Instantiate the API decoding functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxDecompressedSize: c_int,
) -> c_int {
    unsafe {
        LZ4_decompress_generic(
            source,
            dest,
            compressedSize,
            maxDecompressedSize,
            decode_full_block,
            noDict,
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
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    unsafe {
        let dstCapacity = MINi(targetOutputSize, dstCapacity);
        LZ4_decompress_generic(
            src,
            dst,
            compressedSize,
            dstCapacity,
            partial_decode,
            noDict,
            dst as *const u8,
            core::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
) -> c_int {
    unsafe {
        LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            originalSize,
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
    compressedSize: c_int,
    maxOutputSize: c_int,
) -> c_int {
    unsafe {
        LZ4_decompress_generic(
            source,
            dest,
            compressedSize,
            maxOutputSize,
            decode_full_block,
            withPrefix64k,
            csub(dest as *const u8, 64 * KB),
            core::ptr::null(),
            0,
        )
    }
}

unsafe fn LZ4_decompress_safe_partial_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    unsafe {
        let dstCapacity = MINi(targetOutputSize, dstCapacity);
        LZ4_decompress_generic(
            source,
            dest,
            compressedSize,
            dstCapacity,
            partial_decode,
            withPrefix64k,
            csub(dest as *const u8, 64 * KB),
            core::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
) -> c_int {
    unsafe {
        LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            originalSize,
            64 * KB,
            core::ptr::null(),
            0,
        )
    }
}

unsafe fn LZ4_decompress_safe_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    maxOutputSize: c_int,
    prefixSize: usize,
) -> c_int {
    unsafe {
        LZ4_decompress_generic(
            source,
            dest,
            compressedSize,
            maxOutputSize,
            decode_full_block,
            noDict,
            csub(dest as *const u8, prefixSize),
            core::ptr::null(),
            0,
        )
    }
}

unsafe fn LZ4_decompress_safe_partial_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
    prefixSize: usize,
) -> c_int {
    unsafe {
        let dstCapacity = MINi(targetOutputSize, dstCapacity);
        LZ4_decompress_generic(
            source,
            dest,
            compressedSize,
            dstCapacity,
            partial_decode,
            noDict,
            csub(dest as *const u8, prefixSize),
            core::ptr::null(),
            0,
        )
    }
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
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial_forceExtDict(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    unsafe {
        let dstCapacity = MINi(targetOutputSize, dstCapacity);
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
}

unsafe fn LZ4_decompress_fast_extDict(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    unsafe {
        LZ4_decompress_unsafe_generic(
            source as *const u8,
            dest as *mut u8,
            originalSize,
            0,
            dictStart as *const u8,
            dictSize,
        )
    }
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
    unsafe {
        LZ4_decompress_generic(
            source,
            dest,
            compressedSize,
            maxOutputSize,
            decode_full_block,
            usingExtDict,
            csub(dest as *const u8, prefixSize),
            dictStart as *const u8,
            dictSize,
        )
    }
}

/* ===== streaming decompression ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamDecode() -> *mut LZ4_streamDecode_t {
    unsafe { ALLOC_AND_ZERO(core::mem::size_of::<LZ4_streamDecode_t>()) as *mut LZ4_streamDecode_t }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamDecode(LZ4_stream: *mut LZ4_streamDecode_t) -> c_int {
    unsafe {
        if LZ4_stream.is_null() {
            return 0;
        }
        FREEMEM(LZ4_stream as *mut c_void);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setStreamDecode(
    LZ4_streamDecode: *mut LZ4_streamDecode_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    unsafe {
        let lz4sd: *mut LZ4_streamDecode_t_internal = LZ4_streamDecode;
        (*lz4sd).prefixSize = dictSize as usize;
        if dictSize != 0 {
            (*lz4sd).prefixEnd = cadd(dictionary as *const u8, dictSize as usize);
        } else {
            (*lz4sd).prefixEnd = dictionary as *const u8;
        }
        (*lz4sd).externalDict = core::ptr::null();
        (*lz4sd).extDictSize = 0;
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_decoderRingBufferSize(mut maxBlockSize: c_int) -> c_int {
    if maxBlockSize < 0 {
        return 0;
    }
    if maxBlockSize > LZ4_MAX_INPUT_SIZE as c_int {
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
    unsafe {
        let lz4sd: *mut LZ4_streamDecode_t_internal = LZ4_streamDecode;
        let result: c_int;

        if (*lz4sd).prefixSize == 0 {
            result = LZ4_decompress_safe(source, dest, compressedSize, maxOutputSize);
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefixSize = result as usize;
            (*lz4sd).prefixEnd = cadd(dest as *const u8, result as usize);
        } else if (*lz4sd).prefixEnd == dest as *const u8 {
            if (*lz4sd).prefixSize >= 64 * KB - 1 {
                result =
                    LZ4_decompress_safe_withPrefix64k(source, dest, compressedSize, maxOutputSize);
            } else if (*lz4sd).extDictSize == 0 {
                result = LZ4_decompress_safe_withSmallPrefix(
                    source,
                    dest,
                    compressedSize,
                    maxOutputSize,
                    (*lz4sd).prefixSize,
                );
            } else {
                result = LZ4_decompress_safe_doubleDict(
                    source,
                    dest,
                    compressedSize,
                    maxOutputSize,
                    (*lz4sd).prefixSize,
                    (*lz4sd).externalDict as *const c_void,
                    (*lz4sd).extDictSize,
                );
            }
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefixSize += result as usize;
            (*lz4sd).prefixEnd = cadd((*lz4sd).prefixEnd, result as usize);
        } else {
            (*lz4sd).extDictSize = (*lz4sd).prefixSize;
            (*lz4sd).externalDict = csub((*lz4sd).prefixEnd, (*lz4sd).extDictSize);
            result = LZ4_decompress_safe_forceExtDict(
                source,
                dest,
                compressedSize,
                maxOutputSize,
                (*lz4sd).externalDict as *const c_void,
                (*lz4sd).extDictSize,
            );
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefixSize = result as usize;
            (*lz4sd).prefixEnd = cadd(dest as *const u8, result as usize);
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_continue(
    LZ4_streamDecode: *mut LZ4_streamDecode_t,
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
) -> c_int {
    unsafe {
        let lz4sd: *mut LZ4_streamDecode_t_internal = LZ4_streamDecode;
        let result: c_int;

        if (*lz4sd).prefixSize == 0 {
            result = LZ4_decompress_fast(source, dest, originalSize);
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefixSize = originalSize as usize;
            (*lz4sd).prefixEnd = cadd(dest as *const u8, originalSize as usize);
        } else if (*lz4sd).prefixEnd == dest as *const u8 {
            result = LZ4_decompress_unsafe_generic(
                source as *const u8,
                dest as *mut u8,
                originalSize,
                (*lz4sd).prefixSize,
                (*lz4sd).externalDict,
                (*lz4sd).extDictSize,
            );
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefixSize += originalSize as usize;
            (*lz4sd).prefixEnd = cadd((*lz4sd).prefixEnd, originalSize as usize);
        } else {
            (*lz4sd).extDictSize = (*lz4sd).prefixSize;
            (*lz4sd).externalDict = csub((*lz4sd).prefixEnd, (*lz4sd).extDictSize);
            result = LZ4_decompress_fast_extDict(
                source,
                dest,
                originalSize,
                (*lz4sd).externalDict as *const c_void,
                (*lz4sd).extDictSize,
            );
            if result <= 0 {
                return result;
            }
            (*lz4sd).prefixSize = originalSize as usize;
            (*lz4sd).prefixEnd = cadd(dest as *const u8, originalSize as usize);
        }

        result
    }
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
    unsafe {
        if dictSize == 0 {
            return LZ4_decompress_safe(source, dest, compressedSize, maxOutputSize);
        }
        if cadd(dictStart as *const u8, dictSize as usize) == dest as *const u8 {
            if dictSize >= (64 * KB) as c_int - 1 {
                return LZ4_decompress_safe_withPrefix64k(
                    source,
                    dest,
                    compressedSize,
                    maxOutputSize,
                );
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
    unsafe {
        if dictSize == 0 {
            return LZ4_decompress_safe_partial(
                source,
                dest,
                compressedSize,
                targetOutputSize,
                dstCapacity,
            );
        }
        if cadd(dictStart as *const u8, dictSize as usize) == dest as *const u8 {
            if dictSize >= (64 * KB) as c_int - 1 {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_fast_usingDict(
    source: *const c_char,
    dest: *mut c_char,
    originalSize: c_int,
    dictStart: *const c_char,
    dictSize: c_int,
) -> c_int {
    unsafe {
        if dictSize == 0 || cadd(dictStart as *const u8, dictSize as usize) == dest as *const u8 {
            return LZ4_decompress_unsafe_generic(
                source as *const u8,
                dest as *mut u8,
                originalSize,
                dictSize as usize,
                core::ptr::null(),
                0,
            );
        }
        LZ4_decompress_fast_extDict(
            source,
            dest,
            originalSize,
            dictStart as *const c_void,
            dictSize as usize,
        )
    }
}

/* ===== Obsolete Functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput(
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    maxOutputSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_default(source, dest, inputSize, maxOutputSize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress(
    src: *const c_char,
    dest: *mut c_char,
    srcSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_default(src, dest, srcSize, LZ4_compressBound(srcSize)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstSize: c_int,
) -> c_int {
    unsafe { LZ4_compress_fast_extState(state, src, dst, srcSize, dstSize, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_withState(
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_fast_extState(state, src, dst, srcSize, LZ4_compressBound(srcSize), 1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_limitedOutput_continue(
    LZ4_stream: *mut LZ4_stream_t,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    unsafe { LZ4_compress_fast_continue(LZ4_stream, src, dst, srcSize, dstCapacity, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_continue(
    LZ4_stream: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
) -> c_int {
    unsafe {
        LZ4_compress_fast_continue(
            LZ4_stream,
            source,
            dest,
            inputSize,
            LZ4_compressBound(inputSize),
            1,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress(
    source: *const c_char,
    dest: *mut c_char,
    outputSize: c_int,
) -> c_int {
    unsafe { LZ4_decompress_fast(source, dest, outputSize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_uncompress_unknownOutputSize(
    source: *const c_char,
    dest: *mut c_char,
    isize_: c_int,
    maxOutputSize: c_int,
) -> c_int {
    unsafe { LZ4_decompress_safe(source, dest, isize_, maxOutputSize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_sizeofStreamState() -> c_int {
    core::mem::size_of::<LZ4_stream_t>() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamState(
    state: *mut c_void,
    _inputBuffer: *mut c_char,
) -> c_int {
    unsafe {
        LZ4_resetStream(state as *mut LZ4_stream_t);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_create(_inputBuffer: *mut c_char) -> *mut c_void {
    unsafe { LZ4_createStream() as *mut c_void }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char {
    unsafe { (*(state as *mut LZ4_stream_t)).dictionary as *mut c_char }
}
