//! Translation of `compress/zstd_compress_superblock.c` (super-block compression).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;
use crate::compress::hist::*;
use crate::compress::huf_compress::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_compress_literals::*;
use crate::compress::zstd_compress_sequences::*;

use core::ffi::{c_int, c_short, c_uint, c_void};

/** ZSTD_compressSubBlock_literal() :
 *  Compresses literals section for a sub-block. */
unsafe fn ZSTD_compressSubBlock_literal(
    hufTable: *const HUF_CElt,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    literals: *const BYTE,
    litSize: size_t,
    dst: *mut c_void,
    dstSize: size_t,
    bmi2: c_int,
    writeEntropy: c_int,
    entropyWritten: *mut c_int,
) -> size_t {
    let header: size_t = if writeEntropy != 0 { 200 } else { 0 };
    let lhSize: size_t = 3
        + (litSize >= ((1 << 10) - header)) as size_t
        + (litSize >= ((16 << 10) - header)) as size_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstSize);
    let mut op: *mut BYTE = ostart.add(lhSize);
    let singleStream: U32 = (lhSize == 3) as U32;
    let hType: SymbolEncodingType_e = if writeEntropy != 0 {
        (*hufMetadata).hType
    } else {
        set_repeat
    };
    let mut cLitSize: size_t = 0;

    *entropyWritten = 0;
    if litSize == 0 || (*hufMetadata).hType == set_basic {
        return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
    } else if (*hufMetadata).hType == set_rle {
        return ZSTD_compressRleLiteralsBlock(dst, dstSize, literals as *const c_void, litSize);
    }

    if writeEntropy != 0 && (*hufMetadata).hType == set_compressed {
        ZSTD_memcpy(
            op,
            (*hufMetadata).hufDesBuffer.as_ptr(),
            (*hufMetadata).hufDesSize,
        );
        op = op.add((*hufMetadata).hufDesSize);
        cLitSize += (*hufMetadata).hufDesSize;
    }

    {
        let flags: c_int = if bmi2 != 0 { HUF_flags_bmi2 as c_int } else { 0 };
        let cSize: size_t = if singleStream != 0 {
            HUF_compress1X_usingCTable(
                op as *mut c_void,
                (oend as size_t).wrapping_sub(op as size_t),
                literals as *const c_void,
                litSize,
                hufTable,
                flags,
            )
        } else {
            HUF_compress4X_usingCTable(
                op as *mut c_void,
                (oend as size_t).wrapping_sub(op as size_t),
                literals as *const c_void,
                litSize,
                hufTable,
                flags,
            )
        };
        op = op.add(cSize);
        cLitSize += cSize;
        if cSize == 0 || ERR_isError(cSize) != 0 {
            return 0;
        }
        /* If we expand and we aren't writing a header then emit uncompressed */
        if writeEntropy == 0 && cLitSize >= litSize {
            return ZSTD_noCompressLiterals(dst, dstSize, literals as *const c_void, litSize);
        }
        /* If we are writing headers then allow expansion that doesn't change our header size. */
        if lhSize
            < (3 + (cLitSize >= (1 << 10)) as size_t + (cLitSize >= (16 << 10)) as size_t)
        {
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
            MEM_writeLE24(ostart, lhc);
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            let lhc: U32 =
                hType + (2 << 2) + ((litSize as U32) << 4) + ((cLitSize as U32) << 18);
            MEM_writeLE32(ostart, lhc);
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            let lhc: U32 =
                hType + (3 << 2) + ((litSize as U32) << 4) + ((cLitSize as U32) << 22);
            MEM_writeLE32(ostart, lhc);
            *ostart.add(4) = (cLitSize >> 10) as BYTE;
        }
        _ => {
            /* not possible : lhSize is {3,4,5} */
        }
    }
    *entropyWritten = 1;
    (op as size_t).wrapping_sub(ostart as size_t)
}

unsafe fn ZSTD_seqDecompressedSize(
    seqStore: *const SeqStore_t,
    sequences: *const SeqDef,
    nbSeqs: size_t,
    litSize: size_t,
    lastSubBlock: c_int,
) -> size_t {
    let mut matchLengthSum: size_t = 0;
    let mut litLengthSum: size_t = 0;
    let mut n: size_t = 0;
    while n < nbSeqs {
        let seqLen: ZSTD_SequenceLength = ZSTD_getSequenceLength(seqStore, sequences.add(n));
        litLengthSum += seqLen.litLength as size_t;
        matchLengthSum += seqLen.matchLength as size_t;
        n += 1;
    }
    let _ = lastSubBlock;
    let _ = litLengthSum;
    matchLengthSum + litSize
}

/** ZSTD_compressSubBlock_sequences() :
 *  Compresses sequences section for a sub-block. */
unsafe fn ZSTD_compressSubBlock_sequences(
    fseTables: *const ZSTD_fseCTables_t,
    fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    sequences: *const SeqDef,
    nbSeq: size_t,
    llCode: *const BYTE,
    mlCode: *const BYTE,
    ofCode: *const BYTE,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: size_t,
    bmi2: c_int,
    writeEntropy: c_int,
    entropyWritten: *mut c_int,
) -> size_t {
    let longOffsets: c_int =
        ((*cctxParams).cParams.windowLog > STREAM_ACCUMULATOR_MIN()) as c_int;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let seqHead: *mut BYTE;

    *entropyWritten = 0;
    /* Sequences Header */
    if ((oend as size_t).wrapping_sub(op as size_t) as isize)
        < (3 /*max nbSeq Size*/ + 1 /*seqHead*/)
    {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbSeq < 128 {
        *op = nbSeq as BYTE;
        op = op.add(1);
    } else if nbSeq < LONGNBSEQ as size_t {
        *op.add(0) = ((nbSeq >> 8) + 0x80) as BYTE;
        *op.add(1) = nbSeq as BYTE;
        op = op.add(2);
    } else {
        *op.add(0) = 0xFF;
        MEM_writeLE16(op.add(1), (nbSeq - LONGNBSEQ as size_t) as U16);
        op = op.add(3);
    }
    if nbSeq == 0 {
        return (op as size_t).wrapping_sub(ostart as size_t);
    }

    /* seqHead : flags for FSE encoding type */
    seqHead = op;
    op = op.add(1);

    if writeEntropy != 0 {
        let LLtype: U32 = (*fseMetadata).llType;
        let Offtype: U32 = (*fseMetadata).ofType;
        let MLtype: U32 = (*fseMetadata).mlType;
        *seqHead = ((LLtype << 6) + (Offtype << 4) + (MLtype << 2)) as BYTE;
        ZSTD_memcpy(
            op,
            (*fseMetadata).fseTablesBuffer.as_ptr(),
            (*fseMetadata).fseTablesSize,
        );
        op = op.add((*fseMetadata).fseTablesSize);
    } else {
        let repeat: U32 = set_repeat;
        *seqHead = ((repeat << 6) + (repeat << 4) + (repeat << 2)) as BYTE;
    }

    {
        let bitstreamSize: size_t = ZSTD_encodeSequences(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
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
    if ((op as isize) - (seqHead as isize)) < 4 {
        return 0;
    }

    *entropyWritten = 1;
    (op as size_t).wrapping_sub(ostart as size_t)
}

/** ZSTD_compressSubBlock() :
 *  Compresses a single sub-block. */
unsafe fn ZSTD_compressSubBlock(
    entropy: *const ZSTD_entropyCTables_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    sequences: *const SeqDef,
    nbSeq: size_t,
    literals: *const BYTE,
    litSize: size_t,
    llCode: *const BYTE,
    mlCode: *const BYTE,
    ofCode: *const BYTE,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: size_t,
    bmi2: c_int,
    writeLitEntropy: c_int,
    writeSeqEntropy: c_int,
    litEntropyWritten: *mut c_int,
    seqEntropyWritten: *mut c_int,
    lastBlock: U32,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart.add(ZSTD_blockHeaderSize);
    {
        let cLitSize: size_t = ZSTD_compressSubBlock_literal(
            (*entropy).huf.CTable.as_ptr() as *const HUF_CElt,
            &(*entropyMetadata).hufMetadata,
            literals,
            litSize,
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
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
        op = op.add(cLitSize);
    }
    {
        let cSeqSize: size_t = ZSTD_compressSubBlock_sequences(
            &(*entropy).fse,
            &(*entropyMetadata).fseMetadata,
            sequences,
            nbSeq,
            llCode,
            mlCode,
            ofCode,
            cctxParams,
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
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
        op = op.add(cSeqSize);
    }
    /* Write block header */
    {
        let cSize: size_t =
            (op as size_t).wrapping_sub(ostart as size_t) - ZSTD_blockHeaderSize;
        let cBlockHeader24: U32 =
            lastBlock + (((bt_compressed as U32) << 1)) + ((cSize << 3) as U32);
        MEM_writeLE24(ostart, cBlockHeader24);
    }
    (op as size_t).wrapping_sub(ostart as size_t)
}

unsafe fn ZSTD_estimateSubBlockSize_literal(
    literals: *const BYTE,
    litSize: size_t,
    huf: *const ZSTD_hufCTables_t,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: size_t,
    writeEntropy: c_int,
) -> size_t {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut maxSymbolValue: c_uint = 255;
    let literalSectionHeaderSize: size_t = 3; /* Use hard coded size of 3 bytes */

    if (*hufMetadata).hType == set_basic {
        return litSize;
    } else if (*hufMetadata).hType == set_rle {
        return 1;
    } else if (*hufMetadata).hType == set_compressed || (*hufMetadata).hType == set_repeat {
        let largest: size_t = HIST_count_wksp(
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
            let mut cLitSizeEstimate: size_t = HUF_estimateCompressedSize(
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
    0
}

unsafe fn ZSTD_estimateSubBlockSize_symbolType(
    type_: SymbolEncodingType_e,
    codeTable: *const BYTE,
    maxCode: c_uint,
    nbSeq: size_t,
    fseCTable: *const FSE_CTable,
    additionalBits: *const U8,
    defaultNorm: *const c_short,
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

    HIST_countFast_wksp(
        countWksp,
        &mut max,
        codeTable as *const c_void,
        nbSeq,
        workspace,
        wkspSize,
    ); /* can't fail */
    if type_ == set_basic {
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
        return nbSeq * 10;
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits += *additionalBits.add(*ctp as usize) as size_t;
        } else {
            cSymbolTypeSizeEstimateInBits += *ctp as size_t; /* for offset, offset code is also the number of additional bits */
        }
        ctp = ctp.add(1);
    }
    cSymbolTypeSizeEstimateInBits / 8
}

unsafe fn ZSTD_estimateSubBlockSize_sequences(
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
    let sequencesSectionHeaderSize: size_t = 3; /* Use hard coded size of 3 bytes */
    let mut cSeqSizeEstimate: size_t = 0;
    if nbSeq == 0 {
        return sequencesSectionHeaderSize;
    }
    cSeqSizeEstimate += ZSTD_estimateSubBlockSize_symbolType(
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

#[repr(C)]
#[derive(Clone, Copy)]
struct EstimatedBlockSize {
    estLitSize: size_t,
    estBlockSize: size_t,
}
unsafe fn ZSTD_estimateSubBlockSize(
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

unsafe fn countLiterals(seqStore: *const SeqStore_t, sp: *const SeqDef, seqCount: size_t) -> size_t {
    let mut n: size_t = 0;
    let mut total: size_t = 0;
    while n < seqCount {
        total += ZSTD_getSequenceLength(seqStore, sp.add(n)).litLength as size_t;
        n += 1;
    }
    total
}

const BYTESCALE: size_t = 256;

unsafe fn sizeBlockSequences(
    sp: *const SeqDef,
    nbSeqs: size_t,
    targetBudget: size_t,
    avgLitCost: size_t,
    avgSeqCost: size_t,
    firstSubBlock: c_int,
) -> size_t {
    let mut n: size_t;
    let mut budget: size_t = 0;
    let mut inSize: size_t;
    /* entropy headers */
    let headerSize: size_t = firstSubBlock as size_t * 120 * BYTESCALE; /* generous estimate */
    budget += headerSize;

    /* first sequence => at least one sequence*/
    budget += (*sp.add(0)).litLength as size_t * avgLitCost + avgSeqCost;
    if budget > targetBudget {
        return 1;
    }
    inSize = (*sp.add(0)).litLength as size_t + ((*sp.add(0)).mlBase as size_t + MINMATCH as size_t);

    /* loop over sequences */
    n = 1;
    while n < nbSeqs {
        let currentCost: size_t = (*sp.add(n)).litLength as size_t * avgLitCost + avgSeqCost;
        budget += currentCost;
        inSize +=
            (*sp.add(n)).litLength as size_t + ((*sp.add(n)).mlBase as size_t + MINMATCH as size_t);
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

/** ZSTD_compressSubBlock_multi() :
 *  Breaks super-block into multiple sub-blocks and compresses them. */
unsafe fn ZSTD_compressSubBlock_multi(
    seqStorePtr: *const SeqStore_t,
    prevCBlock: *const ZSTD_compressedBlockState_t,
    nextCBlock: *mut ZSTD_compressedBlockState_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    bmi2: c_int,
    lastBlock: U32,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let sstart: *const SeqDef = (*seqStorePtr).sequencesStart;
    let send: *const SeqDef = (*seqStorePtr).sequences;
    let mut sp: *const SeqDef = sstart; /* tracks progresses within seqStorePtr->sequences */
    let nbSeqs: size_t = (send as size_t).wrapping_sub(sstart as size_t)
        / core::mem::size_of::<SeqDef>();
    let lstart: *const BYTE = (*seqStorePtr).litStart;
    let lend: *const BYTE = (*seqStorePtr).lit;
    let mut lp: *const BYTE = lstart;
    let nbLiterals: size_t = (lend as size_t).wrapping_sub(lstart as size_t);
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.wrapping_add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let mut llCodePtr: *const BYTE = (*seqStorePtr).llCode;
    let mut mlCodePtr: *const BYTE = (*seqStorePtr).mlCode;
    let mut ofCodePtr: *const BYTE = (*seqStorePtr).ofCode;
    let minTarget: size_t = ZSTD_TARGETCBLOCKSIZE_MIN as size_t; /* enforce minimum size */
    let targetCBlockSize: size_t = MAX(minTarget, (*cctxParams).targetCBlockSize);
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
        let avgLitCost: size_t = if nbLiterals != 0 {
            (ebs.estLitSize * BYTESCALE) / nbLiterals
        } else {
            BYTESCALE
        };
        let avgSeqCost: size_t = ((ebs.estBlockSize - ebs.estLitSize) * BYTESCALE) / nbSeqs;
        let nbSubBlocks: size_t =
            MAX((ebs.estBlockSize + (targetCBlockSize / 2)) / targetCBlockSize, 1);
        let mut n: size_t;
        let avgBlockBudget: size_t;
        let mut blockBudgetSupp: size_t = 0;
        avgBlockBudget = (ebs.estBlockSize * BYTESCALE) / nbSubBlocks;
        /* simplification: if estimates states that the full superblock doesn't compress, just bail out immediately */
        if ebs.estBlockSize > srcSize {
            return 0;
        }

        /* compress and write sub-blocks */
        n = 0;
        while n < nbSubBlocks - 1 {
            /* determine nb of sequences for current sub-block + nbLiterals from next sequence */
            let seqCount: size_t = sizeBlockSequences(
                sp,
                (send as size_t).wrapping_sub(sp as size_t) / core::mem::size_of::<SeqDef>(),
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
                let litSize: size_t = countLiterals(seqStorePtr, sp, seqCount);
                let decompressedSize: size_t =
                    ZSTD_seqDecompressedSize(seqStorePtr, sp, seqCount, litSize, 0);
                let cSize: size_t = ZSTD_compressSubBlock(
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
                    (oend as size_t).wrapping_sub(op as size_t),
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
        let litSize: size_t = (lend as size_t).wrapping_sub(lp as size_t);
        let seqCount: size_t =
            (send as size_t).wrapping_sub(sp as size_t) / core::mem::size_of::<SeqDef>();
        let decompressedSize: size_t =
            ZSTD_seqDecompressedSize(seqStorePtr, sp, seqCount, litSize, 1);
        let cSize: size_t = ZSTD_compressSubBlock(
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
            (oend as size_t).wrapping_sub(op as size_t),
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
        ZSTD_memcpy(
            &mut (*nextCBlock).entropy.huf as *mut ZSTD_hufCTables_t as *mut BYTE,
            &(*prevCBlock).entropy.huf as *const ZSTD_hufCTables_t as *const BYTE,
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
        let rSize: size_t = (iend as size_t).wrapping_sub(ip as size_t);
        let cSize: size_t = ZSTD_noCompressBlock(
            op as *mut c_void,
            (oend as size_t).wrapping_sub(op as size_t),
            ip as *const c_void,
            rSize,
            lastBlock,
        );
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        op = op.add(cSize);
        /* We have to regenerate the repcodes because we've skipped some sequences */
        if sp < send {
            let mut seq: *const SeqDef;
            let mut rep: Repcodes_t = core::mem::zeroed();
            ZSTD_memcpy(
                &mut rep as *mut Repcodes_t as *mut BYTE,
                (*prevCBlock).rep.as_ptr() as *const BYTE,
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
                (*nextCBlock).rep.as_mut_ptr() as *mut BYTE,
                &rep as *const Repcodes_t as *const BYTE,
                core::mem::size_of::<Repcodes_t>(),
            );
        }
    }

    (op as size_t).wrapping_sub(ostart as size_t)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressSuperBlock(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    lastBlock: c_uint,
) -> size_t {
    let mut entropyMetadata: ZSTD_entropyCTablesMetadata_t = core::mem::zeroed();

    {
        let err = ZSTD_buildBlockEntropyStats(
            &(*zc).seqStore,
            &(*(*zc).blockState.prevCBlock).entropy,
            &mut (*(*zc).blockState.nextCBlock).entropy,
            &(*zc).appliedParams,
            &mut entropyMetadata,
            (*zc).tmpWorkspace,
            (*zc).tmpWkspSize, /* statically allocated in resetCCtx */
        );
        if ERR_isError(err) != 0 {
            return err;
        }
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
        (*zc).tmpWkspSize, /* statically allocated in resetCCtx */
    )
}
