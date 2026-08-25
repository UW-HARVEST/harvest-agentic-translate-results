use ::core::arch::asm;
#[cfg(target_arch = "x86")]
pub use ::core::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
#[cfg(target_arch = "x86_64")]
pub use ::core::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
pub type __m128i_u = __m128i;
use ::libc;
extern "C" {
    pub type ZSTD_DDict_s;
    fn FSE_readNCount(
        normalizedCounter: *mut ::core::ffi::c_short,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        tableLogPtr: *mut ::core::ffi::c_uint,
        rBuffer: *const ::core::ffi::c_void,
        rBuffSize: size_t,
    ) -> size_t;
    fn HUF_decompress1X_usingDTable(
        dst: *mut ::core::ffi::c_void,
        maxDstSize: size_t,
        cSrc: *const ::core::ffi::c_void,
        cSrcSize: size_t,
        DTable: *const HUF_DTable,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn HUF_decompress1X1_DCtx_wksp(
        dctx: *mut HUF_DTable,
        dst: *mut ::core::ffi::c_void,
        dstSize: size_t,
        cSrc: *const ::core::ffi::c_void,
        cSrcSize: size_t,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn HUF_decompress4X_usingDTable(
        dst: *mut ::core::ffi::c_void,
        maxDstSize: size_t,
        cSrc: *const ::core::ffi::c_void,
        cSrcSize: size_t,
        DTable: *const HUF_DTable,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn HUF_decompress4X_hufOnly_wksp(
        dctx: *mut HUF_DTable,
        dst: *mut ::core::ffi::c_void,
        dstSize: size_t,
        cSrc: *const ::core::ffi::c_void,
        cSrcSize: size_t,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        flags: ::core::ffi::c_int,
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
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct __loadu_si128 {
    pub __v: __m128i_u,
}
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct __storeu_si128 {
    pub __v: __m128i_u,
}
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
pub type HUF_DTable = U32;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const HUF_flags_disableFast: C2RustUnnamed_0 = 32;
pub const HUF_flags_disableAsm: C2RustUnnamed_0 = 16;
pub const HUF_flags_suspectUncompressible: C2RustUnnamed_0 = 8;
pub const HUF_flags_preferRepeat: C2RustUnnamed_0 = 4;
pub const HUF_flags_optimalDepth: C2RustUnnamed_0 = 2;
pub const HUF_flags_bmi2: C2RustUnnamed_0 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_DCtx_s {
    pub LLTptr: *const ZSTD_seqSymbol,
    pub MLTptr: *const ZSTD_seqSymbol,
    pub OFTptr: *const ZSTD_seqSymbol,
    pub HUFptr: *const HUF_DTable,
    pub entropy: ZSTD_entropyDTables_t,
    pub workspace: [U32; 640],
    pub previousDstEnd: *const ::core::ffi::c_void,
    pub prefixStart: *const ::core::ffi::c_void,
    pub virtualStart: *const ::core::ffi::c_void,
    pub dictEnd: *const ::core::ffi::c_void,
    pub expected: size_t,
    pub fParams: ZSTD_FrameHeader,
    pub processedCSize: U64,
    pub decodedSize: U64,
    pub bType: blockType_e,
    pub stage: ZSTD_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: size_t,
    pub format: ZSTD_format_e,
    pub forceIgnoreChecksum: ZSTD_forceIgnoreChecksum_e,
    pub validateChecksum: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTD_customMem,
    pub litSize: size_t,
    pub rleSize: size_t,
    pub staticSize: size_t,
    pub isFrameDecompression: ::core::ffi::c_int,
    pub ddictLocal: *mut ZSTD_DDict,
    pub ddict: *const ZSTD_DDict,
    pub dictID: U32,
    pub ddictIsCold: ::core::ffi::c_int,
    pub dictUses: ZSTD_dictUses_e,
    pub ddictSet: *mut ZSTD_DDictHashSet,
    pub refMultipleDDicts: ZSTD_refMultipleDDicts_e,
    pub disableHufAsm: ::core::ffi::c_int,
    pub maxBlockSizeParam: ::core::ffi::c_int,
    pub streamStage: ZSTD_dStreamStage,
    pub inBuff: *mut ::core::ffi::c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub maxWindowSize: size_t,
    pub outBuff: *mut ::core::ffi::c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub lhSize: size_t,
    pub legacyContext: *mut ::core::ffi::c_void,
    pub previousLegacyVersion: U32,
    pub legacyVersion: U32,
    pub hostageByte: U32,
    pub noForwardProgress: ::core::ffi::c_int,
    pub outBufferMode: ZSTD_bufferMode_e,
    pub expectedOutBuffer: ZSTD_outBuffer,
    pub litBuffer: *mut BYTE,
    pub litBufferEnd: *const BYTE,
    pub litBufferLocation: ZSTD_litLocation_e,
    pub litExtraBuffer: [BYTE; 65568],
    pub headerBuffer: [BYTE; 18],
    pub oversizedDuration: size_t,
    pub traceCtx: ZSTD_TraceCtx,
}
pub type ZSTD_TraceCtx = ::core::ffi::c_ulonglong;
pub type ZSTD_litLocation_e = ::core::ffi::c_uint;
pub const ZSTD_split: ZSTD_litLocation_e = 2;
pub const ZSTD_in_dst: ZSTD_litLocation_e = 1;
pub const ZSTD_not_in_dst: ZSTD_litLocation_e = 0;
pub type ZSTD_outBuffer = ZSTD_outBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_outBuffer_s {
    pub dst: *mut ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_bufferMode_e = ::core::ffi::c_uint;
pub const ZSTD_bm_stable: ZSTD_bufferMode_e = 1;
pub const ZSTD_bm_buffered: ZSTD_bufferMode_e = 0;
pub type ZSTD_dStreamStage = ::core::ffi::c_uint;
pub const zdss_flush: ZSTD_dStreamStage = 4;
pub const zdss_load: ZSTD_dStreamStage = 3;
pub const zdss_read: ZSTD_dStreamStage = 2;
pub const zdss_loadHeader: ZSTD_dStreamStage = 1;
pub const zdss_init: ZSTD_dStreamStage = 0;
pub type ZSTD_refMultipleDDicts_e = ::core::ffi::c_uint;
pub const ZSTD_rmd_refMultipleDDicts: ZSTD_refMultipleDDicts_e = 1;
pub const ZSTD_rmd_refSingleDDict: ZSTD_refMultipleDDicts_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_DDictHashSet {
    pub ddictPtrTable: *mut *const ZSTD_DDict,
    pub ddictPtrTableSize: size_t,
    pub ddictPtrCount: size_t,
}
pub type ZSTD_DDict = ZSTD_DDict_s;
pub type ZSTD_dictUses_e = ::core::ffi::c_int;
pub const ZSTD_use_once: ZSTD_dictUses_e = 1;
pub const ZSTD_dont_use: ZSTD_dictUses_e = 0;
pub const ZSTD_use_indefinitely: ZSTD_dictUses_e = -1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut ::core::ffi::c_void,
}
pub type ZSTD_freeFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>;
pub type ZSTD_allocFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, size_t) -> *mut ::core::ffi::c_void>;
pub type ZSTD_forceIgnoreChecksum_e = ::core::ffi::c_uint;
pub const ZSTD_d_ignoreChecksum: ZSTD_forceIgnoreChecksum_e = 1;
pub const ZSTD_d_validateChecksum: ZSTD_forceIgnoreChecksum_e = 0;
pub type ZSTD_format_e = ::core::ffi::c_uint;
pub const ZSTD_f_zstd1_magicless: ZSTD_format_e = 1;
pub const ZSTD_f_zstd1: ZSTD_format_e = 0;
pub type XXH64_state_t = XXH64_state_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct XXH64_state_s {
    pub total_len: XXH64_hash_t,
    pub v: [XXH64_hash_t; 4],
    pub mem64: [XXH64_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved32: XXH32_hash_t,
    pub reserved64: XXH64_hash_t,
}
pub type XXH64_hash_t = uint64_t;
pub type XXH32_hash_t = uint32_t;
pub type ZSTD_dStage = ::core::ffi::c_uint;
pub const ZSTDds_skipFrame: ZSTD_dStage = 7;
pub const ZSTDds_decodeSkippableHeader: ZSTD_dStage = 6;
pub const ZSTDds_checkChecksum: ZSTD_dStage = 5;
pub const ZSTDds_decompressLastBlock: ZSTD_dStage = 4;
pub const ZSTDds_decompressBlock: ZSTD_dStage = 3;
pub const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
pub const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
pub const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
pub type blockType_e = ::core::ffi::c_uint;
pub const bt_reserved: blockType_e = 3;
pub const bt_compressed: blockType_e = 2;
pub const bt_rle: blockType_e = 1;
pub const bt_raw: blockType_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: ::core::ffi::c_ulonglong,
    pub windowSize: ::core::ffi::c_ulonglong,
    pub blockSizeMax: ::core::ffi::c_uint,
    pub frameType: ZSTD_FrameType_e,
    pub headerSize: ::core::ffi::c_uint,
    pub dictID: ::core::ffi::c_uint,
    pub checksumFlag: ::core::ffi::c_uint,
    pub _reserved1: ::core::ffi::c_uint,
    pub _reserved2: ::core::ffi::c_uint,
}
pub type ZSTD_FrameType_e = ::core::ffi::c_uint;
pub const ZSTD_skippableFrame: ZSTD_FrameType_e = 1;
pub const ZSTD_frame: ZSTD_FrameType_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_entropyDTables_t {
    pub LLTable: [ZSTD_seqSymbol; 513],
    pub OFTable: [ZSTD_seqSymbol; 257],
    pub MLTable: [ZSTD_seqSymbol; 513],
    pub hufTable: [HUF_DTable; 4097],
    pub rep: [U32; 3],
    pub workspace: [U32; 157],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_seqSymbol {
    pub nextState: U16,
    pub nbAdditionalBits: BYTE,
    pub nbBits: BYTE,
    pub baseValue: U32,
}
pub type ZSTD_DCtx = ZSTD_DCtx_s;
pub type streaming_operation = ::core::ffi::c_uint;
pub const is_streaming: streaming_operation = 1;
pub const not_streaming: streaming_operation = 0;
pub type ZSTD_longOffset_e = ::core::ffi::c_uint;
pub const ZSTD_lo_isLongOffset: ZSTD_longOffset_e = 1;
pub const ZSTD_lo_isRegularOffset: ZSTD_longOffset_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seqState_t {
    pub DStream: BIT_DStream_t,
    pub stateLL: ZSTD_fseState,
    pub stateOffb: ZSTD_fseState,
    pub stateML: ZSTD_fseState,
    pub prevOffset: [size_t; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_fseState {
    pub state: size_t,
    pub table: *const ZSTD_seqSymbol,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seq_t {
    pub litLength: size_t,
    pub matchLength: size_t,
    pub offset: size_t,
}
pub type ZSTD_overlap_e = ::core::ffi::c_uint;
pub const ZSTD_overlap_src_before_dst: ZSTD_overlap_e = 1;
pub const ZSTD_no_overlap: ZSTD_overlap_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_seqSymbol_header {
    pub fastMode: U32,
    pub tableLog: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_OffsetInfo {
    pub longOffsetShare: ::core::ffi::c_uint,
    pub maxNbAdditionalBits: ::core::ffi::c_uint,
}
pub type SymbolEncodingType_e = ::core::ffi::c_uint;
pub const set_repeat: SymbolEncodingType_e = 3;
pub const set_compressed: SymbolEncodingType_e = 2;
pub const set_rle: SymbolEncodingType_e = 1;
pub const set_basic: SymbolEncodingType_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct blockProperties_t {
    pub blockType: blockType_e,
    pub lastBlock: U32,
    pub origSize: U32,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const CACHELINE_SIZE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTD_wrappedPtrAdd(
    mut ptr: *const ::core::ffi::c_uchar,
    mut add: ptrdiff_t,
) -> *const ::core::ffi::c_uchar {
    return ptr.offset(add as isize);
}
#[inline]
unsafe extern "C" fn ZSTD_wrappedPtrSub(
    mut ptr: *const ::core::ffi::c_uchar,
    mut sub: ptrdiff_t,
) -> *const ::core::ffi::c_uchar {
    return ptr.offset(-(sub as isize));
}
#[inline]
unsafe extern "C" fn ZSTD_maybeNullPtrAdd(
    mut ptr: *mut ::core::ffi::c_uchar,
    mut add: ptrdiff_t,
) -> *mut ::core::ffi::c_uchar {
    return if add > 0 as ptrdiff_t {
        ptr.offset(add as isize)
    } else {
        ptr
    };
}
#[inline]
unsafe extern "C" fn MEM_32bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<size_t>() as usize == 4 as usize) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_64bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<size_t>() as usize == 8 as usize) as ::core::ffi::c_int
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
unsafe extern "C" fn MEM_readLE16(mut memPtr: *const ::core::ffi::c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read16(memPtr);
    } else {
        let mut p: *const BYTE = memPtr as *const BYTE;
        return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            + ((*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int)) as U16;
    };
}
#[inline]
unsafe extern "C" fn MEM_readLE24(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    return (MEM_readLE16(memPtr) as U32).wrapping_add(
        (*(memPtr as *const BYTE).offset(2 as ::core::ffi::c_int as isize) as U32)
            << 16 as ::core::ffi::c_int,
    );
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
pub const STREAM_ACCUMULATOR_MIN_32: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const STREAM_ACCUMULATOR_MIN_64: ::core::ffi::c_int = 57 as ::core::ffi::c_int;
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
                current_block_32 = 6104373638092305747;
            }
            6 => {
                current_block_32 = 6104373638092305747;
            }
            5 => {
                current_block_32 = 16350303764663281140;
            }
            4 => {
                current_block_32 = 6149137102233524447;
            }
            3 => {
                current_block_32 = 4946879549817297424;
            }
            2 => {
                current_block_32 = 3603544538426133943;
            }
            _ => {
                current_block_32 = 16203760046146113240;
            }
        }
        match current_block_32 {
            6104373638092305747 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(5 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(24 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 16350303764663281140;
            }
            _ => {}
        }
        match current_block_32 {
            16350303764663281140 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(4 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(32 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 6149137102233524447;
            }
            _ => {}
        }
        match current_block_32 {
            6149137102233524447 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(3 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 4946879549817297424;
            }
            _ => {}
        }
        match current_block_32 {
            4946879549817297424 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 3603544538426133943;
            }
            _ => {}
        }
        match current_block_32 {
            3603544538426133943 => {
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
unsafe extern "C" fn BIT_endOfDStream(mut DStream: *const BIT_DStream_t) -> ::core::ffi::c_uint {
    return ((*DStream).ptr == (*DStream).start
        && (*DStream).bitsConsumed as usize
            == (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize))
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
pub const ZSTD_BLOCKSIZELOG_MAX: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const ZSTD_BLOCKSIZE_MAX: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << ZSTD_BLOCKSIZELOG_MAX;
pub const ZSTD_WINDOWLOG_MAX_32: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const ZSTD_REP_NUM: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ZSTD_BLOCKHEADERSIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut ZSTD_blockHeaderSize: size_t = ZSTD_BLOCKHEADERSIZE as size_t;
pub const LONGNBSEQ: ::core::ffi::c_int = 0x7f00 as ::core::ffi::c_int;
pub const MaxML: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const MaxLL: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const MaxOff: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
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
pub const LL_DEFAULTNORMLOG: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
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
pub const ML_DEFAULTNORMLOG: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const OF_DEFAULTNORMLOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_copy8(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    ::libc::memcpy(
        dst,
        src,
        8 as ::core::ffi::c_int as ::core::ffi::c_ulong as ::libc::size_t,
    );
}
unsafe extern "C" fn ZSTD_copy16(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    _mm_storeu_si128(
        dst as *mut __m128i_u,
        _mm_loadu_si128(src as *const __m128i_u),
    );
}
pub const WILDCOPY_OVERLENGTH: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const WILDCOPY_VECLEN: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
#[inline(always)]
unsafe extern "C" fn ZSTD_wildcopy(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
    mut length: ptrdiff_t,
    ovtype: ZSTD_overlap_e,
) {
    let mut diff: ptrdiff_t = (dst as *mut BYTE).offset_from(src as *const BYTE) as ptrdiff_t;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.offset(length as isize);
    if ovtype as ::core::ffi::c_uint
        == ZSTD_overlap_src_before_dst as ::core::ffi::c_int as ::core::ffi::c_uint
        && diff < WILDCOPY_VECLEN as ptrdiff_t
    {
        loop {
            ZSTD_copy8(
                op as *mut ::core::ffi::c_void,
                ip as *const ::core::ffi::c_void,
            );
            op = op.offset(8 as ::core::ffi::c_int as isize);
            ip = ip.offset(8 as ::core::ffi::c_int as isize);
            if !(op < oend) {
                break;
            }
        }
    } else {
        ZSTD_copy16(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
        );
        if 16 as ptrdiff_t >= length {
            return;
        }
        op = op.offset(16 as ::core::ffi::c_int as isize);
        ip = ip.offset(16 as ::core::ffi::c_int as isize);
        loop {
            ZSTD_copy16(
                op as *mut ::core::ffi::c_void,
                ip as *const ::core::ffi::c_void,
            );
            op = op.offset(16 as ::core::ffi::c_int as isize);
            ip = ip.offset(16 as ::core::ffi::c_int as isize);
            ZSTD_copy16(
                op as *mut ::core::ffi::c_void,
                ip as *const ::core::ffi::c_void,
            );
            op = op.offset(16 as ::core::ffi::c_int as isize);
            ip = ip.offset(16 as ::core::ffi::c_int as isize);
            if !(op < oend) {
                break;
            }
        }
    };
}
static mut LL_base: [U32; 36] = [
    0 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    2 as ::core::ffi::c_int as U32,
    3 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
    6 as ::core::ffi::c_int as U32,
    7 as ::core::ffi::c_int as U32,
    8 as ::core::ffi::c_int as U32,
    9 as ::core::ffi::c_int as U32,
    10 as ::core::ffi::c_int as U32,
    11 as ::core::ffi::c_int as U32,
    12 as ::core::ffi::c_int as U32,
    13 as ::core::ffi::c_int as U32,
    14 as ::core::ffi::c_int as U32,
    15 as ::core::ffi::c_int as U32,
    16 as ::core::ffi::c_int as U32,
    18 as ::core::ffi::c_int as U32,
    20 as ::core::ffi::c_int as U32,
    22 as ::core::ffi::c_int as U32,
    24 as ::core::ffi::c_int as U32,
    28 as ::core::ffi::c_int as U32,
    32 as ::core::ffi::c_int as U32,
    40 as ::core::ffi::c_int as U32,
    48 as ::core::ffi::c_int as U32,
    64 as ::core::ffi::c_int as U32,
    0x80 as ::core::ffi::c_int as U32,
    0x100 as ::core::ffi::c_int as U32,
    0x200 as ::core::ffi::c_int as U32,
    0x400 as ::core::ffi::c_int as U32,
    0x800 as ::core::ffi::c_int as U32,
    0x1000 as ::core::ffi::c_int as U32,
    0x2000 as ::core::ffi::c_int as U32,
    0x4000 as ::core::ffi::c_int as U32,
    0x8000 as ::core::ffi::c_int as U32,
    0x10000 as ::core::ffi::c_int as U32,
];
static mut OF_base: [U32; 32] = [
    0 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
    0xd as ::core::ffi::c_int as U32,
    0x1d as ::core::ffi::c_int as U32,
    0x3d as ::core::ffi::c_int as U32,
    0x7d as ::core::ffi::c_int as U32,
    0xfd as ::core::ffi::c_int as U32,
    0x1fd as ::core::ffi::c_int as U32,
    0x3fd as ::core::ffi::c_int as U32,
    0x7fd as ::core::ffi::c_int as U32,
    0xffd as ::core::ffi::c_int as U32,
    0x1ffd as ::core::ffi::c_int as U32,
    0x3ffd as ::core::ffi::c_int as U32,
    0x7ffd as ::core::ffi::c_int as U32,
    0xfffd as ::core::ffi::c_int as U32,
    0x1fffd as ::core::ffi::c_int as U32,
    0x3fffd as ::core::ffi::c_int as U32,
    0x7fffd as ::core::ffi::c_int as U32,
    0xffffd as ::core::ffi::c_int as U32,
    0x1ffffd as ::core::ffi::c_int as U32,
    0x3ffffd as ::core::ffi::c_int as U32,
    0x7ffffd as ::core::ffi::c_int as U32,
    0xfffffd as ::core::ffi::c_int as U32,
    0x1fffffd as ::core::ffi::c_int as U32,
    0x3fffffd as ::core::ffi::c_int as U32,
    0x7fffffd as ::core::ffi::c_int as U32,
    0xffffffd as ::core::ffi::c_int as U32,
    0x1ffffffd as ::core::ffi::c_int as U32,
    0x3ffffffd as ::core::ffi::c_int as U32,
    0x7ffffffd as ::core::ffi::c_int as U32,
];
static mut OF_bits: [U8; 32] = [
    0 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    5 as ::core::ffi::c_int as U8,
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
    17 as ::core::ffi::c_int as U8,
    18 as ::core::ffi::c_int as U8,
    19 as ::core::ffi::c_int as U8,
    20 as ::core::ffi::c_int as U8,
    21 as ::core::ffi::c_int as U8,
    22 as ::core::ffi::c_int as U8,
    23 as ::core::ffi::c_int as U8,
    24 as ::core::ffi::c_int as U8,
    25 as ::core::ffi::c_int as U8,
    26 as ::core::ffi::c_int as U8,
    27 as ::core::ffi::c_int as U8,
    28 as ::core::ffi::c_int as U8,
    29 as ::core::ffi::c_int as U8,
    30 as ::core::ffi::c_int as U8,
    31 as ::core::ffi::c_int as U8,
];
static mut ML_base: [U32; 53] = [
    3 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
    6 as ::core::ffi::c_int as U32,
    7 as ::core::ffi::c_int as U32,
    8 as ::core::ffi::c_int as U32,
    9 as ::core::ffi::c_int as U32,
    10 as ::core::ffi::c_int as U32,
    11 as ::core::ffi::c_int as U32,
    12 as ::core::ffi::c_int as U32,
    13 as ::core::ffi::c_int as U32,
    14 as ::core::ffi::c_int as U32,
    15 as ::core::ffi::c_int as U32,
    16 as ::core::ffi::c_int as U32,
    17 as ::core::ffi::c_int as U32,
    18 as ::core::ffi::c_int as U32,
    19 as ::core::ffi::c_int as U32,
    20 as ::core::ffi::c_int as U32,
    21 as ::core::ffi::c_int as U32,
    22 as ::core::ffi::c_int as U32,
    23 as ::core::ffi::c_int as U32,
    24 as ::core::ffi::c_int as U32,
    25 as ::core::ffi::c_int as U32,
    26 as ::core::ffi::c_int as U32,
    27 as ::core::ffi::c_int as U32,
    28 as ::core::ffi::c_int as U32,
    29 as ::core::ffi::c_int as U32,
    30 as ::core::ffi::c_int as U32,
    31 as ::core::ffi::c_int as U32,
    32 as ::core::ffi::c_int as U32,
    33 as ::core::ffi::c_int as U32,
    34 as ::core::ffi::c_int as U32,
    35 as ::core::ffi::c_int as U32,
    37 as ::core::ffi::c_int as U32,
    39 as ::core::ffi::c_int as U32,
    41 as ::core::ffi::c_int as U32,
    43 as ::core::ffi::c_int as U32,
    47 as ::core::ffi::c_int as U32,
    51 as ::core::ffi::c_int as U32,
    59 as ::core::ffi::c_int as U32,
    67 as ::core::ffi::c_int as U32,
    83 as ::core::ffi::c_int as U32,
    99 as ::core::ffi::c_int as U32,
    0x83 as ::core::ffi::c_int as U32,
    0x103 as ::core::ffi::c_int as U32,
    0x203 as ::core::ffi::c_int as U32,
    0x403 as ::core::ffi::c_int as U32,
    0x803 as ::core::ffi::c_int as U32,
    0x1003 as ::core::ffi::c_int as U32,
    0x2003 as ::core::ffi::c_int as U32,
    0x4003 as ::core::ffi::c_int as U32,
    0x8003 as ::core::ffi::c_int as U32,
    0x10003 as ::core::ffi::c_int as U32,
];
#[inline]
unsafe extern "C" fn ZSTD_DCtx_get_bmi2(mut dctx: *const ZSTD_DCtx_s) -> ::core::ffi::c_int {
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTD_copy4(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    ::libc::memcpy(
        dst,
        src,
        4 as ::core::ffi::c_int as ::core::ffi::c_ulong as ::libc::size_t,
    );
}
unsafe extern "C" fn ZSTD_blockSizeMax(mut dctx: *const ZSTD_DCtx) -> size_t {
    let blockSizeMax: size_t = (if (*dctx).isFrameDecompression != 0 {
        (*dctx).fParams.blockSizeMax
    } else {
        ZSTD_BLOCKSIZE_MAX as ::core::ffi::c_uint
    }) as size_t;
    return blockSizeMax;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getcBlockSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut bpPtr: *mut blockProperties_t,
) -> size_t {
    if srcSize < ZSTD_blockHeaderSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let cBlockHeader: U32 = MEM_readLE24(src) as U32;
    let cSize: U32 = cBlockHeader >> 3 as ::core::ffi::c_int;
    (*bpPtr).lastBlock = cBlockHeader & 1 as U32;
    (*bpPtr).blockType = (cBlockHeader >> 1 as ::core::ffi::c_int & 3 as U32) as blockType_e;
    (*bpPtr).origSize = cSize;
    if (*bpPtr).blockType as ::core::ffi::c_uint
        == bt_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as size_t;
    }
    if (*bpPtr).blockType as ::core::ffi::c_uint
        == bt_reserved as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return cSize as size_t;
}
unsafe extern "C" fn ZSTD_allocateLiteralsBuffer(
    mut dctx: *mut ZSTD_DCtx,
    dst: *mut ::core::ffi::c_void,
    dstCapacity: size_t,
    litSize: size_t,
    streaming: streaming_operation,
    expectedWriteSize: size_t,
    splitImmediately: ::core::ffi::c_uint,
) {
    let blockSizeMax: size_t = ZSTD_blockSizeMax(dctx) as size_t;
    if streaming as ::core::ffi::c_uint
        == not_streaming as ::core::ffi::c_int as ::core::ffi::c_uint
        && dstCapacity
            > blockSizeMax
                .wrapping_add(WILDCOPY_OVERLENGTH as size_t)
                .wrapping_add(litSize)
                .wrapping_add(WILDCOPY_OVERLENGTH as size_t)
    {
        (*dctx).litBuffer = (dst as *mut BYTE)
            .offset(blockSizeMax as isize)
            .offset(WILDCOPY_OVERLENGTH as isize);
        (*dctx).litBufferEnd = (*dctx).litBuffer.offset(litSize as isize);
        (*dctx).litBufferLocation = ZSTD_in_dst;
    } else if litSize
        <= (if 64 as ::core::ffi::c_int
            > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
            {
                (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
            } else {
                (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
            }) {
            64 as ::core::ffi::c_int
        } else {
            (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
            {
                (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
            } else {
                (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
            })
        }) as size_t
    {
        (*dctx).litBuffer = &raw mut (*dctx).litExtraBuffer as *mut BYTE;
        (*dctx).litBufferEnd = (*dctx).litBuffer.offset(litSize as isize);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    } else {
        if splitImmediately != 0 {
            (*dctx).litBuffer = (dst as *mut BYTE)
                .offset(expectedWriteSize as isize)
                .offset(-(litSize as isize))
                .offset(
                    (if 64 as ::core::ffi::c_int
                        > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    {
                        64 as ::core::ffi::c_int
                    } else {
                        (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    }) as isize,
                )
                .offset(-(WILDCOPY_OVERLENGTH as isize));
            (*dctx).litBufferEnd = (*dctx).litBuffer.offset(litSize as isize).offset(
                -((if 64 as ::core::ffi::c_int
                    > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                {
                    64 as ::core::ffi::c_int
                } else {
                    (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                }) as isize),
            );
        } else {
            (*dctx).litBuffer = (dst as *mut BYTE)
                .offset(expectedWriteSize as isize)
                .offset(-(litSize as isize));
            (*dctx).litBufferEnd = (dst as *mut BYTE).offset(expectedWriteSize as isize);
        }
        (*dctx).litBufferLocation = ZSTD_split;
    };
}
unsafe extern "C" fn ZSTD_decodeLiteralsBlock(
    mut dctx: *mut ZSTD_DCtx,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    streaming: streaming_operation,
) -> size_t {
    if srcSize < (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = src as *const BYTE;
    let litEncType: SymbolEncodingType_e =
        (*istart.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 3 as ::core::ffi::c_int) as SymbolEncodingType_e;
    let blockSizeMax: size_t = ZSTD_blockSizeMax(dctx) as size_t;
    match litEncType as ::core::ffi::c_uint {
        3 => {
            if (*dctx).litEntropy == 0 as U32 {
                return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
            }
        }
        2 => {}
        0 => {
            let mut litSize_0: size_t = 0;
            let mut lhSize_0: size_t = 0;
            let lhlCode_0: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                >> 2 as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int) as U32;
            let mut expectedWriteSize_0: size_t = if blockSizeMax < dstCapacity {
                blockSizeMax
            } else {
                dstCapacity
            };
            match lhlCode_0 {
                1 => {
                    lhSize_0 = 2 as size_t;
                    litSize_0 = (MEM_readLE16(istart as *const ::core::ffi::c_void)
                        as ::core::ffi::c_int
                        >> 4 as ::core::ffi::c_int) as size_t;
                }
                3 => {
                    lhSize_0 = 3 as size_t;
                    if srcSize < 3 as size_t {
                        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                    }
                    litSize_0 = (MEM_readLE24(istart as *const ::core::ffi::c_void)
                        >> 4 as ::core::ffi::c_int) as size_t;
                }
                0 | 2 | _ => {
                    lhSize_0 = 1 as size_t;
                    litSize_0 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        >> 3 as ::core::ffi::c_int) as size_t;
                }
            }
            if litSize_0 > 0 as size_t && dst.is_null() {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            if litSize_0 > blockSizeMax {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if expectedWriteSize_0 < litSize_0 {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            ZSTD_allocateLiteralsBuffer(
                dctx,
                dst,
                dstCapacity,
                litSize_0,
                streaming,
                expectedWriteSize_0,
                1 as ::core::ffi::c_uint,
            );
            if lhSize_0
                .wrapping_add(litSize_0)
                .wrapping_add(WILDCOPY_OVERLENGTH as size_t)
                > srcSize
            {
                if litSize_0.wrapping_add(lhSize_0) > srcSize {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
                if (*dctx).litBufferLocation as ::core::ffi::c_uint
                    == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    ::libc::memcpy(
                        (*dctx).litBuffer as *mut ::core::ffi::c_void,
                        istart.offset(lhSize_0 as isize) as *const ::core::ffi::c_void,
                        litSize_0.wrapping_sub(
                            (if 64 as ::core::ffi::c_int
                                > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                {
                                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                                } else {
                                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                })
                            {
                                64 as ::core::ffi::c_int
                            } else {
                                (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                {
                                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                                } else {
                                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                })
                            }) as size_t,
                        ) as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        &raw mut (*dctx).litExtraBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                        istart
                            .offset(lhSize_0 as isize)
                            .offset(litSize_0 as isize)
                            .offset(
                                -((if 64 as ::core::ffi::c_int
                                    > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                    {
                                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                                    } else {
                                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                    })
                                {
                                    64 as ::core::ffi::c_int
                                } else {
                                    (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                    {
                                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                                    } else {
                                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                                    })
                                }) as isize),
                            ) as *const ::core::ffi::c_void,
                        (if 64 as ::core::ffi::c_int
                            > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                            {
                                (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                            } else {
                                (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                            })
                        {
                            64 as ::core::ffi::c_int
                        } else if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        }) as ::core::ffi::c_ulong as ::libc::size_t,
                    );
                } else {
                    ::libc::memcpy(
                        (*dctx).litBuffer as *mut ::core::ffi::c_void,
                        istart.offset(lhSize_0 as isize) as *const ::core::ffi::c_void,
                        litSize_0 as ::libc::size_t,
                    );
                }
                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = litSize_0;
                return lhSize_0.wrapping_add(litSize_0);
            }
            (*dctx).litPtr = istart.offset(lhSize_0 as isize);
            (*dctx).litSize = litSize_0;
            (*dctx).litBufferEnd = (*dctx).litPtr.offset(litSize_0 as isize);
            (*dctx).litBufferLocation = ZSTD_not_in_dst;
            return lhSize_0.wrapping_add(litSize_0);
        }
        1 => {
            let lhlCode_1: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                >> 2 as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int) as U32;
            let mut litSize_1: size_t = 0;
            let mut lhSize_1: size_t = 0;
            let mut expectedWriteSize_1: size_t = if blockSizeMax < dstCapacity {
                blockSizeMax
            } else {
                dstCapacity
            };
            match lhlCode_1 {
                1 => {
                    lhSize_1 = 2 as size_t;
                    if srcSize < 3 as size_t {
                        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                    }
                    litSize_1 = (MEM_readLE16(istart as *const ::core::ffi::c_void)
                        as ::core::ffi::c_int
                        >> 4 as ::core::ffi::c_int) as size_t;
                }
                3 => {
                    lhSize_1 = 3 as size_t;
                    if srcSize < 4 as size_t {
                        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                    }
                    litSize_1 = (MEM_readLE24(istart as *const ::core::ffi::c_void)
                        >> 4 as ::core::ffi::c_int) as size_t;
                }
                0 | 2 | _ => {
                    lhSize_1 = 1 as size_t;
                    litSize_1 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        >> 3 as ::core::ffi::c_int) as size_t;
                }
            }
            if litSize_1 > 0 as size_t && dst.is_null() {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            if litSize_1 > blockSizeMax {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if expectedWriteSize_1 < litSize_1 {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            ZSTD_allocateLiteralsBuffer(
                dctx,
                dst,
                dstCapacity,
                litSize_1,
                streaming,
                expectedWriteSize_1,
                1 as ::core::ffi::c_uint,
            );
            if (*dctx).litBufferLocation as ::core::ffi::c_uint
                == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                ::libc::memset(
                    (*dctx).litBuffer as *mut ::core::ffi::c_void,
                    *istart.offset(lhSize_1 as isize) as ::core::ffi::c_int,
                    litSize_1.wrapping_sub(
                        (if 64 as ::core::ffi::c_int
                            > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                            {
                                (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                            } else {
                                (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                            })
                        {
                            64 as ::core::ffi::c_int
                        } else {
                            (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                                < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                            {
                                (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                            } else {
                                (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                            })
                        }) as size_t,
                    ) as ::libc::size_t,
                );
                ::libc::memset(
                    &raw mut (*dctx).litExtraBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                    *istart.offset(lhSize_1 as isize) as ::core::ffi::c_int,
                    (if 64 as ::core::ffi::c_int
                        > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    {
                        64 as ::core::ffi::c_int
                    } else if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    }) as ::core::ffi::c_ulong as ::libc::size_t,
                );
            } else {
                ::libc::memset(
                    (*dctx).litBuffer as *mut ::core::ffi::c_void,
                    *istart.offset(lhSize_1 as isize) as ::core::ffi::c_int,
                    litSize_1 as ::libc::size_t,
                );
            }
            (*dctx).litPtr = (*dctx).litBuffer;
            (*dctx).litSize = litSize_1;
            return lhSize_1.wrapping_add(1 as size_t);
        }
        _ => return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t,
    }
    if srcSize < 5 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let mut lhSize: size_t = 0;
    let mut litSize: size_t = 0;
    let mut litCSize: size_t = 0;
    let mut singleStream: U32 = 0 as U32;
    let lhlCode: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        >> 2 as ::core::ffi::c_int
        & 3 as ::core::ffi::c_int) as U32;
    let lhc: U32 = MEM_readLE32(istart as *const ::core::ffi::c_void) as U32;
    let mut hufSuccess: size_t = 0;
    let mut expectedWriteSize: size_t = if blockSizeMax < dstCapacity {
        blockSizeMax
    } else {
        dstCapacity
    };
    let flags: ::core::ffi::c_int = 0 as ::core::ffi::c_int
        | (if ZSTD_DCtx_get_bmi2(dctx) != 0 {
            HUF_flags_bmi2 as ::core::ffi::c_int as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        })
        | (if (*dctx).disableHufAsm != 0 {
            HUF_flags_disableAsm as ::core::ffi::c_int as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        });
    match lhlCode {
        2 => {
            lhSize = 4 as size_t;
            litSize = (lhc >> 4 as ::core::ffi::c_int & 0x3fff as U32) as size_t;
            litCSize = (lhc >> 18 as ::core::ffi::c_int) as size_t;
        }
        3 => {
            lhSize = 5 as size_t;
            litSize = (lhc >> 4 as ::core::ffi::c_int & 0x3ffff as U32) as size_t;
            litCSize = ((lhc >> 22 as ::core::ffi::c_int) as size_t).wrapping_add(
                (*istart.offset(4 as ::core::ffi::c_int as isize) as size_t)
                    << 10 as ::core::ffi::c_int,
            );
        }
        0 | 1 | _ => {
            singleStream = (lhlCode == 0) as ::core::ffi::c_int as U32;
            lhSize = 3 as size_t;
            litSize = (lhc >> 4 as ::core::ffi::c_int & 0x3ff as U32) as size_t;
            litCSize = (lhc >> 14 as ::core::ffi::c_int & 0x3ff as U32) as size_t;
        }
    }
    if litSize > 0 as size_t && dst.is_null() {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if litSize > blockSizeMax {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if singleStream == 0 {
        if litSize < 6 as size_t {
            return -(ZSTD_error_literals_headerWrong as ::core::ffi::c_int) as size_t;
        }
    }
    if litCSize.wrapping_add(lhSize) > srcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if expectedWriteSize < litSize {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    ZSTD_allocateLiteralsBuffer(
        dctx,
        dst,
        dstCapacity,
        litSize,
        streaming,
        expectedWriteSize,
        0 as ::core::ffi::c_uint,
    );
    if (*dctx).ddictIsCold != 0 && litSize > 768 as size_t {
        let _ptr: *const ::core::ffi::c_char = (*dctx).HUFptr as *const ::core::ffi::c_char;
        let _size: size_t = ::core::mem::size_of::<[HUF_DTable; 4097]>();
        let mut _pos: size_t = 0;
        _pos = 0 as size_t;
        while _pos < _size {
            _pos = (_pos as ::core::ffi::c_ulong)
                .wrapping_add(CACHELINE_SIZE as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    }
    if litEncType as ::core::ffi::c_uint == set_repeat as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if singleStream != 0 {
            hufSuccess = HUF_decompress1X_usingDTable(
                (*dctx).litBuffer as *mut ::core::ffi::c_void,
                litSize,
                istart.offset(lhSize as isize) as *const ::core::ffi::c_void,
                litCSize,
                (*dctx).HUFptr,
                flags,
            );
        } else {
            hufSuccess = HUF_decompress4X_usingDTable(
                (*dctx).litBuffer as *mut ::core::ffi::c_void,
                litSize,
                istart.offset(lhSize as isize) as *const ::core::ffi::c_void,
                litCSize,
                (*dctx).HUFptr,
                flags,
            );
        }
    } else if singleStream != 0 {
        hufSuccess = HUF_decompress1X1_DCtx_wksp(
            &raw mut (*dctx).entropy.hufTable as *mut HUF_DTable,
            (*dctx).litBuffer as *mut ::core::ffi::c_void,
            litSize,
            istart.offset(lhSize as isize) as *const ::core::ffi::c_void,
            litCSize,
            &raw mut (*dctx).workspace as *mut U32 as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[U32; 640]>() as size_t,
            flags,
        );
    } else {
        hufSuccess = HUF_decompress4X_hufOnly_wksp(
            &raw mut (*dctx).entropy.hufTable as *mut HUF_DTable,
            (*dctx).litBuffer as *mut ::core::ffi::c_void,
            litSize,
            istart.offset(lhSize as isize) as *const ::core::ffi::c_void,
            litCSize,
            &raw mut (*dctx).workspace as *mut U32 as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[U32; 640]>() as size_t,
            flags,
        );
    }
    if (*dctx).litBufferLocation as ::core::ffi::c_uint
        == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ::libc::memcpy(
            &raw mut (*dctx).litExtraBuffer as *mut BYTE as *mut ::core::ffi::c_void,
            (*dctx).litBufferEnd.offset(
                -((if 64 as ::core::ffi::c_int
                    > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                {
                    64 as ::core::ffi::c_int
                } else {
                    (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                }) as isize),
            ) as *const ::core::ffi::c_void,
            (if 64 as ::core::ffi::c_int
                > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            {
                64 as ::core::ffi::c_int
            } else if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
            {
                (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
            } else {
                (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
            }) as ::core::ffi::c_ulong as ::libc::size_t,
        );
        ::libc::memmove(
            (*dctx)
                .litBuffer
                .offset(
                    (if 64 as ::core::ffi::c_int
                        > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    {
                        64 as ::core::ffi::c_int
                    } else {
                        (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    }) as isize,
                )
                .offset(-(32 as ::core::ffi::c_int as isize))
                as *mut ::core::ffi::c_void,
            (*dctx).litBuffer as *const ::core::ffi::c_void,
            litSize.wrapping_sub(
                (if 64 as ::core::ffi::c_int
                    > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                {
                    64 as ::core::ffi::c_int
                } else {
                    (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                }) as size_t,
            ) as ::libc::size_t,
        );
        (*dctx).litBuffer = (*dctx).litBuffer.offset(
            ((if 64 as ::core::ffi::c_int
                > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            {
                64 as ::core::ffi::c_int
            } else {
                (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            }) - WILDCOPY_OVERLENGTH) as isize,
        );
        (*dctx).litBufferEnd = (*dctx).litBufferEnd.offset(-(WILDCOPY_OVERLENGTH as isize));
    }
    if ERR_isError(hufSuccess) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    (*dctx).litPtr = (*dctx).litBuffer;
    (*dctx).litSize = litSize;
    (*dctx).litEntropy = 1 as U32;
    if litEncType as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (*dctx).HUFptr = &raw mut (*dctx).entropy.hufTable as *mut HUF_DTable;
    }
    return litCSize.wrapping_add(lhSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeLiteralsBlock_wrapper(
    mut dctx: *mut ZSTD_DCtx,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
) -> size_t {
    (*dctx).isFrameDecompression = 0 as ::core::ffi::c_int;
    return ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, not_streaming);
}
static mut LL_defaultDTable: [ZSTD_seqSymbol; 65] = [
    ZSTD_seqSymbol {
        nextState: 1 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 1 as BYTE,
        baseValue: LL_DEFAULTNORMLOG as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 0 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 0 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 1 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 3 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 4 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 6 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 7 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 9 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 10 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 12 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 14 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 16 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 20 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 22 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 28 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 32 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 4 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 48 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 6 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 64 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 7 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 128 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 8 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 256 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 10 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 1024 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 12 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 4096 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 0 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 1 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 2 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 4 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 5 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 7 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 8 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 10 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 11 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 13 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 16 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 18 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 22 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 24 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 32 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 40 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 6 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 64 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 6 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 64 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 7 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 128 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 9 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 512 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 11 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 2048 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 48 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 0 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 1 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 2 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 3 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 5 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 6 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 8 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 9 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 11 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 12 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 15 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 18 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 20 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 24 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 28 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 40 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 4 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 48 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 16 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 65536 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 15 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 32768 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 14 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 16384 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 13 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 8192 as U32,
    },
];
static mut OF_defaultDTable: [ZSTD_seqSymbol; 33] = [
    ZSTD_seqSymbol {
        nextState: 1 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 1 as BYTE,
        baseValue: OF_DEFAULTNORMLOG as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 0 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 6 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 61 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 9 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 509 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 15 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 32765 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 21 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 2097149 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 5 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 7 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 125 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 12 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 4093 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 18 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 262141 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 23 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 8388605 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 5 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 29 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 8 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 253 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 14 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 16381 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 20 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 1048573 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 1 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 7 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 125 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 11 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 2045 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 17 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 131069 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 22 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 4194301 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 4 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 13 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 8 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 253 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 13 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 8189 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 19 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 524285 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 1 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 6 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 61 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 10 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 1021 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 16 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 65533 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 28 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 268435453 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 27 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 134217725 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 26 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 67108861 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 25 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 33554429 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 24 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 16777213 as U32,
    },
];
static mut ML_defaultDTable: [ZSTD_seqSymbol; 65] = [
    ZSTD_seqSymbol {
        nextState: 1 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 1 as BYTE,
        baseValue: ML_DEFAULTNORMLOG as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 3 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 4 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 5 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 6 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 8 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 9 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 11 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 13 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 16 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 19 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 22 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 25 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 28 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 31 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 34 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 37 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 41 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 47 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 59 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 4 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 83 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 7 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 131 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 9 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 515 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 4 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 5 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 6 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 7 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 9 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 10 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 12 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 15 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 18 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 21 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 24 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 27 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 30 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 33 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 35 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 1 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 39 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 2 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 43 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 3 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 51 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 4 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 67 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 5 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 99 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 8 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 259 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 4 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 48 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 4 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 16 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 4 as BYTE,
        baseValue: 5 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 7 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 8 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 10 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 32 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 5 as BYTE,
        baseValue: 11 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 14 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 17 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 20 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 23 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 26 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 29 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 0 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 32 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 16 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 65539 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 15 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 32771 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 14 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 16387 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 13 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 8195 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 12 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 4099 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 11 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 2051 as U32,
    },
    ZSTD_seqSymbol {
        nextState: 0 as U16,
        nbAdditionalBits: 10 as BYTE,
        nbBits: 6 as BYTE,
        baseValue: 1027 as U32,
    },
];
unsafe extern "C" fn ZSTD_buildSeqTable_rle(
    mut dt: *mut ZSTD_seqSymbol,
    mut baseValue: U32,
    mut nbAddBits: U8,
) {
    let mut ptr: *mut ::core::ffi::c_void = dt as *mut ::core::ffi::c_void;
    let DTableH: *mut ZSTD_seqSymbol_header = ptr as *mut ZSTD_seqSymbol_header;
    let cell: *mut ZSTD_seqSymbol = dt.offset(1 as ::core::ffi::c_int as isize);
    (*DTableH).tableLog = 0 as U32;
    (*DTableH).fastMode = 0 as U32;
    (*cell).nbBits = 0 as BYTE;
    (*cell).nextState = 0 as U16;
    (*cell).nbAdditionalBits = nbAddBits as BYTE;
    (*cell).baseValue = baseValue;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_buildFSETable_body(
    mut dt: *mut ZSTD_seqSymbol,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut baseValue: *const U32,
    mut nbAdditionalBits: *const U8,
    mut tableLog: ::core::ffi::c_uint,
    mut wksp: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) {
    let tableDecode: *mut ZSTD_seqSymbol = dt.offset(1 as ::core::ffi::c_int as isize);
    let maxSV1: U32 = (maxSymbolValue as U32).wrapping_add(1 as U32);
    let tableSize: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let mut symbolNext: *mut U16 = wksp as *mut U16;
    let mut spread: *mut BYTE = symbolNext
        .offset(
            (if 35 as ::core::ffi::c_int > 52 as ::core::ffi::c_int {
                35 as ::core::ffi::c_int
            } else {
                52 as ::core::ffi::c_int
            }) as isize,
        )
        .offset(1 as ::core::ffi::c_int as isize) as *mut BYTE;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1 as U32);
    let mut DTableH: ZSTD_seqSymbol_header = ZSTD_seqSymbol_header {
        fastMode: 0,
        tableLog: 0,
    };
    DTableH.tableLog = tableLog as U32;
    DTableH.fastMode = 1 as U32;
    let largeLimit: S16 =
        ((1 as ::core::ffi::c_int) << tableLog.wrapping_sub(1 as ::core::ffi::c_uint)) as S16;
    let mut s: U32 = 0;
    s = 0 as U32;
    while s < maxSV1 {
        if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int == -(1 as ::core::ffi::c_int)
        {
            let fresh10 = highThreshold;
            highThreshold = highThreshold.wrapping_sub(1);
            (*tableDecode.offset(fresh10 as isize)).baseValue = s;
            *symbolNext.offset(s as isize) = 1 as U16;
        } else {
            if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int
                >= largeLimit as ::core::ffi::c_int
            {
                DTableH.fastMode = 0 as U32;
            }
            *symbolNext.offset(s as isize) = *normalizedCounter.offset(s as isize) as U16;
        }
        s = s.wrapping_add(1);
    }
    ::libc::memcpy(
        dt as *mut ::core::ffi::c_void,
        &raw mut DTableH as *const ::core::ffi::c_void,
        ::core::mem::size_of::<ZSTD_seqSymbol_header>() as ::libc::size_t,
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
                (*tableDecode.offset(uPosition as isize)).baseValue =
                    *spread.offset(s_1.wrapping_add(u) as isize) as U32;
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
            let n_0: ::core::ffi::c_int =
                *normalizedCounter.offset(s_2 as isize) as ::core::ffi::c_int;
            i_0 = 0 as ::core::ffi::c_int;
            while i_0 < n_0 {
                (*tableDecode.offset(position_0 as isize)).baseValue = s_2;
                position_0 = position_0.wrapping_add(step_0) & tableMask_0;
                while (position_0 > highThreshold) as ::core::ffi::c_int as ::core::ffi::c_long != 0
                {
                    position_0 = position_0.wrapping_add(step_0) & tableMask_0;
                }
                i_0 += 1;
            }
            s_2 = s_2.wrapping_add(1);
        }
    }
    let mut u_0: U32 = 0;
    u_0 = 0 as U32;
    while u_0 < tableSize {
        let symbol: U32 = (*tableDecode.offset(u_0 as isize)).baseValue;
        let ref mut fresh11 = *symbolNext.offset(symbol as isize);
        let fresh12 = *fresh11;
        *fresh11 = (*fresh11).wrapping_add(1);
        let nextState: U32 = fresh12 as U32;
        (*tableDecode.offset(u_0 as isize)).nbBits =
            tableLog.wrapping_sub(ZSTD_highbit32(nextState)) as BYTE;
        (*tableDecode.offset(u_0 as isize)).nextState = (nextState
            << (*tableDecode.offset(u_0 as isize)).nbBits as ::core::ffi::c_int)
            .wrapping_sub(tableSize) as U16;
        (*tableDecode.offset(u_0 as isize)).nbAdditionalBits =
            *nbAdditionalBits.offset(symbol as isize) as BYTE;
        (*tableDecode.offset(u_0 as isize)).baseValue = *baseValue.offset(symbol as isize);
        u_0 = u_0.wrapping_add(1);
    }
}
unsafe extern "C" fn ZSTD_buildFSETable_body_default(
    mut dt: *mut ZSTD_seqSymbol,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut baseValue: *const U32,
    mut nbAdditionalBits: *const U8,
    mut tableLog: ::core::ffi::c_uint,
    mut wksp: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) {
    ZSTD_buildFSETable_body(
        dt,
        normalizedCounter,
        maxSymbolValue,
        baseValue,
        nbAdditionalBits,
        tableLog,
        wksp,
        wkspSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildFSETable(
    mut dt: *mut ZSTD_seqSymbol,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut baseValue: *const U32,
    mut nbAdditionalBits: *const U8,
    mut tableLog: ::core::ffi::c_uint,
    mut wksp: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut bmi2: ::core::ffi::c_int,
) {
    ZSTD_buildFSETable_body_default(
        dt,
        normalizedCounter,
        maxSymbolValue,
        baseValue,
        nbAdditionalBits,
        tableLog,
        wksp,
        wkspSize,
    );
}
unsafe extern "C" fn ZSTD_buildSeqTable(
    mut DTableSpace: *mut ZSTD_seqSymbol,
    mut DTablePtr: *mut *const ZSTD_seqSymbol,
    mut type_0: SymbolEncodingType_e,
    mut max: ::core::ffi::c_uint,
    mut maxLog: U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut baseValue: *const U32,
    mut nbAdditionalBits: *const U8,
    mut defaultTable: *const ZSTD_seqSymbol,
    mut flagRepeatTable: U32,
    mut ddictIsCold: ::core::ffi::c_int,
    mut nbSeq: ::core::ffi::c_int,
    mut wksp: *mut U32,
    mut wkspSize: size_t,
    mut bmi2: ::core::ffi::c_int,
) -> size_t {
    match type_0 as ::core::ffi::c_uint {
        1 => {
            if srcSize == 0 {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            if *(src as *const BYTE) as ::core::ffi::c_uint > max {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            let symbol: U32 = *(src as *const BYTE) as U32;
            let baseline: U32 = *baseValue.offset(symbol as isize);
            let nbBits: U8 = *nbAdditionalBits.offset(symbol as isize);
            ZSTD_buildSeqTable_rle(DTableSpace, baseline, nbBits);
            *DTablePtr = DTableSpace;
            return 1 as size_t;
        }
        0 => {
            *DTablePtr = defaultTable;
            return 0 as size_t;
        }
        3 => {
            if flagRepeatTable == 0 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if ddictIsCold != 0 && nbSeq > 24 as ::core::ffi::c_int {
                let pStart: *const ::core::ffi::c_void = *DTablePtr as *const ::core::ffi::c_void;
                let pSize: size_t = (::core::mem::size_of::<ZSTD_seqSymbol>() as size_t)
                    .wrapping_mul(
                        (1 as ::core::ffi::c_int + ((1 as ::core::ffi::c_int) << maxLog)) as size_t,
                    );
                let _ptr: *const ::core::ffi::c_char = pStart as *const ::core::ffi::c_char;
                let _size: size_t = pSize;
                let mut _pos: size_t = 0;
                _pos = 0 as size_t;
                while _pos < _size {
                    _pos = (_pos as ::core::ffi::c_ulong)
                        .wrapping_add(CACHELINE_SIZE as ::core::ffi::c_ulong)
                        as size_t as size_t;
                }
            }
            return 0 as size_t;
        }
        2 => {
            let mut tableLog: ::core::ffi::c_uint = 0;
            let mut norm: [S16; 53] = [0; 53];
            let headerSize: size_t = FSE_readNCount(
                &raw mut norm as *mut ::core::ffi::c_short,
                &raw mut max,
                &raw mut tableLog,
                src,
                srcSize,
            ) as size_t;
            if ERR_isError(headerSize) != 0 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if tableLog as U32 > maxLog {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            ZSTD_buildFSETable(
                DTableSpace,
                &raw mut norm as *mut S16,
                max,
                baseValue,
                nbAdditionalBits,
                tableLog,
                wksp as *mut ::core::ffi::c_void,
                wkspSize,
                bmi2,
            );
            *DTablePtr = DTableSpace;
            return headerSize;
        }
        _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeSeqHeaders(
    mut dctx: *mut ZSTD_DCtx,
    mut nbSeqPtr: *mut ::core::ffi::c_int,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let mut ip: *const BYTE = istart;
    let mut nbSeq: ::core::ffi::c_int = 0;
    if srcSize < 1 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let fresh8 = ip;
    ip = ip.offset(1);
    nbSeq = *fresh8 as ::core::ffi::c_int;
    if nbSeq > 0x7f as ::core::ffi::c_int {
        if nbSeq == 0xff as ::core::ffi::c_int {
            if ip.offset(2 as ::core::ffi::c_int as isize) > iend {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            nbSeq =
                MEM_readLE16(ip as *const ::core::ffi::c_void) as ::core::ffi::c_int + LONGNBSEQ;
            ip = ip.offset(2 as ::core::ffi::c_int as isize);
        } else {
            if ip >= iend {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            let fresh9 = ip;
            ip = ip.offset(1);
            nbSeq = ((nbSeq - 0x80 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                + *fresh9 as ::core::ffi::c_int;
        }
    }
    *nbSeqPtr = nbSeq;
    if nbSeq == 0 as ::core::ffi::c_int {
        if ip != iend {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
    }
    if ip.offset(1 as ::core::ffi::c_int as isize) > iend {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if *ip as ::core::ffi::c_int & 3 as ::core::ffi::c_int != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let LLtype: SymbolEncodingType_e =
        (*ip as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as SymbolEncodingType_e;
    let OFtype: SymbolEncodingType_e = (*ip as ::core::ffi::c_int >> 4 as ::core::ffi::c_int
        & 3 as ::core::ffi::c_int) as SymbolEncodingType_e;
    let MLtype: SymbolEncodingType_e = (*ip as ::core::ffi::c_int >> 2 as ::core::ffi::c_int
        & 3 as ::core::ffi::c_int) as SymbolEncodingType_e;
    ip = ip.offset(1);
    let llhSize: size_t = ZSTD_buildSeqTable(
        &raw mut (*dctx).entropy.LLTable as *mut ZSTD_seqSymbol,
        &raw mut (*dctx).LLTptr,
        LLtype,
        MaxLL as ::core::ffi::c_uint,
        LLFSELog as U32,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        &raw const LL_base as *const U32,
        &raw const LL_bits as *const U8,
        &raw const LL_defaultDTable as *const ZSTD_seqSymbol,
        (*dctx).fseEntropy,
        (*dctx).ddictIsCold,
        nbSeq,
        &raw mut (*dctx).workspace as *mut U32,
        ::core::mem::size_of::<[U32; 640]>() as size_t,
        ZSTD_DCtx_get_bmi2(dctx),
    ) as size_t;
    if ERR_isError(llhSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(llhSize as isize);
    let ofhSize: size_t = ZSTD_buildSeqTable(
        &raw mut (*dctx).entropy.OFTable as *mut ZSTD_seqSymbol,
        &raw mut (*dctx).OFTptr,
        OFtype,
        MaxOff as ::core::ffi::c_uint,
        OffFSELog as U32,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        &raw const OF_base as *const U32,
        &raw const OF_bits as *const U8,
        &raw const OF_defaultDTable as *const ZSTD_seqSymbol,
        (*dctx).fseEntropy,
        (*dctx).ddictIsCold,
        nbSeq,
        &raw mut (*dctx).workspace as *mut U32,
        ::core::mem::size_of::<[U32; 640]>() as size_t,
        ZSTD_DCtx_get_bmi2(dctx),
    ) as size_t;
    if ERR_isError(ofhSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(ofhSize as isize);
    let mlhSize: size_t = ZSTD_buildSeqTable(
        &raw mut (*dctx).entropy.MLTable as *mut ZSTD_seqSymbol,
        &raw mut (*dctx).MLTptr,
        MLtype,
        MaxML as ::core::ffi::c_uint,
        MLFSELog as U32,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        &raw const ML_base as *const U32,
        &raw const ML_bits as *const U8,
        &raw const ML_defaultDTable as *const ZSTD_seqSymbol,
        (*dctx).fseEntropy,
        (*dctx).ddictIsCold,
        nbSeq,
        &raw mut (*dctx).workspace as *mut U32,
        ::core::mem::size_of::<[U32; 640]>() as size_t,
        ZSTD_DCtx_get_bmi2(dctx),
    ) as size_t;
    if ERR_isError(mlhSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(mlhSize as isize);
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_overlapCopy8(
    mut op: *mut *mut BYTE,
    mut ip: *mut *const BYTE,
    mut offset: size_t,
) {
    if offset < 8 as size_t {
        static mut dec32table: [U32; 8] = [
            0 as ::core::ffi::c_int as U32,
            1 as ::core::ffi::c_int as U32,
            2 as ::core::ffi::c_int as U32,
            1 as ::core::ffi::c_int as U32,
            4 as ::core::ffi::c_int as U32,
            4 as ::core::ffi::c_int as U32,
            4 as ::core::ffi::c_int as U32,
            4 as ::core::ffi::c_int as U32,
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
        let sub2: ::core::ffi::c_int = dec64table[offset as usize];
        *(*op).offset(0 as ::core::ffi::c_int as isize) =
            *(*ip).offset(0 as ::core::ffi::c_int as isize);
        *(*op).offset(1 as ::core::ffi::c_int as isize) =
            *(*ip).offset(1 as ::core::ffi::c_int as isize);
        *(*op).offset(2 as ::core::ffi::c_int as isize) =
            *(*ip).offset(2 as ::core::ffi::c_int as isize);
        *(*op).offset(3 as ::core::ffi::c_int as isize) =
            *(*ip).offset(3 as ::core::ffi::c_int as isize);
        *ip = (*ip).offset(dec32table[offset as usize] as isize);
        ZSTD_copy4(
            (*op).offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            *ip as *const ::core::ffi::c_void,
        );
        *ip = (*ip).offset(-(sub2 as isize));
    } else {
        ZSTD_copy8(
            *op as *mut ::core::ffi::c_void,
            *ip as *const ::core::ffi::c_void,
        );
    }
    *ip = (*ip).offset(8 as ::core::ffi::c_int as isize);
    *op = (*op).offset(8 as ::core::ffi::c_int as isize);
}
unsafe extern "C" fn ZSTD_safecopy(
    mut op: *mut BYTE,
    oend_w: *const BYTE,
    mut ip: *const BYTE,
    mut length: ptrdiff_t,
    mut ovtype: ZSTD_overlap_e,
) {
    let diff: ptrdiff_t = op.offset_from(ip) as ptrdiff_t;
    let oend: *mut BYTE = op.offset(length as isize);
    if length < 8 as ptrdiff_t {
        while op < oend {
            let fresh0 = ip;
            ip = ip.offset(1);
            let fresh1 = op;
            op = op.offset(1);
            *fresh1 = *fresh0;
        }
        return;
    }
    if ovtype as ::core::ffi::c_uint
        == ZSTD_overlap_src_before_dst as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_overlapCopy8(&raw mut op, &raw mut ip, diff as size_t);
        length = (length as ::core::ffi::c_long - 8 as ::core::ffi::c_long) as ptrdiff_t;
    }
    if oend <= oend_w as *mut BYTE {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
            length,
            ovtype,
        );
        return;
    }
    if op <= oend_w as *mut BYTE {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
            oend_w.offset_from(op) as ptrdiff_t,
            ovtype,
        );
        ip = ip.offset(oend_w.offset_from(op) as ::core::ffi::c_long as isize);
        op = op.offset(oend_w.offset_from(op) as ::core::ffi::c_long as isize);
    }
    while op < oend {
        let fresh2 = ip;
        ip = ip.offset(1);
        let fresh3 = op;
        op = op.offset(1);
        *fresh3 = *fresh2;
    }
}
unsafe extern "C" fn ZSTD_safecopyDstBeforeSrc(
    mut op: *mut BYTE,
    mut ip: *const BYTE,
    mut length: ptrdiff_t,
) {
    let diff: ptrdiff_t = op.offset_from(ip) as ptrdiff_t;
    let oend: *mut BYTE = op.offset(length as isize);
    if length < 8 as ptrdiff_t || diff > -(8 as ::core::ffi::c_int) as ptrdiff_t {
        while op < oend {
            let fresh4 = ip;
            ip = ip.offset(1);
            let fresh5 = op;
            op = op.offset(1);
            *fresh5 = *fresh4;
        }
        return;
    }
    if op <= oend.offset(-(WILDCOPY_OVERLENGTH as isize)) && diff < -WILDCOPY_VECLEN as ptrdiff_t {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
            oend.offset(-(WILDCOPY_OVERLENGTH as isize)).offset_from(op) as ptrdiff_t,
            ZSTD_no_overlap,
        );
        ip = ip.offset(oend.offset(-(WILDCOPY_OVERLENGTH as isize)).offset_from(op)
            as ::core::ffi::c_long as isize);
        op = op.offset(oend.offset(-(WILDCOPY_OVERLENGTH as isize)).offset_from(op)
            as ::core::ffi::c_long as isize);
    }
    while op < oend {
        let fresh6 = ip;
        ip = ip.offset(1);
        let fresh7 = op;
        op = op.offset(1);
        *fresh7 = *fresh6;
    }
}
#[inline(never)]
unsafe extern "C" fn ZSTD_execSequenceEnd(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    mut litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.offset(sequence.litLength as isize);
    let sequenceLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    let iLitEnd: *const BYTE = (*litPtr).offset(sequence.litLength as isize);
    let mut match_0: *const BYTE = oLitEnd.offset(-(sequence.offset as isize));
    let oend_w: *mut BYTE = oend.offset(-(WILDCOPY_OVERLENGTH as isize));
    if sequenceLength > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ZSTD_safecopy(
        op,
        oend_w,
        *litPtr,
        sequence.litLength as ptrdiff_t,
        ZSTD_no_overlap,
    );
    op = oLitEnd;
    *litPtr = iLitEnd;
    if sequence.offset > oLitEnd.offset_from(prefixStart) as ::core::ffi::c_long as size_t {
        if sequence.offset > oLitEnd.offset_from(virtualStart) as ::core::ffi::c_long as size_t {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        match_0 =
            dictEnd.offset(-(prefixStart.offset_from(match_0) as ::core::ffi::c_long as isize));
        if match_0.offset(sequence.matchLength as isize) <= dictEnd {
            ::libc::memmove(
                oLitEnd as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                sequence.matchLength as ::libc::size_t,
            );
            return sequenceLength;
        }
        let length1: size_t = dictEnd.offset_from(match_0) as ::core::ffi::c_long as size_t;
        ::libc::memmove(
            oLitEnd as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            length1 as ::libc::size_t,
        );
        op = oLitEnd.offset(length1 as isize);
        sequence.matchLength = (sequence.matchLength as ::core::ffi::c_ulong)
            .wrapping_sub(length1 as ::core::ffi::c_ulong) as size_t
            as size_t;
        match_0 = prefixStart;
    }
    ZSTD_safecopy(
        op,
        oend_w,
        match_0,
        sequence.matchLength as ptrdiff_t,
        ZSTD_overlap_src_before_dst,
    );
    return sequenceLength;
}
#[inline(never)]
unsafe extern "C" fn ZSTD_execSequenceEndSplitLitBuffer(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    oend_w: *const BYTE,
    mut sequence: seq_t,
    mut litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.offset(sequence.litLength as isize);
    let sequenceLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    let iLitEnd: *const BYTE = (*litPtr).offset(sequence.litLength as isize);
    let mut match_0: *const BYTE = oLitEnd.offset(-(sequence.offset as isize));
    if sequenceLength > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if op > *litPtr as *mut BYTE && op < (*litPtr).offset(sequence.litLength as isize) as *mut BYTE
    {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    ZSTD_safecopyDstBeforeSrc(op, *litPtr, sequence.litLength as ptrdiff_t);
    op = oLitEnd;
    *litPtr = iLitEnd;
    if sequence.offset > oLitEnd.offset_from(prefixStart) as ::core::ffi::c_long as size_t {
        if sequence.offset > oLitEnd.offset_from(virtualStart) as ::core::ffi::c_long as size_t {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        match_0 =
            dictEnd.offset(-(prefixStart.offset_from(match_0) as ::core::ffi::c_long as isize));
        if match_0.offset(sequence.matchLength as isize) <= dictEnd {
            ::libc::memmove(
                oLitEnd as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                sequence.matchLength as ::libc::size_t,
            );
            return sequenceLength;
        }
        let length1: size_t = dictEnd.offset_from(match_0) as ::core::ffi::c_long as size_t;
        ::libc::memmove(
            oLitEnd as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            length1 as ::libc::size_t,
        );
        op = oLitEnd.offset(length1 as isize);
        sequence.matchLength = (sequence.matchLength as ::core::ffi::c_ulong)
            .wrapping_sub(length1 as ::core::ffi::c_ulong) as size_t
            as size_t;
        match_0 = prefixStart;
    }
    ZSTD_safecopy(
        op,
        oend_w,
        match_0,
        sequence.matchLength as ptrdiff_t,
        ZSTD_overlap_src_before_dst,
    );
    return sequenceLength;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    mut litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.offset(sequence.litLength as isize);
    let sequenceLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd: *mut BYTE = op.offset(sequenceLength as isize);
    let oend_w: *mut BYTE = oend.offset(-(WILDCOPY_OVERLENGTH as isize));
    let iLitEnd: *const BYTE = (*litPtr).offset(sequence.litLength as isize);
    let mut match_0: *const BYTE = oLitEnd.offset(-(sequence.offset as isize));
    if (iLitEnd > litLimit
        || oMatchEnd > oend_w
        || MEM_32bits() != 0
            && (oend.offset_from(op) as ::core::ffi::c_long as size_t)
                < sequenceLength.wrapping_add(32 as size_t)) as ::core::ffi::c_int
        as ::core::ffi::c_long
        != 0
    {
        return ZSTD_execSequenceEnd(
            op,
            oend,
            sequence,
            litPtr,
            litLimit,
            prefixStart,
            virtualStart,
            dictEnd,
        );
    }
    ZSTD_copy16(
        op as *mut ::core::ffi::c_void,
        *litPtr as *const ::core::ffi::c_void,
    );
    if (sequence.litLength > 16 as size_t) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        ZSTD_wildcopy(
            op.offset(16 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            (*litPtr).offset(16 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            sequence.litLength.wrapping_sub(16 as size_t) as ptrdiff_t,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd;
    if sequence.offset > oLitEnd.offset_from(prefixStart) as ::core::ffi::c_long as size_t {
        if (sequence.offset > oLitEnd.offset_from(virtualStart) as ::core::ffi::c_long as size_t)
            as ::core::ffi::c_int as ::core::ffi::c_long
            != 0
        {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        match_0 = dictEnd.offset(match_0.offset_from(prefixStart) as ::core::ffi::c_long as isize);
        if match_0.offset(sequence.matchLength as isize) <= dictEnd {
            ::libc::memmove(
                oLitEnd as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                sequence.matchLength as ::libc::size_t,
            );
            return sequenceLength;
        }
        let length1: size_t = dictEnd.offset_from(match_0) as ::core::ffi::c_long as size_t;
        ::libc::memmove(
            oLitEnd as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            length1 as ::libc::size_t,
        );
        op = oLitEnd.offset(length1 as isize);
        sequence.matchLength = (sequence.matchLength as ::core::ffi::c_ulong)
            .wrapping_sub(length1 as ::core::ffi::c_ulong) as size_t
            as size_t;
        match_0 = prefixStart;
    }
    if (sequence.offset >= 16 as size_t) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            sequence.matchLength as ptrdiff_t,
            ZSTD_no_overlap,
        );
        return sequenceLength;
    }
    ZSTD_overlapCopy8(&raw mut op, &raw mut match_0, sequence.offset);
    if sequence.matchLength > 8 as size_t {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            sequence.matchLength as ptrdiff_t - 8 as ptrdiff_t,
            ZSTD_overlap_src_before_dst,
        );
    }
    return sequenceLength;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_execSequenceSplitLitBuffer(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    oend_w: *const BYTE,
    mut sequence: seq_t,
    mut litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.offset(sequence.litLength as isize);
    let sequenceLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd: *mut BYTE = op.offset(sequenceLength as isize);
    let iLitEnd: *const BYTE = (*litPtr).offset(sequence.litLength as isize);
    let mut match_0: *const BYTE = oLitEnd.offset(-(sequence.offset as isize));
    if (iLitEnd > litLimit
        || oMatchEnd > oend_w as *mut BYTE
        || MEM_32bits() != 0
            && (oend.offset_from(op) as ::core::ffi::c_long as size_t)
                < sequenceLength.wrapping_add(32 as size_t)) as ::core::ffi::c_int
        as ::core::ffi::c_long
        != 0
    {
        return ZSTD_execSequenceEndSplitLitBuffer(
            op,
            oend,
            oend_w,
            sequence,
            litPtr,
            litLimit,
            prefixStart,
            virtualStart,
            dictEnd,
        );
    }
    ZSTD_copy16(
        op as *mut ::core::ffi::c_void,
        *litPtr as *const ::core::ffi::c_void,
    );
    if (sequence.litLength > 16 as size_t) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        ZSTD_wildcopy(
            op.offset(16 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            (*litPtr).offset(16 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            sequence.litLength.wrapping_sub(16 as size_t) as ptrdiff_t,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd;
    if sequence.offset > oLitEnd.offset_from(prefixStart) as ::core::ffi::c_long as size_t {
        if (sequence.offset > oLitEnd.offset_from(virtualStart) as ::core::ffi::c_long as size_t)
            as ::core::ffi::c_int as ::core::ffi::c_long
            != 0
        {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        match_0 = dictEnd.offset(match_0.offset_from(prefixStart) as ::core::ffi::c_long as isize);
        if match_0.offset(sequence.matchLength as isize) <= dictEnd {
            ::libc::memmove(
                oLitEnd as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                sequence.matchLength as ::libc::size_t,
            );
            return sequenceLength;
        }
        let length1: size_t = dictEnd.offset_from(match_0) as ::core::ffi::c_long as size_t;
        ::libc::memmove(
            oLitEnd as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            length1 as ::libc::size_t,
        );
        op = oLitEnd.offset(length1 as isize);
        sequence.matchLength = (sequence.matchLength as ::core::ffi::c_ulong)
            .wrapping_sub(length1 as ::core::ffi::c_ulong) as size_t
            as size_t;
        match_0 = prefixStart;
    }
    if (sequence.offset >= 16 as size_t) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            sequence.matchLength as ptrdiff_t,
            ZSTD_no_overlap,
        );
        return sequenceLength;
    }
    ZSTD_overlapCopy8(&raw mut op, &raw mut match_0, sequence.offset);
    if sequence.matchLength > 8 as size_t {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            sequence.matchLength as ptrdiff_t - 8 as ptrdiff_t,
            ZSTD_overlap_src_before_dst,
        );
    }
    return sequenceLength;
}
unsafe extern "C" fn ZSTD_initFseState(
    mut DStatePtr: *mut ZSTD_fseState,
    mut bitD: *mut BIT_DStream_t,
    mut dt: *const ZSTD_seqSymbol,
) {
    let mut ptr: *const ::core::ffi::c_void = dt as *const ::core::ffi::c_void;
    let DTableH: *const ZSTD_seqSymbol_header = ptr as *const ZSTD_seqSymbol_header;
    (*DStatePtr).state = BIT_readBits(bitD, (*DTableH).tableLog as ::core::ffi::c_uint) as size_t;
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.offset(1 as ::core::ffi::c_int as isize);
}
#[inline(always)]
unsafe extern "C" fn ZSTD_updateFseStateWithDInfo(
    mut DStatePtr: *mut ZSTD_fseState,
    mut bitD: *mut BIT_DStream_t,
    mut nextState: U16,
    mut nbBits: U32,
) {
    let lowBits: size_t = BIT_readBits(bitD, nbBits as ::core::ffi::c_uint) as size_t;
    (*DStatePtr).state = (nextState as size_t).wrapping_add(lowBits);
}
#[inline(always)]
unsafe extern "C" fn ZSTD_decodeSequence(
    mut seqState: *mut seqState_t,
    longOffsets: ZSTD_longOffset_e,
    isLastSeq: ::core::ffi::c_int,
) -> seq_t {
    let mut seq: seq_t = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };
    let llDInfo: *const ZSTD_seqSymbol = (*seqState)
        .stateLL
        .table
        .offset((*seqState).stateLL.state as isize);
    let mlDInfo: *const ZSTD_seqSymbol = (*seqState)
        .stateML
        .table
        .offset((*seqState).stateML.state as isize);
    let ofDInfo: *const ZSTD_seqSymbol = (*seqState)
        .stateOffb
        .table
        .offset((*seqState).stateOffb.state as isize);
    seq.matchLength = (*mlDInfo).baseValue as size_t;
    seq.litLength = (*llDInfo).baseValue as size_t;
    let ofBase: U32 = (*ofDInfo).baseValue;
    let llBits: BYTE = (*llDInfo).nbAdditionalBits;
    let mlBits: BYTE = (*mlDInfo).nbAdditionalBits;
    let ofBits: BYTE = (*ofDInfo).nbAdditionalBits;
    let totalBits: BYTE = (llBits as ::core::ffi::c_int
        + mlBits as ::core::ffi::c_int
        + ofBits as ::core::ffi::c_int) as BYTE;
    let llNext: U16 = (*llDInfo).nextState;
    let mlNext: U16 = (*mlDInfo).nextState;
    let ofNext: U16 = (*ofDInfo).nextState;
    let llnbBits: U32 = (*llDInfo).nbBits as U32;
    let mlnbBits: U32 = (*mlDInfo).nbBits as U32;
    let ofnbBits: U32 = (*ofDInfo).nbBits as U32;
    let mut offset: size_t = 0;
    if ofBits as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
        if MEM_32bits() != 0
            && longOffsets as ::core::ffi::c_uint != 0
            && ofBits as ::core::ffi::c_int >= STREAM_ACCUMULATOR_MIN_32
        {
            let extraBits: U32 = (if ZSTD_WINDOWLOG_MAX_32 > STREAM_ACCUMULATOR_MIN_32 {
                ZSTD_WINDOWLOG_MAX_32 - STREAM_ACCUMULATOR_MIN_32
            } else {
                0 as ::core::ffi::c_int
            }) as U32;
            offset = (ofBase as size_t).wrapping_add(
                BIT_readBitsFast(
                    &raw mut (*seqState).DStream,
                    (ofBits as ::core::ffi::c_uint).wrapping_sub(extraBits as ::core::ffi::c_uint),
                ) << extraBits,
            );
            BIT_reloadDStream(&raw mut (*seqState).DStream);
            offset = (offset as ::core::ffi::c_ulong).wrapping_add(BIT_readBitsFast(
                &raw mut (*seqState).DStream,
                extraBits as ::core::ffi::c_uint,
            )
                as ::core::ffi::c_ulong) as size_t as size_t;
        } else {
            offset = (ofBase as size_t).wrapping_add(BIT_readBitsFast(
                &raw mut (*seqState).DStream,
                ofBits as ::core::ffi::c_uint,
            ));
            if MEM_32bits() != 0 {
                BIT_reloadDStream(&raw mut (*seqState).DStream);
            }
        }
        (*seqState).prevOffset[2 as ::core::ffi::c_int as usize] =
            (*seqState).prevOffset[1 as ::core::ffi::c_int as usize];
        (*seqState).prevOffset[1 as ::core::ffi::c_int as usize] =
            (*seqState).prevOffset[0 as ::core::ffi::c_int as usize];
        (*seqState).prevOffset[0 as ::core::ffi::c_int as usize] = offset;
    } else {
        let ll0: U32 = ((*llDInfo).baseValue == 0 as U32) as ::core::ffi::c_int as U32;
        if (ofBits as ::core::ffi::c_int == 0 as ::core::ffi::c_int) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            offset = (*seqState).prevOffset[ll0 as usize];
            (*seqState).prevOffset[1 as ::core::ffi::c_int as usize] =
                (*seqState).prevOffset[(ll0 == 0) as ::core::ffi::c_int as usize];
            (*seqState).prevOffset[0 as ::core::ffi::c_int as usize] = offset;
        } else {
            offset = (ofBase.wrapping_add(ll0) as size_t).wrapping_add(BIT_readBitsFast(
                &raw mut (*seqState).DStream,
                1 as ::core::ffi::c_uint,
            ));
            let mut temp: size_t = if offset == 3 as size_t {
                (*seqState).prevOffset[0 as ::core::ffi::c_int as usize].wrapping_sub(1 as size_t)
            } else {
                (*seqState).prevOffset[offset as usize]
            };
            temp = (temp as ::core::ffi::c_ulong)
                .wrapping_sub((temp == 0) as ::core::ffi::c_int as ::core::ffi::c_ulong)
                as size_t as size_t;
            if offset != 1 as size_t {
                (*seqState).prevOffset[2 as ::core::ffi::c_int as usize] =
                    (*seqState).prevOffset[1 as ::core::ffi::c_int as usize];
            }
            (*seqState).prevOffset[1 as ::core::ffi::c_int as usize] =
                (*seqState).prevOffset[0 as ::core::ffi::c_int as usize];
            offset = temp;
            (*seqState).prevOffset[0 as ::core::ffi::c_int as usize] = offset;
        }
    }
    seq.offset = offset;
    if mlBits as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        seq.matchLength = (seq.matchLength as ::core::ffi::c_ulong).wrapping_add(BIT_readBitsFast(
            &raw mut (*seqState).DStream,
            mlBits as ::core::ffi::c_uint,
        )
            as ::core::ffi::c_ulong) as size_t as size_t;
    }
    if MEM_32bits() != 0
        && mlBits as ::core::ffi::c_int + llBits as ::core::ffi::c_int
            >= STREAM_ACCUMULATOR_MIN_32
                - (if ZSTD_WINDOWLOG_MAX_32 > STREAM_ACCUMULATOR_MIN_32 {
                    ZSTD_WINDOWLOG_MAX_32 - STREAM_ACCUMULATOR_MIN_32
                } else {
                    0 as ::core::ffi::c_int
                })
    {
        BIT_reloadDStream(&raw mut (*seqState).DStream);
    }
    if MEM_64bits() != 0
        && (totalBits as ::core::ffi::c_int
            >= 57 as ::core::ffi::c_int
                - (9 as ::core::ffi::c_int + 9 as ::core::ffi::c_int + 8 as ::core::ffi::c_int))
            as ::core::ffi::c_int as ::core::ffi::c_long
            != 0
    {
        BIT_reloadDStream(&raw mut (*seqState).DStream);
    }
    if llBits as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        seq.litLength = (seq.litLength as ::core::ffi::c_ulong).wrapping_add(BIT_readBitsFast(
            &raw mut (*seqState).DStream,
            llBits as ::core::ffi::c_uint,
        )
            as ::core::ffi::c_ulong) as size_t as size_t;
    }
    if MEM_32bits() != 0 {
        BIT_reloadDStream(&raw mut (*seqState).DStream);
    }
    if isLastSeq == 0 {
        ZSTD_updateFseStateWithDInfo(
            &raw mut (*seqState).stateLL,
            &raw mut (*seqState).DStream,
            llNext,
            llnbBits,
        );
        ZSTD_updateFseStateWithDInfo(
            &raw mut (*seqState).stateML,
            &raw mut (*seqState).DStream,
            mlNext,
            mlnbBits,
        );
        if MEM_32bits() != 0 {
            BIT_reloadDStream(&raw mut (*seqState).DStream);
        }
        ZSTD_updateFseStateWithDInfo(
            &raw mut (*seqState).stateOffb,
            &raw mut (*seqState).DStream,
            ofNext,
            ofnbBits,
        );
        BIT_reloadDStream(&raw mut (*seqState).DStream);
    }
    return seq;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_decompressSequences_bodySplitLitBuffer(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.offset(seqSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE =
        ZSTD_maybeNullPtrAdd(ostart as *mut ::core::ffi::c_uchar, maxDstSize as ptrdiff_t)
            as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let mut litBufferEnd: *const BYTE = (*dctx).litBufferEnd;
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let vBase: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;
    if nbSeq != 0 {
        let mut seqState: seqState_t = seqState_t {
            DStream: BIT_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: ::core::ptr::null::<::core::ffi::c_char>(),
                start: ::core::ptr::null::<::core::ffi::c_char>(),
                limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
            },
            stateLL: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            stateOffb: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            stateML: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            prevOffset: [0; 3],
        };
        (*dctx).fseEntropy = 1 as U32;
        let mut i: U32 = 0;
        i = 0 as U32;
        while i < ZSTD_REP_NUM as U32 {
            seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as size_t;
            i = i.wrapping_add(1);
        }
        if ERR_isError(BIT_initDStream(
            &raw mut seqState.DStream,
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        )) != 0
        {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        ZSTD_initFseState(
            &raw mut seqState.stateLL,
            &raw mut seqState.DStream,
            (*dctx).LLTptr,
        );
        ZSTD_initFseState(
            &raw mut seqState.stateOffb,
            &raw mut seqState.DStream,
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(
            &raw mut seqState.stateML,
            &raw mut seqState.DStream,
            (*dctx).MLTptr,
        );
        let mut sequence: seq_t = seq_t {
            litLength: 0 as size_t,
            matchLength: 0 as size_t,
            offset: 0 as size_t,
        };
        asm!(".p2align 6\n", options(preserves_flags, att_syntax));
        while nbSeq != 0 {
            sequence = ZSTD_decodeSequence(
                &raw mut seqState,
                isLongOffset,
                (nbSeq == 1 as ::core::ffi::c_int) as ::core::ffi::c_int,
            );
            if litPtr.offset(sequence.litLength as isize) > (*dctx).litBufferEnd {
                break;
            }
            let oneSeqSize: size_t = ZSTD_execSequenceSplitLitBuffer(
                op,
                oend,
                litPtr
                    .offset(sequence.litLength as isize)
                    .offset(-(WILDCOPY_OVERLENGTH as isize)),
                sequence,
                &raw mut litPtr,
                litBufferEnd,
                prefixStart,
                vBase,
                dictEnd,
            ) as size_t;
            if ERR_isError(oneSeqSize) as ::core::ffi::c_long != 0 {
                return oneSeqSize;
            }
            op = op.offset(oneSeqSize as isize);
            nbSeq -= 1;
        }
        if nbSeq > 0 as ::core::ffi::c_int {
            let leftoverLit: size_t =
                (*dctx).litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
            if leftoverLit != 0 {
                if leftoverLit > oend.offset_from(op) as ::core::ffi::c_long as size_t {
                    return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
                }
                ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as ptrdiff_t);
                sequence.litLength = (sequence.litLength as ::core::ffi::c_ulong)
                    .wrapping_sub(leftoverLit as ::core::ffi::c_ulong)
                    as size_t as size_t;
                op = op.offset(leftoverLit as isize);
            }
            litPtr = &raw mut (*dctx).litExtraBuffer as *mut BYTE;
            litBufferEnd = (&raw mut (*dctx).litExtraBuffer as *mut BYTE).offset(
                (if 64 as ::core::ffi::c_int
                    > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                {
                    64 as ::core::ffi::c_int
                } else {
                    (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                        < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    {
                        (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                    } else {
                        (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                    })
                }) as isize,
            );
            (*dctx).litBufferLocation = ZSTD_not_in_dst;
            let oneSeqSize_0: size_t = ZSTD_execSequence(
                op,
                oend,
                sequence,
                &raw mut litPtr,
                litBufferEnd,
                prefixStart,
                vBase,
                dictEnd,
            ) as size_t;
            if ERR_isError(oneSeqSize_0) as ::core::ffi::c_long != 0 {
                return oneSeqSize_0;
            }
            op = op.offset(oneSeqSize_0 as isize);
            nbSeq -= 1;
        }
        if nbSeq > 0 as ::core::ffi::c_int {
            asm!(".p2align 6\n", options(preserves_flags, att_syntax));
            asm!("nop\n", options(preserves_flags, att_syntax));
            asm!(".p2align 4\n", options(preserves_flags, att_syntax));
            asm!("nop\n", options(preserves_flags, att_syntax));
            asm!(".p2align 3\n", options(preserves_flags, att_syntax));
            while nbSeq != 0 {
                let sequence_0: seq_t = ZSTD_decodeSequence(
                    &raw mut seqState,
                    isLongOffset,
                    (nbSeq == 1 as ::core::ffi::c_int) as ::core::ffi::c_int,
                ) as seq_t;
                let oneSeqSize_1: size_t = ZSTD_execSequence(
                    op,
                    oend,
                    sequence_0,
                    &raw mut litPtr,
                    litBufferEnd,
                    prefixStart,
                    vBase,
                    dictEnd,
                ) as size_t;
                if ERR_isError(oneSeqSize_1) as ::core::ffi::c_long != 0 {
                    return oneSeqSize_1;
                }
                op = op.offset(oneSeqSize_1 as isize);
                nbSeq -= 1;
            }
        }
        if nbSeq != 0 {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        if BIT_endOfDStream(&raw mut seqState.DStream) == 0 {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        let mut i_0: U32 = 0;
        i_0 = 0 as U32;
        while i_0 < ZSTD_REP_NUM as U32 {
            (*dctx).entropy.rep[i_0 as usize] = seqState.prevOffset[i_0 as usize] as U32;
            i_0 = i_0.wrapping_add(1);
        }
    }
    if (*dctx).litBufferLocation as ::core::ffi::c_uint
        == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let lastLLSize: size_t = litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
        if lastLLSize > oend.offset_from(op) as ::core::ffi::c_long as size_t {
            return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
        }
        if !op.is_null() {
            ::libc::memmove(
                op as *mut ::core::ffi::c_void,
                litPtr as *const ::core::ffi::c_void,
                lastLLSize as ::libc::size_t,
            );
            op = op.offset(lastLLSize as isize);
        }
        litPtr = &raw mut (*dctx).litExtraBuffer as *mut BYTE;
        litBufferEnd = (&raw mut (*dctx).litExtraBuffer as *mut BYTE).offset(
            (if 64 as ::core::ffi::c_int
                > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            {
                64 as ::core::ffi::c_int
            } else {
                (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            }) as isize,
        );
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    }
    let lastLLSize_0: size_t = litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
    if lastLLSize_0 > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if !op.is_null() {
        ::libc::memcpy(
            op as *mut ::core::ffi::c_void,
            litPtr as *const ::core::ffi::c_void,
            lastLLSize_0 as ::libc::size_t,
        );
        op = op.offset(lastLLSize_0 as isize);
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_decompressSequences_body(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.offset(seqSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation as ::core::ffi::c_uint
        == ZSTD_not_in_dst as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_maybeNullPtrAdd(ostart as *mut ::core::ffi::c_uchar, maxDstSize as ptrdiff_t)
            as *mut BYTE
    } else {
        (*dctx).litBuffer
    };
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.offset((*dctx).litSize as isize);
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let vBase: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;
    if nbSeq != 0 {
        let mut seqState: seqState_t = seqState_t {
            DStream: BIT_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: ::core::ptr::null::<::core::ffi::c_char>(),
                start: ::core::ptr::null::<::core::ffi::c_char>(),
                limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
            },
            stateLL: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            stateOffb: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            stateML: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            prevOffset: [0; 3],
        };
        (*dctx).fseEntropy = 1 as U32;
        let mut i: U32 = 0;
        i = 0 as U32;
        while i < ZSTD_REP_NUM as U32 {
            seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as size_t;
            i = i.wrapping_add(1);
        }
        if ERR_isError(BIT_initDStream(
            &raw mut seqState.DStream,
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        )) != 0
        {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        ZSTD_initFseState(
            &raw mut seqState.stateLL,
            &raw mut seqState.DStream,
            (*dctx).LLTptr,
        );
        ZSTD_initFseState(
            &raw mut seqState.stateOffb,
            &raw mut seqState.DStream,
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(
            &raw mut seqState.stateML,
            &raw mut seqState.DStream,
            (*dctx).MLTptr,
        );
        asm!(".p2align 6\n", options(preserves_flags, att_syntax));
        asm!("nop\n", options(preserves_flags, att_syntax));
        asm!(".p2align 4\n", options(preserves_flags, att_syntax));
        asm!("nop\n", options(preserves_flags, att_syntax));
        asm!(".p2align 3\n", options(preserves_flags, att_syntax));
        while nbSeq != 0 {
            let sequence: seq_t = ZSTD_decodeSequence(
                &raw mut seqState,
                isLongOffset,
                (nbSeq == 1 as ::core::ffi::c_int) as ::core::ffi::c_int,
            ) as seq_t;
            let oneSeqSize: size_t = ZSTD_execSequence(
                op,
                oend,
                sequence,
                &raw mut litPtr,
                litEnd,
                prefixStart,
                vBase,
                dictEnd,
            ) as size_t;
            if ERR_isError(oneSeqSize) as ::core::ffi::c_long != 0 {
                return oneSeqSize;
            }
            op = op.offset(oneSeqSize as isize);
            nbSeq -= 1;
        }
        if BIT_endOfDStream(&raw mut seqState.DStream) == 0 {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        let mut i_0: U32 = 0;
        i_0 = 0 as U32;
        while i_0 < ZSTD_REP_NUM as U32 {
            (*dctx).entropy.rep[i_0 as usize] = seqState.prevOffset[i_0 as usize] as U32;
            i_0 = i_0.wrapping_add(1);
        }
    }
    let lastLLSize: size_t = litEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
    if lastLLSize > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if !op.is_null() {
        ::libc::memcpy(
            op as *mut ::core::ffi::c_void,
            litPtr as *const ::core::ffi::c_void,
            lastLLSize as ::libc::size_t,
        );
        op = op.offset(lastLLSize as isize);
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompressSequences_default(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    return ZSTD_decompressSequences_body(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    );
}
unsafe extern "C" fn ZSTD_decompressSequencesSplitLitBuffer_default(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    return ZSTD_decompressSequences_bodySplitLitBuffer(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    );
}
#[inline(always)]
unsafe extern "C" fn ZSTD_prefetchMatch(
    mut prefetchPos: size_t,
    sequence: seq_t,
    prefixStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    prefetchPos = (prefetchPos as ::core::ffi::c_ulong)
        .wrapping_add(sequence.litLength as ::core::ffi::c_ulong) as size_t
        as size_t;
    let matchBase: *const BYTE = if sequence.offset > prefetchPos {
        dictEnd
    } else {
        prefixStart
    };
    let match_0: *const BYTE = ZSTD_wrappedPtrSub(
        ZSTD_wrappedPtrAdd(
            matchBase as *const ::core::ffi::c_uchar,
            prefetchPos as ptrdiff_t,
        ),
        sequence.offset as ptrdiff_t,
    ) as *const BYTE;
    return prefetchPos.wrapping_add(sequence.matchLength);
}
#[inline(always)]
unsafe extern "C" fn ZSTD_decompressSequencesLong_body(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.offset(seqSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation as ::core::ffi::c_uint
        == ZSTD_in_dst as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (*dctx).litBuffer
    } else {
        ZSTD_maybeNullPtrAdd(ostart as *mut ::core::ffi::c_uchar, maxDstSize as ptrdiff_t)
            as *mut BYTE
    };
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let mut litBufferEnd: *const BYTE = (*dctx).litBufferEnd;
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let dictStart: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;
    if nbSeq != 0 {
        let mut sequences: [seq_t; 8] = [seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        }; 8];
        let seqAdvance: ::core::ffi::c_int = if nbSeq < 8 as ::core::ffi::c_int {
            nbSeq
        } else {
            8 as ::core::ffi::c_int
        };
        let mut seqState: seqState_t = seqState_t {
            DStream: BIT_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: ::core::ptr::null::<::core::ffi::c_char>(),
                start: ::core::ptr::null::<::core::ffi::c_char>(),
                limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
            },
            stateLL: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            stateOffb: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            stateML: ZSTD_fseState {
                state: 0,
                table: ::core::ptr::null::<ZSTD_seqSymbol>(),
            },
            prevOffset: [0; 3],
        };
        let mut seqNb: ::core::ffi::c_int = 0;
        let mut prefetchPos: size_t = op.offset_from(prefixStart) as ::core::ffi::c_long as size_t;
        (*dctx).fseEntropy = 1 as U32;
        let mut i: ::core::ffi::c_int = 0;
        i = 0 as ::core::ffi::c_int;
        while i < ZSTD_REP_NUM {
            seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as size_t;
            i += 1;
        }
        if ERR_isError(BIT_initDStream(
            &raw mut seqState.DStream,
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        )) != 0
        {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        ZSTD_initFseState(
            &raw mut seqState.stateLL,
            &raw mut seqState.DStream,
            (*dctx).LLTptr,
        );
        ZSTD_initFseState(
            &raw mut seqState.stateOffb,
            &raw mut seqState.DStream,
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(
            &raw mut seqState.stateML,
            &raw mut seqState.DStream,
            (*dctx).MLTptr,
        );
        seqNb = 0 as ::core::ffi::c_int;
        while seqNb < seqAdvance {
            let sequence: seq_t = ZSTD_decodeSequence(
                &raw mut seqState,
                isLongOffset,
                (seqNb == nbSeq - 1 as ::core::ffi::c_int) as ::core::ffi::c_int,
            ) as seq_t;
            prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
            sequences[seqNb as usize] = sequence;
            seqNb += 1;
        }
        while seqNb < nbSeq {
            let mut sequence_0: seq_t = ZSTD_decodeSequence(
                &raw mut seqState,
                isLongOffset,
                (seqNb == nbSeq - 1 as ::core::ffi::c_int) as ::core::ffi::c_int,
            );
            if (*dctx).litBufferLocation as ::core::ffi::c_uint
                == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
                && litPtr.offset(
                    sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize].litLength
                        as isize,
                ) > (*dctx).litBufferEnd
            {
                let leftoverLit: size_t =
                    (*dctx).litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
                if leftoverLit != 0 {
                    if leftoverLit > oend.offset_from(op) as ::core::ffi::c_long as size_t {
                        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as ptrdiff_t);
                    sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize].litLength =
                        (sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize].litLength
                            as ::core::ffi::c_ulong)
                            .wrapping_sub(leftoverLit as ::core::ffi::c_ulong)
                            as size_t as size_t;
                    op = op.offset(leftoverLit as isize);
                }
                litPtr = &raw mut (*dctx).litExtraBuffer as *mut BYTE;
                litBufferEnd = (&raw mut (*dctx).litExtraBuffer as *mut BYTE).offset(
                    (if 64 as ::core::ffi::c_int
                        > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    {
                        64 as ::core::ffi::c_int
                    } else {
                        (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    }) as isize,
                );
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                let oneSeqSize: size_t = ZSTD_execSequence(
                    op,
                    oend,
                    sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize],
                    &raw mut litPtr,
                    litBufferEnd,
                    prefixStart,
                    dictStart,
                    dictEnd,
                ) as size_t;
                if ERR_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence_0, prefixStart, dictEnd);
                sequences[(seqNb & STORED_SEQS_MASK) as usize] = sequence_0;
                op = op.offset(oneSeqSize as isize);
            } else {
                let oneSeqSize_0: size_t = if (*dctx).litBufferLocation as ::core::ffi::c_uint
                    == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .offset(
                                sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize]
                                    .litLength as isize,
                            )
                            .offset(-(WILDCOPY_OVERLENGTH as isize)),
                        sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize],
                        &raw mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    ) as size_t
                } else {
                    ZSTD_execSequence(
                        op,
                        oend,
                        sequences[(seqNb - ADVANCED_SEQS & STORED_SEQS_MASK) as usize],
                        &raw mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    ) as size_t
                };
                if ERR_isError(oneSeqSize_0) != 0 {
                    return oneSeqSize_0;
                }
                prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence_0, prefixStart, dictEnd);
                sequences[(seqNb & STORED_SEQS_MASK) as usize] = sequence_0;
                op = op.offset(oneSeqSize_0 as isize);
            }
            seqNb += 1;
        }
        if BIT_endOfDStream(&raw mut seqState.DStream) == 0 {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        seqNb -= seqAdvance;
        while seqNb < nbSeq {
            let mut sequence_1: *mut seq_t = (&raw mut sequences as *mut seq_t)
                .offset((seqNb & STORED_SEQS_MASK) as isize)
                as *mut seq_t;
            if (*dctx).litBufferLocation as ::core::ffi::c_uint
                == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
                && litPtr.offset((*sequence_1).litLength as isize) > (*dctx).litBufferEnd
            {
                let leftoverLit_0: size_t =
                    (*dctx).litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
                if leftoverLit_0 != 0 {
                    if leftoverLit_0 > oend.offset_from(op) as ::core::ffi::c_long as size_t {
                        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit_0 as ptrdiff_t);
                    (*sequence_1).litLength = ((*sequence_1).litLength as ::core::ffi::c_ulong)
                        .wrapping_sub(leftoverLit_0 as ::core::ffi::c_ulong)
                        as size_t as size_t;
                    op = op.offset(leftoverLit_0 as isize);
                }
                litPtr = &raw mut (*dctx).litExtraBuffer as *mut BYTE;
                litBufferEnd = (&raw mut (*dctx).litExtraBuffer as *mut BYTE).offset(
                    (if 64 as ::core::ffi::c_int
                        > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    {
                        64 as ::core::ffi::c_int
                    } else {
                        (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                            < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        {
                            (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                        } else {
                            (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                        })
                    }) as isize,
                );
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                let oneSeqSize_1: size_t = ZSTD_execSequence(
                    op,
                    oend,
                    *sequence_1,
                    &raw mut litPtr,
                    litBufferEnd,
                    prefixStart,
                    dictStart,
                    dictEnd,
                ) as size_t;
                if ERR_isError(oneSeqSize_1) != 0 {
                    return oneSeqSize_1;
                }
                op = op.offset(oneSeqSize_1 as isize);
            } else {
                let oneSeqSize_2: size_t = if (*dctx).litBufferLocation as ::core::ffi::c_uint
                    == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .offset((*sequence_1).litLength as isize)
                            .offset(-(WILDCOPY_OVERLENGTH as isize)),
                        *sequence_1,
                        &raw mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    ) as size_t
                } else {
                    ZSTD_execSequence(
                        op,
                        oend,
                        *sequence_1,
                        &raw mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    ) as size_t
                };
                if ERR_isError(oneSeqSize_2) != 0 {
                    return oneSeqSize_2;
                }
                op = op.offset(oneSeqSize_2 as isize);
            }
            seqNb += 1;
        }
        let mut i_0: U32 = 0;
        i_0 = 0 as U32;
        while i_0 < ZSTD_REP_NUM as U32 {
            (*dctx).entropy.rep[i_0 as usize] = seqState.prevOffset[i_0 as usize] as U32;
            i_0 = i_0.wrapping_add(1);
        }
    }
    if (*dctx).litBufferLocation as ::core::ffi::c_uint
        == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let lastLLSize: size_t = litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
        if lastLLSize > oend.offset_from(op) as ::core::ffi::c_long as size_t {
            return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
        }
        if !op.is_null() {
            ::libc::memmove(
                op as *mut ::core::ffi::c_void,
                litPtr as *const ::core::ffi::c_void,
                lastLLSize as ::libc::size_t,
            );
            op = op.offset(lastLLSize as isize);
        }
        litPtr = &raw mut (*dctx).litExtraBuffer as *mut BYTE;
        litBufferEnd = (&raw mut (*dctx).litExtraBuffer as *mut BYTE).offset(
            (if 64 as ::core::ffi::c_int
                > (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            {
                64 as ::core::ffi::c_int
            } else {
                (if ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
                    < (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                {
                    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
                } else {
                    (128 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int
                })
            }) as isize,
        );
    }
    let lastLLSize_0: size_t = litBufferEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
    if lastLLSize_0 > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if !op.is_null() {
        ::libc::memmove(
            op as *mut ::core::ffi::c_void,
            litPtr as *const ::core::ffi::c_void,
            lastLLSize_0 as ::libc::size_t,
        );
        op = op.offset(lastLLSize_0 as isize);
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
pub const STORED_SEQS: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const STORED_SEQS_MASK: ::core::ffi::c_int = STORED_SEQS - 1 as ::core::ffi::c_int;
pub const ADVANCED_SEQS: ::core::ffi::c_int = STORED_SEQS;
unsafe extern "C" fn ZSTD_decompressSequencesLong_default(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    return ZSTD_decompressSequencesLong_body(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    );
}
unsafe extern "C" fn ZSTD_decompressSequences(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    return ZSTD_decompressSequences_default(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    );
}
unsafe extern "C" fn ZSTD_decompressSequencesSplitLitBuffer(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    return ZSTD_decompressSequencesSplitLitBuffer_default(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    );
}
unsafe extern "C" fn ZSTD_decompressSequencesLong(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
    mut nbSeq: ::core::ffi::c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    return ZSTD_decompressSequencesLong_default(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    );
}
unsafe extern "C" fn ZSTD_totalHistorySize(
    mut op: *mut BYTE,
    mut virtualStart: *const BYTE,
) -> size_t {
    return op.offset_from(virtualStart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_getOffsetInfo(
    mut offTable: *const ZSTD_seqSymbol,
    mut nbSeq: ::core::ffi::c_int,
) -> ZSTD_OffsetInfo {
    let mut info: ZSTD_OffsetInfo = ZSTD_OffsetInfo {
        longOffsetShare: 0 as ::core::ffi::c_uint,
        maxNbAdditionalBits: 0 as ::core::ffi::c_uint,
    };
    if nbSeq != 0 as ::core::ffi::c_int {
        let mut ptr: *const ::core::ffi::c_void = offTable as *const ::core::ffi::c_void;
        let tableLog: U32 = (*(ptr as *const ZSTD_seqSymbol_header)
            .offset(0 as ::core::ffi::c_int as isize))
        .tableLog;
        let mut table: *const ZSTD_seqSymbol = offTable.offset(1 as ::core::ffi::c_int as isize);
        let max: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
        let mut u: U32 = 0;
        u = 0 as U32;
        while u < max {
            info.maxNbAdditionalBits = if info.maxNbAdditionalBits
                > (*table.offset(u as isize)).nbAdditionalBits as ::core::ffi::c_uint
            {
                info.maxNbAdditionalBits
            } else {
                (*table.offset(u as isize)).nbAdditionalBits as ::core::ffi::c_uint
            };
            if (*table.offset(u as isize)).nbAdditionalBits as ::core::ffi::c_int
                > 22 as ::core::ffi::c_int
            {
                info.longOffsetShare = info.longOffsetShare.wrapping_add(1 as ::core::ffi::c_uint);
            }
            u = u.wrapping_add(1);
        }
        info.longOffsetShare <<= (OffFSELog as U32).wrapping_sub(tableLog);
    }
    return info;
}
unsafe extern "C" fn ZSTD_maxShortOffset() -> size_t {
    if MEM_64bits() != 0 {
        return -(1 as ::core::ffi::c_int) as size_t;
    } else {
        let maxOffbase: size_t = ((1 as ::core::ffi::c_int as size_t)
            << ((if MEM_32bits() != 0 {
                STREAM_ACCUMULATOR_MIN_32
            } else {
                STREAM_ACCUMULATOR_MIN_64
            }) as U32)
                .wrapping_add(1 as U32))
        .wrapping_sub(1 as size_t);
        let maxOffset: size_t = maxOffbase.wrapping_sub(ZSTD_REP_NUM as size_t);
        return maxOffset;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_internal(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    streaming: streaming_operation,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    if srcSize > ZSTD_blockSizeMax(dctx) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let litCSize: size_t =
        ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, streaming) as size_t;
    if ERR_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.offset(litCSize as isize);
    srcSize = (srcSize as ::core::ffi::c_ulong).wrapping_sub(litCSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    let blockSizeMax: size_t = if dstCapacity < ZSTD_blockSizeMax(dctx) {
        dstCapacity
    } else {
        ZSTD_blockSizeMax(dctx) as size_t
    };
    let totalHistorySize: size_t = ZSTD_totalHistorySize(
        ZSTD_maybeNullPtrAdd(dst as *mut ::core::ffi::c_uchar, blockSizeMax as ptrdiff_t)
            as *mut BYTE,
        (*dctx).virtualStart as *const BYTE,
    ) as size_t;
    let mut isLongOffset: ZSTD_longOffset_e = (MEM_32bits() != 0
        && totalHistorySize > ZSTD_maxShortOffset())
        as ::core::ffi::c_int as ZSTD_longOffset_e;
    let mut usePrefetchDecoder: ::core::ffi::c_int = (*dctx).ddictIsCold;
    let mut nbSeq: ::core::ffi::c_int = 0;
    let seqHSize: size_t = ZSTD_decodeSeqHeaders(
        dctx,
        &raw mut nbSeq,
        ip as *const ::core::ffi::c_void,
        srcSize,
    ) as size_t;
    if ERR_isError(seqHSize) != 0 {
        return seqHSize;
    }
    ip = ip.offset(seqHSize as isize);
    srcSize = (srcSize as ::core::ffi::c_ulong).wrapping_sub(seqHSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    if (dst.is_null() || dstCapacity == 0 as size_t) && nbSeq > 0 as ::core::ffi::c_int {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if MEM_64bits() != 0
        && ::core::mem::size_of::<size_t>() as usize
            == ::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize
        && (-(1 as ::core::ffi::c_int) as size_t).wrapping_sub(dst as size_t)
            < ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as size_t
    {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if isLongOffset as ::core::ffi::c_uint != 0
        || usePrefetchDecoder == 0
            && totalHistorySize > ((1 as ::core::ffi::c_uint) << 24 as ::core::ffi::c_int) as size_t
            && nbSeq > 8 as ::core::ffi::c_int
    {
        let info: ZSTD_OffsetInfo = ZSTD_getOffsetInfo((*dctx).OFTptr, nbSeq) as ZSTD_OffsetInfo;
        if isLongOffset as ::core::ffi::c_uint != 0
            && info.maxNbAdditionalBits as U32
                <= (if MEM_32bits() != 0 {
                    STREAM_ACCUMULATOR_MIN_32
                } else {
                    STREAM_ACCUMULATOR_MIN_64
                }) as U32
        {
            isLongOffset = ZSTD_lo_isRegularOffset;
        }
        if usePrefetchDecoder == 0 {
            let minShare: U32 = (if MEM_64bits() != 0 {
                7 as ::core::ffi::c_int
            } else {
                20 as ::core::ffi::c_int
            }) as U32;
            usePrefetchDecoder = (info.longOffsetShare as U32 >= minShare) as ::core::ffi::c_int;
        }
    }
    (*dctx).ddictIsCold = 0 as ::core::ffi::c_int;
    if usePrefetchDecoder != 0 {
        return ZSTD_decompressSequencesLong(
            dctx,
            dst,
            dstCapacity,
            ip as *const ::core::ffi::c_void,
            srcSize,
            nbSeq,
            isLongOffset,
        );
    }
    if (*dctx).litBufferLocation as ::core::ffi::c_uint
        == ZSTD_split as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ZSTD_decompressSequencesSplitLitBuffer(
            dctx,
            dst,
            dstCapacity,
            ip as *const ::core::ffi::c_void,
            srcSize,
            nbSeq,
            isLongOffset,
        );
    } else {
        return ZSTD_decompressSequences(
            dctx,
            dst,
            dstCapacity,
            ip as *const ::core::ffi::c_void,
            srcSize,
            nbSeq,
            isLongOffset,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkContinuity(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *const ::core::ffi::c_void,
    mut dstSize: size_t,
) {
    if dst != (*dctx).previousDstEnd && dstSize > 0 as size_t {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).virtualStart = (dst as *const ::core::ffi::c_char).offset(
            -(((*dctx).previousDstEnd as *const ::core::ffi::c_char)
                .offset_from((*dctx).prefixStart as *const ::core::ffi::c_char)
                as ::core::ffi::c_long as isize),
        ) as *const ::core::ffi::c_void;
        (*dctx).prefixStart = dst;
        (*dctx).previousDstEnd = dst;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_deprecated(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut dSize: size_t = 0;
    (*dctx).isFrameDecompression = 0 as ::core::ffi::c_int;
    ZSTD_checkContinuity(dctx, dst, dstCapacity);
    dSize = ZSTD_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize, not_streaming);
    let err_code: size_t = dSize;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    (*dctx).previousDstEnd =
        (dst as *mut ::core::ffi::c_char).offset(dSize as isize) as *const ::core::ffi::c_void;
    return dSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_decompressBlock_deprecated(dctx, dst, dstCapacity, src, srcSize);
}
