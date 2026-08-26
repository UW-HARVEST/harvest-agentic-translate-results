use ::libc;
extern "C" {
    fn HUF_readStats_wksp(
        huffWeight: *mut BYTE,
        hwSize: size_t,
        rankStats: *mut U32,
        nbSymbolsPtr: *mut U32,
        tableLogPtr: *mut U32,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        workspace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        flags: ::core::ffi::c_int,
    ) -> size_t;
}
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
pub struct algo_time_t {
    pub tableTime: U32,
    pub decode256Time: U32,
}
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
pub struct HUF_DEltX1 {
    pub nbBits: BYTE,
    pub byte: BYTE,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_ReadDTableX1_Workspace {
    pub rankVal: [U32; 13],
    pub rankStart: [U32; 13],
    pub statsWksp: [U32; 219],
    pub symbols: [BYTE; 256],
    pub huffWeight: [BYTE; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_DEltX2 {
    pub sequence: U16,
    pub nbBits: BYTE,
    pub length: BYTE,
}
pub type rankValCol_t = [U32; 13];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_ReadDTableX2_Workspace {
    pub rankVal: [rankValCol_t; 12],
    pub rankStats: [U32; 13],
    pub rankStart0: [U32; 15],
    pub sortedSymbol: [sortedSymbol_t; 256],
    pub weightList: [BYTE; 256],
    pub calleeWksp: [U32; 219],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sortedSymbol_t {
    pub symbol: BYTE,
}
pub type HUF_DecompressUsingDTableFn = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        size_t,
        *const ::core::ffi::c_void,
        size_t,
        *const HUF_DTable,
    ) -> size_t,
>;
pub type HUF_DecompressFastLoopFn = Option<unsafe extern "C" fn(*mut HUF_DecompressFastArgs) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_DecompressFastArgs {
    pub ip: [*const BYTE; 4],
    pub op: [*mut BYTE; 4],
    pub bits: [U64; 4],
    pub dt: *const ::core::ffi::c_void,
    pub ilowest: *const BYTE,
    pub oend: *mut BYTE,
    pub iend: [*const BYTE; 4],
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
unsafe extern "C" fn MEM_write16(mut memPtr: *mut ::core::ffi::c_void, mut value: U16) {
    *(memPtr as *mut unalign16) = value as unalign16;
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
unsafe extern "C" fn ZSTD_countTrailingZeros64(mut val: U64) -> ::core::ffi::c_uint {
    return (val as ::core::ffi::c_ulonglong).trailing_zeros() as i32 as ::core::ffi::c_uint;
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
                current_block_32 = 10927942518115323718;
            }
            6 => {
                current_block_32 = 10927942518115323718;
            }
            5 => {
                current_block_32 = 773547889098531213;
            }
            4 => {
                current_block_32 = 2748150213303030314;
            }
            3 => {
                current_block_32 = 12191294733800570861;
            }
            2 => {
                current_block_32 = 1456856057684902487;
            }
            _ => {
                current_block_32 = 16203760046146113240;
            }
        }
        match current_block_32 {
            10927942518115323718 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(5 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(24 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 773547889098531213;
            }
            _ => {}
        }
        match current_block_32 {
            773547889098531213 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(4 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << (::core::mem::size_of::<BitContainerType>() as usize)
                            .wrapping_mul(8 as usize)
                            .wrapping_sub(32 as usize)) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 2748150213303030314;
            }
            _ => {}
        }
        match current_block_32 {
            2748150213303030314 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(3 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 12191294733800570861;
            }
            _ => {}
        }
        match current_block_32 {
            12191294733800570861 => {
                (*bitD).bitContainer = ((*bitD).bitContainer as ::core::ffi::c_ulong).wrapping_add(
                    ((*(srcBuffer as *const BYTE).offset(2 as ::core::ffi::c_int as isize)
                        as BitContainerType)
                        << 16 as ::core::ffi::c_int) as ::core::ffi::c_ulong,
                ) as BitContainerType as BitContainerType;
                current_block_32 = 1456856057684902487;
            }
            _ => {}
        }
        match current_block_32 {
            1456856057684902487 => {
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
#[inline]
unsafe extern "C" fn BIT_reloadDStreamFast(mut bitD: *mut BIT_DStream_t) -> BIT_DStream_status {
    if ((*bitD).ptr < (*bitD).limitPtr) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        return BIT_DStream_overflow;
    }
    return BIT_reloadDStream_internal(bitD);
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
pub const HUF_TABLELOG_MAX: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const HUF_SYMBOLVALUE_MAX: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const HUF_DECODER_FAST_TABLELOG: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const HUF_ENABLE_FAST_DECODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_getDTableDesc(mut table: *const HUF_DTable) -> DTableDesc {
    let mut dtd: DTableDesc = DTableDesc {
        maxTableLog: 0,
        tableType: 0,
        tableLog: 0,
        reserved: 0,
    };
    ::libc::memcpy(
        &raw mut dtd as *mut ::core::ffi::c_void,
        table as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DTableDesc>() as ::libc::size_t,
    );
    return dtd;
}
unsafe extern "C" fn HUF_initFastDStream(mut ip: *const BYTE) -> size_t {
    let lastByte: BYTE = *ip.offset(7 as ::core::ffi::c_int as isize);
    let bitsConsumed: size_t = (if lastByte as ::core::ffi::c_int != 0 {
        (8 as ::core::ffi::c_uint).wrapping_sub(ZSTD_highbit32(lastByte as U32))
    } else {
        0 as ::core::ffi::c_uint
    }) as size_t;
    let value: size_t = MEM_readLEST(ip as *const ::core::ffi::c_void) as size_t | 1 as size_t;
    return value << bitsConsumed;
}
unsafe extern "C" fn HUF_DecompressFastArgs_init(
    mut args: *mut HUF_DecompressFastArgs,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    let mut dt: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dtLog: U32 = HUF_getDTableDesc(DTable).tableLog as U32;
    let istart: *const BYTE = src as *const BYTE;
    let oend: *mut BYTE =
        ZSTD_maybeNullPtrAdd(dst as *mut ::core::ffi::c_uchar, dstSize as ptrdiff_t) as *mut BYTE;
    if MEM_isLittleEndian() == 0 || MEM_32bits() != 0 {
        return 0 as size_t;
    }
    if dstSize == 0 as size_t {
        return 0 as size_t;
    }
    if srcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if dtLog != HUF_DECODER_FAST_TABLELOG as U32 {
        return 0 as size_t;
    }
    let length1: size_t = MEM_readLE16(istart as *const ::core::ffi::c_void) as size_t;
    let length2: size_t =
        MEM_readLE16(istart.offset(2 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as size_t;
    let length3: size_t =
        MEM_readLE16(istart.offset(4 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
            as size_t;
    let length4: size_t = srcSize.wrapping_sub(
        length1
            .wrapping_add(length2)
            .wrapping_add(length3)
            .wrapping_add(6 as size_t),
    );
    (*args).iend[0 as ::core::ffi::c_int as usize] =
        istart.offset(6 as ::core::ffi::c_int as isize);
    (*args).iend[1 as ::core::ffi::c_int as usize] =
        (*args).iend[0 as ::core::ffi::c_int as usize].offset(length1 as isize);
    (*args).iend[2 as ::core::ffi::c_int as usize] =
        (*args).iend[1 as ::core::ffi::c_int as usize].offset(length2 as isize);
    (*args).iend[3 as ::core::ffi::c_int as usize] =
        (*args).iend[2 as ::core::ffi::c_int as usize].offset(length3 as isize);
    if length1 < 8 as size_t
        || length2 < 8 as size_t
        || length3 < 8 as size_t
        || length4 < 8 as size_t
    {
        return 0 as size_t;
    }
    if length4 > srcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    (*args).ip[0 as ::core::ffi::c_int as usize] = (*args).iend[1 as ::core::ffi::c_int as usize]
        .offset(-(::core::mem::size_of::<U64>() as usize as isize));
    (*args).ip[1 as ::core::ffi::c_int as usize] = (*args).iend[2 as ::core::ffi::c_int as usize]
        .offset(-(::core::mem::size_of::<U64>() as usize as isize));
    (*args).ip[2 as ::core::ffi::c_int as usize] = (*args).iend[3 as ::core::ffi::c_int as usize]
        .offset(-(::core::mem::size_of::<U64>() as usize as isize));
    (*args).ip[3 as ::core::ffi::c_int as usize] = (src as *const BYTE)
        .offset(srcSize as isize)
        .offset(-(::core::mem::size_of::<U64>() as usize as isize));
    (*args).op[0 as ::core::ffi::c_int as usize] = dst as *mut BYTE;
    (*args).op[1 as ::core::ffi::c_int as usize] = (*args).op[0 as ::core::ffi::c_int as usize]
        .offset(dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t) as isize);
    (*args).op[2 as ::core::ffi::c_int as usize] = (*args).op[1 as ::core::ffi::c_int as usize]
        .offset(dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t) as isize);
    (*args).op[3 as ::core::ffi::c_int as usize] = (*args).op[2 as ::core::ffi::c_int as usize]
        .offset(dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t) as isize);
    if (*args).op[3 as ::core::ffi::c_int as usize] >= oend {
        return 0 as size_t;
    }
    (*args).bits[0 as ::core::ffi::c_int as usize] =
        HUF_initFastDStream((*args).ip[0 as ::core::ffi::c_int as usize]) as U64;
    (*args).bits[1 as ::core::ffi::c_int as usize] =
        HUF_initFastDStream((*args).ip[1 as ::core::ffi::c_int as usize]) as U64;
    (*args).bits[2 as ::core::ffi::c_int as usize] =
        HUF_initFastDStream((*args).ip[2 as ::core::ffi::c_int as usize]) as U64;
    (*args).bits[3 as ::core::ffi::c_int as usize] =
        HUF_initFastDStream((*args).ip[3 as ::core::ffi::c_int as usize]) as U64;
    (*args).ilowest = istart;
    (*args).oend = oend;
    (*args).dt = dt;
    return 1 as size_t;
}
unsafe extern "C" fn HUF_initRemainingDStream(
    mut bit: *mut BIT_DStream_t,
    mut args: *const HUF_DecompressFastArgs,
    mut stream: ::core::ffi::c_int,
    mut segmentEnd: *mut BYTE,
) -> size_t {
    if (*args).op[stream as usize] > segmentEnd {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if (*args).ip[stream as usize]
        < (*args).iend[stream as usize].offset(-(8 as ::core::ffi::c_int as isize))
    {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    (*bit).bitContainer =
        MEM_readLEST((*args).ip[stream as usize] as *const ::core::ffi::c_void) as BitContainerType;
    (*bit).bitsConsumed = ZSTD_countTrailingZeros64((*args).bits[stream as usize]);
    (*bit).start = (*args).ilowest as *const ::core::ffi::c_char;
    (*bit).limitPtr = (*bit)
        .start
        .offset(::core::mem::size_of::<size_t>() as usize as isize);
    (*bit).ptr = (*args).ip[stream as usize] as *const ::core::ffi::c_char;
    return 0 as size_t;
}
unsafe extern "C" fn HUF_DEltX1_set4(mut symbol: BYTE, mut nbBits: BYTE) -> U64 {
    let mut D4: U64 = 0;
    if MEM_isLittleEndian() != 0 {
        D4 = (((symbol as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
            + nbBits as ::core::ffi::c_int) as U64;
    } else {
        D4 = (symbol as ::core::ffi::c_int
            + ((nbBits as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)) as U64;
    }
    D4 = (D4 as ::core::ffi::c_ulonglong).wrapping_mul(0x1000100010001 as ::core::ffi::c_ulonglong)
        as U64 as U64;
    return D4;
}
unsafe extern "C" fn HUF_rescaleStats(
    mut huffWeight: *mut BYTE,
    mut rankVal: *mut U32,
    mut nbSymbols: U32,
    mut tableLog: U32,
    mut targetTableLog: U32,
) -> U32 {
    if tableLog > targetTableLog {
        return tableLog;
    }
    if tableLog < targetTableLog {
        let scale: U32 = targetTableLog.wrapping_sub(tableLog);
        let mut s: U32 = 0;
        s = 0 as U32;
        while s < nbSymbols {
            let ref mut fresh8 = *huffWeight.offset(s as isize);
            *fresh8 = (*fresh8 as ::core::ffi::c_int
                + (if *huffWeight.offset(s as isize) as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int
                {
                    0 as U32
                } else {
                    scale
                }) as BYTE as ::core::ffi::c_int) as BYTE;
            s = s.wrapping_add(1);
        }
        s = targetTableLog;
        while s > scale {
            *rankVal.offset(s as isize) = *rankVal.offset(s.wrapping_sub(scale) as isize);
            s = s.wrapping_sub(1);
        }
        s = scale;
        while s > 0 as U32 {
            *rankVal.offset(s as isize) = 0 as U32;
            s = s.wrapping_sub(1);
        }
    }
    return targetTableLog;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX1_wksp(
    mut DTable: *mut HUF_DTable,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut tableLog: U32 = 0 as U32;
    let mut nbSymbols: U32 = 0 as U32;
    let mut iSize: size_t = 0;
    let dtPtr: *mut ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dt: *mut HUF_DEltX1 = dtPtr as *mut HUF_DEltX1;
    let mut wksp: *mut HUF_ReadDTableX1_Workspace = workSpace as *mut HUF_ReadDTableX1_Workspace;
    if ::core::mem::size_of::<HUF_ReadDTableX1_Workspace>() as usize > wkspSize {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    iSize = HUF_readStats_wksp(
        &raw mut (*wksp).huffWeight as *mut BYTE,
        (HUF_SYMBOLVALUE_MAX + 1 as ::core::ffi::c_int) as size_t,
        &raw mut (*wksp).rankVal as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
        &raw mut (*wksp).statsWksp as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 219]>() as size_t,
        flags,
    );
    if ERR_isError(iSize) != 0 {
        return iSize;
    }
    let mut dtd: DTableDesc = HUF_getDTableDesc(DTable);
    let maxTableLog: U32 = (dtd.maxTableLog as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as U32;
    let targetTableLog: U32 = if maxTableLog < 11 as U32 {
        maxTableLog
    } else {
        11 as U32
    };
    tableLog = HUF_rescaleStats(
        &raw mut (*wksp).huffWeight as *mut BYTE,
        &raw mut (*wksp).rankVal as *mut U32,
        nbSymbols,
        tableLog,
        targetTableLog,
    );
    if tableLog > (dtd.maxTableLog as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    dtd.tableType = 0 as BYTE;
    dtd.tableLog = tableLog as BYTE;
    ::libc::memcpy(
        DTable as *mut ::core::ffi::c_void,
        &raw mut dtd as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DTableDesc>() as ::libc::size_t,
    );
    let mut n: ::core::ffi::c_int = 0;
    let mut nextRankStart: U32 = 0 as U32;
    let unroll: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
    let nLimit: ::core::ffi::c_int =
        nbSymbols as ::core::ffi::c_int - unroll + 1 as ::core::ffi::c_int;
    n = 0 as ::core::ffi::c_int;
    while n < tableLog as ::core::ffi::c_int + 1 as ::core::ffi::c_int {
        let curr: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add((*wksp).rankVal[n as usize] as ::core::ffi::c_uint)
            as U32 as U32;
        (*wksp).rankStart[n as usize] = curr;
        n += 1;
    }
    n = 0 as ::core::ffi::c_int;
    while n < nLimit {
        let mut u: ::core::ffi::c_int = 0;
        u = 0 as ::core::ffi::c_int;
        while u < unroll {
            let w: size_t = (*wksp).huffWeight[(n + u) as usize] as size_t;
            let fresh6 = (*wksp).rankStart[w as usize];
            (*wksp).rankStart[w as usize] = (*wksp).rankStart[w as usize].wrapping_add(1);
            (*wksp).symbols[fresh6 as usize] = (n + u) as BYTE;
            u += 1;
        }
        n += unroll;
    }
    while n < nbSymbols as ::core::ffi::c_int {
        let w_0: size_t = (*wksp).huffWeight[n as usize] as size_t;
        let fresh7 = (*wksp).rankStart[w_0 as usize];
        (*wksp).rankStart[w_0 as usize] = (*wksp).rankStart[w_0 as usize].wrapping_add(1);
        (*wksp).symbols[fresh7 as usize] = n as BYTE;
        n += 1;
    }
    let mut w_1: U32 = 0;
    let mut symbol: ::core::ffi::c_int =
        (*wksp).rankVal[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int;
    let mut rankStart: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    w_1 = 1 as U32;
    while w_1 < tableLog.wrapping_add(1 as U32) {
        let symbolCount: ::core::ffi::c_int = (*wksp).rankVal[w_1 as usize] as ::core::ffi::c_int;
        let length: ::core::ffi::c_int =
            (1 as ::core::ffi::c_int) << w_1 >> 1 as ::core::ffi::c_int;
        let mut uStart: ::core::ffi::c_int = rankStart;
        let nbBits: BYTE = tableLog.wrapping_add(1 as U32).wrapping_sub(w_1) as BYTE;
        let mut s: ::core::ffi::c_int = 0;
        let mut u_0: ::core::ffi::c_int = 0;
        match length {
            1 => {
                s = 0 as ::core::ffi::c_int;
                while s < symbolCount {
                    let mut D: HUF_DEltX1 = HUF_DEltX1 { nbBits: 0, byte: 0 };
                    D.byte = (*wksp).symbols[(symbol + s) as usize];
                    D.nbBits = nbBits;
                    *dt.offset(uStart as isize) = D;
                    uStart += 1 as ::core::ffi::c_int;
                    s += 1;
                }
            }
            2 => {
                s = 0 as ::core::ffi::c_int;
                while s < symbolCount {
                    let mut D_0: HUF_DEltX1 = HUF_DEltX1 { nbBits: 0, byte: 0 };
                    D_0.byte = (*wksp).symbols[(symbol + s) as usize];
                    D_0.nbBits = nbBits;
                    *dt.offset((uStart + 0 as ::core::ffi::c_int) as isize) = D_0;
                    *dt.offset((uStart + 1 as ::core::ffi::c_int) as isize) = D_0;
                    uStart += 2 as ::core::ffi::c_int;
                    s += 1;
                }
            }
            4 => {
                s = 0 as ::core::ffi::c_int;
                while s < symbolCount {
                    let D4: U64 =
                        HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits) as U64;
                    MEM_write64(dt.offset(uStart as isize) as *mut ::core::ffi::c_void, D4);
                    uStart += 4 as ::core::ffi::c_int;
                    s += 1;
                }
            }
            8 => {
                s = 0 as ::core::ffi::c_int;
                while s < symbolCount {
                    let D4_0: U64 =
                        HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits) as U64;
                    MEM_write64(dt.offset(uStart as isize) as *mut ::core::ffi::c_void, D4_0);
                    MEM_write64(
                        dt.offset(uStart as isize)
                            .offset(4 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        D4_0,
                    );
                    uStart += 8 as ::core::ffi::c_int;
                    s += 1;
                }
            }
            _ => {
                s = 0 as ::core::ffi::c_int;
                while s < symbolCount {
                    let D4_1: U64 =
                        HUF_DEltX1_set4((*wksp).symbols[(symbol + s) as usize], nbBits) as U64;
                    u_0 = 0 as ::core::ffi::c_int;
                    while u_0 < length {
                        MEM_write64(
                            dt.offset(uStart as isize)
                                .offset(u_0 as isize)
                                .offset(0 as ::core::ffi::c_int as isize)
                                as *mut ::core::ffi::c_void,
                            D4_1,
                        );
                        MEM_write64(
                            dt.offset(uStart as isize)
                                .offset(u_0 as isize)
                                .offset(4 as ::core::ffi::c_int as isize)
                                as *mut ::core::ffi::c_void,
                            D4_1,
                        );
                        MEM_write64(
                            dt.offset(uStart as isize)
                                .offset(u_0 as isize)
                                .offset(8 as ::core::ffi::c_int as isize)
                                as *mut ::core::ffi::c_void,
                            D4_1,
                        );
                        MEM_write64(
                            dt.offset(uStart as isize)
                                .offset(u_0 as isize)
                                .offset(12 as ::core::ffi::c_int as isize)
                                as *mut ::core::ffi::c_void,
                            D4_1,
                        );
                        u_0 += 16 as ::core::ffi::c_int;
                    }
                    uStart += length;
                    s += 1;
                }
            }
        }
        symbol += symbolCount;
        rankStart += symbolCount * length;
        w_1 = w_1.wrapping_add(1);
    }
    return iSize;
}
#[inline(always)]
unsafe extern "C" fn HUF_decodeSymbolX1(
    mut Dstream: *mut BIT_DStream_t,
    mut dt: *const HUF_DEltX1,
    dtLog: U32,
) -> BYTE {
    let val: size_t = BIT_lookBitsFast(Dstream, dtLog) as size_t;
    let c: BYTE = (*dt.offset(val as isize)).byte;
    BIT_skipBits(Dstream, (*dt.offset(val as isize)).nbBits as U32);
    return c;
}
#[inline(always)]
unsafe extern "C" fn HUF_decodeStreamX1(
    mut p: *mut BYTE,
    bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX1,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;
    if pEnd.offset_from(p) as ::core::ffi::c_long > 3 as ::core::ffi::c_long {
        while (BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
            == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
            as ::core::ffi::c_int
            & (p < pEnd.offset(-(3 as ::core::ffi::c_int as isize))) as ::core::ffi::c_int
            != 0
        {
            if MEM_64bits() != 0 {
                let fresh0 = p;
                p = p.offset(1);
                *fresh0 = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                let fresh1 = p;
                p = p.offset(1);
                *fresh1 = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh2 = p;
                p = p.offset(1);
                *fresh2 = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
            }
            let fresh3 = p;
            p = p.offset(1);
            *fresh3 = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }
    if MEM_32bits() != 0 {
        while (BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
            == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
            as ::core::ffi::c_int
            & (p < pEnd) as ::core::ffi::c_int
            != 0
        {
            let fresh4 = p;
            p = p.offset(1);
            *fresh4 = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
        }
    }
    while p < pEnd {
        let fresh5 = p;
        p = p.offset(1);
        *fresh5 = HUF_decodeSymbolX1(bitDPtr, dt, dtLog);
    }
    return pEnd.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
#[inline(always)]
unsafe extern "C" fn HUF_decompress1X1_usingDTable_internal_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE =
        ZSTD_maybeNullPtrAdd(op as *mut ::core::ffi::c_uchar, dstSize as ptrdiff_t) as *mut BYTE;
    let mut dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUF_DEltX1 = dtPtr as *const HUF_DEltX1;
    let mut bitD: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let dtd: DTableDesc = HUF_getDTableDesc(DTable) as DTableDesc;
    let dtLog: U32 = dtd.tableLog as U32;
    let _var_err__: size_t = BIT_initDStream(&raw mut bitD, cSrc, cSrcSize) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    HUF_decodeStreamX1(op, &raw mut bitD, oend, dt, dtLog);
    if BIT_endOfDStream(&raw mut bitD) == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
#[inline(always)]
unsafe extern "C" fn HUF_decompress4X1_usingDTable_internal_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    if cSrcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if dstSize < 6 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let olimit: *mut BYTE = oend.offset(-(3 as ::core::ffi::c_int as isize));
    let dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUF_DEltX1 = dtPtr as *const HUF_DEltX1;
    let mut bitD1: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
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
    let dtd: DTableDesc = HUF_getDTableDesc(DTable) as DTableDesc;
    let dtLog: U32 = dtd.tableLog as U32;
    let mut endSignal: U32 = 1 as U32;
    if length4 > cSrcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if opStart4 > oend {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let _var_err__: size_t = BIT_initDStream(
        &raw mut bitD1,
        istart1 as *const ::core::ffi::c_void,
        length1,
    ) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    let _var_err___0: size_t = BIT_initDStream(
        &raw mut bitD2,
        istart2 as *const ::core::ffi::c_void,
        length2,
    ) as size_t;
    if ERR_isError(_var_err___0) != 0 {
        return _var_err___0;
    }
    let _var_err___1: size_t = BIT_initDStream(
        &raw mut bitD3,
        istart3 as *const ::core::ffi::c_void,
        length3,
    ) as size_t;
    if ERR_isError(_var_err___1) != 0 {
        return _var_err___1;
    }
    let _var_err___2: size_t = BIT_initDStream(
        &raw mut bitD4,
        istart4 as *const ::core::ffi::c_void,
        length4,
    ) as size_t;
    if ERR_isError(_var_err___2) != 0 {
        return _var_err___2;
    }
    if oend.offset_from(op4) as ::core::ffi::c_long as size_t
        >= ::core::mem::size_of::<size_t>() as usize
    {
        while endSignal & (op4 < olimit) as ::core::ffi::c_int as U32 != 0 {
            if MEM_64bits() != 0 {
                let fresh12 = op1;
                op1 = op1.offset(1);
                *fresh12 = HUF_decodeSymbolX1(&raw mut bitD1, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh13 = op2;
                op2 = op2.offset(1);
                *fresh13 = HUF_decodeSymbolX1(&raw mut bitD2, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh14 = op3;
                op3 = op3.offset(1);
                *fresh14 = HUF_decodeSymbolX1(&raw mut bitD3, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh15 = op4;
                op4 = op4.offset(1);
                *fresh15 = HUF_decodeSymbolX1(&raw mut bitD4, dt, dtLog);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                let fresh16 = op1;
                op1 = op1.offset(1);
                *fresh16 = HUF_decodeSymbolX1(&raw mut bitD1, dt, dtLog);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                let fresh17 = op2;
                op2 = op2.offset(1);
                *fresh17 = HUF_decodeSymbolX1(&raw mut bitD2, dt, dtLog);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                let fresh18 = op3;
                op3 = op3.offset(1);
                *fresh18 = HUF_decodeSymbolX1(&raw mut bitD3, dt, dtLog);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                let fresh19 = op4;
                op4 = op4.offset(1);
                *fresh19 = HUF_decodeSymbolX1(&raw mut bitD4, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh20 = op1;
                op1 = op1.offset(1);
                *fresh20 = HUF_decodeSymbolX1(&raw mut bitD1, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh21 = op2;
                op2 = op2.offset(1);
                *fresh21 = HUF_decodeSymbolX1(&raw mut bitD2, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh22 = op3;
                op3 = op3.offset(1);
                *fresh22 = HUF_decodeSymbolX1(&raw mut bitD3, dt, dtLog);
            }
            if MEM_64bits() != 0 {
                let fresh23 = op4;
                op4 = op4.offset(1);
                *fresh23 = HUF_decodeSymbolX1(&raw mut bitD4, dt, dtLog);
            }
            let fresh24 = op1;
            op1 = op1.offset(1);
            *fresh24 = HUF_decodeSymbolX1(&raw mut bitD1, dt, dtLog);
            let fresh25 = op2;
            op2 = op2.offset(1);
            *fresh25 = HUF_decodeSymbolX1(&raw mut bitD2, dt, dtLog);
            let fresh26 = op3;
            op3 = op3.offset(1);
            *fresh26 = HUF_decodeSymbolX1(&raw mut bitD3, dt, dtLog);
            let fresh27 = op4;
            op4 = op4.offset(1);
            *fresh27 = HUF_decodeSymbolX1(&raw mut bitD4, dt, dtLog);
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD1) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD2) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD3) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD4) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
        }
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
    HUF_decodeStreamX1(op1, &raw mut bitD1, opStart2, dt, dtLog);
    HUF_decodeStreamX1(op2, &raw mut bitD2, opStart3, dt, dtLog);
    HUF_decodeStreamX1(op3, &raw mut bitD3, opStart4, dt, dtLog);
    HUF_decodeStreamX1(op4, &raw mut bitD4, oend, dt, dtLog);
    let endCheck: U32 = BIT_endOfDStream(&raw mut bitD1) as U32
        & BIT_endOfDStream(&raw mut bitD2) as U32
        & BIT_endOfDStream(&raw mut bitD3) as U32
        & BIT_endOfDStream(&raw mut bitD4) as U32;
    if endCheck == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
unsafe extern "C" fn HUF_decompress4X1_usingDTable_internal_default(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    return HUF_decompress4X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable);
}
unsafe extern "C" fn HUF_decompress4X1_usingDTable_internal_fast_c_loop(
    mut args: *mut HUF_DecompressFastArgs,
) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [::core::ptr::null::<BYTE>(); 4];
    let mut op: [*mut BYTE; 4] = [::core::ptr::null_mut::<BYTE>(); 4];
    let dtable: *const U16 = (*args).dt as *const U16;
    let oend: *mut BYTE = (*args).oend;
    let ilowest: *const BYTE = (*args).ilowest;
    ::libc::memcpy(
        &raw mut bits as *mut ::core::ffi::c_void,
        &raw mut (*args).bits as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U64; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut ip as *mut ::core::ffi::c_void,
        &raw mut (*args).ip as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*const BYTE; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut op as *mut ::core::ffi::c_void,
        &raw mut (*args).op as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*mut BYTE; 4]>() as ::libc::size_t,
    );
    's_33: loop {
        let mut olimit: *mut BYTE = ::core::ptr::null_mut::<BYTE>();
        let mut stream: ::core::ffi::c_int = 0;
        let oiters: size_t = (oend.offset_from(op[3 as ::core::ffi::c_int as usize])
            as ::core::ffi::c_long as size_t)
            .wrapping_div(5 as size_t);
        let iiters: size_t = (ip[0 as ::core::ffi::c_int as usize].offset_from(ilowest)
            as ::core::ffi::c_long as size_t)
            .wrapping_div(7 as size_t);
        let iters: size_t = if oiters < iiters { oiters } else { iiters };
        let symbols: size_t = iters.wrapping_mul(5 as size_t);
        olimit = op[3 as ::core::ffi::c_int as usize].offset(symbols as isize);
        if op[3 as ::core::ffi::c_int as usize] == olimit {
            break;
        }
        stream = 1 as ::core::ffi::c_int;
        while stream < 4 as ::core::ffi::c_int {
            if ip[stream as usize] < ip[(stream - 1 as ::core::ffi::c_int) as usize] {
                break 's_33;
            }
            stream += 1;
        }
        loop {
            let index: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry: ::core::ffi::c_int = *dtable.offset(index as isize) as ::core::ffi::c_int;
            bits[0 as ::core::ffi::c_int as usize] <<= entry & 0x3f as ::core::ffi::c_int;
            *op[0 as ::core::ffi::c_int as usize].offset(0 as ::core::ffi::c_int as isize) =
                (entry >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_0: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_0: ::core::ffi::c_int =
                *dtable.offset(index_0 as isize) as ::core::ffi::c_int;
            bits[1 as ::core::ffi::c_int as usize] <<= entry_0 & 0x3f as ::core::ffi::c_int;
            *op[1 as ::core::ffi::c_int as usize].offset(0 as ::core::ffi::c_int as isize) =
                (entry_0 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_1: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_1: ::core::ffi::c_int =
                *dtable.offset(index_1 as isize) as ::core::ffi::c_int;
            bits[2 as ::core::ffi::c_int as usize] <<= entry_1 & 0x3f as ::core::ffi::c_int;
            *op[2 as ::core::ffi::c_int as usize].offset(0 as ::core::ffi::c_int as isize) =
                (entry_1 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_2: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_2: ::core::ffi::c_int =
                *dtable.offset(index_2 as isize) as ::core::ffi::c_int;
            bits[3 as ::core::ffi::c_int as usize] <<= entry_2 & 0x3f as ::core::ffi::c_int;
            *op[3 as ::core::ffi::c_int as usize].offset(0 as ::core::ffi::c_int as isize) =
                (entry_2 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_3: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_3: ::core::ffi::c_int =
                *dtable.offset(index_3 as isize) as ::core::ffi::c_int;
            bits[0 as ::core::ffi::c_int as usize] <<= entry_3 & 0x3f as ::core::ffi::c_int;
            *op[0 as ::core::ffi::c_int as usize].offset(1 as ::core::ffi::c_int as isize) =
                (entry_3 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_4: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_4: ::core::ffi::c_int =
                *dtable.offset(index_4 as isize) as ::core::ffi::c_int;
            bits[1 as ::core::ffi::c_int as usize] <<= entry_4 & 0x3f as ::core::ffi::c_int;
            *op[1 as ::core::ffi::c_int as usize].offset(1 as ::core::ffi::c_int as isize) =
                (entry_4 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_5: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_5: ::core::ffi::c_int =
                *dtable.offset(index_5 as isize) as ::core::ffi::c_int;
            bits[2 as ::core::ffi::c_int as usize] <<= entry_5 & 0x3f as ::core::ffi::c_int;
            *op[2 as ::core::ffi::c_int as usize].offset(1 as ::core::ffi::c_int as isize) =
                (entry_5 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_6: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_6: ::core::ffi::c_int =
                *dtable.offset(index_6 as isize) as ::core::ffi::c_int;
            bits[3 as ::core::ffi::c_int as usize] <<= entry_6 & 0x3f as ::core::ffi::c_int;
            *op[3 as ::core::ffi::c_int as usize].offset(1 as ::core::ffi::c_int as isize) =
                (entry_6 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_7: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_7: ::core::ffi::c_int =
                *dtable.offset(index_7 as isize) as ::core::ffi::c_int;
            bits[0 as ::core::ffi::c_int as usize] <<= entry_7 & 0x3f as ::core::ffi::c_int;
            *op[0 as ::core::ffi::c_int as usize].offset(2 as ::core::ffi::c_int as isize) =
                (entry_7 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_8: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_8: ::core::ffi::c_int =
                *dtable.offset(index_8 as isize) as ::core::ffi::c_int;
            bits[1 as ::core::ffi::c_int as usize] <<= entry_8 & 0x3f as ::core::ffi::c_int;
            *op[1 as ::core::ffi::c_int as usize].offset(2 as ::core::ffi::c_int as isize) =
                (entry_8 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_9: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_9: ::core::ffi::c_int =
                *dtable.offset(index_9 as isize) as ::core::ffi::c_int;
            bits[2 as ::core::ffi::c_int as usize] <<= entry_9 & 0x3f as ::core::ffi::c_int;
            *op[2 as ::core::ffi::c_int as usize].offset(2 as ::core::ffi::c_int as isize) =
                (entry_9 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_10: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_10: ::core::ffi::c_int =
                *dtable.offset(index_10 as isize) as ::core::ffi::c_int;
            bits[3 as ::core::ffi::c_int as usize] <<= entry_10 & 0x3f as ::core::ffi::c_int;
            *op[3 as ::core::ffi::c_int as usize].offset(2 as ::core::ffi::c_int as isize) =
                (entry_10 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_11: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_11: ::core::ffi::c_int =
                *dtable.offset(index_11 as isize) as ::core::ffi::c_int;
            bits[0 as ::core::ffi::c_int as usize] <<= entry_11 & 0x3f as ::core::ffi::c_int;
            *op[0 as ::core::ffi::c_int as usize].offset(3 as ::core::ffi::c_int as isize) =
                (entry_11 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_12: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_12: ::core::ffi::c_int =
                *dtable.offset(index_12 as isize) as ::core::ffi::c_int;
            bits[1 as ::core::ffi::c_int as usize] <<= entry_12 & 0x3f as ::core::ffi::c_int;
            *op[1 as ::core::ffi::c_int as usize].offset(3 as ::core::ffi::c_int as isize) =
                (entry_12 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_13: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_13: ::core::ffi::c_int =
                *dtable.offset(index_13 as isize) as ::core::ffi::c_int;
            bits[2 as ::core::ffi::c_int as usize] <<= entry_13 & 0x3f as ::core::ffi::c_int;
            *op[2 as ::core::ffi::c_int as usize].offset(3 as ::core::ffi::c_int as isize) =
                (entry_13 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_14: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_14: ::core::ffi::c_int =
                *dtable.offset(index_14 as isize) as ::core::ffi::c_int;
            bits[3 as ::core::ffi::c_int as usize] <<= entry_14 & 0x3f as ::core::ffi::c_int;
            *op[3 as ::core::ffi::c_int as usize].offset(3 as ::core::ffi::c_int as isize) =
                (entry_14 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_15: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_15: ::core::ffi::c_int =
                *dtable.offset(index_15 as isize) as ::core::ffi::c_int;
            bits[0 as ::core::ffi::c_int as usize] <<= entry_15 & 0x3f as ::core::ffi::c_int;
            *op[0 as ::core::ffi::c_int as usize].offset(4 as ::core::ffi::c_int as isize) =
                (entry_15 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_16: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_16: ::core::ffi::c_int =
                *dtable.offset(index_16 as isize) as ::core::ffi::c_int;
            bits[1 as ::core::ffi::c_int as usize] <<= entry_16 & 0x3f as ::core::ffi::c_int;
            *op[1 as ::core::ffi::c_int as usize].offset(4 as ::core::ffi::c_int as isize) =
                (entry_16 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_17: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_17: ::core::ffi::c_int =
                *dtable.offset(index_17 as isize) as ::core::ffi::c_int;
            bits[2 as ::core::ffi::c_int as usize] <<= entry_17 & 0x3f as ::core::ffi::c_int;
            *op[2 as ::core::ffi::c_int as usize].offset(4 as ::core::ffi::c_int as isize) =
                (entry_17 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let index_18: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                >> 53 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let entry_18: ::core::ffi::c_int =
                *dtable.offset(index_18 as isize) as ::core::ffi::c_int;
            bits[3 as ::core::ffi::c_int as usize] <<= entry_18 & 0x3f as ::core::ffi::c_int;
            *op[3 as ::core::ffi::c_int as usize].offset(4 as ::core::ffi::c_int as isize) =
                (entry_18 >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as BYTE;
            let ctz: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[0 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits: ::core::ffi::c_int = ctz & 7 as ::core::ffi::c_int;
            let nbBytes: ::core::ffi::c_int = ctz >> 3 as ::core::ffi::c_int;
            op[0 as ::core::ffi::c_int as usize] =
                op[0 as ::core::ffi::c_int as usize].offset(5 as ::core::ffi::c_int as isize);
            ip[0 as ::core::ffi::c_int as usize] =
                ip[0 as ::core::ffi::c_int as usize].offset(-(nbBytes as isize));
            bits[0 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[0 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[0 as ::core::ffi::c_int as usize] <<= nbBits;
            let ctz_0: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[1 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits_0: ::core::ffi::c_int = ctz_0 & 7 as ::core::ffi::c_int;
            let nbBytes_0: ::core::ffi::c_int = ctz_0 >> 3 as ::core::ffi::c_int;
            op[1 as ::core::ffi::c_int as usize] =
                op[1 as ::core::ffi::c_int as usize].offset(5 as ::core::ffi::c_int as isize);
            ip[1 as ::core::ffi::c_int as usize] =
                ip[1 as ::core::ffi::c_int as usize].offset(-(nbBytes_0 as isize));
            bits[1 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[1 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[1 as ::core::ffi::c_int as usize] <<= nbBits_0;
            let ctz_1: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[2 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits_1: ::core::ffi::c_int = ctz_1 & 7 as ::core::ffi::c_int;
            let nbBytes_1: ::core::ffi::c_int = ctz_1 >> 3 as ::core::ffi::c_int;
            op[2 as ::core::ffi::c_int as usize] =
                op[2 as ::core::ffi::c_int as usize].offset(5 as ::core::ffi::c_int as isize);
            ip[2 as ::core::ffi::c_int as usize] =
                ip[2 as ::core::ffi::c_int as usize].offset(-(nbBytes_1 as isize));
            bits[2 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[2 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[2 as ::core::ffi::c_int as usize] <<= nbBits_1;
            let ctz_2: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[3 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits_2: ::core::ffi::c_int = ctz_2 & 7 as ::core::ffi::c_int;
            let nbBytes_2: ::core::ffi::c_int = ctz_2 >> 3 as ::core::ffi::c_int;
            op[3 as ::core::ffi::c_int as usize] =
                op[3 as ::core::ffi::c_int as usize].offset(5 as ::core::ffi::c_int as isize);
            ip[3 as ::core::ffi::c_int as usize] =
                ip[3 as ::core::ffi::c_int as usize].offset(-(nbBytes_2 as isize));
            bits[3 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[3 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[3 as ::core::ffi::c_int as usize] <<= nbBits_2;
            if !(op[3 as ::core::ffi::c_int as usize] < olimit) {
                break;
            }
        }
    }
    ::libc::memcpy(
        &raw mut (*args).bits as *mut ::core::ffi::c_void,
        &raw mut bits as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U64; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut (*args).ip as *mut ::core::ffi::c_void,
        &raw mut ip as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*const BYTE; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut (*args).op as *mut ::core::ffi::c_void,
        &raw mut op as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*mut BYTE; 4]>() as ::libc::size_t,
    );
}
unsafe extern "C" fn HUF_decompress4X1_usingDTable_internal_fast(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut loopFn: HUF_DecompressFastLoopFn,
) -> size_t {
    let mut dt: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let ilowest: *const BYTE = cSrc as *const BYTE;
    let oend: *mut BYTE =
        ZSTD_maybeNullPtrAdd(dst as *mut ::core::ffi::c_uchar, dstSize as ptrdiff_t) as *mut BYTE;
    let mut args: HUF_DecompressFastArgs = HUF_DecompressFastArgs {
        ip: [::core::ptr::null::<BYTE>(); 4],
        op: [::core::ptr::null_mut::<BYTE>(); 4],
        bits: [0; 4],
        dt: ::core::ptr::null::<::core::ffi::c_void>(),
        ilowest: ::core::ptr::null::<BYTE>(),
        oend: ::core::ptr::null_mut::<BYTE>(),
        iend: [::core::ptr::null::<BYTE>(); 4],
    };
    let ret: size_t =
        HUF_DecompressFastArgs_init(&raw mut args, dst, dstSize, cSrc, cSrcSize, DTable) as size_t;
    let err_code: size_t = ret;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    if ret == 0 as size_t {
        return 0 as size_t;
    }
    loopFn.expect("non-null function pointer")(&raw mut args);
    let segmentSize: size_t = dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t);
    let mut segmentEnd: *mut BYTE = dst as *mut BYTE;
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        let mut bit: BIT_DStream_t = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: ::core::ptr::null::<::core::ffi::c_char>(),
            start: ::core::ptr::null::<::core::ffi::c_char>(),
            limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
        };
        if segmentSize <= oend.offset_from(segmentEnd) as ::core::ffi::c_long as size_t {
            segmentEnd = segmentEnd.offset(segmentSize as isize);
        } else {
            segmentEnd = oend;
        }
        let err_code_0: size_t =
            HUF_initRemainingDStream(&raw mut bit, &raw mut args, i, segmentEnd) as size_t;
        if ERR_isError(err_code_0) != 0 {
            return err_code_0;
        }
        args.op[i as usize] = args.op[i as usize].offset(HUF_decodeStreamX1(
            args.op[i as usize],
            &raw mut bit,
            segmentEnd,
            dt as *const HUF_DEltX1,
            HUF_DECODER_FAST_TABLELOG as U32,
        ) as isize);
        if args.op[i as usize] != segmentEnd {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        i += 1;
    }
    return dstSize;
}
unsafe extern "C" fn HUF_decompress1X1_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_decompress1X1_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable);
}
unsafe extern "C" fn HUF_decompress4X1_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut fallbackFn: HUF_DecompressUsingDTableFn = Some(
        HUF_decompress4X1_usingDTable_internal_default
            as unsafe extern "C" fn(
                *mut ::core::ffi::c_void,
                size_t,
                *const ::core::ffi::c_void,
                size_t,
                *const HUF_DTable,
            ) -> size_t,
    );
    let mut loopFn: HUF_DecompressFastLoopFn = Some(
        HUF_decompress4X1_usingDTable_internal_fast_c_loop
            as unsafe extern "C" fn(*mut HUF_DecompressFastArgs) -> (),
    );
    if HUF_ENABLE_FAST_DECODE != 0 && flags & HUF_flags_disableFast as ::core::ffi::c_int == 0 {
        let ret: size_t = HUF_decompress4X1_usingDTable_internal_fast(
            dst, dstSize, cSrc, cSrcSize, DTable, loopFn,
        ) as size_t;
        if ret != 0 as size_t {
            return ret;
        }
    }
    return fallbackFn.expect("non-null function pointer")(dst, dstSize, cSrc, cSrcSize, DTable);
}
unsafe extern "C" fn HUF_decompress4X1_DCtx_wksp(
    mut dctx: *mut HUF_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let hSize: size_t =
        HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags) as size_t;
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress4X1_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        dctx,
        flags,
    );
}
unsafe extern "C" fn HUF_buildDEltX2U32(
    mut symbol: U32,
    mut nbBits: U32,
    mut baseSeq: U32,
    mut level: ::core::ffi::c_int,
) -> U32 {
    let mut seq: U32 = 0;
    if MEM_isLittleEndian() != 0 {
        seq = if level == 1 as ::core::ffi::c_int {
            symbol
        } else {
            baseSeq.wrapping_add(symbol << 8 as ::core::ffi::c_int)
        };
        return seq
            .wrapping_add(nbBits << 16 as ::core::ffi::c_int)
            .wrapping_add((level as U32) << 24 as ::core::ffi::c_int);
    } else {
        seq = if level == 1 as ::core::ffi::c_int {
            symbol << 8 as ::core::ffi::c_int
        } else {
            (baseSeq << 8 as ::core::ffi::c_int).wrapping_add(symbol)
        };
        return (seq << 16 as ::core::ffi::c_int)
            .wrapping_add(nbBits << 8 as ::core::ffi::c_int)
            .wrapping_add(level as U32);
    };
}
unsafe extern "C" fn HUF_buildDEltX2(
    mut symbol: U32,
    mut nbBits: U32,
    mut baseSeq: U32,
    mut level: ::core::ffi::c_int,
) -> HUF_DEltX2 {
    let mut DElt: HUF_DEltX2 = HUF_DEltX2 {
        sequence: 0,
        nbBits: 0,
        length: 0,
    };
    let val: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq, level) as U32;
    ::libc::memcpy(
        &raw mut DElt as *mut ::core::ffi::c_void,
        &raw const val as *const ::core::ffi::c_void,
        ::core::mem::size_of::<U32>() as ::libc::size_t,
    );
    return DElt;
}
unsafe extern "C" fn HUF_buildDEltX2U64(
    mut symbol: U32,
    mut nbBits: U32,
    mut baseSeq: U16,
    mut level: ::core::ffi::c_int,
) -> U64 {
    let mut DElt: U32 = HUF_buildDEltX2U32(symbol, nbBits, baseSeq as U32, level);
    return (DElt as U64).wrapping_add((DElt as U64) << 32 as ::core::ffi::c_int);
}
unsafe extern "C" fn HUF_fillDTableX2ForWeight(
    mut DTableRank: *mut HUF_DEltX2,
    mut begin: *const sortedSymbol_t,
    mut end: *const sortedSymbol_t,
    mut nbBits: U32,
    mut tableLog: U32,
    mut baseSeq: U16,
    level: ::core::ffi::c_int,
) {
    let length: U32 = (1 as U32) << (tableLog.wrapping_sub(nbBits) & 0x1f as U32);
    let mut ptr: *const sortedSymbol_t = ::core::ptr::null::<sortedSymbol_t>();
    match length {
        1 => {
            ptr = begin;
            while ptr != end {
                let DElt: HUF_DEltX2 =
                    HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level)
                        as HUF_DEltX2;
                let fresh11 = DTableRank;
                DTableRank = DTableRank.offset(1);
                *fresh11 = DElt;
                ptr = ptr.offset(1);
            }
        }
        2 => {
            ptr = begin;
            while ptr != end {
                let DElt_0: HUF_DEltX2 =
                    HUF_buildDEltX2((*ptr).symbol as U32, nbBits, baseSeq as U32, level)
                        as HUF_DEltX2;
                *DTableRank.offset(0 as ::core::ffi::c_int as isize) = DElt_0;
                *DTableRank.offset(1 as ::core::ffi::c_int as isize) = DElt_0;
                DTableRank = DTableRank.offset(2 as ::core::ffi::c_int as isize);
                ptr = ptr.offset(1);
            }
        }
        4 => {
            ptr = begin;
            while ptr != end {
                let DEltX2: U64 =
                    HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level) as U64;
                ::libc::memcpy(
                    DTableRank.offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                ::libc::memcpy(
                    DTableRank.offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                DTableRank = DTableRank.offset(4 as ::core::ffi::c_int as isize);
                ptr = ptr.offset(1);
            }
        }
        8 => {
            ptr = begin;
            while ptr != end {
                let DEltX2_0: U64 =
                    HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level) as U64;
                ::libc::memcpy(
                    DTableRank.offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2_0 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                ::libc::memcpy(
                    DTableRank.offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2_0 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                ::libc::memcpy(
                    DTableRank.offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2_0 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                ::libc::memcpy(
                    DTableRank.offset(6 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2_0 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                DTableRank = DTableRank.offset(8 as ::core::ffi::c_int as isize);
                ptr = ptr.offset(1);
            }
        }
        _ => {
            ptr = begin;
            while ptr != end {
                let DEltX2_1: U64 =
                    HUF_buildDEltX2U64((*ptr).symbol as U32, nbBits, baseSeq, level) as U64;
                let DTableRankEnd: *mut HUF_DEltX2 = DTableRank.offset(length as isize);
                while DTableRank != DTableRankEnd {
                    ::libc::memcpy(
                        DTableRank.offset(0 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2_1 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        DTableRank.offset(2 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2_1 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        DTableRank.offset(4 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2_1 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        DTableRank.offset(6 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2_1 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    DTableRank = DTableRank.offset(8 as ::core::ffi::c_int as isize);
                }
                ptr = ptr.offset(1);
            }
        }
    };
}
unsafe extern "C" fn HUF_fillDTableX2Level2(
    mut DTable: *mut HUF_DEltX2,
    mut targetLog: U32,
    consumedBits: U32,
    mut rankVal: *const U32,
    minWeight: ::core::ffi::c_int,
    maxWeight1: ::core::ffi::c_int,
    mut sortedSymbols: *const sortedSymbol_t,
    mut rankStart: *const U32,
    mut nbBitsBaseline: U32,
    mut baseSeq: U16,
) {
    if minWeight > 1 as ::core::ffi::c_int {
        let length: U32 = (1 as U32) << (targetLog.wrapping_sub(consumedBits) & 0x1f as U32);
        let DEltX2: U64 = HUF_buildDEltX2U64(
            baseSeq as U32,
            consumedBits,
            0 as U16,
            1 as ::core::ffi::c_int,
        ) as U64;
        let skipSize: ::core::ffi::c_int =
            *rankVal.offset(minWeight as isize) as ::core::ffi::c_int;
        match length {
            2 => {
                ::libc::memcpy(
                    DTable as *mut ::core::ffi::c_void,
                    &raw const DEltX2 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
            }
            4 => {
                ::libc::memcpy(
                    DTable.offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
                ::libc::memcpy(
                    DTable.offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
                    &raw const DEltX2 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<U64>() as ::libc::size_t,
                );
            }
            _ => {
                let mut i: ::core::ffi::c_int = 0;
                i = 0 as ::core::ffi::c_int;
                while i < skipSize {
                    ::libc::memcpy(
                        DTable
                            .offset(i as isize)
                            .offset(0 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        DTable
                            .offset(i as isize)
                            .offset(2 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        DTable
                            .offset(i as isize)
                            .offset(4 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    ::libc::memcpy(
                        DTable
                            .offset(i as isize)
                            .offset(6 as ::core::ffi::c_int as isize)
                            as *mut ::core::ffi::c_void,
                        &raw const DEltX2 as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<U64>() as ::libc::size_t,
                    );
                    i += 8 as ::core::ffi::c_int;
                }
            }
        }
    }
    let mut w: ::core::ffi::c_int = 0;
    w = minWeight;
    while w < maxWeight1 {
        let begin: ::core::ffi::c_int = *rankStart.offset(w as isize) as ::core::ffi::c_int;
        let end: ::core::ffi::c_int =
            *rankStart.offset((w + 1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(w as U32);
        let totalBits: U32 = nbBits.wrapping_add(consumedBits);
        HUF_fillDTableX2ForWeight(
            DTable.offset(*rankVal.offset(w as isize) as isize),
            sortedSymbols.offset(begin as isize),
            sortedSymbols.offset(end as isize),
            totalBits,
            targetLog,
            baseSeq,
            2 as ::core::ffi::c_int,
        );
        w += 1;
    }
}
unsafe extern "C" fn HUF_fillDTableX2(
    mut DTable: *mut HUF_DEltX2,
    targetLog: U32,
    mut sortedList: *const sortedSymbol_t,
    mut rankStart: *const U32,
    mut rankValOrigin: *mut rankValCol_t,
    maxWeight: U32,
    nbBitsBaseline: U32,
) {
    let rankVal: *mut U32 =
        &raw mut *rankValOrigin.offset(0 as ::core::ffi::c_int as isize) as *mut U32;
    let scaleLog: ::core::ffi::c_int = nbBitsBaseline.wrapping_sub(targetLog) as ::core::ffi::c_int;
    let minBits: U32 = nbBitsBaseline.wrapping_sub(maxWeight);
    let mut w: ::core::ffi::c_int = 0;
    let wEnd: ::core::ffi::c_int = maxWeight as ::core::ffi::c_int + 1 as ::core::ffi::c_int;
    w = 1 as ::core::ffi::c_int;
    while w < wEnd {
        let begin: ::core::ffi::c_int = *rankStart.offset(w as isize) as ::core::ffi::c_int;
        let end: ::core::ffi::c_int =
            *rankStart.offset((w + 1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int;
        let nbBits: U32 = nbBitsBaseline.wrapping_sub(w as U32);
        if targetLog.wrapping_sub(nbBits) >= minBits {
            let mut start: ::core::ffi::c_int = *rankVal.offset(w as isize) as ::core::ffi::c_int;
            let length: U32 = (1 as U32) << (targetLog.wrapping_sub(nbBits) & 0x1f as U32);
            let mut minWeight: ::core::ffi::c_int =
                nbBits.wrapping_add(scaleLog as U32) as ::core::ffi::c_int;
            let mut s: ::core::ffi::c_int = 0;
            if minWeight < 1 as ::core::ffi::c_int {
                minWeight = 1 as ::core::ffi::c_int;
            }
            s = begin;
            while s != end {
                HUF_fillDTableX2Level2(
                    DTable.offset(start as isize),
                    targetLog,
                    nbBits,
                    &raw mut *rankValOrigin.offset(nbBits as isize) as *mut U32,
                    minWeight,
                    wEnd,
                    sortedList,
                    rankStart,
                    nbBitsBaseline,
                    (*sortedList.offset(s as isize)).symbol as U16,
                );
                start = (start as ::core::ffi::c_uint).wrapping_add(length as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_int;
                s += 1;
            }
        } else {
            HUF_fillDTableX2ForWeight(
                DTable.offset(*rankVal.offset(w as isize) as isize),
                sortedList.offset(begin as isize),
                sortedList.offset(end as isize),
                nbBits,
                targetLog,
                0 as U16,
                1 as ::core::ffi::c_int,
            );
        }
        w += 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readDTableX2_wksp(
    mut DTable: *mut HUF_DTable,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut tableLog: U32 = 0;
    let mut maxW: U32 = 0;
    let mut nbSymbols: U32 = 0;
    let mut dtd: DTableDesc = HUF_getDTableDesc(DTable);
    let mut maxTableLog: U32 = dtd.maxTableLog as U32;
    let mut iSize: size_t = 0;
    let mut dtPtr: *mut ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void;
    let dt: *mut HUF_DEltX2 = dtPtr as *mut HUF_DEltX2;
    let mut rankStart: *mut U32 = ::core::ptr::null_mut::<U32>();
    let wksp: *mut HUF_ReadDTableX2_Workspace = workSpace as *mut HUF_ReadDTableX2_Workspace;
    if ::core::mem::size_of::<HUF_ReadDTableX2_Workspace>() as usize > wkspSize {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    rankStart = (&raw mut (*wksp).rankStart0 as *mut U32).offset(1 as ::core::ffi::c_int as isize);
    ::libc::memset(
        &raw mut (*wksp).rankStats as *mut U32 as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[U32; 13]>() as ::libc::size_t,
    );
    ::libc::memset(
        &raw mut (*wksp).rankStart0 as *mut U32 as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[U32; 15]>() as ::libc::size_t,
    );
    if maxTableLog > HUF_TABLELOG_MAX as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    iSize = HUF_readStats_wksp(
        &raw mut (*wksp).weightList as *mut BYTE,
        (HUF_SYMBOLVALUE_MAX + 1 as ::core::ffi::c_int) as size_t,
        &raw mut (*wksp).rankStats as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
        &raw mut (*wksp).calleeWksp as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 219]>() as size_t,
        flags,
    );
    if ERR_isError(iSize) != 0 {
        return iSize;
    }
    if tableLog > maxTableLog {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if tableLog <= HUF_DECODER_FAST_TABLELOG as U32
        && maxTableLog > HUF_DECODER_FAST_TABLELOG as U32
    {
        maxTableLog = HUF_DECODER_FAST_TABLELOG as U32;
    }
    maxW = tableLog;
    while (*wksp).rankStats[maxW as usize] == 0 as U32 {
        maxW = maxW.wrapping_sub(1);
    }
    let mut w: U32 = 0;
    let mut nextRankStart: U32 = 0 as U32;
    w = 1 as U32;
    while w < maxW.wrapping_add(1 as U32) {
        let mut curr: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add((*wksp).rankStats[w as usize] as ::core::ffi::c_uint)
            as U32 as U32;
        *rankStart.offset(w as isize) = curr;
        w = w.wrapping_add(1);
    }
    *rankStart.offset(0 as ::core::ffi::c_int as isize) = nextRankStart;
    *rankStart.offset(maxW.wrapping_add(1 as U32) as isize) = nextRankStart;
    let mut s: U32 = 0;
    s = 0 as U32;
    while s < nbSymbols {
        let w_0: U32 = (*wksp).weightList[s as usize] as U32;
        let ref mut fresh9 = *rankStart.offset(w_0 as isize);
        let fresh10 = *fresh9;
        *fresh9 = (*fresh9).wrapping_add(1);
        let r: U32 = fresh10;
        (*wksp).sortedSymbol[r as usize].symbol = s as BYTE;
        s = s.wrapping_add(1);
    }
    *rankStart.offset(0 as ::core::ffi::c_int as isize) = 0 as U32;
    let rankVal0: *mut U32 = &raw mut *(&raw mut (*wksp).rankVal as *mut rankValCol_t)
        .offset(0 as ::core::ffi::c_int as isize) as *mut U32;
    let rescale: ::core::ffi::c_int =
        maxTableLog.wrapping_sub(tableLog).wrapping_sub(1 as U32) as ::core::ffi::c_int;
    let mut nextRankVal: U32 = 0 as U32;
    let mut w_1: U32 = 0;
    w_1 = 1 as U32;
    while w_1 < maxW.wrapping_add(1 as U32) {
        let mut curr_0: U32 = nextRankVal;
        nextRankVal = (nextRankVal as ::core::ffi::c_uint).wrapping_add(
            ((*wksp).rankStats[w_1 as usize] << w_1.wrapping_add(rescale as U32))
                as ::core::ffi::c_uint,
        ) as U32 as U32;
        *rankVal0.offset(w_1 as isize) = curr_0;
        w_1 = w_1.wrapping_add(1);
    }
    let minBits: U32 = tableLog.wrapping_add(1 as U32).wrapping_sub(maxW);
    let mut consumed: U32 = 0;
    consumed = minBits;
    while consumed < maxTableLog.wrapping_sub(minBits).wrapping_add(1 as U32) {
        let rankValPtr: *mut U32 = &raw mut *(&raw mut (*wksp).rankVal as *mut rankValCol_t)
            .offset(consumed as isize) as *mut U32;
        let mut w_2: U32 = 0;
        w_2 = 1 as U32;
        while w_2 < maxW.wrapping_add(1 as U32) {
            *rankValPtr.offset(w_2 as isize) = *rankVal0.offset(w_2 as isize) >> consumed;
            w_2 = w_2.wrapping_add(1);
        }
        consumed = consumed.wrapping_add(1);
    }
    HUF_fillDTableX2(
        dt,
        maxTableLog,
        &raw mut (*wksp).sortedSymbol as *mut sortedSymbol_t,
        &raw mut (*wksp).rankStart0 as *mut U32,
        &raw mut (*wksp).rankVal as *mut rankValCol_t,
        maxW,
        tableLog.wrapping_add(1 as U32),
    );
    dtd.tableLog = maxTableLog as BYTE;
    dtd.tableType = 1 as BYTE;
    ::libc::memcpy(
        DTable as *mut ::core::ffi::c_void,
        &raw mut dtd as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DTableDesc>() as ::libc::size_t,
    );
    return iSize;
}
#[inline(always)]
unsafe extern "C" fn HUF_decodeSymbolX2(
    mut op: *mut ::core::ffi::c_void,
    mut DStream: *mut BIT_DStream_t,
    mut dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val: size_t = BIT_lookBitsFast(DStream, dtLog) as size_t;
    ::libc::memcpy(
        op,
        &raw const (*dt.offset(val as isize)).sequence as *const ::core::ffi::c_void,
        2 as ::core::ffi::c_int as ::core::ffi::c_ulong as ::libc::size_t,
    );
    BIT_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
    return (*dt.offset(val as isize)).length as U32;
}
#[inline(always)]
unsafe extern "C" fn HUF_decodeLastSymbolX2(
    mut op: *mut ::core::ffi::c_void,
    mut DStream: *mut BIT_DStream_t,
    mut dt: *const HUF_DEltX2,
    dtLog: U32,
) -> U32 {
    let val: size_t = BIT_lookBitsFast(DStream, dtLog) as size_t;
    ::libc::memcpy(
        op,
        &raw const (*dt.offset(val as isize)).sequence as *const ::core::ffi::c_void,
        1 as ::core::ffi::c_int as ::core::ffi::c_ulong as ::libc::size_t,
    );
    if (*dt.offset(val as isize)).length as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
        BIT_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
    } else if ((*DStream).bitsConsumed as usize)
        < (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
    {
        BIT_skipBits(DStream, (*dt.offset(val as isize)).nbBits as U32);
        if (*DStream).bitsConsumed as usize
            > (::core::mem::size_of::<BitContainerType>() as usize).wrapping_mul(8 as usize)
        {
            (*DStream).bitsConsumed = (::core::mem::size_of::<BitContainerType>() as usize)
                .wrapping_mul(8 as usize)
                as ::core::ffi::c_uint;
        }
    }
    return 1 as U32;
}
#[inline(always)]
unsafe extern "C" fn HUF_decodeStreamX2(
    mut p: *mut BYTE,
    mut bitDPtr: *mut BIT_DStream_t,
    pEnd: *mut BYTE,
    dt: *const HUF_DEltX2,
    dtLog: U32,
) -> size_t {
    let pStart: *mut BYTE = p;
    if pEnd.offset_from(p) as ::core::ffi::c_long as size_t
        >= ::core::mem::size_of::<BitContainerType>() as usize
    {
        if dtLog <= 11 as U32 && MEM_64bits() != 0 {
            while (BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
                == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                as ::core::ffi::c_int
                & (p < pEnd.offset(-(9 as ::core::ffi::c_int as isize))) as ::core::ffi::c_int
                != 0
            {
                p = p.offset(
                    HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
                );
                p = p.offset(
                    HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
                );
                p = p.offset(
                    HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
                );
                p = p.offset(
                    HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
                );
                p = p.offset(
                    HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
                );
            }
        } else {
            while (BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
                == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                as ::core::ffi::c_int
                & (p < pEnd.offset(
                    -((::core::mem::size_of::<BitContainerType>() as usize).wrapping_sub(1 as usize)
                        as isize),
                )) as ::core::ffi::c_int
                != 0
            {
                if MEM_64bits() != 0 {
                    p = p.offset(HUF_decodeSymbolX2(
                        p as *mut ::core::ffi::c_void,
                        bitDPtr,
                        dt,
                        dtLog,
                    ) as isize);
                }
                if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                    p = p.offset(HUF_decodeSymbolX2(
                        p as *mut ::core::ffi::c_void,
                        bitDPtr,
                        dt,
                        dtLog,
                    ) as isize);
                }
                if MEM_64bits() != 0 {
                    p = p.offset(HUF_decodeSymbolX2(
                        p as *mut ::core::ffi::c_void,
                        bitDPtr,
                        dt,
                        dtLog,
                    ) as isize);
                }
                p = p.offset(
                    HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
                );
            }
        }
    } else {
        BIT_reloadDStream(bitDPtr);
    }
    if pEnd.offset_from(p) as ::core::ffi::c_long as size_t >= 2 as size_t {
        while (BIT_reloadDStream(bitDPtr) as ::core::ffi::c_uint
            == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
            as ::core::ffi::c_int
            & (p <= pEnd.offset(-(2 as ::core::ffi::c_int as isize))) as ::core::ffi::c_int
            != 0
        {
            p = p.offset(
                HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
        while p <= pEnd.offset(-(2 as ::core::ffi::c_int as isize)) {
            p = p.offset(
                HUF_decodeSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
            );
        }
    }
    if p < pEnd {
        p = p.offset(
            HUF_decodeLastSymbolX2(p as *mut ::core::ffi::c_void, bitDPtr, dt, dtLog) as isize,
        );
    }
    return p.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
#[inline(always)]
unsafe extern "C" fn HUF_decompress1X2_usingDTable_internal_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    let mut bitD: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let _var_err__: size_t = BIT_initDStream(&raw mut bitD, cSrc, cSrcSize) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE =
        ZSTD_maybeNullPtrAdd(ostart as *mut ::core::ffi::c_uchar, dstSize as ptrdiff_t)
            as *mut BYTE;
    let dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUF_DEltX2 = dtPtr as *const HUF_DEltX2;
    let dtd: DTableDesc = HUF_getDTableDesc(DTable) as DTableDesc;
    HUF_decodeStreamX2(ostart, &raw mut bitD, oend, dt, dtd.tableLog as U32);
    if BIT_endOfDStream(&raw mut bitD) == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
#[inline(always)]
unsafe extern "C" fn HUF_decompress4X2_usingDTable_internal_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    if cSrcSize < 10 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if dstSize < 6 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let istart: *const BYTE = cSrc as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let olimit: *mut BYTE = oend
        .offset(-((::core::mem::size_of::<size_t>() as usize).wrapping_sub(1 as usize) as isize));
    let dtPtr: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let dt: *const HUF_DEltX2 = dtPtr as *const HUF_DEltX2;
    let mut bitD1: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD2: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD3: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut bitD4: BIT_DStream_t = BIT_DStream_t {
        bitContainer: 0,
        bitsConsumed: 0,
        ptr: ::core::ptr::null::<::core::ffi::c_char>(),
        start: ::core::ptr::null::<::core::ffi::c_char>(),
        limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
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
    let mut endSignal: U32 = 1 as U32;
    let dtd: DTableDesc = HUF_getDTableDesc(DTable) as DTableDesc;
    let dtLog: U32 = dtd.tableLog as U32;
    if length4 > cSrcSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if opStart4 > oend {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let _var_err__: size_t = BIT_initDStream(
        &raw mut bitD1,
        istart1 as *const ::core::ffi::c_void,
        length1,
    ) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    let _var_err___0: size_t = BIT_initDStream(
        &raw mut bitD2,
        istart2 as *const ::core::ffi::c_void,
        length2,
    ) as size_t;
    if ERR_isError(_var_err___0) != 0 {
        return _var_err___0;
    }
    let _var_err___1: size_t = BIT_initDStream(
        &raw mut bitD3,
        istart3 as *const ::core::ffi::c_void,
        length3,
    ) as size_t;
    if ERR_isError(_var_err___1) != 0 {
        return _var_err___1;
    }
    let _var_err___2: size_t = BIT_initDStream(
        &raw mut bitD4,
        istart4 as *const ::core::ffi::c_void,
        length4,
    ) as size_t;
    if ERR_isError(_var_err___2) != 0 {
        return _var_err___2;
    }
    if oend.offset_from(op4) as ::core::ffi::c_long as size_t
        >= ::core::mem::size_of::<size_t>() as usize
    {
        while endSignal & (op4 < olimit) as ::core::ffi::c_int as U32 != 0 {
            if MEM_64bits() != 0 {
                op1 = op1.offset(HUF_decodeSymbolX2(
                    op1 as *mut ::core::ffi::c_void,
                    &raw mut bitD1,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                op1 = op1.offset(HUF_decodeSymbolX2(
                    op1 as *mut ::core::ffi::c_void,
                    &raw mut bitD1,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 {
                op1 = op1.offset(HUF_decodeSymbolX2(
                    op1 as *mut ::core::ffi::c_void,
                    &raw mut bitD1,
                    dt,
                    dtLog,
                ) as isize);
            }
            op1 = op1.offset(HUF_decodeSymbolX2(
                op1 as *mut ::core::ffi::c_void,
                &raw mut bitD1,
                dt,
                dtLog,
            ) as isize);
            if MEM_64bits() != 0 {
                op2 = op2.offset(HUF_decodeSymbolX2(
                    op2 as *mut ::core::ffi::c_void,
                    &raw mut bitD2,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                op2 = op2.offset(HUF_decodeSymbolX2(
                    op2 as *mut ::core::ffi::c_void,
                    &raw mut bitD2,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 {
                op2 = op2.offset(HUF_decodeSymbolX2(
                    op2 as *mut ::core::ffi::c_void,
                    &raw mut bitD2,
                    dt,
                    dtLog,
                ) as isize);
            }
            op2 = op2.offset(HUF_decodeSymbolX2(
                op2 as *mut ::core::ffi::c_void,
                &raw mut bitD2,
                dt,
                dtLog,
            ) as isize);
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD1) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD2) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
            if MEM_64bits() != 0 {
                op3 = op3.offset(HUF_decodeSymbolX2(
                    op3 as *mut ::core::ffi::c_void,
                    &raw mut bitD3,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                op3 = op3.offset(HUF_decodeSymbolX2(
                    op3 as *mut ::core::ffi::c_void,
                    &raw mut bitD3,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 {
                op3 = op3.offset(HUF_decodeSymbolX2(
                    op3 as *mut ::core::ffi::c_void,
                    &raw mut bitD3,
                    dt,
                    dtLog,
                ) as isize);
            }
            op3 = op3.offset(HUF_decodeSymbolX2(
                op3 as *mut ::core::ffi::c_void,
                &raw mut bitD3,
                dt,
                dtLog,
            ) as isize);
            if MEM_64bits() != 0 {
                op4 = op4.offset(HUF_decodeSymbolX2(
                    op4 as *mut ::core::ffi::c_void,
                    &raw mut bitD4,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 || HUF_TABLELOG_MAX <= 12 as ::core::ffi::c_int {
                op4 = op4.offset(HUF_decodeSymbolX2(
                    op4 as *mut ::core::ffi::c_void,
                    &raw mut bitD4,
                    dt,
                    dtLog,
                ) as isize);
            }
            if MEM_64bits() != 0 {
                op4 = op4.offset(HUF_decodeSymbolX2(
                    op4 as *mut ::core::ffi::c_void,
                    &raw mut bitD4,
                    dt,
                    dtLog,
                ) as isize);
            }
            op4 = op4.offset(HUF_decodeSymbolX2(
                op4 as *mut ::core::ffi::c_void,
                &raw mut bitD4,
                dt,
                dtLog,
            ) as isize);
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD3) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
            endSignal = (endSignal as ::core::ffi::c_uint
                & (BIT_reloadDStreamFast(&raw mut bitD4) as ::core::ffi::c_uint
                    == BIT_DStream_unfinished as ::core::ffi::c_int as ::core::ffi::c_uint)
                    as ::core::ffi::c_int as ::core::ffi::c_uint) as U32;
        }
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
    let endCheck: U32 = BIT_endOfDStream(&raw mut bitD1) as U32
        & BIT_endOfDStream(&raw mut bitD2) as U32
        & BIT_endOfDStream(&raw mut bitD3) as U32
        & BIT_endOfDStream(&raw mut bitD4) as U32;
    if endCheck == 0 {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    return dstSize;
}
unsafe extern "C" fn HUF_decompress4X2_usingDTable_internal_default(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
) -> size_t {
    return HUF_decompress4X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable);
}
unsafe extern "C" fn HUF_decompress4X2_usingDTable_internal_fast_c_loop(
    mut args: *mut HUF_DecompressFastArgs,
) {
    let mut bits: [U64; 4] = [0; 4];
    let mut ip: [*const BYTE; 4] = [::core::ptr::null::<BYTE>(); 4];
    let mut op: [*mut BYTE; 4] = [::core::ptr::null_mut::<BYTE>(); 4];
    let mut oend: [*mut BYTE; 4] = [::core::ptr::null_mut::<BYTE>(); 4];
    let dtable: *const HUF_DEltX2 = (*args).dt as *const HUF_DEltX2;
    let ilowest: *const BYTE = (*args).ilowest;
    ::libc::memcpy(
        &raw mut bits as *mut ::core::ffi::c_void,
        &raw mut (*args).bits as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U64; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut ip as *mut ::core::ffi::c_void,
        &raw mut (*args).ip as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*const BYTE; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut op as *mut ::core::ffi::c_void,
        &raw mut (*args).op as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*mut BYTE; 4]>() as ::libc::size_t,
    );
    oend[0 as ::core::ffi::c_int as usize] = op[1 as ::core::ffi::c_int as usize];
    oend[1 as ::core::ffi::c_int as usize] = op[2 as ::core::ffi::c_int as usize];
    oend[2 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize];
    oend[3 as ::core::ffi::c_int as usize] = (*args).oend;
    's_45: loop {
        let mut olimit: *mut BYTE = ::core::ptr::null_mut::<BYTE>();
        let mut stream: ::core::ffi::c_int = 0;
        let mut iters: size_t = (ip[0 as ::core::ffi::c_int as usize].offset_from(ilowest)
            as ::core::ffi::c_long as size_t)
            .wrapping_div(7 as size_t);
        stream = 0 as ::core::ffi::c_int;
        while stream < 4 as ::core::ffi::c_int {
            let oiters: size_t = (oend[stream as usize].offset_from(op[stream as usize])
                as ::core::ffi::c_long as size_t)
                .wrapping_div(10 as size_t);
            iters = if iters < oiters { iters } else { oiters };
            stream += 1;
        }
        olimit =
            op[3 as ::core::ffi::c_int as usize].offset(iters.wrapping_mul(5 as size_t) as isize);
        if op[3 as ::core::ffi::c_int as usize] == olimit {
            break;
        }
        stream = 1 as ::core::ffi::c_int;
        while stream < 4 as ::core::ffi::c_int {
            if ip[stream as usize] < ip[(stream - 1 as ::core::ffi::c_int) as usize] {
                break 's_45;
            }
            stream += 1;
        }
        loop {
            if 0 as ::core::ffi::c_int != 0 || 0 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry: HUF_DEltX2 = *dtable.offset(index as isize);
                MEM_write16(
                    op[0 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry.sequence,
                );
                bits[0 as ::core::ffi::c_int as usize] <<=
                    entry.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[0 as ::core::ffi::c_int as usize] = op[0 as ::core::ffi::c_int as usize]
                    .offset(entry.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 1 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_0: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_0: HUF_DEltX2 = *dtable.offset(index_0 as isize);
                MEM_write16(
                    op[1 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_0.sequence,
                );
                bits[1 as ::core::ffi::c_int as usize] <<=
                    entry_0.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[1 as ::core::ffi::c_int as usize] = op[1 as ::core::ffi::c_int as usize]
                    .offset(entry_0.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 2 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_1: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_1: HUF_DEltX2 = *dtable.offset(index_1 as isize);
                MEM_write16(
                    op[2 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_1.sequence,
                );
                bits[2 as ::core::ffi::c_int as usize] <<=
                    entry_1.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[2 as ::core::ffi::c_int as usize] = op[2 as ::core::ffi::c_int as usize]
                    .offset(entry_1.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_2: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_2: HUF_DEltX2 = *dtable.offset(index_2 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_2.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_2.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_2.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 0 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_3: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_3: HUF_DEltX2 = *dtable.offset(index_3 as isize);
                MEM_write16(
                    op[0 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_3.sequence,
                );
                bits[0 as ::core::ffi::c_int as usize] <<=
                    entry_3.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[0 as ::core::ffi::c_int as usize] = op[0 as ::core::ffi::c_int as usize]
                    .offset(entry_3.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 1 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_4: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_4: HUF_DEltX2 = *dtable.offset(index_4 as isize);
                MEM_write16(
                    op[1 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_4.sequence,
                );
                bits[1 as ::core::ffi::c_int as usize] <<=
                    entry_4.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[1 as ::core::ffi::c_int as usize] = op[1 as ::core::ffi::c_int as usize]
                    .offset(entry_4.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 2 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_5: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_5: HUF_DEltX2 = *dtable.offset(index_5 as isize);
                MEM_write16(
                    op[2 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_5.sequence,
                );
                bits[2 as ::core::ffi::c_int as usize] <<=
                    entry_5.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[2 as ::core::ffi::c_int as usize] = op[2 as ::core::ffi::c_int as usize]
                    .offset(entry_5.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_6: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_6: HUF_DEltX2 = *dtable.offset(index_6 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_6.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_6.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_6.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 0 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_7: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_7: HUF_DEltX2 = *dtable.offset(index_7 as isize);
                MEM_write16(
                    op[0 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_7.sequence,
                );
                bits[0 as ::core::ffi::c_int as usize] <<=
                    entry_7.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[0 as ::core::ffi::c_int as usize] = op[0 as ::core::ffi::c_int as usize]
                    .offset(entry_7.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 1 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_8: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_8: HUF_DEltX2 = *dtable.offset(index_8 as isize);
                MEM_write16(
                    op[1 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_8.sequence,
                );
                bits[1 as ::core::ffi::c_int as usize] <<=
                    entry_8.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[1 as ::core::ffi::c_int as usize] = op[1 as ::core::ffi::c_int as usize]
                    .offset(entry_8.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 2 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_9: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_9: HUF_DEltX2 = *dtable.offset(index_9 as isize);
                MEM_write16(
                    op[2 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_9.sequence,
                );
                bits[2 as ::core::ffi::c_int as usize] <<=
                    entry_9.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[2 as ::core::ffi::c_int as usize] = op[2 as ::core::ffi::c_int as usize]
                    .offset(entry_9.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_10: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_10: HUF_DEltX2 = *dtable.offset(index_10 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_10.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_10.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_10.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 0 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_11: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_11: HUF_DEltX2 = *dtable.offset(index_11 as isize);
                MEM_write16(
                    op[0 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_11.sequence,
                );
                bits[0 as ::core::ffi::c_int as usize] <<=
                    entry_11.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[0 as ::core::ffi::c_int as usize] = op[0 as ::core::ffi::c_int as usize]
                    .offset(entry_11.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 1 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_12: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_12: HUF_DEltX2 = *dtable.offset(index_12 as isize);
                MEM_write16(
                    op[1 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_12.sequence,
                );
                bits[1 as ::core::ffi::c_int as usize] <<=
                    entry_12.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[1 as ::core::ffi::c_int as usize] = op[1 as ::core::ffi::c_int as usize]
                    .offset(entry_12.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 2 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_13: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_13: HUF_DEltX2 = *dtable.offset(index_13 as isize);
                MEM_write16(
                    op[2 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_13.sequence,
                );
                bits[2 as ::core::ffi::c_int as usize] <<=
                    entry_13.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[2 as ::core::ffi::c_int as usize] = op[2 as ::core::ffi::c_int as usize]
                    .offset(entry_13.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_14: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_14: HUF_DEltX2 = *dtable.offset(index_14 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_14.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_14.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_14.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 0 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_15: ::core::ffi::c_int = (bits[0 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_15: HUF_DEltX2 = *dtable.offset(index_15 as isize);
                MEM_write16(
                    op[0 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_15.sequence,
                );
                bits[0 as ::core::ffi::c_int as usize] <<=
                    entry_15.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[0 as ::core::ffi::c_int as usize] = op[0 as ::core::ffi::c_int as usize]
                    .offset(entry_15.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 1 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_16: ::core::ffi::c_int = (bits[1 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_16: HUF_DEltX2 = *dtable.offset(index_16 as isize);
                MEM_write16(
                    op[1 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_16.sequence,
                );
                bits[1 as ::core::ffi::c_int as usize] <<=
                    entry_16.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[1 as ::core::ffi::c_int as usize] = op[1 as ::core::ffi::c_int as usize]
                    .offset(entry_16.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 2 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_17: ::core::ffi::c_int = (bits[2 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_17: HUF_DEltX2 = *dtable.offset(index_17 as isize);
                MEM_write16(
                    op[2 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_17.sequence,
                );
                bits[2 as ::core::ffi::c_int as usize] <<=
                    entry_17.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[2 as ::core::ffi::c_int as usize] = op[2 as ::core::ffi::c_int as usize]
                    .offset(entry_17.length as ::core::ffi::c_int as isize);
            }
            if 0 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_18: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_18: HUF_DEltX2 = *dtable.offset(index_18 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_18.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_18.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_18.length as ::core::ffi::c_int as isize);
            }
            if 1 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_19: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_19: HUF_DEltX2 = *dtable.offset(index_19 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_19.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_19.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_19.length as ::core::ffi::c_int as isize);
            }
            if 1 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_20: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_20: HUF_DEltX2 = *dtable.offset(index_20 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_20.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_20.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_20.length as ::core::ffi::c_int as isize);
            }
            let ctz: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[0 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits: ::core::ffi::c_int = ctz & 7 as ::core::ffi::c_int;
            let nbBytes: ::core::ffi::c_int = ctz >> 3 as ::core::ffi::c_int;
            ip[0 as ::core::ffi::c_int as usize] =
                ip[0 as ::core::ffi::c_int as usize].offset(-(nbBytes as isize));
            bits[0 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[0 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[0 as ::core::ffi::c_int as usize] <<= nbBits;
            if 1 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_21: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_21: HUF_DEltX2 = *dtable.offset(index_21 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_21.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_21.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_21.length as ::core::ffi::c_int as isize);
            }
            let ctz_0: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[1 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits_0: ::core::ffi::c_int = ctz_0 & 7 as ::core::ffi::c_int;
            let nbBytes_0: ::core::ffi::c_int = ctz_0 >> 3 as ::core::ffi::c_int;
            ip[1 as ::core::ffi::c_int as usize] =
                ip[1 as ::core::ffi::c_int as usize].offset(-(nbBytes_0 as isize));
            bits[1 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[1 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[1 as ::core::ffi::c_int as usize] <<= nbBits_0;
            if 1 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_22: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_22: HUF_DEltX2 = *dtable.offset(index_22 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_22.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_22.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_22.length as ::core::ffi::c_int as isize);
            }
            let ctz_1: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[2 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits_1: ::core::ffi::c_int = ctz_1 & 7 as ::core::ffi::c_int;
            let nbBytes_1: ::core::ffi::c_int = ctz_1 >> 3 as ::core::ffi::c_int;
            ip[2 as ::core::ffi::c_int as usize] =
                ip[2 as ::core::ffi::c_int as usize].offset(-(nbBytes_1 as isize));
            bits[2 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[2 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[2 as ::core::ffi::c_int as usize] <<= nbBits_1;
            if 1 as ::core::ffi::c_int != 0 || 3 as ::core::ffi::c_int != 3 as ::core::ffi::c_int {
                let index_23: ::core::ffi::c_int = (bits[3 as ::core::ffi::c_int as usize]
                    >> 53 as ::core::ffi::c_int)
                    as ::core::ffi::c_int;
                let entry_23: HUF_DEltX2 = *dtable.offset(index_23 as isize);
                MEM_write16(
                    op[3 as ::core::ffi::c_int as usize] as *mut ::core::ffi::c_void,
                    entry_23.sequence,
                );
                bits[3 as ::core::ffi::c_int as usize] <<=
                    entry_23.nbBits as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int;
                op[3 as ::core::ffi::c_int as usize] = op[3 as ::core::ffi::c_int as usize]
                    .offset(entry_23.length as ::core::ffi::c_int as isize);
            }
            let ctz_2: ::core::ffi::c_int =
                ZSTD_countTrailingZeros64(bits[3 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_int;
            let nbBits_2: ::core::ffi::c_int = ctz_2 & 7 as ::core::ffi::c_int;
            let nbBytes_2: ::core::ffi::c_int = ctz_2 >> 3 as ::core::ffi::c_int;
            ip[3 as ::core::ffi::c_int as usize] =
                ip[3 as ::core::ffi::c_int as usize].offset(-(nbBytes_2 as isize));
            bits[3 as ::core::ffi::c_int as usize] =
                MEM_read64(ip[3 as ::core::ffi::c_int as usize] as *const ::core::ffi::c_void)
                    | 1 as U64;
            bits[3 as ::core::ffi::c_int as usize] <<= nbBits_2;
            if !(op[3 as ::core::ffi::c_int as usize] < olimit) {
                break;
            }
        }
    }
    ::libc::memcpy(
        &raw mut (*args).bits as *mut ::core::ffi::c_void,
        &raw mut bits as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U64; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut (*args).ip as *mut ::core::ffi::c_void,
        &raw mut ip as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*const BYTE; 4]>() as ::libc::size_t,
    );
    ::libc::memcpy(
        &raw mut (*args).op as *mut ::core::ffi::c_void,
        &raw mut op as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[*mut BYTE; 4]>() as ::libc::size_t,
    );
}
unsafe extern "C" fn HUF_decompress4X2_usingDTable_internal_fast(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut loopFn: HUF_DecompressFastLoopFn,
) -> size_t {
    let mut dt: *const ::core::ffi::c_void =
        DTable.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void;
    let ilowest: *const BYTE = cSrc as *const BYTE;
    let oend: *mut BYTE =
        ZSTD_maybeNullPtrAdd(dst as *mut ::core::ffi::c_uchar, dstSize as ptrdiff_t) as *mut BYTE;
    let mut args: HUF_DecompressFastArgs = HUF_DecompressFastArgs {
        ip: [::core::ptr::null::<BYTE>(); 4],
        op: [::core::ptr::null_mut::<BYTE>(); 4],
        bits: [0; 4],
        dt: ::core::ptr::null::<::core::ffi::c_void>(),
        ilowest: ::core::ptr::null::<BYTE>(),
        oend: ::core::ptr::null_mut::<BYTE>(),
        iend: [::core::ptr::null::<BYTE>(); 4],
    };
    let ret: size_t =
        HUF_DecompressFastArgs_init(&raw mut args, dst, dstSize, cSrc, cSrcSize, DTable) as size_t;
    let err_code: size_t = ret;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    if ret == 0 as size_t {
        return 0 as size_t;
    }
    loopFn.expect("non-null function pointer")(&raw mut args);
    let segmentSize: size_t = dstSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t);
    let mut segmentEnd: *mut BYTE = dst as *mut BYTE;
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        let mut bit: BIT_DStream_t = BIT_DStream_t {
            bitContainer: 0,
            bitsConsumed: 0,
            ptr: ::core::ptr::null::<::core::ffi::c_char>(),
            start: ::core::ptr::null::<::core::ffi::c_char>(),
            limitPtr: ::core::ptr::null::<::core::ffi::c_char>(),
        };
        if segmentSize <= oend.offset_from(segmentEnd) as ::core::ffi::c_long as size_t {
            segmentEnd = segmentEnd.offset(segmentSize as isize);
        } else {
            segmentEnd = oend;
        }
        let err_code_0: size_t =
            HUF_initRemainingDStream(&raw mut bit, &raw mut args, i, segmentEnd) as size_t;
        if ERR_isError(err_code_0) != 0 {
            return err_code_0;
        }
        args.op[i as usize] = args.op[i as usize].offset(HUF_decodeStreamX2(
            args.op[i as usize],
            &raw mut bit,
            segmentEnd,
            dt as *const HUF_DEltX2,
            HUF_DECODER_FAST_TABLELOG as U32,
        ) as isize);
        if args.op[i as usize] != segmentEnd {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        i += 1;
    }
    return dstSize;
}
unsafe extern "C" fn HUF_decompress4X2_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut fallbackFn: HUF_DecompressUsingDTableFn = Some(
        HUF_decompress4X2_usingDTable_internal_default
            as unsafe extern "C" fn(
                *mut ::core::ffi::c_void,
                size_t,
                *const ::core::ffi::c_void,
                size_t,
                *const HUF_DTable,
            ) -> size_t,
    );
    let mut loopFn: HUF_DecompressFastLoopFn = Some(
        HUF_decompress4X2_usingDTable_internal_fast_c_loop
            as unsafe extern "C" fn(*mut HUF_DecompressFastArgs) -> (),
    );
    if HUF_ENABLE_FAST_DECODE != 0 && flags & HUF_flags_disableFast as ::core::ffi::c_int == 0 {
        let ret: size_t = HUF_decompress4X2_usingDTable_internal_fast(
            dst, dstSize, cSrc, cSrcSize, DTable, loopFn,
        ) as size_t;
        if ret != 0 as size_t {
            return ret;
        }
    }
    return fallbackFn.expect("non-null function pointer")(dst, dstSize, cSrc, cSrcSize, DTable);
}
unsafe extern "C" fn HUF_decompress1X2_usingDTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_decompress1X2_usingDTable_internal_body(dst, dstSize, cSrc, cSrcSize, DTable);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X2_DCtx_wksp(
    mut DCtx: *mut HUF_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let hSize: size_t =
        HUF_readDTableX2_wksp(DCtx, cSrc, cSrcSize, workSpace, wkspSize, flags) as size_t;
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress1X2_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        DCtx,
        flags,
    );
}
unsafe extern "C" fn HUF_decompress4X2_DCtx_wksp(
    mut dctx: *mut HUF_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let mut hSize: size_t = HUF_readDTableX2_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags);
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress4X2_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        dctx,
        flags,
    );
}
static mut algoTime: [[algo_time_t; 2]; 16] = [
    [
        algo_time_t {
            tableTime: 0 as U32,
            decode256Time: 0 as U32,
        },
        algo_time_t {
            tableTime: 1 as U32,
            decode256Time: 1 as U32,
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
    ],
    [
        algo_time_t {
            tableTime: 150 as U32,
            decode256Time: 216 as U32,
        },
        algo_time_t {
            tableTime: 381 as U32,
            decode256Time: 119 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 170 as U32,
            decode256Time: 205 as U32,
        },
        algo_time_t {
            tableTime: 514 as U32,
            decode256Time: 112 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 177 as U32,
            decode256Time: 199 as U32,
        },
        algo_time_t {
            tableTime: 539 as U32,
            decode256Time: 110 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 197 as U32,
            decode256Time: 194 as U32,
        },
        algo_time_t {
            tableTime: 644 as U32,
            decode256Time: 107 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 221 as U32,
            decode256Time: 192 as U32,
        },
        algo_time_t {
            tableTime: 735 as U32,
            decode256Time: 107 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 256 as U32,
            decode256Time: 189 as U32,
        },
        algo_time_t {
            tableTime: 881 as U32,
            decode256Time: 106 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 359 as U32,
            decode256Time: 188 as U32,
        },
        algo_time_t {
            tableTime: 1167 as U32,
            decode256Time: 109 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 582 as U32,
            decode256Time: 187 as U32,
        },
        algo_time_t {
            tableTime: 1570 as U32,
            decode256Time: 114 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 688 as U32,
            decode256Time: 187 as U32,
        },
        algo_time_t {
            tableTime: 1712 as U32,
            decode256Time: 122 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 825 as U32,
            decode256Time: 186 as U32,
        },
        algo_time_t {
            tableTime: 1965 as U32,
            decode256Time: 136 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 976 as U32,
            decode256Time: 185 as U32,
        },
        algo_time_t {
            tableTime: 2131 as U32,
            decode256Time: 150 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1180 as U32,
            decode256Time: 186 as U32,
        },
        algo_time_t {
            tableTime: 2070 as U32,
            decode256Time: 175 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1377 as U32,
            decode256Time: 185 as U32,
        },
        algo_time_t {
            tableTime: 1731 as U32,
            decode256Time: 202 as U32,
        },
    ],
    [
        algo_time_t {
            tableTime: 1412 as U32,
            decode256Time: 185 as U32,
        },
        algo_time_t {
            tableTime: 1695 as U32,
            decode256Time: 202 as U32,
        },
    ],
];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_selectDecoder(mut dstSize: size_t, mut cSrcSize: size_t) -> U32 {
    let Q: U32 = if cSrcSize >= dstSize {
        15 as U32
    } else {
        cSrcSize.wrapping_mul(16 as size_t).wrapping_div(dstSize) as U32
    };
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
        .wrapping_add((DTime1 >> 5 as ::core::ffi::c_int) as ::core::ffi::c_uint)
        as U32 as U32;
    return (DTime1 < DTime0) as ::core::ffi::c_int as U32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_DCtx_wksp(
    mut dctx: *mut HUF_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    if dstSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize > dstSize {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize == dstSize {
        ::libc::memcpy(dst, cSrc, dstSize as ::libc::size_t);
        return dstSize;
    }
    if cSrcSize == 1 as size_t {
        ::libc::memset(
            dst,
            *(cSrc as *const BYTE) as ::core::ffi::c_int,
            dstSize as ::libc::size_t,
        );
        return dstSize;
    }
    let algoNb: U32 = HUF_selectDecoder(dstSize, cSrcSize) as U32;
    return if algoNb != 0 {
        HUF_decompress1X2_DCtx_wksp(
            dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
        )
    } else {
        HUF_decompress1X1_DCtx_wksp(
            dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
        )
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let dtd: DTableDesc = HUF_getDTableDesc(DTable) as DTableDesc;
    return if dtd.tableType as ::core::ffi::c_int != 0 {
        HUF_decompress1X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    } else {
        HUF_decompress1X1_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress1X1_DCtx_wksp(
    mut dctx: *mut HUF_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let mut ip: *const BYTE = cSrc as *const BYTE;
    let hSize: size_t =
        HUF_readDTableX1_wksp(dctx, cSrc, cSrcSize, workSpace, wkspSize, flags) as size_t;
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if hSize >= cSrcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ip = ip.offset(hSize as isize);
    cSrcSize = (cSrcSize as ::core::ffi::c_ulong).wrapping_sub(hSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return HUF_decompress1X1_usingDTable_internal(
        dst,
        dstSize,
        ip as *const ::core::ffi::c_void,
        cSrcSize,
        dctx,
        flags,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_usingDTable(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut DTable: *const HUF_DTable,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let dtd: DTableDesc = HUF_getDTableDesc(DTable) as DTableDesc;
    return if dtd.tableType as ::core::ffi::c_int != 0 {
        HUF_decompress4X2_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    } else {
        HUF_decompress4X1_usingDTable_internal(dst, maxDstSize, cSrc, cSrcSize, DTable, flags)
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_decompress4X_hufOnly_wksp(
    mut dctx: *mut HUF_DTable,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut cSrc: *const ::core::ffi::c_void,
    mut cSrcSize: size_t,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    if dstSize == 0 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if cSrcSize == 0 as size_t {
        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
    }
    let algoNb: U32 = HUF_selectDecoder(dstSize, cSrcSize) as U32;
    return if algoNb != 0 {
        HUF_decompress4X2_DCtx_wksp(
            dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
        )
    } else {
        HUF_decompress4X1_DCtx_wksp(
            dctx, dst, dstSize, cSrc, cSrcSize, workSpace, wkspSize, flags,
        )
    };
}
