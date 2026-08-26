use ::libc;
extern "C" {
    fn FSE_optimalTableLog(
        maxTableLog: ::core::ffi::c_uint,
        srcSize: size_t,
        maxSymbolValue: ::core::ffi::c_uint,
    ) -> ::core::ffi::c_uint;
    fn FSE_normalizeCount(
        normalizedCounter: *mut ::core::ffi::c_short,
        tableLog: ::core::ffi::c_uint,
        count: *const ::core::ffi::c_uint,
        srcSize: size_t,
        maxSymbolValue: ::core::ffi::c_uint,
        useLowProbCount: ::core::ffi::c_uint,
    ) -> size_t;
    fn FSE_writeNCount(
        buffer: *mut ::core::ffi::c_void,
        bufferSize: size_t,
        normalizedCounter: *const ::core::ffi::c_short,
        maxSymbolValue: ::core::ffi::c_uint,
        tableLog: ::core::ffi::c_uint,
    ) -> size_t;
    fn FSE_buildCTable_rle(ct: *mut FSE_CTable, symbolValue: ::core::ffi::c_uchar) -> size_t;
    fn FSE_buildCTable_wksp(
        ct: *mut FSE_CTable,
        normalizedCounter: *const ::core::ffi::c_short,
        maxSymbolValue: ::core::ffi::c_uint,
        tableLog: ::core::ffi::c_uint,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
    ) -> size_t;
}
pub type ptrdiff_t = isize;
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
pub type U8 = uint8_t;
pub type U16 = uint16_t;
pub type S16 = int16_t;
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
pub type SymbolEncodingType_e = ::core::ffi::c_uint;
pub const set_repeat: SymbolEncodingType_e = 3;
pub const set_compressed: SymbolEncodingType_e = 2;
pub const set_rle: SymbolEncodingType_e = 1;
pub const set_basic: SymbolEncodingType_e = 0;
pub type SeqDef = SeqDef_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SeqDef_s {
    pub offBase: U32,
    pub litLength: U16,
    pub mlBase: U16,
}
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
pub type FSE_repeat = ::core::ffi::c_uint;
pub const FSE_repeat_valid: FSE_repeat = 2;
pub const FSE_repeat_check: FSE_repeat = 1;
pub const FSE_repeat_none: FSE_repeat = 0;
pub type FSE_CTable = ::core::ffi::c_uint;
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
pub type ZSTD_DefaultPolicy_e = ::core::ffi::c_uint;
pub const ZSTD_defaultAllowed: ZSTD_DefaultPolicy_e = 1;
pub const ZSTD_defaultDisallowed: ZSTD_DefaultPolicy_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_BuildCTableWksp {
    pub norm: [S16; 53],
    pub wksp: [U32; 285],
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
unsafe extern "C" fn _force_has_format_string(
    mut format: *const ::core::ffi::c_char,
    mut args: ...
) {
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
#[inline]
unsafe extern "C" fn FSE_bitCost(
    mut symbolTTPtr: *const ::core::ffi::c_void,
    mut tableLog: U32,
    mut symbolValue: U32,
    mut accuracyLog: U32,
) -> U32 {
    let mut symbolTT: *const FSE_symbolCompressionTransform =
        symbolTTPtr as *const FSE_symbolCompressionTransform;
    let minNbBits: U32 =
        (*symbolTT.offset(symbolValue as isize)).deltaNbBits >> 16 as ::core::ffi::c_int;
    let threshold: U32 = minNbBits.wrapping_add(1 as U32) << 16 as ::core::ffi::c_int;
    let tableSize: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let deltaFromThreshold: U32 = threshold.wrapping_sub(
        (*symbolTT.offset(symbolValue as isize))
            .deltaNbBits
            .wrapping_add(tableSize),
    );
    let normalizedDeltaFromThreshold: U32 = deltaFromThreshold << accuracyLog >> tableLog;
    let bitMultiplier: U32 = ((1 as ::core::ffi::c_int) << accuracyLog) as U32;
    return minNbBits
        .wrapping_add(1 as U32)
        .wrapping_mul(bitMultiplier)
        .wrapping_sub(normalizedDeltaFromThreshold);
}
pub const MLFSELog: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const LLFSELog: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const OffFSELog: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
static mut LL_bits: [U8; 36] = [
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    6 as ::core::ffi::c_int as U8,
    7 as ::core::ffi::c_int as U8,
    8 as ::core::ffi::c_int as U8,
    9 as ::core::ffi::c_int as U8,
    10 as ::core::ffi::c_int as U8,
    11 as ::core::ffi::c_int as U8,
    12 as ::core::ffi::c_int as U8,
    13 as ::core::ffi::c_int as U8,
    14 as ::core::ffi::c_int as U8,
    15 as ::core::ffi::c_int as U8,
    16 as ::core::ffi::c_int as U8,
];
static mut ML_bits: [U8; 53] = [
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    5 as ::core::ffi::c_int as U8,
    7 as ::core::ffi::c_int as U8,
    8 as ::core::ffi::c_int as U8,
    9 as ::core::ffi::c_int as U8,
    10 as ::core::ffi::c_int as U8,
    11 as ::core::ffi::c_int as U8,
    12 as ::core::ffi::c_int as U8,
    13 as ::core::ffi::c_int as U8,
    14 as ::core::ffi::c_int as U8,
    15 as ::core::ffi::c_int as U8,
    16 as ::core::ffi::c_int as U8,
];
static mut kInverseProbabilityLog256: [::core::ffi::c_uint; 256] = [
    0 as ::core::ffi::c_int as ::core::ffi::c_uint,
    2048 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1792 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1642 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1536 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1453 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1386 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1329 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1280 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1236 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1197 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1162 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1130 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1100 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1073 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1047 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1024 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1001 as ::core::ffi::c_int as ::core::ffi::c_uint,
    980 as ::core::ffi::c_int as ::core::ffi::c_uint,
    960 as ::core::ffi::c_int as ::core::ffi::c_uint,
    941 as ::core::ffi::c_int as ::core::ffi::c_uint,
    923 as ::core::ffi::c_int as ::core::ffi::c_uint,
    906 as ::core::ffi::c_int as ::core::ffi::c_uint,
    889 as ::core::ffi::c_int as ::core::ffi::c_uint,
    874 as ::core::ffi::c_int as ::core::ffi::c_uint,
    859 as ::core::ffi::c_int as ::core::ffi::c_uint,
    844 as ::core::ffi::c_int as ::core::ffi::c_uint,
    830 as ::core::ffi::c_int as ::core::ffi::c_uint,
    817 as ::core::ffi::c_int as ::core::ffi::c_uint,
    804 as ::core::ffi::c_int as ::core::ffi::c_uint,
    791 as ::core::ffi::c_int as ::core::ffi::c_uint,
    779 as ::core::ffi::c_int as ::core::ffi::c_uint,
    768 as ::core::ffi::c_int as ::core::ffi::c_uint,
    756 as ::core::ffi::c_int as ::core::ffi::c_uint,
    745 as ::core::ffi::c_int as ::core::ffi::c_uint,
    734 as ::core::ffi::c_int as ::core::ffi::c_uint,
    724 as ::core::ffi::c_int as ::core::ffi::c_uint,
    714 as ::core::ffi::c_int as ::core::ffi::c_uint,
    704 as ::core::ffi::c_int as ::core::ffi::c_uint,
    694 as ::core::ffi::c_int as ::core::ffi::c_uint,
    685 as ::core::ffi::c_int as ::core::ffi::c_uint,
    676 as ::core::ffi::c_int as ::core::ffi::c_uint,
    667 as ::core::ffi::c_int as ::core::ffi::c_uint,
    658 as ::core::ffi::c_int as ::core::ffi::c_uint,
    650 as ::core::ffi::c_int as ::core::ffi::c_uint,
    642 as ::core::ffi::c_int as ::core::ffi::c_uint,
    633 as ::core::ffi::c_int as ::core::ffi::c_uint,
    626 as ::core::ffi::c_int as ::core::ffi::c_uint,
    618 as ::core::ffi::c_int as ::core::ffi::c_uint,
    610 as ::core::ffi::c_int as ::core::ffi::c_uint,
    603 as ::core::ffi::c_int as ::core::ffi::c_uint,
    595 as ::core::ffi::c_int as ::core::ffi::c_uint,
    588 as ::core::ffi::c_int as ::core::ffi::c_uint,
    581 as ::core::ffi::c_int as ::core::ffi::c_uint,
    574 as ::core::ffi::c_int as ::core::ffi::c_uint,
    567 as ::core::ffi::c_int as ::core::ffi::c_uint,
    561 as ::core::ffi::c_int as ::core::ffi::c_uint,
    554 as ::core::ffi::c_int as ::core::ffi::c_uint,
    548 as ::core::ffi::c_int as ::core::ffi::c_uint,
    542 as ::core::ffi::c_int as ::core::ffi::c_uint,
    535 as ::core::ffi::c_int as ::core::ffi::c_uint,
    529 as ::core::ffi::c_int as ::core::ffi::c_uint,
    523 as ::core::ffi::c_int as ::core::ffi::c_uint,
    517 as ::core::ffi::c_int as ::core::ffi::c_uint,
    512 as ::core::ffi::c_int as ::core::ffi::c_uint,
    506 as ::core::ffi::c_int as ::core::ffi::c_uint,
    500 as ::core::ffi::c_int as ::core::ffi::c_uint,
    495 as ::core::ffi::c_int as ::core::ffi::c_uint,
    489 as ::core::ffi::c_int as ::core::ffi::c_uint,
    484 as ::core::ffi::c_int as ::core::ffi::c_uint,
    478 as ::core::ffi::c_int as ::core::ffi::c_uint,
    473 as ::core::ffi::c_int as ::core::ffi::c_uint,
    468 as ::core::ffi::c_int as ::core::ffi::c_uint,
    463 as ::core::ffi::c_int as ::core::ffi::c_uint,
    458 as ::core::ffi::c_int as ::core::ffi::c_uint,
    453 as ::core::ffi::c_int as ::core::ffi::c_uint,
    448 as ::core::ffi::c_int as ::core::ffi::c_uint,
    443 as ::core::ffi::c_int as ::core::ffi::c_uint,
    438 as ::core::ffi::c_int as ::core::ffi::c_uint,
    434 as ::core::ffi::c_int as ::core::ffi::c_uint,
    429 as ::core::ffi::c_int as ::core::ffi::c_uint,
    424 as ::core::ffi::c_int as ::core::ffi::c_uint,
    420 as ::core::ffi::c_int as ::core::ffi::c_uint,
    415 as ::core::ffi::c_int as ::core::ffi::c_uint,
    411 as ::core::ffi::c_int as ::core::ffi::c_uint,
    407 as ::core::ffi::c_int as ::core::ffi::c_uint,
    402 as ::core::ffi::c_int as ::core::ffi::c_uint,
    398 as ::core::ffi::c_int as ::core::ffi::c_uint,
    394 as ::core::ffi::c_int as ::core::ffi::c_uint,
    390 as ::core::ffi::c_int as ::core::ffi::c_uint,
    386 as ::core::ffi::c_int as ::core::ffi::c_uint,
    382 as ::core::ffi::c_int as ::core::ffi::c_uint,
    377 as ::core::ffi::c_int as ::core::ffi::c_uint,
    373 as ::core::ffi::c_int as ::core::ffi::c_uint,
    370 as ::core::ffi::c_int as ::core::ffi::c_uint,
    366 as ::core::ffi::c_int as ::core::ffi::c_uint,
    362 as ::core::ffi::c_int as ::core::ffi::c_uint,
    358 as ::core::ffi::c_int as ::core::ffi::c_uint,
    354 as ::core::ffi::c_int as ::core::ffi::c_uint,
    350 as ::core::ffi::c_int as ::core::ffi::c_uint,
    347 as ::core::ffi::c_int as ::core::ffi::c_uint,
    343 as ::core::ffi::c_int as ::core::ffi::c_uint,
    339 as ::core::ffi::c_int as ::core::ffi::c_uint,
    336 as ::core::ffi::c_int as ::core::ffi::c_uint,
    332 as ::core::ffi::c_int as ::core::ffi::c_uint,
    329 as ::core::ffi::c_int as ::core::ffi::c_uint,
    325 as ::core::ffi::c_int as ::core::ffi::c_uint,
    322 as ::core::ffi::c_int as ::core::ffi::c_uint,
    318 as ::core::ffi::c_int as ::core::ffi::c_uint,
    315 as ::core::ffi::c_int as ::core::ffi::c_uint,
    311 as ::core::ffi::c_int as ::core::ffi::c_uint,
    308 as ::core::ffi::c_int as ::core::ffi::c_uint,
    305 as ::core::ffi::c_int as ::core::ffi::c_uint,
    302 as ::core::ffi::c_int as ::core::ffi::c_uint,
    298 as ::core::ffi::c_int as ::core::ffi::c_uint,
    295 as ::core::ffi::c_int as ::core::ffi::c_uint,
    292 as ::core::ffi::c_int as ::core::ffi::c_uint,
    289 as ::core::ffi::c_int as ::core::ffi::c_uint,
    286 as ::core::ffi::c_int as ::core::ffi::c_uint,
    282 as ::core::ffi::c_int as ::core::ffi::c_uint,
    279 as ::core::ffi::c_int as ::core::ffi::c_uint,
    276 as ::core::ffi::c_int as ::core::ffi::c_uint,
    273 as ::core::ffi::c_int as ::core::ffi::c_uint,
    270 as ::core::ffi::c_int as ::core::ffi::c_uint,
    267 as ::core::ffi::c_int as ::core::ffi::c_uint,
    264 as ::core::ffi::c_int as ::core::ffi::c_uint,
    261 as ::core::ffi::c_int as ::core::ffi::c_uint,
    258 as ::core::ffi::c_int as ::core::ffi::c_uint,
    256 as ::core::ffi::c_int as ::core::ffi::c_uint,
    253 as ::core::ffi::c_int as ::core::ffi::c_uint,
    250 as ::core::ffi::c_int as ::core::ffi::c_uint,
    247 as ::core::ffi::c_int as ::core::ffi::c_uint,
    244 as ::core::ffi::c_int as ::core::ffi::c_uint,
    241 as ::core::ffi::c_int as ::core::ffi::c_uint,
    239 as ::core::ffi::c_int as ::core::ffi::c_uint,
    236 as ::core::ffi::c_int as ::core::ffi::c_uint,
    233 as ::core::ffi::c_int as ::core::ffi::c_uint,
    230 as ::core::ffi::c_int as ::core::ffi::c_uint,
    228 as ::core::ffi::c_int as ::core::ffi::c_uint,
    225 as ::core::ffi::c_int as ::core::ffi::c_uint,
    222 as ::core::ffi::c_int as ::core::ffi::c_uint,
    220 as ::core::ffi::c_int as ::core::ffi::c_uint,
    217 as ::core::ffi::c_int as ::core::ffi::c_uint,
    215 as ::core::ffi::c_int as ::core::ffi::c_uint,
    212 as ::core::ffi::c_int as ::core::ffi::c_uint,
    209 as ::core::ffi::c_int as ::core::ffi::c_uint,
    207 as ::core::ffi::c_int as ::core::ffi::c_uint,
    204 as ::core::ffi::c_int as ::core::ffi::c_uint,
    202 as ::core::ffi::c_int as ::core::ffi::c_uint,
    199 as ::core::ffi::c_int as ::core::ffi::c_uint,
    197 as ::core::ffi::c_int as ::core::ffi::c_uint,
    194 as ::core::ffi::c_int as ::core::ffi::c_uint,
    192 as ::core::ffi::c_int as ::core::ffi::c_uint,
    190 as ::core::ffi::c_int as ::core::ffi::c_uint,
    187 as ::core::ffi::c_int as ::core::ffi::c_uint,
    185 as ::core::ffi::c_int as ::core::ffi::c_uint,
    182 as ::core::ffi::c_int as ::core::ffi::c_uint,
    180 as ::core::ffi::c_int as ::core::ffi::c_uint,
    178 as ::core::ffi::c_int as ::core::ffi::c_uint,
    175 as ::core::ffi::c_int as ::core::ffi::c_uint,
    173 as ::core::ffi::c_int as ::core::ffi::c_uint,
    171 as ::core::ffi::c_int as ::core::ffi::c_uint,
    168 as ::core::ffi::c_int as ::core::ffi::c_uint,
    166 as ::core::ffi::c_int as ::core::ffi::c_uint,
    164 as ::core::ffi::c_int as ::core::ffi::c_uint,
    162 as ::core::ffi::c_int as ::core::ffi::c_uint,
    159 as ::core::ffi::c_int as ::core::ffi::c_uint,
    157 as ::core::ffi::c_int as ::core::ffi::c_uint,
    155 as ::core::ffi::c_int as ::core::ffi::c_uint,
    153 as ::core::ffi::c_int as ::core::ffi::c_uint,
    151 as ::core::ffi::c_int as ::core::ffi::c_uint,
    149 as ::core::ffi::c_int as ::core::ffi::c_uint,
    146 as ::core::ffi::c_int as ::core::ffi::c_uint,
    144 as ::core::ffi::c_int as ::core::ffi::c_uint,
    142 as ::core::ffi::c_int as ::core::ffi::c_uint,
    140 as ::core::ffi::c_int as ::core::ffi::c_uint,
    138 as ::core::ffi::c_int as ::core::ffi::c_uint,
    136 as ::core::ffi::c_int as ::core::ffi::c_uint,
    134 as ::core::ffi::c_int as ::core::ffi::c_uint,
    132 as ::core::ffi::c_int as ::core::ffi::c_uint,
    130 as ::core::ffi::c_int as ::core::ffi::c_uint,
    128 as ::core::ffi::c_int as ::core::ffi::c_uint,
    126 as ::core::ffi::c_int as ::core::ffi::c_uint,
    123 as ::core::ffi::c_int as ::core::ffi::c_uint,
    121 as ::core::ffi::c_int as ::core::ffi::c_uint,
    119 as ::core::ffi::c_int as ::core::ffi::c_uint,
    117 as ::core::ffi::c_int as ::core::ffi::c_uint,
    115 as ::core::ffi::c_int as ::core::ffi::c_uint,
    114 as ::core::ffi::c_int as ::core::ffi::c_uint,
    112 as ::core::ffi::c_int as ::core::ffi::c_uint,
    110 as ::core::ffi::c_int as ::core::ffi::c_uint,
    108 as ::core::ffi::c_int as ::core::ffi::c_uint,
    106 as ::core::ffi::c_int as ::core::ffi::c_uint,
    104 as ::core::ffi::c_int as ::core::ffi::c_uint,
    102 as ::core::ffi::c_int as ::core::ffi::c_uint,
    100 as ::core::ffi::c_int as ::core::ffi::c_uint,
    98 as ::core::ffi::c_int as ::core::ffi::c_uint,
    96 as ::core::ffi::c_int as ::core::ffi::c_uint,
    94 as ::core::ffi::c_int as ::core::ffi::c_uint,
    93 as ::core::ffi::c_int as ::core::ffi::c_uint,
    91 as ::core::ffi::c_int as ::core::ffi::c_uint,
    89 as ::core::ffi::c_int as ::core::ffi::c_uint,
    87 as ::core::ffi::c_int as ::core::ffi::c_uint,
    85 as ::core::ffi::c_int as ::core::ffi::c_uint,
    83 as ::core::ffi::c_int as ::core::ffi::c_uint,
    82 as ::core::ffi::c_int as ::core::ffi::c_uint,
    80 as ::core::ffi::c_int as ::core::ffi::c_uint,
    78 as ::core::ffi::c_int as ::core::ffi::c_uint,
    76 as ::core::ffi::c_int as ::core::ffi::c_uint,
    74 as ::core::ffi::c_int as ::core::ffi::c_uint,
    73 as ::core::ffi::c_int as ::core::ffi::c_uint,
    71 as ::core::ffi::c_int as ::core::ffi::c_uint,
    69 as ::core::ffi::c_int as ::core::ffi::c_uint,
    67 as ::core::ffi::c_int as ::core::ffi::c_uint,
    66 as ::core::ffi::c_int as ::core::ffi::c_uint,
    64 as ::core::ffi::c_int as ::core::ffi::c_uint,
    62 as ::core::ffi::c_int as ::core::ffi::c_uint,
    61 as ::core::ffi::c_int as ::core::ffi::c_uint,
    59 as ::core::ffi::c_int as ::core::ffi::c_uint,
    57 as ::core::ffi::c_int as ::core::ffi::c_uint,
    55 as ::core::ffi::c_int as ::core::ffi::c_uint,
    54 as ::core::ffi::c_int as ::core::ffi::c_uint,
    52 as ::core::ffi::c_int as ::core::ffi::c_uint,
    50 as ::core::ffi::c_int as ::core::ffi::c_uint,
    49 as ::core::ffi::c_int as ::core::ffi::c_uint,
    47 as ::core::ffi::c_int as ::core::ffi::c_uint,
    46 as ::core::ffi::c_int as ::core::ffi::c_uint,
    44 as ::core::ffi::c_int as ::core::ffi::c_uint,
    42 as ::core::ffi::c_int as ::core::ffi::c_uint,
    41 as ::core::ffi::c_int as ::core::ffi::c_uint,
    39 as ::core::ffi::c_int as ::core::ffi::c_uint,
    37 as ::core::ffi::c_int as ::core::ffi::c_uint,
    36 as ::core::ffi::c_int as ::core::ffi::c_uint,
    34 as ::core::ffi::c_int as ::core::ffi::c_uint,
    33 as ::core::ffi::c_int as ::core::ffi::c_uint,
    31 as ::core::ffi::c_int as ::core::ffi::c_uint,
    30 as ::core::ffi::c_int as ::core::ffi::c_uint,
    28 as ::core::ffi::c_int as ::core::ffi::c_uint,
    26 as ::core::ffi::c_int as ::core::ffi::c_uint,
    25 as ::core::ffi::c_int as ::core::ffi::c_uint,
    23 as ::core::ffi::c_int as ::core::ffi::c_uint,
    22 as ::core::ffi::c_int as ::core::ffi::c_uint,
    20 as ::core::ffi::c_int as ::core::ffi::c_uint,
    19 as ::core::ffi::c_int as ::core::ffi::c_uint,
    17 as ::core::ffi::c_int as ::core::ffi::c_uint,
    16 as ::core::ffi::c_int as ::core::ffi::c_uint,
    14 as ::core::ffi::c_int as ::core::ffi::c_uint,
    13 as ::core::ffi::c_int as ::core::ffi::c_uint,
    11 as ::core::ffi::c_int as ::core::ffi::c_uint,
    10 as ::core::ffi::c_int as ::core::ffi::c_uint,
    8 as ::core::ffi::c_int as ::core::ffi::c_uint,
    7 as ::core::ffi::c_int as ::core::ffi::c_uint,
    5 as ::core::ffi::c_int as ::core::ffi::c_uint,
    4 as ::core::ffi::c_int as ::core::ffi::c_uint,
    2 as ::core::ffi::c_int as ::core::ffi::c_uint,
    1 as ::core::ffi::c_int as ::core::ffi::c_uint,
];
unsafe extern "C" fn ZSTD_getFSEMaxSymbolValue(
    mut ctable: *const FSE_CTable,
) -> ::core::ffi::c_uint {
    let mut ptr: *const ::core::ffi::c_void = ctable as *const ::core::ffi::c_void;
    let mut u16ptr: *const U16 = ptr as *const U16;
    let maxSymbolValue: U32 =
        MEM_read16(u16ptr.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as U32;
    return maxSymbolValue as ::core::ffi::c_uint;
}
unsafe extern "C" fn ZSTD_useLowProbCount(nbSeq: size_t) -> ::core::ffi::c_uint {
    return (nbSeq >= 2048 as size_t) as ::core::ffi::c_int as ::core::ffi::c_uint;
}
unsafe extern "C" fn ZSTD_NCountCost(
    mut count: *const ::core::ffi::c_uint,
    max: ::core::ffi::c_uint,
    nbSeq: size_t,
    FSELog: ::core::ffi::c_uint,
) -> size_t {
    let mut wksp: [BYTE; 512] = [0; 512];
    let mut norm: [S16; 53] = [0; 53];
    let tableLog: U32 = FSE_optimalTableLog(FSELog, nbSeq, max) as U32;
    let err_code: size_t = FSE_normalizeCount(
        &raw mut norm as *mut ::core::ffi::c_short,
        tableLog as ::core::ffi::c_uint,
        count,
        nbSeq,
        max,
        ZSTD_useLowProbCount(nbSeq),
    ) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    return FSE_writeNCount(
        &raw mut wksp as *mut BYTE as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[BYTE; 512]>() as size_t,
        &raw mut norm as *mut S16,
        max,
        tableLog as ::core::ffi::c_uint,
    );
}
unsafe extern "C" fn ZSTD_entropyCost(
    mut count: *const ::core::ffi::c_uint,
    max: ::core::ffi::c_uint,
    total: size_t,
) -> size_t {
    let mut cost: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut s: ::core::ffi::c_uint = 0;
    s = 0 as ::core::ffi::c_uint;
    while s <= max {
        let mut norm: ::core::ffi::c_uint =
            ((256 as ::core::ffi::c_uint).wrapping_mul(*count.offset(s as isize)) as size_t)
                .wrapping_div(total) as ::core::ffi::c_uint;
        if *count.offset(s as isize) != 0 as ::core::ffi::c_uint && norm == 0 as ::core::ffi::c_uint
        {
            norm = 1 as ::core::ffi::c_uint;
        }
        cost = cost.wrapping_add(
            (*count.offset(s as isize)).wrapping_mul(kInverseProbabilityLog256[norm as usize]),
        );
        s = s.wrapping_add(1);
    }
    return (cost >> 8 as ::core::ffi::c_int) as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fseBitCost(
    mut ctable: *const FSE_CTable,
    mut count: *const ::core::ffi::c_uint,
    max: ::core::ffi::c_uint,
) -> size_t {
    let kAccuracyLog: ::core::ffi::c_uint = 8 as ::core::ffi::c_uint;
    let mut cost: size_t = 0 as size_t;
    let mut s: ::core::ffi::c_uint = 0;
    let mut cstate: FSE_CState_t = FSE_CState_t {
        value: 0,
        stateTable: ::core::ptr::null::<::core::ffi::c_void>(),
        symbolTT: ::core::ptr::null::<::core::ffi::c_void>(),
        stateLog: 0,
    };
    FSE_initCState(&raw mut cstate, ctable);
    if ZSTD_getFSEMaxSymbolValue(ctable) < max {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    s = 0 as ::core::ffi::c_uint;
    while s <= max {
        let tableLog: ::core::ffi::c_uint = cstate.stateLog;
        let badCost: ::core::ffi::c_uint =
            tableLog.wrapping_add(1 as ::core::ffi::c_uint) << kAccuracyLog;
        let bitCost: ::core::ffi::c_uint = FSE_bitCost(
            cstate.symbolTT,
            tableLog as U32,
            s as U32,
            kAccuracyLog as U32,
        ) as ::core::ffi::c_uint;
        if !(*count.offset(s as isize) == 0 as ::core::ffi::c_uint) {
            if bitCost >= badCost {
                return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
            }
            cost = (cost as ::core::ffi::c_ulong).wrapping_add(
                (*count.offset(s as isize) as size_t).wrapping_mul(bitCost as size_t)
                    as ::core::ffi::c_ulong,
            ) as size_t as size_t;
        }
        s = s.wrapping_add(1);
    }
    return cost >> kAccuracyLog;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_crossEntropyCost(
    mut norm: *const ::core::ffi::c_short,
    mut accuracyLog: ::core::ffi::c_uint,
    mut count: *const ::core::ffi::c_uint,
    max: ::core::ffi::c_uint,
) -> size_t {
    let shift: ::core::ffi::c_uint = (8 as ::core::ffi::c_uint).wrapping_sub(accuracyLog);
    let mut cost: size_t = 0 as size_t;
    let mut s: ::core::ffi::c_uint = 0;
    s = 0 as ::core::ffi::c_uint;
    while s <= max {
        let normAcc: ::core::ffi::c_uint =
            if *norm.offset(s as isize) as ::core::ffi::c_int != -(1 as ::core::ffi::c_int) {
                *norm.offset(s as isize) as ::core::ffi::c_uint
            } else {
                1 as ::core::ffi::c_uint
            };
        let norm256: ::core::ffi::c_uint = normAcc << shift;
        cost = (cost as ::core::ffi::c_ulong).wrapping_add(
            (*count.offset(s as isize)).wrapping_mul(kInverseProbabilityLog256[norm256 as usize])
                as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        s = s.wrapping_add(1);
    }
    return cost >> 8 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectEncodingType(
    mut repeatMode: *mut FSE_repeat,
    mut count: *const ::core::ffi::c_uint,
    max: ::core::ffi::c_uint,
    mostFrequent: size_t,
    mut nbSeq: size_t,
    FSELog: ::core::ffi::c_uint,
    mut prevCTable: *const FSE_CTable,
    mut defaultNorm: *const ::core::ffi::c_short,
    mut defaultNormLog: U32,
    isDefaultAllowed: ZSTD_DefaultPolicy_e,
    strategy: ZSTD_strategy,
) -> SymbolEncodingType_e {
    if mostFrequent == nbSeq {
        *repeatMode = FSE_repeat_none;
        if isDefaultAllowed as ::core::ffi::c_uint != 0 && nbSeq <= 2 as size_t {
            return set_basic;
        }
        return set_rle;
    }
    if (strategy as ::core::ffi::c_uint) < ZSTD_lazy as ::core::ffi::c_int as ::core::ffi::c_uint {
        if isDefaultAllowed as u64 != 0 {
            let staticFse_nbSeq_max: size_t = 1000 as size_t;
            let mult: size_t =
                (10 as ::core::ffi::c_uint).wrapping_sub(strategy as ::core::ffi::c_uint) as size_t;
            let baseLog: size_t = 3 as size_t;
            let dynamicFse_nbSeq_min: size_t =
                ((1 as ::core::ffi::c_int as size_t) << defaultNormLog).wrapping_mul(mult)
                    >> baseLog;
            if *repeatMode as ::core::ffi::c_uint
                == FSE_repeat_valid as ::core::ffi::c_int as ::core::ffi::c_uint
                && nbSeq < staticFse_nbSeq_max
            {
                return set_repeat;
            }
            if nbSeq < dynamicFse_nbSeq_min
                || mostFrequent < nbSeq >> defaultNormLog.wrapping_sub(1 as U32)
            {
                *repeatMode = FSE_repeat_none;
                return set_basic;
            }
        }
    } else {
        let basicCost: size_t = if isDefaultAllowed as ::core::ffi::c_uint != 0 {
            ZSTD_crossEntropyCost(
                defaultNorm,
                defaultNormLog as ::core::ffi::c_uint,
                count,
                max,
            ) as size_t
        } else {
            -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t
        };
        let repeatCost: size_t = if *repeatMode as ::core::ffi::c_uint
            != FSE_repeat_none as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            ZSTD_fseBitCost(prevCTable, count, max) as size_t
        } else {
            -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t
        };
        let NCountCost: size_t = ZSTD_NCountCost(count, max, nbSeq, FSELog) as size_t;
        let compressedCost: size_t = (NCountCost << 3 as ::core::ffi::c_int)
            .wrapping_add(ZSTD_entropyCost(count, max, nbSeq) as size_t);
        isDefaultAllowed as u64 != 0;
        if basicCost <= repeatCost && basicCost <= compressedCost {
            *repeatMode = FSE_repeat_none;
            return set_basic;
        }
        if repeatCost <= compressedCost {
            return set_repeat;
        }
    }
    *repeatMode = FSE_repeat_check;
    return set_compressed;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildCTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut nextCTable: *mut FSE_CTable,
    mut FSELog: U32,
    mut type_0: SymbolEncodingType_e,
    mut count: *mut ::core::ffi::c_uint,
    mut max: U32,
    mut codeTable: *const BYTE,
    mut nbSeq: size_t,
    mut defaultNorm: *const S16,
    mut defaultNormLog: U32,
    mut defaultMax: U32,
    mut prevCTable: *const FSE_CTable,
    mut prevCTableSize: size_t,
    mut entropyWorkspace: *mut ::core::ffi::c_void,
    mut entropyWorkspaceSize: size_t,
) -> size_t {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *const BYTE = op.offset(dstCapacity as isize);
    match type_0 as ::core::ffi::c_uint {
        1 => {
            let err_code: size_t =
                FSE_buildCTable_rle(nextCTable, max as ::core::ffi::c_uchar) as size_t;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
            if dstCapacity == 0 as size_t {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            *op = *codeTable.offset(0 as ::core::ffi::c_int as isize);
            return 1 as size_t;
        }
        3 => {
            ::libc::memcpy(
                nextCTable as *mut ::core::ffi::c_void,
                prevCTable as *const ::core::ffi::c_void,
                prevCTableSize as ::libc::size_t,
            );
            return 0 as size_t;
        }
        0 => {
            let err_code_0: size_t = FSE_buildCTable_wksp(
                nextCTable,
                defaultNorm as *const ::core::ffi::c_short,
                defaultMax as ::core::ffi::c_uint,
                defaultNormLog as ::core::ffi::c_uint,
                entropyWorkspace,
                entropyWorkspaceSize,
            ) as size_t;
            if ERR_isError(err_code_0) != 0 {
                return err_code_0;
            }
            return 0 as size_t;
        }
        2 => {
            let mut wksp: *mut ZSTD_BuildCTableWksp = entropyWorkspace as *mut ZSTD_BuildCTableWksp;
            let mut nbSeq_1: size_t = nbSeq;
            let tableLog: U32 = FSE_optimalTableLog(
                FSELog as ::core::ffi::c_uint,
                nbSeq,
                max as ::core::ffi::c_uint,
            ) as U32;
            if *count.offset(*codeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as isize)
                > 1 as ::core::ffi::c_uint
            {
                let ref mut fresh0 = *count
                    .offset(*codeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as isize);
                *fresh0 = (*fresh0).wrapping_sub(1);
                nbSeq_1 = nbSeq_1.wrapping_sub(1);
            }
            let err_code_1: size_t = FSE_normalizeCount(
                &raw mut (*wksp).norm as *mut ::core::ffi::c_short,
                tableLog as ::core::ffi::c_uint,
                count,
                nbSeq_1,
                max as ::core::ffi::c_uint,
                ZSTD_useLowProbCount(nbSeq_1),
            ) as size_t;
            if ERR_isError(err_code_1) != 0 {
                return err_code_1;
            }
            let NCountSize: size_t = FSE_writeNCount(
                op as *mut ::core::ffi::c_void,
                oend.offset_from(op) as ::core::ffi::c_long as size_t,
                &raw mut (*wksp).norm as *mut S16,
                max as ::core::ffi::c_uint,
                tableLog as ::core::ffi::c_uint,
            ) as size_t;
            let err_code_2: size_t = NCountSize;
            if ERR_isError(err_code_2) != 0 {
                return err_code_2;
            }
            let err_code_3: size_t = FSE_buildCTable_wksp(
                nextCTable,
                &raw mut (*wksp).norm as *mut S16,
                max as ::core::ffi::c_uint,
                tableLog as ::core::ffi::c_uint,
                &raw mut (*wksp).wksp as *mut U32 as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[U32; 285]>() as size_t,
            ) as size_t;
            if ERR_isError(err_code_3) != 0 {
                return err_code_3;
            }
            return NCountSize;
        }
        _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
    };
}
#[inline(always)]
unsafe extern "C" fn ZSTD_encodeSequences_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut CTable_MatchLength: *const FSE_CTable,
    mut mlCodeTable: *const BYTE,
    mut CTable_OffsetBits: *const FSE_CTable,
    mut ofCodeTable: *const BYTE,
    mut CTable_LitLength: *const FSE_CTable,
    mut llCodeTable: *const BYTE,
    mut sequences: *const SeqDef,
    mut nbSeq: size_t,
    mut longOffsets: ::core::ffi::c_int,
) -> size_t {
    let mut blockStream: BIT_CStream_t = BIT_CStream_t {
        bitContainer: 0,
        bitPos: 0,
        startPtr: ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ptr: ::core::ptr::null_mut::<::core::ffi::c_char>(),
        endPtr: ::core::ptr::null_mut::<::core::ffi::c_char>(),
    };
    let mut stateMatchLength: FSE_CState_t = FSE_CState_t {
        value: 0,
        stateTable: ::core::ptr::null::<::core::ffi::c_void>(),
        symbolTT: ::core::ptr::null::<::core::ffi::c_void>(),
        stateLog: 0,
    };
    let mut stateOffsetBits: FSE_CState_t = FSE_CState_t {
        value: 0,
        stateTable: ::core::ptr::null::<::core::ffi::c_void>(),
        symbolTT: ::core::ptr::null::<::core::ffi::c_void>(),
        stateLog: 0,
    };
    let mut stateLitLength: FSE_CState_t = FSE_CState_t {
        value: 0,
        stateTable: ::core::ptr::null::<::core::ffi::c_void>(),
        symbolTT: ::core::ptr::null::<::core::ffi::c_void>(),
        stateLog: 0,
    };
    if ERR_isError(BIT_initCStream(&raw mut blockStream, dst, dstCapacity)) != 0 {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    FSE_initCState2(
        &raw mut stateMatchLength,
        CTable_MatchLength,
        *mlCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as U32,
    );
    FSE_initCState2(
        &raw mut stateOffsetBits,
        CTable_OffsetBits,
        *ofCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as U32,
    );
    FSE_initCState2(
        &raw mut stateLitLength,
        CTable_LitLength,
        *llCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as U32,
    );
    BIT_addBits(
        &raw mut blockStream,
        (*sequences.offset(nbSeq.wrapping_sub(1 as size_t) as isize)).litLength as BitContainerType,
        LL_bits[*llCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as usize]
            as ::core::ffi::c_uint,
    );
    if MEM_32bits() != 0 {
        BIT_flushBits(&raw mut blockStream);
    }
    BIT_addBits(
        &raw mut blockStream,
        (*sequences.offset(nbSeq.wrapping_sub(1 as size_t) as isize)).mlBase as BitContainerType,
        ML_bits[*mlCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as usize]
            as ::core::ffi::c_uint,
    );
    if MEM_32bits() != 0 {
        BIT_flushBits(&raw mut blockStream);
    }
    if longOffsets != 0 {
        let ofBits: U32 = *ofCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as U32;
        let extraBits: ::core::ffi::c_uint = (ofBits as ::core::ffi::c_uint).wrapping_sub(
            (if ofBits
                < ((if MEM_32bits() != 0 {
                    25 as ::core::ffi::c_int
                } else {
                    57 as ::core::ffi::c_int
                }) as U32)
                    .wrapping_sub(1 as U32)
            {
                ofBits as ::core::ffi::c_uint
            } else {
                ((if MEM_32bits() != 0 {
                    25 as ::core::ffi::c_int
                } else {
                    57 as ::core::ffi::c_int
                }) as ::core::ffi::c_uint)
                    .wrapping_sub(1 as ::core::ffi::c_uint)
            }),
        );
        if extraBits != 0 {
            BIT_addBits(
                &raw mut blockStream,
                (*sequences.offset(nbSeq.wrapping_sub(1 as size_t) as isize)).offBase
                    as BitContainerType,
                extraBits,
            );
            BIT_flushBits(&raw mut blockStream);
        }
        BIT_addBits(
            &raw mut blockStream,
            ((*sequences.offset(nbSeq.wrapping_sub(1 as size_t) as isize)).offBase >> extraBits)
                as BitContainerType,
            (ofBits as ::core::ffi::c_uint).wrapping_sub(extraBits),
        );
    } else {
        BIT_addBits(
            &raw mut blockStream,
            (*sequences.offset(nbSeq.wrapping_sub(1 as size_t) as isize)).offBase
                as BitContainerType,
            *ofCodeTable.offset(nbSeq.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_uint,
        );
    }
    BIT_flushBits(&raw mut blockStream);
    let mut n: size_t = 0;
    n = nbSeq.wrapping_sub(2 as size_t);
    while n < nbSeq {
        let llCode: BYTE = *llCodeTable.offset(n as isize);
        let ofCode: BYTE = *ofCodeTable.offset(n as isize);
        let mlCode: BYTE = *mlCodeTable.offset(n as isize);
        let llBits: U32 = LL_bits[llCode as usize] as U32;
        let ofBits_0: U32 = ofCode as U32;
        let mlBits: U32 = ML_bits[mlCode as usize] as U32;
        FSE_encodeSymbol(
            &raw mut blockStream,
            &raw mut stateOffsetBits,
            ofCode as ::core::ffi::c_uint,
        );
        FSE_encodeSymbol(
            &raw mut blockStream,
            &raw mut stateMatchLength,
            mlCode as ::core::ffi::c_uint,
        );
        if MEM_32bits() != 0 {
            BIT_flushBits(&raw mut blockStream);
        }
        FSE_encodeSymbol(
            &raw mut blockStream,
            &raw mut stateLitLength,
            llCode as ::core::ffi::c_uint,
        );
        if MEM_32bits() != 0
            || ofBits_0.wrapping_add(mlBits).wrapping_add(llBits)
                >= (64 as ::core::ffi::c_int
                    - 7 as ::core::ffi::c_int
                    - (LLFSELog + MLFSELog + OffFSELog)) as U32
        {
            BIT_flushBits(&raw mut blockStream);
        }
        BIT_addBits(
            &raw mut blockStream,
            (*sequences.offset(n as isize)).litLength as BitContainerType,
            llBits as ::core::ffi::c_uint,
        );
        if MEM_32bits() != 0 && llBits.wrapping_add(mlBits) > 24 as U32 {
            BIT_flushBits(&raw mut blockStream);
        }
        BIT_addBits(
            &raw mut blockStream,
            (*sequences.offset(n as isize)).mlBase as BitContainerType,
            mlBits as ::core::ffi::c_uint,
        );
        if MEM_32bits() != 0 || ofBits_0.wrapping_add(mlBits).wrapping_add(llBits) > 56 as U32 {
            BIT_flushBits(&raw mut blockStream);
        }
        if longOffsets != 0 {
            let extraBits_0: ::core::ffi::c_uint = (ofBits_0 as ::core::ffi::c_uint).wrapping_sub(
                (if ofBits_0
                    < ((if MEM_32bits() != 0 {
                        25 as ::core::ffi::c_int
                    } else {
                        57 as ::core::ffi::c_int
                    }) as U32)
                        .wrapping_sub(1 as U32)
                {
                    ofBits_0 as ::core::ffi::c_uint
                } else {
                    ((if MEM_32bits() != 0 {
                        25 as ::core::ffi::c_int
                    } else {
                        57 as ::core::ffi::c_int
                    }) as ::core::ffi::c_uint)
                        .wrapping_sub(1 as ::core::ffi::c_uint)
                }),
            );
            if extraBits_0 != 0 {
                BIT_addBits(
                    &raw mut blockStream,
                    (*sequences.offset(n as isize)).offBase as BitContainerType,
                    extraBits_0,
                );
                BIT_flushBits(&raw mut blockStream);
            }
            BIT_addBits(
                &raw mut blockStream,
                ((*sequences.offset(n as isize)).offBase >> extraBits_0) as BitContainerType,
                (ofBits_0 as ::core::ffi::c_uint).wrapping_sub(extraBits_0),
            );
        } else {
            BIT_addBits(
                &raw mut blockStream,
                (*sequences.offset(n as isize)).offBase as BitContainerType,
                ofBits_0 as ::core::ffi::c_uint,
            );
        }
        BIT_flushBits(&raw mut blockStream);
        n = n.wrapping_sub(1);
    }
    FSE_flushCState(&raw mut blockStream, &raw mut stateMatchLength);
    FSE_flushCState(&raw mut blockStream, &raw mut stateOffsetBits);
    FSE_flushCState(&raw mut blockStream, &raw mut stateLitLength);
    let streamSize: size_t = BIT_closeCStream(&raw mut blockStream) as size_t;
    if streamSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    return streamSize;
}
unsafe extern "C" fn ZSTD_encodeSequences_default(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut CTable_MatchLength: *const FSE_CTable,
    mut mlCodeTable: *const BYTE,
    mut CTable_OffsetBits: *const FSE_CTable,
    mut ofCodeTable: *const BYTE,
    mut CTable_LitLength: *const FSE_CTable,
    mut llCodeTable: *const BYTE,
    mut sequences: *const SeqDef,
    mut nbSeq: size_t,
    mut longOffsets: ::core::ffi::c_int,
) -> size_t {
    return ZSTD_encodeSequences_body(
        dst,
        dstCapacity,
        CTable_MatchLength,
        mlCodeTable,
        CTable_OffsetBits,
        ofCodeTable,
        CTable_LitLength,
        llCodeTable,
        sequences,
        nbSeq,
        longOffsets,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_encodeSequences(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut CTable_MatchLength: *const FSE_CTable,
    mut mlCodeTable: *const BYTE,
    mut CTable_OffsetBits: *const FSE_CTable,
    mut ofCodeTable: *const BYTE,
    mut CTable_LitLength: *const FSE_CTable,
    mut llCodeTable: *const BYTE,
    mut sequences: *const SeqDef,
    mut nbSeq: size_t,
    mut longOffsets: ::core::ffi::c_int,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    return ZSTD_encodeSequences_default(
        dst,
        dstCapacity,
        CTable_MatchLength,
        mlCodeTable,
        CTable_OffsetBits,
        ofCodeTable,
        CTable_LitLength,
        llCodeTable,
        sequences,
        nbSeq,
        longOffsets,
    );
}
