//! Translation of compress/zstd_compress_sequences.c (+ compress/zstd_compress_sequences.h)
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use crate::bitstream::*;
use crate::error_private::*;
use crate::fse::*;
use crate::mem::*;
use crate::zstd_compress_internal::SeqDef;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ===  from zstd_compress_sequences.h  === */

/// ```c
/// typedef enum {
///     ZSTD_defaultDisallowed = 0,
///     ZSTD_defaultAllowed = 1
/// } ZSTD_DefaultPolicy_e;
/// ```
pub type ZSTD_DefaultPolicy_e = core::ffi::c_int;
pub const ZSTD_defaultDisallowed: ZSTD_DefaultPolicy_e = 0;
pub const ZSTD_defaultAllowed: ZSTD_DefaultPolicy_e = 1;

/**
 * -log2(x / 256) lookup table for x in [0, 256).
 * If x == 0: Return 0
 * Else: Return floor(-log2(x / 256) * 256)
 */
pub static kInverseProbabilityLog256: [core::ffi::c_uint; 256] = [
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

/// `static unsigned ZSTD_getFSEMaxSymbolValue(FSE_CTable const* ctable)`
pub unsafe fn ZSTD_getFSEMaxSymbolValue(ctable: *const FSE_CTable) -> core::ffi::c_uint {
    let ptr: *const core::ffi::c_void = ctable as *const core::ffi::c_void;
    let u16ptr: *const U16 = ptr as *const U16;
    let maxSymbolValue: U32 = MEM_read16(u16ptr.add(1) as *const u8) as U32;
    maxSymbolValue
}

/**
 * Returns true if we should use ncount=-1 else we should
 * use ncount=1 for low probability symbols instead.
 */
pub fn ZSTD_useLowProbCount(nbSeq: usize) -> core::ffi::c_uint {
    /* Heuristic: This should cover most blocks <= 16K and
     * start to fade out after 16K to about 32K depending on
     * compressibility.
     */
    (nbSeq >= 2048) as core::ffi::c_uint
}

/**
 * Returns the cost in bytes of encoding the normalized count header.
 * Returns an error if any of the helper functions return an error.
 */
pub unsafe fn ZSTD_NCountCost(
    count: *const core::ffi::c_uint,
    max: core::ffi::c_uint,
    nbSeq: usize,
    FSELog: core::ffi::c_uint,
) -> usize {
    let mut wksp: [BYTE; FSE_NCOUNTBOUND] = [0; FSE_NCOUNTBOUND];
    let mut norm: [S16; MaxSeq as usize + 1] = [0; MaxSeq as usize + 1];
    let tableLog: U32 = crate::fse_compress::FSE_optimalTableLog(FSELog, nbSeq, max);
    {
        let err_code = crate::fse_compress::FSE_normalizeCount(
            norm.as_mut_ptr(),
            tableLog,
            count,
            nbSeq,
            max,
            ZSTD_useLowProbCount(nbSeq),
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    crate::fse_compress::FSE_writeNCount(
        wksp.as_mut_ptr() as *mut core::ffi::c_void,
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
pub unsafe fn ZSTD_entropyCost(
    count: *const core::ffi::c_uint,
    max: core::ffi::c_uint,
    total: usize,
) -> usize {
    let mut cost: core::ffi::c_uint = 0;
    let mut s: core::ffi::c_uint;

    s = 0;
    while s <= max {
        /* `256 * count[s]` is computed in `unsigned` (i.e. 32-bit, wrapping),
         * then promoted to `size_t` for the division. */
        let mut norm: core::ffi::c_uint =
            ((256u32.wrapping_mul(*count.add(s as usize)) as usize) / total) as core::ffi::c_uint;
        if *count.add(s as usize) != 0 && norm == 0 {
            norm = 1;
        }
        cost = cost.wrapping_add(
            (*count.add(s as usize)).wrapping_mul(kInverseProbabilityLog256[norm as usize]),
        );
        s = s.wrapping_add(1);
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
    count: *const core::ffi::c_uint,
    max: core::ffi::c_uint,
) -> usize {
    let kAccuracyLog: core::ffi::c_uint = 8;
    let mut cost: usize = 0;
    let mut s: core::ffi::c_uint;
    let mut cstate: FSE_CState_t = FSE_CState_t::default();
    FSE_initCState(&mut cstate, ctable);
    if ZSTD_getFSEMaxSymbolValue(ctable) < max {
        return ERROR(ZSTD_error_GENERIC);
    }
    s = 0;
    while s <= max {
        let tableLog: core::ffi::c_uint = cstate.stateLog;
        let badCost: core::ffi::c_uint = tableLog.wrapping_add(1) << kAccuracyLog;
        let bitCost: core::ffi::c_uint = FSE_bitCost(cstate.symbolTT, tableLog, s, kAccuracyLog);
        if *count.add(s as usize) == 0 {
            s = s.wrapping_add(1);
            continue;
        }
        if bitCost >= badCost {
            return ERROR(ZSTD_error_GENERIC);
        }
        cost = cost.wrapping_add((*count.add(s as usize) as usize).wrapping_mul(bitCost as usize));
        s = s.wrapping_add(1);
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
    norm: *const core::ffi::c_short,
    accuracyLog: core::ffi::c_uint,
    count: *const core::ffi::c_uint,
    max: core::ffi::c_uint,
) -> usize {
    let shift: core::ffi::c_uint = 8u32.wrapping_sub(accuracyLog);
    let mut cost: usize = 0;
    let mut s: core::ffi::c_uint;
    s = 0;
    while s <= max {
        let normAcc: core::ffi::c_uint = if *norm.add(s as usize) != -1 {
            *norm.add(s as usize) as core::ffi::c_uint
        } else {
            1
        };
        let norm256: core::ffi::c_uint = normAcc.wrapping_shl(shift);
        cost = cost.wrapping_add(
            (*count.add(s as usize)).wrapping_mul(kInverseProbabilityLog256[norm256 as usize])
                as usize,
        );
        s = s.wrapping_add(1);
    }
    cost >> 8
}

/// ```c
/// SymbolEncodingType_e
/// ZSTD_selectEncodingType(
///         FSE_repeat* repeatMode, unsigned const* count, unsigned const max,
///         size_t const mostFrequent, size_t nbSeq, unsigned const FSELog,
///         FSE_CTable const* prevCTable,
///         short const* defaultNorm, U32 defaultNormLog,
///         ZSTD_DefaultPolicy_e const isDefaultAllowed,
///         ZSTD_strategy const strategy);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectEncodingType(
    repeatMode: *mut FSE_repeat,
    count: *const core::ffi::c_uint,
    max: core::ffi::c_uint,
    mostFrequent: usize,
    nbSeq: usize,
    FSELog: core::ffi::c_uint,
    prevCTable: *const FSE_CTable,
    defaultNorm: *const core::ffi::c_short,
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
            let mult: usize = (10i32.wrapping_sub(strategy)) as usize;
            let baseLog: usize = 3;
            /* 28-36 for offset, 56-72 for lengths */
            let dynamicFse_nbSeq_min: usize =
                ((1usize << defaultNormLog).wrapping_mul(mult)) >> baseLog;
            if (*repeatMode == FSE_repeat_valid) && (nbSeq < staticFse_nbSeq_max) {
                return set_repeat;
            }
            if (nbSeq < dynamicFse_nbSeq_min)
                || (mostFrequent < (nbSeq >> (defaultNormLog.wrapping_sub(1))))
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

/// `U32 wksp[FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(MaxSeq, MaxFSELog)]`
pub const ZSTD_BuildCTableWksp_wksp_size_u32: usize =
    FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(MaxSeq, MaxFSELog);

/// ```c
/// typedef struct {
///     S16 norm[MaxSeq + 1];
///     U32 wksp[FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(MaxSeq, MaxFSELog)];
/// } ZSTD_BuildCTableWksp;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_BuildCTableWksp {
    pub norm: [S16; MaxSeq as usize + 1],
    pub wksp: [U32; ZSTD_BuildCTableWksp_wksp_size_u32],
}

/// ```c
/// size_t
/// ZSTD_buildCTable(void* dst, size_t dstCapacity,
///                 FSE_CTable* nextCTable, U32 FSELog, SymbolEncodingType_e type,
///                 unsigned* count, U32 max,
///                 const BYTE* codeTable, size_t nbSeq,
///                 const S16* defaultNorm, U32 defaultNormLog, U32 defaultMax,
///                 const FSE_CTable* prevCTable, size_t prevCTableSize,
///                 void* entropyWorkspace, size_t entropyWorkspaceSize);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildCTable(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    nextCTable: *mut FSE_CTable,
    FSELog: U32,
    type_: SymbolEncodingType_e,
    count: *mut core::ffi::c_uint,
    max: U32,
    codeTable: *const BYTE,
    nbSeq: usize,
    defaultNorm: *const S16,
    defaultNormLog: U32,
    defaultMax: U32,
    prevCTable: *const FSE_CTable,
    prevCTableSize: usize,
    entropyWorkspace: *mut core::ffi::c_void,
    entropyWorkspaceSize: usize,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    let oend: *const BYTE = op.wrapping_add(dstCapacity) as *const BYTE;

    match type_ {
        crate::zstd_internal::set_rle => {
            {
                let err_code = crate::fse_compress::FSE_buildCTable_rle(nextCTable, max as BYTE);
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            if dstCapacity == 0 {
                return ERROR(ZSTD_error_dstSize_tooSmall);
            }
            *op = *codeTable.add(0);
            return 1;
        }
        crate::zstd_internal::set_repeat => {
            ZSTD_memcpy(
                nextCTable as *mut u8,
                prevCTable as *const u8,
                prevCTableSize,
            );
            return 0;
        }
        crate::zstd_internal::set_basic => {
            /* note : could be pre-calculated */
            let err_code = crate::fse_compress::FSE_buildCTable_wksp(
                nextCTable,
                defaultNorm,
                defaultMax,
                defaultNormLog,
                entropyWorkspace,
                entropyWorkspaceSize,
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
            return 0;
        }
        crate::zstd_internal::set_compressed => {
            let wksp: *mut ZSTD_BuildCTableWksp = entropyWorkspace as *mut ZSTD_BuildCTableWksp;
            let mut nbSeq_1: usize = nbSeq;
            let tableLog: U32 = crate::fse_compress::FSE_optimalTableLog(FSELog, nbSeq, max);
            if *count.add(*codeTable.add(nbSeq - 1) as usize) > 1 {
                *count.add(*codeTable.add(nbSeq - 1) as usize) =
                    (*count.add(*codeTable.add(nbSeq - 1) as usize)).wrapping_sub(1);
                nbSeq_1 = nbSeq_1.wrapping_sub(1);
            }
            {
                let err_code = crate::fse_compress::FSE_normalizeCount(
                    core::ptr::addr_of_mut!((*wksp).norm) as *mut core::ffi::c_short,
                    tableLog,
                    count,
                    nbSeq_1,
                    max,
                    ZSTD_useLowProbCount(nbSeq_1),
                );
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            {
                /* overflow protected */
                let NCountSize: usize = crate::fse_compress::FSE_writeNCount(
                    op as *mut core::ffi::c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    core::ptr::addr_of!((*wksp).norm) as *const core::ffi::c_short,
                    max,
                    tableLog,
                );
                {
                    let err_code = NCountSize;
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                }
                {
                    let err_code = crate::fse_compress::FSE_buildCTable_wksp(
                        nextCTable,
                        core::ptr::addr_of!((*wksp).norm) as *const core::ffi::c_short,
                        max,
                        tableLog,
                        core::ptr::addr_of_mut!((*wksp).wksp) as *mut core::ffi::c_void,
                        ZSTD_BuildCTableWksp_wksp_size_u32 * core::mem::size_of::<U32>(),
                    );
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                }
                return NCountSize;
            }
        }
        _ => {
            return ERROR(ZSTD_error_GENERIC);
        }
    }
}

/// `FORCE_INLINE_TEMPLATE size_t ZSTD_encodeSequences_body(...)`
#[inline(always)]
pub unsafe fn ZSTD_encodeSequences_body(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    CTable_MatchLength: *const FSE_CTable,
    mlCodeTable: *const BYTE,
    CTable_OffsetBits: *const FSE_CTable,
    ofCodeTable: *const BYTE,
    CTable_LitLength: *const FSE_CTable,
    llCodeTable: *const BYTE,
    sequences: *const SeqDef,
    nbSeq: usize,
    longOffsets: core::ffi::c_int,
) -> usize {
    let mut blockStream: BIT_CStream_t = BIT_CStream_t::default();
    let mut stateMatchLength: FSE_CState_t = FSE_CState_t::default();
    let mut stateOffsetBits: FSE_CState_t = FSE_CState_t::default();
    let mut stateLitLength: FSE_CState_t = FSE_CState_t::default();

    if ERR_isError(BIT_initCStream(
        &mut blockStream,
        dst as *mut u8,
        dstCapacity,
    )) != 0
    {
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
        LL_bits[*llCodeTable.add(nbSeq - 1) as usize] as U32,
    );
    if MEM_32bits() != 0 {
        BIT_flushBits(&mut blockStream);
    }
    BIT_addBits(
        &mut blockStream,
        (*sequences.add(nbSeq - 1)).mlBase as BitContainerType,
        ML_bits[*mlCodeTable.add(nbSeq - 1) as usize] as U32,
    );
    if MEM_32bits() != 0 {
        BIT_flushBits(&mut blockStream);
    }
    if longOffsets != 0 {
        let ofBits: U32 = *ofCodeTable.add(nbSeq - 1) as U32;
        let extraBits: core::ffi::c_uint =
            ofBits.wrapping_sub(MIN(ofBits, STREAM_ACCUMULATOR_MIN().wrapping_sub(1)));
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
            ofBits.wrapping_sub(extraBits),
        );
    } else {
        BIT_addBits(
            &mut blockStream,
            (*sequences.add(nbSeq - 1)).offBase as BitContainerType,
            *ofCodeTable.add(nbSeq - 1) as U32,
        );
    }
    BIT_flushBits(&mut blockStream);

    {
        let mut n: usize;
        n = nbSeq.wrapping_sub(2);
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
            FSE_encodeSymbol(
                &mut blockStream,
                &mut stateOffsetBits,
                ofCode as core::ffi::c_uint,
            ); /* 15 */ /* 15 */
            FSE_encodeSymbol(
                &mut blockStream,
                &mut stateMatchLength,
                mlCode as core::ffi::c_uint,
            ); /* 24 */ /* 24 */
            if MEM_32bits() != 0 {
                BIT_flushBits(&mut blockStream);
            } /* (7)*/
            FSE_encodeSymbol(
                &mut blockStream,
                &mut stateLitLength,
                llCode as core::ffi::c_uint,
            ); /* 16 */ /* 33 */
            if MEM_32bits() != 0
                || (ofBits.wrapping_add(mlBits).wrapping_add(llBits)
                    >= 64u32 - 7 - (LLFSELog + MLFSELog + OffFSELog))
            {
                BIT_flushBits(&mut blockStream); /* (7)*/
            }
            BIT_addBits(
                &mut blockStream,
                (*sequences.add(n)).litLength as BitContainerType,
                llBits,
            );
            if MEM_32bits() != 0 && ((llBits.wrapping_add(mlBits)) > 24) {
                BIT_flushBits(&mut blockStream);
            }
            BIT_addBits(
                &mut blockStream,
                (*sequences.add(n)).mlBase as BitContainerType,
                mlBits,
            );
            if MEM_32bits() != 0 || (ofBits.wrapping_add(mlBits).wrapping_add(llBits) > 56) {
                BIT_flushBits(&mut blockStream);
            }
            if longOffsets != 0 {
                let extraBits: core::ffi::c_uint =
                    ofBits.wrapping_sub(MIN(ofBits, STREAM_ACCUMULATOR_MIN().wrapping_sub(1)));
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
                    ofBits.wrapping_sub(extraBits),
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
        return streamSize;
    }
}

/// `static size_t ZSTD_encodeSequences_default(...)`
pub unsafe fn ZSTD_encodeSequences_default(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    CTable_MatchLength: *const FSE_CTable,
    mlCodeTable: *const BYTE,
    CTable_OffsetBits: *const FSE_CTable,
    ofCodeTable: *const BYTE,
    CTable_LitLength: *const FSE_CTable,
    llCodeTable: *const BYTE,
    sequences: *const SeqDef,
    nbSeq: usize,
    longOffsets: core::ffi::c_int,
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

/* #if DYNAMIC_BMI2 : DYNAMIC_BMI2 == 0, so ZSTD_encodeSequences_bmi2 does not exist. */

/// ```c
/// size_t ZSTD_encodeSequences(
///             void* dst, size_t dstCapacity,
///             FSE_CTable const* CTable_MatchLength, BYTE const* mlCodeTable,
///             FSE_CTable const* CTable_OffsetBits, BYTE const* ofCodeTable,
///             FSE_CTable const* CTable_LitLength, BYTE const* llCodeTable,
///             SeqDef const* sequences, size_t nbSeq, int longOffsets, int bmi2);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_encodeSequences(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    CTable_MatchLength: *const FSE_CTable,
    mlCodeTable: *const BYTE,
    CTable_OffsetBits: *const FSE_CTable,
    ofCodeTable: *const BYTE,
    CTable_LitLength: *const FSE_CTable,
    llCodeTable: *const BYTE,
    sequences: *const SeqDef,
    nbSeq: usize,
    longOffsets: core::ffi::c_int,
    bmi2: core::ffi::c_int,
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
