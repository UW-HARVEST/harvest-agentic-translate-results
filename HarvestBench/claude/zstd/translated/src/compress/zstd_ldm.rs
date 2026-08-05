/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

use core::ffi::{c_uint, c_void};

use crate::common::error;
use crate::common::mem::{U32, U64};
use crate::common::xxhash::ZSTD_XXH64;
use crate::compress::zstd_compress_internal::{
    ldmEntry_t, ldmMatchCandidate_t, ldmParams_t, ldmState_t, rawSeq, RawSeqStore_t,
    SeqStore_t, ZSTD_BlockCompressor_f, ZSTD_MatchState_t, ZSTD_count, ZSTD_count_2segments,
    ZSTD_dictMode_e, ZSTD_dictTableLoadMethod_e, ZSTD_matchState_dictMode, ZSTD_storeSeq,
    ZSTD_tableFillPurpose_e, ZSTD_window_correctOverflow, ZSTD_window_enforceMaxDist,
    ZSTD_window_hasExtDict, ZSTD_window_needOverflowCorrection, ZSTD_dtlm_fast,
    ZSTD_tfp_forCCtx, OFFSET_TO_OFFBASE, ZSTD_ParamSwitch_e, ZSTD_ps_enable, HASH_READ_SIZE,
    LDM_BATCH_SIZE,
};
use crate::compress::zstd_cwksp::ZSTD_cwksp_alloc_size;
use crate::common::zstd_internal::ZSTD_REP_NUM;
use crate::zstd_h::{
    ZSTD_btopt, ZSTD_btultra, ZSTD_compressionParameters, ZSTD_dfast, ZSTD_fast, ZSTD_strategy,
};

/*-*************************************
*  gear hash table (verbatim)
***************************************/
static ZSTD_ldm_gearTab: [U64; 256] = [
    0xf5b8f72c5f77775c, 0x84935f266b7ac412, 0xb647ada9ca730ccc,
    0xb065bb4b114fb1de, 0x34584e7e8c3a9fd0, 0x4e97e17c6ae26b05,
    0x3a03d743bc99a604, 0xcecd042422c4044f, 0x76de76c58524259e,
    0x9c8528f65badeaca, 0x86563706e2097529, 0x2902475fa375d889,
    0xafb32a9739a5ebe6, 0xce2714da3883e639, 0x21eaf821722e69e,
    0x37b628620b628,    0x49a8d455d88caf5,  0x8556d711e6958140,
    0x4f7ae74fc605c1f,  0x829f0c3468bd3a20, 0x4ffdc885c625179e,
    0x8473de048a3daf1b, 0x51008822b05646b2, 0x69d75d12b2d1cc5f,
    0x8c9d4a19159154bc, 0xc3cc10f4abbd4003, 0xd06ddc1cecb97391,
    0xbe48e6e7ed80302e, 0x3481db31cee03547, 0xacc3f67cdaa1d210,
    0x65cb771d8c7f96cc, 0x8eb27177055723dd, 0xc789950d44cd94be,
    0x934feadc3700b12b, 0x5e485f11edbdf182, 0x1e2e2a46fd64767a,
    0x2969ca71d82efa7c, 0x9d46e9935ebbba2e, 0xe056b67e05e6822b,
    0x94d73f55739d03a0, 0xcd7010bdb69b5a03, 0x455ef9fcd79b82f4,
    0x869cb54a8749c161, 0x38d1a4fa6185d225, 0xb475166f94bbe9bb,
    0xa4143548720959f1, 0x7aed4780ba6b26ba, 0xd0ce264439e02312,
    0x84366d746078d508, 0xa8ce973c72ed17be, 0x21c323a29a430b01,
    0x9962d617e3af80ee, 0xab0ce91d9c8cf75b, 0x530e8ee6d19a4dbc,
    0x2ef68c0cf53f5d72, 0xc03a681640a85506, 0x496e4e9f9c310967,
    0x78580472b59b14a0, 0x273824c23b388577, 0x66bf923ad45cb553,
    0x47ae1a5a2492ba86, 0x35e304569e229659, 0x4765182a46870b6f,
    0x6cbab625e9099412, 0xddac9a2e598522c1, 0x7172086e666624f2,
    0xdf5003ca503b7837, 0x88c0c1db78563d09, 0x58d51865acfc289d,
    0x177671aec65224f1, 0xfb79d8a241e967d7, 0x2be1e101cad9a49a,
    0x6625682f6e29186b, 0x399553457ac06e50, 0x35dffb4c23abb74,
    0x429db2591f54aade, 0xc52802a8037d1009, 0x6acb27381f0b25f3,
    0xf45e2551ee4f823b, 0x8b0ea2d99580c2f7, 0x3bed519cbcb4e1e1,
    0xff452823dbb010a,  0x9d42ed614f3dd267, 0x5b9313c06257c57b,
    0xa114b8008b5e1442, 0xc1fe311c11c13d4b, 0x66e8763ea34c5568,
    0x8b982af1c262f05d, 0xee8876faaa75fbb7, 0x8a62a4d0d172bb2a,
    0xc13d94a3b7449a97, 0x6dbbba9dc15d037c, 0xc786101f1d92e0f1,
    0xd78681a907a0b79b, 0xf61aaf2962c9abb9, 0x2cfd16fcd3cb7ad9,
    0x868c5b6744624d21, 0x25e650899c74ddd7, 0xba042af4a7c37463,
    0x4eb1a539465a3eca, 0xbe09dbf03b05d5ca, 0x774e5a362b5472ba,
    0x47a1221229d183cd, 0x504b0ca18ef5a2df, 0xdffbdfbde2456eb9,
    0x46cd2b2fbee34634, 0xf2aef8fe819d98c3, 0x357f5276d4599d61,
    0x24a5483879c453e3, 0x88026889192b4b9,  0x28da96671782dbec,
    0x4ef37c40588e9aaa, 0x8837b90651bc9fb3, 0xc164f741d3f0e5d6,
    0xbc135a0a704b70ba, 0x69cd868f7622ada,  0xbc37ba89e0b9c0ab,
    0x47c14a01323552f6, 0x4f00794bacee98bb, 0x7107de7d637a69d5,
    0x88af793bb6f2255e, 0xf3c6466b8799b598, 0xc288c616aa7f3b59,
    0x81ca63cf42fca3fd, 0x88d85ace36a2674b, 0xd056bd3792389e7,
    0xe55c396c4e9dd32d, 0xbefb504571e6c0a6, 0x96ab32115e91e8cc,
    0xbf8acb18de8f38d1, 0x66dae58801672606, 0x833b6017872317fb,
    0xb87c16f2d1c92864, 0xdb766a74e58b669c, 0x89659f85c61417be,
    0xc8daad856011ea0c, 0x76a4b565b6fe7eae, 0xa469d085f6237312,
    0xaaf0365683a3e96c, 0x4dbb746f8424f7b8, 0x638755af4e4acc1,
    0x3d7807f5bde64486, 0x17be6d8f5bbb7639, 0x903f0cd44dc35dc,
    0x67b672eafdf1196c, 0xa676ff93ed4c82f1, 0x521d1004c5053d9d,
    0x37ba9ad09ccc9202, 0x84e54d297aacfb51, 0xa0b4b776a143445,
    0x820d471e20b348e,  0x1874383cb83d46dc, 0x97edeec7a1efe11c,
    0xb330e50b1bdc42aa, 0x1dd91955ce70e032, 0xa514cdb88f2939d5,
    0x2791233fd90db9d3, 0x7b670a4cc50f7a9b, 0x77c07d2a05c6dfa5,
    0xe3778b6646d0a6fa, 0xb39c8eda47b56749, 0x933ed448addbef28,
    0xaf846af6ab7d0bf4, 0xe5af208eb666e49,  0x5e6622f73534cd6a,
    0x297daeca42ef5b6e, 0x862daef3d35539a6, 0xe68722498f8e1ea9,
    0x981c53093dc0d572, 0xfa09b0bfbf86fbf5, 0x30b1e96166219f15,
    0x70e7d466bdc4fb83, 0x5a66736e35f2a8e9, 0xcddb59d2b7c1baef,
    0xd6c7d247d26d8996, 0xea4e39eac8de1ba3, 0x539c8bb19fa3aff2,
    0x9f90e4c5fd508d8,  0xa34e5956fbaf3385, 0x2e2f8e151d3ef375,
    0x173691e9b83faec1, 0xb85a8d56bf016379, 0x8382381267408ae3,
    0xb90f901bbdc0096d, 0x7c6ad32933bcec65, 0x76bb5e2f2c8ad595,
    0x390f851a6cf46d28, 0xc3e6064da1c2da72, 0xc52a0c101cfa5389,
    0xd78eaf84a3fbc530, 0x3781b9e2288b997e, 0x73c2f6dea83d05c4,
    0x4228e364c5b5ed7,  0x9d7a3edf0da43911, 0x8edcfeda24686756,
    0x5e7667a7b7a9b3a1, 0x4c4f389fa143791d, 0xb08bc1023da7cddc,
    0x7ab4be3ae529b1cc, 0x754e6132dbe74ff9, 0x71635442a839df45,
    0x2f6fb1643fbe52de, 0x961e0a42cf7a8177, 0xf3b45d83d89ef2ea,
    0xee3de4cf4a6e3e9b, 0xcd6848542c3295e7, 0xe4cee1664c78662f,
    0x9947548b474c68c4, 0x25d73777a5ed8b0b, 0xc915b1d636b7fc,
    0x21c2ba75d9b0d2da, 0x5f6b5dcf608a64a1, 0xdcf333255ff9570c,
    0x633b922418ced4ee, 0xc136dde0b004b34a, 0x58cc83b05d4b2f5a,
    0x5eb424dda28e42d2, 0x62df47369739cd98, 0xb4e0b42485e4ce17,
    0x16e1f0c1f9a8d1e7, 0x8ec3916707560ebf, 0x62ba6e2df2cc9db3,
    0xcbf9f4ff77d83a16, 0x78d9d7d07d2bbcc4, 0xef554ce1e02c41f4,
    0x8d7581127eccf94d, 0xa9b53336cb3c8a05, 0x38c42c0bf45c4f91,
    0x640893cdf4488863, 0x80ec34bc575ea568, 0x39f324f5b48eaa40,
    0xe9d9ed1f8eff527f, 0x9224fc058cc5a214, 0xbaba00b04cfe7741,
    0x309a9f120fcf52af, 0xa558f3ec65626212, 0x424bec8b7adabe2f,
    0x41622513a6aea433, 0xb88da2d5324ca798, 0xd287733b245528a4,
    0x9a44697e6d68aec3, 0x7b1093be2f49bb28, 0x50bbec632e3d8aad,
    0x6cd90723e1ea8283, 0x897b9e7431b02bf3, 0x219efdcb338a7047,
    0x3b0311f0a27c0656, 0xdb17bf91c0db96e7, 0x8cd4fd6b4e85a5b2,
    0xfab071054ba6409d, 0x40d6fe831fa9dfd9, 0xaf358debad7d791e,
    0xeb8d0e25a65e3e58, 0xbbcbd3df14e08580, 0xcf751f27ecdab2b,
    0x2b4da14f2613d8f4,
];

/*-*************************************
*  external symbols (siblings)
***************************************/
extern "C" {
    /* zstd_fast.c */
    fn ZSTD_fillHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    /* zstd_double_fast.c */
    fn ZSTD_fillDoubleHashTable(
        ms: *mut ZSTD_MatchState_t,
        end: *const c_void,
        dtlm: ZSTD_dictTableLoadMethod_e,
        tfp: ZSTD_tableFillPurpose_e,
    );
    /* zstd_compress.c */
    fn ZSTD_selectBlockCompressor(
        strat: ZSTD_strategy,
        useRowMatchFinder: ZSTD_ParamSwitch_e,
        dictMode: ZSTD_dictMode_e,
    ) -> ZSTD_BlockCompressor_f;
}

const LDM_BUCKET_SIZE_LOG: U32 = 4;
const LDM_MIN_MATCH_LENGTH: U32 = 64;
#[allow(dead_code)]
const LDM_HASH_RLOG: U32 = 7;

const ZSTD_LDM_BUCKETSIZELOG_MAX: U32 = 8;
const ZSTD_HASHLOG_MIN: U32 = 6;
const ZSTD_HASHLOG_MAX: U32 = 30;

#[inline]
fn PREFETCH_L1(_ptr: *const c_void) {}

#[repr(C)]
struct ldmRollingHashState_t {
    rolling: U64,
    stopMask: U64,
}

/** ZSTD_ldm_gear_init():
 *
 * Initializes the rolling hash state such that it will honor the
 * settings in params. */
unsafe fn ZSTD_ldm_gear_init(state: *mut ldmRollingHashState_t, params: *const ldmParams_t) {
    let maxBitsInMask: c_uint = ((*params).minMatchLength).min(64);
    let hashRateLog: c_uint = (*params).hashRateLog;

    (*state).rolling = !(0u32 as U64);

    /* The choice of the splitting criterion is subject to two conditions:
     *   1. it has to trigger on average every 2^(hashRateLog) bytes;
     *   2. ideally, it has to depend on a window of minMatchLength bytes.
     *
     * In the gear hash algorithm, bit n depends on the last n bytes;
     * so in order to obtain a good quality splitting criterion it is
     * preferable to use bits with high weight.
     *
     * To match condition 1 we use a mask with hashRateLog bits set
     * and, because of the previous remark, we make sure these bits
     * have the highest possible weight while still respecting
     * condition 2.
     */
    if hashRateLog > 0 && hashRateLog <= maxBitsInMask {
        (*state).stopMask =
            (((1u64 << hashRateLog) - 1) as U64) << (maxBitsInMask - hashRateLog);
    } else {
        /* In this degenerate case we simply honor the hash rate. */
        (*state).stopMask = (1u64 << hashRateLog) - 1;
    }
}

/** ZSTD_ldm_gear_reset()
 * Feeds [data, data + minMatchLength) into the hash without registering any
 * splits. This effectively resets the hash state. This is used when skipping
 * over data, either at the beginning of a block, or skipping sections.
 */
unsafe fn ZSTD_ldm_gear_reset(
    state: *mut ldmRollingHashState_t,
    data: *const u8,
    minMatchLength: usize,
) {
    let mut hash: U64 = (*state).rolling;
    let mut n: usize = 0;

    macro_rules! gear_iter_once {
        () => {{
            hash = (hash << 1).wrapping_add(ZSTD_ldm_gearTab[(*data.add(n) & 0xff) as usize]);
            n += 1;
        }};
    }
    while n + 3 < minMatchLength {
        gear_iter_once!();
        gear_iter_once!();
        gear_iter_once!();
        gear_iter_once!();
    }
    while n < minMatchLength {
        gear_iter_once!();
    }
    (*state).rolling = hash;
}

/** ZSTD_ldm_gear_feed():
 *
 * Registers in the splits array all the split points found in the first
 * size bytes following the data pointer. This function terminates when
 * either all the data has been processed or LDM_BATCH_SIZE splits are
 * present in the splits array.
 *
 * Precondition: The splits array must not be full.
 * Returns: The number of bytes processed. */
unsafe fn ZSTD_ldm_gear_feed(
    state: *mut ldmRollingHashState_t,
    data: *const u8,
    size: usize,
    splits: *mut usize,
    numSplits: *mut c_uint,
) -> usize {
    let mut hash: U64;
    let mask: U64;
    let mut n: usize;

    hash = (*state).rolling;
    mask = (*state).stopMask;
    n = 0;

    macro_rules! gear_iter_once {
        () => {{
            hash = (hash << 1).wrapping_add(ZSTD_ldm_gearTab[(*data.add(n) & 0xff) as usize]);
            n += 1;
            (hash & mask) == 0 && {
                *splits.add(*numSplits as usize) = n;
                *numSplits += 1;
                *numSplits as usize == LDM_BATCH_SIZE
            }
        }};
    }

    'done: {
        while n + 3 < size {
            if gear_iter_once!() {
                break 'done;
            }
            if gear_iter_once!() {
                break 'done;
            }
            if gear_iter_once!() {
                break 'done;
            }
            if gear_iter_once!() {
                break 'done;
            }
        }
        while n < size {
            if gear_iter_once!() {
                break 'done;
            }
        }
    }

    (*state).rolling = hash;
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_adjustParameters(
    params: *mut ldmParams_t,
    cParams: *const ZSTD_compressionParameters,
) {
    (*params).windowLog = (*cParams).windowLog;
    if (*params).hashRateLog == 0 {
        if (*params).hashLog > 0 {
            /* if params->hashLog is set, derive hashRateLog from it */
            if (*params).windowLog > (*params).hashLog {
                (*params).hashRateLog = (*params).windowLog - (*params).hashLog;
            }
        } else {
            /* mapping from [fast, rate7] to [btultra2, rate4] */
            (*params).hashRateLog = (7 - ((*cParams).strategy / 3)) as U32;
        }
    }
    if (*params).hashLog == 0 {
        (*params).hashLog = BOUNDED(
            ZSTD_HASHLOG_MIN,
            (*params).windowLog.wrapping_sub((*params).hashRateLog),
            ZSTD_HASHLOG_MAX,
        );
    }
    if (*params).minMatchLength == 0 {
        (*params).minMatchLength = LDM_MIN_MATCH_LENGTH;
        if (*cParams).strategy >= ZSTD_btultra {
            (*params).minMatchLength /= 2;
        }
    }
    if (*params).bucketSizeLog == 0 {
        (*params).bucketSizeLog = BOUNDED(
            LDM_BUCKET_SIZE_LOG,
            (*cParams).strategy as U32,
            ZSTD_LDM_BUCKETSIZELOG_MAX,
        );
    }
    (*params).bucketSizeLog = (*params).bucketSizeLog.min((*params).hashLog);
}

#[inline]
fn BOUNDED(min: U32, val: U32, max: U32) -> U32 {
    min.max(val.min(max))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getTableSize(params: ldmParams_t) -> usize {
    let ldmHSize: usize = 1usize << params.hashLog;
    let ldmBucketSizeLog: usize = (params.bucketSizeLog.min(params.hashLog)) as usize;
    let ldmBucketSize: usize = 1usize << ((params.hashLog as usize) - ldmBucketSizeLog);
    let totalSize: usize = ZSTD_cwksp_alloc_size(ldmBucketSize)
        + ZSTD_cwksp_alloc_size(ldmHSize * core::mem::size_of::<ldmEntry_t>());
    if params.enableLdm == ZSTD_ps_enable {
        totalSize
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getMaxNbSeq(params: ldmParams_t, maxChunkSize: usize) -> usize {
    if params.enableLdm == ZSTD_ps_enable {
        maxChunkSize / params.minMatchLength as usize
    } else {
        0
    }
}

/** ZSTD_ldm_getBucket() :
 *  Returns a pointer to the start of the bucket associated with hash. */
unsafe fn ZSTD_ldm_getBucket(
    ldmState: *const ldmState_t,
    hash: usize,
    bucketSizeLog: U32,
) -> *mut ldmEntry_t {
    (*ldmState).hashTable.add(hash << bucketSizeLog)
}

/** ZSTD_ldm_insertEntry() :
 *  Insert the entry with corresponding hash into the hash table */
unsafe fn ZSTD_ldm_insertEntry(
    ldmState: *mut ldmState_t,
    hash: usize,
    entry: ldmEntry_t,
    bucketSizeLog: U32,
) {
    let pOffset: *mut u8 = (*ldmState).bucketOffsets.add(hash);
    let offset: c_uint = *pOffset as c_uint;

    *(ZSTD_ldm_getBucket(ldmState, hash, bucketSizeLog).add(offset as usize)) = entry;
    *pOffset = ((offset + 1) & ((1u32 << bucketSizeLog) - 1)) as u8;
}

/** ZSTD_ldm_countBackwardsMatch() :
 *  Returns the number of bytes that match backwards before pIn and pMatch.
 *
 *  We count only bytes where pMatch >= pBase and pIn >= pAnchor. */
unsafe fn ZSTD_ldm_countBackwardsMatch(
    pIn: *const u8,
    pAnchor: *const u8,
    pMatch: *const u8,
    pMatchBase: *const u8,
) -> usize {
    let mut pIn = pIn;
    let mut pMatch = pMatch;
    let mut matchLength: usize = 0;
    while pIn > pAnchor && pMatch > pMatchBase && *pIn.offset(-1) == *pMatch.offset(-1) {
        pIn = pIn.offset(-1);
        pMatch = pMatch.offset(-1);
        matchLength += 1;
    }
    matchLength
}

/** ZSTD_ldm_countBackwardsMatch_2segments() :
 *  Returns the number of bytes that match backwards from pMatch,
 *  even with the backwards match spanning 2 different segments.
 *
 *  On reaching `pMatchBase`, start counting from mEnd */
unsafe fn ZSTD_ldm_countBackwardsMatch_2segments(
    pIn: *const u8,
    pAnchor: *const u8,
    pMatch: *const u8,
    pMatchBase: *const u8,
    pExtDictStart: *const u8,
    pExtDictEnd: *const u8,
) -> usize {
    let mut matchLength: usize =
        ZSTD_ldm_countBackwardsMatch(pIn, pAnchor, pMatch, pMatchBase);
    if pMatch.wrapping_sub(matchLength) != pMatchBase || pMatchBase == pExtDictStart {
        /* If backwards match is entirely in the extDict or prefix, immediately return */
        return matchLength;
    }
    matchLength += ZSTD_ldm_countBackwardsMatch(
        pIn.wrapping_sub(matchLength),
        pAnchor,
        pExtDictEnd,
        pExtDictStart,
    );
    matchLength
}

/** ZSTD_ldm_fillFastTables() :
 *
 *  Fills the relevant tables for the ZSTD_fast and ZSTD_dfast strategies.
 *  This is similar to ZSTD_loadDictionaryContent.
 *
 *  The tables for the other strategies are filled within their
 *  block compressors. */
unsafe fn ZSTD_ldm_fillFastTables(ms: *mut ZSTD_MatchState_t, end: *const c_void) -> usize {
    let iend: *const u8 = end as *const u8;

    match (*ms).cParams.strategy {
        s if s == ZSTD_fast => {
            ZSTD_fillHashTable(ms, iend as *const c_void, ZSTD_dtlm_fast, ZSTD_tfp_forCCtx);
        }
        s if s == ZSTD_dfast => {
            ZSTD_fillDoubleHashTable(ms, iend as *const c_void, ZSTD_dtlm_fast, ZSTD_tfp_forCCtx);
        }
        _ => {
            /* greedy/lazy/lazy2/btlazy2/btopt/btultra/btultra2 -> nothing */
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_fillHashTable(
    ldmState: *mut ldmState_t,
    ip: *const u8,
    iend: *const u8,
    params: *const ldmParams_t,
) {
    let minMatchLength: U32 = (*params).minMatchLength;
    let bucketSizeLog: U32 = (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog - bucketSizeLog;
    let base: *const u8 = (*ldmState).window.base;
    let istart: *const u8 = ip;
    let mut hashState = ldmRollingHashState_t {
        rolling: 0,
        stopMask: 0,
    };
    let splits: *mut usize = (*ldmState).splitIndices.as_mut_ptr();
    let mut numSplits: c_uint;

    let mut ip = ip;

    ZSTD_ldm_gear_init(&mut hashState, params);
    while ip < iend {
        let hashed: usize;
        let mut n: c_uint;

        numSplits = 0;
        hashed = ZSTD_ldm_gear_feed(
            &mut hashState,
            ip,
            iend.offset_from(ip) as usize,
            splits,
            &mut numSplits,
        );

        n = 0;
        while n < numSplits {
            if ip.add(*splits.add(n as usize)) >= istart.add(minMatchLength as usize) {
                let split: *const u8 =
                    ip.add(*splits.add(n as usize)).wrapping_sub(minMatchLength as usize);
                let xxhash: U64 = ZSTD_XXH64(split as *const c_void, minMatchLength as usize, 0);
                let hash: U32 = (xxhash & (((1u32 << hBits) - 1) as U64)) as U32;
                let mut entry = ldmEntry_t {
                    offset: 0,
                    checksum: 0,
                };

                entry.offset = split.offset_from(base) as U32;
                entry.checksum = (xxhash >> 32) as U32;
                ZSTD_ldm_insertEntry(ldmState, hash as usize, entry, (*params).bucketSizeLog);
            }
            n += 1;
        }

        ip = ip.add(hashed);
    }
}

/** ZSTD_ldm_limitTableUpdate() :
 *
 *  Sets cctx->nextToUpdate to a position corresponding closer to anchor
 *  if it is far way
 *  (after a long match, only update tables a limited amount). */
unsafe fn ZSTD_ldm_limitTableUpdate(ms: *mut ZSTD_MatchState_t, anchor: *const u8) {
    let curr: U32 = anchor.offset_from((*ms).window.base) as U32;
    if curr > (*ms).nextToUpdate + 1024 {
        (*ms).nextToUpdate =
            curr - (512u32).min(curr - (*ms).nextToUpdate - 1024);
    }
}

unsafe fn ZSTD_ldm_generateSequences_internal(
    ldmState: *mut ldmState_t,
    rawSeqStore: *mut RawSeqStore_t,
    params: *const ldmParams_t,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    /* LDM parameters */
    let extDict: i32 = ZSTD_window_hasExtDict((*ldmState).window) as i32;
    let minMatchLength: U32 = (*params).minMatchLength;
    let entsPerBucket: U32 = 1u32 << (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog - (*params).bucketSizeLog;
    /* Prefix and extDict parameters */
    let dictLimit: U32 = (*ldmState).window.dictLimit;
    let lowestIndex: U32 = if extDict != 0 {
        (*ldmState).window.lowLimit
    } else {
        dictLimit
    };
    let base: *const u8 = (*ldmState).window.base;
    let dictBase: *const u8 = if extDict != 0 {
        (*ldmState).window.dictBase
    } else {
        core::ptr::null()
    };
    let dictStart: *const u8 = if extDict != 0 {
        dictBase.add(lowestIndex as usize)
    } else {
        core::ptr::null()
    };
    let dictEnd: *const u8 = if extDict != 0 {
        dictBase.add(dictLimit as usize)
    } else {
        core::ptr::null()
    };
    let lowPrefixPtr: *const u8 = base.add(dictLimit as usize);
    /* Input bounds */
    let istart: *const u8 = src as *const u8;
    let iend: *const u8 = istart.add(srcSize);
    let ilimit: *const u8 = iend.offset(-(HASH_READ_SIZE as isize));
    /* Input positions */
    let mut anchor: *const u8 = istart;
    let mut ip: *const u8 = istart;
    /* Rolling hash state */
    let mut hashState = ldmRollingHashState_t {
        rolling: 0,
        stopMask: 0,
    };
    /* Arrays for staged-processing */
    let splits: *mut usize = (*ldmState).splitIndices.as_mut_ptr();
    let candidates: *mut ldmMatchCandidate_t = (*ldmState).matchCandidates.as_mut_ptr();
    let mut numSplits: c_uint;

    if srcSize < minMatchLength as usize {
        return iend.offset_from(anchor) as usize;
    }

    /* Initialize the rolling hash state with the first minMatchLength bytes */
    ZSTD_ldm_gear_init(&mut hashState, params);
    ZSTD_ldm_gear_reset(&mut hashState, ip, minMatchLength as usize);
    ip = ip.add(minMatchLength as usize);

    while ip < ilimit {
        let mut hashed: usize;
        let mut n: c_uint;

        numSplits = 0;
        hashed = ZSTD_ldm_gear_feed(
            &mut hashState,
            ip,
            ilimit.offset_from(ip) as usize,
            splits,
            &mut numSplits,
        );

        n = 0;
        while n < numSplits {
            let split: *const u8 =
                ip.add(*splits.add(n as usize)).wrapping_sub(minMatchLength as usize);
            let xxhash: U64 = ZSTD_XXH64(split as *const c_void, minMatchLength as usize, 0);
            let hash: U32 = (xxhash & (((1u32 << hBits) - 1) as U64)) as U32;

            (*candidates.add(n as usize)).split = split;
            (*candidates.add(n as usize)).hash = hash;
            (*candidates.add(n as usize)).checksum = (xxhash >> 32) as U32;
            (*candidates.add(n as usize)).bucket =
                ZSTD_ldm_getBucket(ldmState, hash as usize, (*params).bucketSizeLog);
            PREFETCH_L1((*candidates.add(n as usize)).bucket as *const c_void);
            n += 1;
        }

        n = 0;
        'candidate_loop: while n < numSplits {
            let mut forwardMatchLength: usize = 0;
            let mut backwardMatchLength: usize = 0;
            let mut bestMatchLength: usize = 0;
            let mLength: usize;
            let offset: U32;
            let split: *const u8 = (*candidates.add(n as usize)).split;
            let checksum: U32 = (*candidates.add(n as usize)).checksum;
            let hash: U32 = (*candidates.add(n as usize)).hash;
            let bucket: *mut ldmEntry_t = (*candidates.add(n as usize)).bucket;
            let mut cur: *const ldmEntry_t;
            let mut bestEntry: *const ldmEntry_t = core::ptr::null();
            let mut newEntry = ldmEntry_t {
                offset: 0,
                checksum: 0,
            };

            newEntry.offset = split.offset_from(base) as U32;
            newEntry.checksum = checksum;

            /* If a split point would generate a sequence overlapping with
             * the previous one, we merely register it in the hash table and
             * move on */
            if split < anchor {
                ZSTD_ldm_insertEntry(ldmState, hash as usize, newEntry, (*params).bucketSizeLog);
                n += 1;
                continue 'candidate_loop;
            }

            cur = bucket;
            while cur < bucket.add(entsPerBucket as usize) {
                let curForwardMatchLength: usize;
                let curBackwardMatchLength: usize;
                let curTotalMatchLength: usize;
                if (*cur).checksum != checksum || (*cur).offset <= lowestIndex {
                    cur = cur.add(1);
                    continue;
                }
                if extDict != 0 {
                    let curMatchBase: *const u8 = if (*cur).offset < dictLimit {
                        dictBase
                    } else {
                        base
                    };
                    let pMatch: *const u8 = curMatchBase.add((*cur).offset as usize);
                    let matchEnd: *const u8 = if (*cur).offset < dictLimit {
                        dictEnd
                    } else {
                        iend
                    };
                    let lowMatchPtr: *const u8 = if (*cur).offset < dictLimit {
                        dictStart
                    } else {
                        lowPrefixPtr
                    };
                    curForwardMatchLength =
                        ZSTD_count_2segments(split, pMatch, iend, matchEnd, lowPrefixPtr);
                    if curForwardMatchLength < minMatchLength as usize {
                        cur = cur.add(1);
                        continue;
                    }
                    curBackwardMatchLength = ZSTD_ldm_countBackwardsMatch_2segments(
                        split, anchor, pMatch, lowMatchPtr, dictStart, dictEnd,
                    );
                } else {
                    /* !extDict */
                    let pMatch: *const u8 = base.add((*cur).offset as usize);
                    curForwardMatchLength = ZSTD_count(split, pMatch, iend);
                    if curForwardMatchLength < minMatchLength as usize {
                        cur = cur.add(1);
                        continue;
                    }
                    curBackwardMatchLength =
                        ZSTD_ldm_countBackwardsMatch(split, anchor, pMatch, lowPrefixPtr);
                }
                curTotalMatchLength = curForwardMatchLength + curBackwardMatchLength;

                if curTotalMatchLength > bestMatchLength {
                    bestMatchLength = curTotalMatchLength;
                    forwardMatchLength = curForwardMatchLength;
                    backwardMatchLength = curBackwardMatchLength;
                    bestEntry = cur;
                }
                cur = cur.add(1);
            }

            /* No match found -- insert an entry into the hash table
             * and process the next candidate match */
            if bestEntry.is_null() {
                ZSTD_ldm_insertEntry(ldmState, hash as usize, newEntry, (*params).bucketSizeLog);
                n += 1;
                continue 'candidate_loop;
            }

            /* Match found */
            offset = (split.offset_from(base) as U32).wrapping_sub((*bestEntry).offset);
            mLength = forwardMatchLength + backwardMatchLength;
            {
                let seq: *mut rawSeq = (*rawSeqStore).seq.add((*rawSeqStore).size);

                /* Out of sequence storage */
                if (*rawSeqStore).size == (*rawSeqStore).capacity {
                    return error::error(error::code::DSTSIZE_TOOSMALL);
                }
                (*seq).litLength =
                    split.wrapping_sub(backwardMatchLength).offset_from(anchor) as U32;
                (*seq).matchLength = mLength as U32;
                (*seq).offset = offset;
                (*rawSeqStore).size += 1;
            }

            /* Insert the current entry into the hash table --- it must be
             * done after the previous block to avoid clobbering bestEntry */
            ZSTD_ldm_insertEntry(ldmState, hash as usize, newEntry, (*params).bucketSizeLog);

            anchor = split.add(forwardMatchLength);

            /* If we find a match that ends after the data that we've hashed
             * then we have a repeating, overlapping, pattern. E.g. all zeros.
             * If one repetition of the pattern matches our `stopMask` then all
             * repetitions will. We don't need to insert them all into out table,
             * only the first one. So skip over overlapping matches.
             * This is a major speed boost (20x) for compressing a single byte
             * repeated, when that byte ends up in the table.
             */
            if anchor > ip.add(hashed) {
                ZSTD_ldm_gear_reset(
                    &mut hashState,
                    anchor.wrapping_sub(minMatchLength as usize),
                    minMatchLength as usize,
                );
                /* Continue the outer loop at anchor (ip + hashed == anchor). */
                ip = anchor.wrapping_sub(hashed);
                break 'candidate_loop;
            }
            n += 1;
        }

        ip = ip.add(hashed);
    }

    iend.offset_from(anchor) as usize
}

/* ZSTD_ldm_reduceTable() :
 *  reduce table indexes by `reducerValue` */
unsafe fn ZSTD_ldm_reduceTable(table: *mut ldmEntry_t, size: U32, reducerValue: U32) {
    let mut u: U32 = 0;
    while u < size {
        if (*table.add(u as usize)).offset < reducerValue {
            (*table.add(u as usize)).offset = 0;
        } else {
            (*table.add(u as usize)).offset -= reducerValue;
        }
        u += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_generateSequences(
    ldmState: *mut ldmState_t,
    sequences: *mut RawSeqStore_t,
    params: *const ldmParams_t,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let maxDist: U32 = 1u32 << (*params).windowLog;
    let istart: *const u8 = src as *const u8;
    let iend: *const u8 = istart.add(srcSize);
    let kMaxChunkSize: usize = 1 << 20;
    let nbChunks: usize =
        (srcSize / kMaxChunkSize) + ((srcSize % kMaxChunkSize != 0) as usize);
    let mut chunk: usize;
    let mut leftoverSize: usize = 0;

    /* The input could be very large (in zstdmt), so it must be broken up into
     * chunks to enforce the maximum distance and handle overflow correction.
     */
    chunk = 0;
    while chunk < nbChunks && (*sequences).size < (*sequences).capacity {
        let chunkStart: *const u8 = istart.add(chunk * kMaxChunkSize);
        let remaining: usize = iend.offset_from(chunkStart) as usize;
        let chunkEnd: *const u8 = if remaining < kMaxChunkSize {
            iend
        } else {
            chunkStart.add(kMaxChunkSize)
        };
        let chunkSize: usize = chunkEnd.offset_from(chunkStart) as usize;
        let newLeftoverSize: usize;
        let prevSize: usize = (*sequences).size;

        /* 1. Perform overflow correction if necessary. */
        if ZSTD_window_needOverflowCorrection(
            (*ldmState).window,
            0,
            maxDist,
            (*ldmState).loadedDictEnd,
            chunkStart as *const c_void,
            chunkEnd as *const c_void,
        ) != 0
        {
            let ldmHSize: U32 = 1u32 << (*params).hashLog;
            let correction: U32 = ZSTD_window_correctOverflow(
                &mut (*ldmState).window,
                /* cycleLog */ 0,
                maxDist,
                chunkStart as *const c_void,
            );
            ZSTD_ldm_reduceTable((*ldmState).hashTable, ldmHSize, correction);
            /* invalidate dictionaries on overflow correction */
            (*ldmState).loadedDictEnd = 0;
        }
        /* 2. We enforce the maximum offset allowed. */
        ZSTD_window_enforceMaxDist(
            &mut (*ldmState).window,
            chunkEnd as *const c_void,
            maxDist,
            &mut (*ldmState).loadedDictEnd,
            core::ptr::null_mut(),
        );
        /* 3. Generate the sequences for the chunk, and get newLeftoverSize. */
        newLeftoverSize = ZSTD_ldm_generateSequences_internal(
            ldmState,
            sequences,
            params,
            chunkStart as *const c_void,
            chunkSize,
        );
        if error::err_is_error(newLeftoverSize) != 0 {
            return newLeftoverSize;
        }
        /* 4. We add the leftover literals from previous iterations to the first
         *    newly generated sequence, or add the `newLeftoverSize` if none are
         *    generated.
         */
        /* Prepend the leftover literals from the last call */
        if prevSize < (*sequences).size {
            (*(*sequences).seq.add(prevSize)).litLength += leftoverSize as U32;
            leftoverSize = newLeftoverSize;
        } else {
            leftoverSize += chunkSize;
        }
        chunk += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipSequences(
    rawSeqStore: *mut RawSeqStore_t,
    srcSize: usize,
    minMatch: U32,
) {
    let mut srcSize = srcSize;
    while srcSize > 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let seq: *mut rawSeq = (*rawSeqStore).seq.add((*rawSeqStore).pos);
        if srcSize <= (*seq).litLength as usize {
            /* Skip past srcSize literals */
            (*seq).litLength -= srcSize as U32;
            return;
        }
        srcSize -= (*seq).litLength as usize;
        (*seq).litLength = 0;
        if srcSize < (*seq).matchLength as usize {
            /* Skip past the first srcSize of the match */
            (*seq).matchLength -= srcSize as U32;
            if (*seq).matchLength < minMatch {
                /* The match is too short, omit it */
                if (*rawSeqStore).pos + 1 < (*rawSeqStore).size {
                    (*seq.add(1)).litLength += (*seq.add(0)).matchLength;
                }
                (*rawSeqStore).pos += 1;
            }
            return;
        }
        srcSize -= (*seq).matchLength as usize;
        (*seq).matchLength = 0;
        (*rawSeqStore).pos += 1;
    }
}

/**
 * If the sequence length is longer than remaining then the sequence is split
 * between this block and the next.
 *
 * Returns the current sequence to handle, or if the rest of the block should
 * be literals, it returns a sequence with offset == 0.
 */
unsafe fn maybeSplitSequence(
    rawSeqStore: *mut RawSeqStore_t,
    remaining: U32,
    minMatch: U32,
) -> rawSeq {
    let mut sequence: rawSeq = *(*rawSeqStore).seq.add((*rawSeqStore).pos);
    /* Likely: No partial sequence */
    if remaining >= sequence.litLength + sequence.matchLength {
        (*rawSeqStore).pos += 1;
        return sequence;
    }
    /* Cut the sequence short (offset == 0 ==> rest is literals). */
    if remaining <= sequence.litLength {
        sequence.offset = 0;
    } else if remaining < sequence.litLength + sequence.matchLength {
        sequence.matchLength = remaining - sequence.litLength;
        if sequence.matchLength < minMatch {
            sequence.offset = 0;
        }
    }
    /* Skip past `remaining` bytes for the future sequences. */
    ZSTD_ldm_skipSequences(rawSeqStore, remaining as usize, minMatch);
    sequence
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipRawSeqStoreBytes(
    rawSeqStore: *mut RawSeqStore_t,
    nbBytes: usize,
) {
    let mut currPos: U32 = ((*rawSeqStore).posInSequence + nbBytes) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let currSeq: rawSeq = *(*rawSeqStore).seq.add((*rawSeqStore).pos);
        if currPos >= currSeq.litLength + currSeq.matchLength {
            currPos -= currSeq.litLength + currSeq.matchLength;
            (*rawSeqStore).pos += 1;
        } else {
            (*rawSeqStore).posInSequence = currPos as usize;
            break;
        }
    }
    if currPos == 0 || (*rawSeqStore).pos == (*rawSeqStore).size {
        (*rawSeqStore).posInSequence = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_blockCompress(
    rawSeqStore: *mut RawSeqStore_t,
    ms: *mut ZSTD_MatchState_t,
    seqStore: *mut SeqStore_t,
    rep: *mut U32,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let minMatch: c_uint = (*cParams).minMatch;
    let blockCompressor: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
        (*cParams).strategy,
        useRowMatchFinder,
        ZSTD_matchState_dictMode(ms),
    );
    /* Input bounds */
    let istart: *const u8 = src as *const u8;
    let iend: *const u8 = istart.add(srcSize);
    /* Input positions */
    let mut ip: *const u8 = istart;

    /* If using opt parser, use LDMs only as candidates rather than always accepting them */
    if (*cParams).strategy >= ZSTD_btopt {
        let lastLLSize: usize;
        (*ms).ldmSeqStore = rawSeqStore;
        lastLLSize = (blockCompressor.unwrap())(ms, seqStore, rep, src, srcSize);
        ZSTD_ldm_skipRawSeqStoreBytes(rawSeqStore, srcSize);
        return lastLLSize;
    }

    /* Loop through each sequence and apply the block compressor to the literals */
    while (*rawSeqStore).pos < (*rawSeqStore).size && ip < iend {
        /* maybeSplitSequence updates rawSeqStore->pos */
        let sequence: rawSeq =
            maybeSplitSequence(rawSeqStore, iend.offset_from(ip) as U32, minMatch);
        /* End signal */
        if sequence.offset == 0 {
            break;
        }

        /* Fill tables for block compressor */
        ZSTD_ldm_limitTableUpdate(ms, ip);
        ZSTD_ldm_fillFastTables(ms, ip as *const c_void);
        /* Run the block compressor */
        {
            let mut i: i32;
            let newLitLength: usize = (blockCompressor.unwrap())(
                ms,
                seqStore,
                rep,
                ip as *const c_void,
                sequence.litLength as usize,
            );
            ip = ip.add(sequence.litLength as usize);
            /* Update the repcodes */
            i = ZSTD_REP_NUM as i32 - 1;
            while i > 0 {
                *rep.add(i as usize) = *rep.add((i - 1) as usize);
                i -= 1;
            }
            *rep.add(0) = sequence.offset;
            /* Store the sequence */
            ZSTD_storeSeq(
                seqStore,
                newLitLength,
                ip.wrapping_sub(newLitLength),
                iend,
                OFFSET_TO_OFFBASE(sequence.offset),
                sequence.matchLength as usize,
            );
            ip = ip.add(sequence.matchLength as usize);
        }
    }
    /* Fill the tables for the block compressor */
    ZSTD_ldm_limitTableUpdate(ms, ip);
    ZSTD_ldm_fillFastTables(ms, ip as *const c_void);
    /* Compress the last literals */
    (blockCompressor.unwrap())(ms, seqStore, rep, ip as *const c_void, iend.offset_from(ip) as usize)
}
