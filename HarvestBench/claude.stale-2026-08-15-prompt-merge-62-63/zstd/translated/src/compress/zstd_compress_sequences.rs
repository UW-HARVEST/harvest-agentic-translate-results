//! Translation of compress/zstd_compress_sequences.c

use crate::common::bitstream::*;
use crate::common::error::{code, err_is_error, error};
use crate::common::fse::*;
use crate::common::mem::*;
use crate::common::zstd_internal::{
    set_basic, set_compressed, set_repeat, set_rle, LLFSELog, LL_bits, MLFSELog, ML_bits,
    MINMATCH, MaxFSELog, MaxSeq, OffFSELog,
};
use crate::compress::fse_compress::{
    FSE_buildCTable_rle, FSE_buildCTable_wksp, FSE_normalizeCount, FSE_optimalTableLog,
    FSE_writeNCount,
};
use crate::compress::zstd_compress_internal::SeqDef;
use crate::zstd_h::{ZSTD_lazy, ZSTD_strategy};
use core::ffi::c_void;

pub type SymbolEncodingType_e = u32;

/* ZSTD_DefaultPolicy_e — defined in the .h */
pub const ZSTD_defaultDisallowed: u32 = 0;
pub const ZSTD_defaultAllowed: u32 = 1;
pub type ZSTD_DefaultPolicy_e = u32;

/**
 * -log2(x / 256) lookup table for x in [0, 256).
 * If x == 0: Return 0
 * Else: Return floor(-log2(x / 256) * 256)
 */
static kInverseProbabilityLog256: [u32; 256] = [
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

unsafe fn ZSTD_getFSEMaxSymbolValue(ctable: *const FSE_CTable) -> u32 {
    let ptr = ctable as *const c_void;
    let u16ptr = ptr as *const u16;
    let maxSymbolValue = mem_read16(u16ptr.add(1) as *const c_void) as u32;
    maxSymbolValue
}

/**
 * Returns true if we should use ncount=-1 else we should
 * use ncount=1 for low probability symbols instead.
 */
fn ZSTD_useLowProbCount(nbSeq: usize) -> u32 {
    /* Heuristic: This should cover most blocks <= 16K and
     * start to fade out after 16K to about 32K depending on
     * compressibility.
     */
    (nbSeq >= 2048) as u32
}

/**
 * Returns the cost in bytes of encoding the normalized count header.
 * Returns an error if any of the helper functions return an error.
 */
unsafe fn ZSTD_NCountCost(count: *const u32, max: u32, nbSeq: usize, FSELog: u32) -> usize {
    let mut wksp = [0u8; FSE_NCOUNTBOUND];
    let mut norm = [0i16; MaxSeq as usize + 1];
    let tableLog = FSE_optimalTableLog(FSELog, nbSeq, max);
    {
        let err = FSE_normalizeCount(
            norm.as_mut_ptr(),
            tableLog,
            count,
            nbSeq,
            max,
            ZSTD_useLowProbCount(nbSeq),
        );
        if err_is_error(err) != 0 {
            return err;
        }
    }
    FSE_writeNCount(
        wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&wksp),
        norm.as_ptr(),
        max,
        tableLog,
    )
}

/**
 * Returns the cost in bits of encoding the distribution described by count
 * using the entropy bound.
 */
unsafe fn ZSTD_entropyCost(count: *const u32, max: u32, total: usize) -> usize {
    let mut cost: u32 = 0;
    let mut s: u32;

    debug_assert!(total > 0);
    s = 0;
    while s <= max {
        let mut norm = ((256usize * (*count.add(s as usize)) as usize) / total) as u32;
        if *count.add(s as usize) != 0 && norm == 0 {
            norm = 1;
        }
        debug_assert!((*count.add(s as usize) as usize) < total);
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
    count: *const u32,
    max: u32,
) -> usize {
    let kAccuracyLog: u32 = 8;
    let mut cost: usize = 0;
    let mut s: u32;
    let mut cstate = FSE_CState_t {
        value: 0,
        stateTable: core::ptr::null(),
        symbolTT: core::ptr::null(),
        stateLog: 0,
    };
    fse_init_cstate(&mut cstate, ctable);
    if ZSTD_getFSEMaxSymbolValue(ctable) < max {
        return error(code::GENERIC);
    }
    s = 0;
    while s <= max {
        let tableLog = cstate.stateLog;
        let badCost = (tableLog + 1) << kAccuracyLog;
        let bitCost = fse_bit_cost(cstate.symbolTT, tableLog, s, kAccuracyLog);
        if *count.add(s as usize) == 0 {
            s += 1;
            continue;
        }
        if bitCost >= badCost {
            return error(code::GENERIC);
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
    accuracyLog: u32,
    count: *const u32,
    max: u32,
) -> usize {
    let shift = 8 - accuracyLog;
    let mut cost: usize = 0;
    let mut s: u32;
    debug_assert!(accuracyLog <= 8);
    s = 0;
    while s <= max {
        let normAcc: u32 = if *norm.add(s as usize) != -1 {
            *norm.add(s as usize) as u32
        } else {
            1
        };
        let norm256 = normAcc << shift;
        debug_assert!(norm256 > 0);
        debug_assert!(norm256 < 256);
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
    count: *const u32,
    max: u32,
    mostFrequent: usize,
    nbSeq: usize,
    FSELog: u32,
    prevCTable: *const FSE_CTable,
    defaultNorm: *const i16,
    defaultNormLog: u32,
    isDefaultAllowed: ZSTD_DefaultPolicy_e,
    strategy: ZSTD_strategy,
) -> SymbolEncodingType_e {
    /* ZSTD_STATIC_ASSERT(ZSTD_defaultDisallowed == 0 && ZSTD_defaultAllowed != 0); */
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
            let _staticFse_nbSeq_max: usize = 1000;
            let mult: usize = (10 - strategy) as usize;
            let baseLog: usize = 3;
            let dynamicFse_nbSeq_min: usize =
                ((1usize << defaultNormLog) * mult) >> baseLog; /* 28-36 for offset, 56-72 for lengths */
            debug_assert!(defaultNormLog >= 5 && defaultNormLog <= 6); /* xx_DEFAULTNORMLOG */
            debug_assert!(mult <= 9 && mult >= 7);
            if (*repeatMode == FSE_repeat_valid) && (nbSeq < _staticFse_nbSeq_max) {
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
            error(code::GENERIC)
        };
        let repeatCost: usize = if *repeatMode != FSE_repeat_none {
            ZSTD_fseBitCost(prevCTable, count, max)
        } else {
            error(code::GENERIC)
        };
        let NCountCost: usize = ZSTD_NCountCost(count, max, nbSeq, FSELog);
        let compressedCost: usize = (NCountCost << 3) + ZSTD_entropyCost(count, max, nbSeq);

        if isDefaultAllowed != 0 {
            debug_assert!(err_is_error(basicCost) == 0);
            debug_assert!(!(*repeatMode == FSE_repeat_valid && err_is_error(repeatCost) != 0));
        }
        debug_assert!(err_is_error(NCountCost) == 0);
        debug_assert!(compressedCost < error(code::MAXCODE));
        if basicCost <= repeatCost && basicCost <= compressedCost {
            debug_assert!(isDefaultAllowed != 0);
            *repeatMode = FSE_repeat_none;
            return set_basic;
        }
        if repeatCost <= compressedCost {
            debug_assert!(err_is_error(repeatCost) == 0);
            return set_repeat;
        }
        debug_assert!(compressedCost < basicCost && compressedCost < repeatCost);
    }
    *repeatMode = FSE_repeat_check;
    set_compressed
}

const ZSTD_BUILD_CTABLE_WKSP_WKSP_U32: usize =
    ((MaxSeq as usize + 2) + (1usize << MaxFSELog)) / 2 + (8 / 4);

#[repr(C)]
struct ZSTD_BuildCTableWksp {
    norm: [i16; MaxSeq as usize + 1],
    wksp: [u32; ZSTD_BUILD_CTABLE_WKSP_WKSP_U32],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildCTable(
    dst: *mut c_void,
    dstCapacity: usize,
    nextCTable: *mut FSE_CTable,
    FSELog: u32,
    r#type: SymbolEncodingType_e,
    count: *mut u32,
    max: u32,
    codeTable: *const u8,
    nbSeq: usize,
    defaultNorm: *const i16,
    defaultNormLog: u32,
    defaultMax: u32,
    prevCTable: *const FSE_CTable,
    prevCTableSize: usize,
    entropyWorkspace: *mut c_void,
    entropyWorkspaceSize: usize,
) -> usize {
    let op = dst as *mut u8;
    let oend = op.add(dstCapacity);

    match r#type {
        x if x == set_rle => {
            {
                let err = FSE_buildCTable_rle(nextCTable, max as u8);
                if err_is_error(err) != 0 {
                    return err;
                }
            }
            if dstCapacity == 0 {
                return error(code::DSTSIZE_TOOSMALL);
            }
            *op = *codeTable.add(0);
            1
        }
        x if x == set_repeat => {
            core::ptr::copy_nonoverlapping(
                prevCTable as *const u8,
                nextCTable as *mut u8,
                prevCTableSize,
            );
            0
        }
        x if x == set_basic => {
            {
                let err = FSE_buildCTable_wksp(
                    nextCTable,
                    defaultNorm,
                    defaultMax,
                    defaultNormLog,
                    entropyWorkspace,
                    entropyWorkspaceSize,
                ); /* note : could be pre-calculated */
                if err_is_error(err) != 0 {
                    return err;
                }
            }
            0
        }
        x if x == set_compressed => {
            let wksp = entropyWorkspace as *mut ZSTD_BuildCTableWksp;
            let mut nbSeq_1 = nbSeq;
            let tableLog = FSE_optimalTableLog(FSELog, nbSeq, max);
            if *count.add(*codeTable.add(nbSeq - 1) as usize) > 1 {
                *count.add(*codeTable.add(nbSeq - 1) as usize) -= 1;
                nbSeq_1 -= 1;
            }
            debug_assert!(nbSeq_1 > 1);
            debug_assert!(entropyWorkspaceSize >= core::mem::size_of::<ZSTD_BuildCTableWksp>());
            let _ = entropyWorkspaceSize;
            {
                let err = FSE_normalizeCount(
                    (*wksp).norm.as_mut_ptr(),
                    tableLog,
                    count,
                    nbSeq_1,
                    max,
                    ZSTD_useLowProbCount(nbSeq_1),
                );
                if err_is_error(err) != 0 {
                    return err;
                }
            }
            debug_assert!(oend >= op);
            {
                let NCountSize = FSE_writeNCount(
                    op as *mut c_void,
                    oend as usize - op as usize,
                    (*wksp).norm.as_ptr(),
                    max,
                    tableLog,
                ); /* overflow protected */
                if err_is_error(NCountSize) != 0 {
                    return NCountSize;
                }
                let err = FSE_buildCTable_wksp(
                    nextCTable,
                    (*wksp).norm.as_ptr(),
                    max,
                    tableLog,
                    (*wksp).wksp.as_mut_ptr() as *mut c_void,
                    core::mem::size_of_val(&(*wksp).wksp),
                );
                if err_is_error(err) != 0 {
                    return err;
                }
                NCountSize
            }
        }
        _ => {
            debug_assert!(false);
            error(code::GENERIC)
        }
    }
}

#[inline(always)]
unsafe fn ZSTD_encodeSequences_body(
    dst: *mut c_void,
    dstCapacity: usize,
    CTable_MatchLength: *const FSE_CTable,
    mlCodeTable: *const u8,
    CTable_OffsetBits: *const FSE_CTable,
    ofCodeTable: *const u8,
    CTable_LitLength: *const FSE_CTable,
    llCodeTable: *const u8,
    sequences: *const SeqDef,
    nbSeq: usize,
    longOffsets: i32,
) -> usize {
    let mut blockStream = BIT_CStream_t {
        bitContainer: 0,
        bitPos: 0,
        startPtr: core::ptr::null_mut(),
        ptr: core::ptr::null_mut(),
        endPtr: core::ptr::null_mut(),
    };
    let mut stateMatchLength = FSE_CState_t {
        value: 0,
        stateTable: core::ptr::null(),
        symbolTT: core::ptr::null(),
        stateLog: 0,
    };
    let mut stateOffsetBits = FSE_CState_t {
        value: 0,
        stateTable: core::ptr::null(),
        symbolTT: core::ptr::null(),
        stateLog: 0,
    };
    let mut stateLitLength = FSE_CState_t {
        value: 0,
        stateTable: core::ptr::null(),
        symbolTT: core::ptr::null(),
        stateLog: 0,
    };

    if err_is_error(bit_init_cstream(&mut blockStream, dst, dstCapacity)) != 0 {
        return error(code::DSTSIZE_TOOSMALL);
    }

    /* first symbols */
    fse_init_cstate2(
        &mut stateMatchLength,
        CTable_MatchLength,
        *mlCodeTable.add(nbSeq - 1) as u32,
    );
    fse_init_cstate2(
        &mut stateOffsetBits,
        CTable_OffsetBits,
        *ofCodeTable.add(nbSeq - 1) as u32,
    );
    fse_init_cstate2(
        &mut stateLitLength,
        CTable_LitLength,
        *llCodeTable.add(nbSeq - 1) as u32,
    );
    bit_add_bits(
        &mut blockStream,
        (*sequences.add(nbSeq - 1)).litLength as BitContainerType,
        LL_bits[*llCodeTable.add(nbSeq - 1) as usize] as u32,
    );
    if mem_32bits() != 0 {
        bit_flush_bits(&mut blockStream);
    }
    bit_add_bits(
        &mut blockStream,
        (*sequences.add(nbSeq - 1)).mlBase as BitContainerType,
        ML_bits[*mlCodeTable.add(nbSeq - 1) as usize] as u32,
    );
    if mem_32bits() != 0 {
        bit_flush_bits(&mut blockStream);
    }
    if longOffsets != 0 {
        let ofBits = *ofCodeTable.add(nbSeq - 1) as u32;
        let extraBits = ofBits - core::cmp::min(ofBits, stream_accumulator_min() - 1);
        if extraBits != 0 {
            bit_add_bits(
                &mut blockStream,
                (*sequences.add(nbSeq - 1)).offBase as BitContainerType,
                extraBits,
            );
            bit_flush_bits(&mut blockStream);
        }
        bit_add_bits(
            &mut blockStream,
            ((*sequences.add(nbSeq - 1)).offBase >> extraBits) as BitContainerType,
            ofBits - extraBits,
        );
    } else {
        bit_add_bits(
            &mut blockStream,
            (*sequences.add(nbSeq - 1)).offBase as BitContainerType,
            *ofCodeTable.add(nbSeq - 1) as u32,
        );
    }
    bit_flush_bits(&mut blockStream);

    {
        let mut n: usize = nbSeq - 2;
        while n < nbSeq {
            /* intentional underflow */
            let llCode = *llCodeTable.add(n);
            let ofCode = *ofCodeTable.add(n);
            let mlCode = *mlCodeTable.add(n);
            let llBits = LL_bits[llCode as usize] as u32;
            let ofBits = ofCode as u32;
            let mlBits = ML_bits[mlCode as usize] as u32;
            let _ = MINMATCH;
            /* 32b*/ /* 64b*/
            /* (7)*/ /* (7)*/
            fse_encode_symbol(&mut blockStream, &mut stateOffsetBits, ofCode as u32); /* 15 */ /* 15 */
            fse_encode_symbol(&mut blockStream, &mut stateMatchLength, mlCode as u32); /* 24 */ /* 24 */
            if mem_32bits() != 0 {
                bit_flush_bits(&mut blockStream);
            } /* (7)*/
            fse_encode_symbol(&mut blockStream, &mut stateLitLength, llCode as u32); /* 16 */ /* 33 */
            if mem_32bits() != 0
                || (ofBits + mlBits + llBits >= 64 - 7 - (LLFSELog + MLFSELog + OffFSELog))
            {
                bit_flush_bits(&mut blockStream); /* (7)*/
            }
            bit_add_bits(
                &mut blockStream,
                (*sequences.add(n)).litLength as BitContainerType,
                llBits,
            );
            if mem_32bits() != 0 && ((llBits + mlBits) > 24) {
                bit_flush_bits(&mut blockStream);
            }
            bit_add_bits(
                &mut blockStream,
                (*sequences.add(n)).mlBase as BitContainerType,
                mlBits,
            );
            if mem_32bits() != 0 || (ofBits + mlBits + llBits > 56) {
                bit_flush_bits(&mut blockStream);
            }
            if longOffsets != 0 {
                let extraBits = ofBits - core::cmp::min(ofBits, stream_accumulator_min() - 1);
                if extraBits != 0 {
                    bit_add_bits(
                        &mut blockStream,
                        (*sequences.add(n)).offBase as BitContainerType,
                        extraBits,
                    );
                    bit_flush_bits(&mut blockStream); /* (7)*/
                }
                bit_add_bits(
                    &mut blockStream,
                    ((*sequences.add(n)).offBase >> extraBits) as BitContainerType,
                    ofBits - extraBits,
                ); /* 31 */
            } else {
                bit_add_bits(
                    &mut blockStream,
                    (*sequences.add(n)).offBase as BitContainerType,
                    ofBits,
                ); /* 31 */
            }
            bit_flush_bits(&mut blockStream); /* (7)*/
            n = n.wrapping_sub(1);
        }
    }

    fse_flush_cstate(&mut blockStream, &stateMatchLength);
    fse_flush_cstate(&mut blockStream, &stateOffsetBits);
    fse_flush_cstate(&mut blockStream, &stateLitLength);

    {
        let streamSize = bit_close_cstream(&mut blockStream);
        if streamSize == 0 {
            return error(code::DSTSIZE_TOOSMALL);
        }
        streamSize
    }
}

unsafe fn ZSTD_encodeSequences_default(
    dst: *mut c_void,
    dstCapacity: usize,
    CTable_MatchLength: *const FSE_CTable,
    mlCodeTable: *const u8,
    CTable_OffsetBits: *const FSE_CTable,
    ofCodeTable: *const u8,
    CTable_LitLength: *const FSE_CTable,
    llCodeTable: *const u8,
    sequences: *const SeqDef,
    nbSeq: usize,
    longOffsets: i32,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_encodeSequences(
    dst: *mut c_void,
    dstCapacity: usize,
    CTable_MatchLength: *const FSE_CTable,
    mlCodeTable: *const u8,
    CTable_OffsetBits: *const FSE_CTable,
    ofCodeTable: *const u8,
    CTable_LitLength: *const FSE_CTable,
    llCodeTable: *const u8,
    sequences: *const SeqDef,
    nbSeq: usize,
    longOffsets: i32,
    bmi2: i32,
) -> usize {
    let _ = bmi2;
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
