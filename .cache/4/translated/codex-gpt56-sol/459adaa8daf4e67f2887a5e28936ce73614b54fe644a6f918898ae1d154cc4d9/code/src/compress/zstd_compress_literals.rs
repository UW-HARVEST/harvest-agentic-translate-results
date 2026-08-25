use ::libc;
extern "C" {
    fn HUF_compress4X_repeat(
        dst: *mut ::core::ffi::c_void,
        dstSize: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        maxSymbolValue: ::core::ffi::c_uint,
        tableLog: ::core::ffi::c_uint,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        hufTable: *mut HUF_CElt,
        repeat: *mut HUF_repeat,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn HUF_compress1X_repeat(
        dst: *mut ::core::ffi::c_void,
        dstSize: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        maxSymbolValue: ::core::ffi::c_uint,
        tableLog: ::core::ffi::c_uint,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        hufTable: *mut HUF_CElt,
        repeat: *mut HUF_repeat,
        flags: ::core::ffi::c_int,
    ) -> size_t;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type BYTE = uint8_t;
pub type U16 = uint16_t;
pub type U32 = uint32_t;
pub type unalign16 = U16;
pub type unalign32 = U32;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ZSTD_error_maxCode: C2RustUnnamed = 120;
pub const ZSTD_error_externalSequences_invalid: C2RustUnnamed = 107;
pub const ZSTD_error_sequenceProducer_failed: C2RustUnnamed = 106;
pub const ZSTD_error_srcBuffer_wrong: C2RustUnnamed = 105;
pub const ZSTD_error_dstBuffer_wrong: C2RustUnnamed = 104;
pub const ZSTD_error_seekableIO: C2RustUnnamed = 102;
pub const ZSTD_error_frameIndex_tooLarge: C2RustUnnamed = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: C2RustUnnamed = 82;
pub const ZSTD_error_noForwardProgress_destFull: C2RustUnnamed = 80;
pub const ZSTD_error_dstBuffer_null: C2RustUnnamed = 74;
pub const ZSTD_error_srcSize_wrong: C2RustUnnamed = 72;
pub const ZSTD_error_dstSize_tooSmall: C2RustUnnamed = 70;
pub const ZSTD_error_workSpace_tooSmall: C2RustUnnamed = 66;
pub const ZSTD_error_memory_allocation: C2RustUnnamed = 64;
pub const ZSTD_error_init_missing: C2RustUnnamed = 62;
pub const ZSTD_error_stage_wrong: C2RustUnnamed = 60;
pub const ZSTD_error_stabilityCondition_notRespected: C2RustUnnamed = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: C2RustUnnamed = 49;
pub const ZSTD_error_maxSymbolValue_tooSmall: C2RustUnnamed = 48;
pub const ZSTD_error_maxSymbolValue_tooLarge: C2RustUnnamed = 46;
pub const ZSTD_error_tableLog_tooLarge: C2RustUnnamed = 44;
pub const ZSTD_error_parameter_outOfBound: C2RustUnnamed = 42;
pub const ZSTD_error_parameter_combination_unsupported: C2RustUnnamed = 41;
pub const ZSTD_error_parameter_unsupported: C2RustUnnamed = 40;
pub const ZSTD_error_dictionaryCreation_failed: C2RustUnnamed = 34;
pub const ZSTD_error_dictionary_wrong: C2RustUnnamed = 32;
pub const ZSTD_error_dictionary_corrupted: C2RustUnnamed = 30;
pub const ZSTD_error_literals_headerWrong: C2RustUnnamed = 24;
pub const ZSTD_error_checksum_wrong: C2RustUnnamed = 22;
pub const ZSTD_error_corruption_detected: C2RustUnnamed = 20;
pub const ZSTD_error_frameParameter_windowTooLarge: C2RustUnnamed = 16;
pub const ZSTD_error_frameParameter_unsupported: C2RustUnnamed = 14;
pub const ZSTD_error_version_unsupported: C2RustUnnamed = 12;
pub const ZSTD_error_prefix_unknown: C2RustUnnamed = 10;
pub const ZSTD_error_GENERIC: C2RustUnnamed = 1;
pub const ZSTD_error_no_error: C2RustUnnamed = 0;
pub type SymbolEncodingType_e = ::core::ffi::c_uint;
pub const set_repeat: SymbolEncodingType_e = 3;
pub const set_compressed: SymbolEncodingType_e = 2;
pub const set_rle: SymbolEncodingType_e = 1;
pub const set_basic: SymbolEncodingType_e = 0;
pub type ZSTD_strategy = ::core::ffi::c_uint;
pub const ZSTD_btultra2: ZSTD_strategy = 9;
pub const ZSTD_btultra: ZSTD_strategy = 8;
pub const ZSTD_btopt: ZSTD_strategy = 7;
pub const ZSTD_btlazy2: ZSTD_strategy = 6;
pub const ZSTD_lazy2: ZSTD_strategy = 5;
pub const ZSTD_lazy: ZSTD_strategy = 4;
pub const ZSTD_greedy: ZSTD_strategy = 3;
pub const ZSTD_dfast: ZSTD_strategy = 2;
pub const ZSTD_fast: ZSTD_strategy = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_hufCTables_t {
    pub CTable: [HUF_CElt; 257],
    pub repeatMode: HUF_repeat,
}
pub type HUF_repeat = ::core::ffi::c_uint;
pub const HUF_repeat_valid: HUF_repeat = 2;
pub const HUF_repeat_check: HUF_repeat = 1;
pub const HUF_repeat_none: HUF_repeat = 0;
pub type HUF_CElt = size_t;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const HUF_flags_disableFast: C2RustUnnamed_0 = 32;
pub const HUF_flags_disableAsm: C2RustUnnamed_0 = 16;
pub const HUF_flags_suspectUncompressible: C2RustUnnamed_0 = 8;
pub const HUF_flags_preferRepeat: C2RustUnnamed_0 = 4;
pub const HUF_flags_optimalDepth: C2RustUnnamed_0 = 2;
pub const HUF_flags_bmi2: C2RustUnnamed_0 = 1;
pub type huf_compress_f = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        size_t,
        *const ::core::ffi::c_void,
        size_t,
        ::core::ffi::c_uint,
        ::core::ffi::c_uint,
        *mut ::core::ffi::c_void,
        size_t,
        *mut HUF_CElt,
        *mut HUF_repeat,
        ::core::ffi::c_int,
    ) -> size_t,
>;
#[inline]
unsafe extern "C" fn MEM_isLittleEndian() -> ::core::ffi::c_uint {
    return 1 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_write16(mut memPtr: *mut ::core::ffi::c_void, mut value: U16) {
    *(memPtr as *mut unalign16) = value as unalign16;
}
#[inline]
unsafe extern "C" fn MEM_write32(mut memPtr: *mut ::core::ffi::c_void, mut value: U32) {
    *(memPtr as *mut unalign32) = value as unalign32;
}
#[inline]
unsafe extern "C" fn MEM_swap32(mut in_0: U32) -> U32 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_writeLE16(mut memPtr: *mut ::core::ffi::c_void, mut val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let mut p: *mut BYTE = memPtr as *mut BYTE;
        *p.offset(0 as ::core::ffi::c_int as isize) = val as BYTE;
        *p.offset(1 as ::core::ffi::c_int as isize) =
            (val as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as BYTE;
    };
}
#[inline]
unsafe extern "C" fn MEM_writeLE24(mut memPtr: *mut ::core::ffi::c_void, mut val: U32) {
    MEM_writeLE16(memPtr, val as U16);
    *(memPtr as *mut BYTE).offset(2 as ::core::ffi::c_int as isize) =
        (val >> 16 as ::core::ffi::c_int) as BYTE;
}
#[inline]
unsafe extern "C" fn MEM_writeLE32(mut memPtr: *mut ::core::ffi::c_void, mut val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, val32);
    } else {
        MEM_write32(memPtr, MEM_swap32(val32));
    };
}
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn _force_has_format_string(
    mut format: *const ::core::ffi::c_char,
    mut args: ...
) {
}
pub const HUF_SYMBOLVALUE_MAX: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const LitHufLog: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTD_minGain(mut srcSize: size_t, mut strat: ZSTD_strategy) -> size_t {
    let minlog: U32 = if strat as ::core::ffi::c_uint
        >= ZSTD_btultra as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (strat as U32).wrapping_sub(1 as U32)
    } else {
        6 as U32
    };
    return (srcSize >> minlog).wrapping_add(2 as size_t);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_noCompressLiterals(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let flSize: U32 = (1 as ::core::ffi::c_int
        + (srcSize > 31 as size_t) as ::core::ffi::c_int
        + (srcSize > 4095 as size_t) as ::core::ffi::c_int) as U32;
    if srcSize.wrapping_add(flSize as size_t) > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    match flSize {
        1 => {
            *ostart.offset(0 as ::core::ffi::c_int as isize) =
                (set_basic as ::core::ffi::c_int as U32 as size_t)
                    .wrapping_add(srcSize << 3 as ::core::ffi::c_int) as BYTE;
        }
        2 => {
            MEM_writeLE16(
                ostart as *mut ::core::ffi::c_void,
                ((set_basic as ::core::ffi::c_int as U32)
                    .wrapping_add(((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                    as size_t)
                    .wrapping_add(srcSize << 4 as ::core::ffi::c_int) as U16,
            );
        }
        3 => {
            MEM_writeLE32(
                ostart as *mut ::core::ffi::c_void,
                ((set_basic as ::core::ffi::c_int as U32)
                    .wrapping_add(((3 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                    as size_t)
                    .wrapping_add(srcSize << 4 as ::core::ffi::c_int) as U32,
            );
        }
        _ => {}
    }
    ::libc::memcpy(
        ostart.offset(flSize as isize) as *mut ::core::ffi::c_void,
        src,
        srcSize as ::libc::size_t,
    );
    return srcSize.wrapping_add(flSize as size_t);
}
unsafe extern "C" fn allBytesIdentical(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_int {
    let b: BYTE = *(src as *const BYTE).offset(0 as ::core::ffi::c_int as isize);
    let mut p: size_t = 0;
    p = 1 as size_t;
    while p < srcSize {
        if *(src as *const BYTE).offset(p as isize) as ::core::ffi::c_int != b as ::core::ffi::c_int
        {
            return 0 as ::core::ffi::c_int;
        }
        p = p.wrapping_add(1);
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressRleLiteralsBlock(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let flSize: U32 = (1 as ::core::ffi::c_int
        + (srcSize > 31 as size_t) as ::core::ffi::c_int
        + (srcSize > 4095 as size_t) as ::core::ffi::c_int) as U32;
    match flSize {
        1 => {
            *ostart.offset(0 as ::core::ffi::c_int as isize) =
                (set_rle as ::core::ffi::c_int as U32 as size_t)
                    .wrapping_add(srcSize << 3 as ::core::ffi::c_int) as BYTE;
        }
        2 => {
            MEM_writeLE16(
                ostart as *mut ::core::ffi::c_void,
                ((set_rle as ::core::ffi::c_int as U32)
                    .wrapping_add(((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                    as size_t)
                    .wrapping_add(srcSize << 4 as ::core::ffi::c_int) as U16,
            );
        }
        3 => {
            MEM_writeLE32(
                ostart as *mut ::core::ffi::c_void,
                ((set_rle as ::core::ffi::c_int as U32)
                    .wrapping_add(((3 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                    as size_t)
                    .wrapping_add(srcSize << 4 as ::core::ffi::c_int) as U32,
            );
        }
        _ => {}
    }
    *ostart.offset(flSize as isize) = *(src as *const BYTE);
    return flSize.wrapping_add(1 as U32) as size_t;
}
unsafe extern "C" fn ZSTD_minLiteralsToCompress(
    mut strategy: ZSTD_strategy,
    mut huf_repeat: HUF_repeat,
) -> size_t {
    let shift: ::core::ffi::c_int =
        if (9 as ::core::ffi::c_int - strategy as ::core::ffi::c_int) < 3 as ::core::ffi::c_int {
            9 as ::core::ffi::c_int - strategy as ::core::ffi::c_int
        } else {
            3 as ::core::ffi::c_int
        };
    let mintc: size_t = if huf_repeat as ::core::ffi::c_uint
        == HUF_repeat_valid as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        6 as size_t
    } else {
        (8 as ::core::ffi::c_int as size_t) << shift
    };
    return mintc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressLiterals(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut entropyWorkspace: *mut ::core::ffi::c_void,
    mut entropyWorkspaceSize: size_t,
    mut prevHuf: *const ZSTD_hufCTables_t,
    mut nextHuf: *mut ZSTD_hufCTables_t,
    mut strategy: ZSTD_strategy,
    mut disableLiteralCompression: ::core::ffi::c_int,
    mut suspectUncompressible: ::core::ffi::c_int,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    let lhSize: size_t = (3 as ::core::ffi::c_int
        + (srcSize
            >= (1 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
                as size_t) as ::core::ffi::c_int
        + (srcSize
            >= (16 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
                as size_t) as ::core::ffi::c_int) as size_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut singleStream: U32 = (srcSize < 256 as size_t) as ::core::ffi::c_int as U32;
    let mut hType: SymbolEncodingType_e = set_compressed;
    let mut cLitSize: size_t = 0;
    ::libc::memcpy(
        nextHuf as *mut ::core::ffi::c_void,
        prevHuf as *const ::core::ffi::c_void,
        ::core::mem::size_of::<ZSTD_hufCTables_t>() as ::libc::size_t,
    );
    if disableLiteralCompression != 0 {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }
    if srcSize < ZSTD_minLiteralsToCompress(strategy, (*prevHuf).repeatMode) {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }
    if dstCapacity < lhSize.wrapping_add(1 as size_t) {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    let mut repeat: HUF_repeat = (*prevHuf).repeatMode;
    let flags: ::core::ffi::c_int = 0 as ::core::ffi::c_int
        | (if bmi2 != 0 {
            HUF_flags_bmi2 as ::core::ffi::c_int as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        })
        | (if (strategy as ::core::ffi::c_uint)
            < ZSTD_lazy as ::core::ffi::c_int as ::core::ffi::c_uint
            && srcSize <= 1024 as size_t
        {
            HUF_flags_preferRepeat as ::core::ffi::c_int as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        })
        | (if strategy as ::core::ffi::c_uint
            >= ZSTD_btultra as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            HUF_flags_optimalDepth as ::core::ffi::c_int as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        })
        | (if suspectUncompressible != 0 {
            HUF_flags_suspectUncompressible as ::core::ffi::c_int as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        });
    let mut huf_compress: huf_compress_f = None;
    if repeat as ::core::ffi::c_uint
        == HUF_repeat_valid as ::core::ffi::c_int as ::core::ffi::c_uint
        && lhSize == 3 as size_t
    {
        singleStream = 1 as U32;
    }
    huf_compress = (if singleStream != 0 {
        Some(
            HUF_compress1X_repeat
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    size_t,
                    *const ::core::ffi::c_void,
                    size_t,
                    ::core::ffi::c_uint,
                    ::core::ffi::c_uint,
                    *mut ::core::ffi::c_void,
                    size_t,
                    *mut HUF_CElt,
                    *mut HUF_repeat,
                    ::core::ffi::c_int,
                ) -> size_t,
        )
    } else {
        Some(
            HUF_compress4X_repeat
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    size_t,
                    *const ::core::ffi::c_void,
                    size_t,
                    ::core::ffi::c_uint,
                    ::core::ffi::c_uint,
                    *mut ::core::ffi::c_void,
                    size_t,
                    *mut HUF_CElt,
                    *mut HUF_repeat,
                    ::core::ffi::c_int,
                ) -> size_t,
        )
    }) as huf_compress_f;
    cLitSize = huf_compress.expect("non-null function pointer")(
        ostart.offset(lhSize as isize) as *mut ::core::ffi::c_void,
        dstCapacity.wrapping_sub(lhSize),
        src,
        srcSize,
        HUF_SYMBOLVALUE_MAX as ::core::ffi::c_uint,
        LitHufLog as ::core::ffi::c_uint,
        entropyWorkspace,
        entropyWorkspaceSize,
        &raw mut (*nextHuf).CTable as *mut HUF_CElt,
        &raw mut repeat,
        flags,
    );
    if repeat as ::core::ffi::c_uint != HUF_repeat_none as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        hType = set_repeat;
    }
    let minGain: size_t = ZSTD_minGain(srcSize, strategy) as size_t;
    if cLitSize == 0 as size_t
        || cLitSize >= srcSize.wrapping_sub(minGain)
        || ERR_isError(cLitSize) != 0
    {
        ::libc::memcpy(
            nextHuf as *mut ::core::ffi::c_void,
            prevHuf as *const ::core::ffi::c_void,
            ::core::mem::size_of::<ZSTD_hufCTables_t>() as ::libc::size_t,
        );
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }
    if cLitSize == 1 as size_t {
        if srcSize >= 8 as size_t || allBytesIdentical(src, srcSize) != 0 {
            ::libc::memcpy(
                nextHuf as *mut ::core::ffi::c_void,
                prevHuf as *const ::core::ffi::c_void,
                ::core::mem::size_of::<ZSTD_hufCTables_t>() as ::libc::size_t,
            );
            return ZSTD_compressRleLiteralsBlock(dst, dstCapacity, src, srcSize);
        }
    }
    if hType as ::core::ffi::c_uint == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint {
        (*nextHuf).repeatMode = HUF_repeat_check;
    }
    match lhSize {
        3 => {
            singleStream == 0;
            let lhc: U32 = (hType as U32)
                .wrapping_add(
                    ((singleStream == 0) as ::core::ffi::c_int as U32) << 2 as ::core::ffi::c_int,
                )
                .wrapping_add((srcSize as U32) << 4 as ::core::ffi::c_int)
                .wrapping_add((cLitSize as U32) << 14 as ::core::ffi::c_int);
            MEM_writeLE24(ostart as *mut ::core::ffi::c_void, lhc);
        }
        4 => {
            let lhc_0: U32 = (hType as U32)
                .wrapping_add(((2 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                .wrapping_add((srcSize as U32) << 4 as ::core::ffi::c_int)
                .wrapping_add((cLitSize as U32) << 18 as ::core::ffi::c_int);
            MEM_writeLE32(ostart as *mut ::core::ffi::c_void, lhc_0);
        }
        5 => {
            let lhc_1: U32 = (hType as U32)
                .wrapping_add(((3 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                .wrapping_add((srcSize as U32) << 4 as ::core::ffi::c_int)
                .wrapping_add((cLitSize as U32) << 22 as ::core::ffi::c_int);
            MEM_writeLE32(ostart as *mut ::core::ffi::c_void, lhc_1);
            *ostart.offset(4 as ::core::ffi::c_int as isize) =
                (cLitSize >> 10 as ::core::ffi::c_int) as BYTE;
        }
        _ => {}
    }
    return lhSize.wrapping_add(cLitSize);
}
