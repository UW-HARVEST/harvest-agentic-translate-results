/* ===========================================================================
 * zstd_compress.c — part 4  (C lines 3631..4790)
 *
 * Block entropy statistics (literals + sequences), block size estimation,
 * block splitting (post-block splitter), block compression entry points,
 * overflow correction, pre-block splitting, frame chunk compression and
 * frame header emission.
 *
 * This file is `include!`d by `zstd_compress.rs`: it must contain items only,
 * no `use` statements and no `extern "C"` blocks.
 * ======================================================================== */

/* heuristic */
pub const COMPRESS_LITERALS_SIZE_MIN: usize = 63;

/** ZSTD_buildBlockEntropyStats_literals() :
 *  Builds entropy for the literals.
 *  Stores literals block type (raw, rle, compressed, repeat) and
 *  huffman description table to hufMetadata.
 *  Requires ENTROPY_WORKSPACE_SIZE workspace
 * @return : size of huffman description table, or an error code
 */
unsafe fn ZSTD_buildBlockEntropyStats_literals(
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
    let wkspEnd: *mut BYTE = wkspStart.add(wkspSize);
    let countWkspStart: *mut BYTE = wkspStart;
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let countWkspSize: usize =
        (HUF_SYMBOLVALUE_MAX as usize + 1) * core::mem::size_of::<c_uint>();
    let nodeWksp: *mut BYTE = countWkspStart.add(countWkspSize);
    let nodeWkspSize: usize = wkspEnd.offset_from(nodeWksp) as usize;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX as c_uint;
    let mut huffLog: c_uint = LitHufLog as c_uint;
    let mut repeat: HUF_repeat = (*prevHuf).repeatMode;

    /* Prepare nextEntropy assuming reusing the existing table */
    ZSTD_memcpy(
        nextHuf as *mut c_void,
        prevHuf as *const c_void,
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
        let largest: usize = HIST_count_wksp(
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
        && HUF_validateCTable(
            (*prevHuf).CTable.as_ptr() as *const HUF_CElt,
            countWksp,
            maxSymbolValue,
        ) == 0
    {
        repeat = HUF_repeat_none;
    }

    /* Build Huffman Tree */
    ZSTD_memset(
        (*nextHuf).CTable.as_mut_ptr() as *mut c_void,
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
        let maxBits: usize = HUF_buildCTable_wksp(
            (*nextHuf).CTable.as_mut_ptr() as *mut HUF_CElt,
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
        let newCSize: usize = HUF_estimateCompressedSize(
            (*nextHuf).CTable.as_ptr() as *const HUF_CElt,
            countWksp,
            maxSymbolValue,
        );
        let hSize: usize = HUF_writeCTable_wksp(
            (*hufMetadata).hufDesBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*hufMetadata).hufDesBuffer),
            (*nextHuf).CTable.as_ptr() as *const HUF_CElt,
            maxSymbolValue,
            huffLog,
            nodeWksp as *mut c_void,
            nodeWkspSize,
        );
        /* Check against repeating the previous CTable */
        if repeat != HUF_repeat_none {
            let oldCSize: usize = HUF_estimateCompressedSize(
                (*prevHuf).CTable.as_ptr() as *const HUF_CElt,
                countWksp,
                maxSymbolValue,
            );
            if oldCSize < srcSize && (oldCSize <= hSize + newCSize || hSize + 12 >= srcSize) {
                ZSTD_memcpy(
                    nextHuf as *mut c_void,
                    prevHuf as *const c_void,
                    core::mem::size_of::<ZSTD_hufCTables_t>(),
                );
                (*hufMetadata).hType = set_repeat;
                return 0;
            }
        }
        if newCSize + hSize >= srcSize {
            ZSTD_memcpy(
                nextHuf as *mut c_void,
                prevHuf as *const c_void,
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
 * and updates nextEntropy to the appropriate repeatMode.
 */
unsafe fn ZSTD_buildDummySequencesStatistics(
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

/** ZSTD_buildBlockEntropyStats_sequences() :
 *  Builds entropy for the sequences.
 *  Stores symbol compression modes and fse table to fseMetadata.
 *  Requires ENTROPY_WORKSPACE_SIZE wksp.
 * @return : size of fse tables or error code */
unsafe fn ZSTD_buildBlockEntropyStats_sequences(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    fseMetadata: *mut ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let nbSeq: usize = (*seqStorePtr)
        .sequences
        .offset_from((*seqStorePtr).sequencesStart) as usize;
    let ostart: *mut BYTE = (*fseMetadata).fseTablesBuffer.as_mut_ptr();
    let oend: *const BYTE = ostart.add(core::mem::size_of_val(&(*fseMetadata).fseTablesBuffer));
    let op: *mut BYTE = ostart;
    let countWorkspace: *mut c_uint = workspace as *mut c_uint;
    let entropyWorkspace: *mut c_uint = countWorkspace.add(MaxSeq as usize + 1);
    let entropyWorkspaceSize: usize =
        wkspSize - (MaxSeq as usize + 1) * core::mem::size_of::<c_uint>();
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
 *  Builds entropy for the block.
 *  Requires workspace size ENTROPY_WORKSPACE_SIZE
 * @return : 0 on success, or an error code
 *  Note : also employed in superblock
 */
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
    let litSize: usize = (*seqStorePtr).lit.offset_from((*seqStorePtr).litStart) as usize;
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
        core::ptr::addr_of!((*prevEntropy).huf),
        core::ptr::addr_of_mut!((*nextEntropy).huf),
        core::ptr::addr_of_mut!((*entropyMetadata).hufMetadata),
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
        core::ptr::addr_of!((*prevEntropy).fse),
        core::ptr::addr_of_mut!((*nextEntropy).fse),
        cctxParams,
        core::ptr::addr_of_mut!((*entropyMetadata).fseMetadata),
        workspace,
        wkspSize,
    );
    if ERR_isError((*entropyMetadata).fseMetadata.fseTablesSize) != 0 {
        return (*entropyMetadata).fseMetadata.fseTablesSize;
    }
    0
}

/* Returns the size estimate for the literals section (header + content) of a block */
unsafe fn ZSTD_estimateBlockSize_literal(
    literals: *const BYTE,
    litSize: usize,
    huf: *const ZSTD_hufCTables_t,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeEntropy: c_int,
) -> usize {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX as c_uint;
    let literalSectionHeaderSize: usize =
        3 + (litSize >= 1024) as usize + (litSize >= 16 * 1024) as usize;
    let singleStream: U32 = (litSize < 256) as U32;

    if (*hufMetadata).hType == set_basic {
        return litSize;
    } else if (*hufMetadata).hType == set_rle {
        return 1;
    } else if (*hufMetadata).hType == set_compressed || (*hufMetadata).hType == set_repeat {
        let largest: usize = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            literals as *const BYTE as *const c_void,
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
                cLitSizeEstimate += (*hufMetadata).hufDesSize;
            }
            if singleStream == 0 {
                cLitSizeEstimate += 6; /* multi-stream huffman uses 6-byte jump table */
            }
            return cLitSizeEstimate + literalSectionHeaderSize;
        }
    }
    0
}

/* Returns the size estimate for the FSE-compressed symbols (of, ml, ll) of a block */
unsafe fn ZSTD_estimateBlockSize_symbolType(
    type_: SymbolEncodingType_e,
    codeTable: *const BYTE,
    nbSeq: usize,
    maxCode: c_uint,
    fseCTable: *const FSE_CTable,
    additionalBits: *const U8,
    defaultNorm: *const i16,
    defaultNormLog: U32,
    defaultMax: U32,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut ctp: *const BYTE = codeTable;
    let ctStart: *const BYTE = ctp;
    let ctEnd: *const BYTE = ctStart.add(nbSeq);
    let mut cSymbolTypeSizeEstimateInBits: usize = 0;
    let mut max: c_uint = maxCode;

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
        let _ = defaultMax;
        cSymbolTypeSizeEstimateInBits =
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, countWksp, max);
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
            cSymbolTypeSizeEstimateInBits += *additionalBits.offset(*ctp as isize) as usize;
        } else {
            /* for offset, offset code is also the number of additional bits */
            cSymbolTypeSizeEstimateInBits += *ctp as usize;
        }
        ctp = ctp.add(1);
    }
    cSymbolTypeSizeEstimateInBits >> 3
}

/* Returns the size estimate for the sequences section (header + content) of a block */
unsafe fn ZSTD_estimateBlockSize_sequences(
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
    let sequencesSectionHeaderSize: usize = 1 /* seqHead */ + 1 /* min seqSize size */
        + (nbSeq >= 128) as usize
        + (nbSeq >= LONGNBSEQ as usize) as usize;
    let mut cSeqSizeEstimate: usize = 0;
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).ofType,
        ofCodeTable,
        nbSeq,
        MaxOff as c_uint,
        (*fseTables).offcodeCTable.as_ptr(),
        core::ptr::null(),
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
        MaxLL as c_uint,
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
        MaxML as c_uint,
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
unsafe fn ZSTD_estimateBlockSize(
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
    let literalsSize: usize = ZSTD_estimateBlockSize_literal(
        literals,
        litSize,
        core::ptr::addr_of!((*entropy).huf),
        core::ptr::addr_of!((*entropyMetadata).hufMetadata),
        workspace,
        wkspSize,
        writeLitEntropy,
    );
    let seqSize: usize = ZSTD_estimateBlockSize_sequences(
        ofCodeTable,
        llCodeTable,
        mlCodeTable,
        nbSeq,
        core::ptr::addr_of!((*entropy).fse),
        core::ptr::addr_of!((*entropyMetadata).fseMetadata),
        workspace,
        wkspSize,
        writeSeqEntropy,
    );
    seqSize + literalsSize + ZSTD_blockHeaderSize
}

/* Builds entropy statistics and uses them for blocksize estimation.
 *
 * @return: estimated compressed size of the seqStore, or a zstd error.
 */
unsafe fn ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(
    seqStore: *mut SeqStore_t,
    zc: *mut ZSTD_CCtx,
) -> usize {
    let entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t =
        core::ptr::addr_of_mut!((*zc).blockSplitCtx.entropyMetadata);
    {
        let err_code = ZSTD_buildBlockEntropyStats(
            seqStore,
            core::ptr::addr_of!((*(*zc).blockState.prevCBlock).entropy),
            core::ptr::addr_of_mut!((*(*zc).blockState.nextCBlock).entropy),
            core::ptr::addr_of!((*zc).appliedParams),
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
        (*seqStore).lit.offset_from((*seqStore).litStart) as usize,
        (*seqStore).ofCode,
        (*seqStore).llCode,
        (*seqStore).mlCode,
        (*seqStore).sequences.offset_from((*seqStore).sequencesStart) as usize,
        core::ptr::addr_of!((*(*zc).blockState.nextCBlock).entropy),
        entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
        ((*entropyMetadata).hufMetadata.hType == set_compressed) as c_int,
        1,
    )
}

/* Returns literals bytes represented in a seqStore */
unsafe fn ZSTD_countSeqStoreLiteralsBytes(seqStore: *const SeqStore_t) -> usize {
    let mut literalsBytes: usize = 0;
    let nbSeqs: usize = (*seqStore)
        .sequences
        .offset_from((*seqStore).sequencesStart) as usize;
    let mut i: usize = 0;
    while i < nbSeqs {
        let seq: SeqDef = *(*seqStore).sequencesStart.add(i);
        literalsBytes += seq.litLength as usize;
        if i == (*seqStore).longLengthPos as usize
            && (*seqStore).longLengthType == ZSTD_llt_literalLength
        {
            literalsBytes += 0x10000;
        }
        i += 1;
    }
    literalsBytes
}

/* Returns match bytes represented in a seqStore */
unsafe fn ZSTD_countSeqStoreMatchBytes(seqStore: *const SeqStore_t) -> usize {
    let mut matchBytes: usize = 0;
    let nbSeqs: usize = (*seqStore)
        .sequences
        .offset_from((*seqStore).sequencesStart) as usize;
    let mut i: usize = 0;
    while i < nbSeqs {
        let seq: SeqDef = *(*seqStore).sequencesStart.add(i);
        matchBytes += seq.mlBase as usize + MINMATCH as usize;
        if i == (*seqStore).longLengthPos as usize
            && (*seqStore).longLengthType == ZSTD_llt_matchLength
        {
            matchBytes += 0x10000;
        }
        i += 1;
    }
    matchBytes
}

/* Derives the seqStore that is a chunk of the originalSeqStore from [startIdx, endIdx).
 * Stores the result in resultSeqStore.
 */
unsafe fn ZSTD_deriveSeqStoreChunk(
    resultSeqStore: *mut SeqStore_t,
    originalSeqStore: *const SeqStore_t,
    startIdx: usize,
    endIdx: usize,
) {
    *resultSeqStore = *originalSeqStore;
    if startIdx > 0 {
        (*resultSeqStore).sequences = (*originalSeqStore).sequencesStart.add(startIdx);
        (*resultSeqStore).litStart = (*resultSeqStore)
            .litStart
            .add(ZSTD_countSeqStoreLiteralsBytes(resultSeqStore));
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
    (*resultSeqStore).sequencesStart = (*originalSeqStore).sequencesStart.add(startIdx);
    (*resultSeqStore).sequences = (*originalSeqStore).sequencesStart.add(endIdx);
    if endIdx
        == (*originalSeqStore)
            .sequences
            .offset_from((*originalSeqStore).sequencesStart) as usize
    {
        /* This accounts for possible last literals if the derived chunk reaches the end of the block */
    } else {
        let literalsBytes: usize = ZSTD_countSeqStoreLiteralsBytes(resultSeqStore);
        (*resultSeqStore).lit = (*resultSeqStore).litStart.add(literalsBytes);
    }
    (*resultSeqStore).llCode = (*resultSeqStore).llCode.add(startIdx);
    (*resultSeqStore).mlCode = (*resultSeqStore).mlCode.add(startIdx);
    (*resultSeqStore).ofCode = (*resultSeqStore).ofCode.add(startIdx);
}

/**
 * Returns the raw offset represented by the combination of offBase, ll0, and repcode history.
 * offBase must represent a repcode in the numeric representation of ZSTD_storeSeq().
 */
unsafe fn ZSTD_resolveRepcodeToRawOffset(rep: *const U32, offBase: U32, ll0: U32) -> U32 {
    /* [ 0 - 3 ] */
    let adjustedRepCode: U32 = OFFBASE_TO_REPCODE(offBase)
        .wrapping_sub(1)
        .wrapping_add(ll0);
    if adjustedRepCode == ZSTD_REP_NUM as U32 {
        /* litlength == 0 and offCode == 2 implies selection of first repcode - 1
         * This is only valid if it results in a valid offset value, aka > 0.
         * Note : it may happen that `rep[0]==1` in exceptional circumstances.
         * In which case this function will return 0, which is an invalid offset.
         * It's not an issue though, since this value will be
         * compared and discarded within ZSTD_seqStore_resolveOffCodes().
         */
        return (*rep.add(0)).wrapping_sub(1);
    }
    *rep.offset(adjustedRepCode as isize)
}

/**
 * ZSTD_seqStore_resolveOffCodes() reconciles any possible divergences in offset history that may arise
 * due to emission of RLE/raw blocks that disturb the offset history,
 * and replaces any repcodes within the seqStore that may be invalid.
 *
 * dRepcodes are updated as would be on the decompression side.
 * cRepcodes are updated exactly in accordance with the seqStore.
 *
 * Note : this function assumes seq->offBase respects the following numbering scheme :
 *        0 : invalid
 *        1-3 : repcode 1-3
 *        4+ : real_offset+3
 */
unsafe fn ZSTD_seqStore_resolveOffCodes(
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
        let seq: *mut SeqDef = (*seqStore).sequencesStart.offset(idx as isize);
        let ll0: U32 = (((*seq).litLength == 0) && (idx != longLitLenIdx)) as U32;
        let offBase: U32 = (*seq).offBase;
        if OFFBASE_IS_REPCODE(offBase) {
            let dRawOffset: U32 =
                ZSTD_resolveRepcodeToRawOffset((*dRepcodes).rep.as_ptr(), offBase, ll0);
            let cRawOffset: U32 =
                ZSTD_resolveRepcodeToRawOffset((*cRepcodes).rep.as_ptr(), offBase, ll0);
            /* Adjust simulated decompression repcode history if we come across a mismatch. Replace
             * the repcode with the offset it actually references, determined by the compression
             * repcode history.
             */
            if dRawOffset != cRawOffset {
                (*seq).offBase = OFFSET_TO_OFFBASE(cRawOffset);
            }
        }
        /* Compression repcode history is always updated with values directly from the unmodified seqStore.
         * Decompression repcode history may use modified seq->offset value taken from compression repcode history.
         */
        ZSTD_updateRep((*dRepcodes).rep.as_mut_ptr(), (*seq).offBase, ll0);
        ZSTD_updateRep((*cRepcodes).rep.as_mut_ptr(), offBase, ll0);
        idx = idx.wrapping_add(1);
    }
}

/* ZSTD_compressSeqStore_singleBlock():
 * Compresses a seqStore into a block with a block header, into the buffer dst.
 *
 * Returns the total size of that block (including header) or a ZSTD error code.
 */
unsafe fn ZSTD_compressSeqStore_singleBlock(
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
    let cSize: usize;
    let mut cSeqsSize: usize;

    /* In case of an RLE or raw block, the simulated decompression repcode history must be reset */
    let dRepOriginal: Repcodes_t = *dRep;
    if isPartition != 0 {
        ZSTD_seqStore_resolveOffCodes(
            dRep,
            cRep,
            seqStore,
            (*seqStore)
                .sequences
                .offset_from((*seqStore).sequencesStart) as U32,
        );
    }

    if dstCapacity < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    cSeqsSize = ZSTD_entropyCompressSeqStore(
        seqStore,
        core::ptr::addr_of!((*(*zc).blockState.prevCBlock).entropy),
        core::ptr::addr_of_mut!((*(*zc).blockState.nextCBlock).entropy),
        core::ptr::addr_of!((*zc).appliedParams),
        op.add(ZSTD_blockHeaderSize) as *mut c_void,
        dstCapacity - ZSTD_blockHeaderSize,
        srcSize,
        (*zc).tmpWorkspace, /* statically allocated in resetCCtx */
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
        /* We don't want to emit our first block as a RLE even if it qualifies because
         * doing so will cause the decoder (cli only) to throw a "should consume all input error."
         * This is only an issue for zstd <= v1.4.3
         */
        cSeqsSize = 1;
    }

    /* Sequence collection not supported when block splitting */
    if (*zc).seqCollector.collectSequences != 0 {
        {
            let err_code = ZSTD_copyBlockSequences(
                core::ptr::addr_of_mut!((*zc).seqCollector),
                seqStore,
                dRepOriginal.rep.as_ptr(),
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        ZSTD_blockState_confirmRepcodesAndEntropyTables(core::ptr::addr_of_mut!((*zc).blockState));
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
        ZSTD_blockState_confirmRepcodesAndEntropyTables(core::ptr::addr_of_mut!((*zc).blockState));
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
#[derive(Copy, Clone)]
pub struct seqStoreSplits {
    pub splitLocations: *mut U32, /* Array of split indices */
    pub idx: usize,               /* The current index within splitLocations being worked on */
}

pub const MIN_SEQUENCES_BLOCK_SPLITTING: usize = 300;

/* Helper function to perform the recursive search for block splits.
 * Estimates the cost of seqStore prior to split, and estimates the cost of splitting the sequences in half.
 * If advantageous to split, then we recurse down the two sub-blocks.
 * If not, or if an error occurred in estimation, then we do not recurse.
 *
 * Note: The recursion depth is capped by a heuristic minimum number of sequences,
 * defined by MIN_SEQUENCES_BLOCK_SPLITTING.
 * In theory, this means the absolute largest recursion depth is 10 == log2(maxNbSeqInBlock/MIN_SEQUENCES_BLOCK_SPLITTING).
 * In practice, recursion depth usually doesn't go beyond 4.
 *
 * Furthermore, the number of splits is capped by ZSTD_MAX_NB_BLOCK_SPLITS.
 * At ZSTD_MAX_NB_BLOCK_SPLITS == 196 with the current existing blockSize
 * maximum of 128 KB, this value is actually impossible to reach.
 */
unsafe fn ZSTD_deriveBlockSplitsHelper(
    splits: *mut seqStoreSplits,
    startIdx: usize,
    endIdx: usize,
    zc: *mut ZSTD_CCtx,
    origSeqStore: *const SeqStore_t,
) {
    let fullSeqStoreChunk: *mut SeqStore_t =
        core::ptr::addr_of_mut!((*zc).blockSplitCtx.fullSeqStoreChunk);
    let firstHalfSeqStore: *mut SeqStore_t =
        core::ptr::addr_of_mut!((*zc).blockSplitCtx.firstHalfSeqStore);
    let secondHalfSeqStore: *mut SeqStore_t =
        core::ptr::addr_of_mut!((*zc).blockSplitCtx.secondHalfSeqStore);
    let estimatedOriginalSize: usize;
    let estimatedFirstHalfSize: usize;
    let estimatedSecondHalfSize: usize;
    let midIdx: usize = (startIdx + endIdx) / 2;

    if endIdx - startIdx < MIN_SEQUENCES_BLOCK_SPLITTING
        || (*splits).idx >= ZSTD_MAX_NB_BLOCK_SPLITS
    {
        return;
    }
    ZSTD_deriveSeqStoreChunk(fullSeqStoreChunk, origSeqStore, startIdx, endIdx);
    ZSTD_deriveSeqStoreChunk(firstHalfSeqStore, origSeqStore, startIdx, midIdx);
    ZSTD_deriveSeqStoreChunk(secondHalfSeqStore, origSeqStore, midIdx, endIdx);
    estimatedOriginalSize = ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(fullSeqStoreChunk, zc);
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
    if estimatedFirstHalfSize + estimatedSecondHalfSize < estimatedOriginalSize {
        ZSTD_deriveBlockSplitsHelper(splits, startIdx, midIdx, zc, origSeqStore);
        *(*splits).splitLocations.add((*splits).idx) = midIdx as U32;
        (*splits).idx += 1;
        ZSTD_deriveBlockSplitsHelper(splits, midIdx, endIdx, zc, origSeqStore);
    }
}

/* Base recursive function.
 * Populates a table with intra-block partition indices that can improve compression ratio.
 *
 * @return: number of splits made (which equals the size of the partition table - 1).
 */
unsafe fn ZSTD_deriveBlockSplits(zc: *mut ZSTD_CCtx, partitions: *mut U32, nbSeq: U32) -> usize {
    let mut splits: seqStoreSplits = seqStoreSplits {
        splitLocations: core::ptr::null_mut(),
        idx: 0,
    };
    splits.splitLocations = partitions;
    splits.idx = 0;
    if nbSeq <= 4 {
        /* Refuse to try and split anything with less than 4 sequences */
        return 0;
    }
    ZSTD_deriveBlockSplitsHelper(
        &mut splits,
        0,
        nbSeq as usize,
        zc,
        core::ptr::addr_of!((*zc).seqStore),
    );
    *splits.splitLocations.add(splits.idx) = nbSeq;
    splits.idx
}

/* ZSTD_compressBlock_splitBlock():
 * Attempts to split a given block into multiple blocks to improve compression ratio.
 *
 * Returns combined size of all blocks (which includes headers), or a ZSTD error code.
 */
unsafe fn ZSTD_compressBlock_splitBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    src: *const c_void,
    blockSize: usize,
    lastBlock: U32,
    nbSeq: U32,
) -> usize {
    let mut cSize: usize = 0;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut i: usize = 0;
    let mut srcBytesTotal: usize = 0;
    /* size == ZSTD_MAX_NB_BLOCK_SPLITS */
    let partitions: *mut U32 = (*zc).blockSplitCtx.partitions.as_mut_ptr();
    let nextSeqStore: *mut SeqStore_t = core::ptr::addr_of_mut!((*zc).blockSplitCtx.nextSeqStore);
    let currSeqStore: *mut SeqStore_t = core::ptr::addr_of_mut!((*zc).blockSplitCtx.currSeqStore);
    let numSplits: usize = ZSTD_deriveBlockSplits(zc, partitions, nbSeq);

    /* If a block is split and some partitions are emitted as RLE/uncompressed, then repcode history
     * may become invalid. In order to reconcile potentially invalid repcodes, we keep track of two
     * separate repcode histories that simulate repcode history on compression and decompression side,
     * and use the histories to determine whether we must replace a particular repcode with its raw offset.
     *
     * 1) cRep gets updated for each partition, regardless of whether the block was emitted as uncompressed
     *    or RLE. This allows us to retrieve the offset value that an invalid repcode references within
     *    a nocompress/RLE block.
     * 2) dRep gets updated only for compressed partitions, and when a repcode gets replaced, will use
     *    the replacement offset value rather than the original repcode to update the repcode history.
     *    dRep also will be the final repcode history sent to the next block.
     *
     * See ZSTD_seqStore_resolveOffCodes() for more details.
     */
    let mut dRep: Repcodes_t = Repcodes_t::default();
    let mut cRep: Repcodes_t = Repcodes_t::default();
    ZSTD_memcpy(
        dRep.rep.as_mut_ptr() as *mut c_void,
        (*(*zc).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    ZSTD_memcpy(
        cRep.rep.as_mut_ptr() as *mut c_void,
        (*(*zc).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    ZSTD_memset(
        nextSeqStore as *mut c_void,
        0,
        core::mem::size_of::<SeqStore_t>(),
    );

    if numSplits == 0 {
        let cSizeSingleBlock: usize = ZSTD_compressSeqStore_singleBlock(
            zc,
            core::ptr::addr_of!((*zc).seqStore),
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
        core::ptr::addr_of!((*zc).seqStore),
        0,
        *partitions.add(0) as usize,
    );
    i = 0;
    while i <= numSplits {
        let cSizeChunk: usize;
        let lastPartition: U32 = (i == numSplits) as U32;
        let mut lastBlockEntireSrc: U32 = 0;

        let mut srcBytes: usize = ZSTD_countSeqStoreLiteralsBytes(currSeqStore)
            + ZSTD_countSeqStoreMatchBytes(currSeqStore);
        srcBytesTotal += srcBytes;
        if lastPartition != 0 {
            /* This is the final partition, need to account for possible last literals */
            srcBytes += blockSize - srcBytesTotal;
            lastBlockEntireSrc = lastBlock;
        } else {
            ZSTD_deriveSeqStoreChunk(
                nextSeqStore,
                core::ptr::addr_of!((*zc).seqStore),
                *partitions.add(i) as usize,
                *partitions.add(i + 1) as usize,
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

        ip = ip.add(srcBytes);
        op = op.add(cSizeChunk);
        dstCapacity -= cSizeChunk;
        cSize += cSizeChunk;
        *currSeqStore = *nextSeqStore;
        i += 1;
    }
    /* cRep and dRep may have diverged during the compression.
     * If so, we use the dRep repcodes for the next block.
     */
    ZSTD_memcpy(
        (*(*zc).blockState.prevCBlock).rep.as_mut_ptr() as *mut c_void,
        dRep.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    cSize
}

unsafe fn ZSTD_compressBlock_splitBlock(
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
        let bss: usize = ZSTD_buildSeqStore(zc, src, srcSize);
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
        nbSeq = (*zc)
            .seqStore
            .sequences
            .offset_from((*zc).seqStore.sequencesStart) as U32;
    }

    cSize = ZSTD_compressBlock_splitBlock_internal(
        zc, dst, dstCapacity, src, srcSize, lastBlock, nbSeq,
    );
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    cSize
}

unsafe fn ZSTD_compressBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    frame: U32,
) -> usize {
    /* This is an estimated upper bound for the length of an rle block.
     * This isn't the actual upper bound.
     * Finding the real threshold needs further investigation.
     */
    let rleMaxLength: U32 = 25;
    let mut cSize: usize = 0;
    let ip: *const BYTE = src as *const BYTE;
    let op: *mut BYTE = dst as *mut BYTE;

    'out: {
        {
            let bss: usize = ZSTD_buildSeqStore(zc, src, srcSize);
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
                    core::ptr::addr_of_mut!((*zc).seqCollector),
                    ZSTD_getSeqStore(zc),
                    (*(*zc).blockState.prevCBlock).rep.as_ptr(),
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            ZSTD_blockState_confirmRepcodesAndEntropyTables(core::ptr::addr_of_mut!(
                (*zc).blockState
            ));
            return 0;
        }

        /* encode sequences and literals */
        cSize = ZSTD_entropyCompressSeqStore(
            core::ptr::addr_of!((*zc).seqStore),
            core::ptr::addr_of!((*(*zc).blockState.prevCBlock).entropy),
            core::ptr::addr_of_mut!((*(*zc).blockState.nextCBlock).entropy),
            core::ptr::addr_of!((*zc).appliedParams),
            dst,
            dstCapacity,
            srcSize,
            (*zc).tmpWorkspace, /* statically allocated in resetCCtx */
            (*zc).tmpWkspSize,
            (*zc).bmi2,
        );

        if frame != 0
            /* We don't want to emit our first block as a RLE even if it qualifies because
             * doing so will cause the decoder (cli only) to throw a "should consume all input error."
             * This is only an issue for zstd <= v1.4.3
             */
            && (*zc).isFirstBlock == 0
            && cSize < rleMaxLength as usize
            && ZSTD_isRLE(ip, srcSize) != 0
        {
            cSize = 1;
            *op.add(0) = *ip.add(0);
        }
    }

    /* out: */
    if ERR_isError(cSize) == 0 && cSize > 1 {
        ZSTD_blockState_confirmRepcodesAndEntropyTables(core::ptr::addr_of_mut!((*zc).blockState));
    }
    /* We check that dictionaries have offset codes available for the first
     * block. After the first block, the offcode table might not have large
     * enough codes to represent the offsets in the data.
     */
    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

unsafe fn ZSTD_compressBlock_targetCBlockSize_body(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    bss: usize,
    lastBlock: U32,
) -> usize {
    if bss == ZSTDbss_compress as usize {
        if
        /* We don't want to emit our first block as a RLE even if it qualifies because
         * doing so will cause the decoder (cli only) to throw a "should consume all input error."
         * This is only an issue for zstd <= v1.4.3
         */
        (*zc).isFirstBlock == 0
            && ZSTD_maybeRLE(core::ptr::addr_of!((*zc).seqStore)) != 0
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
        /* Attempt superblock compression.
         *
         * Note that compressed size of ZSTD_compressSuperBlock() is not bound by the
         * standard ZSTD_compressBound(). This is a problem, because even if we have
         * space now, taking an extra byte now could cause us to run out of space later
         * and violate ZSTD_compressBound().
         *
         * Define blockBound(blockSize) = blockSize + ZSTD_blockHeaderSize.
         *
         * In order to respect ZSTD_compressBound() we must attempt to emit a raw
         * uncompressed block in these cases:
         *   * cSize == 0: Return code for an uncompressed block.
         *   * cSize == dstSize_tooSmall: We may have expanded beyond blockBound(srcSize).
         *     ZSTD_noCompressBlock() will return dstSize_tooSmall if we are really out of
         *     output space.
         *   * cSize >= blockBound(srcSize): We have expanded the block too much so
         *     emit an uncompressed block.
         */
        {
            let cSize: usize =
                ZSTD_compressSuperBlock(zc, dst, dstCapacity, src, srcSize, lastBlock);
            if cSize != ERROR(ZSTD_error_dstSize_tooSmall) {
                let maxCSize: usize =
                    srcSize - ZSTD_minGain(srcSize, (*zc).appliedParams.cParams.strategy);
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
                if cSize != 0 && cSize < maxCSize + ZSTD_blockHeaderSize {
                    ZSTD_blockState_confirmRepcodesAndEntropyTables(core::ptr::addr_of_mut!(
                        (*zc).blockState
                    ));
                    return cSize;
                }
            }
        }
    } /* if (bss == ZSTDbss_compress)*/

    /* Superblock compression failed, attempt to emit a single no compress block.
     * The decoder will be able to stream this block since it is uncompressed.
     */
    ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock)
}

unsafe fn ZSTD_compressBlock_targetCBlockSize(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let mut cSize: usize = 0;
    let bss: usize = ZSTD_buildSeqStore(zc, src, srcSize);
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

unsafe fn ZSTD_overflowCorrectIfNeeded(
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    ip: *const c_void,
    iend: *const c_void,
) {
    let cycleLog: U32 = ZSTD_cycleLog((*params).cParams.chainLog, (*params).cParams.strategy);
    let maxDist: U32 = (1 as U32) << (*params).cParams.windowLog;
    if ZSTD_window_needOverflowCorrection(
        (*ms).window,
        cycleLog,
        maxDist,
        (*ms).loadedDictEnd,
        ip,
        iend,
    ) != 0
    {
        let correction: U32 = ZSTD_window_correctOverflow(
            core::ptr::addr_of_mut!((*ms).window),
            cycleLog,
            maxDist,
            ip,
        );
        ZSTD_cwksp_mark_tables_dirty(ws);
        ZSTD_reduceIndex(ms, params, correction);
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

/* split level based on compression strategy, from `fast` to `btultra2` */
static ZSTD_optimalBlockSize_splitLevels: [c_int; 10] = [0, 0, 1, 2, 2, 3, 3, 4, 4, 4];

unsafe fn ZSTD_optimalBlockSize(
    cctx: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: usize,
    blockSizeMax: usize,
    mut splitLevel: c_int,
    strat: ZSTD_strategy,
    savings: S64,
) -> usize {
    /* note: conservatively only split full blocks (128 KB) currently.
     * While it's possible to go lower, let's keep it simple for a first implementation.
     * Besides, benefits of splitting are reduced when blocks are already small.
     */
    if srcSize < 128 * 1024 || blockSizeMax < 128 * 1024 {
        return MIN(srcSize, blockSizeMax);
    }
    /* do not split incompressible data though:
     * require verified savings to allow pre-splitting.
     * Note: as a consequence, the first full block is not split.
     */
    if savings < 3 {
        return 128 * 1024;
    }
    /* apply @splitLevel, or use default value (which depends on @strat).
     * note that splitting heuristic is still conditioned by @savings >= 3,
     * so the first block will not reach this code path */
    if splitLevel == 1 {
        return 128 * 1024;
    }
    if splitLevel == 0 {
        splitLevel = ZSTD_optimalBlockSize_splitLevels[strat as usize];
    } else {
        splitLevel -= 2;
    }
    ZSTD_splitBlock(
        src,
        blockSizeMax,
        splitLevel,
        (*cctx).tmpWorkspace,
        (*cctx).tmpWkspSize,
    )
}

/* ZSTD_compress_frameChunk() :
*   Compress a chunk of data into one or multiple blocks.
*   All blocks will be terminated, all input will be consumed.
*   Function will issue an error if there is not enough `dstCapacity` to hold the compressed content.
*   Frame is supposed already started (header already produced)
*  @return : compressed size, or an error code
*/
unsafe fn ZSTD_compress_frameChunk(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastFrameChunk: U32,
) -> usize {
    let blockSizeMax: usize = (*cctx).blockSizeMax;
    let mut remaining: usize = srcSize;
    let mut ip: *const BYTE = src as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let maxDist: U32 = (1 as U32) << (*cctx).appliedParams.cParams.windowLog;
    let mut savings: S64 = ((*cctx).consumedSrcSize as S64) - ((*cctx).producedCSize as S64);

    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        ZSTD_XXH64_update(core::ptr::addr_of_mut!((*cctx).xxhState), src, srcSize);
    }

    while remaining != 0 {
        let ms: *mut ZSTD_MatchState_t = core::ptr::addr_of_mut!((*cctx).blockState.matchState);
        let blockSize: usize = ZSTD_optimalBlockSize(
            cctx,
            ip as *const c_void,
            remaining,
            blockSizeMax,
            (*cctx).appliedParams.preBlockSplitter_level,
            (*cctx).appliedParams.cParams.strategy,
            savings,
        );
        let lastBlock: U32 = lastFrameChunk & ((blockSize == remaining) as U32);

        /* TODO: See 3090. We reduced MIN_CBLOCK_SIZE from 3 to 2 so to compensate we are adding
         * additional 1. We need to revisit and change this logic to be more consistent */
        if dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        ZSTD_overflowCorrectIfNeeded(
            ms,
            core::ptr::addr_of_mut!((*cctx).workspace),
            core::ptr::addr_of!((*cctx).appliedParams),
            ip as *const c_void,
            ip.add(blockSize) as *const c_void,
        );
        ZSTD_checkDictValidity(
            core::ptr::addr_of!((*ms).window),
            ip.add(blockSize) as *const c_void,
            maxDist,
            core::ptr::addr_of_mut!((*ms).loadedDictEnd),
            core::ptr::addr_of_mut!((*ms).dictMatchState),
        );
        ZSTD_window_enforceMaxDist(
            core::ptr::addr_of_mut!((*ms).window),
            ip as *const c_void,
            maxDist,
            core::ptr::addr_of_mut!((*ms).loadedDictEnd),
            core::ptr::addr_of_mut!((*ms).dictMatchState),
        );

        /* Ensure hash/chain table insertion resumes no sooner than lowlimit */
        if (*ms).nextToUpdate < (*ms).window.lowLimit {
            (*ms).nextToUpdate = (*ms).window.lowLimit;
        }

        {
            let mut cSize: usize;
            if ZSTD_useTargetCBlockSize(core::ptr::addr_of!((*cctx).appliedParams)) != 0 {
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
            } else if ZSTD_blockSplitterEnabled(core::ptr::addr_of_mut!((*cctx).appliedParams)) != 0
            {
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
                    op.add(ZSTD_blockHeaderSize) as *mut c_void,
                    dstCapacity - ZSTD_blockHeaderSize,
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
                    MEM_writeLE24(op as *mut c_void, cBlockHeader);
                    cSize += ZSTD_blockHeaderSize;
                }
            } /* if (ZSTD_useTargetCBlockSize(&cctx->appliedParams))*/

            /* @savings is employed to ensure that splitting doesn't worsen expansion of incompressible data.
             * Without splitting, the maximum expansion is 3 bytes per full block.
             * An adversarial input could attempt to fudge the split detector,
             * and make it split incompressible data, resulting in more block headers.
             * Note that, since ZSTD_COMPRESSBOUND() assumes a worst case scenario of 1KB per block,
             * and the splitter never creates blocks that small (current lower limit is 8 KB),
             * there is already no risk to expand beyond ZSTD_COMPRESSBOUND() limit.
             * But if the goal is to not expand by more than 3-bytes per 128 KB full block,
             * then yes, it becomes possible to make the block splitter oversplit incompressible data.
             * Using @savings, we enforce an even more conservative condition,
             * requiring the presence of enough savings (at least 3 bytes) to authorize splitting,
             * otherwise only full blocks are used.
             * But being conservative is fine,
             * since splitting barely compressible blocks is not fruitful anyway */
            savings += (blockSize as S64) - (cSize as S64);

            ip = ip.add(blockSize);
            remaining -= blockSize;
            op = op.add(cSize);
            dstCapacity -= cSize;
            (*cctx).isFirstBlock = 0;
        }
    }

    if lastFrameChunk != 0 && op > ostart {
        (*cctx).stage = ZSTDcs_ending;
    }
    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_writeFrameHeader(
    dst: *mut c_void,
    dstCapacity: usize,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    dictID: U32,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    /* 0-3 */
    let dictIDSizeCodeLength: U32 = (dictID > 0) as U32
        + (dictID >= 256) as U32
        + (dictID >= 65536) as U32;
    /* 0-3 */
    let dictIDSizeCode: U32 = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        dictIDSizeCodeLength
    };
    let checksumFlag: U32 = ((*params).fParams.checksumFlag > 0) as U32;
    let windowSize: U32 = (1 as U32) << (*params).cParams.windowLog;
    let singleSegment: U32 = ((*params).fParams.contentSizeFlag != 0
        && (windowSize as U64) >= pledgedSrcSize) as U32;
    let windowLogByte: BYTE =
        (((*params).cParams.windowLog.wrapping_sub(ZSTD_WINDOWLOG_ABSOLUTEMIN)) << 3) as BYTE;
    /* 0-3 */
    let fcsCode: U32 = if (*params).fParams.contentSizeFlag != 0 {
        (pledgedSrcSize >= 256) as U32
            + (pledgedSrcSize >= 65536 + 256) as U32
            + (pledgedSrcSize >= 0xFFFFFFFFu64) as U32
    } else {
        0
    };
    let frameHeaderDescriptionByte: BYTE = (dictIDSizeCode
        .wrapping_add(checksumFlag << 2)
        .wrapping_add(singleSegment << 5)
        .wrapping_add(fcsCode << 6)) as BYTE;
    let mut pos: usize = 0;

    if dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (*params).format == ZSTD_f_zstd1 {
        MEM_writeLE32(dst, ZSTD_MAGICNUMBER);
        pos = 4;
    }
    *op.add(pos) = frameHeaderDescriptionByte;
    pos += 1;
    if singleSegment == 0 {
        *op.add(pos) = windowLogByte;
        pos += 1;
    }
    match dictIDSizeCode {
        1 => {
            *op.add(pos) = dictID as BYTE;
            pos += 1;
        }
        2 => {
            MEM_writeLE16(op.add(pos) as *mut c_void, dictID as U16);
            pos += 2;
        }
        3 => {
            MEM_writeLE32(op.add(pos) as *mut c_void, dictID);
            pos += 4;
        }
        _ => {}
    }
    match fcsCode {
        1 => {
            MEM_writeLE16(
                op.add(pos) as *mut c_void,
                pledgedSrcSize.wrapping_sub(256) as U16,
            );
            pos += 2;
        }
        2 => {
            MEM_writeLE32(op.add(pos) as *mut c_void, pledgedSrcSize as U32);
            pos += 4;
        }
        3 => {
            MEM_writeLE64(op.add(pos) as *mut c_void, pledgedSrcSize as U64);
            pos += 8;
        }
        _ => {
            if singleSegment != 0 {
                *op.add(pos) = pledgedSrcSize as BYTE;
                pos += 1;
            }
        }
    }
    pos
}

/* ZSTD_writeSkippableFrame_advanced() :
 * Writes out a skippable frame with the specified magic number variant (16 are supported),
 * from ZSTD_MAGIC_SKIPPABLE_START to ZSTD_MAGIC_SKIPPABLE_START+15, and the desired source data.
 *
 * Returns the total number of bytes written, or a ZSTD error code.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    magicVariant: c_uint,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    /* Skippable frame overhead */
    if dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0xFFFFFFFFu32 as usize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if magicVariant > 15 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }

    MEM_writeLE32(
        op as *mut c_void,
        ZSTD_MAGIC_SKIPPABLE_START.wrapping_add(magicVariant as U32),
    );
    MEM_writeLE32(op.add(4) as *mut c_void, srcSize as U32);
    ZSTD_memcpy(op.add(8) as *mut c_void, src, srcSize);
    srcSize + ZSTD_SKIPPABLEHEADERSIZE
}

/* ZSTD_writeLastEmptyBlock() :
 * output an empty Block with end-of-frame mark to complete a frame
 * @return : size of data written into `dst` (== ZSTD_blockHeaderSize (defined in zstd_internal.h))
 *           or an error code if `dstCapacity` is too small (<ZSTD_blockHeaderSize)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeLastEmptyBlock(dst: *mut c_void, dstCapacity: usize) -> usize {
    if dstCapacity < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        /* 0 size */
        let cBlockHeader24: U32 = 1 /*lastBlock*/ + ((bt_raw as U32) << 1);
        MEM_writeLE24(dst, cBlockHeader24);
        ZSTD_blockHeaderSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_referenceExternalSequences(
    cctx: *mut ZSTD_CCtx,
    seq: *mut rawSeq,
    nbSeq: usize,
) {
    (*cctx).externSeqStore.seq = seq;
    (*cctx).externSeqStore.size = nbSeq;
    (*cctx).externSeqStore.capacity = nbSeq;
    (*cctx).externSeqStore.pos = 0;
    (*cctx).externSeqStore.posInSequence = 0;
}
