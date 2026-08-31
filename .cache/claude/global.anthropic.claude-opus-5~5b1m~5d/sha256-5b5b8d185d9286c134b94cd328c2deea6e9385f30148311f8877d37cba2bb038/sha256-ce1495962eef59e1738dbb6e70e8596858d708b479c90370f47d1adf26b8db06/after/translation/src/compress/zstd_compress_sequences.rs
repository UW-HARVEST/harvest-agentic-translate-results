//! Translation of `compress/zstd_compress_sequences.c`
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bitstream::*;
use crate::cmem::*;
use crate::compress::zstd_compress_internal::*;
use crate::error_private::*;
use crate::fse::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ===== zstd_compress_sequences.h ===== */

pub type ZSTD_DefaultPolicy_e = c_uint;
pub const ZSTD_defaultDisallowed: ZSTD_DefaultPolicy_e = 0;
pub const ZSTD_defaultAllowed: ZSTD_DefaultPolicy_e = 1;

/**
 * -log2(x / 256) lookup table for x in [0, 256).
 * If x == 0: Return 0
 * Else: Return floor(-log2(x / 256) * 256)
 */
static kInverseProbabilityLog256: [c_uint; 256] = [
    0, 2048, 1792, 1642, 1536, 1453, 1386, 1329, 1280, 1236, 1197, 1162, 1130, 1100, 1073, 1047,
    1024, 1001, 980, 960, 941, 923, 906, 889, 874, 859, 844, 830, 817, 804, 791, 779, 768, 756,
    745, 734, 724, 714, 704, 694, 685, 676, 667, 658, 650, 642, 633, 626, 618, 610, 603, 595, 588,
    581, 574, 567, 561, 554, 548, 542, 535, 529, 523, 517, 512, 506, 500, 495, 489, 484, 478, 473,
    468, 463, 458, 453, 448, 443, 438, 434, 429, 424, 420, 415, 411, 407, 402, 398, 394, 390, 386,
    382, 377, 373, 370, 366, 362, 358, 354, 350, 347, 343, 339, 336, 332, 329, 325, 322, 318, 315,
    311, 308, 305, 302, 298, 295, 292, 289, 286, 282, 279, 276, 273, 270, 267, 264, 261, 258, 256,
    253, 250, 247, 244, 241, 239, 236, 233, 230, 228, 225, 222, 220, 217, 215, 212, 209, 207, 204,
    202, 199, 197, 194, 192, 190, 187, 185, 182, 180, 178, 175, 173, 171, 168, 166, 164, 162, 159,
    157, 155, 153, 151, 149, 146, 144, 142, 140, 138, 136, 134, 132, 130, 128, 126, 123, 121, 119,
    117, 115, 114, 112, 110, 108, 106, 104, 102, 100, 98, 96, 94, 93, 91, 89, 87, 85, 83, 82, 80,
    78, 76, 74, 73, 71, 69, 67, 66, 64, 62, 61, 59, 57, 55, 54, 52, 50, 49, 47, 46, 44, 42, 41, 39,
    37, 36, 34, 33, 31, 30, 28, 26, 25, 23, 22, 20, 19, 17, 16, 14, 13, 11, 10, 8, 7, 5, 4, 2, 1,
];

pub(crate) unsafe fn ZSTD_getFSEMaxSymbolValue(ctable: *const FSE_CTable) -> c_uint {
    let ptr = ctable as *const c_void;
    let u16ptr = ptr as *const U16;
    let maxSymbolValue: U32 = MEM_read16(u16ptr.add(1) as *const c_void) as U32;
    maxSymbolValue
}

/**
 * Returns true if we should use ncount=-1 else we should
 * use ncount=1 for low probability symbols instead.
 */
pub(crate) unsafe fn ZSTD_useLowProbCount(nbSeq: usize) -> c_uint {
    /* Heuristic: This should cover most blocks <= 16K and
     * start to fade out after 16K to about 32K depending on
     * compressibility.
     */
    (nbSeq >= 2048) as c_uint
}

/**
 * Returns the cost in bytes of encoding the normalized count header.
 * Returns an error if any of the helper functions return an error.
 */
pub(crate) unsafe fn ZSTD_NCountCost(
    count: *const c_uint,
    max: c_uint,
    nbSeq: usize,
    FSELog: c_uint,
) -> usize {
    let mut wksp: [BYTE; FSE_NCOUNTBOUND] = [0; FSE_NCOUNTBOUND];
    let mut norm: [S16; (MaxSeq + 1) as usize] = [0; (MaxSeq + 1) as usize];
    let tableLog: U32 = crate::compress::fse_compress::FSE_optimalTableLog(FSELog, nbSeq, max);
    {
        let e = crate::compress::fse_compress::FSE_normalizeCount(
            norm.as_mut_ptr(),
            tableLog,
            count,
            nbSeq,
            max,
            ZSTD_useLowProbCount(nbSeq),
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    crate::compress::fse_compress::FSE_writeNCount(
        wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[BYTE; FSE_NCOUNTBOUND]>(),
        norm.as_ptr(),
        max,
        tableLog,
    )
}

/**
 * Returns the cost in bits of encoding the distribution described by count
 * using the entropy bound.
 */
pub(crate) unsafe fn ZSTD_entropyCost(count: *const c_uint, max: c_uint, total: usize) -> usize {
    let mut cost: c_uint = 0;
    let mut s: c_uint;

    s = 0;
    while s <= max {
        let mut norm: c_uint =
            ((256u32.wrapping_mul(*count.add(s as usize))) as usize / total) as c_uint;
        if *count.add(s as usize) != 0 && norm == 0 {
            norm = 1;
        }
        cost = cost.wrapping_add(
            (*count.add(s as usize)).wrapping_mul(kInverseProbabilityLog256[norm as usize]),
        );
        s += 1;
    }
    (cost >> 8) as usize
}

/**
 * Returns the cost in bits of encoding the distribution in count using ctable.
 * Returns an error if ctable cannot represent all the symbols in count.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_fseBitCost(
    ctable: *const FSE_CTable,
    count: *const c_uint,
    max: c_uint,
) -> usize {
    let kAccuracyLog: c_uint = 8;
    let mut cost: usize = 0;
    let mut s: c_uint;
    let mut cstate = FSE_CState_t::default();
    FSE_initCState(&mut cstate, ctable);
    if ZSTD_getFSEMaxSymbolValue(ctable) < max {
        return ERROR(ZSTD_error_GENERIC);
    }
    s = 0;
    while s <= max {
        let tableLog: c_uint = cstate.stateLog;
        let badCost: c_uint = (tableLog + 1) << kAccuracyLog;
        let bitCost: c_uint = FSE_bitCost(cstate.symbolTT, tableLog, s, kAccuracyLog);
        if *count.add(s as usize) == 0 {
            s += 1;
            continue;
        }
        if bitCost >= badCost {
            return ERROR(ZSTD_error_GENERIC);
        }
        cost = cost.wrapping_add((*count.add(s as usize) as usize).wrapping_mul(bitCost as usize));
        s += 1;
    }
    cost >> kAccuracyLog
}

/**
 * Returns the cost in bits of encoding the distribution in count using the
 * table described by norm. The max symbol support by norm is assumed >= max.
 * norm must be valid for every symbol with non-zero probability in count.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_crossEntropyCost(
    norm: *const i16,
    accuracyLog: c_uint,
    count: *const c_uint,
    max: c_uint,
) -> usize {
    let shift: c_uint = 8 - accuracyLog;
    let mut cost: usize = 0;
    let mut s: c_uint;

    s = 0;
    while s <= max {
        let normAcc: c_uint = if *norm.add(s as usize) != -1 {
            *norm.add(s as usize) as c_int as c_uint
        } else {
            1
        };
        let norm256: c_uint = normAcc << shift;
        cost = cost.wrapping_add(
            (*count.add(s as usize)).wrapping_mul(kInverseProbabilityLog256[norm256 as usize])
                as usize,
        );
        s += 1;
    }
    cost >> 8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectEncodingType(
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
) -> SymbolEncodingType_e {
    if mostFrequent == nbSeq {
        *repeatMode = FSE_repeat_none;
        if isDefaultAllowed != 0 && nbSeq <= 2 {
            /* Prefer set_basic over set_rle when there are 2 or fewer symbols,
             * since RLE uses 1 byte, but set_basic uses 5-6 bits per symbol.
             * If basic encoding isn't possible, always choose RLE.
             */
            return set_basic;
        }
        return set_rle;
    }
    if strategy < ZSTD_lazy {
        if isDefaultAllowed != 0 {
            let staticFse_nbSeq_max: usize = 1000;
            let mult: usize = (10u32.wrapping_sub(strategy)) as usize;
            let baseLog: usize = 3;
            /* 28-36 for offset, 56-72 for lengths */
            let dynamicFse_nbSeq_min: usize =
                ((1usize << defaultNormLog).wrapping_mul(mult)) >> baseLog;
            if (*repeatMode == FSE_repeat_valid) && (nbSeq < staticFse_nbSeq_max) {
                return set_repeat;
            }
            if (nbSeq < dynamicFse_nbSeq_min)
                || (mostFrequent < (nbSeq >> (defaultNormLog - 1)))
            {
                /* The format allows default tables to be repeated, but it isn't useful.
                 * When using simple heuristics to select encoding type, we don't want
                 * to confuse these tables with dictionaries. When running more careful
                 * analysis, we don't need to waste time checking both repeating tables
                 * and default tables.
                 */
                *repeatMode = FSE_repeat_none;
                return set_basic;
            }
        }
    } else {
        let basicCost: usize = if isDefaultAllowed != 0 {
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, count, max)
        } else {
            ERROR(ZSTD_error_GENERIC)
        };
        let repeatCost: usize = if *repeatMode != FSE_repeat_none {
            ZSTD_fseBitCost(prevCTable, count, max)
        } else {
            ERROR(ZSTD_error_GENERIC)
        };
        let NCountCost: usize = ZSTD_NCountCost(count, max, nbSeq, FSELog);
        let compressedCost: usize =
            (NCountCost << 3).wrapping_add(ZSTD_entropyCost(count, max, nbSeq));

        if basicCost <= repeatCost && basicCost <= compressedCost {
            *repeatMode = FSE_repeat_none;
            return set_basic;
        }
        if repeatCost <= compressedCost {
            return set_repeat;
        }
    }
    *repeatMode = FSE_repeat_check;
    set_compressed
}

#[repr(C)]
pub struct ZSTD_BuildCTableWksp {
    pub norm: [S16; (MaxSeq + 1) as usize],
    pub wksp: [U32; FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(MaxSeq, MaxFSELog)],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildCTable(
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
) -> usize {
    let op = dst as *mut BYTE;
    let oend = op.add(dstCapacity) as *const BYTE;

    match type_ {
        set_rle => {
            {
                let e = crate::compress::fse_compress::FSE_buildCTable_rle(nextCTable, max as BYTE);
                if ERR_isError(e) != 0 {
                    return e;
                }
            }
            if dstCapacity == 0 {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            *op = *codeTable;
            1
        }
        set_repeat => {
            ZSTD_memcpy(
                nextCTable as *mut c_void,
                prevCTable as *const c_void,
                prevCTableSize,
            );
            0
        }
        set_basic => {
            /* note : could be pre-calculated */
            let e = crate::compress::fse_compress::FSE_buildCTable_wksp(
                nextCTable,
                defaultNorm,
                defaultMax,
                defaultNormLog,
                entropyWorkspace,
                entropyWorkspaceSize,
            );
            if ERR_isError(e) != 0 {
                return e;
            }
            0
        }
        set_compressed => {
            let wksp = entropyWorkspace as *mut ZSTD_BuildCTableWksp;
            let mut nbSeq_1: usize = nbSeq;
            let tableLog: U32 =
                crate::compress::fse_compress::FSE_optimalTableLog(FSELog, nbSeq, max);
            if *count.add(*codeTable.add(nbSeq - 1) as usize) > 1 {
                *count.add(*codeTable.add(nbSeq - 1) as usize) -= 1;
                nbSeq_1 -= 1;
            }
            {
                /* "FSE_normalizeCount failed" */
                let e = crate::compress::fse_compress::FSE_normalizeCount(
                    (*wksp).norm.as_mut_ptr(),
                    tableLog,
                    count,
                    nbSeq_1,
                    max,
                    ZSTD_useLowProbCount(nbSeq_1),
                );
                if ERR_isError(e) != 0 {
                    return e;
                }
            }
            {
                /* overflow protected */
                let NCountSize: usize = crate::compress::fse_compress::FSE_writeNCount(
                    op as *mut c_void,
                    (oend as usize - op as usize) as usize,
                    (*wksp).norm.as_ptr(),
                    max,
                    tableLog,
                );
                if ERR_isError(NCountSize) != 0 {
                    return NCountSize;
                }
                {
                    /* "FSE_buildCTable_wksp failed" */
                    let e = crate::compress::fse_compress::FSE_buildCTable_wksp(
                        nextCTable,
                        (*wksp).norm.as_ptr(),
                        max,
                        tableLog,
                        (*wksp).wksp.as_mut_ptr() as *mut c_void,
                        core::mem::size_of_val(&(*wksp).wksp),
                    );
                    if ERR_isError(e) != 0 {
                        return e;
                    }
                }
                NCountSize
            }
        }
        _ => ERROR(ZSTD_error_GENERIC),
    }
}

#[inline(always)]
pub(crate) unsafe fn ZSTD_encodeSequences_body(
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
) -> usize {
    let mut blockStream = BIT_CStream_t::default();
    let mut stateMatchLength = FSE_CState_t::default();
    let mut stateOffsetBits = FSE_CState_t::default();
    let mut stateLitLength = FSE_CState_t::default();

    if ERR_isError(BIT_initCStream(&mut blockStream, dst, dstCapacity)) != 0 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    /* first symbols */
    FSE_initCState2(
        &mut stateMatchLength,
        CTable_MatchLength,
        *mlCodeTable.add(nbSeq - 1) as U32,
    );
    FSE_initCState2(
        &mut stateOffsetBits,
        CTable_OffsetBits,
        *ofCodeTable.add(nbSeq - 1) as U32,
    );
    FSE_initCState2(
        &mut stateLitLength,
        CTable_LitLength,
        *llCodeTable.add(nbSeq - 1) as U32,
    );
    BIT_addBits(
        &mut blockStream,
        (*sequences.add(nbSeq - 1)).litLength as BitContainerType,
        LL_bits[*llCodeTable.add(nbSeq - 1) as usize] as u32,
    );
    if MEM_32bits() != 0 {
        BIT_flushBits(&mut blockStream);
    }
    BIT_addBits(
        &mut blockStream,
        (*sequences.add(nbSeq - 1)).mlBase as BitContainerType,
        ML_bits[*mlCodeTable.add(nbSeq - 1) as usize] as u32,
    );
    if MEM_32bits() != 0 {
        BIT_flushBits(&mut blockStream);
    }
    if longOffsets != 0 {
        let ofBits: U32 = *ofCodeTable.add(nbSeq - 1) as U32;
        let lim: U32 = STREAM_ACCUMULATOR_MIN() - 1;
        let extraBits: c_uint = ofBits - (if ofBits < lim { ofBits } else { lim });
        if extraBits != 0 {
            BIT_addBits(
                &mut blockStream,
                (*sequences.add(nbSeq - 1)).offBase as BitContainerType,
                extraBits,
            );
            BIT_flushBits(&mut blockStream);
        }
        BIT_addBits(
            &mut blockStream,
            ((*sequences.add(nbSeq - 1)).offBase >> extraBits) as BitContainerType,
            ofBits - extraBits,
        );
    } else {
        BIT_addBits(
            &mut blockStream,
            (*sequences.add(nbSeq - 1)).offBase as BitContainerType,
            *ofCodeTable.add(nbSeq - 1) as u32,
        );
    }
    BIT_flushBits(&mut blockStream);

    {
        let mut n: usize = nbSeq.wrapping_sub(2);
        while n < nbSeq {
            /* intentional underflow */
            let llCode: BYTE = *llCodeTable.add(n);
            let ofCode: BYTE = *ofCodeTable.add(n);
            let mlCode: BYTE = *mlCodeTable.add(n);
            let llBits: U32 = LL_bits[llCode as usize] as U32;
            let ofBits: U32 = ofCode as U32;
            let mlBits: U32 = ML_bits[mlCode as usize] as U32;
            /* 32b*/ /* 64b*/
            /* (7)*/ /* (7)*/
            FSE_encodeSymbol(&mut blockStream, &mut stateOffsetBits, ofCode as c_uint); /* 15 */ /* 15 */
            FSE_encodeSymbol(&mut blockStream, &mut stateMatchLength, mlCode as c_uint); /* 24 */ /* 24 */
            if MEM_32bits() != 0 {
                BIT_flushBits(&mut blockStream); /* (7)*/
            }
            FSE_encodeSymbol(&mut blockStream, &mut stateLitLength, llCode as c_uint); /* 16 */ /* 33 */
            if MEM_32bits() != 0
                || (ofBits + mlBits + llBits >= 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
            {
                BIT_flushBits(&mut blockStream); /* (7)*/
            }
            BIT_addBits(
                &mut blockStream,
                (*sequences.add(n)).litLength as BitContainerType,
                llBits,
            );
            if MEM_32bits() != 0 && ((llBits + mlBits) > 24) {
                BIT_flushBits(&mut blockStream);
            }
            BIT_addBits(
                &mut blockStream,
                (*sequences.add(n)).mlBase as BitContainerType,
                mlBits,
            );
            if MEM_32bits() != 0 || (ofBits + mlBits + llBits > 56) {
                BIT_flushBits(&mut blockStream);
            }
            if longOffsets != 0 {
                let lim: U32 = STREAM_ACCUMULATOR_MIN() - 1;
                let extraBits: c_uint = ofBits - (if ofBits < lim { ofBits } else { lim });
                if extraBits != 0 {
                    BIT_addBits(
                        &mut blockStream,
                        (*sequences.add(n)).offBase as BitContainerType,
                        extraBits,
                    );
                    BIT_flushBits(&mut blockStream); /* (7)*/
                }
                BIT_addBits(
                    &mut blockStream,
                    ((*sequences.add(n)).offBase >> extraBits) as BitContainerType,
                    ofBits - extraBits,
                ); /* 31 */
            } else {
                BIT_addBits(
                    &mut blockStream,
                    (*sequences.add(n)).offBase as BitContainerType,
                    ofBits,
                ); /* 31 */
            }
            BIT_flushBits(&mut blockStream); /* (7)*/

            n = n.wrapping_sub(1);
        }
    }

    FSE_flushCState(&mut blockStream, &stateMatchLength);
    FSE_flushCState(&mut blockStream, &stateOffsetBits);
    FSE_flushCState(&mut blockStream, &stateLitLength);

    {
        let streamSize: usize = BIT_closeCStream(&mut blockStream);
        if streamSize == 0 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        streamSize
    }
}

pub(crate) unsafe fn ZSTD_encodeSequences_default(
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
) -> usize {
    ZSTD_encodeSequences_body(
        dst,
        dstCapacity,
        CTable_MatchLength,
        mlCodeTable,
        CTable_OffsetBits,
        ofCodeTable,
        CTable_LitLength,
        llCodeTable,
        sequences,
        nbSeq,
        longOffsets,
    )
}

/* DYNAMIC_BMI2 is 0 for this build: `ZSTD_encodeSequences_bmi2()` is not compiled. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_encodeSequences(
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
) -> usize {
    ZSTD_encodeSequences_default(
        dst,
        dstCapacity,
        CTable_MatchLength,
        mlCodeTable,
        CTable_OffsetBits,
        ofCodeTable,
        CTable_LitLength,
        llCodeTable,
        sequences,
        nbSeq,
        longOffsets,
    )
}
