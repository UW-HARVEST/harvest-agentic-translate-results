use ::core::arch::asm;
#[cfg(target_arch = "x86")]
pub use ::core::arch::x86::{
    __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8, _mm_set_epi8,
    _mm_setzero_si128, _mm_storeu_si128,
};
#[cfg(target_arch = "x86_64")]
pub use ::core::arch::x86_64::{
    __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8, _mm_set_epi8,
    _mm_setzero_si128, _mm_storeu_si128,
};
pub type __m128i_u = __m128i;
use ::libc;
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
pub type unalign64 = U64;
pub type unalignArch = size_t;
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
pub type ZSTD_overlap_e = ::core::ffi::c_uint;
pub const ZSTD_overlap_src_before_dst: ZSTD_overlap_e = 1;
pub const ZSTD_no_overlap: ZSTD_overlap_e = 0;
pub type ZSTD_dictMode_e = ::core::ffi::c_uint;
pub const ZSTD_dedicatedDictSearch: ZSTD_dictMode_e = 3;
pub const ZSTD_dictMatchState: ZSTD_dictMode_e = 2;
pub const ZSTD_extDict: ZSTD_dictMode_e = 1;
pub const ZSTD_noDict: ZSTD_dictMode_e = 0;
pub type searchMethod_e = ::core::ffi::c_uint;
pub const search_rowHash: searchMethod_e = 2;
pub const search_binaryTree: searchMethod_e = 1;
pub const search_hashChain: searchMethod_e = 0;
pub type ZSTD_VecMask = U64;
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
unsafe extern "C" fn MEM_read64(mut ptr: *const ::core::ffi::c_void) -> U64 {
    return *(ptr as *const unalign64);
}
#[inline]
unsafe extern "C" fn MEM_readST(mut ptr: *const ::core::ffi::c_void) -> size_t {
    return *(ptr as *const unalignArch);
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
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
#[inline]
unsafe extern "C" fn ZSTD_rotateRight_U64(value: U64, mut count: U32) -> U64 {
    count = (count as ::core::ffi::c_uint & 0x3f as ::core::ffi::c_uint) as U32;
    return value >> count | value << ((0 as U32).wrapping_sub(count) & 0x3f as U32);
}
#[inline]
unsafe extern "C" fn ZSTD_rotateRight_U32(value: U32, mut count: U32) -> U32 {
    count = (count as ::core::ffi::c_uint & 0x1f as ::core::ffi::c_uint) as U32;
    return value >> count | value << ((0 as U32).wrapping_sub(count) & 0x1f as U32);
}
#[inline]
unsafe extern "C" fn ZSTD_rotateRight_U16(value: U16, mut count: U32) -> U16 {
    count = (count as ::core::ffi::c_uint & 0xf as ::core::ffi::c_uint) as U32;
    return (value as ::core::ffi::c_int >> count
        | ((value as ::core::ffi::c_int) << ((0 as U32).wrapping_sub(count) & 0xf as U32)) as U16
            as ::core::ffi::c_int) as U16;
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
pub const kSearchStrength: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const ZSTD_DUBT_UNSORTED_MARK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ZSTD_ROW_HASH_CACHE_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
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
pub const REPCODE1_TO_OFFBASE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
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
static mut prime4bytes: U32 = 2654435761 as U32;
unsafe extern "C" fn ZSTD_hash4(mut u: U32, mut h: U32, mut s: U32) -> U32 {
    return (u.wrapping_mul(prime4bytes) ^ s) >> (32 as U32).wrapping_sub(h);
}
unsafe extern "C" fn ZSTD_hash4Ptr(mut ptr: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash4(MEM_readLE32(ptr), h, 0 as U32) as size_t;
}
unsafe extern "C" fn ZSTD_hash4PtrS(
    mut ptr: *const ::core::ffi::c_void,
    mut h: U32,
    mut s: U32,
) -> size_t {
    return ZSTD_hash4(MEM_readLE32(ptr), h, s) as size_t;
}
static mut prime5bytes: U64 = 889523592379 as U64;
unsafe extern "C" fn ZSTD_hash5(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return (((u << 64 as ::core::ffi::c_int - 40 as ::core::ffi::c_int).wrapping_mul(prime5bytes)
        ^ s)
        >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash5Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash5(MEM_readLE64(p), h, 0 as U64);
}
unsafe extern "C" fn ZSTD_hash5PtrS(
    mut p: *const ::core::ffi::c_void,
    mut h: U32,
    mut s: U64,
) -> size_t {
    return ZSTD_hash5(MEM_readLE64(p), h, s);
}
static mut prime6bytes: U64 = 227718039650203 as U64;
unsafe extern "C" fn ZSTD_hash6(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return (((u << 64 as ::core::ffi::c_int - 48 as ::core::ffi::c_int).wrapping_mul(prime6bytes)
        ^ s)
        >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash6Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash6(MEM_readLE64(p), h, 0 as U64);
}
unsafe extern "C" fn ZSTD_hash6PtrS(
    mut p: *const ::core::ffi::c_void,
    mut h: U32,
    mut s: U64,
) -> size_t {
    return ZSTD_hash6(MEM_readLE64(p), h, s);
}
static mut prime7bytes: U64 = 58295818150454627 as U64;
unsafe extern "C" fn ZSTD_hash7(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return (((u << 64 as ::core::ffi::c_int - 56 as ::core::ffi::c_int).wrapping_mul(prime7bytes)
        ^ s)
        >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash7Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash7(MEM_readLE64(p), h, 0 as U64);
}
unsafe extern "C" fn ZSTD_hash7PtrS(
    mut p: *const ::core::ffi::c_void,
    mut h: U32,
    mut s: U64,
) -> size_t {
    return ZSTD_hash7(MEM_readLE64(p), h, s);
}
static mut prime8bytes: U64 = 0xcf1bbcdcb7a56463 as U64;
unsafe extern "C" fn ZSTD_hash8(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return ((u.wrapping_mul(prime8bytes) ^ s) >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash8Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash8(MEM_readLE64(p), h, 0 as U64);
}
unsafe extern "C" fn ZSTD_hash8PtrS(
    mut p: *const ::core::ffi::c_void,
    mut h: U32,
    mut s: U64,
) -> size_t {
    return ZSTD_hash8(MEM_readLE64(p), h, s);
}
#[inline(always)]
unsafe extern "C" fn ZSTD_hashPtr(
    mut p: *const ::core::ffi::c_void,
    mut hBits: U32,
    mut mls: U32,
) -> size_t {
    match mls {
        5 => return ZSTD_hash5Ptr(p, hBits),
        6 => return ZSTD_hash6Ptr(p, hBits),
        7 => return ZSTD_hash7Ptr(p, hBits),
        8 => return ZSTD_hash8Ptr(p, hBits),
        4 | _ => return ZSTD_hash4Ptr(p, hBits),
    };
}
#[inline(always)]
unsafe extern "C" fn ZSTD_hashPtrSalted(
    mut p: *const ::core::ffi::c_void,
    mut hBits: U32,
    mut mls: U32,
    hashSalt: U64,
) -> size_t {
    match mls {
        5 => return ZSTD_hash5PtrS(p, hBits, hashSalt),
        6 => return ZSTD_hash6PtrS(p, hBits, hashSalt),
        7 => return ZSTD_hash7PtrS(p, hBits, hashSalt),
        8 => return ZSTD_hash8PtrS(p, hBits, hashSalt),
        4 | _ => return ZSTD_hash4PtrS(p, hBits, hashSalt as U32),
    };
}
#[inline]
unsafe extern "C" fn ZSTD_getLowestMatchIndex(
    mut ms: *const ZSTD_MatchState_t,
    mut curr: U32,
    mut windowLog: ::core::ffi::c_uint,
) -> U32 {
    let maxDistance: U32 = (1 as U32) << windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinWindow: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0 as U32) as ::core::ffi::c_int as U32;
    let matchLowest: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    };
    return matchLowest;
}
#[inline]
unsafe extern "C" fn ZSTD_getLowestPrefixIndex(
    mut ms: *const ZSTD_MatchState_t,
    mut curr: U32,
    mut windowLog: ::core::ffi::c_uint,
) -> U32 {
    let maxDistance: U32 = (1 as U32) << windowLog;
    let lowestValid: U32 = (*ms).window.dictLimit;
    let withinWindow: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0 as U32) as ::core::ffi::c_int as U32;
    let matchLowest: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    };
    return matchLowest;
}
#[inline]
unsafe extern "C" fn ZSTD_index_overlap_check(
    prefixLowestIndex: U32,
    repIndex: U32,
) -> ::core::ffi::c_int {
    return (prefixLowestIndex
        .wrapping_sub(1 as U32)
        .wrapping_sub(repIndex)
        >= 3 as U32) as ::core::ffi::c_int;
}
pub const ZSTD_LAZY_DDSS_BUCKET_LOG: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ZSTD_ROW_HASH_TAG_BITS: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const kLazySkippingStep: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_updateDUBT(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    mut iend: *const BYTE,
    mut mls: U32,
) {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog as U32;
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = ((*cParams).chainLog as U32).wrapping_sub(1 as U32);
    let btMask: U32 = (((1 as ::core::ffi::c_int) << btLog) - 1 as ::core::ffi::c_int) as U32;
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let mut idx: U32 = (*ms).nextToUpdate;
    idx != target;
    while idx < target {
        let h: size_t = ZSTD_hashPtr(
            base.offset(idx as isize) as *const ::core::ffi::c_void,
            hashLog,
            mls,
        ) as size_t;
        let matchIndex: U32 = *hashTable.offset(h as isize);
        let nextCandidatePtr: *mut U32 = bt.offset((2 as U32).wrapping_mul(idx & btMask) as isize);
        let sortMarkPtr: *mut U32 = nextCandidatePtr.offset(1 as ::core::ffi::c_int as isize);
        *hashTable.offset(h as isize) = idx;
        *nextCandidatePtr = matchIndex;
        *sortMarkPtr = ZSTD_DUBT_UNSORTED_MARK as U32;
        idx = idx.wrapping_add(1);
    }
    (*ms).nextToUpdate = target;
}
unsafe extern "C" fn ZSTD_insertDUBT1(
    mut ms: *const ZSTD_MatchState_t,
    mut curr: U32,
    mut inputEnd: *const BYTE,
    mut nbCompares: U32,
    mut btLow: U32,
    dictMode: ZSTD_dictMode_e,
) {
    let cParams: *const ZSTD_compressionParameters = &raw const (*ms).cParams;
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = ((*cParams).chainLog as U32).wrapping_sub(1 as U32);
    let btMask: U32 = (((1 as ::core::ffi::c_int) << btLog) - 1 as ::core::ffi::c_int) as U32;
    let mut commonLengthSmaller: size_t = 0 as size_t;
    let mut commonLengthLarger: size_t = 0 as size_t;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let ip: *const BYTE = if curr >= dictLimit {
        base.offset(curr as isize)
    } else {
        dictBase.offset(curr as isize)
    };
    let iend: *const BYTE = if curr >= dictLimit {
        inputEnd
    } else {
        dictBase.offset(dictLimit as isize)
    };
    let dictEnd: *const BYTE = dictBase.offset(dictLimit as isize);
    let prefixStart: *const BYTE = base.offset(dictLimit as isize);
    let mut match_0: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut smallerPtr: *mut U32 = bt.offset((2 as U32).wrapping_mul(curr & btMask) as isize);
    let mut largerPtr: *mut U32 = smallerPtr.offset(1 as ::core::ffi::c_int as isize);
    let mut matchIndex: U32 = *smallerPtr;
    let mut dummy32: U32 = 0;
    let windowValid: U32 = (*ms).window.lowLimit;
    let maxDistance: U32 = (1 as U32) << (*cParams).windowLog;
    let windowLow: U32 = if curr.wrapping_sub(windowValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        windowValid
    };
    while nbCompares != 0 && matchIndex > windowLow {
        let nextPtr: *mut U32 = bt.offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize);
        let mut matchLength: size_t = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };
        if dictMode as ::core::ffi::c_uint
            != ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
            || (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t
            || curr < dictLimit
        {
            let mBase: *const BYTE = if dictMode as ::core::ffi::c_uint
                != ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
                || (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t
            {
                base
            } else {
                dictBase
            };
            match_0 = mBase.offset(matchIndex as isize);
            matchLength = (matchLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count(
                ip.offset(matchLength as isize),
                match_0.offset(matchLength as isize),
                iend,
            )
                as ::core::ffi::c_ulong) as size_t as size_t;
        } else {
            match_0 = dictBase.offset(matchIndex as isize);
            matchLength = (matchLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count_2segments(
                ip.offset(matchLength as isize),
                match_0.offset(matchLength as isize),
                iend,
                dictEnd,
                prefixStart,
            )
                as ::core::ffi::c_ulong) as size_t as size_t;
            if (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t {
                match_0 = base.offset(matchIndex as isize);
            }
        }
        if ip.offset(matchLength as isize) == iend {
            break;
        }
        if (*match_0.offset(matchLength as isize) as ::core::ffi::c_int)
            < *ip.offset(matchLength as isize) as ::core::ffi::c_int
        {
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &raw mut dummy32;
                break;
            } else {
                smallerPtr = nextPtr.offset(1 as ::core::ffi::c_int as isize);
                matchIndex = *nextPtr.offset(1 as ::core::ffi::c_int as isize);
            }
        } else {
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &raw mut dummy32;
                break;
            } else {
                largerPtr = nextPtr;
                matchIndex = *nextPtr.offset(0 as ::core::ffi::c_int as isize);
            }
        }
        nbCompares = nbCompares.wrapping_sub(1);
    }
    *largerPtr = 0 as U32;
    *smallerPtr = *largerPtr;
}
unsafe extern "C" fn ZSTD_DUBT_findBetterDictMatch(
    mut ms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    mut offsetPtr: *mut size_t,
    mut bestLength: size_t,
    mut nbCompares: U32,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dmsCParams: *const ZSTD_compressionParameters = &raw const (*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let hashLog: U32 = (*dmsCParams).hashLog as U32;
    let h: size_t = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hashLog, mls) as size_t;
    let mut dictMatchIndex: U32 = *dictHashTable.offset(h as isize);
    let base: *const BYTE = (*ms).window.base;
    let prefixStart: *const BYTE = base.offset((*ms).window.dictLimit as isize);
    let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictHighLimit: U32 =
        (*dms).window.nextSrc.offset_from((*dms).window.base) as ::core::ffi::c_long as U32;
    let dictLowLimit: U32 = (*dms).window.lowLimit;
    let dictIndexDelta: U32 = (*ms).window.lowLimit.wrapping_sub(dictHighLimit);
    let dictBt: *mut U32 = (*dms).chainTable;
    let btLog: U32 = ((*dmsCParams).chainLog as U32).wrapping_sub(1 as U32);
    let btMask: U32 = (((1 as ::core::ffi::c_int) << btLog) - 1 as ::core::ffi::c_int) as U32;
    let btLow: U32 = if btMask >= dictHighLimit.wrapping_sub(dictLowLimit) {
        dictLowLimit
    } else {
        dictHighLimit.wrapping_sub(btMask)
    };
    let mut commonLengthSmaller: size_t = 0 as size_t;
    let mut commonLengthLarger: size_t = 0 as size_t;
    while nbCompares != 0 && dictMatchIndex > dictLowLimit {
        let nextPtr: *mut U32 =
            dictBt.offset((2 as U32).wrapping_mul(dictMatchIndex & btMask) as isize);
        let mut matchLength: size_t = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };
        let mut match_0: *const BYTE = dictBase.offset(dictMatchIndex as isize);
        matchLength = (matchLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count_2segments(
            ip.offset(matchLength as isize),
            match_0.offset(matchLength as isize),
            iend,
            dictEnd,
            prefixStart,
        )
            as ::core::ffi::c_ulong) as size_t as size_t;
        if (dictMatchIndex as size_t).wrapping_add(matchLength) >= dictHighLimit as size_t {
            match_0 = base
                .offset(dictMatchIndex as isize)
                .offset(dictIndexDelta as isize);
        }
        if matchLength > bestLength {
            let mut matchIndex: U32 = dictMatchIndex.wrapping_add(dictIndexDelta);
            if 4 as ::core::ffi::c_int * matchLength.wrapping_sub(bestLength) as ::core::ffi::c_int
                > ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1 as U32)).wrapping_sub(
                    ZSTD_highbit32(
                        (*offsetPtr.offset(0 as ::core::ffi::c_int as isize) as U32)
                            .wrapping_add(1 as U32),
                    ),
                ) as ::core::ffi::c_int
            {
                bestLength = matchLength;
                *offsetPtr = (curr as ::core::ffi::c_uint)
                    .wrapping_sub(matchIndex as ::core::ffi::c_uint)
                    .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                    as size_t;
            }
            if ip.offset(matchLength as isize) == iend {
                break;
            }
        }
        if (*match_0.offset(matchLength as isize) as ::core::ffi::c_int)
            < *ip.offset(matchLength as isize) as ::core::ffi::c_int
        {
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthSmaller = matchLength;
            dictMatchIndex = *nextPtr.offset(1 as ::core::ffi::c_int as isize);
        } else {
            if dictMatchIndex <= btLow {
                break;
            }
            commonLengthLarger = matchLength;
            dictMatchIndex = *nextPtr.offset(0 as ::core::ffi::c_int as isize);
        }
        nbCompares = nbCompares.wrapping_sub(1);
    }
    if bestLength >= MINMATCH as size_t {
        let mIndex: U32 = curr
            .wrapping_sub((*offsetPtr).wrapping_sub(ZSTD_REP_NUM as size_t) as U32);
    }
    return bestLength;
}
unsafe extern "C" fn ZSTD_DUBT_findBestMatch(
    mut ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    mut offBasePtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog as U32;
    let h: size_t = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hashLog, mls) as size_t;
    let mut matchIndex: U32 = *hashTable.offset(h as isize);
    let base: *const BYTE = (*ms).window.base;
    let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let windowLow: U32 = ZSTD_getLowestMatchIndex(ms, curr, (*cParams).windowLog) as U32;
    let bt: *mut U32 = (*ms).chainTable;
    let btLog: U32 = ((*cParams).chainLog as U32).wrapping_sub(1 as U32);
    let btMask: U32 = (((1 as ::core::ffi::c_int) << btLog) - 1 as ::core::ffi::c_int) as U32;
    let btLow: U32 = if btMask >= curr {
        0 as U32
    } else {
        curr.wrapping_sub(btMask)
    };
    let unsortLimit: U32 = if btLow > windowLow { btLow } else { windowLow };
    let mut nextCandidate: *mut U32 =
        bt.offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize);
    let mut unsortedMark: *mut U32 = bt
        .offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize)
        .offset(1 as ::core::ffi::c_int as isize);
    let mut nbCompares: U32 = (1 as U32) << (*cParams).searchLog;
    let mut nbCandidates: U32 = nbCompares;
    let mut previousCandidate: U32 = 0 as U32;
    while matchIndex > unsortLimit
        && *unsortedMark == ZSTD_DUBT_UNSORTED_MARK as U32
        && nbCandidates > 1 as U32
    {
        *unsortedMark = previousCandidate;
        previousCandidate = matchIndex;
        matchIndex = *nextCandidate;
        nextCandidate = bt.offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize);
        unsortedMark = bt
            .offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize)
            .offset(1 as ::core::ffi::c_int as isize);
        nbCandidates = nbCandidates.wrapping_sub(1);
    }
    if matchIndex > unsortLimit && *unsortedMark == ZSTD_DUBT_UNSORTED_MARK as U32 {
        *unsortedMark = 0 as U32;
        *nextCandidate = *unsortedMark;
    }
    matchIndex = previousCandidate;
    while matchIndex != 0 {
        let nextCandidateIdxPtr: *mut U32 = bt
            .offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize)
            .offset(1 as ::core::ffi::c_int as isize);
        let nextCandidateIdx: U32 = *nextCandidateIdxPtr;
        ZSTD_insertDUBT1(ms, matchIndex, iend, nbCandidates, unsortLimit, dictMode);
        matchIndex = nextCandidateIdx;
        nbCandidates = nbCandidates.wrapping_add(1);
    }
    let mut commonLengthSmaller: size_t = 0 as size_t;
    let mut commonLengthLarger: size_t = 0 as size_t;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let dictEnd: *const BYTE = dictBase.offset(dictLimit as isize);
    let prefixStart: *const BYTE = base.offset(dictLimit as isize);
    let mut smallerPtr: *mut U32 = bt.offset((2 as U32).wrapping_mul(curr & btMask) as isize);
    let mut largerPtr: *mut U32 = bt
        .offset((2 as U32).wrapping_mul(curr & btMask) as isize)
        .offset(1 as ::core::ffi::c_int as isize);
    let mut matchEndIdx: U32 = curr.wrapping_add(8 as U32).wrapping_add(1 as U32);
    let mut dummy32: U32 = 0;
    let mut bestLength: size_t = 0 as size_t;
    matchIndex = *hashTable.offset(h as isize);
    *hashTable.offset(h as isize) = curr;
    while nbCompares != 0 && matchIndex > windowLow {
        let nextPtr: *mut U32 = bt.offset((2 as U32).wrapping_mul(matchIndex & btMask) as isize);
        let mut matchLength: size_t = if commonLengthSmaller < commonLengthLarger {
            commonLengthSmaller
        } else {
            commonLengthLarger
        };
        let mut match_0: *const BYTE = ::core::ptr::null::<BYTE>();
        if dictMode as ::core::ffi::c_uint
            != ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
            || (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t
        {
            match_0 = base.offset(matchIndex as isize);
            matchLength = (matchLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count(
                ip.offset(matchLength as isize),
                match_0.offset(matchLength as isize),
                iend,
            )
                as ::core::ffi::c_ulong) as size_t as size_t;
        } else {
            match_0 = dictBase.offset(matchIndex as isize);
            matchLength = (matchLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count_2segments(
                ip.offset(matchLength as isize),
                match_0.offset(matchLength as isize),
                iend,
                dictEnd,
                prefixStart,
            )
                as ::core::ffi::c_ulong) as size_t as size_t;
            if (matchIndex as size_t).wrapping_add(matchLength) >= dictLimit as size_t {
                match_0 = base.offset(matchIndex as isize);
            }
        }
        if matchLength > bestLength {
            if matchLength > matchEndIdx.wrapping_sub(matchIndex) as size_t {
                matchEndIdx = matchIndex.wrapping_add(matchLength as U32);
            }
            if 4 as ::core::ffi::c_int * matchLength.wrapping_sub(bestLength) as ::core::ffi::c_int
                > ZSTD_highbit32(curr.wrapping_sub(matchIndex).wrapping_add(1 as U32))
                    .wrapping_sub(ZSTD_highbit32(*offBasePtr as U32))
                    as ::core::ffi::c_int
            {
                bestLength = matchLength;
                *offBasePtr = (curr as ::core::ffi::c_uint)
                    .wrapping_sub(matchIndex as ::core::ffi::c_uint)
                    .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                    as size_t;
            }
            if ip.offset(matchLength as isize) == iend {
                if dictMode as ::core::ffi::c_uint
                    == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    nbCompares = 0 as U32;
                }
                break;
            }
        }
        if (*match_0.offset(matchLength as isize) as ::core::ffi::c_int)
            < *ip.offset(matchLength as isize) as ::core::ffi::c_int
        {
            *smallerPtr = matchIndex;
            commonLengthSmaller = matchLength;
            if matchIndex <= btLow {
                smallerPtr = &raw mut dummy32;
                break;
            } else {
                smallerPtr = nextPtr.offset(1 as ::core::ffi::c_int as isize);
                matchIndex = *nextPtr.offset(1 as ::core::ffi::c_int as isize);
            }
        } else {
            *largerPtr = matchIndex;
            commonLengthLarger = matchLength;
            if matchIndex <= btLow {
                largerPtr = &raw mut dummy32;
                break;
            } else {
                largerPtr = nextPtr;
                matchIndex = *nextPtr.offset(0 as ::core::ffi::c_int as isize);
            }
        }
        nbCompares = nbCompares.wrapping_sub(1);
    }
    *largerPtr = 0 as U32;
    *smallerPtr = *largerPtr;
    if dictMode as ::core::ffi::c_uint
        == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
        && nbCompares != 0
    {
        bestLength = ZSTD_DUBT_findBetterDictMatch(
            ms, ip, iend, offBasePtr, bestLength, nbCompares, mls, dictMode,
        );
    }
    (*ms).nextToUpdate = matchEndIdx.wrapping_sub(8 as U32);
    if bestLength >= MINMATCH as size_t {
        let mIndex: U32 = curr
            .wrapping_sub((*offBasePtr).wrapping_sub(ZSTD_REP_NUM as size_t) as U32);
    }
    return bestLength;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_BtFindBestMatch(
    mut ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    if ip < (*ms).window.base.offset((*ms).nextToUpdate as isize) {
        return 0 as size_t;
    }
    ZSTD_updateDUBT(ms, ip, iLimit, mls);
    return ZSTD_DUBT_findBestMatch(ms, ip, iLimit, offBasePtr, mls, dictMode);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_dedicatedDictSearch_lazy_loadDictionary(
    mut ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
) {
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let hashTable: *mut U32 = (*ms).hashTable;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = ((1 as ::core::ffi::c_int) << (*ms).cParams.chainLog) as U32;
    let mut idx: U32 = (*ms).nextToUpdate;
    let minChain: U32 = if chainSize < target.wrapping_sub(idx) {
        target.wrapping_sub(chainSize)
    } else {
        idx
    };
    let bucketSize: U32 = ((1 as ::core::ffi::c_int) << ZSTD_LAZY_DDSS_BUCKET_LOG) as U32;
    let cacheSize: U32 = bucketSize.wrapping_sub(1 as U32);
    let chainAttempts: U32 =
        (((1 as ::core::ffi::c_int) << (*ms).cParams.searchLog) as U32).wrapping_sub(cacheSize);
    let chainLimit: U32 = if chainAttempts > 255 as U32 {
        255 as U32
    } else {
        chainAttempts
    };
    let hashLog: U32 =
        ((*ms).cParams.hashLog as U32).wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG as U32);
    let tmpHashTable: *mut U32 = hashTable;
    let tmpChainTable: *mut U32 =
        hashTable.offset(((1 as ::core::ffi::c_int as size_t) << hashLog) as isize);
    let tmpChainSize: U32 = ((((1 as ::core::ffi::c_int) << ZSTD_LAZY_DDSS_BUCKET_LOG)
        - 1 as ::core::ffi::c_int) as U32)
        << hashLog;
    let tmpMinChain: U32 = if tmpChainSize < target {
        target.wrapping_sub(tmpChainSize)
    } else {
        idx
    };
    let mut hashIdx: U32 = 0;
    while idx < target {
        let h: U32 = ZSTD_hashPtr(
            base.offset(idx as isize) as *const ::core::ffi::c_void,
            hashLog,
            (*ms).cParams.minMatch as U32,
        ) as U32;
        if idx >= tmpMinChain {
            *tmpChainTable.offset(idx.wrapping_sub(tmpMinChain) as isize) =
                *hashTable.offset(h as isize);
        }
        *tmpHashTable.offset(h as isize) = idx;
        idx = idx.wrapping_add(1);
    }
    let mut chainPos: U32 = 0 as U32;
    hashIdx = 0 as U32;
    while hashIdx < (1 as U32) << hashLog {
        let mut count: U32 = 0;
        let mut countBeyondMinChain: U32 = 0 as U32;
        let mut i: U32 = *tmpHashTable.offset(hashIdx as isize);
        count = 0 as U32;
        while i >= tmpMinChain && count < cacheSize {
            if i < minChain {
                countBeyondMinChain = countBeyondMinChain.wrapping_add(1);
            }
            i = *tmpChainTable.offset(i.wrapping_sub(tmpMinChain) as isize);
            count = count.wrapping_add(1);
        }
        if count == cacheSize {
            count = 0 as U32;
            while count < chainLimit {
                if i < minChain {
                    if i == 0 || {
                        countBeyondMinChain = countBeyondMinChain.wrapping_add(1);
                        countBeyondMinChain > cacheSize
                    } {
                        break;
                    }
                }
                let fresh2 = chainPos;
                chainPos = chainPos.wrapping_add(1);
                *chainTable.offset(fresh2 as isize) = i;
                count = count.wrapping_add(1);
                if i < tmpMinChain {
                    break;
                }
                i = *tmpChainTable.offset(i.wrapping_sub(tmpMinChain) as isize);
            }
        } else {
            count = 0 as U32;
        }
        if count != 0 {
            *tmpHashTable.offset(hashIdx as isize) =
                (chainPos.wrapping_sub(count) << 8 as ::core::ffi::c_int).wrapping_add(count);
        } else {
            *tmpHashTable.offset(hashIdx as isize) = 0 as U32;
        }
        hashIdx = hashIdx.wrapping_add(1);
    }
    hashIdx = ((1 as ::core::ffi::c_int) << hashLog) as U32;
    while hashIdx != 0 {
        hashIdx = hashIdx.wrapping_sub(1);
        let bucketIdx: U32 = hashIdx << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let chainPackedPointer: U32 = *tmpHashTable.offset(hashIdx as isize);
        let mut i_0: U32 = 0;
        i_0 = 0 as U32;
        while i_0 < cacheSize {
            *hashTable.offset(bucketIdx.wrapping_add(i_0) as isize) = 0 as U32;
            i_0 = i_0.wrapping_add(1);
        }
        *hashTable.offset(bucketIdx.wrapping_add(bucketSize).wrapping_sub(1 as U32) as isize) =
            chainPackedPointer;
    }
    idx = (*ms).nextToUpdate;
    while idx < target {
        let h_0: U32 = (ZSTD_hashPtr(
            base.offset(idx as isize) as *const ::core::ffi::c_void,
            hashLog,
            (*ms).cParams.minMatch as U32,
        ) as U32)
            << ZSTD_LAZY_DDSS_BUCKET_LOG;
        let mut i_1: U32 = 0;
        i_1 = cacheSize.wrapping_sub(1 as U32);
        while i_1 != 0 {
            *hashTable.offset(h_0.wrapping_add(i_1) as isize) =
                *hashTable.offset(h_0.wrapping_add(i_1).wrapping_sub(1 as U32) as isize);
            i_1 = i_1.wrapping_sub(1);
        }
        *hashTable.offset(h_0 as isize) = idx;
        idx = idx.wrapping_add(1);
    }
    (*ms).nextToUpdate = target;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_dedicatedDictSearch_lazy_search(
    mut offsetPtr: *mut size_t,
    mut ml: size_t,
    mut nbAttempts: U32,
    dms: *const ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    prefixStart: *const BYTE,
    curr: U32,
    dictLimit: U32,
    ddsIdx: size_t,
) -> size_t {
    let ddsLowestIndex: U32 = (*dms).window.dictLimit;
    let ddsBase: *const BYTE = (*dms).window.base;
    let ddsEnd: *const BYTE = (*dms).window.nextSrc;
    let ddsSize: U32 = ddsEnd.offset_from(ddsBase) as ::core::ffi::c_long as U32;
    let ddsIndexDelta: U32 = dictLimit.wrapping_sub(ddsSize);
    let bucketSize: U32 = ((1 as ::core::ffi::c_int) << ZSTD_LAZY_DDSS_BUCKET_LOG) as U32;
    let bucketLimit: U32 = if nbAttempts < bucketSize.wrapping_sub(1 as U32) {
        nbAttempts
    } else {
        bucketSize.wrapping_sub(1 as U32)
    };
    let mut ddsAttempt: U32 = 0;
    let mut matchIndex: U32 = 0;
    ddsAttempt = 0 as U32;
    while ddsAttempt < bucketSize.wrapping_sub(1 as U32) {
        ddsAttempt = ddsAttempt.wrapping_add(1);
    }
    let chainPackedPointer: U32 = *(*dms).hashTable.offset(
        ddsIdx
            .wrapping_add(bucketSize as size_t)
            .wrapping_sub(1 as size_t) as isize,
    );
    let chainIndex: U32 = chainPackedPointer >> 8 as ::core::ffi::c_int;
    (*dms).chainTable.offset(chainIndex as isize) as *mut U32;
    ddsAttempt = 0 as U32;
    while ddsAttempt < bucketLimit {
        let mut currentMl: size_t = 0 as size_t;
        let mut match_0: *const BYTE = ::core::ptr::null::<BYTE>();
        matchIndex = *(*dms)
            .hashTable
            .offset(ddsIdx.wrapping_add(ddsAttempt as size_t) as isize);
        match_0 = ddsBase.offset(matchIndex as isize);
        if matchIndex == 0 {
            return ml;
        }
        if MEM_read32(match_0 as *const ::core::ffi::c_void)
            == MEM_read32(ip as *const ::core::ffi::c_void)
        {
            currentMl = ZSTD_count_2segments(
                ip.offset(4 as ::core::ffi::c_int as isize),
                match_0.offset(4 as ::core::ffi::c_int as isize),
                iLimit,
                ddsEnd,
                prefixStart,
            )
            .wrapping_add(4 as size_t);
        }
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = (curr as ::core::ffi::c_uint)
                .wrapping_sub(
                    (matchIndex as ::core::ffi::c_uint)
                        .wrapping_add(ddsIndexDelta as ::core::ffi::c_uint),
                )
                .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                as size_t;
            if ip.offset(currentMl as isize) == iLimit {
                return ml;
            }
        }
        ddsAttempt = ddsAttempt.wrapping_add(1);
    }
    let chainPackedPointer_0: U32 = *(*dms).hashTable.offset(
        ddsIdx
            .wrapping_add(bucketSize as size_t)
            .wrapping_sub(1 as size_t) as isize,
    );
    let mut chainIndex_0: U32 = chainPackedPointer_0 >> 8 as ::core::ffi::c_int;
    let chainLength: U32 = chainPackedPointer_0 & 0xff as U32;
    let chainAttempts: U32 = nbAttempts.wrapping_sub(ddsAttempt);
    let chainLimit: U32 = if chainAttempts > chainLength {
        chainLength
    } else {
        chainAttempts
    };
    let mut chainAttempt: U32 = 0;
    chainAttempt = 0 as U32;
    while chainAttempt < chainLimit {
        chainAttempt = chainAttempt.wrapping_add(1);
    }
    chainAttempt = 0 as U32;
    while chainAttempt < chainLimit {
        let mut currentMl_0: size_t = 0 as size_t;
        let mut match_1: *const BYTE = ::core::ptr::null::<BYTE>();
        matchIndex = *(*dms).chainTable.offset(chainIndex_0 as isize);
        match_1 = ddsBase.offset(matchIndex as isize);
        if MEM_read32(match_1 as *const ::core::ffi::c_void)
            == MEM_read32(ip as *const ::core::ffi::c_void)
        {
            currentMl_0 = ZSTD_count_2segments(
                ip.offset(4 as ::core::ffi::c_int as isize),
                match_1.offset(4 as ::core::ffi::c_int as isize),
                iLimit,
                ddsEnd,
                prefixStart,
            )
            .wrapping_add(4 as size_t);
        }
        if currentMl_0 > ml {
            ml = currentMl_0;
            *offsetPtr = (curr as ::core::ffi::c_uint)
                .wrapping_sub(
                    (matchIndex as ::core::ffi::c_uint)
                        .wrapping_add(ddsIndexDelta as ::core::ffi::c_uint),
                )
                .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                as size_t;
            if ip.offset(currentMl_0 as isize) == iLimit {
                break;
            }
        }
        chainAttempt = chainAttempt.wrapping_add(1);
        chainIndex_0 = chainIndex_0.wrapping_add(1);
    }
    return ml;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_insertAndFindFirstIndex_internal(
    mut ms: *mut ZSTD_MatchState_t,
    cParams: *const ZSTD_compressionParameters,
    mut ip: *const BYTE,
    mls: U32,
    lazySkipping: U32,
) -> U32 {
    let hashTable: *mut U32 = (*ms).hashTable;
    let hashLog: U32 = (*cParams).hashLog as U32;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainMask: U32 =
        (((1 as ::core::ffi::c_int) << (*cParams).chainLog) - 1 as ::core::ffi::c_int) as U32;
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let mut idx: U32 = (*ms).nextToUpdate;
    while idx < target {
        let h: size_t = ZSTD_hashPtr(
            base.offset(idx as isize) as *const ::core::ffi::c_void,
            hashLog,
            mls,
        ) as size_t;
        *chainTable.offset((idx & chainMask) as isize) = *hashTable.offset(h as isize);
        *hashTable.offset(h as isize) = idx;
        idx = idx.wrapping_add(1);
        if lazySkipping != 0 {
            break;
        }
    }
    (*ms).nextToUpdate = target;
    return *hashTable
        .offset(ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hashLog, mls) as isize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertAndFindFirstIndex(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
) -> U32 {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    return ZSTD_insertAndFindFirstIndex_internal(
        ms,
        cParams,
        ip,
        (*ms).cParams.minMatch as U32,
        0 as U32,
    );
}
#[inline(always)]
unsafe extern "C" fn ZSTD_HcFindBestMatch(
    mut ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let chainTable: *mut U32 = (*ms).chainTable;
    let chainSize: U32 = ((1 as ::core::ffi::c_int) << (*cParams).chainLog) as U32;
    let chainMask: U32 = chainSize.wrapping_sub(1 as U32);
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.offset(dictLimit as isize);
    let dictEnd: *const BYTE = dictBase.offset(dictLimit as isize);
    let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let maxDistance: U32 = (1 as U32) << (*cParams).windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinMaxDistance: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0 as U32) as ::core::ffi::c_int as U32;
    let lowLimit: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let minChain: U32 = if curr > chainSize {
        curr.wrapping_sub(chainSize)
    } else {
        0 as U32
    };
    let mut nbAttempts: U32 = (1 as U32) << (*cParams).searchLog;
    let mut ml: size_t = (4 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as size_t;
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let ddsHashLog: U32 = if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ((*dms).cParams.hashLog as U32).wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG as U32)
    } else {
        0 as U32
    };
    let ddsIdx: size_t = if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (ZSTD_hashPtr(ip as *const ::core::ffi::c_void, ddsHashLog, mls) as size_t)
            << ZSTD_LAZY_DDSS_BUCKET_LOG
    } else {
        0 as size_t
    };
    let mut matchIndex: U32 = 0;
    if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut entry: *const U32 = (*dms).hashTable.offset(ddsIdx as isize) as *mut U32;
    }
    matchIndex =
        ZSTD_insertAndFindFirstIndex_internal(ms, cParams, ip, mls, (*ms).lazySkipping as U32);
    while (matchIndex >= lowLimit) as ::core::ffi::c_int
        & (nbAttempts > 0 as U32) as ::core::ffi::c_int
        != 0
    {
        let mut currentMl: size_t = 0 as size_t;
        if dictMode as ::core::ffi::c_uint
            != ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
            || matchIndex >= dictLimit
        {
            let match_0: *const BYTE = base.offset(matchIndex as isize);
            if MEM_read32(
                match_0
                    .offset(ml as isize)
                    .offset(-(3 as ::core::ffi::c_int as isize))
                    as *const ::core::ffi::c_void,
            ) == MEM_read32(
                ip.offset(ml as isize)
                    .offset(-(3 as ::core::ffi::c_int as isize))
                    as *const ::core::ffi::c_void,
            ) {
                currentMl = ZSTD_count(ip, match_0, iLimit);
            }
        } else {
            let match_1: *const BYTE = dictBase.offset(matchIndex as isize);
            if MEM_read32(match_1 as *const ::core::ffi::c_void)
                == MEM_read32(ip as *const ::core::ffi::c_void)
            {
                currentMl = ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    match_1.offset(4 as ::core::ffi::c_int as isize),
                    iLimit,
                    dictEnd,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
            }
        }
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = (curr as ::core::ffi::c_uint)
                .wrapping_sub(matchIndex as ::core::ffi::c_uint)
                .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                as size_t;
            if ip.offset(currentMl as isize) == iLimit {
                break;
            }
        }
        if matchIndex <= minChain {
            break;
        }
        matchIndex = *chainTable.offset((matchIndex & chainMask) as isize);
        nbAttempts = nbAttempts.wrapping_sub(1);
    }
    if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ml = ZSTD_dedicatedDictSearch_lazy_search(
            offsetPtr,
            ml,
            nbAttempts,
            dms,
            ip,
            iLimit,
            prefixStart,
            curr,
            dictLimit,
            ddsIdx,
        );
    } else if dictMode as ::core::ffi::c_uint
        == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let dmsChainTable: *const U32 = (*dms).chainTable;
        let dmsChainSize: U32 = ((1 as ::core::ffi::c_int) << (*dms).cParams.chainLog) as U32;
        let dmsChainMask: U32 = dmsChainSize.wrapping_sub(1 as U32);
        let dmsLowestIndex: U32 = (*dms).window.dictLimit;
        let dmsBase: *const BYTE = (*dms).window.base;
        let dmsEnd: *const BYTE = (*dms).window.nextSrc;
        let dmsSize: U32 = dmsEnd.offset_from(dmsBase) as ::core::ffi::c_long as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);
        let dmsMinChain: U32 = if dmsSize > dmsChainSize {
            dmsSize.wrapping_sub(dmsChainSize)
        } else {
            0 as U32
        };
        matchIndex = *(*dms).hashTable.offset(ZSTD_hashPtr(
            ip as *const ::core::ffi::c_void,
            (*dms).cParams.hashLog as U32,
            mls,
        ) as isize);
        while (matchIndex >= dmsLowestIndex) as ::core::ffi::c_int
            & (nbAttempts > 0 as U32) as ::core::ffi::c_int
            != 0
        {
            let mut currentMl_0: size_t = 0 as size_t;
            let match_2: *const BYTE = dmsBase.offset(matchIndex as isize);
            if MEM_read32(match_2 as *const ::core::ffi::c_void)
                == MEM_read32(ip as *const ::core::ffi::c_void)
            {
                currentMl_0 = ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    match_2.offset(4 as ::core::ffi::c_int as isize),
                    iLimit,
                    dmsEnd,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
            }
            if currentMl_0 > ml {
                ml = currentMl_0;
                *offsetPtr = (curr as ::core::ffi::c_uint)
                    .wrapping_sub(
                        (matchIndex as ::core::ffi::c_uint)
                            .wrapping_add(dmsIndexDelta as ::core::ffi::c_uint),
                    )
                    .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                    as size_t;
                if ip.offset(currentMl_0 as isize) == iLimit {
                    break;
                }
            }
            if matchIndex <= dmsMinChain {
                break;
            }
            matchIndex = *dmsChainTable.offset((matchIndex & dmsChainMask) as isize);
            nbAttempts = nbAttempts.wrapping_sub(1);
        }
    }
    return ml;
}
pub const ZSTD_ROW_HASH_TAG_MASK: ::core::ffi::c_uint =
    ((1 as ::core::ffi::c_uint) << ZSTD_ROW_HASH_TAG_BITS).wrapping_sub(1 as ::core::ffi::c_uint);
pub const ZSTD_ROW_HASH_CACHE_MASK: ::core::ffi::c_int =
    ZSTD_ROW_HASH_CACHE_SIZE - 1 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTD_VecMask_next(mut val: ZSTD_VecMask) -> U32 {
    return ZSTD_countTrailingZeros64(val as U64) as U32;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_nextIndex(tagRow: *mut BYTE, rowMask: U32) -> U32 {
    let mut next: U32 = (*tagRow as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as U32 & rowMask;
    next = (next as ::core::ffi::c_uint)
        .wrapping_add((if next == 0 as U32 { rowMask } else { 0 as U32 }) as ::core::ffi::c_uint)
        as U32 as U32;
    *tagRow = next as BYTE;
    return next;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_prefetch(
    mut hashTable: *const U32,
    mut tagTable: *const BYTE,
    relRow: U32,
    rowLog: U32,
) {
    rowLog >= 5 as U32;
    rowLog == 6 as U32;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_fillHashCache(
    mut ms: *mut ZSTD_MatchState_t,
    mut base: *const BYTE,
    rowLog: U32,
    mls: U32,
    mut idx: U32,
    iLimit: *const BYTE,
) {
    let hashTable: *const U32 = (*ms).hashTable;
    let tagTable: *const BYTE = (*ms).tagTable;
    let hashLog: U32 = (*ms).rowHashLog;
    let maxElemsToPrefetch: U32 = if base.offset(idx as isize) > iLimit {
        0 as U32
    } else {
        (iLimit.offset_from(base.offset(idx as isize)) as ::core::ffi::c_long
            + 1 as ::core::ffi::c_long) as U32
    };
    let lim: U32 = idx.wrapping_add(
        (if (8 as U32) < maxElemsToPrefetch {
            8 as U32
        } else {
            maxElemsToPrefetch
        }),
    );
    while idx < lim {
        let hash: U32 = ZSTD_hashPtrSalted(
            base.offset(idx as isize) as *const ::core::ffi::c_void,
            hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS as U32),
            mls,
            (*ms).hashSalt,
        ) as U32;
        let row: U32 = hash >> ZSTD_ROW_HASH_TAG_BITS << rowLog;
        ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
        (*ms).hashCache[(idx & ZSTD_ROW_HASH_CACHE_MASK as U32) as usize] = hash;
        idx = idx.wrapping_add(1);
    }
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_nextCachedHash(
    mut cache: *mut U32,
    mut hashTable: *const U32,
    mut tagTable: *const BYTE,
    mut base: *const BYTE,
    mut idx: U32,
    hashLog: U32,
    rowLog: U32,
    mls: U32,
    hashSalt: U64,
) -> U32 {
    let newHash: U32 = ZSTD_hashPtrSalted(
        base.offset(idx as isize)
            .offset(ZSTD_ROW_HASH_CACHE_SIZE as isize) as *const ::core::ffi::c_void,
        hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS as U32),
        mls,
        hashSalt,
    ) as U32;
    let row: U32 = newHash >> ZSTD_ROW_HASH_TAG_BITS << rowLog;
    ZSTD_row_prefetch(hashTable, tagTable, row, rowLog);
    let hash: U32 = *cache.offset((idx & ZSTD_ROW_HASH_CACHE_MASK as U32) as isize);
    *cache.offset((idx & ZSTD_ROW_HASH_CACHE_MASK as U32) as isize) = newHash;
    return hash;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_update_internalImpl(
    mut ms: *mut ZSTD_MatchState_t,
    mut updateStartIdx: U32,
    updateEndIdx: U32,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let hashTable: *mut U32 = (*ms).hashTable;
    let tagTable: *mut BYTE = (*ms).tagTable;
    let hashLog: U32 = (*ms).rowHashLog;
    let base: *const BYTE = (*ms).window.base;
    while updateStartIdx < updateEndIdx {
        let hash: U32 = if useCache != 0 {
            ZSTD_row_nextCachedHash(
                &raw mut (*ms).hashCache as *mut U32,
                hashTable,
                tagTable,
                base,
                updateStartIdx,
                hashLog,
                rowLog,
                mls,
                (*ms).hashSalt,
            ) as U32
        } else {
            ZSTD_hashPtrSalted(
                base.offset(updateStartIdx as isize) as *const ::core::ffi::c_void,
                hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS as U32),
                mls,
                (*ms).hashSalt,
            ) as U32
        };
        let relRow: U32 = hash >> ZSTD_ROW_HASH_TAG_BITS << rowLog;
        let row: *mut U32 = hashTable.offset(relRow as isize);
        let mut tagRow: *mut BYTE = tagTable.offset(relRow as isize);
        let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask) as U32;
        *tagRow.offset(pos as isize) = (hash & ZSTD_ROW_HASH_TAG_MASK as U32) as BYTE;
        *row.offset(pos as isize) = updateStartIdx;
        updateStartIdx = updateStartIdx.wrapping_add(1);
    }
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_update_internal(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    mls: U32,
    rowLog: U32,
    rowMask: U32,
    useCache: U32,
) {
    let mut idx: U32 = (*ms).nextToUpdate;
    let base: *const BYTE = (*ms).window.base;
    let target: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let kSkipThreshold: U32 = 384 as U32;
    let kMaxMatchStartPositionsToUpdate: U32 = 96 as U32;
    let kMaxMatchEndPositionsToUpdate: U32 = 32 as U32;
    if useCache != 0 {
        if (target.wrapping_sub(idx) > kSkipThreshold) as ::core::ffi::c_int as ::core::ffi::c_long
            != 0
        {
            let bound: U32 = idx.wrapping_add(kMaxMatchStartPositionsToUpdate);
            ZSTD_row_update_internalImpl(ms, idx, bound, mls, rowLog, rowMask, useCache);
            idx = target.wrapping_sub(kMaxMatchEndPositionsToUpdate);
            ZSTD_row_fillHashCache(
                ms,
                base,
                rowLog,
                mls,
                idx,
                ip.offset(1 as ::core::ffi::c_int as isize),
            );
        }
    }
    ZSTD_row_update_internalImpl(ms, idx, target, mls, rowLog, rowMask, useCache);
    (*ms).nextToUpdate = target;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_row_update(ms: *mut ZSTD_MatchState_t, mut ip: *const BYTE) {
    let rowLog: U32 = if 4 as ::core::ffi::c_uint
        > (if (*ms).cParams.searchLog < 6 as ::core::ffi::c_uint {
            (*ms).cParams.searchLog
        } else {
            6 as ::core::ffi::c_uint
        }) {
        4 as U32
    } else if (*ms).cParams.searchLog < 6 as ::core::ffi::c_uint {
        (*ms).cParams.searchLog as U32
    } else {
        6 as U32
    };
    let rowMask: U32 = ((1 as U32) << rowLog).wrapping_sub(1 as U32);
    let mls: U32 = if (*ms).cParams.minMatch < 6 as ::core::ffi::c_uint {
        (*ms).cParams.minMatch as U32
    } else {
        6 as U32
    };
    ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 0 as U32);
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_matchMaskGroupWidth(rowEntries: U32) -> U32 {
    return 1 as U32;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_getSSEMask(
    mut nbChunks: ::core::ffi::c_int,
    src: *const BYTE,
    tag: BYTE,
    head: U32,
) -> ZSTD_VecMask {
    let comparisonMask: __m128i = _mm_set1_epi8(tag as ::core::ffi::c_char) as __m128i;
    let mut matches: [::core::ffi::c_int; 4] = [0 as ::core::ffi::c_int; 4];
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < nbChunks {
        let chunk: __m128i = _mm_loadu_si128(src.offset((16 as ::core::ffi::c_int * i) as isize)
            as *const ::core::ffi::c_void
            as *const __m128i_u) as __m128i;
        let equalMask: __m128i = _mm_cmpeq_epi8(chunk, comparisonMask) as __m128i;
        matches[i as usize] = _mm_movemask_epi8(equalMask);
        i += 1;
    }
    if nbChunks == 1 as ::core::ffi::c_int {
        return ZSTD_rotateRight_U16(matches[0 as ::core::ffi::c_int as usize] as U16, head)
            as ZSTD_VecMask;
    }
    if nbChunks == 2 as ::core::ffi::c_int {
        return ZSTD_rotateRight_U32(
            (matches[1 as ::core::ffi::c_int as usize] as U32) << 16 as ::core::ffi::c_int
                | matches[0 as ::core::ffi::c_int as usize] as U32,
            head,
        ) as ZSTD_VecMask;
    }
    return ZSTD_rotateRight_U64(
        (matches[3 as ::core::ffi::c_int as usize] as U64) << 48 as ::core::ffi::c_int
            | (matches[2 as ::core::ffi::c_int as usize] as U64) << 32 as ::core::ffi::c_int
            | (matches[1 as ::core::ffi::c_int as usize] as U64) << 16 as ::core::ffi::c_int
            | matches[0 as ::core::ffi::c_int as usize] as U64,
        head,
    ) as ZSTD_VecMask;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_row_getMatchMask(
    tagRow: *const BYTE,
    tag: BYTE,
    headGrouped: U32,
    rowEntries: U32,
) -> ZSTD_VecMask {
    let src: *const BYTE = tagRow;
    return ZSTD_row_getSSEMask(
        rowEntries.wrapping_div(16 as U32) as ::core::ffi::c_int,
        src,
        tag,
        headGrouped,
    );
}
#[inline(always)]
unsafe extern "C" fn ZSTD_RowFindBestMatch(
    mut ms: *mut ZSTD_MatchState_t,
    ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
    mls: U32,
    dictMode: ZSTD_dictMode_e,
    rowLog: U32,
) -> size_t {
    let hashTable: *mut U32 = (*ms).hashTable;
    let tagTable: *mut BYTE = (*ms).tagTable;
    let hashCache: *mut U32 = &raw mut (*ms).hashCache as *mut U32;
    let hashLog: U32 = (*ms).rowHashLog;
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.offset(dictLimit as isize);
    let dictEnd: *const BYTE = dictBase.offset(dictLimit as isize);
    let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let maxDistance: U32 = (1 as U32) << (*cParams).windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinMaxDistance: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0 as U32) as ::core::ffi::c_int as U32;
    let lowLimit: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinMaxDistance
    };
    let rowEntries: U32 = (1 as U32) << rowLog;
    let rowMask: U32 = rowEntries.wrapping_sub(1 as U32);
    let cappedSearchLog: U32 = if ((*cParams).searchLog as U32) < rowLog {
        (*cParams).searchLog as U32
    } else {
        rowLog
    };
    let groupWidth: U32 = ZSTD_row_matchMaskGroupWidth(rowEntries) as U32;
    let hashSalt: U64 = (*ms).hashSalt;
    let mut nbAttempts: U32 = (1 as U32) << cappedSearchLog;
    let mut ml: size_t = (4 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as size_t;
    let mut hash: U32 = 0;
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let mut ddsIdx: size_t = 0 as size_t;
    let mut ddsExtraAttempts: U32 = 0 as U32;
    let mut dmsTag: U32 = 0 as U32;
    let mut dmsRow: *mut U32 = ::core::ptr::null_mut::<U32>();
    let mut dmsTagRow: *mut BYTE = ::core::ptr::null_mut::<BYTE>();
    if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let ddsHashLog: U32 =
            ((*dms).cParams.hashLog as U32).wrapping_sub(ZSTD_LAZY_DDSS_BUCKET_LOG as U32);
        ddsIdx = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, ddsHashLog, mls)
            << ZSTD_LAZY_DDSS_BUCKET_LOG;
        (*dms).hashTable.offset(ddsIdx as isize) as *mut U32;
        ddsExtraAttempts = (if (*cParams).searchLog as U32 > rowLog {
            (1 as ::core::ffi::c_uint) << ((*cParams).searchLog as U32).wrapping_sub(rowLog)
        } else {
            0 as ::core::ffi::c_uint
        }) as U32;
    }
    if dictMode as ::core::ffi::c_uint
        == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let dmsHashTable: *mut U32 = (*dms).hashTable;
        let dmsTagTable: *mut BYTE = (*dms).tagTable;
        let dmsHash: U32 = ZSTD_hashPtr(
            ip as *const ::core::ffi::c_void,
            (*dms)
                .rowHashLog
                .wrapping_add(ZSTD_ROW_HASH_TAG_BITS as U32),
            mls,
        ) as U32;
        let dmsRelRow: U32 = dmsHash >> ZSTD_ROW_HASH_TAG_BITS << rowLog;
        dmsTag = dmsHash & ZSTD_ROW_HASH_TAG_MASK as U32;
        dmsTagRow = dmsTagTable.offset(dmsRelRow as isize);
        dmsRow = dmsHashTable.offset(dmsRelRow as isize);
        ZSTD_row_prefetch(dmsHashTable, dmsTagTable, dmsRelRow, rowLog);
    }
    if (*ms).lazySkipping == 0 {
        ZSTD_row_update_internal(ms, ip, mls, rowLog, rowMask, 1 as U32);
        hash = ZSTD_row_nextCachedHash(
            hashCache, hashTable, tagTable, base, curr, hashLog, rowLog, mls, hashSalt,
        );
    } else {
        hash = ZSTD_hashPtrSalted(
            ip as *const ::core::ffi::c_void,
            hashLog.wrapping_add(ZSTD_ROW_HASH_TAG_BITS as U32),
            mls,
            hashSalt,
        ) as U32;
        (*ms).nextToUpdate = curr;
    }
    (*ms).hashSaltEntropy = ((*ms).hashSaltEntropy as ::core::ffi::c_uint)
        .wrapping_add(hash as ::core::ffi::c_uint) as U32 as U32;
    let relRow: U32 = hash >> ZSTD_ROW_HASH_TAG_BITS << rowLog;
    let tag: U32 = hash & ZSTD_ROW_HASH_TAG_MASK as U32;
    let row: *mut U32 = hashTable.offset(relRow as isize);
    let mut tagRow: *mut BYTE = tagTable.offset(relRow as isize);
    let headGrouped: U32 = (*tagRow as U32 & rowMask).wrapping_mul(groupWidth);
    let mut matchBuffer: [U32; 64] = [0; 64];
    let mut numMatches: size_t = 0 as size_t;
    let mut currMatch: size_t = 0 as size_t;
    let mut matches: ZSTD_VecMask =
        ZSTD_row_getMatchMask(tagRow, tag as BYTE, headGrouped, rowEntries);
    while matches > 0 as ZSTD_VecMask && nbAttempts > 0 as U32 {
        let matchPos: U32 = headGrouped
            .wrapping_add(ZSTD_VecMask_next(matches) as U32)
            .wrapping_div(groupWidth)
            & rowMask;
        let matchIndex: U32 = *row.offset(matchPos as isize);
        if !(matchPos == 0 as U32) {
            if matchIndex < lowLimit {
                break;
            }
            dictMode as ::core::ffi::c_uint
                != ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
                || matchIndex >= dictLimit;
            let fresh3 = numMatches;
            numMatches = numMatches.wrapping_add(1);
            matchBuffer[fresh3 as usize] = matchIndex;
            nbAttempts = nbAttempts.wrapping_sub(1);
        }
        matches = (matches as ::core::ffi::c_ulong
            & matches.wrapping_sub(1 as ZSTD_VecMask) as ::core::ffi::c_ulong)
            as ZSTD_VecMask;
    }
    let pos: U32 = ZSTD_row_nextIndex(tagRow, rowMask) as U32;
    *tagRow.offset(pos as isize) = tag as BYTE;
    let fresh4 = (*ms).nextToUpdate;
    (*ms).nextToUpdate = (*ms).nextToUpdate.wrapping_add(1);
    *row.offset(pos as isize) = fresh4;
    while currMatch < numMatches {
        let matchIndex_0: U32 = matchBuffer[currMatch as usize];
        let mut currentMl: size_t = 0 as size_t;
        if dictMode as ::core::ffi::c_uint
            != ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
            || matchIndex_0 >= dictLimit
        {
            let match_0: *const BYTE = base.offset(matchIndex_0 as isize);
            if MEM_read32(
                match_0
                    .offset(ml as isize)
                    .offset(-(3 as ::core::ffi::c_int as isize))
                    as *const ::core::ffi::c_void,
            ) == MEM_read32(
                ip.offset(ml as isize)
                    .offset(-(3 as ::core::ffi::c_int as isize))
                    as *const ::core::ffi::c_void,
            ) {
                currentMl = ZSTD_count(ip, match_0, iLimit);
            }
        } else {
            let match_1: *const BYTE = dictBase.offset(matchIndex_0 as isize);
            if MEM_read32(match_1 as *const ::core::ffi::c_void)
                == MEM_read32(ip as *const ::core::ffi::c_void)
            {
                currentMl = ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    match_1.offset(4 as ::core::ffi::c_int as isize),
                    iLimit,
                    dictEnd,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
            }
        }
        if currentMl > ml {
            ml = currentMl;
            *offsetPtr = (curr as ::core::ffi::c_uint)
                .wrapping_sub(matchIndex_0 as ::core::ffi::c_uint)
                .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                as size_t;
            if ip.offset(currentMl as isize) == iLimit {
                break;
            }
        }
        currMatch = currMatch.wrapping_add(1);
    }
    if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ml = ZSTD_dedicatedDictSearch_lazy_search(
            offsetPtr,
            ml,
            nbAttempts.wrapping_add(ddsExtraAttempts),
            dms,
            ip,
            iLimit,
            prefixStart,
            curr,
            dictLimit,
            ddsIdx,
        );
    } else if dictMode as ::core::ffi::c_uint
        == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let dmsLowestIndex: U32 = (*dms).window.dictLimit;
        let dmsBase: *const BYTE = (*dms).window.base;
        let dmsEnd: *const BYTE = (*dms).window.nextSrc;
        let dmsSize: U32 = dmsEnd.offset_from(dmsBase) as ::core::ffi::c_long as U32;
        let dmsIndexDelta: U32 = dictLimit.wrapping_sub(dmsSize);
        let headGrouped_0: U32 = (*dmsTagRow as U32 & rowMask).wrapping_mul(groupWidth);
        let mut matchBuffer_0: [U32; 64] = [0; 64];
        let mut numMatches_0: size_t = 0 as size_t;
        let mut currMatch_0: size_t = 0 as size_t;
        let mut matches_0: ZSTD_VecMask =
            ZSTD_row_getMatchMask(dmsTagRow, dmsTag as BYTE, headGrouped_0, rowEntries);
        while matches_0 > 0 as ZSTD_VecMask && nbAttempts > 0 as U32 {
            let matchPos_0: U32 = headGrouped_0
                .wrapping_add(ZSTD_VecMask_next(matches_0) as U32)
                .wrapping_div(groupWidth)
                & rowMask;
            let matchIndex_1: U32 = *dmsRow.offset(matchPos_0 as isize);
            if !(matchPos_0 == 0 as U32) {
                if matchIndex_1 < dmsLowestIndex {
                    break;
                }
                let fresh5 = numMatches_0;
                numMatches_0 = numMatches_0.wrapping_add(1);
                matchBuffer_0[fresh5 as usize] = matchIndex_1;
                nbAttempts = nbAttempts.wrapping_sub(1);
            }
            matches_0 = (matches_0 as ::core::ffi::c_ulong
                & matches_0.wrapping_sub(1 as ZSTD_VecMask) as ::core::ffi::c_ulong)
                as ZSTD_VecMask;
        }
        while currMatch_0 < numMatches_0 {
            let matchIndex_2: U32 = matchBuffer_0[currMatch_0 as usize];
            let mut currentMl_0: size_t = 0 as size_t;
            let match_2: *const BYTE = dmsBase.offset(matchIndex_2 as isize);
            if MEM_read32(match_2 as *const ::core::ffi::c_void)
                == MEM_read32(ip as *const ::core::ffi::c_void)
            {
                currentMl_0 = ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    match_2.offset(4 as ::core::ffi::c_int as isize),
                    iLimit,
                    dmsEnd,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
            }
            if currentMl_0 > ml {
                ml = currentMl_0;
                *offsetPtr = (curr as ::core::ffi::c_uint)
                    .wrapping_sub(
                        (matchIndex_2 as ::core::ffi::c_uint)
                            .wrapping_add(dmsIndexDelta as ::core::ffi::c_uint),
                    )
                    .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                    as size_t;
                if ip.offset(currentMl_0 as isize) == iLimit {
                    break;
                }
            }
            currMatch_0 = currMatch_0.wrapping_add(1);
        }
    }
    return ml;
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_5_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_noDict, 5 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_5_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dedicatedDictSearch,
        5 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_5_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dedicatedDictSearch,
        4 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_6_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dedicatedDictSearch,
        6 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_4_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dedicatedDictSearch,
        6 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_6_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dictMatchState,
        6 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_6_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dictMatchState,
        5 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_6_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dictMatchState,
        4 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_5_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dictMatchState,
        6 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_5_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dictMatchState,
        5 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_5_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dictMatchState,
        4 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_4_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dictMatchState,
        6 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_4_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dictMatchState,
        5 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dictMatchState_4_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dictMatchState,
        4 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_6_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_extDict, 6 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_6_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_extDict, 5 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_5_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dedicatedDictSearch,
        6 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_5_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_extDict, 6 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_5_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_extDict, 5 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_5_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_extDict, 4 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_4_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_extDict, 6 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_4_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_extDict, 5 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_4_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_extDict, 4 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_4_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dedicatedDictSearch,
        5 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_6_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_noDict, 6 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_6_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_noDict, 5 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_6_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_noDict, 4 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_5_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_noDict, 6 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_4_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dedicatedDictSearch,
        4 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_5_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_noDict, 4 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_4_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_noDict, 6 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_4_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_noDict, 5 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_noDict_4_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_noDict, 4 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_6_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dedicatedDictSearch,
        5 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_dedicatedDictSearch_6_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dedicatedDictSearch,
        4 as U32,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_RowFindBestMatch_extDict_6_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_RowFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_extDict, 4 as U32);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_extDict_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6 as U32, ZSTD_extDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_dictMatchState_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5 as U32, ZSTD_dictMatchState);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_dedicatedDictSearch_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(
        ms,
        ip,
        iLimit,
        offBasePtr,
        4 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_extDict_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4 as U32, ZSTD_extDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_dedicatedDictSearch_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(
        ms,
        ip,
        iLimit,
        offBasePtr,
        5 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_extDict_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5 as U32, ZSTD_extDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_dictMatchState_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6 as U32, ZSTD_dictMatchState);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_dictMatchState_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4 as U32, ZSTD_dictMatchState);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_noDict_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 5 as U32, ZSTD_noDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_noDict_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 4 as U32, ZSTD_noDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_dedicatedDictSearch_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(
        ms,
        ip,
        iLimit,
        offBasePtr,
        6 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_BtFindBestMatch_noDict_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offBasePtr: *mut size_t,
) -> size_t {
    return ZSTD_BtFindBestMatch(ms, ip, iLimit, offBasePtr, 6 as U32, ZSTD_noDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_extDict_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_extDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_dedicatedDictSearch_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        6 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_extDict_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_extDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_extDict_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_extDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_dictMatchState_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_dictMatchState);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_dictMatchState_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_dictMatchState);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_dictMatchState_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_dictMatchState);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_noDict_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 6 as U32, ZSTD_noDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_noDict_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 5 as U32, ZSTD_noDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_noDict_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(ms, ip, iLimit, offsetPtr, 4 as U32, ZSTD_noDict);
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_dedicatedDictSearch_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        5 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[inline(never)]
unsafe extern "C" fn ZSTD_HcFindBestMatch_dedicatedDictSearch_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    iLimit: *const BYTE,
    mut offsetPtr: *mut size_t,
) -> size_t {
    return ZSTD_HcFindBestMatch(
        ms,
        ip,
        iLimit,
        offsetPtr,
        4 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[inline(always)]
unsafe extern "C" fn ZSTD_searchMax(
    mut ms: *mut ZSTD_MatchState_t,
    mut ip: *const BYTE,
    mut iend: *const BYTE,
    mut offsetPtr: *mut size_t,
    mls: U32,
    rowLog: U32,
    searchMethod: searchMethod_e,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    if dictMode as ::core::ffi::c_uint == ZSTD_noDict as ::core::ffi::c_int as ::core::ffi::c_uint {
        match searchMethod as ::core::ffi::c_uint {
            0 => match mls {
                4 => return ZSTD_HcFindBestMatch_noDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_HcFindBestMatch_noDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_HcFindBestMatch_noDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            1 => match mls {
                4 => return ZSTD_BtFindBestMatch_noDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_BtFindBestMatch_noDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_BtFindBestMatch_noDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            2 => match mls {
                4 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_noDict_4_4(ms, ip, iend, offsetPtr);
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_noDict_4_5(ms, ip, iend, offsetPtr);
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_noDict_4_6(ms, ip, iend, offsetPtr);
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                5 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_noDict_5_4(ms, ip, iend, offsetPtr);
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_noDict_5_5(ms, ip, iend, offsetPtr);
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_noDict_5_6(ms, ip, iend, offsetPtr);
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                6 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_noDict_6_4(ms, ip, iend, offsetPtr);
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_noDict_6_5(ms, ip, iend, offsetPtr);
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_noDict_6_6(ms, ip, iend, offsetPtr);
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    } else if dictMode as ::core::ffi::c_uint
        == ZSTD_extDict as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        match searchMethod as ::core::ffi::c_uint {
            0 => match mls {
                4 => return ZSTD_HcFindBestMatch_extDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_HcFindBestMatch_extDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_HcFindBestMatch_extDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            1 => match mls {
                4 => return ZSTD_BtFindBestMatch_extDict_4(ms, ip, iend, offsetPtr),
                5 => return ZSTD_BtFindBestMatch_extDict_5(ms, ip, iend, offsetPtr),
                6 => return ZSTD_BtFindBestMatch_extDict_6(ms, ip, iend, offsetPtr),
                _ => {}
            },
            2 => match mls {
                4 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_extDict_4_4(ms, ip, iend, offsetPtr);
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_extDict_4_5(ms, ip, iend, offsetPtr);
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_extDict_4_6(ms, ip, iend, offsetPtr);
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                5 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_extDict_5_4(ms, ip, iend, offsetPtr);
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_extDict_5_5(ms, ip, iend, offsetPtr);
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_extDict_5_6(ms, ip, iend, offsetPtr);
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                6 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_extDict_6_4(ms, ip, iend, offsetPtr);
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_extDict_6_5(ms, ip, iend, offsetPtr);
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_extDict_6_6(ms, ip, iend, offsetPtr);
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    } else if dictMode as ::core::ffi::c_uint
        == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        match searchMethod as ::core::ffi::c_uint {
            0 => match mls {
                4 => {
                    return ZSTD_HcFindBestMatch_dictMatchState_4(ms, ip, iend, offsetPtr);
                }
                5 => {
                    return ZSTD_HcFindBestMatch_dictMatchState_5(ms, ip, iend, offsetPtr);
                }
                6 => {
                    return ZSTD_HcFindBestMatch_dictMatchState_6(ms, ip, iend, offsetPtr);
                }
                _ => {}
            },
            1 => match mls {
                4 => {
                    return ZSTD_BtFindBestMatch_dictMatchState_4(ms, ip, iend, offsetPtr);
                }
                5 => {
                    return ZSTD_BtFindBestMatch_dictMatchState_5(ms, ip, iend, offsetPtr);
                }
                6 => {
                    return ZSTD_BtFindBestMatch_dictMatchState_6(ms, ip, iend, offsetPtr);
                }
                _ => {}
            },
            2 => match mls {
                4 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_4_4(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_4_5(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_4_6(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                5 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_5_4(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_5_5(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_5_6(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                6 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_6_4(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_6_5(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_dictMatchState_6_6(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    } else if dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        match searchMethod as ::core::ffi::c_uint {
            0 => match mls {
                4 => {
                    return ZSTD_HcFindBestMatch_dedicatedDictSearch_4(ms, ip, iend, offsetPtr);
                }
                5 => {
                    return ZSTD_HcFindBestMatch_dedicatedDictSearch_5(ms, ip, iend, offsetPtr);
                }
                6 => {
                    return ZSTD_HcFindBestMatch_dedicatedDictSearch_6(ms, ip, iend, offsetPtr);
                }
                _ => {}
            },
            1 => match mls {
                4 => {
                    return ZSTD_BtFindBestMatch_dedicatedDictSearch_4(ms, ip, iend, offsetPtr);
                }
                5 => {
                    return ZSTD_BtFindBestMatch_dedicatedDictSearch_5(ms, ip, iend, offsetPtr);
                }
                6 => {
                    return ZSTD_BtFindBestMatch_dedicatedDictSearch_6(ms, ip, iend, offsetPtr);
                }
                _ => {}
            },
            2 => match mls {
                4 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_4_4(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_4_5(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_4_6(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                5 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_5_4(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_5_5(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_5_6(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                6 => {
                    match rowLog {
                        4 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_6_4(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        5 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_6_5(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        6 => {
                            return ZSTD_RowFindBestMatch_dedicatedDictSearch_6_6(
                                ms, ip, iend, offsetPtr,
                            );
                        }
                        _ => {}
                    }
                    unreachable!();
                }
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    }
    unreachable!();
    return 0 as size_t;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_compressBlock_lazy_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    searchMethod: searchMethod_e,
    depth: U32,
    dictMode: ZSTD_dictMode_e,
) -> size_t {
    let mut current_block: u64;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = if searchMethod as ::core::ffi::c_uint
        == search_rowHash as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        iend.offset(-(8 as ::core::ffi::c_int as isize))
            .offset(-(ZSTD_ROW_HASH_CACHE_SIZE as isize))
    } else {
        iend.offset(-(8 as ::core::ffi::c_int as isize))
    };
    let base: *const BYTE = (*ms).window.base;
    let prefixLowestIndex: U32 = (*ms).window.dictLimit;
    let prefixLowest: *const BYTE = base.offset(prefixLowestIndex as isize);
    let mls: U32 = if 4 as ::core::ffi::c_uint
        > (if (*ms).cParams.minMatch < 6 as ::core::ffi::c_uint {
            (*ms).cParams.minMatch
        } else {
            6 as ::core::ffi::c_uint
        }) {
        4 as U32
    } else if (*ms).cParams.minMatch < 6 as ::core::ffi::c_uint {
        (*ms).cParams.minMatch as U32
    } else {
        6 as U32
    };
    let rowLog: U32 = if 4 as ::core::ffi::c_uint
        > (if (*ms).cParams.searchLog < 6 as ::core::ffi::c_uint {
            (*ms).cParams.searchLog
        } else {
            6 as ::core::ffi::c_uint
        }) {
        4 as U32
    } else if (*ms).cParams.searchLog < 6 as ::core::ffi::c_uint {
        (*ms).cParams.searchLog as U32
    } else {
        6 as U32
    };
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    let mut offsetSaved1: U32 = 0 as U32;
    let mut offsetSaved2: U32 = 0 as U32;
    let isDMS: ::core::ffi::c_int = (dictMode as ::core::ffi::c_uint
        == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
    let isDDS: ::core::ffi::c_int = (dictMode as ::core::ffi::c_uint
        == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
    let isDxS: ::core::ffi::c_int = (isDMS != 0 || isDDS != 0) as ::core::ffi::c_int;
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictLowestIndex: U32 = if isDxS != 0 {
        (*dms).window.dictLimit
    } else {
        0 as U32
    };
    let dictBase: *const BYTE = if isDxS != 0 {
        (*dms).window.base
    } else {
        ::core::ptr::null::<BYTE>()
    };
    let dictLowest: *const BYTE = if isDxS != 0 {
        dictBase.offset(dictLowestIndex as isize)
    } else {
        ::core::ptr::null::<BYTE>()
    };
    let dictEnd: *const BYTE = if isDxS != 0 {
        (*dms).window.nextSrc
    } else {
        ::core::ptr::null::<BYTE>()
    };
    let dictIndexDelta: U32 = if isDxS != 0 {
        prefixLowestIndex.wrapping_sub(dictEnd.offset_from(dictBase) as ::core::ffi::c_long as U32)
    } else {
        0 as U32
    };
    let dictAndPrefixLength: U32 = (ip.offset_from(prefixLowest) as ::core::ffi::c_long
        + dictEnd.offset_from(dictLowest) as ::core::ffi::c_long)
        as U32;
    ip = ip.offset((dictAndPrefixLength == 0 as U32) as ::core::ffi::c_int as isize);
    if dictMode as ::core::ffi::c_uint == ZSTD_noDict as ::core::ffi::c_int as ::core::ffi::c_uint {
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, curr, (*ms).cParams.windowLog) as U32;
        let maxRep: U32 = curr.wrapping_sub(windowLow);
        if offset_2 > maxRep {
            offsetSaved2 = offset_2;
            offset_2 = 0 as U32;
        }
        if offset_1 > maxRep {
            offsetSaved1 = offset_1;
            offset_1 = 0 as U32;
        }
    }
    isDxS != 0;
    (*ms).lazySkipping = 0 as ::core::ffi::c_int;
    if searchMethod as ::core::ffi::c_uint
        == search_rowHash as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }
    asm!(".p2align 5\n", options(preserves_flags, att_syntax));
    while ip < ilimit {
        let mut matchLength: size_t = 0 as size_t;
        let mut offBase: size_t = REPCODE1_TO_OFFBASE as size_t;
        let mut start: *const BYTE = ip.offset(1 as ::core::ffi::c_int as isize);
        if isDxS != 0 {
            let repIndex: U32 = (ip.offset_from(base) as ::core::ffi::c_long as U32)
                .wrapping_add(1 as U32)
                .wrapping_sub(offset_1);
            let mut repMatch: *const BYTE = if (dictMode as ::core::ffi::c_uint
                == ZSTD_dictMatchState as ::core::ffi::c_int as ::core::ffi::c_uint
                || dictMode as ::core::ffi::c_uint
                    == ZSTD_dedicatedDictSearch as ::core::ffi::c_int as ::core::ffi::c_uint)
                && repIndex < prefixLowestIndex
            {
                dictBase.offset(repIndex.wrapping_sub(dictIndexDelta) as isize)
            } else {
                base.offset(repIndex as isize)
            };
            if ZSTD_index_overlap_check(prefixLowestIndex, repIndex) != 0
                && MEM_read32(repMatch as *const ::core::ffi::c_void)
                    == MEM_read32(
                        ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void
                    )
            {
                let mut repMatchEnd: *const BYTE = if repIndex < prefixLowestIndex {
                    dictEnd
                } else {
                    iend
                };
                matchLength = ZSTD_count_2segments(
                    ip.offset(1 as ::core::ffi::c_int as isize)
                        .offset(4 as ::core::ffi::c_int as isize),
                    repMatch.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repMatchEnd,
                    prefixLowest,
                )
                .wrapping_add(4 as size_t);
                if depth == 0 as U32 {
                    current_block = 5207103047907367337;
                } else {
                    current_block = 14136749492126903395;
                }
            } else {
                current_block = 14136749492126903395;
            }
        } else {
            current_block = 14136749492126903395;
        }
        match current_block {
            14136749492126903395 => {
                if dictMode as ::core::ffi::c_uint
                    == ZSTD_noDict as ::core::ffi::c_int as ::core::ffi::c_uint
                    && (offset_1 > 0 as U32) as ::core::ffi::c_int
                        & (MEM_read32(
                            ip.offset(1 as ::core::ffi::c_int as isize)
                                .offset(-(offset_1 as isize))
                                as *const ::core::ffi::c_void,
                        ) == MEM_read32(ip.offset(1 as ::core::ffi::c_int as isize)
                            as *const ::core::ffi::c_void))
                            as ::core::ffi::c_int
                        != 0
                {
                    matchLength = ZSTD_count(
                        ip.offset(1 as ::core::ffi::c_int as isize)
                            .offset(4 as ::core::ffi::c_int as isize),
                        ip.offset(1 as ::core::ffi::c_int as isize)
                            .offset(4 as ::core::ffi::c_int as isize)
                            .offset(-(offset_1 as isize)),
                        iend,
                    )
                    .wrapping_add(4 as size_t);
                    if depth == 0 as U32 {
                        current_block = 5207103047907367337;
                    } else {
                        current_block = 6450636197030046351;
                    }
                } else {
                    current_block = 6450636197030046351;
                }
                match current_block {
                    5207103047907367337 => {}
                    _ => {
                        let mut offbaseFound: size_t = 999999999 as ::core::ffi::c_int as size_t;
                        let ml2: size_t = ZSTD_searchMax(
                            ms,
                            ip,
                            iend,
                            &raw mut offbaseFound,
                            mls,
                            rowLog,
                            searchMethod,
                            dictMode,
                        ) as size_t;
                        if ml2 > matchLength {
                            matchLength = ml2;
                            start = ip;
                            offBase = offbaseFound;
                        }
                        if matchLength < 4 as size_t {
                            let step: size_t = (ip.offset_from(anchor) as ::core::ffi::c_long
                                as size_t
                                >> kSearchStrength)
                                .wrapping_add(1 as size_t);
                            ip = ip.offset(step as isize);
                            (*ms).lazySkipping =
                                (step > kLazySkippingStep as size_t) as ::core::ffi::c_int;
                            continue;
                        } else {
                            if depth >= 1 as U32 {
                                while ip < ilimit {
                                    ip = ip.offset(1);
                                    if dictMode as ::core::ffi::c_uint
                                        == ZSTD_noDict as ::core::ffi::c_int as ::core::ffi::c_uint
                                        && offBase != 0
                                        && (offset_1 > 0 as U32) as ::core::ffi::c_int
                                            & (MEM_read32(ip as *const ::core::ffi::c_void)
                                                == MEM_read32(ip.offset(-(offset_1 as isize))
                                                    as *const ::core::ffi::c_void))
                                                as ::core::ffi::c_int
                                            != 0
                                    {
                                        let mlRep: size_t = (ZSTD_count(
                                            ip.offset(4 as ::core::ffi::c_int as isize),
                                            ip.offset(4 as ::core::ffi::c_int as isize)
                                                .offset(-(offset_1 as isize)),
                                            iend,
                                        )
                                            as size_t)
                                            .wrapping_add(4 as size_t);
                                        let gain2: ::core::ffi::c_int =
                                            mlRep.wrapping_mul(3 as size_t) as ::core::ffi::c_int;
                                        let gain1: ::core::ffi::c_int = matchLength
                                            .wrapping_mul(3 as size_t)
                                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as size_t)
                                            .wrapping_add(1 as size_t)
                                            as ::core::ffi::c_int;
                                        if mlRep >= 4 as size_t && gain2 > gain1 {
                                            matchLength = mlRep;
                                            offBase = REPCODE1_TO_OFFBASE as size_t;
                                            start = ip;
                                        }
                                    }
                                    if isDxS != 0 {
                                        let repIndex_0: U32 =
                                            (ip.offset_from(base) as ::core::ffi::c_long as U32)
                                                .wrapping_sub(offset_1);
                                        let mut repMatch_0: *const BYTE =
                                            if repIndex_0 < prefixLowestIndex {
                                                dictBase
                                                    .offset(repIndex_0.wrapping_sub(dictIndexDelta)
                                                        as isize)
                                            } else {
                                                base.offset(repIndex_0 as isize)
                                            };
                                        if ZSTD_index_overlap_check(prefixLowestIndex, repIndex_0)
                                            != 0
                                            && MEM_read32(repMatch_0 as *const ::core::ffi::c_void)
                                                == MEM_read32(ip as *const ::core::ffi::c_void)
                                        {
                                            let mut repMatchEnd_0: *const BYTE =
                                                if repIndex_0 < prefixLowestIndex {
                                                    dictEnd
                                                } else {
                                                    iend
                                                };
                                            let mlRep_0: size_t = (ZSTD_count_2segments(
                                                ip.offset(4 as ::core::ffi::c_int as isize),
                                                repMatch_0.offset(4 as ::core::ffi::c_int as isize),
                                                iend,
                                                repMatchEnd_0,
                                                prefixLowest,
                                            )
                                                as size_t)
                                                .wrapping_add(4 as size_t);
                                            let gain2_0: ::core::ffi::c_int = mlRep_0
                                                .wrapping_mul(3 as size_t)
                                                as ::core::ffi::c_int;
                                            let gain1_0: ::core::ffi::c_int = matchLength
                                                .wrapping_mul(3 as size_t)
                                                .wrapping_sub(
                                                    ZSTD_highbit32(offBase as U32) as size_t
                                                )
                                                .wrapping_add(1 as size_t)
                                                as ::core::ffi::c_int;
                                            if mlRep_0 >= 4 as size_t && gain2_0 > gain1_0 {
                                                matchLength = mlRep_0;
                                                offBase = REPCODE1_TO_OFFBASE as size_t;
                                                start = ip;
                                            }
                                        }
                                    }
                                    let mut ofbCandidate: size_t =
                                        999999999 as ::core::ffi::c_int as size_t;
                                    let ml2_0: size_t = ZSTD_searchMax(
                                        ms,
                                        ip,
                                        iend,
                                        &raw mut ofbCandidate,
                                        mls,
                                        rowLog,
                                        searchMethod,
                                        dictMode,
                                    )
                                        as size_t;
                                    let gain2_1: ::core::ffi::c_int = ml2_0
                                        .wrapping_mul(4 as size_t)
                                        .wrapping_sub(ZSTD_highbit32(ofbCandidate as U32) as size_t)
                                        as ::core::ffi::c_int;
                                    let gain1_1: ::core::ffi::c_int = matchLength
                                        .wrapping_mul(4 as size_t)
                                        .wrapping_sub(ZSTD_highbit32(offBase as U32) as size_t)
                                        .wrapping_add(4 as size_t)
                                        as ::core::ffi::c_int;
                                    if ml2_0 >= 4 as size_t && gain2_1 > gain1_1 {
                                        matchLength = ml2_0;
                                        offBase = ofbCandidate;
                                        start = ip;
                                    } else {
                                        if !(depth == 2 as U32 && ip < ilimit) {
                                            break;
                                        }
                                        ip = ip.offset(1);
                                        if dictMode as ::core::ffi::c_uint
                                            == ZSTD_noDict as ::core::ffi::c_int
                                                as ::core::ffi::c_uint
                                            && offBase != 0
                                            && (offset_1 > 0 as U32) as ::core::ffi::c_int
                                                & (MEM_read32(ip as *const ::core::ffi::c_void)
                                                    == MEM_read32(ip.offset(-(offset_1 as isize))
                                                        as *const ::core::ffi::c_void))
                                                    as ::core::ffi::c_int
                                                != 0
                                        {
                                            let mlRep_1: size_t = (ZSTD_count(
                                                ip.offset(4 as ::core::ffi::c_int as isize),
                                                ip.offset(4 as ::core::ffi::c_int as isize)
                                                    .offset(-(offset_1 as isize)),
                                                iend,
                                            )
                                                as size_t)
                                                .wrapping_add(4 as size_t);
                                            let gain2_2: ::core::ffi::c_int = mlRep_1
                                                .wrapping_mul(4 as size_t)
                                                as ::core::ffi::c_int;
                                            let gain1_2: ::core::ffi::c_int = matchLength
                                                .wrapping_mul(4 as size_t)
                                                .wrapping_sub(
                                                    ZSTD_highbit32(offBase as U32) as size_t
                                                )
                                                .wrapping_add(1 as size_t)
                                                as ::core::ffi::c_int;
                                            if mlRep_1 >= 4 as size_t && gain2_2 > gain1_2 {
                                                matchLength = mlRep_1;
                                                offBase = REPCODE1_TO_OFFBASE as size_t;
                                                start = ip;
                                            }
                                        }
                                        if isDxS != 0 {
                                            let repIndex_1: U32 = (ip.offset_from(base)
                                                as ::core::ffi::c_long
                                                as U32)
                                                .wrapping_sub(offset_1);
                                            let mut repMatch_1: *const BYTE = if repIndex_1
                                                < prefixLowestIndex
                                            {
                                                dictBase
                                                    .offset(repIndex_1.wrapping_sub(dictIndexDelta)
                                                        as isize)
                                            } else {
                                                base.offset(repIndex_1 as isize)
                                            };
                                            if ZSTD_index_overlap_check(
                                                prefixLowestIndex,
                                                repIndex_1,
                                            ) != 0
                                                && MEM_read32(
                                                    repMatch_1 as *const ::core::ffi::c_void,
                                                ) == MEM_read32(ip as *const ::core::ffi::c_void)
                                            {
                                                let mut repMatchEnd_1: *const BYTE =
                                                    if repIndex_1 < prefixLowestIndex {
                                                        dictEnd
                                                    } else {
                                                        iend
                                                    };
                                                let mlRep_2: size_t = (ZSTD_count_2segments(
                                                    ip.offset(4 as ::core::ffi::c_int as isize),
                                                    repMatch_1
                                                        .offset(4 as ::core::ffi::c_int as isize),
                                                    iend,
                                                    repMatchEnd_1,
                                                    prefixLowest,
                                                )
                                                    as size_t)
                                                    .wrapping_add(4 as size_t);
                                                let gain2_3: ::core::ffi::c_int = mlRep_2
                                                    .wrapping_mul(4 as size_t)
                                                    as ::core::ffi::c_int;
                                                let gain1_3: ::core::ffi::c_int = matchLength
                                                    .wrapping_mul(4 as size_t)
                                                    .wrapping_sub(
                                                        ZSTD_highbit32(offBase as U32) as size_t
                                                    )
                                                    .wrapping_add(1 as size_t)
                                                    as ::core::ffi::c_int;
                                                if mlRep_2 >= 4 as size_t && gain2_3 > gain1_3 {
                                                    matchLength = mlRep_2;
                                                    offBase = REPCODE1_TO_OFFBASE as size_t;
                                                    start = ip;
                                                }
                                            }
                                        }
                                        let mut ofbCandidate_0: size_t =
                                            999999999 as ::core::ffi::c_int as size_t;
                                        let ml2_1: size_t = ZSTD_searchMax(
                                            ms,
                                            ip,
                                            iend,
                                            &raw mut ofbCandidate_0,
                                            mls,
                                            rowLog,
                                            searchMethod,
                                            dictMode,
                                        )
                                            as size_t;
                                        let gain2_4: ::core::ffi::c_int =
                                            ml2_1.wrapping_mul(4 as size_t).wrapping_sub(
                                                ZSTD_highbit32(ofbCandidate_0 as U32) as size_t,
                                            )
                                                as ::core::ffi::c_int;
                                        let gain1_4: ::core::ffi::c_int = matchLength
                                            .wrapping_mul(4 as size_t)
                                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as size_t)
                                            .wrapping_add(7 as size_t)
                                            as ::core::ffi::c_int;
                                        if !(ml2_1 >= 4 as size_t && gain2_4 > gain1_4) {
                                            break;
                                        }
                                        matchLength = ml2_1;
                                        offBase = ofbCandidate_0;
                                        start = ip;
                                    }
                                }
                            }
                            if offBase > ZSTD_REP_NUM as size_t {
                                if dictMode as ::core::ffi::c_uint
                                    == ZSTD_noDict as ::core::ffi::c_int as ::core::ffi::c_uint
                                {
                                    while (start > anchor) as ::core::ffi::c_int
                                        & (start.offset(
                                            -((offBase as ::core::ffi::c_ulong)
                                                .wrapping_sub(ZSTD_REP_NUM as ::core::ffi::c_ulong)
                                                as isize),
                                        ) > prefixLowest)
                                            as ::core::ffi::c_int
                                        != 0
                                        && *start.offset(-(1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                            == *start
                                                .offset(
                                                    -((offBase as ::core::ffi::c_ulong)
                                                        .wrapping_sub(
                                                            ZSTD_REP_NUM as ::core::ffi::c_ulong,
                                                        )
                                                        as isize),
                                                )
                                                .offset(-(1 as ::core::ffi::c_int) as isize)
                                                as ::core::ffi::c_int
                                    {
                                        start = start.offset(-1);
                                        matchLength = matchLength.wrapping_add(1);
                                    }
                                }
                                if isDxS != 0 {
                                    let matchIndex: U32 = (start.offset_from(base)
                                        as ::core::ffi::c_long
                                        as size_t)
                                        .wrapping_sub(offBase.wrapping_sub(ZSTD_REP_NUM as size_t))
                                        as U32;
                                    let mut match_0: *const BYTE = if matchIndex < prefixLowestIndex
                                    {
                                        dictBase
                                            .offset(matchIndex as isize)
                                            .offset(-(dictIndexDelta as isize))
                                    } else {
                                        base.offset(matchIndex as isize)
                                    };
                                    let mStart: *const BYTE = if matchIndex < prefixLowestIndex {
                                        dictLowest
                                    } else {
                                        prefixLowest
                                    };
                                    while start > anchor
                                        && match_0 > mStart
                                        && *start.offset(-(1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                            == *match_0.offset(-(1 as ::core::ffi::c_int) as isize)
                                                as ::core::ffi::c_int
                                    {
                                        start = start.offset(-1);
                                        match_0 = match_0.offset(-1);
                                        matchLength = matchLength.wrapping_add(1);
                                    }
                                }
                                offset_2 = offset_1;
                                offset_1 = (offBase as ::core::ffi::c_ulong)
                                    .wrapping_sub(ZSTD_REP_NUM as ::core::ffi::c_ulong)
                                    as U32;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        let litLength: size_t = start.offset_from(anchor) as ::core::ffi::c_long as size_t;
        ZSTD_storeSeq(
            seqStore,
            litLength,
            anchor,
            iend,
            offBase as U32,
            matchLength,
        );
        ip = start.offset(matchLength as isize);
        anchor = ip;
        if (*ms).lazySkipping != 0 {
            if searchMethod as ::core::ffi::c_uint
                == search_rowHash as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
            }
            (*ms).lazySkipping = 0 as ::core::ffi::c_int;
        }
        if isDxS != 0 {
            while ip <= ilimit {
                let current2: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
                let repIndex_2: U32 = current2.wrapping_sub(offset_2);
                let mut repMatch_2: *const BYTE = if repIndex_2 < prefixLowestIndex {
                    dictBase
                        .offset(-(dictIndexDelta as isize))
                        .offset(repIndex_2 as isize)
                } else {
                    base.offset(repIndex_2 as isize)
                };
                if !(ZSTD_index_overlap_check(prefixLowestIndex, repIndex_2) != 0
                    && MEM_read32(repMatch_2 as *const ::core::ffi::c_void)
                        == MEM_read32(ip as *const ::core::ffi::c_void))
                {
                    break;
                }
                let repEnd2: *const BYTE = if repIndex_2 < prefixLowestIndex {
                    dictEnd
                } else {
                    iend
                };
                matchLength = ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    repMatch_2.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repEnd2,
                    prefixLowest,
                )
                .wrapping_add(4 as size_t);
                offBase = offset_2 as size_t;
                offset_2 = offset_1;
                offset_1 = offBase as U32;
                ZSTD_storeSeq(
                    seqStore,
                    0 as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE as U32,
                    matchLength,
                );
                ip = ip.offset(matchLength as isize);
                anchor = ip;
            }
        }
        if dictMode as ::core::ffi::c_uint
            == ZSTD_noDict as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            while (ip <= ilimit) as ::core::ffi::c_int & (offset_2 > 0 as U32) as ::core::ffi::c_int
                != 0
                && MEM_read32(ip as *const ::core::ffi::c_void)
                    == MEM_read32(ip.offset(-(offset_2 as isize)) as *const ::core::ffi::c_void)
            {
                matchLength = ZSTD_count(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    ip.offset(4 as ::core::ffi::c_int as isize)
                        .offset(-(offset_2 as isize)),
                    iend,
                )
                .wrapping_add(4 as size_t);
                offBase = offset_2 as size_t;
                offset_2 = offset_1;
                offset_1 = offBase as U32;
                ZSTD_storeSeq(
                    seqStore,
                    0 as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE as U32,
                    matchLength,
                );
                ip = ip.offset(matchLength as isize);
                anchor = ip;
            }
        }
    }
    offsetSaved2 = if offsetSaved1 != 0 as U32 && offset_1 != 0 as U32 {
        offsetSaved1
    } else {
        offsetSaved2
    };
    *rep.offset(0 as ::core::ffi::c_int as isize) = if offset_1 != 0 {
        offset_1
    } else {
        offsetSaved1
    };
    *rep.offset(1 as ::core::ffi::c_int as isize) = if offset_2 != 0 {
        offset_2
    } else {
        offsetSaved2
    };
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0 as U32,
        ZSTD_dictMatchState,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dictMatchState_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0 as U32,
        ZSTD_dictMatchState,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_dedicatedDictSearch_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1 as U32,
        ZSTD_dictMatchState,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dictMatchState_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1 as U32,
        ZSTD_dictMatchState,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_dedicatedDictSearch_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2 as U32,
        ZSTD_dictMatchState,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dictMatchState_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2 as U32,
        ZSTD_dictMatchState,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_dedicatedDictSearch_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2 as U32,
        ZSTD_dedicatedDictSearch,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_binaryTree,
        2 as U32,
        ZSTD_noDict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_dictMatchState(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_binaryTree,
        2 as U32,
        ZSTD_dictMatchState,
    );
}
#[inline(always)]
unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    searchMethod: searchMethod_e,
    depth: U32,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = if searchMethod as ::core::ffi::c_uint
        == search_rowHash as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        iend.offset(-(8 as ::core::ffi::c_int as isize))
            .offset(-(ZSTD_ROW_HASH_CACHE_SIZE as isize))
    } else {
        iend.offset(-(8 as ::core::ffi::c_int as isize))
    };
    let base: *const BYTE = (*ms).window.base;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.offset(dictLimit as isize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictEnd: *const BYTE = dictBase.offset(dictLimit as isize);
    let dictStart: *const BYTE = dictBase.offset((*ms).window.lowLimit as isize);
    let windowLog: U32 = (*ms).cParams.windowLog as U32;
    let mls: U32 = if 4 as ::core::ffi::c_uint
        > (if (*ms).cParams.minMatch < 6 as ::core::ffi::c_uint {
            (*ms).cParams.minMatch
        } else {
            6 as ::core::ffi::c_uint
        }) {
        4 as U32
    } else if (*ms).cParams.minMatch < 6 as ::core::ffi::c_uint {
        (*ms).cParams.minMatch as U32
    } else {
        6 as U32
    };
    let rowLog: U32 = if 4 as ::core::ffi::c_uint
        > (if (*ms).cParams.searchLog < 6 as ::core::ffi::c_uint {
            (*ms).cParams.searchLog
        } else {
            6 as ::core::ffi::c_uint
        }) {
        4 as U32
    } else if (*ms).cParams.searchLog < 6 as ::core::ffi::c_uint {
        (*ms).cParams.searchLog as U32
    } else {
        6 as U32
    };
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    (*ms).lazySkipping = 0 as ::core::ffi::c_int;
    ip = ip.offset((ip == prefixStart) as ::core::ffi::c_int as isize);
    if searchMethod as ::core::ffi::c_uint
        == search_rowHash as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
    }
    asm!(".p2align 5\n", options(preserves_flags, att_syntax));
    let mut current_block_61: u64;
    while ip < ilimit {
        let mut matchLength: size_t = 0 as size_t;
        let mut offBase: size_t = REPCODE1_TO_OFFBASE as size_t;
        let mut start: *const BYTE = ip.offset(1 as ::core::ffi::c_int as isize);
        let mut curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let windowLow: U32 = ZSTD_getLowestMatchIndex(
            ms,
            curr.wrapping_add(1 as U32),
            windowLog as ::core::ffi::c_uint,
        ) as U32;
        let repIndex: U32 = curr.wrapping_add(1 as U32).wrapping_sub(offset_1);
        let repBase: *const BYTE = if repIndex < dictLimit { dictBase } else { base };
        let repMatch: *const BYTE = repBase.offset(repIndex as isize);
        if ZSTD_index_overlap_check(dictLimit, repIndex)
            & (offset_1 <= curr.wrapping_add(1 as U32).wrapping_sub(windowLow))
                as ::core::ffi::c_int
            != 0
        {
            if MEM_read32(ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void)
                == MEM_read32(repMatch as *const ::core::ffi::c_void)
            {
                let repEnd: *const BYTE = if repIndex < dictLimit { dictEnd } else { iend };
                matchLength = ZSTD_count_2segments(
                    ip.offset(1 as ::core::ffi::c_int as isize)
                        .offset(4 as ::core::ffi::c_int as isize),
                    repMatch.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repEnd,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
                if depth == 0 as U32 {
                    current_block_61 = 14866757260291549646;
                } else {
                    current_block_61 = 12147880666119273379;
                }
            } else {
                current_block_61 = 12147880666119273379;
            }
        } else {
            current_block_61 = 12147880666119273379;
        }
        match current_block_61 {
            12147880666119273379 => {
                let mut ofbCandidate: size_t = 999999999 as ::core::ffi::c_int as size_t;
                let ml2: size_t = ZSTD_searchMax(
                    ms,
                    ip,
                    iend,
                    &raw mut ofbCandidate,
                    mls,
                    rowLog,
                    searchMethod,
                    ZSTD_extDict,
                ) as size_t;
                if ml2 > matchLength {
                    matchLength = ml2;
                    start = ip;
                    offBase = ofbCandidate;
                }
                if matchLength < 4 as size_t {
                    let step: size_t =
                        ip.offset_from(anchor) as ::core::ffi::c_long as size_t >> kSearchStrength;
                    ip = ip.offset(step.wrapping_add(1 as size_t) as isize);
                    (*ms).lazySkipping = (step > kLazySkippingStep as size_t) as ::core::ffi::c_int;
                    continue;
                } else {
                    if depth >= 1 as U32 {
                        while ip < ilimit {
                            ip = ip.offset(1);
                            curr = curr.wrapping_add(1);
                            if offBase != 0 {
                                let windowLow_0: U32 = ZSTD_getLowestMatchIndex(
                                    ms,
                                    curr,
                                    windowLog as ::core::ffi::c_uint,
                                ) as U32;
                                let repIndex_0: U32 = curr.wrapping_sub(offset_1);
                                let repBase_0: *const BYTE = if repIndex_0 < dictLimit {
                                    dictBase
                                } else {
                                    base
                                };
                                let repMatch_0: *const BYTE = repBase_0.offset(repIndex_0 as isize);
                                if ZSTD_index_overlap_check(dictLimit, repIndex_0)
                                    & (offset_1 <= curr.wrapping_sub(windowLow_0))
                                        as ::core::ffi::c_int
                                    != 0
                                {
                                    if MEM_read32(ip as *const ::core::ffi::c_void)
                                        == MEM_read32(repMatch_0 as *const ::core::ffi::c_void)
                                    {
                                        let repEnd_0: *const BYTE = if repIndex_0 < dictLimit {
                                            dictEnd
                                        } else {
                                            iend
                                        };
                                        let repLength: size_t = (ZSTD_count_2segments(
                                            ip.offset(4 as ::core::ffi::c_int as isize),
                                            repMatch_0.offset(4 as ::core::ffi::c_int as isize),
                                            iend,
                                            repEnd_0,
                                            prefixStart,
                                        )
                                            as size_t)
                                            .wrapping_add(4 as size_t);
                                        let gain2: ::core::ffi::c_int = repLength
                                            .wrapping_mul(3 as size_t)
                                            as ::core::ffi::c_int;
                                        let gain1: ::core::ffi::c_int = matchLength
                                            .wrapping_mul(3 as size_t)
                                            .wrapping_sub(ZSTD_highbit32(offBase as U32) as size_t)
                                            .wrapping_add(1 as size_t)
                                            as ::core::ffi::c_int;
                                        if repLength >= 4 as size_t && gain2 > gain1 {
                                            matchLength = repLength;
                                            offBase = REPCODE1_TO_OFFBASE as size_t;
                                            start = ip;
                                        }
                                    }
                                }
                            }
                            let mut ofbCandidate_0: size_t =
                                999999999 as ::core::ffi::c_int as size_t;
                            let ml2_0: size_t = ZSTD_searchMax(
                                ms,
                                ip,
                                iend,
                                &raw mut ofbCandidate_0,
                                mls,
                                rowLog,
                                searchMethod,
                                ZSTD_extDict,
                            ) as size_t;
                            let gain2_0: ::core::ffi::c_int = ml2_0
                                .wrapping_mul(4 as size_t)
                                .wrapping_sub(ZSTD_highbit32(ofbCandidate_0 as U32) as size_t)
                                as ::core::ffi::c_int;
                            let gain1_0: ::core::ffi::c_int = matchLength
                                .wrapping_mul(4 as size_t)
                                .wrapping_sub(ZSTD_highbit32(offBase as U32) as size_t)
                                .wrapping_add(4 as size_t)
                                as ::core::ffi::c_int;
                            if ml2_0 >= 4 as size_t && gain2_0 > gain1_0 {
                                matchLength = ml2_0;
                                offBase = ofbCandidate_0;
                                start = ip;
                            } else {
                                if !(depth == 2 as U32 && ip < ilimit) {
                                    break;
                                }
                                ip = ip.offset(1);
                                curr = curr.wrapping_add(1);
                                if offBase != 0 {
                                    let windowLow_1: U32 = ZSTD_getLowestMatchIndex(
                                        ms,
                                        curr,
                                        windowLog as ::core::ffi::c_uint,
                                    )
                                        as U32;
                                    let repIndex_1: U32 = curr.wrapping_sub(offset_1);
                                    let repBase_1: *const BYTE = if repIndex_1 < dictLimit {
                                        dictBase
                                    } else {
                                        base
                                    };
                                    let repMatch_1: *const BYTE =
                                        repBase_1.offset(repIndex_1 as isize);
                                    if ZSTD_index_overlap_check(dictLimit, repIndex_1)
                                        & (offset_1 <= curr.wrapping_sub(windowLow_1))
                                            as ::core::ffi::c_int
                                        != 0
                                    {
                                        if MEM_read32(ip as *const ::core::ffi::c_void)
                                            == MEM_read32(repMatch_1 as *const ::core::ffi::c_void)
                                        {
                                            let repEnd_1: *const BYTE = if repIndex_1 < dictLimit {
                                                dictEnd
                                            } else {
                                                iend
                                            };
                                            let repLength_0: size_t = (ZSTD_count_2segments(
                                                ip.offset(4 as ::core::ffi::c_int as isize),
                                                repMatch_1.offset(4 as ::core::ffi::c_int as isize),
                                                iend,
                                                repEnd_1,
                                                prefixStart,
                                            )
                                                as size_t)
                                                .wrapping_add(4 as size_t);
                                            let gain2_1: ::core::ffi::c_int = repLength_0
                                                .wrapping_mul(4 as size_t)
                                                as ::core::ffi::c_int;
                                            let gain1_1: ::core::ffi::c_int = matchLength
                                                .wrapping_mul(4 as size_t)
                                                .wrapping_sub(
                                                    ZSTD_highbit32(offBase as U32) as size_t
                                                )
                                                .wrapping_add(1 as size_t)
                                                as ::core::ffi::c_int;
                                            if repLength_0 >= 4 as size_t && gain2_1 > gain1_1 {
                                                matchLength = repLength_0;
                                                offBase = REPCODE1_TO_OFFBASE as size_t;
                                                start = ip;
                                            }
                                        }
                                    }
                                }
                                let mut ofbCandidate_1: size_t =
                                    999999999 as ::core::ffi::c_int as size_t;
                                let ml2_1: size_t = ZSTD_searchMax(
                                    ms,
                                    ip,
                                    iend,
                                    &raw mut ofbCandidate_1,
                                    mls,
                                    rowLog,
                                    searchMethod,
                                    ZSTD_extDict,
                                ) as size_t;
                                let gain2_2: ::core::ffi::c_int = ml2_1
                                    .wrapping_mul(4 as size_t)
                                    .wrapping_sub(ZSTD_highbit32(ofbCandidate_1 as U32) as size_t)
                                    as ::core::ffi::c_int;
                                let gain1_2: ::core::ffi::c_int = matchLength
                                    .wrapping_mul(4 as size_t)
                                    .wrapping_sub(ZSTD_highbit32(offBase as U32) as size_t)
                                    .wrapping_add(7 as size_t)
                                    as ::core::ffi::c_int;
                                if !(ml2_1 >= 4 as size_t && gain2_2 > gain1_2) {
                                    break;
                                }
                                matchLength = ml2_1;
                                offBase = ofbCandidate_1;
                                start = ip;
                            }
                        }
                    }
                    if offBase > ZSTD_REP_NUM as size_t {
                        let matchIndex: U32 = (start.offset_from(base) as ::core::ffi::c_long
                            as size_t)
                            .wrapping_sub(offBase.wrapping_sub(ZSTD_REP_NUM as size_t))
                            as U32;
                        let mut match_0: *const BYTE = if matchIndex < dictLimit {
                            dictBase.offset(matchIndex as isize)
                        } else {
                            base.offset(matchIndex as isize)
                        };
                        let mStart: *const BYTE = if matchIndex < dictLimit {
                            dictStart
                        } else {
                            prefixStart
                        };
                        while start > anchor
                            && match_0 > mStart
                            && *start.offset(-(1 as ::core::ffi::c_int) as isize)
                                as ::core::ffi::c_int
                                == *match_0.offset(-(1 as ::core::ffi::c_int) as isize)
                                    as ::core::ffi::c_int
                        {
                            start = start.offset(-1);
                            match_0 = match_0.offset(-1);
                            matchLength = matchLength.wrapping_add(1);
                        }
                        offset_2 = offset_1;
                        offset_1 = (offBase as ::core::ffi::c_ulong)
                            .wrapping_sub(ZSTD_REP_NUM as ::core::ffi::c_ulong)
                            as U32;
                    }
                }
            }
            _ => {}
        }
        let litLength: size_t = start.offset_from(anchor) as ::core::ffi::c_long as size_t;
        ZSTD_storeSeq(
            seqStore,
            litLength,
            anchor,
            iend,
            offBase as U32,
            matchLength,
        );
        ip = start.offset(matchLength as isize);
        anchor = ip;
        if (*ms).lazySkipping != 0 {
            if searchMethod as ::core::ffi::c_uint
                == search_rowHash as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                ZSTD_row_fillHashCache(ms, base, rowLog, mls, (*ms).nextToUpdate, ilimit);
            }
            (*ms).lazySkipping = 0 as ::core::ffi::c_int;
        }
        while ip <= ilimit {
            let repCurrent: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
            let windowLow_2: U32 =
                ZSTD_getLowestMatchIndex(ms, repCurrent, windowLog as ::core::ffi::c_uint) as U32;
            let repIndex_2: U32 = repCurrent.wrapping_sub(offset_2);
            let repBase_2: *const BYTE = if repIndex_2 < dictLimit {
                dictBase
            } else {
                base
            };
            let repMatch_2: *const BYTE = repBase_2.offset(repIndex_2 as isize);
            if !(ZSTD_index_overlap_check(dictLimit, repIndex_2)
                & (offset_2 <= repCurrent.wrapping_sub(windowLow_2)) as ::core::ffi::c_int
                != 0)
            {
                break;
            }
            if !(MEM_read32(ip as *const ::core::ffi::c_void)
                == MEM_read32(repMatch_2 as *const ::core::ffi::c_void))
            {
                break;
            }
            let repEnd_2: *const BYTE = if repIndex_2 < dictLimit {
                dictEnd
            } else {
                iend
            };
            matchLength = ZSTD_count_2segments(
                ip.offset(4 as ::core::ffi::c_int as isize),
                repMatch_2.offset(4 as ::core::ffi::c_int as isize),
                iend,
                repEnd_2,
                prefixStart,
            )
            .wrapping_add(4 as size_t);
            offBase = offset_2 as size_t;
            offset_2 = offset_1;
            offset_1 = offBase as U32;
            ZSTD_storeSeq(
                seqStore,
                0 as size_t,
                anchor,
                iend,
                REPCODE1_TO_OFFBASE as U32,
                matchLength,
            );
            ip = ip.offset(matchLength as isize);
            anchor = ip;
        }
    }
    *rep.offset(0 as ::core::ffi::c_int as isize) = offset_1;
    *rep.offset(1 as ::core::ffi::c_int as isize) = offset_2;
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        0 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_greedy_extDict_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        0 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        1 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy_extDict_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        1 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_hashChain,
        2 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_lazy2_extDict_row(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_rowHash,
        2 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_btlazy2_extDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_lazy_extDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        search_binaryTree,
        2 as U32,
    );
}
