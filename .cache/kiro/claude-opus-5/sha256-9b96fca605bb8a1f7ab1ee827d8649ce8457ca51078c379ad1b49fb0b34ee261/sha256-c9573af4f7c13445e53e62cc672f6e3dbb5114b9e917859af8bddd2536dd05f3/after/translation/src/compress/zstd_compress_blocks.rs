//! Translation of `compress/zstd_compress.c` lines 2687-4523:
//! sequence-store handling, block entropy statistics, block compression and
//! the block splitter.  Literal, semantics-preserving transliteration.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::bits::*;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;
use crate::compress::hist::*;
use crate::compress::huf_compress::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_compress_literals::*;
use crate::compress::zstd_compress_sequences::*;
use crate::compress::zstd_compress_superblock::*;
use crate::compress::zstd_ldm::*;

use core::ffi::{c_int, c_uint, c_void};
use core::ptr::null_mut;

/* ===== Cross-file imports ===== */
/* Part-1 helpers (c_src lines 1-2686 -> zstd_compress.rs) */
use crate::compress::zstd_compress::ZSTD_rowMatchFinderUsed;
/* Part-3 helper (c_src lines 4526-7843 -> zstd_compress_frame.rs) */
use crate::compress::zstd_compress_frame::ZSTD_transferSequences_wBlockDelim;

/* Exported C symbols implemented by other parts, linked at runtime */
extern "C" {
    fn ZSTD_compress2(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_CCtx_getParameter(
        cctx: *const ZSTD_CCtx,
        param: ZSTD_cParameter,
        value: *mut c_int,
    ) -> size_t;
    fn ZSTD_compressBound(srcSize: size_t) -> size_t;
}

/* Block compressor functions (all exported C symbols; defined across the
 * zstd_fast / zstd_double_fast / zstd_lazy / zstd_opt modules). Declared here
 * as extern "C" so the dispatch tables can take their addresses. */
extern "C" {
    fn ZSTD_compressBlock_fast(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_fast_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_fast_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_doubleFast(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_doubleFast_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_doubleFast_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_extDict_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_dictMatchState_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_dedicatedDictSearch(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_greedy_dedicatedDictSearch_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_extDict_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_dictMatchState_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_dedicatedDictSearch(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy_dedicatedDictSearch_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_extDict_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_dictMatchState_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_dedicatedDictSearch(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_lazy2_dedicatedDictSearch_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btlazy2(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btlazy2_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btlazy2_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btopt(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btopt_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btopt_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btultra(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btultra_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btultra_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressBlock_btultra2(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
}

const KB: size_t = 1 << 10;

/* ***************************************************************
*  Block entropic compression
*********************************************************/

/* See doc/zstd_compression_format.md for detailed format description */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int {
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let llCodeTable: *mut BYTE = (*seqStorePtr).llCode;
    let ofCodeTable: *mut BYTE = (*seqStorePtr).ofCode;
    let mlCodeTable: *mut BYTE = (*seqStorePtr).mlCode;
    let nbSeq: U32 =
        (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as U32;
    let mut u: U32;
    let mut longOffsets: c_int = 0;
    u = 0;
    while u < nbSeq {
        let llv: U32 = (*sequences.wrapping_add(u as usize)).litLength as U32;
        let ofCode: U32 = ZSTD_highbit32((*sequences.wrapping_add(u as usize)).offBase);
        let mlv: U32 = (*sequences.wrapping_add(u as usize)).mlBase as U32;
        *llCodeTable.wrapping_add(u as usize) = ZSTD_LLcode(llv) as BYTE;
        *ofCodeTable.wrapping_add(u as usize) = ofCode as BYTE;
        *mlCodeTable.wrapping_add(u as usize) = ZSTD_MLcode(mlv) as BYTE;
        if MEM_32bits() != 0 && ofCode >= STREAM_ACCUMULATOR_MIN() {
            longOffsets = 1;
        }
        u += 1;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_literalLength {
        *llCodeTable.wrapping_add((*seqStorePtr).longLengthPos as usize) = MaxLL as BYTE;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_matchLength {
        *mlCodeTable.wrapping_add((*seqStorePtr).longLengthPos as usize) = MaxML as BYTE;
    }
    longOffsets
}

/* ZSTD_useTargetCBlockSize():
 * Returns if target compressed block size param is being used. */
pub unsafe fn ZSTD_useTargetCBlockSize(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).targetCBlockSize != 0) as c_int
}

/* ZSTD_blockSplitterEnabled():
 * Returns if block splitting param is being used. */
pub unsafe fn ZSTD_blockSplitterEnabled(cctxParams: *mut ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).postBlockSplitter == ZSTD_ps_enable) as c_int
}

/* Type returned by ZSTD_buildSequencesStatistics containing finalized symbol
 * encoding types and size of the sequences statistics. */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_symbolEncodingTypeStats_t {
    pub LLtype: U32,
    pub Offtype: U32,
    pub MLtype: U32,
    pub size: size_t,
    pub lastCountSize: size_t, /* Accounts for bug in 1.3.4. */
    pub longOffsets: c_int,
}

/* ZSTD_buildSequencesStatistics():
 * Returns a ZSTD_symbolEncodingTypeStats_t, or a zstd error code in the `size` field. */
pub unsafe fn ZSTD_buildSequencesStatistics(
    seqStorePtr: *const SeqStore_t,
    nbSeq: size_t,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    dst: *mut BYTE,
    dstEnd: *const BYTE,
    strategy: ZSTD_strategy,
    countWorkspace: *mut c_uint,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: size_t,
) -> ZSTD_symbolEncodingTypeStats_t {
    let ostart: *mut BYTE = dst;
    let oend: *const BYTE = dstEnd;
    let mut op: *mut BYTE = ostart;
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).matchlengthCTable.as_mut_ptr();
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let mut stats: ZSTD_symbolEncodingTypeStats_t = core::mem::zeroed();

    stats.lastCountSize = 0;
    /* convert length/distances into codes */
    stats.longOffsets = ZSTD_seqToCodes(seqStorePtr);
    let _ = oend;
    /* build CTable for Literal Lengths */
    {
        let mut max: c_uint = MaxLL;
        let mostFrequent: size_t = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            llCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
        (*nextEntropy).litlength_repeatMode = (*prevEntropy).litlength_repeatMode;
        stats.LLtype = ZSTD_selectEncodingType(
            &mut (*nextEntropy).litlength_repeatMode,
            countWorkspace,
            max,
            mostFrequent,
            nbSeq,
            LLFSELog as c_uint,
            (*prevEntropy).litlengthCTable.as_ptr(),
            LL_defaultNorm.as_ptr(),
            LL_defaultNormLog,
            ZSTD_defaultAllowed,
            strategy,
        ) as U32;
        {
            let countSize: size_t = ZSTD_buildCTable(
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                CTable_LitLength,
                LLFSELog as c_uint,
                stats.LLtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                llCodeTable,
                nbSeq,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                MaxLL,
                (*prevEntropy).litlengthCTable.as_ptr(),
                core::mem::size_of_val(&(*prevEntropy).litlengthCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ZSTD_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.LLtype == set_compressed {
                stats.lastCountSize = countSize;
            }
            op = op.wrapping_add(countSize);
        }
    }
    /* build CTable for Offsets */
    {
        let mut max: c_uint = MaxOff;
        let mostFrequent: size_t = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            ofCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
        /* We can only use the basic table if max <= DefaultMaxOff, otherwise the offsets are too large */
        let defaultPolicy: ZSTD_DefaultPolicy_e = if max <= DefaultMaxOff {
            ZSTD_defaultAllowed
        } else {
            ZSTD_defaultDisallowed
        };
        (*nextEntropy).offcode_repeatMode = (*prevEntropy).offcode_repeatMode;
        stats.Offtype = ZSTD_selectEncodingType(
            &mut (*nextEntropy).offcode_repeatMode,
            countWorkspace,
            max,
            mostFrequent,
            nbSeq,
            OffFSELog as c_uint,
            (*prevEntropy).offcodeCTable.as_ptr(),
            OF_defaultNorm.as_ptr(),
            OF_defaultNormLog,
            defaultPolicy,
            strategy,
        ) as U32;
        {
            let countSize: size_t = ZSTD_buildCTable(
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                CTable_OffsetBits,
                OffFSELog as c_uint,
                stats.Offtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                ofCodeTable,
                nbSeq,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                DefaultMaxOff,
                (*prevEntropy).offcodeCTable.as_ptr(),
                core::mem::size_of_val(&(*prevEntropy).offcodeCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ZSTD_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.Offtype == set_compressed {
                stats.lastCountSize = countSize;
            }
            op = op.wrapping_add(countSize);
        }
    }
    /* build CTable for MatchLengths */
    {
        let mut max: c_uint = MaxML;
        let mostFrequent: size_t = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            mlCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
        (*nextEntropy).matchlength_repeatMode = (*prevEntropy).matchlength_repeatMode;
        stats.MLtype = ZSTD_selectEncodingType(
            &mut (*nextEntropy).matchlength_repeatMode,
            countWorkspace,
            max,
            mostFrequent,
            nbSeq,
            MLFSELog as c_uint,
            (*prevEntropy).matchlengthCTable.as_ptr(),
            ML_defaultNorm.as_ptr(),
            ML_defaultNormLog,
            ZSTD_defaultAllowed,
            strategy,
        ) as U32;
        {
            let countSize: size_t = ZSTD_buildCTable(
                op as *mut c_void,
                oend.offset_from(op) as size_t,
                CTable_MatchLength,
                MLFSELog as c_uint,
                stats.MLtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                mlCodeTable,
                nbSeq,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                MaxML,
                (*prevEntropy).matchlengthCTable.as_ptr(),
                core::mem::size_of_val(&(*prevEntropy).matchlengthCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ZSTD_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.MLtype == set_compressed {
                stats.lastCountSize = countSize;
            }
            op = op.wrapping_add(countSize);
        }
    }
    stats.size = op.offset_from(ostart) as size_t;
    stats
}

/* ZSTD_entropyCompressSeqStore_internal():
 * compresses both literals and sequences
 * Returns compressed size of block, or a zstd error. */
const SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO: size_t = 20;

pub unsafe fn ZSTD_entropyCompressSeqStore_internal(
    dst: *mut c_void,
    dstCapacity: size_t,
    literals: *const c_void,
    litSize: size_t,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let count: *mut c_uint = entropyWorkspace as *mut c_uint;
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).fse.litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).fse.offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).fse.matchlengthCTable.as_mut_ptr();
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let nbSeq: size_t =
        (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as size_t;
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let lastCountSize: size_t;
    let mut longOffsets: c_int = 0;

    let entropyWorkspace: *mut c_void = count.wrapping_add((MaxSeq + 1) as usize) as *mut c_void;
    let entropyWkspSize: size_t =
        entropyWkspSize - (MaxSeq as size_t + 1) * core::mem::size_of::<c_uint>();

    /* Compress literals */
    {
        let numSequences: size_t =
            (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as size_t;
        /* Base suspicion of uncompressibility on ratio of literals to sequences */
        let suspectUncompressible: c_int = ((numSequences == 0)
            || (litSize / numSequences >= SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO))
            as c_int;

        let cSize: size_t = ZSTD_compressLiterals(
            op as *mut c_void,
            dstCapacity,
            literals,
            litSize,
            entropyWorkspace,
            entropyWkspSize,
            &(*prevEntropy).huf,
            &mut (*nextEntropy).huf,
            (*cctxParams).cParams.strategy,
            ZSTD_literalsCompressionIsDisabled(cctxParams),
            suspectUncompressible,
            bmi2,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        op = op.wrapping_add(cSize);
    }

    /* Sequences Header */
    if (oend.offset_from(op) as size_t) < 3 /*max nbSeq Size*/ + 1
    /*seqHead*/
    {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbSeq < 128 {
        *op = nbSeq as BYTE;
        op = op.wrapping_add(1);
    } else if nbSeq < LONGNBSEQ as size_t {
        *op.wrapping_add(0) = ((nbSeq >> 8) + 0x80) as BYTE;
        *op.wrapping_add(1) = nbSeq as BYTE;
        op = op.wrapping_add(2);
    } else {
        *op.wrapping_add(0) = 0xFF;
        MEM_writeLE16(op.wrapping_add(1), (nbSeq - LONGNBSEQ as size_t) as U16);
        op = op.wrapping_add(3);
    }
    if nbSeq == 0 {
        /* Copy the old tables over as if we repeated them */
        ZSTD_memcpy(
            &mut (*nextEntropy).fse as *mut ZSTD_fseCTables_t as *mut u8,
            &(*prevEntropy).fse as *const ZSTD_fseCTables_t as *const u8,
            core::mem::size_of::<ZSTD_fseCTables_t>(),
        );
        return op.offset_from(ostart) as size_t;
    }
    {
        let seqHead: *mut BYTE = op;
        op = op.wrapping_add(1);
        /* build stats for sequences */
        let stats: ZSTD_symbolEncodingTypeStats_t = ZSTD_buildSequencesStatistics(
            seqStorePtr,
            nbSeq,
            &(*prevEntropy).fse,
            &mut (*nextEntropy).fse,
            op,
            oend,
            strategy,
            count,
            entropyWorkspace,
            entropyWkspSize,
        );
        if ERR_isError(stats.size) != 0 {
            return stats.size;
        }
        *seqHead = ((stats.LLtype << 6) + (stats.Offtype << 4) + (stats.MLtype << 2)) as BYTE;
        lastCountSize = stats.lastCountSize;
        op = op.wrapping_add(stats.size);
        longOffsets = stats.longOffsets;
    }

    {
        let bitstreamSize: size_t = ZSTD_encodeSequences(
            op as *mut c_void,
            oend.offset_from(op) as size_t,
            CTable_MatchLength,
            mlCodeTable,
            CTable_OffsetBits,
            ofCodeTable,
            CTable_LitLength,
            llCodeTable,
            sequences,
            nbSeq,
            longOffsets,
            bmi2,
        );
        if ERR_isError(bitstreamSize) != 0 {
            return bitstreamSize;
        }
        op = op.wrapping_add(bitstreamSize);
        /* zstd versions <= 1.3.4 mistakenly report corruption when
         * FSE_readNCount() receives a buffer < 4 bytes. */
        if lastCountSize != 0 && (lastCountSize + bitstreamSize) < 4 {
            return 0;
        }
    }

    op.offset_from(ostart) as size_t
}

pub unsafe fn ZSTD_entropyCompressSeqStore_wExtLitBuffer(
    dst: *mut c_void,
    dstCapacity: size_t,
    literals: *const c_void,
    litSize: size_t,
    blockSize: size_t,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    let cSize: size_t = ZSTD_entropyCompressSeqStore_internal(
        dst,
        dstCapacity,
        literals,
        litSize,
        seqStorePtr,
        prevEntropy,
        nextEntropy,
        cctxParams,
        entropyWorkspace,
        entropyWkspSize,
        bmi2,
    );
    if cSize == 0 {
        return 0;
    }
    /* When srcSize <= dstCapacity, there is enough space to write a raw
     * uncompressed block. Since we ran out of space, block must be not
     * compressible, so fall back to raw uncompressed block. */
    if ((cSize == ERROR(ZSTD_error_dstSize_tooSmall)) as size_t & (blockSize <= dstCapacity) as size_t)
        != 0
    {
        return 0; /* block not compressed */
    }
    if ERR_isError(cSize) != 0 {
        return cSize;
    }

    /* Check compressibility */
    {
        let maxCSize: size_t = blockSize - ZSTD_minGain(blockSize, (*cctxParams).cParams.strategy);
        if cSize >= maxCSize {
            return 0;
        } /* block not compressed */
    }
    cSize
}

pub unsafe fn ZSTD_entropyCompressSeqStore(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: size_t,
    srcSize: size_t,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    ZSTD_entropyCompressSeqStore_wExtLitBuffer(
        dst,
        dstCapacity,
        (*seqStorePtr).litStart as *const c_void,
        (*seqStorePtr).lit.offset_from((*seqStorePtr).litStart) as size_t,
        srcSize,
        seqStorePtr,
        prevEntropy,
        nextEntropy,
        cctxParams,
        entropyWorkspace,
        entropyWkspSize,
        bmi2,
    )
}

/* ZSTD_selectBlockCompressor() :
 * Not static, but internal use only (used by long distance matcher)
 * assumption : strat is a valid strategy */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectBlockCompressor(
    strat: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    dictMode: ZSTD_dictMode_e,
) -> ZSTD_BlockCompressor_f {
    static blockCompressor: [[ZSTD_BlockCompressor_f; (ZSTD_STRATEGY_MAX + 1) as usize]; 4] = [
        [
            Some(ZSTD_compressBlock_fast), /* default for 0 */
            Some(ZSTD_compressBlock_fast),
            Some(ZSTD_compressBlock_doubleFast),
            Some(ZSTD_compressBlock_greedy),
            Some(ZSTD_compressBlock_lazy),
            Some(ZSTD_compressBlock_lazy2),
            Some(ZSTD_compressBlock_btlazy2),
            Some(ZSTD_compressBlock_btopt),
            Some(ZSTD_compressBlock_btultra),
            Some(ZSTD_compressBlock_btultra2),
        ],
        [
            Some(ZSTD_compressBlock_fast_extDict), /* default for 0 */
            Some(ZSTD_compressBlock_fast_extDict),
            Some(ZSTD_compressBlock_doubleFast_extDict),
            Some(ZSTD_compressBlock_greedy_extDict),
            Some(ZSTD_compressBlock_lazy_extDict),
            Some(ZSTD_compressBlock_lazy2_extDict),
            Some(ZSTD_compressBlock_btlazy2_extDict),
            Some(ZSTD_compressBlock_btopt_extDict),
            Some(ZSTD_compressBlock_btultra_extDict),
            Some(ZSTD_compressBlock_btultra_extDict),
        ],
        [
            Some(ZSTD_compressBlock_fast_dictMatchState), /* default for 0 */
            Some(ZSTD_compressBlock_fast_dictMatchState),
            Some(ZSTD_compressBlock_doubleFast_dictMatchState),
            Some(ZSTD_compressBlock_greedy_dictMatchState),
            Some(ZSTD_compressBlock_lazy_dictMatchState),
            Some(ZSTD_compressBlock_lazy2_dictMatchState),
            Some(ZSTD_compressBlock_btlazy2_dictMatchState),
            Some(ZSTD_compressBlock_btopt_dictMatchState),
            Some(ZSTD_compressBlock_btultra_dictMatchState),
            Some(ZSTD_compressBlock_btultra_dictMatchState),
        ],
        [
            None, /* default for 0 */
            None,
            None,
            Some(ZSTD_compressBlock_greedy_dedicatedDictSearch),
            Some(ZSTD_compressBlock_lazy_dedicatedDictSearch),
            Some(ZSTD_compressBlock_lazy2_dedicatedDictSearch),
            None,
            None,
            None,
            None,
        ],
    ];
    let selectedCompressor: ZSTD_BlockCompressor_f;

    if ZSTD_rowMatchFinderUsed(strat, useRowMatchFinder) != 0 {
        static rowBasedBlockCompressors: [[ZSTD_BlockCompressor_f; 3]; 4] = [
            [
                Some(ZSTD_compressBlock_greedy_row),
                Some(ZSTD_compressBlock_lazy_row),
                Some(ZSTD_compressBlock_lazy2_row),
            ],
            [
                Some(ZSTD_compressBlock_greedy_extDict_row),
                Some(ZSTD_compressBlock_lazy_extDict_row),
                Some(ZSTD_compressBlock_lazy2_extDict_row),
            ],
            [
                Some(ZSTD_compressBlock_greedy_dictMatchState_row),
                Some(ZSTD_compressBlock_lazy_dictMatchState_row),
                Some(ZSTD_compressBlock_lazy2_dictMatchState_row),
            ],
            [
                Some(ZSTD_compressBlock_greedy_dedicatedDictSearch_row),
                Some(ZSTD_compressBlock_lazy_dedicatedDictSearch_row),
                Some(ZSTD_compressBlock_lazy2_dedicatedDictSearch_row),
            ],
        ];
        selectedCompressor = rowBasedBlockCompressors[dictMode as usize]
            [(strat as c_int - ZSTD_greedy as c_int) as usize];
    } else {
        selectedCompressor = blockCompressor[dictMode as usize][strat as usize];
    }
    selectedCompressor
}

pub unsafe fn ZSTD_storeLastLiterals(
    seqStorePtr: *mut SeqStore_t,
    anchor: *const BYTE,
    lastLLSize: size_t,
) {
    ZSTD_memcpy((*seqStorePtr).lit, anchor, lastLLSize);
    (*seqStorePtr).lit = (*seqStorePtr).lit.wrapping_add(lastLLSize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetSeqStore(ssPtr: *mut SeqStore_t) {
    (*ssPtr).lit = (*ssPtr).litStart;
    (*ssPtr).sequences = (*ssPtr).sequencesStart;
    (*ssPtr).longLengthType = ZSTD_llt_none;
}

/* ZSTD_postProcessSequenceProducerResult() :
 * Validates and post-processes sequences obtained through the external
 * matchfinder API. Returns the number of sequences after post-processing,
 * or an error code. */
pub unsafe fn ZSTD_postProcessSequenceProducerResult(
    outSeqs: *mut ZSTD_Sequence,
    nbExternalSeqs: size_t,
    outSeqsCapacity: size_t,
    srcSize: size_t,
) -> size_t {
    if nbExternalSeqs > outSeqsCapacity {
        return ERROR(ZSTD_error_sequenceProducer_failed);
    }

    if nbExternalSeqs == 0 && srcSize > 0 {
        return ERROR(ZSTD_error_sequenceProducer_failed);
    }

    if srcSize == 0 {
        ZSTD_memset(
            &mut *outSeqs.wrapping_add(0) as *mut ZSTD_Sequence as *mut u8,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        return 1;
    }

    {
        let lastSeq: ZSTD_Sequence = *outSeqs.wrapping_add(nbExternalSeqs - 1);

        /* We can return early if lastSeq is already a block delimiter. */
        if lastSeq.offset == 0 && lastSeq.matchLength == 0 {
            return nbExternalSeqs;
        }

        /* This error condition is only possible if the external matchfinder
         * produced an invalid parse, by definition of ZSTD_sequenceBound(). */
        if nbExternalSeqs == outSeqsCapacity {
            return ERROR(ZSTD_error_sequenceProducer_failed);
        }

        /* lastSeq is not a block delimiter, so we need to append one. */
        ZSTD_memset(
            &mut *outSeqs.wrapping_add(nbExternalSeqs) as *mut ZSTD_Sequence as *mut u8,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        nbExternalSeqs + 1
    }
}

/* ZSTD_fastSequenceLengthSum() :
 * Returns sum(litLen) + sum(matchLen) + lastLits for *seqBuf*. */
pub unsafe fn ZSTD_fastSequenceLengthSum(
    seqBuf: *const ZSTD_Sequence,
    seqBufSize: size_t,
) -> size_t {
    let mut matchLenSum: size_t;
    let mut litLenSum: size_t;
    let mut i: size_t;
    matchLenSum = 0;
    litLenSum = 0;
    i = 0;
    while i < seqBufSize {
        litLenSum += (*seqBuf.wrapping_add(i)).litLength as size_t;
        matchLenSum += (*seqBuf.wrapping_add(i)).matchLength as size_t;
        i += 1;
    }
    litLenSum + matchLenSum
}

/**
 * Function to validate sequences produced by a block compressor.
 * (DEBUGLEVEL == 0 -> body is a no-op) */
pub unsafe fn ZSTD_validateSeqStore(
    seqStore: *const SeqStore_t,
    cParams: *const ZSTD_compressionParameters,
) {
    let _ = seqStore;
    let _ = cParams;
}

pub type ZSTD_BuildSeqStore_e = c_uint;
pub const ZSTDbss_compress: ZSTD_BuildSeqStore_e = 0;
pub const ZSTDbss_noCompress: ZSTD_BuildSeqStore_e = 1;

pub unsafe fn ZSTD_buildSeqStore(
    zc: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ms: *mut ZSTD_MatchState_t = &mut (*zc).blockState.matchState;
    /* TODO: See 3090. We reduced MIN_CBLOCK_SIZE from 3 to 2 so to compensate we are adding
     * additional 1. We need to revisit and change this logic to be more consistent */
    if srcSize < MIN_CBLOCK_SIZE as size_t + ZSTD_blockHeaderSize + 1 + 1 {
        if (*zc).appliedParams.cParams.strategy >= ZSTD_btopt {
            ZSTD_ldm_skipRawSeqStoreBytes(&mut (*zc).externSeqStore, srcSize);
        } else {
            ZSTD_ldm_skipSequences(
                &mut (*zc).externSeqStore,
                srcSize,
                (*zc).appliedParams.cParams.minMatch,
            );
        }
        return ZSTDbss_noCompress as size_t; /* don't even attempt compression below a certain srcSize */
    }
    ZSTD_resetSeqStore(&mut (*zc).seqStore);
    /* required for optimal parser to read stats from dictionary */
    (*ms).opt.symbolCosts = &(*(*zc).blockState.prevCBlock).entropy;
    /* tell the optimal parser how we expect to compress literals */
    (*ms).opt.literalCompressionMode = (*zc).appliedParams.literalCompressionMode;

    /* limited update after a very long match */
    {
        let base: *const BYTE = (*ms).window.base;
        let istart: *const BYTE = src as *const BYTE;
        let curr: U32 = istart.offset_from(base) as U32;
        if curr > (*ms).nextToUpdate + 384 {
            (*ms).nextToUpdate = curr
                - MIN(192, (curr - (*ms).nextToUpdate - 384) as size_t) as U32;
        }
    }

    /* select and store sequences */
    {
        let dictMode: ZSTD_dictMode_e = ZSTD_matchState_dictMode(ms);
        let lastLLSize: size_t;
        {
            let mut i: c_int = 0;
            while i < ZSTD_REP_NUM as c_int {
                (*(*zc).blockState.nextCBlock).rep[i as usize] =
                    (*(*zc).blockState.prevCBlock).rep[i as usize];
                i += 1;
            }
        }
        if (*zc).externSeqStore.pos < (*zc).externSeqStore.size {
            /* External matchfinder + LDM is technically possible, just not implemented yet. */
            if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            /* Updates ldmSeqStore.pos */
            lastLLSize = ZSTD_ldm_blockCompress(
                &mut (*zc).externSeqStore,
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
        } else if (*zc).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
            let mut ldmSeqStore: RawSeqStore_t = kNullRawSeqStore;

            /* External matchfinder + LDM is technically possible, just not implemented yet. */
            if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            ldmSeqStore.seq = (*zc).ldmSequences;
            ldmSeqStore.capacity = (*zc).maxNbLdmSequences;
            /* Updates ldmSeqStore.size */
            {
                let err = ZSTD_ldm_generateSequences(
                    &mut (*zc).ldmState,
                    &mut ldmSeqStore,
                    &(*zc).appliedParams.ldmParams,
                    src,
                    srcSize,
                );
                if ERR_isError(err) != 0 {
                    return err;
                }
            }
            /* Updates ldmSeqStore.pos */
            lastLLSize = ZSTD_ldm_blockCompress(
                &mut ldmSeqStore,
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
        } else if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
            {
                let windowSize: U32 = (1u32) << (*zc).appliedParams.cParams.windowLog;

                let nbExternalSeqs: size_t =
                    ((*zc).appliedParams.extSeqProdFunc.unwrap())(
                        (*zc).appliedParams.extSeqProdState,
                        (*zc).extSeqBuf,
                        (*zc).extSeqBufCapacity,
                        src,
                        srcSize,
                        null_mut(),
                        0, /* dict and dictSize, currently not supported */
                        (*zc).appliedParams.compressionLevel,
                        windowSize as size_t,
                    );

                let nbPostProcessedSeqs: size_t = ZSTD_postProcessSequenceProducerResult(
                    (*zc).extSeqBuf,
                    nbExternalSeqs,
                    (*zc).extSeqBufCapacity,
                    srcSize,
                );

                /* Return early if there is no error, since we don't need to worry about last literals */
                if ZSTD_isError(nbPostProcessedSeqs) == 0 {
                    let mut seqPos: ZSTD_SequencePosition = ZSTD_SequencePosition {
                        idx: 0,
                        posInSequence: 0,
                        posInSrc: 0,
                    };
                    let seqLenSum: size_t =
                        ZSTD_fastSequenceLengthSum((*zc).extSeqBuf, nbPostProcessedSeqs);
                    if seqLenSum > srcSize {
                        return ERROR(ZSTD_error_externalSequences_invalid);
                    }
                    {
                        let err = ZSTD_transferSequences_wBlockDelim(
                            zc,
                            &mut seqPos,
                            (*zc).extSeqBuf,
                            nbPostProcessedSeqs,
                            src,
                            srcSize,
                            (*zc).appliedParams.searchForExternalRepcodes,
                        );
                        if ERR_isError(err) != 0 {
                            return err;
                        }
                    }
                    (*ms).ldmSeqStore = null_mut();
                    return ZSTDbss_compress as size_t;
                }

                /* Propagate the error if fallback is disabled */
                if (*zc).appliedParams.enableMatchFinderFallback == 0 {
                    return nbPostProcessedSeqs;
                }

                /* Fallback to software matchfinder */
                {
                    let blockCompressor: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
                        (*zc).appliedParams.cParams.strategy,
                        (*zc).appliedParams.useRowMatchFinder,
                        dictMode,
                    );
                    (*ms).ldmSeqStore = null_mut();
                    lastLLSize = (blockCompressor.unwrap())(
                        ms,
                        &mut (*zc).seqStore,
                        (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                        src,
                        srcSize,
                    );
                }
            }
        } else {
            /* not long range mode and no external matchfinder */
            let blockCompressor: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
                (*zc).appliedParams.cParams.strategy,
                (*zc).appliedParams.useRowMatchFinder,
                dictMode,
            );
            (*ms).ldmSeqStore = null_mut();
            lastLLSize = (blockCompressor.unwrap())(
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                src,
                srcSize,
            );
        }
        {
            let lastLiterals: *const BYTE =
                (src as *const BYTE).wrapping_add(srcSize).wrapping_sub(lastLLSize);
            ZSTD_storeLastLiterals(&mut (*zc).seqStore, lastLiterals, lastLLSize);
        }
    }
    ZSTD_validateSeqStore(&(*zc).seqStore, &(*zc).appliedParams.cParams);
    ZSTDbss_compress as size_t
}

pub unsafe fn ZSTD_copyBlockSequences(
    seqCollector: *mut SeqCollector,
    seqStore: *const SeqStore_t,
    prevRepcodes: *const U32, /* U32[ZSTD_REP_NUM] */
) -> size_t {
    let inSeqs: *const SeqDef = (*seqStore).sequencesStart;
    let nbInSequences: size_t = (*seqStore).sequences.offset_from(inSeqs) as size_t;
    let nbInLiterals: size_t =
        (*seqStore).lit.offset_from((*seqStore).litStart) as size_t;

    let outSeqs: *mut ZSTD_Sequence = if (*seqCollector).seqIndex == 0 {
        (*seqCollector).seqStart
    } else {
        (*seqCollector).seqStart.wrapping_add((*seqCollector).seqIndex)
    };
    let nbOutSequences: size_t = nbInSequences + 1;
    let mut nbOutLiterals: size_t = 0;
    let mut repcodes: Repcodes_t = core::mem::zeroed();
    let mut i: size_t;

    /* Bounds check that we have enough space for every input sequence
     * and the block delimiter */
    if nbOutSequences > ((*seqCollector).maxSequences - (*seqCollector).seqIndex) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    ZSTD_memcpy(
        &mut repcodes as *mut Repcodes_t as *mut u8,
        prevRepcodes as *const u8,
        core::mem::size_of::<Repcodes_t>(),
    );
    i = 0;
    while i < nbInSequences {
        let rawOffset: U32;
        (*outSeqs.wrapping_add(i)).litLength = (*inSeqs.wrapping_add(i)).litLength as U32;
        (*outSeqs.wrapping_add(i)).matchLength =
            (*inSeqs.wrapping_add(i)).mlBase as U32 + MINMATCH;
        (*outSeqs.wrapping_add(i)).rep = 0;

        /* Handle the possible single length >= 64K */
        if i == (*seqStore).longLengthPos as size_t {
            if (*seqStore).longLengthType == ZSTD_llt_literalLength {
                (*outSeqs.wrapping_add(i)).litLength += 0x10000;
            } else if (*seqStore).longLengthType == ZSTD_llt_matchLength {
                (*outSeqs.wrapping_add(i)).matchLength += 0x10000;
            }
        }

        /* Determine the raw offset given the offBase, which may be a repcode. */
        if OFFBASE_IS_REPCODE((*inSeqs.wrapping_add(i)).offBase) {
            let repcode: U32 = OFFBASE_TO_REPCODE((*inSeqs.wrapping_add(i)).offBase);
            (*outSeqs.wrapping_add(i)).rep = repcode;
            if (*outSeqs.wrapping_add(i)).litLength != 0 {
                rawOffset = repcodes.rep[(repcode - 1) as usize];
            } else {
                if repcode == 3 {
                    rawOffset = repcodes.rep[0] - 1;
                } else {
                    rawOffset = repcodes.rep[repcode as usize];
                }
            }
        } else {
            rawOffset = OFFBASE_TO_OFFSET((*inSeqs.wrapping_add(i)).offBase);
        }
        (*outSeqs.wrapping_add(i)).offset = rawOffset;

        /* Update repcode history for the sequence */
        ZSTD_updateRep(
            repcodes.rep.as_mut_ptr(),
            (*inSeqs.wrapping_add(i)).offBase,
            ((*inSeqs.wrapping_add(i)).litLength == 0) as U32,
        );

        nbOutLiterals += (*outSeqs.wrapping_add(i)).litLength as size_t;
        i += 1;
    }
    /* Insert last literals (if any exist) in the block as a sequence with ml == off == 0. */
    {
        let lastLLSize: size_t = nbInLiterals - nbOutLiterals;
        (*outSeqs.wrapping_add(nbInSequences)).litLength = lastLLSize as U32;
        (*outSeqs.wrapping_add(nbInSequences)).matchLength = 0;
        (*outSeqs.wrapping_add(nbInSequences)).offset = 0;
    }
    (*seqCollector).seqIndex += nbOutSequences;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sequenceBound(srcSize: size_t) -> size_t {
    let maxNbSeq: size_t = (srcSize / ZSTD_MINMATCH_MIN as size_t) + 1;
    let maxNbDelims: size_t = (srcSize / ZSTD_BLOCKSIZE_MAX_MIN as size_t) + 1;
    maxNbSeq + maxNbDelims
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_generateSequences(
    zc: *mut ZSTD_CCtx,
    outSeqs: *mut ZSTD_Sequence,
    outSeqsSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let dstCapacity: size_t = ZSTD_compressBound(srcSize);
    let dst: *mut c_void; /* Make C90 happy. */
    let mut seqCollector: SeqCollector = core::mem::zeroed();
    {
        let mut targetCBlockSize: c_int = 0;
        {
            let err = ZSTD_CCtx_getParameter(zc, ZSTD_c_targetCBlockSize, &mut targetCBlockSize);
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        if targetCBlockSize != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    }
    {
        let mut nbWorkers: c_int = 0;
        {
            let err = ZSTD_CCtx_getParameter(zc, ZSTD_c_nbWorkers, &mut nbWorkers);
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        if nbWorkers != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    }

    dst = ZSTD_customMalloc(dstCapacity, ZSTD_defaultCMem);
    if dst.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }

    seqCollector.collectSequences = 1;
    seqCollector.seqStart = outSeqs;
    seqCollector.seqIndex = 0;
    seqCollector.maxSequences = outSeqsSize;
    (*zc).seqCollector = seqCollector;

    {
        let ret: size_t = ZSTD_compress2(zc, dst, dstCapacity, src, srcSize);
        ZSTD_customFree(dst, ZSTD_defaultCMem);
        if ERR_isError(ret) != 0 {
            return ret;
        }
    }
    (*zc).seqCollector.seqIndex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_mergeBlockDelimiters(
    sequences: *mut ZSTD_Sequence,
    seqsSize: size_t,
) -> size_t {
    let mut r#in: size_t = 0;
    let mut out: size_t = 0;
    while r#in < seqsSize {
        if (*sequences.wrapping_add(r#in)).offset == 0
            && (*sequences.wrapping_add(r#in)).matchLength == 0
        {
            if r#in != seqsSize - 1 {
                (*sequences.wrapping_add(r#in + 1)).litLength +=
                    (*sequences.wrapping_add(r#in)).litLength;
            }
        } else {
            *sequences.wrapping_add(out) = *sequences.wrapping_add(r#in);
            out += 1;
        }
        r#in += 1;
    }
    out
}

/* Unrolled loop to read four size_ts of input at a time. Returns 1 if is RLE, 0 if not. */
pub unsafe fn ZSTD_isRLE(src: *const BYTE, length: size_t) -> c_int {
    let ip: *const BYTE = src;
    let value: BYTE = *ip.wrapping_add(0);
    let valueST: size_t =
        ((value as U64).wrapping_mul(0x0101010101010101u64)) as size_t;
    let unrollSize: size_t = core::mem::size_of::<size_t>() * 4;
    let unrollMask: size_t = unrollSize - 1;
    let prefixLength: size_t = length & unrollMask;
    let mut i: size_t;
    if length == 1 {
        return 1;
    }
    /* Check if prefix is RLE first before using unrolled loop */
    if prefixLength != 0
        && ZSTD_count(ip.wrapping_add(1), ip, ip.wrapping_add(prefixLength)) != prefixLength - 1
    {
        return 0;
    }
    i = prefixLength;
    while i != length {
        let mut u: size_t = 0;
        while u < unrollSize {
            if MEM_readST(ip.wrapping_add(i).wrapping_add(u)) != valueST {
                return 0;
            }
            u += core::mem::size_of::<size_t>();
        }
        i += unrollSize;
    }
    1
}

/* Returns true if the given block may be RLE. */
pub unsafe fn ZSTD_maybeRLE(seqStore: *const SeqStore_t) -> c_int {
    let nbSeqs: size_t =
        (*seqStore).sequences.offset_from((*seqStore).sequencesStart) as size_t;
    let nbLits: size_t = (*seqStore).lit.offset_from((*seqStore).litStart) as size_t;

    (nbSeqs < 4 && nbLits < 10) as c_int
}

pub unsafe fn ZSTD_blockState_confirmRepcodesAndEntropyTables(bs: *mut ZSTD_blockState_t) {
    let tmp: *mut ZSTD_compressedBlockState_t = (*bs).prevCBlock;
    (*bs).prevCBlock = (*bs).nextCBlock;
    (*bs).nextCBlock = tmp;
}

/* Writes the block header */
unsafe fn writeBlockHeader(op: *mut c_void, cSize: size_t, blockSize: size_t, lastBlock: U32) {
    let cBlockHeader: U32 = if cSize == 1 {
        lastBlock + (((bt_rle as U32)) << 1) + ((blockSize << 3) as U32)
    } else {
        lastBlock + (((bt_compressed as U32)) << 1) + ((cSize << 3) as U32)
    };
    MEM_writeLE24(op as *mut u8, cBlockHeader);
}

/** ZSTD_buildBlockEntropyStats_literals() :
 *  Builds entropy for the literals. */
const COMPRESS_LITERALS_SIZE_MIN: size_t = 63; /* heuristic */

pub unsafe fn ZSTD_buildBlockEntropyStats_literals(
    src: *mut c_void,
    srcSize: size_t,
    prevHuf: *const ZSTD_hufCTables_t,
    nextHuf: *mut ZSTD_hufCTables_t,
    hufMetadata: *mut ZSTD_hufCTablesMetadata_t,
    literalsCompressionIsDisabled: c_int,
    workspace: *mut c_void,
    wkspSize: size_t,
    hufFlags: c_int,
) -> size_t {
    let wkspStart: *mut BYTE = workspace as *mut BYTE;
    let wkspEnd: *mut BYTE = wkspStart.wrapping_add(wkspSize);
    let countWkspStart: *mut BYTE = wkspStart;
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let countWkspSize: size_t =
        (HUF_SYMBOLVALUE_MAX as size_t + 1) * core::mem::size_of::<c_uint>();
    let nodeWksp: *mut BYTE = countWkspStart.wrapping_add(countWkspSize);
    let nodeWkspSize: size_t = wkspEnd.offset_from(nodeWksp) as size_t;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX;
    let mut huffLog: U32 = LitHufLog;
    let mut repeat: HUF_repeat = (*prevHuf).repeatMode;

    /* Prepare nextEntropy assuming reusing the existing table */
    ZSTD_memcpy(
        nextHuf as *mut u8,
        prevHuf as *const u8,
        core::mem::size_of::<ZSTD_hufCTables_t>(),
    );

    if literalsCompressionIsDisabled != 0 {
        (*hufMetadata).hType = set_basic;
        return 0;
    }

    /* small ? don't even attempt compression (speed opt) */
    {
        let minLitSize: size_t = if (*prevHuf).repeatMode == HUF_repeat_valid {
            6
        } else {
            COMPRESS_LITERALS_SIZE_MIN
        };
        if srcSize <= minLitSize {
            (*hufMetadata).hType = set_basic;
            return 0;
        }
    }

    /* Scan input and build symbol stats */
    {
        let largest: size_t = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            src as *const BYTE as *const c_void,
            srcSize,
            workspace,
            wkspSize,
        );
        if ERR_isError(largest) != 0 {
            return largest;
        }
        if largest == srcSize {
            /* only one literal symbol */
            (*hufMetadata).hType = set_rle;
            return 0;
        }
        if largest <= (srcSize >> 7) + 4 {
            /* heuristic: likely not compressible */
            (*hufMetadata).hType = set_basic;
            return 0;
        }
    }

    /* Validate the previous Huffman table */
    if repeat == HUF_repeat_check
        && HUF_validateCTable((*prevHuf).CTable.as_ptr(), countWksp, maxSymbolValue) == 0
    {
        repeat = HUF_repeat_none;
    }

    /* Build Huffman Tree */
    ZSTD_memset(
        (*nextHuf).CTable.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of_val(&(*nextHuf).CTable),
    );
    huffLog = HUF_optimalTableLog(
        huffLog,
        srcSize,
        maxSymbolValue,
        nodeWksp as *mut c_void,
        nodeWkspSize,
        (*nextHuf).CTable.as_mut_ptr(),
        countWksp,
        hufFlags,
    );
    {
        let maxBits: size_t = HUF_buildCTable_wksp(
            (*nextHuf).CTable.as_mut_ptr(),
            countWksp,
            maxSymbolValue,
            huffLog,
            nodeWksp as *mut c_void,
            nodeWkspSize,
        );
        if ERR_isError(maxBits) != 0 {
            return maxBits;
        }
        huffLog = maxBits as U32;
    }
    {
        /* Build and write the CTable */
        let newCSize: size_t = HUF_estimateCompressedSize(
            (*nextHuf).CTable.as_ptr(),
            countWksp,
            maxSymbolValue,
        );
        let hSize: size_t = HUF_writeCTable_wksp(
            (*hufMetadata).hufDesBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*hufMetadata).hufDesBuffer),
            (*nextHuf).CTable.as_ptr(),
            maxSymbolValue,
            huffLog,
            nodeWksp as *mut c_void,
            nodeWkspSize,
        );
        /* Check against repeating the previous CTable */
        if repeat != HUF_repeat_none {
            let oldCSize: size_t = HUF_estimateCompressedSize(
                (*prevHuf).CTable.as_ptr(),
                countWksp,
                maxSymbolValue,
            );
            if oldCSize < srcSize && (oldCSize <= hSize + newCSize || hSize + 12 >= srcSize) {
                ZSTD_memcpy(
                    nextHuf as *mut u8,
                    prevHuf as *const u8,
                    core::mem::size_of::<ZSTD_hufCTables_t>(),
                );
                (*hufMetadata).hType = set_repeat;
                return 0;
            }
        }
        if newCSize + hSize >= srcSize {
            ZSTD_memcpy(
                nextHuf as *mut u8,
                prevHuf as *const u8,
                core::mem::size_of::<ZSTD_hufCTables_t>(),
            );
            (*hufMetadata).hType = set_basic;
            return 0;
        }
        (*hufMetadata).hType = set_compressed;
        (*nextHuf).repeatMode = HUF_repeat_check;
        hSize
    }
}

/* ZSTD_buildDummySequencesStatistics():
 * Returns a ZSTD_symbolEncodingTypeStats_t with all encoding types as set_basic,
 * and updates nextEntropy to the appropriate repeatMode. */
pub unsafe fn ZSTD_buildDummySequencesStatistics(
    nextEntropy: *mut ZSTD_fseCTables_t,
) -> ZSTD_symbolEncodingTypeStats_t {
    let stats: ZSTD_symbolEncodingTypeStats_t = ZSTD_symbolEncodingTypeStats_t {
        LLtype: set_basic,
        Offtype: set_basic,
        MLtype: set_basic,
        size: 0,
        lastCountSize: 0,
        longOffsets: 0,
    };
    (*nextEntropy).litlength_repeatMode = FSE_repeat_none;
    (*nextEntropy).offcode_repeatMode = FSE_repeat_none;
    (*nextEntropy).matchlength_repeatMode = FSE_repeat_none;
    stats
}

/** ZSTD_buildBlockEntropyStats_sequences() :
 *  Builds entropy for the sequences. */
pub unsafe fn ZSTD_buildBlockEntropyStats_sequences(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    fseMetadata: *mut ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let nbSeq: size_t =
        (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as size_t;
    let ostart: *mut BYTE = (*fseMetadata).fseTablesBuffer.as_mut_ptr();
    let oend: *const BYTE =
        ostart.wrapping_add(core::mem::size_of_val(&(*fseMetadata).fseTablesBuffer));
    let op: *mut BYTE = ostart;
    let countWorkspace: *mut c_uint = workspace as *mut c_uint;
    let entropyWorkspace: *mut c_uint = countWorkspace.wrapping_add((MaxSeq + 1) as usize);
    let entropyWorkspaceSize: size_t =
        wkspSize - (MaxSeq as size_t + 1) * core::mem::size_of::<c_uint>();
    let stats: ZSTD_symbolEncodingTypeStats_t;

    stats = if nbSeq != 0 {
        ZSTD_buildSequencesStatistics(
            seqStorePtr,
            nbSeq,
            prevEntropy,
            nextEntropy,
            op,
            oend,
            strategy,
            countWorkspace,
            entropyWorkspace as *mut c_void,
            entropyWorkspaceSize,
        )
    } else {
        ZSTD_buildDummySequencesStatistics(nextEntropy)
    };
    if ERR_isError(stats.size) != 0 {
        return stats.size;
    }
    (*fseMetadata).llType = stats.LLtype as SymbolEncodingType_e;
    (*fseMetadata).ofType = stats.Offtype as SymbolEncodingType_e;
    (*fseMetadata).mlType = stats.MLtype as SymbolEncodingType_e;
    (*fseMetadata).lastCountSize = stats.lastCountSize;
    stats.size
}

/** ZSTD_buildBlockEntropyStats() :
 *  Builds entropy for the block. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildBlockEntropyStats(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let litSize: size_t =
        (*seqStorePtr).lit.offset_from((*seqStorePtr).litStart) as size_t;
    let huf_useOptDepth: c_int =
        ((*cctxParams).cParams.strategy >= HUF_OPTIMAL_DEPTH_THRESHOLD) as c_int;
    let hufFlags: c_int = if huf_useOptDepth != 0 {
        HUF_flags_optimalDepth as c_int
    } else {
        0
    };

    (*entropyMetadata).hufMetadata.hufDesSize = ZSTD_buildBlockEntropyStats_literals(
        (*seqStorePtr).litStart as *mut c_void,
        litSize,
        &(*prevEntropy).huf,
        &mut (*nextEntropy).huf,
        &mut (*entropyMetadata).hufMetadata,
        ZSTD_literalsCompressionIsDisabled(cctxParams),
        workspace,
        wkspSize,
        hufFlags,
    );

    if ERR_isError((*entropyMetadata).hufMetadata.hufDesSize) != 0 {
        return (*entropyMetadata).hufMetadata.hufDesSize;
    }
    (*entropyMetadata).fseMetadata.fseTablesSize = ZSTD_buildBlockEntropyStats_sequences(
        seqStorePtr,
        &(*prevEntropy).fse,
        &mut (*nextEntropy).fse,
        cctxParams,
        &mut (*entropyMetadata).fseMetadata,
        workspace,
        wkspSize,
    );
    if ERR_isError((*entropyMetadata).fseMetadata.fseTablesSize) != 0 {
        return (*entropyMetadata).fseMetadata.fseTablesSize;
    }
    0
}

/* Returns the size estimate for the literals section (header + content) of a block */
pub unsafe fn ZSTD_estimateBlockSize_literal(
    literals: *const BYTE,
    litSize: size_t,
    huf: *const ZSTD_hufCTables_t,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: size_t,
    writeEntropy: c_int,
) -> size_t {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX;
    let literalSectionHeaderSize: size_t =
        3 + (litSize >= 1 * KB) as size_t + (litSize >= 16 * KB) as size_t;
    let singleStream: U32 = (litSize < 256) as U32;

    if (*hufMetadata).hType == set_basic {
        return litSize;
    } else if (*hufMetadata).hType == set_rle {
        return 1;
    } else if (*hufMetadata).hType == set_compressed || (*hufMetadata).hType == set_repeat {
        let largest: size_t = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            literals as *const BYTE as *const c_void,
            litSize,
            workspace,
            wkspSize,
        );
        if ZSTD_isError(largest) != 0 {
            return litSize;
        }
        {
            let mut cLitSizeEstimate: size_t = HUF_estimateCompressedSize(
                (*huf).CTable.as_ptr(),
                countWksp,
                maxSymbolValue,
            );
            if writeEntropy != 0 {
                cLitSizeEstimate += (*hufMetadata).hufDesSize;
            }
            if singleStream == 0 {
                cLitSizeEstimate += 6;
            } /* multi-stream huffman uses 6-byte jump table */
            return cLitSizeEstimate + literalSectionHeaderSize;
        }
    }
    0
}

/* Returns the size estimate for the FSE-compressed symbols (of, ml, ll) of a block */
pub unsafe fn ZSTD_estimateBlockSize_symbolType(
    r#type: SymbolEncodingType_e,
    codeTable: *const BYTE,
    nbSeq: size_t,
    maxCode: c_uint,
    fseCTable: *const FSE_CTable,
    additionalBits: *const U8,
    defaultNorm: *const core::ffi::c_short,
    defaultNormLog: U32,
    defaultMax: U32,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut ctp: *const BYTE = codeTable;
    let ctStart: *const BYTE = ctp;
    let ctEnd: *const BYTE = ctStart.wrapping_add(nbSeq);
    let mut cSymbolTypeSizeEstimateInBits: size_t = 0;
    let mut max: c_uint = maxCode;

    HIST_countFast_wksp(countWksp, &mut max, codeTable as *const c_void, nbSeq, workspace, wkspSize); /* can't fail */
    let _ = defaultMax;
    if r#type == set_basic {
        cSymbolTypeSizeEstimateInBits =
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, countWksp, max);
    } else if r#type == set_rle {
        cSymbolTypeSizeEstimateInBits = 0;
    } else if r#type == set_compressed || r#type == set_repeat {
        cSymbolTypeSizeEstimateInBits = ZSTD_fseBitCost(fseCTable, countWksp, max);
    }
    if ZSTD_isError(cSymbolTypeSizeEstimateInBits) != 0 {
        return nbSeq * 10;
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits +=
                *additionalBits.wrapping_add(*ctp as usize) as size_t;
        } else {
            cSymbolTypeSizeEstimateInBits += *ctp as size_t;
        } /* for offset, offset code is also the number of additional bits */
        ctp = ctp.wrapping_add(1);
    }
    cSymbolTypeSizeEstimateInBits >> 3
}

/* Returns the size estimate for the sequences section (header + content) of a block */
pub unsafe fn ZSTD_estimateBlockSize_sequences(
    ofCodeTable: *const BYTE,
    llCodeTable: *const BYTE,
    mlCodeTable: *const BYTE,
    nbSeq: size_t,
    fseTables: *const ZSTD_fseCTables_t,
    fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: size_t,
    writeEntropy: c_int,
) -> size_t {
    let sequencesSectionHeaderSize: size_t = 1 /* seqHead */ + 1 /* min seqSize size */
        + (nbSeq >= 128) as size_t
        + (nbSeq >= LONGNBSEQ as size_t) as size_t;
    let mut cSeqSizeEstimate: size_t = 0;
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).ofType,
        ofCodeTable,
        nbSeq,
        MaxOff,
        (*fseTables).offcodeCTable.as_ptr(),
        null_mut(),
        OF_defaultNorm.as_ptr(),
        OF_defaultNormLog,
        DefaultMaxOff,
        workspace,
        wkspSize,
    );
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).llType,
        llCodeTable,
        nbSeq,
        MaxLL,
        (*fseTables).litlengthCTable.as_ptr(),
        LL_bits.as_ptr(),
        LL_defaultNorm.as_ptr(),
        LL_defaultNormLog,
        MaxLL,
        workspace,
        wkspSize,
    );
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).mlType,
        mlCodeTable,
        nbSeq,
        MaxML,
        (*fseTables).matchlengthCTable.as_ptr(),
        ML_bits.as_ptr(),
        ML_defaultNorm.as_ptr(),
        ML_defaultNormLog,
        MaxML,
        workspace,
        wkspSize,
    );
    if writeEntropy != 0 {
        cSeqSizeEstimate += (*fseMetadata).fseTablesSize;
    }
    cSeqSizeEstimate + sequencesSectionHeaderSize
}

/* Returns the size estimate for a given stream of literals, of, ll, ml */
pub unsafe fn ZSTD_estimateBlockSize(
    literals: *const BYTE,
    litSize: size_t,
    ofCodeTable: *const BYTE,
    llCodeTable: *const BYTE,
    mlCodeTable: *const BYTE,
    nbSeq: size_t,
    entropy: *const ZSTD_entropyCTables_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: size_t,
    writeLitEntropy: c_int,
    writeSeqEntropy: c_int,
) -> size_t {
    let literalsSize: size_t = ZSTD_estimateBlockSize_literal(
        literals,
        litSize,
        &(*entropy).huf,
        &(*entropyMetadata).hufMetadata,
        workspace,
        wkspSize,
        writeLitEntropy,
    );
    let seqSize: size_t = ZSTD_estimateBlockSize_sequences(
        ofCodeTable,
        llCodeTable,
        mlCodeTable,
        nbSeq,
        &(*entropy).fse,
        &(*entropyMetadata).fseMetadata,
        workspace,
        wkspSize,
        writeSeqEntropy,
    );
    seqSize + literalsSize + ZSTD_blockHeaderSize
}

/* Builds entropy statistics and uses them for blocksize estimation. */
pub unsafe fn ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(
    seqStore: *mut SeqStore_t,
    zc: *mut ZSTD_CCtx,
) -> size_t {
    let entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t =
        &mut (*zc).blockSplitCtx.entropyMetadata;
    {
        let err = ZSTD_buildBlockEntropyStats(
            seqStore,
            &(*(*zc).blockState.prevCBlock).entropy,
            &mut (*(*zc).blockState.nextCBlock).entropy,
            &(*zc).appliedParams,
            entropyMetadata,
            (*zc).tmpWorkspace,
            (*zc).tmpWkspSize,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_estimateBlockSize(
        (*seqStore).litStart,
        (*seqStore).lit.offset_from((*seqStore).litStart) as size_t,
        (*seqStore).ofCode,
        (*seqStore).llCode,
        (*seqStore).mlCode,
        (*seqStore).sequences.offset_from((*seqStore).sequencesStart) as size_t,
        &(*(*zc).blockState.nextCBlock).entropy,
        entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
        ((*entropyMetadata).hufMetadata.hType == set_compressed) as c_int,
        1,
    )
}

/* Returns literals bytes represented in a seqStore */
pub unsafe fn ZSTD_countSeqStoreLiteralsBytes(seqStore: *const SeqStore_t) -> size_t {
    let mut literalsBytes: size_t = 0;
    let nbSeqs: size_t =
        (*seqStore).sequences.offset_from((*seqStore).sequencesStart) as size_t;
    let mut i: size_t;
    i = 0;
    while i < nbSeqs {
        let seq: SeqDef = *(*seqStore).sequencesStart.wrapping_add(i);
        literalsBytes += seq.litLength as size_t;
        if i == (*seqStore).longLengthPos as size_t
            && (*seqStore).longLengthType == ZSTD_llt_literalLength
        {
            literalsBytes += 0x10000;
        }
        i += 1;
    }
    literalsBytes
}

/* Returns match bytes represented in a seqStore */
pub unsafe fn ZSTD_countSeqStoreMatchBytes(seqStore: *const SeqStore_t) -> size_t {
    let mut matchBytes: size_t = 0;
    let nbSeqs: size_t =
        (*seqStore).sequences.offset_from((*seqStore).sequencesStart) as size_t;
    let mut i: size_t;
    i = 0;
    while i < nbSeqs {
        let seq: SeqDef = *(*seqStore).sequencesStart.wrapping_add(i);
        matchBytes += seq.mlBase as size_t + MINMATCH as size_t;
        if i == (*seqStore).longLengthPos as size_t
            && (*seqStore).longLengthType == ZSTD_llt_matchLength
        {
            matchBytes += 0x10000;
        }
        i += 1;
    }
    matchBytes
}

/* Derives the seqStore that is a chunk of the originalSeqStore from [startIdx, endIdx). */
pub unsafe fn ZSTD_deriveSeqStoreChunk(
    resultSeqStore: *mut SeqStore_t,
    originalSeqStore: *const SeqStore_t,
    startIdx: size_t,
    endIdx: size_t,
) {
    core::ptr::copy_nonoverlapping(originalSeqStore, resultSeqStore, 1);
    if startIdx > 0 {
        (*resultSeqStore).sequences =
            (*originalSeqStore).sequencesStart.wrapping_add(startIdx);
        (*resultSeqStore).litStart = (*resultSeqStore)
            .litStart
            .wrapping_add(ZSTD_countSeqStoreLiteralsBytes(resultSeqStore));
    }

    /* Move longLengthPos into the correct position if necessary */
    if (*originalSeqStore).longLengthType != ZSTD_llt_none {
        if ((*originalSeqStore).longLengthPos as size_t) < startIdx
            || (*originalSeqStore).longLengthPos as size_t > endIdx
        {
            (*resultSeqStore).longLengthType = ZSTD_llt_none;
        } else {
            (*resultSeqStore).longLengthPos -= startIdx as U32;
        }
    }
    (*resultSeqStore).sequencesStart =
        (*originalSeqStore).sequencesStart.wrapping_add(startIdx);
    (*resultSeqStore).sequences =
        (*originalSeqStore).sequencesStart.wrapping_add(endIdx);
    if endIdx
        == (*originalSeqStore)
            .sequences
            .offset_from((*originalSeqStore).sequencesStart) as size_t
    {
        /* This accounts for possible last literals if the derived chunk reaches the end of the block */
    } else {
        let literalsBytes: size_t = ZSTD_countSeqStoreLiteralsBytes(resultSeqStore);
        (*resultSeqStore).lit = (*resultSeqStore).litStart.wrapping_add(literalsBytes);
    }
    (*resultSeqStore).llCode = (*resultSeqStore).llCode.wrapping_add(startIdx);
    (*resultSeqStore).mlCode = (*resultSeqStore).mlCode.wrapping_add(startIdx);
    (*resultSeqStore).ofCode = (*resultSeqStore).ofCode.wrapping_add(startIdx);
}

/**
 * Returns the raw offset represented by the combination of offBase, ll0, and repcode history. */
pub unsafe fn ZSTD_resolveRepcodeToRawOffset(
    rep: *const U32, /* U32[ZSTD_REP_NUM] */
    offBase: U32,
    ll0: U32,
) -> U32 {
    let adjustedRepCode: U32 = OFFBASE_TO_REPCODE(offBase) - 1 + ll0; /* [ 0 - 3 ] */
    if adjustedRepCode == ZSTD_REP_NUM as U32 {
        return *rep.wrapping_add(0) - 1;
    }
    *rep.wrapping_add(adjustedRepCode as usize)
}

/**
 * ZSTD_seqStore_resolveOffCodes() reconciles any possible divergences in offset
 * history that may arise due to emission of RLE/raw blocks. */
pub unsafe fn ZSTD_seqStore_resolveOffCodes(
    dRepcodes: *mut Repcodes_t,
    cRepcodes: *mut Repcodes_t,
    seqStore: *const SeqStore_t,
    nbSeq: U32,
) {
    let mut idx: U32 = 0;
    let longLitLenIdx: U32 = if (*seqStore).longLengthType == ZSTD_llt_literalLength {
        (*seqStore).longLengthPos
    } else {
        nbSeq
    };
    while idx < nbSeq {
        let seq: *mut SeqDef = (*seqStore).sequencesStart.wrapping_add(idx as usize);
        let ll0: U32 = (((*seq).litLength == 0) && (idx != longLitLenIdx)) as U32;
        let offBase: U32 = (*seq).offBase;
        if OFFBASE_IS_REPCODE(offBase) {
            let dRawOffset: U32 =
                ZSTD_resolveRepcodeToRawOffset((*dRepcodes).rep.as_ptr(), offBase, ll0);
            let cRawOffset: U32 =
                ZSTD_resolveRepcodeToRawOffset((*cRepcodes).rep.as_ptr(), offBase, ll0);
            if dRawOffset != cRawOffset {
                (*seq).offBase = OFFSET_TO_OFFBASE(cRawOffset);
            }
        }
        ZSTD_updateRep((*dRepcodes).rep.as_mut_ptr(), (*seq).offBase, ll0);
        ZSTD_updateRep((*cRepcodes).rep.as_mut_ptr(), offBase, ll0);
        idx += 1;
    }
}

/* ZSTD_compressSeqStore_singleBlock():
 * Compresses a seqStore into a block with a block header, into the buffer dst. */
pub unsafe fn ZSTD_compressSeqStore_singleBlock(
    zc: *mut ZSTD_CCtx,
    seqStore: *const SeqStore_t,
    dRep: *mut Repcodes_t,
    cRep: *mut Repcodes_t,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    lastBlock: U32,
    isPartition: U32,
) -> size_t {
    let rleMaxLength: U32 = 25;
    let op: *mut BYTE = dst as *mut BYTE;
    let ip: *const BYTE = src as *const BYTE;
    let cSize: size_t;
    let mut cSeqsSize: size_t;

    /* In case of an RLE or raw block, the simulated decompression repcode history must be reset */
    let dRepOriginal: Repcodes_t = *dRep;
    if isPartition != 0 {
        ZSTD_seqStore_resolveOffCodes(
            dRep,
            cRep,
            seqStore,
            (*seqStore).sequences.offset_from((*seqStore).sequencesStart) as U32,
        );
    }

    if dstCapacity < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    cSeqsSize = ZSTD_entropyCompressSeqStore(
        seqStore,
        &(*(*zc).blockState.prevCBlock).entropy,
        &mut (*(*zc).blockState.nextCBlock).entropy,
        &(*zc).appliedParams,
        op.wrapping_add(ZSTD_blockHeaderSize) as *mut c_void,
        dstCapacity - ZSTD_blockHeaderSize,
        srcSize,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize, /* statically allocated in resetCCtx */
        (*zc).bmi2,
    );
    if ERR_isError(cSeqsSize) != 0 {
        return cSeqsSize;
    }

    if (*zc).isFirstBlock == 0
        && cSeqsSize < rleMaxLength as size_t
        && ZSTD_isRLE(src as *const BYTE, srcSize) != 0
    {
        /* We don't want to emit our first block as a RLE even if it qualifies. */
        cSeqsSize = 1;
    }

    /* Sequence collection not supported when block splitting */
    if (*zc).seqCollector.collectSequences != 0 {
        {
            let err = ZSTD_copyBlockSequences(
                &mut (*zc).seqCollector,
                seqStore,
                dRepOriginal.rep.as_ptr(),
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
        return 0;
    }

    if cSeqsSize == 0 {
        cSize = ZSTD_noCompressBlock(op as *mut c_void, dstCapacity, ip as *const c_void, srcSize, lastBlock);
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        *dRep = dRepOriginal; /* reset simulated decompression repcode history */
    } else if cSeqsSize == 1 {
        cSize = ZSTD_rleCompressBlock(op as *mut c_void, dstCapacity, *ip, srcSize, lastBlock);
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        *dRep = dRepOriginal; /* reset simulated decompression repcode history */
    } else {
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
        writeBlockHeader(op as *mut c_void, cSeqsSize, srcSize, lastBlock);
        cSize = ZSTD_blockHeaderSize + cSeqsSize;
    }

    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

/* Struct to keep track of where we are in our recursive calls. */
#[repr(C)]
pub struct seqStoreSplits {
    pub splitLocations: *mut U32, /* Array of split indices */
    pub idx: size_t,              /* The current index within splitLocations being worked on */
}

const MIN_SEQUENCES_BLOCK_SPLITTING: size_t = 300;

/* Helper function to perform the recursive search for block splits. */
pub unsafe fn ZSTD_deriveBlockSplitsHelper(
    splits: *mut seqStoreSplits,
    startIdx: size_t,
    endIdx: size_t,
    zc: *mut ZSTD_CCtx,
    origSeqStore: *const SeqStore_t,
) {
    let fullSeqStoreChunk: *mut SeqStore_t = &mut (*zc).blockSplitCtx.fullSeqStoreChunk;
    let firstHalfSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.firstHalfSeqStore;
    let secondHalfSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.secondHalfSeqStore;
    let estimatedOriginalSize: size_t;
    let estimatedFirstHalfSize: size_t;
    let estimatedSecondHalfSize: size_t;
    let midIdx: size_t = (startIdx + endIdx) / 2;

    if endIdx - startIdx < MIN_SEQUENCES_BLOCK_SPLITTING
        || (*splits).idx >= ZSTD_MAX_NB_BLOCK_SPLITS
    {
        return;
    }
    ZSTD_deriveSeqStoreChunk(fullSeqStoreChunk, origSeqStore, startIdx, endIdx);
    ZSTD_deriveSeqStoreChunk(firstHalfSeqStore, origSeqStore, startIdx, midIdx);
    ZSTD_deriveSeqStoreChunk(secondHalfSeqStore, origSeqStore, midIdx, endIdx);
    estimatedOriginalSize = ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(fullSeqStoreChunk, zc);
    estimatedFirstHalfSize = ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(firstHalfSeqStore, zc);
    estimatedSecondHalfSize =
        ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(secondHalfSeqStore, zc);
    if ZSTD_isError(estimatedOriginalSize) != 0
        || ZSTD_isError(estimatedFirstHalfSize) != 0
        || ZSTD_isError(estimatedSecondHalfSize) != 0
    {
        return;
    }
    if estimatedFirstHalfSize + estimatedSecondHalfSize < estimatedOriginalSize {
        ZSTD_deriveBlockSplitsHelper(splits, startIdx, midIdx, zc, origSeqStore);
        *(*splits).splitLocations.wrapping_add((*splits).idx) = midIdx as U32;
        (*splits).idx += 1;
        ZSTD_deriveBlockSplitsHelper(splits, midIdx, endIdx, zc, origSeqStore);
    }
}

/* Base recursive function. */
pub unsafe fn ZSTD_deriveBlockSplits(
    zc: *mut ZSTD_CCtx,
    partitions: *mut U32,
    nbSeq: U32,
) -> size_t {
    let mut splits: seqStoreSplits = seqStoreSplits {
        splitLocations: partitions,
        idx: 0,
    };
    if nbSeq <= 4 {
        /* Refuse to try and split anything with less than 4 sequences */
        return 0;
    }
    ZSTD_deriveBlockSplitsHelper(&mut splits, 0, nbSeq as size_t, zc, &(*zc).seqStore);
    *splits.splitLocations.wrapping_add(splits.idx) = nbSeq;
    splits.idx
}

/* ZSTD_compressBlock_splitBlock():
 * Attempts to split a given block into multiple blocks to improve compression ratio. */
pub unsafe fn ZSTD_compressBlock_splitBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: size_t,
    src: *const c_void,
    blockSize: size_t,
    lastBlock: U32,
    nbSeq: U32,
) -> size_t {
    let mut cSize: size_t = 0;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut i: size_t = 0;
    let mut srcBytesTotal: size_t = 0;
    let partitions: *mut U32 = (*zc).blockSplitCtx.partitions.as_mut_ptr(); /* size == ZSTD_MAX_NB_BLOCK_SPLITS */
    let nextSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.nextSeqStore;
    let currSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.currSeqStore;
    let numSplits: size_t = ZSTD_deriveBlockSplits(zc, partitions, nbSeq);

    let mut dRep: Repcodes_t = core::mem::zeroed();
    let mut cRep: Repcodes_t = core::mem::zeroed();
    ZSTD_memcpy(
        dRep.rep.as_mut_ptr() as *mut u8,
        (*(*zc).blockState.prevCBlock).rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>(),
    );
    ZSTD_memcpy(
        cRep.rep.as_mut_ptr() as *mut u8,
        (*(*zc).blockState.prevCBlock).rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>(),
    );
    ZSTD_memset(
        nextSeqStore as *mut u8,
        0,
        core::mem::size_of::<SeqStore_t>(),
    );

    if numSplits == 0 {
        let cSizeSingleBlock: size_t = ZSTD_compressSeqStore_singleBlock(
            zc,
            &(*zc).seqStore,
            &mut dRep,
            &mut cRep,
            op as *mut c_void,
            dstCapacity,
            ip as *const c_void,
            blockSize,
            lastBlock,
            0, /* isPartition */
        );
        if ERR_isError(cSizeSingleBlock) != 0 {
            return cSizeSingleBlock;
        }
        return cSizeSingleBlock;
    }

    ZSTD_deriveSeqStoreChunk(currSeqStore, &(*zc).seqStore, 0, *partitions.wrapping_add(0) as size_t);
    i = 0;
    while i <= numSplits {
        let cSizeChunk: size_t;
        let lastPartition: U32 = (i == numSplits) as U32;
        let mut lastBlockEntireSrc: U32 = 0;

        let mut srcBytes: size_t = ZSTD_countSeqStoreLiteralsBytes(currSeqStore)
            + ZSTD_countSeqStoreMatchBytes(currSeqStore);
        srcBytesTotal += srcBytes;
        if lastPartition != 0 {
            /* This is the final partition, need to account for possible last literals */
            srcBytes += blockSize - srcBytesTotal;
            lastBlockEntireSrc = lastBlock;
        } else {
            ZSTD_deriveSeqStoreChunk(
                nextSeqStore,
                &(*zc).seqStore,
                *partitions.wrapping_add(i) as size_t,
                *partitions.wrapping_add(i + 1) as size_t,
            );
        }

        cSizeChunk = ZSTD_compressSeqStore_singleBlock(
            zc,
            currSeqStore,
            &mut dRep,
            &mut cRep,
            op as *mut c_void,
            dstCapacity,
            ip as *const c_void,
            srcBytes,
            lastBlockEntireSrc,
            1, /* isPartition */
        );
        if ERR_isError(cSizeChunk) != 0 {
            return cSizeChunk;
        }

        ip = ip.wrapping_add(srcBytes);
        op = op.wrapping_add(cSizeChunk);
        dstCapacity -= cSizeChunk;
        cSize += cSizeChunk;
        core::ptr::copy_nonoverlapping(nextSeqStore as *const SeqStore_t, currSeqStore, 1);
        i += 1;
    }
    /* cRep and dRep may have diverged during the compression.
     * If so, we use the dRep repcodes for the next block. */
    ZSTD_memcpy(
        (*(*zc).blockState.prevCBlock).rep.as_mut_ptr() as *mut u8,
        dRep.rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>(),
    );
    cSize
}

pub unsafe fn ZSTD_compressBlock_splitBlock(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    lastBlock: U32,
) -> size_t {
    let nbSeq: U32;
    let cSize: size_t;

    {
        let bss: size_t = ZSTD_buildSeqStore(zc, src, srcSize);
        if ERR_isError(bss) != 0 {
            return bss;
        }
        if bss == ZSTDbss_noCompress as size_t {
            if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }
            if (*zc).seqCollector.collectSequences != 0 {
                return ERROR(ZSTD_error_sequenceProducer_failed);
            }
            let cSizeNc = ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock);
            if ERR_isError(cSizeNc) != 0 {
                return cSizeNc;
            }
            return cSizeNc;
        }
        nbSeq = (*zc).seqStore.sequences.offset_from((*zc).seqStore.sequencesStart) as U32;
    }

    cSize = ZSTD_compressBlock_splitBlock_internal(zc, dst, dstCapacity, src, srcSize, lastBlock, nbSeq);
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    cSize
}

pub unsafe fn ZSTD_compressBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    frame: U32,
) -> size_t {
    /* This is an estimated upper bound for the length of an rle block. */
    let rleMaxLength: U32 = 25;
    let mut cSize: size_t;
    let ip: *const BYTE = src as *const BYTE;
    let op: *mut BYTE = dst as *mut BYTE;

    {
        let bss: size_t = ZSTD_buildSeqStore(zc, src, srcSize);
        if ERR_isError(bss) != 0 {
            return bss;
        }
        if bss == ZSTDbss_noCompress as size_t {
            if (*zc).seqCollector.collectSequences != 0 {
                return ERROR(ZSTD_error_sequenceProducer_failed);
            }
            cSize = 0;
            /* goto out */
            if ZSTD_isError(cSize) == 0 && cSize > 1 {
                ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
            }
            if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }
            return cSize;
        }
    }

    if (*zc).seqCollector.collectSequences != 0 {
        {
            let err = ZSTD_copyBlockSequences(
                &mut (*zc).seqCollector,
                ZSTD_getSeqStore(zc),
                (*(*zc).blockState.prevCBlock).rep.as_ptr(),
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
        return 0;
    }

    /* encode sequences and literals */
    cSize = ZSTD_entropyCompressSeqStore(
        &(*zc).seqStore,
        &(*(*zc).blockState.prevCBlock).entropy,
        &mut (*(*zc).blockState.nextCBlock).entropy,
        &(*zc).appliedParams,
        dst,
        dstCapacity,
        srcSize,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize, /* statically allocated in resetCCtx */
        (*zc).bmi2,
    );

    if frame != 0
        && (*zc).isFirstBlock == 0
        && cSize < rleMaxLength as size_t
        && ZSTD_isRLE(ip, srcSize) != 0
    {
        cSize = 1;
        *op.wrapping_add(0) = *ip.wrapping_add(0);
    }

    /* out: */
    if ZSTD_isError(cSize) == 0 && cSize > 1 {
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
    }
    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

pub unsafe fn ZSTD_compressBlock_targetCBlockSize_body(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    bss: size_t,
    lastBlock: U32,
) -> size_t {
    if bss == ZSTDbss_compress as size_t {
        if
        /* We don't want to emit our first block as a RLE even if it qualifies. */
        (*zc).isFirstBlock == 0
            && ZSTD_maybeRLE(&(*zc).seqStore) != 0
            && ZSTD_isRLE(src as *const BYTE, srcSize) != 0
        {
            return ZSTD_rleCompressBlock(dst, dstCapacity, *(src as *const BYTE), srcSize, lastBlock);
        }
        /* Attempt superblock compression. */
        {
            let cSize: size_t =
                ZSTD_compressSuperBlock(zc, dst, dstCapacity, src, srcSize, lastBlock);
            if cSize != ERROR(ZSTD_error_dstSize_tooSmall) {
                let maxCSize: size_t =
                    srcSize - ZSTD_minGain(srcSize, (*zc).appliedParams.cParams.strategy);
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
                if cSize != 0 && cSize < maxCSize + ZSTD_blockHeaderSize {
                    ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
                    return cSize;
                }
            }
        }
    } /* if (bss == ZSTDbss_compress)*/

    /* Superblock compression failed, attempt to emit a single no compress block. */
    ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock)
}

pub unsafe fn ZSTD_compressBlock_targetCBlockSize(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    lastBlock: U32,
) -> size_t {
    let mut cSize: size_t = 0;
    let bss: size_t = ZSTD_buildSeqStore(zc, src, srcSize);
    if ERR_isError(bss) != 0 {
        return bss;
    }

    cSize = ZSTD_compressBlock_targetCBlockSize_body(zc, dst, dstCapacity, src, srcSize, bss, lastBlock);
    if ERR_isError(cSize) != 0 {
        return cSize;
    }

    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}
