//! Translation of `compress/zstd_compress_superblock.c`
#![allow(dead_code)]

use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_compress_sequences::{
    ZSTD_crossEntropyCost, ZSTD_encodeSequences, ZSTD_fseBitCost,
};
use crate::libc::*;
use core::ffi::{c_int, c_uint, c_void};

/* `KB` macro from zstd_internal.h : #define KB *(1 <<10) */
const KB: usize = 1 << 10;

extern "C" {
    /* compress/zstd_compress_literals.c */
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

    /* compress/huf_compress.c */
    fn HUF_compress1X_usingCTable(
        dst: *mut c_void,
        dstSize: usize,
        src: *const c_void,
        srcSize: usize,
        CTable: *const HUF_CElt,
        flags: c_int,
    ) -> usize;
    fn HUF_compress4X_usingCTable(
        dst: *mut c_void,
        dstSize: usize,
        src: *const c_void,
        srcSize: usize,
        CTable: *const HUF_CElt,
        flags: c_int,
    ) -> usize;
    fn HUF_estimateCompressedSize(
        CTable: *const HUF_CElt,
        count: *const u32,
        maxSymbolValue: u32,
    ) -> usize;

    /* compress/hist.c */
    fn HIST_count_wksp(
        count: *mut u32,
        maxSymbolValuePtr: *mut u32,
        source: *const c_void,
        sourceSize: usize,
        workSpace: *mut c_void,
        workSpaceSize: usize,
    ) -> usize;
    fn HIST_countFast_wksp(
        count: *mut u32,
        maxSymbolValuePtr: *mut u32,
        source: *const c_void,
        sourceSize: usize,
        workSpace: *mut c_void,
        workSpaceSize: usize,
    ) -> usize;

    /* compress/zstd_compress.c */
    fn ZSTD_buildBlockEntropyStats(
        seqStorePtr: *const SeqStore_t,
        prevEntropy: *const ZSTD_entropyCTables_t,
        nextEntropy: *mut ZSTD_entropyCTables_t,
        cctxParams: *const ZSTD_CCtx_params,
        entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
        workspace: *mut c_void,
        wkspSize: usize,
    ) -> usize;
}

/** ZSTD_compressSubBlock_literal() :
 *  Compresses literals section for a sub-block.
 *  @return : compressed size of literals section of a sub-block
 *            Or 0 if unable to compress.
 *            Or error code */
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
    let lhSize: usize = 3
        + ((litSize >= (1 * KB - header)) as usize)
        + ((litSize >= (16 * KB - header)) as usize);
    let ostart = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut op: *mut BYTE = ostart.wrapping_add(lhSize);
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
        ZSTD_memcpy(
            op as *mut c_void,
            (*hufMetadata).hufDesBuffer.as_ptr() as *const c_void,
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
        op = op.wrapping_add(cSize);
        cLitSize = cLitSize.wrapping_add(cSize);
        if cSize == 0 || ERR_isError(cSize) != 0 {
            return 0;
        }
        /* If we expand and we aren't writing a header then emit uncompressed */
        if writeEntropy == 0 && cLitSize >= litSize {
            return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
        }
        /* If we are writing headers then allow expansion that doesn't change our header size. */
        if lhSize
            < (3 + ((cLitSize >= 1 * KB) as usize) + ((cLitSize >= 16 * KB) as usize))
        {
            return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
        }
    }

    /* Build header */
    match lhSize {
        3 => {
            /* 2 - 2 - 10 - 10 */
            let lhc: U32 = (hType as U32)
                .wrapping_add(((singleStream == 0) as U32) << 2)
                .wrapping_add((litSize as U32) << 4)
                .wrapping_add((cLitSize as U32) << 14);
            MEM_writeLE24(ostart as *mut c_void, lhc);
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            let lhc: U32 = (hType as U32)
                .wrapping_add(2 << 2)
                .wrapping_add((litSize as U32) << 4)
                .wrapping_add((cLitSize as U32) << 18);
            MEM_writeLE32(ostart as *mut c_void, lhc);
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            let lhc: U32 = (hType as U32)
                .wrapping_add(3 << 2)
                .wrapping_add((litSize as U32) << 4)
                .wrapping_add((cLitSize as U32) << 22);
            MEM_writeLE32(ostart as *mut c_void, lhc);
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
    let mut n: usize;
    n = 0;
    while n < nbSeqs {
        let seqLen = ZSTD_getSequenceLength(seqStore, sequences.add(n));
        litLengthSum = litLengthSum.wrapping_add(seqLen.litLength as usize);
        matchLengthSum = matchLengthSum.wrapping_add(seqLen.matchLength as usize);
        n += 1;
    }
    let _ = lastSubBlock;
    let _ = litLengthSum;
    matchLengthSum.wrapping_add(litSize)
}

/** ZSTD_compressSubBlock_sequences() :
 *  Compresses sequences section for a sub-block.
 *  @return : compressed size of sequences section of a sub-block
 *            Or 0 if it is unable to compress
 *            Or error code. */
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
        ((*cctxParams).cParams.windowLog as U32 > STREAM_ACCUMULATOR_MIN()) as c_int;
    let ostart = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let seqHead: *mut BYTE;

    *entropyWritten = 0;
    /* Sequences Header */
    if oend.offset_from(op) < (3 /*max nbSeq Size*/ + 1/*seqHead*/) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
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
        MEM_writeLE16(op.add(1) as *mut c_void, (nbSeq - LONGNBSEQ as usize) as U16);
        op = op.add(3);
    }
    if nbSeq == 0 {
        return op.offset_from(ostart) as usize;
    }

    /* seqHead : flags for FSE encoding type */
    seqHead = op;
    op = op.add(1);

    if writeEntropy != 0 {
        let LLtype: U32 = (*fseMetadata).llType as U32;
        let Offtype: U32 = (*fseMetadata).ofType as U32;
        let MLtype: U32 = (*fseMetadata).mlType as U32;
        *seqHead = ((LLtype << 6).wrapping_add(Offtype << 4).wrapping_add(MLtype << 2)) as BYTE;
        ZSTD_memcpy(
            op as *mut c_void,
            (*fseMetadata).fseTablesBuffer.as_ptr() as *const c_void,
            (*fseMetadata).fseTablesSize,
        );
        op = op.add((*fseMetadata).fseTablesSize);
    } else {
        let repeat: U32 = set_repeat as U32;
        *seqHead = ((repeat << 6).wrapping_add(repeat << 4).wrapping_add(repeat << 2)) as BYTE;
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
        if ERR_isError(bitstreamSize) != 0 {
            return bitstreamSize;
        }
        op = op.wrapping_add(bitstreamSize);
        /* zstd versions <= 1.3.4 mistakenly report corruption when
         * FSE_readNCount() receives a buffer < 4 bytes.
         * In this exceedingly rare case, we will simply emit an uncompressed
         * block, since it isn't worth optimizing.
         */
        if writeEntropy != 0
            && (*fseMetadata).lastCountSize != 0
            && (*fseMetadata).lastCountSize + bitstreamSize < 4
        {
            /* NCountSize >= 2 && bitstreamSize > 0 ==> lastCountSize == 3 */
            return 0;
        }
    }

    /* zstd versions <= 1.4.0 mistakenly report error when
     * sequences section body size is less than 3 bytes.
     */
    if op.offset_from(seqHead) < 4 {
        return 0;
    }

    *entropyWritten = 1;
    op.offset_from(ostart) as usize
}

/** ZSTD_compressSubBlock() :
 *  Compresses a single sub-block.
 *  @return : compressed size of the sub-block
 *            Or 0 if it failed to compress. */
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
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart.wrapping_add(ZSTD_blockHeaderSize);
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
        if ERR_isError(cLitSize) != 0 {
            return cLitSize;
        }
        if cLitSize == 0 {
            return 0;
        }
        op = op.wrapping_add(cLitSize);
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
        if ERR_isError(cSeqSize) != 0 {
            return cSeqSize;
        }
        if cSeqSize == 0 {
            return 0;
        }
        op = op.wrapping_add(cSeqSize);
    }
    /* Write block header */
    {
        let cSize: usize = (op.offset_from(ostart) as usize) - ZSTD_blockHeaderSize;
        let cBlockHeader24: U32 = lastBlock
            .wrapping_add((bt_compressed as U32) << 1)
            .wrapping_add((cSize << 3) as U32);
        MEM_writeLE24(ostart as *mut c_void, cBlockHeader24);
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
        if ERR_isError(largest) != 0 {
            return litSize;
        }
        {
            let mut cLitSizeEstimate: usize = HUF_estimateCompressedSize(
                (*huf).CTable.as_ptr() as *const HUF_CElt,
                countWksp,
                maxSymbolValue,
            );
            if writeEntropy != 0 {
                cLitSizeEstimate =
                    cLitSizeEstimate.wrapping_add((*hufMetadata).hufDesSize);
            }
            return cLitSizeEstimate.wrapping_add(literalSectionHeaderSize);
        }
    }
    0
}

unsafe fn ZSTD_estimateSubBlockSize_symbolType(
    type_: SymbolEncodingType_e,
    codeTable: *const BYTE,
    maxCode: u32,
    nbSeq: usize,
    fseCTable: *const FSE_CTable,
    additionalBits: *const U8,
    defaultNorm: *const i16,
    defaultNormLog: U32,
    defaultMax: U32,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let countWksp = workspace as *mut u32;
    let mut ctp: *const BYTE = codeTable;
    let ctStart: *const BYTE = ctp;
    let ctEnd: *const BYTE = ctStart.wrapping_add(nbSeq);
    let mut cSymbolTypeSizeEstimateInBits: usize = 0;
    let mut max: u32 = maxCode;

    /* can't fail */
    HIST_countFast_wksp(
        countWksp,
        &mut max,
        codeTable as *const c_void,
        nbSeq,
        workspace,
        wkspSize,
    );
    if type_ == set_basic {
        /* We selected this encoding type, so it must be valid. */
        cSymbolTypeSizeEstimateInBits = if max <= defaultMax {
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, countWksp, max)
        } else {
            ERROR(ZSTD_error_GENERIC)
        };
    } else if type_ == set_rle {
        cSymbolTypeSizeEstimateInBits = 0;
    } else if type_ == set_compressed || type_ == set_repeat {
        cSymbolTypeSizeEstimateInBits = ZSTD_fseBitCost(fseCTable, countWksp, max);
    }
    if ERR_isError(cSymbolTypeSizeEstimateInBits) != 0 {
        return nbSeq.wrapping_mul(10);
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits = cSymbolTypeSizeEstimateInBits
                .wrapping_add(*additionalBits.add(*ctp as usize) as usize);
        } else {
            /* for offset, offset code is also the number of additional bits */
            cSymbolTypeSizeEstimateInBits =
                cSymbolTypeSizeEstimateInBits.wrapping_add(*ctp as usize);
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
    cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add(ZSTD_estimateSubBlockSize_symbolType(
        (*fseMetadata).ofType,
        ofCodeTable,
        MaxOff,
        nbSeq,
        (*fseTables).offcodeCTable.as_ptr(),
        core::ptr::null(),
        OF_defaultNorm.as_ptr(),
        OF_defaultNormLog,
        DefaultMaxOff,
        workspace,
        wkspSize,
    ));
    cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add(ZSTD_estimateSubBlockSize_symbolType(
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
    ));
    cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add(ZSTD_estimateSubBlockSize_symbolType(
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
    ));
    if writeEntropy != 0 {
        cSeqSizeEstimate = cSeqSizeEstimate.wrapping_add((*fseMetadata).fseTablesSize);
    }
    cSeqSizeEstimate.wrapping_add(sequencesSectionHeaderSize)
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
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
    let mut ebs: EstimatedBlockSize = EstimatedBlockSize::default();
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
    ebs.estBlockSize = ebs
        .estBlockSize
        .wrapping_add(ebs.estLitSize)
        .wrapping_add(ZSTD_blockHeaderSize);
    ebs
}

unsafe fn ZSTD_needSequenceEntropyTables(
    fseMetadata: *const ZSTD_fseCTablesMetadata_t,
) -> c_int {
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

unsafe fn countLiterals(
    seqStore: *const SeqStore_t,
    sp: *const SeqDef,
    seqCount: usize,
) -> usize {
    let mut n: usize;
    let mut total: usize = 0;
    n = 0;
    while n < seqCount {
        total = total
            .wrapping_add(ZSTD_getSequenceLength(seqStore, sp.add(n)).litLength as usize);
        n += 1;
    }
    total
}

const BYTESCALE: usize = 256;

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
    /* generous estimate */
    let headerSize: usize = (firstSubBlock as usize)
        .wrapping_mul(120)
        .wrapping_mul(BYTESCALE);
    budget = budget.wrapping_add(headerSize);

    /* first sequence => at least one sequence*/
    budget = budget.wrapping_add(
        ((*sp.add(0)).litLength as usize)
            .wrapping_mul(avgLitCost)
            .wrapping_add(avgSeqCost),
    );
    if budget > targetBudget {
        return 1;
    }
    inSize = ((*sp.add(0)).litLength as usize)
        .wrapping_add(((*sp.add(0)).mlBase as usize).wrapping_add(MINMATCH));

    /* loop over sequences */
    n = 1;
    while n < nbSeqs {
        let currentCost: usize = ((*sp.add(n)).litLength as usize)
            .wrapping_mul(avgLitCost)
            .wrapping_add(avgSeqCost);
        budget = budget.wrapping_add(currentCost);
        inSize = inSize.wrapping_add(
            ((*sp.add(n)).litLength as usize)
                .wrapping_add(((*sp.add(n)).mlBase as usize).wrapping_add(MINMATCH)),
        );
        /* stop when sub-block budget is reached */
        if (budget > targetBudget)
            /* though continue to expand until the sub-block is deemed compressible */
            && (budget < inSize.wrapping_mul(BYTESCALE))
        {
            break;
        }
        n += 1;
    }

    n
}

/** ZSTD_compressSubBlock_multi() :
 *  Breaks super-block into multiple sub-blocks and compresses them.
 *  Entropy will be written into the first block.
 *  The following blocks use repeat_mode to compress.
 *  Sub-blocks are all compressed, except the last one when beneficial.
 *  @return : compressed size of the super block (which features multiple ZSTD blocks)
 *            or 0 if it failed to compress. */
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
    /* tracks progresses within seqStorePtr->sequences */
    let mut sp: *const SeqDef = sstart;
    let nbSeqs: usize = send.offset_from(sstart) as usize;
    let lstart: *const BYTE = (*seqStorePtr).litStart;
    let lend: *const BYTE = (*seqStorePtr).lit;
    let mut lp: *const BYTE = lstart;
    let nbLiterals: usize = lend.offset_from(lstart) as usize;
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let mut llCodePtr: *const BYTE = (*seqStorePtr).llCode;
    let mut mlCodePtr: *const BYTE = (*seqStorePtr).mlCode;
    let mut ofCodePtr: *const BYTE = (*seqStorePtr).ofCode;
    /* enforce minimum size, to reduce undesirable side effects */
    let minTarget: usize = ZSTD_TARGETCBLOCKSIZE_MIN as usize;
    let targetCBlockSize: usize = MAX(minTarget, (*cctxParams).targetCBlockSize);
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
            (ebs.estLitSize.wrapping_mul(BYTESCALE)) / nbLiterals
        } else {
            BYTESCALE
        };
        let avgSeqCost: usize =
            ((ebs.estBlockSize.wrapping_sub(ebs.estLitSize)).wrapping_mul(BYTESCALE)) / nbSeqs;
        let nbSubBlocks: usize = MAX(
            (ebs.estBlockSize.wrapping_add(targetCBlockSize / 2)) / targetCBlockSize,
            1,
        );
        let mut n: usize;
        let avgBlockBudget: usize;
        let mut blockBudgetSupp: usize = 0;
        avgBlockBudget = (ebs.estBlockSize.wrapping_mul(BYTESCALE)) / nbSubBlocks;
        /* simplification: if estimates states that the full superblock doesn't compress,
         * just bail out immediately. this will result in the production of a single
         * uncompressed block covering @srcSize.*/
        if ebs.estBlockSize > srcSize {
            return 0;
        }

        /* compress and write sub-blocks */
        n = 0;
        while n < nbSubBlocks - 1 {
            /* determine nb of sequences for current sub-block + nbLiterals from next sequence */
            let seqCount: usize = sizeBlockSequences(
                sp,
                send.offset_from(sp) as usize,
                avgBlockBudget.wrapping_add(blockBudgetSupp),
                avgLitCost,
                avgSeqCost,
                (n == 0) as c_int,
            );
            /* if reached last sequence : break to last sub-block (simplification) */
            if sp.wrapping_add(seqCount) == send {
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
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }

                /* check compressibility, update state components */
                if cSize > 0 && cSize < decompressedSize {
                    ip = ip.wrapping_add(decompressedSize);
                    lp = lp.wrapping_add(litSize);
                    op = op.wrapping_add(cSize);
                    llCodePtr = llCodePtr.wrapping_add(seqCount);
                    mlCodePtr = mlCodePtr.wrapping_add(seqCount);
                    ofCodePtr = ofCodePtr.wrapping_add(seqCount);
                    /* Entropy only needs to be written once */
                    if litEntropyWritten != 0 {
                        writeLitEntropy = 0;
                    }
                    if seqEntropyWritten != 0 {
                        writeSeqEntropy = 0;
                    }
                    sp = sp.wrapping_add(seqCount);
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
        if ERR_isError(cSize) != 0 {
            return cSize;
        }

        /* update pointers, the nb of literals borrowed from next sequence must be preserved */
        if cSize > 0 && cSize < decompressedSize {
            ip = ip.wrapping_add(decompressedSize);
            lp = lp.wrapping_add(litSize);
            op = op.wrapping_add(cSize);
            llCodePtr = llCodePtr.wrapping_add(seqCount);
            mlCodePtr = mlCodePtr.wrapping_add(seqCount);
            ofCodePtr = ofCodePtr.wrapping_add(seqCount);
            /* Entropy only needs to be written once */
            if litEntropyWritten != 0 {
                writeLitEntropy = 0;
            }
            if seqEntropyWritten != 0 {
                writeSeqEntropy = 0;
            }
            sp = sp.wrapping_add(seqCount);
        }
    }

    if writeLitEntropy != 0 {
        ZSTD_memcpy(
            &mut (*nextCBlock).entropy.huf as *mut ZSTD_hufCTables_t as *mut c_void,
            &(*prevCBlock).entropy.huf as *const ZSTD_hufCTables_t as *const c_void,
            core::mem::size_of::<ZSTD_hufCTables_t>(),
        );
    }
    if writeSeqEntropy != 0
        && ZSTD_needSequenceEntropyTables(&(*entropyMetadata).fseMetadata) != 0
    {
        /* If we haven't written our entropy tables, then we've violated our contract and
         * must emit an uncompressed block.
         */
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
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        op = op.wrapping_add(cSize);
        /* We have to regenerate the repcodes because we've skipped some sequences */
        if sp < send {
            let mut seq: *const SeqDef;
            let mut rep: Repcodes_t = Repcodes_t { rep: [0; 3] };
            ZSTD_memcpy(
                &mut rep as *mut Repcodes_t as *mut c_void,
                (*prevCBlock).rep.as_ptr() as *const c_void,
                core::mem::size_of::<Repcodes_t>(),
            );
            seq = sstart;
            while seq < sp {
                ZSTD_updateRep(
                    rep.rep.as_mut_ptr(),
                    (*seq).offBase,
                    (ZSTD_getSequenceLength(seqStorePtr, seq).litLength == 0) as U32,
                );
                seq = seq.add(1);
            }
            ZSTD_memcpy(
                (*nextCBlock).rep.as_mut_ptr() as *mut c_void,
                &rep as *const Repcodes_t as *const c_void,
                core::mem::size_of::<Repcodes_t>(),
            );
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
    /* C: `ZSTD_entropyCTablesMetadata_t entropyMetadata;` - uninitialized stack object */
    let mut entropyMetadata_storage =
        core::mem::MaybeUninit::<ZSTD_entropyCTablesMetadata_t>::uninit();
    let entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t = entropyMetadata_storage.as_mut_ptr();

    {
        /* statically allocated in resetCCtx */
        let err_code = ZSTD_buildBlockEntropyStats(
            &(*zc).seqStore,
            &(*(*zc).blockState.prevCBlock).entropy,
            &mut (*(*zc).blockState.nextCBlock).entropy,
            &(*zc).appliedParams,
            entropyMetadata,
            (*zc).tmpWorkspace,
            (*zc).tmpWkspSize,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    ZSTD_compressSubBlock_multi(
        &(*zc).seqStore,
        (*zc).blockState.prevCBlock,
        (*zc).blockState.nextCBlock,
        entropyMetadata,
        &(*zc).appliedParams,
        dst,
        dstCapacity,
        src,
        srcSize,
        (*zc).bmi2,
        lastBlock,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize, /* statically allocated in resetCCtx */
    )
}
