//!
//! Literal, semantics-preserving transliteration of `zstd_ldm.c` +
//! `zstd_ldm.h` + `zstd_ldm_geartab.h`.
//!
//! Build configuration: `DYNAMIC_BMI2=0`, no `ZSTD_MULTITHREAD`, `DEBUGLEVEL 0`
//! (asserts / DEBUGLOG dropped). No `ZSTD_EXCLUDE_DFAST_BLOCK_COMPRESSOR`
//! defined, so the dfast branch of ZSTD_ldm_fillFastTables compiles.
//!
//! `XXH_NAMESPACE=ZSTD_` makes `XXH64` link as `ZSTD_XXH64`.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_assignments)]

use core::ffi::{c_int, c_uint, c_void};
use core::ptr::null_mut;

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::xxhash::ZSTD_XXH64;
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;

use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::ZSTD_cwksp_alloc_size;
use crate::compress::zstd_double_fast::ZSTD_fillDoubleHashTable;
use crate::compress::zstd_fast::ZSTD_fillHashTable;

/* ZSTD_selectBlockCompressor is defined in zstd_compress.c (not yet
 * translated). It is an exported symbol of the same cdylib, so it will link
 * later. */
unsafe extern "C" {
    fn ZSTD_selectBlockCompressor(
        strat: ZSTD_strategy,
        useRowMatchFinder: ZSTD_ParamSwitch_e,
        dictMode: ZSTD_dictMode_e,
    ) -> ZSTD_BlockCompressor_f;
}

/* PREFETCH_L1: a no-op prefetch hint (does not change observable behaviour). */
#[inline(always)]
unsafe fn PREFETCH_L1<T>(_ptr: *const T) {}

const LDM_BUCKET_SIZE_LOG: U32 = 4;
const LDM_MIN_MATCH_LENGTH: U32 = 64;
const LDM_HASH_RLOG: U32 = 7;

/* ==========================================================================
 *  zstd_ldm_geartab.h : ZSTD_ldm_gearTab, transcribed EXACTLY.
 * ========================================================================== */
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

/* ==========================================================================
 *  zstd_ldm.c
 * ========================================================================== */

#[repr(C)]
#[derive(Clone, Copy)]
struct ldmRollingHashState_t {
    rolling: U64,
    stopMask: U64,
}

/** ZSTD_ldm_gear_init():
 * Initializes the rolling hash state such that it will honor the
 * settings in params. */
unsafe fn ZSTD_ldm_gear_init(state: *mut ldmRollingHashState_t, params: *const ldmParams_t) {
    let maxBitsInMask: c_uint = MIN((*params).minMatchLength, 64);
    let hashRateLog: c_uint = (*params).hashRateLog;

    (*state).rolling = !(0 as U32) as U64;

    if hashRateLog > 0 && hashRateLog <= maxBitsInMask {
        (*state).stopMask =
            (((1 as U64) << hashRateLog) - 1) << (maxBitsInMask - hashRateLog);
    } else {
        /* In this degenerate case we simply honor the hash rate. */
        (*state).stopMask = ((1 as U64) << hashRateLog) - 1;
    }
}

/** ZSTD_ldm_gear_reset()
 * Feeds [data, data + minMatchLength) into the hash without registering any
 * splits. This effectively resets the hash state. */
unsafe fn ZSTD_ldm_gear_reset(
    state: *mut ldmRollingHashState_t,
    data: *const BYTE,
    minMatchLength: size_t,
) {
    let mut hash: U64 = (*state).rolling;
    let mut n: size_t = 0;

    macro_rules! GEAR_ITER_ONCE {
        () => {{
            hash = (hash << 1).wrapping_add(
                ZSTD_ldm_gearTab[(*data.wrapping_add(n) & 0xff) as usize],
            );
            n += 1;
        }};
    }
    while n + 3 < minMatchLength {
        GEAR_ITER_ONCE!();
        GEAR_ITER_ONCE!();
        GEAR_ITER_ONCE!();
        GEAR_ITER_ONCE!();
    }
    while n < minMatchLength {
        GEAR_ITER_ONCE!();
    }
    /* note: state->rolling is intentionally NOT written back here (matches C) */
    let _ = hash;
}

/** ZSTD_ldm_gear_feed():
 * Registers in the splits array all the split points found in the first
 * size bytes following the data pointer. Terminates when either all the data
 * has been processed or LDM_BATCH_SIZE splits are present in the splits array.
 *
 * Precondition: The splits array must not be full.
 * Returns: The number of bytes processed. */
unsafe fn ZSTD_ldm_gear_feed(
    state: *mut ldmRollingHashState_t,
    data: *const BYTE,
    size: size_t,
    splits: *mut size_t,
    numSplits: *mut c_uint,
) -> size_t {
    let mut n: size_t;
    let mut hash: U64;
    let mask: U64;

    hash = (*state).rolling;
    mask = (*state).stopMask;
    n = 0;

    /* GEAR_ITER_ONCE with a `done` early-exit; emulated via a labeled block. */
    'done: {
        macro_rules! GEAR_ITER_ONCE {
            () => {{
                hash = (hash << 1).wrapping_add(
                    ZSTD_ldm_gearTab[(*data.wrapping_add(n) & 0xff) as usize],
                );
                n += 1;
                if (hash & mask) == 0 {
                    *splits.wrapping_add(*numSplits as usize) = n;
                    *numSplits += 1;
                    if *numSplits as usize == LDM_BATCH_SIZE {
                        break 'done;
                    }
                }
            }};
        }

        while n + 3 < size {
            GEAR_ITER_ONCE!();
            GEAR_ITER_ONCE!();
            GEAR_ITER_ONCE!();
            GEAR_ITER_ONCE!();
        }
        while n < size {
            GEAR_ITER_ONCE!();
        }
    } /* 'done */

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
            (*params).hashRateLog = 7 - ((*cParams).strategy / 3);
        }
    }
    if (*params).hashLog == 0 {
        (*params).hashLog = BOUNDED(
            ZSTD_HASHLOG_MIN as U32,
            (*params).windowLog - (*params).hashRateLog,
            ZSTD_HASHLOG_MAX as U32,
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
            ZSTD_LDM_BUCKETSIZELOG_MAX as U32,
        );
    }
    (*params).bucketSizeLog = MIN((*params).bucketSizeLog, (*params).hashLog);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getTableSize(params: ldmParams_t) -> size_t {
    let ldmHSize: size_t = (1 as size_t) << params.hashLog;
    let ldmBucketSizeLog: size_t = MIN(params.bucketSizeLog, params.hashLog) as size_t;
    let ldmBucketSize: size_t = (1 as size_t) << (params.hashLog as size_t - ldmBucketSizeLog);
    let totalSize: size_t = ZSTD_cwksp_alloc_size(ldmBucketSize)
        + ZSTD_cwksp_alloc_size(ldmHSize.wrapping_mul(core::mem::size_of::<ldmEntry_t>() as size_t));
    if params.enableLdm == ZSTD_ps_enable {
        totalSize
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_getMaxNbSeq(params: ldmParams_t, maxChunkSize: size_t) -> size_t {
    if params.enableLdm == ZSTD_ps_enable {
        maxChunkSize / params.minMatchLength as size_t
    } else {
        0
    }
}

/** ZSTD_ldm_getBucket() :
 *  Returns a pointer to the start of the bucket associated with hash. */
unsafe fn ZSTD_ldm_getBucket(
    ldmState: *const ldmState_t,
    hash: size_t,
    bucketSizeLog: U32,
) -> *mut ldmEntry_t {
    (*ldmState).hashTable.wrapping_add((hash << bucketSizeLog) as usize)
}

/** ZSTD_ldm_insertEntry() :
 *  Insert the entry with corresponding hash into the hash table */
unsafe fn ZSTD_ldm_insertEntry(
    ldmState: *mut ldmState_t,
    hash: size_t,
    entry: ldmEntry_t,
    bucketSizeLog: U32,
) {
    let pOffset: *mut BYTE = (*ldmState).bucketOffsets.wrapping_add(hash);
    let offset: c_uint = *pOffset as c_uint;

    *(ZSTD_ldm_getBucket(ldmState, hash, bucketSizeLog).wrapping_add(offset as usize)) = entry;
    *pOffset = ((offset + 1) & ((1u32 << bucketSizeLog) - 1)) as BYTE;
}

/** ZSTD_ldm_countBackwardsMatch() :
 *  Returns the number of bytes that match backwards before pIn and pMatch.
 *  We count only bytes where pMatch >= pBase and pIn >= pAnchor. */
unsafe fn ZSTD_ldm_countBackwardsMatch(
    pIn: *const BYTE,
    pAnchor: *const BYTE,
    pMatch: *const BYTE,
    pMatchBase: *const BYTE,
) -> size_t {
    let mut matchLength: size_t = 0;
    let mut pIn = pIn;
    let mut pMatch = pMatch;
    while pIn > pAnchor
        && pMatch > pMatchBase
        && *pIn.wrapping_offset(-1) == *pMatch.wrapping_offset(-1)
    {
        pIn = pIn.wrapping_offset(-1);
        pMatch = pMatch.wrapping_offset(-1);
        matchLength += 1;
    }
    matchLength
}

/** ZSTD_ldm_countBackwardsMatch_2segments() :
 *  Returns the number of bytes that match backwards from pMatch,
 *  even with the backwards match spanning 2 different segments.
 *  On reaching `pMatchBase`, start counting from mEnd */
unsafe fn ZSTD_ldm_countBackwardsMatch_2segments(
    pIn: *const BYTE,
    pAnchor: *const BYTE,
    pMatch: *const BYTE,
    pMatchBase: *const BYTE,
    pExtDictStart: *const BYTE,
    pExtDictEnd: *const BYTE,
) -> size_t {
    let mut matchLength: size_t =
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
 *  Fills the relevant tables for the ZSTD_fast and ZSTD_dfast strategies. */
unsafe fn ZSTD_ldm_fillFastTables(ms: *mut ZSTD_MatchState_t, end: *const c_void) -> size_t {
    let iend: *const BYTE = end as *const BYTE;

    match (*ms).cParams.strategy {
        x if x == ZSTD_fast => {
            ZSTD_fillHashTable(ms, iend as *const c_void, ZSTD_dtlm_fast, ZSTD_tfp_forCCtx);
        }
        x if x == ZSTD_dfast => {
            ZSTD_fillDoubleHashTable(ms, iend as *const c_void, ZSTD_dtlm_fast, ZSTD_tfp_forCCtx);
        }
        x if x == ZSTD_greedy
            || x == ZSTD_lazy
            || x == ZSTD_lazy2
            || x == ZSTD_btlazy2
            || x == ZSTD_btopt
            || x == ZSTD_btultra
            || x == ZSTD_btultra2 => {}
        _ => { /* assert(0) dropped */ }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_fillHashTable(
    ldmState: *mut ldmState_t,
    ip: *const BYTE,
    iend: *const BYTE,
    params: *const ldmParams_t,
) {
    let minMatchLength: U32 = (*params).minMatchLength;
    let bucketSizeLog: U32 = (*params).bucketSizeLog;
    let hBits: U32 = (*params).hashLog - bucketSizeLog;
    let base: *const BYTE = (*ldmState).window.base;
    let istart: *const BYTE = ip;
    let mut hashState: ldmRollingHashState_t = core::mem::zeroed();
    let splits: *mut size_t = (*ldmState).splitIndices.as_mut_ptr();
    let mut numSplits: c_uint;

    let mut ip = ip;

    ZSTD_ldm_gear_init(&mut hashState, params);
    while ip < iend {
        let hashed: size_t;
        let mut n: c_uint;

        numSplits = 0;
        hashed = ZSTD_ldm_gear_feed(
            &mut hashState,
            ip,
            iend.offset_from(ip) as size_t,
            splits,
            &mut numSplits,
        );

        n = 0;
        while n < numSplits {
            if ip.wrapping_add(*splits.wrapping_add(n as usize))
                >= istart.wrapping_add(minMatchLength as usize)
            {
                let split: *const BYTE = ip
                    .wrapping_add(*splits.wrapping_add(n as usize))
                    .wrapping_sub(minMatchLength as usize);
                let xxhash: U64 =
                    ZSTD_XXH64(split as *const c_void, minMatchLength as size_t, 0);
                let hash: U32 = (xxhash & (((1 as U32) << hBits) - 1) as U64) as U32;
                let mut entry: ldmEntry_t = core::mem::zeroed();

                entry.offset = split.offset_from(base) as U32;
                entry.checksum = (xxhash >> 32) as U32;
                ZSTD_ldm_insertEntry(ldmState, hash as size_t, entry, (*params).bucketSizeLog);
            }
            n += 1;
        }

        ip = ip.wrapping_add(hashed);
    }
}

/** ZSTD_ldm_limitTableUpdate() :
 *  Sets cctx->nextToUpdate to a position corresponding closer to anchor
 *  if it is far way (after a long match, only update tables a limited amount). */
unsafe fn ZSTD_ldm_limitTableUpdate(ms: *mut ZSTD_MatchState_t, anchor: *const BYTE) {
    let curr: U32 = anchor.offset_from((*ms).window.base) as U32;
    if curr > (*ms).nextToUpdate + 1024 {
        (*ms).nextToUpdate =
            curr - MIN(512, curr - (*ms).nextToUpdate - 1024);
    }
}

unsafe fn ZSTD_ldm_generateSequences_internal(
    ldmState: *mut ldmState_t,
    rawSeqStore: *mut RawSeqStore_t,
    params: *const ldmParams_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    /* LDM parameters */
    let extDict: c_int = ZSTD_window_hasExtDict((*ldmState).window) as c_int;
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
    let base: *const BYTE = (*ldmState).window.base;
    let dictBase: *const BYTE = if extDict != 0 {
        (*ldmState).window.dictBase
    } else {
        null_mut()
    };
    let dictStart: *const BYTE = if extDict != 0 {
        dictBase.wrapping_add(lowestIndex as usize)
    } else {
        null_mut()
    };
    let dictEnd: *const BYTE = if extDict != 0 {
        dictBase.wrapping_add(dictLimit as usize)
    } else {
        null_mut()
    };
    let lowPrefixPtr: *const BYTE = base.wrapping_add(dictLimit as usize);
    /* Input bounds */
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let ilimit: *const BYTE = iend.wrapping_sub(HASH_READ_SIZE as usize);
    /* Input positions */
    let mut anchor: *const BYTE = istart;
    let mut ip: *const BYTE = istart;
    /* Rolling hash state */
    let mut hashState: ldmRollingHashState_t = core::mem::zeroed();
    /* Arrays for staged-processing */
    let splits: *mut size_t = (*ldmState).splitIndices.as_mut_ptr();
    let candidates: *mut ldmMatchCandidate_t = (*ldmState).matchCandidates.as_mut_ptr();
    let mut numSplits: c_uint;

    if srcSize < minMatchLength as size_t {
        return iend.offset_from(anchor) as size_t;
    }

    /* Initialize the rolling hash state with the first minMatchLength bytes */
    ZSTD_ldm_gear_init(&mut hashState, params);
    ZSTD_ldm_gear_reset(&mut hashState, ip, minMatchLength as size_t);
    ip = ip.wrapping_add(minMatchLength as usize);

    while ip < ilimit {
        let mut hashed: size_t;
        let mut n: c_uint;

        numSplits = 0;
        hashed = ZSTD_ldm_gear_feed(
            &mut hashState,
            ip,
            ilimit.offset_from(ip) as size_t,
            splits,
            &mut numSplits,
        );

        n = 0;
        while n < numSplits {
            let split: *const BYTE = ip
                .wrapping_add(*splits.wrapping_add(n as usize))
                .wrapping_sub(minMatchLength as usize);
            let xxhash: U64 = ZSTD_XXH64(split as *const c_void, minMatchLength as size_t, 0);
            let hash: U32 = (xxhash & (((1 as U32) << hBits) - 1) as U64) as U32;

            (*candidates.wrapping_add(n as usize)).split = split;
            (*candidates.wrapping_add(n as usize)).hash = hash;
            (*candidates.wrapping_add(n as usize)).checksum = (xxhash >> 32) as U32;
            (*candidates.wrapping_add(n as usize)).bucket =
                ZSTD_ldm_getBucket(ldmState, hash as size_t, (*params).bucketSizeLog);
            PREFETCH_L1((*candidates.wrapping_add(n as usize)).bucket);
            n += 1;
        }

        n = 0;
        'nloop: while n < numSplits {
            let mut forwardMatchLength: size_t = 0;
            let mut backwardMatchLength: size_t = 0;
            let mut bestMatchLength: size_t = 0;
            let mLength: size_t;
            let offset: U32;
            let split: *const BYTE = (*candidates.wrapping_add(n as usize)).split;
            let checksum: U32 = (*candidates.wrapping_add(n as usize)).checksum;
            let hash: U32 = (*candidates.wrapping_add(n as usize)).hash;
            let bucket: *mut ldmEntry_t = (*candidates.wrapping_add(n as usize)).bucket;
            let mut cur: *const ldmEntry_t;
            let mut bestEntry: *const ldmEntry_t = null_mut();
            let mut newEntry: ldmEntry_t = core::mem::zeroed();

            newEntry.offset = split.offset_from(base) as U32;
            newEntry.checksum = checksum;

            /* If a split point would generate a sequence overlapping with
             * the previous one, we merely register it in the hash table and
             * move on */
            if split < anchor {
                ZSTD_ldm_insertEntry(ldmState, hash as size_t, newEntry, (*params).bucketSizeLog);
                n += 1;
                continue 'nloop;
            }

            cur = bucket;
            while cur < bucket.wrapping_add(entsPerBucket as usize) {
                let curForwardMatchLength: size_t;
                let curBackwardMatchLength: size_t;
                let curTotalMatchLength: size_t;
                if (*cur).checksum != checksum || (*cur).offset <= lowestIndex {
                    cur = cur.wrapping_add(1);
                    continue;
                }
                if extDict != 0 {
                    let curMatchBase: *const BYTE = if (*cur).offset < dictLimit {
                        dictBase
                    } else {
                        base
                    };
                    let pMatch: *const BYTE = curMatchBase.wrapping_add((*cur).offset as usize);
                    let matchEnd: *const BYTE = if (*cur).offset < dictLimit {
                        dictEnd
                    } else {
                        iend
                    };
                    let lowMatchPtr: *const BYTE = if (*cur).offset < dictLimit {
                        dictStart
                    } else {
                        lowPrefixPtr
                    };
                    curForwardMatchLength =
                        ZSTD_count_2segments(split, pMatch, iend, matchEnd, lowPrefixPtr);
                    if curForwardMatchLength < minMatchLength as size_t {
                        cur = cur.wrapping_add(1);
                        continue;
                    }
                    curBackwardMatchLength = ZSTD_ldm_countBackwardsMatch_2segments(
                        split, anchor, pMatch, lowMatchPtr, dictStart, dictEnd,
                    );
                } else {
                    /* !extDict */
                    let pMatch: *const BYTE = base.wrapping_add((*cur).offset as usize);
                    curForwardMatchLength = ZSTD_count(split, pMatch, iend);
                    if curForwardMatchLength < minMatchLength as size_t {
                        cur = cur.wrapping_add(1);
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
                cur = cur.wrapping_add(1);
            }

            /* No match found -- insert an entry into the hash table
             * and process the next candidate match */
            if bestEntry == null_mut() {
                ZSTD_ldm_insertEntry(ldmState, hash as size_t, newEntry, (*params).bucketSizeLog);
                n += 1;
                continue 'nloop;
            }

            /* Match found */
            offset = (split.offset_from(base) as U32).wrapping_sub((*bestEntry).offset);
            mLength = forwardMatchLength + backwardMatchLength;
            {
                let seq: *mut rawSeq = (*rawSeqStore).seq.wrapping_add((*rawSeqStore).size);

                /* Out of sequence storage */
                if (*rawSeqStore).size == (*rawSeqStore).capacity {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }
                (*seq).litLength =
                    (split.wrapping_sub(backwardMatchLength).offset_from(anchor)) as U32;
                (*seq).matchLength = mLength as U32;
                (*seq).offset = offset;
                (*rawSeqStore).size += 1;
            }

            /* Insert the current entry into the hash table --- it must be
             * done after the previous block to avoid clobbering bestEntry */
            ZSTD_ldm_insertEntry(ldmState, hash as size_t, newEntry, (*params).bucketSizeLog);

            anchor = split.wrapping_add(forwardMatchLength);

            /* If we find a match that ends after the data that we've hashed
             * then we have a repeating, overlapping, pattern. Skip over
             * overlapping matches. */
            if anchor > ip.wrapping_add(hashed) {
                ZSTD_ldm_gear_reset(
                    &mut hashState,
                    anchor.wrapping_sub(minMatchLength as usize),
                    minMatchLength as size_t,
                );
                /* Continue the outer loop at anchor (ip + hashed == anchor). */
                ip = anchor.wrapping_sub(hashed);
                break 'nloop;
            }
            n += 1;
        }

        ip = ip.wrapping_add(hashed);
    }

    iend.offset_from(anchor) as size_t
}

/* ZSTD_ldm_reduceTable() :
 *  reduce table indexes by `reducerValue` */
unsafe fn ZSTD_ldm_reduceTable(table: *mut ldmEntry_t, size: U32, reducerValue: U32) {
    let mut u: U32 = 0;
    while u < size {
        if (*table.wrapping_add(u as usize)).offset < reducerValue {
            (*table.wrapping_add(u as usize)).offset = 0;
        } else {
            (*table.wrapping_add(u as usize)).offset -= reducerValue;
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
    srcSize: size_t,
) -> size_t {
    let maxDist: U32 = 1u32 << (*params).windowLog;
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    let kMaxChunkSize: size_t = 1 << 20;
    let nbChunks: size_t =
        (srcSize / kMaxChunkSize) + ((srcSize % kMaxChunkSize != 0) as size_t);
    let mut chunk: size_t;
    let mut leftoverSize: size_t = 0;

    chunk = 0;
    while chunk < nbChunks && (*sequences).size < (*sequences).capacity {
        let chunkStart: *const BYTE = istart.wrapping_add(chunk * kMaxChunkSize);
        let remaining: size_t = iend.offset_from(chunkStart) as size_t;
        let chunkEnd: *const BYTE = if remaining < kMaxChunkSize {
            iend
        } else {
            chunkStart.wrapping_add(kMaxChunkSize)
        };
        let chunkSize: size_t = chunkEnd.offset_from(chunkStart) as size_t;
        let newLeftoverSize: size_t;
        let prevSize: size_t = (*sequences).size;

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
            null_mut(),
        );
        /* 3. Generate the sequences for the chunk, and get newLeftoverSize. */
        newLeftoverSize = ZSTD_ldm_generateSequences_internal(
            ldmState,
            sequences,
            params,
            chunkStart as *const c_void,
            chunkSize,
        );
        if ZSTD_isError(newLeftoverSize) != 0 {
            return newLeftoverSize;
        }
        /* 4. Add the leftover literals from previous iterations. */
        if prevSize < (*sequences).size {
            (*(*sequences).seq.wrapping_add(prevSize)).litLength += leftoverSize as U32;
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
    srcSize: size_t,
    minMatch: U32,
) {
    let mut srcSize = srcSize;
    while srcSize > 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let seq: *mut rawSeq = (*rawSeqStore).seq.wrapping_add((*rawSeqStore).pos);
        if srcSize <= (*seq).litLength as size_t {
            /* Skip past srcSize literals */
            (*seq).litLength -= srcSize as U32;
            return;
        }
        srcSize -= (*seq).litLength as size_t;
        (*seq).litLength = 0;
        if srcSize < (*seq).matchLength as size_t {
            /* Skip past the first srcSize of the match */
            (*seq).matchLength -= srcSize as U32;
            if (*seq).matchLength < minMatch {
                /* The match is too short, omit it */
                if (*rawSeqStore).pos + 1 < (*rawSeqStore).size {
                    (*seq.wrapping_add(1)).litLength += (*seq.wrapping_add(0)).matchLength;
                }
                (*rawSeqStore).pos += 1;
            }
            return;
        }
        srcSize -= (*seq).matchLength as size_t;
        (*seq).matchLength = 0;
        (*rawSeqStore).pos += 1;
    }
}

/**
 * If the sequence length is longer than remaining then the sequence is split
 * between this block and the next.
 * Returns the current sequence to handle, or if the rest of the block should
 * be literals, it returns a sequence with offset == 0. */
unsafe fn maybeSplitSequence(
    rawSeqStore: *mut RawSeqStore_t,
    remaining: U32,
    minMatch: U32,
) -> rawSeq {
    let mut sequence: rawSeq = *(*rawSeqStore).seq.wrapping_add((*rawSeqStore).pos);
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
    ZSTD_ldm_skipSequences(rawSeqStore, remaining as size_t, minMatch);
    sequence
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_ldm_skipRawSeqStoreBytes(
    rawSeqStore: *mut RawSeqStore_t,
    nbBytes: size_t,
) {
    let mut currPos: U32 = ((*rawSeqStore).posInSequence + nbBytes) as U32;
    while currPos != 0 && (*rawSeqStore).pos < (*rawSeqStore).size {
        let currSeq: rawSeq = *(*rawSeqStore).seq.wrapping_add((*rawSeqStore).pos);
        if currPos >= currSeq.litLength + currSeq.matchLength {
            currPos -= currSeq.litLength + currSeq.matchLength;
            (*rawSeqStore).pos += 1;
        } else {
            (*rawSeqStore).posInSequence = currPos as size_t;
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
    rep: *mut U32, /* rep[ZSTD_REP_NUM] */
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let cParams: *const ZSTD_compressionParameters = &(*ms).cParams;
    let minMatch: c_uint = (*cParams).minMatch;
    let blockCompressor: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
        (*cParams).strategy,
        useRowMatchFinder,
        ZSTD_matchState_dictMode(ms),
    );
    /* Input bounds */
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.wrapping_add(srcSize);
    /* Input positions */
    let mut ip: *const BYTE = istart;

    /* If using opt parser, use LDMs only as candidates rather than always accepting them */
    if (*cParams).strategy >= ZSTD_btopt {
        let lastLLSize: size_t;
        (*ms).ldmSeqStore = rawSeqStore;
        lastLLSize =
            (blockCompressor.unwrap_unchecked())(ms, seqStore, rep, src, srcSize);
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
            let mut i: c_int;
            let newLitLength: size_t = (blockCompressor.unwrap_unchecked())(
                ms,
                seqStore,
                rep,
                ip as *const c_void,
                sequence.litLength as size_t,
            );
            ip = ip.wrapping_add(sequence.litLength as usize);
            /* Update the repcodes */
            i = ZSTD_REP_NUM as c_int - 1;
            while i > 0 {
                *rep.wrapping_add(i as usize) = *rep.wrapping_add((i - 1) as usize);
                i -= 1;
            }
            *rep.wrapping_add(0) = sequence.offset;
            /* Store the sequence */
            ZSTD_storeSeq(
                seqStore,
                newLitLength,
                ip.wrapping_sub(newLitLength),
                iend,
                OFFSET_TO_OFFBASE(sequence.offset),
                sequence.matchLength as size_t,
            );
            ip = ip.wrapping_add(sequence.matchLength as usize);
        }
    }
    /* Fill the tables for the block compressor */
    ZSTD_ldm_limitTableUpdate(ms, ip);
    ZSTD_ldm_fillFastTables(ms, ip as *const c_void);
    /* Compress the last literals */
    (blockCompressor.unwrap_unchecked())(ms, seqStore, rep, ip as *const c_void, iend.offset_from(ip) as size_t)
}
