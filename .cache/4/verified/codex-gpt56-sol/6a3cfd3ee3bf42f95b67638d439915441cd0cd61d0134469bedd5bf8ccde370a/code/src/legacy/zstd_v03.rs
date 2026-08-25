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
pub type ZSTD_DCtx = ZSTDv03_Dctx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv03_Dctx_s {
    pub LLTable: [U32; 1025],
    pub OffTable: [U32; 513],
    pub MLTable: [U32; 1025],
    pub previousDstEnd: *mut ::core::ffi::c_void,
    pub base: *mut ::core::ffi::c_void,
    pub expected: size_t,
    pub bType: blockType_t,
    pub phase: U32,
    pub litPtr: *const BYTE,
    pub litSize: size_t,
    pub litBuffer: [BYTE; 131080],
}
pub type BYTE = uint8_t;
pub type uint8_t = __uint8_t;
pub type __uint8_t = u8;
pub type U32 = uint32_t;
pub type uint32_t = __uint32_t;
pub type __uint32_t = u32;
pub type blockType_t = ::core::ffi::c_uint;
pub const bt_end: blockType_t = 3;
pub const bt_rle: blockType_t = 2;
pub const bt_raw: blockType_t = 1;
pub const bt_compressed: blockType_t = 0;
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
pub struct BIT_DStream_t {
    pub bitContainer: size_t,
    pub bitsConsumed: ::core::ffi::c_uint,
    pub ptr: *const ::core::ffi::c_char,
    pub start: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct seqState_t {
    pub DStream: BIT_DStream_t,
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
pub type U16 = uint16_t;
pub type uint16_t = __uint16_t;
pub type __uint16_t = u16;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub u: U32,
    pub c: [BYTE; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_decode_t {
    pub newState: ::core::ffi::c_ushort,
    pub symbol: ::core::ffi::c_uchar,
    pub nbBits: ::core::ffi::c_uchar,
}
pub type BIT_DStream_status = ::core::ffi::c_uint;
pub const BIT_DStream_overflow: BIT_DStream_status = 3;
pub const BIT_DStream_completed: BIT_DStream_status = 2;
pub const BIT_DStream_endOfBuffer: BIT_DStream_status = 1;
pub const BIT_DStream_unfinished: BIT_DStream_status = 0;
pub type U64 = uint64_t;
pub type uint64_t = __uint64_t;
pub type __uint64_t = u64;
pub type FSE_DTable = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}
pub type S16 = int16_t;
pub type int16_t = __int16_t;
pub type __int16_t = i16;
pub const ZSTD_error_tableLog_tooLarge: C2RustUnnamed_2 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: C2RustUnnamed_2 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: C2RustUnnamed_2 = 48;
pub type decompressionAlgo = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        size_t,
        *const ::core::ffi::c_void,
        size_t,
    ) -> size_t,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_DEltX4 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}
pub type rankVal_t = [[U32; 17]; 16];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
    pub weight: BYTE,
}
pub type DTable_max_t = [U32; 4097];
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const HUF_static_assert: C2RustUnnamed_0 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_DEltX2 {
    pub byte: BYTE,
    pub nbBits: BYTE,
}
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const HUF_static_assert_0: C2RustUnnamed_1 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}
pub const ZSTD_error_prefix_unknown: C2RustUnnamed_2 = 10;
pub type ZSTDv03_Dctx = ZSTDv03_Dctx_s;
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_32bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize == 4 as usize)
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_64bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize == 8 as usize)
        as ::core::ffi::c_int as ::core::ffi::c_uint;
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
unsafe extern "C" fn MEM_readLE24(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    return (MEM_readLE16(memPtr) as ::core::ffi::c_int
        + ((*(memPtr as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int)) as U32;
}
#[inline]
unsafe extern "C" fn MEM_readLE32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read32(memPtr);
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
#[inline]
unsafe extern "C" fn MEM_readLE64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read64(memPtr);
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
#[inline]
unsafe extern "C" fn MEM_readLEST(mut memPtr: *const ::core::ffi::c_void) -> size_t {
    if MEM_32bits() != 0 {
        return MEM_readLE32(memPtr) as size_t;
    } else {
        return MEM_readLE64(memPtr) as size_t;
    };
}
#[inline]
unsafe extern "C" fn BIT_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (val.leading_zeros() as i32 ^ 31 as ::core::ffi::c_int) as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn BIT_initDStream(
    mut bitD: *mut BIT_DStream_t,
    mut srcBuffer: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize < 1 as size_t {
        memset(
            bitD as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<BIT_DStream_t>() as size_t,
        );
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if srcSize >= ::core::mem::size_of::<size_t>() as usize {
        let mut contain32: U32 = 0;
        (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
        (*bitD).ptr = (srcBuffer as *const ::core::ffi::c_char)
            .offset(srcSize as isize)
            .offset(-(::core::mem::size_of::<size_t>() as usize as isize));
        (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
        contain32 =
            *(srcBuffer as *const BYTE).offset(srcSize.wrapping_sub(1 as size_t) as isize) as U32;
        if contain32 == 0 as U32 {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
        (*bitD).bitsConsumed = (8 as ::core::ffi::c_uint).wrapping_sub(BIT_highbit32(contain32));
    } else {
        let mut contain32_0: U32 = 0;
        (*bitD).start = srcBuffer as *const ::core::ffi::c_char;
        (*bitD).ptr = (*bitD).start;
        (*bitD).bitContainer = *((*bitD).start as *const BYTE) as size_t;
        let mut current_block_21: u64;
        match srcSize {
            7 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(6 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(16 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_21 = 15978115633438717675;
            }
            6 => {
                current_block_21 = 15978115633438717675;
            }
            5 => {
                current_block_21 = 16415479014711445570;
            }
            4 => {
                current_block_21 = 10832694274390588670;
            }
            3 => {
                current_block_21 = 13290781644980186288;
            }
            2 => {
                current_block_21 = 5105936230539602861;
            }
            _ => {
                current_block_21 = 13242334135786603907;
            }
        }
        match current_block_21 {
            15978115633438717675 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(5 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(24 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_21 = 16415479014711445570;
            }
            _ => {}
        }
        match current_block_21 {
            16415479014711445570 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(4 as ::core::ffi::c_int as isize)
                        as size_t)
                        << (::core::mem::size_of::<size_t>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(32 as usize)) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_21 = 10832694274390588670;
            }
            _ => {}
        }
        match current_block_21 {
            10832694274390588670 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(3 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_21 = 13290781644980186288;
            }
            _ => {}
        }
        match current_block_21 {
            13290781644980186288 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*((*bitD).start as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
                        as size_t)
                        << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as size_t as size_t;
                current_block_21 = 5105936230539602861;
            }
            _ => {}
        }
        match current_block_21 {
            5105936230539602861 => {
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
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
        (*bitD).bitsConsumed = (8 as ::core::ffi::c_uint).wrapping_sub(BIT_highbit32(contain32_0));
        (*bitD).bitsConsumed = (*bitD).bitsConsumed.wrapping_add(
            ((::core::mem::size_of::<size_t>() as usize).wrapping_sub(srcSize as usize) as U32)
                .wrapping_mul(8 as U32) as ::core::ffi::c_uint,
        );
    }
    return srcSize;
}
#[inline]
unsafe extern "C" fn BIT_lookBits(mut bitD: *mut BIT_DStream_t, mut nbBits: U32) -> size_t {
    let bitMask: U32 = (::core::mem::size_of::<size_t>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & bitMask)
        >> 1 as ::core::ffi::c_int
        >> (bitMask.wrapping_sub(nbBits) & bitMask);
}
#[inline]
unsafe extern "C" fn BIT_lookBitsFast(mut bitD: *mut BIT_DStream_t, mut nbBits: U32) -> size_t {
    let bitMask: U32 = (::core::mem::size_of::<size_t>() as usize)
        .wrapping_mul(8 as usize)
        .wrapping_sub(1 as usize) as U32;
    return (*bitD).bitContainer << ((*bitD).bitsConsumed as U32 & bitMask)
        >> (bitMask.wrapping_add(1 as U32).wrapping_sub(nbBits) & bitMask);
}
#[inline]
unsafe extern "C" fn BIT_skipBits(mut bitD: *mut BIT_DStream_t, mut nbBits: U32) {
    (*bitD).bitsConsumed = (*bitD)
        .bitsConsumed
        .wrapping_add(nbBits as ::core::ffi::c_uint);
}
#[inline]
unsafe extern "C" fn BIT_readBits(mut bitD: *mut BIT_DStream_t, mut nbBits: U32) -> size_t {
    let mut value: size_t = BIT_lookBits(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    return value;
}
#[inline]
unsafe extern "C" fn BIT_readBitsFast(mut bitD: *mut BIT_DStream_t, mut nbBits: U32) -> size_t {
    let mut value: size_t = BIT_lookBitsFast(bitD, nbBits);
    BIT_skipBits(bitD, nbBits);
    return value;
}
#[inline]
unsafe extern "C" fn BIT_reloadDStream(mut bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if (*bitD).bitsConsumed as usize
        > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
    {
        return BIT_DStream_overflow;
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
        return BIT_DStream_unfinished;
    }
    if (*bitD).ptr == (*bitD).start {
        if ((*bitD).bitsConsumed as usize)
            < (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
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
    (*bitD).bitContainer = MEM_readLEST((*bitD).ptr as *const ::core::ffi::c_void);
    return result;
}
#[inline]
unsafe extern "C" fn BIT_endOfDStream(mut DStream: *const BIT_DStream_t) -> ::core::ffi::c_uint {
    return ((*DStream).ptr == (*DStream).start
        && (*DStream).bitsConsumed as usize
            == (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize))
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn FSE_initDState(
    mut DStatePtr: *mut FSE_DState_t,
    mut bitD: *mut BIT_DStream_t,
    mut dt: *const FSE_DTable,
) {
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    memcpy(
        &raw mut DTableH as *mut ::core::ffi::c_void,
        dt as *const ::core::ffi::c_void,
        ::core::mem::size_of::<FSE_DTableHeader>() as size_t,
    );
    (*DStatePtr).state = BIT_readBits(bitD, DTableH.tableLog as U32);
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
    let mut symbol: BYTE = DInfo.symbol as BYTE;
    let mut lowBits: size_t = BIT_readBits(bitD, nbBits);
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
    let mut symbol: BYTE = DInfo.symbol as BYTE;
    let mut lowBits: size_t = BIT_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = (DInfo.newState as size_t).wrapping_add(lowBits);
    return symbol as ::core::ffi::c_uchar;
}
#[inline]
unsafe extern "C" fn FSE_endOfDState(mut DStatePtr: *const FSE_DState_t) -> ::core::ffi::c_uint {
    return ((*DStatePtr).state == 0 as size_t) as ::core::ffi::c_int as ::core::ffi::c_uint;
}
pub const ZSTD_magicNumber: ::core::ffi::c_uint = 0xfd2fb523 as ::core::ffi::c_uint;
pub const FSE_MAX_MEMORY_USAGE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const FSE_MAX_SYMBOL_VALUE: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const FSE_MAX_TABLELOG: ::core::ffi::c_int = FSE_MAX_MEMORY_USAGE - 2 as ::core::ffi::c_int;
pub const FSE_MIN_TABLELOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const FSE_TABLELOG_ABSOLUTE_MAX: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
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
    let mut ptr: *mut ::core::ffi::c_void =
        dt.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let mut DTableH: FSE_DTableHeader = FSE_DTableHeader {
        tableLog: 0,
        fastMode: 0,
    };
    let tableDecode: *mut FSE_decode_t = ptr as *mut FSE_decode_t;
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
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog > FSE_MAX_TABLELOG as ::core::ffi::c_uint {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    DTableH.tableLog = tableLog as U16;
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
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    let mut i_0: U32 = 0;
    i_0 = 0 as U32;
    while i_0 < tableSize {
        let mut symbol: BYTE = (*tableDecode.offset(i_0 as isize)).symbol as BYTE;
        let fresh8 = symbolNext[symbol as usize];
        symbolNext[symbol as usize] = symbolNext[symbol as usize].wrapping_add(1);
        let mut nextState: U16 = fresh8;
        (*tableDecode.offset(i_0 as isize)).nbBits =
            tableLog.wrapping_sub(BIT_highbit32(nextState as U32)) as BYTE as ::core::ffi::c_uchar;
        (*tableDecode.offset(i_0 as isize)).newState = (((nextState as ::core::ffi::c_int)
            << (*tableDecode.offset(i_0 as isize)).nbBits as ::core::ffi::c_int)
            as U32)
            .wrapping_sub(tableSize) as U16
            as ::core::ffi::c_ushort;
        i_0 = i_0.wrapping_add(1);
    }
    DTableH.fastMode = noLarge as U16;
    memcpy(
        dt as *mut ::core::ffi::c_void,
        &raw mut DTableH as *const ::core::ffi::c_void,
        ::core::mem::size_of::<FSE_DTableHeader>() as size_t,
    );
    return 0 as size_t;
}
unsafe extern "C" fn FSE_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
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
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
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
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
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
    errorCode = BIT_initDStream(&raw mut bitD, cSrc, cSrcSize);
    if FSE_isError(errorCode) != 0 {
        return errorCode;
    }
    FSE_initDState(&raw mut state1, &raw mut bitD, dt);
    FSE_initDState(&raw mut state2, &raw mut bitD, dt);
    while BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
        == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
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
            BIT_reloadDStream(&raw mut bitD);
        }
        *op.offset(1 as ::core::ffi::c_int as isize) = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if (FSE_MAX_TABLELOG * 4 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as usize
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
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
            > (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
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
    while !(BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
        > BIT_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
        || op == omax
        || BIT_endOfDStream(&raw mut bitD) != 0
            && (fast != 0 || FSE_endOfDState(&raw mut state1) != 0))
    {
        let fresh16 = op;
        op = op.offset(1);
        *fresh16 = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state1, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
        if BIT_reloadDStream(&raw mut bitD) as ::core::ffi::c_uint
            > BIT_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
            || op == omax
            || BIT_endOfDStream(&raw mut bitD) != 0
                && (fast != 0 || FSE_endOfDState(&raw mut state2) != 0)
        {
            break;
        }
        let fresh17 = op;
        op = op.offset(1);
        *fresh17 = (if fast != 0 {
            FSE_decodeSymbolFast(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        } else {
            FSE_decodeSymbol(&raw mut state2, &raw mut bitD) as ::core::ffi::c_int
        }) as BYTE;
    }
    if BIT_endOfDStream(&raw mut bitD) != 0
        && FSE_endOfDState(&raw mut state1) != 0
        && FSE_endOfDState(&raw mut state2) != 0
    {
        return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
    }
    if op == omax {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
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
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
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
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
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
unsafe extern "C" fn HUF_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
pub const HUF_ABSOLUTEMAX_TABLELOG: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const HUF_MAX_TABLELOG: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const HUF_MAX_SYMBOL_VALUE: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_readStats(
    mut huffWeight: *mut BYTE,
    mut hwSize: size_t,
    mut rankStats: *mut U32,
    mut nbSymbolsPtr: *mut U32,
    mut tableLogPtr: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut weightTotal: U32 = 0;
    let mut tableLog: U32 = 0;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: size_t = 0;
    let mut oSize: size_t = 0;
    let mut n: U32 = 0;
    if srcSize == 0 {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
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
        oSize = FSE_decompress(
            huffWeight as *mut ::core::ffi::c_void,
            hwSize.wrapping_sub(1 as size_t),
            ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            iSize,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }
    memset(
        rankStats as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ((HUF_ABSOLUTEMAX_TABLELOG + 1 as ::core::ffi::c_int) as size_t)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    );
    weightTotal = 0 as U32;
    n = 0 as U32;
    while (n as size_t) < oSize {
        if *huffWeight.offset(n as isize) as ::core::ffi::c_int >= HUF_ABSOLUTEMAX_TABLELOG {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        let ref mut fresh14 = *rankStats.offset(*huffWeight.offset(n as isize) as isize);
        *fresh14 = (*fresh14).wrapping_add(1);
        weightTotal = (weightTotal as ::core::ffi::c_uint).wrapping_add(
            ((1 as ::core::ffi::c_int) << *huffWeight.offset(n as isize) as ::core::ffi::c_int
                >> 1 as ::core::ffi::c_int) as ::core::ffi::c_uint,
        ) as U32 as U32;
        n = n.wrapping_add(1);
    }
    if weightTotal == 0 as U32 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    tableLog = BIT_highbit32(weightTotal).wrapping_add(1 as ::core::ffi::c_uint) as U32;
    if tableLog > HUF_ABSOLUTEMAX_TABLELOG as U32 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let mut total: U32 = ((1 as ::core::ffi::c_int) << tableLog) as U32;
    let mut rest: U32 = total.wrapping_sub(weightTotal);
    let mut verif: U32 = ((1 as ::core::ffi::c_int) << BIT_highbit32(rest)) as U32;
    let mut lastWeight: U32 = (BIT_highbit32(rest) as U32).wrapping_add(1 as U32);
    if verif != rest {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *huffWeight.offset(oSize as isize) = lastWeight as BYTE;
    let ref mut fresh15 = *rankStats.offset(lastWeight as isize);
    *fresh15 = (*fresh15).wrapping_add(1);
    if *rankStats.offset(1 as ::core::ffi::c_int as isize) < 2 as U32
        || *rankStats.offset(1 as ::core::ffi::c_int as isize) & 1 as U32 != 0
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *nbSymbolsPtr = oSize.wrapping_add(1 as size_t) as U32;
    *tableLogPtr = tableLog;
    return iSize.wrapping_add(1 as size_t);
}
unsafe extern "C" fn HUF_readDTableX2(
    mut DTable: *mut U16,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut huffWeight: [BYTE; 256] = [0; 256];
    let mut rankVal: [U32; 17] = [0; 17];
    let mut tableLog: U32 = 0 as U32;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: size_t = *ip.offset(0 as ::core::ffi::c_int as isize) as size_t;
    let mut nbSymbols: U32 = 0 as U32;
    let mut n: U32 = 0;
    let mut nextRankStart: U32 = 0;
    let mut ptr: *mut ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dt: *mut HUF_DEltX2 = ptr as *mut HUF_DEltX2;
    iSize = HUF_readStats(
        &raw mut huffWeight as *mut BYTE,
        (HUF_MAX_SYMBOL_VALUE + 1 as ::core::ffi::c_int) as size_t,
        &raw mut rankVal as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }
    if tableLog > *DTable.offset(0 as ::core::ffi::c_int as isize) as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    *DTable.offset(0 as ::core::ffi::c_int as isize) = tableLog as U16;
    nextRankStart = 0 as U32;
    n = 1 as U32;
    while n <= tableLog {
        let mut current: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add((rankVal[n as usize] << n.wrapping_sub(1 as U32)) as ::core::ffi::c_uint)
            as U32 as U32;
        rankVal[n as usize] = current;
        n = n.wrapping_add(1);
    }
    n = 0 as U32;
    while n < nbSymbols {
        let w: U32 = huffWeight[n as usize] as U32;
        let length: U32 = ((1 as ::core::ffi::c_int) << w >> 1 as ::core::ffi::c_int) as U32;
        let mut i: U32 = 0;
        let mut D: HUF_DEltX2 = HUF_DEltX2 { byte: 0, nbBits: 0 };
        D.byte = n as BYTE;
        D.nbBits = tableLog.wrapping_add(1 as U32).wrapping_sub(w) as BYTE;
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
    return iSize;
}
unsafe extern "C" fn HUF_decodeSymbolX2(
    mut Dstream: *mut BIT_DStream_t,
    mut dt: *const HUF_DEltX2,
    dtLog: U32,
) -> BYTE {
    let val: size_t = BIT_lookBitsFast(Dstream, dtLog) as size_t;
    let c: BYTE = (*dt.offset(val as isize)).byte;
    BIT_skipBits(Dstream, (*dt.offset(val as isize)).nbBits as U32);
    return c;
}
#[inline]
unsafe extern "C" fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;
    while BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p <= pEnd.offset(-(4 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            let fresh34 = p;
            p = p.offset(1);
            *fresh34 = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            let fresh35 = p;
            p = p.offset(1);
            *fresh35 = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh36 = p;
            p = p.offset(1);
            *fresh36 = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
        }
        let fresh37 = p;
        p = p.offset(1);
        *fresh37 = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
    }
    while BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p < pEnd
    {
        let fresh38 = p;
        p = p.offset(1);
        *fresh38 = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
    }
    while p < pEnd {
        let fresh39 = p;
        p = p.offset(1);
        *fresh39 = HUF_decodeSymbolX2(bitDPtr, dt, dtLog);
    }
    return pEnd.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn HUF_decompress4X2_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const U16,
) -> size_t {
    if cSrcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut ptr: *const ::core::ffi::c_void = DTable as *const ::core::ffi::c_void;
    let dt: *const HUF_DEltX2 = (ptr as *const HUF_DEltX2).offset(1 as ::core::ffi::c_int as isize);
    let dtLog: U32 = *DTable.offset(0 as ::core::ffi::c_int as isize) as U32;
    let mut errorCode: size_t = 0;
    let mut bitD1: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: BIT_DStream_t = BIT_DStream_t {
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
    let mut length4: size_t = 0;
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
    length4 = cSrcSize.wrapping_sub(
        length1
            .wrapping_add(length2)
            .wrapping_add(length3)
            .wrapping_add(6 as size_t),
    );
    if length4 > cSrcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD1,
        istart1 as *const ::core::ffi::c_void,
        length1,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD2,
        istart2 as *const ::core::ffi::c_void,
        length2,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD3,
        istart3 as *const ::core::ffi::c_void,
        length3,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD4,
        istart4 as *const ::core::ffi::c_void,
        length4,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    endSignal = (BIT_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
        | BIT_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
        | BIT_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
        | BIT_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint) as U32;
    while endSignal == BIT_DStream_unfinished as ::core::ffi::c_int as U32
        && op4 < oend.offset(-(7 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            let fresh18 = op1;
            op1 = op1.offset(1);
            *fresh18 = HUF_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh19 = op2;
            op2 = op2.offset(1);
            *fresh19 = HUF_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh20 = op3;
            op3 = op3.offset(1);
            *fresh20 = HUF_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh21 = op4;
            op4 = op4.offset(1);
            *fresh21 = HUF_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            let fresh22 = op1;
            op1 = op1.offset(1);
            *fresh22 = HUF_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            let fresh23 = op2;
            op2 = op2.offset(1);
            *fresh23 = HUF_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            let fresh24 = op3;
            op3 = op3.offset(1);
            *fresh24 = HUF_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            let fresh25 = op4;
            op4 = op4.offset(1);
            *fresh25 = HUF_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh26 = op1;
            op1 = op1.offset(1);
            *fresh26 = HUF_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh27 = op2;
            op2 = op2.offset(1);
            *fresh27 = HUF_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh28 = op3;
            op3 = op3.offset(1);
            *fresh28 = HUF_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        }
        if MEM_64bits() != 0 {
            let fresh29 = op4;
            op4 = op4.offset(1);
            *fresh29 = HUF_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        }
        let fresh30 = op1;
        op1 = op1.offset(1);
        *fresh30 = HUF_decodeSymbolX2(&raw mut bitD1, dt, dtLog);
        let fresh31 = op2;
        op2 = op2.offset(1);
        *fresh31 = HUF_decodeSymbolX2(&raw mut bitD2, dt, dtLog);
        let fresh32 = op3;
        op3 = op3.offset(1);
        *fresh32 = HUF_decodeSymbolX2(&raw mut bitD3, dt, dtLog);
        let fresh33 = op4;
        op4 = op4.offset(1);
        *fresh33 = HUF_decodeSymbolX2(&raw mut bitD4, dt, dtLog);
        endSignal = (BIT_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
            | BIT_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
            | BIT_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
            | BIT_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint) as U32;
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
    HUF_decodeStreamX2(op1, &raw mut bitD1, opStart2, dt, dtLog);
    HUF_decodeStreamX2(op2, &raw mut bitD2, opStart3, dt, dtLog);
    HUF_decodeStreamX2(op3, &raw mut bitD3, opStart4, dt, dtLog);
    HUF_decodeStreamX2(op4, &raw mut bitD4, oend, dt, dtLog);
    endSignal = (BIT_endOfDStream(&raw mut bitD1)
        & BIT_endOfDStream(&raw mut bitD2)
        & BIT_endOfDStream(&raw mut bitD3)
        & BIT_endOfDStream(&raw mut bitD4)) as U32;
    if endSignal == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
unsafe extern "C" fn HUF_decompress4X2(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
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
    errorCode = HUF_readDTableX2(&raw mut DTable as *mut U16, cSrc, cSrcSize);
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    if errorCode >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(errorCode as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(errorCode as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress4X2_usingDTable(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        &raw mut DTable as *mut ::core::ffi::c_ushort,
    );
}
unsafe extern "C" fn HUF_fillDTableX4Level2(
    mut DTable: *mut HUF_DEltX4,
    mut sizeLog: U32,
    consumed: U32,
    mut rankValOrigin: *const U32,
    minWeight: ::core::ffi::c_int,
    mut sortedSymbols: *const sortedSymbol_t,
    sortedListSize: U32,
    mut nbBitsBaseline: U32,
    mut baseSeq: U16,
) {
    let mut DElt: HUF_DEltX4 = HUF_DEltX4 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let mut rankVal: [U32; 17] = [0; 17];
    let mut s: U32 = 0;
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
            let fresh13 = i_0;
            i_0 = i_0.wrapping_add(1);
            *DTable.offset(fresh13 as isize) = DElt;
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
unsafe extern "C" fn HUF_fillDTableX4(
    mut DTable: *mut HUF_DEltX4,
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
            HUF_fillDTableX4Level2(
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
            let mut i: U32 = 0;
            let end: U32 = start.wrapping_add(length);
            let mut DElt: HUF_DEltX4 = HUF_DEltX4 {
                sequence: 0,
                nbBits: 0,
                length: 0,
            };
            MEM_writeLE16(&raw mut DElt.sequence as *mut ::core::ffi::c_void, symbol);
            DElt.nbBits = nbBits as BYTE;
            DElt.length = 1 as BYTE;
            i = start;
            while i < end {
                *DTable.offset(i as isize) = DElt;
                i = i.wrapping_add(1);
            }
        }
        rankVal[weight as usize] = (rankVal[weight as usize] as ::core::ffi::c_uint)
            .wrapping_add(length as ::core::ffi::c_uint) as U32
            as U32;
        s = s.wrapping_add(1);
    }
}
unsafe extern "C" fn HUF_readDTableX4(
    mut DTable: *mut U32,
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
    let memLog: U32 = *DTable.offset(0 as ::core::ffi::c_int as isize);
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: size_t = *ip.offset(0 as ::core::ffi::c_int as isize) as size_t;
    let mut ptr: *mut ::core::ffi::c_void = DTable as *mut ::core::ffi::c_void;
    let dt: *mut HUF_DEltX4 = (ptr as *mut HUF_DEltX4).offset(1 as ::core::ffi::c_int as isize);
    if memLog > HUF_ABSOLUTEMAX_TABLELOG as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    iSize = HUF_readStats(
        &raw mut weightList as *mut BYTE,
        (HUF_MAX_SYMBOL_VALUE + 1 as ::core::ffi::c_int) as size_t,
        &raw mut rankStats as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
    );
    if HUF_isError(iSize) != 0 {
        return iSize;
    }
    if tableLog > memLog {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    maxW = tableLog;
    while rankStats[maxW as usize] == 0 as U32 {
        if maxW == 0 {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
        maxW = maxW.wrapping_sub(1);
    }
    let mut w: U32 = 0;
    let mut nextRankStart: U32 = 0 as U32;
    w = 1 as U32;
    while w <= maxW {
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
        let mut w_0: U32 = weightList[s as usize] as U32;
        let ref mut fresh11 = *rankStart.offset(w_0 as isize);
        let fresh12 = *fresh11;
        *fresh11 = (*fresh11).wrapping_add(1);
        let mut r: U32 = fresh12;
        sortedSymbol[r as usize].symbol = s as BYTE;
        sortedSymbol[r as usize].weight = w_0 as BYTE;
        s = s.wrapping_add(1);
    }
    *rankStart.offset(0 as ::core::ffi::c_int as isize) = 0 as U32;
    let minBits: U32 = tableLog.wrapping_add(1 as U32).wrapping_sub(maxW);
    let mut nextRankVal: U32 = 0 as U32;
    let mut w_1: U32 = 0;
    let mut consumed: U32 = 0;
    let rescale: ::core::ffi::c_int =
        memLog.wrapping_sub(tableLog).wrapping_sub(1 as U32) as ::core::ffi::c_int;
    let mut rankVal0: *mut U32 = &raw mut *(&raw mut rankVal as *mut [U32; 17])
        .offset(0 as ::core::ffi::c_int as isize) as *mut U32;
    w_1 = 1 as U32;
    while w_1 <= maxW {
        let mut current_0: U32 = nextRankVal;
        nextRankVal = (nextRankVal as ::core::ffi::c_uint).wrapping_add(
            (rankStats[w_1 as usize] << w_1.wrapping_add(rescale as U32)) as ::core::ffi::c_uint,
        ) as U32 as U32;
        *rankVal0.offset(w_1 as isize) = current_0;
        w_1 = w_1.wrapping_add(1);
    }
    consumed = minBits;
    while consumed <= memLog.wrapping_sub(minBits) {
        let mut rankValPtr: *mut U32 =
            &raw mut *(&raw mut rankVal as *mut [U32; 17]).offset(consumed as isize) as *mut U32;
        w_1 = 1 as U32;
        while w_1 <= maxW {
            *rankValPtr.offset(w_1 as isize) = *rankVal0.offset(w_1 as isize) >> consumed;
            w_1 = w_1.wrapping_add(1);
        }
        consumed = consumed.wrapping_add(1);
    }
    HUF_fillDTableX4(
        dt,
        memLog,
        &raw mut sortedSymbol as *mut sortedSymbol_t,
        sizeOfSort,
        &raw mut rankStart0 as *mut U32,
        &raw mut rankVal as *mut [U32; 17],
        maxW,
        tableLog.wrapping_add(1 as U32),
    );
    return iSize;
}
unsafe extern "C" fn HUF_decodeSymbolX4(
    mut op: *mut ::core::ffi::c_void,
    mut DStream: *mut BIT_DStream_t,
    mut dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: size_t = BIT_lookBitsFast(DStream, dtLog) as size_t;
    memcpy(
        op,
        dt.offset(val as isize) as *const ::core::ffi::c_void,
        2 as size_t,
    );
    BIT_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
    return (*dt.offset(val as isize)).length as U32;
}
unsafe extern "C" fn HUF_decodeLastSymbolX4(
    mut op: *mut ::core::ffi::c_void,
    mut DStream: *mut BIT_DStream_t,
    mut dt: *const HUF_DEltX4,
    dtLog: U32,
) -> U32 {
    let val: size_t = BIT_lookBitsFast(DStream, dtLog) as size_t;
    memcpy(
        op,
        dt.offset(val as isize) as *const ::core::ffi::c_void,
        1 as size_t,
    );
    if (*dt.offset(val as isize)).length as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
        BIT_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
    } else if ((*DStream).bitsConsumed as usize)
        < (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize)
    {
        BIT_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
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
unsafe extern "C" fn HUF_decodeStreamX4(
    mut p: *mut BYTE,
    mut bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX4,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;
    while BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p < pEnd.offset(-(7 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            p = p.offset(
                HUF_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            p = p.offset(
                HUF_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        if MEM_64bits() != 0 {
            p = p.offset(
                HUF_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        p = p
            .offset(HUF_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize);
    }
    while BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
        == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint
        && p <= pEnd.offset(-(2 as ::core::ffi::c_int as isize))
    {
        p = p
            .offset(HUF_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize);
    }
    while p <= pEnd.offset(-(2 as ::core::ffi::c_int as isize)) {
        p = p
            .offset(HUF_decodeSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize);
    }
    if p < pEnd {
        p = p.offset(
            HUF_decodeLastSymbolX4(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
        );
    }
    return p.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn HUF_decompress4X4_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const U32,
) -> size_t {
    if cSrcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut ptr: *const ::core::ffi::c_void = DTable as *const ::core::ffi::c_void;
    let dt: *const HUF_DEltX4 = (ptr as *const HUF_DEltX4).offset(1 as ::core::ffi::c_int as isize);
    let dtLog: U32 = *DTable.offset(0 as ::core::ffi::c_int as isize);
    let mut errorCode: size_t = 0;
    let mut bitD1: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: BIT_DStream_t = BIT_DStream_t {
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
    let mut length4: size_t = 0;
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
    length4 = cSrcSize.wrapping_sub(
        length1
            .wrapping_add(length2)
            .wrapping_add(length3)
            .wrapping_add(6 as size_t),
    );
    if length4 > cSrcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD1,
        istart1 as *const ::core::ffi::c_void,
        length1,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD2,
        istart2 as *const ::core::ffi::c_void,
        length2,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD3,
        istart3 as *const ::core::ffi::c_void,
        length3,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    errorCode = BIT_initDStream(
        &raw mut bitD4,
        istart4 as *const ::core::ffi::c_void,
        length4,
    );
    if HUF_isError(errorCode) != 0 {
        return errorCode;
    }
    endSignal = (BIT_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
        | BIT_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
        | BIT_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
        | BIT_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint) as U32;
    while endSignal == BIT_DStream_unfinished as ::core::ffi::c_int as U32
        && op4 < oend.offset(-(7 as ::core::ffi::c_int as isize))
    {
        if MEM_64bits() != 0 {
            op1 = op1.offset(HUF_decodeSymbolX4(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op2 = op2.offset(HUF_decodeSymbolX4(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op3 = op3.offset(HUF_decodeSymbolX4(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op4 = op4.offset(HUF_decodeSymbolX4(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            op1 = op1.offset(HUF_decodeSymbolX4(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            op2 = op2.offset(HUF_decodeSymbolX4(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            op3 = op3.offset(HUF_decodeSymbolX4(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 || HUF_MAX_TABLELOG <= 12 as ::core::ffi::c_int {
            op4 = op4.offset(HUF_decodeSymbolX4(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op1 = op1.offset(HUF_decodeSymbolX4(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op2 = op2.offset(HUF_decodeSymbolX4(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op3 = op3.offset(HUF_decodeSymbolX4(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
        }
        if MEM_64bits() != 0 {
            op4 = op4.offset(HUF_decodeSymbolX4(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
        }
        op1 = op1.offset(HUF_decodeSymbolX4(
            op1 as *mut ::core::ffi::c_void,
            &raw mut bitD1,
            dt,
            dtLog,
        ) as isize);
        op2 = op2.offset(HUF_decodeSymbolX4(
            op2 as *mut ::core::ffi::c_void,
            &raw mut bitD2,
            dt,
            dtLog,
        ) as isize);
        op3 = op3.offset(HUF_decodeSymbolX4(
            op3 as *mut ::core::ffi::c_void,
            &raw mut bitD3,
            dt,
            dtLog,
        ) as isize);
        op4 = op4.offset(HUF_decodeSymbolX4(
            op4 as *mut ::core::ffi::c_void,
            &raw mut bitD4,
            dt,
            dtLog,
        ) as isize);
        endSignal = (BIT_reloadDStream(&raw mut bitD1) as ::core::ffi::c_uint
            | BIT_reloadDStream(&raw mut bitD2) as ::core::ffi::c_uint
            | BIT_reloadDStream(&raw mut bitD3) as ::core::ffi::c_uint
            | BIT_reloadDStream(&raw mut bitD4) as ::core::ffi::c_uint) as U32;
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
    HUF_decodeStreamX4(op1, &raw mut bitD1, opStart2, dt, dtLog);
    HUF_decodeStreamX4(op2, &raw mut bitD2, opStart3, dt, dtLog);
    HUF_decodeStreamX4(op3, &raw mut bitD3, opStart4, dt, dtLog);
    HUF_decodeStreamX4(op4, &raw mut bitD4, oend, dt, dtLog);
    endSignal = (BIT_endOfDStream(&raw mut bitD1)
        & BIT_endOfDStream(&raw mut bitD2)
        & BIT_endOfDStream(&raw mut bitD3)
        & BIT_endOfDStream(&raw mut bitD4)) as U32;
    if endSignal == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
unsafe extern "C" fn HUF_decompress4X4(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    let mut DTable: [::core::ffi::c_uint; 4097] = [
        12 as ::core::ffi::c_int as ::core::ffi::c_uint,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    let mut hSize: size_t = HUF_readDTableX4(&raw mut DTable as *mut U32, cSrc, cSrcSize);
    if HUF_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress4X4_usingDTable(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        &raw mut DTable as *mut ::core::ffi::c_uint,
    );
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
unsafe extern "C" fn HUF_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
) -> size_t {
    static mut decompress: [decompressionAlgo; 3] = unsafe {
        [
            Some(
                HUF_decompress4X2
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        size_t,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> size_t,
            ),
            Some(
                HUF_decompress4X4
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        size_t,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> size_t,
            ),
            None,
        ]
    };
    let mut Q: U32 = 0;
    let D256: U32 = (dstSize >> 8 as ::core::ffi::c_int) as U32;
    let mut Dtime: [U32; 3] = [0; 3];
    let mut algoNb: U32 = 0 as U32;
    let mut n: ::core::ffi::c_int = 0;
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
    Q = cSrcSize.wrapping_mul(16 as size_t).wrapping_div(dstSize) as U32;
    n = 0 as ::core::ffi::c_int;
    while n < 3 as ::core::ffi::c_int {
        Dtime[n as usize] = algoTime[Q as usize][n as usize].tableTime.wrapping_add(
            algoTime[Q as usize][n as usize]
                .decode256Time
                .wrapping_mul(D256),
        );
        n += 1;
    }
    Dtime[1 as ::core::ffi::c_int as usize] =
        (Dtime[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint).wrapping_add(
            (Dtime[1 as ::core::ffi::c_int as usize] >> 4 as ::core::ffi::c_int)
                as ::core::ffi::c_uint,
        ) as U32 as U32;
    Dtime[2 as ::core::ffi::c_int as usize] =
        (Dtime[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint).wrapping_add(
            (Dtime[2 as ::core::ffi::c_int as usize] >> 3 as ::core::ffi::c_int)
                as ::core::ffi::c_uint,
        ) as U32 as U32;
    if Dtime[1 as ::core::ffi::c_int as usize] < Dtime[0 as ::core::ffi::c_int as usize] {
        algoNb = 1 as U32;
    }
    return decompress[algoNb as usize].expect("non-null function pointer")(
        dst, dstSize, cSrc, cSrcSize,
    );
}
pub const BIT1: ::core::ffi::c_int = 2;
pub const BIT0: ::core::ffi::c_int = 1;
pub const BLOCKSIZE: ::core::ffi::c_int =
    128 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int);
pub const MIN_SEQUENCES_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int
    + 2 as ::core::ffi::c_int
    + 3 as ::core::ffi::c_int
    + 1 as ::core::ffi::c_int;
pub const MIN_CBLOCK_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int + MIN_SEQUENCES_SIZE;
pub const IS_RAW: ::core::ffi::c_int = BIT0;
pub const IS_RLE: ::core::ffi::c_int = BIT1;
pub const MINMATCH: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const MLbits: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const LLbits: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const Offbits: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const MaxML: ::core::ffi::c_int =
    ((1 as ::core::ffi::c_int) << MLbits) - 1 as ::core::ffi::c_int;
pub const MaxLL: ::core::ffi::c_int =
    ((1 as ::core::ffi::c_int) << LLbits) - 1 as ::core::ffi::c_int;
pub const MaxOff: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const MLFSELog: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const LLFSELog: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const OffFSELog: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const ZSTD_CONTENTSIZE_ERROR: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(2 as ::core::ffi::c_ulonglong);
static mut ZSTD_blockHeaderSize: size_t = 3 as size_t;
static mut ZSTD_frameHeaderSize: size_t = 4 as size_t;
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
}
unsafe extern "C" fn ZSTD_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ERR_isError(code);
}
unsafe extern "C" fn ZSTD_getcBlockSize(
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
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSizePtr: *mut size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    let litSize: size_t =
        ((MEM_readLE32(src) & 0x1fffff as U32) >> 2 as ::core::ffi::c_int) as size_t;
    let litCSize: size_t =
        ((MEM_readLE32(ip.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            & 0xffffff as U32)
            >> 5 as ::core::ffi::c_int) as size_t;
    if litSize > *maxDstSizePtr {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if litCSize.wrapping_add(5 as size_t) > srcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if HUF_isError(HUF_decompress(
        dst,
        litSize,
        ip.offset(5 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
        litCSize,
    )) != 0
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    *maxDstSizePtr = litSize;
    return litCSize.wrapping_add(5 as size_t);
}
unsafe extern "C" fn ZSTD_decodeLiteralsBlock(
    mut ctx: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut dctx: *mut ZSTD_DCtx = ctx as *mut ZSTD_DCtx;
    let istart: *const BYTE = src as *const BYTE;
    if srcSize < MIN_CBLOCK_SIZE as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    match *istart as ::core::ffi::c_int & 3 as ::core::ffi::c_int {
        IS_RAW => {
            let litSize_0: size_t = ((MEM_readLE32(istart as *const ::core::ffi::c_void)
                & 0xffffff as U32)
                >> 2 as ::core::ffi::c_int) as size_t;
            if litSize_0 > srcSize.wrapping_sub(11 as size_t) {
                if litSize_0 > BLOCKSIZE as size_t {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
                if litSize_0 > srcSize.wrapping_sub(3 as size_t) {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
                memcpy(
                    &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                    istart as *const ::core::ffi::c_void,
                    litSize_0,
                );
                (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
                (*dctx).litSize = litSize_0;
                memset(
                    (&raw mut (*dctx).litBuffer as *mut BYTE).offset((*dctx).litSize as isize)
                        as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    8 as size_t,
                );
                return litSize_0.wrapping_add(3 as size_t);
            }
            (*dctx).litPtr = istart.offset(3 as ::core::ffi::c_int as isize);
            (*dctx).litSize = litSize_0;
            return litSize_0.wrapping_add(3 as size_t);
        }
        IS_RLE => {
            let litSize_1: size_t = ((MEM_readLE32(istart as *const ::core::ffi::c_void)
                & 0xffffff as U32)
                >> 2 as ::core::ffi::c_int) as size_t;
            if litSize_1 > BLOCKSIZE as size_t {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            memset(
                &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                *istart.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int,
                litSize_1.wrapping_add(8 as size_t),
            );
            (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
            (*dctx).litSize = litSize_1;
            return 4 as size_t;
        }
        0 | _ => {
            let mut litSize: size_t = BLOCKSIZE as size_t;
            let readSize: size_t = ZSTD_decompressLiterals(
                &raw mut (*dctx).litBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                &raw mut litSize,
                src,
                srcSize,
            ) as size_t;
            (*dctx).litPtr = &raw mut (*dctx).litBuffer as *mut BYTE;
            (*dctx).litSize = litSize;
            memset(
                (&raw mut (*dctx).litBuffer as *mut BYTE).offset((*dctx).litSize as isize)
                    as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                8 as size_t,
            );
            return readSize;
        }
    };
}
unsafe extern "C" fn ZSTD_decodeSeqHeaders(
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
    *nbSeq = MEM_readLE16(ip as *const ::core::ffi::c_void) as ::core::ffi::c_int;
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
            FSE_buildDTable_rle(DTableOffb, (*fresh5 as ::core::ffi::c_int & MaxOff) as BYTE);
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
        } else if dumps.offset(3 as ::core::ffi::c_int as isize) <= de {
            litLength = MEM_readLE24(dumps as *const ::core::ffi::c_void) as size_t;
            dumps = dumps.offset(3 as ::core::ffi::c_int as isize);
        }
        if dumps >= de {
            dumps = de.offset(-(1 as ::core::ffi::c_int as isize));
        }
    }
    static mut offsetPrefix: [size_t; 32] = [
        1 as ::core::ffi::c_int as size_t,
        1 as ::core::ffi::c_int as size_t,
        2 as ::core::ffi::c_int as size_t,
        4 as ::core::ffi::c_int as size_t,
        8 as ::core::ffi::c_int as size_t,
        16 as ::core::ffi::c_int as size_t,
        32 as ::core::ffi::c_int as size_t,
        64 as ::core::ffi::c_int as size_t,
        128 as ::core::ffi::c_int as size_t,
        256 as ::core::ffi::c_int as size_t,
        512 as ::core::ffi::c_int as size_t,
        1024 as ::core::ffi::c_int as size_t,
        2048 as ::core::ffi::c_int as size_t,
        4096 as ::core::ffi::c_int as size_t,
        8192 as ::core::ffi::c_int as size_t,
        16384 as ::core::ffi::c_int as size_t,
        32768 as ::core::ffi::c_int as size_t,
        65536 as ::core::ffi::c_int as size_t,
        131072 as ::core::ffi::c_int as size_t,
        262144 as ::core::ffi::c_int as size_t,
        524288 as ::core::ffi::c_int as size_t,
        1048576 as ::core::ffi::c_int as size_t,
        2097152 as ::core::ffi::c_int as size_t,
        4194304 as ::core::ffi::c_int as size_t,
        8388608 as ::core::ffi::c_int as size_t,
        16777216 as ::core::ffi::c_int as size_t,
        33554432 as ::core::ffi::c_int as size_t,
        1 as ::core::ffi::c_int as size_t,
        1 as ::core::ffi::c_int as size_t,
        1 as ::core::ffi::c_int as size_t,
        1 as ::core::ffi::c_int as size_t,
        1 as ::core::ffi::c_int as size_t,
    ];
    let mut offsetCode: U32 = 0;
    let mut nbBits: U32 = 0;
    offsetCode =
        FSE_decodeSymbol(&raw mut (*seqState).stateOffb, &raw mut (*seqState).DStream) as U32;
    if MEM_32bits() != 0 {
        BIT_reloadDStream(&raw mut (*seqState).DStream);
    }
    nbBits = offsetCode.wrapping_sub(1 as U32);
    if offsetCode == 0 as U32 {
        nbBits = 0 as U32;
    }
    offset = offsetPrefix[offsetCode as usize]
        .wrapping_add(BIT_readBits(&raw mut (*seqState).DStream, nbBits));
    if MEM_32bits() != 0 {
        BIT_reloadDStream(&raw mut (*seqState).DStream);
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
        } else if dumps.offset(3 as ::core::ffi::c_int as isize) <= de {
            matchLength = MEM_readLE24(dumps as *const ::core::ffi::c_void) as size_t;
            dumps = dumps.offset(3 as ::core::ffi::c_int as isize);
        }
        if dumps >= de {
            dumps = de.offset(-(1 as ::core::ffi::c_int as isize));
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
    let oMatchEnd: *mut BYTE = op
        .offset(sequence.litLength as isize)
        .offset(sequence.matchLength as isize);
    let oend_8: *mut BYTE = oend.offset(-(8 as ::core::ffi::c_int as isize));
    let litEnd: *const BYTE = (*litPtr).offset(sequence.litLength as isize);
    let seqLength: size_t = sequence.litLength.wrapping_add(sequence.matchLength);
    if seqLength > oend.offset_from(op) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as ::core::ffi::c_long as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if oLitEnd > oend_8 {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if sequence.offset > oLitEnd.offset_from(base) as ::core::ffi::c_long as U32 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if oMatchEnd > oend {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if litEnd > litLimit {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    ZSTD_wildcopy(
        op as *mut ::core::ffi::c_void,
        *litPtr as *const ::core::ffi::c_void,
        sequence.litLength as ptrdiff_t,
    );
    op = oLitEnd;
    *litPtr = litEnd;
    let mut match_0: *const BYTE = op.offset(-(sequence.offset as isize));
    if sequence.offset > op as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if match_0 < base as *const BYTE {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
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
    if oMatchEnd > oend.offset(-((16 as ::core::ffi::c_int - MINMATCH) as isize)) {
        if op < oend_8 {
            ZSTD_wildcopy(
                op as *mut ::core::ffi::c_void,
                match_0 as *const ::core::ffi::c_void,
                oend_8.offset_from(op) as ptrdiff_t,
            );
            match_0 = match_0.offset(oend_8.offset_from(op) as ::core::ffi::c_long as isize);
            op = oend_8;
        }
        while op < oMatchEnd {
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
    return oMatchEnd.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompressSequences(
    mut ctx: *mut ::core::ffi::c_void,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut seqStart: *const ::core::ffi::c_void,
    mut seqSize: size_t,
) -> size_t {
    let mut dctx: *mut ZSTD_DCtx = ctx as *mut ZSTD_DCtx;
    let mut ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.offset(seqSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.offset(maxDstSize as isize);
    let mut errorCode: size_t = 0;
    let mut dumpsLength: size_t = 0;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.offset((*dctx).litSize as isize);
    let mut nbSeq: ::core::ffi::c_int = 0;
    let mut dumps: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut DTableLL: *mut U32 = &raw mut (*dctx).LLTable as *mut U32;
    let mut DTableML: *mut U32 = &raw mut (*dctx).MLTable as *mut U32;
    let mut DTableOffb: *mut U32 = &raw mut (*dctx).OffTable as *mut U32;
    let base: *mut BYTE = (*dctx).base as *mut BYTE;
    errorCode = ZSTD_decodeSeqHeaders(
        &raw mut nbSeq,
        &raw mut dumps,
        &raw mut dumpsLength,
        DTableLL as *mut FSE_DTable,
        DTableML as *mut FSE_DTable,
        DTableOffb as *mut FSE_DTable,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
    );
    if ZSTD_isError(errorCode) != 0 {
        return errorCode;
    }
    ip = ip.offset(errorCode as isize);
    let mut sequence: seq_t = seq_t {
        litLength: 0,
        offset: 0,
        matchLength: 0,
    };
    let mut seqState: seqState_t = seqState_t {
        DStream: BIT_DStream_t {
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
    sequence.offset = 4 as size_t;
    seqState.prevOffset = sequence.offset;
    errorCode = BIT_initDStream(
        &raw mut seqState.DStream,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
    );
    if ERR_isError(errorCode) != 0 {
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
    while BIT_reloadDStream(&raw mut seqState.DStream) as ::core::ffi::c_uint
        <= BIT_DStream_completed as ::core::ffi::c_int as ::core::ffi::c_uint
        && nbSeq > 0 as ::core::ffi::c_int
    {
        let mut oneSeqSize: size_t = 0;
        nbSeq -= 1;
        ZSTD_decodeSequence(&raw mut sequence, &raw mut seqState);
        oneSeqSize = ZSTD_execSequence(op, sequence, &raw mut litPtr, litEnd, base, oend);
        if ZSTD_isError(oneSeqSize) != 0 {
            return oneSeqSize;
        }
        op = op.offset(oneSeqSize as isize);
    }
    if BIT_endOfDStream(&raw mut seqState.DStream) == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if nbSeq < 0 as ::core::ffi::c_int {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let mut lastLLSize: size_t = litEnd.offset_from(litPtr) as ::core::ffi::c_long as size_t;
    if litPtr > litEnd {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
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
    let mut litCSize: size_t = ZSTD_decodeLiteralsBlock(ctx, src, srcSize);
    if ZSTD_isError(litCSize) != 0 {
        return litCSize;
    }
    ip = ip.offset(litCSize as isize);
    srcSize = (srcSize as ::core::ffi::c_ulong).wrapping_sub(litCSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return ZSTD_decompressSequences(
        ctx,
        dst,
        maxDstSize,
        ip as *const ::core::ffi::c_void,
        srcSize,
    );
}
unsafe extern "C" fn ZSTD_decompressDCtx(
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
    let mut blockProperties: blockProperties_t = blockProperties_t {
        blockType: bt_compressed,
        origSize: 0,
    };
    if srcSize < ZSTD_frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_magicNumber as U32 {
        return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(ZSTD_frameHeaderSize as isize);
    remainingSize = (remainingSize as ::core::ffi::c_ulong)
        .wrapping_sub(ZSTD_frameHeaderSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    loop {
        let mut decodedSize: size_t = 0 as size_t;
        let mut cBlockSize: size_t = ZSTD_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            &raw mut blockProperties,
        );
        if ZSTD_isError(cBlockSize) != 0 {
            return cBlockSize;
        }
        ip = ip.offset(ZSTD_blockHeaderSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTD_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if cBlockSize > remainingSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        match blockProperties.blockType as ::core::ffi::c_uint {
            0 => {
                decodedSize = ZSTD_decompressBlock(
                    ctx,
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    cBlockSize,
                );
            }
            1 => {
                decodedSize = ZSTD_copyUncompressedBlock(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    cBlockSize,
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
        if cBlockSize == 0 as size_t {
            break;
        }
        if ZSTD_isError(decodedSize) != 0 {
            return decodedSize;
        }
        op = op.offset(decodedSize as isize);
        ip = ip.offset(cBlockSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(cBlockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut ctx: ZSTD_DCtx = ZSTD_DCtx {
        LLTable: [0; 1025],
        OffTable: [0; 513],
        MLTable: [0; 1025],
        previousDstEnd: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        base: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        expected: 0,
        bType: bt_compressed,
        phase: 0,
        litPtr: ::core::ptr::null::<BYTE>(),
        litSize: 0,
        litBuffer: [0; 131080],
    };
    ctx.base = dst;
    return ZSTD_decompressDCtx(
        &raw mut ctx as *mut ::core::ffi::c_void,
        dst,
        maxDstSize,
        src,
        srcSize,
    );
}
#[inline]
unsafe extern "C" fn ZSTD_errorFrameSizeInfoLegacy(
    mut cSize: *mut size_t,
    mut dBound: *mut ::core::ffi::c_ulonglong,
    mut ret: size_t,
) {
    *cSize = ret;
    *dBound = ZSTD_CONTENTSIZE_ERROR;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_findFrameSizeInfoLegacy(
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
    magicNumber = MEM_readLE32(src);
    if magicNumber != ZSTD_magicNumber as U32 {
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
        let mut cBlockSize: size_t = ZSTD_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            remainingSize,
            &raw mut blockProperties,
        );
        if ZSTD_isError(cBlockSize) != 0 {
            ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, cBlockSize);
            return;
        }
        ip = ip.offset(ZSTD_blockHeaderSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTD_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if cBlockSize > remainingSize {
            ZSTD_errorFrameSizeInfoLegacy(
                cSize,
                dBound,
                -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
            );
            return;
        }
        if cBlockSize == 0 as size_t {
            break;
        }
        ip = ip.offset(cBlockSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(cBlockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
        nbBlocks = nbBlocks.wrapping_add(1);
    }
    *cSize = ip.offset_from(src as *const BYTE) as ::core::ffi::c_long as size_t;
    *dBound = nbBlocks.wrapping_mul(BLOCKSIZE as size_t) as ::core::ffi::c_ulonglong;
}
unsafe extern "C" fn ZSTD_resetDCtx(mut dctx: *mut ZSTD_DCtx) -> size_t {
    (*dctx).expected = ZSTD_frameHeaderSize;
    (*dctx).phase = 0 as U32;
    (*dctx).previousDstEnd = NULL;
    (*dctx).base = NULL;
    return 0 as size_t;
}
unsafe extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    let mut dctx: *mut ZSTD_DCtx =
        malloc(::core::mem::size_of::<ZSTD_DCtx>() as size_t) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return ::core::ptr::null_mut::<ZSTD_DCtx>();
    }
    ZSTD_resetDCtx(dctx);
    return dctx;
}
unsafe extern "C" fn ZSTD_freeDCtx(mut dctx: *mut ZSTD_DCtx) -> size_t {
    free(dctx as *mut ::core::ffi::c_void);
    return 0 as size_t;
}
unsafe extern "C" fn ZSTD_nextSrcSizeToDecompress(mut dctx: *mut ZSTD_DCtx) -> size_t {
    return (*dctx).expected;
}
unsafe extern "C" fn ZSTD_decompressContinue(
    mut ctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize != (*ctx).expected {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if dst != (*ctx).previousDstEnd {
        (*ctx).base = dst;
    }
    if (*ctx).phase == 0 as U32 {
        let mut magicNumber: U32 = MEM_readLE32(src);
        if magicNumber != ZSTD_magicNumber as U32 {
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
        let mut blockSize: size_t = ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &raw mut bp);
        if ZSTD_isError(blockSize) != 0 {
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
    if ZSTD_isError(rSize) != 0 {
        return rSize;
    }
    (*ctx).previousDstEnd =
        (dst as *mut ::core::ffi::c_char).offset(rSize as isize) as *mut ::core::ffi::c_void;
    return rSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return ZSTD_isError(code);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut maxOriginalSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut compressedSize: size_t,
) -> size_t {
    return ZSTD_decompress(dst, maxOriginalSize, src, compressedSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_createDCtx() -> *mut ZSTDv03_Dctx {
    return ZSTD_createDCtx() as *mut ZSTDv03_Dctx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_freeDCtx(mut dctx: *mut ZSTDv03_Dctx) -> size_t {
    return ZSTD_freeDCtx(dctx as *mut ZSTD_DCtx);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_resetDCtx(mut dctx: *mut ZSTDv03_Dctx) -> size_t {
    return ZSTD_resetDCtx(dctx as *mut ZSTD_DCtx);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_nextSrcSizeToDecompress(mut dctx: *mut ZSTDv03_Dctx) -> size_t {
    return ZSTD_nextSrcSizeToDecompress(dctx as *mut ZSTD_DCtx);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDv03_decompressContinue(
    mut dctx: *mut ZSTDv03_Dctx,
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_decompressContinue(dctx as *mut ZSTD_DCtx, dst, maxDstSize, src, srcSize);
}
