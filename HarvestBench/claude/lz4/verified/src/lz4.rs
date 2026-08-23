//! Translation of lz4.c
//!
//! Target assumptions (matching the C build on x86_64 Linux/gcc):
//!  - little endian, `sizeof(void*) == 8`, `reg_t == U64`, `STEPSIZE == 8`
//!  - `LZ4_FAST_DEC_LOOP == 1`
//!  - `LZ4_HEAPMODE == 0`

use crate::common::*;
use core::ffi::{c_char, c_int, c_void};

pub const LZ4_ACCELERATION_DEFAULT: c_int = 1;
pub const LZ4_ACCELERATION_MAX: c_int = 65537;

const LZ4_64Klimit: c_int = (64 * 1024) + (MFLIMIT as c_int - 1);
const LZ4_skipTrigger: u32 = 6;

/* ================================================================ *
 *  Local Utils
 * ================================================================ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_versionNumber() -> c_int {
    LZ4_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_versionString() -> *const c_char {
    LZ4_VERSION_STRING.as_ptr() as *const c_char
}

#[inline]
pub fn lz4_compress_bound(isize_: c_int) -> c_int {
    if (isize_ as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        0
    } else {
        isize_ + (isize_ / 255) + 16
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressBound(isize_: c_int) -> c_int {
    lz4_compress_bound(isize_)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_sizeofState() -> c_int {
    SIZEOF_LZ4_STREAM_T as c_int
}

/* ================================================================ *
 *  Compression functions
 * ================================================================ */

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
    let prime5bytes: u64 = 889523592379;
    (((sequence << 24).wrapping_mul(prime5bytes)) >> (64 - hashLog)) as u32
}

#[inline(always)]
unsafe fn LZ4_hashPosition(p: *const u8, tableType: u32) -> u32 {
    if tableType != byU16 {
        return LZ4_hash5(LZ4_read_ARCH(p), tableType);
    }
    LZ4_hash4(LZ4_read32(p), tableType)
}

#[inline(always)]
unsafe fn LZ4_clearHash(h: u32, tableBase: *mut u8, tableType: u32) {
    match tableType {
        x if x == byPtr => {
            let hashTable = tableBase as *mut *const u8;
            *hashTable.wrapping_add(h as usize) = core::ptr::null();
        }
        x if x == byU32 => {
            let hashTable = tableBase as *mut u32;
            *hashTable.wrapping_add(h as usize) = 0;
        }
        x if x == byU16 => {
            let hashTable = tableBase as *mut u16;
            *hashTable.wrapping_add(h as usize) = 0;
        }
        _ => {}
    }
}

#[inline(always)]
unsafe fn LZ4_putIndexOnHash(idx: u32, h: u32, tableBase: *mut u8, tableType: u32) {
    match tableType {
        x if x == byU32 => {
            let hashTable = tableBase as *mut u32;
            *hashTable.wrapping_add(h as usize) = idx;
        }
        x if x == byU16 => {
            let hashTable = tableBase as *mut u16;
            *hashTable.wrapping_add(h as usize) = idx as u16;
        }
        _ => {}
    }
}

#[inline(always)]
unsafe fn LZ4_putPositionOnHash(p: *const u8, h: u32, tableBase: *mut u8, _tableType: u32) {
    let hashTable = tableBase as *mut *const u8;
    *hashTable.wrapping_add(h as usize) = p;
}

#[inline(always)]
unsafe fn LZ4_putPosition(p: *const u8, tableBase: *mut u8, tableType: u32) {
    let h = LZ4_hashPosition(p, tableType);
    LZ4_putPositionOnHash(p, h, tableBase, tableType);
}

#[inline(always)]
unsafe fn LZ4_getIndexOnHash(h: u32, tableBase: *const u8, tableType: u32) -> u32 {
    if tableType == byU32 {
        let hashTable = tableBase as *const u32;
        return *hashTable.wrapping_add(h as usize);
    }
    if tableType == byU16 {
        let hashTable = tableBase as *const u16;
        return *hashTable.wrapping_add(h as usize) as u32;
    }
    0
}

#[inline(always)]
unsafe fn LZ4_getPositionOnHash(h: u32, tableBase: *const u8, _tableType: u32) -> *const u8 {
    let hashTable = tableBase as *const *const u8;
    *hashTable.wrapping_add(h as usize)
}

#[inline(always)]
unsafe fn LZ4_getPosition(p: *const u8, tableBase: *const u8, tableType: u32) -> *const u8 {
    let h = LZ4_hashPosition(p, tableType);
    LZ4_getPositionOnHash(h, tableBase, tableType)
}

#[inline(always)]
unsafe fn LZ4_prepareTable(
    cctx: *mut LZ4_stream_t_internal,
    inputSize: c_int,
    tableType: u32,
) {
    if (*cctx).tableType != clearedTable {
        if (*cctx).tableType != tableType
            || ((tableType == byU16)
                && (*cctx).currentOffset.wrapping_add(inputSize as u32) >= 0xFFFFu32)
            || ((tableType == byU32) && (*cctx).currentOffset > (1u32 << 30))
            || tableType == byPtr
            || inputSize >= (4 * 1024)
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
        (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(64 * 1024);
    }

    /* Finally, clear history */
    (*cctx).dictCtx = core::ptr::null();
    (*cctx).dictionary = core::ptr::null();
    (*cctx).dictSize = 0;
}

/// LZ4_compress_generic_validated()
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
    let result: c_int;
    let mut ip: *const u8 = source as *const u8;

    let startIndex: u32 = (*cctx).currentOffset;
    let base: *const u8 = (source as *const u8).wrapping_sub(startIndex as usize);
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

    let maybe_extMem: bool = (dictDirective == usingExtDict) || (dictDirective == usingDictCtx);
    let prefixIdxLimit: u32 = startIndex.wrapping_sub(dictSize);
    let dictEnd: *const u8 = if !dictionary.is_null() {
        dictionary.wrapping_add(dictSize as usize)
    } else {
        dictionary
    };
    let mut anchor: *const u8 = source as *const u8;
    let iend: *const u8 = ip.wrapping_add(inputSize as usize);
    let mflimitPlusOne: *const u8 = iend.wrapping_sub(MFLIMIT).wrapping_add(1);
    let matchlimit: *const u8 = iend.wrapping_sub(LASTLITERALS);

    let dictBase: *const u8 = if dictionary.is_null() {
        core::ptr::null()
    } else if dictDirective == usingDictCtx {
        dictionary
            .wrapping_add(dictSize as usize)
            .wrapping_sub((*dictCtx).currentOffset as usize)
    } else {
        dictionary
            .wrapping_add(dictSize as usize)
            .wrapping_sub(startIndex as usize)
    };

    let mut op: *mut u8 = dest as *mut u8;
    let olimit: *mut u8 = op.wrapping_add(maxOutputSize as usize);

    let mut offset: u32 = 0;
    let mut forwardH: u32;

    if outputDirective == fillOutput && maxOutputSize < 1 {
        return 0;
    }

    lowLimit = (source as *const u8).wrapping_sub(if dictDirective == withPrefix64k {
        dictSize as usize
    } else {
        0
    });

    /* Update context state */
    if dictDirective == usingDictCtx {
        (*cctx).dictCtx = core::ptr::null();
        (*cctx).dictSize = inputSize as u32;
    } else {
        (*cctx).dictSize = (*cctx).dictSize.wrapping_add(inputSize as u32);
    }
    (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(inputSize as u32);
    (*cctx).tableType = tableType;

    let hashTable: *mut u8 = (*cctx).hashTable.as_mut_ptr() as *mut u8;

    let mut token: *mut u8 = core::ptr::null_mut();
    let mut filledIp: *const u8 = core::ptr::null();
    let mut r#match: *const u8 = core::ptr::null();

    'last_literals: {
        if inputSize < LZ4_minLength {
            break 'last_literals;
        }

        /* First Byte */
        {
            let h = LZ4_hashPosition(ip, tableType);
            if tableType == byPtr {
                LZ4_putPositionOnHash(ip, h, hashTable, byPtr);
            } else {
                LZ4_putIndexOnHash(startIndex, h, hashTable, tableType);
            }
        }
        ip = ip.wrapping_add(1);
        forwardH = LZ4_hashPosition(ip, tableType);

        /* Main Loop */
        let mut goto_next_match = false;
        loop {
            if !goto_next_match {
                /* Find a match */
                if tableType == byPtr {
                    let mut forwardIp = ip;
                    let mut step: i32 = 1;
                    let mut searchMatchNb: i32 = acceleration << LZ4_skipTrigger;
                    loop {
                        let h = forwardH;
                        ip = forwardIp;
                        forwardIp = forwardIp.wrapping_add(step as usize);
                        step = searchMatchNb >> LZ4_skipTrigger;
                        searchMatchNb += 1;

                        if forwardIp > mflimitPlusOne {
                            break 'last_literals;
                        }

                        r#match = LZ4_getPositionOnHash(h, hashTable, tableType);
                        forwardH = LZ4_hashPosition(forwardIp, tableType);
                        LZ4_putPositionOnHash(ip, h, hashTable, tableType);

                        if !((r#match.wrapping_add(LZ4_DISTANCE_MAX as usize) < ip)
                            || (LZ4_read32(r#match) != LZ4_read32(ip)))
                        {
                            break;
                        }
                    }
                } else {
                    /* byU32, byU16 */
                    let mut forwardIp = ip;
                    let mut step: i32 = 1;
                    let mut searchMatchNb: i32 = acceleration << LZ4_skipTrigger;
                    loop {
                        let h = forwardH;
                        let current: u32 = pdiff(forwardIp, base) as u32;
                        let mut matchIndex: u32 = LZ4_getIndexOnHash(h, hashTable, tableType);
                        ip = forwardIp;
                        forwardIp = forwardIp.wrapping_add(step as usize);
                        step = searchMatchNb >> LZ4_skipTrigger;
                        searchMatchNb += 1;

                        if forwardIp > mflimitPlusOne {
                            break 'last_literals;
                        }

                        if dictDirective == usingDictCtx {
                            if matchIndex < startIndex {
                                /* there was no match, try the dictionary */
                                matchIndex = LZ4_getIndexOnHash(
                                    h,
                                    (*dictCtx).hashTable.as_ptr() as *const u8,
                                    byU32,
                                );
                                r#match = dictBase.wrapping_add(matchIndex as usize);
                                matchIndex = matchIndex.wrapping_add(dictDelta);
                                lowLimit = dictionary;
                            } else {
                                r#match = base.wrapping_add(matchIndex as usize);
                                lowLimit = source as *const u8;
                            }
                        } else if dictDirective == usingExtDict {
                            if matchIndex < startIndex {
                                r#match = dictBase.wrapping_add(matchIndex as usize);
                                lowLimit = dictionary;
                            } else {
                                r#match = base.wrapping_add(matchIndex as usize);
                                lowLimit = source as *const u8;
                            }
                        } else {
                            /* single continuous memory segment */
                            r#match = base.wrapping_add(matchIndex as usize);
                        }
                        forwardH = LZ4_hashPosition(forwardIp, tableType);
                        LZ4_putIndexOnHash(current, h, hashTable, tableType);

                        if (dictIssue == dictSmall) && (matchIndex < prefixIdxLimit) {
                            continue; /* match outside of valid area */
                        }
                        if ((tableType != byU16) || (LZ4_DISTANCE_MAX < LZ4_DISTANCE_ABSOLUTE_MAX))
                            && (matchIndex.wrapping_add(LZ4_DISTANCE_MAX) < current)
                        {
                            continue; /* too far */
                        }

                        if LZ4_read32(r#match) == LZ4_read32(ip) {
                            if maybe_extMem {
                                offset = current.wrapping_sub(matchIndex);
                            }
                            break; /* match found */
                        }
                    }
                }

                /* Catch up */
                filledIp = ip;
                if (r#match > lowLimit) && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1)) {
                    loop {
                        ip = ip.wrapping_sub(1);
                        r#match = r#match.wrapping_sub(1);
                        if !(((ip > anchor) & (r#match > lowLimit))
                            && (*ip.wrapping_sub(1) == *r#match.wrapping_sub(1)))
                        {
                            break;
                        }
                    }
                }

                /* Encode Literals */
                {
                    let litLength: u32 = pdiff(ip, anchor) as u32;
                    token = op;
                    op = op.wrapping_add(1);
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
                        op = op.wrapping_sub(1);
                        break 'last_literals;
                    }
                    if litLength >= RUN_MASK {
                        let mut len = litLength - RUN_MASK;
                        *token = (RUN_MASK << ML_BITS) as u8;
                        while len >= 255 {
                            *op = 255;
                            op = op.wrapping_add(1);
                            len -= 255;
                        }
                        *op = len as u8;
                        op = op.wrapping_add(1);
                    } else {
                        *token = ((litLength << ML_BITS) & 0xFF) as u8;
                    }

                    /* Copy Literals */
                    LZ4_wildCopy8(op, anchor, op.wrapping_add(litLength as usize));
                    op = op.wrapping_add(litLength as usize);
                }
            }
            goto_next_match = false;

            /* _next_match: */
            if (outputDirective == fillOutput)
                && (op
                    .wrapping_add(2)
                    .wrapping_add(1)
                    .wrapping_add(MFLIMIT - MINMATCH)
                    > olimit)
            {
                /* the match was too close to the end, rewind and go to last literals */
                op = token;
                break 'last_literals;
            }

            /* Encode Offset */
            if maybe_extMem {
                LZ4_writeLE16(op, offset as u16);
                op = op.wrapping_add(2);
            } else {
                LZ4_writeLE16(op, pdiff(ip, r#match) as u16);
                op = op.wrapping_add(2);
            }

            /* Encode MatchLength */
            {
                let mut matchCode: u32;

                if (dictDirective == usingExtDict || dictDirective == usingDictCtx)
                    && (lowLimit == dictionary)
                {
                    let mut limit = ip.wrapping_add(pdiff(dictEnd, r#match));
                    if limit > matchlimit {
                        limit = matchlimit;
                    }
                    matchCode = LZ4_count(
                        ip.wrapping_add(MINMATCH),
                        r#match.wrapping_add(MINMATCH),
                        limit,
                    );
                    ip = ip.wrapping_add(matchCode as usize + MINMATCH);
                    if ip == limit {
                        let more = LZ4_count(limit, source as *const u8, matchlimit);
                        matchCode = matchCode.wrapping_add(more);
                        ip = ip.wrapping_add(more as usize);
                    }
                } else {
                    matchCode = LZ4_count(
                        ip.wrapping_add(MINMATCH),
                        r#match.wrapping_add(MINMATCH),
                        matchlimit,
                    );
                    ip = ip.wrapping_add(matchCode as usize + MINMATCH);
                }

                if (outputDirective != notLimited)
                    && (op
                        .wrapping_add(1 + LASTLITERALS)
                        .wrapping_add(((matchCode + 240) / 255) as usize)
                        > olimit)
                {
                    if outputDirective == fillOutput {
                        /* Match description too long : reduce it */
                        let newMatchCode: u32 = 15u32.wrapping_sub(1).wrapping_add(
                            (pdiff(olimit, op) as u32)
                                .wrapping_sub(1)
                                .wrapping_sub(LASTLITERALS as u32)
                                .wrapping_mul(255),
                        );
                        ip = ip.wrapping_sub((matchCode - newMatchCode) as usize);
                        matchCode = newMatchCode;
                        if ip <= filledIp {
                            let mut ptr = ip;
                            while ptr <= filledIp {
                                let h = LZ4_hashPosition(ptr, tableType);
                                LZ4_clearHash(h, hashTable, tableType);
                                ptr = ptr.wrapping_add(1);
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
                        op = op.wrapping_add(4);
                        LZ4_write32(op, 0xFFFFFFFF);
                        matchCode -= 4 * 255;
                    }
                    op = op.wrapping_add((matchCode / 255) as usize);
                    *op = (matchCode % 255) as u8;
                    op = op.wrapping_add(1);
                } else {
                    *token = (*token).wrapping_add(matchCode as u8);
                }
            }

            anchor = ip;

            /* Test end of chunk */
            if ip >= mflimitPlusOne {
                break;
            }

            /* Fill table */
            {
                let h = LZ4_hashPosition(ip.wrapping_sub(2), tableType);
                if tableType == byPtr {
                    LZ4_putPositionOnHash(ip.wrapping_sub(2), h, hashTable, byPtr);
                } else {
                    let idx: u32 = pdiff(ip.wrapping_sub(2), base) as u32;
                    LZ4_putIndexOnHash(idx, h, hashTable, tableType);
                }
            }

            /* Test next position */
            if tableType == byPtr {
                r#match = LZ4_getPosition(ip, hashTable, tableType);
                LZ4_putPosition(ip, hashTable, tableType);
                if (r#match.wrapping_add(LZ4_DISTANCE_MAX as usize) >= ip)
                    && (LZ4_read32(r#match) == LZ4_read32(ip))
                {
                    token = op;
                    op = op.wrapping_add(1);
                    *token = 0;
                    goto_next_match = true;
                    continue;
                }
            } else {
                /* byU32, byU16 */
                let h = LZ4_hashPosition(ip, tableType);
                let current: u32 = pdiff(ip, base) as u32;
                let mut matchIndex: u32 = LZ4_getIndexOnHash(h, hashTable, tableType);
                if dictDirective == usingDictCtx {
                    if matchIndex < startIndex {
                        matchIndex = LZ4_getIndexOnHash(
                            h,
                            (*dictCtx).hashTable.as_ptr() as *const u8,
                            byU32,
                        );
                        r#match = dictBase.wrapping_add(matchIndex as usize);
                        lowLimit = dictionary;
                        matchIndex = matchIndex.wrapping_add(dictDelta);
                    } else {
                        r#match = base.wrapping_add(matchIndex as usize);
                        lowLimit = source as *const u8;
                    }
                } else if dictDirective == usingExtDict {
                    if matchIndex < startIndex {
                        r#match = dictBase.wrapping_add(matchIndex as usize);
                        lowLimit = dictionary;
                    } else {
                        r#match = base.wrapping_add(matchIndex as usize);
                        lowLimit = source as *const u8;
                    }
                } else {
                    r#match = base.wrapping_add(matchIndex as usize);
                }
                LZ4_putIndexOnHash(current, h, hashTable, tableType);
                if (if dictIssue == dictSmall {
                    matchIndex >= prefixIdxLimit
                } else {
                    true
                })
                    && (if (tableType == byU16)
                        && (LZ4_DISTANCE_MAX == LZ4_DISTANCE_ABSOLUTE_MAX)
                    {
                        true
                    } else {
                        matchIndex.wrapping_add(LZ4_DISTANCE_MAX) >= current
                    })
                    && (LZ4_read32(r#match) == LZ4_read32(ip))
                {
                    token = op;
                    op = op.wrapping_add(1);
                    *token = 0;
                    if maybe_extMem {
                        offset = current.wrapping_sub(matchIndex);
                    }
                    goto_next_match = true;
                    continue;
                }
            }

            /* Prepare next loop */
            ip = ip.wrapping_add(1);
            forwardH = LZ4_hashPosition(ip, tableType);
        }
    }

    /* _last_literals: Encode Last Literals */
    {
        let mut lastRun: usize = pdiff(iend, anchor);
        if (outputDirective != notLimited)
            && (op
                .wrapping_add(lastRun)
                .wrapping_add(1)
                .wrapping_add((lastRun + 255 - RUN_MASK as usize) / 255)
                > olimit)
        {
            if outputDirective == fillOutput {
                /* adapt lastRun to fill 'dst' */
                lastRun = pdiff(olimit, op) - 1 /*token*/;
                lastRun -= (lastRun + 256 - RUN_MASK as usize) / 256;
            } else {
                return 0;
            }
        }
        if lastRun >= RUN_MASK as usize {
            let mut accumulator = lastRun - RUN_MASK as usize;
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
            *op = ((lastRun << ML_BITS) & 0xFF) as u8;
            op = op.wrapping_add(1);
        }
        LZ4_memcpy(op, anchor, lastRun);
        ip = anchor.wrapping_add(lastRun);
        op = op.wrapping_add(lastRun);
    }

    if outputDirective == fillOutput {
        *inputConsumed = pdiff(ip as *const c_char, source) as c_int;
    }
    result = pdiff(op as *const c_char, dest) as c_int;
    result
}

/// LZ4_compress_generic() : takes care of src == (NULL, 0)
#[inline(always)]
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
    let ctx = LZ4_initStream(state, SIZEOF_LZ4_STREAM_T) as *mut LZ4_stream_t_internal;
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }
    if maxOutputSize >= lz4_compress_bound(inputSize) {
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
    } else {
        if inputSize < LZ4_64Klimit {
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
    let ctx = state as *mut LZ4_stream_t_internal;
    if acceleration < 1 {
        acceleration = LZ4_ACCELERATION_DEFAULT;
    }
    if acceleration > LZ4_ACCELERATION_MAX {
        acceleration = LZ4_ACCELERATION_MAX;
    }

    if dstCapacity >= lz4_compress_bound(srcSize) {
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
    } else {
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
    /* LZ4_HEAPMODE == 0 : state on stack */
    let mut ctx = LZ4_stream_u {
        minStateSize: [0u8; SIZEOF_LZ4_STREAM_T],
    };
    LZ4_compress_fast_extState(
        ctx.minStateSize.as_mut_ptr() as *mut c_void,
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
    state: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
    acceleration: c_int,
) -> c_int {
    let _s = LZ4_initStream(state, SIZEOF_LZ4_STREAM_T);

    if targetDstSize >= lz4_compress_bound(*srcSizePtr) {
        return LZ4_compress_fast_extState(
            state,
            src,
            dst,
            *srcSizePtr,
            targetDstSize,
            acceleration,
        );
    } else {
        let ctx = state as *mut LZ4_stream_t_internal;
        if *srcSizePtr < LZ4_64Klimit {
            LZ4_compress_generic(
                ctx,
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
                ctx,
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
        state,
        src,
        dst,
        srcSizePtr,
        targetDstSize,
        acceleration,
    );
    /* clean the state on exit */
    LZ4_initStream(state, SIZEOF_LZ4_STREAM_T);
    r
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_destSize(
    src: *const c_char,
    dst: *mut c_char,
    srcSizePtr: *mut c_int,
    targetDstSize: c_int,
) -> c_int {
    let mut ctxBody = LZ4_stream_u {
        minStateSize: [0u8; SIZEOF_LZ4_STREAM_T],
    };
    LZ4_compress_destSize_extState_internal(
        ctxBody.minStateSize.as_mut_ptr() as *mut c_void,
        src,
        dst,
        srcSizePtr,
        targetDstSize,
        1,
    )
}

/* ================================================================ *
 *  Streaming functions
 * ================================================================ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStream() -> *mut LZ4_stream_t {
    let lz4s = malloc(SIZEOF_LZ4_STREAM_T) as *mut LZ4_stream_t;
    if lz4s.is_null() {
        return core::ptr::null_mut();
    }
    LZ4_initStream(lz4s as *mut c_void, SIZEOF_LZ4_STREAM_T);
    lz4s
}

fn LZ4_stream_t_alignment() -> usize {
    /* LZ4_ALIGN_TEST == 1 : alignof(LZ4_stream_t) */
    8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_initStream(buffer: *mut c_void, size: usize) -> *mut LZ4_stream_t {
    if buffer.is_null() {
        return core::ptr::null_mut();
    }
    if size < SIZEOF_LZ4_STREAM_T {
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
    LZ4_prepareTable(ctx as *mut LZ4_stream_t_internal, 0, byU32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStream(LZ4_stream: *mut LZ4_stream_t) -> c_int {
    if LZ4_stream.is_null() {
        return 0;
    }
    free(LZ4_stream as *mut u8);
    0
}

/* LoadDict_mode_e */
const _ld_fast: c_int = 0;
const _ld_slow: c_int = 1;
const HASH_UNIT: usize = 8; /* sizeof(reg_t) */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDict_internal(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
    _ld: c_int,
) -> c_int {
    let dict = LZ4_dict as *mut LZ4_stream_t_internal;
    let tableType = byU32;
    let mut p: *const u8 = dictionary as *const u8;
    let dictEnd: *const u8 = p.wrapping_add(dictSize as usize);
    let mut idx32: u32;

    LZ4_resetStream(LZ4_dict);

    (*dict).currentOffset = (*dict).currentOffset.wrapping_add(64 * 1024);

    if dictSize < HASH_UNIT as c_int {
        return 0;
    }

    if pdiff_i(dictEnd, p) > (64 * 1024) {
        p = dictEnd.wrapping_sub(64 * 1024);
    }
    (*dict).dictionary = p;
    (*dict).dictSize = pdiff(dictEnd, p) as u32;
    (*dict).tableType = tableType;
    idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);

    let hashTable = (*dict).hashTable.as_mut_ptr() as *mut u8;

    while p <= dictEnd.wrapping_sub(HASH_UNIT) {
        let h = LZ4_hashPosition(p, tableType);
        LZ4_putIndexOnHash(idx32, h, hashTable, tableType);
        p = p.wrapping_add(3);
        idx32 = idx32.wrapping_add(3);
    }

    if _ld == _ld_slow {
        p = (*dict).dictionary;
        idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);
        while p <= dictEnd.wrapping_sub(HASH_UNIT) {
            let h = LZ4_hashPosition(p, tableType);
            let limit = (*dict).currentOffset.wrapping_sub(64 * 1024);
            if LZ4_getIndexOnHash(h, hashTable as *const u8, tableType) <= limit {
                LZ4_putIndexOnHash(idx32, h, hashTable, tableType);
            }
            p = p.wrapping_add(1);
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
    LZ4_loadDict_internal(LZ4_dict, dictionary, dictSize, _ld_fast)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_loadDictSlow(
    LZ4_dict: *mut LZ4_stream_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    LZ4_loadDict_internal(LZ4_dict, dictionary, dictSize, _ld_slow)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_attach_dictionary(
    workingStream: *mut LZ4_stream_t,
    dictionaryStream: *const LZ4_stream_t,
) {
    let mut dictCtx: *const LZ4_stream_t_internal = if dictionaryStream.is_null() {
        core::ptr::null()
    } else {
        dictionaryStream as *const LZ4_stream_t_internal
    };

    if !dictCtx.is_null() {
        if (*(workingStream as *mut LZ4_stream_t_internal)).currentOffset == 0 {
            (*(workingStream as *mut LZ4_stream_t_internal)).currentOffset = 64 * 1024;
        }
        if (*dictCtx).dictSize == 0 {
            dictCtx = core::ptr::null();
        }
    }
    (*(workingStream as *mut LZ4_stream_t_internal)).dictCtx = dictCtx;
}

unsafe fn LZ4_renormDictT(LZ4_dict: *mut LZ4_stream_t_internal, nextSize: c_int) {
    if (*LZ4_dict).currentOffset.wrapping_add(nextSize as u32) > 0x80000000u32 {
        /* rescale hash table */
        let delta: u32 = (*LZ4_dict).currentOffset.wrapping_sub(64 * 1024);
        let dictEnd: *const u8 = (*LZ4_dict)
            .dictionary
            .wrapping_add((*LZ4_dict).dictSize as usize);
        for i in 0..LZ4_HASH_SIZE_U32 {
            if (*LZ4_dict).hashTable[i] < delta {
                (*LZ4_dict).hashTable[i] = 0;
            } else {
                (*LZ4_dict).hashTable[i] -= delta;
            }
        }
        (*LZ4_dict).currentOffset = 64 * 1024;
        if (*LZ4_dict).dictSize > 64 * 1024 {
            (*LZ4_dict).dictSize = 64 * 1024;
        }
        (*LZ4_dict).dictionary = dictEnd.wrapping_sub((*LZ4_dict).dictSize as usize);
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
    let streamPtr = LZ4_stream as *mut LZ4_stream_t_internal;
    let mut dictEnd: *const c_char = if (*streamPtr).dictSize != 0 {
        ((*streamPtr).dictionary as *const c_char).wrapping_add((*streamPtr).dictSize as usize)
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
        && ((*streamPtr).dictCtx.is_null())
    {
        (*streamPtr).dictSize = 0;
        (*streamPtr).dictionary = source as *const u8;
        dictEnd = source;
    }

    /* Check overlapping input/dictionary space */
    {
        let sourceEnd: *const c_char = source.wrapping_add(inputSize as usize);
        if (sourceEnd > (*streamPtr).dictionary as *const c_char) && (sourceEnd < dictEnd) {
            (*streamPtr).dictSize = pdiff(dictEnd, sourceEnd) as u32;
            if (*streamPtr).dictSize > 64 * 1024 {
                (*streamPtr).dictSize = 64 * 1024;
            }
            if (*streamPtr).dictSize < 4 {
                (*streamPtr).dictSize = 0;
            }
            (*streamPtr).dictionary =
                (dictEnd as *const u8).wrapping_sub((*streamPtr).dictSize as usize);
        }
    }

    /* prefix mode : source data follows dictionary */
    if dictEnd == source {
        if ((*streamPtr).dictSize < 64 * 1024) && ((*streamPtr).dictSize < (*streamPtr).currentOffset)
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
            if inputSize > 4 * 1024 {
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
        } else {
            if ((*streamPtr).dictSize < 64 * 1024)
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
        }
        (*streamPtr).dictionary = source as *const u8;
        (*streamPtr).dictSize = inputSize as u32;
        result
    }
}

/* Hidden debug function, to force-test external dictionary mode */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compress_forceExtDict(
    LZ4_dict: *mut LZ4_stream_t,
    source: *const c_char,
    dest: *mut c_char,
    srcSize: c_int,
) -> c_int {
    let streamPtr = LZ4_dict as *mut LZ4_stream_t_internal;
    let result: c_int;

    LZ4_renormDictT(streamPtr, srcSize);

    if ((*streamPtr).dictSize < 64 * 1024) && ((*streamPtr).dictSize < (*streamPtr).currentOffset) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_saveDict(
    LZ4_dict: *mut LZ4_stream_t,
    safeBuffer: *mut c_char,
    mut dictSize: c_int,
) -> c_int {
    let dict = LZ4_dict as *mut LZ4_stream_t_internal;

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

/* ================================================================ *
 *  Decompression functions
 * ================================================================ */

#[inline(always)]
fn MIN_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
fn MIN_int(a: c_int, b: c_int) -> c_int {
    if a < b {
        a
    } else {
        b
    }
}

/* variant for decompress_unsafe() */
unsafe fn read_long_length_no_check(pp: &mut *const u8) -> usize {
    let mut b: usize;
    let mut l: usize = 0;
    loop {
        b = **pp as usize;
        *pp = (*pp).wrapping_add(1);
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
        /* start new sequence */
        let token: u32 = *ip as u32;
        ip = ip.wrapping_add(1);

        /* literals */
        {
            let mut ll: usize = (token >> ML_BITS) as usize;
            if ll == 15 {
                ll += read_long_length_no_check(&mut ip);
            }
            if pdiff(oend, op) < ll {
                return -1;
            }
            LZ4_memmove(op, ip, ll);
            op = op.wrapping_add(ll);
            ip = ip.wrapping_add(ll);
            if pdiff(oend, op) < MFLIMIT {
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
            ip = ip.wrapping_add(2);

            if ml == 15 {
                ml += read_long_length_no_check(&mut ip);
            }
            ml += MINMATCH;

            if pdiff(oend, op) < ml {
                return -1;
            }

            {
                let mut r#match: *const u8 = op.wrapping_sub(offset);

                /* out of range */
                if offset > pdiff(op, prefixStart) + dictSize {
                    return -1;
                }

                /* check special case : extDict */
                if offset > pdiff(op, prefixStart) {
                    let dictEnd = dictStart.wrapping_add(dictSize);
                    let extMatch = dictEnd.wrapping_sub(offset - pdiff(op, prefixStart));
                    let extml: usize = pdiff(dictEnd, extMatch);
                    if extml > ml {
                        LZ4_memmove(op, extMatch, ml);
                        op = op.wrapping_add(ml);
                        ml = 0;
                    } else {
                        LZ4_memmove(op, extMatch, extml);
                        op = op.wrapping_add(extml);
                        ml -= extml;
                    }
                    r#match = prefixStart;
                }

                /* match copy - slow variant, supporting overlap copy */
                {
                    let mut u: usize = 0;
                    while u < ml {
                        *op.wrapping_add(u) = *r#match.wrapping_add(u);
                        u += 1;
                    }
                }
            }
            op = op.wrapping_add(ml);
            if pdiff(oend, op) < LASTLITERALS {
                return -1;
            }
        }
    }
    pdiff(ip, istart) as c_int
}

const rvl_error: usize = usize::MAX;

#[inline(always)]
unsafe fn read_variable_length(
    ip: &mut *const u8,
    ilimit: *const u8,
    initial_check: bool,
) -> usize {
    let mut s: usize;
    let mut length: usize = 0;
    if initial_check && (*ip >= ilimit) {
        return rvl_error;
    }
    s = **ip as usize;
    *ip = (*ip).wrapping_add(1);
    length += s;
    if *ip > ilimit {
        return rvl_error;
    }
    if s != 255 {
        return length;
    }
    loop {
        s = **ip as usize;
        *ip = (*ip).wrapping_add(1);
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

/* safe-loop entry points */
const ST_TOP: u8 = 0;
const ST_LITCOPY: u8 = 1;
const ST_COPYMATCH: u8 = 2;
const ST_MATCHCOPY: u8 = 3;

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
    if src.is_null() || outputSize < 0 {
        return -1;
    }

    let mut ip: *const u8 = src as *const u8;
    let iend: *const u8 = ip.wrapping_add(srcSize as usize);

    let mut op: *mut u8 = dst as *mut u8;
    let oend: *mut u8 = op.wrapping_add(outputSize as usize);
    let mut cpy: *mut u8 = core::ptr::null_mut();

    let dictEnd: *const u8 = if dictStart.is_null() {
        core::ptr::null()
    } else {
        dictStart.wrapping_add(dictSize)
    };

    let checkOffset: bool = dictSize < (64 * 1024);

    /* Set up the "end" pointers for the shortcut. */
    let shortiend: *const u8 = iend.wrapping_sub(14).wrapping_sub(2);
    let shortoend: *mut u8 = oend.wrapping_sub(14).wrapping_sub(18);

    let mut r#match: *const u8 = core::ptr::null();
    let mut offset: usize = 0;
    let mut token: u32 = 0;
    let mut length: usize = 0;

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

    let mut state: u8 = ST_TOP;

    'fastloop: {
        if pdiff(oend, op) < FASTLOOP_SAFE_DISTANCE {
            break 'fastloop;
        }

        /* Fast loop */
        loop {
            token = *ip as u32;
            ip = ip.wrapping_add(1);
            length = (token >> ML_BITS) as usize; /* literal length */

            /* decode literal length */
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

                /* copy literals */
                if (op.wrapping_add(length) > oend.wrapping_sub(32))
                    || (ip.wrapping_add(length) > iend.wrapping_sub(32))
                {
                    state = ST_LITCOPY;
                    break 'fastloop;
                }
                LZ4_wildCopy32(op, ip, op.wrapping_add(length));
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
            } else if ip <= iend.wrapping_sub(16 + 1) {
                copy16(op, ip);
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
            } else {
                state = ST_LITCOPY;
                break 'fastloop;
            }

            /* get offset */
            offset = LZ4_readLE16(ip) as usize;
            ip = ip.wrapping_add(2);
            r#match = op.wrapping_sub(offset);

            /* get matchlength */
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
                    state = ST_MATCHCOPY;
                    break 'fastloop;
                }
            } else {
                length += MINMATCH;
                if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                    state = ST_MATCHCOPY;
                    break 'fastloop;
                }

                /* Fastpath check: skip LZ4_wildCopy32 when true */
                if (dict == withPrefix64k) || (r#match >= lowPrefix) {
                    if offset >= 8 {
                        copy8(op, r#match);
                        copy8(op.wrapping_add(8), r#match.wrapping_add(8));
                        copy2(op.wrapping_add(16), r#match.wrapping_add(16));
                        op = op.wrapping_add(length);
                        continue;
                    }
                }
            }

            if checkOffset && (r#match.wrapping_add(dictSize) < lowPrefix) {
                return output_error(ip, src);
            }
            /* match starting within external dictionary */
            if (dict == usingExtDict) && (r#match < lowPrefix) {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partialDecoding != 0 {
                        length = MIN_usize(length, pdiff(oend, op));
                    } else {
                        return output_error(ip, src);
                    }
                }

                if length <= pdiff(lowPrefix, r#match) {
                    /* match fits entirely within external dictionary : just copy */
                    LZ4_memmove(op, dictEnd.wrapping_sub(pdiff(lowPrefix, r#match)), length);
                    op = op.wrapping_add(length);
                } else {
                    /* match stretches into both external dictionary and current block */
                    let copySize: usize = pdiff(lowPrefix, r#match);
                    let restSize: usize = length - copySize;
                    LZ4_memcpy(op, dictEnd.wrapping_sub(copySize), copySize);
                    op = op.wrapping_add(copySize);
                    if restSize > pdiff(op, lowPrefix) {
                        /* overlap copy */
                        let endOfMatch = op.wrapping_add(restSize);
                        let mut copyFrom = lowPrefix;
                        while op < endOfMatch {
                            *op = *copyFrom;
                            op = op.wrapping_add(1);
                            copyFrom = copyFrom.wrapping_add(1);
                        }
                    } else {
                        LZ4_memcpy(op, lowPrefix, restSize);
                        op = op.wrapping_add(restSize);
                    }
                }
                continue;
            }

            /* copy match within block */
            cpy = op.wrapping_add(length);

            if offset < 16 {
                LZ4_memcpy_using_offset(op, r#match, cpy, offset);
            } else {
                LZ4_wildCopy32(op, r#match, cpy);
            }

            op = cpy; /* wildcopy correction */
        }
    }

    /* safe_decode: */
    loop {
        if state == ST_TOP {
            token = *ip as u32;
            ip = ip.wrapping_add(1);
            length = (token >> ML_BITS) as usize; /* literal length */

            /* A two-stage shortcut for the most common case */
            if (length != RUN_MASK as usize) && ((ip < shortiend) & (op <= shortoend)) {
                /* Copy the literals */
                copy16(op, ip);
                op = op.wrapping_add(length);
                ip = ip.wrapping_add(length);

                length = (token & ML_MASK) as usize; /* match length */
                offset = LZ4_readLE16(ip) as usize;
                ip = ip.wrapping_add(2);
                r#match = op.wrapping_sub(offset);

                /* Do not deal with overlapping matches. */
                if (length != ML_MASK as usize)
                    && (offset >= 8)
                    && (dict == withPrefix64k || r#match >= lowPrefix)
                {
                    /* Copy the match. */
                    copy8(op, r#match);
                    copy8(op.wrapping_add(8), r#match.wrapping_add(8));
                    copy2(op.wrapping_add(16), r#match.wrapping_add(16));
                    op = op.wrapping_add(length + MINMATCH);
                    /* Both stages worked, load the next token. */
                    continue;
                }

                /* The second stage didn't work out, but the info is ready. */
                state = ST_COPYMATCH;
            } else {
                /* decode literal length */
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
                state = ST_LITCOPY;
            }
        }

        if state == ST_LITCOPY {
            state = ST_TOP;
            /* safe_literal_copy: copy literals */
            cpy = op.wrapping_add(length);

            if (cpy > oend.wrapping_sub(MFLIMIT))
                || (ip.wrapping_add(length) > iend.wrapping_sub(2 + 1 + LASTLITERALS))
            {
                if partialDecoding != 0 {
                    if ip.wrapping_add(length) > iend {
                        length = pdiff(iend, ip);
                        cpy = op.wrapping_add(length);
                    }
                    if cpy > oend {
                        cpy = oend;
                        length = pdiff(oend, op);
                    }
                } else {
                    if (ip.wrapping_add(length) != iend) || (cpy > oend) {
                        return output_error(ip, src);
                    }
                }
                LZ4_memmove(op, ip, length);
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
                if (partialDecoding == 0) || (cpy == oend) || (ip >= iend.wrapping_sub(2)) {
                    break;
                }
            } else {
                LZ4_wildCopy8(op, ip, cpy);
                ip = ip.wrapping_add(length);
                op = cpy;
            }

            /* get offset */
            offset = LZ4_readLE16(ip) as usize;
            ip = ip.wrapping_add(2);
            r#match = op.wrapping_sub(offset);

            /* get matchlength */
            length = (token & ML_MASK) as usize;

            state = ST_COPYMATCH;
        }

        if state == ST_COPYMATCH {
            state = ST_TOP;
            /* _copy_match: */
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

            state = ST_MATCHCOPY;
        }

        if state == ST_MATCHCOPY {
            state = ST_TOP;
            /* safe_match_copy: */
            if checkOffset && (r#match.wrapping_add(dictSize) < lowPrefix) {
                return output_error(ip, src);
            }
            /* match starting within external dictionary */
            if (dict == usingExtDict) && (r#match < lowPrefix) {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partialDecoding != 0 {
                        length = MIN_usize(length, pdiff(oend, op));
                    } else {
                        return output_error(ip, src);
                    }
                }

                if length <= pdiff(lowPrefix, r#match) {
                    LZ4_memmove(op, dictEnd.wrapping_sub(pdiff(lowPrefix, r#match)), length);
                    op = op.wrapping_add(length);
                } else {
                    let copySize: usize = pdiff(lowPrefix, r#match);
                    let restSize: usize = length - copySize;
                    LZ4_memcpy(op, dictEnd.wrapping_sub(copySize), copySize);
                    op = op.wrapping_add(copySize);
                    if restSize > pdiff(op, lowPrefix) {
                        let endOfMatch = op.wrapping_add(restSize);
                        let mut copyFrom = lowPrefix;
                        while op < endOfMatch {
                            *op = *copyFrom;
                            op = op.wrapping_add(1);
                            copyFrom = copyFrom.wrapping_add(1);
                        }
                    } else {
                        LZ4_memcpy(op, lowPrefix, restSize);
                        op = op.wrapping_add(restSize);
                    }
                }
                continue;
            }

            /* copy match within block */
            cpy = op.wrapping_add(length);

            /* partialDecoding : may end anywhere within the block */
            if (partialDecoding != 0) && (cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE)) {
                let mlen: usize = MIN_usize(length, pdiff(oend, op));
                let matchEnd = r#match.wrapping_add(mlen);
                let copyEnd = op.wrapping_add(mlen);
                if matchEnd > op {
                    /* overlap copy */
                    while op < copyEnd {
                        *op = *r#match;
                        op = op.wrapping_add(1);
                        r#match = r#match.wrapping_add(1);
                    }
                } else {
                    LZ4_memcpy(op, r#match, mlen);
                }
                op = copyEnd;
                if op == oend {
                    break;
                }
                continue;
            }

            if offset < 8 {
                LZ4_write32(op, 0); /* silence msan warning when offset==0 */
                *op.wrapping_add(0) = *r#match.wrapping_add(0);
                *op.wrapping_add(1) = *r#match.wrapping_add(1);
                *op.wrapping_add(2) = *r#match.wrapping_add(2);
                *op.wrapping_add(3) = *r#match.wrapping_add(3);
                r#match = r#match.wrapping_add(inc32table[offset] as usize);
                copy4(op.wrapping_add(4), r#match);
                r#match = r#match.wrapping_offset(-(dec64table[offset] as isize));
            } else {
                copy8(op, r#match);
                r#match = r#match.wrapping_add(8);
            }
            op = op.wrapping_add(8);

            if cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let oCopyLimit = oend.wrapping_sub(WILDCOPYLENGTH - 1);
                if cpy > oend.wrapping_sub(LASTLITERALS) {
                    return output_error(ip, src);
                }
                if op < oCopyLimit {
                    LZ4_wildCopy8(op, r#match, oCopyLimit);
                    r#match = r#match.wrapping_add(pdiff(oCopyLimit, op));
                    op = oCopyLimit;
                }
                while op < cpy {
                    *op = *r#match;
                    op = op.wrapping_add(1);
                    r#match = r#match.wrapping_add(1);
                }
            } else {
                copy8(op, r#match);
                if length > 16 {
                    LZ4_wildCopy8(op.wrapping_add(8), r#match.wrapping_add(8), cpy);
                }
            }
            op = cpy; /* wildcopy correction */
        }
    }

    /* end of decoding */
    pdiff(op as *const c_char, dst) as c_int
}

#[inline(always)]
fn output_error(ip: *const u8, src: *const c_char) -> c_int {
    (-((ip as isize).wrapping_sub(src as isize)) as c_int) - 1
}

/* ===== Instantiate the API decoding functions. ===== */

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
        core::ptr::null(),
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_decompress_safe_partial(
    src: *const c_char,
    dst: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    let dstCapacity = MIN_int(targetOutputSize, dstCapacity);
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
        core::ptr::null(),
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
        core::ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_partial_withPrefix64k(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
) -> c_int {
    let dstCapacity = MIN_int(targetOutputSize, dstCapacity);
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        dstCapacity,
        partial_decode,
        withPrefix64k,
        (dest as *const u8).wrapping_sub(64 * 1024),
        core::ptr::null(),
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
        core::ptr::null(),
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
        core::ptr::null(),
        0,
    )
}

unsafe fn LZ4_decompress_safe_partial_withSmallPrefix(
    source: *const c_char,
    dest: *mut c_char,
    compressedSize: c_int,
    targetOutputSize: c_int,
    dstCapacity: c_int,
    prefixSize: usize,
) -> c_int {
    let dstCapacity = MIN_int(targetOutputSize, dstCapacity);
    LZ4_decompress_generic(
        source,
        dest,
        compressedSize,
        dstCapacity,
        partial_decode,
        noDict,
        (dest as *const u8).wrapping_sub(prefixSize),
        core::ptr::null(),
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
    dstCapacity: c_int,
    dictStart: *const c_void,
    dictSize: usize,
) -> c_int {
    let dstCapacity = MIN_int(targetOutputSize, dstCapacity);
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

/* ===== streaming decompression functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamDecode() -> *mut LZ4_streamDecode_t {
    calloc(1, SIZEOF_LZ4_STREAMDECODE_T) as *mut LZ4_streamDecode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamDecode(LZ4_stream: *mut LZ4_streamDecode_t) -> c_int {
    if LZ4_stream.is_null() {
        return 0;
    }
    free(LZ4_stream as *mut u8);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_setStreamDecode(
    LZ4_streamDecode: *mut LZ4_streamDecode_t,
    dictionary: *const c_char,
    dictSize: c_int,
) -> c_int {
    let lz4sd = LZ4_streamDecode as *mut LZ4_streamDecode_t_internal;
    (*lz4sd).prefixSize = dictSize as usize;
    if dictSize != 0 {
        (*lz4sd).prefixEnd = (dictionary as *const u8).wrapping_add(dictSize as usize);
    } else {
        (*lz4sd).prefixEnd = dictionary as *const u8;
    }
    (*lz4sd).externalDict = core::ptr::null();
    (*lz4sd).extDictSize = 0;
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
    let lz4sd = LZ4_streamDecode as *mut LZ4_streamDecode_t_internal;
    let result: c_int;

    if (*lz4sd).prefixSize == 0 {
        /* The first call, no dictionary yet. */
        result = LZ4_decompress_safe(source, dest, compressedSize, maxOutputSize);
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = result as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(result as usize);
    } else if (*lz4sd).prefixEnd == dest as *const u8 {
        /* They're rolling the current segment. */
        if (*lz4sd).prefixSize >= 64 * 1024 - 1 {
            result = LZ4_decompress_safe_withPrefix64k(source, dest, compressedSize, maxOutputSize);
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
        (*lz4sd).prefixEnd = (*lz4sd).prefixEnd.wrapping_add(result as usize);
    } else {
        /* The buffer wraps around, or they're switching to another buffer. */
        (*lz4sd).extDictSize = (*lz4sd).prefixSize;
        (*lz4sd).externalDict = (*lz4sd).prefixEnd.wrapping_sub((*lz4sd).extDictSize);
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
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(result as usize);
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
    let lz4sd = LZ4_streamDecode as *mut LZ4_streamDecode_t_internal;
    let result: c_int;

    if (*lz4sd).prefixSize == 0 {
        result = LZ4_decompress_fast(source, dest, originalSize);
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = originalSize as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(originalSize as usize);
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
        (*lz4sd).prefixEnd = (*lz4sd).prefixEnd.wrapping_add(originalSize as usize);
    } else {
        (*lz4sd).extDictSize = (*lz4sd).prefixSize;
        (*lz4sd).externalDict = (*lz4sd).prefixEnd.wrapping_sub((*lz4sd).extDictSize);
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
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(originalSize as usize);
    }

    result
}

/* ===== usingDict variants ===== */

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
    if dictStart.wrapping_add(dictSize as usize) == dest as *const c_char {
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
        return LZ4_decompress_safe_partial(
            source,
            dest,
            compressedSize,
            targetOutputSize,
            dstCapacity,
        );
    }
    if dictStart.wrapping_add(dictSize as usize) == dest as *const c_char {
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
    if dictSize == 0 || dictStart.wrapping_add(dictSize as usize) == dest as *const c_char {
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

/* ================================================================ *
 *  Obsolete Functions
 * ================================================================ */

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
    LZ4_compress_default(src, dest, srcSize, lz4_compress_bound(srcSize))
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
    LZ4_compress_fast_extState(state, src, dst, srcSize, lz4_compress_bound(srcSize), 1)
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
    LZ4_compress_fast_continue(
        LZ4_stream,
        source,
        dest,
        inputSize,
        lz4_compress_bound(inputSize),
        1,
    )
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
pub unsafe extern "C" fn LZ4_sizeofStreamState() -> c_int {
    SIZEOF_LZ4_STREAM_T as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStreamState(
    state: *mut c_void,
    _inputBuffer: *mut c_char,
) -> c_int {
    LZ4_resetStream(state as *mut LZ4_stream_t);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_create(_inputBuffer: *mut c_char) -> *mut c_void {
    LZ4_createStream() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char {
    (*(state as *mut LZ4_stream_t_internal)).dictionary as *mut c_char
}
