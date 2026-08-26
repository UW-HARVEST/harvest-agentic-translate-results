#[cfg(target_arch = "x86")]
pub use ::core::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
#[cfg(target_arch = "x86_64")]
pub use ::core::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
pub type __m128i_u = __m128i;
use ::libc;
extern "C" {
    fn ZSTD_XXH64(
        input: *const ::core::ffi::c_void,
        length: size_t,
        seed: XXH64_hash_t,
    ) -> XXH64_hash_t;
    fn ZSTD_selectBlockCompressor(
        strat: ZSTD_strategy,
        rowMatchfinderMode: ZSTD_ParamSwitch_e,
        dictMode: ZSTD_dictMode_e,
    ) -> ZSTD_BlockCompressor_f;
    fn ZSTD_fillHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const ::core::ffi::c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    fn ZSTD_fillDoubleHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const ::core::ffi::c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
}
pub type ptrdiff_t = isize;
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
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
pub type U16 = uint16_t;
pub type U32 = uint32_t;
pub type U64 = uint64_t;
pub type unalign16 = U16;
pub type unalign32 = U32;
pub type unalignArch = size_t;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SeqStore_t {
    pub sequencesStart: *mut SeqDef,
    pub sequences: *mut SeqDef,
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE,
    pub llCode: *mut BYTE,
    pub mlCode: *mut BYTE,
    pub ofCode: *mut BYTE,
    pub maxNbSeq: size_t,
    pub maxNbLit: size_t,
    pub longLengthType: ZSTD_longLengthType_e,
    pub longLengthPos: U32,
}
pub type ZSTD_longLengthType_e = ::core::ffi::c_uint;
pub const ZSTD_llt_matchLength: ZSTD_longLengthType_e = 2;
pub const ZSTD_llt_literalLength: ZSTD_longLengthType_e = 1;
pub const ZSTD_llt_none: ZSTD_longLengthType_e = 0;
pub type SeqDef = SeqDef_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SeqDef_s {
    pub offBase: U32,
    pub litLength: U16,
    pub mlBase: U16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_MatchState_t {
    pub window: ZSTD_window_t,
    pub loadedDictEnd: U32,
    pub nextToUpdate: U32,
    pub hashLog3: U32,
    pub rowHashLog: U32,
    pub tagTable: *mut BYTE,
    pub hashCache: [U32; 8],
    pub hashSalt: U64,
    pub hashSaltEntropy: U32,
    pub hashTable: *mut U32,
    pub hashTable3: *mut U32,
    pub chainTable: *mut U32,
    pub forceNonContiguous: ::core::ffi::c_int,
    pub dedicatedDictSearch: ::core::ffi::c_int,
    pub opt: optState_t,
    pub dictMatchState: *const ZSTD_MatchState_t,
    pub cParams: ZSTD_compressionParameters,
    pub ldmSeqStore: *const RawSeqStore_t,
    pub prefetchCDictTables: ::core::ffi::c_int,
    pub lazySkipping: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RawSeqStore_t {
    pub seq: *mut rawSeq,
    pub pos: size_t,
    pub posInSequence: size_t,
    pub size: size_t,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rawSeq {
    pub offset: U32,
    pub litLength: U32,
    pub matchLength: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_compressionParameters {
    pub windowLog: ::core::ffi::c_uint,
    pub chainLog: ::core::ffi::c_uint,
    pub hashLog: ::core::ffi::c_uint,
    pub searchLog: ::core::ffi::c_uint,
    pub minMatch: ::core::ffi::c_uint,
    pub targetLength: ::core::ffi::c_uint,
    pub strategy: ZSTD_strategy,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct optState_t {
    pub litFreq: *mut ::core::ffi::c_uint,
    pub litLengthFreq: *mut ::core::ffi::c_uint,
    pub matchLengthFreq: *mut ::core::ffi::c_uint,
    pub offCodeFreq: *mut ::core::ffi::c_uint,
    pub matchTable: *mut ZSTD_match_t,
    pub priceTable: *mut ZSTD_optimal_t,
    pub litSum: U32,
    pub litLengthSum: U32,
    pub matchLengthSum: U32,
    pub offCodeSum: U32,
    pub litSumBasePrice: U32,
    pub litLengthSumBasePrice: U32,
    pub matchLengthSumBasePrice: U32,
    pub offCodeSumBasePrice: U32,
    pub priceType: ZSTD_OptPrice_e,
    pub symbolCosts: *const ZSTD_entropyCTables_t,
    pub literalCompressionMode: ZSTD_ParamSwitch_e,
}
pub type ZSTD_ParamSwitch_e = ::core::ffi::c_uint;
pub const ZSTD_ps_disable: ZSTD_ParamSwitch_e = 2;
pub const ZSTD_ps_enable: ZSTD_ParamSwitch_e = 1;
pub const ZSTD_ps_auto: ZSTD_ParamSwitch_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_entropyCTables_t {
    pub huf: ZSTD_hufCTables_t,
    pub fse: ZSTD_fseCTables_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_fseCTables_t {
    pub offcodeCTable: [FSE_CTable; 193],
    pub matchlengthCTable: [FSE_CTable; 363],
    pub litlengthCTable: [FSE_CTable; 329],
    pub offcode_repeatMode: FSE_repeat,
    pub matchlength_repeatMode: FSE_repeat,
    pub litlength_repeatMode: FSE_repeat,
}
pub type FSE_repeat = ::core::ffi::c_uint;
pub const FSE_repeat_valid: FSE_repeat = 2;
pub const FSE_repeat_check: FSE_repeat = 1;
pub const FSE_repeat_none: FSE_repeat = 0;
pub type FSE_CTable = ::core::ffi::c_uint;
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
pub type ZSTD_OptPrice_e = ::core::ffi::c_uint;
pub const zop_predef: ZSTD_OptPrice_e = 1;
pub const zop_dynamic: ZSTD_OptPrice_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_optimal_t {
    pub price: ::core::ffi::c_int,
    pub off: U32,
    pub mlen: U32,
    pub litlen: U32,
    pub rep: [U32; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_match_t {
    pub off: U32,
    pub len: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_window_t {
    pub nextSrc: *const BYTE,
    pub base: *const BYTE,
    pub dictBase: *const BYTE,
    pub dictLimit: U32,
    pub lowLimit: U32,
    pub nbOverflowCorrections: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmState_t {
    pub window: ZSTD_window_t,
    pub hashTable: *mut ldmEntry_t,
    pub loadedDictEnd: U32,
    pub bucketOffsets: *mut BYTE,
    pub splitIndices: [size_t; 64],
    pub matchCandidates: [ldmMatchCandidate_t; 64],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmMatchCandidate_t {
    pub split: *const BYTE,
    pub hash: U32,
    pub checksum: U32,
    pub bucket: *mut ldmEntry_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmEntry_t {
    pub offset: U32,
    pub checksum: U32,
}
pub type XXH64_hash_t = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmParams_t {
    pub enableLdm: ZSTD_ParamSwitch_e,
    pub hashLog: U32,
    pub bucketSizeLog: U32,
    pub minMatchLength: U32,
    pub hashRateLog: U32,
    pub windowLog: U32,
}
pub type ZSTD_overlap_e = ::core::ffi::c_uint;
pub const ZSTD_overlap_src_before_dst: ZSTD_overlap_e = 1;
pub const ZSTD_no_overlap: ZSTD_overlap_e = 0;
pub type ZSTD_dictTableLoadMethod_e = ::core::ffi::c_uint;
pub const ZSTD_dtlm_full: ZSTD_dictTableLoadMethod_e = 1;
pub const ZSTD_dtlm_fast: ZSTD_dictTableLoadMethod_e = 0;
pub type ZSTD_tableFillPurpose_e = ::core::ffi::c_uint;
pub const ZSTD_tfp_forCDict: ZSTD_tableFillPurpose_e = 1;
pub const ZSTD_tfp_forCCtx: ZSTD_tableFillPurpose_e = 0;
pub type ZSTD_dictMode_e = ::core::ffi::c_uint;
pub const ZSTD_dedicatedDictSearch: ZSTD_dictMode_e = 3;
pub const ZSTD_dictMatchState: ZSTD_dictMode_e = 2;
pub const ZSTD_extDict: ZSTD_dictMode_e = 1;
pub const ZSTD_noDict: ZSTD_dictMode_e = 0;
pub type ZSTD_BlockCompressor_f = Option<
    unsafe extern "C" fn(
        *mut ZSTD_MatchState_t,
        *mut SeqStore_t,
        *mut U32,
        *const ::core::ffi::c_void,
        size_t,
    ) -> size_t,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmRollingHashState_t {
    pub rolling: U64,
    pub stopMask: U64,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
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
unsafe extern "C" fn MEM_readST(mut ptr: *const ::core::ffi::c_void) -> size_t {
    return *(ptr as *const unalignArch);
}
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_countTrailingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.trailing_zeros() as i32 as ::core::ffi::c_uint;
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
unsafe extern "C" fn ZSTD_countLeadingZeros64(mut val: U64) -> ::core::ffi::c_uint {
    return (val as ::core::ffi::c_ulonglong).leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_NbCommonBytes(mut val: size_t) -> ::core::ffi::c_uint {
    if MEM_isLittleEndian() != 0 {
        if MEM_64bits() != 0 {
            return ZSTD_countTrailingZeros64(val as U64) >> 3 as ::core::ffi::c_int;
        } else {
            return ZSTD_countTrailingZeros32(val as U32) >> 3 as ::core::ffi::c_int;
        }
    } else if MEM_64bits() != 0 {
        return ZSTD_countLeadingZeros64(val as U64) >> 3 as ::core::ffi::c_int;
    } else {
        return ZSTD_countLeadingZeros32(val as U32) >> 3 as ::core::ffi::c_int;
    };
}
pub const ZSTD_REP_NUM: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MINMATCH: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
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
#[inline]
unsafe extern "C" fn ZSTD_cwksp_alloc_size(mut size: size_t) -> size_t {
    if size == 0 as size_t {
        return 0 as size_t;
    }
    return size;
}
pub const HASH_READ_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const ZSTD_WINDOW_START_INDEX: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const LDM_BATCH_SIZE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_safecopyLiterals(
    mut op: *mut BYTE,
    mut ip: *const BYTE,
    iend: *const BYTE,
    mut ilimit_w: *const BYTE,
) {
    if ip <= ilimit_w {
        ZSTD_wildcopy(
            op as *mut ::core::ffi::c_void,
            ip as *const ::core::ffi::c_void,
            ilimit_w.offset_from(ip) as ptrdiff_t,
            ZSTD_no_overlap,
        );
        op = op.offset(ilimit_w.offset_from(ip) as ::core::ffi::c_long as isize);
        ip = ilimit_w;
    }
    while ip < iend {
        let fresh0 = ip;
        ip = ip.offset(1);
        let fresh1 = op;
        op = op.offset(1);
        *fresh1 = *fresh0;
    }
}
#[inline(always)]
unsafe extern "C" fn ZSTD_storeSeqOnly(
    mut seqStorePtr: *mut SeqStore_t,
    mut litLength: size_t,
    mut offBase: U32,
    mut matchLength: size_t,
) {
    if (litLength > 0xffff as size_t) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        (*seqStorePtr).longLengthType = ZSTD_llt_literalLength;
        (*seqStorePtr).longLengthPos = (*seqStorePtr)
            .sequences
            .offset_from((*seqStorePtr).sequencesStart)
            as ::core::ffi::c_long as U32;
    }
    (*(*seqStorePtr)
        .sequences
        .offset(0 as ::core::ffi::c_int as isize))
    .litLength = litLength as U16;
    (*(*seqStorePtr)
        .sequences
        .offset(0 as ::core::ffi::c_int as isize))
    .offBase = offBase;
    let mlBase: size_t = matchLength.wrapping_sub(MINMATCH as size_t);
    if (mlBase > 0xffff as size_t) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        (*seqStorePtr).longLengthType = ZSTD_llt_matchLength;
        (*seqStorePtr).longLengthPos = (*seqStorePtr)
            .sequences
            .offset_from((*seqStorePtr).sequencesStart)
            as ::core::ffi::c_long as U32;
    }
    (*(*seqStorePtr)
        .sequences
        .offset(0 as ::core::ffi::c_int as isize))
    .mlBase = mlBase as U16;
    (*seqStorePtr).sequences = (*seqStorePtr).sequences.offset(1);
}
#[inline(always)]
unsafe extern "C" fn ZSTD_storeSeq(
    mut seqStorePtr: *mut SeqStore_t,
    mut litLength: size_t,
    mut literals: *const BYTE,
    mut litLimit: *const BYTE,
    mut offBase: U32,
    mut matchLength: size_t,
) {
    let litLimit_w: *const BYTE = litLimit.offset(-(WILDCOPY_OVERLENGTH as isize));
    let litEnd: *const BYTE = literals.offset(litLength as isize);
    if litEnd <= litLimit_w {
        ZSTD_copy16(
            (*seqStorePtr).lit as *mut ::core::ffi::c_void,
            literals as *const ::core::ffi::c_void,
        );
        if litLength > 16 as size_t {
            ZSTD_wildcopy(
                (*seqStorePtr).lit.offset(16 as ::core::ffi::c_int as isize)
                    as *mut ::core::ffi::c_void,
                literals.offset(16 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
                litLength as ptrdiff_t - 16 as ptrdiff_t,
                ZSTD_no_overlap,
            );
        }
    } else {
        ZSTD_safecopyLiterals((*seqStorePtr).lit, literals, litEnd, litLimit_w);
    }
    (*seqStorePtr).lit = (*seqStorePtr).lit.offset(litLength as isize);
    ZSTD_storeSeqOnly(seqStorePtr, litLength, offBase, matchLength);
}
#[inline]
unsafe extern "C" fn ZSTD_count(
    mut pIn: *const BYTE,
    mut pMatch: *const BYTE,
    pInLimit: *const BYTE,
) -> size_t {
    let pStart: *const BYTE = pIn;
    let pInLoopLimit: *const BYTE = pInLimit
        .offset(-((::core::mem::size_of::<size_t>() as usize).wrapping_sub(1 as usize) as isize));
    if pIn < pInLoopLimit {
        let diff: size_t = MEM_readST(pMatch as *const ::core::ffi::c_void) as size_t
            ^ MEM_readST(pIn as *const ::core::ffi::c_void) as size_t;
        if diff != 0 {
            return ZSTD_NbCommonBytes(diff) as size_t;
        }
        pIn = pIn.offset(::core::mem::size_of::<size_t>() as usize as isize);
        pMatch = pMatch.offset(::core::mem::size_of::<size_t>() as usize as isize);
        while pIn < pInLoopLimit {
            let diff_0: size_t = MEM_readST(pMatch as *const ::core::ffi::c_void) as size_t
                ^ MEM_readST(pIn as *const ::core::ffi::c_void) as size_t;
            if diff_0 == 0 {
                pIn = pIn.offset(::core::mem::size_of::<size_t>() as usize as isize);
                pMatch = pMatch.offset(::core::mem::size_of::<size_t>() as usize as isize);
            } else {
                pIn = pIn.offset(ZSTD_NbCommonBytes(diff_0) as isize);
                return pIn.offset_from(pStart) as ::core::ffi::c_long as size_t;
            }
        }
    }
    if MEM_64bits() != 0
        && pIn < pInLimit.offset(-(3 as ::core::ffi::c_int as isize))
        && MEM_read32(pMatch as *const ::core::ffi::c_void)
            == MEM_read32(pIn as *const ::core::ffi::c_void)
    {
        pIn = pIn.offset(4 as ::core::ffi::c_int as isize);
        pMatch = pMatch.offset(4 as ::core::ffi::c_int as isize);
    }
    if pIn < pInLimit.offset(-(1 as ::core::ffi::c_int as isize))
        && MEM_read16(pMatch as *const ::core::ffi::c_void) as ::core::ffi::c_int
            == MEM_read16(pIn as *const ::core::ffi::c_void) as ::core::ffi::c_int
    {
        pIn = pIn.offset(2 as ::core::ffi::c_int as isize);
        pMatch = pMatch.offset(2 as ::core::ffi::c_int as isize);
    }
    if pIn < pInLimit && *pMatch as ::core::ffi::c_int == *pIn as ::core::ffi::c_int {
        pIn = pIn.offset(1);
    }
    return pIn.offset_from(pStart) as ::core::ffi::c_long as size_t;
}
#[inline]
unsafe extern "C" fn ZSTD_count_2segments(
    mut ip: *const BYTE,
    mut match_0: *const BYTE,
    mut iEnd: *const BYTE,
    mut mEnd: *const BYTE,
    mut iStart: *const BYTE,
) -> size_t {
    let vEnd: *const BYTE =
        if ip.offset(mEnd.offset_from(match_0) as ::core::ffi::c_long as isize) < iEnd {
            ip.offset(mEnd.offset_from(match_0) as ::core::ffi::c_long as isize)
        } else {
            iEnd
        };
    let matchLength: size_t = ZSTD_count(ip, match_0, vEnd) as size_t;
    if match_0.offset(matchLength as isize) != mEnd {
        return matchLength;
    }
    return matchLength.wrapping_add(ZSTD_count(ip.offset(matchLength as isize), iStart, iEnd));
}
#[inline]
unsafe extern "C" fn ZSTD_window_hasExtDict(window: ZSTD_window_t) -> U32 {
    return (window.lowLimit < window.dictLimit) as ::core::ffi::c_int as U32;
}
#[inline]
unsafe extern "C" fn ZSTD_matchState_dictMode(mut ms: *const ZSTD_MatchState_t) -> ZSTD_dictMode_e {
    return (if ZSTD_window_hasExtDict((*ms).window) != 0 {
        ZSTD_extDict as ::core::ffi::c_int
    } else if !(*ms).dictMatchState.is_null() {
        if (*(*ms).dictMatchState).dedicatedDictSearch != 0 {
            ZSTD_dedicatedDictSearch as ::core::ffi::c_int
        } else {
            ZSTD_dictMatchState as ::core::ffi::c_int
        }
    } else {
        ZSTD_noDict as ::core::ffi::c_int
    }) as ZSTD_dictMode_e;
}
pub const ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTD_window_canOverflowCorrect(
    window: ZSTD_window_t,
    mut cycleLog: U32,
    mut maxDist: U32,
    mut loadedDictEnd: U32,
    mut src: *const ::core::ffi::c_void,
) -> U32 {
    let cycleSize: U32 = (1 as U32) << cycleLog;
    let curr: U32 = (src as *const BYTE).offset_from(window.base) as ::core::ffi::c_long as U32;
    let minIndexToOverflowCorrect: U32 = cycleSize
        .wrapping_add(
            (if maxDist > cycleSize {
                maxDist
            } else {
                cycleSize
            }),
        )
        .wrapping_add(ZSTD_WINDOW_START_INDEX as U32);
    let adjustment: U32 = window.nbOverflowCorrections.wrapping_add(1 as U32);
    let adjustedIndex: U32 =
        if minIndexToOverflowCorrect.wrapping_mul(adjustment) > minIndexToOverflowCorrect {
            minIndexToOverflowCorrect.wrapping_mul(adjustment)
        } else {
            minIndexToOverflowCorrect
        };
    let indexLargeEnough: U32 = (curr > adjustedIndex) as ::core::ffi::c_int as U32;
    let dictionaryInvalidated: U32 =
        (curr > maxDist.wrapping_add(loadedDictEnd)) as ::core::ffi::c_int as U32;
    return (indexLargeEnough != 0 && dictionaryInvalidated != 0) as ::core::ffi::c_int as U32;
}
#[inline]
unsafe extern "C" fn ZSTD_window_needOverflowCorrection(
    window: ZSTD_window_t,
    mut cycleLog: U32,
    mut maxDist: U32,
    mut loadedDictEnd: U32,
    mut src: *const ::core::ffi::c_void,
    mut srcEnd: *const ::core::ffi::c_void,
) -> U32 {
    let curr: U32 = (srcEnd as *const BYTE).offset_from(window.base) as ::core::ffi::c_long as U32;
    return (curr
        > (if MEM_64bits() != 0 {
            (3500 as U32)
                .wrapping_mul(((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as U32)
        } else {
            (2000 as U32)
                .wrapping_mul(((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as U32)
        })) as ::core::ffi::c_int as U32;
}
#[inline]
unsafe extern "C" fn ZSTD_window_correctOverflow(
    mut window: *mut ZSTD_window_t,
    mut cycleLog: U32,
    mut maxDist: U32,
    mut src: *const ::core::ffi::c_void,
) -> U32 {
    let cycleSize: U32 = (1 as U32) << cycleLog;
    let cycleMask: U32 = cycleSize.wrapping_sub(1 as U32);
    let curr: U32 = (src as *const BYTE).offset_from((*window).base) as ::core::ffi::c_long as U32;
    let currentCycle: U32 = curr & cycleMask;
    let currentCycleCorrection: U32 = if currentCycle < ZSTD_WINDOW_START_INDEX as U32 {
        if cycleSize > 2 as U32 {
            cycleSize
        } else {
            2 as U32
        }
    } else {
        0 as U32
    };
    let newCurrent: U32 = currentCycle
        .wrapping_add(currentCycleCorrection)
        .wrapping_add(
            (if maxDist > cycleSize {
                maxDist
            } else {
                cycleSize
            }),
        );
    let correction: U32 = curr.wrapping_sub(newCurrent);
    ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY == 0;
    (*window).base = (*window).base.offset(correction as isize);
    (*window).dictBase = (*window).dictBase.offset(correction as isize);
    if (*window).lowLimit < correction.wrapping_add(ZSTD_WINDOW_START_INDEX as U32) {
        (*window).lowLimit = ZSTD_WINDOW_START_INDEX as U32;
    } else {
        (*window).lowLimit = ((*window).lowLimit as ::core::ffi::c_uint)
            .wrapping_sub(correction as ::core::ffi::c_uint) as U32
            as U32;
    }
    if (*window).dictLimit < correction.wrapping_add(ZSTD_WINDOW_START_INDEX as U32) {
        (*window).dictLimit = ZSTD_WINDOW_START_INDEX as U32;
    } else {
        (*window).dictLimit = ((*window).dictLimit as ::core::ffi::c_uint)
            .wrapping_sub(correction as ::core::ffi::c_uint) as U32
            as U32;
    }
    (*window).nbOverflowCorrections = (*window).nbOverflowCorrections.wrapping_add(1);
    return correction;
}
#[inline]
unsafe extern "C" fn ZSTD_window_enforceMaxDist(
    mut window: *mut ZSTD_window_t,
    mut blockEnd: *const ::core::ffi::c_void,
    mut maxDist: U32,
    mut loadedDictEndPtr: *mut U32,
    mut dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    let blockEndIdx: U32 =
        (blockEnd as *const BYTE).offset_from((*window).base) as ::core::ffi::c_long as U32;
    let loadedDictEnd: U32 = if !loadedDictEndPtr.is_null() {
        *loadedDictEndPtr
    } else {
        0 as U32
    };
    if blockEndIdx > maxDist.wrapping_add(loadedDictEnd) {
        let newLowLimit: U32 = blockEndIdx.wrapping_sub(maxDist);
        if (*window).lowLimit < newLowLimit {
            (*window).lowLimit = newLowLimit;
        }
        if (*window).dictLimit < (*window).lowLimit {
            (*window).dictLimit = (*window).lowLimit;
        }
        if !loadedDictEndPtr.is_null() {
            *loadedDictEndPtr = 0 as U32;
        }
        if !dictMatchStatePtr.is_null() {
            *dictMatchStatePtr = ::core::ptr::null::<ZSTD_MatchState_t>();
        }
    }
}
static mut ZSTD_ldm_gearTab: [U64; 256] = [
    0xf5b8f72c5f77775c as ::core::ffi::c_ulong,
    0x84935f266b7ac412 as ::core::ffi::c_ulong,
    0xb647ada9ca730ccc as ::core::ffi::c_ulong,
    0xb065bb4b114fb1de as ::core::ffi::c_ulong,
    0x34584e7e8c3a9fd0 as ::core::ffi::c_long as U64,
    0x4e97e17c6ae26b05 as ::core::ffi::c_long as U64,
    0x3a03d743bc99a604 as ::core::ffi::c_long as U64,
    0xcecd042422c4044f as ::core::ffi::c_ulong,
    0x76de76c58524259e as ::core::ffi::c_long as U64,
    0x9c8528f65badeaca as ::core::ffi::c_ulong,
    0x86563706e2097529 as ::core::ffi::c_ulong,
    0x2902475fa375d889 as ::core::ffi::c_long as U64,
    0xafb32a9739a5ebe6 as ::core::ffi::c_ulong,
    0xce2714da3883e639 as ::core::ffi::c_ulong,
    0x21eaf821722e69e as ::core::ffi::c_long as U64,
    0x37b628620b628 as ::core::ffi::c_long as U64,
    0x49a8d455d88caf5 as ::core::ffi::c_long as U64,
    0x8556d711e6958140 as ::core::ffi::c_ulong,
    0x4f7ae74fc605c1f as ::core::ffi::c_long as U64,
    0x829f0c3468bd3a20 as ::core::ffi::c_ulong,
    0x4ffdc885c625179e as ::core::ffi::c_long as U64,
    0x8473de048a3daf1b as ::core::ffi::c_ulong,
    0x51008822b05646b2 as ::core::ffi::c_long as U64,
    0x69d75d12b2d1cc5f as ::core::ffi::c_long as U64,
    0x8c9d4a19159154bc as ::core::ffi::c_ulong,
    0xc3cc10f4abbd4003 as ::core::ffi::c_ulong,
    0xd06ddc1cecb97391 as ::core::ffi::c_ulong,
    0xbe48e6e7ed80302e as ::core::ffi::c_ulong,
    0x3481db31cee03547 as ::core::ffi::c_long as U64,
    0xacc3f67cdaa1d210 as ::core::ffi::c_ulong,
    0x65cb771d8c7f96cc as ::core::ffi::c_long as U64,
    0x8eb27177055723dd as ::core::ffi::c_ulong,
    0xc789950d44cd94be as ::core::ffi::c_ulong,
    0x934feadc3700b12b as ::core::ffi::c_ulong,
    0x5e485f11edbdf182 as ::core::ffi::c_long as U64,
    0x1e2e2a46fd64767a as ::core::ffi::c_long as U64,
    0x2969ca71d82efa7c as ::core::ffi::c_long as U64,
    0x9d46e9935ebbba2e as ::core::ffi::c_ulong,
    0xe056b67e05e6822b as ::core::ffi::c_ulong,
    0x94d73f55739d03a0 as ::core::ffi::c_ulong,
    0xcd7010bdb69b5a03 as ::core::ffi::c_ulong,
    0x455ef9fcd79b82f4 as ::core::ffi::c_long as U64,
    0x869cb54a8749c161 as ::core::ffi::c_ulong,
    0x38d1a4fa6185d225 as ::core::ffi::c_long as U64,
    0xb475166f94bbe9bb as ::core::ffi::c_ulong,
    0xa4143548720959f1 as ::core::ffi::c_ulong,
    0x7aed4780ba6b26ba as ::core::ffi::c_long as U64,
    0xd0ce264439e02312 as ::core::ffi::c_ulong,
    0x84366d746078d508 as ::core::ffi::c_ulong,
    0xa8ce973c72ed17be as ::core::ffi::c_ulong,
    0x21c323a29a430b01 as ::core::ffi::c_long as U64,
    0x9962d617e3af80ee as ::core::ffi::c_ulong,
    0xab0ce91d9c8cf75b as ::core::ffi::c_ulong,
    0x530e8ee6d19a4dbc as ::core::ffi::c_long as U64,
    0x2ef68c0cf53f5d72 as ::core::ffi::c_long as U64,
    0xc03a681640a85506 as ::core::ffi::c_ulong,
    0x496e4e9f9c310967 as ::core::ffi::c_long as U64,
    0x78580472b59b14a0 as ::core::ffi::c_long as U64,
    0x273824c23b388577 as ::core::ffi::c_long as U64,
    0x66bf923ad45cb553 as ::core::ffi::c_long as U64,
    0x47ae1a5a2492ba86 as ::core::ffi::c_long as U64,
    0x35e304569e229659 as ::core::ffi::c_long as U64,
    0x4765182a46870b6f as ::core::ffi::c_long as U64,
    0x6cbab625e9099412 as ::core::ffi::c_long as U64,
    0xddac9a2e598522c1 as ::core::ffi::c_ulong,
    0x7172086e666624f2 as ::core::ffi::c_long as U64,
    0xdf5003ca503b7837 as ::core::ffi::c_ulong,
    0x88c0c1db78563d09 as ::core::ffi::c_ulong,
    0x58d51865acfc289d as ::core::ffi::c_long as U64,
    0x177671aec65224f1 as ::core::ffi::c_long as U64,
    0xfb79d8a241e967d7 as ::core::ffi::c_ulong,
    0x2be1e101cad9a49a as ::core::ffi::c_long as U64,
    0x6625682f6e29186b as ::core::ffi::c_long as U64,
    0x399553457ac06e50 as ::core::ffi::c_long as U64,
    0x35dffb4c23abb74 as ::core::ffi::c_long as U64,
    0x429db2591f54aade as ::core::ffi::c_long as U64,
    0xc52802a8037d1009 as ::core::ffi::c_ulong,
    0x6acb27381f0b25f3 as ::core::ffi::c_long as U64,
    0xf45e2551ee4f823b as ::core::ffi::c_ulong,
    0x8b0ea2d99580c2f7 as ::core::ffi::c_ulong,
    0x3bed519cbcb4e1e1 as ::core::ffi::c_long as U64,
    0xff452823dbb010a as ::core::ffi::c_long as U64,
    0x9d42ed614f3dd267 as ::core::ffi::c_ulong,
    0x5b9313c06257c57b as ::core::ffi::c_long as U64,
    0xa114b8008b5e1442 as ::core::ffi::c_ulong,
    0xc1fe311c11c13d4b as ::core::ffi::c_ulong,
    0x66e8763ea34c5568 as ::core::ffi::c_long as U64,
    0x8b982af1c262f05d as ::core::ffi::c_ulong,
    0xee8876faaa75fbb7 as ::core::ffi::c_ulong,
    0x8a62a4d0d172bb2a as ::core::ffi::c_ulong,
    0xc13d94a3b7449a97 as ::core::ffi::c_ulong,
    0x6dbbba9dc15d037c as ::core::ffi::c_long as U64,
    0xc786101f1d92e0f1 as ::core::ffi::c_ulong,
    0xd78681a907a0b79b as ::core::ffi::c_ulong,
    0xf61aaf2962c9abb9 as ::core::ffi::c_ulong,
    0x2cfd16fcd3cb7ad9 as ::core::ffi::c_long as U64,
    0x868c5b6744624d21 as ::core::ffi::c_ulong,
    0x25e650899c74ddd7 as ::core::ffi::c_long as U64,
    0xba042af4a7c37463 as ::core::ffi::c_ulong,
    0x4eb1a539465a3eca as ::core::ffi::c_long as U64,
    0xbe09dbf03b05d5ca as ::core::ffi::c_ulong,
    0x774e5a362b5472ba as ::core::ffi::c_long as U64,
    0x47a1221229d183cd as ::core::ffi::c_long as U64,
    0x504b0ca18ef5a2df as ::core::ffi::c_long as U64,
    0xdffbdfbde2456eb9 as ::core::ffi::c_ulong,
    0x46cd2b2fbee34634 as ::core::ffi::c_long as U64,
    0xf2aef8fe819d98c3 as ::core::ffi::c_ulong,
    0x357f5276d4599d61 as ::core::ffi::c_long as U64,
    0x24a5483879c453e3 as ::core::ffi::c_long as U64,
    0x88026889192b4b9 as ::core::ffi::c_long as U64,
    0x28da96671782dbec as ::core::ffi::c_long as U64,
    0x4ef37c40588e9aaa as ::core::ffi::c_long as U64,
    0x8837b90651bc9fb3 as ::core::ffi::c_ulong,
    0xc164f741d3f0e5d6 as ::core::ffi::c_ulong,
    0xbc135a0a704b70ba as ::core::ffi::c_ulong,
    0x69cd868f7622ada as ::core::ffi::c_long as U64,
    0xbc37ba89e0b9c0ab as ::core::ffi::c_ulong,
    0x47c14a01323552f6 as ::core::ffi::c_long as U64,
    0x4f00794bacee98bb as ::core::ffi::c_long as U64,
    0x7107de7d637a69d5 as ::core::ffi::c_long as U64,
    0x88af793bb6f2255e as ::core::ffi::c_ulong,
    0xf3c6466b8799b598 as ::core::ffi::c_ulong,
    0xc288c616aa7f3b59 as ::core::ffi::c_ulong,
    0x81ca63cf42fca3fd as ::core::ffi::c_ulong,
    0x88d85ace36a2674b as ::core::ffi::c_ulong,
    0xd056bd3792389e7 as ::core::ffi::c_long as U64,
    0xe55c396c4e9dd32d as ::core::ffi::c_ulong,
    0xbefb504571e6c0a6 as ::core::ffi::c_ulong,
    0x96ab32115e91e8cc as ::core::ffi::c_ulong,
    0xbf8acb18de8f38d1 as ::core::ffi::c_ulong,
    0x66dae58801672606 as ::core::ffi::c_long as U64,
    0x833b6017872317fb as ::core::ffi::c_ulong,
    0xb87c16f2d1c92864 as ::core::ffi::c_ulong,
    0xdb766a74e58b669c as ::core::ffi::c_ulong,
    0x89659f85c61417be as ::core::ffi::c_ulong,
    0xc8daad856011ea0c as ::core::ffi::c_ulong,
    0x76a4b565b6fe7eae as ::core::ffi::c_long as U64,
    0xa469d085f6237312 as ::core::ffi::c_ulong,
    0xaaf0365683a3e96c as ::core::ffi::c_ulong,
    0x4dbb746f8424f7b8 as ::core::ffi::c_long as U64,
    0x638755af4e4acc1 as ::core::ffi::c_long as U64,
    0x3d7807f5bde64486 as ::core::ffi::c_long as U64,
    0x17be6d8f5bbb7639 as ::core::ffi::c_long as U64,
    0x903f0cd44dc35dc as ::core::ffi::c_long as U64,
    0x67b672eafdf1196c as ::core::ffi::c_long as U64,
    0xa676ff93ed4c82f1 as ::core::ffi::c_ulong,
    0x521d1004c5053d9d as ::core::ffi::c_long as U64,
    0x37ba9ad09ccc9202 as ::core::ffi::c_long as U64,
    0x84e54d297aacfb51 as ::core::ffi::c_ulong,
    0xa0b4b776a143445 as ::core::ffi::c_long as U64,
    0x820d471e20b348e as ::core::ffi::c_long as U64,
    0x1874383cb83d46dc as ::core::ffi::c_long as U64,
    0x97edeec7a1efe11c as ::core::ffi::c_ulong,
    0xb330e50b1bdc42aa as ::core::ffi::c_ulong,
    0x1dd91955ce70e032 as ::core::ffi::c_long as U64,
    0xa514cdb88f2939d5 as ::core::ffi::c_ulong,
    0x2791233fd90db9d3 as ::core::ffi::c_long as U64,
    0x7b670a4cc50f7a9b as ::core::ffi::c_long as U64,
    0x77c07d2a05c6dfa5 as ::core::ffi::c_long as U64,
    0xe3778b6646d0a6fa as ::core::ffi::c_ulong,
    0xb39c8eda47b56749 as ::core::ffi::c_ulong,
    0x933ed448addbef28 as ::core::ffi::c_ulong,
    0xaf846af6ab7d0bf4 as ::core::ffi::c_ulong,
    0xe5af208eb666e49 as ::core::ffi::c_long as U64,
    0x5e6622f73534cd6a as ::core::ffi::c_long as U64,
    0x297daeca42ef5b6e as ::core::ffi::c_long as U64,
    0x862daef3d35539a6 as ::core::ffi::c_ulong,
    0xe68722498f8e1ea9 as ::core::ffi::c_ulong,
    0x981c53093dc0d572 as ::core::ffi::c_ulong,
    0xfa09b0bfbf86fbf5 as ::core::ffi::c_ulong,
    0x30b1e96166219f15 as ::core::ffi::c_long as U64,
    0x70e7d466bdc4fb83 as ::core::ffi::c_long as U64,
    0x5a66736e35f2a8e9 as ::core::ffi::c_long as U64,
    0xcddb59d2b7c1baef as ::core::ffi::c_ulong,
    0xd6c7d247d26d8996 as ::core::ffi::c_ulong,
    0xea4e39eac8de1ba3 as ::core::ffi::c_ulong,
    0x539c8bb19fa3aff2 as ::core::ffi::c_long as U64,
    0x9f90e4c5fd508d8 as ::core::ffi::c_long as U64,
    0xa34e5956fbaf3385 as ::core::ffi::c_ulong,
    0x2e2f8e151d3ef375 as ::core::ffi::c_long as U64,
    0x173691e9b83faec1 as ::core::ffi::c_long as U64,
    0xb85a8d56bf016379 as ::core::ffi::c_ulong,
    0x8382381267408ae3 as ::core::ffi::c_ulong,
    0xb90f901bbdc0096d as ::core::ffi::c_ulong,
    0x7c6ad32933bcec65 as ::core::ffi::c_long as U64,
    0x76bb5e2f2c8ad595 as ::core::ffi::c_long as U64,
    0x390f851a6cf46d28 as ::core::ffi::c_long as U64,
    0xc3e6064da1c2da72 as ::core::ffi::c_ulong,
    0xc52a0c101cfa5389 as ::core::ffi::c_ulong,
    0xd78eaf84a3fbc530 as ::core::ffi::c_ulong,
    0x3781b9e2288b997e as ::core::ffi::c_long as U64,
    0x73c2f6dea83d05c4 as ::core::ffi::c_long as U64,
    0x4228e364c5b5ed7 as ::core::ffi::c_long as U64,
    0x9d7a3edf0da43911 as ::core::ffi::c_ulong,
    0x8edcfeda24686756 as ::core::ffi::c_ulong,
    0x5e7667a7b7a9b3a1 as ::core::ffi::c_long as U64,
    0x4c4f389fa143791d as ::core::ffi::c_long as U64,
    0xb08bc1023da7cddc as ::core::ffi::c_ulong,
    0x7ab4be3ae529b1cc as ::core::ffi::c_long as U64,
    0x754e6132dbe74ff9 as ::core::ffi::c_long as U64,
    0x71635442a839df45 as ::core::ffi::c_long as U64,
    0x2f6fb1643fbe52de as ::core::ffi::c_long as U64,
    0x961e0a42cf7a8177 as ::core::ffi::c_ulong,
    0xf3b45d83d89ef2ea as ::core::ffi::c_ulong,
    0xee3de4cf4a6e3e9b as ::core::ffi::c_ulong,
    0xcd6848542c3295e7 as ::core::ffi::c_ulong,
    0xe4cee1664c78662f as ::core::ffi::c_ulong,
    0x9947548b474c68c4 as ::core::ffi::c_ulong,
    0x25d73777a5ed8b0b as ::core::ffi::c_long as U64,
    0xc915b1d636b7fc as ::core::ffi::c_long as U64,
    0x21c2ba75d9b0d2da as ::core::ffi::c_long as U64,
    0x5f6b5dcf608a64a1 as ::core::ffi::c_long as U64,
    0xdcf333255ff9570c as ::core::ffi::c_ulong,
    0x633b922418ced4ee as ::core::ffi::c_long as U64,
    0xc136dde0b004b34a as ::core::ffi::c_ulong,
    0x58cc83b05d4b2f5a as ::core::ffi::c_long as U64,
    0x5eb424dda28e42d2 as ::core::ffi::c_long as U64,
    0x62df47369739cd98 as ::core::ffi::c_long as U64,
    0xb4e0b42485e4ce17 as ::core::ffi::c_ulong,
    0x16e1f0c1f9a8d1e7 as ::core::ffi::c_long as U64,
    0x8ec3916707560ebf as ::core::ffi::c_ulong,
    0x62ba6e2df2cc9db3 as ::core::ffi::c_long as U64,
    0xcbf9f4ff77d83a16 as ::core::ffi::c_ulong,
    0x78d9d7d07d2bbcc4 as ::core::ffi::c_long as U64,
    0xef554ce1e02c41f4 as ::core::ffi::c_ulong,
    0x8d7581127eccf94d as ::core::ffi::c_ulong,
    0xa9b53336cb3c8a05 as ::core::ffi::c_ulong,
    0x38c42c0bf45c4f91 as ::core::ffi::c_long as U64,
    0x640893cdf4488863 as ::core::ffi::c_long as U64,
    0x80ec34bc575ea568 as ::core::ffi::c_ulong,
    0x39f324f5b48eaa40 as ::core::ffi::c_long as U64,
    0xe9d9ed1f8eff527f as ::core::ffi::c_ulong,
    0x9224fc058cc5a214 as ::core::ffi::c_ulong,
    0xbaba00b04cfe7741 as ::core::ffi::c_ulong,
    0x309a9f120fcf52af as ::core::ffi::c_long as U64,
    0xa558f3ec65626212 as ::core::ffi::c_ulong,
    0x424bec8b7adabe2f as ::core::ffi::c_long as U64,
    0x41622513a6aea433 as ::core::ffi::c_long as U64,
    0xb88da2d5324ca798 as ::core::ffi::c_ulong,
    0xd287733b245528a4 as ::core::ffi::c_ulong,
    0x9a44697e6d68aec3 as ::core::ffi::c_ulong,
    0x7b1093be2f49bb28 as ::core::ffi::c_long as U64,
    0x50bbec632e3d8aad as ::core::ffi::c_long as U64,
    0x6cd90723e1ea8283 as ::core::ffi::c_long as U64,
    0x897b9e7431b02bf3 as ::core::ffi::c_ulong,
    0x219efdcb338a7047 as ::core::ffi::c_long as U64,
    0x3b0311f0a27c0656 as ::core::ffi::c_long as U64,
    0xdb17bf91c0db96e7 as ::core::ffi::c_ulong,
    0x8cd4fd6b4e85a5b2 as ::core::ffi::c_ulong,
    0xfab071054ba6409d as ::core::ffi::c_ulong,
    0x40d6fe831fa9dfd9 as ::core::ffi::c_long as U64,
    0xaf358debad7d791e as ::core::ffi::c_ulong,
    0xeb8d0e25a65e3e58 as ::core::ffi::c_ulong,
    0xbbcbd3df14e08580 as ::core::ffi::c_ulong,
    0xcf751f27ecdab2b as ::core::ffi::c_long as U64,
    0x2b4da14f2613d8f4 as ::core::ffi::c_long as U64,
];
pub const LDM_MIN_MATCH_LENGTH: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_ldm_gear_init(
    mut state: *mut ldmRollingHashState_t,
    mut params: *const ldmParams_t,
) {
    let mut maxBitsInMask: ::core::ffi::c_uint = if (*params).minMatchLength < 64 as U32 {
        (*params).minMatchLength as ::core::ffi::c_uint
    } else {
        64 as ::core::ffi::c_uint
    };
    let mut hashRateLog: ::core::ffi::c_uint = (*params).hashRateLog as ::core::ffi::c_uint;
    (*state).rolling = !(0 as ::core::ffi::c_int as U32) as U64;
    if hashRateLog > 0 as ::core::ffi::c_uint && hashRateLog <= maxBitsInMask {
        (*state).stopMask = ((1 as ::core::ffi::c_int as U64) << hashRateLog)
            .wrapping_sub(1 as U64)
            << maxBitsInMask.wrapping_sub(hashRateLog);
    } else {
        (*state).stopMask =
            ((1 as ::core::ffi::c_int as U64) << hashRateLog).wrapping_sub(1 as U64);
    };
}
unsafe extern "C" fn ZSTD_ldm_gear_reset(
    mut state: *mut ldmRollingHashState_t,
    mut data: *const BYTE,
    mut minMatchLength: size_t,
) {
    let mut hash: U64 = (*state).rolling;
    let mut n: size_t = 0 as size_t;
    while n.wrapping_add(3 as size_t) < minMatchLength {
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
    }
    while n < minMatchLength {
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
    }
}
unsafe extern "C" fn ZSTD_ldm_gear_feed(
    mut state: *mut ldmRollingHashState_t,
    mut data: *const BYTE,
    mut size: size_t,
    mut splits: *mut size_t,
    mut numSplits: *mut ::core::ffi::c_uint,
) -> size_t {
    let mut current_block: u64;
    let mut n: size_t = 0;
    let mut hash: U64 = 0;
    let mut mask: U64 = 0;
    hash = (*state).rolling;
    mask = (*state).stopMask;
    n = 0 as size_t;
    loop {
        if !(n.wrapping_add(3 as size_t) < size) {
            current_block = 5689316957504528238;
            break;
        }
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        if (hash & mask == 0 as U64) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
            *splits.offset(*numSplits as isize) = n;
            *numSplits = (*numSplits).wrapping_add(1 as ::core::ffi::c_uint);
            if *numSplits == LDM_BATCH_SIZE as ::core::ffi::c_uint {
                current_block = 8540360772094939182;
                break;
            }
        }
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        if (hash & mask == 0 as U64) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
            *splits.offset(*numSplits as isize) = n;
            *numSplits = (*numSplits).wrapping_add(1 as ::core::ffi::c_uint);
            if *numSplits == LDM_BATCH_SIZE as ::core::ffi::c_uint {
                current_block = 8540360772094939182;
                break;
            }
        }
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        if (hash & mask == 0 as U64) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
            *splits.offset(*numSplits as isize) = n;
            *numSplits = (*numSplits).wrapping_add(1 as ::core::ffi::c_uint);
            if *numSplits == LDM_BATCH_SIZE as ::core::ffi::c_uint {
                current_block = 8540360772094939182;
                break;
            }
        }
        hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
            ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as usize],
        );
        n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t as size_t;
        if !((hash & mask == 0 as U64) as ::core::ffi::c_int as ::core::ffi::c_long != 0) {
            continue;
        }
        *splits.offset(*numSplits as isize) = n;
        *numSplits = (*numSplits).wrapping_add(1 as ::core::ffi::c_uint);
        if *numSplits == LDM_BATCH_SIZE as ::core::ffi::c_uint {
            current_block = 8540360772094939182;
            break;
        }
    }
    loop {
        match current_block {
            8540360772094939182 => {
                (*state).rolling = hash;
                break;
            }
            _ => {
                if !(n < size) {
                    current_block = 8540360772094939182;
                    continue;
                }
                hash = (hash << 1 as ::core::ffi::c_int).wrapping_add(
                    ZSTD_ldm_gearTab[(*data.offset(n as isize) as ::core::ffi::c_int
                        & 0xff as ::core::ffi::c_int)
                        as usize],
                );
                n = (n as ::core::ffi::c_ulong).wrapping_add(1 as ::core::ffi::c_ulong) as size_t
                    as size_t;
                if !((hash & mask == 0 as U64) as ::core::ffi::c_int as ::core::ffi::c_long != 0) {
                    current_block = 5689316957504528238;
                    continue;
                }
                *splits.offset(*numSplits as isize) = n;
                *numSplits = (*numSplits).wrapping_add(1 as ::core::ffi::c_uint);
                if *numSplits == LDM_BATCH_SIZE as ::core::ffi::c_uint {
                    current_block = 8540360772094939182;
                } else {
                    current_block = 5689316957504528238;
                }
            }
        }
    }
    return n;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_adjustParameters(
    mut params: *mut ldmParams_t,
    mut cParams: *const ZSTD_compressionParameters,
) {
    (*params).windowLog = (*cParams).windowLog as U32;
    if (*params).hashRateLog == 0 as U32 {
        if (*params).hashLog > 0 as U32 {
            if (*params).windowLog > (*params).hashLog {
                (*params).hashRateLog = (*params).windowLog.wrapping_sub((*params).hashLog);
            }
        } else {
            (*params).hashRateLog = (7 as ::core::ffi::c_uint).wrapping_sub(
                ((*cParams).strategy as ::core::ffi::c_uint).wrapping_div(3 as ::core::ffi::c_uint),
            ) as U32;
        }
    }
    if (*params).hashLog == 0 as U32 {
        (*params).hashLog = if 6 as U32
            > (if (*params).windowLog.wrapping_sub((*params).hashRateLog)
                < (if (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                    30 as ::core::ffi::c_int
                } else {
                    31 as ::core::ffi::c_int
                }) < 30 as ::core::ffi::c_int
                {
                    (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                        30 as ::core::ffi::c_int
                    } else {
                        31 as ::core::ffi::c_int
                    })
                } else {
                    30 as ::core::ffi::c_int
                }) as U32
            {
                (*params).windowLog.wrapping_sub((*params).hashRateLog)
            } else {
                (if (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                    30 as ::core::ffi::c_int
                } else {
                    31 as ::core::ffi::c_int
                }) < 30 as ::core::ffi::c_int
                {
                    (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                        30 as ::core::ffi::c_int
                    } else {
                        31 as ::core::ffi::c_int
                    })
                } else {
                    30 as ::core::ffi::c_int
                }) as U32
            }) {
            6 as U32
        } else if (*params).windowLog.wrapping_sub((*params).hashRateLog)
            < (if (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                30 as ::core::ffi::c_int
            } else {
                31 as ::core::ffi::c_int
            }) < 30 as ::core::ffi::c_int
            {
                (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                    30 as ::core::ffi::c_int
                } else {
                    31 as ::core::ffi::c_int
                })
            } else {
                30 as ::core::ffi::c_int
            }) as U32
        {
            (*params).windowLog.wrapping_sub((*params).hashRateLog)
        } else {
            (if (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                30 as ::core::ffi::c_int
            } else {
                31 as ::core::ffi::c_int
            }) < 30 as ::core::ffi::c_int
            {
                if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                    30 as ::core::ffi::c_int
                } else {
                    31 as ::core::ffi::c_int
                }
            } else {
                30 as ::core::ffi::c_int
            }) as U32
        };
    }
    if (*params).minMatchLength == 0 as U32 {
        (*params).minMatchLength = LDM_MIN_MATCH_LENGTH as U32;
        if (*cParams).strategy as ::core::ffi::c_uint
            >= ZSTD_btultra as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            (*params).minMatchLength = ((*params).minMatchLength as ::core::ffi::c_uint)
                .wrapping_div(2 as ::core::ffi::c_uint)
                as U32 as U32;
        }
    }
    if (*params).bucketSizeLog == 0 as U32 {
        (*params).bucketSizeLog = if 4 as U32
            > (if ((*cParams).strategy as U32) < 8 as U32 {
                (*cParams).strategy as U32
            } else {
                8 as U32
            }) {
            4 as U32
        } else if ((*cParams).strategy as U32) < 8 as U32 {
            (*cParams).strategy as U32
        } else {
            8 as U32
        };
    }
    (*params).bucketSizeLog = if (*params).bucketSizeLog < (*params).hashLog {
        (*params).bucketSizeLog
    } else {
        (*params).hashLog
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getTableSize(mut params: ldmParams_t) -> size_t {
    let ldmHSize: size_t = (1 as ::core::ffi::c_int as size_t) << params.hashLog;
    let ldmBucketSizeLog: size_t = (if params.bucketSizeLog < params.hashLog {
        params.bucketSizeLog
    } else {
        params.hashLog
    }) as size_t;
    let ldmBucketSize: size_t = (1 as ::core::ffi::c_int as size_t)
        << (params.hashLog as size_t).wrapping_sub(ldmBucketSizeLog);
    let totalSize: size_t = (ZSTD_cwksp_alloc_size(ldmBucketSize) as size_t)
        .wrapping_add(ZSTD_cwksp_alloc_size(
            ldmHSize.wrapping_mul(::core::mem::size_of::<ldmEntry_t>() as size_t),
        ) as size_t);
    return if params.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        totalSize
    } else {
        0 as size_t
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getMaxNbSeq(
    mut params: ldmParams_t,
    mut maxChunkSize: size_t,
) -> size_t {
    return if params.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        maxChunkSize.wrapping_div(params.minMatchLength as size_t)
    } else {
        0 as size_t
    };
}
unsafe extern "C" fn ZSTD_ldm_getBucket(
    mut ldmState: *const ldmState_t,
    mut hash: size_t,
    bucketSizeLog: U32,
) -> *mut ldmEntry_t {
    return (*ldmState)
        .hashTable
        .offset((hash << bucketSizeLog) as isize);
}
unsafe extern "C" fn ZSTD_ldm_insertEntry(
    mut ldmState: *mut ldmState_t,
    hash: size_t,
    entry: ldmEntry_t,
    bucketSizeLog: U32,
) {
    let pOffset: *mut BYTE = (*ldmState).bucketOffsets.offset(hash as isize);
    let offset: ::core::ffi::c_uint = *pOffset as ::core::ffi::c_uint;
    *ZSTD_ldm_getBucket(ldmState, hash, bucketSizeLog).offset(offset as isize) = entry;
    *pOffset = (offset.wrapping_add(1 as ::core::ffi::c_uint)
        & ((1 as ::core::ffi::c_uint) << bucketSizeLog).wrapping_sub(1 as ::core::ffi::c_uint))
        as BYTE;
}
unsafe extern "C" fn ZSTD_ldm_countBackwardsMatch(
    mut pIn: *const BYTE,
    mut pAnchor: *const BYTE,
    mut pMatch: *const BYTE,
    mut pMatchBase: *const BYTE,
) -> size_t {
    let mut matchLength: size_t = 0 as size_t;
    while pIn > pAnchor
        && pMatch > pMatchBase
        && *pIn.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
            == *pMatch.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
    {
        pIn = pIn.offset(-1);
        pMatch = pMatch.offset(-1);
        matchLength = matchLength.wrapping_add(1);
    }
    return matchLength;
}
unsafe extern "C" fn ZSTD_ldm_countBackwardsMatch_2segments(
    mut pIn: *const BYTE,
    mut pAnchor: *const BYTE,
    mut pMatch: *const BYTE,
    mut pMatchBase: *const BYTE,
    mut pExtDictStart: *const BYTE,
    mut pExtDictEnd: *const BYTE,
) -> size_t {
    let mut matchLength: size_t = ZSTD_ldm_countBackwardsMatch(pIn, pAnchor, pMatch, pMatchBase);
    if pMatch.offset(-(matchLength as isize)) != pMatchBase || pMatchBase == pExtDictStart {
        return matchLength;
    }
    matchLength = (matchLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_ldm_countBackwardsMatch(
        pIn.offset(-(matchLength as isize)),
        pAnchor,
        pExtDictEnd,
        pExtDictStart,
    ) as ::core::ffi::c_ulong) as size_t as size_t;
    return matchLength;
}
unsafe extern "C" fn ZSTD_ldm_fillFastTables(
    mut ms: *mut ZSTD_MatchState_t,
    mut end: *const ::core::ffi::c_void,
) -> size_t {
    let iend: *const BYTE = end as *const BYTE;
    match (*ms).cParams.strategy as ::core::ffi::c_uint {
        1 => {
            ZSTD_fillHashTable(
                ms,
                iend as *const ::core::ffi::c_void,
                ZSTD_dtlm_fast,
                ZSTD_tfp_forCCtx,
            );
        }
        2 => {
            ZSTD_fillDoubleHashTable(
                ms,
                iend as *const ::core::ffi::c_void,
                ZSTD_dtlm_fast,
                ZSTD_tfp_forCCtx,
            );
        }
        3 | 4 | 5 | 6 | 7 | 8 | 9 | _ => {}
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_fillHashTable(
    mut ldmState: *mut ldmState_t,
    mut ip: *const BYTE,
    mut iend: *const BYTE,
    mut params: *const ldmParams_t,
) {
    let minMatchLength: U32 = (*params).minMatchLength;
    let bucketSizeLog: U32 = (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog.wrapping_sub(bucketSizeLog);
    let base: *const BYTE = (*ldmState).window.base;
    let istart: *const BYTE = ip;
    let mut hashState: ldmRollingHashState_t = ldmRollingHashState_t {
        rolling: 0,
        stopMask: 0,
    };
    let splits: *mut size_t = &raw mut (*ldmState).splitIndices as *mut size_t;
    let mut numSplits: ::core::ffi::c_uint = 0;
    ZSTD_ldm_gear_init(&raw mut hashState, params);
    while ip < iend {
        let mut hashed: size_t = 0;
        let mut n: ::core::ffi::c_uint = 0;
        numSplits = 0 as ::core::ffi::c_uint;
        hashed = ZSTD_ldm_gear_feed(
            &raw mut hashState,
            ip,
            iend.offset_from(ip) as ::core::ffi::c_long as size_t,
            splits,
            &raw mut numSplits,
        );
        n = 0 as ::core::ffi::c_uint;
        while n < numSplits {
            if ip.offset(*splits.offset(n as isize) as isize)
                >= istart.offset(minMatchLength as isize)
            {
                let split: *const BYTE = ip
                    .offset(*splits.offset(n as isize) as isize)
                    .offset(-(minMatchLength as isize));
                let xxhash: U64 = ZSTD_XXH64(
                    split as *const ::core::ffi::c_void,
                    minMatchLength as size_t,
                    0 as XXH64_hash_t,
                ) as U64;
                let hash: U32 = (xxhash
                    & ((1 as ::core::ffi::c_int as U32) << hBits).wrapping_sub(1 as U32) as U64)
                    as U32;
                let mut entry: ldmEntry_t = ldmEntry_t {
                    offset: 0,
                    checksum: 0,
                };
                entry.offset = split.offset_from(base) as ::core::ffi::c_long as U32;
                entry.checksum = (xxhash >> 32 as ::core::ffi::c_int) as U32;
                ZSTD_ldm_insertEntry(ldmState, hash as size_t, entry, (*params).bucketSizeLog);
            }
            n = n.wrapping_add(1);
        }
        ip = ip.offset(hashed as isize);
    }
}
unsafe extern "C" fn ZSTD_ldm_limitTableUpdate(
    mut ms: *mut ZSTD_MatchState_t,
    mut anchor: *const BYTE,
) {
    let curr: U32 = anchor.offset_from((*ms).window.base) as ::core::ffi::c_long as U32;
    if curr > (*ms).nextToUpdate.wrapping_add(1024 as U32) {
        (*ms).nextToUpdate = curr.wrapping_sub(
            (if (512 as U32)
                < curr
                    .wrapping_sub((*ms).nextToUpdate)
                    .wrapping_sub(1024 as U32)
            {
                512 as U32
            } else {
                curr.wrapping_sub((*ms).nextToUpdate)
                    .wrapping_sub(1024 as U32)
            }),
        );
    }
}
unsafe extern "C" fn ZSTD_ldm_generateSequences_internal(
    mut ldmState: *mut ldmState_t,
    mut rawSeqStore: *mut RawSeqStore_t,
    mut params: *const ldmParams_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let extDict: ::core::ffi::c_int =
        ZSTD_window_hasExtDict((*ldmState).window) as ::core::ffi::c_int;
    let minMatchLength: U32 = (*params).minMatchLength;
    let entsPerBucket: U32 = (1 as U32) << (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog.wrapping_sub((*params).bucketSizeLog);
    let dictLimit: U32 = (*ldmState).window.dictLimit;
    let lowestIndex: U32 = if extDict != 0 {
        (*ldmState).window.lowLimit
    } else {
        dictLimit
    };
    let base: *const BYTE = (*ldmState).window.base;
    let dictBase: *const BYTE = if extDict != 0 {
        (*ldmState).window.dictBase
    } else {
        ::core::ptr::null::<BYTE>()
    };
    let dictStart: *const BYTE = if extDict != 0 {
        dictBase.offset(lowestIndex as isize)
    } else {
        ::core::ptr::null::<BYTE>()
    };
    let dictEnd: *const BYTE = if extDict != 0 {
        dictBase.offset(dictLimit as isize)
    } else {
        ::core::ptr::null::<BYTE>()
    };
    let lowPrefixPtr: *const BYTE = base.offset(dictLimit as isize);
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(HASH_READ_SIZE as isize));
    let mut anchor: *const BYTE = istart;
    let mut ip: *const BYTE = istart;
    let mut hashState: ldmRollingHashState_t = ldmRollingHashState_t {
        rolling: 0,
        stopMask: 0,
    };
    let splits: *mut size_t = &raw mut (*ldmState).splitIndices as *mut size_t;
    let candidates: *mut ldmMatchCandidate_t =
        &raw mut (*ldmState).matchCandidates as *mut ldmMatchCandidate_t;
    let mut numSplits: ::core::ffi::c_uint = 0;
    if srcSize < minMatchLength as size_t {
        return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
    }
    ZSTD_ldm_gear_init(&raw mut hashState, params);
    ZSTD_ldm_gear_reset(&raw mut hashState, ip, minMatchLength as size_t);
    ip = ip.offset(minMatchLength as isize);
    while ip < ilimit {
        let mut hashed: size_t = 0;
        let mut n: ::core::ffi::c_uint = 0;
        numSplits = 0 as ::core::ffi::c_uint;
        hashed = ZSTD_ldm_gear_feed(
            &raw mut hashState,
            ip,
            ilimit.offset_from(ip) as ::core::ffi::c_long as size_t,
            splits,
            &raw mut numSplits,
        );
        n = 0 as ::core::ffi::c_uint;
        while n < numSplits {
            let split: *const BYTE = ip
                .offset(*splits.offset(n as isize) as isize)
                .offset(-(minMatchLength as isize));
            let xxhash: U64 = ZSTD_XXH64(
                split as *const ::core::ffi::c_void,
                minMatchLength as size_t,
                0 as XXH64_hash_t,
            ) as U64;
            let hash: U32 = (xxhash
                & ((1 as ::core::ffi::c_int as U32) << hBits).wrapping_sub(1 as U32) as U64)
                as U32;
            let ref mut fresh3 = (*candidates.offset(n as isize)).split;
            *fresh3 = split;
            (*candidates.offset(n as isize)).hash = hash;
            (*candidates.offset(n as isize)).checksum = (xxhash >> 32 as ::core::ffi::c_int) as U32;
            let ref mut fresh4 = (*candidates.offset(n as isize)).bucket;
            *fresh4 = ZSTD_ldm_getBucket(ldmState, hash as size_t, (*params).bucketSizeLog);
            n = n.wrapping_add(1);
        }
        n = 0 as ::core::ffi::c_uint;
        while n < numSplits {
            let mut forwardMatchLength: size_t = 0 as size_t;
            let mut backwardMatchLength: size_t = 0 as size_t;
            let mut bestMatchLength: size_t = 0 as size_t;
            let mut mLength: size_t = 0;
            let mut offset: U32 = 0;
            let split_0: *const BYTE = (*candidates.offset(n as isize)).split;
            let checksum: U32 = (*candidates.offset(n as isize)).checksum;
            let hash_0: U32 = (*candidates.offset(n as isize)).hash;
            let bucket: *mut ldmEntry_t = (*candidates.offset(n as isize)).bucket;
            let mut cur: *const ldmEntry_t = ::core::ptr::null::<ldmEntry_t>();
            let mut bestEntry: *const ldmEntry_t = ::core::ptr::null::<ldmEntry_t>();
            let mut newEntry: ldmEntry_t = ldmEntry_t {
                offset: 0,
                checksum: 0,
            };
            newEntry.offset = split_0.offset_from(base) as ::core::ffi::c_long as U32;
            newEntry.checksum = checksum;
            if split_0 < anchor {
                ZSTD_ldm_insertEntry(
                    ldmState,
                    hash_0 as size_t,
                    newEntry,
                    (*params).bucketSizeLog,
                );
            } else {
                let mut current_block_30: u64;
                cur = bucket;
                while cur < bucket.offset(entsPerBucket as isize) as *const ldmEntry_t {
                    let mut curForwardMatchLength: size_t = 0;
                    let mut curBackwardMatchLength: size_t = 0;
                    let mut curTotalMatchLength: size_t = 0;
                    if !((*cur).checksum != checksum || (*cur).offset <= lowestIndex) {
                        if extDict != 0 {
                            let curMatchBase: *const BYTE = if (*cur).offset < dictLimit {
                                dictBase
                            } else {
                                base
                            };
                            let pMatch: *const BYTE = curMatchBase.offset((*cur).offset as isize);
                            let matchEnd: *const BYTE = if (*cur).offset < dictLimit {
                                dictEnd
                            } else {
                                iend
                            };
                            let lowMatchPtr: *const BYTE = if (*cur).offset < dictLimit {
                                dictStart
                            } else {
                                lowPrefixPtr
                            };
                            curForwardMatchLength =
                                ZSTD_count_2segments(split_0, pMatch, iend, matchEnd, lowPrefixPtr);
                            if curForwardMatchLength < minMatchLength as size_t {
                                current_block_30 = 17788412896529399552;
                            } else {
                                curBackwardMatchLength = ZSTD_ldm_countBackwardsMatch_2segments(
                                    split_0,
                                    anchor,
                                    pMatch,
                                    lowMatchPtr,
                                    dictStart,
                                    dictEnd,
                                );
                                current_block_30 = 15512526488502093901;
                            }
                        } else {
                            let pMatch_0: *const BYTE = base.offset((*cur).offset as isize);
                            curForwardMatchLength = ZSTD_count(split_0, pMatch_0, iend);
                            if curForwardMatchLength < minMatchLength as size_t {
                                current_block_30 = 17788412896529399552;
                            } else {
                                curBackwardMatchLength = ZSTD_ldm_countBackwardsMatch(
                                    split_0,
                                    anchor,
                                    pMatch_0,
                                    lowPrefixPtr,
                                );
                                current_block_30 = 15512526488502093901;
                            }
                        }
                        match current_block_30 {
                            17788412896529399552 => {}
                            _ => {
                                curTotalMatchLength =
                                    curForwardMatchLength.wrapping_add(curBackwardMatchLength);
                                if curTotalMatchLength > bestMatchLength {
                                    bestMatchLength = curTotalMatchLength;
                                    forwardMatchLength = curForwardMatchLength;
                                    backwardMatchLength = curBackwardMatchLength;
                                    bestEntry = cur;
                                }
                            }
                        }
                    }
                    cur = cur.offset(1);
                }
                if bestEntry.is_null() {
                    ZSTD_ldm_insertEntry(
                        ldmState,
                        hash_0 as size_t,
                        newEntry,
                        (*params).bucketSizeLog,
                    );
                } else {
                    offset = (split_0.offset_from(base) as ::core::ffi::c_long as U32)
                        .wrapping_sub((*bestEntry).offset);
                    mLength = forwardMatchLength.wrapping_add(backwardMatchLength);
                    let seq: *mut rawSeq = (*rawSeqStore).seq.offset((*rawSeqStore).size as isize);
                    if (*rawSeqStore).size == (*rawSeqStore).capacity {
                        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
                    }
                    (*seq).litLength = split_0
                        .offset(-(backwardMatchLength as isize))
                        .offset_from(anchor)
                        as ::core::ffi::c_long as U32;
                    (*seq).matchLength = mLength as U32;
                    (*seq).offset = offset;
                    (*rawSeqStore).size = (*rawSeqStore).size.wrapping_add(1);
                    ZSTD_ldm_insertEntry(
                        ldmState,
                        hash_0 as size_t,
                        newEntry,
                        (*params).bucketSizeLog,
                    );
                    anchor = split_0.offset(forwardMatchLength as isize);
                    if anchor > ip.offset(hashed as isize) {
                        ZSTD_ldm_gear_reset(
                            &raw mut hashState,
                            anchor.offset(-(minMatchLength as isize)),
                            minMatchLength as size_t,
                        );
                        ip = anchor.offset(-(hashed as isize));
                        break;
                    }
                }
            }
            n = n.wrapping_add(1);
        }
        ip = ip.offset(hashed as isize);
    }
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_ldm_reduceTable(table: *mut ldmEntry_t, size: U32, reducerValue: U32) {
    let mut u: U32 = 0;
    u = 0 as U32;
    while u < size {
        if (*table.offset(u as isize)).offset < reducerValue {
            (*table.offset(u as isize)).offset = 0 as U32;
        } else {
            let ref mut fresh5 = (*table.offset(u as isize)).offset;
            *fresh5 = (*fresh5 as ::core::ffi::c_uint)
                .wrapping_sub(reducerValue as ::core::ffi::c_uint) as U32
                as U32;
        }
        u = u.wrapping_add(1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_generateSequences(
    mut ldmState: *mut ldmState_t,
    mut sequences: *mut RawSeqStore_t,
    mut params: *const ldmParams_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let maxDist: U32 = (1 as U32) << (*params).windowLog;
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let kMaxChunkSize: size_t = ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as size_t;
    let nbChunks: size_t = srcSize.wrapping_div(kMaxChunkSize).wrapping_add(
        (srcSize.wrapping_rem(kMaxChunkSize) != 0 as size_t) as ::core::ffi::c_int as size_t,
    );
    let mut chunk: size_t = 0;
    let mut leftoverSize: size_t = 0 as size_t;
    chunk = 0 as size_t;
    while chunk < nbChunks && (*sequences).size < (*sequences).capacity {
        let chunkStart: *const BYTE = istart.offset(chunk.wrapping_mul(kMaxChunkSize) as isize);
        let remaining: size_t = iend.offset_from(chunkStart) as ::core::ffi::c_long as size_t;
        let chunkEnd: *const BYTE = if remaining < kMaxChunkSize {
            iend
        } else {
            chunkStart.offset(kMaxChunkSize as isize)
        };
        let chunkSize: size_t = chunkEnd.offset_from(chunkStart) as ::core::ffi::c_long as size_t;
        let mut newLeftoverSize: size_t = 0;
        let prevSize: size_t = (*sequences).size;
        if ZSTD_window_needOverflowCorrection(
            (*ldmState).window,
            0 as U32,
            maxDist,
            (*ldmState).loadedDictEnd,
            chunkStart as *const ::core::ffi::c_void,
            chunkEnd as *const ::core::ffi::c_void,
        ) != 0
        {
            let ldmHSize: U32 = (1 as U32) << (*params).hashLog;
            let correction: U32 = ZSTD_window_correctOverflow(
                &raw mut (*ldmState).window,
                0 as U32,
                maxDist,
                chunkStart as *const ::core::ffi::c_void,
            ) as U32;
            ZSTD_ldm_reduceTable((*ldmState).hashTable, ldmHSize, correction);
            (*ldmState).loadedDictEnd = 0 as U32;
        }
        ZSTD_window_enforceMaxDist(
            &raw mut (*ldmState).window,
            chunkEnd as *const ::core::ffi::c_void,
            maxDist,
            &raw mut (*ldmState).loadedDictEnd,
            ::core::ptr::null_mut::<*const ZSTD_MatchState_t>(),
        );
        newLeftoverSize = ZSTD_ldm_generateSequences_internal(
            ldmState,
            sequences,
            params,
            chunkStart as *const ::core::ffi::c_void,
            chunkSize,
        );
        if ERR_isError(newLeftoverSize) != 0 {
            return newLeftoverSize;
        }
        if prevSize < (*sequences).size {
            let ref mut fresh2 = (*(*sequences).seq.offset(prevSize as isize)).litLength;
            *fresh2 = (*fresh2 as ::core::ffi::c_uint)
                .wrapping_add(leftoverSize as U32 as ::core::ffi::c_uint)
                as U32 as U32;
            leftoverSize = newLeftoverSize;
        } else {
            leftoverSize = (leftoverSize as ::core::ffi::c_ulong)
                .wrapping_add(chunkSize as ::core::ffi::c_ulong)
                as size_t as size_t;
        }
        chunk = chunk.wrapping_add(1);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipSequences(
    mut rawSeqStore: *mut RawSeqStore_t,
    mut srcSize: size_t,
    minMatch: U32,
) {
    while srcSize > 0 as size_t && (*rawSeqStore).pos < (*rawSeqStore).size {
        let mut seq: *mut rawSeq = (*rawSeqStore).seq.offset((*rawSeqStore).pos as isize);
        if srcSize <= (*seq).litLength as size_t {
            (*seq).litLength = ((*seq).litLength as ::core::ffi::c_uint)
                .wrapping_sub(srcSize as U32 as ::core::ffi::c_uint)
                as U32 as U32;
            return;
        }
        srcSize = (srcSize as ::core::ffi::c_ulong)
            .wrapping_sub((*seq).litLength as ::core::ffi::c_ulong) as size_t
            as size_t;
        (*seq).litLength = 0 as U32;
        if srcSize < (*seq).matchLength as size_t {
            (*seq).matchLength = ((*seq).matchLength as ::core::ffi::c_uint)
                .wrapping_sub(srcSize as U32 as ::core::ffi::c_uint)
                as U32 as U32;
            if (*seq).matchLength < minMatch {
                if (*rawSeqStore).pos.wrapping_add(1 as size_t) < (*rawSeqStore).size {
                    let ref mut fresh6 = (*seq.offset(1 as ::core::ffi::c_int as isize)).litLength;
                    *fresh6 = (*fresh6 as ::core::ffi::c_uint).wrapping_add(
                        (*seq.offset(0 as ::core::ffi::c_int as isize)).matchLength
                            as ::core::ffi::c_uint,
                    ) as U32 as U32;
                }
                (*rawSeqStore).pos = (*rawSeqStore).pos.wrapping_add(1);
            }
            return;
        }
        srcSize = (srcSize as ::core::ffi::c_ulong)
            .wrapping_sub((*seq).matchLength as ::core::ffi::c_ulong) as size_t
            as size_t;
        (*seq).matchLength = 0 as U32;
        (*rawSeqStore).pos = (*rawSeqStore).pos.wrapping_add(1);
    }
}
unsafe extern "C" fn maybeSplitSequence(
    mut rawSeqStore: *mut RawSeqStore_t,
    remaining: U32,
    minMatch: U32,
) -> rawSeq {
    let mut sequence: rawSeq = *(*rawSeqStore).seq.offset((*rawSeqStore).pos as isize);
    if remaining >= sequence.litLength.wrapping_add(sequence.matchLength) {
        (*rawSeqStore).pos = (*rawSeqStore).pos.wrapping_add(1);
        return sequence;
    }
    if remaining <= sequence.litLength {
        sequence.offset = 0 as U32;
    } else if remaining < sequence.litLength.wrapping_add(sequence.matchLength) {
        sequence.matchLength = remaining.wrapping_sub(sequence.litLength);
        if sequence.matchLength < minMatch {
            sequence.offset = 0 as U32;
        }
    }
    ZSTD_ldm_skipSequences(rawSeqStore, remaining as size_t, minMatch);
    return sequence;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipRawSeqStoreBytes(
    mut rawSeqStore: *mut RawSeqStore_t,
    mut nbBytes: size_t,
) {
    let mut currPos: U32 = (*rawSeqStore).posInSequence.wrapping_add(nbBytes) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let mut currSeq: rawSeq = *(*rawSeqStore).seq.offset((*rawSeqStore).pos as isize);
        if currPos >= currSeq.litLength.wrapping_add(currSeq.matchLength) {
            currPos = (currPos as ::core::ffi::c_uint).wrapping_sub(
                currSeq.litLength.wrapping_add(currSeq.matchLength) as ::core::ffi::c_uint,
            ) as U32 as U32;
            (*rawSeqStore).pos = (*rawSeqStore).pos.wrapping_add(1);
        } else {
            (*rawSeqStore).posInSequence = currPos as size_t;
            break;
        }
    }
    if currPos == 0 as U32 || (*rawSeqStore).pos == (*rawSeqStore).size {
        (*rawSeqStore).posInSequence = 0 as size_t;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_blockCompress(
    mut rawSeqStore: *mut RawSeqStore_t,
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut useRowMatchFinder: ZSTD_ParamSwitch_e,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let minMatch: ::core::ffi::c_uint = (*cParams).minMatch;
    let blockCompressor: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
        (*cParams).strategy,
        useRowMatchFinder,
        ZSTD_matchState_dictMode(ms),
    ) as ZSTD_BlockCompressor_f;
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let mut ip: *const BYTE = istart;
    if (*cParams).strategy as ::core::ffi::c_uint
        >= ZSTD_btopt as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut lastLLSize: size_t = 0;
        (*ms).ldmSeqStore = rawSeqStore;
        lastLLSize =
            blockCompressor.expect("non-null function pointer")(ms, seqStore, rep, src, srcSize);
        ZSTD_ldm_skipRawSeqStoreBytes(rawSeqStore, srcSize);
        return lastLLSize;
    }
    while (*rawSeqStore).pos < (*rawSeqStore).size && ip < iend {
        let sequence: rawSeq = maybeSplitSequence(
            rawSeqStore,
            iend.offset_from(ip) as ::core::ffi::c_long as U32,
            minMatch as U32,
        ) as rawSeq;
        if sequence.offset == 0 as U32 {
            break;
        }
        ZSTD_ldm_limitTableUpdate(ms, ip);
        ZSTD_ldm_fillFastTables(ms, ip as *const ::core::ffi::c_void);
        let mut i: ::core::ffi::c_int = 0;
        let newLitLength: size_t = blockCompressor.expect("non-null function pointer")(
            ms,
            seqStore,
            rep,
            ip as *const ::core::ffi::c_void,
            sequence.litLength as size_t,
        ) as size_t;
        ip = ip.offset(sequence.litLength as isize);
        i = ZSTD_REP_NUM - 1 as ::core::ffi::c_int;
        while i > 0 as ::core::ffi::c_int {
            *rep.offset(i as isize) = *rep.offset((i - 1 as ::core::ffi::c_int) as isize);
            i -= 1;
        }
        *rep.offset(0 as ::core::ffi::c_int as isize) = sequence.offset;
        ZSTD_storeSeq(
            seqStore,
            newLitLength,
            ip.offset(-(newLitLength as isize)),
            iend,
            sequence.offset.wrapping_add(ZSTD_REP_NUM as U32),
            sequence.matchLength as size_t,
        );
        ip = ip.offset(sequence.matchLength as isize);
    }
    ZSTD_ldm_limitTableUpdate(ms, ip);
    ZSTD_ldm_fillFastTables(ms, ip as *const ::core::ffi::c_void);
    return blockCompressor.expect("non-null function pointer")(
        ms,
        seqStore,
        rep,
        ip as *const ::core::ffi::c_void,
        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
    );
}
