/* ===========================================================================
 * zstd_compress.c — part 3  (C lines 2601..3630)
 *
 * Index rescaling, sequence-code building, entropy compression of the
 * seqStore, block-compressor selection, seqStore construction, and the
 * sequence-extraction API (ZSTD_generateSequences & friends).
 *
 * This file is `include!`d by `zstd_compress.rs`: it must contain items only,
 * no `use` statements and no `extern "C"` blocks.
 * ======================================================================== */

pub const ZSTD_ROWSIZE: c_int = 16;

/* ZSTD_reduceTable() :
 *  reduce table indexes by `reducerValue`, or squash to zero.
 *  PreserveMark preserves "unsorted mark" for btlazy2 strategy.
 *  It must be set to a clear 0/1 value, to remove branch during inlining.
 *  Presume table size is a multiple of ZSTD_ROWSIZE
 *  to help auto-vectorization */
#[inline(always)]
unsafe fn ZSTD_reduceTable_internal(
    table: *mut U32,
    size: U32,
    reducerValue: U32,
    preserveMark: c_int,
) {
    let nbRows: c_int = (size as c_int) / ZSTD_ROWSIZE;
    let mut cellNb: c_int = 0;
    let mut rowNb: c_int;
    /* Protect special index values < ZSTD_WINDOW_START_INDEX. */
    let reducerThreshold: U32 = reducerValue.wrapping_add(ZSTD_WINDOW_START_INDEX);

    rowNb = 0;
    while rowNb < nbRows {
        let mut column: c_int = 0;
        while column < ZSTD_ROWSIZE {
            let newVal: U32;
            if preserveMark != 0 && *table.offset(cellNb as isize) == ZSTD_DUBT_UNSORTED_MARK {
                /* This write is pointless, but is required(?) for the compiler
                 * to auto-vectorize the loop. */
                newVal = ZSTD_DUBT_UNSORTED_MARK;
            } else if *table.offset(cellNb as isize) < reducerThreshold {
                newVal = 0;
            } else {
                newVal = (*table.offset(cellNb as isize)).wrapping_sub(reducerValue);
            }
            *table.offset(cellNb as isize) = newVal;
            cellNb += 1;
            column += 1;
        }
        rowNb += 1;
    }
}

unsafe fn ZSTD_reduceTable(table: *mut U32, size: U32, reducerValue: U32) {
    ZSTD_reduceTable_internal(table, size, reducerValue, 0);
}

unsafe fn ZSTD_reduceTable_btlazy2(table: *mut U32, size: U32, reducerValue: U32) {
    ZSTD_reduceTable_internal(table, size, reducerValue, 1);
}

/* ZSTD_reduceIndex() :
 *   rescale all indexes to avoid future overflow (indexes are U32) */
unsafe fn ZSTD_reduceIndex(
    ms: *mut ZSTD_MatchState_t,
    params: *const ZSTD_CCtx_params,
    reducerValue: U32,
) {
    {
        let hSize: U32 = (1 as U32) << (*params).cParams.hashLog;
        ZSTD_reduceTable((*ms).hashTable, hSize, reducerValue);
    }

    if ZSTD_allocateChainTable(
        (*params).cParams.strategy,
        (*params).useRowMatchFinder,
        (*ms).dedicatedDictSearch as U32,
    ) != 0
    {
        let chainSize: U32 = (1 as U32) << (*params).cParams.chainLog;
        if (*params).cParams.strategy == ZSTD_btlazy2 {
            ZSTD_reduceTable_btlazy2((*ms).chainTable, chainSize, reducerValue);
        } else {
            ZSTD_reduceTable((*ms).chainTable, chainSize, reducerValue);
        }
    }

    if (*ms).hashLog3 != 0 {
        let h3Size: U32 = (1 as U32) << (*ms).hashLog3;
        ZSTD_reduceTable((*ms).hashTable3, h3Size, reducerValue);
    }
}

/*-*******************************************************
*  Block entropic compression
*********************************************************/

/* See doc/zstd_compression_format.md for detailed format description */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int {
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let llCodeTable: *mut BYTE = (*seqStorePtr).llCode;
    let ofCodeTable: *mut BYTE = (*seqStorePtr).ofCode;
    let mlCodeTable: *mut BYTE = (*seqStorePtr).mlCode;
    let nbSeq: U32 = (*seqStorePtr)
        .sequences
        .offset_from((*seqStorePtr).sequencesStart) as U32;
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

/* ZSTD_useTargetCBlockSize():
 * Returns if target compressed block size param is being used.
 * If used, compression will do best effort to make a compressed block size to be around targetCBlockSize.
 * Returns 1 if true, 0 otherwise. */
unsafe fn ZSTD_useTargetCBlockSize(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).targetCBlockSize != 0) as c_int
}

/* ZSTD_blockSplitterEnabled():
 * Returns if block splitting param is being used
 * If used, compression will do best effort to split a block in order to improve compression ratio.
 * At the time this function is called, the parameter must be finalized.
 * Returns 1 if true, 0 otherwise. */
unsafe fn ZSTD_blockSplitterEnabled(cctxParams: *mut ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).postBlockSplitter == ZSTD_ps_enable) as c_int
}

/* Type returned by ZSTD_buildSequencesStatistics containing finalized symbol encoding types
 * and size of the sequences statistics
 */
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_symbolEncodingTypeStats_t {
    pub LLtype: U32,
    pub Offtype: U32,
    pub MLtype: U32,
    pub size: usize,
    /* Accounts for bug in 1.3.4. More detail in ZSTD_entropyCompressSeqStore_internal() */
    pub lastCountSize: usize,
    pub longOffsets: c_int,
}

/* ZSTD_buildSequencesStatistics():
 * Returns a ZSTD_symbolEncodingTypeStats_t, or a zstd error code in the `size` field.
 * Modifies `nextEntropy` to have the appropriate values as a side effect.
 * nbSeq must be greater than 0.
 *
 * entropyWkspSize must be of size at least ENTROPY_WORKSPACE_SIZE - (MaxSeq + 1)*sizeof(U32)
 */
unsafe fn ZSTD_buildSequencesStatistics(
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
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).matchlengthCTable.as_mut_ptr();
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let mut stats: ZSTD_symbolEncodingTypeStats_t = ZSTD_symbolEncodingTypeStats_t::default();

    stats.lastCountSize = 0;
    /* convert length/distances into codes */
    stats.longOffsets = ZSTD_seqToCodes(seqStorePtr);
    /* build CTable for Literal Lengths */
    {
        let mut max: c_uint = MaxLL as c_uint;
        let mostFrequent: usize = HIST_countFast_wksp(
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
            let countSize: usize = ZSTD_buildCTable(
                op as *mut c_void,
                oend.offset_from(op) as usize,
                CTable_LitLength,
                LLFSELog as U32,
                stats.LLtype as SymbolEncodingType_e,
                countWorkspace,
                max as U32,
                llCodeTable,
                nbSeq,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                MaxLL as U32,
                (*prevEntropy).litlengthCTable.as_ptr(),
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
            op = op.add(countSize);
        }
    }
    /* build CTable for Offsets */
    {
        let mut max: c_uint = MaxOff as c_uint;
        let mostFrequent: usize = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            ofCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
        /* We can only use the basic table if max <= DefaultMaxOff, otherwise the offsets are too large */
        let defaultPolicy: ZSTD_DefaultPolicy_e = if max <= DefaultMaxOff as c_uint {
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
            let countSize: usize = ZSTD_buildCTable(
                op as *mut c_void,
                oend.offset_from(op) as usize,
                CTable_OffsetBits,
                OffFSELog as U32,
                stats.Offtype as SymbolEncodingType_e,
                countWorkspace,
                max as U32,
                ofCodeTable,
                nbSeq,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                DefaultMaxOff as U32,
                (*prevEntropy).offcodeCTable.as_ptr(),
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
            op = op.add(countSize);
        }
    }
    /* build CTable for MatchLengths */
    {
        let mut max: c_uint = MaxML as c_uint;
        let mostFrequent: usize = HIST_countFast_wksp(
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
            let countSize: usize = ZSTD_buildCTable(
                op as *mut c_void,
                oend.offset_from(op) as usize,
                CTable_MatchLength,
                MLFSELog as U32,
                stats.MLtype as SymbolEncodingType_e,
                countWorkspace,
                max as U32,
                mlCodeTable,
                nbSeq,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                MaxML as U32,
                (*prevEntropy).matchlengthCTable.as_ptr(),
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
            op = op.add(countSize);
        }
    }
    stats.size = op.offset_from(ostart) as usize;
    stats
}

/* ZSTD_entropyCompressSeqStore_internal():
 * compresses both literals and sequences
 * Returns compressed size of block, or a zstd error.
 */
pub const SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO: usize = 20;

unsafe fn ZSTD_entropyCompressSeqStore_internal(
    dst: *mut c_void,
    dstCapacity: usize,
    literals: *const c_void,
    litSize: usize,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    mut entropyWorkspace: *mut c_void,
    mut entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let count: *mut c_uint = entropyWorkspace as *mut c_uint;
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).fse.litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).fse.offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).fse.matchlengthCTable.as_mut_ptr();
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let nbSeq: usize = (*seqStorePtr)
        .sequences
        .offset_from((*seqStorePtr).sequencesStart) as usize;
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let lastCountSize: usize;
    let mut longOffsets: c_int = 0;

    entropyWorkspace = count.add(MaxSeq as usize + 1) as *mut c_void;
    entropyWkspSize -= (MaxSeq as usize + 1) * core::mem::size_of::<c_uint>();

    /* Compress literals */
    {
        let numSequences: usize = (*seqStorePtr)
            .sequences
            .offset_from((*seqStorePtr).sequencesStart) as usize;
        /* Base suspicion of uncompressibility on ratio of literals to sequences */
        let suspectUncompressible: c_int = ((numSequences == 0)
            || (litSize / numSequences >= SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO))
            as c_int;

        let cSize: usize = ZSTD_compressLiterals(
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
        op = op.add(cSize);
    }

    /* Sequences Header */
    if oend.offset_from(op) < (3 /*max nbSeq Size*/ + 1 /*seqHead*/) {
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
        MEM_writeLE16(
            op.add(1) as *mut c_void,
            (nbSeq - LONGNBSEQ as usize) as U16,
        );
        op = op.add(3);
    }
    if nbSeq == 0 {
        /* Copy the old tables over as if we repeated them */
        ZSTD_memcpy(
            &mut (*nextEntropy).fse as *mut ZSTD_fseCTables_t as *mut c_void,
            &(*prevEntropy).fse as *const ZSTD_fseCTables_t as *const c_void,
            core::mem::size_of_val(&(*prevEntropy).fse),
        );
        return op.offset_from(ostart) as usize;
    }
    {
        let seqHead: *mut BYTE = op;
        op = op.add(1);
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
        op = op.add(stats.size);
        longOffsets = stats.longOffsets;
    }

    {
        let bitstreamSize: usize = ZSTD_encodeSequences(
            op as *mut c_void,
            oend.offset_from(op) as usize,
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
        op = op.add(bitstreamSize);
        /* zstd versions <= 1.3.4 mistakenly report corruption when
         * FSE_readNCount() receives a buffer < 4 bytes.
         * Fixed by https://github.com/facebook/zstd/pull/1146.
         * This can happen when the last set_compressed table present is 2
         * bytes and the bitstream is only one byte.
         * In this exceedingly rare case, we will simply emit an uncompressed
         * block, since it isn't worth optimizing.
         */
        if lastCountSize != 0 && (lastCountSize + bitstreamSize) < 4 {
            /* lastCountSize >= 2 && bitstreamSize > 0 ==> lastCountSize == 3 */
            return 0;
        }
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_entropyCompressSeqStore_wExtLitBuffer(
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
    let cSize: usize = ZSTD_entropyCompressSeqStore_internal(
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
    /* When srcSize <= dstCapacity, there is enough space to write a raw uncompressed block.
     * Since we ran out of space, block must be not compressible, so fall back to raw uncompressed block.
     */
    if ((cSize == ERROR(ZSTD_error_dstSize_tooSmall)) as c_int
        & ((blockSize <= dstCapacity) as c_int))
        != 0
    {
        return 0; /* block not compressed */
    }
    if ERR_isError(cSize) != 0 {
        return cSize;
    }

    /* Check compressibility */
    {
        let maxCSize: usize = blockSize - ZSTD_minGain(blockSize, (*cctxParams).cParams.strategy);
        if cSize >= maxCSize {
            return 0; /* block not compressed */
        }
    }
    /* libzstd decoder before  > v1.5.4 is not compatible with compressed blocks of size ZSTD_BLOCKSIZE_MAX exactly.
     * This restriction is indirectly already fulfilled by respecting ZSTD_minGain() condition above.
     */
    cSize
}

unsafe fn ZSTD_entropyCompressSeqStore(
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
        (*seqStorePtr).lit.offset_from((*seqStorePtr).litStart) as usize,
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

unsafe fn ZSTD_storeLastLiterals(
    seqStorePtr: *mut SeqStore_t,
    anchor: *const BYTE,
    lastLLSize: usize,
) {
    ZSTD_memcpy(
        (*seqStorePtr).lit as *mut c_void,
        anchor as *const c_void,
        lastLLSize,
    );
    (*seqStorePtr).lit = (*seqStorePtr).lit.add(lastLLSize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetSeqStore(ssPtr: *mut SeqStore_t) {
    (*ssPtr).lit = (*ssPtr).litStart;
    (*ssPtr).sequences = (*ssPtr).sequencesStart;
    (*ssPtr).longLengthType = ZSTD_llt_none;
}

/* ZSTD_postProcessSequenceProducerResult() :
 * Validates and post-processes sequences obtained through the external matchfinder API:
 *   - Checks whether nbExternalSeqs represents an error condition.
 *   - Appends a block delimiter to outSeqs if one is not already present.
 *     See zstd.h for context regarding block delimiters.
 * Returns the number of sequences after post-processing, or an error code. */
unsafe fn ZSTD_postProcessSequenceProducerResult(
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
            &mut *outSeqs.add(0) as *mut ZSTD_Sequence as *mut c_void,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        return 1;
    }

    {
        let lastSeq: ZSTD_Sequence = *outSeqs.add(nbExternalSeqs - 1);

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
            &mut *outSeqs.add(nbExternalSeqs) as *mut ZSTD_Sequence as *mut c_void,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        nbExternalSeqs + 1
    }
}

/* ZSTD_fastSequenceLengthSum() :
 * Returns sum(litLen) + sum(matchLen) + lastLits for *seqBuf*.
 * Similar to another function in zstd_compress.c (determine_blockSize),
 * except it doesn't check for a block delimiter to end summation.
 * Removing the early exit allows the compiler to auto-vectorize.
 * This function can be deleted and replaced by determine_blockSize after we resolve issue #3456. */
unsafe fn ZSTD_fastSequenceLengthSum(seqBuf: *const ZSTD_Sequence, seqBufSize: usize) -> usize {
    let mut matchLenSum: usize;
    let mut litLenSum: usize;
    let mut i: usize;
    matchLenSum = 0;
    litLenSum = 0;
    i = 0;
    while i < seqBufSize {
        litLenSum += (*seqBuf.add(i)).litLength as usize;
        matchLenSum += (*seqBuf.add(i)).matchLength as usize;
        i += 1;
    }
    litLenSum + matchLenSum
}

/**
 * Function to validate sequences produced by a block compressor.
 */
unsafe fn ZSTD_validateSeqStore(
    seqStore: *const SeqStore_t,
    cParams: *const ZSTD_compressionParameters,
) {
    /* DEBUGLEVEL == 0: nothing to do. */
}

/* ZSTD_BuildSeqStore_e */
pub type ZSTD_BuildSeqStore_e = c_int;
pub const ZSTDbss_compress: ZSTD_BuildSeqStore_e = 0;
pub const ZSTDbss_noCompress: ZSTD_BuildSeqStore_e = 1;

unsafe fn ZSTD_buildSeqStore(zc: *mut ZSTD_CCtx, src: *const c_void, srcSize: usize) -> usize {
    let ms: *mut ZSTD_MatchState_t = &mut (*zc).blockState.matchState;
    /* Assert that we have correctly flushed the ctx params into the ms's copy
     * (ZSTD_assertEqualCParams is all asserts => compiled out). */
    /* TODO: See 3090. We reduced MIN_CBLOCK_SIZE from 3 to 2 so to compensate we are adding
     * additional 1. We need to revisit and change this logic to be more consistent */
    if srcSize < MIN_CBLOCK_SIZE + ZSTD_blockHeaderSize + 1 + 1 {
        if (*zc).appliedParams.cParams.strategy >= ZSTD_btopt {
            ZSTD_ldm_skipRawSeqStoreBytes(&mut (*zc).externSeqStore, srcSize);
        } else {
            ZSTD_ldm_skipSequences(
                &mut (*zc).externSeqStore,
                srcSize,
                (*zc).appliedParams.cParams.minMatch as U32,
            );
        }
        return ZSTDbss_noCompress as usize; /* don't even attempt compression below a certain srcSize */
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
        if curr > (*ms).nextToUpdate.wrapping_add(384) {
            (*ms).nextToUpdate = curr.wrapping_sub(MIN(
                192,
                curr.wrapping_sub((*ms).nextToUpdate).wrapping_sub(384),
            ));
        }
    }

    /* select and store sequences */
    {
        let dictMode: ZSTD_dictMode_e = ZSTD_matchState_dictMode(ms);
        let lastLLSize: usize;
        {
            let mut i: c_int = 0;
            while i < ZSTD_REP_NUM as c_int {
                (*(*zc).blockState.nextCBlock).rep[i as usize] =
                    (*(*zc).blockState.prevCBlock).rep[i as usize];
                i += 1;
            }
        }
        if (*zc).externSeqStore.pos < (*zc).externSeqStore.size {
            /* External matchfinder + LDM is technically possible, just not implemented yet.
             * We need to revisit soon and implement it. */
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

            /* External matchfinder + LDM is technically possible, just not implemented yet.
             * We need to revisit soon and implement it. */
            if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            ldmSeqStore.seq = (*zc).ldmSequences;
            ldmSeqStore.capacity = (*zc).maxNbLdmSequences;
            /* Updates ldmSeqStore.size */
            {
                let err_code = ZSTD_ldm_generateSequences(
                    &mut (*zc).ldmState,
                    &mut ldmSeqStore,
                    &(*zc).appliedParams.ldmParams,
                    src,
                    srcSize,
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
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
                let windowSize: U32 = (1 as U32) << (*zc).appliedParams.cParams.windowLog;

                let nbExternalSeqs: usize =
                    ((*zc).appliedParams.extSeqProdFunc.unwrap())(
                        (*zc).appliedParams.extSeqProdState,
                        (*zc).extSeqBuf,
                        (*zc).extSeqBufCapacity,
                        src,
                        srcSize,
                        core::ptr::null(), /* dict, currently not supported */
                        0,                 /* dictSize, currently not supported */
                        (*zc).appliedParams.compressionLevel,
                        windowSize as usize,
                    );

                let nbPostProcessedSeqs: usize = ZSTD_postProcessSequenceProducerResult(
                    (*zc).extSeqBuf,
                    nbExternalSeqs,
                    (*zc).extSeqBufCapacity,
                    srcSize,
                );

                /* Return early if there is no error, since we don't need to worry about last literals */
                if ERR_isError(nbPostProcessedSeqs) == 0 {
                    let mut seqPos: ZSTD_SequencePosition = ZSTD_SequencePosition {
                        idx: 0,
                        posInSequence: 0,
                        posInSrc: 0,
                    };
                    let seqLenSum: usize =
                        ZSTD_fastSequenceLengthSum((*zc).extSeqBuf, nbPostProcessedSeqs);
                    if seqLenSum > srcSize {
                        return ERROR(ZSTD_error_externalSequences_invalid);
                    }
                    {
                        let err_code = ZSTD_transferSequences_wBlockDelim(
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
            (*ms).ldmSeqStore = core::ptr::null();
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
                (src as *const BYTE).add(srcSize).wrapping_sub(lastLLSize);
            ZSTD_storeLastLiterals(&mut (*zc).seqStore, lastLiterals, lastLLSize);
        }
    }
    ZSTD_validateSeqStore(&(*zc).seqStore, &(*zc).appliedParams.cParams);
    ZSTDbss_compress as usize
}

unsafe fn ZSTD_copyBlockSequences(
    seqCollector: *mut SeqCollector,
    seqStore: *const SeqStore_t,
    prevRepcodes: *const U32,
) -> usize {
    let inSeqs: *const SeqDef = (*seqStore).sequencesStart;
    let nbInSequences: usize = (*seqStore).sequences.offset_from(inSeqs) as usize;
    let nbInLiterals: usize = (*seqStore).lit.offset_from((*seqStore).litStart) as usize;

    let outSeqs: *mut ZSTD_Sequence = if (*seqCollector).seqIndex == 0 {
        (*seqCollector).seqStart
    } else {
        (*seqCollector).seqStart.add((*seqCollector).seqIndex)
    };
    let nbOutSequences: usize = nbInSequences + 1;
    let mut nbOutLiterals: usize = 0;
    let mut repcodes: Repcodes_t = Repcodes_t { rep: [0; 3] };
    let mut i: usize;

    /* Bounds check that we have enough space for every input sequence
     * and the block delimiter
     */
    if nbOutSequences > ((*seqCollector).maxSequences - (*seqCollector).seqIndex) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    ZSTD_memcpy(
        &mut repcodes as *mut Repcodes_t as *mut c_void,
        prevRepcodes as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    i = 0;
    while i < nbInSequences {
        let rawOffset: U32;
        (*outSeqs.add(i)).litLength = (*inSeqs.add(i)).litLength as c_uint;
        (*outSeqs.add(i)).matchLength = (*inSeqs.add(i)).mlBase as c_uint + MINMATCH as c_uint;
        (*outSeqs.add(i)).rep = 0;

        /* Handle the possible single length >= 64K
         * There can only be one because we add MINMATCH to every match length,
         * and blocks are at most 128K.
         */
        if i == (*seqStore).longLengthPos as usize {
            if (*seqStore).longLengthType == ZSTD_llt_literalLength {
                (*outSeqs.add(i)).litLength += 0x10000;
            } else if (*seqStore).longLengthType == ZSTD_llt_matchLength {
                (*outSeqs.add(i)).matchLength += 0x10000;
            }
        }

        /* Determine the raw offset given the offBase, which may be a repcode. */
        if OFFBASE_IS_REPCODE((*inSeqs.add(i)).offBase) {
            let repcode: U32 = OFFBASE_TO_REPCODE((*inSeqs.add(i)).offBase);
            (*outSeqs.add(i)).rep = repcode as c_uint;
            if (*outSeqs.add(i)).litLength != 0 {
                rawOffset = repcodes.rep[(repcode - 1) as usize];
            } else {
                if repcode == 3 {
                    rawOffset = repcodes.rep[0] - 1;
                } else {
                    rawOffset = repcodes.rep[repcode as usize];
                }
            }
        } else {
            rawOffset = OFFBASE_TO_OFFSET((*inSeqs.add(i)).offBase);
        }
        (*outSeqs.add(i)).offset = rawOffset as c_uint;

        /* Update repcode history for the sequence */
        ZSTD_updateRep(
            repcodes.rep.as_mut_ptr(),
            (*inSeqs.add(i)).offBase,
            ((*inSeqs.add(i)).litLength == 0) as U32,
        );

        nbOutLiterals += (*outSeqs.add(i)).litLength as usize;
        i += 1;
    }
    /* Insert last literals (if any exist) in the block as a sequence with ml == off == 0.
     * If there are no last literals, then we'll emit (of: 0, ml: 0, ll: 0), which is a marker
     * for the block boundary, according to the API.
     */
    {
        let lastLLSize: usize = nbInLiterals - nbOutLiterals;
        (*outSeqs.add(nbInSequences)).litLength = lastLLSize as U32 as c_uint;
        (*outSeqs.add(nbInSequences)).matchLength = 0;
        (*outSeqs.add(nbInSequences)).offset = 0;
    }
    (*seqCollector).seqIndex += nbOutSequences;

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
    let dstCapacity: usize = ZSTD_compressBound(srcSize);
    let dst: *mut c_void; /* Make C90 happy. */
    let mut seqCollector: SeqCollector = SeqCollector::default();
    {
        let mut targetCBlockSize: c_int = 0;
        {
            let err_code =
                ZSTD_CCtx_getParameter(zc, ZSTD_c_targetCBlockSize, &mut targetCBlockSize);
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
            let err_code = ZSTD_CCtx_getParameter(zc, ZSTD_c_nbWorkers, &mut nbWorkers);
            if ERR_isError(err_code) != 0 {
                return err_code;
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
        let ret: usize = ZSTD_compress2(zc, dst, dstCapacity, src, srcSize);
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
    seqsSize: usize,
) -> usize {
    let mut in_: usize = 0;
    let mut out: usize = 0;
    while in_ < seqsSize {
        if (*sequences.add(in_)).offset == 0 && (*sequences.add(in_)).matchLength == 0 {
            if in_ != seqsSize - 1 {
                /* C: `sequences[in+1].litLength += sequences[in].litLength;` on
                 * two `unsigned` fields. `ZSTD_mergeBlockDelimiters` accepts any
                 * caller-supplied array, so a pair of literal lengths summing
                 * past 2^32 is a real input and the C wraps it. Spelled out so
                 * the result does not depend on `overflow-checks`. */
                (*sequences.add(in_ + 1)).litLength = (*sequences.add(in_ + 1))
                    .litLength
                    .wrapping_add((*sequences.add(in_)).litLength);
            }
        } else {
            *sequences.add(out) = *sequences.add(in_);
            out += 1;
        }
        in_ += 1;
    }
    out
}

/* Unrolled loop to read four size_ts of input at a time. Returns 1 if is RLE, 0 if not. */
unsafe fn ZSTD_isRLE(src: *const BYTE, length: usize) -> c_int {
    let ip: *const BYTE = src;
    let value: BYTE = *ip.add(0);
    let valueST: usize = (value as U64).wrapping_mul(0x0101010101010101u64) as usize;
    let unrollSize: usize = core::mem::size_of::<usize>() * 4;
    let unrollMask: usize = unrollSize - 1;
    let prefixLength: usize = length & unrollMask;
    let mut i: usize;
    if length == 1 {
        return 1;
    }
    /* Check if prefix is RLE first before using unrolled loop */
    if prefixLength != 0 && ZSTD_count(ip.add(1), ip, ip.add(prefixLength)) != prefixLength - 1 {
        return 0;
    }
    i = prefixLength;
    while i != length {
        let mut u: usize = 0;
        while u < unrollSize {
            if MEM_readST(ip.add(i).add(u) as *const c_void) != valueST {
                return 0;
            }
            u += core::mem::size_of::<usize>();
        }
        i += unrollSize;
    }
    1
}

/* Returns true if the given block may be RLE.
 * This is just a heuristic based on the compressibility.
 * It may return both false positives and false negatives.
 */
unsafe fn ZSTD_maybeRLE(seqStore: *const SeqStore_t) -> c_int {
    let nbSeqs: usize = (*seqStore)
        .sequences
        .offset_from((*seqStore).sequencesStart) as usize;
    let nbLits: usize = (*seqStore).lit.offset_from((*seqStore).litStart) as usize;

    (nbSeqs < 4 && nbLits < 10) as c_int
}

unsafe fn ZSTD_blockState_confirmRepcodesAndEntropyTables(bs: *mut ZSTD_blockState_t) {
    let tmp: *mut ZSTD_compressedBlockState_t = (*bs).prevCBlock;
    (*bs).prevCBlock = (*bs).nextCBlock;
    (*bs).nextCBlock = tmp;
}

/* Writes the block header */
unsafe fn writeBlockHeader(op: *mut c_void, cSize: usize, blockSize: usize, lastBlock: U32) {
    let cBlockHeader: U32 = if cSize == 1 {
        lastBlock
            .wrapping_add(((bt_rle as U32) << 1))
            .wrapping_add((blockSize << 3) as U32)
    } else {
        lastBlock
            .wrapping_add(((bt_compressed as U32) << 1))
            .wrapping_add((cSize << 3) as U32)
    };
    MEM_writeLE24(op, cBlockHeader);
}
