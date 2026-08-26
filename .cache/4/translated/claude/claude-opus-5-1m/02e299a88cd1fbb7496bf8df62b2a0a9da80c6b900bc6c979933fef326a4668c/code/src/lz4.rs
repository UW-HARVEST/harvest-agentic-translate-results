//! Translation of `c_src/src/lz4.c`

use crate::common::*;
use core::ffi::{c_char, c_int, c_void};

/*-************************************
*  Public state structures
**************************************/

#[repr(C)]
pub struct LZ4_stream_t_internal {
    pub hashTable: [u32; LZ4_HASH_SIZE_U32],
    pub dictionary: *const u8,
    pub dictCtx: *const LZ4_stream_t_internal,
    pub currentOffset: u32,
    pub tableType: u32,
    pub dictSize: u32,
}

/// `sizeof(LZ4_stream_t)` == `LZ4_STREAM_MINSIZE` == (1<<14)+32
pub const SIZEOF_LZ4_STREAM_T: usize = (1usize << LZ4_MEMORY_USAGE) + 32;
/// `sizeof(LZ4_stream_t_internal)`, with trailing padding
pub const SIZEOF_LZ4_STREAM_T_INTERNAL: usize = 16416;

pub type LZ4_stream_t = LZ4_stream_t_internal;

#[repr(C)]
pub struct LZ4_streamDecode_t_internal {
    pub externalDict: *const u8,
    pub prefixEnd: *const u8,
    pub extDictSize: usize,
    pub prefixSize: usize,
}

pub const SIZEOF_LZ4_STREAMDECODE_T: usize = 32;
pub type LZ4_streamDecode_t = LZ4_streamDecode_t_internal;

/*-************************************
*  Local Utils
**************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_versionNumber() -> c_int {
    LZ4_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_versionString() -> *const c_char {
    LZ4_VERSION_STRING.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_compressBound(isize_: c_int) -> c_int {
    LZ4_COMPRESSBOUND(isize_)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_sizeofState() -> c_int {
    SIZEOF_LZ4_STREAM_T as c_int
}

/*-******************************
*  Compression functions
********************************/

#[inline(always)]
fn LZ4_hash4(sequence: u32, tableType: i32) -> u32 {
    if tableType == byU16 {
        (sequence.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - (LZ4_HASHLOG + 1))
    } else {
        (sequence.wrapping_mul(2654435761u32)) >> ((MINMATCH as u32 * 8) - LZ4_HASHLOG)
    }
}

#[inline(always)]
fn LZ4_hash5(sequence: u64, tableType: i32) -> u32 {
    let hashLog: u32 = if tableType == byU16 {
        LZ4_HASHLOG + 1
    } else {
        LZ4_HASHLOG
    };
    if LZ4_isLittleEndian() {
        let prime5bytes: u64 = 889523592379u64;
        (((sequence << 24).wrapping_mul(prime5bytes)) >> (64 - hashLog)) as u32
    } else {
        let prime8bytes: u64 = 11400714785074694791u64;
        (((sequence >> 24).wrapping_mul(prime8bytes)) >> (64 - hashLog)) as u32
    }
}

#[inline(always)]
unsafe fn LZ4_hashPosition(p: *const u8, tableType: i32) -> u32 {
    if (core::mem::size_of::<RegT>() == 8) && (tableType != byU16) {
        return LZ4_hash5(LZ4_read_ARCH(p) as u64, tableType);
    }
    LZ4_hash4(LZ4_read32(p), tableType)
}

#[inline(always)]
unsafe fn LZ4_clearHash(h: u32, tableBase: *mut c_void, tableType: i32) {
    match tableType {
        x if x == byPtr => {
            let hashTable = tableBase as *mut *const u8;
            *hashTable.add(h as usize) = core::ptr::null();
        }
        x if x == byU32 => {
            let hashTable = tableBase as *mut u32;
            *hashTable.add(h as usize) = 0;
        }
        x if x == byU16 => {
            let hashTable = tableBase as *mut u16;
            *hashTable.add(h as usize) = 0;
        }
        _ => {}
    }
}

#[inline(always)]
unsafe fn LZ4_putIndexOnHash(idx: u32, h: u32, tableBase: *mut c_void, tableType: i32) {
    match tableType {
        x if x == byU32 => {
            let hashTable = tableBase as *mut u32;
            *hashTable.add(h as usize) = idx;
        }
        x if x == byU16 => {
            let hashTable = tableBase as *mut u16;
            *hashTable.add(h as usize) = idx as u16;
        }
        _ => {}
    }
}

#[inline(always)]
unsafe fn LZ4_putPositionOnHash(p: *const u8, h: u32, tableBase: *mut c_void, _tableType: i32) {
    let hashTable = tableBase as *mut *const u8;
    *hashTable.add(h as usize) = p;
}

#[inline(always)]
unsafe fn LZ4_putPosition(p: *const u8, tableBase: *mut c_void, tableType: i32) {
    let h = LZ4_hashPosition(p, tableType);
    LZ4_putPositionOnHash(p, h, tableBase, tableType);
}

#[inline(always)]
unsafe fn LZ4_getIndexOnHash(h: u32, tableBase: *const c_void, tableType: i32) -> u32 {
    if tableType == byU32 {
        let hashTable = tableBase as *const u32;
        return *hashTable.add(h as usize);
    }
    if tableType == byU16 {
        let hashTable = tableBase as *const u16;
        return *hashTable.add(h as usize) as u32;
    }
    0
}

#[inline(always)]
unsafe fn LZ4_getPositionOnHash(h: u32, tableBase: *const c_void, _tableType: i32) -> *const u8 {
    let hashTable = tableBase as *const *const u8;
    *hashTable.add(h as usize)
}

#[inline(always)]
unsafe fn LZ4_getPosition(p: *const u8, tableBase: *const c_void, tableType: i32) -> *const u8 {
    let h = LZ4_hashPosition(p, tableType);
    LZ4_getPositionOnHash(h, tableBase, tableType)
}

unsafe fn LZ4_prepareTable(cctx: *mut LZ4_stream_t_internal, inputSize: c_int, tableType: i32) {
    if (*cctx).tableType != clearedTable as u32 {
        if (*cctx).tableType != tableType as u32
            || ((tableType == byU16)
                && (*cctx).currentOffset.wrapping_add(inputSize as u32) >= 0xFFFFu32)
            || ((tableType == byU32) && (*cctx).currentOffset > (1u32 << 30))
            || tableType == byPtr
            || inputSize >= 4 * 1024
        {
            mem_init(
                (*cctx).hashTable.as_mut_ptr() as *mut u8,
                0,
                LZ4_HASHTABLESIZE,
            );
            (*cctx).currentOffset = 0;
            (*cctx).tableType = clearedTable as u32;
        }
    }

    if (*cctx).currentOffset != 0 && tableType == byU32 {
        (*cctx).currentOffset = (*cctx).currentOffset.wrapping_add(64 * 1024);
    }

    (*cctx).dictCtx = core::ptr::null();
    (*cctx).dictionary = core::ptr::null();
    (*cctx).dictSize = 0;
}

/// `LZ4_compress_generic_validated()`
unsafe fn LZ4_compress_generic_validated(
    cctx: *mut LZ4_stream_t_internal,
    source: *const c_char,
    dest: *mut c_char,
    inputSize: c_int,
    inputConsumed: *mut c_int,
    maxOutputSize: c_int,
    outputDirective: i32,
    tableType: i32,
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
    (*cctx).tableType = tableType as u32;

    let mut goto_last_literals = false;
    let mut jump_next_match = false;
    let mut match_: *const u8 = core::ptr::null();
    let mut token: *mut u8 = core::ptr::null_mut();
    let mut filledIp: *const u8 = core::ptr::null();

    if inputSize < LZ4_minLength {
        goto_last_literals = true;
    }

    if !goto_last_literals {
        /* First Byte */
        {
            let h = LZ4_hashPosition(ip, tableType);
            if tableType == byPtr {
                LZ4_putPositionOnHash(
                    ip,
                    h,
                    (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                    byPtr,
                );
            } else {
                LZ4_putIndexOnHash(
                    startIndex,
                    h,
                    (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                    tableType,
                );
            }
        }
        ip = ip.wrapping_add(1);
        forwardH = LZ4_hashPosition(ip, tableType);

        /* Main Loop */
        'main: loop {
            if !jump_next_match {
                /* Find a match */
                if tableType == byPtr {
                    let mut forwardIp: *const u8 = ip;
                    let mut step: i32 = 1;
                    let mut searchMatchNb: i32 = acceleration << LZ4_skipTrigger;
                    loop {
                        let h = forwardH;
                        ip = forwardIp;
                        forwardIp = forwardIp.wrapping_add(step as usize);
                        step = {
                            let v = searchMatchNb;
                            searchMatchNb = searchMatchNb.wrapping_add(1);
                            v >> LZ4_skipTrigger
                        };

                        if forwardIp > mflimitPlusOne {
                            goto_last_literals = true;
                            break;
                        }

                        match_ = LZ4_getPositionOnHash(
                            h,
                            (*cctx).hashTable.as_ptr() as *const c_void,
                            tableType,
                        );
                        forwardH = LZ4_hashPosition(forwardIp, tableType);
                        LZ4_putPositionOnHash(
                            ip,
                            h,
                            (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                            tableType,
                        );

                        if !((match_.wrapping_add(LZ4_DISTANCE_MAX as usize) < ip)
                            || (LZ4_read32(match_) != LZ4_read32(ip)))
                        {
                            break;
                        }
                    }
                    if goto_last_literals {
                        break 'main;
                    }
                } else {
                    /* byU32, byU16 */
                    let mut forwardIp: *const u8 = ip;
                    let mut step: i32 = 1;
                    let mut searchMatchNb: i32 = acceleration << LZ4_skipTrigger;
                    loop {
                        let h = forwardH;
                        let current: u32 = (forwardIp as usize - base as usize) as u32;
                        let mut matchIndex: u32 = LZ4_getIndexOnHash(
                            h,
                            (*cctx).hashTable.as_ptr() as *const c_void,
                            tableType,
                        );
                        ip = forwardIp;
                        forwardIp = forwardIp.wrapping_add(step as usize);
                        step = {
                            let v = searchMatchNb;
                            searchMatchNb = searchMatchNb.wrapping_add(1);
                            v >> LZ4_skipTrigger
                        };

                        if forwardIp > mflimitPlusOne {
                            goto_last_literals = true;
                            break;
                        }

                        if dictDirective == usingDictCtx {
                            if matchIndex < startIndex {
                                matchIndex = LZ4_getIndexOnHash(
                                    h,
                                    (*dictCtx).hashTable.as_ptr() as *const c_void,
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
                        LZ4_putIndexOnHash(
                            current,
                            h,
                            (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                            tableType,
                        );

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
                    if goto_last_literals {
                        break 'main;
                    }
                }

                /* Catch up */
                filledIp = ip;
                if (match_ > lowLimit) && (*ip.wrapping_sub(1) == *match_.wrapping_sub(1)) {
                    loop {
                        ip = ip.wrapping_sub(1);
                        match_ = match_.wrapping_sub(1);
                        if !(((ip > anchor) & (match_ > lowLimit))
                            && (*ip.wrapping_sub(1) == *match_.wrapping_sub(1)))
                        {
                            break;
                        }
                    }
                }

                /* Encode Literals */
                {
                    let litLength: u32 = (ip as usize - anchor as usize) as u32;
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
                        goto_last_literals = true;
                        break 'main;
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
            jump_next_match = false;

            /* _next_match: */
            if (outputDirective == fillOutput)
                && (op
                    .wrapping_add(2)
                    .wrapping_add(1)
                    .wrapping_add(MFLIMIT - MINMATCH)
                    > olimit)
            {
                op = token;
                goto_last_literals = true;
                break 'main;
            }

            /* Encode Offset */
            if maybe_extMem {
                LZ4_writeLE16(op, offset as u16);
                op = op.wrapping_add(2);
            } else {
                LZ4_writeLE16(op, (ip as usize - match_ as usize) as u16);
                op = op.wrapping_add(2);
            }

            /* Encode MatchLength */
            {
                let mut matchCode: u32;

                if (dictDirective == usingExtDict || dictDirective == usingDictCtx)
                    && (lowLimit == dictionary)
                {
                    let mut limit: *const u8 =
                        ip.wrapping_add(dictEnd as usize - match_ as usize);
                    if limit > matchlimit {
                        limit = matchlimit;
                    }
                    matchCode = LZ4_count(
                        ip.wrapping_add(MINMATCH),
                        match_.wrapping_add(MINMATCH),
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
                        match_.wrapping_add(MINMATCH),
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
                        let newMatchCode: u32 = 15u32
                            .wrapping_sub(1)
                            .wrapping_add(
                                ((olimit as usize - op as usize) as u32)
                                    .wrapping_sub(1)
                                    .wrapping_sub(LASTLITERALS as u32)
                                    .wrapping_mul(255),
                            );
                        ip = ip.wrapping_sub(matchCode.wrapping_sub(newMatchCode) as usize);
                        matchCode = newMatchCode;
                        if ip <= filledIp {
                            let mut ptr = ip;
                            while ptr <= filledIp {
                                let h = LZ4_hashPosition(ptr, tableType);
                                LZ4_clearHash(
                                    h,
                                    (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                                    tableType,
                                );
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
                break 'main;
            }

            /* Fill table */
            {
                let h = LZ4_hashPosition(ip.wrapping_sub(2), tableType);
                if tableType == byPtr {
                    LZ4_putPositionOnHash(
                        ip.wrapping_sub(2),
                        h,
                        (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                        byPtr,
                    );
                } else {
                    let idx: u32 = (ip.wrapping_sub(2) as usize - base as usize) as u32;
                    LZ4_putIndexOnHash(
                        idx,
                        h,
                        (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                        tableType,
                    );
                }
            }

            /* Test next position */
            if tableType == byPtr {
                match_ = LZ4_getPosition(ip, (*cctx).hashTable.as_ptr() as *const c_void, tableType);
                LZ4_putPosition(ip, (*cctx).hashTable.as_mut_ptr() as *mut c_void, tableType);
                if (match_.wrapping_add(LZ4_DISTANCE_MAX as usize) >= ip)
                    && (LZ4_read32(match_) == LZ4_read32(ip))
                {
                    token = op;
                    op = op.wrapping_add(1);
                    *token = 0;
                    jump_next_match = true;
                    continue 'main;
                }
            } else {
                let h = LZ4_hashPosition(ip, tableType);
                let current: u32 = (ip as usize - base as usize) as u32;
                let mut matchIndex: u32 =
                    LZ4_getIndexOnHash(h, (*cctx).hashTable.as_ptr() as *const c_void, tableType);
                if dictDirective == usingDictCtx {
                    if matchIndex < startIndex {
                        matchIndex = LZ4_getIndexOnHash(
                            h,
                            (*dictCtx).hashTable.as_ptr() as *const c_void,
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
                LZ4_putIndexOnHash(
                    current,
                    h,
                    (*cctx).hashTable.as_mut_ptr() as *mut c_void,
                    tableType,
                );
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
                    && (LZ4_read32(match_) == LZ4_read32(ip))
                {
                    token = op;
                    op = op.wrapping_add(1);
                    *token = 0;
                    if maybe_extMem {
                        offset = current.wrapping_sub(matchIndex);
                    }
                    jump_next_match = true;
                    continue 'main;
                }
            }

            /* Prepare next loop */
            ip = ip.wrapping_add(1);
            forwardH = LZ4_hashPosition(ip, tableType);
        }
    }

    /* _last_literals: Encode Last Literals */
    {
        let mut lastRun: usize = iend as usize - anchor as usize;
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
        mem_copy(op, anchor, lastRun);
        ip = anchor.wrapping_add(lastRun);
        op = op.wrapping_add(lastRun);
    }

    if outputDirective == fillOutput {
        *inputConsumed = (ip as usize).wrapping_sub(source as usize) as c_int;
    }
    result = (op as usize - dest as usize) as c_int;
    result
}

/// `LZ4_compress_generic()`
unsafe fn LZ4_compress_generic(
    cctx: *mut LZ4_stream_t_internal,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    inputConsumed: *mut c_int,
    dstCapacity: c_int,
    outputDirective: i32,
    tableType: i32,
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
            let tableType = if (core::mem::size_of::<*const c_void>() == 4)
                && ((source as usize) > LZ4_DISTANCE_MAX as usize)
            {
                byPtr
            } else {
                byU32
            };
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
            let tableType = if (core::mem::size_of::<*const c_void>() == 4)
                && ((source as usize) > LZ4_DISTANCE_MAX as usize)
            {
                byPtr
            } else {
                byU32
            };
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
            let tableType = if (core::mem::size_of::<*const c_void>() == 4)
                && ((src as usize) > LZ4_DISTANCE_MAX as usize)
            {
                byPtr
            } else {
                byU32
            };
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
            let tableType = if (core::mem::size_of::<*const c_void>() == 4)
                && ((src as usize) > LZ4_DISTANCE_MAX as usize)
            {
                byPtr
            } else {
                byU32
            };
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
    /* LZ4_HEAPMODE == 0 : state on the stack */
    let mut ctx = core::mem::MaybeUninit::<LZ4_stream_t>::uninit();
    let ctxPtr = ctx.as_mut_ptr() as *mut c_void;
    LZ4_compress_fast_extState(ctxPtr, src, dest, srcSize, dstCapacity, acceleration)
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
    LZ4_initStream(state as *mut c_void, SIZEOF_LZ4_STREAM_T);

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
                state as *mut LZ4_stream_t_internal,
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
            let addrMode = if (core::mem::size_of::<*const c_void>() == 4)
                && ((src as usize) > LZ4_DISTANCE_MAX as usize)
            {
                byPtr
            } else {
                byU32
            };
            LZ4_compress_generic(
                state as *mut LZ4_stream_t_internal,
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
    let mut ctxBody = core::mem::MaybeUninit::<LZ4_stream_t>::uninit();
    let ctx = ctxBody.as_mut_ptr() as *mut LZ4_stream_t;
    LZ4_compress_destSize_extState_internal(ctx, src, dst, srcSizePtr, targetDstSize, 1)
}

/*-******************************
*  Streaming functions
********************************/

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
    if !LZ4_isAligned(buffer as *const u8, LZ4_stream_t_alignment()) {
        return core::ptr::null_mut();
    }
    mem_init(buffer as *mut u8, 0, SIZEOF_LZ4_STREAM_T_INTERNAL);
    buffer as *mut LZ4_stream_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_resetStream(LZ4_stream: *mut LZ4_stream_t) {
    mem_init(LZ4_stream as *mut u8, 0, SIZEOF_LZ4_STREAM_T_INTERNAL);
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
    free(LZ4_stream as *mut c_void);
    0
}

/* LoadDict_mode_e */
pub const _ld_fast: c_int = 0;
pub const _ld_slow: c_int = 1;

const HASH_UNIT: usize = core::mem::size_of::<RegT>();

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

    if (dictEnd as usize - p as usize) > 64 * 1024 {
        p = dictEnd.wrapping_sub(64 * 1024);
    }
    (*dict).dictionary = p;
    (*dict).dictSize = (dictEnd as usize - p as usize) as u32;
    (*dict).tableType = tableType as u32;
    idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);

    while p <= dictEnd.wrapping_sub(HASH_UNIT) {
        let h = LZ4_hashPosition(p, tableType);
        LZ4_putIndexOnHash(
            idx32,
            h,
            (*dict).hashTable.as_mut_ptr() as *mut c_void,
            tableType,
        );
        p = p.wrapping_add(3);
        idx32 = idx32.wrapping_add(3);
    }

    if _ld == _ld_slow {
        p = (*dict).dictionary;
        idx32 = (*dict).currentOffset.wrapping_sub((*dict).dictSize);
        while p <= dictEnd.wrapping_sub(HASH_UNIT) {
            let h = LZ4_hashPosition(p, tableType);
            let limit = (*dict).currentOffset.wrapping_sub(64 * 1024);
            if LZ4_getIndexOnHash(h, (*dict).hashTable.as_ptr() as *const c_void, tableType)
                <= limit
            {
                LZ4_putIndexOnHash(
                    idx32,
                    h,
                    (*dict).hashTable.as_mut_ptr() as *mut c_void,
                    tableType,
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
        let delta: u32 = (*LZ4_dict).currentOffset.wrapping_sub(64 * 1024);
        let dictEnd: *const u8 = (*LZ4_dict)
            .dictionary
            .wrapping_add((*LZ4_dict).dictSize as usize);
        let mut i = 0usize;
        while i < LZ4_HASH_SIZE_U32 {
            if (*LZ4_dict).hashTable[i] < delta {
                (*LZ4_dict).hashTable[i] = 0;
            } else {
                (*LZ4_dict).hashTable[i] -= delta;
            }
            i += 1;
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
            (*streamPtr).dictSize = (dictEnd as usize - sourceEnd as usize) as u32;
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
                mem_copy(
                    streamPtr as *mut u8,
                    (*streamPtr).dictCtx as *const u8,
                    SIZEOF_LZ4_STREAM_T_INTERNAL,
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
        mem_move(
            safeBuffer as *mut u8,
            previousDictEnd.wrapping_sub(dictSize as usize),
            dictSize as usize,
        );
    }

    (*dict).dictionary = safeBuffer as *const u8;
    (*dict).dictSize = dictSize as u32;

    dictSize
}

/*-*******************************
 *  Decompression functions
 ********************************/

#[inline(always)]
fn MINuz(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
fn MINi(a: c_int, b: c_int) -> c_int {
    if a < b {
        a
    } else {
        b
    }
}

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
    let mut ip: *const u8 = istart;
    let mut op: *mut u8 = ostart;
    let oend: *mut u8 = ostart.wrapping_add(decompressedSize as usize);
    let prefixStart: *const u8 = (ostart as *const u8).wrapping_sub(prefixSize);

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
            let mut ml: usize = (token & 15) as usize;
            let offset: usize = LZ4_readLE16(ip) as usize;
            ip = ip.wrapping_add(2);

            if ml == 15 {
                ml += read_long_length_no_check(&mut ip);
            }
            ml += MINMATCH;

            if (oend as usize - op as usize) < ml {
                return -1;
            }

            {
                let mut match_: *const u8 = (op as *const u8).wrapping_sub(offset);

                /* out of range */
                if offset > (op as usize - prefixStart as usize) + dictSize {
                    return -1;
                }

                /* check special case : extDict */
                if offset > (op as usize - prefixStart as usize) {
                    let dictEnd: *const u8 = dictStart.wrapping_add(dictSize);
                    let extMatch: *const u8 = dictEnd
                        .wrapping_sub(offset - (op as usize - prefixStart as usize));
                    let extml: usize = dictEnd as usize - extMatch as usize;
                    if extml > ml {
                        mem_move(op, extMatch, ml);
                        op = op.wrapping_add(ml);
                        ml = 0;
                    } else {
                        mem_move(op, extMatch, extml);
                        op = op.wrapping_add(extml);
                        ml -= extml;
                    }
                    match_ = prefixStart;
                }

                /* match copy - slow variant, supporting overlap copy */
                {
                    let mut u = 0usize;
                    while u < ml {
                        *op.wrapping_add(u) = *match_.wrapping_add(u);
                        u += 1;
                    }
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
    length = length.wrapping_add(s);
    if *ip > ilimit {
        return rvl_error;
    }
    if (core::mem::size_of::<usize>() < 8) && (length > (usize::MAX / 2)) {
        return rvl_error;
    }
    if s != 255 {
        return length;
    }
    loop {
        s = **ip as usize;
        *ip = (*ip).wrapping_add(1);
        length = length.wrapping_add(s);
        if *ip > ilimit {
            return rvl_error;
        }
        if (core::mem::size_of::<usize>() < 8) && (length > (usize::MAX / 2)) {
            return rvl_error;
        }
        if s != 255 {
            break;
        }
    }
    length
}

/* LZ4_FAST_DEC_LOOP is 1 on x86/x86_64 and aarch64 */
#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
const LZ4_FAST_DEC_LOOP: bool = true;
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
const LZ4_FAST_DEC_LOOP: bool = false;

#[derive(PartialEq, Eq, Copy, Clone)]
enum SafeStage {
    Top,
    LiteralCopy,
    CopyMatch,
    MatchCopy,
}

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
    let mut cpy: *mut u8;

    let dictEnd: *const u8 = if dictStart.is_null() {
        core::ptr::null()
    } else {
        dictStart.wrapping_add(dictSize)
    };

    let checkOffset: bool = dictSize < (64 * 1024);

    let shortiend: *const u8 = iend.wrapping_sub(14).wrapping_sub(2);
    let shortoend: *mut u8 = oend.wrapping_sub(14).wrapping_sub(18);

    let mut match_: *const u8 = core::ptr::null();
    let mut offset: usize = 0;
    let mut token: u32 = 0;
    let mut length: usize = 0;

    macro_rules! output_error {
        () => {{
            let d = (ip as isize).wrapping_sub(src as isize);
            return (-(d as i32)).wrapping_sub(1);
        }};
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

    let mut entry = SafeStage::Top;
    let mut goto_safe = false;

    if LZ4_FAST_DEC_LOOP {
        if (oend as usize - op as usize) < FASTLOOP_SAFE_DISTANCE {
            goto_safe = true;
        }

        if !goto_safe {
            'fast: loop {
                token = *ip as u32;
                ip = ip.wrapping_add(1);
                length = (token >> ML_BITS) as usize; /* literal length */

                /* decode literal length */
                if length == RUN_MASK as usize {
                    let addl =
                        read_variable_length(&mut ip, iend.wrapping_sub(RUN_MASK as usize), true);
                    if addl == rvl_error {
                        output_error!();
                    }
                    length = length.wrapping_add(addl);
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                    if (ip as usize).wrapping_add(length) < (ip as usize) {
                        output_error!();
                    }

                    /* copy literals */
                    if (op.wrapping_add(length) > oend.wrapping_sub(32))
                        || (ip.wrapping_add(length) > iend.wrapping_sub(32))
                    {
                        entry = SafeStage::LiteralCopy;
                        break 'fast;
                    }
                    LZ4_wildCopy32(op, ip, op.wrapping_add(length));
                    ip = ip.wrapping_add(length);
                    op = op.wrapping_add(length);
                } else if ip <= iend.wrapping_sub(16 + 1) {
                    copy16(op, ip);
                    ip = ip.wrapping_add(length);
                    op = op.wrapping_add(length);
                } else {
                    entry = SafeStage::LiteralCopy;
                    break 'fast;
                }

                /* get offset */
                offset = LZ4_readLE16(ip) as usize;
                ip = ip.wrapping_add(2);
                match_ = (op as *const u8).wrapping_sub(offset);

                /* get matchlength */
                length = (token & ML_MASK) as usize;

                if length == ML_MASK as usize {
                    let addl = read_variable_length(
                        &mut ip,
                        iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                        false,
                    );
                    if addl == rvl_error {
                        output_error!();
                    }
                    length = length.wrapping_add(addl);
                    length = length.wrapping_add(MINMATCH);
                    if (op as usize).wrapping_add(length) < (op as usize) {
                        output_error!();
                    }
                    if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                        entry = SafeStage::MatchCopy;
                        break 'fast;
                    }
                } else {
                    length = length.wrapping_add(MINMATCH);
                    if op.wrapping_add(length) >= oend.wrapping_sub(FASTLOOP_SAFE_DISTANCE) {
                        entry = SafeStage::MatchCopy;
                        break 'fast;
                    }

                    /* Fastpath check: skip LZ4_wildCopy32 when true */
                    if (dict == withPrefix64k) || (match_ >= lowPrefix) {
                        if offset >= 8 {
                            copy8(op, match_);
                            copy8(op.wrapping_add(8), match_.wrapping_add(8));
                            copy2(op.wrapping_add(16), match_.wrapping_add(16));
                            op = op.wrapping_add(length);
                            continue 'fast;
                        }
                    }
                }

                if checkOffset && (match_.wrapping_add(dictSize) < lowPrefix) {
                    output_error!();
                }
                /* match starting within external dictionary */
                if (dict == usingExtDict) && (match_ < lowPrefix) {
                    if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                        if partialDecoding != 0 {
                            length = MINuz(length, oend as usize - op as usize);
                        } else {
                            output_error!();
                        }
                    }

                    if length <= (lowPrefix as usize - match_ as usize) {
                        mem_move(
                            op,
                            dictEnd
                                .wrapping_sub(lowPrefix as usize - match_ as usize),
                            length,
                        );
                        op = op.wrapping_add(length);
                    } else {
                        let copySize: usize = lowPrefix as usize - match_ as usize;
                        let restSize: usize = length - copySize;
                        mem_copy(op, dictEnd.wrapping_sub(copySize), copySize);
                        op = op.wrapping_add(copySize);
                        if restSize > (op as usize - lowPrefix as usize) {
                            let endOfMatch: *mut u8 = op.wrapping_add(restSize);
                            let mut copyFrom: *const u8 = lowPrefix;
                            while op < endOfMatch {
                                *op = *copyFrom;
                                op = op.wrapping_add(1);
                                copyFrom = copyFrom.wrapping_add(1);
                            }
                        } else {
                            mem_copy(op, lowPrefix, restSize);
                            op = op.wrapping_add(restSize);
                        }
                    }
                    continue 'fast;
                }

                /* copy match within block */
                cpy = op.wrapping_add(length);

                if offset < 16 {
                    LZ4_memcpy_using_offset(op, match_, cpy, offset);
                } else {
                    LZ4_wildCopy32(op, match_, cpy);
                }

                op = cpy;
            }
        }
    }

    /* safe_decode: */
    'safe: loop {
        if entry == SafeStage::Top {
            token = *ip as u32;
            ip = ip.wrapping_add(1);
            length = (token >> ML_BITS) as usize;

            let mut goto_copy_match = false;
            if (length != RUN_MASK as usize) && ((ip < shortiend) & (op <= shortoend)) {
                /* Copy the literals */
                copy16(op, ip);
                op = op.wrapping_add(length);
                ip = ip.wrapping_add(length);

                length = (token & ML_MASK) as usize;
                offset = LZ4_readLE16(ip) as usize;
                ip = ip.wrapping_add(2);
                match_ = (op as *const u8).wrapping_sub(offset);

                if (length != ML_MASK as usize)
                    && (offset >= 8)
                    && (dict == withPrefix64k || match_ >= lowPrefix)
                {
                    copy8(op.wrapping_add(0), match_.wrapping_add(0));
                    copy8(op.wrapping_add(8), match_.wrapping_add(8));
                    copy2(op.wrapping_add(16), match_.wrapping_add(16));
                    op = op.wrapping_add(length + MINMATCH);
                    continue 'safe;
                }

                goto_copy_match = true;
            }

            if goto_copy_match {
                entry = SafeStage::CopyMatch;
            } else {
                /* decode literal length */
                if length == RUN_MASK as usize {
                    let addl =
                        read_variable_length(&mut ip, iend.wrapping_sub(RUN_MASK as usize), true);
                    if addl == rvl_error {
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
                entry = SafeStage::LiteralCopy;
            }
        }

        if entry == SafeStage::LiteralCopy {
            /* safe_literal_copy: copy literals */
            cpy = op.wrapping_add(length);

            if (cpy > oend.wrapping_sub(MFLIMIT))
                || (ip.wrapping_add(length) > iend.wrapping_sub(2 + 1 + LASTLITERALS))
            {
                if partialDecoding != 0 {
                    if ip.wrapping_add(length) > iend {
                        length = iend as usize - ip as usize;
                        cpy = op.wrapping_add(length);
                    }
                    if cpy > oend {
                        cpy = oend;
                        length = oend as usize - op as usize;
                    }
                } else {
                    if (ip.wrapping_add(length) != iend) || (cpy > oend) {
                        output_error!();
                    }
                }
                mem_move(op, ip, length);
                ip = ip.wrapping_add(length);
                op = op.wrapping_add(length);
                if (partialDecoding == 0) || (cpy == oend) || (ip >= iend.wrapping_sub(2)) {
                    break 'safe;
                }
            } else {
                LZ4_wildCopy8(op, ip, cpy);
                ip = ip.wrapping_add(length);
                op = cpy;
            }

            /* get offset */
            offset = LZ4_readLE16(ip) as usize;
            ip = ip.wrapping_add(2);
            match_ = (op as *const u8).wrapping_sub(offset);

            /* get matchlength */
            length = (token & ML_MASK) as usize;

            entry = SafeStage::CopyMatch;
        }

        if entry == SafeStage::CopyMatch {
            /* _copy_match: */
            if length == ML_MASK as usize {
                let addl = read_variable_length(
                    &mut ip,
                    iend.wrapping_sub(LASTLITERALS).wrapping_add(1),
                    false,
                );
                if addl == rvl_error {
                    output_error!();
                }
                length = length.wrapping_add(addl);
                if (op as usize).wrapping_add(length) < (op as usize) {
                    output_error!();
                }
            }
            length = length.wrapping_add(MINMATCH);
            entry = SafeStage::MatchCopy;
        }

        /* safe_match_copy: */
        {
            if checkOffset && (match_.wrapping_add(dictSize) < lowPrefix) {
                output_error!();
            }
            /* match starting within external dictionary */
            if (dict == usingExtDict) && (match_ < lowPrefix) {
                if op.wrapping_add(length) > oend.wrapping_sub(LASTLITERALS) {
                    if partialDecoding != 0 {
                        length = MINuz(length, oend as usize - op as usize);
                    } else {
                        output_error!();
                    }
                }

                if length <= (lowPrefix as usize - match_ as usize) {
                    mem_move(
                        op,
                        dictEnd.wrapping_sub(lowPrefix as usize - match_ as usize),
                        length,
                    );
                    op = op.wrapping_add(length);
                } else {
                    let copySize: usize = lowPrefix as usize - match_ as usize;
                    let restSize: usize = length - copySize;
                    mem_copy(op, dictEnd.wrapping_sub(copySize), copySize);
                    op = op.wrapping_add(copySize);
                    if restSize > (op as usize - lowPrefix as usize) {
                        let endOfMatch: *mut u8 = op.wrapping_add(restSize);
                        let mut copyFrom: *const u8 = lowPrefix;
                        while op < endOfMatch {
                            *op = *copyFrom;
                            op = op.wrapping_add(1);
                            copyFrom = copyFrom.wrapping_add(1);
                        }
                    } else {
                        mem_copy(op, lowPrefix, restSize);
                        op = op.wrapping_add(restSize);
                    }
                }
                entry = SafeStage::Top;
                continue 'safe;
            }

            /* copy match within block */
            cpy = op.wrapping_add(length);

            /* partialDecoding : may end anywhere within the block */
            if (partialDecoding != 0) && (cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE)) {
                let mlen: usize = MINuz(length, oend as usize - op as usize);
                let matchEnd: *const u8 = match_.wrapping_add(mlen);
                let copyEnd: *mut u8 = op.wrapping_add(mlen);
                if matchEnd > (op as *const u8) {
                    while op < copyEnd {
                        *op = *match_;
                        op = op.wrapping_add(1);
                        match_ = match_.wrapping_add(1);
                    }
                } else {
                    mem_copy(op, match_, mlen);
                }
                op = copyEnd;
                if op == oend {
                    break 'safe;
                }
                entry = SafeStage::Top;
                continue 'safe;
            }

            if offset < 8 {
                LZ4_write32(op, 0);
                *op.wrapping_add(0) = *match_.wrapping_add(0);
                *op.wrapping_add(1) = *match_.wrapping_add(1);
                *op.wrapping_add(2) = *match_.wrapping_add(2);
                *op.wrapping_add(3) = *match_.wrapping_add(3);
                match_ = match_.wrapping_add(inc32table[offset] as usize);
                copy4(op.wrapping_add(4), match_);
                match_ = match_.wrapping_offset(-(dec64table[offset] as isize));
            } else {
                copy8(op, match_);
                match_ = match_.wrapping_add(8);
            }
            op = op.wrapping_add(8);

            if cpy > oend.wrapping_sub(MATCH_SAFEGUARD_DISTANCE) {
                let oCopyLimit: *mut u8 = oend.wrapping_sub(WILDCOPYLENGTH - 1);
                if cpy > oend.wrapping_sub(LASTLITERALS) {
                    output_error!();
                }
                if op < oCopyLimit {
                    LZ4_wildCopy8(op, match_, oCopyLimit);
                    match_ = match_.wrapping_add(oCopyLimit as usize - op as usize);
                    op = oCopyLimit;
                }
                while op < cpy {
                    *op = *match_;
                    op = op.wrapping_add(1);
                    match_ = match_.wrapping_add(1);
                }
            } else {
                copy8(op, match_);
                if length > 16 {
                    LZ4_wildCopy8(op.wrapping_add(8), match_.wrapping_add(8), cpy);
                }
            }
            op = cpy;
        }

        entry = SafeStage::Top;
    }

    /* end of decoding */
    (op as usize - dst as usize) as c_int
}

/*===== Instantiate the API decoding functions. =====*/

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
    let dstCapacity = MINi(targetOutputSize, dstCapacity);
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
    let dstCapacity = MINi(targetOutputSize, dstCapacity);
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

/*===== streaming decompression functions =====*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_createStreamDecode() -> *mut LZ4_streamDecode_t {
    calloc(1, SIZEOF_LZ4_STREAMDECODE_T) as *mut LZ4_streamDecode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_freeStreamDecode(LZ4_stream: *mut LZ4_streamDecode_t) -> c_int {
    if LZ4_stream.is_null() {
        return 0;
    }
    free(LZ4_stream as *mut c_void);
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
    LZ4_DECODER_RING_BUFFER_SIZE(maxBlockSize)
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
        result = LZ4_decompress_safe(source, dest, compressedSize, maxOutputSize);
        if result <= 0 {
            return result;
        }
        (*lz4sd).prefixSize = result as usize;
        (*lz4sd).prefixEnd = (dest as *const u8).wrapping_add(result as usize);
    } else if (*lz4sd).prefixEnd == (dest as *const u8) {
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
    } else if (*lz4sd).prefixEnd == (dest as *const u8) {
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

/*
Advanced decoding functions
*/

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
    if dictStart.wrapping_add(dictSize as usize) == (dest as *const c_char) {
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
    if dictStart.wrapping_add(dictSize as usize) == (dest as *const c_char) {
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
    if dictSize == 0 || dictStart.wrapping_add(dictSize as usize) == (dest as *const c_char) {
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

/*=*************************************************
*  Obsolete Functions
***************************************************/

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
    LZ4_compress_fast_continue(
        LZ4_stream,
        source,
        dest,
        inputSize,
        LZ4_compressBound(inputSize),
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
