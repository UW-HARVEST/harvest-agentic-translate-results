use ::libc;
extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type ptrdiff_t = isize;
pub type size_t = usize;
pub type dctx_t = ZSTDv01_Dctx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv01_Dctx_s {
    pub LLTable: [U32; 1025],
    pub OffTable: [U32; 513],
    pub MLTable: [U32; 1025],
    pub previousDstEnd: *mut ::core::ffi::c_void,
    pub base: *mut ::core::ffi::c_void,
    pub expected: size_t,
    pub bType: blockType_t,
    pub phase: U32,
}
pub type U32 = uint32_t;
pub type uint32_t = __uint32_t;
pub type __uint32_t = u32;
pub type blockType_t = ::core::ffi::c_uint;
pub const bt_end: blockType_t = 3;
pub const bt_rle: blockType_t = 2;
pub const bt_raw: blockType_t = 1;
pub const bt_compressed: blockType_t = 0;
pub type BYTE = uint8_t;
pub type uint8_t = __uint8_t;
pub type __uint8_t = u8;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}
pub const ZSTD_error_srcSize_wrong: C2RustUnnamed_2 = 72;
pub const ZSTD_error_maxCode: C2RustUnnamed_2 = 120;
pub const ZSTD_error_GENERIC: C2RustUnnamed_2 = 1;
pub const ZSTD_error_dstSize_tooSmall: C2RustUnnamed_2 = 70;
pub const ZSTD_error_corruption_detected: C2RustUnnamed_2 = 20;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: ::core::ffi::c_uint,
    pub ptr: *const ::core::ffi::c_char,
    pub start: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seqState_t {
    pub DStream: FSE_DStream_t,
    pub stateLL: FSE_DState_t,
    pub stateOffb: FSE_DState_t,
    pub stateML: FSE_DState_t,
    pub prevOffset: size_t,
    pub dumps: *const BYTE,
    pub dumpsEnd: *const BYTE,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DState_t {
    pub state: size_t,
    pub table: *const ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seq_t {
    pub litLength: size_t,
    pub offset: size_t,
    pub matchLength: size_t,
}
pub type U64 = uint64_t;
pub type uint64_t = __uint64_t;
pub type __uint64_t = u64;
pub type U16 = uint16_t;
pub type uint16_t = __uint16_t;
pub type __uint16_t = u16;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub i: U32,
    pub c: [BYTE; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_decode_t {
    pub newState: ::core::ffi::c_ushort,
    pub symbol: ::core::ffi::c_uchar,
    pub nbBits: ::core::ffi::c_uchar,
}
pub const FSE_DStream_unfinished: C2RustUnnamed_4 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_0 {
    pub i: U32,
    pub c: [BYTE; 4],
}
pub const FSE_DStream_endOfBuffer: C2RustUnnamed_4 = 1;
pub const FSE_DStream_completed: C2RustUnnamed_4 = 2;
pub const FSE_DStream_tooFar: C2RustUnnamed_4 = 3;
pub type FSE_DTable = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}
pub const FSE_ERROR_maxCode: C2RustUnnamed_3 = 8;
pub const FSE_ERROR_GENERIC: C2RustUnnamed_3 = 1;
pub const FSE_ERROR_srcSize_wrong: C2RustUnnamed_3 = 6;
pub type S16 = int16_t;
pub type int16_t = __int16_t;
pub type __int16_t = i16;
pub const FSE_ERROR_tableLog_tooLarge: C2RustUnnamed_3 = 2;
pub const FSE_ERROR_maxSymbolValue_tooLarge: C2RustUnnamed_3 = 3;
pub const FSE_ERROR_maxSymbolValue_tooSmall: C2RustUnnamed_3 = 4;
pub const FSE_ERROR_corruptionDetected: C2RustUnnamed_3 = 7;
pub const FSE_ERROR_dstSize_tooSmall: C2RustUnnamed_3 = 5;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_DElt {
    pub byte: BYTE,
    pub nbBits: BYTE,
}
pub type DTable_max_t = [U32; 4097];
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const FSE_static_assert: C2RustUnnamed_1 = 1;
pub const ZSTD_error_prefix_unknown: C2RustUnnamed_2 = 10;
pub type ZSTDv01_Dctx = ZSTDv01_Dctx_s;
pub type C2RustUnnamed_2 = ::core::ffi::c_uint;
pub const ZSTD_error_externalSequences_invalid: C2RustUnnamed_2 = 107;
pub const ZSTD_error_sequenceProducer_failed: C2RustUnnamed_2 = 106;
pub const ZSTD_error_srcBuffer_wrong: C2RustUnnamed_2 = 105;
pub const ZSTD_error_dstBuffer_wrong: C2RustUnnamed_2 = 104;
pub const ZSTD_error_seekableIO: C2RustUnnamed_2 = 102;
pub const ZSTD_error_frameIndex_tooLarge: C2RustUnnamed_2 = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: C2RustUnnamed_2 = 82;
pub const ZSTD_error_noForwardProgress_destFull: C2RustUnnamed_2 = 80;
pub const ZSTD_error_dstBuffer_null: C2RustUnnamed_2 = 74;
pub const ZSTD_error_workSpace_tooSmall: C2RustUnnamed_2 = 66;
pub const ZSTD_error_memory_allocation: C2RustUnnamed_2 = 64;
pub const ZSTD_error_init_missing: C2RustUnnamed_2 = 62;
pub const ZSTD_error_stage_wrong: C2RustUnnamed_2 = 60;
pub const ZSTD_error_stabilityCondition_notRespected: C2RustUnnamed_2 = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: C2RustUnnamed_2 = 49;
pub const ZSTD_error_maxSymbolValue_tooSmall: C2RustUnnamed_2 = 48;
pub const ZSTD_error_maxSymbolValue_tooLarge: C2RustUnnamed_2 = 46;
pub const ZSTD_error_tableLog_tooLarge: C2RustUnnamed_2 = 44;
pub const ZSTD_error_parameter_outOfBound: C2RustUnnamed_2 = 42;
pub const ZSTD_error_parameter_combination_unsupported: C2RustUnnamed_2 = 41;
pub const ZSTD_error_parameter_unsupported: C2RustUnnamed_2 = 40;
pub const ZSTD_error_dictionaryCreation_failed: C2RustUnnamed_2 = 34;
pub const ZSTD_error_dictionary_wrong: C2RustUnnamed_2 = 32;
pub const ZSTD_error_dictionary_corrupted: C2RustUnnamed_2 = 30;
pub const ZSTD_error_literals_headerWrong: C2RustUnnamed_2 = 24;
pub const ZSTD_error_checksum_wrong: C2RustUnnamed_2 = 22;
pub const ZSTD_error_frameParameter_windowTooLarge: C2RustUnnamed_2 = 16;
pub const ZSTD_error_frameParameter_unsupported: C2RustUnnamed_2 = 14;
pub const ZSTD_error_version_unsupported: C2RustUnnamed_2 = 12;
pub const ZSTD_error_no_error: C2RustUnnamed_2 = 0;
pub type C2RustUnnamed_3 = ::core::ffi::c_uint;
pub const FSE_OK_NoError: C2RustUnnamed_3 = 0;
pub type C2RustUnnamed_4 = ::core::ffi::c_uint;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
pub const FSE_MAX_MEMORY_USAGE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const FSE_MAX_SYMBOL_VALUE: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
unsafe extern "C" fn FSE_32bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize == 4 as usize)
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
unsafe extern "C" fn FSE_isLittleEndian() -> ::core::ffi::c_uint {
    let one: C2RustUnnamed_0 = C2RustUnnamed_0 {
        i: 1 as ::core::ffi::c_int as U32,
    };
    return one.c[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint;
}
unsafe extern "C" fn FSE_read16(mut memPtr: *const ::core::ffi::c_void) -> U16 {
    let mut val: U16 = 0;
    memcpy(
        &raw mut val as *mut ::core::ffi::c_void,
        memPtr,
        ::core::mem::size_of::<U16>() as size_t,
    );
    return val;
}
unsafe extern "C" fn FSE_read32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    let mut val: U32 = 0;
    memcpy(
        &raw mut val as *mut ::core::ffi::c_void,
        memPtr,
        ::core::mem::size_of::<U32>() as size_t,
    );
    return val;
}
unsafe extern "C" fn FSE_read64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    let mut val: U64 = 0;
    memcpy(
        &raw mut val as *mut ::core::ffi::c_void,
        memPtr,
        ::core::mem::size_of::<U64>() as size_t,
    );
    return val;
}
unsafe extern "C" fn FSE_readLE16(mut memPtr: *const ::core::ffi::c_void) -> U16 {
    if FSE_isLittleEndian() != 0 {
        return FSE_read16(memPtr);
    } else {
        let mut p: *const BYTE = memPtr as *const BYTE;
        return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            + ((*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int)) as U16;
    };
}
unsafe extern "C" fn FSE_readLE32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    if FSE_isLittleEndian() != 0 {
        return FSE_read32(memPtr);
    } else {
        let mut p: *const BYTE = memPtr as *const BYTE;
        return (*p.offset(0 as ::core::ffi::c_int as isize) as U32)
            .wrapping_add(
                (*p.offset(1 as ::core::ffi::c_int as isize) as U32) << 8 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(2 as ::core::ffi::c_int as isize) as U32) << 16 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(3 as ::core::ffi::c_int as isize) as U32) << 24 as ::core::ffi::c_int,
            );
    };
}
unsafe extern "C" fn FSE_readLE64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    if FSE_isLittleEndian() != 0 {
        return FSE_read64(memPtr);
    } else {
        let mut p: *const BYTE = memPtr as *const BYTE;
        return (*p.offset(0 as ::core::ffi::c_int as isize) as U64)
            .wrapping_add(
                (*p.offset(1 as ::core::ffi::c_int as isize) as U64) << 8 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(2 as ::core::ffi::c_int as isize) as U64) << 16 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(3 as ::core::ffi::c_int as isize) as U64) << 24 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(4 as ::core::ffi::c_int as isize) as U64) << 32 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(5 as ::core::ffi::c_int as isize) as U64) << 40 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(6 as ::core::ffi::c_int as isize) as U64) << 48 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*p.offset(7 as ::core::ffi::c_int as isize) as U64) << 56 as ::core::ffi::c_int,
            );
    };
}
unsafe extern "C" fn FSE_readLEST(mut memPtr: *const ::core::ffi::c_void) -> size_t {
    if FSE_32bits() != 0 {
        return FSE_readLE32(memPtr) as size_t;
    } else {
        return FSE_readLE64(memPtr) as size_t;
    };
}
pub const FSE_MAX_TABLELOG: ::core::ffi::c_int = FSE_MAX_MEMORY_USAGE - 2 as ::core::ffi::c_int;
pub const FSE_MIN_TABLELOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const FSE_TABLELOG_ABSOLUTE_MAX: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
#[inline(always)]
unsafe extern "C" fn FSE_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (val.leading_zeros() as i32 ^ 31 as ::core::ffi::c_int) as ::core::ffi::c_uint;
}
unsafe extern "C" fn FSE_tableStep(mut tableSize: U32) -> U32 {
    return (tableSize >> 1 as ::core::ffi::c_int)
        .wrapping_add(tableSize >> 3 as ::core::ffi::c_int)
        .wrapping_add(3 as U32);
}
unsafe extern "C" fn FSE_buildDTable(
    mut dt: *mut FSE_DTable,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
) -> size_t {
    let mut ptr: *mut ::core::ffi::c_void = dt as *mut ::core::ffi::c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let tableDecode: *mut FSE_decode_t =
        (ptr as *mut FSE_decode_t).offset(1 as ::core::ffi::c_int as isize);
    let tableSize: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let tableMask: U32 = tableSize.wrapping_sub(1 as U32);
    let step: U32 = FSE_tableStep(tableSize) as U32;
    let mut symbolNext: [U16; 256] = [0; 256];
    let mut position: U32 = 0 as U32;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1 as U32);
    let largeLimit: S16 =
        ((1 as ::core::ffi::c_int) << tableLog.wrapping_sub(1 as ::core::ffi::c_uint)) as S16;
    let mut noLarge: U32 = 1 as U32;
    let mut s: U32 = 0;
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE as ::core::ffi::c_uint {
        return -(FSE_ERROR_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog > FSE_MAX_TABLELOG as ::core::ffi::c_uint {
        return -(FSE_ERROR_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    (*DTableH.offset(0 as ::core::ffi::c_int as isize)).tableLog = tableLog as U16;
    s = 0 as U32;
    while s <= maxSymbolValue as U32 {
        if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int == -(1 as ::core::ffi::c_int)
        {
            let fresh7 = highThreshold;
            highThreshold = highThreshold.wrapping_sub(1);
            (*tableDecode.offset(fresh7 as isize)).symbol = s as BYTE as ::core::ffi::c_uchar;
            symbolNext[s as usize] = 1 as U16;
        } else {
            if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int
                >= largeLimit as ::core::ffi::c_int
            {
                noLarge = 0 as U32;
            }
            symbolNext[s as usize] = *normalizedCounter.offset(s as isize) as U16;
        }
        s = s.wrapping_add(1);
    }
    s = 0 as U32;
    while s <= maxSymbolValue as U32 {
        let mut i: ::core::ffi::c_int = 0;
        i = 0 as ::core::ffi::c_int;
        while i < *normalizedCounter.offset(s as isize) as ::core::ffi::c_int {
            (*tableDecode.offset(position as isize)).symbol = s as BYTE as ::core::ffi::c_uchar;
            position = position.wrapping_add(step) & tableMask;
            while position > highThreshold {
                position = position.wrapping_add(step) & tableMask;
            }
            i += 1;
        }
        s = s.wrapping_add(1);
    }
    if position != 0 as U32 {
        return -(FSE_ERROR_GENERIC as ::core::ffi::c_int) as size_t;
    }
    let mut i_0: U32 = 0;
    i_0 = 0 as U32;
    while i_0 < tableSize {
        let mut symbol: BYTE = (*tableDecode.offset(i_0 as isize)).symbol as BYTE;
        let fresh8 = symbolNext[symbol as usize];
        symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
        let mut nextState: U16 = fresh8;
        (*tableDecode.offset(i_0 as isize)).nbBits =
            tableLog.wrapping_sub(FSE_highbit32(nextState as U32)) as BYTE as ::core::ffi::c_uchar;
        (*tableDecode.offset(i_0 as isize)).newState = (((nextState as ::core::ffi::c_int)
            << (*tableDecode.offset(i_0 as isize)).nbBits as ::core::ffi::c_int)
            as U32)
            .wrapping_sub(tableSize) as U16
            as ::core::ffi::c_ushort;
        i_0 = i_0.wrapping_add(1);
    }
    (*DTableH).fastMode = noLarge as U16;
    return 0 as size_t;
}
unsafe extern "C" fn FSE_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(FSE_ERROR_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
unsafe extern "C" fn FSE_abs(mut a: ::core::ffi::c_short) -> ::core::ffi::c_short {
    return (if (a as ::core::ffi::c_int) < 0 as ::core::ffi::c_int {
        -(a as ::core::ffi::c_int)
    } else {
        a as ::core::ffi::c_int
    }) as ::core::ffi::c_short;
}
unsafe extern "C" fn FSE_readNCount(
    mut normalizedCounter: *mut ::core::ffi::c_short,
    mut maxSVPtr: *mut ::core::ffi::c_uint,
    mut tableLogPtr: *mut ::core::ffi::c_uint,
    mut headerBuffer: *const ::core::ffi::c_void,
    mut hbSize: size_t,
) -> size_t {
    let istart: *const BYTE = headerBuffer as *const BYTE;
    let iend: *const BYTE = istart.offset(hbSize as isize);
    let mut ip: *const BYTE = istart;
    let mut nbBits: ::core::ffi::c_int = 0;
    let mut remaining: ::core::ffi::c_int = 0;
    let mut threshold: ::core::ffi::c_int = 0;
    let mut bitStream: U32 = 0;
    let mut bitCount: ::core::ffi::c_int = 0;
    let mut charnum: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut previous0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if hbSize < 4 as size_t {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    bitStream = FSE_readLE32(ip as *const ::core::ffi::c_void);
    nbBits = (bitStream & 0xf as U32).wrapping_add(FSE_MIN_TABLELOG as U32) as ::core::ffi::c_int;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX {
        return -(FSE_ERROR_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    bitStream >>= 4 as ::core::ffi::c_int;
    bitCount = 4 as ::core::ffi::c_int;
    *tableLogPtr = nbBits as ::core::ffi::c_uint;
    remaining = ((1 as ::core::ffi::c_int) << nbBits) + 1 as ::core::ffi::c_int;
    threshold = (1 as ::core::ffi::c_int) << nbBits;
    nbBits += 1;
    while remaining > 1 as ::core::ffi::c_int && charnum <= *maxSVPtr {
        if previous0 != 0 {
            let mut n0: ::core::ffi::c_uint = charnum;
            while bitStream & 0xffff as U32 == 0xffff as U32 {
                n0 = n0.wrapping_add(24 as ::core::ffi::c_uint);
                if ip < iend.offset(-(5 as ::core::ffi::c_int as isize)) {
                    ip = ip.offset(2 as ::core::ffi::c_int as isize);
                    bitStream = FSE_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
                } else {
                    bitStream >>= 16 as ::core::ffi::c_int;
                    bitCount += 16 as ::core::ffi::c_int;
                }
            }
            while bitStream & 3 as U32 == 3 as U32 {
                n0 = n0.wrapping_add(3 as ::core::ffi::c_uint);
                bitStream >>= 2 as ::core::ffi::c_int;
                bitCount += 2 as ::core::ffi::c_int;
            }
            n0 = n0.wrapping_add((bitStream & 3 as U32) as ::core::ffi::c_uint);
            bitCount += 2 as ::core::ffi::c_int;
            if n0 > *maxSVPtr {
                return -(FSE_ERROR_maxSymbolValue_tooSmall as ::core::ffi::c_int) as size_t;
            }
            while charnum < n0 {
                let fresh9 = charnum;
                charnum = charnum.wrapping_add(1);
                *normalizedCounter.offset(fresh9 as isize) = 0 as ::core::ffi::c_short;
            }
            if ip <= iend.offset(-(7 as ::core::ffi::c_int as isize))
                || ip.offset((bitCount >> 3 as ::core::ffi::c_int) as isize)
                    <= iend.offset(-(4 as ::core::ffi::c_int as isize))
            {
                ip = ip.offset((bitCount >> 3 as ::core::ffi::c_int) as isize);
                bitCount &= 7 as ::core::ffi::c_int;
                bitStream = FSE_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
            } else {
                bitStream >>= 2 as ::core::ffi::c_int;
            }
        }
        let max: ::core::ffi::c_short = (2 as ::core::ffi::c_int * threshold
            - 1 as ::core::ffi::c_int
            - remaining) as ::core::ffi::c_short;
        let mut count: ::core::ffi::c_short = 0;
        if (bitStream & (threshold - 1 as ::core::ffi::c_int) as U32) < max as U32 {
            count =
                (bitStream & (threshold - 1 as ::core::ffi::c_int) as U32) as ::core::ffi::c_short;
            bitCount += nbBits - 1 as ::core::ffi::c_int;
        } else {
            count = (bitStream
                & (2 as ::core::ffi::c_int * threshold - 1 as ::core::ffi::c_int) as U32)
                as ::core::ffi::c_short;
            if count as ::core::ffi::c_int >= threshold {
                count = (count as ::core::ffi::c_int - max as ::core::ffi::c_int)
                    as ::core::ffi::c_short;
            }
            bitCount += nbBits;
        }
        count -= 1;
        remaining -= FSE_abs(count) as ::core::ffi::c_int;
        let fresh10 = charnum;
        charnum = charnum.wrapping_add(1);
        *normalizedCounter.offset(fresh10 as isize) = count;
        previous0 = (count == 0) as ::core::ffi::c_int;
        while remaining < threshold {
            nbBits -= 1;
            threshold >>= 1 as ::core::ffi::c_int;
        }
        if ip <= iend.offset(-(7 as ::core::ffi::c_int as isize))
            || ip.offset((bitCount >> 3 as ::core::ffi::c_int) as isize)
                <= iend.offset(-(4 as ::core::ffi::c_int as isize))
        {
            ip = ip.offset((bitCount >> 3 as ::core::ffi::c_int) as isize);
            bitCount &= 7 as ::core::ffi::c_int;
        } else {
            bitCount -= (8 as ::core::ffi::c_long
                * iend
                    .offset(-(4 as ::core::ffi::c_int as isize))
                    .offset_from(ip) as ::core::ffi::c_long)
                as ::core::ffi::c_int;
            ip = iend.offset(-(4 as ::core::ffi::c_int as isize));
        }
        bitStream =
            FSE_readLE32(ip as *const ::core::ffi::c_void) >> (bitCount & 31 as ::core::ffi::c_int);
    }
    if remaining != 1 as ::core::ffi::c_int {
        return -(FSE_ERROR_GENERIC as ::core::ffi::c_int) as size_t;
    }
    *maxSVPtr = charnum.wrapping_sub(1 as ::core::ffi::c_uint);
    ip = ip.offset((bitCount + 7 as ::core::ffi::c_int >> 3 as ::core::ffi::c_int) as isize);
    if ip.offset_from(istart) as ::core::ffi::c_long as size_t > hbSize {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn FSE_buildDTable_rle(mut dt: *mut FSE_DTable, mut symbolValue: BYTE) -> size_t {
    let mut ptr: *mut ::core::ffi::c_void = dt as *mut ::core::ffi::c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let cell: *mut FSE_decode_t =
        (ptr as *mut FSE_decode_t).offset(1 as ::core::ffi::c_int as isize);
    (*DTableH).tableLog = 0 as U16;
    (*DTableH).fastMode = 0 as U16;
    (*cell).newState = 0 as ::core::ffi::c_ushort;
    (*cell).symbol = symbolValue as ::core::ffi::c_uchar;
    (*cell).nbBits = 0 as ::core::ffi::c_uchar;
    return 0 as size_t;
}
unsafe extern "C" fn FSE_buildDTable_raw(
    mut dt: *mut FSE_DTable,
    mut nbBits: ::core::ffi::c_uint,
) -> size_t {
    let mut ptr: *mut ::core::ffi::c_void = dt as *mut ::core::ffi::c_void;
    let DTableH: *mut FSE_DTableHeader = ptr as *mut FSE_DTableHeader;
    let dinfo: *mut FSE_decode_t =
        (ptr as *mut FSE_decode_t).offset(1 as ::core::ffi::c_int as isize);
    let tableSize: ::core::ffi::c_uint =
        ((1 as ::core::ffi::c_int) << nbBits) as ::core::ffi::c_uint;
    let tableMask: ::core::ffi::c_uint = tableSize.wrapping_sub(1 as ::core::ffi::c_uint);
    let maxSymbolValue: ::core::ffi::c_uint = tableMask;
    let mut s: ::core::ffi::c_uint = 0;
    if nbBits < 1 as ::core::ffi::c_uint {
        return -(FSE_ERROR_GENERIC as ::core::ffi::c_int) as size_t;
    }
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1 as U16;
    s = 0 as ::core::ffi::c_uint;
    while s <= maxSymbolValue {
        (*dinfo.offset(s as isize)).newState = 0 as ::core::ffi::c_ushort;
        (*dinfo.offset(s as isize)).symbol = s as BYTE as ::core::ffi::c_uchar;
        (*dinfo.offset(s as isize)).nbBits = nbBits as BYTE as ::core::ffi::c_uchar;
        s = s.wrapping_add(1);
    }
    return 0 as size_t;
}
unsafe extern "C" fn FSE_initDStream(
    mut bitD: *mut FSE_DStream_t,
    mut srcBuffer: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize < 1 as size_t {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if srcSize >= ::core::mem::size_of::<size_t>() as usize {
        let mut contain32: U32 = 0;
        (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
        (*bitD).ptr = (srcBuffer as *const ::core::ffi::c_char)
            .offset(srcSize as isize)
            .offset(-(::core::mem::size_of::<size_t>() as usize as isize));
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
        contain32 =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize) as U32;
        if contain32 == 0 as U32 {
            return -(FSE_ERROR_GENERIC as ::core::ffi::c_int) as size_t;
        }
        (*bitD).bitsConsumed = (8 as ::core::ffi::c_uint).wrapping_sub(FSE_highbit32(contain32));
    } else {
        let mut contain32_0: U32 = 0;
        (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let mut current_block_19: u64;
        match srcSize {
            7 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(6 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(16 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_19 = 6957588700556279685;
            }
            6 => {
                current_block_19 = 6957588700556279685;
            }
            5 => {
                current_block_19 = 11611054839759236981;
            }
            4 => {
                current_block_19 = 11143351646552154414;
            }
            3 => {
                current_block_19 = 3925063879267423178;
            }
            2 => {
                current_block_19 = 4158518201651876567;
            }
            _ => {
                current_block_19 = 6009453772311597924;
            }
        }
        match current_block_19 {
            6957588700556279685 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(5 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(24 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_19 = 11611054839759236981;
            }
            _ => {}
        }
        match current_block_19 {
            11611054839759236981 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(4 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(32 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_19 = 11143351646552154414;
            }
            _ => {}
        }
        match current_block_19 {
            11143351646552154414 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(3 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_19 = 3925063879267423178;
            }
            _ => {}
        }
        match current_block_19 {
            3925063879267423178 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_19 = 4158518201651876567;
            }
            _ => {}
        }
        match current_block_19 {
            4158518201651876567 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(1 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
            }
            _ => {}
        }
        contain32_0 =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize) as U32;
        if contain32_0 == 0 as U32 {
            return -(FSE_ERROR_GENERIC as ::core::ffi::c_int) as size_t;
        }
        (*bitD).bitsConsumed = (8 as ::core::ffi::c_uint).wrapping_sub(FSE_highbit32(contain32_0));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((::core::mem::size_of::<size_t>() as usize).wrapping_sub(srcSize as usize) as U32)
                .wrapping_mul(8 as U32) as ::core::ffi::c_uint,
        );
    }
    return srcSize;
}
unsafe extern "C" fn FSE_lookBits(mut bitD: *mut FSE_DStream_t, mut nbBits: U32) -> size_t {
    let bitMask: U32 = (::core::mem::size_of::<size_t>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & bitMask)
        >> 1 as ::core::ffi::c_int
        >> (bitMask.wrapping_sub(nbBits) & bitMask);
}
unsafe extern "C" fn FSE_lookBitsFast(mut bitD: *mut FSE_DStream_t, mut nbBits: U32) -> size_t {
    let bitMask: U32 = (::core::mem::size_of::<size_t>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & bitMask)
        >> (bitMask.wrapping_add(1 as U32).wrapping_sub(nbBits) & bitMask);
}
unsafe extern "C" fn FSE_skipBits(mut bitD: *mut FSE_DStream_t, mut nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_add(nbBits as ::core::ffi::c_uint);
}
unsafe extern "C" fn FSE_readBits(mut bitD: *mut FSE_DStream_t, mut nbBits: U32) -> size_t {
    let mut value: size_t = FSE_lookBits(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    return value;
}
unsafe extern "C" fn FSE_readBitsFast(mut bitD: *mut FSE_DStream_t, mut nbBits: U32) -> size_t {
    let mut value: size_t = FSE_lookBitsFast(bitD, nbBits);
    FSE_skipBits(bitD, nbBits);
    return value;
}
unsafe extern "C" fn FSE_reloadDStream(mut bitD: *mut FSE_DStream_t) -> ::core::ffi::c_uint {
    if (*bitD).bitsConsumed as usize
        > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
    {
        return FSE_DStream_tooFar as ::core::ffi::c_int as ::core::ffi::c_uint;
    }
    if (*bitD).ptr
        >= (*bitD)
            .start
            .offset(::core::mem::size_of::<size_t>() as usize as isize)
    {
        (*bitD).ptr = (*bitD)
            .ptr
            .offset(-(((*bitD).bitsConsumed >> 3 as ::core::ffi::c_int) as isize));
        (*bitD).bitsConsumed &= 7 as ::core::ffi::c_uint;
        (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
        return FSE_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize)
            < (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            return FSE_DStream_endOfBuffer as ::core::ffi::c_int as ::core::ffi::c_uint;
        }
        return FSE_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint;
    }
    let mut nbBytes: U32 = (*bitD).bitsConsumed as U32 >> 3 as ::core::ffi::c_int;
    let mut result: U32 = FSE_DStream_unfinished as ::core::ffi::c_int as U32;
    if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
        nbBytes = (*bitD).ptr.offset_from((*bitD).start) as ::core::ffi::c_long as U32;
        result = FSE_DStream_endOfBuffer as ::core::ffi::c_int as U32;
    }
    (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_sub(nbBytes.wrapping_mul(8 as U32) as ::core::ffi::c_uint);
    (*bitD).bitContainer = FSE_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
    return result as ::core::ffi::c_uint;
}
unsafe extern "C" fn FSE_initDState(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut FSE_DStream_t,
    mut dt: *const FSE_DTable,
) {
    let mut ptr: *const ::core::ffi::c_void = dt as *const ::core::ffi::c_void;
    let DTableH: *const FSE_DTableHeader = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = FSE_readBits(bitD, (*DTableH).tableLog as U32);
    FSE_reloadDStream(bitD);
    (*DStatePtr).table = dt.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
}
unsafe extern "C" fn FSE_decodeSymbol(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut FSE_DStream_t,
) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let mut symbol: BYTE = DInfo.symbol as BYTE;
    let mut lowBits: size_t = FSE_readBits(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol;
}
unsafe extern "C" fn FSE_decodeSymbolFast(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut FSE_DStream_t,
) -> BYTE {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let mut symbol: BYTE = DInfo.symbol as BYTE;
    let mut lowBits: size_t = FSE_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol;
}
unsafe extern "C" fn FSE_endOfDStream(mut bitD: *const FSE_DStream_t) -> ::core::ffi::c_uint {
    return ((*bitD).ptr == (*bitD).start
        && (*bitD).bitsConsumed as usize
            == (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize))
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
unsafe extern "C" fn FSE_endOfDState(mut DStatePtr: *const FSE_DState_t) -> ::core::ffi::c_uint {
    return ((*DStatePtr).state == 0 as size_t) as ::core::ffi::c_int as ::core::ffi::c_uint;
}
#[inline(always)]
unsafe extern "C" fn FSE_decompress_usingDTable_generic(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut dt: *const FSE_DTable,
    fast: ::core::ffi::c_uint,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.offset(maxDstSize as isize);
    let olimit: *mut BYTE = omax.offset(-(3 as ::core::ffi::c_int as isize));
    let mut bitD: FSE_DStream_t = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut state1: FSE_DState_t = FSE_DState_t {
        state: 0,
        table: ::core::ptr::null::<::core::ffi::c_void>(),
    };
    let mut state2: FSE_DState_t = FSE_DState_t {
        state: 0,
        table: ::core::ptr::null::<::core::ffi::c_void>(),
    };
    let mut errorCode: size_t = 0;
    errorCode = FSE_initDStream(&raw mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    FSE_initDState(&raw mut state1, &raw mut bitD, dt);
    FSE_initDState(&raw mut state2, &raw mut bitD, dt);
    while FSE_reloadDStream(&raw mut bitD)
        == FSE_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && op < olimit
    {
        *op.offset(0 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSE_MAX_TABLELOG * 2 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            FSE_reloadDStream(&raw mut bitD);
        }
        *op.offset(1 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSE_MAX_TABLELOG * 4 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            if FSE_reloadDStream(&raw mut bitD)
                > FSE_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                op = op.offset(2 as ::core::ffi::c_int as isize);
                break;
            }
        }
        *op.offset(2 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSE_MAX_TABLELOG * 2 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            FSE_reloadDStream(&raw mut bitD);
        }
        *op.offset(3 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        op = op.offset(4 as ::core::ffi::c_int as isize);
    }
    while !(FSE_reloadDStream(&raw mut bitD)
        > FSE_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
        || op == omax
        || FSE_endOfDStream(&raw mut bitD) != 0
            && (fast != 0 || FSE_endOfDState(&raw mut state1) != 0))
    {
        let fresh11 = op;
        op = op.offset(1);
        *fresh11 = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if FSE_reloadDStream(&raw mut bitD)
            > FSE_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
            || op == omax
            || FSE_endOfDStream(&raw mut bitD) != 0
                && (fast != 0 || FSE_endOfDState(&raw mut state2) != 0)
        {
            break;
        }
        let fresh12 = op;
        op = op.offset(1);
        *fresh12 = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
    }
    if FSE_endOfDStream(&raw mut bitD) != 0
        && FSE_endOfDState(&raw mut state1) != 0
        && FSE_endOfDState(&raw mut state2) != 0
    {
        return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
    }
    if op == omax {
        return -(FSE_ERROR_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
}
unsafe extern "C" fn FSE_decompress_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut originalSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut dt: *const FSE_DTable,
) -> size_t {
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    memcpy(
        &raw mut DTableH as *mut ::core::ffi::c_void,
        dt as *const ::core::ffi::c_void,
        ::core::mem::size_of::<FSE_DTableHeader>() as size_t,
    );
    if DTableH.fastMode != 0 {
        return FSE_decompress_usingDTable_generic(
            dst,
            originalSize,
            cSrc,
            cSrcSize,
            dt,
            1 as ::core::ffi::c_uint,
        );
    }
    return FSE_decompress_usingDTable_generic(
        dst,
        originalSize,
        cSrc,
        cSrcSize,
        dt,
        0 as ::core::ffi::c_uint,
    );
}
unsafe extern "C" fn FSE_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut counting: [::core::ffi::c_short; 256] = [0; 256];
    let mut dt: DTable_max_t = [0; 4097];
    let mut tableLog: ::core::ffi::c_uint = 0;
    let mut maxSymbolValue: ::core::ffi::c_uint = FSE_MAX_SYMBOL_VALUE as ::core::ffi::c_uint;
    let mut errorCode: size_t = 0;
    if cSrcSize < 2 as size_t {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    errorCode = FSE_readNCount(
        &raw mut counting as *mut ::core::ffi::c_short,
        &raw mut maxSymbolValue,
        &raw mut tableLog,
        istart as *const ::core::ffi::c_void,
        cSrcSize,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(errorCode as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(errorCode as ::core::ffi::c_ulong)
        as size_t as size_t;
    errorCode = FSE_buildDTable(
        &raw mut dt as *mut FSE_DTable,
        &raw mut counting as *mut ::core::ffi::c_short,
        maxSymbolValue,
        tableLog,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    return FSE_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        &raw mut dt as *mut U32,
    );
}
pub const HUF_MAX_SYMBOL_VALUE: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const HUF_MAX_TABLELOG: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const HUF_ABSOLUTEMAX_TABLELOG: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_readDTable(
    mut DTable: *mut U16,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut huffWeight: [BYTE; 256] = [0; 256];
    let mut rankVal: [U32; 17] = [0; 17];
    let mut weightTotal: U32 = 0;
    let mut maxBits: U32 = 0;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: size_t = 0;
    let mut oSize: size_t = 0;
    let mut n: U32 = 0;
    let mut nextRankStart: U32 = 0;
    let mut ptr: *mut ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dt: *mut HUF_DElt = ptr as *mut HUF_DElt;
    if srcSize == 0 {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    iSize = *ip.offset(0 as ::core::ffi::c_int as isize) as size_t;
    if iSize >= 128 as size_t {
        if iSize >= 242 as size_t {
            static mut l: [::core::ffi::c_int; 14] = [
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
                4 as ::core::ffi::c_int,
                7 as ::core::ffi::c_int,
                8 as ::core::ffi::c_int,
                15 as ::core::ffi::c_int,
                16 as ::core::ffi::c_int,
                31 as ::core::ffi::c_int,
                32 as ::core::ffi::c_int,
                63 as ::core::ffi::c_int,
                64 as ::core::ffi::c_int,
                127 as ::core::ffi::c_int,
                128 as ::core::ffi::c_int,
            ];
            oSize = l[iSize.wrapping_sub(242 as size_t) as usize] as size_t;
            memset(
                &raw mut huffWeight as *mut BYTE as *mut ::core::ffi::c_void,
                1 as ::core::ffi::c_int,
                ::core::mem::size_of::<[BYTE; 256]>() as size_t,
            );
            iSize = 0 as size_t;
        } else {
            oSize = iSize.wrapping_sub(127 as size_t);
            iSize = oSize.wrapping_add(1 as size_t).wrapping_div(2 as size_t);
            if iSize.wrapping_add(1 as size_t) > srcSize {
                return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            ip = ip.offset(1 as ::core::ffi::c_int as isize);
            n = 0 as U32;
            while (n as size_t) < oSize {
                huffWeight[n as usize] = (*ip.offset(n.wrapping_div(2 as U32) as isize)
                    as ::core::ffi::c_int
                    >> 4 as ::core::ffi::c_int) as BYTE;
                huffWeight[n.wrapping_add(1 as U32) as usize] =
                    (*ip.offset(n.wrapping_div(2 as U32) as isize) as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int) as BYTE;
                n = (n as ::core::ffi::c_uint).wrapping_add(2 as ::core::ffi::c_uint) as U32 as U32;
            }
        }
    } else {
        if iSize.wrapping_add(1 as size_t) > srcSize {
            return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        oSize = FSE_decompress(
            &raw mut huffWeight as *mut BYTE as *mut ::core::ffi::c_void,
            HUF_MAX_SYMBOL_VALUE as size_t,
            ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            iSize,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }
    memset(
        &raw mut rankVal as *mut U32 as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[U32; 17]>() as size_t,
    );
    weightTotal = 0 as U32;
    n = 0 as U32;
    while (n as size_t) < oSize {
        if huffWeight[n as usize] as ::core::ffi::c_int >= HUF_ABSOLUTEMAX_TABLELOG {
            return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
        }
        rankVal[huffWeight[n as usize] as usize] =
            rankVal[huffWeight[n as usize] as usize].wrapping_add(1);
        weightTotal = (weightTotal as ::core::ffi::c_uint).wrapping_add(
            ((1 as ::core::ffi::c_int) << huffWeight[n as usize] as ::core::ffi::c_int
                >> 1 as ::core::ffi::c_int) as ::core::ffi::c_uint,
        ) as U32 as U32;
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 as U32 {
        return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
    }
    maxBits = FSE_highbit32(weightTotal).wrapping_add(1 as ::core::ffi::c_uint) as U32;
    if maxBits > *DTable.offset(0 as ::core::ffi::c_int as isize) as U32 {
        return -(FSE_ERROR_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    *DTable.offset(0 as ::core::ffi::c_int as isize) = maxBits as U16;
    let mut total: U32 = ((1 as ::core::ffi::c_int) << maxBits) as U32;
    let mut rest: U32 = total.wrapping_sub(weightTotal);
    let mut verif: U32 = ((1 as ::core::ffi::c_int) << FSE_highbit32(rest)) as U32;
    let mut lastWeight: U32 = (FSE_highbit32(rest) as U32).wrapping_add(1 as U32);
    if verif != rest {
        return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
    }
    huffWeight[oSize as usize] = lastWeight as BYTE;
    rankVal[lastWeight as usize] = rankVal[lastWeight as usize].wrapping_add(1);
    if rankVal[1 as ::core::ffi::c_int as usize] < 2 as U32
        || rankVal[1 as ::core::ffi::c_int as usize] & 1 as U32 != 0
    {
        return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
    }
    nextRankStart = 0 as U32;
    n = 1 as U32;
    while n <= maxBits {
        let mut current: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add((rankVal[n as usize] << n.wrapping_sub(1 as U32)) as ::core::ffi::c_uint)
            as U32 as U32;
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }
    n = 0 as U32;
    while n as size_t <= oSize {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = ((1 as ::core::ffi::c_int) << w >> 1 as ::core::ffi::c_int) as U32;
        let mut i: U32 = 0;
        let mut D: HUF_DElt = HUF_DElt { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = maxBits.wrapping_add(1 as U32).wrapping_sub(w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.offset(i as isize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = (rankVal[w as usize] as ::core::ffi::c_uint)
            .wrapping_add(length as ::core::ffi::c_uint) as U32
            as U32;
        n = n.wrapping_add(1);
    }
    return iSize.wrapping_add(1 as size_t);
}
unsafe extern "C" fn HUF_decodeSymbol(
    mut Dstream: *mut FSE_DStream_t,
    mut dt: *const HUF_DElt,
    dtLog: U32,
) -> BYTE {
    let val: size_t = FSE_lookBitsFast(Dstream, dtLog) as size_t;
    let c: BYTE = (*dt.offset(val as isize)).byte;
    FSE_skipBits(Dstream, (*dt.offset(val as isize)).nbBits as U32);
    return c;
}
unsafe extern "C" fn HUF_decompress_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const U16,
) -> size_t {
    if cSrcSize < 6 as size_t {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.offset(maxDstSize as isize);
    let olimit: *mut BYTE = if maxDstSize < 15 as size_t {
        op
    } else {
        omax.offset(-(15 as ::core::ffi::c_int as isize))
    };
    let mut ptr: *const ::core::ffi::c_void = DTable as *const ::core::ffi::c_void;
    let dt: *const HUF_DElt = (ptr as *const HUF_DElt).offset(1 as ::core::ffi::c_int as isize);
    let dtLog: U32 = *DTable.offset(0 as ::core::ffi::c_int as isize) as U32;
    let mut errorCode: size_t = 0;
    let mut reloadStatus: U32 = 0;
    let mut jumpTable: *const U16 = cSrc as *const U16;
    let length1: size_t = FSE_readLE16(jumpTable as *const ::core::ffi::c_void) as size_t;
    let length2: size_t = FSE_readLE16(
        jumpTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void
    ) as size_t;
    let length3: size_t = FSE_readLE16(
        jumpTable.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void
    ) as size_t;
    let length4: size_t = cSrcSize
        .wrapping_sub(6 as size_t)
        .wrapping_sub(length1)
        .wrapping_sub(length2)
        .wrapping_sub(length3);
    let start1: *const ::core::ffi::c_char =
        (cSrc as *const ::core::ffi::c_char).offset(6 as ::core::ffi::c_int as isize);
    let start2: *const ::core::ffi::c_char = start1.offset(length1 as isize);
    let start3: *const ::core::ffi::c_char = start2.offset(length2 as isize);
    let start4: *const ::core::ffi::c_char = start3.offset(length3 as isize);
    let mut bitD1: FSE_DStream_t = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: FSE_DStream_t = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: FSE_DStream_t = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: FSE_DStream_t = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    if length1
        .wrapping_add(length2)
        .wrapping_add(length3)
        .wrapping_add(6 as size_t)
        >= cSrcSize
    {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    errorCode = FSE_initDStream(
        &raw mut bitD1,
        start1 as *const ::core::ffi::c_void,
        length1,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = FSE_initDStream(
        &raw mut bitD2,
        start2 as *const ::core::ffi::c_void,
        length2,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = FSE_initDStream(
        &raw mut bitD3,
        start3 as *const ::core::ffi::c_void,
        length3,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = FSE_initDStream(
        &raw mut bitD4,
        start4 as *const ::core::ffi::c_void,
        length4,
    );
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    reloadStatus = FSE_reloadDStream(&raw mut bitD2) as U32;
    while reloadStatus < FSE_DStream_completed as ::core::ffi::c_int as U32 && op < olimit {
        *op.offset(0 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD1, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD1);
        }
        *op.offset(1 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD2, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD2);
        }
        *op.offset(2 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD3, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD3);
        }
        *op.offset(3 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD4, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD4);
        }
        *op.offset(4 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD1, dt, dtLog);
        if FSE_32bits() != 0 {
            FSE_reloadDStream(&raw mut bitD1);
        }
        *op.offset(5 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD2, dt, dtLog);
        if FSE_32bits() != 0 {
            FSE_reloadDStream(&raw mut bitD2);
        }
        *op.offset(6 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD3, dt, dtLog);
        if FSE_32bits() != 0 {
            FSE_reloadDStream(&raw mut bitD3);
        }
        *op.offset(7 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD4, dt, dtLog);
        if FSE_32bits() != 0 {
            FSE_reloadDStream(&raw mut bitD4);
        }
        *op.offset(8 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD1, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD1);
        }
        *op.offset(9 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD2, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD2);
        }
        *op.offset(10 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD3, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD3);
        }
        *op.offset(11 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD4, dt, dtLog);
        if FSE_32bits() != 0 && HUF_MAX_TABLELOG > 12 as ::core::ffi::c_int {
            FSE_reloadDStream(&raw mut bitD4);
        }
        *op.offset(12 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD1, dt, dtLog);
        *op.offset(13 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD2, dt, dtLog);
        *op.offset(14 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD3, dt, dtLog);
        *op.offset(15 as ::core::ffi::c_int as isize) = HUF_decodeSymbol(&raw mut bitD4, dt, dtLog);
        op = op.offset(16 as ::core::ffi::c_int as isize);
        reloadStatus = (FSE_reloadDStream(&raw mut bitD2)
            | FSE_reloadDStream(&raw mut bitD3)
            | FSE_reloadDStream(&raw mut bitD4)) as U32;
        FSE_reloadDStream(&raw mut bitD1);
    }
    if reloadStatus != FSE_DStream_completed as ::core::ffi::c_int as U32 {
        return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
    }
    let mut bitTail: FSE_DStream_t = FSE_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    bitTail.ptr = bitD1.ptr;
    bitTail.bitsConsumed = bitD1.bitsConsumed;
    bitTail.bitContainer = bitD1.bitContainer;
    bitTail.start = start1;
    while FSE_reloadDStream(&raw mut bitTail)
        < FSE_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
        && op < omax
    {
        *op.offset(0 as ::core::ffi::c_int as isize) =
            HUF_decodeSymbol(&raw mut bitTail, dt, dtLog);
        op = op.offset(1);
    }
    if FSE_endOfDStream(&raw mut bitTail) != 0 {
        return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
    }
    if op == omax {
        return -(FSE_ERROR_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    return -(FSE_ERROR_corruptionDetected as ::core::ffi::c_int) as size_t;
}
unsafe extern "C" fn HUF_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable: [::core::ffi::c_ushort; 4097] = [
        12 as ::core::ffi::c_int as ::core::ffi::c_ushort,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let mut errorCode: size_t = 0;
    errorCode = HUF_readDTable(&raw mut DTable as *mut U16, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return -(FSE_ERROR_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(errorCode as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(errorCode as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        &raw mut DTable as *mut ::core::ffi::c_ushort,
    );
}
static mut ZSTD_magicNumber: U32 = 0xfd2fb51e as U32;
pub const BLOCKSIZE: ::core::ffi::c_int =
    128 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int);
pub const MINMATCH: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const MLbits: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const LLbits: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const Offbits: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const MaxML: ::core::ffi::c_int =
    ((1 as ::core::ffi::c_int) << MLbits) - 1 as ::core::ffi::c_int;
pub const MaxLL: ::core::ffi::c_int =
    ((1 as ::core::ffi::c_int) << LLbits) - 1 as ::core::ffi::c_int;
pub const MaxOff: ::core::ffi::c_int =
    ((1 as ::core::ffi::c_int) << Offbits) - 1 as ::core::ffi::c_int;
pub const MLFSELog: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const LLFSELog: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const OffFSELog: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const ZSTD_CONTENTSIZE_ERROR: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(2 as ::core::ffi::c_ulonglong);
static mut ZSTD_blockHeaderSize: size_t = 3 as size_t;
static mut ZSTD_frameHeaderSize: size_t = 4 as size_t;
unsafe extern "C" fn ZSTD_32bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize == 4 as usize)
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
unsafe extern "C" fn ZSTD_isLittleEndian() -> ::core::ffi::c_uint {
    let one: C2RustUnnamed = C2RustUnnamed {
        i: 1 as ::core::ffi::c_int as U32,
    };
    return one.c[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint;
}
unsafe extern "C" fn ZSTD_read16(mut p: *const ::core::ffi::c_void) -> U16 {
    let mut r: U16 = 0;
    memcpy(
        &raw mut r as *mut ::core::ffi::c_void,
        p,
        ::core::mem::size_of::<U16>() as size_t,
    );
    return r;
}
unsafe extern "C" fn ZSTD_copy4(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    memcpy(dst, src, 4 as size_t);
}
unsafe extern "C" fn ZSTD_copy8(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    memcpy(dst, src, 8 as size_t);
}
unsafe extern "C" fn ZSTD_wildcopy(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
    mut length: ptrdiff_t,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.offset(length as isize);
    while op < oend {
        ZSTD_copy8(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
        );
        op = op.offset(8 as ::core::ffi::c_int as isize);
        ip = ip.offset(8 as ::core::ffi::c_int as isize);
    }
}
unsafe extern "C" fn ZSTD_readLE16(mut memPtr: *const ::core::ffi::c_void) -> U16 {
    if ZSTD_isLittleEndian() != 0 {
        return ZSTD_read16(memPtr);
    } else {
        let mut p: *const BYTE = memPtr as *const BYTE;
        return (*p.offset(0 as ::core::ffi::c_int as isize) as U16 as ::core::ffi::c_int
            + ((*p.offset(1 as ::core::ffi::c_int as isize) as U16 as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int)) as U16;
    };
}
unsafe extern "C" fn ZSTD_readLE24(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    return (ZSTD_readLE16(memPtr) as ::core::ffi::c_int
        + ((*(memPtr as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int)) as U32;
}
unsafe extern "C" fn ZSTD_readBE32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    let mut p: *const BYTE = memPtr as *const BYTE;
    return ((*p.offset(0 as ::core::ffi::c_int as isize) as U32) << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*p.offset(1 as ::core::ffi::c_int as isize) as U32) << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*p.offset(2 as ::core::ffi::c_int as isize) as U32) << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*p.offset(3 as ::core::ffi::c_int as isize) as U32) << 0 as ::core::ffi::c_int,
        );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
unsafe extern "C" fn ZSTDv01_getcBlockSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut bpPtr: *mut blockProperties_t,
) -> size_t {
    let in_0: *const BYTE = src as *const BYTE;
    let mut headerFlags: BYTE = 0;
    let mut cSize: U32 = 0;
    if srcSize < 3 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    headerFlags = *in_0;
    cSize = (*in_0.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        + ((*in_0.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int)
        + ((*in_0.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 7 as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int)) as U32;
    (*bpPtr).blockType =
        (headerFlags as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as blockType_t;
    (*bpPtr).origSize = if (*bpPtr).blockType as ::core::ffi::c_uint
        == bt_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        cSize
    } else {
        0 as U32
    };
    if (*bpPtr).blockType as ::core::ffi::c_uint
        == bt_end as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 0 as size_t;
    }
    if (*bpPtr).blockType as ::core::ffi::c_uint
        == bt_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as size_t;
    }
    return cSize as size_t;
}
unsafe extern "C" fn ZSTD_copyUncompressedBlock(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize > maxDstSize {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if srcSize > 0 as size_t {
        memcpy(dst, src, srcSize);
    }
    return srcSize;
}
unsafe extern "C" fn ZSTD_decompressLiterals(
    mut ctx: *mut ::core::ffi::c_void,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.offset(maxDstSize as isize);
    let mut ip: *const BYTE = src as *const BYTE;
    let mut errorCode: size_t = 0;
    let mut litSize: size_t = 0;
    if srcSize <= 3 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    litSize = (*ip.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        + ((*ip.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int)) as size_t;
    litSize = (litSize as ::core::ffi::c_ulong).wrapping_add(
        ((*ip.offset(-(3 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
            >> 3 as ::core::ffi::c_int
            & 7 as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
    ) as size_t as size_t;
    op = oend.offset(-(litSize as isize));
    if litSize > maxDstSize {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    errorCode = HUF_decompress(
        op as *mut ::core::ffi::c_void,
        litSize,
        ip.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
        srcSize.wrapping_sub(2 as size_t),
    );
    if FSE_isError(errorCode) != 0 {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    return litSize;
}
unsafe extern "C" fn ZSTDv01_decodeLiteralsBlock(
    mut ctx: *mut ::core::ffi::c_void,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut litStart: *mut *const BYTE,
    mut litSize: *mut size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(maxDstSize as isize);
    let mut litbp: blockProperties_t = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };
    let mut litcSize: size_t = ZSTDv01_getcBlockSize(src, srcSize, &raw mut litbp);
    if ZSTDv01_isError(litcSize) != 0 {
        return litcSize;
    }
    if litcSize > srcSize.wrapping_sub(ZSTD_blockHeaderSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(ZSTD_blockHeaderSize as isize);
    match litbp.blockType as ::core::ffi::c_uint {
        1 => {
            *litStart = ip;
            ip = ip.offset(litcSize as isize);
            *litSize = litcSize;
        }
        2 => {
            let mut rleSize: size_t = litbp.origSize as size_t;
            if rleSize > maxDstSize {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            if srcSize == 0 {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            if rleSize > 0 as size_t {
                memset(
                    oend.offset(-(rleSize as isize)) as *mut ::core::ffi::c_void,
                    *ip as ::core::ffi::c_int,
                    rleSize,
                );
            }
            *litStart = oend.offset(-(rleSize as isize));
            *litSize = rleSize;
            ip = ip.offset(1);
        }
        0 => {
            let mut decodedLitSize: size_t = ZSTD_decompressLiterals(
                ctx,
                dst,
                maxDstSize,
                ip as *const ::core::ffi::c_void,
                litcSize,
            );
            if ZSTDv01_isError(decodedLitSize) != 0 {
                return decodedLitSize;
            }
            *litStart = oend.offset(-(decodedLitSize as isize));
            *litSize = decodedLitSize;
            ip = ip.offset(litcSize as isize);
        }
        3 | _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
    }
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTDv01_decodeSeqHeaders(
    mut nbSeq: *mut ::core::ffi::c_int,
    mut dumpsPtr: *mut *const BYTE,
    mut dumpsLengthPtr: *mut size_t,
    mut DTableLL: *mut FSE_DTable,
    mut DTableML: *mut FSE_DTable,
    mut DTableOffb: *mut FSE_DTable,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let mut LLtype: U32 = 0;
    let mut Offtype: U32 = 0;
    let mut MLtype: U32 = 0;
    let mut LLlog: U32 = 0;
    let mut Offlog: U32 = 0;
    let mut MLlog: U32 = 0;
    let mut dumpsLength: size_t = 0;
    if srcSize < 5 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    *nbSeq = ZSTD_readLE16(ip as *const ::core::ffi::c_void) as ::core::ffi::c_int;
    ip = ip.offset(2 as ::core::ffi::c_int as isize);
    LLtype = (*ip as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as U32;
    Offtype =
        (*ip as ::core::ffi::c_int >> 4 as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    MLtype =
        (*ip as ::core::ffi::c_int >> 2 as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    if *ip as ::core::ffi::c_int & 2 as ::core::ffi::c_int != 0 {
        dumpsLength = *ip.offset(2 as ::core::ffi::c_int as isize) as size_t;
        dumpsLength = (dumpsLength as ::core::ffi::c_ulong).wrapping_add(
            ((*ip.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        ip = ip.offset(3 as ::core::ffi::c_int as isize);
    } else {
        dumpsLength = *ip.offset(1 as ::core::ffi::c_int as isize) as size_t;
        dumpsLength = (dumpsLength as ::core::ffi::c_ulong).wrapping_add(
            ((*ip.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 1 as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        ip = ip.offset(2 as ::core::ffi::c_int as isize);
    }
    *dumpsPtr = ip;
    ip = ip.offset(dumpsLength as isize);
    *dumpsLengthPtr = dumpsLength;
    if ip > iend.offset(-(3 as ::core::ffi::c_int as isize)) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let mut norm: [S16; 128] = [0; 128];
    let mut headerSize: size_t = 0;
    match LLtype {
        2 => {
            LLlog = 0 as U32;
            let fresh4 = ip;
            ip = ip.offset(1);
            FSE_buildDTable_rle(DTableLL, *fresh4);
        }
        1 => {
            LLlog = LLbits as U32;
            FSE_buildDTable_raw(DTableLL, LLbits as ::core::ffi::c_uint);
        }
        _ => {
            let mut max: U32 = MaxLL as U32;
            headerSize = FSE_readNCount(
                &raw mut norm as *mut ::core::ffi::c_short,
                &raw mut max,
                &raw mut LLlog,
                ip as *const ::core::ffi::c_void,
                iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            );
            if FSE_isError(headerSize) != 0 {
                return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
            }
            if LLlog > LLFSELog as U32 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            ip = ip.offset(headerSize as isize);
            FSE_buildDTable(
                DTableLL,
                &raw mut norm as *mut S16,
                max as ::core::ffi::c_uint,
                LLlog as ::core::ffi::c_uint,
            );
        }
    }
    match Offtype {
        2 => {
            Offlog = 0 as U32;
            if ip > iend.offset(-(2 as ::core::ffi::c_int as isize)) {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            let fresh5 = ip;
            ip = ip.offset(1);
            FSE_buildDTable_rle(DTableOffb, *fresh5);
        }
        1 => {
            Offlog = Offbits as U32;
            FSE_buildDTable_raw(DTableOffb, Offbits as ::core::ffi::c_uint);
        }
        _ => {
            let mut max_0: U32 = MaxOff as U32;
            headerSize = FSE_readNCount(
                &raw mut norm as *mut ::core::ffi::c_short,
                &raw mut max_0,
                &raw mut Offlog,
                ip as *const ::core::ffi::c_void,
                iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            );
            if FSE_isError(headerSize) != 0 {
                return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
            }
            if Offlog > OffFSELog as U32 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            ip = ip.offset(headerSize as isize);
            FSE_buildDTable(
                DTableOffb,
                &raw mut norm as *mut S16,
                max_0 as ::core::ffi::c_uint,
                Offlog as ::core::ffi::c_uint,
            );
        }
    }
    match MLtype {
        2 => {
            MLlog = 0 as U32;
            if ip > iend.offset(-(2 as ::core::ffi::c_int as isize)) {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            let fresh6 = ip;
            ip = ip.offset(1);
            FSE_buildDTable_rle(DTableML, *fresh6);
        }
        1 => {
            MLlog = MLbits as U32;
            FSE_buildDTable_raw(DTableML, MLbits as ::core::ffi::c_uint);
        }
        _ => {
            let mut max_1: U32 = MaxML as U32;
            headerSize = FSE_readNCount(
                &raw mut norm as *mut ::core::ffi::c_short,
                &raw mut max_1,
                &raw mut MLlog,
                ip as *const ::core::ffi::c_void,
                iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            );
            if FSE_isError(headerSize) != 0 {
                return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
            }
            if MLlog > MLFSELog as U32 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            ip = ip.offset(headerSize as isize);
            FSE_buildDTable(
                DTableML,
                &raw mut norm as *mut S16,
                max_1 as ::core::ffi::c_uint,
                MLlog as ::core::ffi::c_uint,
            );
        }
    }
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decodeSequence(mut seq: *mut seq_t, mut seqState: *mut seqState_t) {
    let mut litLength: size_t = 0;
    let mut prevOffset: size_t = 0;
    let mut offset: size_t = 0;
    let mut matchLength: size_t = 0;
    let mut dumps: *const BYTE = (*seqState).dumps;
    let de: *const BYTE = (*seqState).dumpsEnd;
    litLength =
        FSE_decodeSymbol(&raw mut (*seqState).stateLL, &raw mut (*seqState).DStream) as size_t;
    prevOffset = if litLength != 0 {
        (*seq).offset
    } else {
        (*seqState).prevOffset
    };
    (*seqState).prevOffset = (*seq).offset;
    if litLength == MaxLL as size_t {
        let add: U32 = (if dumps < de {
            let fresh2 = dumps;
            dumps = dumps.offset(1);
            *fresh2 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) as U32;
        if add < 255 as U32 {
            litLength = (litLength as ::core::ffi::c_ulong)
                .wrapping_add(add as ::core::ffi::c_ulong) as size_t
                as size_t;
        } else if dumps <= de.offset(-(3 as ::core::ffi::c_int as isize)) {
            litLength = ZSTD_readLE24(dumps as *const ::core::ffi::c_void) as size_t;
            dumps = dumps.offset(3 as ::core::ffi::c_int as isize);
        }
    }
    let mut offsetCode: U32 = 0;
    let mut nbBits: U32 = 0;
    offsetCode =
        FSE_decodeSymbol(&raw mut (*seqState).stateOffb, &raw mut (*seqState).DStream) as U32;
    if ZSTD_32bits() != 0 {
        FSE_reloadDStream(&raw mut (*seqState).DStream);
    }
    nbBits = offsetCode.wrapping_sub(1 as U32);
    if offsetCode == 0 as U32 {
        nbBits = 0 as U32;
    }
    offset = ((1 as ::core::ffi::c_int as size_t)
        << (nbBits as usize
            & (::core::mem::size_of::<size_t>() as usize)
                .wrapping_mul(8 as usize)
                .wrapping_sub(1 as usize)))
    .wrapping_add(FSE_readBits(&raw mut (*seqState).DStream, nbBits));
    if ZSTD_32bits() != 0 {
        FSE_reloadDStream(&raw mut (*seqState).DStream);
    }
    if offsetCode == 0 as U32 {
        offset = prevOffset;
    }
    matchLength =
        FSE_decodeSymbol(&raw mut (*seqState).stateML, &raw mut (*seqState).DStream) as size_t;
    if matchLength == MaxML as size_t {
        let add_0: U32 = (if dumps < de {
            let fresh3 = dumps;
            dumps = dumps.offset(1);
            *fresh3 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) as U32;
        if add_0 < 255 as U32 {
            matchLength = (matchLength as ::core::ffi::c_ulong)
                .wrapping_add(add_0 as ::core::ffi::c_ulong) as size_t
                as size_t;
        } else if dumps <= de.offset(-(3 as ::core::ffi::c_int as isize)) {
            matchLength = ZSTD_readLE24(dumps as *const ::core::ffi::c_void) as size_t;
            dumps = dumps.offset(3 as ::core::ffi::c_int as isize);
        }
    }
    matchLength = (matchLength as ::core::ffi::c_ulong)
        .wrapping_add(MINMATCH as ::core::ffi::c_ulong) as size_t as size_t;
    (*seq).litLength = litLength;
    (*seq).offset = offset;
    (*seq).matchLength = matchLength;
    (*seqState).dumps = dumps;
}
unsafe extern "C" fn ZSTD_execSequence(
    mut op: *mut BYTE,
    mut sequence: seq_t,
    mut litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *mut BYTE,
    oend: *mut BYTE,
) -> size_t {
    static mut dec32table: [::core::ffi::c_int; 8] = [
        0 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
    ];
    static mut dec64table: [::core::ffi::c_int; 8] = [
        8 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        7 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        9 as ::core::ffi::c_int,
        10 as ::core::ffi::c_int,
        11 as ::core::ffi::c_int,
    ];
    let ostart: *const BYTE = op;
    let oLitEnd: *mut BYTE = op.offset(sequence.litLength as isize);
    let litLength: size_t = sequence.litLength;
    let endMatch: *mut BYTE = op
        .offset(litLength as isize)
        .offset(sequence.matchLength as isize);
    let litEnd: *const BYTE = (*litPtr).offset(litLength as isize);
    let seqLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    if seqLength > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if sequence.offset > oLitEnd.offset_from(base) as ::core::ffi::c_long as U32 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if endMatch > oend {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if litEnd > litLimit {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if sequence.matchLength > (*litPtr).offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    ::libc::memmove(
        op as *mut ::core::ffi::c_void,
        *litPtr as *const ::core::ffi::c_void,
        sequence.litLength as ::libc::size_t,
    );
    op = op.offset(litLength as isize);
    *litPtr = litEnd;
    if (oend.offset_from(op) as ::core::ffi::c_long) < 8 as ::core::ffi::c_long {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    let overlapRisk: U32 = ((litEnd.offset_from(endMatch) as ::core::ffi::c_long as size_t)
        < 12 as size_t) as ::core::ffi::c_int as U32;
    let mut match_0: *const BYTE = op.offset(-(sequence.offset as isize));
    let mut qutt: size_t = 12 as size_t;
    let mut saved: [U64; 2] = [0; 2];
    if match_0 < base as *const BYTE {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if sequence.offset > base as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if overlapRisk != 0 {
        if endMatch.offset(qutt as isize) > oend {
            qutt = oend.offset_from(endMatch) as ::core::ffi::c_long as size_t;
        }
        memcpy(
            &raw mut saved as *mut U64 as *mut ::core::ffi::c_void,
            endMatch as *const ::core::ffi::c_void,
            qutt,
        );
    }
    if sequence.offset < 8 as size_t {
        let dec64: ::core::ffi::c_int = dec64table[sequence.offset as usize];
        *op.offset(0 as ::core::ffi::c_int as isize) =
            *match_0.offset(0 as ::core::ffi::c_int as isize);
        *op.offset(1 as ::core::ffi::c_int as isize) =
            *match_0.offset(1 as ::core::ffi::c_int as isize);
        *op.offset(2 as ::core::ffi::c_int as isize) =
            *match_0.offset(2 as ::core::ffi::c_int as isize);
        *op.offset(3 as ::core::ffi::c_int as isize) =
            *match_0.offset(3 as ::core::ffi::c_int as isize);
        match_0 = match_0.offset(dec32table[sequence.offset as usize] as isize);
        ZSTD_copy4(
            op.offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
        );
        match_0 = match_0.offset(-(dec64 as isize));
    } else {
        ZSTD_copy8(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
        );
    }
    op = op.offset(8 as ::core::ffi::c_int as isize);
    match_0 = match_0.offset(8 as ::core::ffi::c_int as isize);
    if endMatch > oend.offset(-((16 as ::core::ffi::c_int - MINMATCH) as isize)) {
        if op < oend.offset(-(8 as ::core::ffi::c_int as isize)) {
            ZSTD_wildcopy(
                op as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                oend.offset(-(8 as ::core::ffi::c_int as isize))
                    .offset_from(op) as ptrdiff_t,
            );
            match_0 = match_0.offset(
                oend.offset(-(8 as ::core::ffi::c_int as isize))
                    .offset_from(op) as ::core::ffi::c_long as isize,
            );
            op = oend.offset(-(8 as ::core::ffi::c_int as isize));
        }
        while op < endMatch {
            let fresh0 = match_0;
            match_0 = match_0.offset(1);
            let fresh1 = op;
            op = op.offset(1);
            *fresh1 = *fresh0;
        }
    } else {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            sequence.matchLength as ptrdiff_t - 8 as ptrdiff_t,
        );
    }
    if overlapRisk != 0 {
        memcpy(
            endMatch as *mut ::core::ffi::c_void,
            &raw mut saved as *mut U64 as *const ::core::ffi::c_void,
            qutt,
        );
    }
    return endMatch.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompressSequences(
    mut ctx: *mut ::core::ffi::c_void,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut litStart: *const BYTE,
    mut litSize: size_t,
) -> size_t {
    let mut dctx: *mut dctx_t = ctx as *mut dctx_t;
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.offset(seqSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.offset(maxDstSize as isize);
    let mut errorCode: size_t = 0;
    let mut dumpsLength: size_t = 0;
    let mut litPtr: *const BYTE = litStart;
    let litEnd: *const BYTE = litStart.offset(litSize as isize);
    let mut nbSeq: ::core::ffi::c_int = 0;
    let mut dumps: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut DTableLL: *mut U32 = &raw mut (*dctx).LLTable as *mut U32;
    let mut DTableML: *mut U32 = &raw mut (*dctx).MLTable as *mut U32;
    let mut DTableOffb: *mut U32 = &raw mut (*dctx).OffTable as *mut U32;
    let base: *mut BYTE = (*dctx).base as *mut BYTE;
    errorCode = ZSTDv01_decodeSeqHeaders(
        &raw mut nbSeq,
        &raw mut dumps,
        &raw mut dumpsLength,
        DTableLL as *mut FSE_DTable,
        DTableML as *mut FSE_DTable,
        DTableOffb as *mut FSE_DTable,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
    );
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.offset(errorCode as isize);
    let mut sequence: seq_t = seq_t {
        litLength: 0,
        offset: 0,
        matchLength: 0,
    };
    let mut seqState: seqState_t = seqState_t {
        DStream: FSE_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: ::core::ptr::null::<::core::ffi::c_char>(),
            start: ::core::ptr::null::<::core::ffi::c_char>(),
        },
        stateLL: FSE_DState_t {
            state: 0,
            table: ::core::ptr::null::<::core::ffi::c_void>(),
        },
        stateOffb: FSE_DState_t {
            state: 0,
            table: ::core::ptr::null::<::core::ffi::c_void>(),
        },
        stateML: FSE_DState_t {
            state: 0,
            table: ::core::ptr::null::<::core::ffi::c_void>(),
        },
        prevOffset: 0,
        dumps: ::core::ptr::null::<BYTE>(),
        dumpsEnd: ::core::ptr::null::<BYTE>(),
    };
    memset(
        &raw mut sequence as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<seq_t>() as size_t,
    );
    seqState.dumps = dumps;
    seqState.dumpsEnd = dumps.offset(dumpsLength as isize);
    seqState.prevOffset = 1 as size_t;
    errorCode = FSE_initDStream(
        &raw mut seqState.DStream,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
    );
    if FSE_isError(errorCode) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    FSE_initDState(
        &raw mut seqState.stateLL,
        &raw mut seqState.DStream,
        DTableLL,
    );
    FSE_initDState(
        &raw mut seqState.stateOffb,
        &raw mut seqState.DStream,
        DTableOffb,
    );
    FSE_initDState(
        &raw mut seqState.stateML,
        &raw mut seqState.DStream,
        DTableML,
    );
    while FSE_reloadDStream(&raw mut seqState.DStream)
        <= FSE_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
        && nbSeq > 0 as ::core::ffi::c_int
    {
        let mut oneSeqSize: size_t = 0;
        nbSeq -= 1;
        ZSTD_decodeSequence(&raw mut sequence, &raw mut seqState);
        oneSeqSize = ZSTD_execSequence(op, sequence, &raw mut litPtr, litEnd, base, oend);
        if ZSTDv01_isError(oneSeqSize) != 0 {
            return oneSeqSize;
        }
        op = op.offset(oneSeqSize as isize);
    }
    if FSE_endOfDStream(&raw mut seqState.DStream) == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if nbSeq < 0 as ::core::ffi::c_int {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let mut lastLLSize: size_t = litEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
    if op.offset(lastLLSize as isize) > oend {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if lastLLSize > 0 as size_t {
        if op != litPtr as *mut BYTE {
            memmove(
                op as *mut ::core::ffi::c_void,
                litPtr as *const ::core::ffi::c_void,
                lastLLSize,
            );
        }
        op = op.offset(lastLLSize as isize);
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompressBlock(
    mut ctx: *mut ::core::ffi::c_void,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut litPtr: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut litSize: size_t = 0 as size_t;
    let mut errorCode: size_t = 0;
    errorCode = ZSTDv01_decodeLiteralsBlock(
        ctx,
        dst,
        maxDstSize,
        &raw mut litPtr,
        &raw mut litSize,
        src,
        srcSize,
    );
    if ZSTDv01_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.offset(errorCode as isize);
    srcSize = (srcSize as ::core::ffi::c_ulong).wrapping_sub(errorCode as ::core::ffi::c_ulong)
        as size_t as size_t;
    return ZSTD_decompressSequences(
        ctx,
        dst,
        maxDstSize,
        ip as *const ::core::ffi::c_void,
        srcSize,
        litPtr,
        litSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompressDCtx(
    mut ctx: *mut ::core::ffi::c_void,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iend: *const BYTE = ip.offset(srcSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.offset(maxDstSize as isize);
    let mut remainingSize: size_t = srcSize;
    let mut magicNumber: U32 = 0;
    let mut errorCode: size_t = 0 as size_t;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };
    if srcSize < ZSTD_frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(ZSTD_frameHeaderSize as isize);
    remainingSize = (remainingSize as ::core::ffi::c_ulong)
        .wrapping_sub(ZSTD_frameHeaderSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    loop {
        let mut blockSize: size_t = ZSTDv01_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            &raw mut blockProperties,
        );
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }
        ip = ip.offset(ZSTD_blockHeaderSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTD_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if blockSize > remainingSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        match blockProperties.blockType as ::core::ffi::c_uint {
            0 => {
                errorCode = ZSTD_decompressBlock(
                    ctx,
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    blockSize,
                );
            }
            1 => {
                errorCode = ZSTD_copyUncompressedBlock(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    blockSize,
                );
            }
            2 => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
            3 => {
                if remainingSize != 0 {
                    return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
                }
            }
            _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
        }
        if blockSize == 0 as size_t {
            break;
        }
        if ZSTDv01_isError(errorCode) != 0 {
            return errorCode;
        }
        op = op.offset(errorCode as isize);
        ip = ip.offset(blockSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(blockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ctx: dctx_t = dctx_t {
        LLTable: [0; 1025],
        OffTable: [0; 513],
        MLTable: [0; 1025],
        previousDstEnd: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        base: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        expected: 0,
        bType: bt_compressed,
        phase: 0,
    };
    ctx.base = dst;
    return ZSTDv01_decompressDCtx(
        &raw mut ctx as *mut ::core::ffi::c_void,
        dst,
        maxDstSize,
        src,
        srcSize,
    );
}
unsafe extern "C" fn ZSTD_errorFrameSizeInfoLegacy(
    mut cSize: *mut size_t,
    mut dBound: *mut ::core::ffi::c_ulonglong,
    mut ret: size_t,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_findFrameSizeInfoLegacy(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut cSize: *mut size_t,
    mut dBound: *mut ::core::ffi::c_ulonglong,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: size_t = srcSize;
    let mut nbBlocks: size_t = 0 as size_t;
    let mut magicNumber: U32 = 0;
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };
    if srcSize < ZSTD_frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
        ZSTD_errorFrameSizeInfoLegacy(
            cSize,
            dBound,
            -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
        );
        return;
    }
    magicNumber = ZSTD_readBE32(src);
    if magicNumber != ZSTD_magicNumber {
        ZSTD_errorFrameSizeInfoLegacy(
            cSize,
            dBound,
            -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t,
        );
        return;
    }
    ip = ip.offset(ZSTD_frameHeaderSize as isize);
    remainingSize = (remainingSize as ::core::ffi::c_ulong)
        .wrapping_sub(ZSTD_frameHeaderSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    loop {
        let mut blockSize: size_t = ZSTDv01_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            remainingSize,
            &raw mut blockProperties,
        );
        if ZSTDv01_isError(blockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, blockSize);
            return;
        }
        ip = ip.offset(ZSTD_blockHeaderSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTD_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if blockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(
                cSize,
                dBound,
                -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
            );
            return;
        }
        if blockSize == 0 as size_t {
            break;
        }
        ip = ip.offset(blockSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(blockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
        nbBlocks = nbBlocks.wrapping_add(1);
    }
    *cSize = ip.offset_from(src as *const BYTE) as ::core::ffi::c_long as size_t;
    *dBound = nbBlocks.wrapping_mul(BLOCKSIZE as size_t) as ::core::ffi::c_ulonglong;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_resetDCtx(mut dctx: *mut ZSTDv01_Dctx) -> size_t {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0 as U32;
    (*dctx).previousDstEnd = NULL;
    (*dctx).base = NULL;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_createDCtx() -> *mut ZSTDv01_Dctx {
    let mut dctx: *mut ZSTDv01_Dctx =
        malloc(::core::mem::size_of::<ZSTDv01_Dctx>() as size_t) as *mut ZSTDv01_Dctx;
    if dctx.is_null() {
        return ::core::ptr::null_mut::<ZSTDv01_Dctx>();
    }
    ZSTDv01_resetDCtx(dctx);
    return dctx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_freeDCtx(mut dctx: *mut ZSTDv01_Dctx) -> size_t {
    free(dctx as *mut ::core::ffi::c_void);
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_nextSrcSizeToDecompress(mut dctx: *mut ZSTDv01_Dctx) -> size_t {
    return (*(dctx as *mut dctx_t)).expected;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv01_decompressContinue(
    mut dctx: *mut ZSTDv01_Dctx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ctx: *mut dctx_t = dctx as *mut dctx_t;
    if srcSize != (*ctx).expected {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if dst != (*ctx).previousDstEnd {
        (*ctx).base = dst;
    }
    if (*ctx).phase == 0 as U32 {
        let mut magicNumber: U32 = ZSTD_readBE32(src);
        if magicNumber != ZSTD_magicNumber {
            return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
        }
        (*ctx).phase = 1 as U32;
        (*ctx).expected = ZSTD_blockHeaderSize;
        return 0 as size_t;
    }
    if (*ctx).phase == 1 as U32 {
        let mut bp: blockProperties_t = blockProperties_t {
            blockType: bt_compressed,
            origSize: 0,
        };
        let mut blockSize: size_t = ZSTDv01_getcBlockSize(src, ZSTD_blockHeaderSize, &raw mut bp);
        if ZSTDv01_isError(blockSize) != 0 {
            return blockSize;
        }
        if bp.blockType as ::core::ffi::c_uint
            == bt_end as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            (*ctx).expected = 0 as size_t;
            (*ctx).phase = 0 as U32;
        } else {
            (*ctx).expected = blockSize;
            (*ctx).bType = bp.blockType;
            (*ctx).phase = 2 as U32;
        }
        return 0 as size_t;
    }
    let mut rSize: size_t = 0;
    match (*ctx).bType as ::core::ffi::c_uint {
        0 => {
            rSize = ZSTD_decompressBlock(
                ctx as *mut ::core::ffi::c_void,
                dst,
                maxDstSize,
                src,
                srcSize,
            );
        }
        1 => {
            rSize = ZSTD_copyUncompressedBlock(dst, maxDstSize, src, srcSize);
        }
        2 => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
        3 => {
            rSize = 0 as size_t;
        }
        _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
    }
    (*ctx).phase = 1 as U32;
    (*ctx).expected = ZSTD_blockHeaderSize;
    if ZSTDv01_isError(rSize) != 0 {
        return rSize;
    }
    (*ctx).previousDstEnd =
        (dst as *mut ::core::ffi::c_char).offset(rSize as isize) as *mut ::core::ffi::c_void;
    return rSize;
}
