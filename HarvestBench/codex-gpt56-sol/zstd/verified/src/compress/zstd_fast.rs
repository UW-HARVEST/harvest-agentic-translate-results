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
pub type ZSTD_match4Found =
    Option<unsafe extern "C" fn(*const BYTE, *const BYTE, U32, U32) -> ::core::ffi::c_int>;
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
unsafe extern "C" fn ZSTD_fillHashTableForCDict(
    mut ms: *mut ZSTD_MatchState_t,
    end: *const ::core::ffi::c_void,
    mut dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hBits: U32 = ((*cParams).hashLog as U32).wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS as U32);
    let mls: U32 = (*cParams).minMatch as U32;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.offset((*ms).nextToUpdate as isize);
    let iend: *const BYTE = (end as *const BYTE).offset(-(HASH_READ_SIZE as isize));
    let fastHashFillStep: U32 = 3 as U32;
    while ip.offset(fastHashFillStep as isize) < iend.offset(2 as ::core::ffi::c_int as isize) {
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let hashAndTag: size_t =
            ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBits, mls) as size_t;
        ZSTD_writeTaggedIndex(hashTable, hashAndTag, curr);
        if !(dtlm as ::core::ffi::c_uint
            == ZSTD_dtlm_fast as ::core::ffi::c_int as ::core::ffi::c_uint)
        {
            let mut p: U32 = 0;
            p = 1 as U32;
            while p < fastHashFillStep {
                let hashAndTag_0: size_t = ZSTD_hashPtr(
                    ip.offset(p as isize) as *const ::core::ffi::c_void,
                    hBits,
                    mls,
                ) as size_t;
                if *hashTable.offset((hashAndTag_0 >> ZSTD_SHORT_CACHE_TAG_BITS) as isize)
                    == 0 as U32
                {
                    ZSTD_writeTaggedIndex(hashTable, hashAndTag_0, curr.wrapping_add(p));
                }
                p = p.wrapping_add(1);
            }
        }
        ip = ip.offset(fastHashFillStep as isize);
    }
}
unsafe extern "C" fn ZSTD_fillHashTableForCCtx(
    mut ms: *mut ZSTD_MatchState_t,
    end: *const ::core::ffi::c_void,
    mut dtlm: ZSTD_dictTableLoadMethod_e,
) {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hBits: U32 = (*cParams).hashLog as U32;
    let mls: U32 = (*cParams).minMatch as U32;
    let base: *const BYTE = (*ms).window.base;
    let mut ip: *const BYTE = base.offset((*ms).nextToUpdate as isize);
    let iend: *const BYTE = (end as *const BYTE).offset(-(HASH_READ_SIZE as isize));
    let fastHashFillStep: U32 = 3 as U32;
    while ip.offset(fastHashFillStep as isize) < iend.offset(2 as ::core::ffi::c_int as isize) {
        let curr: U32 = ip.offset_from(base) as ::core::ffi::c_long as U32;
        let hash0: size_t = ZSTD_hashPtr(ip as *const ::core::ffi::c_void, hBits, mls) as size_t;
        *hashTable.offset(hash0 as isize) = curr;
        if !(dtlm as ::core::ffi::c_uint
            == ZSTD_dtlm_fast as ::core::ffi::c_int as ::core::ffi::c_uint)
        {
            let mut p: U32 = 0;
            p = 1 as U32;
            while p < fastHashFillStep {
                let hash: size_t = ZSTD_hashPtr(
                    ip.offset(p as isize) as *const ::core::ffi::c_void,
                    hBits,
                    mls,
                ) as size_t;
                if *hashTable.offset(hash as isize) == 0 as U32 {
                    *hashTable.offset(hash as isize) = curr.wrapping_add(p);
                }
                p = p.wrapping_add(1);
            }
        }
        ip = ip.offset(fastHashFillStep as isize);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fillHashTable(
    mut ms: *mut ZSTD_MatchState_t,
    end: *const ::core::ffi::c_void,
    mut dtlm: ZSTD_dictTableLoadMethod_e,
    mut tfp: ZSTD_tableFillPurpose_e,
) {
    if tfp as ::core::ffi::c_uint == ZSTD_tfp_forCDict as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_fillHashTableForCDict(ms, end, dtlm);
    } else {
        ZSTD_fillHashTableForCCtx(ms, end, dtlm);
    };
}
unsafe extern "C" fn ZSTD_match4Found_cmov(
    mut currentPtr: *const BYTE,
    mut matchAddress: *const BYTE,
    mut matchIdx: U32,
    mut idxLowLimit: U32,
) -> ::core::ffi::c_int {
    static mut dummy: [BYTE; 4] = [
        0x12 as ::core::ffi::c_int as BYTE,
        0x34 as ::core::ffi::c_int as BYTE,
        0x56 as ::core::ffi::c_int as BYTE,
        0x78 as ::core::ffi::c_int as BYTE,
    ];
    let mut mvalAddr: *const BYTE = ZSTD_selectAddr(
        matchIdx,
        idxLowLimit,
        matchAddress,
        &raw const dummy as *const BYTE,
    );
    if MEM_read32(currentPtr as *const ::core::ffi::c_void)
        != MEM_read32(mvalAddr as *const ::core::ffi::c_void)
    {
        return 0 as ::core::ffi::c_int;
    }
    asm!("\n", options(preserves_flags, att_syntax));
    return (matchIdx >= idxLowLimit) as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTD_match4Found_branch(
    mut currentPtr: *const BYTE,
    mut matchAddress: *const BYTE,
    mut matchIdx: U32,
    mut idxLowLimit: U32,
) -> ::core::ffi::c_int {
    let mut mval: U32 = 0;
    if matchIdx >= idxLowLimit {
        mval = MEM_read32(matchAddress as *const ::core::ffi::c_void);
    } else {
        mval = MEM_read32(currentPtr as *const ::core::ffi::c_void) ^ 1 as U32;
    }
    return (MEM_read32(currentPtr as *const ::core::ffi::c_void) == mval) as ::core::ffi::c_int;
}
#[inline(always)]
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mls: U32,
    mut useCmov: ::core::ffi::c_int,
) -> size_t {
    let mut current_block: u64;
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog as U32;
    let stepSize: size_t = (*cParams)
        .targetLength
        .wrapping_add(((*cParams).targetLength == 0) as ::core::ffi::c_int as ::core::ffi::c_uint)
        .wrapping_add(1 as ::core::ffi::c_uint) as size_t;
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let endIndex: U32 =
        (istart.offset_from(base) as ::core::ffi::c_long as size_t).wrapping_add(srcSize) as U32;
    let prefixStartIndex: U32 =
        ZSTD_getLowestPrefixIndex(ms, endIndex, (*cParams).windowLog) as U32;
    let prefixStart: *const BYTE = base.offset(prefixStartIndex as isize);
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(HASH_READ_SIZE as isize));
    let mut anchor: *const BYTE = istart;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut ip2: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut ip3: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut current0: U32 = 0;
    let mut rep_offset1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut rep_offset2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    let mut offsetSaved1: U32 = 0 as U32;
    let mut offsetSaved2: U32 = 0 as U32;
    let mut hash0: size_t = 0;
    let mut hash1: size_t = 0;
    let mut matchIdx: U32 = 0;
    let mut offcode: U32 = 0;
    let mut match0: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut mLength: size_t = 0;
    let mut step: size_t = 0;
    let mut nextStep: *const BYTE = ::core::ptr::null::<BYTE>();
    let kStepIncr: size_t =
        ((1 as ::core::ffi::c_int) << kSearchStrength - 1 as ::core::ffi::c_int) as size_t;
    let matchFound: ZSTD_match4Found = if useCmov != 0 {
        Some(
            ZSTD_match4Found_cmov
                as unsafe extern "C" fn(*const BYTE, *const BYTE, U32, U32) -> ::core::ffi::c_int,
        )
    } else {
        Some(
            ZSTD_match4Found_branch
                as unsafe extern "C" fn(*const BYTE, *const BYTE, U32, U32) -> ::core::ffi::c_int,
        )
    };
    ip0 = ip0.offset((ip0 == prefixStart) as ::core::ffi::c_int as isize);
    let curr: U32 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
    let windowLow: U32 = ZSTD_getLowestPrefixIndex(ms, curr, (*cParams).windowLog) as U32;
    let maxRep: U32 = curr.wrapping_sub(windowLow);
    if rep_offset2 > maxRep {
        offsetSaved2 = rep_offset2;
        rep_offset2 = 0 as U32;
    }
    if rep_offset1 > maxRep {
        offsetSaved1 = rep_offset1;
        rep_offset1 = 0 as U32;
    }
    '__start: loop {
        step = stepSize;
        nextStep = ip0.offset(kStepIncr as isize);
        ip1 = ip0.offset(1 as ::core::ffi::c_int as isize);
        ip2 = ip0.offset(step as isize);
        ip3 = ip2.offset(1 as ::core::ffi::c_int as isize);
        if ip3 >= ilimit {
            break;
        }
        hash0 = ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, hlog, mls);
        hash1 = ZSTD_hashPtr(ip1 as *const ::core::ffi::c_void, hlog, mls);
        matchIdx = *hashTable.offset(hash0 as isize);
        loop {
            let rval: U32 =
                MEM_read32(ip2.offset(-(rep_offset1 as isize)) as *const ::core::ffi::c_void)
                    as U32;
            current0 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
            *hashTable.offset(hash0 as isize) = current0;
            if (MEM_read32(ip2 as *const ::core::ffi::c_void) == rval) as ::core::ffi::c_int
                & (rep_offset1 > 0 as U32) as ::core::ffi::c_int
                != 0
            {
                ip0 = ip2;
                match0 = ip0.offset(-(rep_offset1 as isize));
                mLength = (*ip0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                    == *match0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int)
                    as ::core::ffi::c_int as size_t;
                ip0 = ip0.offset(-(mLength as isize));
                match0 = match0.offset(-(mLength as isize));
                offcode = REPCODE1_TO_OFFBASE as U32;
                mLength = (mLength as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong)
                    as size_t as size_t;
                *hashTable.offset(hash1 as isize) =
                    ip1.offset_from(base) as ::core::ffi::c_long as U32;
                current_block = 2834630252673068682;
                break;
            } else if matchFound.expect("non-null function pointer")(
                ip0,
                base.offset(matchIdx as isize),
                matchIdx,
                prefixStartIndex,
            ) != 0
            {
                *hashTable.offset(hash1 as isize) =
                    ip1.offset_from(base) as ::core::ffi::c_long as U32;
                current_block = 13545933595378752019;
                break;
            } else {
                matchIdx = *hashTable.offset(hash1 as isize);
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const ::core::ffi::c_void, hlog, mls);
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip3;
                current0 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
                *hashTable.offset(hash0 as isize) = current0;
                if matchFound.expect("non-null function pointer")(
                    ip0,
                    base.offset(matchIdx as isize),
                    matchIdx,
                    prefixStartIndex,
                ) != 0
                {
                    if step <= 4 as size_t {
                        *hashTable.offset(hash1 as isize) =
                            ip1.offset_from(base) as ::core::ffi::c_long as U32;
                    }
                    current_block = 13545933595378752019;
                    break;
                } else {
                    matchIdx = *hashTable.offset(hash1 as isize);
                    hash0 = hash1;
                    hash1 = ZSTD_hashPtr(ip2 as *const ::core::ffi::c_void, hlog, mls);
                    ip0 = ip1;
                    ip1 = ip2;
                    ip2 = ip0.offset(step as isize);
                    ip3 = ip1.offset(step as isize);
                    if ip2 >= nextStep {
                        step = step.wrapping_add(1);
                        nextStep = nextStep.offset(kStepIncr as isize);
                    }
                    if !(ip3 < ilimit) {
                        break '__start;
                    }
                }
            }
        }
        match current_block {
            13545933595378752019 => {
                match0 = base.offset(matchIdx as isize);
                rep_offset2 = rep_offset1;
                rep_offset1 = ip0.offset_from(match0) as ::core::ffi::c_long as U32;
                offcode = (rep_offset1 as ::core::ffi::c_uint)
                    .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                    as U32;
                mLength = 4 as size_t;
                while (ip0 > anchor) as ::core::ffi::c_int
                    & (match0 > prefixStart) as ::core::ffi::c_int
                    != 0
                    && *ip0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == *match0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                {
                    ip0 = ip0.offset(-1);
                    match0 = match0.offset(-1);
                    mLength = mLength.wrapping_add(1);
                }
            }
            _ => {}
        }
        mLength = (mLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count(
            ip0.offset(mLength as isize),
            match0.offset(mLength as isize),
            iend,
        ) as ::core::ffi::c_ulong) as size_t as size_t;
        ZSTD_storeSeq(
            seqStore,
            ip0.offset_from(anchor) as ::core::ffi::c_long as size_t,
            anchor,
            iend,
            offcode,
            mLength,
        );
        ip0 = ip0.offset(mLength as isize);
        anchor = ip0;
        if ip0 <= ilimit {
            *hashTable.offset(ZSTD_hashPtr(
                base.offset(current0 as isize)
                    .offset(2 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
                hlog,
                mls,
            ) as isize) = current0.wrapping_add(2 as U32);
            *hashTable.offset(ZSTD_hashPtr(
                ip0.offset(-(2 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hlog,
                mls,
            ) as isize) = ip0
                .offset(-(2 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            if rep_offset2 > 0 as U32 {
                while ip0 <= ilimit
                    && MEM_read32(ip0 as *const ::core::ffi::c_void)
                        == MEM_read32(
                            ip0.offset(-(rep_offset2 as isize)) as *const ::core::ffi::c_void
                        )
                {
                    let rLength: size_t = (ZSTD_count(
                        ip0.offset(4 as ::core::ffi::c_int as isize),
                        ip0.offset(4 as ::core::ffi::c_int as isize)
                            .offset(-(rep_offset2 as isize)),
                        iend,
                    ) as size_t)
                        .wrapping_add(4 as size_t);
                    let tmpOff: U32 = rep_offset2;
                    rep_offset2 = rep_offset1;
                    rep_offset1 = tmpOff;
                    *hashTable.offset(
                        ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, hlog, mls) as isize
                    ) = ip0.offset_from(base) as ::core::ffi::c_long as U32;
                    ip0 = ip0.offset(rLength as isize);
                    ZSTD_storeSeq(
                        seqStore,
                        0 as size_t,
                        anchor,
                        iend,
                        REPCODE1_TO_OFFBASE as U32,
                        rLength,
                    );
                    anchor = ip0;
                }
            }
        }
    }
    offsetSaved2 = if offsetSaved1 != 0 as U32 && rep_offset1 != 0 as U32 {
        offsetSaved1
    } else {
        offsetSaved2
    };
    *rep.offset(0 as ::core::ffi::c_int as isize) = if rep_offset1 != 0 {
        rep_offset1
    } else {
        offsetSaved1
    };
    *rep.offset(1 as ::core::ffi::c_int as isize) = if rep_offset2 != 0 {
        rep_offset2
    } else {
        offsetSaved2
    };
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_4_1(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        4 as U32,
        1 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_5_1(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        5 as U32,
        1 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_6_1(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        6 as U32,
        1 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_7_1(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        7 as U32,
        1 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_4_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        4 as U32,
        0 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_5_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        5 as U32,
        0 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_6_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        6 as U32,
        0 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_noDict_7_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_noDict_generic(
        ms,
        seqStore,
        rep,
        src,
        srcSize,
        7 as U32,
        0 as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mml: U32 = (*ms).cParams.minMatch as U32;
    let useCmov: ::core::ffi::c_int =
        ((*ms).cParams.windowLog < 19 as ::core::ffi::c_uint) as ::core::ffi::c_int;
    if useCmov != 0 {
        match mml {
            5 => {
                return ZSTD_compressBlock_fast_noDict_5_1(ms, seqStore, rep, src, srcSize);
            }
            6 => {
                return ZSTD_compressBlock_fast_noDict_6_1(ms, seqStore, rep, src, srcSize);
            }
            7 => {
                return ZSTD_compressBlock_fast_noDict_7_1(ms, seqStore, rep, src, srcSize);
            }
            4 | _ => {
                return ZSTD_compressBlock_fast_noDict_4_1(ms, seqStore, rep, src, srcSize);
            }
        }
    } else {
        match mml {
            5 => {
                return ZSTD_compressBlock_fast_noDict_5_0(ms, seqStore, rep, src, srcSize);
            }
            6 => {
                return ZSTD_compressBlock_fast_noDict_6_0(ms, seqStore, rep, src, srcSize);
            }
            7 => {
                return ZSTD_compressBlock_fast_noDict_7_0(ms, seqStore, rep, src, srcSize);
            }
            4 | _ => {
                return ZSTD_compressBlock_fast_noDict_4_0(ms, seqStore, rep, src, srcSize);
            }
        }
    };
}
#[inline(always)]
unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mls: U32,
    hasStep: U32,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog as U32;
    let stepSize: U32 = ((*cParams).targetLength as U32)
        .wrapping_add(((*cParams).targetLength == 0) as ::core::ffi::c_int as U32);
    let base: *const BYTE = (*ms).window.base;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE = ip0.offset(stepSize as isize);
    let mut anchor: *const BYTE = istart;
    let prefixStartIndex: U32 = (*ms).window.dictLimit;
    let prefixStart: *const BYTE = base.offset(prefixStartIndex as isize);
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(HASH_READ_SIZE as isize));
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    let dms: *const ZSTD_MatchState_t = (*ms).dictMatchState;
    let dictCParams: *const ZSTD_compressionParameters = &raw const (*dms).cParams;
    let dictHashTable: *const U32 = (*dms).hashTable;
    let dictStartIndex: U32 = (*dms).window.dictLimit;
    let dictBase: *const BYTE = (*dms).window.base;
    let dictStart: *const BYTE = dictBase.offset(dictStartIndex as isize);
    let dictEnd: *const BYTE = (*dms).window.nextSrc;
    let dictIndexDelta: U32 =
        prefixStartIndex.wrapping_sub(dictEnd.offset_from(dictBase) as ::core::ffi::c_long as U32);
    let dictAndPrefixLength: U32 = dictEnd
        .offset(istart.offset_from(prefixStart) as ::core::ffi::c_long as isize)
        .offset_from(dictStart) as ::core::ffi::c_long as U32;
    let dictHBits: U32 =
        ((*dictCParams).hashLog as U32).wrapping_add(ZSTD_SHORT_CACHE_TAG_BITS as U32);
    let maxDistance: U32 = (1 as U32) << (*cParams).windowLog;
    let endIndex: U32 =
        (istart.offset_from(base) as ::core::ffi::c_long as size_t).wrapping_add(srcSize) as U32;
    if (*ms).prefetchCDictTables != 0 {
        let hashTableBytes: size_t = ((1 as ::core::ffi::c_int as size_t)
            << (*dictCParams).hashLog)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t);
        let _ptr: *const ::core::ffi::c_char = dictHashTable as *const ::core::ffi::c_char;
        let _size: size_t = hashTableBytes;
        let mut _pos: size_t = 0;
        _pos = 0 as size_t;
        while _pos < _size {
            _pos = (_pos as ::core::ffi::c_ulong)
                .wrapping_add(CACHELINE_SIZE as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    }
    ip0 = ip0.offset((dictAndPrefixLength == 0 as U32) as ::core::ffi::c_int as isize);
    's_135: while ip1 <= ilimit {
        let mut mLength: size_t = 0;
        let mut hash0: size_t = ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, hlog, mls);
        let dictHashAndTag0: size_t =
            ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, dictHBits, mls) as size_t;
        let mut dictMatchIndexAndTag: U32 =
            *dictHashTable.offset((dictHashAndTag0 >> ZSTD_SHORT_CACHE_TAG_BITS) as isize);
        let mut dictTagsMatch: ::core::ffi::c_int =
            ZSTD_comparePackedTags(dictMatchIndexAndTag as size_t, dictHashAndTag0);
        let mut matchIndex: U32 = *hashTable.offset(hash0 as isize);
        let mut curr: U32 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
        let mut step: size_t = stepSize as size_t;
        let kStepIncr: size_t = ((1 as ::core::ffi::c_int) << kSearchStrength) as size_t;
        let mut nextStep: *const BYTE = ip0.offset(kStepIncr as isize);
        loop {
            let mut match_0: *const BYTE = base.offset(matchIndex as isize);
            let repIndex: U32 = curr.wrapping_add(1 as U32).wrapping_sub(offset_1);
            let mut repMatch: *const BYTE = if repIndex < prefixStartIndex {
                dictBase.offset(repIndex.wrapping_sub(dictIndexDelta) as isize)
            } else {
                base.offset(repIndex as isize)
            };
            let hash1: size_t =
                ZSTD_hashPtr(ip1 as *const ::core::ffi::c_void, hlog, mls) as size_t;
            let dictHashAndTag1: size_t =
                ZSTD_hashPtr(ip1 as *const ::core::ffi::c_void, dictHBits, mls) as size_t;
            *hashTable.offset(hash0 as isize) = curr;
            if ZSTD_index_overlap_check(prefixStartIndex, repIndex) != 0
                && MEM_read32(repMatch as *const ::core::ffi::c_void)
                    == MEM_read32(
                        ip0.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void
                    )
            {
                let repMatchEnd: *const BYTE = if repIndex < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                mLength = ZSTD_count_2segments(
                    ip0.offset(1 as ::core::ffi::c_int as isize)
                        .offset(4 as ::core::ffi::c_int as isize),
                    repMatch.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repMatchEnd,
                    prefixStart,
                )
                .wrapping_add(4 as size_t);
                ip0 = ip0.offset(1);
                ZSTD_storeSeq(
                    seqStore,
                    ip0.offset_from(anchor) as ::core::ffi::c_long as size_t,
                    anchor,
                    iend,
                    REPCODE1_TO_OFFBASE as U32,
                    mLength,
                );
                break;
            } else {
                if dictTagsMatch != 0 {
                    let dictMatchIndex: U32 = dictMatchIndexAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
                    let mut dictMatch: *const BYTE = dictBase.offset(dictMatchIndex as isize);
                    if dictMatchIndex > dictStartIndex
                        && MEM_read32(dictMatch as *const ::core::ffi::c_void)
                            == MEM_read32(ip0 as *const ::core::ffi::c_void)
                    {
                        if matchIndex <= prefixStartIndex {
                            let offset: U32 = curr
                                .wrapping_sub(dictMatchIndex)
                                .wrapping_sub(dictIndexDelta);
                            mLength = ZSTD_count_2segments(
                                ip0.offset(4 as ::core::ffi::c_int as isize),
                                dictMatch.offset(4 as ::core::ffi::c_int as isize),
                                iend,
                                dictEnd,
                                prefixStart,
                            )
                            .wrapping_add(4 as size_t);
                            while (ip0 > anchor) as ::core::ffi::c_int
                                & (dictMatch > dictStart) as ::core::ffi::c_int
                                != 0
                                && *ip0.offset(-(1 as ::core::ffi::c_int) as isize)
                                    as ::core::ffi::c_int
                                    == *dictMatch.offset(-(1 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_int
                            {
                                ip0 = ip0.offset(-1);
                                dictMatch = dictMatch.offset(-1);
                                mLength = mLength.wrapping_add(1);
                            }
                            offset_2 = offset_1;
                            offset_1 = offset;
                            ZSTD_storeSeq(
                                seqStore,
                                ip0.offset_from(anchor) as ::core::ffi::c_long as size_t,
                                anchor,
                                iend,
                                offset.wrapping_add(ZSTD_REP_NUM as U32),
                                mLength,
                            );
                            break;
                        }
                    }
                }
                if ZSTD_match4Found_cmov(ip0, match_0, matchIndex, prefixStartIndex) != 0 {
                    let offset_0: U32 = ip0.offset_from(match_0) as ::core::ffi::c_long as U32;
                    mLength = ZSTD_count(
                        ip0.offset(4 as ::core::ffi::c_int as isize),
                        match_0.offset(4 as ::core::ffi::c_int as isize),
                        iend,
                    )
                    .wrapping_add(4 as size_t);
                    while (ip0 > anchor) as ::core::ffi::c_int
                        & (match_0 > prefixStart) as ::core::ffi::c_int
                        != 0
                        && *ip0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                            == *match_0.offset(-(1 as ::core::ffi::c_int) as isize)
                                as ::core::ffi::c_int
                    {
                        ip0 = ip0.offset(-1);
                        match_0 = match_0.offset(-1);
                        mLength = mLength.wrapping_add(1);
                    }
                    offset_2 = offset_1;
                    offset_1 = offset_0;
                    ZSTD_storeSeq(
                        seqStore,
                        ip0.offset_from(anchor) as ::core::ffi::c_long as size_t,
                        anchor,
                        iend,
                        offset_0.wrapping_add(ZSTD_REP_NUM as U32),
                        mLength,
                    );
                    break;
                } else {
                    dictMatchIndexAndTag = *dictHashTable
                        .offset((dictHashAndTag1 >> ZSTD_SHORT_CACHE_TAG_BITS) as isize);
                    dictTagsMatch =
                        ZSTD_comparePackedTags(dictMatchIndexAndTag as size_t, dictHashAndTag1);
                    matchIndex = *hashTable.offset(hash1 as isize);
                    if ip1 >= nextStep {
                        step = step.wrapping_add(1);
                        nextStep = nextStep.offset(kStepIncr as isize);
                    }
                    ip0 = ip1;
                    ip1 = ip1.offset(step as isize);
                    if ip1 > ilimit {
                        break 's_135;
                    }
                    curr = ip0.offset_from(base) as ::core::ffi::c_long as U32;
                    hash0 = hash1;
                }
            }
        }
        ip0 = ip0.offset(mLength as isize);
        anchor = ip0;
        if ip0 <= ilimit {
            *hashTable.offset(ZSTD_hashPtr(
                base.offset(curr as isize)
                    .offset(2 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
                hlog,
                mls,
            ) as isize) = curr.wrapping_add(2 as U32);
            *hashTable.offset(ZSTD_hashPtr(
                ip0.offset(-(2 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hlog,
                mls,
            ) as isize) = ip0
                .offset(-(2 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            while ip0 <= ilimit {
                let current2: U32 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
                let repIndex2: U32 = current2.wrapping_sub(offset_2);
                let mut repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase
                        .offset(-(dictIndexDelta as isize))
                        .offset(repIndex2 as isize)
                } else {
                    base.offset(repIndex2 as isize)
                };
                if !(ZSTD_index_overlap_check(prefixStartIndex, repIndex2) != 0
                    && MEM_read32(repMatch2 as *const ::core::ffi::c_void)
                        == MEM_read32(ip0 as *const ::core::ffi::c_void))
                {
                    break;
                }
                let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let repLength2: size_t = (ZSTD_count_2segments(
                    ip0.offset(4 as ::core::ffi::c_int as isize),
                    repMatch2.offset(4 as ::core::ffi::c_int as isize),
                    iend,
                    repEnd2,
                    prefixStart,
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
                *hashTable
                    .offset(ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, hlog, mls) as isize) =
                    current2;
                ip0 = ip0.offset(repLength2 as isize);
                anchor = ip0;
            }
        }
        ip1 = ip0.offset(stepSize as isize);
    }
    *rep.offset(0 as ::core::ffi::c_int as isize) = offset_1;
    *rep.offset(1 as ::core::ffi::c_int as isize) = offset_2;
    return iend.offset_from(anchor) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState_4_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 4 as U32, 0 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState_5_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 5 as U32, 0 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState_6_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 6 as U32, 0 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState_7_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_dictMatchState_generic(
        ms, seqStore, rep, src, srcSize, 7 as U32, 0 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_dictMatchState(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch as U32;
    match mls {
        5 => {
            return ZSTD_compressBlock_fast_dictMatchState_5_0(ms, seqStore, rep, src, srcSize);
        }
        6 => {
            return ZSTD_compressBlock_fast_dictMatchState_6_0(ms, seqStore, rep, src, srcSize);
        }
        7 => {
            return ZSTD_compressBlock_fast_dictMatchState_7_0(ms, seqStore, rep, src, srcSize);
        }
        4 | _ => {
            return ZSTD_compressBlock_fast_dictMatchState_4_0(ms, seqStore, rep, src, srcSize);
        }
    };
}
unsafe extern "C" fn ZSTD_compressBlock_fast_extDict_generic(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mls: U32,
    hasStep: U32,
) -> size_t {
    let mut current_block: u64;
    let cParams: *const ZSTD_compressionParameters = &raw mut (*ms).cParams;
    let hashTable: *mut U32 = (*ms).hashTable;
    let hlog: U32 = (*cParams).hashLog as U32;
    let stepSize: size_t = (*cParams)
        .targetLength
        .wrapping_add(((*cParams).targetLength == 0) as ::core::ffi::c_int as ::core::ffi::c_uint)
        .wrapping_add(1 as ::core::ffi::c_uint) as size_t;
    let base: *const BYTE = (*ms).window.base;
    let dictBase: *const BYTE = (*ms).window.dictBase;
    let istart: *const BYTE = src as *const BYTE;
    let mut anchor: *const BYTE = istart;
    let endIndex: U32 =
        (istart.offset_from(base) as ::core::ffi::c_long as size_t).wrapping_add(srcSize) as U32;
    let lowLimit: U32 = ZSTD_getLowestMatchIndex(ms, endIndex, (*cParams).windowLog) as U32;
    let dictStartIndex: U32 = lowLimit;
    let dictStart: *const BYTE = dictBase.offset(dictStartIndex as isize);
    let dictLimit: U32 = (*ms).window.dictLimit;
    let prefixStartIndex: U32 = if dictLimit < lowLimit {
        lowLimit
    } else {
        dictLimit
    };
    let prefixStart: *const BYTE = base.offset(prefixStartIndex as isize);
    let dictEnd: *const BYTE = dictBase.offset(prefixStartIndex as isize);
    let iend: *const BYTE = istart.offset(srcSize as isize);
    let ilimit: *const BYTE = iend.offset(-(8 as ::core::ffi::c_int as isize));
    let mut offset_1: U32 = *rep.offset(0 as ::core::ffi::c_int as isize);
    let mut offset_2: U32 = *rep.offset(1 as ::core::ffi::c_int as isize);
    let mut offsetSaved1: U32 = 0 as U32;
    let mut offsetSaved2: U32 = 0 as U32;
    let mut ip0: *const BYTE = istart;
    let mut ip1: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut ip2: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut ip3: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut current0: U32 = 0;
    let mut hash0: size_t = 0;
    let mut hash1: size_t = 0;
    let mut idx: U32 = 0;
    let mut idxBase: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut offcode: U32 = 0;
    let mut match0: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut mLength: size_t = 0;
    let mut matchEnd: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut step: size_t = 0;
    let mut nextStep: *const BYTE = ::core::ptr::null::<BYTE>();
    let kStepIncr: size_t =
        ((1 as ::core::ffi::c_int) << kSearchStrength - 1 as ::core::ffi::c_int) as size_t;
    if prefixStartIndex == dictStartIndex {
        return ZSTD_compressBlock_fast(ms, seqStore, rep, src, srcSize);
    }
    let curr: U32 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
    let maxRep: U32 = curr.wrapping_sub(dictStartIndex);
    if offset_2 >= maxRep {
        offsetSaved2 = offset_2;
        offset_2 = 0 as U32;
    }
    if offset_1 >= maxRep {
        offsetSaved1 = offset_1;
        offset_1 = 0 as U32;
    }
    '__start: loop {
        step = stepSize;
        nextStep = ip0.offset(kStepIncr as isize);
        ip1 = ip0.offset(1 as ::core::ffi::c_int as isize);
        ip2 = ip0.offset(step as isize);
        ip3 = ip2.offset(1 as ::core::ffi::c_int as isize);
        if ip3 >= ilimit {
            break;
        }
        hash0 = ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, hlog, mls);
        hash1 = ZSTD_hashPtr(ip1 as *const ::core::ffi::c_void, hlog, mls);
        idx = *hashTable.offset(hash0 as isize);
        idxBase = if idx < prefixStartIndex {
            dictBase
        } else {
            base
        };
        loop {
            let current2: U32 = ip2.offset_from(base) as ::core::ffi::c_long as U32;
            let repIndex: U32 = current2.wrapping_sub(offset_1);
            let repBase: *const BYTE = if repIndex < prefixStartIndex {
                dictBase
            } else {
                base
            };
            let mut rval: U32 = 0;
            if (prefixStartIndex.wrapping_sub(repIndex) >= 4 as U32) as ::core::ffi::c_int
                & (offset_1 > 0 as U32) as ::core::ffi::c_int
                != 0
            {
                rval = MEM_read32(repBase.offset(repIndex as isize) as *const ::core::ffi::c_void);
            } else {
                rval = MEM_read32(ip2 as *const ::core::ffi::c_void) ^ 1 as U32;
            }
            current0 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
            *hashTable.offset(hash0 as isize) = current0;
            if MEM_read32(ip2 as *const ::core::ffi::c_void) == rval {
                ip0 = ip2;
                match0 = repBase.offset(repIndex as isize);
                matchEnd = if repIndex < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                mLength = (*ip0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                    == *match0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int)
                    as ::core::ffi::c_int as size_t;
                ip0 = ip0.offset(-(mLength as isize));
                match0 = match0.offset(-(mLength as isize));
                offcode = REPCODE1_TO_OFFBASE as U32;
                mLength = (mLength as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong)
                    as size_t as size_t;
                current_block = 1352918242886884122;
                break;
            } else {
                let mval: U32 = if idx >= dictStartIndex {
                    MEM_read32(idxBase.offset(idx as isize) as *const ::core::ffi::c_void) as U32
                } else {
                    MEM_read32(ip0 as *const ::core::ffi::c_void) as U32 ^ 1 as U32
                };
                if MEM_read32(ip0 as *const ::core::ffi::c_void) == mval {
                    current_block = 12302453269099463970;
                    break;
                }
                idx = *hashTable.offset(hash1 as isize);
                idxBase = if idx < prefixStartIndex {
                    dictBase
                } else {
                    base
                };
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const ::core::ffi::c_void, hlog, mls);
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip3;
                current0 = ip0.offset_from(base) as ::core::ffi::c_long as U32;
                *hashTable.offset(hash0 as isize) = current0;
                let mval_0: U32 = if idx >= dictStartIndex {
                    MEM_read32(idxBase.offset(idx as isize) as *const ::core::ffi::c_void) as U32
                } else {
                    MEM_read32(ip0 as *const ::core::ffi::c_void) as U32 ^ 1 as U32
                };
                if MEM_read32(ip0 as *const ::core::ffi::c_void) == mval_0 {
                    current_block = 12302453269099463970;
                    break;
                }
                idx = *hashTable.offset(hash1 as isize);
                idxBase = if idx < prefixStartIndex {
                    dictBase
                } else {
                    base
                };
                hash0 = hash1;
                hash1 = ZSTD_hashPtr(ip2 as *const ::core::ffi::c_void, hlog, mls);
                ip0 = ip1;
                ip1 = ip2;
                ip2 = ip0.offset(step as isize);
                ip3 = ip1.offset(step as isize);
                if ip2 >= nextStep {
                    step = step.wrapping_add(1);
                    nextStep = nextStep.offset(kStepIncr as isize);
                }
                if !(ip3 < ilimit) {
                    break '__start;
                }
            }
        }
        match current_block {
            12302453269099463970 => {
                let offset: U32 = current0.wrapping_sub(idx);
                let lowMatchPtr: *const BYTE = if idx < prefixStartIndex {
                    dictStart
                } else {
                    prefixStart
                };
                matchEnd = if idx < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                match0 = idxBase.offset(idx as isize);
                offset_2 = offset_1;
                offset_1 = offset;
                offcode = (offset as ::core::ffi::c_uint)
                    .wrapping_add(ZSTD_REP_NUM as ::core::ffi::c_uint)
                    as U32;
                mLength = 4 as size_t;
                while (ip0 > anchor) as ::core::ffi::c_int
                    & (match0 > lowMatchPtr) as ::core::ffi::c_int
                    != 0
                    && *ip0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == *match0.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                {
                    ip0 = ip0.offset(-1);
                    match0 = match0.offset(-1);
                    mLength = mLength.wrapping_add(1);
                }
            }
            _ => {}
        }
        mLength = (mLength as ::core::ffi::c_ulong).wrapping_add(ZSTD_count_2segments(
            ip0.offset(mLength as isize),
            match0.offset(mLength as isize),
            iend,
            matchEnd,
            prefixStart,
        ) as ::core::ffi::c_ulong) as size_t as size_t;
        ZSTD_storeSeq(
            seqStore,
            ip0.offset_from(anchor) as ::core::ffi::c_long as size_t,
            anchor,
            iend,
            offcode,
            mLength,
        );
        ip0 = ip0.offset(mLength as isize);
        anchor = ip0;
        if ip1 < ip0 {
            *hashTable.offset(hash1 as isize) = ip1.offset_from(base) as ::core::ffi::c_long as U32;
        }
        if ip0 <= ilimit {
            *hashTable.offset(ZSTD_hashPtr(
                base.offset(current0 as isize)
                    .offset(2 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
                hlog,
                mls,
            ) as isize) = current0.wrapping_add(2 as U32);
            *hashTable.offset(ZSTD_hashPtr(
                ip0.offset(-(2 as ::core::ffi::c_int as isize)) as *const ::core::ffi::c_void,
                hlog,
                mls,
            ) as isize) = ip0
                .offset(-(2 as ::core::ffi::c_int as isize))
                .offset_from(base) as ::core::ffi::c_long as U32;
            while ip0 <= ilimit {
                let repIndex2: U32 =
                    (ip0.offset_from(base) as ::core::ffi::c_long as U32).wrapping_sub(offset_2);
                let repMatch2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictBase.offset(repIndex2 as isize)
                } else {
                    base.offset(repIndex2 as isize)
                };
                if !(ZSTD_index_overlap_check(prefixStartIndex, repIndex2)
                    & (offset_2 > 0 as U32) as ::core::ffi::c_int
                    != 0
                    && MEM_read32(repMatch2 as *const ::core::ffi::c_void)
                        == MEM_read32(ip0 as *const ::core::ffi::c_void))
                {
                    break;
                }
                let repEnd2: *const BYTE = if repIndex2 < prefixStartIndex {
                    dictEnd
                } else {
                    iend
                };
                let repLength2: size_t = (ZSTD_count_2segments(
                    ip0.offset(4 as ::core::ffi::c_int as isize),
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
                *hashTable
                    .offset(ZSTD_hashPtr(ip0 as *const ::core::ffi::c_void, hlog, mls) as isize) =
                    ip0.offset_from(base) as ::core::ffi::c_long as U32;
                ip0 = ip0.offset(repLength2 as isize);
                anchor = ip0;
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
unsafe extern "C" fn ZSTD_compressBlock_fast_extDict_4_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 4 as U32, 0 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_extDict_5_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 5 as U32, 0 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_extDict_6_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 6 as U32, 0 as U32,
    );
}
unsafe extern "C" fn ZSTD_compressBlock_fast_extDict_7_0(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_compressBlock_fast_extDict_generic(
        ms, seqStore, rep, src, srcSize, 7 as U32, 0 as U32,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_fast_extDict(
    mut ms: *mut ZSTD_MatchState_t,
    mut seqStore: *mut SeqStore_t,
    mut rep: *mut U32,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mls: U32 = (*ms).cParams.minMatch as U32;
    match mls {
        5 => return ZSTD_compressBlock_fast_extDict_5_0(ms, seqStore, rep, src, srcSize),
        6 => return ZSTD_compressBlock_fast_extDict_6_0(ms, seqStore, rep, src, srcSize),
        7 => return ZSTD_compressBlock_fast_extDict_7_0(ms, seqStore, rep, src, srcSize),
        4 | _ => {
            return ZSTD_compressBlock_fast_extDict_4_0(ms, seqStore, rep, src, srcSize);
        }
    };
}
