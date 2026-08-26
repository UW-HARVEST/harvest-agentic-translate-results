//! Translation of `compress/zstd_compress.c`.
//!
//! The C file is ~7850 lines, so the translation is split into six part files
//! that are textually `include!`d here. All of them live in this single Rust
//! module, exactly like the single C translation unit, so they share the private
//! helpers freely.
//!
//! **Part files must not contain `use` statements or `extern "C"` blocks** —
//! every import and every cross-module declaration lives in this file, so that
//! the parts cannot collide with each other.
#![allow(dead_code)]

use crate::common::bits::*;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::xxhash::*;
use crate::common::zstd_internal::*;
use crate::compress::clevels::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::*;
use crate::libc::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/* ---- Local build configuration (from the C `#define`s at the top) ---- */
pub const ZSTD_COMPRESS_HEAPMODE: c_int = 0;
pub const ZSTD_HASHLOG3_MAX: U32 = 17;

/* `ZSTD_DefaultPolicy_e`, from compress/zstd_compress_sequences.h */
pub type ZSTD_DefaultPolicy_e = c_int;
pub const ZSTD_defaultDisallowed: ZSTD_DefaultPolicy_e = 0;
pub const ZSTD_defaultAllowed: ZSTD_DefaultPolicy_e = 1;

/* ---- Cross-translation-unit functions (all exported symbols) ---- */
extern "C" {
    /* common */
    pub fn ZSTD_isError(code: usize) -> c_uint;
    pub fn FSE_isError(code: usize) -> c_uint;
    pub fn HUF_isError(code: usize) -> c_uint;
    pub fn FSE_readNCount(
        normalizedCounter: *mut i16,
        maxSVPtr: *mut c_uint,
        tableLogPtr: *mut c_uint,
        rBuffer: *const c_void,
        rBuffSize: usize,
    ) -> usize;

    /* compress/fse_compress.c */
    pub fn FSE_buildCTable_wksp(
        ct: *mut FSE_CTable,
        normalizedCounter: *const i16,
        maxSymbolValue: c_uint,
        tableLog: c_uint,
        workSpace: *mut c_void,
        wkspSize: usize,
    ) -> usize;

    /* compress/hist.c */
    pub fn HIST_count_wksp(
        count: *mut c_uint,
        maxSymbolValuePtr: *mut c_uint,
        src: *const c_void,
        srcSize: usize,
        workSpace: *mut c_void,
        workSpaceSize: usize,
    ) -> usize;
    pub fn HIST_countFast_wksp(
        count: *mut c_uint,
        maxSymbolValuePtr: *mut c_uint,
        src: *const c_void,
        srcSize: usize,
        workSpace: *mut c_void,
        workSpaceSize: usize,
    ) -> usize;

    /* compress/huf_compress.c */
    pub fn HUF_buildCTable_wksp(
        tree: *mut HUF_CElt,
        count: *const c_uint,
        maxSymbolValue: U32,
        maxNbBits: U32,
        workSpace: *mut c_void,
        wkspSize: usize,
    ) -> usize;
    pub fn HUF_estimateCompressedSize(
        CTable: *const HUF_CElt,
        count: *const c_uint,
        maxSymbolValue: c_uint,
    ) -> usize;
    pub fn HUF_optimalTableLog(
        maxTableLog: c_uint,
        srcSize: usize,
        maxSymbolValue: c_uint,
        workSpace: *mut c_void,
        wkspSize: usize,
        table: *mut HUF_CElt,
        count: *const c_uint,
        flags: c_int,
    ) -> c_uint;
    pub fn HUF_readCTable(
        CTable: *mut HUF_CElt,
        maxSymbolValuePtr: *mut c_uint,
        src: *const c_void,
        srcSize: usize,
        hasZeroWeights: *mut c_uint,
    ) -> usize;
    pub fn HUF_validateCTable(
        CTable: *const HUF_CElt,
        count: *const c_uint,
        maxSymbolValue: c_uint,
    ) -> c_int;
    pub fn HUF_writeCTable_wksp(
        dst: *mut c_void,
        maxDstSize: usize,
        CTable: *const HUF_CElt,
        maxSymbolValue: c_uint,
        huffLog: c_uint,
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> usize;

    /* compress/zstd_compress_literals.c */
    pub fn ZSTD_noCompressLiterals(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressRleLiteralsBlock(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressLiterals(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        entropyWorkspace: *mut c_void,
        entropyWorkspaceSize: usize,
        prevHuf: *const ZSTD_hufCTables_t,
        nextHuf: *mut ZSTD_hufCTables_t,
        strategy: ZSTD_strategy,
        disableLiteralCompression: c_int,
        suspectUncompressible: c_int,
        bmi2: c_int,
    ) -> usize;

    /* compress/zstd_compress_sequences.c */
    pub fn ZSTD_selectEncodingType(
        repeatMode: *mut FSE_repeat,
        count: *const c_uint,
        max: c_uint,
        mostFrequent: usize,
        nbSeq: usize,
        FSELog: c_uint,
        prevCTable: *const FSE_CTable,
        defaultNorm: *const i16,
        defaultNormLog: U32,
        isDefaultAllowed: ZSTD_DefaultPolicy_e,
        strategy: ZSTD_strategy,
    ) -> SymbolEncodingType_e;
    pub fn ZSTD_buildCTable(
        dst: *mut c_void,
        dstCapacity: usize,
        nextCTable: *mut FSE_CTable,
        FSELog: U32,
        type_: SymbolEncodingType_e,
        count: *mut c_uint,
        max: U32,
        codeTable: *const BYTE,
        nbSeq: usize,
        defaultNorm: *const S16,
        defaultNormLog: U32,
        defaultMax: U32,
        prevCTable: *const FSE_CTable,
        prevCTableSize: usize,
        entropyWorkspace: *mut c_void,
        entropyWorkspaceSize: usize,
    ) -> usize;
    pub fn ZSTD_encodeSequences(
        dst: *mut c_void,
        dstCapacity: usize,
        CTable_MatchLength: *const FSE_CTable,
        mlCodeTable: *const BYTE,
        CTable_OffsetBits: *const FSE_CTable,
        ofCodeTable: *const BYTE,
        CTable_LitLength: *const FSE_CTable,
        llCodeTable: *const BYTE,
        sequences: *const SeqDef,
        nbSeq: usize,
        longOffsets: c_int,
        bmi2: c_int,
    ) -> usize;
    pub fn ZSTD_fseBitCost(
        ctable: *const FSE_CTable,
        count: *const c_uint,
        max: c_uint,
    ) -> usize;
    pub fn ZSTD_crossEntropyCost(
        norm: *const i16,
        accuracyLog: c_uint,
        count: *const c_uint,
        max: c_uint,
    ) -> usize;

    /* compress/zstd_compress_superblock.c */
    pub fn ZSTD_compressSuperBlock(
        zc: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        lastBlock: c_uint,
    ) -> usize;

    /* compress/zstd_preSplit.c */
    pub fn ZSTD_splitBlock(
        blockStart: *const c_void,
        blockSize: usize,
        level: c_int,
        workspace: *mut c_void,
        wkspSize: usize,
    ) -> usize;

    /* compress/zstd_fast.c */
    pub fn ZSTD_fillHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    pub fn ZSTD_compressBlock_fast(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_fast_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_fast_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;

    /* compress/zstd_double_fast.c */
    pub fn ZSTD_fillDoubleHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    pub fn ZSTD_compressBlock_doubleFast(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_doubleFast_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_doubleFast_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;

    /* compress/zstd_lazy.c */
    pub fn ZSTD_insertAndFindFirstIndex(ms: *mut ZSTD_MatchState_t, ip: *const BYTE) -> U32;
    pub fn ZSTD_row_update(ms: *mut ZSTD_MatchState_t, ip: *const BYTE);
    pub fn ZSTD_dedicatedDictSearch_lazy_loadDictionary(
        ms: *mut ZSTD_MatchState_t,
        ip: *const BYTE,
    );
    pub fn ZSTD_compressBlock_greedy(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_dictMatchState_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_dedicatedDictSearch(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_dedicatedDictSearch_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_greedy_extDict_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_dictMatchState_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_dedicatedDictSearch(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_dedicatedDictSearch_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy_extDict_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_dictMatchState_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_dedicatedDictSearch(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_dedicatedDictSearch_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_lazy2_extDict_row(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btlazy2(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btlazy2_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btlazy2_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;

    /* compress/zstd_opt.c */
    pub fn ZSTD_updateTree(ms: *mut ZSTD_MatchState_t, ip: *const BYTE, iend: *const BYTE);
    pub fn ZSTD_compressBlock_btopt(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btopt_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btopt_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btultra(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btultra_dictMatchState(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btultra_extDict(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_compressBlock_btultra2(
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;

    /* compress/zstd_ldm.c */
    pub fn ZSTD_ldm_fillHashTable(
        state: *mut ldmState_t,
        ip: *const BYTE,
        iend: *const BYTE,
        params: *const ldmParams_t,
    );
    pub fn ZSTD_ldm_generateSequences(
        ldms: *mut ldmState_t,
        sequences: *mut RawSeqStore_t,
        params: *const ldmParams_t,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_ldm_blockCompress(
        rawSeqStore: *mut RawSeqStore_t,
        ms: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32,
        useRowMatchFinder: ZSTD_ParamSwitch_e,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    pub fn ZSTD_ldm_skipSequences(
        rawSeqStore: *mut RawSeqStore_t,
        srcSize: usize,
        minMatch: U32,
    );
    pub fn ZSTD_ldm_skipRawSeqStoreBytes(rawSeqStore: *mut RawSeqStore_t, nbBytes: usize);
    pub fn ZSTD_ldm_getTableSize(params: ldmParams_t) -> usize;
    pub fn ZSTD_ldm_getMaxNbSeq(params: ldmParams_t, maxChunkSize: usize) -> usize;
    pub fn ZSTD_ldm_adjustParameters(
        params: *mut ldmParams_t,
        cParams: *const ZSTD_compressionParameters,
    );
}

include!("zstd_compress_p1.rs");
include!("zstd_compress_p2.rs");
include!("zstd_compress_p3.rs");
include!("zstd_compress_p4.rs");
include!("zstd_compress_p5.rs");
include!("zstd_compress_p6.rs");
