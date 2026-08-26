//! Translation of `compress/zstd_compress.c`, C lines 2693 - 4694
//! ("Block entropic compression" + block splitting + frame chunk).
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_variables,
    unused_assignments,
    unused_parens,
    unused_imports,
    unused_unsafe
)]

use core::ffi::{c_char, c_int, c_short, c_uint, c_void};
use core::ptr::{addr_of, addr_of_mut};

use crate::bits::*;
use crate::error_private::*;
use crate::fse::*;
use crate::hist::*;
use crate::huf::*;
use crate::mem::*;
use crate::zstd_compress_internal::*;
use crate::zstd_cwksp::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

use crate::zstd_compress_sequences::{
    ZSTD_DefaultPolicy_e, ZSTD_buildCTable, ZSTD_crossEntropyCost, ZSTD_encodeSequences,
    ZSTD_fseBitCost, ZSTD_selectEncodingType, ZSTD_defaultAllowed, ZSTD_defaultDisallowed,
};

/* ============================================================ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int {
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let llCodeTable: *mut BYTE = (*seqStorePtr).llCode;
    let ofCodeTable: *mut BYTE = (*seqStorePtr).ofCode;
    let mlCodeTable: *mut BYTE = (*seqStorePtr).mlCode;
    let nbSeq: U32 = ((*seqStorePtr).sequences as usize)
        .wrapping_sub((*seqStorePtr).sequencesStart as usize)
        .wrapping_div(core::mem::size_of::<SeqDef>()) as U32;
    let mut u: U32;
    let mut longOffsets: c_int = 0;
    u = 0;
    while u < nbSeq {
        let llv: U32 = (*sequences.offset(u as isize)).litLength as U32;
        let ofCode: U32 = ZSTD_highbit32((*sequences.offset(u as isize)).offBase);
        let mlv: U32 = (*sequences.offset(u as isize)).mlBase as U32;
        *llCodeTable.offset(u as isize) = ZSTD_LLcode(llv) as BYTE;
        *ofCodeTable.offset(u as isize) = ofCode as BYTE;
        *mlCodeTable.offset(u as isize) = ZSTD_MLcode(mlv) as BYTE;
        if MEM_32bits() != 0 && ofCode >= STREAM_ACCUMULATOR_MIN() {
            longOffsets = 1;
        }
        u = u.wrapping_add(1);
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_literalLength {
        *llCodeTable.offset((*seqStorePtr).longLengthPos as isize) = MaxLL as BYTE;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_matchLength {
        *mlCodeTable.offset((*seqStorePtr).longLengthPos as isize) = MaxML as BYTE;
    }
    longOffsets
}

use crate::bitstream::STREAM_ACCUMULATOR_MIN;

/* ZSTD_useTargetCBlockSize() */
pub unsafe fn ZSTD_useTargetCBlockSize(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).targetCBlockSize != 0) as c_int
}

/* ZSTD_blockSplitterEnabled() */
pub unsafe fn ZSTD_blockSplitterEnabled(cctxParams: *mut ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).postBlockSplitter == ZSTD_ps_enable) as c_int
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_symbolEncodingTypeStats_t {
    pub LLtype: U32,
    pub Offtype: U32,
    pub MLtype: U32,
    pub size: usize,
    pub lastCountSize: usize,
    pub longOffsets: c_int,
}

pub unsafe fn ZSTD_buildSequencesStatistics(
    seqStorePtr: *const SeqStore_t,
    nbSeq: usize,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    dst: *mut BYTE,
    dstEnd: *const BYTE,
    strategy: ZSTD_strategy,
    countWorkspace: *mut c_uint,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
) -> ZSTD_symbolEncodingTypeStats_t {
    let ostart: *mut BYTE = dst;
    let oend: *const BYTE = dstEnd;
    let mut op: *mut BYTE = ostart;
    let CTable_LitLength: *mut FSE_CTable = addr_of_mut!((*nextEntropy).litlengthCTable) as *mut FSE_CTable;
    let CTable_OffsetBits: *mut FSE_CTable = addr_of_mut!((*nextEntropy).offcodeCTable) as *mut FSE_CTable;
    let CTable_MatchLength: *mut FSE_CTable =
        addr_of_mut!((*nextEntropy).matchlengthCTable) as *mut FSE_CTable;
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let mut stats = ZSTD_symbolEncodingTypeStats_t::default();

    stats.lastCountSize = 0;
    /* convert length/distances into codes */
    stats.longOffsets = ZSTD_seqToCodes(seqStorePtr);

    /* build CTable for Literal Lengths */
    {
        let mut max: c_uint = MaxLL;
        let mostFrequent = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            llCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        );
        (*nextEntropy).litlength_repeatMode = (*prevEntropy).litlength_repeatMode;
        stats.LLtype = ZSTD_selectEncodingType(
            addr_of_mut!((*nextEntropy).litlength_repeatMode),
            countWorkspace as *const c_uint,
            max,
            mostFrequent,
            nbSeq,
            LLFSELog,
            addr_of!((*prevEntropy).litlengthCTable) as *const FSE_CTable,
            LL_defaultNorm.as_ptr() as *const c_short,
            LL_defaultNormLog,
            ZSTD_defaultAllowed,
            strategy,
        ) as U32;
        {
            let countSize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                CTable_LitLength,
                LLFSELog,
                stats.LLtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                llCodeTable,
                nbSeq,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                MaxLL,
                addr_of!((*prevEntropy).litlengthCTable) as *const FSE_CTable,
                core::mem::size_of_val(&(*prevEntropy).litlengthCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ERR_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.LLtype == set_compressed as U32 {
                stats.lastCountSize = countSize;
            }
            op = op.wrapping_add(countSize);
        }
    }
    /* build CTable for Offsets */
    {
        let mut max: c_uint = MaxOff;
        let mostFrequent = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            ofCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        );
        let defaultPolicy: ZSTD_DefaultPolicy_e = if max <= DefaultMaxOff {
            ZSTD_defaultAllowed
        } else {
            ZSTD_defaultDisallowed
        };
        (*nextEntropy).offcode_repeatMode = (*prevEntropy).offcode_repeatMode;
        stats.Offtype = ZSTD_selectEncodingType(
            addr_of_mut!((*nextEntropy).offcode_repeatMode),
            countWorkspace as *const c_uint,
            max,
            mostFrequent,
            nbSeq,
            OffFSELog,
            addr_of!((*prevEntropy).offcodeCTable) as *const FSE_CTable,
            OF_defaultNorm.as_ptr() as *const c_short,
            OF_defaultNormLog,
            defaultPolicy,
            strategy,
        ) as U32;
        {
            let countSize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                CTable_OffsetBits,
                OffFSELog,
                stats.Offtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                ofCodeTable,
                nbSeq,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                DefaultMaxOff,
                addr_of!((*prevEntropy).offcodeCTable) as *const FSE_CTable,
                core::mem::size_of_val(&(*prevEntropy).offcodeCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ERR_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.Offtype == set_compressed as U32 {
                stats.lastCountSize = countSize;
            }
            op = op.wrapping_add(countSize);
        }
    }
    /* build CTable for MatchLengths */
    {
        let mut max: c_uint = MaxML;
        let mostFrequent = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            mlCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        );
        (*nextEntropy).matchlength_repeatMode = (*prevEntropy).matchlength_repeatMode;
        stats.MLtype = ZSTD_selectEncodingType(
            addr_of_mut!((*nextEntropy).matchlength_repeatMode),
            countWorkspace as *const c_uint,
            max,
            mostFrequent,
            nbSeq,
            MLFSELog,
            addr_of!((*prevEntropy).matchlengthCTable) as *const FSE_CTable,
            ML_defaultNorm.as_ptr() as *const c_short,
            ML_defaultNormLog,
            ZSTD_defaultAllowed,
            strategy,
        ) as U32;
        {
            let countSize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
                CTable_MatchLength,
                MLFSELog,
                stats.MLtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                mlCodeTable,
                nbSeq,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                MaxML,
                addr_of!((*prevEntropy).matchlengthCTable) as *const FSE_CTable,
                core::mem::size_of_val(&(*prevEntropy).matchlengthCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ERR_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.MLtype == set_compressed as U32 {
                stats.lastCountSize = countSize;
            }
            op = op.wrapping_add(countSize);
        }
    }
    stats.size = (op as usize).wrapping_sub(ostart as usize);
    stats
}

pub const SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO: usize = 20;

pub unsafe fn ZSTD_entropyCompressSeqStore_internal(
    dst: *mut c_void,
    dstCapacity: usize,
    literals: *const c_void,
    litSize: usize,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let count: *mut c_uint = entropyWorkspace as *mut c_uint;
    let CTable_LitLength: *mut FSE_CTable =
        addr_of_mut!((*nextEntropy).fse.litlengthCTable) as *mut FSE_CTable;
    let CTable_OffsetBits: *mut FSE_CTable =
        addr_of_mut!((*nextEntropy).fse.offcodeCTable) as *mut FSE_CTable;
    let CTable_MatchLength: *mut FSE_CTable =
        addr_of_mut!((*nextEntropy).fse.matchlengthCTable) as *mut FSE_CTable;
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let nbSeq: usize = ((*seqStorePtr).sequences as usize)
        .wrapping_sub((*seqStorePtr).sequencesStart as usize)
        / core::mem::size_of::<SeqDef>();
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let mut lastCountSize: usize;
    let mut longOffsets: c_int = 0;

    let mut entropyWorkspace = count.wrapping_add(MaxSeq as usize + 1) as *mut c_void;
    let mut entropyWkspSize =
        entropyWkspSize.wrapping_sub((MaxSeq as usize + 1) * core::mem::size_of::<c_uint>());

    /* Compress literals */
    {
        let numSequences: usize = ((*seqStorePtr).sequences as usize)
            .wrapping_sub((*seqStorePtr).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>();
        let suspectUncompressible: c_int = ((numSequences == 0)
            || (litSize / numSequences >= SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO))
            as c_int;

        let cSize = crate::zstd_compress_literals::ZSTD_compressLiterals(
            op as *mut c_void,
            dstCapacity,
            literals,
            litSize,
            entropyWorkspace,
            entropyWkspSize,
            addr_of!((*prevEntropy).huf),
            addr_of_mut!((*nextEntropy).huf),
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
    if ((oend as usize).wrapping_sub(op as usize)) < 3 + 1 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbSeq < 128 {
        *op = nbSeq as BYTE;
        op = op.wrapping_add(1);
    } else if (nbSeq as U32) < LONGNBSEQ {
        *op.offset(0) = ((nbSeq >> 8) + 0x80) as BYTE;
        *op.offset(1) = nbSeq as BYTE;
        op = op.wrapping_add(2);
    } else {
        *op.offset(0) = 0xFF;
        MEM_writeLE16(op.wrapping_add(1), (nbSeq.wrapping_sub(LONGNBSEQ as usize)) as U16);
        op = op.wrapping_add(3);
    }
    if nbSeq == 0 {
        /* Copy the old tables over as if we repeated them */
        ZSTD_memcpy(
            addr_of_mut!((*nextEntropy).fse) as *mut u8,
            addr_of!((*prevEntropy).fse) as *const u8,
            core::mem::size_of::<ZSTD_fseCTables_t>(),
        );
        return (op as usize).wrapping_sub(ostart as usize);
    }
    {
        let seqHead: *mut BYTE = op;
        op = op.wrapping_add(1);
        /* build stats for sequences */
        let stats = ZSTD_buildSequencesStatistics(
            seqStorePtr,
            nbSeq,
            addr_of!((*prevEntropy).fse),
            addr_of_mut!((*nextEntropy).fse),
            op,
            oend as *const BYTE,
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
        let bitstreamSize = ZSTD_encodeSequences(
            op as *mut c_void,
            (oend as usize).wrapping_sub(op as usize),
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

    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTD_entropyCompressSeqStore_wExtLitBuffer(
    dst: *mut c_void,
    dstCapacity: usize,
    literals: *const c_void,
    litSize: usize,
    blockSize: usize,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    let cSize = ZSTD_entropyCompressSeqStore_internal(
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
    /* When srcSize <= dstCapacity, there is enough space to write a raw uncompressed block. */
    if ((cSize == ERROR(ZSTD_error_dstSize_tooSmall)) as c_int & (blockSize <= dstCapacity) as c_int)
        != 0
    {
        return 0; /* block not compressed */
    }
    if ERR_isError(cSize) != 0 {
        return cSize;
    }

    /* Check compressibility */
    {
        let maxCSize =
            blockSize.wrapping_sub(ZSTD_minGain(blockSize, (*cctxParams).cParams.strategy));
        if cSize >= maxCSize {
            return 0; /* block not compressed */
        }
    }
    cSize
}

pub unsafe fn ZSTD_entropyCompressSeqStore(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: usize,
    srcSize: usize,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    ZSTD_entropyCompressSeqStore_wExtLitBuffer(
        dst,
        dstCapacity,
        (*seqStorePtr).litStart as *const c_void,
        ((*seqStorePtr).lit as usize).wrapping_sub((*seqStorePtr).litStart as usize),
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

/* ---- block compressor selection ---- */

static blockCompressorTable: [[ZSTD_BlockCompressor_f; (ZSTD_STRATEGY_MAX + 1) as usize]; 4] = [
    [
        Some(crate::zstd_fast::ZSTD_compressBlock_fast), /* default for 0 */
        Some(crate::zstd_fast::ZSTD_compressBlock_fast),
        Some(crate::zstd_double_fast::ZSTD_compressBlock_doubleFast),
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2),
        Some(crate::zstd_lazy::ZSTD_compressBlock_btlazy2),
        Some(crate::zstd_opt::ZSTD_compressBlock_btopt),
        Some(crate::zstd_opt::ZSTD_compressBlock_btultra),
        Some(crate::zstd_opt::ZSTD_compressBlock_btultra2),
    ],
    [
        Some(crate::zstd_fast::ZSTD_compressBlock_fast_extDict), /* default for 0 */
        Some(crate::zstd_fast::ZSTD_compressBlock_fast_extDict),
        Some(crate::zstd_double_fast::ZSTD_compressBlock_doubleFast_extDict),
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_extDict),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_extDict),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_extDict),
        Some(crate::zstd_lazy::ZSTD_compressBlock_btlazy2_extDict),
        Some(crate::zstd_opt::ZSTD_compressBlock_btopt_extDict),
        Some(crate::zstd_opt::ZSTD_compressBlock_btultra_extDict),
        Some(crate::zstd_opt::ZSTD_compressBlock_btultra_extDict),
    ],
    [
        Some(crate::zstd_fast::ZSTD_compressBlock_fast_dictMatchState), /* default for 0 */
        Some(crate::zstd_fast::ZSTD_compressBlock_fast_dictMatchState),
        Some(crate::zstd_double_fast::ZSTD_compressBlock_doubleFast_dictMatchState),
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_dictMatchState),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_dictMatchState),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_dictMatchState),
        Some(crate::zstd_lazy::ZSTD_compressBlock_btlazy2_dictMatchState),
        Some(crate::zstd_opt::ZSTD_compressBlock_btopt_dictMatchState),
        Some(crate::zstd_opt::ZSTD_compressBlock_btultra_dictMatchState),
        Some(crate::zstd_opt::ZSTD_compressBlock_btultra_dictMatchState),
    ],
    [
        None, /* default for 0 */
        None,
        None,
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_dedicatedDictSearch),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_dedicatedDictSearch),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_dedicatedDictSearch),
        None,
        None,
        None,
        None,
    ],
];

static rowBasedBlockCompressors: [[ZSTD_BlockCompressor_f; 3]; 4] = [
    [
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_row),
    ],
    [
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_extDict_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_extDict_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_extDict_row),
    ],
    [
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_dictMatchState_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_dictMatchState_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_dictMatchState_row),
    ],
    [
        Some(crate::zstd_lazy::ZSTD_compressBlock_greedy_dedicatedDictSearch_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy_dedicatedDictSearch_row),
        Some(crate::zstd_lazy::ZSTD_compressBlock_lazy2_dedicatedDictSearch_row),
    ],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectBlockCompressor(
    strat: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    dictMode: ZSTD_dictMode_e,
) -> ZSTD_BlockCompressor_f {
    let selectedCompressor: ZSTD_BlockCompressor_f;

    if crate::zstd_compress::ZSTD_rowMatchFinderUsed(strat, useRowMatchFinder) != 0 {
        selectedCompressor =
            rowBasedBlockCompressors[dictMode as usize][(strat - ZSTD_greedy) as usize];
    } else {
        selectedCompressor = blockCompressorTable[dictMode as usize][strat as usize];
    }
    selectedCompressor
}

pub unsafe fn ZSTD_storeLastLiterals(
    seqStorePtr: *mut SeqStore_t,
    anchor: *const BYTE,
    lastLLSize: usize,
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

pub unsafe fn ZSTD_postProcessSequenceProducerResult(
    outSeqs: *mut ZSTD_Sequence,
    nbExternalSeqs: usize,
    outSeqsCapacity: usize,
    srcSize: usize,
) -> usize {
    if nbExternalSeqs > outSeqsCapacity {
        return ERROR(ZSTD_error_sequenceProducer_failed);
    }

    if nbExternalSeqs == 0 && srcSize > 0 {
        return ERROR(ZSTD_error_sequenceProducer_failed);
    }

    if srcSize == 0 {
        ZSTD_memset(
            outSeqs as *mut u8,
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

        if nbExternalSeqs == outSeqsCapacity {
            return ERROR(ZSTD_error_sequenceProducer_failed);
        }

        /* lastSeq is not a block delimiter, so we need to append one. */
        ZSTD_memset(
            outSeqs.wrapping_add(nbExternalSeqs) as *mut u8,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        nbExternalSeqs + 1
    }
}

pub unsafe fn ZSTD_fastSequenceLengthSum(
    seqBuf: *const ZSTD_Sequence,
    seqBufSize: usize,
) -> usize {
    let mut matchLenSum: usize;
    let mut litLenSum: usize;
    let mut i: usize;
    matchLenSum = 0;
    litLenSum = 0;
    i = 0;
    while i < seqBufSize {
        litLenSum = litLenSum.wrapping_add((*seqBuf.wrapping_add(i)).litLength as usize);
        matchLenSum = matchLenSum.wrapping_add((*seqBuf.wrapping_add(i)).matchLength as usize);
        i += 1;
    }
    litLenSum.wrapping_add(matchLenSum)
}

/* DEBUGLEVEL == 0 -> body compiles to nothing */
pub unsafe fn ZSTD_validateSeqStore(
    seqStore: *const SeqStore_t,
    cParams: *const ZSTD_compressionParameters,
) {
}

pub type ZSTD_BuildSeqStore_e = c_int;
pub const ZSTDbss_compress: ZSTD_BuildSeqStore_e = 0;
pub const ZSTDbss_noCompress: ZSTD_BuildSeqStore_e = 1;

pub unsafe fn ZSTD_buildSeqStore(
    zc: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ms: *mut ZSTD_MatchState_t = addr_of_mut!((*zc).blockState.matchState);
    /* TODO: See 3090. */
    if srcSize < MIN_CBLOCK_SIZE + ZSTD_blockHeaderSize + 1 + 1 {
        if (*zc).appliedParams.cParams.strategy >= ZSTD_btopt {
            crate::zstd_ldm::ZSTD_ldm_skipRawSeqStoreBytes(
                addr_of_mut!((*zc).externSeqStore),
                srcSize,
            );
        } else {
            crate::zstd_ldm::ZSTD_ldm_skipSequences(
                addr_of_mut!((*zc).externSeqStore),
                srcSize,
                (*zc).appliedParams.cParams.minMatch,
            );
        }
        return ZSTDbss_noCompress as usize; /* don't even attempt compression below a certain srcSize */
    }
    ZSTD_resetSeqStore(addr_of_mut!((*zc).seqStore));
    /* required for optimal parser to read stats from dictionary */
    (*ms).opt.symbolCosts = addr_of!((*(*zc).blockState.prevCBlock).entropy);
    /* tell the optimal parser how we expect to compress literals */
    (*ms).opt.literalCompressionMode = (*zc).appliedParams.literalCompressionMode;

    /* limited update after a very long match */
    {
        let base: *const BYTE = (*ms).window.base;
        let istart: *const BYTE = src as *const BYTE;
        let curr: U32 = (istart as usize).wrapping_sub(base as usize) as U32;
        if curr > (*ms).nextToUpdate.wrapping_add(384) {
            (*ms).nextToUpdate = curr.wrapping_sub(MIN(
                192u32,
                curr.wrapping_sub((*ms).nextToUpdate).wrapping_sub(384),
            ));
        }
    }

    /* select and store sequences */
    {
        let dictMode: ZSTD_dictMode_e = ZSTD_matchState_dictMode(ms);
        let mut lastLLSize: usize;
        {
            let mut i: c_int = 0;
            while i < ZSTD_REP_NUM as c_int {
                (*(*zc).blockState.nextCBlock).rep[i as usize] =
                    (*(*zc).blockState.prevCBlock).rep[i as usize];
                i += 1;
            }
        }
        if (*zc).externSeqStore.pos < (*zc).externSeqStore.size {
            if ZSTD_hasExtSeqProd(addr_of!((*zc).appliedParams)) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            /* Updates ldmSeqStore.pos */
            lastLLSize = crate::zstd_ldm::ZSTD_ldm_blockCompress(
                addr_of_mut!((*zc).externSeqStore),
                ms,
                addr_of_mut!((*zc).seqStore),
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
        } else if (*zc).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
            let mut ldmSeqStore: RawSeqStore_t = kNullRawSeqStore;

            if ZSTD_hasExtSeqProd(addr_of!((*zc).appliedParams)) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            ldmSeqStore.seq = (*zc).ldmSequences;
            ldmSeqStore.capacity = (*zc).maxNbLdmSequences;
            /* Updates ldmSeqStore.size */
            {
                let err_code = crate::zstd_ldm::ZSTD_ldm_generateSequences(
                    addr_of_mut!((*zc).ldmState),
                    &mut ldmSeqStore,
                    addr_of!((*zc).appliedParams.ldmParams),
                    src,
                    srcSize,
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            /* Updates ldmSeqStore.pos */
            lastLLSize = crate::zstd_ldm::ZSTD_ldm_blockCompress(
                &mut ldmSeqStore,
                ms,
                addr_of_mut!((*zc).seqStore),
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
        } else if ZSTD_hasExtSeqProd(addr_of!((*zc).appliedParams)) != 0 {
            {
                let windowSize: U32 = (1u32) << (*zc).appliedParams.cParams.windowLog;

                let nbExternalSeqs = ((*zc).appliedParams.extSeqProdFunc.unwrap())(
                    (*zc).appliedParams.extSeqProdState,
                    (*zc).extSeqBuf,
                    (*zc).extSeqBufCapacity,
                    src,
                    srcSize,
                    core::ptr::null(),
                    0, /* dict and dictSize, currently not supported */
                    (*zc).appliedParams.compressionLevel,
                    windowSize as usize,
                );

                let nbPostProcessedSeqs = ZSTD_postProcessSequenceProducerResult(
                    (*zc).extSeqBuf,
                    nbExternalSeqs,
                    (*zc).extSeqBufCapacity,
                    srcSize,
                );

                /* Return early if there is no error, since we don't need to worry about last literals */
                if ERR_isError(nbPostProcessedSeqs) == 0 {
                    let mut seqPos = ZSTD_SequencePosition {
                        idx: 0,
                        posInSequence: 0,
                        posInSrc: 0,
                    };
                    let seqLenSum =
                        ZSTD_fastSequenceLengthSum((*zc).extSeqBuf, nbPostProcessedSeqs);
                    if seqLenSum > srcSize {
                        return ERROR(ZSTD_error_externalSequences_invalid);
                    }
                    {
                        let err_code =
                            crate::zstd_compress_p4::ZSTD_transferSequences_wBlockDelim(
                                zc,
                                &mut seqPos,
                                (*zc).extSeqBuf,
                                nbPostProcessedSeqs,
                                src,
                                srcSize,
                                (*zc).appliedParams.searchForExternalRepcodes,
                            );
                        if ERR_isError(err_code) != 0 {
                            return err_code;
                        }
                    }
                    (*ms).ldmSeqStore = core::ptr::null();
                    return ZSTDbss_compress as usize;
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
                    (*ms).ldmSeqStore = core::ptr::null();
                    lastLLSize = (blockCompressor.unwrap())(
                        ms,
                        addr_of_mut!((*zc).seqStore),
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
            (*ms).ldmSeqStore = core::ptr::null();
            lastLLSize = (blockCompressor.unwrap())(
                ms,
                addr_of_mut!((*zc).seqStore),
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                src,
                srcSize,
            );
        }
        {
            let lastLiterals: *const BYTE = (src as *const BYTE)
                .wrapping_add(srcSize)
                .wrapping_sub(lastLLSize);
            ZSTD_storeLastLiterals(addr_of_mut!((*zc).seqStore), lastLiterals, lastLLSize);
        }
    }
    ZSTD_validateSeqStore(
        addr_of!((*zc).seqStore),
        addr_of!((*zc).appliedParams.cParams),
    );
    ZSTDbss_compress as usize
}

pub unsafe fn ZSTD_copyBlockSequences(
    seqCollector: *mut SeqCollector,
    seqStore: *const SeqStore_t,
    prevRepcodes: *const U32,
) -> usize {
    let inSeqs: *const SeqDef = (*seqStore).sequencesStart;
    let nbInSequences: usize =
        ((*seqStore).sequences as usize).wrapping_sub(inSeqs as usize)
            / core::mem::size_of::<SeqDef>();
    let nbInLiterals: usize =
        ((*seqStore).lit as usize).wrapping_sub((*seqStore).litStart as usize);

    let outSeqs: *mut ZSTD_Sequence = if (*seqCollector).seqIndex == 0 {
        (*seqCollector).seqStart
    } else {
        (*seqCollector).seqStart.wrapping_add((*seqCollector).seqIndex)
    };
    let nbOutSequences: usize = nbInSequences + 1;
    let mut nbOutLiterals: usize = 0;
    let mut repcodes = Repcodes_t::default();
    let mut i: usize;

    /* Bounds check that we have enough space for every input sequence
     * and the block delimiter
     */
    if nbOutSequences > (*seqCollector).maxSequences.wrapping_sub((*seqCollector).seqIndex) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    ZSTD_memcpy(
        addr_of_mut!(repcodes) as *mut u8,
        prevRepcodes as *const u8,
        core::mem::size_of::<Repcodes_t>(),
    );
    i = 0;
    while i < nbInSequences {
        let mut rawOffset: U32;
        (*outSeqs.wrapping_add(i)).litLength = (*inSeqs.wrapping_add(i)).litLength as c_uint;
        (*outSeqs.wrapping_add(i)).matchLength =
            ((*inSeqs.wrapping_add(i)).mlBase as c_uint).wrapping_add(MINMATCH);
        (*outSeqs.wrapping_add(i)).rep = 0;

        if i == (*seqStore).longLengthPos as usize {
            if (*seqStore).longLengthType == ZSTD_llt_literalLength {
                (*outSeqs.wrapping_add(i)).litLength =
                    (*outSeqs.wrapping_add(i)).litLength.wrapping_add(0x10000);
            } else if (*seqStore).longLengthType == ZSTD_llt_matchLength {
                (*outSeqs.wrapping_add(i)).matchLength =
                    (*outSeqs.wrapping_add(i)).matchLength.wrapping_add(0x10000);
            }
        }

        /* Determine the raw offset given the offBase, which may be a repcode. */
        if OFFBASE_IS_REPCODE((*inSeqs.wrapping_add(i)).offBase) {
            let repcode: U32 = OFFBASE_TO_REPCODE((*inSeqs.wrapping_add(i)).offBase);
            (*outSeqs.wrapping_add(i)).rep = repcode as c_uint;
            if (*outSeqs.wrapping_add(i)).litLength != 0 {
                rawOffset = repcodes.rep[(repcode - 1) as usize];
            } else {
                if repcode == 3 {
                    rawOffset = repcodes.rep[0].wrapping_sub(1);
                } else {
                    rawOffset = repcodes.rep[repcode as usize];
                }
            }
        } else {
            rawOffset = OFFBASE_TO_OFFSET((*inSeqs.wrapping_add(i)).offBase);
        }
        (*outSeqs.wrapping_add(i)).offset = rawOffset as c_uint;

        /* Update repcode history for the sequence */
        ZSTD_updateRep(
            repcodes.rep.as_mut_ptr(),
            (*inSeqs.wrapping_add(i)).offBase,
            ((*inSeqs.wrapping_add(i)).litLength == 0) as U32,
        );

        nbOutLiterals =
            nbOutLiterals.wrapping_add((*outSeqs.wrapping_add(i)).litLength as usize);
        i += 1;
    }
    {
        let lastLLSize: usize = nbInLiterals.wrapping_sub(nbOutLiterals);
        (*outSeqs.wrapping_add(nbInSequences)).litLength = lastLLSize as c_uint;
        (*outSeqs.wrapping_add(nbInSequences)).matchLength = 0;
        (*outSeqs.wrapping_add(nbInSequences)).offset = 0;
    }
    (*seqCollector).seqIndex = (*seqCollector).seqIndex.wrapping_add(nbOutSequences);

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_sequenceBound(srcSize: usize) -> usize {
    let maxNbSeq: usize = (srcSize / ZSTD_MINMATCH_MIN as usize) + 1;
    let maxNbDelims: usize = (srcSize / ZSTD_BLOCKSIZE_MAX_MIN as usize) + 1;
    maxNbSeq + maxNbDelims
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_generateSequences(
    zc: *mut ZSTD_CCtx,
    outSeqs: *mut ZSTD_Sequence,
    outSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let dstCapacity: usize = crate::zstd_compress::ZSTD_compressBound(srcSize);
    let dst: *mut c_void;
    let mut seqCollector = SeqCollector::default();
    {
        let mut targetCBlockSize: c_int = 0;
        {
            let err_code = crate::zstd_compress::ZSTD_CCtx_getParameter(
                zc,
                ZSTD_c_targetCBlockSize,
                &mut targetCBlockSize,
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if targetCBlockSize != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    }
    {
        let mut nbWorkers: c_int = 0;
        {
            let err_code = crate::zstd_compress::ZSTD_CCtx_getParameter(
                zc,
                ZSTD_c_nbWorkers,
                &mut nbWorkers,
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if nbWorkers != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    }

    dst = ZSTD_customMalloc(dstCapacity, ZSTD_defaultCMem) as *mut c_void;
    if dst.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }

    seqCollector.collectSequences = 1;
    seqCollector.seqStart = outSeqs;
    seqCollector.seqIndex = 0;
    seqCollector.maxSequences = outSeqsSize;
    (*zc).seqCollector = seqCollector;

    {
        let ret = crate::zstd_compress_p4::ZSTD_compress2(zc, dst, dstCapacity, src, srcSize);
        ZSTD_customFree(dst as *mut u8, ZSTD_defaultCMem);
        if ERR_isError(ret) != 0 {
            return ret;
        }
    }
    (*zc).seqCollector.seqIndex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_mergeBlockDelimiters(
    sequences: *mut ZSTD_Sequence,
    seqsSize: usize,
) -> usize {
    let mut in_: usize = 0;
    let mut out: usize = 0;
    while in_ < seqsSize {
        if (*sequences.wrapping_add(in_)).offset == 0
            && (*sequences.wrapping_add(in_)).matchLength == 0
        {
            if in_ != seqsSize - 1 {
                (*sequences.wrapping_add(in_ + 1)).litLength = (*sequences
                    .wrapping_add(in_ + 1))
                .litLength
                .wrapping_add((*sequences.wrapping_add(in_)).litLength);
            }
        } else {
            *sequences.wrapping_add(out) = *sequences.wrapping_add(in_);
            out += 1;
        }
        in_ += 1;
    }
    out
}

/* Unrolled loop to read four size_ts of input at a time. Returns 1 if is RLE, 0 if not. */
pub unsafe fn ZSTD_isRLE(src: *const BYTE, length: usize) -> c_int {
    let ip: *const BYTE = src;
    let value: BYTE = *ip.offset(0);
    let valueST: usize = ((value as U64).wrapping_mul(0x0101010101010101u64)) as usize;
    let unrollSize: usize = core::mem::size_of::<usize>() * 4;
    let unrollMask: usize = unrollSize - 1;
    let prefixLength: usize = length & unrollMask;
    let mut i: usize;
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
        let mut u: usize = 0;
        while u < unrollSize {
            if MEM_readST(ip.wrapping_add(i).wrapping_add(u)) != valueST {
                return 0;
            }
            u += core::mem::size_of::<usize>();
        }
        i = i.wrapping_add(unrollSize);
    }
    1
}

pub unsafe fn ZSTD_maybeRLE(seqStore: *const SeqStore_t) -> c_int {
    let nbSeqs: usize = ((*seqStore).sequences as usize)
        .wrapping_sub((*seqStore).sequencesStart as usize)
        / core::mem::size_of::<SeqDef>();
    let nbLits: usize =
        ((*seqStore).lit as usize).wrapping_sub((*seqStore).litStart as usize);

    ((nbSeqs < 4) && (nbLits < 10)) as c_int
}

pub unsafe fn ZSTD_blockState_confirmRepcodesAndEntropyTables(bs: *mut ZSTD_blockState_t) {
    let tmp: *mut ZSTD_compressedBlockState_t = (*bs).prevCBlock;
    (*bs).prevCBlock = (*bs).nextCBlock;
    (*bs).nextCBlock = tmp;
}

/* Writes the block header */
pub unsafe fn writeBlockHeader(op: *mut c_void, cSize: usize, blockSize: usize, lastBlock: U32) {
    let cBlockHeader: U32 = if cSize == 1 {
        lastBlock
            .wrapping_add((bt_rle as U32) << 1)
            .wrapping_add((blockSize << 3) as U32)
    } else {
        lastBlock
            .wrapping_add((bt_compressed as U32) << 1)
            .wrapping_add((cSize << 3) as U32)
    };
    MEM_writeLE24(op as *mut u8, cBlockHeader);
}

pub const COMPRESS_LITERALS_SIZE_MIN: usize = 63;

pub unsafe fn ZSTD_buildBlockEntropyStats_literals(
    src: *mut c_void,
    srcSize: usize,
    prevHuf: *const ZSTD_hufCTables_t,
    nextHuf: *mut ZSTD_hufCTables_t,
    hufMetadata: *mut ZSTD_hufCTablesMetadata_t,
    literalsCompressionIsDisabled: c_int,
    workspace: *mut c_void,
    wkspSize: usize,
    hufFlags: c_int,
) -> usize {
    let wkspStart: *mut BYTE = workspace as *mut BYTE;
    let wkspEnd: *mut BYTE = wkspStart.wrapping_add(wkspSize);
    let countWkspStart: *mut BYTE = wkspStart;
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let countWkspSize: usize =
        (HUF_SYMBOLVALUE_MAX as usize + 1) * core::mem::size_of::<c_uint>();
    let nodeWksp: *mut BYTE = countWkspStart.wrapping_add(countWkspSize);
    let nodeWkspSize: usize = (wkspEnd as usize).wrapping_sub(nodeWksp as usize);
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX;
    let mut huffLog: c_uint = LitHufLog;
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
        let minLitSize: usize = if (*prevHuf).repeatMode == HUF_repeat_valid {
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
        let largest = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            src as *const c_void,
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
        && crate::huf_compress::HUF_validateCTable(
            (*prevHuf).CTable.as_ptr(),
            countWksp as *const c_uint,
            maxSymbolValue,
        ) == 0
    {
        repeat = HUF_repeat_none;
    }

    /* Build Huffman Tree */
    ZSTD_memset(
        (*nextHuf).CTable.as_mut_ptr() as *mut u8,
        0,
        core::mem::size_of_val(&(*nextHuf).CTable),
    );
    huffLog = crate::huf_compress::HUF_optimalTableLog(
        huffLog,
        srcSize,
        maxSymbolValue,
        nodeWksp as *mut c_void,
        nodeWkspSize,
        (*nextHuf).CTable.as_mut_ptr(),
        countWksp as *const c_uint,
        hufFlags,
    );
    {
        let maxBits = crate::huf_compress::HUF_buildCTable_wksp(
            (*nextHuf).CTable.as_mut_ptr(),
            countWksp as *const c_uint,
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
        let newCSize = crate::huf_compress::HUF_estimateCompressedSize(
            (*nextHuf).CTable.as_ptr(),
            countWksp as *const c_uint,
            maxSymbolValue,
        );
        let hSize = crate::huf_compress::HUF_writeCTable_wksp(
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
            let oldCSize = crate::huf_compress::HUF_estimateCompressedSize(
                (*prevHuf).CTable.as_ptr(),
                countWksp as *const c_uint,
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
        return hSize;
    }
}

pub unsafe fn ZSTD_buildDummySequencesStatistics(
    nextEntropy: *mut ZSTD_fseCTables_t,
) -> ZSTD_symbolEncodingTypeStats_t {
    let stats = ZSTD_symbolEncodingTypeStats_t {
        LLtype: set_basic as U32,
        Offtype: set_basic as U32,
        MLtype: set_basic as U32,
        size: 0,
        lastCountSize: 0,
        longOffsets: 0,
    };
    (*nextEntropy).litlength_repeatMode = FSE_repeat_none;
    (*nextEntropy).offcode_repeatMode = FSE_repeat_none;
    (*nextEntropy).matchlength_repeatMode = FSE_repeat_none;
    stats
}

pub unsafe fn ZSTD_buildBlockEntropyStats_sequences(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    fseMetadata: *mut ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let nbSeq: usize = ((*seqStorePtr).sequences as usize)
        .wrapping_sub((*seqStorePtr).sequencesStart as usize)
        / core::mem::size_of::<SeqDef>();
    let ostart: *mut BYTE = (*fseMetadata).fseTablesBuffer.as_mut_ptr();
    let oend: *mut BYTE =
        ostart.wrapping_add(core::mem::size_of_val(&(*fseMetadata).fseTablesBuffer));
    let op: *mut BYTE = ostart;
    let countWorkspace: *mut c_uint = workspace as *mut c_uint;
    let entropyWorkspace: *mut c_uint = countWorkspace.wrapping_add(MaxSeq as usize + 1);
    let entropyWorkspaceSize: usize =
        wkspSize.wrapping_sub((MaxSeq as usize + 1) * core::mem::size_of::<c_uint>());
    let stats: ZSTD_symbolEncodingTypeStats_t;

    stats = if nbSeq != 0 {
        ZSTD_buildSequencesStatistics(
            seqStorePtr,
            nbSeq,
            prevEntropy,
            nextEntropy,
            op,
            oend as *const BYTE,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildBlockEntropyStats(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let litSize: usize =
        ((*seqStorePtr).lit as usize).wrapping_sub((*seqStorePtr).litStart as usize);
    let huf_useOptDepth: c_int =
        ((*cctxParams).cParams.strategy >= HUF_OPTIMAL_DEPTH_THRESHOLD) as c_int;
    let hufFlags: c_int = if huf_useOptDepth != 0 {
        HUF_flags_optimalDepth
    } else {
        0
    };

    (*entropyMetadata).hufMetadata.hufDesSize = ZSTD_buildBlockEntropyStats_literals(
        (*seqStorePtr).litStart as *mut c_void,
        litSize,
        addr_of!((*prevEntropy).huf),
        addr_of_mut!((*nextEntropy).huf),
        addr_of_mut!((*entropyMetadata).hufMetadata),
        ZSTD_literalsCompressionIsDisabled(cctxParams),
        workspace,
        wkspSize,
        hufFlags,
    );

    {
        let err_code = (*entropyMetadata).hufMetadata.hufDesSize;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    (*entropyMetadata).fseMetadata.fseTablesSize = ZSTD_buildBlockEntropyStats_sequences(
        seqStorePtr,
        addr_of!((*prevEntropy).fse),
        addr_of_mut!((*nextEntropy).fse),
        cctxParams,
        addr_of_mut!((*entropyMetadata).fseMetadata),
        workspace,
        wkspSize,
    );
    {
        let err_code = (*entropyMetadata).fseMetadata.fseTablesSize;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

/* Returns the size estimate for the literals section (header + content) of a block */
pub unsafe fn ZSTD_estimateBlockSize_literal(
    literals: *const BYTE,
    litSize: usize,
    huf: *const ZSTD_hufCTables_t,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeEntropy: c_int,
) -> usize {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX;
    let literalSectionHeaderSize: usize =
        3 + (litSize >= (1 * (1 << 10))) as usize + (litSize >= (16 * (1 << 10))) as usize;
    let singleStream: U32 = (litSize < 256) as U32;

    if (*hufMetadata).hType == set_basic {
        return litSize;
    } else if (*hufMetadata).hType == set_rle {
        return 1;
    } else if (*hufMetadata).hType == set_compressed || (*hufMetadata).hType == set_repeat {
        let largest = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            literals as *const c_void,
            litSize,
            workspace,
            wkspSize,
        );
        if ERR_isError(largest) != 0 {
            return litSize;
        }
        {
            let mut cLitSizeEstimate = crate::huf_compress::HUF_estimateCompressedSize(
                (*huf).CTable.as_ptr(),
                countWksp as *const c_uint,
                maxSymbolValue,
            );
            if writeEntropy != 0 {
                cLitSizeEstimate =
                    cLitSizeEstimate.wrapping_add((*hufMetadata).hufDesSize);
            }
            if singleStream == 0 {
                cLitSizeEstimate = cLitSizeEstimate.wrapping_add(6);
            }
            return cLitSizeEstimate.wrapping_add(literalSectionHeaderSize);
        }
    }
    0
}

/* Returns the size estimate for the FSE-compressed symbols (of, ml, ll) of a block */
pub unsafe fn ZSTD_estimateBlockSize_symbolType(
    type_: SymbolEncodingType_e,
    codeTable: *const BYTE,
    nbSeq: usize,
    maxCode: c_uint,
    fseCTable: *const FSE_CTable,
    additionalBits: *const U8,
    defaultNorm: *const c_short,
    defaultNormLog: U32,
    defaultMax: U32,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut ctp: *const BYTE = codeTable;
    let ctStart: *const BYTE = ctp;
    let ctEnd: *const BYTE = ctStart.wrapping_add(nbSeq);
    let mut cSymbolTypeSizeEstimateInBits: usize = 0;
    let mut max: c_uint = maxCode;

    HIST_countFast_wksp(
        countWksp,
        &mut max,
        codeTable as *const c_void,
        nbSeq,
        workspace,
        wkspSize,
    );
    if type_ == set_basic {
        cSymbolTypeSizeEstimateInBits =
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, countWksp as *const c_uint, max);
    } else if type_ == set_rle {
        cSymbolTypeSizeEstimateInBits = 0;
    } else if type_ == set_compressed || type_ == set_repeat {
        cSymbolTypeSizeEstimateInBits =
            ZSTD_fseBitCost(fseCTable, countWksp as *const c_uint, max);
    }
    if ERR_isError(cSymbolTypeSizeEstimateInBits) != 0 {
        return nbSeq.wrapping_mul(10);
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits = cSymbolTypeSizeEstimateInBits
                .wrapping_add(*additionalBits.offset(*ctp as isize) as usize);
        } else {
            cSymbolTypeSizeEstimateInBits =
                cSymbolTypeSizeEstimateInBits.wrapping_add(*ctp as usize);
        }
        ctp = ctp.wrapping_add(1);
    }
    cSymbolTypeSizeEstimateInBits >> 3
}

/* Returns the size estimate for the sequences section (header + content) of a block */
pub unsafe fn ZSTD_estimateBlockSize_sequences(
    ofCodeTable: *const BYTE,
    llCodeTable: *const BYTE,
    mlCodeTable: *const BYTE,
    nbSeq: usize,
    fseTables: *const ZSTD_fseCTables_t,
    fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeEntropy: c_int,
) -> usize {
    let sequencesSectionHeaderSize: usize =
        1 + 1 + (nbSeq >= 128) as usize + (nbSeq >= LONGNBSEQ as usize) as usize;
    let mut cSeqSizeEstimate: usize = 0;
    cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add(ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).ofType,
        ofCodeTable,
        nbSeq,
        MaxOff,
        addr_of!((*fseTables).offcodeCTable) as *const FSE_CTable,
        core::ptr::null(),
        OF_defaultNorm.as_ptr() as *const c_short,
        OF_defaultNormLog,
        DefaultMaxOff,
        workspace,
        wkspSize,
    ));
    cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add(ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).llType,
        llCodeTable,
        nbSeq,
        MaxLL,
        addr_of!((*fseTables).litlengthCTable) as *const FSE_CTable,
        LL_bits.as_ptr(),
        LL_defaultNorm.as_ptr() as *const c_short,
        LL_defaultNormLog,
        MaxLL,
        workspace,
        wkspSize,
    ));
    cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add(ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).mlType,
        mlCodeTable,
        nbSeq,
        MaxML,
        addr_of!((*fseTables).matchlengthCTable) as *const FSE_CTable,
        ML_bits.as_ptr(),
        ML_defaultNorm.as_ptr() as *const c_short,
        ML_defaultNormLog,
        MaxML,
        workspace,
        wkspSize,
    ));
    if writeEntropy != 0 {
        cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add((*fseMetadata).fseTablesSize);
    }
    cSeqSizeEstimate.wrapping_add(sequencesSectionHeaderSize)
}

/* Returns the size estimate for a given stream of literals, of, ll, ml */
pub unsafe fn ZSTD_estimateBlockSize(
    literals: *const BYTE,
    litSize: usize,
    ofCodeTable: *const BYTE,
    llCodeTable: *const BYTE,
    mlCodeTable: *const BYTE,
    nbSeq: usize,
    entropy: *const ZSTD_entropyCTables_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeLitEntropy: c_int,
    writeSeqEntropy: c_int,
) -> usize {
    let literalsSize = ZSTD_estimateBlockSize_literal(
        literals,
        litSize,
        addr_of!((*entropy).huf),
        addr_of!((*entropyMetadata).hufMetadata),
        workspace,
        wkspSize,
        writeLitEntropy,
    );
    let seqSize = ZSTD_estimateBlockSize_sequences(
        ofCodeTable,
        llCodeTable,
        mlCodeTable,
        nbSeq,
        addr_of!((*entropy).fse),
        addr_of!((*entropyMetadata).fseMetadata),
        workspace,
        wkspSize,
        writeSeqEntropy,
    );
    seqSize
        .wrapping_add(literalsSize)
        .wrapping_add(ZSTD_blockHeaderSize)
}

pub unsafe fn ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(
    seqStore: *mut SeqStore_t,
    zc: *mut ZSTD_CCtx,
) -> usize {
    let entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t =
        addr_of_mut!((*zc).blockSplitCtx.entropyMetadata);
    {
        let err_code = ZSTD_buildBlockEntropyStats(
            seqStore,
            addr_of!((*(*zc).blockState.prevCBlock).entropy),
            addr_of_mut!((*(*zc).blockState.nextCBlock).entropy),
            addr_of!((*zc).appliedParams),
            entropyMetadata,
            (*zc).tmpWorkspace,
            (*zc).tmpWkspSize,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_estimateBlockSize(
        (*seqStore).litStart,
        ((*seqStore).lit as usize).wrapping_sub((*seqStore).litStart as usize),
        (*seqStore).ofCode,
        (*seqStore).llCode,
        (*seqStore).mlCode,
        ((*seqStore).sequences as usize).wrapping_sub((*seqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>(),
        addr_of!((*(*zc).blockState.nextCBlock).entropy),
        entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
        ((*entropyMetadata).hufMetadata.hType == set_compressed) as c_int,
        1,
    )
}

/* Returns literals bytes represented in a seqStore */
pub unsafe fn ZSTD_countSeqStoreLiteralsBytes(seqStore: *const SeqStore_t) -> usize {
    let mut literalsBytes: usize = 0;
    let nbSeqs: usize = ((*seqStore).sequences as usize)
        .wrapping_sub((*seqStore).sequencesStart as usize)
        / core::mem::size_of::<SeqDef>();
    let mut i: usize = 0;
    while i < nbSeqs {
        let seq: SeqDef = *(*seqStore).sequencesStart.wrapping_add(i);
        literalsBytes = literalsBytes.wrapping_add(seq.litLength as usize);
        if i == (*seqStore).longLengthPos as usize
            && (*seqStore).longLengthType == ZSTD_llt_literalLength
        {
            literalsBytes = literalsBytes.wrapping_add(0x10000);
        }
        i += 1;
    }
    literalsBytes
}

/* Returns match bytes represented in a seqStore */
pub unsafe fn ZSTD_countSeqStoreMatchBytes(seqStore: *const SeqStore_t) -> usize {
    let mut matchBytes: usize = 0;
    let nbSeqs: usize = ((*seqStore).sequences as usize)
        .wrapping_sub((*seqStore).sequencesStart as usize)
        / core::mem::size_of::<SeqDef>();
    let mut i: usize = 0;
    while i < nbSeqs {
        let seq: SeqDef = *(*seqStore).sequencesStart.wrapping_add(i);
        matchBytes = matchBytes.wrapping_add(seq.mlBase as usize + MINMATCH as usize);
        if i == (*seqStore).longLengthPos as usize
            && (*seqStore).longLengthType == ZSTD_llt_matchLength
        {
            matchBytes = matchBytes.wrapping_add(0x10000);
        }
        i += 1;
    }
    matchBytes
}

pub unsafe fn ZSTD_deriveSeqStoreChunk(
    resultSeqStore: *mut SeqStore_t,
    originalSeqStore: *const SeqStore_t,
    startIdx: usize,
    endIdx: usize,
) {
    *resultSeqStore = *originalSeqStore;
    if startIdx > 0 {
        (*resultSeqStore).sequences =
            (*originalSeqStore).sequencesStart.wrapping_add(startIdx);
        (*resultSeqStore).litStart = (*resultSeqStore)
            .litStart
            .wrapping_add(ZSTD_countSeqStoreLiteralsBytes(resultSeqStore));
    }

    /* Move longLengthPos into the correct position if necessary */
    if (*originalSeqStore).longLengthType != ZSTD_llt_none {
        if ((*originalSeqStore).longLengthPos as usize) < startIdx
            || (*originalSeqStore).longLengthPos as usize > endIdx
        {
            (*resultSeqStore).longLengthType = ZSTD_llt_none;
        } else {
            (*resultSeqStore).longLengthPos =
                (*resultSeqStore).longLengthPos.wrapping_sub(startIdx as U32);
        }
    }
    (*resultSeqStore).sequencesStart =
        (*originalSeqStore).sequencesStart.wrapping_add(startIdx);
    (*resultSeqStore).sequences = (*originalSeqStore).sequencesStart.wrapping_add(endIdx);
    if endIdx
        == ((*originalSeqStore).sequences as usize)
            .wrapping_sub((*originalSeqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>()
    {
        /* This accounts for possible last literals if the derived chunk reaches the end of the block */
    } else {
        let literalsBytes = ZSTD_countSeqStoreLiteralsBytes(resultSeqStore);
        (*resultSeqStore).lit = (*resultSeqStore).litStart.wrapping_add(literalsBytes);
    }
    (*resultSeqStore).llCode = (*resultSeqStore).llCode.wrapping_add(startIdx);
    (*resultSeqStore).mlCode = (*resultSeqStore).mlCode.wrapping_add(startIdx);
    (*resultSeqStore).ofCode = (*resultSeqStore).ofCode.wrapping_add(startIdx);
}

pub unsafe fn ZSTD_resolveRepcodeToRawOffset(
    rep: *const U32,
    offBase: U32,
    ll0: U32,
) -> U32 {
    let adjustedRepCode: U32 = OFFBASE_TO_REPCODE(offBase).wrapping_sub(1).wrapping_add(ll0);
    if adjustedRepCode == ZSTD_REP_NUM as U32 {
        return (*rep.offset(0)).wrapping_sub(1);
    }
    *rep.offset(adjustedRepCode as isize)
}

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
        idx = idx.wrapping_add(1);
    }
}

pub unsafe fn ZSTD_compressSeqStore_singleBlock(
    zc: *mut ZSTD_CCtx,
    seqStore: *const SeqStore_t,
    dRep: *mut Repcodes_t,
    cRep: *mut Repcodes_t,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
    isPartition: U32,
) -> usize {
    let rleMaxLength: U32 = 25;
    let op: *mut BYTE = dst as *mut BYTE;
    let ip: *const BYTE = src as *const BYTE;
    let mut cSize: usize;
    let mut cSeqsSize: usize;

    /* In case of an RLE or raw block, the simulated decompression repcode history must be reset */
    let dRepOriginal: Repcodes_t = *dRep;
    if isPartition != 0 {
        ZSTD_seqStore_resolveOffCodes(
            dRep,
            cRep,
            seqStore,
            (((*seqStore).sequences as usize)
                .wrapping_sub((*seqStore).sequencesStart as usize)
                / core::mem::size_of::<SeqDef>()) as U32,
        );
    }

    if dstCapacity < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    cSeqsSize = ZSTD_entropyCompressSeqStore(
        seqStore,
        addr_of!((*(*zc).blockState.prevCBlock).entropy),
        addr_of_mut!((*(*zc).blockState.nextCBlock).entropy),
        addr_of!((*zc).appliedParams),
        op.wrapping_add(ZSTD_blockHeaderSize) as *mut c_void,
        dstCapacity.wrapping_sub(ZSTD_blockHeaderSize),
        srcSize,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
        (*zc).bmi2,
    );
    if ERR_isError(cSeqsSize) != 0 {
        return cSeqsSize;
    }

    if (*zc).isFirstBlock == 0
        && cSeqsSize < rleMaxLength as usize
        && ZSTD_isRLE(src as *const BYTE, srcSize) != 0
    {
        cSeqsSize = 1;
    }

    /* Sequence collection not supported when block splitting */
    if (*zc).seqCollector.collectSequences != 0 {
        {
            let err_code = ZSTD_copyBlockSequences(
                addr_of_mut!((*zc).seqCollector),
                seqStore,
                dRepOriginal.rep.as_ptr(),
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        ZSTD_blockState_confirmRepcodesAndEntropyTables(addr_of_mut!((*zc).blockState));
        return 0;
    }

    if cSeqsSize == 0 {
        cSize = ZSTD_noCompressBlock(
            op as *mut c_void,
            dstCapacity,
            ip as *const c_void,
            srcSize,
            lastBlock,
        );
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
        ZSTD_blockState_confirmRepcodesAndEntropyTables(addr_of_mut!((*zc).blockState));
        writeBlockHeader(op as *mut c_void, cSeqsSize, srcSize, lastBlock);
        cSize = ZSTD_blockHeaderSize.wrapping_add(cSeqsSize);
    }

    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

/* Struct to keep track of where we are in our recursive calls. */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct seqStoreSplits {
    pub splitLocations: *mut U32,
    pub idx: usize,
}

pub const MIN_SEQUENCES_BLOCK_SPLITTING: usize = 300;

pub unsafe fn ZSTD_deriveBlockSplitsHelper(
    splits: *mut seqStoreSplits,
    startIdx: usize,
    endIdx: usize,
    zc: *mut ZSTD_CCtx,
    origSeqStore: *const SeqStore_t,
) {
    let fullSeqStoreChunk: *mut SeqStore_t = addr_of_mut!((*zc).blockSplitCtx.fullSeqStoreChunk);
    let firstHalfSeqStore: *mut SeqStore_t = addr_of_mut!((*zc).blockSplitCtx.firstHalfSeqStore);
    let secondHalfSeqStore: *mut SeqStore_t =
        addr_of_mut!((*zc).blockSplitCtx.secondHalfSeqStore);
    let estimatedOriginalSize: usize;
    let estimatedFirstHalfSize: usize;
    let estimatedSecondHalfSize: usize;
    let midIdx: usize = (startIdx + endIdx) / 2;

    if endIdx.wrapping_sub(startIdx) < MIN_SEQUENCES_BLOCK_SPLITTING
        || (*splits).idx >= ZSTD_MAX_NB_BLOCK_SPLITS
    {
        return;
    }
    ZSTD_deriveSeqStoreChunk(fullSeqStoreChunk, origSeqStore, startIdx, endIdx);
    ZSTD_deriveSeqStoreChunk(firstHalfSeqStore, origSeqStore, startIdx, midIdx);
    ZSTD_deriveSeqStoreChunk(secondHalfSeqStore, origSeqStore, midIdx, endIdx);
    estimatedOriginalSize =
        ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(fullSeqStoreChunk, zc);
    estimatedFirstHalfSize =
        ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(firstHalfSeqStore, zc);
    estimatedSecondHalfSize =
        ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(secondHalfSeqStore, zc);
    if ERR_isError(estimatedOriginalSize) != 0
        || ERR_isError(estimatedFirstHalfSize) != 0
        || ERR_isError(estimatedSecondHalfSize) != 0
    {
        return;
    }
    if estimatedFirstHalfSize.wrapping_add(estimatedSecondHalfSize) < estimatedOriginalSize {
        ZSTD_deriveBlockSplitsHelper(splits, startIdx, midIdx, zc, origSeqStore);
        *(*splits).splitLocations.wrapping_add((*splits).idx) = midIdx as U32;
        (*splits).idx += 1;
        ZSTD_deriveBlockSplitsHelper(splits, midIdx, endIdx, zc, origSeqStore);
    }
}

pub unsafe fn ZSTD_deriveBlockSplits(
    zc: *mut ZSTD_CCtx,
    partitions: *mut U32,
    nbSeq: U32,
) -> usize {
    let mut splits = seqStoreSplits {
        splitLocations: partitions,
        idx: 0,
    };
    if nbSeq <= 4 {
        /* Refuse to try and split anything with less than 4 sequences */
        return 0;
    }
    ZSTD_deriveBlockSplitsHelper(&mut splits, 0, nbSeq as usize, zc, addr_of!((*zc).seqStore));
    *splits.splitLocations.wrapping_add(splits.idx) = nbSeq;
    splits.idx
}

pub unsafe fn ZSTD_compressBlock_splitBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    blockSize: usize,
    lastBlock: U32,
    nbSeq: U32,
) -> usize {
    let mut dstCapacity = dstCapacity;
    let mut cSize: usize = 0;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut i: usize = 0;
    let mut srcBytesTotal: usize = 0;
    let partitions: *mut U32 = (*zc).blockSplitCtx.partitions.as_mut_ptr();
    let nextSeqStore: *mut SeqStore_t = addr_of_mut!((*zc).blockSplitCtx.nextSeqStore);
    let currSeqStore: *mut SeqStore_t = addr_of_mut!((*zc).blockSplitCtx.currSeqStore);
    let numSplits: usize = ZSTD_deriveBlockSplits(zc, partitions, nbSeq);

    let mut dRep = Repcodes_t::default();
    let mut cRep = Repcodes_t::default();
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
        let cSizeSingleBlock = ZSTD_compressSeqStore_singleBlock(
            zc,
            addr_of!((*zc).seqStore),
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

    ZSTD_deriveSeqStoreChunk(
        currSeqStore,
        addr_of!((*zc).seqStore),
        0,
        *partitions.offset(0) as usize,
    );
    i = 0;
    while i <= numSplits {
        let cSizeChunk: usize;
        let lastPartition: U32 = (i == numSplits) as U32;
        let mut lastBlockEntireSrc: U32 = 0;

        let mut srcBytes: usize = ZSTD_countSeqStoreLiteralsBytes(currSeqStore)
            .wrapping_add(ZSTD_countSeqStoreMatchBytes(currSeqStore));
        srcBytesTotal = srcBytesTotal.wrapping_add(srcBytes);
        if lastPartition != 0 {
            /* This is the final partition, need to account for possible last literals */
            srcBytes = srcBytes.wrapping_add(blockSize.wrapping_sub(srcBytesTotal));
            lastBlockEntireSrc = lastBlock;
        } else {
            ZSTD_deriveSeqStoreChunk(
                nextSeqStore,
                addr_of!((*zc).seqStore),
                *partitions.wrapping_add(i) as usize,
                *partitions.wrapping_add(i + 1) as usize,
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
        dstCapacity = dstCapacity.wrapping_sub(cSizeChunk);
        cSize = cSize.wrapping_add(cSizeChunk);
        *currSeqStore = *nextSeqStore;
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
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let nbSeq: U32;
    let mut cSize: usize;

    {
        let bss = ZSTD_buildSeqStore(zc, src, srcSize);
        if ERR_isError(bss) != 0 {
            return bss;
        }
        if bss == ZSTDbss_noCompress as usize {
            if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }
            if (*zc).seqCollector.collectSequences != 0 {
                return ERROR(ZSTD_error_sequenceProducer_failed);
            }
            cSize = ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock);
            if ERR_isError(cSize) != 0 {
                return cSize;
            }
            return cSize;
        }
        nbSeq = (((*zc).seqStore.sequences as usize)
            .wrapping_sub((*zc).seqStore.sequencesStart as usize)
            / core::mem::size_of::<SeqDef>()) as U32;
    }

    cSize = ZSTD_compressBlock_splitBlock_internal(
        zc, dst, dstCapacity, src, srcSize, lastBlock, nbSeq,
    );
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    cSize
}

pub unsafe fn ZSTD_compressBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    frame: U32,
) -> usize {
    let rleMaxLength: U32 = 25;
    let mut cSize: usize;
    let ip: *const BYTE = src as *const BYTE;
    let op: *mut BYTE = dst as *mut BYTE;

    'out: {
        {
            let bss = ZSTD_buildSeqStore(zc, src, srcSize);
            if ERR_isError(bss) != 0 {
                return bss;
            }
            if bss == ZSTDbss_noCompress as usize {
                if (*zc).seqCollector.collectSequences != 0 {
                    return ERROR(ZSTD_error_sequenceProducer_failed);
                }
                cSize = 0;
                break 'out;
            }
        }

        if (*zc).seqCollector.collectSequences != 0 {
            {
                let err_code = ZSTD_copyBlockSequences(
                    addr_of_mut!((*zc).seqCollector),
                    crate::zstd_compress::ZSTD_getSeqStore(zc),
                    (*(*zc).blockState.prevCBlock).rep.as_ptr(),
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            ZSTD_blockState_confirmRepcodesAndEntropyTables(addr_of_mut!((*zc).blockState));
            return 0;
        }

        /* encode sequences and literals */
        cSize = ZSTD_entropyCompressSeqStore(
            addr_of!((*zc).seqStore),
            addr_of!((*(*zc).blockState.prevCBlock).entropy),
            addr_of_mut!((*(*zc).blockState.nextCBlock).entropy),
            addr_of!((*zc).appliedParams),
            dst,
            dstCapacity,
            srcSize,
            (*zc).tmpWorkspace,
            (*zc).tmpWkspSize,
            (*zc).bmi2,
        );

        if frame != 0
            && (*zc).isFirstBlock == 0
            && cSize < rleMaxLength as usize
            && ZSTD_isRLE(ip, srcSize) != 0
        {
            cSize = 1;
            *op.offset(0) = *ip.offset(0);
        }
    }

    /* out: */
    if ERR_isError(cSize) == 0 && cSize > 1 {
        ZSTD_blockState_confirmRepcodesAndEntropyTables(addr_of_mut!((*zc).blockState));
    }
    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

pub unsafe fn ZSTD_compressBlock_targetCBlockSize_body(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    bss: usize,
    lastBlock: U32,
) -> usize {
    if bss == ZSTDbss_compress as usize {
        if (*zc).isFirstBlock == 0
            && ZSTD_maybeRLE(addr_of!((*zc).seqStore)) != 0
            && ZSTD_isRLE(src as *const BYTE, srcSize) != 0
        {
            return ZSTD_rleCompressBlock(
                dst,
                dstCapacity,
                *(src as *const BYTE),
                srcSize,
                lastBlock,
            );
        }
        {
            let cSize = crate::zstd_compress_superblock::ZSTD_compressSuperBlock(
                zc,
                dst,
                dstCapacity,
                src,
                srcSize,
                lastBlock,
            );
            if cSize != ERROR(ZSTD_error_dstSize_tooSmall) {
                let maxCSize = srcSize.wrapping_sub(ZSTD_minGain(
                    srcSize,
                    (*zc).appliedParams.cParams.strategy,
                ));
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
                if cSize != 0 && cSize < maxCSize.wrapping_add(ZSTD_blockHeaderSize) {
                    ZSTD_blockState_confirmRepcodesAndEntropyTables(addr_of_mut!(
                        (*zc).blockState
                    ));
                    return cSize;
                }
            }
        }
    } /* if (bss == ZSTDbss_compress) */

    ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock)
}

pub unsafe fn ZSTD_compressBlock_targetCBlockSize(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let mut cSize: usize = 0;
    let bss = ZSTD_buildSeqStore(zc, src, srcSize);
    if ERR_isError(bss) != 0 {
        return bss;
    }

    cSize = ZSTD_compressBlock_targetCBlockSize_body(
        zc, dst, dstCapacity, src, srcSize, bss, lastBlock,
    );
    if ERR_isError(cSize) != 0 {
        return cSize;
    }

    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

pub unsafe fn ZSTD_overflowCorrectIfNeeded(
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    ip: *const c_void,
    iend: *const c_void,
) {
    let cycleLog: U32 = crate::zstd_compress::ZSTD_cycleLog(
        (*params).cParams.chainLog,
        (*params).cParams.strategy,
    );
    let maxDist: U32 = (1u32) << (*params).cParams.windowLog;
    if ZSTD_window_needOverflowCorrection(
        (*ms).window,
        cycleLog,
        maxDist,
        (*ms).loadedDictEnd,
        ip,
        iend,
    ) != 0
    {
        let correction: U32 =
            ZSTD_window_correctOverflow(addr_of_mut!((*ms).window), cycleLog, maxDist, ip);
        ZSTD_cwksp_mark_tables_dirty(ws);
        crate::zstd_compress::ZSTD_reduceIndex(ms, params, correction);
        ZSTD_cwksp_mark_tables_clean(ws);
        if (*ms).nextToUpdate < correction {
            (*ms).nextToUpdate = 0;
        } else {
            (*ms).nextToUpdate = (*ms).nextToUpdate.wrapping_sub(correction);
        }
        /* invalidate dictionaries on overflow correction */
        (*ms).loadedDictEnd = 0;
        (*ms).dictMatchState = core::ptr::null();
    }
}

static splitLevels: [c_int; 10] = [0, 0, 1, 2, 2, 3, 3, 4, 4, 4];

pub unsafe fn ZSTD_optimalBlockSize(
    cctx: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: usize,
    blockSizeMax: usize,
    splitLevel: c_int,
    strat: ZSTD_strategy,
    savings: i64,
) -> usize {
    let mut splitLevel = splitLevel;
    /* note: conservatively only split full blocks (128 KB) currently. */
    if srcSize < 128 * (1 << 10) || blockSizeMax < 128 * (1 << 10) {
        return MIN(srcSize, blockSizeMax);
    }
    /* do not split incompressible data though */
    if savings < 3 {
        return 128 * (1 << 10);
    }
    if splitLevel == 1 {
        return 128 * (1 << 10);
    }
    if splitLevel == 0 {
        splitLevel = splitLevels[strat as usize];
    } else {
        splitLevel -= 2;
    }
    crate::zstd_presplit::ZSTD_splitBlock(
        src,
        blockSizeMax,
        splitLevel,
        (*cctx).tmpWorkspace,
        (*cctx).tmpWkspSize,
    )
}

pub unsafe fn ZSTD_compress_frameChunk(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastFrameChunk: U32,
) -> usize {
    let mut dstCapacity = dstCapacity;
    let blockSizeMax: usize = (*cctx).blockSizeMax;
    let mut remaining: usize = srcSize;
    let mut ip: *const BYTE = src as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let maxDist: U32 = (1u32) << (*cctx).appliedParams.cParams.windowLog;
    let mut savings: i64 =
        ((*cctx).consumedSrcSize as i64).wrapping_sub((*cctx).producedCSize as i64);

    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        crate::xxhash::ZSTD_XXH64_update(addr_of_mut!((*cctx).xxhState), src, srcSize);
    }

    while remaining != 0 {
        let ms: *mut ZSTD_MatchState_t = addr_of_mut!((*cctx).blockState.matchState);
        let blockSize: usize = ZSTD_optimalBlockSize(
            cctx,
            ip as *const c_void,
            remaining,
            blockSizeMax,
            (*cctx).appliedParams.preBlockSplitter_level,
            (*cctx).appliedParams.cParams.strategy,
            savings,
        );
        let lastBlock: U32 = lastFrameChunk & (blockSize == remaining) as U32;

        if dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        ZSTD_overflowCorrectIfNeeded(
            ms,
            addr_of_mut!((*cctx).workspace),
            addr_of!((*cctx).appliedParams),
            ip as *const c_void,
            ip.wrapping_add(blockSize) as *const c_void,
        );
        ZSTD_checkDictValidity(
            addr_of!((*ms).window),
            ip.wrapping_add(blockSize) as *const c_void,
            maxDist,
            addr_of_mut!((*ms).loadedDictEnd),
            addr_of_mut!((*ms).dictMatchState),
        );
        ZSTD_window_enforceMaxDist(
            addr_of_mut!((*ms).window),
            ip as *const c_void,
            maxDist,
            addr_of_mut!((*ms).loadedDictEnd),
            addr_of_mut!((*ms).dictMatchState),
        );

        /* Ensure hash/chain table insertion resumes no sooner than lowlimit */
        if (*ms).nextToUpdate < (*ms).window.lowLimit {
            (*ms).nextToUpdate = (*ms).window.lowLimit;
        }

        {
            let mut cSize: usize;
            if ZSTD_useTargetCBlockSize(addr_of!((*cctx).appliedParams)) != 0 {
                cSize = ZSTD_compressBlock_targetCBlockSize(
                    cctx,
                    op as *mut c_void,
                    dstCapacity,
                    ip as *const c_void,
                    blockSize,
                    lastBlock,
                );
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
            } else if ZSTD_blockSplitterEnabled(addr_of_mut!((*cctx).appliedParams)) != 0 {
                cSize = ZSTD_compressBlock_splitBlock(
                    cctx,
                    op as *mut c_void,
                    dstCapacity,
                    ip as *const c_void,
                    blockSize,
                    lastBlock,
                );
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
            } else {
                cSize = ZSTD_compressBlock_internal(
                    cctx,
                    op.wrapping_add(ZSTD_blockHeaderSize) as *mut c_void,
                    dstCapacity.wrapping_sub(ZSTD_blockHeaderSize),
                    ip as *const c_void,
                    blockSize,
                    1, /* frame */
                );
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }

                if cSize == 0 {
                    /* block is not compressible */
                    cSize = ZSTD_noCompressBlock(
                        op as *mut c_void,
                        dstCapacity,
                        ip as *const c_void,
                        blockSize,
                        lastBlock,
                    );
                    if ERR_isError(cSize) != 0 {
                        return cSize;
                    }
                } else {
                    let cBlockHeader: U32 = if cSize == 1 {
                        lastBlock
                            .wrapping_add((bt_rle as U32) << 1)
                            .wrapping_add((blockSize << 3) as U32)
                    } else {
                        lastBlock
                            .wrapping_add((bt_compressed as U32) << 1)
                            .wrapping_add((cSize << 3) as U32)
                    };
                    MEM_writeLE24(op, cBlockHeader);
                    cSize = cSize.wrapping_add(ZSTD_blockHeaderSize);
                }
            } /* if (ZSTD_useTargetCBlockSize(&cctx->appliedParams)) */

            savings = savings
                .wrapping_add(blockSize as i64)
                .wrapping_sub(cSize as i64);

            ip = ip.wrapping_add(blockSize);
            remaining = remaining.wrapping_sub(blockSize);
            op = op.wrapping_add(cSize);
            dstCapacity = dstCapacity.wrapping_sub(cSize);
            (*cctx).isFirstBlock = 0;
        }
    }

    if lastFrameChunk != 0 && (op > ostart) {
        (*cctx).stage = ZSTDcs_ending;
    }
    (op as usize).wrapping_sub(ostart as usize)
}
