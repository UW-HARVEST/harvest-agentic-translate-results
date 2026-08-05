//! Faithful translation of compress/zstd_compress_superblock.c
//!
//! Build config: DYNAMIC_BMI2=0, single-threaded, LE 64-bit, byte-identical.
//! FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION is NOT defined (guarded blocks are
//! compiled in). NDEBUG (asserts are no-ops).
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens,
    unused_variables
)]

use core::ffi::{c_int, c_uint, c_void};
use core::ptr;

use crate::common::bitstream::stream_accumulator_min;
use crate::common::error::{code, err_is_error, error};
use crate::common::fse::FSE_CTable;
use crate::common::huf_common::HUF_flags_bmi2;
use crate::common::mem::{mem_write_le16, mem_write_le24, mem_write_le32, U16, U32};
use crate::common::zstd_internal::{
    bt_compressed, set_basic, set_compressed, set_repeat, set_rle, DefaultMaxOff, LL_bits,
    LL_defaultNorm, LL_defaultNormLog, MaxLL, MaxML, MaxOff, MINMATCH, ML_bits, ML_defaultNorm,
    ML_defaultNormLog, OF_defaultNorm, OF_defaultNormLog, LONGNBSEQ, ZSTD_blockHeaderSize,
};

use crate::compress::hist::{HIST_countFast_wksp, HIST_count_wksp};
use crate::compress::huf_compress::{
    HUF_compress1X_usingCTable, HUF_compress4X_usingCTable, HUF_estimateCompressedSize,
};
use crate::compress::zstd_compress_internal::{
    Repcodes_t, SeqDef, SeqStore_t, SymbolEncodingType_e, ZSTD_CCtx, ZSTD_CCtx_params,
    ZSTD_compressedBlockState_t, ZSTD_entropyCTables_t, ZSTD_entropyCTablesMetadata_t,
    ZSTD_fseCTables_t, ZSTD_fseCTablesMetadata_t, ZSTD_getSequenceLength, ZSTD_hufCTables_t,
    ZSTD_hufCTablesMetadata_t, ZSTD_noCompressBlock, ZSTD_updateRep, HUF_CElt,
};

type BYTE = u8;

/* from zstd.h */
const ZSTD_TARGETCBLOCKSIZE_MIN: usize = 1340;

const BYTESCALE: usize = 256;

// ---------------------------------------------------------------------------
// extern "C" sibling symbols (declared in other compress modules / zstd_compress.c)
// ---------------------------------------------------------------------------
extern "C" {
    fn ZSTD_encodeSequences(
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

    fn ZSTD_fseBitCost(ctable: *const FSE_CTable, count: *const u32, max: u32) -> usize;

    fn ZSTD_crossEntropyCost(
        norm: *const i16,
        accuracyLog: u32,
        count: *const u32,
        max: u32,
    ) -> usize;

    fn ZSTD_buildBlockEntropyStats(
        seqStorePtr: *const SeqStore_t,
        prevEntropy: *const ZSTD_entropyCTables_t,
        nextEntropy: *mut ZSTD_entropyCTables_t,
        cctxParams: *const ZSTD_CCtx_params,
        entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
        workspace: *mut c_void,
        wkspSize: usize,
    ) -> usize;

    fn ZSTD_noCompressLiterals(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;

    fn ZSTD_compressRleLiteralsBlock(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
}

/// ZSTD_compressSubBlock_literal() :
///  Compresses literals section for a sub-block.
///  @return : compressed size of literals section of a sub-block
///            Or 0 if unable to compress.
///            Or error code
unsafe fn ZSTD_compressSubBlock_literal(
    hufTable: *const HUF_CElt,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    literals: *const BYTE,
    litSize: usize,
    dst: *mut c_void,
    dstSize: usize,
    bmi2: c_int,
    writeEntropy: c_int,
    entropyWritten: *mut c_int,
) -> usize {
    let header: usize = if writeEntropy != 0 { 200 } else { 0 };
    let lhSize: usize =
        3 + (litSize >= (1024 - header)) as usize + (litSize >= (16 * 1024 - header)) as usize;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstSize);
    let mut op = ostart.add(lhSize);
    let singleStream: U32 = (lhSize == 3) as U32;
    let hType: SymbolEncodingType_e = if writeEntropy != 0 {
        (*hufMetadata).hType
    } else {
        set_repeat
    };
    let mut cLitSize: usize = 0;

    *entropyWritten = 0;
    if litSize == 0 || (*hufMetadata).hType == set_basic {
        return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
    } else if (*hufMetadata).hType == set_rle {
        return ZSTD_compressRleLiteralsBlock(dst, dstSize, literals as *const c_void, litSize);
    }

    if writeEntropy != 0 && (*hufMetadata).hType == set_compressed {
        ptr::copy_nonoverlapping(
            (*hufMetadata).hufDesBuffer.as_ptr(),
            op,
            (*hufMetadata).hufDesSize,
        );
        op = op.add((*hufMetadata).hufDesSize);
        cLitSize += (*hufMetadata).hufDesSize;
    }

    {
        let flags: c_int = if bmi2 != 0 { HUF_flags_bmi2 } else { 0 };
        let cSize: usize = if singleStream != 0 {
            HUF_compress1X_usingCTable(
                op as *mut c_void,
                oend.offset_from(op) as usize,
                literals as *const c_void,
                litSize,
                hufTable,
                flags,
            )
        } else {
            HUF_compress4X_usingCTable(
                op as *mut c_void,
                oend.offset_from(op) as usize,
                literals as *const c_void,
                litSize,
                hufTable,
                flags,
            )
        };
        op = op.add(cSize);
        cLitSize += cSize;
        if cSize == 0 || err_is_error(cSize) != 0 {
            return 0;
        }
        /* If we expand and we aren't writing a header then emit uncompressed */
        if writeEntropy == 0 && cLitSize >= litSize {
            return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
        }
        /* If we are writing headers then allow expansion that doesn't change our header size. */
        if lhSize < (3 + (cLitSize >= 1024) as usize + (cLitSize >= 16 * 1024) as usize) {
            return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
        }
    }

    /* Build header */
    match lhSize {
        3 => {
            /* 2 - 2 - 10 - 10 */
            let lhc: U32 = hType
                + (((singleStream == 0) as U32) << 2)
                + ((litSize as U32) << 4)
                + ((cLitSize as U32) << 14);
            mem_write_le24(ostart as *mut c_void, lhc);
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            let lhc: U32 =
                hType + (2 << 2) + ((litSize as U32) << 4) + ((cLitSize as U32) << 18);
            mem_write_le32(ostart as *mut c_void, lhc);
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            let lhc: U32 =
                hType + (3 << 2) + ((litSize as U32) << 4) + ((cLitSize as U32) << 22);
            mem_write_le32(ostart as *mut c_void, lhc);
            *ostart.add(4) = (cLitSize >> 10) as BYTE;
        }
        _ => {
            /* not possible : lhSize is {3,4,5} */
        }
    }
    *entropyWritten = 1;
    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_seqDecompressedSize(
    seqStore: *const SeqStore_t,
    sequences: *const SeqDef,
    nbSeqs: usize,
    litSize: usize,
    lastSubBlock: c_int,
) -> usize {
    let mut matchLengthSum: usize = 0;
    let mut litLengthSum: usize = 0;
    let mut n: usize = 0;
    while n < nbSeqs {
        let seqLen = ZSTD_getSequenceLength(seqStore, sequences.add(n));
        litLengthSum += seqLen.litLength as usize;
        matchLengthSum += seqLen.matchLength as usize;
        n += 1;
    }
    let _ = litLengthSum;
    matchLengthSum + litSize
}

/// ZSTD_compressSubBlock_sequences() :
///  Compresses sequences section for a sub-block.
///  @return : compressed size of sequences section of a sub-block
///            Or 0 if it is unable to compress
///            Or error code.
unsafe fn ZSTD_compressSubBlock_sequences(
    fseTables: *const ZSTD_fseCTables_t,
    fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    sequences: *const SeqDef,
    nbSeq: usize,
    llCode: *const BYTE,
    mlCode: *const BYTE,
    ofCode: *const BYTE,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: usize,
    bmi2: c_int,
    writeEntropy: c_int,
    entropyWritten: *mut c_int,
) -> usize {
    let longOffsets: c_int =
        ((*cctxParams).cParams.windowLog > stream_accumulator_min()) as c_int;
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstCapacity);
    let mut op = ostart;
    let seqHead: *mut BYTE;

    *entropyWritten = 0;
    /* Sequences Header */
    if (oend.offset_from(op) as isize) < (3 /*max nbSeq Size*/ + 1 /*seqHead*/) {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if nbSeq < 128 {
        *op = nbSeq as BYTE;
        op = op.add(1);
    } else if nbSeq < LONGNBSEQ as usize {
        *op.add(0) = ((nbSeq >> 8) + 0x80) as BYTE;
        *op.add(1) = nbSeq as BYTE;
        op = op.add(2);
    } else {
        *op.add(0) = 0xFF;
        mem_write_le16(op.add(1) as *mut c_void, (nbSeq - LONGNBSEQ as usize) as U16);
        op = op.add(3);
    }
    if nbSeq == 0 {
        return op.offset_from(ostart) as usize;
    }

    /* seqHead : flags for FSE encoding type */
    seqHead = op;
    op = op.add(1);

    if writeEntropy != 0 {
        let LLtype: U32 = (*fseMetadata).llType;
        let Offtype: U32 = (*fseMetadata).ofType;
        let MLtype: U32 = (*fseMetadata).mlType;
        *seqHead = ((LLtype << 6) + (Offtype << 4) + (MLtype << 2)) as BYTE;
        ptr::copy_nonoverlapping(
            (*fseMetadata).fseTablesBuffer.as_ptr(),
            op,
            (*fseMetadata).fseTablesSize,
        );
        op = op.add((*fseMetadata).fseTablesSize);
    } else {
        let repeat: U32 = set_repeat;
        *seqHead = ((repeat << 6) + (repeat << 4) + (repeat << 2)) as BYTE;
    }

    {
        let bitstreamSize: usize = ZSTD_encodeSequences(
            op as *mut c_void,
            oend.offset_from(op) as usize,
            (*fseTables).matchlengthCTable.as_ptr(),
            mlCode,
            (*fseTables).offcodeCTable.as_ptr(),
            ofCode,
            (*fseTables).litlengthCTable.as_ptr(),
            llCode,
            sequences,
            nbSeq,
            longOffsets,
            bmi2,
        );
        if err_is_error(bitstreamSize) != 0 {
            return bitstreamSize;
        }
        op = op.add(bitstreamSize);
        /* zstd versions <= 1.3.4 mistakenly report corruption when
         * FSE_readNCount() receives a buffer < 4 bytes. */
        if writeEntropy != 0
            && (*fseMetadata).lastCountSize != 0
            && (*fseMetadata).lastCountSize + bitstreamSize < 4
        {
            return 0;
        }
    }

    /* zstd versions <= 1.4.0 mistakenly report error when
     * sequences section body size is less than 3 bytes. */
    if (op.offset_from(seqHead) as isize) < 4 {
        return 0;
    }

    *entropyWritten = 1;
    op.offset_from(ostart) as usize
}

/// ZSTD_compressSubBlock() :
///  Compresses a single sub-block.
///  @return : compressed size of the sub-block
///            Or 0 if it failed to compress.
unsafe fn ZSTD_compressSubBlock(
    entropy: *const ZSTD_entropyCTables_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    sequences: *const SeqDef,
    nbSeq: usize,
    literals: *const BYTE,
    litSize: usize,
    llCode: *const BYTE,
    mlCode: *const BYTE,
    ofCode: *const BYTE,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: usize,
    bmi2: c_int,
    writeLitEntropy: c_int,
    writeSeqEntropy: c_int,
    litEntropyWritten: *mut c_int,
    seqEntropyWritten: *mut c_int,
    lastBlock: U32,
) -> usize {
    let ostart = dst as *mut BYTE;
    let oend = ostart.add(dstCapacity);
    let mut op = ostart.add(ZSTD_blockHeaderSize);
    {
        let cLitSize: usize = ZSTD_compressSubBlock_literal(
            (*entropy).huf.CTable.as_ptr() as *const HUF_CElt,
            &(*entropyMetadata).hufMetadata,
            literals,
            litSize,
            op as *mut c_void,
            oend.offset_from(op) as usize,
            bmi2,
            writeLitEntropy,
            litEntropyWritten,
        );
        if err_is_error(cLitSize) != 0 {
            return cLitSize;
        }
        if cLitSize == 0 {
            return 0;
        }
        op = op.add(cLitSize);
    }
    {
        let cSeqSize: usize = ZSTD_compressSubBlock_sequences(
            &(*entropy).fse,
            &(*entropyMetadata).fseMetadata,
            sequences,
            nbSeq,
            llCode,
            mlCode,
            ofCode,
            cctxParams,
            op as *mut c_void,
            oend.offset_from(op) as usize,
            bmi2,
            writeSeqEntropy,
            seqEntropyWritten,
        );
        if err_is_error(cSeqSize) != 0 {
            return cSeqSize;
        }
        if cSeqSize == 0 {
            return 0;
        }
        op = op.add(cSeqSize);
    }
    /* Write block header */
    {
        let cSize: usize = (op.offset_from(ostart) as usize) - ZSTD_blockHeaderSize;
        let cBlockHeader24: U32 =
            lastBlock + ((bt_compressed) << 1) + ((cSize << 3) as U32);
        mem_write_le24(ostart as *mut c_void, cBlockHeader24);
    }
    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_estimateSubBlockSize_literal(
    literals: *const BYTE,
    litSize: usize,
    huf: *const ZSTD_hufCTables_t,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeEntropy: c_int,
) -> usize {
    let countWksp = workspace as *mut u32;
    let mut maxSymbolValue: u32 = 255;
    let literalSectionHeaderSize: usize = 3; /* Use hard coded size of 3 bytes */

    if (*hufMetadata).hType == set_basic {
        return litSize;
    } else if (*hufMetadata).hType == set_rle {
        return 1;
    } else if (*hufMetadata).hType == set_compressed || (*hufMetadata).hType == set_repeat {
        let largest: usize = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            literals as *const c_void,
            litSize,
            workspace,
            wkspSize,
        );
        if err_is_error(largest) != 0 {
            return litSize;
        }
        {
            let mut cLitSizeEstimate: usize = HUF_estimateCompressedSize(
                (*huf).CTable.as_ptr() as *const HUF_CElt,
                countWksp,
                maxSymbolValue,
            );
            if writeEntropy != 0 {
                cLitSizeEstimate += (*hufMetadata).hufDesSize;
            }
            return cLitSizeEstimate + literalSectionHeaderSize;
        }
    }
    /* impossible */
    0
}

unsafe fn ZSTD_estimateSubBlockSize_symbolType(
    type_: SymbolEncodingType_e,
    codeTable: *const BYTE,
    maxCode: u32,
    nbSeq: usize,
    fseCTable: *const FSE_CTable,
    additionalBits: *const u8,
    defaultNorm: *const i16,
    defaultNormLog: U32,
    defaultMax: U32,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let countWksp = workspace as *mut u32;
    let mut ctp = codeTable;
    let ctStart = ctp;
    let ctEnd = ctStart.add(nbSeq);
    let mut cSymbolTypeSizeEstimateInBits: usize = 0;
    let mut max: u32 = maxCode;

    HIST_countFast_wksp(
        countWksp,
        &mut max,
        codeTable as *const c_void,
        nbSeq,
        workspace,
        wkspSize,
    ); /* can't fail */
    if type_ == set_basic {
        /* We selected this encoding type, so it must be valid. */
        cSymbolTypeSizeEstimateInBits = if max <= defaultMax {
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, countWksp, max)
        } else {
            error(code::GENERIC)
        };
    } else if type_ == set_rle {
        cSymbolTypeSizeEstimateInBits = 0;
    } else if type_ == set_compressed || type_ == set_repeat {
        cSymbolTypeSizeEstimateInBits = ZSTD_fseBitCost(fseCTable, countWksp, max);
    }
    if err_is_error(cSymbolTypeSizeEstimateInBits) != 0 {
        return nbSeq * 10;
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits += *additionalBits.add(*ctp as usize) as usize;
        } else {
            cSymbolTypeSizeEstimateInBits += *ctp as usize; /* for offset, offset code is also the number of additional bits */
        }
        ctp = ctp.add(1);
    }
    cSymbolTypeSizeEstimateInBits / 8
}

unsafe fn ZSTD_estimateSubBlockSize_sequences(
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
    let sequencesSectionHeaderSize: usize = 3; /* Use hard coded size of 3 bytes */
    let mut cSeqSizeEstimate: usize = 0;
    if nbSeq == 0 {
        return sequencesSectionHeaderSize;
    }
    cSeqSizeEstimate += ZSTD_estimateSubBlockSize_symbolType(
        (*fseMetadata).ofType,
        ofCodeTable,
        MaxOff,
        nbSeq,
        (*fseTables).offcodeCTable.as_ptr(),
        ptr::null(),
        OF_defaultNorm.as_ptr(),
        OF_defaultNormLog,
        DefaultMaxOff,
        workspace,
        wkspSize,
    );
    cSeqSizeEstimate += ZSTD_estimateSubBlockSize_symbolType(
        (*fseMetadata).llType,
        llCodeTable,
        MaxLL,
        nbSeq,
        (*fseTables).litlengthCTable.as_ptr(),
        LL_bits.as_ptr(),
        LL_defaultNorm.as_ptr(),
        LL_defaultNormLog,
        MaxLL,
        workspace,
        wkspSize,
    );
    cSeqSizeEstimate += ZSTD_estimateSubBlockSize_symbolType(
        (*fseMetadata).mlType,
        mlCodeTable,
        MaxML,
        nbSeq,
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

#[derive(Clone, Copy)]
struct EstimatedBlockSize {
    estLitSize: usize,
    estBlockSize: usize,
}

unsafe fn ZSTD_estimateSubBlockSize(
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
) -> EstimatedBlockSize {
    let mut ebs = EstimatedBlockSize {
        estLitSize: 0,
        estBlockSize: 0,
    };
    ebs.estLitSize = ZSTD_estimateSubBlockSize_literal(
        literals,
        litSize,
        &(*entropy).huf,
        &(*entropyMetadata).hufMetadata,
        workspace,
        wkspSize,
        writeLitEntropy,
    );
    ebs.estBlockSize = ZSTD_estimateSubBlockSize_sequences(
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
    ebs.estBlockSize += ebs.estLitSize + ZSTD_blockHeaderSize;
    ebs
}

unsafe fn ZSTD_needSequenceEntropyTables(fseMetadata: *const ZSTD_fseCTablesMetadata_t) -> c_int {
    if (*fseMetadata).llType == set_compressed || (*fseMetadata).llType == set_rle {
        return 1;
    }
    if (*fseMetadata).mlType == set_compressed || (*fseMetadata).mlType == set_rle {
        return 1;
    }
    if (*fseMetadata).ofType == set_compressed || (*fseMetadata).ofType == set_rle {
        return 1;
    }
    0
}

unsafe fn countLiterals(seqStore: *const SeqStore_t, sp: *const SeqDef, seqCount: usize) -> usize {
    let mut n: usize = 0;
    let mut total: usize = 0;
    while n < seqCount {
        total += ZSTD_getSequenceLength(seqStore, sp.add(n)).litLength as usize;
        n += 1;
    }
    total
}

unsafe fn sizeBlockSequences(
    sp: *const SeqDef,
    nbSeqs: usize,
    targetBudget: usize,
    avgLitCost: usize,
    avgSeqCost: usize,
    firstSubBlock: c_int,
) -> usize {
    let mut n: usize;
    let mut budget: usize = 0;
    let mut inSize: usize = 0;
    /* entropy headers */
    let headerSize: usize = (firstSubBlock as usize) * 120 * BYTESCALE; /* generous estimate */
    budget += headerSize;

    /* first sequence => at least one sequence*/
    budget += (*sp.add(0)).litLength as usize * avgLitCost + avgSeqCost;
    if budget > targetBudget {
        return 1;
    }
    inSize = (*sp.add(0)).litLength as usize + ((*sp.add(0)).mlBase as usize + MINMATCH as usize);

    /* loop over sequences */
    n = 1;
    while n < nbSeqs {
        let currentCost: usize = (*sp.add(n)).litLength as usize * avgLitCost + avgSeqCost;
        budget += currentCost;
        inSize += (*sp.add(n)).litLength as usize + ((*sp.add(n)).mlBase as usize + MINMATCH as usize);
        /* stop when sub-block budget is reached */
        if (budget > targetBudget)
            /* though continue to expand until the sub-block is deemed compressible */
            && (budget < inSize * BYTESCALE)
        {
            break;
        }
        n += 1;
    }

    n
}

/// ZSTD_compressSubBlock_multi() :
///  Breaks super-block into multiple sub-blocks and compresses them.
///  @return : compressed size of the super block (which features multiple ZSTD blocks)
///            or 0 if it failed to compress.
unsafe fn ZSTD_compressSubBlock_multi(
    seqStorePtr: *const SeqStore_t,
    prevCBlock: *const ZSTD_compressedBlockState_t,
    nextCBlock: *mut ZSTD_compressedBlockState_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    bmi2: c_int,
    lastBlock: U32,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let sstart: *const SeqDef = (*seqStorePtr).sequencesStart;
    let send: *const SeqDef = (*seqStorePtr).sequences;
    let mut sp: *const SeqDef = sstart; /* tracks progresses within seqStorePtr->sequences */
    let nbSeqs: usize = send.offset_from(sstart) as usize;
    let lstart: *const BYTE = (*seqStorePtr).litStart;
    let lend: *const BYTE = (*seqStorePtr).lit;
    let mut lp: *const BYTE = lstart;
    let nbLiterals: usize = lend.offset_from(lstart) as usize;
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let mut llCodePtr: *const BYTE = (*seqStorePtr).llCode;
    let mut mlCodePtr: *const BYTE = (*seqStorePtr).mlCode;
    let mut ofCodePtr: *const BYTE = (*seqStorePtr).ofCode;
    let minTarget: usize = ZSTD_TARGETCBLOCKSIZE_MIN; /* enforce minimum size */
    let targetCBlockSize: usize = if minTarget > (*cctxParams).targetCBlockSize {
        minTarget
    } else {
        (*cctxParams).targetCBlockSize
    };
    let mut writeLitEntropy: c_int =
        ((*entropyMetadata).hufMetadata.hType == set_compressed) as c_int;
    let mut writeSeqEntropy: c_int = 1;

    /* let's start by a general estimation for the full block */
    if nbSeqs > 0 {
        let ebs: EstimatedBlockSize = ZSTD_estimateSubBlockSize(
            lp,
            nbLiterals,
            ofCodePtr,
            llCodePtr,
            mlCodePtr,
            nbSeqs,
            &(*nextCBlock).entropy,
            entropyMetadata,
            workspace,
            wkspSize,
            writeLitEntropy,
            writeSeqEntropy,
        );
        /* quick estimation */
        let avgLitCost: usize = if nbLiterals != 0 {
            (ebs.estLitSize * BYTESCALE) / nbLiterals
        } else {
            BYTESCALE
        };
        let avgSeqCost: usize = ((ebs.estBlockSize - ebs.estLitSize) * BYTESCALE) / nbSeqs;
        let nbSubBlocks: usize = {
            let v = (ebs.estBlockSize + (targetCBlockSize / 2)) / targetCBlockSize;
            if v > 1 {
                v
            } else {
                1
            }
        };
        let mut n: usize;
        let mut avgBlockBudget: usize;
        let mut blockBudgetSupp: usize = 0;
        avgBlockBudget = (ebs.estBlockSize * BYTESCALE) / nbSubBlocks;
        /* simplification: if estimates states that the full superblock doesn't compress,
         * just bail out immediately */
        if ebs.estBlockSize > srcSize {
            return 0;
        }

        /* compress and write sub-blocks */
        n = 0;
        while n < nbSubBlocks - 1 {
            /* determine nb of sequences for current sub-block */
            let seqCount: usize = sizeBlockSequences(
                sp,
                send.offset_from(sp) as usize,
                avgBlockBudget + blockBudgetSupp,
                avgLitCost,
                avgSeqCost,
                (n == 0) as c_int,
            );
            /* if reached last sequence : break to last sub-block (simplification) */
            if sp.add(seqCount) == send {
                break;
            }
            /* compress sub-block */
            {
                let mut litEntropyWritten: c_int = 0;
                let mut seqEntropyWritten: c_int = 0;
                let litSize: usize = countLiterals(seqStorePtr, sp, seqCount);
                let decompressedSize: usize =
                    ZSTD_seqDecompressedSize(seqStorePtr, sp, seqCount, litSize, 0);
                let cSize: usize = ZSTD_compressSubBlock(
                    &(*nextCBlock).entropy,
                    entropyMetadata,
                    sp,
                    seqCount,
                    lp,
                    litSize,
                    llCodePtr,
                    mlCodePtr,
                    ofCodePtr,
                    cctxParams,
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    bmi2,
                    writeLitEntropy,
                    writeSeqEntropy,
                    &mut litEntropyWritten,
                    &mut seqEntropyWritten,
                    0,
                );
                if err_is_error(cSize) != 0 {
                    return cSize;
                }

                /* check compressibility, update state components */
                if cSize > 0 && cSize < decompressedSize {
                    ip = ip.add(decompressedSize);
                    lp = lp.add(litSize);
                    op = op.add(cSize);
                    llCodePtr = llCodePtr.add(seqCount);
                    mlCodePtr = mlCodePtr.add(seqCount);
                    ofCodePtr = ofCodePtr.add(seqCount);
                    /* Entropy only needs to be written once */
                    if litEntropyWritten != 0 {
                        writeLitEntropy = 0;
                    }
                    if seqEntropyWritten != 0 {
                        writeSeqEntropy = 0;
                    }
                    sp = sp.add(seqCount);
                    blockBudgetSupp = 0;
                }
            }
            /* otherwise : do not compress yet, coalesce current sub-block with following one */
            n += 1;
        }
    } /* if (nbSeqs > 0) */

    /* write last block */
    {
        let mut litEntropyWritten: c_int = 0;
        let mut seqEntropyWritten: c_int = 0;
        let litSize: usize = lend.offset_from(lp) as usize;
        let seqCount: usize = send.offset_from(sp) as usize;
        let decompressedSize: usize =
            ZSTD_seqDecompressedSize(seqStorePtr, sp, seqCount, litSize, 1);
        let cSize: usize = ZSTD_compressSubBlock(
            &(*nextCBlock).entropy,
            entropyMetadata,
            sp,
            seqCount,
            lp,
            litSize,
            llCodePtr,
            mlCodePtr,
            ofCodePtr,
            cctxParams,
            op as *mut c_void,
            oend.offset_from(op) as usize,
            bmi2,
            writeLitEntropy,
            writeSeqEntropy,
            &mut litEntropyWritten,
            &mut seqEntropyWritten,
            lastBlock,
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }

        /* update pointers, the nb of literals borrowed from next sequence must be preserved */
        if cSize > 0 && cSize < decompressedSize {
            ip = ip.add(decompressedSize);
            lp = lp.add(litSize);
            op = op.add(cSize);
            llCodePtr = llCodePtr.add(seqCount);
            mlCodePtr = mlCodePtr.add(seqCount);
            ofCodePtr = ofCodePtr.add(seqCount);
            /* Entropy only needs to be written once */
            if litEntropyWritten != 0 {
                writeLitEntropy = 0;
            }
            if seqEntropyWritten != 0 {
                writeSeqEntropy = 0;
            }
            sp = sp.add(seqCount);
        }
    }

    if writeLitEntropy != 0 {
        ptr::copy_nonoverlapping(
            &(*prevCBlock).entropy.huf as *const ZSTD_hufCTables_t,
            &mut (*nextCBlock).entropy.huf as *mut ZSTD_hufCTables_t,
            1,
        );
    }
    if writeSeqEntropy != 0
        && ZSTD_needSequenceEntropyTables(&(*entropyMetadata).fseMetadata) != 0
    {
        /* If we haven't written our entropy tables, then we've violated our contract and
         * must emit an uncompressed block. */
        return 0;
    }

    if ip < iend {
        /* some data left : last part of the block sent uncompressed */
        let rSize: usize = iend.offset_from(ip) as usize;
        let cSize: usize = ZSTD_noCompressBlock(
            op as *mut c_void,
            oend.offset_from(op) as usize,
            ip as *const c_void,
            rSize,
            lastBlock,
        );
        if err_is_error(cSize) != 0 {
            return cSize;
        }
        op = op.add(cSize);
        /* We have to regenerate the repcodes because we've skipped some sequences */
        if sp < send {
            let mut seq: *const SeqDef;
            let mut rep: Repcodes_t = core::mem::zeroed();
            ptr::copy_nonoverlapping((*prevCBlock).rep.as_ptr(), rep.rep.as_mut_ptr(), 3);
            seq = sstart;
            while seq < sp {
                ZSTD_updateRep(
                    rep.rep.as_mut_ptr(),
                    (*seq).offBase,
                    (ZSTD_getSequenceLength(seqStorePtr, seq).litLength == 0) as U32,
                );
                seq = seq.add(1);
            }
            ptr::copy_nonoverlapping(rep.rep.as_ptr(), (*nextCBlock).rep.as_mut_ptr(), 3);
        }
    }

    op.offset_from(ostart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressSuperBlock(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: c_uint,
) -> usize {
    let mut entropyMetadata: ZSTD_entropyCTablesMetadata_t = core::mem::zeroed();

    let err = ZSTD_buildBlockEntropyStats(
        &(*zc).seqStore,
        &(*(*zc).blockState.prevCBlock).entropy,
        &mut (*(*zc).blockState.nextCBlock).entropy,
        &(*zc).appliedParams,
        &mut entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
    );
    if err_is_error(err) != 0 {
        return err;
    }

    ZSTD_compressSubBlock_multi(
        &(*zc).seqStore,
        (*zc).blockState.prevCBlock,
        (*zc).blockState.nextCBlock,
        &entropyMetadata,
        &(*zc).appliedParams,
        dst,
        dstCapacity,
        src,
        srcSize,
        (*zc).bmi2,
        lastBlock,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
    )
}

