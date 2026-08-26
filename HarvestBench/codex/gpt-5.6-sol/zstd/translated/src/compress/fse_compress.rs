pub type ptrdiff_t = isize;
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type BYTE = uint8_t;
pub type U16 = uint16_t;
pub type U32 = uint32_t;
pub type U64 = uint64_t;
pub type unalign16 = U16;
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
pub struct BIT_CStream_t {
    pub bitContainer: BitContainerType,
    pub bitPos: ::core::ffi::c_uint,
    pub startPtr: *mut ::core::ffi::c_char,
    pub ptr: *mut ::core::ffi::c_char,
    pub endPtr: *mut ::core::ffi::c_char,
}
pub type FSE_CTable = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_CState_t {
    pub value: ptrdiff_t,
    pub stateTable: *const ::core::ffi::c_void,
    pub symbolTT: *const ::core::ffi::c_void,
    pub stateLog: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_symbolCompressionTransform {
    pub deltaFindState: ::core::ffi::c_int,
    pub deltaNbBits: U32,
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
unsafe extern "C" fn MEM_read16(mut ptr: *const ::core::ffi::c_void) -> U16 {
    return *(ptr as *const unalign16);
}
#[inline]
unsafe extern "C" fn MEM_write32(mut memPtr: *mut ::core::ffi::c_void, mut value: U32) {
    *(memPtr as *mut unalign32) = value as unalign32;
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
unsafe extern "C" fn MEM_writeLE32(mut memPtr: *mut ::core::ffi::c_void, mut val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, val32);
    } else {
        MEM_write32(memPtr, MEM_swap32(val32));
    };
}
#[inline]
unsafe extern "C" fn MEM_writeLE64(mut memPtr: *mut ::core::ffi::c_void, mut val64: U64) {
    if MEM_isLittleEndian() != 0 {
        MEM_write64(memPtr, val64);
    } else {
        MEM_write64(memPtr, MEM_swap64(val64));
    };
}
#[inline]
unsafe extern "C" fn MEM_writeLEST(mut memPtr: *mut ::core::ffi::c_void, mut val: size_t) {
    if MEM_32bits() != 0 {
        MEM_writeLE32(memPtr, val as U32);
    } else {
        MEM_writeLE64(memPtr, val as U64);
    };
}
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_countLeadingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
static mut BIT_mask: [::core::ffi::c_uint; 32] = [
    0 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1 as ::core::ffi::c_int as ::core::ffi::c_uint,
    3 as ::core::ffi::c_int as ::core::ffi::c_uint,
    7 as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xf as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1f as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3f as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7f as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1ff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3ff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7ff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xfff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1fff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3fff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7fff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1ffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3ffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7ffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xfffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1fffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3fffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7fffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1ffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3ffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7ffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0xfffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x1fffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x3fffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
    0x7fffffff as ::core::ffi::c_int as ::core::ffi::c_uint,
];
#[inline]
unsafe extern "C" fn BIT_initCStream(
    mut bitC: *mut BIT_CStream_t,
    mut startPtr: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
) -> size_t {
    (*bitC).bitContainer = 0 as BitContainerType;
    (*bitC).bitPos = 0 as ::core::ffi::c_uint;
    (*bitC).startPtr = startPtr as *mut ::core::ffi::c_char;
    (*bitC).ptr = (*bitC).startPtr;
    (*bitC).endPtr = (*bitC)
        .startPtr
        .offset(dstCapacity as isize)
        .offset(-(::core::mem::size_of::<BitContainerType>() as usize as isize));
    if dstCapacity <= ::core::mem::size_of::<BitContainerType>() as usize {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    return 0 as size_t;
}
#[inline(always)]
unsafe extern "C" fn BIT_getLowerBits(
    mut bitContainer: BitContainerType,
    nbBits: U32,
) -> BitContainerType {
    return bitContainer & BIT_mask[nbBits as usize] as BitContainerType;
}
#[inline]
unsafe extern "C" fn BIT_addBits(
    mut bitC: *mut BIT_CStream_t,
    mut value: BitContainerType,
    mut nbBits: ::core::ffi::c_uint,
) {
    (*bitC).bitContainer = ((*bitC).bitContainer as ::core::ffi::c_ulong
        | (BIT_getLowerBits(value, nbBits as U32) << (*bitC).bitPos) as ::core::ffi::c_ulong)
        as BitContainerType;
    (*bitC).bitPos = (*bitC).bitPos.wrapping_add(nbBits);
}
#[inline]
unsafe extern "C" fn BIT_addBitsFast(
    mut bitC: *mut BIT_CStream_t,
    mut value: BitContainerType,
    mut nbBits: ::core::ffi::c_uint,
) {
    (*bitC).bitContainer = ((*bitC).bitContainer as ::core::ffi::c_ulong
        | (value << (*bitC).bitPos) as ::core::ffi::c_ulong)
        as BitContainerType;
    (*bitC).bitPos = (*bitC).bitPos.wrapping_add(nbBits);
}
#[inline]
unsafe extern "C" fn BIT_flushBitsFast(mut bitC: *mut BIT_CStream_t) {
    let nbBytes: size_t = ((*bitC).bitPos >> 3 as ::core::ffi::c_int) as size_t;
    MEM_writeLEST(
        (*bitC).ptr as *mut ::core::ffi::c_void,
        (*bitC).bitContainer as size_t,
    );
    (*bitC).ptr = (*bitC).ptr.offset(nbBytes as isize);
    (*bitC).bitPos &= 7 as ::core::ffi::c_uint;
    (*bitC).bitContainer >>= nbBytes.wrapping_mul(8 as size_t);
}
#[inline]
unsafe extern "C" fn BIT_flushBits(mut bitC: *mut BIT_CStream_t) {
    let nbBytes: size_t = ((*bitC).bitPos >> 3 as ::core::ffi::c_int) as size_t;
    MEM_writeLEST(
        (*bitC).ptr as *mut ::core::ffi::c_void,
        (*bitC).bitContainer as size_t,
    );
    (*bitC).ptr = (*bitC).ptr.offset(nbBytes as isize);
    if (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
    (*bitC).bitPos &= 7 as ::core::ffi::c_uint;
    (*bitC).bitContainer >>= nbBytes.wrapping_mul(8 as size_t);
}
#[inline]
unsafe extern "C" fn BIT_closeCStream(mut bitC: *mut BIT_CStream_t) -> size_t {
    BIT_addBitsFast(bitC, 1 as BitContainerType, 1 as ::core::ffi::c_uint);
    BIT_flushBits(bitC);
    if (*bitC).ptr >= (*bitC).endPtr {
        return 0 as size_t;
    }
    return ((*bitC).ptr.offset_from((*bitC).startPtr) as ::core::ffi::c_long as size_t)
        .wrapping_add(((*bitC).bitPos > 0 as ::core::ffi::c_uint) as ::core::ffi::c_int as size_t);
}
pub const FSE_NCOUNTBOUND: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn FSE_initCState(mut statePtr: *mut FSE_CState_t, mut ct: *const FSE_CTable) {
    let mut ptr: *const ::core::ffi::c_void = ct as *const ::core::ffi::c_void;
    let mut u16ptr: *const U16 = ptr as *const U16;
    let tableLog: U32 = MEM_read16(ptr) as U32;
    (*statePtr).value = (1 as ::core::ffi::c_int as ptrdiff_t) << tableLog;
    (*statePtr).stateTable =
        u16ptr.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    (*statePtr).symbolTT = ct.offset(1 as ::core::ffi::c_int as isize).offset(
        (if tableLog != 0 {
            (1 as ::core::ffi::c_int) << tableLog.wrapping_sub(1 as U32)
        } else {
            1 as ::core::ffi::c_int
        }) as isize,
    ) as *const ::core::ffi::c_void;
    (*statePtr).stateLog = tableLog as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn FSE_initCState2(
    mut statePtr: *mut FSE_CState_t,
    mut ct: *const FSE_CTable,
    mut symbol: U32,
) {
    FSE_initCState(statePtr, ct);
    let symbolTT: FSE_symbolCompressionTransform =
        *((*statePtr).symbolTT as *const FSE_symbolCompressionTransform).offset(symbol as isize);
    let mut stateTable: *const U16 = (*statePtr).stateTable as *const U16;
    let mut nbBitsOut: U32 = symbolTT
        .deltaNbBits
        .wrapping_add(((1 as ::core::ffi::c_int) << 15 as ::core::ffi::c_int) as U32)
        >> 16 as ::core::ffi::c_int;
    (*statePtr).value =
        (nbBitsOut << 16 as ::core::ffi::c_int).wrapping_sub(symbolTT.deltaNbBits) as ptrdiff_t;
    (*statePtr).value = *stateTable
        .offset((((*statePtr).value >> nbBitsOut) + symbolTT.deltaFindState as ptrdiff_t) as isize)
        as ptrdiff_t;
}
#[inline]
unsafe extern "C" fn FSE_encodeSymbol(
    mut bitC: *mut BIT_CStream_t,
    mut statePtr: *mut FSE_CState_t,
    mut symbol: ::core::ffi::c_uint,
) {
    let symbolTT: FSE_symbolCompressionTransform =
        *((*statePtr).symbolTT as *const FSE_symbolCompressionTransform).offset(symbol as isize);
    let stateTable: *const U16 = (*statePtr).stateTable as *const U16;
    let nbBitsOut: U32 =
        ((*statePtr).value + symbolTT.deltaNbBits as ptrdiff_t >> 16 as ::core::ffi::c_int) as U32;
    BIT_addBits(
        bitC,
        (*statePtr).value as BitContainerType,
        nbBitsOut as ::core::ffi::c_uint,
    );
    (*statePtr).value = *stateTable
        .offset((((*statePtr).value >> nbBitsOut) + symbolTT.deltaFindState as ptrdiff_t) as isize)
        as ptrdiff_t;
}
#[inline]
unsafe extern "C" fn FSE_flushCState(
    mut bitC: *mut BIT_CStream_t,
    mut statePtr: *const FSE_CState_t,
) {
    BIT_addBits(
        bitC,
        (*statePtr).value as BitContainerType,
        (*statePtr).stateLog,
    );
    BIT_flushBits(bitC);
}
pub const FSE_MAX_MEMORY_USAGE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const FSE_DEFAULT_MEMORY_USAGE: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const FSE_MAX_TABLELOG: ::core::ffi::c_int = FSE_MAX_MEMORY_USAGE - 2 as ::core::ffi::c_int;
pub const FSE_DEFAULT_TABLELOG: ::core::ffi::c_int =
    FSE_DEFAULT_MEMORY_USAGE - 2 as ::core::ffi::c_int;
pub const FSE_MIN_TABLELOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_wksp(
    mut ct: *mut FSE_CTable,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    let tableSize: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let tableMask: U32 = tableSize.wrapping_sub(1 as U32);
    let ptr: *mut ::core::ffi::c_void = ct as *mut ::core::ffi::c_void;
    let tableU16: *mut U16 = (ptr as *mut U16).offset(2 as ::core::ffi::c_int as isize);
    let FSCT: *mut ::core::ffi::c_void = (ptr as *mut U32)
        .offset(1 as ::core::ffi::c_int as isize)
        .offset(
            (if tableLog != 0 {
                tableSize >> 1 as ::core::ffi::c_int
            } else {
                1 as U32
            }) as isize,
        ) as *mut ::core::ffi::c_void;
    let symbolTT: *mut FSE_symbolCompressionTransform = FSCT as *mut FSE_symbolCompressionTransform;
    let step: U32 = (tableSize >> 1 as ::core::ffi::c_int)
        .wrapping_add(tableSize >> 3 as ::core::ffi::c_int)
        .wrapping_add(3 as U32);
    let maxSV1: U32 = (maxSymbolValue as U32).wrapping_add(1 as U32);
    let mut cumul: *mut U16 = workSpace as *mut U16;
    let tableSymbol: *mut BYTE = cumul.offset(maxSV1.wrapping_add(1 as U32) as isize) as *mut BYTE;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1 as U32);
    if (::core::mem::size_of::<::core::ffi::c_uint>() as ::core::ffi::c_ulonglong).wrapping_mul(
        (maxSymbolValue.wrapping_add(2 as ::core::ffi::c_uint) as ::core::ffi::c_ulonglong)
            .wrapping_add((1 as ::core::ffi::c_ulonglong) << tableLog)
            .wrapping_div(2 as ::core::ffi::c_ulonglong)
            .wrapping_add(
                (::core::mem::size_of::<U64>() as usize)
                    .wrapping_div(::core::mem::size_of::<U32>() as usize)
                    as ::core::ffi::c_ulonglong,
            ),
    ) > wkspSize as ::core::ffi::c_ulonglong
    {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    *tableU16.offset(-(2 as ::core::ffi::c_int) as isize) = tableLog as U16;
    *tableU16.offset(-(1 as ::core::ffi::c_int) as isize) = maxSymbolValue as U16;
    let mut u: U32 = 0;
    *cumul.offset(0 as ::core::ffi::c_int as isize) = 0 as U16;
    u = 1 as U32;
    while u <= maxSV1 {
        if *normalizedCounter.offset(u.wrapping_sub(1 as U32) as isize) as ::core::ffi::c_int
            == -(1 as ::core::ffi::c_int)
        {
            *cumul.offset(u as isize) = (*cumul.offset(u.wrapping_sub(1 as U32) as isize)
                as ::core::ffi::c_int
                + 1 as ::core::ffi::c_int) as U16;
            let fresh4 = highThreshold;
            highThreshold = highThreshold.wrapping_sub(1);
            *tableSymbol.offset(fresh4 as isize) = u.wrapping_sub(1 as U32) as BYTE;
        } else {
            *cumul.offset(u as isize) = (*cumul.offset(u.wrapping_sub(1 as U32) as isize)
                as ::core::ffi::c_int
                + *normalizedCounter.offset(u.wrapping_sub(1 as U32) as isize) as U16
                    as ::core::ffi::c_int) as U16;
        }
        u = u.wrapping_add(1);
    }
    *cumul.offset(maxSV1 as isize) = tableSize.wrapping_add(1 as U32) as U16;
    if highThreshold == tableSize.wrapping_sub(1 as U32) {
        let spread: *mut BYTE = tableSymbol.offset(tableSize as isize);
        let add: U64 = 0x101010101010101 as U64;
        let mut pos: size_t = 0 as size_t;
        let mut sv: U64 = 0 as U64;
        let mut s: U32 = 0;
        s = 0 as U32;
        while s < maxSV1 {
            let mut i: ::core::ffi::c_int = 0;
            let n: ::core::ffi::c_int = *normalizedCounter.offset(s as isize) as ::core::ffi::c_int;
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
            s = s.wrapping_add(1);
            sv = (sv as ::core::ffi::c_ulong).wrapping_add(add as ::core::ffi::c_ulong) as U64
                as U64;
        }
        let mut position: size_t = 0 as size_t;
        let mut s_0: size_t = 0;
        let unroll: size_t = 2 as size_t;
        s_0 = 0 as size_t;
        while s_0 < tableSize as size_t {
            let mut u_0: size_t = 0;
            u_0 = 0 as size_t;
            while u_0 < unroll {
                let uPosition: size_t =
                    position.wrapping_add(u_0.wrapping_mul(step as size_t)) & tableMask as size_t;
                *tableSymbol.offset(uPosition as isize) =
                    *spread.offset(s_0.wrapping_add(u_0) as isize);
                u_0 = u_0.wrapping_add(1);
            }
            position =
                position.wrapping_add(unroll.wrapping_mul(step as size_t)) & tableMask as size_t;
            s_0 = (s_0 as ::core::ffi::c_ulong).wrapping_add(unroll as ::core::ffi::c_ulong)
                as size_t as size_t;
        }
    } else {
        let mut position_0: U32 = 0 as U32;
        let mut symbol: U32 = 0;
        symbol = 0 as U32;
        while symbol < maxSV1 {
            let mut nbOccurrences: ::core::ffi::c_int = 0;
            let freq: ::core::ffi::c_int =
                *normalizedCounter.offset(symbol as isize) as ::core::ffi::c_int;
            nbOccurrences = 0 as ::core::ffi::c_int;
            while nbOccurrences < freq {
                *tableSymbol.offset(position_0 as isize) = symbol as BYTE;
                position_0 = position_0.wrapping_add(step) & tableMask;
                while position_0 > highThreshold {
                    position_0 = position_0.wrapping_add(step) & tableMask;
                }
                nbOccurrences += 1;
            }
            symbol = symbol.wrapping_add(1);
        }
    }
    let mut u_1: U32 = 0;
    u_1 = 0 as U32;
    while u_1 < tableSize {
        let mut s_1: BYTE = *tableSymbol.offset(u_1 as isize);
        let ref mut fresh5 = *cumul.offset(s_1 as isize);
        let fresh6 = *fresh5;
        *fresh5 = (*fresh5).wrapping_add(1);
        *tableU16.offset(fresh6 as isize) = tableSize.wrapping_add(u_1) as U16;
        u_1 = u_1.wrapping_add(1);
    }
    let mut total: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut s_2: ::core::ffi::c_uint = 0;
    s_2 = 0 as ::core::ffi::c_uint;
    while s_2 <= maxSymbolValue {
        match *normalizedCounter.offset(s_2 as isize) as ::core::ffi::c_int {
            0 => {
                (*symbolTT.offset(s_2 as isize)).deltaNbBits = (tableLog
                    .wrapping_add(1 as ::core::ffi::c_uint)
                    << 16 as ::core::ffi::c_int)
                    .wrapping_sub(((1 as ::core::ffi::c_int) << tableLog) as ::core::ffi::c_uint)
                    as U32;
            }
            -1 | 1 => {
                (*symbolTT.offset(s_2 as isize)).deltaNbBits = (tableLog
                    << 16 as ::core::ffi::c_int)
                    .wrapping_sub(((1 as ::core::ffi::c_int) << tableLog) as ::core::ffi::c_uint)
                    as U32;
                (*symbolTT.offset(s_2 as isize)).deltaFindState =
                    total.wrapping_sub(1 as ::core::ffi::c_uint) as ::core::ffi::c_int;
                total = total.wrapping_add(1);
            }
            _ => {
                let maxBitsOut: U32 = (tableLog as U32).wrapping_sub(ZSTD_highbit32(
                    (*normalizedCounter.offset(s_2 as isize) as U32).wrapping_sub(1 as U32),
                ) as U32);
                let minStatePlus: U32 =
                    (*normalizedCounter.offset(s_2 as isize) as U32) << maxBitsOut;
                (*symbolTT.offset(s_2 as isize)).deltaNbBits =
                    (maxBitsOut << 16 as ::core::ffi::c_int).wrapping_sub(minStatePlus);
                (*symbolTT.offset(s_2 as isize)).deltaFindState = total
                    .wrapping_sub(*normalizedCounter.offset(s_2 as isize) as ::core::ffi::c_uint)
                    as ::core::ffi::c_int;
                total = total
                    .wrapping_add(*normalizedCounter.offset(s_2 as isize) as ::core::ffi::c_uint);
            }
        }
        s_2 = s_2.wrapping_add(1);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_NCountWriteBound(
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
) -> size_t {
    let maxHeaderSize: size_t = maxSymbolValue
        .wrapping_add(1 as ::core::ffi::c_uint)
        .wrapping_mul(tableLog)
        .wrapping_add(4 as ::core::ffi::c_uint)
        .wrapping_add(2 as ::core::ffi::c_uint)
        .wrapping_div(8 as ::core::ffi::c_uint)
        .wrapping_add(1 as ::core::ffi::c_uint)
        .wrapping_add(2 as ::core::ffi::c_uint) as size_t;
    return if maxSymbolValue != 0 {
        maxHeaderSize
    } else {
        FSE_NCOUNTBOUND as size_t
    };
}
unsafe extern "C" fn FSE_writeNCount_generic(
    mut header: *mut ::core::ffi::c_void,
    mut headerBufferSize: size_t,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
    mut writeIsSafe: ::core::ffi::c_uint,
) -> size_t {
    let ostart: *mut BYTE = header as *mut BYTE;
    let mut out: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.offset(headerBufferSize as isize);
    let mut nbBits: ::core::ffi::c_int = 0;
    let tableSize: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << tableLog;
    let mut remaining: ::core::ffi::c_int = 0;
    let mut threshold: ::core::ffi::c_int = 0;
    let mut bitStream: U32 = 0 as U32;
    let mut bitCount: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut symbol: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let alphabetSize: ::core::ffi::c_uint = maxSymbolValue.wrapping_add(1 as ::core::ffi::c_uint);
    let mut previousIs0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    bitStream = (bitStream as ::core::ffi::c_uint)
        .wrapping_add(tableLog.wrapping_sub(FSE_MIN_TABLELOG as ::core::ffi::c_uint) << bitCount)
        as U32 as U32;
    bitCount += 4 as ::core::ffi::c_int;
    remaining = tableSize + 1 as ::core::ffi::c_int;
    threshold = tableSize;
    nbBits = tableLog as ::core::ffi::c_int + 1 as ::core::ffi::c_int;
    while symbol < alphabetSize && remaining > 1 as ::core::ffi::c_int {
        if previousIs0 != 0 {
            let mut start: ::core::ffi::c_uint = symbol;
            while symbol < alphabetSize && *normalizedCounter.offset(symbol as isize) == 0 {
                symbol = symbol.wrapping_add(1);
            }
            if symbol == alphabetSize {
                break;
            }
            while symbol >= start.wrapping_add(24 as ::core::ffi::c_uint) {
                start = start.wrapping_add(24 as ::core::ffi::c_uint);
                bitStream = (bitStream as ::core::ffi::c_uint)
                    .wrapping_add((0xffff as ::core::ffi::c_uint) << bitCount)
                    as U32 as U32;
                if writeIsSafe == 0 && out > oend.offset(-(2 as ::core::ffi::c_int as isize)) {
                    return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
                }
                *out.offset(0 as ::core::ffi::c_int as isize) = bitStream as BYTE;
                *out.offset(1 as ::core::ffi::c_int as isize) =
                    (bitStream >> 8 as ::core::ffi::c_int) as BYTE;
                out = out.offset(2 as ::core::ffi::c_int as isize);
                bitStream >>= 16 as ::core::ffi::c_int;
            }
            while symbol >= start.wrapping_add(3 as ::core::ffi::c_uint) {
                start = start.wrapping_add(3 as ::core::ffi::c_uint);
                bitStream = (bitStream as ::core::ffi::c_uint)
                    .wrapping_add((3 as ::core::ffi::c_uint) << bitCount)
                    as U32 as U32;
                bitCount += 2 as ::core::ffi::c_int;
            }
            bitStream = (bitStream as ::core::ffi::c_uint)
                .wrapping_add(symbol.wrapping_sub(start) << bitCount) as U32
                as U32;
            bitCount += 2 as ::core::ffi::c_int;
            if bitCount > 16 as ::core::ffi::c_int {
                if writeIsSafe == 0 && out > oend.offset(-(2 as ::core::ffi::c_int as isize)) {
                    return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
                }
                *out.offset(0 as ::core::ffi::c_int as isize) = bitStream as BYTE;
                *out.offset(1 as ::core::ffi::c_int as isize) =
                    (bitStream >> 8 as ::core::ffi::c_int) as BYTE;
                out = out.offset(2 as ::core::ffi::c_int as isize);
                bitStream >>= 16 as ::core::ffi::c_int;
                bitCount -= 16 as ::core::ffi::c_int;
            }
        }
        let fresh3 = symbol;
        symbol = symbol.wrapping_add(1);
        let mut count: ::core::ffi::c_int =
            *normalizedCounter.offset(fresh3 as isize) as ::core::ffi::c_int;
        let max: ::core::ffi::c_int =
            2 as ::core::ffi::c_int * threshold - 1 as ::core::ffi::c_int - remaining;
        remaining -= if count < 0 as ::core::ffi::c_int {
            -count
        } else {
            count
        };
        count += 1;
        if count >= threshold {
            count += max;
        }
        bitStream = (bitStream as ::core::ffi::c_uint)
            .wrapping_add(((count as U32) << bitCount) as ::core::ffi::c_uint)
            as U32 as U32;
        bitCount += nbBits;
        bitCount -= (count < max) as ::core::ffi::c_int;
        previousIs0 = (count == 1 as ::core::ffi::c_int) as ::core::ffi::c_int;
        if remaining < 1 as ::core::ffi::c_int {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
        while remaining < threshold {
            nbBits -= 1;
            threshold >>= 1 as ::core::ffi::c_int;
        }
        if bitCount > 16 as ::core::ffi::c_int {
            if writeIsSafe == 0 && out > oend.offset(-(2 as ::core::ffi::c_int as isize)) {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            *out.offset(0 as ::core::ffi::c_int as isize) = bitStream as BYTE;
            *out.offset(1 as ::core::ffi::c_int as isize) =
                (bitStream >> 8 as ::core::ffi::c_int) as BYTE;
            out = out.offset(2 as ::core::ffi::c_int as isize);
            bitStream >>= 16 as ::core::ffi::c_int;
            bitCount -= 16 as ::core::ffi::c_int;
        }
    }
    if remaining != 1 as ::core::ffi::c_int {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    if writeIsSafe == 0 && out > oend.offset(-(2 as ::core::ffi::c_int as isize)) {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    *out.offset(0 as ::core::ffi::c_int as isize) = bitStream as BYTE;
    *out.offset(1 as ::core::ffi::c_int as isize) = (bitStream >> 8 as ::core::ffi::c_int) as BYTE;
    out = out.offset(((bitCount + 7 as ::core::ffi::c_int) / 8 as ::core::ffi::c_int) as isize);
    return out.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_writeNCount(
    mut buffer: *mut ::core::ffi::c_void,
    mut bufferSize: size_t,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
) -> size_t {
    if tableLog > FSE_MAX_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog < FSE_MIN_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    if bufferSize < FSE_NCountWriteBound(maxSymbolValue, tableLog) {
        return FSE_writeNCount_generic(
            buffer,
            bufferSize,
            normalizedCounter,
            maxSymbolValue,
            tableLog,
            0 as ::core::ffi::c_uint,
        );
    }
    return FSE_writeNCount_generic(
        buffer,
        bufferSize,
        normalizedCounter,
        maxSymbolValue,
        tableLog,
        1 as ::core::ffi::c_uint,
    );
}
unsafe extern "C" fn FSE_minTableLog(
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
) -> ::core::ffi::c_uint {
    let mut minBitsSrc: U32 = (ZSTD_highbit32(srcSize as U32) as U32).wrapping_add(1 as U32);
    let mut minBitsSymbols: U32 =
        (ZSTD_highbit32(maxSymbolValue as U32) as U32).wrapping_add(2 as U32);
    let mut minBits: U32 = if minBitsSrc < minBitsSymbols {
        minBitsSrc
    } else {
        minBitsSymbols
    };
    return minBits as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog_internal(
    mut maxTableLog: ::core::ffi::c_uint,
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut minus: ::core::ffi::c_uint,
) -> ::core::ffi::c_uint {
    let mut maxBitsSrc: U32 = (ZSTD_highbit32(srcSize.wrapping_sub(1 as size_t) as U32) as U32)
        .wrapping_sub(minus as U32);
    let mut tableLog: U32 = maxTableLog as U32;
    let mut minBits: U32 = FSE_minTableLog(srcSize, maxSymbolValue) as U32;
    if tableLog == 0 as U32 {
        tableLog = FSE_DEFAULT_TABLELOG as U32;
    }
    if maxBitsSrc < tableLog {
        tableLog = maxBitsSrc;
    }
    if minBits > tableLog {
        tableLog = minBits;
    }
    if tableLog < FSE_MIN_TABLELOG as U32 {
        tableLog = FSE_MIN_TABLELOG as U32;
    }
    if tableLog > FSE_MAX_TABLELOG as U32 {
        tableLog = FSE_MAX_TABLELOG as U32;
    }
    return tableLog as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_optimalTableLog(
    mut maxTableLog: ::core::ffi::c_uint,
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
) -> ::core::ffi::c_uint {
    return FSE_optimalTableLog_internal(
        maxTableLog,
        srcSize,
        maxSymbolValue,
        2 as ::core::ffi::c_uint,
    );
}
unsafe extern "C" fn FSE_normalizeM2(
    mut norm: *mut ::core::ffi::c_short,
    mut tableLog: U32,
    mut count: *const ::core::ffi::c_uint,
    mut total: size_t,
    mut maxSymbolValue: U32,
    mut lowProbCount: ::core::ffi::c_short,
) -> size_t {
    let NOT_YET_ASSIGNED: ::core::ffi::c_short = -(2 as ::core::ffi::c_int) as ::core::ffi::c_short;
    let mut s: U32 = 0;
    let mut distributed: U32 = 0 as U32;
    let mut ToDistribute: U32 = 0;
    let lowThreshold: U32 = (total >> tableLog) as U32;
    let mut lowOne: U32 =
        (total.wrapping_mul(3 as size_t) >> tableLog.wrapping_add(1 as U32)) as U32;
    s = 0 as U32;
    while s <= maxSymbolValue {
        if *count.offset(s as isize) == 0 as ::core::ffi::c_uint {
            *norm.offset(s as isize) = 0 as ::core::ffi::c_short;
        } else if *count.offset(s as isize) as U32 <= lowThreshold {
            *norm.offset(s as isize) = lowProbCount;
            distributed = distributed.wrapping_add(1);
            total = (total as ::core::ffi::c_ulong)
                .wrapping_sub(*count.offset(s as isize) as ::core::ffi::c_ulong)
                as size_t as size_t;
        } else if *count.offset(s as isize) as U32 <= lowOne {
            *norm.offset(s as isize) = 1 as ::core::ffi::c_short;
            distributed = distributed.wrapping_add(1);
            total = (total as ::core::ffi::c_ulong)
                .wrapping_sub(*count.offset(s as isize) as ::core::ffi::c_ulong)
                as size_t as size_t;
        } else {
            *norm.offset(s as isize) = NOT_YET_ASSIGNED;
        }
        s = s.wrapping_add(1);
    }
    ToDistribute = (((1 as ::core::ffi::c_int) << tableLog) as U32).wrapping_sub(distributed);
    if ToDistribute == 0 as U32 {
        return 0 as size_t;
    }
    if total.wrapping_div(ToDistribute as size_t) > lowOne as size_t {
        lowOne = total
            .wrapping_mul(3 as size_t)
            .wrapping_div(ToDistribute.wrapping_mul(2 as U32) as size_t) as U32;
        s = 0 as U32;
        while s <= maxSymbolValue {
            if *norm.offset(s as isize) as ::core::ffi::c_int
                == NOT_YET_ASSIGNED as ::core::ffi::c_int
                && *count.offset(s as isize) as U32 <= lowOne
            {
                *norm.offset(s as isize) = 1 as ::core::ffi::c_short;
                distributed = distributed.wrapping_add(1);
                total = (total as ::core::ffi::c_ulong)
                    .wrapping_sub(*count.offset(s as isize) as ::core::ffi::c_ulong)
                    as size_t as size_t;
            }
            s = s.wrapping_add(1);
        }
        ToDistribute = (((1 as ::core::ffi::c_int) << tableLog) as U32).wrapping_sub(distributed);
    }
    if distributed == maxSymbolValue.wrapping_add(1 as U32) {
        let mut maxV: U32 = 0 as U32;
        let mut maxC: U32 = 0 as U32;
        s = 0 as U32;
        while s <= maxSymbolValue {
            if *count.offset(s as isize) as U32 > maxC {
                maxV = s;
                maxC = *count.offset(s as isize) as U32;
            }
            s = s.wrapping_add(1);
        }
        let ref mut fresh1 = *norm.offset(maxV as isize);
        *fresh1 = (*fresh1 as ::core::ffi::c_int
            + ToDistribute as ::core::ffi::c_short as ::core::ffi::c_int)
            as ::core::ffi::c_short;
        return 0 as size_t;
    }
    if total == 0 as size_t {
        s = 0 as U32;
        while ToDistribute > 0 as U32 {
            if *norm.offset(s as isize) as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                ToDistribute = ToDistribute.wrapping_sub(1);
                let ref mut fresh2 = *norm.offset(s as isize);
                *fresh2 += 1;
            }
            s = s
                .wrapping_add(1 as U32)
                .wrapping_rem(maxSymbolValue.wrapping_add(1 as U32));
        }
        return 0 as size_t;
    }
    let vStepLog: U64 = (62 as U32).wrapping_sub(tableLog) as U64;
    let mid: U64 = ((1 as ::core::ffi::c_ulonglong) << vStepLog.wrapping_sub(1 as U64))
        .wrapping_sub(1 as ::core::ffi::c_ulonglong) as U64;
    let rStep: U64 = ((1 as ::core::ffi::c_int as U64) << vStepLog)
        .wrapping_mul(ToDistribute as U64)
        .wrapping_add(mid)
        .wrapping_div(total as U32 as U64);
    let mut tmpTotal: U64 = mid;
    s = 0 as U32;
    while s <= maxSymbolValue {
        if *norm.offset(s as isize) as ::core::ffi::c_int == NOT_YET_ASSIGNED as ::core::ffi::c_int
        {
            let end: U64 =
                tmpTotal.wrapping_add((*count.offset(s as isize) as U64).wrapping_mul(rStep));
            let sStart: U32 = (tmpTotal >> vStepLog) as U32;
            let sEnd: U32 = (end >> vStepLog) as U32;
            let weight: U32 = sEnd.wrapping_sub(sStart);
            if weight < 1 as U32 {
                return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
            }
            *norm.offset(s as isize) = weight as ::core::ffi::c_short;
            tmpTotal = end;
        }
        s = s.wrapping_add(1);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_normalizeCount(
    mut normalizedCounter: *mut ::core::ffi::c_short,
    mut tableLog: ::core::ffi::c_uint,
    mut count: *const ::core::ffi::c_uint,
    mut total: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut useLowProbCount: ::core::ffi::c_uint,
) -> size_t {
    if tableLog == 0 as ::core::ffi::c_uint {
        tableLog = FSE_DEFAULT_TABLELOG as ::core::ffi::c_uint;
    }
    if tableLog < FSE_MIN_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    if tableLog > FSE_MAX_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog < FSE_minTableLog(total, maxSymbolValue) {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    static mut rtbTable: [U32; 8] = [
        0 as ::core::ffi::c_int as U32,
        473195 as ::core::ffi::c_int as U32,
        504333 as ::core::ffi::c_int as U32,
        520860 as ::core::ffi::c_int as U32,
        550000 as ::core::ffi::c_int as U32,
        700000 as ::core::ffi::c_int as U32,
        750000 as ::core::ffi::c_int as U32,
        830000 as ::core::ffi::c_int as U32,
    ];
    let lowProbCount: ::core::ffi::c_short = (if useLowProbCount != 0 {
        -(1 as ::core::ffi::c_int)
    } else {
        1 as ::core::ffi::c_int
    }) as ::core::ffi::c_short;
    let scale: U64 = (62 as ::core::ffi::c_uint).wrapping_sub(tableLog) as U64;
    let step: U64 = ((1 as ::core::ffi::c_int as U64) << 62 as ::core::ffi::c_int)
        .wrapping_div(total as U32 as U64);
    let vStep: U64 = ((1 as ::core::ffi::c_ulonglong) << scale.wrapping_sub(20 as U64)) as U64;
    let mut stillToDistribute: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << tableLog;
    let mut s: ::core::ffi::c_uint = 0;
    let mut largest: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut largestP: ::core::ffi::c_short = 0 as ::core::ffi::c_short;
    let mut lowThreshold: U32 = (total >> tableLog) as U32;
    s = 0 as ::core::ffi::c_uint;
    while s <= maxSymbolValue {
        if *count.offset(s as isize) as size_t == total {
            return 0 as size_t;
        }
        if *count.offset(s as isize) == 0 as ::core::ffi::c_uint {
            *normalizedCounter.offset(s as isize) = 0 as ::core::ffi::c_short;
        } else if *count.offset(s as isize) as U32 <= lowThreshold {
            *normalizedCounter.offset(s as isize) = lowProbCount;
            stillToDistribute -= 1;
        } else {
            let mut proba: ::core::ffi::c_short = ((*count.offset(s as isize) as U64)
                .wrapping_mul(step)
                >> scale) as ::core::ffi::c_short;
            if (proba as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
                let mut restToBeat: U64 = vStep.wrapping_mul(rtbTable[proba as usize] as U64);
                proba = (proba as ::core::ffi::c_int
                    + ((*count.offset(s as isize) as U64)
                        .wrapping_mul(step)
                        .wrapping_sub((proba as U64) << scale)
                        > restToBeat) as ::core::ffi::c_int)
                    as ::core::ffi::c_short;
            }
            if proba as ::core::ffi::c_int > largestP as ::core::ffi::c_int {
                largestP = proba;
                largest = s;
            }
            *normalizedCounter.offset(s as isize) = proba;
            stillToDistribute -= proba as ::core::ffi::c_int;
        }
        s = s.wrapping_add(1);
    }
    if -stillToDistribute
        >= *normalizedCounter.offset(largest as isize) as ::core::ffi::c_int
            >> 1 as ::core::ffi::c_int
    {
        let errorCode: size_t = FSE_normalizeM2(
            normalizedCounter,
            tableLog as U32,
            count,
            total,
            maxSymbolValue as U32,
            lowProbCount,
        ) as size_t;
        if ERR_isError(errorCode) != 0 {
            return errorCode;
        }
    } else {
        let ref mut fresh0 = *normalizedCounter.offset(largest as isize);
        *fresh0 = (*fresh0 as ::core::ffi::c_int
            + stillToDistribute as ::core::ffi::c_short as ::core::ffi::c_int)
            as ::core::ffi::c_short;
    }
    return tableLog as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_buildCTable_rle(
    mut ct: *mut FSE_CTable,
    mut symbolValue: BYTE,
) -> size_t {
    let mut ptr: *mut ::core::ffi::c_void = ct as *mut ::core::ffi::c_void;
    let mut tableU16: *mut U16 = (ptr as *mut U16).offset(2 as ::core::ffi::c_int as isize);
    let mut FSCTptr: *mut ::core::ffi::c_void =
        (ptr as *mut U32).offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let mut symbolTT: *mut FSE_symbolCompressionTransform =
        FSCTptr as *mut FSE_symbolCompressionTransform;
    *tableU16.offset(-(2 as ::core::ffi::c_int) as isize) = 0 as ::core::ffi::c_int as U16;
    *tableU16.offset(-(1 as ::core::ffi::c_int) as isize) = symbolValue as U16;
    *tableU16.offset(0 as ::core::ffi::c_int as isize) = 0 as U16;
    *tableU16.offset(1 as ::core::ffi::c_int as isize) = 0 as U16;
    (*symbolTT.offset(symbolValue as isize)).deltaNbBits = 0 as U32;
    (*symbolTT.offset(symbolValue as isize)).deltaFindState = 0 as ::core::ffi::c_int;
    return 0 as size_t;
}
unsafe extern "C" fn FSE_compress_usingCTable_generic(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut ct: *const FSE_CTable,
    fast: ::core::ffi::c_uint,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let mut ip: *const BYTE = iend;
    let mut bitC: BIT_CStream_t = BIT_CStream_t {
        bitContainer: 0,
        bitPos: 0,
        startPtr: ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ptr: ::core::ptr::null_mut::<::core::ffi::c_char>(),
        endPtr: ::core::ptr::null_mut::<::core::ffi::c_char>(),
    };
    let mut CState1: FSE_CState_t = FSE_CState_t {
        value: 0,
        stateTable: ::core::ptr::null::<::core::ffi::c_void>(),
        symbolTT: ::core::ptr::null::<::core::ffi::c_void>(),
        stateLog: 0,
    };
    let mut CState2: FSE_CState_t = FSE_CState_t {
        value: 0,
        stateTable: ::core::ptr::null::<::core::ffi::c_void>(),
        symbolTT: ::core::ptr::null::<::core::ffi::c_void>(),
        stateLog: 0,
    };
    if srcSize <= 2 as size_t {
        return 0 as size_t;
    }
    let initError: size_t = BIT_initCStream(&raw mut bitC, dst, dstSize) as size_t;
    if ERR_isError(initError) != 0 {
        return 0 as size_t;
    }
    if srcSize & 1 as size_t != 0 {
        ip = ip.offset(-1);
        FSE_initCState2(&raw mut CState1, ct, *ip as U32);
        ip = ip.offset(-1);
        FSE_initCState2(&raw mut CState2, ct, *ip as U32);
        ip = ip.offset(-1);
        FSE_encodeSymbol(&raw mut bitC, &raw mut CState1, *ip as ::core::ffi::c_uint);
        if fast != 0 {
            BIT_flushBitsFast(&raw mut bitC);
        } else {
            BIT_flushBits(&raw mut bitC);
        };
    } else {
        ip = ip.offset(-1);
        FSE_initCState2(&raw mut CState2, ct, *ip as U32);
        ip = ip.offset(-1);
        FSE_initCState2(&raw mut CState1, ct, *ip as U32);
    }
    srcSize = (srcSize as ::core::ffi::c_ulong).wrapping_sub(2 as ::core::ffi::c_ulong) as size_t
        as size_t;
    if (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
        > (FSE_MAX_TABLELOG * 4 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
        && srcSize & 2 as size_t != 0
    {
        ip = ip.offset(-1);
        FSE_encodeSymbol(&raw mut bitC, &raw mut CState2, *ip as ::core::ffi::c_uint);
        ip = ip.offset(-1);
        FSE_encodeSymbol(&raw mut bitC, &raw mut CState1, *ip as ::core::ffi::c_uint);
        if fast != 0 {
            BIT_flushBitsFast(&raw mut bitC);
        } else {
            BIT_flushBits(&raw mut bitC);
        };
    }
    while ip > istart {
        ip = ip.offset(-1);
        FSE_encodeSymbol(&raw mut bitC, &raw mut CState2, *ip as ::core::ffi::c_uint);
        if (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
            < (FSE_MAX_TABLELOG * 2 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
        {
            if fast != 0 {
                BIT_flushBitsFast(&raw mut bitC);
            } else {
                BIT_flushBits(&raw mut bitC);
            };
        }
        ip = ip.offset(-1);
        FSE_encodeSymbol(&raw mut bitC, &raw mut CState1, *ip as ::core::ffi::c_uint);
        if (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
            > (FSE_MAX_TABLELOG * 4 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
        {
            ip = ip.offset(-1);
            FSE_encodeSymbol(&raw mut bitC, &raw mut CState2, *ip as ::core::ffi::c_uint);
            ip = ip.offset(-1);
            FSE_encodeSymbol(&raw mut bitC, &raw mut CState1, *ip as ::core::ffi::c_uint);
        }
        if fast != 0 {
            BIT_flushBitsFast(&raw mut bitC);
        } else {
            BIT_flushBits(&raw mut bitC);
        };
    }
    FSE_flushCState(&raw mut bitC, &raw mut CState2);
    FSE_flushCState(&raw mut bitC, &raw mut CState1);
    return BIT_closeCStream(&raw mut bitC);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compress_usingCTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut ct: *const FSE_CTable,
) -> size_t {
    let fast: ::core::ffi::c_uint = (dstSize
        >= srcSize
            .wrapping_add(srcSize >> 7 as ::core::ffi::c_int)
            .wrapping_add(4 as size_t)
            .wrapping_add(::core::mem::size_of::<size_t>() as size_t))
        as ::core::ffi::c_int as ::core::ffi::c_uint;
    if fast != 0 {
        return FSE_compress_usingCTable_generic(
            dst,
            dstSize,
            src,
            srcSize,
            ct,
            1 as ::core::ffi::c_uint,
        );
    } else {
        return FSE_compress_usingCTable_generic(
            dst,
            dstSize,
            src,
            srcSize,
            ct,
            0 as ::core::ffi::c_uint,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_compressBound(mut size: size_t) -> size_t {
    return (FSE_NCOUNTBOUND as size_t).wrapping_add(
        size.wrapping_add(size >> 7 as ::core::ffi::c_int)
            .wrapping_add(4 as size_t)
            .wrapping_add(::core::mem::size_of::<size_t>() as size_t),
    );
}
