extern "C" {
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
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn ZSTD_XXH64_reset(statePtr: *mut XXH64_state_t, seed: XXH64_hash_t) -> XXH_errorcode;
    fn ZSTD_XXH64_update(
        statePtr: *mut XXH64_state_t,
        input: *const ::core::ffi::c_void,
        length: size_t,
    ) -> XXH_errorcode;
    fn ZSTD_XXH64_digest(statePtr: *const XXH64_state_t) -> XXH64_hash_t;
    fn ERR_getErrorString(code: ERR_enum) -> *const ::core::ffi::c_char;
}
pub type ptrdiff_t = isize;
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __int16_t = i16;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type int16_t = __int16_t;
pub type XXH_errorcode = ::core::ffi::c_uint;
pub const XXH_ERROR: XXH_errorcode = 1;
pub const XXH_OK: XXH_errorcode = 0;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type XXH32_hash_t = uint32_t;
pub type XXH64_hash_t = uint64_t;
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
pub type XXH64_state_t = XXH64_state_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: ::core::ffi::c_ulonglong,
    pub windowSize: ::core::ffi::c_uint,
    pub dictID: ::core::ffi::c_uint,
    pub checksumFlag: ::core::ffi::c_uint,
}
pub type U32 = uint32_t;
pub type BYTE = uint8_t;
pub type U64 = uint64_t;
pub const ZSTD_error_frameParameter_unsupported: ERR_enum = 14;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub u: U32,
    pub c: [BYTE; 4],
}
pub type U16 = uint16_t;
pub const ZSTD_error_srcSize_wrong: ERR_enum = 72;
pub const ZSTD_error_prefix_unknown: ERR_enum = 10;
pub type ZSTDv07_DCtx = ZSTDv07_DCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv07_DCtx_s {
    pub LLTable: [FSEv07_DTable; 513],
    pub OffTable: [FSEv07_DTable; 257],
    pub MLTable: [FSEv07_DTable; 513],
    pub hufTable: [HUFv07_DTable; 4097],
    pub previousDstEnd: *const ::core::ffi::c_void,
    pub base: *const ::core::ffi::c_void,
    pub vBase: *const ::core::ffi::c_void,
    pub dictEnd: *const ::core::ffi::c_void,
    pub expected: size_t,
    pub rep: [U32; 3],
    pub fParams: ZSTDv07_frameParams,
    pub bType: blockType_t,
    pub stage: ZSTDv07_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: size_t,
    pub dictID: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTDv07_customMem,
    pub litSize: size_t,
    pub litBuffer: [BYTE; 131080],
    pub headerBuffer: [BYTE; 18],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv07_customMem {
    pub customAlloc: ZSTDv07_allocFunction,
    pub customFree: ZSTDv07_freeFunction,
    pub opaque: *mut ::core::ffi::c_void,
}
pub type ZSTDv07_freeFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>;
pub type ZSTDv07_allocFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, size_t) -> *mut ::core::ffi::c_void>;
pub type ZSTDv07_dStage = ::core::ffi::c_uint;
pub const ZSTDds_skipFrame: ZSTDv07_dStage = 5;
pub const ZSTDds_decodeSkippableHeader: ZSTDv07_dStage = 4;
pub const ZSTDds_decompressBlock: ZSTDv07_dStage = 3;
pub const ZSTDds_decodeBlockHeader: ZSTDv07_dStage = 2;
pub const ZSTDds_decodeFrameHeader: ZSTDv07_dStage = 1;
pub const ZSTDds_getFrameHeaderSize: ZSTDv07_dStage = 0;
pub type blockType_t = ::core::ffi::c_uint;
pub const bt_end: blockType_t = 3;
pub const bt_rle: blockType_t = 2;
pub const bt_raw: blockType_t = 1;
pub const bt_compressed: blockType_t = 0;
pub type HUFv07_DTable = U32;
pub type FSEv07_DTable = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct blockProperties_t {
    pub blockType: blockType_t,
    pub origSize: U32,
}
pub const ZSTD_error_maxCode: ERR_enum = 120;
pub const ZSTD_error_GENERIC: ERR_enum = 1;
pub const ZSTD_error_dstSize_tooSmall: ERR_enum = 70;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seqState_t {
    pub DStream: BITv07_DStream_t,
    pub stateLL: FSEv07_DState_t,
    pub stateOffb: FSEv07_DState_t,
    pub stateML: FSEv07_DState_t,
    pub prevOffset: [size_t; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSEv07_DState_t {
    pub state: size_t,
    pub table: *const ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BITv07_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: ::core::ffi::c_uint,
    pub ptr: *const ::core::ffi::c_char,
    pub start: *const ::core::ffi::c_char,
}
pub const ZSTD_error_corruption_detected: ERR_enum = 20;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seq_t {
    pub litLength: size_t,
    pub matchLength: size_t,
    pub offset: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSEv07_decode_t {
    pub newState: ::core::ffi::c_ushort,
    pub symbol: ::core::ffi::c_uchar,
    pub nbBits: ::core::ffi::c_uchar,
}
pub type BITv07_DStream_status = ::core::ffi::c_uint;
pub const BITv07_DStream_overflow: BITv07_DStream_status = 3;
pub const BITv07_DStream_completed: BITv07_DStream_status = 2;
pub const BITv07_DStream_endOfBuffer: BITv07_DStream_status = 1;
pub const BITv07_DStream_unfinished: BITv07_DStream_status = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSEv07_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}
pub type S16 = int16_t;
pub const ZSTD_error_maxSymbolValue_tooSmall: ERR_enum = 48;
pub const ZSTD_error_tableLog_tooLarge: ERR_enum = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: ERR_enum = 46;
pub const lbt_rle: litBlockType_t = 3;
pub const lbt_raw: litBlockType_t = 2;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DTableDesc {
    pub maxTableLog: BYTE,
    pub tableType: BYTE,
    pub tableLog: BYTE,
    pub reserved: BYTE,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUFv07_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}
pub const ZSTD_error_dictionary_corrupted: ERR_enum = 30;
pub const lbt_repeat: litBlockType_t = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUFv07_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}
pub type DTable_max_t = [U32; 4097];
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const HUFv07_static_assert: C2RustUnnamed_0 = 1;
pub type rankVal_t = [[U32; 17]; 16];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const HUFv07_static_assert_0: C2RustUnnamed_1 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}
pub const lbt_huffman: litBlockType_t = 0;
pub type litBlockType_t = ::core::ffi::c_uint;
pub const ZSTD_error_dictionary_wrong: ERR_enum = 32;
pub const ZSTD_error_memory_allocation: ERR_enum = 64;
pub type ZSTD_ErrorCode = ERR_enum;
pub type ERR_enum = ::core::ffi::c_uint;
pub const ZSTD_error_externalSequences_invalid: ERR_enum = 107;
pub const ZSTD_error_sequenceProducer_failed: ERR_enum = 106;
pub const ZSTD_error_srcBuffer_wrong: ERR_enum = 105;
pub const ZSTD_error_dstBuffer_wrong: ERR_enum = 104;
pub const ZSTD_error_seekableIO: ERR_enum = 102;
pub const ZSTD_error_frameIndex_tooLarge: ERR_enum = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: ERR_enum = 82;
pub const ZSTD_error_noForwardProgress_destFull: ERR_enum = 80;
pub const ZSTD_error_dstBuffer_null: ERR_enum = 74;
pub const ZSTD_error_workSpace_tooSmall: ERR_enum = 66;
pub const ZSTD_error_init_missing: ERR_enum = 62;
pub const ZSTD_error_stage_wrong: ERR_enum = 60;
pub const ZSTD_error_stabilityCondition_notRespected: ERR_enum = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: ERR_enum = 49;
pub const ZSTD_error_parameter_outOfBound: ERR_enum = 42;
pub const ZSTD_error_parameter_combination_unsupported: ERR_enum = 41;
pub const ZSTD_error_parameter_unsupported: ERR_enum = 40;
pub const ZSTD_error_dictionaryCreation_failed: ERR_enum = 34;
pub const ZSTD_error_literals_headerWrong: ERR_enum = 24;
pub const ZSTD_error_checksum_wrong: ERR_enum = 22;
pub const ZSTD_error_frameParameter_windowTooLarge: ERR_enum = 16;
pub const ZSTD_error_version_unsupported: ERR_enum = 12;
pub const ZSTD_error_no_error: ERR_enum = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv07_DDict_s {
    pub dict: *mut ::core::ffi::c_void,
    pub dictSize: size_t,
    pub refContext: *mut ZSTDv07_DCtx,
}
pub type ZSTDv07_DDict = ZSTDv07_DDict_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZBUFFv07_DCtx_s {
    pub zd: *mut ZSTDv07_DCtx,
    pub fParams: ZSTDv07_frameParams,
    pub stage: ZBUFFv07_dStage,
    pub inBuff: *mut ::core::ffi::c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub outBuff: *mut ::core::ffi::c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub blockSize: size_t,
    pub headerBuffer: [BYTE; 18],
    pub lhSize: size_t,
    pub customMem: ZSTDv07_customMem,
}
pub type ZBUFFv07_dStage = ::core::ffi::c_uint;
pub const ZBUFFds_flush: ZBUFFv07_dStage = 4;
pub const ZBUFFds_load: ZBUFFv07_dStage = 3;
pub const ZBUFFds_read: ZBUFFv07_dStage = 2;
pub const ZBUFFds_loadHeader: ZBUFFv07_dStage = 1;
pub const ZBUFFds_init: ZBUFFv07_dStage = 0;
pub type ZBUFFv07_DCtx = ZBUFFv07_DCtx_s;
pub type decompressionAlgo = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        size_t,
        *const ::core::ffi::c_void,
        size_t,
    ) -> size_t,
>;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ZSTDv07_MAGICNUMBER: ::core::ffi::c_uint = 0xfd2fb527 as ::core::ffi::c_uint;
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
pub const ZSTDv07_MAGIC_SKIPPABLE_START: ::core::ffi::c_uint = 0x184d2a50 as ::core::ffi::c_uint;
pub const ZSTDv07_WINDOWLOG_MAX_32: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const ZSTDv07_WINDOWLOG_MAX_64: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const ZSTDv07_FRAMEHEADERSIZE_MAX: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
static mut ZSTDv07_frameHeaderSize_min: size_t = 5 as size_t;
static mut ZSTDv07_frameHeaderSize_max: size_t = ZSTDv07_FRAMEHEADERSIZE_MAX as size_t;
static mut ZSTDv07_skippableHeaderSize: size_t = 8 as size_t;
pub const ZSTDv07_BLOCKSIZE_ABSOLUTEMAX: ::core::ffi::c_int =
    128 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int;
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
    let one: C2RustUnnamed = C2RustUnnamed {
        u: 1 as ::core::ffi::c_int as U32,
    };
    return one.c[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_read16(mut memPtr: *const ::core::ffi::c_void) -> U16 {
    let mut val: U16 = 0;
    memcpy(
        &raw mut val as *mut ::core::ffi::c_void,
        memPtr,
        ::core::mem::size_of::<U16>() as size_t,
    );
    return val;
}
#[inline]
unsafe extern "C" fn MEM_read32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    let mut val: U32 = 0;
    memcpy(
        &raw mut val as *mut ::core::ffi::c_void,
        memPtr,
        ::core::mem::size_of::<U32>() as size_t,
    );
    return val;
}
#[inline]
unsafe extern "C" fn MEM_read64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    let mut val: U64 = 0;
    memcpy(
        &raw mut val as *mut ::core::ffi::c_void,
        memPtr,
        ::core::mem::size_of::<U64>() as size_t,
    );
    return val;
}
#[inline]
unsafe extern "C" fn MEM_write16(mut memPtr: *mut ::core::ffi::c_void, mut value: U16) {
    memcpy(
        memPtr,
        &raw mut value as *const ::core::ffi::c_void,
        ::core::mem::size_of::<U16>() as size_t,
    );
}
#[inline]
unsafe extern "C" fn MEM_swap32(mut in_0: U32) -> U32 {
    return in_0 << 24 as ::core::ffi::c_int & 0xff000000 as U32
        | in_0 << 8 as ::core::ffi::c_int & 0xff0000 as U32
        | in_0 >> 8 as ::core::ffi::c_int & 0xff00 as U32
        | in_0 >> 24 as ::core::ffi::c_int & 0xff as U32;
}
#[inline]
unsafe extern "C" fn MEM_swap64(mut in_0: U64) -> U64 {
    return ((in_0 << 56 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
        & 0xff00000000000000 as ::core::ffi::c_ulonglong
        | (in_0 << 40 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff000000000000 as ::core::ffi::c_ulonglong
        | (in_0 << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff0000000000 as ::core::ffi::c_ulonglong
        | (in_0 << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff00000000 as ::core::ffi::c_ulonglong
        | (in_0 >> 8 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff000000 as ::core::ffi::c_ulonglong
        | (in_0 >> 24 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff0000 as ::core::ffi::c_ulonglong
        | (in_0 >> 40 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff00 as ::core::ffi::c_ulonglong
        | (in_0 >> 56 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff as ::core::ffi::c_ulonglong) as U64;
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
#[inline]
unsafe extern "C" fn BITv07_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (val.leading_zeros() as i32 ^ 31 as ::core::ffi::c_int) as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn BITv07_initDStream(
    mut bitD: *mut BITv07_DStream_t,
    mut srcBuffer: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize < 1 as size_t {
        memset(
            bitD as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<BITv07_DStream_t>() as size_t,
        );
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if srcSize >= ::core::mem::size_of::<size_t>() as usize {
        (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
        (*bitD).ptr = (srcBuffer as *const ::core::ffi::c_char)
            .offset(srcSize as isize)
            .offset(-(::core::mem::size_of::<size_t>() as usize as isize));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
        let lastByte: BYTE =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize);
        (*bitD).bitsConsumed = if lastByte as ::core::ffi::c_int != 0 {
            (8 as ::core::ffi::c_uint).wrapping_sub(BITv07_highbit32(lastByte as U32))
        } else {
            0 as ::core::ffi::c_uint
        };
        if lastByte as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
    } else {
        (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let mut current_block_20: u64;
        match srcSize {
            7 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(6 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(16 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_20 = 13577794887937023580;
            }
            6 => {
                current_block_20 = 13577794887937023580;
            }
            5 => {
                current_block_20 = 16111697992709662000;
            }
            4 => {
                current_block_20 = 8758779949906403879;
            }
            3 => {
                current_block_20 = 13626997879930997636;
            }
            2 => {
                current_block_20 = 17408806085946970511;
            }
            _ => {
                current_block_20 = 5689001924483802034;
            }
        }
        match current_block_20 {
            13577794887937023580 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(5 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(24 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_20 = 16111697992709662000;
            }
            _ => {}
        }
        match current_block_20 {
            16111697992709662000 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(4 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(32 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_20 = 8758779949906403879;
            }
            _ => {}
        }
        match current_block_20 {
            8758779949906403879 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(3 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_20 = 13626997879930997636;
            }
            _ => {}
        }
        match current_block_20 {
            13626997879930997636 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_20 = 17408806085946970511;
            }
            _ => {}
        }
        match current_block_20 {
            17408806085946970511 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(1 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
            }
            _ => {}
        }
        let lastByte_0: BYTE =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize);
        (*bitD).bitsConsumed = if lastByte_0 as ::core::ffi::c_int != 0 {
            (8 as ::core::ffi::c_uint).wrapping_sub(BITv07_highbit32(lastByte_0 as U32))
        } else {
            0 as ::core::ffi::c_uint
        };
        if lastByte_0 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((::core::mem::size_of::<size_t>() as usize).wrapping_sub(srcSize as usize) as U32)
                .wrapping_mul(8 as U32) as ::core::ffi::c_uint,
        );
    }
    return srcSize;
}
#[inline]
unsafe extern "C" fn BITv07_lookBits(mut bitD: *const BITv07_DStream_t, mut nbBits: U32) -> size_t {
    let bitMask: U32 = (::core::mem::size_of::<size_t>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & bitMask)
        >> 1 as ::core::ffi::c_int
        >> (bitMask.wrapping_sub(nbBits) & bitMask);
}
#[inline]
unsafe extern "C" fn BITv07_lookBitsFast(
    mut bitD: *const BITv07_DStream_t,
    mut nbBits: U32,
) -> size_t {
    let bitMask: U32 = (::core::mem::size_of::<size_t>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & bitMask)
        >> (bitMask.wrapping_add(1 as U32).wrapping_sub(nbBits) & bitMask);
}
#[inline]
unsafe extern "C" fn BITv07_skipBits(mut bitD: *mut BITv07_DStream_t, mut nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_add(nbBits as ::core::ffi::c_uint);
}
#[inline]
unsafe extern "C" fn BITv07_readBits(mut bitD: *mut BITv07_DStream_t, mut nbBits: U32) -> size_t {
    let value: size_t = BITv07_lookBits(bitD, nbBits) as size_t;
    BITv07_skipBits(bitD, nbBits);
    return value;
}
#[inline]
unsafe extern "C" fn BITv07_readBitsFast(
    mut bitD: *mut BITv07_DStream_t,
    mut nbBits: U32,
) -> size_t {
    let value: size_t = BITv07_lookBitsFast(bitD, nbBits) as size_t;
    BITv07_skipBits(bitD, nbBits);
    return value;
}
#[inline]
unsafe extern "C" fn BITv07_reloadDStream(
    mut bitD: *mut BITv07_DStream_t,
) -> BITv07_DStream_status {
    if (*bitD).bitsConsumed as usize
        > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
    {
        return BITv07_DStream_overflow;
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
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
        return BITv07_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize)
            < (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            return BITv07_DStream_endOfBuffer;
        }
        return BITv07_DStream_completed;
    }
    let mut nbBytes: U32 = (*bitD).bitsConsumed as U32 >> 3 as ::core::ffi::c_int;
    let mut result: BITv07_DStream_status = BITv07_DStream_unfinished;
    if (*bitD).ptr.offset(-(nbBytes as isize)) < (*bitD).start {
        nbBytes = (*bitD).ptr.offset_from((*bitD).start) as ::core::ffi::c_long as U32;
        result = BITv07_DStream_endOfBuffer;
    }
    (*bitD).ptr = (*bitD).ptr.offset(-(nbBytes as isize));
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_sub(nbBytes.wrapping_mul(8 as U32) as ::core::ffi::c_uint);
    (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
    return result;
}
#[inline]
unsafe extern "C" fn BITv07_endOfDStream(
    mut DStream: *const BITv07_DStream_t,
) -> ::core::ffi::c_uint {
    return ((*DStream).ptr == (*DStream).start
        && (*DStream).bitsConsumed as usize
            == (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize))
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn FSEv07_initDState(
    mut DStatePtr: *mut FSEv07_DState_t,
    mut bitD: *mut BITv07_DStream_t,
    mut dt: *const FSEv07_DTable,
) {
    let mut ptr: *const ::core::ffi::c_void = dt as *const ::core::ffi::c_void;
    let DTableH: *const FSEv07_DTableHeader = ptr as *const FSEv07_DTableHeader;
    (*DStatePtr).state = BITv07_readBits(bitD, (*DTableH).tableLog as U32);
    BITv07_reloadDStream(bitD);
    (*DStatePtr).table = dt.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
}
#[inline]
unsafe extern "C" fn FSEv07_peekSymbol(mut DStatePtr: *const FSEv07_DState_t) -> BYTE {
    let DInfo: FSEv07_decode_t =
        *((*DStatePtr).table as *const FSEv07_decode_t).offset((*DStatePtr).state as isize);
    return DInfo.symbol as BYTE;
}
#[inline]
unsafe extern "C" fn FSEv07_updateState(
    mut DStatePtr: *mut FSEv07_DState_t,
    mut bitD: *mut BITv07_DStream_t,
) {
    let DInfo: FSEv07_decode_t =
        *((*DStatePtr).table as *const FSEv07_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits: size_t = BITv07_readBits(bitD, nbBits) as size_t;
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
}
#[inline]
unsafe extern "C" fn FSEv07_decodeSymbol(
    mut DStatePtr: *mut FSEv07_DState_t,
    mut bitD: *mut BITv07_DStream_t,
) -> ::core::ffi::c_uchar {
    let DInfo: FSEv07_decode_t =
        *((*DStatePtr).table as *const FSEv07_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol as BYTE;
    let lowBits: size_t = BITv07_readBits(bitD, nbBits) as size_t;
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol as ::core::ffi::c_uchar;
}
#[inline]
unsafe extern "C" fn FSEv07_decodeSymbolFast(
    mut DStatePtr: *mut FSEv07_DState_t,
    mut bitD: *mut BITv07_DStream_t,
) -> ::core::ffi::c_uchar {
    let DInfo: FSEv07_decode_t =
        *((*DStatePtr).table as *const FSEv07_decode_t).offset((*DStatePtr).state as isize);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol as BYTE;
    let lowBits: size_t = BITv07_readBitsFast(bitD, nbBits) as size_t;
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol as ::core::ffi::c_uchar;
}
pub const FSEv07_MAX_MEMORY_USAGE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const FSEv07_MAX_SYMBOL_VALUE: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const FSEv07_MAX_TABLELOG: ::core::ffi::c_int =
    FSEv07_MAX_MEMORY_USAGE - 2 as ::core::ffi::c_int;
pub const FSEv07_MIN_TABLELOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const FSEv07_TABLELOG_ABSOLUTE_MAX: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const HUFv07_TABLELOG_ABSOLUTEMAX: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const HUFv07_TABLELOG_MAX: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const HUFv07_SYMBOLVALUE_MAX: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_getErrorName(mut code: size_t) -> *const ::core::ffi::c_char {
    return ERR_getErrorName(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_getErrorName(mut code: size_t) -> *const ::core::ffi::c_char {
    return ERR_getErrorName(code);
}
unsafe extern "C" fn FSEv07_abs(mut a: ::core::ffi::c_short) -> ::core::ffi::c_short {
    return (if (a as ::core::ffi::c_int) < 0 as ::core::ffi::c_int {
        -(a as ::core::ffi::c_int)
    } else {
        a as ::core::ffi::c_int
    }) as ::core::ffi::c_short;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_readNCount(
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
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void);
    nbBits =
        (bitStream & 0xf as U32).wrapping_add(FSEv07_MIN_TABLELOG as U32) as ::core::ffi::c_int;
    if nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
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
                    bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
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
                return -(ZSTD_error_maxSymbolValue_tooSmall as ::core::ffi::c_int) as size_t;
            }
            while charnum < n0 {
                let fresh7 = charnum;
                charnum = charnum.wrapping_add(1);
                *normalizedCounter.offset(fresh7 as isize) = 0 as ::core::ffi::c_short;
            }
            if ip <= iend.offset(-(7 as ::core::ffi::c_int as isize))
                || ip.offset((bitCount >> 3 as ::core::ffi::c_int) as isize)
                    <= iend.offset(-(4 as ::core::ffi::c_int as isize))
            {
                ip = ip.offset((bitCount >> 3 as ::core::ffi::c_int) as isize);
                bitCount &= 7 as ::core::ffi::c_int;
                bitStream = MEM_readLE32(ip as *const ::core::ffi::c_void) >> bitCount;
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
        remaining -= FSEv07_abs(count) as ::core::ffi::c_int;
        let fresh8 = charnum;
        charnum = charnum.wrapping_add(1);
        *normalizedCounter.offset(fresh8 as isize) = count;
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
            MEM_readLE32(ip as *const ::core::ffi::c_void) >> (bitCount & 31 as ::core::ffi::c_int);
    }
    if remaining != 1 as ::core::ffi::c_int {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    *maxSVPtr = charnum.wrapping_sub(1 as ::core::ffi::c_uint);
    ip = ip.offset((bitCount + 7 as ::core::ffi::c_int >> 3 as ::core::ffi::c_int) as isize);
    if ip.offset_from(istart) as ::core::ffi::c_long as size_t > hbSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readStats(
    mut huffWeight: *mut BYTE,
    mut hwSize: size_t,
    mut rankStats: *mut U32,
    mut nbSymbolsPtr: *mut U32,
    mut tableLogPtr: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
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
        if iSize >= 242 as size_t {
            static mut l: [U32; 14] = [
                1 as ::core::ffi::c_int as U32,
                2 as ::core::ffi::c_int as U32,
                3 as ::core::ffi::c_int as U32,
                4 as ::core::ffi::c_int as U32,
                7 as ::core::ffi::c_int as U32,
                8 as ::core::ffi::c_int as U32,
                15 as ::core::ffi::c_int as U32,
                16 as ::core::ffi::c_int as U32,
                31 as ::core::ffi::c_int as U32,
                32 as ::core::ffi::c_int as U32,
                63 as ::core::ffi::c_int as U32,
                64 as ::core::ffi::c_int as U32,
                127 as ::core::ffi::c_int as U32,
                128 as ::core::ffi::c_int as U32,
            ];
            oSize = l[iSize.wrapping_sub(242 as size_t) as usize] as size_t;
            memset(
                huffWeight as *mut ::core::ffi::c_void,
                1 as ::core::ffi::c_int,
                hwSize,
            );
            iSize = 0 as size_t;
        } else {
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
                *huffWeight.offset(n as isize) =
                    (*ip.offset(n.wrapping_div(2 as U32) as isize) as ::core::ffi::c_int
                        >> 4 as ::core::ffi::c_int) as BYTE;
                *huffWeight.offset(n.wrapping_add(1 as U32) as isize) =
                    (*ip.offset(n.wrapping_div(2 as U32) as isize) as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int) as BYTE;
                n = (n as ::core::ffi::c_uint).wrapping_add(2 as ::core::ffi::c_uint) as U32 as U32;
            }
        }
    } else {
        if iSize.wrapping_add(1 as size_t) > srcSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        oSize = FSEv07_decompress(
            huffWeight as *mut ::core::ffi::c_void,
            hwSize.wrapping_sub(1 as size_t),
            ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            iSize,
        );
        if FSEv07_isError(oSize) != 0 {
            return oSize;
        }
    }
    memset(
        rankStats as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ((HUFv07_TABLELOG_ABSOLUTEMAX + 1 as ::core::ffi::c_int) as size_t)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    );
    weightTotal = 0 as U32;
    let mut n_0: U32 = 0;
    n_0 = 0 as U32;
    while (n_0 as size_t) < oSize {
        if *huffWeight.offset(n_0 as isize) as ::core::ffi::c_int >= HUFv07_TABLELOG_ABSOLUTEMAX {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        let ref mut fresh33 = *rankStats.offset(*huffWeight.offset(n_0 as isize) as isize);
        *fresh33 = (*fresh33).wrapping_add(1);
        weightTotal = (weightTotal as ::core::ffi::c_uint).wrapping_add(
            ((1 as ::core::ffi::c_int) << *huffWeight.offset(n_0 as isize) as ::core::ffi::c_int
                >> 1 as ::core::ffi::c_int) as ::core::ffi::c_uint,
        ) as U32 as U32;
        n_0 = n_0.wrapping_add(1);
    }
    if weightTotal == 0 as U32 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let tableLog: U32 = (BITv07_highbit32(weightTotal) as U32).wrapping_add(1 as U32);
    if tableLog > HUFv07_TABLELOG_ABSOLUTEMAX as U32 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *tableLogPtr = tableLog;
    let total: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let rest: U32 = total.wrapping_sub(weightTotal);
    let verif: U32 = ((1 as ::core::ffi::c_int) << BITv07_highbit32(rest)) as U32;
    let lastWeight: U32 = (BITv07_highbit32(rest) as U32).wrapping_add(1 as U32);
    if verif != rest {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *huffWeight.offset(oSize as isize) = lastWeight as BYTE;
    let ref mut fresh34 = *rankStats.offset(lastWeight as isize);
    *fresh34 = (*fresh34).wrapping_add(1);
    if *rankStats.offset(1 as ::core::ffi::c_int as isize) < 2 as U32
        || *rankStats.offset(1 as ::core::ffi::c_int as isize) & 1 as U32 != 0
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *nbSymbolsPtr = oSize.wrapping_add(1 as size_t) as U32;
    return iSize.wrapping_add(1 as size_t);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_createDTable(
    mut tableLog: ::core::ffi::c_uint,
) -> *mut FSEv07_DTable {
    if tableLog > FSEv07_TABLELOG_ABSOLUTE_MAX as ::core::ffi::c_uint {
        tableLog = FSEv07_TABLELOG_ABSOLUTE_MAX as ::core::ffi::c_uint;
    }
    return malloc(
        ((1 as ::core::ffi::c_int + ((1 as ::core::ffi::c_int) << tableLog)) as size_t)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    ) as *mut FSEv07_DTable;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_freeDTable(mut dt: *mut FSEv07_DTable) {
    free(dt as *mut ::core::ffi::c_void);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable(
    mut dt: *mut FSEv07_DTable,
    mut normalizedCounter: *const ::core::ffi::c_short,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut tableLog: ::core::ffi::c_uint,
) -> size_t {
    let tdPtr: *mut ::core::ffi::c_void =
        dt.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let tableDecode: *mut FSEv07_decode_t = tdPtr as *mut FSEv07_decode_t;
    let mut symbolNext: [U16; 256] = [0; 256];
    let maxSV1: U32 = (maxSymbolValue as U32).wrapping_add(1 as U32);
    let tableSize: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1 as U32);
    if maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE as ::core::ffi::c_uint {
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog > FSEv07_MAX_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    let mut DTableH: FSEv07_DTableHeader = FSEv07_DTableHeader {
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
            let fresh9 = highThreshold;
            highThreshold = highThreshold.wrapping_sub(1);
            (*tableDecode.offset(fresh9 as isize)).symbol = s as BYTE as ::core::ffi::c_uchar;
            symbolNext[s as usize] = 1 as U16;
        } else {
            if *normalizedCounter.offset(s as isize) as ::core::ffi::c_int
                >= largeLimit as ::core::ffi::c_int
            {
                DTableH.fastMode = 0 as U16;
            }
            symbolNext[s as usize] = *normalizedCounter.offset(s as isize) as U16;
        }
        s = s.wrapping_add(1);
    }
    memcpy(
        dt as *mut ::core::ffi::c_void,
        &raw mut DTableH as *const ::core::ffi::c_void,
        ::core::mem::size_of::<FSEv07_DTableHeader>() as size_t,
    );
    let tableMask: U32 = tableSize.wrapping_sub(1 as U32);
    let step: U32 = (tableSize >> 1 as ::core::ffi::c_int)
        .wrapping_add(tableSize >> 3 as ::core::ffi::c_int)
        .wrapping_add(3 as U32);
    let mut s_0: U32 = 0;
    let mut position: U32 = 0 as U32;
    s_0 = 0 as U32;
    while s_0 < maxSV1 {
        let mut i: ::core::ffi::c_int = 0;
        i = 0 as ::core::ffi::c_int;
        while i < *normalizedCounter.offset(s_0 as isize) as ::core::ffi::c_int {
            (*tableDecode.offset(position as isize)).symbol = s_0 as BYTE as ::core::ffi::c_uchar;
            position = position.wrapping_add(step) & tableMask;
            while position > highThreshold {
                position = position.wrapping_add(step) & tableMask;
            }
            i += 1;
        }
        s_0 = s_0.wrapping_add(1);
    }
    if position != 0 as U32 {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    let mut u: U32 = 0;
    u = 0 as U32;
    while u < tableSize {
        let symbol: BYTE = (*tableDecode.offset(u as isize)).symbol as BYTE;
        let fresh10 = symbolNext[symbol as usize];
        symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
        let mut nextState: U16 = fresh10;
        (*tableDecode.offset(u as isize)).nbBits = tableLog
            .wrapping_sub(BITv07_highbit32(nextState as U32))
            as BYTE as ::core::ffi::c_uchar;
        (*tableDecode.offset(u as isize)).newState = (((nextState as ::core::ffi::c_int)
            << (*tableDecode.offset(u as isize)).nbBits as ::core::ffi::c_int)
            as U32)
            .wrapping_sub(tableSize) as U16
            as ::core::ffi::c_ushort;
        u = u.wrapping_add(1);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_rle(
    mut dt: *mut FSEv07_DTable,
    mut symbolValue: BYTE,
) -> size_t {
    let mut ptr: *mut ::core::ffi::c_void = dt as *mut ::core::ffi::c_void;
    let DTableH: *mut FSEv07_DTableHeader = ptr as *mut FSEv07_DTableHeader;
    let mut dPtr: *mut ::core::ffi::c_void =
        dt.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let cell: *mut FSEv07_decode_t = dPtr as *mut FSEv07_decode_t;
    (*DTableH).tableLog = 0 as U16;
    (*DTableH).fastMode = 0 as U16;
    (*cell).newState = 0 as ::core::ffi::c_ushort;
    (*cell).symbol = symbolValue as ::core::ffi::c_uchar;
    (*cell).nbBits = 0 as ::core::ffi::c_uchar;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_buildDTable_raw(
    mut dt: *mut FSEv07_DTable,
    mut nbBits: ::core::ffi::c_uint,
) -> size_t {
    let mut ptr: *mut ::core::ffi::c_void = dt as *mut ::core::ffi::c_void;
    let DTableH: *mut FSEv07_DTableHeader = ptr as *mut FSEv07_DTableHeader;
    let mut dPtr: *mut ::core::ffi::c_void =
        dt.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dinfo: *mut FSEv07_decode_t = dPtr as *mut FSEv07_decode_t;
    let tableSize: ::core::ffi::c_uint =
        ((1 as ::core::ffi::c_int) << nbBits) as ::core::ffi::c_uint;
    let tableMask: ::core::ffi::c_uint = tableSize.wrapping_sub(1 as ::core::ffi::c_uint);
    let maxSV1: ::core::ffi::c_uint = tableMask.wrapping_add(1 as ::core::ffi::c_uint);
    let mut s: ::core::ffi::c_uint = 0;
    if nbBits < 1 as ::core::ffi::c_uint {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    (*DTableH).tableLog = nbBits as U16;
    (*DTableH).fastMode = 1 as U16;
    s = 0 as ::core::ffi::c_uint;
    while s < maxSV1 {
        (*dinfo.offset(s as isize)).newState = 0 as ::core::ffi::c_ushort;
        (*dinfo.offset(s as isize)).symbol = s as BYTE as ::core::ffi::c_uchar;
        (*dinfo.offset(s as isize)).nbBits = nbBits as BYTE as ::core::ffi::c_uchar;
        s = s.wrapping_add(1);
    }
    return 0 as size_t;
}
#[inline(always)]
unsafe extern "C" fn FSEv07_decompress_usingDTable_generic(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut dt: *const FSEv07_DTable,
    fast: ::core::ffi::c_uint,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let omax: *mut BYTE = op.offset(maxDstSize as isize);
    let olimit: *mut BYTE = omax.offset(-(3 as ::core::ffi::c_int as isize));
    let mut bitD: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut state1: FSEv07_DState_t = FSEv07_DState_t {
        state: 0,
        table: ::core::ptr::null::<::core::ffi::c_void>(),
    };
    let mut state2: FSEv07_DState_t = FSEv07_DState_t {
        state: 0,
        table: ::core::ptr::null::<::core::ffi::c_void>(),
    };
    let errorCode: size_t = BITv07_initDStream(&raw mut bitD, cSrc, cSrcSize) as size_t;
    if ERR_isError(errorCode) != 0 {
        return errorCode;
    }
    FSEv07_initDState(&raw mut state1, &raw mut bitD, dt);
    FSEv07_initDState(&raw mut state2, &raw mut bitD, dt);
    while BITv07_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
        == BITv07_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && op < olimit
    {
        *op.offset(0 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSEv07_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSEv07_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSEv07_MAX_TABLELOG * 2 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            BITv07_reloadDStream(&raw mut bitD);
        }
        *op.offset(1 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSEv07_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSEv07_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSEv07_MAX_TABLELOG * 4 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            if BITv07_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
                > BITv07_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                op = op.offset(2 as ::core::ffi::c_int as isize);
                break;
            }
        }
        *op.offset(2 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSEv07_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSEv07_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSEv07_MAX_TABLELOG * 2 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            BITv07_reloadDStream(&raw mut bitD);
        }
        *op.offset(3 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSEv07_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSEv07_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        op = op.offset(4 as ::core::ffi::c_int as isize);
    }
    loop {
        if op > omax.offset(-(2 as ::core::ffi::c_int as isize)) {
            return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
        }
        let fresh35 = op;
        op = op.offset(1);
        *fresh35 = (if fast != 0 {
            FSEv07_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSEv07_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if BITv07_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
            == BITv07_DStream_overflow as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let fresh36 = op;
            op = op.offset(1);
            *fresh36 = (if fast != 0 {
                FSEv07_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            } else {
                FSEv07_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            }) as BYTE;
            break;
        } else {
            if op > omax.offset(-(2 as ::core::ffi::c_int as isize)) {
                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
            }
            let fresh37 = op;
            op = op.offset(1);
            *fresh37 = (if fast != 0 {
                FSEv07_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            } else {
                FSEv07_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
            }) as BYTE;
            if !(BITv07_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
                == BITv07_DStream_overflow as ::core::ffi::c_int as ::core::ffi::c_uint)
            {
                continue;
            }
            let fresh38 = op;
            op = op.offset(1);
            *fresh38 = (if fast != 0 {
                FSEv07_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
            } else {
                FSEv07_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
            }) as BYTE;
            break;
        }
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut originalSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut dt: *const FSEv07_DTable,
) -> size_t {
    let mut ptr: *const ::core::ffi::c_void = dt as *const ::core::ffi::c_void;
    let mut DTableH: *const FSEv07_DTableHeader = ptr as *const FSEv07_DTableHeader;
    let fastMode: U32 = (*DTableH).fastMode as U32;
    if fastMode != 0 {
        return FSEv07_decompress_usingDTable_generic(
            dst,
            originalSize,
            cSrc,
            cSrcSize,
            dt,
            1 as ::core::ffi::c_uint,
        );
    }
    return FSEv07_decompress_usingDTable_generic(
        dst,
        originalSize,
        cSrc,
        cSrcSize,
        dt,
        0 as ::core::ffi::c_uint,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSEv07_decompress(
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
    let mut maxSymbolValue: ::core::ffi::c_uint = FSEv07_MAX_SYMBOL_VALUE as ::core::ffi::c_uint;
    if cSrcSize < 2 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let NCountLength: size_t = FSEv07_readNCount(
        &raw mut counting as *mut ::core::ffi::c_short,
        &raw mut maxSymbolValue,
        &raw mut tableLog,
        istart as *const ::core::ffi::c_void,
        cSrcSize,
    ) as size_t;
    if ERR_isError(NCountLength) != 0 {
        return NCountLength;
    }
    if NCountLength >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(NCountLength as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(NCountLength as ::core::ffi::c_ulong)
        as size_t as size_t;
    let errorCode: size_t = FSEv07_buildDTable(
        &raw mut dt as *mut FSEv07_DTable,
        &raw mut counting as *mut ::core::ffi::c_short,
        maxSymbolValue,
        tableLog,
    ) as size_t;
    if ERR_isError(errorCode) != 0 {
        return errorCode;
    }
    return FSEv07_decompress_usingDTable(
        dst,
        maxDstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        &raw mut dt as *mut U32,
    );
}
unsafe extern "C" fn HUFv07_getDTableDesc(mut table: *const HUFv07_DTable) -> DTableDesc {
    let mut dtd: DTableDesc = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    memcpy(
        &raw mut dtd as *mut ::core::ffi::c_void,
        table as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DTableDesc>() as size_t,
    );
    return dtd;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX2(
    mut DTable: *mut HUFv07_DTable,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut huffWeight: [BYTE; 256] = [0; 256];
    let mut rankVal: [U32; 17] = [0; 17];
    let mut tableLog: U32 = 0 as U32;
    let mut nbSymbols: U32 = 0 as U32;
    let mut iSize: size_t = 0;
    let dtPtr: *mut ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dt: *mut HUFv07_DEltX2 = dtPtr as *mut HUFv07_DEltX2;
    iSize = HUFv07_readStats(
        &raw mut huffWeight as *mut BYTE,
        (HUFv07_SYMBOLVALUE_MAX + 1 as ::core::ffi::c_int) as size_t,
        &raw mut rankVal as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if tableLog > (dtd.maxTableLog as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    dtd.tableType = 0 as BYTE;
    dtd.tableLog = tableLog as BYTE;
    memcpy(
        DTable as *mut ::core::ffi::c_void,
        &raw mut dtd as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DTableDesc>() as size_t,
    );
    let mut n: U32 = 0;
    let mut nextRankStart: U32 = 0 as U32;
    n = 1 as U32;
    while n < tableLog.wrapping_add(1 as U32) {
        let mut current: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add((rankVal[n as usize] << n.wrapping_sub(1 as U32)) as ::core::ffi::c_uint)
            as U32 as U32;
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }
    let mut n_0: U32 = 0;
    n_0 = 0 as U32;
    while n_0 < nbSymbols {
        let w: U32 = huffWeight[n_0 as usize] as U32;
        let length: U32 = ((1 as ::core::ffi::c_int) << w >> 1 as ::core::ffi::c_int) as U32;
        let mut i: U32 = 0;
        let mut D: HUFv07_DEltX2 = HUFv07_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n_0 as BYTE;
        D.nbBits = tableLog.wrapping_add(1 as U32).wrapping_sub(w) as BYTE;
        i = rankVal[w as usize];
        while i < rankVal[w as usize].wrapping_add(length) {
            *dt.offset(i as isize) = D;
            i = i.wrapping_add(1);
        }
        rankVal[w as usize] = (rankVal[w as usize] as ::core::ffi::c_uint)
            .wrapping_add(length as ::core::ffi::c_uint) as U32
            as U32;
        n_0 = n_0.wrapping_add(1);
    }
    return iSize;
}
unsafe extern "C" fn HUFv07_decodeSymbolX2(
    mut Dstream: *mut BITv07_DStream_t,
    mut dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: size_t = BITv07_lookBitsFast(Dstream, dtLog) as size_t;
    let c: BYTE = (*dt.offset(val as isize)).byte;
    BITv07_skipBits(Dstream, (*dt.offset(val as isize)).nbBits as U32);
    return c;
}
#[inline]
unsafe extern "C" fn HUFv07_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;
    while BITv07_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BITv07_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p <= pEnd.offset(-(4 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            let fresh27 = p;
            p = p.offset(1);
            *fresh27 = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            let fresh28 = p;
            p = p.offset(1);
            *fresh28 = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh29 = p;
            p = p.offset(1);
            *fresh29 = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
        }
        let fresh30 = p;
        p = p.offset(1);
        *fresh30 = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
    }
    while BITv07_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BITv07_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p < pEnd
    {
        let fresh31 = p;
        p = p.offset(1);
        *fresh31 = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
    }
    while p < pEnd {
        let fresh32 = p;
        p = p.offset(1);
        *fresh32 = HUFv07_decodeSymbolX2(bitDPtr, dt, dtLog);
    }
    return pEnd.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn HUFv07_decompress1X2_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.offset(dstSize as isize);
    let mut dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUFv07_DEltX2 = dtPtr as *const HUFv07_DEltX2;
    let mut bitD: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable) as DTableDesc;
    let dtLog: U32 = dtd.tableLog as U32;
    let errorCode: size_t = BITv07_initDStream(&raw mut bitD, cSrc, cSrcSize) as size_t;
    if HUFv07_isError(errorCode) != 0 {
        return errorCode;
    }
    HUFv07_decodeStreamX2(op, &raw mut bitD, oend, dt, dtLog);
    if BITv07_endOfDStream(&raw mut bitD) == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    return HUFv07_decompress1X2_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2_DCtx(
    mut DCtx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let hSize: size_t = HUFv07_readDTableX2(DCtx, cSrc, cSrcSize) as size_t;
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUFv07_decompress1X2_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        DCtx,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X2(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable: [HUFv07_DTable; 2049] = [
        ((12 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as U32)
            .wrapping_mul(0x1000001 as U32),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    return HUFv07_decompress1X2_DCtx(
        &raw mut DTable as *mut HUFv07_DTable,
        dst,
        dstSize,
        cSrc,
        cSrcSize,
    );
}
unsafe extern "C" fn HUFv07_decompress4X2_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    if cSrcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUFv07_DEltX2 = dtPtr as *const HUFv07_DEltX2;
    let mut bitD1: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let length1: size_t = MEM_readLE16(istart as *const ::core::ffi::c_void) as size_t;
    let length2: size_t =
        MEM_readLE16(istart.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as size_t;
    let length3: size_t =
        MEM_readLE16(istart.offset(4 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as size_t;
    let length4: size_t = cSrcSize.wrapping_sub(
        length1
            .wrapping_add(length2)
            .wrapping_add(length3)
            .wrapping_add(6 as size_t),
    );
    let istart1: *const BYTE = istart.offset(6 as ::core::ffi::c_int as isize);
    let istart2: *const BYTE = istart1.offset(length1 as isize);
    let istart3: *const BYTE = istart2.offset(length2 as isize);
    let istart4: *const BYTE = istart3.offset(length3 as isize);
    let segmentSize: size_t = dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t);
    let opStart2: *mut BYTE = ostart.offset(segmentSize as isize);
    let opStart3: *mut BYTE = opStart2.offset(segmentSize as isize);
    let opStart4: *mut BYTE = opStart3.offset(segmentSize as isize);
    let mut op1: *mut BYTE = ostart;
    let mut op2: *mut BYTE = opStart2;
    let mut op3: *mut BYTE = opStart3;
    let mut op4: *mut BYTE = opStart4;
    let mut endSignal: U32 = 0;
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable) as DTableDesc;
    let dtLog: U32 = dtd.tableLog as U32;
    if length4 > cSrcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let errorCode: size_t = BITv07_initDStream(
        &raw mut bitD1,
        istart1 as *const ::core::ffi::c_void,
        length1,
    ) as size_t;
    if HUFv07_isError(errorCode) != 0 {
        return errorCode;
    }
    let errorCode_0: size_t = BITv07_initDStream(
        &raw mut bitD2,
        istart2 as *const ::core::ffi::c_void,
        length2,
    ) as size_t;
    if HUFv07_isError(errorCode_0) != 0 {
        return errorCode_0;
    }
    let errorCode_1: size_t = BITv07_initDStream(
        &raw mut bitD3,
        istart3 as *const ::core::ffi::c_void,
        length3,
    ) as size_t;
    if HUFv07_isError(errorCode_1) != 0 {
        return errorCode_1;
    }
    let errorCode_2: size_t = BITv07_initDStream(
        &raw mut bitD4,
        istart4 as *const ::core::ffi::c_void,
        length4,
    ) as size_t;
    if HUFv07_isError(errorCode_2) != 0 {
        return errorCode_2;
    }
    endSignal = (BITv07_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
        | BITv07_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
        | BITv07_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
        | BITv07_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint) as U32;
    while endSignal == BITv07_DStream_unfinished as ::core::ffi::c_int as U32
        && op4 < oend.offset(-(7 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            let fresh11 = op1;
            op1 = op1.offset(1);
            *fresh11 = HUFv07_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh12 = op2;
            op2 = op2.offset(1);
            *fresh12 = HUFv07_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh13 = op3;
            op3 = op3.offset(1);
            *fresh13 = HUFv07_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh14 = op4;
            op4 = op4.offset(1);
            *fresh14 = HUFv07_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            let fresh15 = op1;
            op1 = op1.offset(1);
            *fresh15 = HUFv07_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            let fresh16 = op2;
            op2 = op2.offset(1);
            *fresh16 = HUFv07_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            let fresh17 = op3;
            op3 = op3.offset(1);
            *fresh17 = HUFv07_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            let fresh18 = op4;
            op4 = op4.offset(1);
            *fresh18 = HUFv07_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh19 = op1;
            op1 = op1.offset(1);
            *fresh19 = HUFv07_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh20 = op2;
            op2 = op2.offset(1);
            *fresh20 = HUFv07_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh21 = op3;
            op3 = op3.offset(1);
            *fresh21 = HUFv07_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh22 = op4;
            op4 = op4.offset(1);
            *fresh22 = HUFv07_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        }
        let fresh23 = op1;
        op1 = op1.offset(1);
        *fresh23 = HUFv07_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        let fresh24 = op2;
        op2 = op2.offset(1);
        *fresh24 = HUFv07_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        let fresh25 = op3;
        op3 = op3.offset(1);
        *fresh25 = HUFv07_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        let fresh26 = op4;
        op4 = op4.offset(1);
        *fresh26 = HUFv07_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        endSignal = (BITv07_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
            | BITv07_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
            | BITv07_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
            | BITv07_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint)
            as U32;
    }
    if op1 > opStart2 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if op2 > opStart3 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if op3 > opStart4 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    HUFv07_decodeStreamX2(op1, &raw mut bitD1, opStart2, dt, dtLog);
    HUFv07_decodeStreamX2(op2, &raw mut bitD2, opStart3, dt, dtLog);
    HUFv07_decodeStreamX2(op3, &raw mut bitD3, opStart4, dt, dtLog);
    HUFv07_decodeStreamX2(op4, &raw mut bitD4, oend, dt, dtLog);
    endSignal = (BITv07_endOfDStream(&raw mut bitD1)
        & BITv07_endOfDStream(&raw mut bitD2)
        & BITv07_endOfDStream(&raw mut bitD3)
        & BITv07_endOfDStream(&raw mut bitD4)) as U32;
    if endSignal == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    return HUFv07_decompress4X2_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2_DCtx(
    mut dctx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let hSize: size_t = HUFv07_readDTableX2(dctx, cSrc, cSrcSize) as size_t;
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUFv07_decompress4X2_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        dctx,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X2(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable: [HUFv07_DTable; 2049] = [
        ((12 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as U32)
            .wrapping_mul(0x1000001 as U32),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    return HUFv07_decompress4X2_DCtx(
        &raw mut DTable as *mut HUFv07_DTable,
        dst,
        dstSize,
        cSrc,
        cSrcSize,
    );
}
unsafe extern "C" fn HUFv07_fillDTableX4Level2(
    mut DTable: *mut HUFv07_DEltX4,
    mut sizeLog: U32,
    consumed: U32,
    mut rankValOrigin: *const U32,
    minWeight: ::core::ffi::c_int,
    mut sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    mut nbBitsBaseline: U32,
    mut baseSeq: U16,
) {
    let mut DElt: HUFv07_DEltX4 = HUFv07_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; 17] = [0; 17];
    memcpy(
        &raw mut rankVal as *mut U32 as *mut ::core::ffi::c_void,
        rankValOrigin as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 17]>() as size_t,
    );
    if minWeight > 1 as ::core::ffi::c_int {
        let mut i: U32 = 0;
        let mut skipSize: U32 = rankVal[minWeight as usize];
        MEM_writeLE16(&raw mut DElt.sequence as *mut ::core::ffi::c_void, baseSeq);
        DElt.nbBits = consumed as BYTE;
        DElt.length = 1 as BYTE;
        i = 0 as U32;
        while i < skipSize {
            *DTable.offset(i as isize) = DElt;
            i = i.wrapping_add(1);
        }
    }
    let mut s: U32 = 0;
    s = 0 as U32;
    while s < sortedListSize {
        let symbol: U32 = (*sortedSymbols.offset(s as isize)).symbol as U32;
        let weight: U32 = (*sortedSymbols.offset(s as isize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let length: U32 = ((1 as ::core::ffi::c_int) << sizeLog.wrapping_sub(nbBits)) as U32;
        let start: U32 = rankVal[weight as usize];
        let mut i_0: U32 = start;
        let end: U32 = start.wrapping_add(length);
        MEM_writeLE16(
            &raw mut DElt.sequence as *mut ::core::ffi::c_void,
            (baseSeq as U32).wrapping_add(symbol << 8 as ::core::ffi::c_int) as U16,
        );
        DElt.nbBits = nbBits.wrapping_add(consumed) as BYTE;
        DElt.length = 2 as BYTE;
        loop {
            let fresh41 = i_0;
            i_0 = i_0.wrapping_add(1);
            *DTable.offset(fresh41 as isize) = DElt;
            if !(i_0 < end) {
                break;
            }
        }
        rankVal[weight as usize] = (rankVal[weight as usize] as ::core::ffi::c_uint)
            .wrapping_add(length as ::core::ffi::c_uint) as U32
            as U32;
        s = s.wrapping_add(1);
    }
}
unsafe extern "C" fn HUFv07_fillDTableX4(
    mut DTable: *mut HUFv07_DEltX4,
    targetLog: U32,
    mut sortedList: *const sortedSymbol_t,
    sortedListSize: U32,
    mut rankStart: *const U32,
    mut rankValOrigin: *mut [U32; 17],
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let mut rankVal: [U32; 17] = [0; 17];
    let scaleLog: ::core::ffi::c_int = nbBitsBaseline.wrapping_sub(targetLog) as ::core::ffi::c_int;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut s: U32 = 0;
    memcpy(
        &raw mut rankVal as *mut U32 as *mut ::core::ffi::c_void,
        rankValOrigin as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 17]>() as size_t,
    );
    s = 0 as U32;
    while s < sortedListSize {
        let symbol: U16 = (*sortedList.offset(s as isize)).symbol as U16;
        let weight: U32 = (*sortedList.offset(s as isize)).weight as U32;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(weight);
        let start: U32 = rankVal[weight as usize];
        let length: U32 = ((1 as ::core::ffi::c_int) << targetLog.wrapping_sub(nbBits)) as U32;
        if targetLog.wrapping_sub(nbBits) >= minBits {
            let mut sortedRank: U32 = 0;
            let mut minWeight: ::core::ffi::c_int =
                nbBits.wrapping_add(scaleLog as U32) as ::core::ffi::c_int;
            if minWeight < 1 as ::core::ffi::c_int {
                minWeight = 1 as ::core::ffi::c_int;
            }
            sortedRank = *rankStart.offset(minWeight as isize);
            HUFv07_fillDTableX4Level2(
                DTable.offset(start as isize),
                targetLog.wrapping_sub(nbBits),
                nbBits,
                &raw mut *rankValOrigin.offset(nbBits as isize) as *mut U32,
                minWeight,
                sortedList.offset(sortedRank as isize),
                sortedListSize.wrapping_sub(sortedRank),
                nbBitsBaseline,
                symbol,
            );
        } else {
            let mut DElt: HUFv07_DEltX4 = HUFv07_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };
            MEM_writeLE16(&raw mut DElt.sequence as *mut ::core::ffi::c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1 as BYTE;
            let mut u: U32 = 0;
            let end: U32 = start.wrapping_add(length);
            u = start;
            while u < end {
                *DTable.offset(u as isize) = DElt;
                u = u.wrapping_add(1);
            }
        }
        rankVal[weight as usize] = (rankVal[weight as usize] as ::core::ffi::c_uint)
            .wrapping_add(length as ::core::ffi::c_uint) as U32
            as U32;
        s = s.wrapping_add(1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_readDTableX4(
    mut DTable: *mut HUFv07_DTable,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut weightList: [BYTE; 256] = [0; 256];
    let mut sortedSymbol: [sortedSymbol_t; 256] = [sortedSymbol_t {
        symbol: 0,
        weight: 0,
    }; 256];
    let mut rankStats: [U32; 17] = [
        0 as ::core::ffi::c_int as U32,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    let mut rankStart0: [U32; 18] = [
        0 as ::core::ffi::c_int as U32,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    let rankStart: *mut U32 =
        (&raw mut rankStart0 as *mut U32).offset(1 as ::core::ffi::c_int as isize);
    let mut rankVal: rankVal_t = [[0; 17]; 16];
    let mut tableLog: U32 = 0;
    let mut maxW: U32 = 0;
    let mut sizeOfSort: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    let maxTableLog: U32 = dtd.maxTableLog as U32;
    let mut iSize: size_t = 0;
    let mut dtPtr: *mut ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dt: *mut HUFv07_DEltX4 = dtPtr as *mut HUFv07_DEltX4;
    if maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    iSize = HUFv07_readStats(
        &raw mut weightList as *mut BYTE,
        (HUFv07_SYMBOLVALUE_MAX + 1 as ::core::ffi::c_int) as size_t,
        &raw mut rankStats as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
    );
    if HUFv07_isError(iSize) != 0 {
        return iSize;
    }
    if tableLog > maxTableLog {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 as U32 {
        maxW = maxW.wrapping_sub(1);
    }
    let mut w: U32 = 0;
    let mut nextRankStart: U32 = 0 as U32;
    w = 1 as U32;
    while w < maxW.wrapping_add(1 as U32) {
        let mut current: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add(rankStats[w as usize] as ::core::ffi::c_uint)
            as U32 as U32;
        *rankStart.offset(w as isize) = current;
        w = w.wrapping_add(1);
    }
    *rankStart.offset(0 as ::core::ffi::c_int as isize) = nextRankStart;
    sizeOfSort = nextRankStart;
    let mut s: U32 = 0;
    s = 0 as U32;
    while s < nbSymbols {
        let w_0: U32 = weightList[s as usize] as U32;
        let ref mut fresh39 = *rankStart.offset(w_0 as isize);
        let fresh40 = *fresh39;
        *fresh39 = (*fresh39).wrapping_add(1);
        let r: U32 = fresh40;
        sortedSymbol[r as usize].symbol = s as BYTE;
        sortedSymbol[r as usize].weight = w_0 as BYTE;
        s = s.wrapping_add(1);
    }
    *rankStart.offset(0 as ::core::ffi::c_int as isize) = 0 as U32;
    let rankVal0: *mut U32 = &raw mut *(&raw mut rankVal as *mut [U32; 17])
        .offset(0 as ::core::ffi::c_int as isize) as *mut U32;
    let rescale: ::core::ffi::c_int =
        maxTableLog.wrapping_sub(tableLog).wrapping_sub(1 as U32) as ::core::ffi::c_int;
    let mut nextRankVal: U32 = 0 as U32;
    let mut w_1: U32 = 0;
    w_1 = 1 as U32;
    while w_1 < maxW.wrapping_add(1 as U32) {
        let mut current_0: U32 = nextRankVal;
        nextRankVal = (nextRankVal as ::core::ffi::c_uint).wrapping_add(
            (rankStats[w_1 as usize] << w_1.wrapping_add(rescale as U32)) as ::core::ffi::c_uint,
        ) as U32 as U32;
        *rankVal0.offset(w_1 as isize) = current_0;
        w_1 = w_1.wrapping_add(1);
    }
    let minBits: U32 = tableLog.wrapping_add(1 as U32).wrapping_sub(maxW);
    let mut consumed: U32 = 0;
    consumed = minBits;
    while consumed < maxTableLog.wrapping_sub(minBits).wrapping_add(1 as U32) {
        let rankValPtr: *mut U32 =
            &raw mut *(&raw mut rankVal as *mut [U32; 17]).offset(consumed as isize) as *mut U32;
        let mut w_2: U32 = 0;
        w_2 = 1 as U32;
        while w_2 < maxW.wrapping_add(1 as U32) {
            *rankValPtr.offset(w_2 as isize) = *rankVal0.offset(w_2 as isize) >> consumed;
            w_2 = w_2.wrapping_add(1);
        }
        consumed = consumed.wrapping_add(1);
    }
    HUFv07_fillDTableX4(
        dt,
        maxTableLog,
        &raw mut sortedSymbol as *mut sortedSymbol_t,
        sizeOfSort,
        &raw mut rankStart0 as *mut U32,
        &raw mut rankVal as *mut [U32; 17],
        maxW,
        tableLog.wrapping_add(1 as U32),
    );
    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1 as BYTE;
    memcpy(
        DTable as *mut ::core::ffi::c_void,
        &raw mut dtd as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DTableDesc>() as size_t,
    );
    return iSize;
}
unsafe extern "C" fn HUFv07_decodeSymbolX4(
    mut op: *mut ::core::ffi::c_void,
    mut DStream: *mut BITv07_DStream_t,
    mut dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: size_t = BITv07_lookBitsFast(DStream, dtLog) as size_t;
    memcpy(
        op,
        dt.offset(val as isize) as *const ::core::ffi::c_void,
        2 as size_t,
    );
    BITv07_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
    return (*dt.offset(val as isize)).length as U32;
}
unsafe extern "C" fn HUFv07_decodeLastSymbolX4(
    mut op: *mut ::core::ffi::c_void,
    mut DStream: *mut BITv07_DStream_t,
    mut dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: size_t = BITv07_lookBitsFast(DStream, dtLog) as size_t;
    memcpy(
        op,
        dt.offset(val as isize) as *const ::core::ffi::c_void,
        1 as size_t,
    );
    if (*dt.offset(val as isize)).length as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
        BITv07_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
    } else if ((*DStream).bitsConsumed as usize)
        < (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
    {
        BITv07_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
        if (*DStream).bitsConsumed as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
        {
            (*DStream).bitsConsumed = (::core::mem::size_of::<size_t>() as usize)
                .wrapping_mul(8 as usize)
                as ::core::ffi::c_uint;
        }
    }
    return 1 as U32;
}
#[inline]
unsafe extern "C" fn HUFv07_decodeStreamX4(
    mut p: *mut BYTE,
    mut bitDPtr: *mut BITv07_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUFv07_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;
    while BITv07_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BITv07_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p < pEnd.offset(-(7 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            p = p.offset(
                HUFv07_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            p = p.offset(
                HUFv07_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        if MEM_64bits() != 0 {
            p = p.offset(
                HUFv07_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        p = p.offset(
            HUFv07_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
        );
    }
    while BITv07_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BITv07_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p <= pEnd.offset(-(2 as ::core::ffi::c_int as isize))
    {
        p = p.offset(
            HUFv07_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
        );
    }
    while p <= pEnd.offset(-(2 as ::core::ffi::c_int as isize)) {
        p = p.offset(
            HUFv07_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
        );
    }
    if p < pEnd {
        p = p.offset(
            HUFv07_decodeLastSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
        );
    }
    return p.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn HUFv07_decompress1X4_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let mut bitD: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let errorCode: size_t = BITv07_initDStream(&raw mut bitD, cSrc, cSrcSize) as size_t;
    if HUFv07_isError(errorCode) != 0 {
        return errorCode;
    }
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUFv07_DEltX4 = dtPtr as *const HUFv07_DEltX4;
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable) as DTableDesc;
    HUFv07_decodeStreamX4(ostart, &raw mut bitD, oend, dt, dtd.tableLog as U32);
    if BITv07_endOfDStream(&raw mut bitD) == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType as ::core::ffi::c_int != 1 as ::core::ffi::c_int {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    return HUFv07_decompress1X4_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4_DCtx(
    mut DCtx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let hSize: size_t = HUFv07_readDTableX4(DCtx, cSrc, cSrcSize) as size_t;
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUFv07_decompress1X4_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        DCtx,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X4(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable: [HUFv07_DTable; 4097] = [
        (12 as ::core::ffi::c_int as U32).wrapping_mul(0x1000001 as U32),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    return HUFv07_decompress1X4_DCtx(
        &raw mut DTable as *mut HUFv07_DTable,
        dst,
        dstSize,
        cSrc,
        cSrcSize,
    );
}
unsafe extern "C" fn HUFv07_decompress4X4_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    if cSrcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUFv07_DEltX4 = dtPtr as *const HUFv07_DEltX4;
    let mut bitD1: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: BITv07_DStream_t = BITv07_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let length1: size_t = MEM_readLE16(istart as *const ::core::ffi::c_void) as size_t;
    let length2: size_t =
        MEM_readLE16(istart.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as size_t;
    let length3: size_t =
        MEM_readLE16(istart.offset(4 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as size_t;
    let length4: size_t = cSrcSize.wrapping_sub(
        length1
            .wrapping_add(length2)
            .wrapping_add(length3)
            .wrapping_add(6 as size_t),
    );
    let istart1: *const BYTE = istart.offset(6 as ::core::ffi::c_int as isize);
    let istart2: *const BYTE = istart1.offset(length1 as isize);
    let istart3: *const BYTE = istart2.offset(length2 as isize);
    let istart4: *const BYTE = istart3.offset(length3 as isize);
    let segmentSize: size_t = dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t);
    let opStart2: *mut BYTE = ostart.offset(segmentSize as isize);
    let opStart3: *mut BYTE = opStart2.offset(segmentSize as isize);
    let opStart4: *mut BYTE = opStart3.offset(segmentSize as isize);
    let mut op1: *mut BYTE = ostart;
    let mut op2: *mut BYTE = opStart2;
    let mut op3: *mut BYTE = opStart3;
    let mut op4: *mut BYTE = opStart4;
    let mut endSignal: U32 = 0;
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable) as DTableDesc;
    let dtLog: U32 = dtd.tableLog as U32;
    if length4 > cSrcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let errorCode: size_t = BITv07_initDStream(
        &raw mut bitD1,
        istart1 as *const ::core::ffi::c_void,
        length1,
    ) as size_t;
    if HUFv07_isError(errorCode) != 0 {
        return errorCode;
    }
    let errorCode_0: size_t = BITv07_initDStream(
        &raw mut bitD2,
        istart2 as *const ::core::ffi::c_void,
        length2,
    ) as size_t;
    if HUFv07_isError(errorCode_0) != 0 {
        return errorCode_0;
    }
    let errorCode_1: size_t = BITv07_initDStream(
        &raw mut bitD3,
        istart3 as *const ::core::ffi::c_void,
        length3,
    ) as size_t;
    if HUFv07_isError(errorCode_1) != 0 {
        return errorCode_1;
    }
    let errorCode_2: size_t = BITv07_initDStream(
        &raw mut bitD4,
        istart4 as *const ::core::ffi::c_void,
        length4,
    ) as size_t;
    if HUFv07_isError(errorCode_2) != 0 {
        return errorCode_2;
    }
    endSignal = (BITv07_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
        | BITv07_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
        | BITv07_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
        | BITv07_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint) as U32;
    while endSignal == BITv07_DStream_unfinished as ::core::ffi::c_int as U32
        && op4 < oend.offset(-(7 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            op1 = op1.offset(HUFv07_decodeSymbolX4(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op2 = op2.offset(HUFv07_decodeSymbolX4(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op3 = op3.offset(HUFv07_decodeSymbolX4(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op4 = op4.offset(HUFv07_decodeSymbolX4(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            op1 = op1.offset(HUFv07_decodeSymbolX4(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            op2 = op2.offset(HUFv07_decodeSymbolX4(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            op3 = op3.offset(HUFv07_decodeSymbolX4(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUFv07_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
            op4 = op4.offset(HUFv07_decodeSymbolX4(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op1 = op1.offset(HUFv07_decodeSymbolX4(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op2 = op2.offset(HUFv07_decodeSymbolX4(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op3 = op3.offset(HUFv07_decodeSymbolX4(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op4 = op4.offset(HUFv07_decodeSymbolX4(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
        }
        op1 = op1.offset(HUFv07_decodeSymbolX4(
            op1 as *mut ::core::ffi::c_void,
            &raw mut bitD1,
            dt,
            dtLog,
        ) as isize);
        op2 = op2.offset(HUFv07_decodeSymbolX4(
            op2 as *mut ::core::ffi::c_void,
            &raw mut bitD2,
            dt,
            dtLog,
        ) as isize);
        op3 = op3.offset(HUFv07_decodeSymbolX4(
            op3 as *mut ::core::ffi::c_void,
            &raw mut bitD3,
            dt,
            dtLog,
        ) as isize);
        op4 = op4.offset(HUFv07_decodeSymbolX4(
            op4 as *mut ::core::ffi::c_void,
            &raw mut bitD4,
            dt,
            dtLog,
        ) as isize);
        endSignal = (BITv07_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
            | BITv07_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
            | BITv07_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
            | BITv07_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint)
            as U32;
    }
    if op1 > opStart2 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if op2 > opStart3 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if op3 > opStart4 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    HUFv07_decodeStreamX4(op1, &raw mut bitD1, opStart2, dt, dtLog);
    HUFv07_decodeStreamX4(op2, &raw mut bitD2, opStart3, dt, dtLog);
    HUFv07_decodeStreamX4(op3, &raw mut bitD3, opStart4, dt, dtLog);
    HUFv07_decodeStreamX4(op4, &raw mut bitD4, oend, dt, dtLog);
    let endCheck: U32 = BITv07_endOfDStream(&raw mut bitD1) as U32
        & BITv07_endOfDStream(&raw mut bitD2) as U32
        & BITv07_endOfDStream(&raw mut bitD3) as U32
        & BITv07_endOfDStream(&raw mut bitD4) as U32;
    if endCheck == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let mut dtd: DTableDesc = HUFv07_getDTableDesc(DTable);
    if dtd.tableType as ::core::ffi::c_int != 1 as ::core::ffi::c_int {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    return HUFv07_decompress4X4_usingDTable_internal(dst, dstSize, cSrc, cSrcSize, DTable);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4_DCtx(
    mut dctx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let mut hSize: size_t = HUFv07_readDTableX4(dctx, cSrc, cSrcSize);
    if HUFv07_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUFv07_decompress4X4_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        dctx,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X4(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable: [HUFv07_DTable; 4097] = [
        (12 as ::core::ffi::c_int as U32).wrapping_mul(0x1000001 as U32),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    return HUFv07_decompress4X4_DCtx(
        &raw mut DTable as *mut HUFv07_DTable,
        dst,
        dstSize,
        cSrc,
        cSrcSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable) as DTableDesc;
    return if dtd.tableType as ::core::ffi::c_int != 0 {
        HUFv07_decompress1X4_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    } else {
        HUFv07_decompress1X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUFv07_DTable,
) -> size_t {
    let dtd: DTableDesc = HUFv07_getDTableDesc(DTable) as DTableDesc;
    return if dtd.tableType as ::core::ffi::c_int != 0 {
        HUFv07_decompress4X4_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    } else {
        HUFv07_decompress4X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable)
    };
}
static mut algoTime: [[algo_time_t; 3]; 16] = [
    [
        algo_time_t {
            tableTime: 0 as U32,
            decode256Time: 0 as U32,
        },
        algo_time_t {
            tableTime: 1 as U32,
            decode256Time: 1 as U32,
        },
        algo_time_t {
            tableTime: 2 as U32,
            decode256Time: 2 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 0 as U32,
            decode256Time: 0 as U32,
        },
        algo_time_t {
            tableTime: 1 as U32,
            decode256Time: 1 as U32,
        },
        algo_time_t {
            tableTime: 2 as U32,
            decode256Time: 2 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 38 as U32,
            decode256Time: 130 as U32,
        },
        algo_time_t {
            tableTime: 1313 as U32,
            decode256Time: 74 as U32,
        },
        algo_time_t {
            tableTime: 2151 as U32,
            decode256Time: 38 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 448 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1353 as U32,
            decode256Time: 74 as U32,
        },
        algo_time_t {
            tableTime: 2238 as U32,
            decode256Time: 41 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 556 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1353 as U32,
            decode256Time: 74 as U32,
        },
        algo_time_t {
            tableTime: 2238 as U32,
            decode256Time: 47 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 714 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1418 as U32,
            decode256Time: 74 as U32,
        },
        algo_time_t {
            tableTime: 2436 as U32,
            decode256Time: 53 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 883 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1437 as U32,
            decode256Time: 74 as U32,
        },
        algo_time_t {
            tableTime: 2464 as U32,
            decode256Time: 61 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 897 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1515 as U32,
            decode256Time: 75 as U32,
        },
        algo_time_t {
            tableTime: 2622 as U32,
            decode256Time: 68 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 926 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1613 as U32,
            decode256Time: 75 as U32,
        },
        algo_time_t {
            tableTime: 2730 as U32,
            decode256Time: 75 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 947 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1729 as U32,
            decode256Time: 77 as U32,
        },
        algo_time_t {
            tableTime: 3359 as U32,
            decode256Time: 77 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1107 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 2083 as U32,
            decode256Time: 81 as U32,
        },
        algo_time_t {
            tableTime: 4006 as U32,
            decode256Time: 84 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1177 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 2379 as U32,
            decode256Time: 87 as U32,
        },
        algo_time_t {
            tableTime: 4785 as U32,
            decode256Time: 88 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1242 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 2415 as U32,
            decode256Time: 93 as U32,
        },
        algo_time_t {
            tableTime: 5155 as U32,
            decode256Time: 84 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1349 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 2644 as U32,
            decode256Time: 106 as U32,
        },
        algo_time_t {
            tableTime: 5260 as U32,
            decode256Time: 106 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1455 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 2422 as U32,
            decode256Time: 124 as U32,
        },
        algo_time_t {
            tableTime: 4174 as U32,
            decode256Time: 124 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 722 as U32,
            decode256Time: 128 as U32,
        },
        algo_time_t {
            tableTime: 1891 as U32,
            decode256Time: 145 as U32,
        },
        algo_time_t {
            tableTime: 1936 as U32,
            decode256Time: 146 as U32,
        },
    ],
];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_selectDecoder(mut dstSize: size_t, mut cSrcSize: size_t) -> U32 {
    let Q: U32 = cSrcSize.wrapping_mul(16 as size_t).wrapping_div(dstSize) as U32;
    let D256: U32 = (dstSize >> 8 as ::core::ffi::c_int) as U32;
    let DTime0: U32 = algoTime[Q as usize][0 as ::core::ffi::c_int as usize]
        .tableTime
        .wrapping_add(
            algoTime[Q as usize][0 as ::core::ffi::c_int as usize]
                .decode256Time
                .wrapping_mul(D256),
        );
    let mut DTime1: U32 = algoTime[Q as usize][1 as ::core::ffi::c_int as usize]
        .tableTime
        .wrapping_add(
            algoTime[Q as usize][1 as ::core::ffi::c_int as usize]
                .decode256Time
                .wrapping_mul(D256),
        );
    DTime1 = (DTime1 as ::core::ffi::c_uint)
        .wrapping_add((DTime1 >> 3 as ::core::ffi::c_int) as ::core::ffi::c_uint)
        as U32 as U32;
    return (DTime1 < DTime0) as ::core::ffi::c_int as U32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    static mut decompress: [decompressionAlgo; 2] = unsafe {
        [
            Some(
                HUFv07_decompress4X2
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        size_t,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> size_t,
            ),
            Some(
                HUFv07_decompress4X4
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        size_t,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> size_t,
            ),
        ]
    };
    if dstSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize > dstSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 as size_t {
        memset(dst, *(cSrc as *const BYTE) as ::core::ffi::c_int, dstSize);
        return dstSize;
    }
    let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize) as U32;
    return decompress[algoNb as usize].expect("non-null function pointer")(
        dst, dstSize, cSrc, cSrcSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_DCtx(
    mut dctx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    if dstSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize > dstSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 as size_t {
        memset(dst, *(cSrc as *const BYTE) as ::core::ffi::c_int, dstSize);
        return dstSize;
    }
    let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize) as U32;
    return if algoNb != 0 {
        HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress4X_hufOnly(
    mut dctx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    if dstSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize >= dstSize || cSrcSize <= 1 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize) as U32;
    return if algoNb != 0 {
        HUFv07_decompress4X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress4X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUFv07_decompress1X_DCtx(
    mut dctx: *mut HUFv07_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    if dstSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize > dstSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize == dstSize {
        memcpy(dst, cSrc, dstSize);
        return dstSize;
    }
    if cSrcSize == 1 as size_t {
        memset(dst, *(cSrc as *const BYTE) as ::core::ffi::c_int, dstSize);
        return dstSize;
    }
    let algoNb: U32 = HUFv07_selectDecoder(dstSize, cSrcSize) as U32;
    return if algoNb != 0 {
        HUFv07_decompress1X4_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    } else {
        HUFv07_decompress1X2_DCtx(dctx, dst, dstSize, cSrc, cSrcSize)
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getErrorName(mut code: size_t) -> *const ::core::ffi::c_char {
    return ERR_getErrorName(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_isError(mut errorCode: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(errorCode);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_getErrorName(
    mut errorCode: size_t,
) -> *const ::core::ffi::c_char {
    return ERR_getErrorName(errorCode);
}
unsafe extern "C" fn ZSTDv07_defaultAllocFunction(
    mut opaque: *mut ::core::ffi::c_void,
    mut size: size_t,
) -> *mut ::core::ffi::c_void {
    let mut address: *mut ::core::ffi::c_void = malloc(size);
    return address;
}
unsafe extern "C" fn ZSTDv07_defaultFreeFunction(
    mut opaque: *mut ::core::ffi::c_void,
    mut address: *mut ::core::ffi::c_void,
) {
    free(address);
}
pub const ZSTDv07_DICT_MAGIC: ::core::ffi::c_uint = 0xec30a437 as ::core::ffi::c_uint;
pub const ZSTDv07_REP_NUM: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ZSTDv07_REP_INIT: ::core::ffi::c_int = ZSTDv07_REP_NUM;
static mut repStartValue: [U32; 3] = [
    1 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    8 as ::core::ffi::c_int as U32,
];
pub const ZSTDv07_WINDOWLOG_ABSOLUTEMIN: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
static mut ZSTDv07_fcs_fieldSize: [size_t; 4] = [
    0 as ::core::ffi::c_int as size_t,
    2 as ::core::ffi::c_int as size_t,
    4 as ::core::ffi::c_int as size_t,
    8 as ::core::ffi::c_int as size_t,
];
static mut ZSTDv07_did_fieldSize: [size_t; 4] = [
    0 as ::core::ffi::c_int as size_t,
    1 as ::core::ffi::c_int as size_t,
    2 as ::core::ffi::c_int as size_t,
    4 as ::core::ffi::c_int as size_t,
];
pub const ZSTDv07_BLOCKHEADERSIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut ZSTDv07_blockHeaderSize: size_t = ZSTDv07_BLOCKHEADERSIZE as size_t;
pub const MIN_SEQUENCES_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MIN_CBLOCK_SIZE: ::core::ffi::c_int =
    1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int + MIN_SEQUENCES_SIZE;
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const LONGNBSEQ: ::core::ffi::c_int = 0x7f00 as ::core::ffi::c_int;
pub const MINMATCH: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MaxML: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const MaxLL: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const MaxOff: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const MLFSELog: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const LLFSELog: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const OffFSELog: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const FSEv07_ENCODING_RAW: U32 = 0 as U32;
pub const FSEv07_ENCODING_RLE: U32 = 1 as U32;
pub const FSEv07_ENCODING_STATIC: U32 = 2 as U32;
pub const FSEv07_ENCODING_DYNAMIC: U32 = 3 as U32;
pub const ZSTD_CONTENTSIZE_ERROR: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(2 as ::core::ffi::c_ulonglong);
static mut LL_bits: [U32; 36] = [
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    2 as ::core::ffi::c_int as U32,
    2 as ::core::ffi::c_int as U32,
    3 as ::core::ffi::c_int as U32,
    3 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
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
];
static mut LL_defaultNorm: [S16; 36] = [
    4 as ::core::ffi::c_int as S16,
    3 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    3 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
];
static mut LL_defaultNormLog: U32 = 6 as U32;
static mut ML_bits: [U32; 53] = [
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    0 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    2 as ::core::ffi::c_int as U32,
    2 as ::core::ffi::c_int as U32,
    3 as ::core::ffi::c_int as U32,
    3 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
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
];
static mut ML_defaultNorm: [S16; 53] = [
    1 as ::core::ffi::c_int as S16,
    4 as ::core::ffi::c_int as S16,
    3 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
];
static mut ML_defaultNormLog: U32 = 6 as U32;
static mut OF_defaultNorm: [S16; 29] = [
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
];
static mut OF_defaultNormLog: U32 = 5 as U32;
unsafe extern "C" fn ZSTDv07_copy8(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    memcpy(dst, src, 8 as size_t);
}
pub const WILDCOPY_OVERLENGTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTDv07_wildcopy(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
    mut length: ptrdiff_t,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = op.offset(length as isize);
    loop {
        ZSTDv07_copy8(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
        );
        op = op.offset(8 as ::core::ffi::c_int as isize);
        ip = ip.offset(8 as ::core::ffi::c_int as isize);
        if !(op < oend) {
            break;
        }
    }
}
static mut defaultCustomMem: ZSTDv07_customMem = unsafe {
    ZSTDv07_customMem {
        customAlloc: Some(
            ZSTDv07_defaultAllocFunction
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    size_t,
                ) -> *mut ::core::ffi::c_void,
        ),
        customFree: Some(
            ZSTDv07_defaultFreeFunction
                as unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
        ),
        opaque: NULL,
    }
};
unsafe extern "C" fn ZSTDv07_copy4(
    mut dst: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
) {
    memcpy(dst, src, 4 as size_t);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_sizeofDCtx(mut dctx: *const ZSTDv07_DCtx) -> size_t {
    return ::core::mem::size_of::<ZSTDv07_DCtx>() as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_estimateDCtxSize() -> size_t {
    return ::core::mem::size_of::<ZSTDv07_DCtx>() as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin(mut dctx: *mut ZSTDv07_DCtx) -> size_t {
    (*dctx).expected = ZSTDv07_frameHeaderSize_min;
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).previousDstEnd = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).base = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).vBase = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).dictEnd = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).hufTable[0 as ::core::ffi::c_int as usize] =
        (12 as ::core::ffi::c_int * 0x1000001 as ::core::ffi::c_int) as HUFv07_DTable;
    (*dctx).fseEntropy = 0 as U32;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    (*dctx).dictID = 0 as U32;
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < ZSTDv07_REP_NUM {
        (*dctx).rep[i as usize] = repStartValue[i as usize];
        i += 1;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx_advanced(
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DCtx {
    let mut dctx: *mut ZSTDv07_DCtx = ::core::ptr::null_mut::<ZSTDv07_DCtx>();
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }
    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return ::core::ptr::null_mut::<ZSTDv07_DCtx>();
    }
    dctx = customMem.customAlloc.expect("non-null function pointer")(
        customMem.opaque,
        ::core::mem::size_of::<ZSTDv07_DCtx>() as size_t,
    ) as *mut ZSTDv07_DCtx;
    if dctx.is_null() {
        return ::core::ptr::null_mut::<ZSTDv07_DCtx>();
    }
    memcpy(
        &raw mut (*dctx).customMem as *mut ::core::ffi::c_void,
        &raw mut customMem as *const ::core::ffi::c_void,
        ::core::mem::size_of::<ZSTDv07_customMem>() as size_t,
    );
    ZSTDv07_decompressBegin(dctx);
    return dctx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDCtx() -> *mut ZSTDv07_DCtx {
    return ZSTDv07_createDCtx_advanced(defaultCustomMem);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_freeDCtx(mut dctx: *mut ZSTDv07_DCtx) -> size_t {
    if dctx.is_null() {
        return 0 as size_t;
    }
    (*dctx)
        .customMem
        .customFree
        .expect("non-null function pointer")(
        (*dctx).customMem.opaque,
        dctx as *mut ::core::ffi::c_void,
    );
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_copyDCtx(
    mut dstDCtx: *mut ZSTDv07_DCtx,
    mut srcDCtx: *const ZSTDv07_DCtx,
) {
    memcpy(
        dstDCtx as *mut ::core::ffi::c_void,
        srcDCtx as *const ::core::ffi::c_void,
        (::core::mem::size_of::<ZSTDv07_DCtx>() as size_t).wrapping_sub(
            ((ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + WILDCOPY_OVERLENGTH) as size_t)
                .wrapping_add(ZSTDv07_frameHeaderSize_max),
        ),
    );
}
unsafe extern "C" fn ZSTDv07_frameHeaderSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize < ZSTDv07_frameHeaderSize_min {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let fhd: BYTE = *(src as *const BYTE).offset(4 as ::core::ffi::c_int as isize);
    let dictID: U32 = (fhd as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    let directMode: U32 =
        (fhd as ::core::ffi::c_int >> 5 as ::core::ffi::c_int & 1 as ::core::ffi::c_int) as U32;
    let fcsId: U32 = (fhd as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as U32;
    return ZSTDv07_frameHeaderSize_min
        .wrapping_add((directMode == 0) as ::core::ffi::c_int as size_t)
        .wrapping_add(ZSTDv07_did_fieldSize[dictID as usize])
        .wrapping_add(ZSTDv07_fcs_fieldSize[fcsId as usize])
        .wrapping_add(
            (directMode != 0 && ZSTDv07_fcs_fieldSize[fcsId as usize] == 0) as ::core::ffi::c_int
                as size_t,
        );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getFrameParams(
    mut fparamsPtr: *mut ZSTDv07_frameParams,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    if srcSize < ZSTDv07_frameHeaderSize_min {
        return ZSTDv07_frameHeaderSize_min;
    }
    memset(
        fparamsPtr as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZSTDv07_frameParams>() as size_t,
    );
    if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER as U32 {
        if MEM_readLE32(src) & 0xfffffff0 as U32 == ZSTDv07_MAGIC_SKIPPABLE_START as U32 {
            if srcSize < ZSTDv07_skippableHeaderSize {
                return ZSTDv07_skippableHeaderSize;
            }
            (*fparamsPtr).frameContentSize = MEM_readLE32(
                (src as *const ::core::ffi::c_char).offset(4 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
            ) as ::core::ffi::c_ulonglong;
            (*fparamsPtr).windowSize = 0 as ::core::ffi::c_uint;
            return 0 as size_t;
        }
        return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
    }
    let fhsize: size_t = ZSTDv07_frameHeaderSize(src, srcSize) as size_t;
    if srcSize < fhsize {
        return fhsize;
    }
    let fhdByte: BYTE = *ip.offset(4 as ::core::ffi::c_int as isize);
    let mut pos: size_t = 5 as size_t;
    let dictIDSizeCode: U32 = (fhdByte as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    let checksumFlag: U32 =
        (fhdByte as ::core::ffi::c_int >> 2 as ::core::ffi::c_int & 1 as ::core::ffi::c_int) as U32;
    let directMode: U32 =
        (fhdByte as ::core::ffi::c_int >> 5 as ::core::ffi::c_int & 1 as ::core::ffi::c_int) as U32;
    let fcsID: U32 = (fhdByte as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as U32;
    let windowSizeMax: U32 = (1 as U32)
        << (if MEM_32bits() != 0 {
            ZSTDv07_WINDOWLOG_MAX_32
        } else {
            ZSTDv07_WINDOWLOG_MAX_64
        }) as U32;
    let mut windowSize: U32 = 0 as U32;
    let mut dictID: U32 = 0 as U32;
    let mut frameContentSize: U64 = 0 as U64;
    if fhdByte as ::core::ffi::c_int & 0x8 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        return -(ZSTD_error_frameParameter_unsupported as ::core::ffi::c_int) as size_t;
    }
    if directMode == 0 {
        let fresh0 = pos;
        pos = pos.wrapping_add(1);
        let wlByte: BYTE = *ip.offset(fresh0 as isize);
        let windowLog: U32 = ((wlByte as ::core::ffi::c_int >> 3 as ::core::ffi::c_int)
            + ZSTDv07_WINDOWLOG_ABSOLUTEMIN) as U32;
        if windowLog
            > (if MEM_32bits() != 0 {
                ZSTDv07_WINDOWLOG_MAX_32
            } else {
                ZSTDv07_WINDOWLOG_MAX_64
            }) as U32
        {
            return -(ZSTD_error_frameParameter_unsupported as ::core::ffi::c_int) as size_t;
        }
        windowSize = ((1 as ::core::ffi::c_uint) << windowLog) as U32;
        windowSize = (windowSize as ::core::ffi::c_uint).wrapping_add(
            (windowSize >> 3 as ::core::ffi::c_int)
                .wrapping_mul((wlByte as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as U32)
                as ::core::ffi::c_uint,
        ) as U32 as U32;
    }
    match dictIDSizeCode {
        1 => {
            dictID = *ip.offset(pos as isize) as U32;
            pos = pos.wrapping_add(1);
        }
        2 => {
            dictID = MEM_readLE16(ip.offset(pos as isize) as *const ::core::ffi::c_void) as U32;
            pos = (pos as ::core::ffi::c_ulong).wrapping_add(2 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        3 => {
            dictID = MEM_readLE32(ip.offset(pos as isize) as *const ::core::ffi::c_void);
            pos = (pos as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        0 | _ => {}
    }
    match fcsID {
        1 => {
            frameContentSize = (MEM_readLE16(ip.offset(pos as isize) as *const ::core::ffi::c_void)
                as ::core::ffi::c_int
                + 256 as ::core::ffi::c_int) as U64;
        }
        2 => {
            frameContentSize =
                MEM_readLE32(ip.offset(pos as isize) as *const ::core::ffi::c_void) as U64;
        }
        3 => {
            frameContentSize = MEM_readLE64(ip.offset(pos as isize) as *const ::core::ffi::c_void);
        }
        0 | _ => {
            if directMode != 0 {
                frameContentSize = *ip.offset(pos as isize) as U64;
            }
        }
    }
    if windowSize == 0 {
        windowSize = frameContentSize as U32;
    }
    if windowSize > windowSizeMax {
        return -(ZSTD_error_frameParameter_unsupported as ::core::ffi::c_int) as size_t;
    }
    (*fparamsPtr).frameContentSize = frameContentSize as ::core::ffi::c_ulonglong;
    (*fparamsPtr).windowSize = windowSize as ::core::ffi::c_uint;
    (*fparamsPtr).dictID = dictID as ::core::ffi::c_uint;
    (*fparamsPtr).checksumFlag = checksumFlag as ::core::ffi::c_uint;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_getDecompressedSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_ulonglong {
    let mut fparams: ZSTDv07_frameParams = ZSTDv07_frameParams {
        frameContentSize: 0,
        windowSize: 0,
        dictID: 0,
        checksumFlag: 0,
    };
    let frResult: size_t = ZSTDv07_getFrameParams(&raw mut fparams, src, srcSize) as size_t;
    if frResult != 0 as size_t {
        return 0 as ::core::ffi::c_ulonglong;
    }
    return fparams.frameContentSize;
}
unsafe extern "C" fn ZSTDv07_decodeFrameHeader(
    mut dctx: *mut ZSTDv07_DCtx,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let result: size_t = ZSTDv07_getFrameParams(&raw mut (*dctx).fParams, src, srcSize) as size_t;
    if (*dctx).fParams.dictID != 0 && (*dctx).dictID != (*dctx).fParams.dictID as U32 {
        return -(ZSTD_error_dictionary_wrong as ::core::ffi::c_int) as size_t;
    }
    if (*dctx).fParams.checksumFlag != 0 {
        ZSTD_XXH64_reset(&raw mut (*dctx).xxhState, 0 as XXH64_hash_t);
    }
    return result;
}
unsafe extern "C" fn ZSTDv07_getcBlockSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut bpPtr: *mut blockProperties_t,
) -> size_t {
    let in_0: *const BYTE = src as *const BYTE;
    let mut cSize: U32 = 0;
    if srcSize < ZSTDv07_blockHeaderSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    (*bpPtr).blockType = (*in_0 as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as blockType_t;
    cSize = (*in_0.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        + ((*in_0.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int)
        + ((*in_0.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 7 as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int)) as U32;
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
unsafe extern "C" fn ZSTDv07_copyRawBlock(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if srcSize > 0 as size_t {
        memcpy(dst, src, srcSize);
    }
    return srcSize;
}
unsafe extern "C" fn ZSTDv07_decodeLiteralsBlock(
    mut dctx: *mut ZSTDv07_DCtx,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    if srcSize < MIN_CBLOCK_SIZE as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    match (*istart.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        >> 6 as ::core::ffi::c_int) as litBlockType_t as ::core::ffi::c_uint
    {
        0 => {
            let mut litSize: size_t = 0;
            let mut litCSize: size_t = 0;
            let mut singleStream: size_t = 0 as size_t;
            let mut lhSize: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                >> 4 as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int) as U32;
            if srcSize < 5 as size_t {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            match lhSize {
                2 => {
                    lhSize = 4 as U32;
                    litSize = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 10 as ::core::ffi::c_int)
                        + ((*istart.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 2 as ::core::ffi::c_int)
                        + (*istart.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            >> 6 as ::core::ffi::c_int)) as size_t;
                    litCSize = (((*istart.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 63 as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *istart.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                }
                3 => {
                    lhSize = 5 as U32;
                    litSize = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 14 as ::core::ffi::c_int)
                        + ((*istart.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 6 as ::core::ffi::c_int)
                        + (*istart.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            >> 2 as ::core::ffi::c_int)) as size_t;
                    litCSize = (((*istart.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 3 as ::core::ffi::c_int)
                        << 16 as ::core::ffi::c_int)
                        + ((*istart.offset(3 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                        + *istart.offset(4 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                }
                0 | 1 | _ => {
                    lhSize = 3 as U32;
                    singleStream = (*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 16 as ::core::ffi::c_int) as size_t;
                    litSize = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 6 as ::core::ffi::c_int)
                        + (*istart.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            >> 2 as ::core::ffi::c_int)) as size_t;
                    litCSize = (((*istart.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 3 as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *istart.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                }
            }
            if litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as size_t {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if litCSize.wrapping_add(lhSize as size_t) > srcSize {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if ERR_isError(if singleStream != 0 {
                HUFv07_decompress1X2_DCtx(
                    &raw mut (*dctx).hufTable as *mut HUFv07_DTable,
                    &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                    litSize,
                    istart.offset(lhSize as isize) as *const ::core::ffi::c_void,
                    litCSize,
                )
            } else {
                HUFv07_decompress4X_hufOnly(
                    &raw mut (*dctx).hufTable as *mut HUFv07_DTable,
                    &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                    litSize,
                    istart.offset(lhSize as isize) as *const ::core::ffi::c_void,
                    litCSize,
                )
            }) != 0
            {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
            (*dctx).litSize = litSize;
            (*dctx).litEntropy = 1 as U32;
            memset(
                (&raw mut (*dctx).litBuffer as *mut BYTE).offset((*dctx).litSize as isize)
                    as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                WILDCOPY_OVERLENGTH as size_t,
            );
            return litCSize.wrapping_add(lhSize as size_t);
        }
        1 => {
            let mut litSize_0: size_t = 0;
            let mut litCSize_0: size_t = 0;
            let mut lhSize_0: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                >> 4 as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int) as U32;
            if lhSize_0 != 1 as U32 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if (*dctx).litEntropy == 0 as U32 {
                return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
            }
            lhSize_0 = 3 as U32;
            litSize_0 = (((*istart.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 15 as ::core::ffi::c_int)
                << 6 as ::core::ffi::c_int)
                + (*istart.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    >> 2 as ::core::ffi::c_int)) as size_t;
            litCSize_0 = (((*istart.offset(1 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int)
                + *istart.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                as size_t;
            if litCSize_0.wrapping_add(lhSize_0 as size_t) > srcSize {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            let errorCode: size_t = HUFv07_decompress1X4_usingDTable(
                &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                litSize_0,
                istart.offset(lhSize_0 as isize) as *const ::core::ffi::c_void,
                litCSize_0,
                &raw mut (*dctx).hufTable as *mut HUFv07_DTable,
            ) as size_t;
            if ERR_isError(errorCode) != 0 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
            (*dctx).litSize = litSize_0;
            memset(
                (&raw mut (*dctx).litBuffer as *mut BYTE).offset((*dctx).litSize as isize)
                    as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                WILDCOPY_OVERLENGTH as size_t,
            );
            return litCSize_0.wrapping_add(lhSize_0 as size_t);
        }
        2 => {
            let mut litSize_1: size_t = 0;
            let mut lhSize_1: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                >> 4 as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int) as U32;
            match lhSize_1 {
                2 => {
                    litSize_1 = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *istart.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                }
                3 => {
                    litSize_1 = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 16 as ::core::ffi::c_int)
                        + ((*istart.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                        + *istart.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                }
                0 | 1 | _ => {
                    lhSize_1 = 1 as U32;
                    litSize_1 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 31 as ::core::ffi::c_int) as size_t;
                }
            }
            if (lhSize_1 as size_t)
                .wrapping_add(litSize_1)
                .wrapping_add(WILDCOPY_OVERLENGTH as size_t)
                > srcSize
            {
                if litSize_1.wrapping_add(lhSize_1 as size_t) > srcSize {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
                memcpy(
                    &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                    istart.offset(lhSize_1 as isize) as *const ::core::ffi::c_void,
                    litSize_1,
                );
                (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
                (*dctx).litSize = litSize_1;
                memset(
                    (&raw mut (*dctx).litBuffer as *mut BYTE).offset((*dctx).litSize as isize)
                        as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    WILDCOPY_OVERLENGTH as size_t,
                );
                return (lhSize_1 as size_t).wrapping_add(litSize_1);
            }
            (*dctx).litPtr = istart.offset(lhSize_1 as isize);
            (*dctx).litSize = litSize_1;
            return (lhSize_1 as size_t).wrapping_add(litSize_1);
        }
        3 => {
            let mut litSize_2: size_t = 0;
            let mut lhSize_2: U32 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                >> 4 as ::core::ffi::c_int
                & 3 as ::core::ffi::c_int) as U32;
            match lhSize_2 {
                2 => {
                    litSize_2 = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *istart.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                }
                3 => {
                    litSize_2 = (((*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 15 as ::core::ffi::c_int)
                        << 16 as ::core::ffi::c_int)
                        + ((*istart.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                        + *istart.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as size_t;
                    if srcSize < 4 as size_t {
                        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                    }
                }
                0 | 1 | _ => {
                    lhSize_2 = 1 as U32;
                    litSize_2 = (*istart.offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 31 as ::core::ffi::c_int) as size_t;
                }
            }
            if litSize_2 > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as size_t {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            memset(
                &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                *istart.offset(lhSize_2 as isize) as ::core::ffi::c_int,
                litSize_2.wrapping_add(WILDCOPY_OVERLENGTH as size_t),
            );
            (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
            (*dctx).litSize = litSize_2;
            return lhSize_2.wrapping_add(1 as U32) as size_t;
        }
        _ => return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t,
    };
}
unsafe extern "C" fn ZSTDv07_buildSeqTable(
    mut DTable: *mut FSEv07_DTable,
    mut type_0: U32,
    mut max: U32,
    mut maxLog: U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut defaultNorm: *const S16,
    mut defaultLog: U32,
    mut flagRepeatTable: U32,
) -> size_t {
    match type_0 {
        1 => {
            if srcSize == 0 {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            if *(src as *const BYTE) as U32 > max {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            FSEv07_buildDTable_rle(DTable, *(src as *const BYTE));
            return 1 as size_t;
        }
        0 => {
            FSEv07_buildDTable(
                DTable,
                defaultNorm as *const ::core::ffi::c_short,
                max as ::core::ffi::c_uint,
                defaultLog as ::core::ffi::c_uint,
            );
            return 0 as size_t;
        }
        2 => {
            if flagRepeatTable == 0 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            return 0 as size_t;
        }
        3 | _ => {
            let mut tableLog: U32 = 0;
            let mut norm: [S16; 53] = [0; 53];
            let headerSize: size_t = FSEv07_readNCount(
                &raw mut norm as *mut ::core::ffi::c_short,
                &raw mut max,
                &raw mut tableLog,
                src,
                srcSize,
            ) as size_t;
            if ERR_isError(headerSize) != 0 {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if tableLog > maxLog {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            FSEv07_buildDTable(
                DTable,
                &raw mut norm as *mut S16,
                max as ::core::ffi::c_uint,
                tableLog as ::core::ffi::c_uint,
            );
            return headerSize;
        }
    };
}
unsafe extern "C" fn ZSTDv07_decodeSeqHeaders(
    mut nbSeqPtr: *mut ::core::ffi::c_int,
    mut DTableLL: *mut FSEv07_DTable,
    mut DTableML: *mut FSEv07_DTable,
    mut DTableOffb: *mut FSEv07_DTable,
    mut flagRepeatTable: U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let mut ip: *const BYTE = istart;
    if srcSize < MIN_SEQUENCES_SIZE as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let fresh5 = ip;
    ip = ip.offset(1);
    let mut nbSeq: ::core::ffi::c_int = *fresh5 as ::core::ffi::c_int;
    if nbSeq == 0 {
        *nbSeqPtr = 0 as ::core::ffi::c_int;
        return 1 as size_t;
    }
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
            let fresh6 = ip;
            ip = ip.offset(1);
            nbSeq = ((nbSeq - 0x80 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                + *fresh6 as ::core::ffi::c_int;
        }
    }
    *nbSeqPtr = nbSeq;
    if ip.offset(4 as ::core::ffi::c_int as isize) > iend {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let LLtype: U32 = (*ip as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as U32;
    let OFtype: U32 =
        (*ip as ::core::ffi::c_int >> 4 as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    let MLtype: U32 =
        (*ip as ::core::ffi::c_int >> 2 as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    ip = ip.offset(1);
    let llhSize: size_t = ZSTDv07_buildSeqTable(
        DTableLL,
        LLtype,
        MaxLL as U32,
        LLFSELog as U32,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        &raw const LL_defaultNorm as *const S16,
        LL_defaultNormLog,
        flagRepeatTable,
    ) as size_t;
    if ERR_isError(llhSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(llhSize as isize);
    let ofhSize: size_t = ZSTDv07_buildSeqTable(
        DTableOffb,
        OFtype,
        MaxOff as U32,
        OffFSELog as U32,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        &raw const OF_defaultNorm as *const S16,
        OF_defaultNormLog,
        flagRepeatTable,
    ) as size_t;
    if ERR_isError(ofhSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(ofhSize as isize);
    let mlhSize: size_t = ZSTDv07_buildSeqTable(
        DTableML,
        MLtype,
        MaxML as U32,
        MLFSELog as U32,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        &raw const ML_defaultNorm as *const S16,
        ML_defaultNormLog,
        flagRepeatTable,
    ) as size_t;
    if ERR_isError(mlhSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(mlhSize as isize);
    return ip.offset_from(istart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTDv07_decodeSequence(mut seqState: *mut seqState_t) -> seq_t {
    let mut seq: seq_t = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };
    let llCode: U32 = FSEv07_peekSymbol(&raw mut (*seqState).stateLL) as U32;
    let mlCode: U32 = FSEv07_peekSymbol(&raw mut (*seqState).stateML) as U32;
    let ofCode: U32 = FSEv07_peekSymbol(&raw mut (*seqState).stateOffb) as U32;
    let llBits: U32 = LL_bits[llCode as usize];
    let mlBits: U32 = ML_bits[mlCode as usize];
    let ofBits: U32 = ofCode;
    let totalBits: U32 = llBits.wrapping_add(mlBits).wrapping_add(ofBits);
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
    static mut OF_base: [U32; 29] = [
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
    ];
    let mut offset: size_t = 0;
    if ofCode == 0 {
        offset = 0 as size_t;
    } else {
        offset = (OF_base[ofCode as usize] as size_t)
            .wrapping_add(BITv07_readBits(&raw mut (*seqState).DStream, ofBits));
        if MEM_32bits() != 0 {
            BITv07_reloadDStream(&raw mut (*seqState).DStream);
        }
    }
    if ofCode <= 1 as U32 {
        if (llCode == 0 as U32) as ::core::ffi::c_int
            & (offset <= 1 as size_t) as ::core::ffi::c_int
            != 0
        {
            offset = (1 as size_t).wrapping_sub(offset);
        }
        if offset != 0 {
            let temp: size_t = (*seqState).prevOffset[offset as usize];
            if offset != 1 as size_t {
                (*seqState).prevOffset[2 as ::core::ffi::c_int as usize] =
                    (*seqState).prevOffset[1 as ::core::ffi::c_int as usize];
            }
            (*seqState).prevOffset[1 as ::core::ffi::c_int as usize] =
                (*seqState).prevOffset[0 as ::core::ffi::c_int as usize];
            offset = temp;
            (*seqState).prevOffset[0 as ::core::ffi::c_int as usize] = offset;
        } else {
            offset = (*seqState).prevOffset[0 as ::core::ffi::c_int as usize];
        }
    } else {
        (*seqState).prevOffset[2 as ::core::ffi::c_int as usize] =
            (*seqState).prevOffset[1 as ::core::ffi::c_int as usize];
        (*seqState).prevOffset[1 as ::core::ffi::c_int as usize] =
            (*seqState).prevOffset[0 as ::core::ffi::c_int as usize];
        (*seqState).prevOffset[0 as ::core::ffi::c_int as usize] = offset;
    }
    seq.offset = offset;
    seq.matchLength = (ML_base[mlCode as usize] as size_t).wrapping_add(
        (if mlCode > 31 as U32 {
            BITv07_readBits(&raw mut (*seqState).DStream, mlBits)
        } else {
            0 as size_t
        }),
    );
    if MEM_32bits() != 0 && mlBits.wrapping_add(llBits) > 24 as U32 {
        BITv07_reloadDStream(&raw mut (*seqState).DStream);
    }
    seq.litLength = (LL_base[llCode as usize] as size_t).wrapping_add(
        (if llCode > 15 as U32 {
            BITv07_readBits(&raw mut (*seqState).DStream, llBits)
        } else {
            0 as size_t
        }),
    );
    if MEM_32bits() != 0
        || totalBits
            > (64 as ::core::ffi::c_int
                - 7 as ::core::ffi::c_int
                - (LLFSELog + MLFSELog + OffFSELog)) as U32
    {
        BITv07_reloadDStream(&raw mut (*seqState).DStream);
    }
    FSEv07_updateState(&raw mut (*seqState).stateLL, &raw mut (*seqState).DStream);
    FSEv07_updateState(&raw mut (*seqState).stateML, &raw mut (*seqState).DStream);
    if MEM_32bits() != 0 {
        BITv07_reloadDStream(&raw mut (*seqState).DStream);
    }
    FSEv07_updateState(&raw mut (*seqState).stateOffb, &raw mut (*seqState).DStream);
    return seq;
}
unsafe extern "C" fn ZSTDv07_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    mut litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    base: *const BYTE,
    vBase: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.offset(sequence.litLength as isize);
    let sequenceLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    let oMatchEnd: *mut BYTE = op.offset(sequenceLength as isize);
    let oend_w: *mut BYTE = oend.offset(-(WILDCOPY_OVERLENGTH as isize));
    let iLitEnd: *const BYTE = (*litPtr).offset(sequence.litLength as isize);
    let mut match_0: *const BYTE = oLitEnd.offset(-(sequence.offset as isize));
    if sequence
        .litLength
        .wrapping_add(WILDCOPY_OVERLENGTH as size_t)
        > oend.offset_from(op) as ::core::ffi::c_long as size_t
    {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequenceLength > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ZSTDv07_wildcopy(
        op as *mut ::core::ffi::c_void,
        *litPtr as *const ::core::ffi::c_void,
        sequence.litLength as ptrdiff_t,
    );
    op = oLitEnd;
    *litPtr = iLitEnd;
    if sequence.offset > oLitEnd.offset_from(base) as ::core::ffi::c_long as size_t {
        if sequence.offset > oLitEnd.offset_from(vBase) as ::core::ffi::c_long as size_t {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        match_0 = dictEnd.offset(-(base.offset_from(match_0) as ::core::ffi::c_long as isize));
        if match_0.offset(sequence.matchLength as isize) <= dictEnd {
            memmove(
                oLitEnd as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        let length1: size_t = dictEnd.offset_from(match_0) as ::core::ffi::c_long as size_t;
        memmove(
            oLitEnd as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            length1,
        );
        op = oLitEnd.offset(length1 as isize);
        sequence.matchLength = (sequence.matchLength as ::core::ffi::c_ulong)
            .wrapping_sub(length1 as ::core::ffi::c_ulong) as size_t
            as size_t;
        match_0 = base;
        if op > oend_w || sequence.matchLength < MINMATCH as size_t {
            while op < oMatchEnd {
                let fresh1 = match_0;
                match_0 = match_0.offset(1);
                let fresh2 = op;
                op = op.offset(1);
                *fresh2 = *fresh1;
            }
            return sequenceLength;
        }
    }
    if sequence.offset < 8 as size_t {
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
        let sub2: ::core::ffi::c_int = dec64table[sequence.offset as usize];
        *op.offset(0 as ::core::ffi::c_int as isize) =
            *match_0.offset(0 as ::core::ffi::c_int as isize);
        *op.offset(1 as ::core::ffi::c_int as isize) =
            *match_0.offset(1 as ::core::ffi::c_int as isize);
        *op.offset(2 as ::core::ffi::c_int as isize) =
            *match_0.offset(2 as ::core::ffi::c_int as isize);
        *op.offset(3 as ::core::ffi::c_int as isize) =
            *match_0.offset(3 as ::core::ffi::c_int as isize);
        match_0 = match_0.offset(dec32table[sequence.offset as usize] as isize);
        ZSTDv07_copy4(
            op.offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
        );
        match_0 = match_0.offset(-(sub2 as isize));
    } else {
        ZSTDv07_copy8(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
        );
    }
    op = op.offset(8 as ::core::ffi::c_int as isize);
    match_0 = match_0.offset(8 as ::core::ffi::c_int as isize);
    if oMatchEnd > oend.offset(-((16 as ::core::ffi::c_int - MINMATCH) as isize)) {
        if op < oend_w {
            ZSTDv07_wildcopy(
                op as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                oend_w.offset_from(op) as ptrdiff_t,
            );
            match_0 = match_0.offset(oend_w.offset_from(op) as ::core::ffi::c_long as isize);
            op = oend_w;
        }
        while op < oMatchEnd {
            let fresh3 = match_0;
            match_0 = match_0.offset(1);
            let fresh4 = op;
            op = op.offset(1);
            *fresh4 = *fresh3;
        }
    } else {
        ZSTDv07_wildcopy(
            op as *mut ::core::ffi::c_void,
            match_0 as *const ::core::ffi::c_void,
            sequence.matchLength as ptrdiff_t - 8 as ptrdiff_t,
        );
    }
    return sequenceLength;
}
unsafe extern "C" fn ZSTDv07_decompressSequences(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.offset(seqSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(maxDstSize as isize);
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.offset((*dctx).litSize as isize);
    let mut DTableLL: *mut FSEv07_DTable = &raw mut (*dctx).LLTable as *mut FSEv07_DTable;
    let mut DTableML: *mut FSEv07_DTable = &raw mut (*dctx).MLTable as *mut FSEv07_DTable;
    let mut DTableOffb: *mut FSEv07_DTable = &raw mut (*dctx).OffTable as *mut FSEv07_DTable;
    let base: *const BYTE = (*dctx).base as *const BYTE;
    let vBase: *const BYTE = (*dctx).vBase as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;
    let mut nbSeq: ::core::ffi::c_int = 0;
    let seqHSize: size_t = ZSTDv07_decodeSeqHeaders(
        &raw mut nbSeq,
        DTableLL,
        DTableML,
        DTableOffb,
        (*dctx).fseEntropy,
        ip as *const ::core::ffi::c_void,
        seqSize,
    ) as size_t;
    if ERR_isError(seqHSize) != 0 {
        return seqHSize;
    }
    ip = ip.offset(seqHSize as isize);
    if nbSeq != 0 {
        let mut seqState: seqState_t = seqState_t {
            DStream: BITv07_DStream_t {
                bitContainer: 0,
                bitsConsumed: 0,
                ptr: ::core::ptr::null::<::core::ffi::c_char>(),
                start: ::core::ptr::null::<::core::ffi::c_char>(),
            },
            stateLL: FSEv07_DState_t {
                state: 0,
                table: ::core::ptr::null::<::core::ffi::c_void>(),
            },
            stateOffb: FSEv07_DState_t {
                state: 0,
                table: ::core::ptr::null::<::core::ffi::c_void>(),
            },
            stateML: FSEv07_DState_t {
                state: 0,
                table: ::core::ptr::null::<::core::ffi::c_void>(),
            },
            prevOffset: [0; 3],
        };
        (*dctx).fseEntropy = 1 as U32;
        let mut i: U32 = 0;
        i = 0 as U32;
        while i < ZSTDv07_REP_INIT as U32 {
            seqState.prevOffset[i as usize] = (*dctx).rep[i as usize] as size_t;
            i = i.wrapping_add(1);
        }
        let errorCode: size_t = BITv07_initDStream(
            &raw mut seqState.DStream,
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        ) as size_t;
        if ERR_isError(errorCode) != 0 {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        FSEv07_initDState(
            &raw mut seqState.stateLL,
            &raw mut seqState.DStream,
            DTableLL,
        );
        FSEv07_initDState(
            &raw mut seqState.stateOffb,
            &raw mut seqState.DStream,
            DTableOffb,
        );
        FSEv07_initDState(
            &raw mut seqState.stateML,
            &raw mut seqState.DStream,
            DTableML,
        );
        while BITv07_reloadDStream(&raw mut seqState.DStream) as ::core::ffi::c_uint
            <= BITv07_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
            && nbSeq != 0
        {
            nbSeq -= 1;
            let sequence: seq_t = ZSTDv07_decodeSequence(&raw mut seqState) as seq_t;
            let oneSeqSize: size_t = ZSTDv07_execSequence(
                op,
                oend,
                sequence,
                &raw mut litPtr,
                litEnd,
                base,
                vBase,
                dictEnd,
            ) as size_t;
            if ERR_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.offset(oneSeqSize as isize);
        }
        if nbSeq != 0 {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        let mut i_0: U32 = 0;
        i_0 = 0 as U32;
        while i_0 < ZSTDv07_REP_INIT as U32 {
            (*dctx).rep[i_0 as usize] = seqState.prevOffset[i_0 as usize] as U32;
            i_0 = i_0.wrapping_add(1);
        }
    }
    let lastLLSize: size_t = litEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
    if lastLLSize > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if lastLLSize > 0 as size_t {
        memcpy(
            op as *mut ::core::ffi::c_void,
            litPtr as *const ::core::ffi::c_void,
            lastLLSize,
        );
        op = op.offset(lastLLSize as isize);
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTDv07_checkContinuity(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *const ::core::ffi::c_void,
) {
    if dst != (*dctx).previousDstEnd {
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).vBase = (dst as *const ::core::ffi::c_char).offset(
            -(((*dctx).previousDstEnd as *const ::core::ffi::c_char)
                .offset_from((*dctx).base as *const ::core::ffi::c_char)
                as ::core::ffi::c_long as isize),
        ) as *const ::core::ffi::c_void;
        (*dctx).base = dst;
        (*dctx).previousDstEnd = dst;
    }
}
unsafe extern "C" fn ZSTDv07_decompressBlock_internal(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    if srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let litCSize: size_t = ZSTDv07_decodeLiteralsBlock(dctx, src, srcSize) as size_t;
    if ERR_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.offset(litCSize as isize);
    srcSize = (srcSize as ::core::ffi::c_ulong).wrapping_sub(litCSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return ZSTDv07_decompressSequences(
        dctx,
        dst,
        dstCapacity,
        ip as *const ::core::ffi::c_void,
        srcSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBlock(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut dSize: size_t = 0;
    ZSTDv07_checkContinuity(dctx, dst);
    dSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
    (*dctx).previousDstEnd =
        (dst as *mut ::core::ffi::c_char).offset(dSize as isize) as *const ::core::ffi::c_void;
    return dSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_insertBlock(
    mut dctx: *mut ZSTDv07_DCtx,
    mut blockStart: *const ::core::ffi::c_void,
    mut blockSize: size_t,
) -> size_t {
    ZSTDv07_checkContinuity(dctx, blockStart);
    (*dctx).previousDstEnd = (blockStart as *const ::core::ffi::c_char).offset(blockSize as isize)
        as *const ::core::ffi::c_void;
    return blockSize;
}
unsafe extern "C" fn ZSTDv07_generateNxBytes(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut byte: BYTE,
    mut length: size_t,
) -> size_t {
    if length > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if length > 0 as size_t {
        memset(dst, byte as ::core::ffi::c_int, length);
    }
    return length;
}
unsafe extern "C" fn ZSTDv07_decompressFrame(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.offset(srcSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstCapacity as isize);
    let mut op: *mut BYTE = ostart;
    let mut remainingSize: size_t = srcSize;
    if srcSize < ZSTDv07_frameHeaderSize_min.wrapping_add(ZSTDv07_blockHeaderSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let frameHeaderSize: size_t =
        ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min) as size_t;
    if ERR_isError(frameHeaderSize) != 0 {
        return frameHeaderSize;
    }
    if srcSize < frameHeaderSize.wrapping_add(ZSTDv07_blockHeaderSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize) != 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(frameHeaderSize as isize);
    remainingSize = (remainingSize as ::core::ffi::c_ulong)
        .wrapping_sub(frameHeaderSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    loop {
        let mut decodedSize: size_t = 0;
        let mut blockProperties: blockProperties_t = blockProperties_t {
            blockType: bt_compressed,
            origSize: 0,
        };
        let cBlockSize: size_t = ZSTDv07_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            &raw mut blockProperties,
        ) as size_t;
        if ERR_isError(cBlockSize) != 0 {
            return cBlockSize;
        }
        ip = ip.offset(ZSTDv07_blockHeaderSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTDv07_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if cBlockSize > remainingSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        match blockProperties.blockType as ::core::ffi::c_uint {
            0 => {
                decodedSize = ZSTDv07_decompressBlock_internal(
                    dctx,
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    cBlockSize,
                );
            }
            1 => {
                decodedSize = ZSTDv07_copyRawBlock(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    cBlockSize,
                );
            }
            2 => {
                decodedSize = ZSTDv07_generateNxBytes(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    *ip,
                    blockProperties.origSize as size_t,
                );
            }
            3 => {
                if remainingSize != 0 {
                    return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
                }
                decodedSize = 0 as size_t;
            }
            _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
        }
        if blockProperties.blockType as ::core::ffi::c_uint
            == bt_end as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            break;
        }
        if ERR_isError(decodedSize) != 0 {
            return decodedSize;
        }
        if (*dctx).fParams.checksumFlag != 0 {
            ZSTD_XXH64_update(
                &raw mut (*dctx).xxhState,
                op as *const ::core::ffi::c_void,
                decodedSize,
            );
        }
        op = op.offset(decodedSize as isize);
        ip = ip.offset(cBlockSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(cBlockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTDv07_decompress_usingPreparedDCtx(
    mut dctx: *mut ZSTDv07_DCtx,
    mut refDCtx: *const ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    ZSTDv07_copyDCtx(dctx, refDCtx);
    ZSTDv07_checkContinuity(dctx, dst);
    return ZSTDv07_decompressFrame(dctx, dst, dstCapacity, src, srcSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress_usingDict(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    ZSTDv07_decompressBegin_usingDict(dctx, dict, dictSize);
    ZSTDv07_checkContinuity(dctx, dst);
    return ZSTDv07_decompressFrame(dctx, dst, dstCapacity, src, srcSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressDCtx(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTDv07_decompress_usingDict(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        ::core::ptr::null::<::core::ffi::c_void>(),
        0 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut regenSize: size_t = 0;
    let dctx: *mut ZSTDv07_DCtx = ZSTDv07_createDCtx() as *mut ZSTDv07_DCtx;
    if dctx.is_null() {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    regenSize = ZSTDv07_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTDv07_freeDCtx(dctx);
    return regenSize;
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
pub unsafe extern "C" fn ZSTDv07_findFrameSizeInfoLegacy(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut cSize: *mut size_t,
    mut dBound: *mut ::core::ffi::c_ulonglong,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let mut remainingSize: size_t = srcSize;
    let mut nbBlocks: size_t = 0 as size_t;
    if srcSize < ZSTDv07_frameHeaderSize_min.wrapping_add(ZSTDv07_blockHeaderSize) {
        ZSTD_errorFrameSizeInfoLegacy(
            cSize,
            dBound,
            -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
        );
        return;
    }
    let frameHeaderSize: size_t = ZSTDv07_frameHeaderSize(src, srcSize) as size_t;
    if ERR_isError(frameHeaderSize) != 0 {
        ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, frameHeaderSize);
        return;
    }
    if MEM_readLE32(src) != ZSTDv07_MAGICNUMBER as U32 {
        ZSTD_errorFrameSizeInfoLegacy(
            cSize,
            dBound,
            -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t,
        );
        return;
    }
    if srcSize < frameHeaderSize.wrapping_add(ZSTDv07_blockHeaderSize) {
        ZSTD_errorFrameSizeInfoLegacy(
            cSize,
            dBound,
            -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
        );
        return;
    }
    ip = ip.offset(frameHeaderSize as isize);
    remainingSize = (remainingSize as ::core::ffi::c_ulong)
        .wrapping_sub(frameHeaderSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    loop {
        let mut blockProperties: blockProperties_t = blockProperties_t {
            blockType: bt_compressed,
            origSize: 0,
        };
        let cBlockSize: size_t = ZSTDv07_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            remainingSize,
            &raw mut blockProperties,
        ) as size_t;
        if ERR_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }
        ip = ip.offset(ZSTDv07_blockHeaderSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTDv07_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if blockProperties.blockType as ::core::ffi::c_uint
            == bt_end as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            break;
        }
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(
                cSize,
                dBound,
                -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
            );
            return;
        }
        ip = ip.offset(cBlockSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(cBlockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
        nbBlocks = nbBlocks.wrapping_add(1);
    }
    *cSize = ip.offset_from(src as *const BYTE) as ::core::ffi::c_long as size_t;
    *dBound =
        nbBlocks.wrapping_mul(ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as size_t) as ::core::ffi::c_ulonglong;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_nextSrcSizeToDecompress(mut dctx: *mut ZSTDv07_DCtx) -> size_t {
    return (*dctx).expected;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_isSkipFrame(mut dctx: *mut ZSTDv07_DCtx) -> ::core::ffi::c_int {
    return ((*dctx).stage as ::core::ffi::c_uint
        == ZSTDds_skipFrame as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressContinue(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize != (*dctx).expected {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if dstCapacity != 0 {
        ZSTDv07_checkContinuity(dctx, dst);
    }
    match (*dctx).stage as ::core::ffi::c_uint {
        0 => {
            if srcSize != ZSTDv07_frameHeaderSize_min {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            if MEM_readLE32(src) & 0xfffffff0 as U32 == ZSTDv07_MAGIC_SKIPPABLE_START as U32 {
                memcpy(
                    &raw mut (*dctx).headerBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                    src,
                    ZSTDv07_frameHeaderSize_min,
                );
                (*dctx).expected =
                    ZSTDv07_skippableHeaderSize.wrapping_sub(ZSTDv07_frameHeaderSize_min);
                (*dctx).stage = ZSTDds_decodeSkippableHeader;
                return 0 as size_t;
            }
            (*dctx).headerSize = ZSTDv07_frameHeaderSize(src, ZSTDv07_frameHeaderSize_min);
            if ERR_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            memcpy(
                &raw mut (*dctx).headerBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                src,
                ZSTDv07_frameHeaderSize_min,
            );
            if (*dctx).headerSize > ZSTDv07_frameHeaderSize_min {
                (*dctx).expected = (*dctx).headerSize.wrapping_sub(ZSTDv07_frameHeaderSize_min);
                (*dctx).stage = ZSTDds_decodeFrameHeader;
                return 0 as size_t;
            }
            (*dctx).expected = 0 as size_t;
        }
        1 => {}
        2 => {
            let mut bp: blockProperties_t = blockProperties_t {
                blockType: bt_compressed,
                origSize: 0,
            };
            let cBlockSize: size_t =
                ZSTDv07_getcBlockSize(src, ZSTDv07_blockHeaderSize, &raw mut bp) as size_t;
            if ERR_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
            if bp.blockType as ::core::ffi::c_uint
                == bt_end as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                if (*dctx).fParams.checksumFlag != 0 {
                    let h64: U64 = ZSTD_XXH64_digest(&raw mut (*dctx).xxhState) as U64;
                    let h32: U32 = (h64 >> 11 as ::core::ffi::c_int) as U32
                        & (((1 as ::core::ffi::c_int) << 22 as ::core::ffi::c_int)
                            - 1 as ::core::ffi::c_int) as U32;
                    let ip: *const BYTE = src as *const BYTE;
                    let check32: U32 = (*ip.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        + ((*ip.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                        + ((*ip.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            & 0x3f as ::core::ffi::c_int)
                            << 16 as ::core::ffi::c_int))
                        as U32;
                    if check32 != h32 {
                        return -(ZSTD_error_checksum_wrong as ::core::ffi::c_int) as size_t;
                    }
                }
                (*dctx).expected = 0 as size_t;
                (*dctx).stage = ZSTDds_getFrameHeaderSize;
            } else {
                (*dctx).expected = cBlockSize;
                (*dctx).bType = bp.blockType;
                (*dctx).stage = ZSTDds_decompressBlock;
            }
            return 0 as size_t;
        }
        3 => {
            let mut rSize: size_t = 0;
            match (*dctx).bType as ::core::ffi::c_uint {
                0 => {
                    rSize = ZSTDv07_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize);
                }
                1 => {
                    rSize = ZSTDv07_copyRawBlock(dst, dstCapacity, src, srcSize);
                }
                2 => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
                3 => {
                    rSize = 0 as size_t;
                }
                _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
            }
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            (*dctx).expected = ZSTDv07_blockHeaderSize;
            if ERR_isError(rSize) != 0 {
                return rSize;
            }
            (*dctx).previousDstEnd = (dst as *mut ::core::ffi::c_char).offset(rSize as isize)
                as *const ::core::ffi::c_void;
            if (*dctx).fParams.checksumFlag != 0 {
                ZSTD_XXH64_update(&raw mut (*dctx).xxhState, dst, rSize);
            }
            return rSize;
        }
        4 => {
            memcpy(
                (&raw mut (*dctx).headerBuffer as *mut BYTE)
                    .offset(ZSTDv07_frameHeaderSize_min as isize)
                    as *mut ::core::ffi::c_void,
                src,
                (*dctx).expected,
            );
            (*dctx).expected = MEM_readLE32(
                (&raw mut (*dctx).headerBuffer as *mut BYTE)
                    .offset(4 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
            ) as size_t;
            (*dctx).stage = ZSTDds_skipFrame;
            return 0 as size_t;
        }
        5 => {
            (*dctx).expected = 0 as size_t;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            return 0 as size_t;
        }
        _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
    }
    let mut result: size_t = 0;
    memcpy(
        (&raw mut (*dctx).headerBuffer as *mut BYTE).offset(ZSTDv07_frameHeaderSize_min as isize)
            as *mut ::core::ffi::c_void,
        src,
        (*dctx).expected,
    );
    result = ZSTDv07_decodeFrameHeader(
        dctx,
        &raw mut (*dctx).headerBuffer as *mut BYTE as *const ::core::ffi::c_void,
        (*dctx).headerSize,
    );
    if ERR_isError(result) != 0 {
        return result;
    }
    (*dctx).expected = ZSTDv07_blockHeaderSize;
    (*dctx).stage = ZSTDds_decodeBlockHeader;
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDv07_refDictContent(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).vBase = (dict as *const ::core::ffi::c_char).offset(
        -(((*dctx).previousDstEnd as *const ::core::ffi::c_char)
            .offset_from((*dctx).base as *const ::core::ffi::c_char)
            as ::core::ffi::c_long as isize),
    ) as *const ::core::ffi::c_void;
    (*dctx).base = dict;
    (*dctx).previousDstEnd = (dict as *const ::core::ffi::c_char).offset(dictSize as isize)
        as *const ::core::ffi::c_void;
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDv07_loadEntropy(
    mut dctx: *mut ZSTDv07_DCtx,
    dict: *const ::core::ffi::c_void,
    dictSize: size_t,
) -> size_t {
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.offset(dictSize as isize);
    let hSize: size_t = HUFv07_readDTableX4(
        &raw mut (*dctx).hufTable as *mut HUFv07_DTable,
        dict,
        dictSize,
    ) as size_t;
    if ERR_isError(hSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(hSize as isize);
    let mut offcodeNCount: [::core::ffi::c_short; 29] = [0; 29];
    let mut offcodeMaxValue: U32 = MaxOff as U32;
    let mut offcodeLog: U32 = 0;
    let offcodeHeaderSize: size_t = FSEv07_readNCount(
        &raw mut offcodeNCount as *mut ::core::ffi::c_short,
        &raw mut offcodeMaxValue,
        &raw mut offcodeLog,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(offcodeHeaderSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if offcodeLog > OffFSELog as U32 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    let errorCode: size_t = FSEv07_buildDTable(
        &raw mut (*dctx).OffTable as *mut FSEv07_DTable,
        &raw mut offcodeNCount as *mut ::core::ffi::c_short,
        offcodeMaxValue as ::core::ffi::c_uint,
        offcodeLog as ::core::ffi::c_uint,
    ) as size_t;
    if ERR_isError(errorCode) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(offcodeHeaderSize as isize);
    let mut matchlengthNCount: [::core::ffi::c_short; 53] = [0; 53];
    let mut matchlengthMaxValue: ::core::ffi::c_uint = MaxML as ::core::ffi::c_uint;
    let mut matchlengthLog: ::core::ffi::c_uint = 0;
    let matchlengthHeaderSize: size_t = FSEv07_readNCount(
        &raw mut matchlengthNCount as *mut ::core::ffi::c_short,
        &raw mut matchlengthMaxValue,
        &raw mut matchlengthLog,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(matchlengthHeaderSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if matchlengthLog > MLFSELog as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    let errorCode_0: size_t = FSEv07_buildDTable(
        &raw mut (*dctx).MLTable as *mut FSEv07_DTable,
        &raw mut matchlengthNCount as *mut ::core::ffi::c_short,
        matchlengthMaxValue,
        matchlengthLog,
    ) as size_t;
    if ERR_isError(errorCode_0) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(matchlengthHeaderSize as isize);
    let mut litlengthNCount: [::core::ffi::c_short; 36] = [0; 36];
    let mut litlengthMaxValue: ::core::ffi::c_uint = MaxLL as ::core::ffi::c_uint;
    let mut litlengthLog: ::core::ffi::c_uint = 0;
    let litlengthHeaderSize: size_t = FSEv07_readNCount(
        &raw mut litlengthNCount as *mut ::core::ffi::c_short,
        &raw mut litlengthMaxValue,
        &raw mut litlengthLog,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(litlengthHeaderSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if litlengthLog > LLFSELog as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    let errorCode_1: size_t = FSEv07_buildDTable(
        &raw mut (*dctx).LLTable as *mut FSEv07_DTable,
        &raw mut litlengthNCount as *mut ::core::ffi::c_short,
        litlengthMaxValue,
        litlengthLog,
    ) as size_t;
    if ERR_isError(errorCode_1) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(litlengthHeaderSize as isize);
    if dictPtr.offset(12 as ::core::ffi::c_int as isize) > dictEnd {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    (*dctx).rep[0 as ::core::ffi::c_int as usize] = MEM_readLE32(
        dictPtr.offset(0 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
    );
    if (*dctx).rep[0 as ::core::ffi::c_int as usize] == 0 as U32
        || (*dctx).rep[0 as ::core::ffi::c_int as usize] as size_t >= dictSize
    {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    (*dctx).rep[1 as ::core::ffi::c_int as usize] = MEM_readLE32(
        dictPtr.offset(4 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
    );
    if (*dctx).rep[1 as ::core::ffi::c_int as usize] == 0 as U32
        || (*dctx).rep[1 as ::core::ffi::c_int as usize] as size_t >= dictSize
    {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    (*dctx).rep[2 as ::core::ffi::c_int as usize] = MEM_readLE32(
        dictPtr.offset(8 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
    );
    if (*dctx).rep[2 as ::core::ffi::c_int as usize] == 0 as U32
        || (*dctx).rep[2 as ::core::ffi::c_int as usize] as size_t >= dictSize
    {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(12 as ::core::ffi::c_int as isize);
    (*dctx).fseEntropy = 1 as U32;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    return dictPtr.offset_from(dict as *const BYTE) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTDv07_decompress_insertDictionary(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    if dictSize < 8 as size_t {
        return ZSTDv07_refDictContent(dctx, dict, dictSize);
    }
    let magic: U32 = MEM_readLE32(dict) as U32;
    if magic != ZSTDv07_DICT_MAGIC as U32 {
        return ZSTDv07_refDictContent(dctx, dict, dictSize);
    }
    (*dctx).dictID = MEM_readLE32(
        (dict as *const ::core::ffi::c_char).offset(4 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
    );
    dict = (dict as *const ::core::ffi::c_char).offset(8 as ::core::ffi::c_int as isize)
        as *const ::core::ffi::c_void;
    dictSize = (dictSize as ::core::ffi::c_ulong).wrapping_sub(8 as ::core::ffi::c_ulong) as size_t
        as size_t;
    let eSize: size_t = ZSTDv07_loadEntropy(dctx, dict, dictSize) as size_t;
    if ERR_isError(eSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dict =
        (dict as *const ::core::ffi::c_char).offset(eSize as isize) as *const ::core::ffi::c_void;
    dictSize = (dictSize as ::core::ffi::c_ulong).wrapping_sub(eSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return ZSTDv07_refDictContent(dctx, dict, dictSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompressBegin_usingDict(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    let errorCode: size_t = ZSTDv07_decompressBegin(dctx) as size_t;
    if ERR_isError(errorCode) != 0 {
        return errorCode;
    }
    if !dict.is_null() && dictSize != 0 {
        let errorCode_0: size_t =
            ZSTDv07_decompress_insertDictionary(dctx, dict, dictSize) as size_t;
        if ERR_isError(errorCode_0) != 0 {
            return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
        }
    }
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDv07_createDDict_advanced(
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
    mut customMem: ZSTDv07_customMem,
) -> *mut ZSTDv07_DDict {
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }
    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return ::core::ptr::null_mut::<ZSTDv07_DDict>();
    }
    let ddict: *mut ZSTDv07_DDict = customMem.customAlloc.expect("non-null function pointer")(
        customMem.opaque,
        ::core::mem::size_of::<ZSTDv07_DDict>() as size_t,
    ) as *mut ZSTDv07_DDict;
    let dictContent: *mut ::core::ffi::c_void =
        customMem.customAlloc.expect("non-null function pointer")(customMem.opaque, dictSize)
            as *mut ::core::ffi::c_void;
    let dctx: *mut ZSTDv07_DCtx = ZSTDv07_createDCtx_advanced(customMem) as *mut ZSTDv07_DCtx;
    if dictContent.is_null() || ddict.is_null() || dctx.is_null() {
        customMem.customFree.expect("non-null function pointer")(customMem.opaque, dictContent);
        customMem.customFree.expect("non-null function pointer")(
            customMem.opaque,
            ddict as *mut ::core::ffi::c_void,
        );
        customMem.customFree.expect("non-null function pointer")(
            customMem.opaque,
            dctx as *mut ::core::ffi::c_void,
        );
        return ::core::ptr::null_mut::<ZSTDv07_DDict>();
    }
    memcpy(dictContent, dict, dictSize);
    let errorCode: size_t =
        ZSTDv07_decompressBegin_usingDict(dctx, dictContent, dictSize) as size_t;
    if ERR_isError(errorCode) != 0 {
        customMem.customFree.expect("non-null function pointer")(customMem.opaque, dictContent);
        customMem.customFree.expect("non-null function pointer")(
            customMem.opaque,
            ddict as *mut ::core::ffi::c_void,
        );
        customMem.customFree.expect("non-null function pointer")(
            customMem.opaque,
            dctx as *mut ::core::ffi::c_void,
        );
        return ::core::ptr::null_mut::<ZSTDv07_DDict>();
    }
    (*ddict).dict = dictContent;
    (*ddict).dictSize = dictSize;
    (*ddict).refContext = dctx;
    return ddict;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_createDDict(
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> *mut ZSTDv07_DDict {
    let allocator: ZSTDv07_customMem = ZSTDv07_customMem {
        customAlloc: None,
        customFree: None,
        opaque: NULL,
    };
    return ZSTDv07_createDDict_advanced(dict, dictSize, allocator);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_freeDDict(mut ddict: *mut ZSTDv07_DDict) -> size_t {
    let cFree: ZSTDv07_freeFunction = (*(*ddict).refContext).customMem.customFree;
    let opaque: *mut ::core::ffi::c_void = (*(*ddict).refContext).customMem.opaque;
    ZSTDv07_freeDCtx((*ddict).refContext);
    cFree.expect("non-null function pointer")(opaque, (*ddict).dict);
    cFree.expect("non-null function pointer")(opaque, ddict as *mut ::core::ffi::c_void);
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv07_decompress_usingDDict(
    mut dctx: *mut ZSTDv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut ddict: *const ZSTDv07_DDict,
) -> size_t {
    return ZSTDv07_decompress_usingPreparedDCtx(
        dctx,
        (*ddict).refContext,
        dst,
        dstCapacity,
        src,
        srcSize,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_createDCtx() -> *mut ZBUFFv07_DCtx {
    return ZBUFFv07_createDCtx_advanced(defaultCustomMem);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_createDCtx_advanced(
    mut customMem: ZSTDv07_customMem,
) -> *mut ZBUFFv07_DCtx {
    let mut zbd: *mut ZBUFFv07_DCtx = ::core::ptr::null_mut::<ZBUFFv07_DCtx>();
    if customMem.customAlloc.is_none() && customMem.customFree.is_none() {
        customMem = defaultCustomMem;
    }
    if customMem.customAlloc.is_none() || customMem.customFree.is_none() {
        return ::core::ptr::null_mut::<ZBUFFv07_DCtx>();
    }
    zbd = customMem.customAlloc.expect("non-null function pointer")(
        customMem.opaque,
        ::core::mem::size_of::<ZBUFFv07_DCtx>() as size_t,
    ) as *mut ZBUFFv07_DCtx;
    if zbd.is_null() {
        return ::core::ptr::null_mut::<ZBUFFv07_DCtx>();
    }
    memset(
        zbd as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZBUFFv07_DCtx>() as size_t,
    );
    memcpy(
        &raw mut (*zbd).customMem as *mut ::core::ffi::c_void,
        &raw mut customMem as *const ::core::ffi::c_void,
        ::core::mem::size_of::<ZSTDv07_customMem>() as size_t,
    );
    (*zbd).zd = ZSTDv07_createDCtx_advanced(customMem);
    if (*zbd).zd.is_null() {
        ZBUFFv07_freeDCtx(zbd);
        return ::core::ptr::null_mut::<ZBUFFv07_DCtx>();
    }
    (*zbd).stage = ZBUFFds_init;
    return zbd;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_freeDCtx(mut zbd: *mut ZBUFFv07_DCtx) -> size_t {
    if zbd.is_null() {
        return 0 as size_t;
    }
    ZSTDv07_freeDCtx((*zbd).zd);
    if !(*zbd).inBuff.is_null() {
        (*zbd)
            .customMem
            .customFree
            .expect("non-null function pointer")(
            (*zbd).customMem.opaque,
            (*zbd).inBuff as *mut ::core::ffi::c_void,
        );
    }
    if !(*zbd).outBuff.is_null() {
        (*zbd)
            .customMem
            .customFree
            .expect("non-null function pointer")(
            (*zbd).customMem.opaque,
            (*zbd).outBuff as *mut ::core::ffi::c_void,
        );
    }
    (*zbd)
        .customMem
        .customFree
        .expect("non-null function pointer")(
        (*zbd).customMem.opaque,
        zbd as *mut ::core::ffi::c_void,
    );
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInitDictionary(
    mut zbd: *mut ZBUFFv07_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    (*zbd).stage = ZBUFFds_loadHeader;
    (*zbd).outEnd = 0 as size_t;
    (*zbd).outStart = (*zbd).outEnd;
    (*zbd).inPos = (*zbd).outStart;
    (*zbd).lhSize = (*zbd).inPos;
    return ZSTDv07_decompressBegin_usingDict((*zbd).zd, dict, dictSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressInit(mut zbd: *mut ZBUFFv07_DCtx) -> size_t {
    return ZBUFFv07_decompressInitDictionary(
        zbd,
        ::core::ptr::null::<::core::ffi::c_void>(),
        0 as size_t,
    );
}
#[inline]
unsafe extern "C" fn ZBUFFv07_limitCopy(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let length: size_t = if dstCapacity < srcSize {
        dstCapacity
    } else {
        srcSize
    };
    if length > 0 as size_t {
        memcpy(dst, src, length);
    }
    return length;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_decompressContinue(
    mut zbd: *mut ZBUFFv07_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacityPtr: *mut size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSizePtr: *mut size_t,
) -> size_t {
    let istart: *const ::core::ffi::c_char = src as *const ::core::ffi::c_char;
    let iend: *const ::core::ffi::c_char = istart.offset(*srcSizePtr as isize);
    let mut ip: *const ::core::ffi::c_char = istart;
    let ostart: *mut ::core::ffi::c_char = dst as *mut ::core::ffi::c_char;
    let oend: *mut ::core::ffi::c_char = ostart.offset(*dstCapacityPtr as isize);
    let mut op: *mut ::core::ffi::c_char = ostart;
    let mut notDone: U32 = 1 as U32;
    while notDone != 0 {
        let mut current_block_66: u64;
        match (*zbd).stage as ::core::ffi::c_uint {
            0 => return -(ZSTD_error_init_missing as ::core::ffi::c_int) as size_t,
            1 => {
                let hSize: size_t = ZSTDv07_getFrameParams(
                    &raw mut (*zbd).fParams,
                    &raw mut (*zbd).headerBuffer as *mut BYTE as *const ::core::ffi::c_void,
                    (*zbd).lhSize,
                ) as size_t;
                if ERR_isError(hSize) != 0 {
                    return hSize;
                }
                if hSize != 0 as size_t {
                    let toLoad: size_t = hSize.wrapping_sub((*zbd).lhSize);
                    if toLoad > iend.offset_from(ip) as ::core::ffi::c_long as size_t {
                        if !ip.is_null() {
                            memcpy(
                                (&raw mut (*zbd).headerBuffer as *mut BYTE)
                                    .offset((*zbd).lhSize as isize)
                                    as *mut ::core::ffi::c_void,
                                ip as *const ::core::ffi::c_void,
                                iend.offset_from(ip) as ::core::ffi::c_long as size_t,
                            );
                        }
                        (*zbd).lhSize =
                            ((*zbd).lhSize as ::core::ffi::c_ulong)
                                .wrapping_add(iend.offset_from(ip) as ::core::ffi::c_long
                                    as ::core::ffi::c_ulong) as size_t
                                as size_t;
                        *dstCapacityPtr = 0 as size_t;
                        return hSize
                            .wrapping_sub((*zbd).lhSize)
                            .wrapping_add(ZSTDv07_blockHeaderSize);
                    }
                    memcpy(
                        (&raw mut (*zbd).headerBuffer as *mut BYTE).offset((*zbd).lhSize as isize)
                            as *mut ::core::ffi::c_void,
                        ip as *const ::core::ffi::c_void,
                        toLoad,
                    );
                    (*zbd).lhSize = hSize;
                    ip = ip.offset(toLoad as isize);
                    current_block_66 = 12961834331865314435;
                } else {
                    let h1Size: size_t = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd) as size_t;
                    let h1Result: size_t = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        NULL,
                        0 as size_t,
                        &raw mut (*zbd).headerBuffer as *mut BYTE as *const ::core::ffi::c_void,
                        h1Size,
                    ) as size_t;
                    if ERR_isError(h1Result) != 0 {
                        return h1Result;
                    }
                    if h1Size < (*zbd).lhSize {
                        let h2Size: size_t = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd) as size_t;
                        let h2Result: size_t = ZSTDv07_decompressContinue(
                            (*zbd).zd,
                            NULL,
                            0 as size_t,
                            (&raw mut (*zbd).headerBuffer as *mut BYTE).offset(h1Size as isize)
                                as *const ::core::ffi::c_void,
                            h2Size,
                        ) as size_t;
                        if ERR_isError(h2Result) != 0 {
                            return h2Result;
                        }
                    }
                    (*zbd).fParams.windowSize = if (*zbd).fParams.windowSize
                        > (1 as ::core::ffi::c_uint) << 10 as ::core::ffi::c_int
                    {
                        (*zbd).fParams.windowSize
                    } else {
                        (1 as ::core::ffi::c_uint) << 10 as ::core::ffi::c_int
                    };
                    let blockSize: size_t = (if (*zbd).fParams.windowSize
                        < (128 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int)
                            as ::core::ffi::c_uint
                    {
                        (*zbd).fParams.windowSize
                    } else {
                        (128 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int)
                            as ::core::ffi::c_uint
                    }) as size_t;
                    (*zbd).blockSize = blockSize;
                    if (*zbd).inBuffSize < blockSize {
                        (*zbd)
                            .customMem
                            .customFree
                            .expect("non-null function pointer")(
                            (*zbd).customMem.opaque,
                            (*zbd).inBuff as *mut ::core::ffi::c_void,
                        );
                        (*zbd).inBuffSize = blockSize;
                        (*zbd).inBuff = (*zbd)
                            .customMem
                            .customAlloc
                            .expect("non-null function pointer")(
                            (*zbd).customMem.opaque, blockSize
                        ) as *mut ::core::ffi::c_char;
                        if (*zbd).inBuff.is_null() {
                            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                        }
                    }
                    let neededOutSize: size_t = ((*zbd).fParams.windowSize as size_t)
                        .wrapping_add(blockSize)
                        .wrapping_add((WILDCOPY_OVERLENGTH * 2 as ::core::ffi::c_int) as size_t);
                    if (*zbd).outBuffSize < neededOutSize {
                        (*zbd)
                            .customMem
                            .customFree
                            .expect("non-null function pointer")(
                            (*zbd).customMem.opaque,
                            (*zbd).outBuff as *mut ::core::ffi::c_void,
                        );
                        (*zbd).outBuffSize = neededOutSize;
                        (*zbd).outBuff = (*zbd)
                            .customMem
                            .customAlloc
                            .expect("non-null function pointer")(
                            (*zbd).customMem.opaque,
                            neededOutSize,
                        ) as *mut ::core::ffi::c_char;
                        if (*zbd).outBuff.is_null() {
                            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                        }
                    }
                    (*zbd).stage = ZBUFFds_read;
                    current_block_66 = 8845338526596852646;
                }
            }
            2 => {
                current_block_66 = 8845338526596852646;
            }
            3 => {
                current_block_66 = 14945149239039849694;
            }
            4 => {
                current_block_66 = 5181772461570869434;
            }
            _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
        }
        match current_block_66 {
            8845338526596852646 => {
                let neededInSize: size_t = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd) as size_t;
                if neededInSize == 0 as size_t {
                    (*zbd).stage = ZBUFFds_init;
                    notDone = 0 as U32;
                    current_block_66 = 12961834331865314435;
                } else if iend.offset_from(ip) as ::core::ffi::c_long as size_t >= neededInSize {
                    let isSkipFrame: ::core::ffi::c_int =
                        ZSTDv07_isSkipFrame((*zbd).zd) as ::core::ffi::c_int;
                    let decodedSize: size_t = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        (*zbd).outBuff.offset((*zbd).outStart as isize) as *mut ::core::ffi::c_void,
                        if isSkipFrame != 0 {
                            0 as size_t
                        } else {
                            (*zbd).outBuffSize.wrapping_sub((*zbd).outStart)
                        },
                        ip as *const ::core::ffi::c_void,
                        neededInSize,
                    ) as size_t;
                    if ERR_isError(decodedSize) != 0 {
                        return decodedSize;
                    }
                    ip = ip.offset(neededInSize as isize);
                    if decodedSize == 0 && isSkipFrame == 0 {
                        current_block_66 = 12961834331865314435;
                    } else {
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize);
                        (*zbd).stage = ZBUFFds_flush;
                        current_block_66 = 12961834331865314435;
                    }
                } else if ip == iend {
                    notDone = 0 as U32;
                    current_block_66 = 12961834331865314435;
                } else {
                    (*zbd).stage = ZBUFFds_load;
                    current_block_66 = 14945149239039849694;
                }
            }
            _ => {}
        }
        match current_block_66 {
            14945149239039849694 => {
                let neededInSize_0: size_t = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd) as size_t;
                let toLoad_0: size_t = neededInSize_0.wrapping_sub((*zbd).inPos);
                let mut loadedSize: size_t = 0;
                if toLoad_0 > (*zbd).inBuffSize.wrapping_sub((*zbd).inPos) {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
                loadedSize = ZBUFFv07_limitCopy(
                    (*zbd).inBuff.offset((*zbd).inPos as isize) as *mut ::core::ffi::c_void,
                    toLoad_0,
                    ip as *const ::core::ffi::c_void,
                    iend.offset_from(ip) as ::core::ffi::c_long as size_t,
                );
                ip = ip.offset(loadedSize as isize);
                (*zbd).inPos = ((*zbd).inPos as ::core::ffi::c_ulong)
                    .wrapping_add(loadedSize as ::core::ffi::c_ulong)
                    as size_t as size_t;
                if loadedSize < toLoad_0 {
                    notDone = 0 as U32;
                    current_block_66 = 12961834331865314435;
                } else {
                    let isSkipFrame_0: ::core::ffi::c_int =
                        ZSTDv07_isSkipFrame((*zbd).zd) as ::core::ffi::c_int;
                    let decodedSize_0: size_t = ZSTDv07_decompressContinue(
                        (*zbd).zd,
                        (*zbd).outBuff.offset((*zbd).outStart as isize) as *mut ::core::ffi::c_void,
                        (*zbd).outBuffSize.wrapping_sub((*zbd).outStart),
                        (*zbd).inBuff as *const ::core::ffi::c_void,
                        neededInSize_0,
                    ) as size_t;
                    if ERR_isError(decodedSize_0) != 0 {
                        return decodedSize_0;
                    }
                    (*zbd).inPos = 0 as size_t;
                    if decodedSize_0 == 0 && isSkipFrame_0 == 0 {
                        (*zbd).stage = ZBUFFds_read;
                        current_block_66 = 12961834331865314435;
                    } else {
                        (*zbd).outEnd = (*zbd).outStart.wrapping_add(decodedSize_0);
                        (*zbd).stage = ZBUFFds_flush;
                        current_block_66 = 5181772461570869434;
                    }
                }
            }
            _ => {}
        }
        match current_block_66 {
            5181772461570869434 => {
                let toFlushSize: size_t = (*zbd).outEnd.wrapping_sub((*zbd).outStart);
                let flushedSize: size_t = ZBUFFv07_limitCopy(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    (*zbd).outBuff.offset((*zbd).outStart as isize) as *const ::core::ffi::c_void,
                    toFlushSize,
                ) as size_t;
                op = op.offset(flushedSize as isize);
                (*zbd).outStart = ((*zbd).outStart as ::core::ffi::c_ulong)
                    .wrapping_add(flushedSize as ::core::ffi::c_ulong)
                    as size_t as size_t;
                if flushedSize == toFlushSize {
                    (*zbd).stage = ZBUFFds_read;
                    if (*zbd).outStart.wrapping_add((*zbd).blockSize) > (*zbd).outBuffSize {
                        (*zbd).outEnd = 0 as size_t;
                        (*zbd).outStart = (*zbd).outEnd;
                    }
                } else {
                    notDone = 0 as U32;
                }
            }
            _ => {}
        }
    }
    *srcSizePtr = ip.offset_from(istart) as ::core::ffi::c_long as size_t;
    *dstCapacityPtr = op.offset_from(ostart) as ::core::ffi::c_long as size_t;
    let mut nextSrcSizeHint: size_t = ZSTDv07_nextSrcSizeToDecompress((*zbd).zd);
    nextSrcSizeHint = (nextSrcSizeHint as ::core::ffi::c_ulong)
        .wrapping_sub((*zbd).inPos as ::core::ffi::c_ulong) as size_t
        as size_t;
    return nextSrcSizeHint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDInSize() -> size_t {
    return (ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as size_t).wrapping_add(ZSTDv07_blockHeaderSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFFv07_recommendedDOutSize() -> size_t {
    return ZSTDv07_BLOCKSIZE_ABSOLUTEMAX as size_t;
}
