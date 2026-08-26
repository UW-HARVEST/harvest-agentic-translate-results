use ::libc;
extern "C" {
    fn HIST_count_wksp(
        count: *mut ::core::ffi::c_uint,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        workSpace: *mut ::core::ffi::c_void,
        workSpaceSize: size_t,
    ) -> size_t;
    fn HIST_count_simple(
        count: *mut ::core::ffi::c_uint,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> ::core::ffi::c_uint;
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
    fn FSE_compress_usingCTable(
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        ct: *const FSE_CTable,
    ) -> size_t;
    fn FSE_optimalTableLog_internal(
        maxTableLog: ::core::ffi::c_uint,
        srcSize: size_t,
        maxSymbolValue: ::core::ffi::c_uint,
        minus: ::core::ffi::c_uint,
    ) -> ::core::ffi::c_uint;
    fn FSE_buildCTable_wksp(
        ct: *mut FSE_CTable,
        normalizedCounter: *const ::core::ffi::c_short,
        maxSymbolValue: ::core::ffi::c_uint,
        tableLog: ::core::ffi::c_uint,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
    ) -> size_t;
    fn HUF_readStats(
        huffWeight: *mut BYTE,
        hwSize: size_t,
        rankStats: *mut U32,
        nbSymbolsPtr: *mut U32,
        tableLogPtr: *mut U32,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
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
pub type FSE_CTable = ::core::ffi::c_uint;
pub type HUF_CElt = size_t;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const HUF_flags_disableFast: C2RustUnnamed_0 = 32;
pub const HUF_flags_disableAsm: C2RustUnnamed_0 = 16;
pub const HUF_flags_suspectUncompressible: C2RustUnnamed_0 = 8;
pub const HUF_flags_preferRepeat: C2RustUnnamed_0 = 4;
pub const HUF_flags_optimalDepth: C2RustUnnamed_0 = 2;
pub const HUF_flags_bmi2: C2RustUnnamed_0 = 1;
pub type nodeElt = nodeElt_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct nodeElt_s {
    pub count: U32,
    pub parent: U16,
    pub byte: BYTE,
    pub nbBits: BYTE,
}
pub type huffNodeTable = [nodeElt; 512];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_buildCTable_wksp_tables {
    pub huffNodeTbl: huffNodeTable,
    pub rankPosition: [rankPos; 192],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rankPos {
    pub base: U16,
    pub curr: U16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_CTableHeader {
    pub tableLog: BYTE,
    pub maxSymbolValue: BYTE,
    pub unused: [BYTE; 6],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_WriteCTableWksp {
    pub wksp: HUF_CompressWeightsWksp,
    pub bitsToWeight: [BYTE; 13],
    pub huffWeight: [BYTE; 255],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_CompressWeightsWksp {
    pub CTable: [FSE_CTable; 59],
    pub scratchBuffer: [U32; 41],
    pub count: [::core::ffi::c_uint; 13],
    pub norm: [S16; 13],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_CStream_t {
    pub bitContainer: [size_t; 2],
    pub bitPos: [size_t; 2],
    pub startPtr: *mut BYTE,
    pub ptr: *mut BYTE,
    pub endPtr: *mut BYTE,
}
pub type HUF_repeat = ::core::ffi::c_uint;
pub const HUF_repeat_valid: HUF_repeat = 2;
pub const HUF_repeat_check: HUF_repeat = 1;
pub const HUF_repeat_none: HUF_repeat = 0;
pub type HUF_nbStreams_e = ::core::ffi::c_uint;
pub const HUF_fourStreams: HUF_nbStreams_e = 1;
pub const HUF_singleStream: HUF_nbStreams_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct HUF_compress_tables_t {
    pub count: [::core::ffi::c_uint; 256],
    pub CTable: [HUF_CElt; 257],
    pub wksps: C2RustUnnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_1 {
    pub buildCTable_wksp: HUF_buildCTable_wksp_tables,
    pub writeCTable_wksp: HUF_WriteCTableWksp,
    pub hist_wksp: [U32; 1024],
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
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
unsafe extern "C" fn MEM_write16(mut memPtr: *mut ::core::ffi::c_void, mut value: U16) {
    *(memPtr as *mut unalign16) = value as unalign16;
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
pub const HUF_BLOCKSIZE_MAX: ::core::ffi::c_int =
    128 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int;
pub const HUF_TABLELOG_MAX: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const HUF_TABLELOG_DEFAULT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const HUF_SYMBOLVALUE_MAX: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const HUF_CTABLEBOUND: ::core::ffi::c_int = 129 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_alignUpWorkspace(
    mut workspace: *mut ::core::ffi::c_void,
    mut workspaceSizePtr: *mut size_t,
    mut align: size_t,
) -> *mut ::core::ffi::c_void {
    let mask: size_t = align.wrapping_sub(1 as size_t);
    let rem: size_t = workspace as size_t & mask;
    let add: size_t = align.wrapping_sub(rem) & mask;
    let aligned: *mut BYTE = (workspace as *mut BYTE).offset(add as isize);
    if *workspaceSizePtr >= add {
        *workspaceSizePtr = (*workspaceSizePtr as ::core::ffi::c_ulong)
            .wrapping_sub(add as ::core::ffi::c_ulong) as size_t
            as size_t;
        return aligned as *mut ::core::ffi::c_void;
    } else {
        *workspaceSizePtr = 0 as size_t;
        return NULL;
    };
}
pub const MAX_FSE_TABLELOG_FOR_HUFF_HEADER: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_compressWeights(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut weightTable: *const ::core::ffi::c_void,
    mut wtSize: size_t,
    mut workspace: *mut ::core::ffi::c_void,
    mut workspaceSize: size_t,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut maxSymbolValue: ::core::ffi::c_uint = HUF_TABLELOG_MAX as ::core::ffi::c_uint;
    let mut tableLog: U32 = MAX_FSE_TABLELOG_FOR_HUFF_HEADER as U32;
    let mut wksp: *mut HUF_CompressWeightsWksp = HUF_alignUpWorkspace(
        workspace,
        &raw mut workspaceSize,
        ::core::mem::align_of::<U32>(),
    ) as *mut HUF_CompressWeightsWksp;
    if workspaceSize < ::core::mem::size_of::<HUF_CompressWeightsWksp>() as usize {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    if wtSize <= 1 as size_t {
        return 0 as size_t;
    }
    let maxCount: ::core::ffi::c_uint = HIST_count_simple(
        &raw mut (*wksp).count as *mut ::core::ffi::c_uint,
        &raw mut maxSymbolValue,
        weightTable,
        wtSize,
    ) as ::core::ffi::c_uint;
    if maxCount as size_t == wtSize {
        return 1 as size_t;
    }
    if maxCount == 1 as ::core::ffi::c_uint {
        return 0 as size_t;
    }
    tableLog = FSE_optimalTableLog(tableLog as ::core::ffi::c_uint, wtSize, maxSymbolValue) as U32;
    let _var_err__: size_t = FSE_normalizeCount(
        &raw mut (*wksp).norm as *mut ::core::ffi::c_short,
        tableLog as ::core::ffi::c_uint,
        &raw mut (*wksp).count as *mut ::core::ffi::c_uint,
        wtSize,
        maxSymbolValue,
        0 as ::core::ffi::c_uint,
    ) as size_t;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    let hSize: size_t = FSE_writeNCount(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        &raw mut (*wksp).norm as *mut S16,
        maxSymbolValue,
        tableLog as ::core::ffi::c_uint,
    ) as size_t;
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    op = op.offset(hSize as isize);
    let _var_err___0: size_t = FSE_buildCTable_wksp(
        &raw mut (*wksp).CTable as *mut FSE_CTable,
        &raw mut (*wksp).norm as *mut S16,
        maxSymbolValue,
        tableLog as ::core::ffi::c_uint,
        &raw mut (*wksp).scratchBuffer as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 41]>() as size_t,
    ) as size_t;
    if ERR_isError(_var_err___0) != 0 {
        return _var_err___0;
    }
    let cSize: size_t = FSE_compress_usingCTable(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        weightTable,
        wtSize,
        &raw mut (*wksp).CTable as *mut FSE_CTable,
    ) as size_t;
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    if cSize == 0 as size_t {
        return 0 as size_t;
    }
    op = op.offset(cSize as isize);
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn HUF_getNbBits(mut elt: HUF_CElt) -> size_t {
    return elt as size_t & 0xff as size_t;
}
unsafe extern "C" fn HUF_getNbBitsFast(mut elt: HUF_CElt) -> size_t {
    return elt as size_t;
}
unsafe extern "C" fn HUF_getValue(mut elt: HUF_CElt) -> size_t {
    return elt as size_t & !(0xff as ::core::ffi::c_int as size_t);
}
unsafe extern "C" fn HUF_getValueFast(mut elt: HUF_CElt) -> size_t {
    return elt as size_t;
}
unsafe extern "C" fn HUF_setNbBits(mut elt: *mut HUF_CElt, mut nbBits: size_t) {
    *elt = nbBits as HUF_CElt;
}
unsafe extern "C" fn HUF_setValue(mut elt: *mut HUF_CElt, mut value: size_t) {
    let nbBits: size_t = HUF_getNbBits(*elt) as size_t;
    if nbBits > 0 as size_t {
        *elt = (*elt as ::core::ffi::c_ulong
            | (value
                << (::core::mem::size_of::<HUF_CElt>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_sub(nbBits as usize)) as ::core::ffi::c_ulong)
            as HUF_CElt;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTableHeader(mut ctable: *const HUF_CElt) -> HUF_CTableHeader {
    let mut header: HUF_CTableHeader = HUF_CTableHeader {
        tableLog: 0,
        maxSymbolValue: 0,
        unused: [0; 6],
    };
    ::libc::memcpy(
        &raw mut header as *mut ::core::ffi::c_void,
        ctable as *const ::core::ffi::c_void,
        ::core::mem::size_of::<HUF_CTableHeader>() as ::libc::size_t,
    );
    return header;
}
unsafe extern "C" fn HUF_writeCTableHeader(
    mut ctable: *mut HUF_CElt,
    mut tableLog: U32,
    mut maxSymbolValue: U32,
) {
    let mut header: HUF_CTableHeader = HUF_CTableHeader {
        tableLog: 0,
        maxSymbolValue: 0,
        unused: [0; 6],
    };
    ::libc::memset(
        &raw mut header as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<HUF_CTableHeader>() as ::libc::size_t,
    );
    header.tableLog = tableLog as BYTE;
    header.maxSymbolValue = maxSymbolValue as BYTE;
    ::libc::memcpy(
        ctable as *mut ::core::ffi::c_void,
        &raw mut header as *const ::core::ffi::c_void,
        ::core::mem::size_of::<HUF_CTableHeader>() as ::libc::size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_writeCTable_wksp(
    mut dst: *mut ::core::ffi::c_void,
    mut maxDstSize: size_t,
    mut CTable: *const HUF_CElt,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut huffLog: ::core::ffi::c_uint,
    mut workspace: *mut ::core::ffi::c_void,
    mut workspaceSize: size_t,
) -> size_t {
    let ct: *const HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut n: U32 = 0;
    let mut wksp: *mut HUF_WriteCTableWksp = HUF_alignUpWorkspace(
        workspace,
        &raw mut workspaceSize,
        ::core::mem::align_of::<U32>(),
    ) as *mut HUF_WriteCTableWksp;
    if workspaceSize < ::core::mem::size_of::<HUF_WriteCTableWksp>() as usize {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX as ::core::ffi::c_uint {
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    (*wksp).bitsToWeight[0 as ::core::ffi::c_int as usize] = 0 as BYTE;
    n = 1 as U32;
    while n < (huffLog as U32).wrapping_add(1 as U32) {
        (*wksp).bitsToWeight[n as usize] =
            (huffLog as U32).wrapping_add(1 as U32).wrapping_sub(n) as BYTE;
        n = n.wrapping_add(1);
    }
    n = 0 as U32;
    while n < maxSymbolValue as U32 {
        (*wksp).huffWeight[n as usize] =
            (*wksp).bitsToWeight[HUF_getNbBits(*ct.offset(n as isize)) as usize];
        n = n.wrapping_add(1);
    }
    if maxDstSize < 1 as size_t {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    let hSize: size_t = HUF_compressWeights(
        op.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        maxDstSize.wrapping_sub(1 as size_t),
        &raw mut (*wksp).huffWeight as *mut BYTE as *const ::core::ffi::c_void,
        maxSymbolValue as size_t,
        &raw mut (*wksp).wksp as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<HUF_CompressWeightsWksp>() as size_t,
    ) as size_t;
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if (hSize > 1 as size_t) as ::core::ffi::c_int
        & (hSize < maxSymbolValue.wrapping_div(2 as ::core::ffi::c_uint) as size_t)
            as ::core::ffi::c_int
        != 0
    {
        *op.offset(0 as ::core::ffi::c_int as isize) = hSize as BYTE;
        return hSize.wrapping_add(1 as size_t);
    }
    if maxSymbolValue
        > (256 as ::core::ffi::c_int - 128 as ::core::ffi::c_int) as ::core::ffi::c_uint
    {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    if maxSymbolValue
        .wrapping_add(1 as ::core::ffi::c_uint)
        .wrapping_div(2 as ::core::ffi::c_uint)
        .wrapping_add(1 as ::core::ffi::c_uint) as size_t
        > maxDstSize
    {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    *op.offset(0 as ::core::ffi::c_int as isize) = (128 as ::core::ffi::c_uint)
        .wrapping_add(maxSymbolValue.wrapping_sub(1 as ::core::ffi::c_uint))
        as BYTE;
    (*wksp).huffWeight[maxSymbolValue as usize] = 0 as BYTE;
    n = 0 as U32;
    while n < maxSymbolValue as U32 {
        *op.offset(n.wrapping_div(2 as U32).wrapping_add(1 as U32) as isize) =
            ((((*wksp).huffWeight[n as usize] as ::core::ffi::c_int) << 4 as ::core::ffi::c_int)
                + (*wksp).huffWeight[n.wrapping_add(1 as U32) as usize] as ::core::ffi::c_int)
                as BYTE;
        n = (n as ::core::ffi::c_uint).wrapping_add(2 as ::core::ffi::c_uint) as U32 as U32;
    }
    return maxSymbolValue
        .wrapping_add(1 as ::core::ffi::c_uint)
        .wrapping_div(2 as ::core::ffi::c_uint)
        .wrapping_add(1 as ::core::ffi::c_uint) as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readCTable(
    mut CTable: *mut HUF_CElt,
    mut maxSymbolValuePtr: *mut ::core::ffi::c_uint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut hasZeroWeights: *mut ::core::ffi::c_uint,
) -> size_t {
    let mut huffWeight: [BYTE; 256] = [0; 256];
    let mut rankVal: [U32; 13] = [0; 13];
    let mut tableLog: U32 = 0 as U32;
    let mut nbSymbols: U32 = 0 as U32;
    let ct: *mut HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    let readSize: size_t = HUF_readStats(
        &raw mut huffWeight as *mut BYTE,
        (255 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as size_t,
        &raw mut rankVal as *mut U32,
        &raw mut nbSymbols,
        &raw mut tableLog,
        src,
        srcSize,
    ) as size_t;
    if ERR_isError(readSize) != 0 {
        return readSize;
    }
    *hasZeroWeights = (rankVal[0 as ::core::ffi::c_int as usize] > 0 as U32) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
    if tableLog > HUF_TABLELOG_MAX as U32 {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if nbSymbols > (*maxSymbolValuePtr).wrapping_add(1 as U32) {
        return -(ZSTD_error_maxSymbolValue_tooSmall as ::core::ffi::c_int) as size_t;
    }
    *maxSymbolValuePtr = nbSymbols.wrapping_sub(1 as U32) as ::core::ffi::c_uint;
    HUF_writeCTableHeader(CTable, tableLog, *maxSymbolValuePtr);
    let mut n: U32 = 0;
    let mut nextRankStart: U32 = 0 as U32;
    n = 1 as U32;
    while n <= tableLog {
        let mut curr: U32 = nextRankStart;
        nextRankStart = (nextRankStart as ::core::ffi::c_uint)
            .wrapping_add((rankVal[n as usize] << n.wrapping_sub(1 as U32)) as ::core::ffi::c_uint)
            as U32 as U32;
        rankVal[n as usize] = curr;
        n = n.wrapping_add(1);
    }
    let mut n_0: U32 = 0;
    n_0 = 0 as U32;
    while n_0 < nbSymbols {
        let w: U32 = huffWeight[n_0 as usize] as U32;
        HUF_setNbBits(
            ct.offset(n_0 as isize),
            (tableLog.wrapping_add(1 as U32).wrapping_sub(w) as BYTE as ::core::ffi::c_int
                & -((w != 0 as U32) as ::core::ffi::c_int)) as size_t,
        );
        n_0 = n_0.wrapping_add(1);
    }
    let mut nbPerRank: [U16; 14] = [
        0 as ::core::ffi::c_int as U16,
        0,
        0,
        0,
        0,
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
    let mut valPerRank: [U16; 14] = [
        0 as ::core::ffi::c_int as U16,
        0,
        0,
        0,
        0,
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
    let mut n_1: U32 = 0;
    n_1 = 0 as U32;
    while n_1 < nbSymbols {
        nbPerRank[HUF_getNbBits(*ct.offset(n_1 as isize)) as usize] =
            nbPerRank[HUF_getNbBits(*ct.offset(n_1 as isize)) as usize].wrapping_add(1);
        n_1 = n_1.wrapping_add(1);
    }
    valPerRank[tableLog.wrapping_add(1 as U32) as usize] = 0 as U16;
    let mut min: U16 = 0 as U16;
    let mut n_2: U32 = 0;
    n_2 = tableLog;
    while n_2 > 0 as U32 {
        valPerRank[n_2 as usize] = min;
        min = (min as ::core::ffi::c_int + nbPerRank[n_2 as usize] as ::core::ffi::c_int) as U16;
        min = (min as ::core::ffi::c_int >> 1 as ::core::ffi::c_int) as U16;
        n_2 = n_2.wrapping_sub(1);
    }
    let mut n_3: U32 = 0;
    n_3 = 0 as U32;
    while n_3 < nbSymbols {
        let fresh14 = valPerRank[HUF_getNbBits(*ct.offset(n_3 as isize)) as usize];
        valPerRank[HUF_getNbBits(*ct.offset(n_3 as isize)) as usize] =
            valPerRank[HUF_getNbBits(*ct.offset(n_3 as isize)) as usize].wrapping_add(1);
        HUF_setValue(ct.offset(n_3 as isize), fresh14 as size_t);
        n_3 = n_3.wrapping_add(1);
    }
    return readSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getNbBitsFromCTable(
    mut CTable: *const HUF_CElt,
    mut symbolValue: U32,
) -> U32 {
    let ct: *const HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    if symbolValue > HUF_readCTableHeader(CTable).maxSymbolValue as U32 {
        return 0 as U32;
    }
    return HUF_getNbBits(*ct.offset(symbolValue as isize)) as U32;
}
unsafe extern "C" fn HUF_setMaxHeight(
    mut huffNode: *mut nodeElt,
    mut lastNonNull: U32,
    mut targetNbBits: U32,
) -> U32 {
    let largestBits: U32 = (*huffNode.offset(lastNonNull as isize)).nbBits as U32;
    if largestBits <= targetNbBits {
        return largestBits;
    }
    let mut totalCost: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let baseCost: U32 =
        ((1 as ::core::ffi::c_int) << largestBits.wrapping_sub(targetNbBits)) as U32;
    let mut n: ::core::ffi::c_int = lastNonNull as ::core::ffi::c_int;
    while (*huffNode.offset(n as isize)).nbBits as U32 > targetNbBits {
        totalCost = (totalCost as ::core::ffi::c_uint).wrapping_add(baseCost.wrapping_sub(
            ((1 as ::core::ffi::c_int)
                << largestBits.wrapping_sub((*huffNode.offset(n as isize)).nbBits as U32))
                as U32,
        )
            as ::core::ffi::c_uint) as ::core::ffi::c_int as ::core::ffi::c_int;
        (*huffNode.offset(n as isize)).nbBits = targetNbBits as BYTE;
        n -= 1;
    }
    while (*huffNode.offset(n as isize)).nbBits as U32 == targetNbBits {
        n -= 1;
    }
    totalCost >>= largestBits.wrapping_sub(targetNbBits);
    let noSymbol: U32 = 0xf0f0f0f0 as U32;
    let mut rankLast: [U32; 14] = [0; 14];
    ::libc::memset(
        &raw mut rankLast as *mut U32 as *mut ::core::ffi::c_void,
        0xf0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[U32; 14]>() as ::libc::size_t,
    );
    let mut currentNbBits: U32 = targetNbBits;
    let mut pos: ::core::ffi::c_int = 0;
    pos = n;
    while pos >= 0 as ::core::ffi::c_int {
        if !((*huffNode.offset(pos as isize)).nbBits as U32 >= currentNbBits) {
            currentNbBits = (*huffNode.offset(pos as isize)).nbBits as U32;
            rankLast[targetNbBits.wrapping_sub(currentNbBits) as usize] = pos as U32;
        }
        pos -= 1;
    }
    while totalCost > 0 as ::core::ffi::c_int {
        let mut nBitsToDecrease: U32 =
            (ZSTD_highbit32(totalCost as U32) as U32).wrapping_add(1 as U32);
        while nBitsToDecrease > 1 as U32 {
            let highPos: U32 = rankLast[nBitsToDecrease as usize];
            let lowPos: U32 = rankLast[nBitsToDecrease.wrapping_sub(1 as U32) as usize];
            if !(highPos == noSymbol) {
                if lowPos == noSymbol {
                    break;
                }
                let highTotal: U32 = (*huffNode.offset(highPos as isize)).count;
                let lowTotal: U32 =
                    (2 as U32).wrapping_mul((*huffNode.offset(lowPos as isize)).count);
                if highTotal <= lowTotal {
                    break;
                }
            }
            nBitsToDecrease = nBitsToDecrease.wrapping_sub(1);
        }
        while nBitsToDecrease <= HUF_TABLELOG_MAX as U32
            && rankLast[nBitsToDecrease as usize] == noSymbol
        {
            nBitsToDecrease = nBitsToDecrease.wrapping_add(1);
        }
        totalCost -= (1 as ::core::ffi::c_int) << nBitsToDecrease.wrapping_sub(1 as U32);
        let ref mut fresh1 = (*huffNode.offset(rankLast[nBitsToDecrease as usize] as isize)).nbBits;
        *fresh1 = (*fresh1).wrapping_add(1);
        if rankLast[nBitsToDecrease.wrapping_sub(1 as U32) as usize] == noSymbol {
            rankLast[nBitsToDecrease.wrapping_sub(1 as U32) as usize] =
                rankLast[nBitsToDecrease as usize];
        }
        if rankLast[nBitsToDecrease as usize] == 0 as U32 {
            rankLast[nBitsToDecrease as usize] = noSymbol;
        } else {
            rankLast[nBitsToDecrease as usize] = rankLast[nBitsToDecrease as usize].wrapping_sub(1);
            if (*huffNode.offset(rankLast[nBitsToDecrease as usize] as isize)).nbBits as U32
                != targetNbBits.wrapping_sub(nBitsToDecrease)
            {
                rankLast[nBitsToDecrease as usize] = noSymbol;
            }
        }
    }
    while totalCost < 0 as ::core::ffi::c_int {
        if rankLast[1 as ::core::ffi::c_int as usize] == noSymbol {
            while (*huffNode.offset(n as isize)).nbBits as U32 == targetNbBits {
                n -= 1;
            }
            let ref mut fresh2 = (*huffNode.offset((n + 1 as ::core::ffi::c_int) as isize)).nbBits;
            *fresh2 = (*fresh2).wrapping_sub(1);
            rankLast[1 as ::core::ffi::c_int as usize] = (n + 1 as ::core::ffi::c_int) as U32;
            totalCost += 1;
        } else {
            let ref mut fresh3 = (*huffNode.offset(
                rankLast[1 as ::core::ffi::c_int as usize].wrapping_add(1 as U32) as isize,
            ))
            .nbBits;
            *fresh3 = (*fresh3).wrapping_sub(1);
            rankLast[1 as ::core::ffi::c_int as usize] =
                rankLast[1 as ::core::ffi::c_int as usize].wrapping_add(1);
            totalCost += 1;
        }
    }
    return targetNbBits;
}
pub const RANK_POSITION_TABLE_SIZE: ::core::ffi::c_int = 192 as ::core::ffi::c_int;
pub const RANK_POSITION_MAX_COUNT_LOG: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const RANK_POSITION_LOG_BUCKETS_BEGIN: ::core::ffi::c_int = RANK_POSITION_TABLE_SIZE
    - 1 as ::core::ffi::c_int
    - RANK_POSITION_MAX_COUNT_LOG
    - 1 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_getIndex(count: U32) -> U32 {
    return if count
        < (RANK_POSITION_LOG_BUCKETS_BEGIN as U32)
            .wrapping_add(ZSTD_highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN as U32) as U32)
    {
        count
    } else {
        (ZSTD_highbit32(count) as U32).wrapping_add(RANK_POSITION_LOG_BUCKETS_BEGIN as U32)
    };
}
unsafe extern "C" fn HUF_swapNodes(mut a: *mut nodeElt, mut b: *mut nodeElt) {
    let mut tmp: nodeElt = *a;
    *a = *b;
    *b = tmp;
}
#[inline(always)]
unsafe extern "C" fn HUF_insertionSort(
    mut huffNode: *mut nodeElt,
    low: ::core::ffi::c_int,
    high: ::core::ffi::c_int,
) {
    let mut i: ::core::ffi::c_int = 0;
    let size: ::core::ffi::c_int = high - low + 1 as ::core::ffi::c_int;
    huffNode = huffNode.offset(low as isize);
    i = 1 as ::core::ffi::c_int;
    while i < size {
        let key: nodeElt = *huffNode.offset(i as isize);
        let mut j: ::core::ffi::c_int = i - 1 as ::core::ffi::c_int;
        while j >= 0 as ::core::ffi::c_int && (*huffNode.offset(j as isize)).count < key.count {
            *huffNode.offset((j + 1 as ::core::ffi::c_int) as isize) = *huffNode.offset(j as isize);
            j -= 1;
        }
        *huffNode.offset((j + 1 as ::core::ffi::c_int) as isize) = key;
        i += 1;
    }
}
unsafe extern "C" fn HUF_quickSortPartition(
    mut arr: *mut nodeElt,
    low: ::core::ffi::c_int,
    high: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let pivot: U32 = (*arr.offset(high as isize)).count;
    let mut i: ::core::ffi::c_int = low - 1 as ::core::ffi::c_int;
    let mut j: ::core::ffi::c_int = low;
    while j < high {
        if (*arr.offset(j as isize)).count > pivot {
            i += 1;
            HUF_swapNodes(
                arr.offset(i as isize) as *mut nodeElt,
                arr.offset(j as isize) as *mut nodeElt,
            );
        }
        j += 1;
    }
    HUF_swapNodes(
        arr.offset((i + 1 as ::core::ffi::c_int) as isize) as *mut nodeElt,
        arr.offset(high as isize) as *mut nodeElt,
    );
    return i + 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn HUF_simpleQuickSort(
    mut arr: *mut nodeElt,
    mut low: ::core::ffi::c_int,
    mut high: ::core::ffi::c_int,
) {
    let kInsertionSortThreshold: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
    if high - low < kInsertionSortThreshold {
        HUF_insertionSort(arr, low, high);
        return;
    }
    while low < high {
        let idx: ::core::ffi::c_int = HUF_quickSortPartition(arr, low, high) as ::core::ffi::c_int;
        if idx - low < high - idx {
            HUF_simpleQuickSort(arr, low, idx - 1 as ::core::ffi::c_int);
            low = idx + 1 as ::core::ffi::c_int;
        } else {
            HUF_simpleQuickSort(arr, idx + 1 as ::core::ffi::c_int, high);
            high = idx - 1 as ::core::ffi::c_int;
        }
    }
}
unsafe extern "C" fn HUF_sort(
    mut huffNode: *mut nodeElt,
    mut count: *const ::core::ffi::c_uint,
    maxSymbolValue: U32,
    mut rankPosition: *mut rankPos,
) {
    let mut n: U32 = 0;
    let maxSymbolValue1: U32 = maxSymbolValue.wrapping_add(1 as U32);
    ::libc::memset(
        rankPosition as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        (::core::mem::size_of::<rankPos>() as usize).wrapping_mul(192 as usize) as ::libc::size_t,
    );
    n = 0 as U32;
    while n < maxSymbolValue1 {
        let mut lowerRank: U32 = HUF_getIndex(*count.offset(n as isize) as U32);
        let ref mut fresh10 = (*rankPosition.offset(lowerRank as isize)).base;
        *fresh10 = (*fresh10).wrapping_add(1);
        n = n.wrapping_add(1);
    }
    n = (RANK_POSITION_TABLE_SIZE - 1 as ::core::ffi::c_int) as U32;
    while n > 0 as U32 {
        let ref mut fresh11 = (*rankPosition.offset(n.wrapping_sub(1 as U32) as isize)).base;
        *fresh11 = (*fresh11 as ::core::ffi::c_int
            + (*rankPosition.offset(n as isize)).base as ::core::ffi::c_int)
            as U16;
        (*rankPosition.offset(n.wrapping_sub(1 as U32) as isize)).curr =
            (*rankPosition.offset(n.wrapping_sub(1 as U32) as isize)).base;
        n = n.wrapping_sub(1);
    }
    n = 0 as U32;
    while n < maxSymbolValue1 {
        let c: U32 = *count.offset(n as isize) as U32;
        let r: U32 = (HUF_getIndex(c) as U32).wrapping_add(1 as U32);
        let ref mut fresh12 = (*rankPosition.offset(r as isize)).curr;
        let fresh13 = *fresh12;
        *fresh12 = (*fresh12).wrapping_add(1);
        let pos: U32 = fresh13 as U32;
        (*huffNode.offset(pos as isize)).count = c;
        (*huffNode.offset(pos as isize)).byte = n as BYTE;
        n = n.wrapping_add(1);
    }
    n = (RANK_POSITION_LOG_BUCKETS_BEGIN as ::core::ffi::c_uint)
        .wrapping_add(ZSTD_highbit32(RANK_POSITION_LOG_BUCKETS_BEGIN as U32)) as U32;
    while n < (RANK_POSITION_TABLE_SIZE - 1 as ::core::ffi::c_int) as U32 {
        let bucketSize: ::core::ffi::c_int = (*rankPosition.offset(n as isize)).curr
            as ::core::ffi::c_int
            - (*rankPosition.offset(n as isize)).base as ::core::ffi::c_int;
        let bucketStartIdx: U32 = (*rankPosition.offset(n as isize)).base as U32;
        if bucketSize > 1 as ::core::ffi::c_int {
            HUF_simpleQuickSort(
                huffNode.offset(bucketStartIdx as isize),
                0 as ::core::ffi::c_int,
                bucketSize - 1 as ::core::ffi::c_int,
            );
        }
        n = n.wrapping_add(1);
    }
}
pub const STARTNODE: ::core::ffi::c_int = HUF_SYMBOLVALUE_MAX + 1 as ::core::ffi::c_int;
unsafe extern "C" fn HUF_buildTree(
    mut huffNode: *mut nodeElt,
    mut maxSymbolValue: U32,
) -> ::core::ffi::c_int {
    let huffNode0: *mut nodeElt = huffNode.offset(-(1 as ::core::ffi::c_int as isize));
    let mut nonNullRank: ::core::ffi::c_int = 0;
    let mut lowS: ::core::ffi::c_int = 0;
    let mut lowN: ::core::ffi::c_int = 0;
    let mut nodeNb: ::core::ffi::c_int = STARTNODE;
    let mut n: ::core::ffi::c_int = 0;
    let mut nodeRoot: ::core::ffi::c_int = 0;
    nonNullRank = maxSymbolValue as ::core::ffi::c_int;
    while (*huffNode.offset(nonNullRank as isize)).count == 0 as U32 {
        nonNullRank -= 1;
    }
    lowS = nonNullRank;
    nodeRoot = nodeNb + lowS - 1 as ::core::ffi::c_int;
    lowN = nodeNb;
    (*huffNode.offset(nodeNb as isize)).count = (*huffNode.offset(lowS as isize))
        .count
        .wrapping_add((*huffNode.offset((lowS - 1 as ::core::ffi::c_int) as isize)).count);
    let ref mut fresh4 = (*huffNode.offset((lowS - 1 as ::core::ffi::c_int) as isize)).parent;
    *fresh4 = nodeNb as U16;
    (*huffNode.offset(lowS as isize)).parent = *fresh4;
    nodeNb += 1;
    lowS -= 2 as ::core::ffi::c_int;
    n = nodeNb;
    while n <= nodeRoot {
        (*huffNode.offset(n as isize)).count =
            ((1 as ::core::ffi::c_uint) << 30 as ::core::ffi::c_int) as U32;
        n += 1;
    }
    (*huffNode0.offset(0 as ::core::ffi::c_int as isize)).count =
        ((1 as ::core::ffi::c_uint) << 31 as ::core::ffi::c_int) as U32;
    while nodeNb <= nodeRoot {
        let n1: ::core::ffi::c_int =
            if (*huffNode.offset(lowS as isize)).count < (*huffNode.offset(lowN as isize)).count {
                let fresh5 = lowS;
                lowS = lowS - 1;
                fresh5
            } else {
                let fresh6 = lowN;
                lowN = lowN + 1;
                fresh6
            };
        let n2: ::core::ffi::c_int =
            if (*huffNode.offset(lowS as isize)).count < (*huffNode.offset(lowN as isize)).count {
                let fresh7 = lowS;
                lowS = lowS - 1;
                fresh7
            } else {
                let fresh8 = lowN;
                lowN = lowN + 1;
                fresh8
            };
        (*huffNode.offset(nodeNb as isize)).count = (*huffNode.offset(n1 as isize))
            .count
            .wrapping_add((*huffNode.offset(n2 as isize)).count);
        let ref mut fresh9 = (*huffNode.offset(n2 as isize)).parent;
        *fresh9 = nodeNb as U16;
        (*huffNode.offset(n1 as isize)).parent = *fresh9;
        nodeNb += 1;
    }
    (*huffNode.offset(nodeRoot as isize)).nbBits = 0 as BYTE;
    n = nodeRoot - 1 as ::core::ffi::c_int;
    while n >= STARTNODE {
        (*huffNode.offset(n as isize)).nbBits = ((*huffNode
            .offset((*huffNode.offset(n as isize)).parent as isize))
        .nbBits as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int) as BYTE;
        n -= 1;
    }
    n = 0 as ::core::ffi::c_int;
    while n <= nonNullRank {
        (*huffNode.offset(n as isize)).nbBits = ((*huffNode
            .offset((*huffNode.offset(n as isize)).parent as isize))
        .nbBits as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int) as BYTE;
        n += 1;
    }
    return nonNullRank;
}
unsafe extern "C" fn HUF_buildCTableFromTree(
    mut CTable: *mut HUF_CElt,
    mut huffNode: *const nodeElt,
    mut nonNullRank: ::core::ffi::c_int,
    mut maxSymbolValue: U32,
    mut maxNbBits: U32,
) {
    let ct: *mut HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    let mut n: ::core::ffi::c_int = 0;
    let mut nbPerRank: [U16; 13] = [
        0 as ::core::ffi::c_int as U16,
        0,
        0,
        0,
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
    let mut valPerRank: [U16; 13] = [
        0 as ::core::ffi::c_int as U16,
        0,
        0,
        0,
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
    let alphabetSize: ::core::ffi::c_int =
        maxSymbolValue.wrapping_add(1 as U32) as ::core::ffi::c_int;
    n = 0 as ::core::ffi::c_int;
    while n <= nonNullRank {
        nbPerRank[(*huffNode.offset(n as isize)).nbBits as usize] =
            nbPerRank[(*huffNode.offset(n as isize)).nbBits as usize].wrapping_add(1);
        n += 1;
    }
    let mut min: U16 = 0 as U16;
    n = maxNbBits as ::core::ffi::c_int;
    while n > 0 as ::core::ffi::c_int {
        valPerRank[n as usize] = min;
        min = (min as ::core::ffi::c_int + nbPerRank[n as usize] as ::core::ffi::c_int) as U16;
        min = (min as ::core::ffi::c_int >> 1 as ::core::ffi::c_int) as U16;
        n -= 1;
    }
    n = 0 as ::core::ffi::c_int;
    while n < alphabetSize {
        HUF_setNbBits(
            ct.offset((*huffNode.offset(n as isize)).byte as ::core::ffi::c_int as isize),
            (*huffNode.offset(n as isize)).nbBits as size_t,
        );
        n += 1;
    }
    n = 0 as ::core::ffi::c_int;
    while n < alphabetSize {
        let fresh0 = valPerRank[HUF_getNbBits(*ct.offset(n as isize)) as usize];
        valPerRank[HUF_getNbBits(*ct.offset(n as isize)) as usize] =
            valPerRank[HUF_getNbBits(*ct.offset(n as isize)) as usize].wrapping_add(1);
        HUF_setValue(ct.offset(n as isize), fresh0 as size_t);
        n += 1;
    }
    HUF_writeCTableHeader(CTable, maxNbBits, maxSymbolValue);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_buildCTable_wksp(
    mut CTable: *mut HUF_CElt,
    mut count: *const ::core::ffi::c_uint,
    mut maxSymbolValue: U32,
    mut maxNbBits: U32,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    let wksp_tables: *mut HUF_buildCTable_wksp_tables =
        HUF_alignUpWorkspace(workSpace, &raw mut wkspSize, ::core::mem::align_of::<U32>())
            as *mut HUF_buildCTable_wksp_tables;
    let huffNode0: *mut nodeElt = &raw mut (*wksp_tables).huffNodeTbl as *mut nodeElt;
    let huffNode: *mut nodeElt = huffNode0.offset(1 as ::core::ffi::c_int as isize);
    let mut nonNullRank: ::core::ffi::c_int = 0;
    if wkspSize < ::core::mem::size_of::<HUF_buildCTable_wksp_tables>() as usize {
        return -(ZSTD_error_workSpace_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if maxNbBits == 0 as U32 {
        maxNbBits = HUF_TABLELOG_DEFAULT as U32;
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX as U32 {
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    ::libc::memset(
        huffNode0 as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<huffNodeTable>() as ::libc::size_t,
    );
    HUF_sort(
        huffNode as *mut nodeElt,
        count as *const ::core::ffi::c_uint,
        maxSymbolValue,
        &raw mut (*wksp_tables).rankPosition as *mut rankPos,
    );
    nonNullRank = HUF_buildTree(huffNode, maxSymbolValue);
    maxNbBits = HUF_setMaxHeight(huffNode, nonNullRank as U32, maxNbBits);
    if maxNbBits > HUF_TABLELOG_MAX as U32 {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    HUF_buildCTableFromTree(CTable, huffNode, nonNullRank, maxSymbolValue, maxNbBits);
    return maxNbBits as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_estimateCompressedSize(
    mut CTable: *const HUF_CElt,
    mut count: *const ::core::ffi::c_uint,
    mut maxSymbolValue: ::core::ffi::c_uint,
) -> size_t {
    let mut ct: *const HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    let mut nbBits: size_t = 0 as size_t;
    let mut s: ::core::ffi::c_int = 0;
    s = 0 as ::core::ffi::c_int;
    while s <= maxSymbolValue as ::core::ffi::c_int {
        nbBits = (nbBits as ::core::ffi::c_ulong).wrapping_add(
            HUF_getNbBits(*ct.offset(s as isize)).wrapping_mul(*count.offset(s as isize) as size_t)
                as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        s += 1;
    }
    return nbBits >> 3 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_validateCTable(
    mut CTable: *const HUF_CElt,
    mut count: *const ::core::ffi::c_uint,
    mut maxSymbolValue: ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    let mut header: HUF_CTableHeader = HUF_readCTableHeader(CTable);
    let mut ct: *const HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    let mut bad: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut s: ::core::ffi::c_int = 0;
    if (header.maxSymbolValue as ::core::ffi::c_uint) < maxSymbolValue {
        return 0 as ::core::ffi::c_int;
    }
    s = 0 as ::core::ffi::c_int;
    while s <= maxSymbolValue as ::core::ffi::c_int {
        bad |= (*count.offset(s as isize) != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int
            & (HUF_getNbBits(*ct.offset(s as isize)) == 0 as size_t) as ::core::ffi::c_int;
        s += 1;
    }
    return (bad == 0) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compressBound(mut size: size_t) -> size_t {
    return (HUF_CTABLEBOUND as size_t).wrapping_add(
        size.wrapping_add(size >> 8 as ::core::ffi::c_int)
            .wrapping_add(8 as size_t),
    );
}
pub const HUF_BITS_IN_CONTAINER: usize =
    (::core::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize);
unsafe extern "C" fn HUF_initCStream(
    mut bitC: *mut HUF_CStream_t,
    mut startPtr: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
) -> size_t {
    ::libc::memset(
        bitC as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<HUF_CStream_t>() as ::libc::size_t,
    );
    (*bitC).startPtr = startPtr as *mut BYTE;
    (*bitC).ptr = (*bitC).startPtr;
    (*bitC).endPtr = (*bitC)
        .startPtr
        .offset(dstCapacity as isize)
        .offset(-(::core::mem::size_of::<size_t>() as usize as isize));
    if dstCapacity <= ::core::mem::size_of::<size_t>() as usize {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    return 0 as size_t;
}
#[inline(always)]
unsafe extern "C" fn HUF_addBits(
    mut bitC: *mut HUF_CStream_t,
    mut elt: HUF_CElt,
    mut idx: ::core::ffi::c_int,
    mut kFast: ::core::ffi::c_int,
) {
    (*bitC).bitContainer[idx as usize] >>= HUF_getNbBits(elt);
    (*bitC).bitContainer[idx as usize] = ((*bitC).bitContainer[idx as usize]
        as ::core::ffi::c_ulong
        | (if kFast != 0 {
            HUF_getValueFast(elt)
        } else {
            HUF_getValue(elt)
        }) as ::core::ffi::c_ulong) as size_t;
    (*bitC).bitPos[idx as usize] = ((*bitC).bitPos[idx as usize] as ::core::ffi::c_ulong)
        .wrapping_add(HUF_getNbBitsFast(elt) as ::core::ffi::c_ulong)
        as size_t as size_t;
}
#[inline(always)]
unsafe extern "C" fn HUF_zeroIndex1(mut bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[1 as ::core::ffi::c_int as usize] = 0 as size_t;
    (*bitC).bitPos[1 as ::core::ffi::c_int as usize] = 0 as size_t;
}
#[inline(always)]
unsafe extern "C" fn HUF_mergeIndex1(mut bitC: *mut HUF_CStream_t) {
    (*bitC).bitContainer[0 as ::core::ffi::c_int as usize] >>=
        (*bitC).bitPos[1 as ::core::ffi::c_int as usize] & 0xff as size_t;
    (*bitC).bitContainer[0 as ::core::ffi::c_int as usize] =
        ((*bitC).bitContainer[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_ulong
            | (*bitC).bitContainer[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_ulong)
            as size_t;
    (*bitC).bitPos[0 as ::core::ffi::c_int as usize] =
        ((*bitC).bitPos[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_ulong)
            .wrapping_add((*bitC).bitPos[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_ulong)
            as size_t as size_t;
}
#[inline(always)]
unsafe extern "C" fn HUF_flushBits(mut bitC: *mut HUF_CStream_t, mut kFast: ::core::ffi::c_int) {
    let nbBits: size_t = (*bitC).bitPos[0 as ::core::ffi::c_int as usize] & 0xff as size_t;
    let nbBytes: size_t = nbBits >> 3 as ::core::ffi::c_int;
    let bitContainer: size_t = (*bitC).bitContainer[0 as ::core::ffi::c_int as usize]
        >> HUF_BITS_IN_CONTAINER.wrapping_sub(nbBits as usize);
    (*bitC).bitPos[0 as ::core::ffi::c_int as usize] =
        ((*bitC).bitPos[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_ulong
            & 7 as ::core::ffi::c_ulong) as size_t;
    MEM_writeLEST((*bitC).ptr as *mut ::core::ffi::c_void, bitContainer);
    (*bitC).ptr = (*bitC).ptr.offset(nbBytes as isize);
    if kFast == 0 && (*bitC).ptr > (*bitC).endPtr {
        (*bitC).ptr = (*bitC).endPtr;
    }
}
unsafe extern "C" fn HUF_endMark() -> HUF_CElt {
    let mut endMark: HUF_CElt = 0;
    HUF_setNbBits(&raw mut endMark, 1 as size_t);
    HUF_setValue(&raw mut endMark, 1 as size_t);
    return endMark;
}
unsafe extern "C" fn HUF_closeCStream(mut bitC: *mut HUF_CStream_t) -> size_t {
    HUF_addBits(
        bitC,
        HUF_endMark(),
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    );
    HUF_flushBits(bitC, 0 as ::core::ffi::c_int);
    let nbBits: size_t = (*bitC).bitPos[0 as ::core::ffi::c_int as usize] & 0xff as size_t;
    if (*bitC).ptr >= (*bitC).endPtr {
        return 0 as size_t;
    }
    return ((*bitC).ptr.offset_from((*bitC).startPtr) as ::core::ffi::c_long as size_t)
        .wrapping_add((nbBits > 0 as size_t) as ::core::ffi::c_int as size_t);
}
#[inline(always)]
unsafe extern "C" fn HUF_encodeSymbol(
    mut bitCPtr: *mut HUF_CStream_t,
    mut symbol: U32,
    mut CTable: *const HUF_CElt,
    mut idx: ::core::ffi::c_int,
    mut fast: ::core::ffi::c_int,
) {
    HUF_addBits(bitCPtr, *CTable.offset(symbol as isize), idx, fast);
}
#[inline(always)]
unsafe extern "C" fn HUF_compress1X_usingCTable_internal_body_loop(
    mut bitC: *mut HUF_CStream_t,
    mut ip: *const BYTE,
    mut srcSize: size_t,
    mut ct: *const HUF_CElt,
    mut kUnroll: ::core::ffi::c_int,
    mut kFastFlush: ::core::ffi::c_int,
    mut kLastFast: ::core::ffi::c_int,
) {
    let mut n: ::core::ffi::c_int = srcSize as ::core::ffi::c_int;
    let mut rem: ::core::ffi::c_int = n % kUnroll;
    if rem > 0 as ::core::ffi::c_int {
        while rem > 0 as ::core::ffi::c_int {
            n -= 1;
            HUF_encodeSymbol(
                bitC,
                *ip.offset(n as isize) as U32,
                ct,
                0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
            rem -= 1;
        }
        HUF_flushBits(bitC, kFastFlush);
    }
    if n % (2 as ::core::ffi::c_int * kUnroll) != 0 {
        let mut u: ::core::ffi::c_int = 0;
        u = 1 as ::core::ffi::c_int;
        while u < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.offset((n - u) as isize) as U32,
                ct,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
            u += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.offset((n - kUnroll) as isize) as U32,
            ct,
            0 as ::core::ffi::c_int,
            kLastFast,
        );
        HUF_flushBits(bitC, kFastFlush);
        n -= kUnroll;
    }
    while n > 0 as ::core::ffi::c_int {
        let mut u_0: ::core::ffi::c_int = 0;
        u_0 = 1 as ::core::ffi::c_int;
        while u_0 < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.offset((n - u_0) as isize) as U32,
                ct,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
            u_0 += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.offset((n - kUnroll) as isize) as U32,
            ct,
            0 as ::core::ffi::c_int,
            kLastFast,
        );
        HUF_flushBits(bitC, kFastFlush);
        HUF_zeroIndex1(bitC);
        u_0 = 1 as ::core::ffi::c_int;
        while u_0 < kUnroll {
            HUF_encodeSymbol(
                bitC,
                *ip.offset((n - kUnroll - u_0) as isize) as U32,
                ct,
                1 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
            u_0 += 1;
        }
        HUF_encodeSymbol(
            bitC,
            *ip.offset((n - kUnroll - kUnroll) as isize) as U32,
            ct,
            1 as ::core::ffi::c_int,
            kLastFast,
        );
        HUF_mergeIndex1(bitC);
        HUF_flushBits(bitC, kFastFlush);
        n -= 2 as ::core::ffi::c_int * kUnroll;
    }
}
unsafe extern "C" fn HUF_tightCompressBound(mut srcSize: size_t, mut tableLog: size_t) -> size_t {
    return (srcSize.wrapping_mul(tableLog) >> 3 as ::core::ffi::c_int).wrapping_add(8 as size_t);
}
#[inline(always)]
unsafe extern "C" fn HUF_compress1X_usingCTable_internal_body(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut CTable: *const HUF_CElt,
) -> size_t {
    let tableLog: U32 = HUF_readCTableHeader(CTable).tableLog as U32;
    let mut ct: *const HUF_CElt = CTable.offset(1 as ::core::ffi::c_int as isize);
    let mut ip: *const BYTE = src as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut bitC: HUF_CStream_t = HUF_CStream_t {
        bitContainer: [0; 2],
        bitPos: [0; 2],
        startPtr: ::core::ptr::null_mut::<BYTE>(),
        ptr: ::core::ptr::null_mut::<BYTE>(),
        endPtr: ::core::ptr::null_mut::<BYTE>(),
    };
    if dstSize < 8 as size_t {
        return 0 as size_t;
    }
    let mut op: *mut BYTE = ostart;
    let initErr: size_t = HUF_initCStream(
        &raw mut bitC,
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(initErr) != 0 {
        return 0 as size_t;
    }
    if dstSize < HUF_tightCompressBound(srcSize, tableLog as size_t) || tableLog > 11 as U32 {
        HUF_compress1X_usingCTable_internal_body_loop(
            &raw mut bitC,
            ip,
            srcSize,
            ct,
            if MEM_32bits() != 0 {
                2 as ::core::ffi::c_int
            } else {
                4 as ::core::ffi::c_int
            },
            0 as ::core::ffi::c_int,
            0 as ::core::ffi::c_int,
        );
    } else if MEM_32bits() != 0 {
        match tableLog {
            11 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    2 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    0 as ::core::ffi::c_int,
                );
            }
            10 | 9 | 8 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    2 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                );
            }
            7 | _ => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    3 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                );
            }
        }
    } else {
        match tableLog {
            11 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    5 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    0 as ::core::ffi::c_int,
                );
            }
            10 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    5 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                );
            }
            9 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    6 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    0 as ::core::ffi::c_int,
                );
            }
            8 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    7 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    0 as ::core::ffi::c_int,
                );
            }
            7 => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    8 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    0 as ::core::ffi::c_int,
                );
            }
            6 | _ => {
                HUF_compress1X_usingCTable_internal_body_loop(
                    &raw mut bitC,
                    ip,
                    srcSize,
                    ct,
                    9 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                    1 as ::core::ffi::c_int,
                );
            }
        }
    }
    return HUF_closeCStream(&raw mut bitC);
}
unsafe extern "C" fn HUF_compress1X_usingCTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut CTable: *const HUF_CElt,
    flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_compress1X_usingCTable_internal_body(dst, dstSize, src, srcSize, CTable);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_usingCTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut CTable: *const HUF_CElt,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_compress1X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags);
}
unsafe extern "C" fn HUF_compress4X_usingCTable_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut CTable: *const HUF_CElt,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let segmentSize: size_t = srcSize.wrapping_add(3 as size_t).wrapping_div(4 as size_t);
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.offset(srcSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut op: *mut BYTE = ostart;
    if dstSize
        < (6 as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int
            + 8 as ::core::ffi::c_int) as size_t
    {
        return 0 as size_t;
    }
    if srcSize < 12 as size_t {
        return 0 as size_t;
    }
    op = op.offset(6 as ::core::ffi::c_int as isize);
    let cSize: size_t = HUF_compress1X_usingCTable_internal(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        ip as *const ::core::ffi::c_void,
        segmentSize,
        CTable,
        flags,
    ) as size_t;
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    if cSize == 0 as size_t || cSize > 65535 as size_t {
        return 0 as size_t;
    }
    MEM_writeLE16(ostart as *mut ::core::ffi::c_void, cSize as U16);
    op = op.offset(cSize as isize);
    ip = ip.offset(segmentSize as isize);
    let cSize_0: size_t = HUF_compress1X_usingCTable_internal(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        ip as *const ::core::ffi::c_void,
        segmentSize,
        CTable,
        flags,
    ) as size_t;
    if ERR_isError(cSize_0) != 0 {
        return cSize_0;
    }
    if cSize_0 == 0 as size_t || cSize_0 > 65535 as size_t {
        return 0 as size_t;
    }
    MEM_writeLE16(
        ostart.offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        cSize_0 as U16,
    );
    op = op.offset(cSize_0 as isize);
    ip = ip.offset(segmentSize as isize);
    let cSize_1: size_t = HUF_compress1X_usingCTable_internal(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        ip as *const ::core::ffi::c_void,
        segmentSize,
        CTable,
        flags,
    ) as size_t;
    if ERR_isError(cSize_1) != 0 {
        return cSize_1;
    }
    if cSize_1 == 0 as size_t || cSize_1 > 65535 as size_t {
        return 0 as size_t;
    }
    MEM_writeLE16(
        ostart.offset(4 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        cSize_1 as U16,
    );
    op = op.offset(cSize_1 as isize);
    ip = ip.offset(segmentSize as isize);
    let cSize_2: size_t = HUF_compress1X_usingCTable_internal(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
        CTable,
        flags,
    ) as size_t;
    if ERR_isError(cSize_2) != 0 {
        return cSize_2;
    }
    if cSize_2 == 0 as size_t || cSize_2 > 65535 as size_t {
        return 0 as size_t;
    }
    op = op.offset(cSize_2 as isize);
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_usingCTable(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut CTable: *const HUF_CElt,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_compress4X_usingCTable_internal(dst, dstSize, src, srcSize, CTable, flags);
}
unsafe extern "C" fn HUF_compressCTable_internal(
    ostart: *mut BYTE,
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut nbStreams: HUF_nbStreams_e,
    mut CTable: *const HUF_CElt,
    flags: ::core::ffi::c_int,
) -> size_t {
    let cSize: size_t = if nbStreams as ::core::ffi::c_uint
        == HUF_singleStream as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        HUF_compress1X_usingCTable_internal(
            op as *mut ::core::ffi::c_void,
            oend.offset_from(op) as ::core::ffi::c_long as size_t,
            src,
            srcSize,
            CTable,
            flags,
        ) as size_t
    } else {
        HUF_compress4X_usingCTable_internal(
            op as *mut ::core::ffi::c_void,
            oend.offset_from(op) as ::core::ffi::c_long as size_t,
            src,
            srcSize,
            CTable,
            flags,
        ) as size_t
    };
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    if cSize == 0 as size_t {
        return 0 as size_t;
    }
    op = op.offset(cSize as isize);
    if op.offset_from(ostart) as ::core::ffi::c_long as size_t >= srcSize.wrapping_sub(1 as size_t)
    {
        return 0 as size_t;
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
pub const SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_cardinality(
    mut count: *const ::core::ffi::c_uint,
    mut maxSymbolValue: ::core::ffi::c_uint,
) -> ::core::ffi::c_uint {
    let mut cardinality: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut i: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < maxSymbolValue.wrapping_add(1 as ::core::ffi::c_uint) {
        if *count.offset(i as isize) != 0 as ::core::ffi::c_uint {
            cardinality = cardinality.wrapping_add(1 as ::core::ffi::c_uint);
        }
        i = i.wrapping_add(1);
    }
    return cardinality;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_minTableLog(
    mut symbolCardinality: ::core::ffi::c_uint,
) -> ::core::ffi::c_uint {
    let mut minBitsSymbols: U32 =
        (ZSTD_highbit32(symbolCardinality as U32) as U32).wrapping_add(1 as U32);
    return minBitsSymbols as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_optimalTableLog(
    mut maxTableLog: ::core::ffi::c_uint,
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut table: *mut HUF_CElt,
    mut count: *const ::core::ffi::c_uint,
    mut flags: ::core::ffi::c_int,
) -> ::core::ffi::c_uint {
    if flags & HUF_flags_optimalDepth as ::core::ffi::c_int == 0 {
        return FSE_optimalTableLog_internal(
            maxTableLog,
            srcSize,
            maxSymbolValue,
            1 as ::core::ffi::c_uint,
        );
    }
    let mut dst: *mut BYTE = (workSpace as *mut BYTE)
        .offset(::core::mem::size_of::<HUF_WriteCTableWksp>() as usize as isize);
    let mut dstSize: size_t =
        wkspSize.wrapping_sub(::core::mem::size_of::<HUF_WriteCTableWksp>() as size_t);
    let mut hSize: size_t = 0;
    let mut newSize: size_t = 0;
    let symbolCardinality: ::core::ffi::c_uint =
        HUF_cardinality(count, maxSymbolValue) as ::core::ffi::c_uint;
    let minTableLog: ::core::ffi::c_uint =
        HUF_minTableLog(symbolCardinality) as ::core::ffi::c_uint;
    let mut optSize: size_t = (!(0 as ::core::ffi::c_int) as size_t).wrapping_sub(1 as size_t);
    let mut optLog: ::core::ffi::c_uint = maxTableLog;
    let mut optLogGuess: ::core::ffi::c_uint = 0;
    optLogGuess = minTableLog;
    while optLogGuess <= maxTableLog {
        let mut maxBits: size_t = HUF_buildCTable_wksp(
            table,
            count,
            maxSymbolValue as U32,
            optLogGuess as U32,
            workSpace,
            wkspSize,
        );
        if !(ERR_isError(maxBits) != 0) {
            if maxBits < optLogGuess as size_t && optLogGuess > minTableLog {
                break;
            }
            hSize = HUF_writeCTable_wksp(
                dst as *mut ::core::ffi::c_void,
                dstSize,
                table,
                maxSymbolValue,
                maxBits as ::core::ffi::c_uint,
                workSpace,
                wkspSize,
            );
            if !(ERR_isError(hSize) != 0) {
                newSize =
                    HUF_estimateCompressedSize(table, count, maxSymbolValue).wrapping_add(hSize);
                if newSize > optSize.wrapping_add(1 as size_t) {
                    break;
                }
                if newSize < optSize {
                    optSize = newSize;
                    optLog = optLogGuess;
                }
            }
        }
        optLogGuess = optLogGuess.wrapping_add(1);
    }
    return optLog;
}
unsafe extern "C" fn HUF_compress_internal(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut huffLog: ::core::ffi::c_uint,
    mut nbStreams: HUF_nbStreams_e,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut oldHufTable: *mut HUF_CElt,
    mut repeat: *mut HUF_repeat,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    let table: *mut HUF_compress_tables_t = HUF_alignUpWorkspace(
        workSpace,
        &raw mut wkspSize,
        ::core::mem::align_of::<size_t>(),
    ) as *mut HUF_compress_tables_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut op: *mut BYTE = ostart;
    if wkspSize < ::core::mem::size_of::<HUF_compress_tables_t>() as usize {
        return -(ZSTD_error_workSpace_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if srcSize == 0 {
        return 0 as size_t;
    }
    if dstSize == 0 {
        return 0 as size_t;
    }
    if srcSize > HUF_BLOCKSIZE_MAX as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if huffLog > HUF_TABLELOG_MAX as ::core::ffi::c_uint {
        return -(ZSTD_error_tableLog_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if maxSymbolValue > HUF_SYMBOLVALUE_MAX as ::core::ffi::c_uint {
        return -(ZSTD_error_maxSymbolValue_tooLarge as ::core::ffi::c_int) as size_t;
    }
    if maxSymbolValue == 0 {
        maxSymbolValue = HUF_SYMBOLVALUE_MAX as ::core::ffi::c_uint;
    }
    if huffLog == 0 {
        huffLog = HUF_TABLELOG_DEFAULT as ::core::ffi::c_uint;
    }
    if flags & HUF_flags_preferRepeat as ::core::ffi::c_int != 0
        && !repeat.is_null()
        && *repeat as ::core::ffi::c_uint
            == HUF_repeat_valid as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return HUF_compressCTable_internal(
            ostart,
            op,
            oend,
            src,
            srcSize,
            nbStreams,
            oldHufTable,
            flags,
        );
    }
    if flags & HUF_flags_suspectUncompressible as ::core::ffi::c_int != 0
        && srcSize
            >= (SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE * SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO) as size_t
    {
        let mut largestTotal: size_t = 0 as size_t;
        let mut maxSymbolValueBegin: ::core::ffi::c_uint = maxSymbolValue;
        let largestBegin: size_t = HIST_count_simple(
            &raw mut (*table).count as *mut ::core::ffi::c_uint,
            &raw mut maxSymbolValueBegin,
            src as *const BYTE as *const ::core::ffi::c_void,
            4096 as size_t,
        ) as size_t;
        if ERR_isError(largestBegin) != 0 {
            return largestBegin;
        }
        largestTotal = (largestTotal as ::core::ffi::c_ulong)
            .wrapping_add(largestBegin as ::core::ffi::c_ulong) as size_t
            as size_t;
        let mut maxSymbolValueEnd: ::core::ffi::c_uint = maxSymbolValue;
        let largestEnd: size_t = HIST_count_simple(
            &raw mut (*table).count as *mut ::core::ffi::c_uint,
            &raw mut maxSymbolValueEnd,
            (src as *const BYTE)
                .offset(srcSize as isize)
                .offset(-(4096 as ::core::ffi::c_int as isize))
                as *const ::core::ffi::c_void,
            4096 as size_t,
        ) as size_t;
        if ERR_isError(largestEnd) != 0 {
            return largestEnd;
        }
        largestTotal = (largestTotal as ::core::ffi::c_ulong)
            .wrapping_add(largestEnd as ::core::ffi::c_ulong) as size_t
            as size_t;
        if largestTotal
            <= ((2 as ::core::ffi::c_int * SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE
                >> 7 as ::core::ffi::c_int)
                + 4 as ::core::ffi::c_int) as size_t
        {
            return 0 as size_t;
        }
    }
    let largest: size_t = HIST_count_wksp(
        &raw mut (*table).count as *mut ::core::ffi::c_uint,
        &raw mut maxSymbolValue,
        src as *const BYTE as *const ::core::ffi::c_void,
        srcSize,
        &raw mut (*table).wksps.hist_wksp as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 1024]>() as size_t,
    ) as size_t;
    if ERR_isError(largest) != 0 {
        return largest;
    }
    if largest == srcSize {
        *ostart = *(src as *const BYTE).offset(0 as ::core::ffi::c_int as isize);
        return 1 as size_t;
    }
    if largest <= (srcSize >> 7 as ::core::ffi::c_int).wrapping_add(4 as size_t) {
        return 0 as size_t;
    }
    if !repeat.is_null()
        && *repeat as ::core::ffi::c_uint
            == HUF_repeat_check as ::core::ffi::c_int as ::core::ffi::c_uint
        && HUF_validateCTable(
            oldHufTable,
            &raw mut (*table).count as *mut ::core::ffi::c_uint,
            maxSymbolValue,
        ) == 0
    {
        *repeat = HUF_repeat_none;
    }
    if flags & HUF_flags_preferRepeat as ::core::ffi::c_int != 0
        && !repeat.is_null()
        && *repeat as ::core::ffi::c_uint
            != HUF_repeat_none as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return HUF_compressCTable_internal(
            ostart,
            op,
            oend,
            src,
            srcSize,
            nbStreams,
            oldHufTable,
            flags,
        );
    }
    huffLog = HUF_optimalTableLog(
        huffLog,
        srcSize,
        maxSymbolValue,
        &raw mut (*table).wksps as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<C2RustUnnamed_1>() as size_t,
        &raw mut (*table).CTable as *mut HUF_CElt,
        &raw mut (*table).count as *mut ::core::ffi::c_uint,
        flags,
    );
    let maxBits: size_t = HUF_buildCTable_wksp(
        &raw mut (*table).CTable as *mut HUF_CElt,
        &raw mut (*table).count as *mut ::core::ffi::c_uint,
        maxSymbolValue as U32,
        huffLog as U32,
        &raw mut (*table).wksps.buildCTable_wksp as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<HUF_buildCTable_wksp_tables>() as size_t,
    ) as size_t;
    let _var_err__: size_t = maxBits;
    if ERR_isError(_var_err__) != 0 {
        return _var_err__;
    }
    huffLog = maxBits as U32 as ::core::ffi::c_uint;
    let hSize: size_t = HUF_writeCTable_wksp(
        op as *mut ::core::ffi::c_void,
        dstSize,
        &raw mut (*table).CTable as *mut HUF_CElt,
        maxSymbolValue,
        huffLog,
        &raw mut (*table).wksps.writeCTable_wksp as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<HUF_WriteCTableWksp>() as size_t,
    ) as size_t;
    if ERR_isError(hSize) != 0 {
        return hSize;
    }
    if !repeat.is_null()
        && *repeat as ::core::ffi::c_uint
            != HUF_repeat_none as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let oldSize: size_t = HUF_estimateCompressedSize(
            oldHufTable,
            &raw mut (*table).count as *mut ::core::ffi::c_uint,
            maxSymbolValue,
        ) as size_t;
        let newSize: size_t = HUF_estimateCompressedSize(
            &raw mut (*table).CTable as *mut HUF_CElt,
            &raw mut (*table).count as *mut ::core::ffi::c_uint,
            maxSymbolValue,
        ) as size_t;
        if oldSize <= hSize.wrapping_add(newSize) || hSize.wrapping_add(12 as size_t) >= srcSize {
            return HUF_compressCTable_internal(
                ostart,
                op,
                oend,
                src,
                srcSize,
                nbStreams,
                oldHufTable,
                flags,
            );
        }
    }
    if hSize.wrapping_add(12 as size_t) >= srcSize {
        return 0 as size_t;
    }
    op = op.offset(hSize as isize);
    if !repeat.is_null() {
        *repeat = HUF_repeat_none;
    }
    if !oldHufTable.is_null() {
        ::libc::memcpy(
            oldHufTable as *mut ::core::ffi::c_void,
            &raw mut (*table).CTable as *mut HUF_CElt as *const ::core::ffi::c_void,
            ::core::mem::size_of::<[HUF_CElt; 257]>() as ::libc::size_t,
        );
    }
    return HUF_compressCTable_internal(
        ostart,
        op,
        oend,
        src,
        srcSize,
        nbStreams,
        &raw mut (*table).CTable as *mut HUF_CElt,
        flags,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress1X_repeat(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut huffLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut hufTable: *mut HUF_CElt,
    mut repeat: *mut HUF_repeat,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_compress_internal(
        dst,
        dstSize,
        src,
        srcSize,
        maxSymbolValue,
        huffLog,
        HUF_singleStream,
        workSpace,
        wkspSize,
        hufTable,
        repeat,
        flags,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_compress4X_repeat(
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut maxSymbolValue: ::core::ffi::c_uint,
    mut huffLog: ::core::ffi::c_uint,
    mut workSpace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut hufTable: *mut HUF_CElt,
    mut repeat: *mut HUF_repeat,
    mut flags: ::core::ffi::c_int,
) -> size_t {
    return HUF_compress_internal(
        dst,
        dstSize,
        src,
        srcSize,
        maxSymbolValue,
        huffLog,
        HUF_fourStreams,
        workSpace,
        wkspSize,
        hufTable,
        repeat,
        flags,
    );
}
