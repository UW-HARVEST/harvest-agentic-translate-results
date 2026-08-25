use ::libc;
extern "C" {
    fn ERR_getErrorString(code: ERR_enum) -> *const ::core::ffi::c_char;
    fn FSE_decompress_wksp_bmi2(
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        cSrc: *const ::core::ffi::c_void,
        cSrcSize: size_t,
        maxLog: ::core::ffi::c_uint,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        bmi2: ::core::ffi::c_int,
    ) -> size_t;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type BYTE = uint8_t;
pub type U32 = uint32_t;
pub type unalign32 = U32;
pub type ZSTD_ErrorCode = ::core::ffi::c_uint;
pub const ZSTD_error_maxCode: ZSTD_ErrorCode = 120;
pub const ZSTD_error_externalSequences_invalid: ZSTD_ErrorCode = 107;
pub const ZSTD_error_sequenceProducer_failed: ZSTD_ErrorCode = 106;
pub const ZSTD_error_srcBuffer_wrong: ZSTD_ErrorCode = 105;
pub const ZSTD_error_dstBuffer_wrong: ZSTD_ErrorCode = 104;
pub const ZSTD_error_seekableIO: ZSTD_ErrorCode = 102;
pub const ZSTD_error_frameIndex_tooLarge: ZSTD_ErrorCode = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: ZSTD_ErrorCode = 82;
pub const ZSTD_error_noForwardProgress_destFull: ZSTD_ErrorCode = 80;
pub const ZSTD_error_dstBuffer_null: ZSTD_ErrorCode = 74;
pub const ZSTD_error_srcSize_wrong: ZSTD_ErrorCode = 72;
pub const ZSTD_error_dstSize_tooSmall: ZSTD_ErrorCode = 70;
pub const ZSTD_error_workSpace_tooSmall: ZSTD_ErrorCode = 66;
pub const ZSTD_error_memory_allocation: ZSTD_ErrorCode = 64;
pub const ZSTD_error_init_missing: ZSTD_ErrorCode = 62;
pub const ZSTD_error_stage_wrong: ZSTD_ErrorCode = 60;
pub const ZSTD_error_stabilityCondition_notRespected: ZSTD_ErrorCode = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: ZSTD_ErrorCode = 49;
pub const ZSTD_error_maxSymbolValue_tooSmall: ZSTD_ErrorCode = 48;
pub const ZSTD_error_maxSymbolValue_tooLarge: ZSTD_ErrorCode = 46;
pub const ZSTD_error_tableLog_tooLarge: ZSTD_ErrorCode = 44;
pub const ZSTD_error_parameter_outOfBound: ZSTD_ErrorCode = 42;
pub const ZSTD_error_parameter_combination_unsupported: ZSTD_ErrorCode = 41;
pub const ZSTD_error_parameter_unsupported: ZSTD_ErrorCode = 40;
pub const ZSTD_error_dictionaryCreation_failed: ZSTD_ErrorCode = 34;
pub const ZSTD_error_dictionary_wrong: ZSTD_ErrorCode = 32;
pub const ZSTD_error_dictionary_corrupted: ZSTD_ErrorCode = 30;
pub const ZSTD_error_literals_headerWrong: ZSTD_ErrorCode = 24;
pub const ZSTD_error_checksum_wrong: ZSTD_ErrorCode = 22;
pub const ZSTD_error_corruption_detected: ZSTD_ErrorCode = 20;
pub const ZSTD_error_frameParameter_windowTooLarge: ZSTD_ErrorCode = 16;
pub const ZSTD_error_frameParameter_unsupported: ZSTD_ErrorCode = 14;
pub const ZSTD_error_version_unsupported: ZSTD_ErrorCode = 12;
pub const ZSTD_error_prefix_unknown: ZSTD_ErrorCode = 10;
pub const ZSTD_error_GENERIC: ZSTD_ErrorCode = 1;
pub const ZSTD_error_no_error: ZSTD_ErrorCode = 0;
pub type ERR_enum = ZSTD_ErrorCode;
#[inline]
unsafe extern "C" fn MEM_isLittleEndian() -> ::core::ffi::c_uint {
    return 1 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_read32(mut ptr: *const ::core::ffi::c_void) -> U32 {
    return *(ptr as *const unalign32);
}
#[inline]
unsafe extern "C" fn MEM_swap32(mut in_0: U32) -> U32 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_readLE32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read32(memPtr);
    } else {
        return MEM_swap32(MEM_read32(memPtr));
    };
}
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
unsafe extern "C" fn ERR_getErrorCode(mut code: size_t) -> ERR_enum {
    if ERR_isError(code) == 0 {
        return ZSTD_error_no_error;
    }
    return (0 as size_t).wrapping_sub(code) as ERR_enum;
}
unsafe extern "C" fn ERR_getErrorName(mut code: size_t) -> *const ::core::ffi::c_char {
    return ERR_getErrorString(ERR_getErrorCode(code));
}
pub const FSE_VERSION_MAJOR: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FSE_VERSION_MINOR: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const FSE_VERSION_RELEASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FSE_VERSION_NUMBER: ::core::ffi::c_int =
    FSE_VERSION_MAJOR * 100 as ::core::ffi::c_int * 100 as ::core::ffi::c_int
        + FSE_VERSION_MINOR * 100 as ::core::ffi::c_int
        + FSE_VERSION_RELEASE;
#[inline]
unsafe extern "C" fn ZSTD_countTrailingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.trailing_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_countLeadingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
pub const FSE_MIN_TABLELOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const FSE_TABLELOG_ABSOLUTE_MAX: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const HUF_TABLELOG_MAX: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_versionNumber() -> ::core::ffi::c_uint {
    return FSE_VERSION_NUMBER as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_getErrorName(mut code: size_t) -> *const ::core::ffi::c_char {
    return ERR_getErrorName(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getErrorName(mut code: size_t) -> *const ::core::ffi::c_char {
    return ERR_getErrorName(code);
}
#[inline(always)]
unsafe extern "C" fn FSE_readNCount_body(
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
    let maxSV1: ::core::ffi::c_uint = (*maxSVPtr).wrapping_add(1 as ::core::ffi::c_uint);
    let mut previous0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if hbSize < 8 as size_t {
        let mut buffer: [::core::ffi::c_char; 8] = [
            0 as ::core::ffi::c_int as ::core::ffi::c_char,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        ::libc::memcpy(
            &raw mut buffer as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            headerBuffer,
            hbSize as ::libc::size_t,
        );
        let countSize: size_t = FSE_readNCount(
            normalizedCounter,
            maxSVPtr,
            tableLogPtr,
            &raw mut buffer as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 8]>() as size_t,
        ) as size_t;
        if FSE_isError(countSize) != 0 {
            return countSize;
        }
        if countSize > hbSize {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        return countSize;
    }
    ::libc::memset(
        normalizedCounter as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ((*maxSVPtr).wrapping_add(1 as ::core::ffi::c_uint) as usize)
            .wrapping_mul(::core::mem::size_of::<::core::ffi::c_short>() as usize)
            as ::libc::size_t,
    );
    bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void);
    nbBits = (bitStream & 0xf as U32).wrapping_add(FSE_MIN_TABLELOG as U32) as ::core::ffi::c_int;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    bitStream >>= 4 as ::core::ffi::c_int;
    bitCount = 4 as ::core::ffi::c_int;
    *tableLogPtr = nbBits as ::core::ffi::c_uint;
    remaining = ((1 as ::core::ffi::c_int) << nbBits) + 1 as ::core::ffi::c_int;
    threshold = (1 as ::core::ffi::c_int) << nbBits;
    nbBits += 1;
    loop {
        if previous0 != 0 {
            let mut repeats: ::core::ffi::c_int =
                (ZSTD_countTrailingZeros32(!bitStream | 0x80000000 as U32)
                    >> 1 as ::core::ffi::c_int) as ::core::ffi::c_int;
            while repeats >= 12 as ::core::ffi::c_int {
                charnum = charnum.wrapping_add(
                    (3 as ::core::ffi::c_int * 12 as ::core::ffi::c_int) as ::core::ffi::c_uint,
                );
                if (ip <= iend.offset(-(7 as ::core::ffi::c_int as isize))) as ::core::ffi::c_int
                    as ::core::ffi::c_long
                    != 0
                {
                    ip = ip.offset(3 as ::core::ffi::c_int as isize);
                } else {
                    bitCount -= (8 as ::core::ffi::c_long
                        * iend
                            .offset(-(7 as ::core::ffi::c_int as isize))
                            .offset_from(ip) as ::core::ffi::c_long)
                        as ::core::ffi::c_int;
                    bitCount &= 31 as ::core::ffi::c_int;
                    ip = iend.offset(-(4 as ::core::ffi::c_int as isize));
                }
                bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
                repeats = (ZSTD_countTrailingZeros32(!bitStream | 0x80000000 as U32)
                    >> 1 as ::core::ffi::c_int) as ::core::ffi::c_int;
            }
            charnum =
                charnum.wrapping_add((3 as ::core::ffi::c_int * repeats) as ::core::ffi::c_uint);
            bitStream >>= 2 as ::core::ffi::c_int * repeats;
            bitCount += 2 as ::core::ffi::c_int * repeats;
            charnum = charnum.wrapping_add((bitStream & 3 as U32) as ::core::ffi::c_uint);
            bitCount += 2 as ::core::ffi::c_int;
            if charnum >= maxSV1 {
                break;
            }
            if (ip <= iend.offset(-(7 as ::core::ffi::c_int as isize))) as ::core::ffi::c_int
                as ::core::ffi::c_long
                != 0
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
                bitCount &= 31 as ::core::ffi::c_int;
                ip = iend.offset(-(4 as ::core::ffi::c_int as isize));
            }
            bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
        }
        let max: ::core::ffi::c_int =
            2 as ::core::ffi::c_int * threshold - 1 as ::core::ffi::c_int - remaining;
        let mut count: ::core::ffi::c_int = 0;
        if (bitStream & (threshold - 1 as ::core::ffi::c_int) as U32) < max as U32 {
            count =
                (bitStream & (threshold - 1 as ::core::ffi::c_int) as U32) as ::core::ffi::c_int;
            bitCount += nbBits - 1 as ::core::ffi::c_int;
        } else {
            count = (bitStream
                & (2 as ::core::ffi::c_int * threshold - 1 as ::core::ffi::c_int) as U32)
                as ::core::ffi::c_int;
            if count >= threshold {
                count -= max;
            }
            bitCount += nbBits;
        }
        count -= 1;
        if count >= 0 as ::core::ffi::c_int {
            remaining -= count;
        } else {
            remaining += count;
        }
        let fresh0 = charnum;
        charnum = charnum.wrapping_add(1);
        *normalizedCounter.offset(fresh0 as isize) = count as ::core::ffi::c_short;
        previous0 = (count == 0) as ::core::ffi::c_int;
        if remaining < threshold {
            if remaining <= 1 as ::core::ffi::c_int {
                break;
            }
            nbBits = ZSTD_highbit32(remaining as U32).wrapping_add(1 as ::core::ffi::c_uint)
                as ::core::ffi::c_int;
            threshold = (1 as ::core::ffi::c_int) << nbBits - 1 as ::core::ffi::c_int;
        }
        if charnum >= maxSV1 {
            break;
        }
        if (ip <= iend.offset(-(7 as ::core::ffi::c_int as isize))) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
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
            bitCount &= 31 as ::core::ffi::c_int;
            ip = iend.offset(-(4 as ::core::ffi::c_int as isize));
        }
        bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
    }
    if remaining != 1 as ::core::ffi::c_int {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if charnum > maxSV1 {
        return -(ZSTD_error_maxSymbolValue_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if bitCount > 32 as ::core::ffi::c_int {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *maxSVPtr = charnum.wrapping_sub(1 as ::core::ffi::c_uint);
    ip = ip.offset((bitCount + 7 as ::core::ffi::c_int >> 3 as ::core::ffi::c_int) as isize);
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn FSE_readNCount_body_default(
    mut normalizedCounter: *mut ::core::ffi::c_short,
    mut maxSVPtr: *mut ::core::ffi::c_uint,
    mut tableLogPtr: *mut ::core::ffi::c_uint,
    mut headerBuffer: *const ::core::ffi::c_void,
    mut hbSize: size_t,
) -> size_t {
    return FSE_readNCount_body(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount_bmi2(
    mut normalizedCounter: *mut ::core::ffi::c_short,
    mut maxSVPtr: *mut ::core::ffi::c_uint,
    mut tableLogPtr: *mut ::core::ffi::c_uint,
    mut headerBuffer: *const ::core::ffi::c_void,
    mut hbSize: size_t,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    return FSE_readNCount_body_default(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount(
    mut normalizedCounter: *mut ::core::ffi::c_short,
    mut maxSVPtr: *mut ::core::ffi::c_uint,
    mut tableLogPtr: *mut ::core::ffi::c_uint,
    mut headerBuffer: *const ::core::ffi::c_void,
    mut hbSize: size_t,
) -> size_t {
    return FSE_readNCount_bmi2(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
        0 as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats(
    mut huffWeight: *mut BYTE,
    mut hwSize: size_t,
    mut rankStats: *mut U32,
    mut nbSymbolsPtr: *mut U32,
    mut tableLogPtr: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut wksp: [U32; 219] = [0; 219];
    return HUF_readStats_wksp(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        &raw mut wksp as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 219]>() as size_t,
        0 as ::core::ffi::c_int,
    );
}
#[inline(always)]
unsafe extern "C" fn HUF_readStats_body(
    mut huffWeight: *mut BYTE,
    mut hwSize: size_t,
    mut rankStats: *mut U32,
    mut nbSymbolsPtr: *mut U32,
    mut tableLogPtr: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    let mut weightTotal: U32 = 0;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: size_t = 0;
    let mut oSize: size_t = 0;
    if srcSize == 0 {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    iSize = *ip.offset(0 as ::core::ffi::c_int as isize) as size_t;
    if iSize >= 128 as size_t {
        oSize = iSize.wrapping_sub(127 as size_t);
        iSize = oSize.wrapping_add(1 as size_t).wrapping_div(2 as size_t);
        if iSize.wrapping_add(1 as size_t) > srcSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        if oSize >= hwSize {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        ip = ip.offset(1 as ::core::ffi::c_int as isize);
        let mut n: U32 = 0;
        n = 0 as U32;
        while (n as size_t) < oSize {
            *huffWeight.offset(n as isize) = (*ip.offset(n.wrapping_div(2 as U32) as isize)
                as ::core::ffi::c_int
                >> 4 as ::core::ffi::c_int) as BYTE;
            *huffWeight.offset(n.wrapping_add(1 as U32) as isize) =
                (*ip.offset(n.wrapping_div(2 as U32) as isize) as ::core::ffi::c_int
                    & 15 as ::core::ffi::c_int) as BYTE;
            n = (n as ::core::ffi::c_uint).wrapping_add(2 as ::core::ffi::c_uint) as U32 as U32;
        }
    } else {
        if iSize.wrapping_add(1 as size_t) > srcSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        oSize = FSE_decompress_wksp_bmi2(
            huffWeight as *mut ::core::ffi::c_void,
            hwSize.wrapping_sub(1 as size_t),
            ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            iSize,
            6 as ::core::ffi::c_uint,
            workSpace,
            wkspSize,
            bmi2,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }
    ::libc::memset(
        rankStats as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ((12 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as usize)
            .wrapping_mul(::core::mem::size_of::<U32>() as usize) as ::libc::size_t,
    );
    weightTotal = 0 as U32;
    let mut n_0: U32 = 0;
    n_0 = 0 as U32;
    while (n_0 as size_t) < oSize {
        if *huffWeight.offset(n_0 as isize) as ::core::ffi::c_int > HUF_TABLELOG_MAX {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        let ref mut fresh1 = *rankStats.offset(*huffWeight.offset(n_0 as isize) as isize);
        *fresh1 = (*fresh1).wrapping_add(1);
        weightTotal = (weightTotal as ::core::ffi::c_uint).wrapping_add(
            ((1 as ::core::ffi::c_int) << *huffWeight.offset(n_0 as isize) as ::core::ffi::c_int
                >> 1 as ::core::ffi::c_int) as ::core::ffi::c_uint,
        ) as U32 as U32;
        n_0 = n_0.wrapping_add(1);
    }
    if weightTotal == 0 as U32 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let tableLog: U32 = (ZSTD_highbit32(weightTotal) as U32).wrapping_add(1 as U32);
    if tableLog > HUF_TABLELOG_MAX as U32 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *tableLogPtr = tableLog;
    let total: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let rest: U32 = total.wrapping_sub(weightTotal);
    let verif: U32 = ((1 as ::core::ffi::c_int) << ZSTD_highbit32(rest)) as U32;
    let lastWeight: U32 = (ZSTD_highbit32(rest) as U32).wrapping_add(1 as U32);
    if verif != rest {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *huffWeight.offset(oSize as isize) = lastWeight as BYTE;
    let ref mut fresh2 = *rankStats.offset(lastWeight as isize);
    *fresh2 = (*fresh2).wrapping_add(1);
    if *rankStats.offset(1 as ::core::ffi::c_int as isize) < 2 as U32
        || *rankStats.offset(1 as ::core::ffi::c_int as isize) & 1 as U32 != 0
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *nbSymbolsPtr = oSize.wrapping_add(1 as size_t) as U32;
    return iSize.wrapping_add(1 as size_t);
}
unsafe extern "C" fn HUF_readStats_body_default(
    mut huffWeight: *mut BYTE,
    mut hwSize: size_t,
    mut rankStats: *mut U32,
    mut nbSymbolsPtr: *mut U32,
    mut tableLogPtr: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    return HUF_readStats_body(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        workSpace,
        wkspSize,
        0 as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats_wksp(
    mut huffWeight: *mut BYTE,
    mut hwSize: size_t,
    mut rankStats: *mut U32,
    mut nbSymbolsPtr: *mut U32,
    mut tableLogPtr: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_readStats_body_default(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        workSpace,
        wkspSize,
    );
}
