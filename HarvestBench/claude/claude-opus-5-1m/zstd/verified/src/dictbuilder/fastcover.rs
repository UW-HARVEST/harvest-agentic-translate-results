//! Translation of `dictBuilder/fastcover.c`.
//!
//! The shared COVER types/helpers live in `crate::dictbuilder::cover`.
//! `ZSTD_MULTITHREAD` is not defined, so `POOL_*` runs jobs synchronously.
//! `DISPLAY*` macros only write to stderr and are therefore dropped, but
//! `g_displayLevel` (this translation unit has its own copy, exactly like the C
//! file) and the `LOCALDISPLAYUPDATE` `clock()`/`g_time` bookkeeping are kept.
//! `assert()` is compiled out.
#![allow(dead_code)]

use crate::common::error_private::{
    ERROR, ERR_isError, ZSTD_error_GENERIC, ZSTD_error_dstSize_tooSmall,
    ZSTD_error_memory_allocation, ZSTD_error_parameter_outOfBound, ZSTD_error_srcSize_wrong,
};
use crate::common::mem::{BYTE, U16, U32, U64};
use crate::common::pool::{POOL_add, POOL_create, POOL_ctx, POOL_free};
use crate::common::zstd_internal::{MAX, MIN};
use crate::compress::zstd_compress_internal::{ZSTD_hash6Ptr, ZSTD_hash8Ptr};
use crate::dictbuilder::cover::*;
use crate::libc::{calloc, clock, free, malloc, memcpy, memset};
use core::ffi::{c_int, c_uint, c_void};
use core::mem::size_of;
use core::ptr;

/*-*************************************
*  `zdict.h` types
***************************************/

/// `ZDICT_fastCover_params_t`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZDICT_fastCover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub f: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub accel: c_uint,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

/*-*************************************
*  Cross-module declarations
***************************************/

extern "C" {
    fn ZDICT_finalizeDictionary(
        dstDictBuffer: *mut c_void,
        maxDictSize: usize,
        dictContent: *const c_void,
        dictContentSize: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: ZDICT_params_t,
    ) -> usize;
}

/*-*************************************
*  Constants
***************************************/

/// `#define FASTCOVER_MAX_SAMPLES_SIZE (sizeof(size_t) == 8 ? ((unsigned)-1) : ((unsigned)1 GB))`
const FASTCOVER_MAX_SAMPLES_SIZE: c_uint = c_uint::MAX;
/// `#define FASTCOVER_MAX_F 31`
const FASTCOVER_MAX_F: c_uint = 31;
/// `#define FASTCOVER_MAX_ACCEL 10`
const FASTCOVER_MAX_ACCEL: c_uint = 10;
/// `#define FASTCOVER_DEFAULT_SPLITPOINT 0.75`
const FASTCOVER_DEFAULT_SPLITPOINT: f64 = 0.75;
/// `#define DEFAULT_F 20`
const DEFAULT_F: c_uint = 20;
/// `#define DEFAULT_ACCEL 1`
const DEFAULT_ACCEL: c_uint = 1;

/*-*************************************
*  Console display
***************************************/

/// `static int g_displayLevel = 0;`
static mut g_displayLevel: c_int = 0;

/// glibc `CLOCKS_PER_SEC`
const CLOCKS_PER_SEC: i64 = 1_000_000;
/// `static const clock_t g_refreshRate = CLOCKS_PER_SEC * 15 / 100;`
const g_refreshRate: i64 = CLOCKS_PER_SEC * 15 / 100;
/// `static clock_t g_time = 0;`
static mut g_time: i64 = 0;

/// `LOCALDISPLAYUPDATE()` — the `DISPLAY()` body is a no-op, but the
/// `clock()`/`g_time` bookkeeping is preserved.
#[inline]
unsafe fn LOCALDISPLAYUPDATE(displayLevel: c_int, l: c_int) {
    if displayLevel >= l {
        if (clock() - g_time > g_refreshRate) || (displayLevel >= 4) {
            g_time = clock();
        }
    }
}

/// `DISPLAYUPDATE(l, ...)`
#[inline]
unsafe fn DISPLAYUPDATE(l: c_int) {
    LOCALDISPLAYUPDATE(g_displayLevel, l)
}

/*-*************************************
* Hash Functions
***************************************/

/// Hash the d-byte value pointed to by p and mod 2^f into the frequency vector
unsafe fn FASTCOVER_hashPtrToIndex(p: *const c_void, f: U32, d: c_uint) -> usize {
    if d == 6 {
        return ZSTD_hash6Ptr(p, f);
    }
    ZSTD_hash8Ptr(p, f)
}

/*-*************************************
* Acceleration
***************************************/

/// `FASTCOVER_accel_t`
#[repr(C)]
#[derive(Copy, Clone)]
struct FASTCOVER_accel_t {
    /// Percentage of training samples used for `ZDICT_finalizeDictionary`
    finalize: c_uint,
    /// Number of dmer skipped between each dmer counted in computeFrequency
    skip: c_uint,
}

static FASTCOVER_defaultAccelParameters: [FASTCOVER_accel_t; FASTCOVER_MAX_ACCEL as usize + 1] = [
    FASTCOVER_accel_t { finalize: 100, skip: 0 }, /* accel = 0 */
    FASTCOVER_accel_t { finalize: 100, skip: 0 }, /* accel = 1 */
    FASTCOVER_accel_t { finalize: 50, skip: 1 },  /* accel = 2 */
    FASTCOVER_accel_t { finalize: 34, skip: 2 },  /* accel = 3 */
    FASTCOVER_accel_t { finalize: 25, skip: 3 },  /* accel = 4 */
    FASTCOVER_accel_t { finalize: 20, skip: 4 },  /* accel = 5 */
    FASTCOVER_accel_t { finalize: 17, skip: 5 },  /* accel = 6 */
    FASTCOVER_accel_t { finalize: 14, skip: 6 },  /* accel = 7 */
    FASTCOVER_accel_t { finalize: 13, skip: 7 },  /* accel = 8 */
    FASTCOVER_accel_t { finalize: 11, skip: 8 },  /* accel = 9 */
    FASTCOVER_accel_t { finalize: 10, skip: 9 },  /* accel = 10 */
];

/*-*************************************
* Context
***************************************/

/// `FASTCOVER_ctx_t`
#[repr(C)]
struct FASTCOVER_ctx_t {
    samples: *const BYTE,
    offsets: *mut usize,
    samplesSizes: *const usize,
    nbSamples: usize,
    nbTrainSamples: usize,
    nbTestSamples: usize,
    nbDmers: usize,
    freqs: *mut U32,
    d: c_uint,
    f: c_uint,
    accelParams: FASTCOVER_accel_t,
}

/*-*************************************
*  Helper functions
***************************************/

/// Selects the best segment in an epoch.
unsafe fn FASTCOVER_selectSegment(
    ctx: *const FASTCOVER_ctx_t,
    freqs: *mut U32,
    begin: U32,
    end: U32,
    parameters: ZDICT_cover_params_t,
    segmentFreqs: *mut U16,
) -> COVER_segment_t {
    /* Constants */
    let k: U32 = parameters.k;
    let d: U32 = parameters.d;
    let f: U32 = (*ctx).f;
    let dmersInK: U32 = k.wrapping_sub(d).wrapping_add(1);

    /* Try each segment (activeSegment) and save the best (bestSegment) */
    let mut bestSegment: COVER_segment_t = COVER_segment_t {
        begin: 0,
        end: 0,
        score: 0,
    };
    let mut activeSegment: COVER_segment_t;

    /* Reset the activeDmers in the segment */
    /* The activeSegment starts at the beginning of the epoch. */
    activeSegment = COVER_segment_t {
        begin: begin,
        end: begin,
        score: 0,
    };

    /* Slide the activeSegment through the whole epoch.
     * Save the best segment in bestSegment. */
    while activeSegment.end < end {
        /* Get hash value of current dmer */
        let idx: usize = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.add(activeSegment.end as usize) as *const c_void,
            f,
            d,
        );

        /* Add frequency of this index to score if this is the first occurrence
         * of index in active segment */
        if *segmentFreqs.add(idx) == 0 {
            activeSegment.score = activeSegment.score.wrapping_add(*freqs.add(idx));
        }
        /* Increment end of segment and segmentFreqs*/
        activeSegment.end = activeSegment.end.wrapping_add(1);
        *segmentFreqs.add(idx) = (*segmentFreqs.add(idx)).wrapping_add(1);
        /* If the window is now too large, drop the first position */
        if activeSegment.end.wrapping_sub(activeSegment.begin) == dmersInK.wrapping_add(1) {
            /* Get hash value of the dmer to be eliminated from active segment */
            let delIndex: usize = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(activeSegment.begin as usize) as *const c_void,
                f,
                d,
            );
            *segmentFreqs.add(delIndex) = (*segmentFreqs.add(delIndex)).wrapping_sub(1);
            /* Subtract frequency of this index from score if this is the last
             * occurrence of this index in active segment */
            if *segmentFreqs.add(delIndex) == 0 {
                activeSegment.score = activeSegment.score.wrapping_sub(*freqs.add(delIndex));
            }
            /* Increment start of segment */
            activeSegment.begin = activeSegment.begin.wrapping_add(1);
        }

        /* If this segment is the best so far save it */
        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }

    /* Zero out rest of segmentFreqs array */
    while activeSegment.begin < end {
        let delIndex: usize = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.add(activeSegment.begin as usize) as *const c_void,
            f,
            d,
        );
        *segmentFreqs.add(delIndex) = (*segmentFreqs.add(delIndex)).wrapping_sub(1);
        activeSegment.begin = activeSegment.begin.wrapping_add(1);
    }

    {
        /* Zero the frequency of hash value of each dmer covered by the chosen
         * segment. */
        let mut pos: U32 = bestSegment.begin;
        while pos != bestSegment.end {
            let i: usize = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(pos as usize) as *const c_void,
                f,
                d,
            );
            *freqs.add(i) = 0;
            pos = pos.wrapping_add(1);
        }
    }

    bestSegment
}

/// Check the validity of the parameters.
unsafe fn FASTCOVER_checkParameters(
    parameters: ZDICT_cover_params_t,
    maxDictSize: usize,
    f: c_uint,
    accel: c_uint,
) -> c_int {
    /* k, d, and f are required parameters */
    if parameters.d == 0 || parameters.k == 0 {
        return 0;
    }
    /* d has to be 6 or 8 */
    if parameters.d != 6 && parameters.d != 8 {
        return 0;
    }
    /* k <= maxDictSize */
    if parameters.k as usize > maxDictSize {
        return 0;
    }
    /* d <= k */
    if parameters.d > parameters.k {
        return 0;
    }
    /* 0 < f <= FASTCOVER_MAX_F*/
    if f > FASTCOVER_MAX_F || f == 0 {
        return 0;
    }
    /* 0 < splitPoint <= 1 */
    if parameters.splitPoint <= 0.0 || parameters.splitPoint > 1.0 {
        return 0;
    }
    /* 0 < accel <= 10 */
    if accel > 10 || accel == 0 {
        return 0;
    }
    1
}

/// Clean up a context initialized with `FASTCOVER_ctx_init()`.
unsafe fn FASTCOVER_ctx_destroy(ctx: *mut FASTCOVER_ctx_t) {
    if ctx.is_null() {
        return;
    }

    free((*ctx).freqs as *mut c_void);
    (*ctx).freqs = ptr::null_mut();

    free((*ctx).offsets as *mut c_void);
    (*ctx).offsets = ptr::null_mut();
}

/// Calculate for frequency of hash value of each dmer in `ctx->samples`
unsafe fn FASTCOVER_computeFrequency(freqs: *mut U32, ctx: *const FASTCOVER_ctx_t) {
    let f: c_uint = (*ctx).f;
    let d: c_uint = (*ctx).d;
    let skip: c_uint = (*ctx).accelParams.skip;
    let readLength: c_uint = MAX(d, 8u32);
    let mut i: usize = 0;
    while i < (*ctx).nbTrainSamples {
        let mut start: usize = *(*ctx).offsets.add(i); /* start of current dmer */
        let currSampleEnd: usize = *(*ctx).offsets.add(i + 1);
        while start.wrapping_add(readLength as usize) <= currSampleEnd {
            let dmerIndex: usize = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(start) as *const c_void,
                f,
                d,
            );
            *freqs.add(dmerIndex) = (*freqs.add(dmerIndex)).wrapping_add(1);
            start = start.wrapping_add(skip as usize).wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
}

/// Prepare a context for dictionary building.
/// Returns 0 on success or error code on error.
unsafe fn FASTCOVER_ctx_init(
    ctx: *mut FASTCOVER_ctx_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    d: c_uint,
    splitPoint: f64,
    f: c_uint,
    accelParams: FASTCOVER_accel_t,
) -> usize {
    let samples: *const BYTE = samplesBuffer as *const BYTE;
    let totalSamplesSize: usize = COVER_sum(samplesSizes, nbSamples);
    /* Split samples into testing and training sets */
    let nbTrainSamples: c_uint = if splitPoint < 1.0 {
        (nbSamples as f64 * splitPoint) as c_uint
    } else {
        nbSamples
    };
    let nbTestSamples: c_uint = if splitPoint < 1.0 {
        nbSamples.wrapping_sub(nbTrainSamples)
    } else {
        nbSamples
    };
    let trainingSamplesSize: usize = if splitPoint < 1.0 {
        COVER_sum(samplesSizes, nbTrainSamples)
    } else {
        totalSamplesSize
    };
    let _testSamplesSize: usize = if splitPoint < 1.0 {
        COVER_sum(samplesSizes.add(nbTrainSamples as usize), nbTestSamples)
    } else {
        totalSamplesSize
    };

    /* Checks */
    if totalSamplesSize < MAX(d as usize, size_of::<U64>())
        || totalSamplesSize >= FASTCOVER_MAX_SAMPLES_SIZE as usize
    {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Check if there are at least 5 training samples */
    if nbTrainSamples < 5 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Check if there's testing sample */
    if nbTestSamples < 1 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Zero the context */
    memset(ctx as *mut c_void, 0, size_of::<FASTCOVER_ctx_t>());

    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as usize;
    (*ctx).nbTrainSamples = nbTrainSamples as usize;
    (*ctx).nbTestSamples = nbTestSamples as usize;
    (*ctx).nbDmers = trainingSamplesSize
        .wrapping_sub(MAX(d as usize, size_of::<U64>()))
        .wrapping_add(1);
    (*ctx).d = d;
    (*ctx).f = f;
    (*ctx).accelParams = accelParams;

    /* The offsets of each file */
    (*ctx).offsets = calloc(nbSamples as usize + 1, size_of::<usize>()) as *mut usize;
    if (*ctx).offsets.is_null() {
        FASTCOVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    /* Fill offsets from the samplesSizes */
    {
        let mut i: U32 = 1;
        *(*ctx).offsets.add(0) = 0;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) = (*(*ctx).offsets.add(i as usize - 1))
                .wrapping_add(*samplesSizes.add(i as usize - 1));
            i = i.wrapping_add(1);
        }
    }

    /* Initialize frequency array of size 2^f */
    (*ctx).freqs = calloc((1u64).wrapping_shl(f) as usize, size_of::<U32>()) as *mut U32;
    if (*ctx).freqs.is_null() {
        FASTCOVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    FASTCOVER_computeFrequency((*ctx).freqs, ctx);

    0
}

/// Given the prepared context build the dictionary.
unsafe fn FASTCOVER_buildDictionary(
    ctx: *const FASTCOVER_ctx_t,
    freqs: *mut U32,
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
    segmentFreqs: *mut U16,
) -> usize {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut tail: usize = dictBufferCapacity;
    /* Divide the data into epochs. We will select one segment from each epoch. */
    let epochs: COVER_epoch_info_t = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).nbDmers as U32,
        parameters.k,
        1,
    );
    let maxZeroScoreRun: usize = 10;
    let mut zeroScoreRun: usize = 0;
    let mut epoch: usize = 0;
    /* Loop through the epochs until there are no more segments or the
     * dictionary is full. */
    'outer: while tail > 0 {
        'body: {
            let epochBegin: U32 = epoch.wrapping_mul(epochs.size as usize) as U32;
            let epochEnd: U32 = epochBegin.wrapping_add(epochs.size);
            let segmentSize: usize;
            /* Select a segment */
            let segment: COVER_segment_t = FASTCOVER_selectSegment(
                ctx,
                freqs,
                epochBegin,
                epochEnd,
                parameters,
                segmentFreqs,
            );

            /* If the segment covers no dmers, then we are out of content. */
            if segment.score == 0 {
                zeroScoreRun = zeroScoreRun.wrapping_add(1);
                if zeroScoreRun >= maxZeroScoreRun {
                    break 'outer;
                }
                break 'body;
            }
            zeroScoreRun = 0;

            /* Trim the segment if necessary and if it is too small we are done */
            segmentSize = MIN(
                segment
                    .end
                    .wrapping_sub(segment.begin)
                    .wrapping_add(parameters.d)
                    .wrapping_sub(1) as usize,
                tail,
            );
            if segmentSize < parameters.d as usize {
                break 'outer;
            }

            /* We fill the dictionary from the back to allow the best segments
             * to be referenced with the smallest offsets. */
            tail = tail.wrapping_sub(segmentSize);
            memcpy(
                dict.add(tail) as *mut c_void,
                (*ctx).samples.add(segment.begin as usize) as *const c_void,
                segmentSize,
            );
            DISPLAYUPDATE(2);
        }
        epoch = (epoch.wrapping_add(1)) % (epochs.num as usize);
    }
    tail
}

/// Parameters for `FASTCOVER_tryParameters()`.
#[repr(C)]
struct FASTCOVER_tryParameters_data_t {
    ctx: *const FASTCOVER_ctx_t,
    best: *mut COVER_best_t,
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
}

/// Tries a set of parameters and updates the `COVER_best_t` with the results.
/// It takes its parameters as an *OWNING* opaque pointer to support threading.
unsafe extern "C" fn FASTCOVER_tryParameters(opaque: *mut c_void) {
    /* Save parameters as local variables */
    let data: *mut FASTCOVER_tryParameters_data_t =
        opaque as *mut FASTCOVER_tryParameters_data_t;
    let ctx: *const FASTCOVER_ctx_t = (*data).ctx;
    let parameters: ZDICT_cover_params_t = (*data).parameters;
    let dictBufferCapacity: usize = (*data).dictBufferCapacity;
    let totalCompressedSize: usize = ERROR(ZSTD_error_GENERIC);
    /* Initialize array to keep track of frequency of dmer within activeSegment */
    let segmentFreqs: *mut U16 =
        calloc((1u64).wrapping_shl((*ctx).f) as usize, size_of::<U16>()) as *mut U16;
    /* Allocate space for hash table, dict, and freqs */
    let dict: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection: COVER_dictSelection_t =
        COVER_dictSelectionError(ERROR(ZSTD_error_GENERIC));
    let freqs: *mut U32 =
        malloc(((1u64).wrapping_shl((*ctx).f) as usize).wrapping_mul(size_of::<U32>())) as *mut U32;
    '_cleanup: {
        if segmentFreqs.is_null() || dict.is_null() || freqs.is_null() {
            break '_cleanup;
        }
        /* Copy the frequencies because we need to modify them */
        memcpy(
            freqs as *mut c_void,
            (*ctx).freqs as *const c_void,
            ((1u64).wrapping_shl((*ctx).f) as usize).wrapping_mul(size_of::<U32>()),
        );
        /* Build the dictionary */
        {
            let tail: usize = FASTCOVER_buildDictionary(
                ctx,
                freqs,
                dict as *mut c_void,
                dictBufferCapacity,
                parameters,
                segmentFreqs,
            );

            let nbFinalizeSamples: c_uint = ((*ctx)
                .nbTrainSamples
                .wrapping_mul((*ctx).accelParams.finalize as usize)
                / 100) as c_uint;
            selection = COVER_selectDict(
                dict.add(tail),
                dictBufferCapacity,
                dictBufferCapacity.wrapping_sub(tail),
                (*ctx).samples,
                (*ctx).samplesSizes,
                nbFinalizeSamples,
                (*ctx).nbTrainSamples,
                (*ctx).nbSamples,
                parameters,
                (*ctx).offsets,
                totalCompressedSize,
            );

            if COVER_dictSelectionIsError(selection) != 0 {
                break '_cleanup;
            }
        }
    }
    free(dict as *mut c_void);
    COVER_best_finish((*data).best, parameters, selection);
    free(data as *mut c_void);
    free(segmentFreqs as *mut c_void);
    COVER_dictSelectionFree(selection);
    free(freqs as *mut c_void);
}

/// `FASTCOVER_convertToCoverParams()`
unsafe fn FASTCOVER_convertToCoverParams(
    fastCoverParams: ZDICT_fastCover_params_t,
    coverParams: *mut ZDICT_cover_params_t,
) {
    (*coverParams).k = fastCoverParams.k;
    (*coverParams).d = fastCoverParams.d;
    (*coverParams).steps = fastCoverParams.steps;
    (*coverParams).nbThreads = fastCoverParams.nbThreads;
    (*coverParams).splitPoint = fastCoverParams.splitPoint;
    (*coverParams).zParams = fastCoverParams.zParams;
    (*coverParams).shrinkDict = fastCoverParams.shrinkDict;
}

/// `FASTCOVER_convertToFastCoverParams()`
unsafe fn FASTCOVER_convertToFastCoverParams(
    coverParams: ZDICT_cover_params_t,
    fastCoverParams: *mut ZDICT_fastCover_params_t,
    f: c_uint,
    accel: c_uint,
) {
    (*fastCoverParams).k = coverParams.k;
    (*fastCoverParams).d = coverParams.d;
    (*fastCoverParams).steps = coverParams.steps;
    (*fastCoverParams).nbThreads = coverParams.nbThreads;
    (*fastCoverParams).splitPoint = coverParams.splitPoint;
    (*fastCoverParams).f = f;
    (*fastCoverParams).accel = accel;
    (*fastCoverParams).zParams = coverParams.zParams;
    (*fastCoverParams).shrinkDict = coverParams.shrinkDict;
}

/// `ZDICT_trainFromBuffer_fastCover()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    mut parameters: ZDICT_fastCover_params_t,
) -> usize {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut ctx: FASTCOVER_ctx_t = core::mem::zeroed();
    let mut coverParams: ZDICT_cover_params_t = core::mem::zeroed();
    let accelParams: FASTCOVER_accel_t;
    /* Initialize global data */
    g_displayLevel = parameters.zParams.notificationLevel as c_int;
    /* Assign splitPoint and f if not provided */
    parameters.splitPoint = 1.0;
    parameters.f = if parameters.f == 0 {
        DEFAULT_F
    } else {
        parameters.f
    };
    parameters.accel = if parameters.accel == 0 {
        DEFAULT_ACCEL
    } else {
        parameters.accel
    };
    /* Convert to cover parameter */
    memset(
        ptr::addr_of_mut!(coverParams) as *mut c_void,
        0,
        size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(parameters, ptr::addr_of_mut!(coverParams));
    /* Checks */
    if FASTCOVER_checkParameters(
        coverParams,
        dictBufferCapacity,
        parameters.f,
        parameters.accel,
    ) == 0
    {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    /* Assign corresponding FASTCOVER_accel_t to accelParams*/
    accelParams = FASTCOVER_defaultAccelParameters[parameters.accel as usize];
    /* Initialize context */
    {
        let initVal: usize = FASTCOVER_ctx_init(
            ptr::addr_of_mut!(ctx),
            samplesBuffer,
            samplesSizes,
            nbSamples,
            coverParams.d,
            parameters.splitPoint,
            parameters.f,
            accelParams,
        );
        if ERR_isError(initVal) != 0 {
            return initVal;
        }
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, g_displayLevel);
    /* Build the dictionary */
    {
        /* Initialize array to keep track of frequency of dmer within activeSegment */
        let segmentFreqs: *mut U16 =
            calloc((1u64).wrapping_shl(parameters.f) as usize, size_of::<U16>()) as *mut U16;
        let tail: usize = FASTCOVER_buildDictionary(
            ptr::addr_of!(ctx),
            ctx.freqs,
            dictBuffer,
            dictBufferCapacity,
            coverParams,
            segmentFreqs,
        );
        let nbFinalizeSamples: c_uint = (ctx
            .nbTrainSamples
            .wrapping_mul(ctx.accelParams.finalize as usize)
            / 100) as c_uint;
        let dictionarySize: usize = ZDICT_finalizeDictionary(
            dict as *mut c_void,
            dictBufferCapacity,
            dict.add(tail) as *const c_void,
            dictBufferCapacity.wrapping_sub(tail),
            samplesBuffer,
            samplesSizes,
            nbFinalizeSamples,
            coverParams.zParams,
        );
        FASTCOVER_ctx_destroy(ptr::addr_of_mut!(ctx));
        free(segmentFreqs as *mut c_void);
        dictionarySize
    }
}

/// `ZDICT_optimizeTrainFromBuffer_fastCover()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_optimizeTrainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: *mut ZDICT_fastCover_params_t,
) -> usize {
    let mut coverParams: ZDICT_cover_params_t = core::mem::zeroed();
    let accelParams: FASTCOVER_accel_t;
    /* constants */
    let nbThreads: c_uint = (*parameters).nbThreads;
    let splitPoint: f64 = if (*parameters).splitPoint <= 0.0 {
        FASTCOVER_DEFAULT_SPLITPOINT
    } else {
        (*parameters).splitPoint
    };
    let kMinD: c_uint = if (*parameters).d == 0 { 6 } else { (*parameters).d };
    let kMaxD: c_uint = if (*parameters).d == 0 { 8 } else { (*parameters).d };
    let kMinK: c_uint = if (*parameters).k == 0 { 50 } else { (*parameters).k };
    let kMaxK: c_uint = if (*parameters).k == 0 { 2000 } else { (*parameters).k };
    let kSteps: c_uint = if (*parameters).steps == 0 {
        40
    } else {
        (*parameters).steps
    };
    let kStepSize: c_uint = MAX(kMaxK.wrapping_sub(kMinK) / kSteps, 1u32);
    let kIterations: c_uint = (1u32.wrapping_add(kMaxD.wrapping_sub(kMinD) / 2))
        .wrapping_mul(1u32.wrapping_add(kMaxK.wrapping_sub(kMinK) / kStepSize));
    let f: c_uint = if (*parameters).f == 0 {
        DEFAULT_F
    } else {
        (*parameters).f
    };
    let accel: c_uint = if (*parameters).accel == 0 {
        DEFAULT_ACCEL
    } else {
        (*parameters).accel
    };
    let shrinkDict: c_uint = 0;
    /* Local variables */
    let displayLevel: c_int = (*parameters).zParams.notificationLevel as c_int;
    let mut iteration: c_uint = 1;
    let mut d: c_uint;
    let mut k: c_uint;
    let mut best: COVER_best_t = core::mem::zeroed();
    let mut pool: *mut POOL_ctx = ptr::null_mut();
    let mut warned: c_int = 0;
    let _ = kIterations;
    /* Checks */
    if splitPoint <= 0.0 || splitPoint > 1.0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if accel == 0 || accel > FASTCOVER_MAX_ACCEL {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbThreads > 1 {
        pool = POOL_create(nbThreads as usize, 1);
        if pool.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
    }
    /* Initialization */
    COVER_best_init(ptr::addr_of_mut!(best));
    memset(
        ptr::addr_of_mut!(coverParams) as *mut c_void,
        0,
        size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(*parameters, ptr::addr_of_mut!(coverParams));
    accelParams = FASTCOVER_defaultAccelParameters[accel as usize];
    /* Turn down global display level to clean up display at level 2 and below */
    g_displayLevel = if displayLevel == 0 {
        0
    } else {
        displayLevel - 1
    };
    /* Loop through d first because each new value needs a new context */
    d = kMinD;
    while d <= kMaxD {
        /* Initialize the context for this value of d */
        let mut ctx: FASTCOVER_ctx_t = core::mem::zeroed();
        {
            let initVal: usize = FASTCOVER_ctx_init(
                ptr::addr_of_mut!(ctx),
                samplesBuffer,
                samplesSizes,
                nbSamples,
                d,
                splitPoint,
                f,
                accelParams,
            );
            if ERR_isError(initVal) != 0 {
                COVER_best_destroy(ptr::addr_of_mut!(best));
                POOL_free(pool);
                return initVal;
            }
        }
        if warned == 0 {
            COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, displayLevel);
            warned = 1;
        }
        /* Loop through k reusing the same context */
        k = kMinK;
        while k <= kMaxK {
            'k_body: {
                /* Prepare the arguments */
                let data: *mut FASTCOVER_tryParameters_data_t =
                    malloc(size_of::<FASTCOVER_tryParameters_data_t>())
                        as *mut FASTCOVER_tryParameters_data_t;
                if data.is_null() {
                    COVER_best_destroy(ptr::addr_of_mut!(best));
                    FASTCOVER_ctx_destroy(ptr::addr_of_mut!(ctx));
                    POOL_free(pool);
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*data).ctx = ptr::addr_of!(ctx);
                (*data).best = ptr::addr_of_mut!(best);
                (*data).dictBufferCapacity = dictBufferCapacity;
                (*data).parameters = coverParams;
                (*data).parameters.k = k;
                (*data).parameters.d = d;
                (*data).parameters.splitPoint = splitPoint;
                (*data).parameters.steps = kSteps;
                (*data).parameters.shrinkDict = shrinkDict;
                (*data).parameters.zParams.notificationLevel = g_displayLevel as c_uint;
                /* Check the parameters */
                if FASTCOVER_checkParameters(
                    (*data).parameters,
                    dictBufferCapacity,
                    (*(*data).ctx).f,
                    accel,
                ) == 0
                {
                    free(data as *mut c_void);
                    break 'k_body;
                }
                /* Call the function and pass ownership of data to it */
                COVER_best_start(ptr::addr_of_mut!(best));
                if !pool.is_null() {
                    POOL_add(pool, Some(FASTCOVER_tryParameters), data as *mut c_void);
                } else {
                    FASTCOVER_tryParameters(data as *mut c_void);
                }
                /* Print status */
                LOCALDISPLAYUPDATE(displayLevel, 2);
                iteration = iteration.wrapping_add(1);
            }
            k = k.wrapping_add(kStepSize);
        }
        COVER_best_wait(ptr::addr_of_mut!(best));
        FASTCOVER_ctx_destroy(ptr::addr_of_mut!(ctx));
        d = d.wrapping_add(2);
    }
    /* Fill the output buffer and parameters with output of the best parameters */
    {
        let dictSize: usize = best.dictSize;
        if ERR_isError(best.compressedSize) != 0 {
            let compressedSize: usize = best.compressedSize;
            COVER_best_destroy(ptr::addr_of_mut!(best));
            POOL_free(pool);
            return compressedSize;
        }
        FASTCOVER_convertToFastCoverParams(best.parameters, parameters, f, accel);
        memcpy(dictBuffer, best.dict as *const c_void, dictSize);
        COVER_best_destroy(ptr::addr_of_mut!(best));
        POOL_free(pool);
        dictSize
    }
}
