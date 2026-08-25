use ::libc;
extern "C" {
    fn FSE_readNCount_bmi2(
        normalizedCounter: *mut ::core::ffi::c_short,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        tableLogPtr: *mut ::core::ffi::c_uint,
        rBuffer: *const ::core::ffi::c_void,
        rBuffSize: size_t,
        bmi2: ::core::ffi::c_int,
    ) -> size_t;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __int16_t = i16;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type int16_t = __int16_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type BYTE = uint8_t;
pub type U16 = uint16_t;
pub type S16 = int16_t;
pub type U32 = uint32_t;
pub type U64 = uint64_t;
pub type unalign32 = U32;
pub type unalign64 = U64;
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
pub type BitContainerType = size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BIT_DStream_t {
    pub bitContainer: BitContainerType,
    pub bitsConsumed: ::core::ffi::c_uint,
    pub ptr: *const ::core::ffi::c_char,
    pub start: *const ::core::ffi::c_char,
    pub limitPtr: *const ::core::ffi::c_char,
}
pub type BIT_DStream_status = ::core::ffi::c_uint;
pub const BIT_DStream_overflow: BIT_DStream_status = 3;
pub const BIT_DStream_completed: BIT_DStream_status = 2;
pub const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
pub const BIT_DStream_unfinished: BIT_DStream_status = 0;
pub type FSE_DTable = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_decode_t {
    pub newState: ::core::ffi::c_ushort,
    pub symbol: ::core::ffi::c_uchar,
    pub nbBits: ::core::ffi::c_uchar,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DecompressWksp {
    pub ncount: [::core::ffi::c_short; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DState_t {
    pub state: size_t,
    pub table: *const ::core::ffi::c_void,
}
#[inline]
unsafe extern "C" fn MEM_32bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<size_t>() as usize == 4 as usize) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_isLittleEndian() -> ::core::ffi::c_uint {
    return 1 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_read32(mut ptr: *const ::core::ffi::c_void) -> U32 {
    return *(ptr as *const unalign32);
}
#[inline]
unsafe extern "C" fn MEM_read64(mut ptr: *const ::core::ffi::c_void) -> U64 {
    return *(ptr as *const unalign64);
}
#[inline]
unsafe extern "C" fn MEM_write64(mut memPtr: *mut ::core::ffi::c_void, mut value: U64) {
    *(memPtr as *mut unalign64) = value as unalign64;
}
#[inline]
unsafe extern "C" fn MEM_swap32(mut in_0: U32) -> U32 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_swap64(mut in_0: U64) -> U64 {
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
#[inline]
unsafe extern "C" fn MEM_readLE64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read64(memPtr);
    } else {
        return MEM_swap64(MEM_read64(memPtr));
    };
}
#[inline]
unsafe extern "C" fn MEM_readLEST(mut memPtr: *const ::core::ffi::c_void) -> size_t {
    if MEM_32bits() != 0 {
        return MEM_readLE32(memPtr) as size_t;
    } else {
        return MEM_readLE64(memPtr) as size_t;
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
#[inline]
unsafe extern "C" fn ZSTD_countLeadingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
#[inline]
unsafe extern "C" fn BIT_initDStream(
    mut bitD: *mut BIT_DStream_t,
    mut srcBuffer: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize < 1 as size_t {
        ::libc::memset(
            bitD as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<BIT_DStream_t>() as ::libc::size_t,
        );
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
    (*bitD).limitPtr = (*bitD)
        .start
        .offset(::core::mem::size_of::<BitContainerType>() as usize as isize);
    if srcSize >= ::core::mem::size_of::<BitContainerType>() as usize {
        (*bitD).ptr = (srcBuffer as *const ::core::ffi::c_char)
            .offset(srcSize as isize)
            .offset(-(::core::mem::size_of::<BitContainerType>() as usize as isize));
        (*bitD).bitContainer =
            MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void) as BitContainerType;
        let lastByte: BYTE =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize);
        (*bitD).bitsConsumed = if lastByte as ::core::ffi::c_int != 0 {
            (8 as ::core::ffi::c_uint).wrapping_sub(ZSTD_highbit32(lastByte as U32))
        } else {
            0 as ::core::ffi::c_uint
        };
        if lastByte as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
    } else {
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as BitContainerType;
        let mut current_block_32: u64;
        match srcSize {
            7 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(6 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(16 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 4456380586314209854;
            }
            6 => {
                current_block_32 = 4456380586314209854;
            }
            5 => {
                current_block_32 = 4123378386445440636;
            }
            4 => {
                current_block_32 = 5027660998269810477;
            }
            3 => {
                current_block_32 = 9705207487401268650;
            }
            2 => {
                current_block_32 = 12242499427560368975;
            }
            _ => {
                current_block_32 = 16203760046146113240;
            }
        }
        match current_block_32 {
            4456380586314209854 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(5 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(24 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 4123378386445440636;
            }
            _ => {}
        }
        match current_block_32 {
            4123378386445440636 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(4 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(32 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 5027660998269810477;
            }
            _ => {}
        }
        match current_block_32 {
            5027660998269810477 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(3 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 9705207487401268650;
            }
            _ => {}
        }
        match current_block_32 {
            9705207487401268650 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 12242499427560368975;
            }
            _ => {}
        }
        match current_block_32 {
            12242499427560368975 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(1 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
            }
            _ => {}
        }
        let lastByte_0: BYTE =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize);
        (*bitD).bitsConsumed = if lastByte_0 as ::core::ffi::c_int != 0 {
            (8 as ::core::ffi::c_uint).wrapping_sub(ZSTD_highbit32(lastByte_0 as U32))
        } else {
            0 as ::core::ffi::c_uint
        };
        if lastByte_0 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((::core::mem::size_of::<BitContainerType>() as usize).wrapping_sub(srcSize as usize)
                as U32)
                .wrapping_mul(8 as U32) as ::core::ffi::c_uint,
        );
    }
    return srcSize;
}
#[inline(always)]
unsafe extern "C" fn BIT_getMiddleBits(
    mut bitContainer: BitContainerType,
    start: U32,
    nbBits: U32,
) -> BitContainerType {
    let regMask: U32 = (::core::mem::size_of::<BitContainerType>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return bitContainer >> (start & regMask)
        & ((1 as ::core::ffi::c_int as BitContainerType) << nbBits)
            .wrapping_sub(1 as BitContainerType);
}
#[inline(always)]
unsafe extern "C" fn BIT_lookBits(
    mut bitD: *const BIT_DStream_t,
    mut nbBits: U32,
) -> BitContainerType {
    return BIT_getMiddleBits(
        (*bitD).bitContainer,
        (::core::mem::size_of::<BitContainerType>() as usize)
            .wrapping_mul(8 as usize)
            .wrapping_sub((*bitD).bitsConsumed as usize)
            .wrapping_sub(nbBits as usize) as U32,
        nbBits,
    );
}
#[inline]
unsafe extern "C" fn BIT_lookBitsFast(
    mut bitD: *const BIT_DStream_t,
    mut nbBits: U32,
) -> BitContainerType {
    let regMask: U32 = (::core::mem::size_of::<BitContainerType>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & regMask)
        >> (regMask.wrapping_add(1 as U32).wrapping_sub(nbBits) & regMask);
}
#[inline(always)]
unsafe extern "C" fn BIT_skipBits(mut bitD: *mut BIT_DStream_t, mut nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_add(nbBits as ::core::ffi::c_uint);
}
#[inline(always)]
unsafe extern "C" fn BIT_readBits(
    mut bitD: *mut BIT_DStream_t,
    mut nbBits: ::core::ffi::c_uint,
) -> BitContainerType {
    let value: BitContainerType = BIT_lookBits(bitD, nbBits as U32) as BitContainerType;
    BIT_skipBits(bitD, nbBits as U32);
    return value;
}
#[inline]
unsafe extern "C" fn BIT_readBitsFast(
    mut bitD: *mut BIT_DStream_t,
    mut nbBits: ::core::ffi::c_uint,
) -> size_t {
    let value: BitContainerType = BIT_lookBitsFast(bitD, nbBits as U32) as BitContainerType;
    BIT_skipBits(bitD, nbBits as U32);
    return value as size_t;
}
#[inline]
unsafe extern "C" fn BIT_reloadDStream_internal(
    mut bitD: *mut BIT_DStream_t,
) -> BIT_DStream_status {
    (*bitD).ptr = (*bitD)
        .ptr
        .offset(-(((*bitD).bitsConsumed >> 3 as ::core::ffi::c_int) as isize));
    (*bitD).bitsConsumed &= 7 as ::core::ffi::c_uint;
    (*bitD).bitContainer =
        MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void) as BitContainerType;
    return BIT_DStream_unfinished;
}
#[inline(always)]
unsafe extern "C" fn BIT_reloadDStream(mut bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if ((*bitD).bitsConsumed as usize
        > (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize))
        as ::core::ffi::c_int as ::core::ffi::c_long
        != 0
    {
        static mut zeroFilled: BitContainerType = 0 as BitContainerType;
        (*bitD).ptr = &raw const zeroFilled as *const ::core::ffi::c_char;
        return BIT_DStream_overflow;
    }
    if (*bitD).ptr >= (*bitD).limitPtr {
        return BIT_reloadDStream_internal(bitD);
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize)
            < (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
        {
            return BIT_DStream_endOfBuffer;
        }
        return BIT_DStream_completed;
    }
    let mut nbBytes: U32 = (*bitD).bitsConsumed as U32 >> 3 as ::core::ffi::c_int;
    let mut result: BIT_DStream_status = BIT_DStream_unfinished;
    if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
        nbBytes = (*bitD).ptr.offset_from((*bitD).start) as ::core::ffi::c_long as U32;
        result = BIT_DStream_endOfBuffer;
    }
    (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_sub(nbBytes.wrapping_mul(8 as U32) as ::core::ffi::c_uint);
    (*bitD).bitContainer =
        MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void) as BitContainerType;
    return result;
}
#[inline]
unsafe extern "C" fn FSE_initDState(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut BIT_DStream_t,
    mut dt: *const FSE_DTable,
) {
    let mut ptr: *const ::core::ffi::c_void = dt as *const ::core::ffi::c_void;
    let DTableH: *const FSE_DTableHeader = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = BIT_readBits(bitD, (*DTableH).tableLog as ::core::ffi::c_uint) as size_t;
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
}
#[inline]
unsafe extern "C" fn FSE_decodeSymbol(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut BIT_DStream_t,
) -> ::core::ffi::c_uchar {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol as BYTE;
    let lowBits: size_t = BIT_readBits(bitD, nbBits as ::core::ffi::c_uint) as size_t;
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol as ::core::ffi::c_uchar;
}
#[inline]
unsafe extern "C" fn FSE_decodeSymbolFast(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut BIT_DStream_t,
) -> ::core::ffi::c_uchar {
    let DInfo: FSE_decode_t =
        *((*DStatePtr).table as *const FSE_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol as BYTE;
    let lowBits: size_t = BIT_readBitsFast(bitD, nbBits as ::core::ffi::c_uint) as size_t;
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol as ::core::ffi::c_uchar;
}
pub const FSE_MAX_MEMORY_USAGE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const FSE_MAX_SYMBOL_VALUE: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const FSE_MAX_TABLELOG: ::core::ffi::c_int = FSE_MAX_MEMORY_USAGE - 2 as ::core::ffi::c_int;
unsafe extern "C" fn FSE_buildDTable_internal(
    mut dt: *mut FSE_DTable,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    let tdPtr: *mut ::core::ffi::c_void =
        dt.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let tableDecode: *mut FSE_decode_t = tdPtr as *mut FSE_decode_t;
    let mut symbolNext: *mut U16 = workSpace as *mut U16;
    let mut spread: *mut BYTE = symbolNext
        .offset(maxSymbolValue as isize)
        .offset(1 as ::core::ffi::c_int as isize) as *mut BYTE;
    let maxSV1: U32 = (maxSymbolValue as U32).wrapping_add(1 as U32);
    let tableSize: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1 as U32);
    if ((::core::mem::size_of::<::core::ffi::c_short>() as usize)
        .wrapping_mul(maxSymbolValue.wrapping_add(1 as ::core::ffi::c_uint) as usize)
        as ::core::ffi::c_ulonglong)
        .wrapping_add((1 as ::core::ffi::c_ulonglong) << tableLog)
        .wrapping_add(8 as ::core::ffi::c_ulonglong)
        > wkspSize as ::core::ffi::c_ulonglong
    {
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if maxSymbolValue > FSE_MAX_SYMBOL_VALUE as ::core::ffi::c_uint {
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog > FSE_MAX_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    DTableH.tableLog = tableLog as U16;
    DTableH.fastMode = 1 as U16;
    let largeLimit: S16 =
        ((1 as ::core::ffi::c_int) << tableLog.wrapping_sub(1 as ::core::ffi::c_uint)) as S16;
    let mut s: U32 = 0;
    s = 0 as U32;
    while s < maxSV1 {
        if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int == -(1 as ::core::ffi::c_int)
        {
            let fresh0 = highThreshold;
            highThreshold = highThreshold.wrapping_sub(1);
            (*tableDecode.offset(fresh0 as isize)).symbol = s as BYTE as ::core::ffi::c_uchar;
            *symbolNext.offset(s as isize) = 1 as U16;
        } else {
            if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int
                >= largeLimit as ::core::ffi::c_int
            {
                DTableH.fastMode = 0 as U16;
            }
            *symbolNext.offset(s as isize) = *normalizedCounter.offset(s as isize) as U16;
        }
        s = s.wrapping_add(1);
    }
    ::libc::memcpy(
        dt as *mut ::core::ffi::c_void,
        &raw mut DTableH as *const ::core::ffi::c_void,
        ::core::mem::size_of::<FSE_DTableHeader>() as ::libc::size_t,
    );
    if highThreshold == tableSize.wrapping_sub(1 as U32) {
        let tableMask: size_t = tableSize.wrapping_sub(1 as U32) as size_t;
        let step: size_t = (tableSize >> 1 as ::core::ffi::c_int)
            .wrapping_add(tableSize >> 3 as ::core::ffi::c_int)
            .wrapping_add(3 as U32) as size_t;
        let add: U64 = 0x101010101010101 as U64;
        let mut pos: size_t = 0 as size_t;
        let mut sv: U64 = 0 as U64;
        let mut s_0: U32 = 0;
        s_0 = 0 as U32;
        while s_0 < maxSV1 {
            let mut i: ::core::ffi::c_int = 0;
            let n: ::core::ffi::c_int =
                *normalizedCounter.offset(s_0 as isize) as ::core::ffi::c_int;
            MEM_write64(spread.offset(pos as isize) as *mut ::core::ffi::c_void, sv);
            i = 8 as ::core::ffi::c_int;
            while i < n {
                MEM_write64(
                    spread.offset(pos as isize).offset(i as isize) as *mut ::core::ffi::c_void,
                    sv,
                );
                i += 8 as ::core::ffi::c_int;
            }
            pos = (pos as ::core::ffi::c_ulong).wrapping_add(n as size_t as ::core::ffi::c_ulong)
                as size_t as size_t;
            s_0 = s_0.wrapping_add(1);
            sv = (sv as ::core::ffi::c_ulong).wrapping_add(add as ::core::ffi::c_ulong) as U64
                as U64;
        }
        let mut position: size_t = 0 as size_t;
        let mut s_1: size_t = 0;
        let unroll: size_t = 2 as size_t;
        s_1 = 0 as size_t;
        while s_1 < tableSize as size_t {
            let mut u: size_t = 0;
            u = 0 as size_t;
            while u < unroll {
                let uPosition: size_t = position.wrapping_add(u.wrapping_mul(step)) & tableMask;
                (*tableDecode.offset(uPosition as isize)).symbol =
                    *spread.offset(s_1.wrapping_add(u) as isize) as ::core::ffi::c_uchar;
                u = u.wrapping_add(1);
            }
            position = position.wrapping_add(unroll.wrapping_mul(step)) & tableMask;
            s_1 = (s_1 as ::core::ffi::c_ulong).wrapping_add(unroll as ::core::ffi::c_ulong)
                as size_t as size_t;
        }
    } else {
        let tableMask_0: U32 = tableSize.wrapping_sub(1 as U32);
        let step_0: U32 = (tableSize >> 1 as ::core::ffi::c_int)
            .wrapping_add(tableSize >> 3 as ::core::ffi::c_int)
            .wrapping_add(3 as U32);
        let mut s_2: U32 = 0;
        let mut position_0: U32 = 0 as U32;
        s_2 = 0 as U32;
        while s_2 < maxSV1 {
            let mut i_0: ::core::ffi::c_int = 0;
            i_0 = 0 as ::core::ffi::c_int;
            while i_0 < *normalizedCounter.offset(s_2 as isize) as ::core::ffi::c_int {
                (*tableDecode.offset(position_0 as isize)).symbol =
                    s_2 as BYTE as ::core::ffi::c_uchar;
                position_0 = position_0.wrapping_add(step_0) & tableMask_0;
                while position_0 > highThreshold {
                    position_0 = position_0.wrapping_add(step_0) & tableMask_0;
                }
                i_0 += 1;
            }
            s_2 = s_2.wrapping_add(1);
        }
        if position_0 != 0 as U32 {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
    }
    let mut u_0: U32 = 0;
    u_0 = 0 as U32;
    while u_0 < tableSize {
        let symbol: BYTE = (*tableDecode.offset(u_0 as isize)).symbol as BYTE;
        let ref mut fresh1 = *symbolNext.offset(symbol as isize);
        let fresh2 = *fresh1;
        *fresh1 = (*fresh1).wrapping_add(1);
        let nextState: U32 = fresh2 as U32;
        (*tableDecode.offset(u_0 as isize)).nbBits =
            tableLog.wrapping_sub(ZSTD_highbit32(nextState)) as BYTE as ::core::ffi::c_uchar;
        (*tableDecode.offset(u_0 as isize)).newState =
            (nextState << (*tableDecode.offset(u_0 as isize)).nbBits as ::core::ffi::c_int)
                .wrapping_sub(tableSize) as U16 as ::core::ffi::c_ushort;
        u_0 = u_0.wrapping_add(1);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildDTable_wksp(
    mut dt: *mut FSE_DTable,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    return FSE_buildDTable_internal(
        dt,
        normalizedCounter,
        maxSymbolValue,
        tableLog,
        workSpace,
        wkspSize,
    );
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
    let mut bitD: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut state1: FSE_DState_t = FSE_DState_t {
        state: 0,
        table: ::core::ptr::null::<::core::ffi::c_void>(),
    };
    let mut state2: FSE_DState_t = FSE_DState_t {
        state: 0,
        table: ::core::ptr::null::<::core::ffi::c_void>(),
    };
    let _var_err__: size_t = BIT_initDStream(&raw mut bitD, cSrc, cSrcSize) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    FSE_initDState(&raw mut state1, &raw mut bitD, dt);
    FSE_initDState(&raw mut state2, &raw mut bitD, dt);
    if BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
        == BIT_DStream_overflow as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    while (BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
        == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int
        & (op < olimit) as ::core::ffi::c_int
        != 0
    {
        *op.offset(0 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSE_MAX_TABLELOG * 2 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
        {
            BIT_reloadDStream(&raw mut bitD);
        }
        *op.offset(1 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSE_MAX_TABLELOG * 4 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
        {
            if BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
                > BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
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
            > (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
        {
            BIT_reloadDStream(&raw mut bitD);
        }
        *op.offset(3 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        op = op.offset(4 as ::core::ffi::c_int as isize);
    }
    loop {
        if op > omax.offset(-(2 as ::core::ffi::c_int as isize)) {
            return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
        }
        let fresh3 = op;
        op = op.offset(1);
        *fresh3 = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
            == BIT_DStream_overflow as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let fresh4 = op;
            op = op.offset(1);
            *fresh4 = (if fast != 0 {
                FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            } else {
                FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            }) as BYTE;
            break;
        } else {
            if op > omax.offset(-(2 as ::core::ffi::c_int as isize)) {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            let fresh5 = op;
            op = op.offset(1);
            *fresh5 = (if fast != 0 {
                FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            } else {
                FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            }) as BYTE;
            if !(BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
                == BIT_DStream_overflow as ::core::ffi::c_int as ::core::ffi::c_uint)
            {
                continue;
            }
            let fresh6 = op;
            op = op.offset(1);
            *fresh6 = (if fast != 0 {
                FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
            } else {
                FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
            }) as BYTE;
            break;
        }
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[inline(always)]
unsafe extern "C" fn FSE_decompress_wksp_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut maxLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    let istart: *const BYTE = cSrc as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut tableLog: ::core::ffi::c_uint = 0;
    let mut maxSymbolValue: ::core::ffi::c_uint = FSE_MAX_SYMBOL_VALUE as ::core::ffi::c_uint;
    let wksp: *mut FSE_DecompressWksp = workSpace as *mut FSE_DecompressWksp;
    let dtablePos: size_t = (::core::mem::size_of::<FSE_DecompressWksp>() as size_t)
        .wrapping_div(::core::mem::size_of::<FSE_DTable>() as size_t);
    let dtable: *mut FSE_DTable = (workSpace as *mut FSE_DTable).offset(dtablePos as isize);
    if wkspSize < ::core::mem::size_of::<FSE_DecompressWksp>() as usize {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    let NCountLength: size_t = FSE_readNCount_bmi2(
        &raw mut (*wksp).ncount as *mut ::core::ffi::c_short,
        &raw mut maxSymbolValue,
        &raw mut tableLog,
        istart as *const ::core::ffi::c_void,
        cSrcSize,
        bmi2,
    ) as size_t;
    if ERR_isError(NCountLength) != 0 {
        return NCountLength;
    }
    if tableLog > maxLog {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(NCountLength as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(NCountLength as ::core::ffi::c_ulong)
        as size_t as size_t;
    if ((1 as ::core::ffi::c_int
        + ((1 as ::core::ffi::c_int) << tableLog)
        + 1 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong)
        .wrapping_add(
            ((::core::mem::size_of::<::core::ffi::c_short>() as usize)
                .wrapping_mul(maxSymbolValue.wrapping_add(1 as ::core::ffi::c_uint) as usize)
                as ::core::ffi::c_ulonglong)
                .wrapping_add((1 as ::core::ffi::c_ulonglong) << tableLog)
                .wrapping_add(8 as ::core::ffi::c_ulonglong)
                .wrapping_add(
                    ::core::mem::size_of::<::core::ffi::c_uint>() as ::core::ffi::c_ulonglong
                )
                .wrapping_sub(1 as ::core::ffi::c_ulonglong)
                .wrapping_div(
                    ::core::mem::size_of::<::core::ffi::c_uint>() as ::core::ffi::c_ulonglong
                ),
        )
        .wrapping_add(
            ((FSE_MAX_SYMBOL_VALUE + 1 as ::core::ffi::c_int) / 2 as ::core::ffi::c_int)
                as ::core::ffi::c_ulonglong,
        )
        .wrapping_add(1 as ::core::ffi::c_ulonglong)
        .wrapping_mul(::core::mem::size_of::<::core::ffi::c_uint>() as ::core::ffi::c_ulonglong)
        > wkspSize as ::core::ffi::c_ulonglong
    {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    workSpace = (workSpace as *mut BYTE)
        .offset(::core::mem::size_of::<FSE_DecompressWksp>() as usize as isize)
        .offset(
            ((1 as ::core::ffi::c_int + ((1 as ::core::ffi::c_int) << tableLog)) as usize)
                .wrapping_mul(::core::mem::size_of::<FSE_DTable>() as usize) as isize,
        ) as *mut ::core::ffi::c_void;
    wkspSize = (wkspSize as ::core::ffi::c_ulong).wrapping_sub(
        (::core::mem::size_of::<FSE_DecompressWksp>() as usize).wrapping_add(
            ((1 as ::core::ffi::c_int + ((1 as ::core::ffi::c_int) << tableLog)) as usize)
                .wrapping_mul(::core::mem::size_of::<FSE_DTable>() as usize),
        ) as ::core::ffi::c_ulong,
    ) as size_t as size_t;
    let _var_err__: size_t = FSE_buildDTable_internal(
        dtable,
        &raw mut (*wksp).ncount as *mut ::core::ffi::c_short,
        maxSymbolValue,
        tableLog,
        workSpace,
        wkspSize,
    ) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    let mut ptr: *const ::core::ffi::c_void = dtable as *const ::core::ffi::c_void;
    let mut DTableH: *const FSE_DTableHeader = ptr as *const FSE_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;
    if fastMode != 0 {
        return FSE_decompress_usingDTable_generic(
            dst,
            dstCapacity,
            ip as *const ::core::ffi::c_void,
            cSrcSize,
            dtable,
            1 as ::core::ffi::c_uint,
        );
    }
    return FSE_decompress_usingDTable_generic(
        dst,
        dstCapacity,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        dtable,
        0 as ::core::ffi::c_uint,
    );
}
unsafe extern "C" fn FSE_decompress_wksp_body_default(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut maxLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    return FSE_decompress_wksp_body(
        dst,
        dstCapacity,
        cSrc,
        cSrcSize,
        maxLog,
        workSpace,
        wkspSize,
        0 as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_decompress_wksp_bmi2(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut maxLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    return FSE_decompress_wksp_body_default(
        dst,
        dstCapacity,
        cSrc,
        cSrcSize,
        maxLog,
        workSpace,
        wkspSize,
    );
}
