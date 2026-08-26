use ::core::arch::asm;
#[cfg(target_arch = "x86")]
pub use ::core::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
#[cfg(target_arch = "x86_64")]
pub use ::core::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
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
pub type ZSTD_dictTableLoadMethod_e = ::core::ffi::c_uint;
pub const ZSTD_dtlm_full: ZSTD_dictTableLoadMethod_e = 1;
pub const ZSTD_dtlm_fast: ZSTD_dictTableLoadMethod_e = 0;
pub type ZSTD_tableFillPurpose_e = ::core::ffi::c_uint;
pub const ZSTD_tfp_forCDict: ZSTD_tableFillPurpose_e = 1;
pub const ZSTD_tfp_forCCtx: ZSTD_tableFillPurpose_e = 0;
pub const CACHELINE_SIZE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
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
pub const HASH_READ_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTD_selectAddr(
    mut index: U32,
    mut lowLimit: U32,
    mut candidate: *const BYTE,
    mut backup: *const BYTE,
) -> *const BYTE {
    asm!(
        "cmp {1}, {2}\n", "cmova {3}, {0}\n", "\n", inlateout(reg) candidate,
        inlateout(reg) index => _, inlateout(reg) lowLimit => _, inlateout(reg) backup =>
        _, options(preserves_flags, pure, readonly, att_syntax)
    );
    return candidate;
}
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
static mut prime5bytes: U64 = 889523592379 as U64;
unsafe extern "C" fn ZSTD_hash5(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return (((u << 64 as ::core::ffi::c_int - 40 as ::core::ffi::c_int).wrapping_mul(prime5bytes)
        ^ s)
        >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash5Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash5(MEM_readLE64(p), h, 0 as U64);
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
static mut prime7bytes: U64 = 58295818150454627 as U64;
unsafe extern "C" fn ZSTD_hash7(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return (((u << 64 as ::core::ffi::c_int - 56 as ::core::ffi::c_int).wrapping_mul(prime7bytes)
        ^ s)
        >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash7Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash7(MEM_readLE64(p), h, 0 as U64);
}
static mut prime8bytes: U64 = 0xcf1bbcdcb7a56463 as U64;
unsafe extern "C" fn ZSTD_hash8(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return ((u.wrapping_mul(prime8bytes) ^ s) >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash8Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash8(MEM_readLE64(p), h, 0 as U64);
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
pub const ZSTD_SHORT_CACHE_TAG_BITS: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const ZSTD_SHORT_CACHE_TAG_MASK: ::core::ffi::c_uint = ((1 as ::core::ffi::c_uint)
    << ZSTD_SHORT_CACHE_TAG_BITS)
    .wrapping_sub(1 as ::core::ffi::c_uint);
#[inline]
unsafe extern "C" fn ZSTD_writeTaggedIndex(
    hashTable: *mut U32,
    mut hashAndTag: size_t,
    mut index: U32,
) {
    let hash: size_t = hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
    let tag: U32 = (hashAndTag & ZSTD_SHORT_CACHE_TAG_MASK as size_t) as U32;
    *hashTable.offset(hash as isize) = index << ZSTD_SHORT_CACHE_TAG_BITS | tag;
}
#[inline]
unsafe extern "C" fn ZSTD_comparePackedTags(
    mut packedTag1: size_t,
    mut packedTag2: size_t,
) -> ::core::ffi::c_int {
    let tag1: U32 = (packedTag1 & ZSTD_SHORT_CACHE_TAG_MASK as size_t) as U32;
    let tag2: U32 = (packedTag2 & ZSTD_SHORT_CACHE_TAG_MASK as size_t) as U32;
    return (tag1 == tag2) as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTD_fillDoubleHashTableForCDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut end: *const ::core::ffi::c_void,
    mut dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashLarge: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = ((*cParams).hashLog as U32).wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS as U32);
    let mls: U32 = (*cParams).minMatch as U32;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = ((*cParams).chainLog as U32).wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS as U32);
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.offset((*ms).nextToUpdate as isize);
    let iend: *const BYTE = (end as *const BYTE).offset(-(HASH_READ_SIZE as isize));
    let fastHashFillStep: U32 = 3 as U32;
    while ip
        .offset(fastHashFillStep as isize)
        .offset(-(1 as ::core::ffi::c_int as isize))
        <= iend
    {
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let mut i: U32 = 0;
        i = 0 as U32;
        while i < fastHashFillStep {
            let smHashAndTag: size_t = ZSTD_hashPtr(
                ip.offset(i as isize) as *const ::core::ffi::c_void,
                hBitsS,
                mls,
            ) as size_t;
            let lgHashAndTag: size_t = ZSTD_hashPtr(
                ip.offset(i as isize) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as size_t;
            if i == 0 as U32 {
                ZSTD_writeTaggedIndex(hashSmall, smHashAndTag, curr.wrapping_add(i));
            }
            if i == 0 as U32
                || *hashLarge.offset((lgHashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS) as isize)
                    == 0 as U32
            {
                ZSTD_writeTaggedIndex(hashLarge, lgHashAndTag, curr.wrapping_add(i));
            }
            if dtlm as ::core::ffi::c_uint
                == ZSTD_dtlm_fast as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                break;
            }
            i = i.wrapping_add(1);
        }
        ip = ip.offset(fastHashFillStep as isize);
    }
}
unsafe extern "C" fn ZSTD_fillDoubleHashTableForCCtx(
    mut ms: *mut ZSTD_MatchState_t,
    mut end: *const ::core::ffi::c_void,
    mut dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashLarge: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog as U32;
    let mls: U32 = (*cParams).minMatch as U32;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog as U32;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.offset((*ms).nextToUpdate as isize);
    let iend: *const BYTE = (end as *const BYTE).offset(-(HASH_READ_SIZE as isize));
    let fastHashFillStep: U32 = 3 as U32;
    while ip
        .offset(fastHashFillStep as isize)
        .offset(-(1 as ::core::ffi::c_int as isize))
        <= iend
    {
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let mut i: U32 = 0;
        i = 0 as U32;
        while i < fastHashFillStep {
            let smHash: size_t = ZSTD_hashPtr(
                ip.offset(i as isize) as *const ::core::ffi::c_void,
                hBitsS,
                mls,
            ) as size_t;
            let lgHash: size_t = ZSTD_hashPtr(
                ip.offset(i as isize) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as size_t;
            if i == 0 as U32 {
                *hashSmall.offset(smHash as isize) = curr.wrapping_add(i);
            }
            if i == 0 as U32 || *hashLarge.offset(lgHash as isize) == 0 as U32 {
                *hashLarge.offset(lgHash as isize) = curr.wrapping_add(i);
            }
            if dtlm as ::core::ffi::c_uint
                == ZSTD_dtlm_fast as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                break;
            }
            i = i.wrapping_add(1);
        }
        ip = ip.offset(fastHashFillStep as isize);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fillDoubleHashTable(
    mut ms: *mut ZSTD_MatchState_t,
    end: *const ::core::ffi::c_void,
    mut dtlm: ZSTD_dictTableLoadMethod_e,
    mut tfp: ZSTD_tableFillPurpose_e,
) {
    if tfp as ::core::ffi::c_uint == ZSTD_tfp_forCDict as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_fillDoubleHashTableForCDict(ms, end, dtlm);
    } else {
        ZSTD_fillDoubleHashTableForCCtx(ms, end, dtlm);
    };
}
#[inline(always)]
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_noDict_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mls: U32,
) -> size_t {
    let mut cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog as U32;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog as U32;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 =
        (istart.offset_from(base) as ::core::ffi::c_long as size_t).wrapping_add(srcSize) as U32;
    let prefixLowestIndex: U32 =
        ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog) as U32;
    let prefixLowest: *const BYTE = base.offset(prefixLowestIndex as isize);
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(HASH_READ_SIZE as isize));
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    let mut offsetSaved1: U32 = 0 as U32;
    let mut offsetSaved2: U32 = 0 as U32;
    let mut mLength: size_t = 0;
    let mut offset: U32 = 0;
    let mut curr: U32 = 0;
    let kStepIncr: size_t = ((1 as ::core::ffi::c_int) << kSearchStrength) as size_t;
    let mut nextStep: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut step: size_t = 0;
    let mut hl0: size_t = 0;
    let mut hl1: size_t = 0;
    let mut idxl0: U32 = 0;
    let mut idxl1: U32 = 0;
    let mut matchl0: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut matchs0: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut matchl1: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut matchs0_safe: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut ip: *const BYTE = istart;
    let mut ip1: *const BYTE = ::core::ptr::null::<BYTE>();
    let dummy: [BYTE; 10] = [
        0x12 as ::core::ffi::c_int as BYTE,
        0x34 as ::core::ffi::c_int as BYTE,
        0x56 as ::core::ffi::c_int as BYTE,
        0x78 as ::core::ffi::c_int as BYTE,
        0x9a as ::core::ffi::c_int as BYTE,
        0xbc as ::core::ffi::c_int as BYTE,
        0xde as ::core::ffi::c_int as BYTE,
        0xf0 as ::core::ffi::c_int as BYTE,
        0xe2 as ::core::ffi::c_int as BYTE,
        0xb4 as ::core::ffi::c_int as BYTE,
    ];
    ip = ip.offset(
        (ip.offset_from(prefixLowest) as ::core::ffi::c_long == 0 as ::core::ffi::c_long)
            as ::core::ffi::c_int as isize,
    );
    let current: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
    let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, current, (*cParams).windowLog) as U32;
    let maxRep: U32 = current.wrapping_sub(windowLow);
    if offset_2 > maxRep {
        offsetSaved2 = offset_2;
        offset_2 = 0 as U32;
    }
    if offset_1 > maxRep {
        offsetSaved1 = offset_1;
        offset_1 = 0 as U32;
    }
    loop {
        's_428: {
            let mut current_block_83: u64;
            step = 1 as size_t;
            nextStep = ip.offset(kStepIncr as isize);
            ip1 = ip.offset(step as isize);
            if !(ip1 > ilimit) {
                hl0 = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsL, 8 as U32);
                idxl0 = *hashLong.offset(hl0 as isize);
                matchl0 = base.offset(idxl0 as isize);
                loop {
                    let hs0: size_t =
                        ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsS, mls) as size_t;
                    let idxs0: U32 = *hashSmall.offset(hs0 as isize);
                    curr = ip.offset_from(base) as ::core::ffi::c_long as U32;
                    matchs0 = base.offset(idxs0 as isize);
                    let ref mut fresh2 = *hashSmall.offset(hs0 as isize);
                    *fresh2 = curr;
                    *hashLong.offset(hl0 as isize) = *fresh2;
                    if (offset_1 > 0 as U32) as ::core::ffi::c_int
                        & (MEM_read32(
                            ip.offset(1 as ::core::ffi::c_int as isize)
                                .offset(-(offset_1 as isize))
                                as *const ::core::ffi::c_void,
                        ) == MEM_read32(ip.offset(1 as ::core::ffi::c_int as isize)
                            as *const ::core::ffi::c_void))
                            as ::core::ffi::c_int
                        != 0
                    {
                        mLength = ZSTD_count(
                            ip.offset(1 as ::core::ffi::c_int as isize)
                                .offset(4 as ::core::ffi::c_int as isize),
                            ip.offset(1 as ::core::ffi::c_int as isize)
                                .offset(4 as ::core::ffi::c_int as isize)
                                .offset(-(offset_1 as isize)),
                            iend,
                        )
                        .wrapping_add(4 as size_t);
                        ip = ip.offset(1);
                        ZSTD_storeSeq(
                            seqStore,
                            ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                            anchor,
                            iend,
                            REPCODE1_TO_OFFBASE as U32,
                            mLength,
                        );
                        current_block_83 = 11437251092972387081;
                        break;
                    } else {
                        hl1 = ZSTD_hashPtr(ip1 as *const ::core::ffi::c_void, hBitsL, 8 as U32);
                        let matchl0_safe: *const BYTE = ZSTD_selectAddr(
                            idxl0,
                            prefixLowestIndex,
                            matchl0,
                            (&raw const dummy as *const BYTE)
                                .offset(0 as ::core::ffi::c_int as isize)
                                as *const BYTE,
                        ) as *const BYTE;
                        if MEM_read64(matchl0_safe as *const ::core::ffi::c_void)
                            == MEM_read64(ip as *const ::core::ffi::c_void)
                            && matchl0_safe == matchl0
                        {
                            mLength = ZSTD_count(
                                ip.offset(8 as ::core::ffi::c_int as isize),
                                matchl0.offset(8 as ::core::ffi::c_int as isize),
                                iend,
                            )
                            .wrapping_add(8 as size_t);
                            offset = ip.offset_from(matchl0) as ::core::ffi::c_long as U32;
                            while (ip > anchor) as ::core::ffi::c_int
                                & (matchl0 > prefixLowest) as ::core::ffi::c_int
                                != 0
                                && *ip.offset(-(1 as ::core::ffi::c_int) as isize)
                                    as ::core::ffi::c_int
                                    == *matchl0.offset(-(1 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_int
                            {
                                ip = ip.offset(-1);
                                matchl0 = matchl0.offset(-1);
                                mLength = mLength.wrapping_add(1);
                            }
                            current_block_83 = 3261909530081418489;
                            break;
                        } else {
                            idxl1 = *hashLong.offset(hl1 as isize);
                            matchl1 = base.offset(idxl1 as isize);
                            matchs0_safe = ZSTD_selectAddr(
                                idxs0,
                                prefixLowestIndex,
                                matchs0,
                                (&raw const dummy as *const BYTE)
                                    .offset(0 as ::core::ffi::c_int as isize)
                                    as *const BYTE,
                            );
                            if MEM_read32(matchs0_safe as *const ::core::ffi::c_void)
                                == MEM_read32(ip as *const ::core::ffi::c_void)
                                && matchs0_safe == matchs0
                            {
                                current_block_83 = 7203934606777217881;
                                break;
                            }
                            if ip1 >= nextStep {
                                step = step.wrapping_add(1);
                                nextStep = nextStep.offset(kStepIncr as isize);
                            }
                            ip = ip1;
                            ip1 = ip1.offset(step as isize);
                            hl0 = hl1;
                            idxl0 = idxl1;
                            matchl0 = matchl1;
                            if !(ip1 <= ilimit) {
                                current_block_83 = 5345901486425218845;
                                break;
                            }
                        }
                    }
                }
                match current_block_83 {
                    5345901486425218845 => {}
                    _ => {
                        match current_block_83 {
                            7203934606777217881 => {
                                mLength = ZSTD_count(
                                    ip.offset(4 as ::core::ffi::c_int as isize),
                                    matchs0.offset(4 as ::core::ffi::c_int as isize),
                                    iend,
                                )
                                .wrapping_add(4 as size_t);
                                offset = ip.offset_from(matchs0) as ::core::ffi::c_long as U32;
                                if idxl1 > prefixLowestIndex
                                    && MEM_read64(matchl1 as *const ::core::ffi::c_void)
                                        == MEM_read64(ip1 as *const ::core::ffi::c_void)
                                {
                                    let l1len: size_t = (ZSTD_count(
                                        ip1.offset(8 as ::core::ffi::c_int as isize),
                                        matchl1.offset(8 as ::core::ffi::c_int as isize),
                                        iend,
                                    )
                                        as size_t)
                                        .wrapping_add(8 as size_t);
                                    if l1len > mLength {
                                        ip = ip1;
                                        mLength = l1len;
                                        offset =
                                            ip.offset_from(matchl1) as ::core::ffi::c_long as U32;
                                        matchs0 = matchl1;
                                    }
                                }
                                while (ip > anchor) as ::core::ffi::c_int
                                    & (matchs0 > prefixLowest) as ::core::ffi::c_int
                                    != 0
                                    && *ip.offset(-(1 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_int
                                        == *matchs0.offset(-(1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                {
                                    ip = ip.offset(-1);
                                    matchs0 = matchs0.offset(-1);
                                    mLength = mLength.wrapping_add(1);
                                }
                                current_block_83 = 3261909530081418489;
                            }
                            _ => {}
                        }
                        match current_block_83 {
                            3261909530081418489 => {
                                offset_2 = offset_1;
                                offset_1 = offset;
                                if step < 4 as size_t {
                                    *hashLong.offset(hl1 as isize) =
                                        ip1.offset_from(base) as ::core::ffi::c_long as U32;
                                }
                                ZSTD_storeSeq(
                                    seqStore,
                                    ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                                    anchor,
                                    iend,
                                    offset.wrapping_add(ZSTD_REP_NUM as U32),
                                    mLength,
                                );
                            }
                            _ => {}
                        }
                        ip = ip.offset(mLength as isize);
                        anchor = ip;
                        if ip <= ilimit {
                            let indexToInsert: U32 = curr.wrapping_add(2 as U32);
                            *hashLong.offset(ZSTD_hashPtr(
                                base.offset(indexToInsert as isize) as *const ::core::ffi::c_void,
                                hBitsL,
                                8 as U32,
                            ) as isize) = indexToInsert;
                            *hashLong.offset(ZSTD_hashPtr(
                                ip.offset(-(2 as ::core::ffi::c_int as isize))
                                    as *const ::core::ffi::c_void,
                                hBitsL,
                                8 as U32,
                            ) as isize) =
                                ip.offset(-(2 as ::core::ffi::c_int as isize))
                                    .offset_from(base)
                                    as ::core::ffi::c_long as U32;
                            *hashSmall.offset(ZSTD_hashPtr(
                                base.offset(indexToInsert as isize) as *const ::core::ffi::c_void,
                                hBitsS,
                                mls,
                            ) as isize) = indexToInsert;
                            *hashSmall.offset(ZSTD_hashPtr(
                                ip.offset(-(1 as ::core::ffi::c_int as isize))
                                    as *const ::core::ffi::c_void,
                                hBitsS,
                                mls,
                            ) as isize) =
                                ip.offset(-(1 as ::core::ffi::c_int as isize))
                                    .offset_from(base)
                                    as ::core::ffi::c_long as U32;
                            while ip <= ilimit
                                && (offset_2 > 0 as U32) as ::core::ffi::c_int
                                    & (MEM_read32(ip as *const ::core::ffi::c_void)
                                        == MEM_read32(ip.offset(-(offset_2 as isize))
                                            as *const ::core::ffi::c_void))
                                        as ::core::ffi::c_int
                                    != 0
                            {
                                let rLength: size_t = (ZSTD_count(
                                    ip.offset(4 as ::core::ffi::c_int as isize),
                                    ip.offset(4 as ::core::ffi::c_int as isize)
                                        .offset(-(offset_2 as isize)),
                                    iend,
                                ) as size_t)
                                    .wrapping_add(4 as size_t);
                                let tmpOff: U32 = offset_2;
                                offset_2 = offset_1;
                                offset_1 = tmpOff;
                                *hashSmall.offset(ZSTD_hashPtr(
                                    ip as *const ::core::ffi::c_void,
                                    hBitsS,
                                    mls,
                                ) as isize) = ip.offset_from(base) as ::core::ffi::c_long as U32;
                                *hashLong.offset(ZSTD_hashPtr(
                                    ip as *const ::core::ffi::c_void,
                                    hBitsL,
                                    8 as U32,
                                ) as isize) = ip.offset_from(base) as ::core::ffi::c_long as U32;
                                ZSTD_storeSeq(
                                    seqStore,
                                    0 as size_t,
                                    anchor,
                                    iend,
                                    REPCODE1_TO_OFFBASE as U32,
                                    rLength,
                                );
                                ip = ip.offset(rLength as isize);
                                anchor = ip;
                            }
                        }
                        break 's_428;
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
    }
}
#[inline(always)]
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mls: U32,
) -> size_t {
    let mut current_block: u64;
    let mut cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog as U32;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog as U32;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 =
        (istart.offset_from(base) as ::core::ffi::c_long as size_t).wrapping_add(srcSize) as U32;
    let prefixLowestIndex: U32 =
        ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog) as U32;
    let prefixLowest: *const BYTE = base.offset(prefixLowestIndex as isize);
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(HASH_READ_SIZE as isize));
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictCParams: *const ZSTD_compressionParameters = &raw const (*dms).cParams;
    let dictHashLong: *const U32 = (*dms).hashTable;
    let dictHashSmall: *const U32 = (*dms).chainTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictStart: *const BYTE = dictBase.offset(dictStartIndex as isize);
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictIndexDelta: U32 =
        prefixLowestIndex.wrapping_sub(dictEnd.offset_from(dictBase) as ::core::ffi::c_long as U32);
    let dictHBitsL: U32 =
        ((*dictCParams).hashLog as U32).wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS as U32);
    let dictHBitsS: U32 =
        ((*dictCParams).chainLog as U32).wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS as U32);
    let dictAndPrefixLength: U32 = (ip.offset_from(prefixLowest) as ::core::ffi::c_long
        + dictEnd.offset_from(dictStart) as ::core::ffi::c_long)
        as U32;
    if (*ms).prefetchCDictTables != 0 {
        let hashTableBytes: size_t = ((1 as ::core::ffi::c_int as size_t)
            << (*dictCParams).hashLog)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t);
        let chainTableBytes: size_t = ((1 as ::core::ffi::c_int as size_t)
            << (*dictCParams).chainLog)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t);
        let _ptr: *const ::core::ffi::c_char = dictHashLong as *const ::core::ffi::c_char;
        let _size: size_t = hashTableBytes;
        let mut _pos: size_t = 0;
        _pos = 0 as size_t;
        while _pos < _size {
            _pos = (_pos as ::core::ffi::c_ulong)
                .wrapping_add(CACHELINE_SIZE as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        let _ptr_0: *const ::core::ffi::c_char = dictHashSmall as *const ::core::ffi::c_char;
        let _size_0: size_t = chainTableBytes;
        let mut _pos_0: size_t = 0;
        _pos_0 = 0 as size_t;
        while _pos_0 < _size_0 {
            _pos_0 = (_pos_0 as ::core::ffi::c_ulong)
                .wrapping_add(CACHELINE_SIZE as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    }
    ip = ip.offset((dictAndPrefixLength == 0 as U32) as ::core::ffi::c_int as isize);
    while ip < ilimit {
        let mut mLength: size_t = 0;
        let mut offset: U32 = 0;
        let h2: size_t = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsL, 8 as U32) as size_t;
        let h: size_t = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsS, mls) as size_t;
        let dictHashAndTagL: size_t =
            ZSTD_hashPtr(ip as *const ::core::ffi::c_void, dictHBitsL, 8 as U32) as size_t;
        let dictHashAndTagS: size_t =
            ZSTD_hashPtr(ip as *const ::core::ffi::c_void, dictHBitsS, mls) as size_t;
        let dictMatchIndexAndTagL: U32 =
            *dictHashLong.offset((dictHashAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS) as isize);
        let dictMatchIndexAndTagS: U32 =
            *dictHashSmall.offset((dictHashAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS) as isize);
        let dictTagsMatchL: ::core::ffi::c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTagL as size_t, dictHashAndTagL)
                as ::core::ffi::c_int;
        let dictTagsMatchS: ::core::ffi::c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTagS as size_t, dictHashAndTagS)
                as ::core::ffi::c_int;
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let matchIndexL: U32 = *hashLong.offset(h2 as isize);
        let mut matchIndexS: U32 = *hashSmall.offset(h as isize);
        let mut matchLong: *const BYTE = base.offset(matchIndexL as isize);
        let mut match_0: *const BYTE = base.offset(matchIndexS as isize);
        let repIndex: U32 = curr.wrapping_add(1 as U32).wrapping_sub(offset_1);
        let mut repMatch: *const BYTE = if repIndex < prefixLowestIndex {
            dictBase.offset(repIndex.wrapping_sub(dictIndexDelta) as isize)
        } else {
            base.offset(repIndex as isize)
        };
        let ref mut fresh3 = *hashSmall.offset(h as isize);
        *fresh3 = curr;
        *hashLong.offset(h2 as isize) = *fresh3;
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
            mLength = ZSTD_count_2segments(
                ip.offset(1 as ::core::ffi::c_int as isize)
                    .offset(4 as ::core::ffi::c_int as isize),
                repMatch.offset(4 as ::core::ffi::c_int as isize),
                iend,
                repMatchEnd,
                prefixLowest,
            )
            .wrapping_add(4 as size_t);
            ip = ip.offset(1);
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                anchor,
                iend,
                REPCODE1_TO_OFFBASE as U32,
                mLength,
            );
        } else {
            if matchIndexL >= prefixLowestIndex
                && MEM_read64(matchLong as *const ::core::ffi::c_void)
                    == MEM_read64(ip as *const ::core::ffi::c_void)
            {
                mLength = ZSTD_count(
                    ip.offset(8 as ::core::ffi::c_int as isize),
                    matchLong.offset(8 as ::core::ffi::c_int as isize),
                    iend,
                )
                .wrapping_add(8 as size_t);
                offset = ip.offset_from(matchLong) as ::core::ffi::c_long as U32;
                while (ip > anchor) as ::core::ffi::c_int
                    & (matchLong > prefixLowest) as ::core::ffi::c_int
                    != 0
                    && *ip.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == *matchLong.offset(-(1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int
                {
                    ip = ip.offset(-1);
                    matchLong = matchLong.offset(-1);
                    mLength = mLength.wrapping_add(1);
                }
            } else {
                if dictTagsMatchL != 0 {
                    let dictMatchIndexL: U32 = dictMatchIndexAndTagL >> ZSTD_SHORT_CACHE_TAG_BITS;
                    let mut dictMatchL: *const BYTE = dictBase.offset(dictMatchIndexL as isize);
                    if dictMatchL > dictStart
                        && MEM_read64(dictMatchL as *const ::core::ffi::c_void)
                            == MEM_read64(ip as *const ::core::ffi::c_void)
                    {
                        mLength = ZSTD_count_2segments(
                            ip.offset(8 as ::core::ffi::c_int as isize),
                            dictMatchL.offset(8 as ::core::ffi::c_int as isize),
                            iend,
                            dictEnd,
                            prefixLowest,
                        )
                        .wrapping_add(8 as size_t);
                        offset = curr
                            .wrapping_sub(dictMatchIndexL)
                            .wrapping_sub(dictIndexDelta);
                        while (ip > anchor) as ::core::ffi::c_int
                            & (dictMatchL > dictStart) as ::core::ffi::c_int
                            != 0
                            && *ip.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                                == *dictMatchL.offset(-(1 as ::core::ffi::c_int) as isize)
                                    as ::core::ffi::c_int
                        {
                            ip = ip.offset(-1);
                            dictMatchL = dictMatchL.offset(-1);
                            mLength = mLength.wrapping_add(1);
                        }
                        current_block = 14687069264259698721;
                    } else {
                        current_block = 6721012065216013753;
                    }
                } else {
                    current_block = 6721012065216013753;
                }
                match current_block {
                    14687069264259698721 => {}
                    _ => {
                        if matchIndexS > prefixLowestIndex {
                            if MEM_read32(match_0 as *const ::core::ffi::c_void)
                                == MEM_read32(ip as *const ::core::ffi::c_void)
                            {
                                current_block = 2631791190359682872;
                            } else {
                                current_block = 5372832139739605200;
                            }
                        } else if dictTagsMatchS != 0 {
                            let dictMatchIndexS: U32 =
                                dictMatchIndexAndTagS >> ZSTD_SHORT_CACHE_TAG_BITS;
                            match_0 = dictBase.offset(dictMatchIndexS as isize);
                            matchIndexS = dictMatchIndexS.wrapping_add(dictIndexDelta);
                            if match_0 > dictStart
                                && MEM_read32(match_0 as *const ::core::ffi::c_void)
                                    == MEM_read32(ip as *const ::core::ffi::c_void)
                            {
                                current_block = 2631791190359682872;
                            } else {
                                current_block = 5372832139739605200;
                            }
                        } else {
                            current_block = 5372832139739605200;
                        }
                        match current_block {
                            5372832139739605200 => {
                                ip = ip.offset(
                                    ((ip.offset_from(anchor) as ::core::ffi::c_long
                                        >> kSearchStrength)
                                        + 1 as ::core::ffi::c_long)
                                        as isize,
                                );
                                continue;
                            }
                            _ => {
                                let hl3: size_t = ZSTD_hashPtr(
                                    ip.offset(1 as ::core::ffi::c_int as isize)
                                        as *const ::core::ffi::c_void,
                                    hBitsL,
                                    8 as U32,
                                ) as size_t;
                                let dictHashAndTagL3: size_t = ZSTD_hashPtr(
                                    ip.offset(1 as ::core::ffi::c_int as isize)
                                        as *const ::core::ffi::c_void,
                                    dictHBitsL,
                                    8 as U32,
                                )
                                    as size_t;
                                let matchIndexL3: U32 = *hashLong.offset(hl3 as isize);
                                let dictMatchIndexAndTagL3: U32 = *dictHashLong.offset(
                                    (dictHashAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS) as isize,
                                );
                                let dictTagsMatchL3: ::core::ffi::c_int = ZSTD_comparePackedTags(
                                    dictMatchIndexAndTagL3 as size_t,
                                    dictHashAndTagL3,
                                )
                                    as ::core::ffi::c_int;
                                let mut matchL3: *const BYTE = base.offset(matchIndexL3 as isize);
                                *hashLong.offset(hl3 as isize) = curr.wrapping_add(1 as U32);
                                if matchIndexL3 >= prefixLowestIndex
                                    && MEM_read64(matchL3 as *const ::core::ffi::c_void)
                                        == MEM_read64(ip.offset(1 as ::core::ffi::c_int as isize)
                                            as *const ::core::ffi::c_void)
                                {
                                    mLength = ZSTD_count(
                                        ip.offset(9 as ::core::ffi::c_int as isize),
                                        matchL3.offset(8 as ::core::ffi::c_int as isize),
                                        iend,
                                    )
                                    .wrapping_add(8 as size_t);
                                    ip = ip.offset(1);
                                    offset = ip.offset_from(matchL3) as ::core::ffi::c_long as U32;
                                    while (ip > anchor) as ::core::ffi::c_int
                                        & (matchL3 > prefixLowest) as ::core::ffi::c_int
                                        != 0
                                        && *ip.offset(-(1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                            == *matchL3.offset(-(1 as ::core::ffi::c_int) as isize)
                                                as ::core::ffi::c_int
                                    {
                                        ip = ip.offset(-1);
                                        matchL3 = matchL3.offset(-1);
                                        mLength = mLength.wrapping_add(1);
                                    }
                                } else {
                                    if dictTagsMatchL3 != 0 {
                                        let dictMatchIndexL3: U32 =
                                            dictMatchIndexAndTagL3 >> ZSTD_SHORT_CACHE_TAG_BITS;
                                        let mut dictMatchL3: *const BYTE =
                                            dictBase.offset(dictMatchIndexL3 as isize);
                                        if dictMatchL3 > dictStart
                                            && MEM_read64(dictMatchL3 as *const ::core::ffi::c_void)
                                                == MEM_read64(
                                                    ip.offset(1 as ::core::ffi::c_int as isize)
                                                        as *const ::core::ffi::c_void,
                                                )
                                        {
                                            mLength = ZSTD_count_2segments(
                                                ip.offset(1 as ::core::ffi::c_int as isize)
                                                    .offset(8 as ::core::ffi::c_int as isize),
                                                dictMatchL3
                                                    .offset(8 as ::core::ffi::c_int as isize),
                                                iend,
                                                dictEnd,
                                                prefixLowest,
                                            )
                                            .wrapping_add(8 as size_t);
                                            ip = ip.offset(1);
                                            offset = curr
                                                .wrapping_add(1 as U32)
                                                .wrapping_sub(dictMatchIndexL3)
                                                .wrapping_sub(dictIndexDelta);
                                            while (ip > anchor) as ::core::ffi::c_int
                                                & (dictMatchL3 > dictStart) as ::core::ffi::c_int
                                                != 0
                                                && *ip.offset(-(1 as ::core::ffi::c_int) as isize)
                                                    as ::core::ffi::c_int
                                                    == *dictMatchL3
                                                        .offset(-(1 as ::core::ffi::c_int) as isize)
                                                        as ::core::ffi::c_int
                                            {
                                                ip = ip.offset(-1);
                                                dictMatchL3 = dictMatchL3.offset(-1);
                                                mLength = mLength.wrapping_add(1);
                                            }
                                            current_block = 14687069264259698721;
                                        } else {
                                            current_block = 1209030638129645089;
                                        }
                                    } else {
                                        current_block = 1209030638129645089;
                                    }
                                    match current_block {
                                        14687069264259698721 => {}
                                        _ => {
                                            if matchIndexS < prefixLowestIndex {
                                                mLength = ZSTD_count_2segments(
                                                    ip.offset(4 as ::core::ffi::c_int as isize),
                                                    match_0
                                                        .offset(4 as ::core::ffi::c_int as isize),
                                                    iend,
                                                    dictEnd,
                                                    prefixLowest,
                                                )
                                                .wrapping_add(4 as size_t);
                                                offset = curr.wrapping_sub(matchIndexS);
                                                while (ip > anchor) as ::core::ffi::c_int
                                                    & (match_0 > dictStart) as ::core::ffi::c_int
                                                    != 0
                                                    && *ip
                                                        .offset(-(1 as ::core::ffi::c_int) as isize)
                                                        as ::core::ffi::c_int
                                                        == *match_0.offset(
                                                            -(1 as ::core::ffi::c_int) as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                {
                                                    ip = ip.offset(-1);
                                                    match_0 = match_0.offset(-1);
                                                    mLength = mLength.wrapping_add(1);
                                                }
                                            } else {
                                                mLength = ZSTD_count(
                                                    ip.offset(4 as ::core::ffi::c_int as isize),
                                                    match_0
                                                        .offset(4 as ::core::ffi::c_int as isize),
                                                    iend,
                                                )
                                                .wrapping_add(4 as size_t);
                                                offset = ip.offset_from(match_0)
                                                    as ::core::ffi::c_long
                                                    as U32;
                                                while (ip > anchor) as ::core::ffi::c_int
                                                    & (match_0 > prefixLowest) as ::core::ffi::c_int
                                                    != 0
                                                    && *ip
                                                        .offset(-(1 as ::core::ffi::c_int) as isize)
                                                        as ::core::ffi::c_int
                                                        == *match_0.offset(
                                                            -(1 as ::core::ffi::c_int) as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                {
                                                    ip = ip.offset(-1);
                                                    match_0 = match_0.offset(-1);
                                                    mLength = mLength.wrapping_add(1);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            offset_2 = offset_1;
            offset_1 = offset;
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                anchor,
                iend,
                offset.wrapping_add(ZSTD_REP_NUM as U32),
                mLength,
            );
        }
        ip = ip.offset(mLength as isize);
        anchor = ip;
        if ip <= ilimit {
            let indexToInsert: U32 = curr.wrapping_add(2 as U32);
            *hashLong.offset(ZSTD_hashPtr(
                base.offset(indexToInsert as isize) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as isize) = indexToInsert;
            *hashLong.offset(ZSTD_hashPtr(
                ip.offset(-(2 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as isize) = ip
                .offset(-(2 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            *hashSmall.offset(ZSTD_hashPtr(
                base.offset(indexToInsert as isize) as *const ::core::ffi::c_void,
                hBitsS,
                mls,
            ) as isize) = indexToInsert;
            *hashSmall.offset(ZSTD_hashPtr(
                ip.offset(-(1 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hBitsS,
                mls,
            ) as isize) = ip
                .offset(-(1 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            while ip <= ilimit {
                let current2: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let mut repMatch2: *const BYTE = if repIndex2 < prefixLowestIndex {
                    dictBase
                        .offset(repIndex2 as isize)
                        .offset(-(dictIndexDelta as isize))
                } else {
                    base.offset(repIndex2 as isize)
                };
                if !(ZSTD_index_overlap_check(prefixLowestIndex, repIndex2) != 0
                    && MEM_read32(repMatch2 as *const ::core::ffi::c_void)
                        == MEM_read32(ip as *const ::core::ffi::c_void))
                {
                    break;
                }
                let repEnd2: *const BYTE = if repIndex2 < prefixLowestIndex {
                    dictEnd
                } else {
                    iend
                };
                let repLength2: size_t = (ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    repMatch2.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repEnd2,
                    prefixLowest,
                ) as size_t)
                    .wrapping_add(4 as size_t);
                let mut tmpOffset: U32 = offset_2;
                offset_2 = offset_1;
                offset_1 = tmpOffset;
                ZSTD_storeSeq(
                    seqStore,
                    0 as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE as U32,
                    repLength2,
                );
                *hashSmall
                    .offset(ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsS, mls) as isize) =
                    current2;
                *hashLong.offset(
                    ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsL, 8 as U32) as isize,
                ) = current2;
                ip = ip.offset(repLength2 as isize);
                anchor = ip;
            }
        }
    }
    *rep.offset(0 as ::core::ffi::c_int as isize) = offset_1;
    *rep.offset(1 as ::core::ffi::c_int as isize) = offset_2;
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_noDict_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 4 as U32);
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_noDict_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 5 as U32);
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_noDict_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 6 as U32);
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_noDict_7(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_noDict_generic(ms, seqStore, rep, src, srcSize, 7 as U32);
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 4 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 5 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 6 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState_7(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 7 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch as U32;
    match mls {
        5 => {
            return ZSTD_compressBlock_doubleFast_noDict_5(ms, seqStore, rep, src, srcSize);
        }
        6 => {
            return ZSTD_compressBlock_doubleFast_noDict_6(ms, seqStore, rep, src, srcSize);
        }
        7 => {
            return ZSTD_compressBlock_doubleFast_noDict_7(ms, seqStore, rep, src, srcSize);
        }
        4 | _ => {
            return ZSTD_compressBlock_doubleFast_noDict_4(ms, seqStore, rep, src, srcSize);
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_dictMatchState(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch as U32;
    match mls {
        5 => {
            return ZSTD_compressBlock_doubleFast_dictMatchState_5(ms, seqStore, rep, src, srcSize);
        }
        6 => {
            return ZSTD_compressBlock_doubleFast_dictMatchState_6(ms, seqStore, rep, src, srcSize);
        }
        7 => {
            return ZSTD_compressBlock_doubleFast_dictMatchState_7(ms, seqStore, rep, src, srcSize);
        }
        4 | _ => {
            return ZSTD_compressBlock_doubleFast_dictMatchState_4(ms, seqStore, rep, src, srcSize);
        }
    };
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mls: U32,
) -> size_t {
    let mut cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashLong: *mut U32 = (*ms).hashTable;
    let hBitsL: U32 = (*cParams).hashLog as U32;
    let hashSmall: *mut U32 = (*ms).chainTable;
    let hBitsS: U32 = (*cParams).chainLog as U32;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut anchor: *const BYTE = istart;
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(8 as ::core::ffi::c_int as isize));
    let base: *const BYTE = (*ms).window.base;
    let endIndex: U32 =
        (istart.offset_from(base) as ::core::ffi::c_long as size_t).wrapping_add(srcSize) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, (*cParams).windowLog) as U32;
    let dictStartIndex: U32 = lowLimit;
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit > lowLimit {
        dictLimit
    } else {
        lowLimit
    };
    let prefixStart: *const BYTE = base.offset(prefixStartIndex as isize);
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let dictStart: *const BYTE = dictBase.offset(dictStartIndex as isize);
    let dictEnd: *const BYTE = dictBase.offset(prefixStartIndex as isize);
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_doubleFast(ms, seqStore, rep, src, srcSize);
    }
    while ip < ilimit {
        let hSmall: size_t = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsS, mls) as size_t;
        let matchIndex: U32 = *hashSmall.offset(hSmall as isize);
        let matchBase: *const BYTE = if matchIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let mut match_0: *const BYTE = matchBase.offset(matchIndex as isize);
        let hLong: size_t =
            ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsL, 8 as U32) as size_t;
        let matchLongIndex: U32 = *hashLong.offset(hLong as isize);
        let matchLongBase: *const BYTE = if matchLongIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let mut matchLong: *const BYTE = matchLongBase.offset(matchLongIndex as isize);
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let repIndex: U32 = curr.wrapping_add(1 as U32).wrapping_sub(offset_1);
        let repBase: *const BYTE = if repIndex < prefixStartIndex {
            dictBase
        } else {
            base
        };
        let repMatch: *const BYTE = repBase.offset(repIndex as isize);
        let mut mLength: size_t = 0;
        let ref mut fresh4 = *hashLong.offset(hLong as isize);
        *fresh4 = curr;
        *hashSmall.offset(hSmall as isize) = *fresh4;
        if ZSTD_index_overlap_check(prefixStartIndex, repIndex)
            & (offset_1 <= curr.wrapping_add(1 as U32).wrapping_sub(dictStartIndex))
                as ::core::ffi::c_int
            != 0
            && MEM_read32(repMatch as *const ::core::ffi::c_void)
                == MEM_read32(
                    ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void
                )
        {
            let mut repMatchEnd: *const BYTE = if repIndex < prefixStartIndex {
                dictEnd
            } else {
                iend
            };
            mLength = ZSTD_count_2segments(
                ip.offset(1 as ::core::ffi::c_int as isize)
                    .offset(4 as ::core::ffi::c_int as isize),
                repMatch.offset(4 as ::core::ffi::c_int as isize),
                iend,
                repMatchEnd,
                prefixStart,
            )
            .wrapping_add(4 as size_t);
            ip = ip.offset(1);
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                anchor,
                iend,
                REPCODE1_TO_OFFBASE as U32,
                mLength,
            );
        } else if matchLongIndex > dictStartIndex
            && MEM_read64(matchLong as *const ::core::ffi::c_void)
                == MEM_read64(ip as *const ::core::ffi::c_void)
        {
            let matchEnd: *const BYTE = if matchLongIndex < prefixStartIndex {
                dictEnd
            } else {
                iend
            };
            let lowMatchPtr: *const BYTE = if matchLongIndex < prefixStartIndex {
                dictStart
            } else {
                prefixStart
            };
            let mut offset: U32 = 0;
            mLength = ZSTD_count_2segments(
                ip.offset(8 as ::core::ffi::c_int as isize),
                matchLong.offset(8 as ::core::ffi::c_int as isize),
                iend,
                matchEnd,
                prefixStart,
            )
            .wrapping_add(8 as size_t);
            offset = curr.wrapping_sub(matchLongIndex);
            while (ip > anchor) as ::core::ffi::c_int
                & (matchLong > lowMatchPtr) as ::core::ffi::c_int
                != 0
                && *ip.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                    == *matchLong.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
            {
                ip = ip.offset(-1);
                matchLong = matchLong.offset(-1);
                mLength = mLength.wrapping_add(1);
            }
            offset_2 = offset_1;
            offset_1 = offset;
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                anchor,
                iend,
                offset.wrapping_add(ZSTD_REP_NUM as U32),
                mLength,
            );
        } else if matchIndex > dictStartIndex
            && MEM_read32(match_0 as *const ::core::ffi::c_void)
                == MEM_read32(ip as *const ::core::ffi::c_void)
        {
            let h3: size_t = ZSTD_hashPtr(
                ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as size_t;
            let matchIndex3: U32 = *hashLong.offset(h3 as isize);
            let match3Base: *const BYTE = if matchIndex3 < prefixStartIndex {
                dictBase
            } else {
                base
            };
            let mut match3: *const BYTE = match3Base.offset(matchIndex3 as isize);
            let mut offset_0: U32 = 0;
            *hashLong.offset(h3 as isize) = curr.wrapping_add(1 as U32);
            if matchIndex3 > dictStartIndex
                && MEM_read64(match3 as *const ::core::ffi::c_void)
                    == MEM_read64(
                        ip.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void
                    )
            {
                let matchEnd_0: *const BYTE = if matchIndex3 < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let lowMatchPtr_0: *const BYTE = if matchIndex3 < prefixStartIndex {
                    dictStart
                } else {
                    prefixStart
                };
                mLength = ZSTD_count_2segments(
                    ip.offset(9 as ::core::ffi::c_int as isize),
                    match3.offset(8 as ::core::ffi::c_int as isize),
                    iend,
                    matchEnd_0,
                    prefixStart,
                )
                .wrapping_add(8 as size_t);
                ip = ip.offset(1);
                offset_0 = curr.wrapping_add(1 as U32).wrapping_sub(matchIndex3);
                while (ip > anchor) as ::core::ffi::c_int
                    & (match3 > lowMatchPtr_0) as ::core::ffi::c_int
                    != 0
                    && *ip.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == *match3.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                {
                    ip = ip.offset(-1);
                    match3 = match3.offset(-1);
                    mLength = mLength.wrapping_add(1);
                }
            } else {
                let matchEnd_1: *const BYTE = if matchIndex < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let lowMatchPtr_1: *const BYTE = if matchIndex < prefixStartIndex {
                    dictStart
                } else {
                    prefixStart
                };
                mLength = ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    match_0.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    matchEnd_1,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
                offset_0 = curr.wrapping_sub(matchIndex);
                while (ip > anchor) as ::core::ffi::c_int
                    & (match_0 > lowMatchPtr_1) as ::core::ffi::c_int
                    != 0
                    && *ip.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == *match_0.offset(-(1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int
                {
                    ip = ip.offset(-1);
                    match_0 = match_0.offset(-1);
                    mLength = mLength.wrapping_add(1);
                }
            }
            offset_2 = offset_1;
            offset_1 = offset_0;
            ZSTD_storeSeq(
                seqStore,
                ip.offset_from(anchor) as ::core::ffi::c_long as size_t,
                anchor,
                iend,
                offset_0.wrapping_add(ZSTD_REP_NUM as U32),
                mLength,
            );
        } else {
            ip = ip.offset(
                ((ip.offset_from(anchor) as ::core::ffi::c_long >> kSearchStrength)
                    + 1 as ::core::ffi::c_long) as isize,
            );
            continue;
        }
        ip = ip.offset(mLength as isize);
        anchor = ip;
        if ip <= ilimit {
            let indexToInsert: U32 = curr.wrapping_add(2 as U32);
            *hashLong.offset(ZSTD_hashPtr(
                base.offset(indexToInsert as isize) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as isize) = indexToInsert;
            *hashLong.offset(ZSTD_hashPtr(
                ip.offset(-(2 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hBitsL,
                8 as U32,
            ) as isize) = ip
                .offset(-(2 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            *hashSmall.offset(ZSTD_hashPtr(
                base.offset(indexToInsert as isize) as *const ::core::ffi::c_void,
                hBitsS,
                mls,
            ) as isize) = indexToInsert;
            *hashSmall.offset(ZSTD_hashPtr(
                ip.offset(-(1 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hBitsS,
                mls,
            ) as isize) = ip
                .offset(-(1 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            while ip <= ilimit {
                let current2: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let mut repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase.offset(repIndex2 as isize)
                } else {
                    base.offset(repIndex2 as isize)
                };
                if !(ZSTD_index_overlap_check(prefixStartIndex, repIndex2)
                    & (offset_2 <= current2.wrapping_sub(dictStartIndex)) as ::core::ffi::c_int
                    != 0
                    && MEM_read32(repMatch2 as *const ::core::ffi::c_void)
                        == MEM_read32(ip as *const ::core::ffi::c_void))
                {
                    break;
                }
                let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let repLength2: size_t = (ZSTD_count_2segments(
                    ip.offset(4 as ::core::ffi::c_int as isize),
                    repMatch2.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repEnd2,
                    prefixStart,
                ) as size_t)
                    .wrapping_add(4 as size_t);
                let tmpOffset: U32 = offset_2;
                offset_2 = offset_1;
                offset_1 = tmpOffset;
                ZSTD_storeSeq(
                    seqStore,
                    0 as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE as U32,
                    repLength2,
                );
                *hashSmall
                    .offset(ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsS, mls) as isize) =
                    current2;
                *hashLong.offset(
                    ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBitsL, 8 as U32) as isize,
                ) = current2;
                ip = ip.offset(repLength2 as isize);
                anchor = ip;
            }
        }
    }
    *rep.offset(0 as ::core::ffi::c_int as isize) = offset_1;
    *rep.offset(1 as ::core::ffi::c_int as isize) = offset_2;
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict_4(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 4 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict_5(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 5 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict_6(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 6 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict_7(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_doubleFast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 7 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_doubleFast_extDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch as U32;
    match mls {
        5 => {
            return ZSTD_compressBlock_doubleFast_extDict_5(ms, seqStore, rep, src, srcSize);
        }
        6 => {
            return ZSTD_compressBlock_doubleFast_extDict_6(ms, seqStore, rep, src, srcSize);
        }
        7 => {
            return ZSTD_compressBlock_doubleFast_extDict_7(ms, seqStore, rep, src, srcSize);
        }
        4 | _ => {
            return ZSTD_compressBlock_doubleFast_extDict_4(ms, seqStore, rep, src, srcSize);
        }
    };
}
