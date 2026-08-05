//! Translation of dictBuilder/fastcover.c
//! Single-threaded build (ZSTD_MULTITHREAD undefined), LE 64-bit.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, unused_mut, unused_assignments, unused_parens)]

use core::ffi::{c_int, c_long, c_uint, c_void};

use crate::common::allocations::{calloc, free, malloc, memcpy, memset};
use crate::common::error::{code, error};
use crate::common::mem::{U16, U32, U64};
use crate::common::pool::{POOL_add, POOL_create, POOL_ctx, POOL_free};
use crate::common::zstd_common::ZSTD_isError;
use crate::compress::zstd_compress_internal::{ZSTD_hash6Ptr, ZSTD_hash8Ptr};

// Shared types
use crate::dictBuilder::cover::{
    COVER_best_t, COVER_dictSelection_t, COVER_epoch_info_t, COVER_segment_t,
};
use crate::dictBuilder::zdict::{ZDICT_cover_params_t, ZDICT_fastCover_params_t, ZDICT_params_t};

type BYTE = u8;

// Cross-file C symbols (defined in cover.rs / zdict.rs), called via the C ABI.
extern "C" {
    fn COVER_computeEpochs(maxDictSize: U32, nbDmers: U32, k: U32, passes: U32)
        -> COVER_epoch_info_t;
    fn COVER_warnOnSmallCorpus(maxDictSize: usize, nbDmers: usize, displayLevel: c_int);
    fn COVER_sum(samplesSizes: *const usize, nbSamples: c_uint) -> usize;
    fn COVER_best_init(best: *mut COVER_best_t);
    fn COVER_best_wait(best: *mut COVER_best_t);
    fn COVER_best_destroy(best: *mut COVER_best_t);
    fn COVER_best_start(best: *mut COVER_best_t);
    fn COVER_best_finish(
        best: *mut COVER_best_t,
        parameters: ZDICT_cover_params_t,
        selection: COVER_dictSelection_t,
    );
    fn COVER_dictSelectionIsError(selection: COVER_dictSelection_t) -> c_uint;
    fn COVER_dictSelectionError(error: usize) -> COVER_dictSelection_t;
    fn COVER_dictSelectionFree(selection: COVER_dictSelection_t);
    fn COVER_selectDict(
        customDictContent: *mut BYTE,
        dictBufferCapacity: usize,
        dictContentSize: usize,
        samplesBuffer: *const BYTE,
        samplesSizes: *const usize,
        nbFinalizeSamples: c_uint,
        nbCheckSamples: usize,
        nbSamples: usize,
        params: ZDICT_cover_params_t,
        offsets: *mut usize,
        totalCompressedSize: usize,
    ) -> COVER_dictSelection_t;
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

    fn clock() -> c_long;
}

/*-*************************************
*  Constants
***************************************/
/* sizeof(size_t) == 8 -> (unsigned)-1 */
const FASTCOVER_MAX_SAMPLES_SIZE: c_uint = u32::MAX;
const FASTCOVER_MAX_F: c_uint = 31;
const FASTCOVER_MAX_ACCEL: c_uint = 10;
const FASTCOVER_DEFAULT_SPLITPOINT: f64 = 0.75;
const DEFAULT_F: c_uint = 20;
const DEFAULT_ACCEL: c_uint = 1;

const ZDICT_DICTSIZE_MIN: usize = 256;

/*-*************************************
*  Console display
***************************************/
static mut g_displayLevel: c_int = 0;

const CLOCKS_PER_SEC: c_long = 1000000;
const g_refreshRate: c_long = CLOCKS_PER_SEC * 15 / 100;
static mut g_time: c_long = 0;

macro_rules! DISPLAY {
    ($($arg:tt)*) => {{
        eprint!($($arg)*);
    }};
}

macro_rules! LOCALDISPLAYLEVEL {
    ($displayLevel:expr, $l:expr, $($arg:tt)*) => {{
        if $displayLevel >= $l {
            DISPLAY!($($arg)*);
        }
    }};
}

macro_rules! DISPLAYLEVEL {
    ($l:expr, $($arg:tt)*) => {{
        LOCALDISPLAYLEVEL!(g_displayLevel, $l, $($arg)*);
    }};
}

macro_rules! LOCALDISPLAYUPDATE {
    ($displayLevel:expr, $l:expr, $($arg:tt)*) => {{
        if $displayLevel >= $l {
            if (clock() - g_time > g_refreshRate) || ($displayLevel >= 4) {
                g_time = clock();
                DISPLAY!($($arg)*);
            }
        }
    }};
}

macro_rules! DISPLAYUPDATE {
    ($l:expr, $($arg:tt)*) => {{
        LOCALDISPLAYUPDATE!(g_displayLevel, $l, $($arg)*);
    }};
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
#[derive(Clone, Copy)]
struct FASTCOVER_accel_t {
    finalize: c_uint, /* Percentage of training samples used for ZDICT_finalizeDictionary */
    skip: c_uint,     /* Number of dmer skipped between each dmer counted in computeFrequency */
}

static FASTCOVER_defaultAccelParameters: [FASTCOVER_accel_t; (FASTCOVER_MAX_ACCEL + 1) as usize] = [
    FASTCOVER_accel_t { finalize: 100, skip: 0 }, /* accel = 0, should not happen because accel = 0 defaults to accel = 1 */
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
    let dmersInK: U32 = k - d + 1;

    /* Try each segment (activeSegment) and save the best (bestSegment) */
    let mut bestSegment = COVER_segment_t { begin: 0, end: 0, score: 0 };
    let mut activeSegment = COVER_segment_t { begin: 0, end: 0, score: 0 };

    /* Reset the activeDmers in the segment */
    /* The activeSegment starts at the beginning of the epoch. */
    activeSegment.begin = begin;
    activeSegment.end = begin;
    activeSegment.score = 0;

    /* Slide the activeSegment through the whole epoch.
     * Save the best segment in bestSegment.
     */
    while activeSegment.end < end {
        /* Get hash value of current dmer */
        let idx: usize = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.add(activeSegment.end as usize) as *const c_void,
            f,
            d,
        );

        /* Add frequency of this index to score if this is the first occurrence of index in active segment */
        if *segmentFreqs.add(idx) == 0 {
            activeSegment.score += *freqs.add(idx);
        }
        /* Increment end of segment and segmentFreqs*/
        activeSegment.end += 1;
        *segmentFreqs.add(idx) += 1;
        /* If the window is now too large, drop the first position */
        if activeSegment.end - activeSegment.begin == dmersInK + 1 {
            /* Get hash value of the dmer to be eliminated from active segment */
            let delIndex: usize = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(activeSegment.begin as usize) as *const c_void,
                f,
                d,
            );
            *segmentFreqs.add(delIndex) -= 1;
            /* Subtract frequency of this index from score if this is the last occurrence of this index in active segment */
            if *segmentFreqs.add(delIndex) == 0 {
                activeSegment.score -= *freqs.add(delIndex);
            }
            /* Increment start of segment */
            activeSegment.begin += 1;
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
        *segmentFreqs.add(delIndex) -= 1;
        activeSegment.begin += 1;
    }

    {
        /*  Zero the frequency of hash value of each dmer covered by the chosen segment. */
        let mut pos: U32 = bestSegment.begin;
        while pos != bestSegment.end {
            let i: usize = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(pos as usize) as *const c_void,
                f,
                d,
            );
            *freqs.add(i) = 0;
            pos += 1;
        }
    }

    bestSegment
}

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
    (*ctx).freqs = core::ptr::null_mut();

    free((*ctx).offsets as *mut c_void);
    (*ctx).offsets = core::ptr::null_mut();
}

/// Calculate for frequency of hash value of each dmer in ctx->samples
unsafe fn FASTCOVER_computeFrequency(freqs: *mut U32, ctx: *const FASTCOVER_ctx_t) {
    let f: c_uint = (*ctx).f;
    let d: c_uint = (*ctx).d;
    let skip: c_uint = (*ctx).accelParams.skip;
    let readLength: c_uint = if d > 8 { d } else { 8 };
    let mut i: usize;
    debug_assert!((*ctx).nbTrainSamples >= 5);
    debug_assert!((*ctx).nbTrainSamples <= (*ctx).nbSamples);
    i = 0;
    while i < (*ctx).nbTrainSamples {
        let mut start: usize = *(*ctx).offsets.add(i); /* start of current dmer */
        let currSampleEnd: usize = *(*ctx).offsets.add(i + 1);
        while start + readLength as usize <= currSampleEnd {
            let dmerIndex: usize = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(start) as *const c_void,
                f,
                d,
            );
            *freqs.add(dmerIndex) += 1;
            start = start + skip as usize + 1;
        }
        i += 1;
    }
}

/// Prepare a context for dictionary building.
/// The context is only dependent on the parameter `d` and can be used multiple
/// times.
/// Returns 0 on success or error code on error.
/// The context must be destroyed with `FASTCOVER_ctx_destroy()`.
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
        nbSamples - nbTrainSamples
    } else {
        nbSamples
    };
    let trainingSamplesSize: usize = if splitPoint < 1.0 {
        COVER_sum(samplesSizes, nbTrainSamples)
    } else {
        totalSamplesSize
    };
    let testSamplesSize: usize = if splitPoint < 1.0 {
        COVER_sum(samplesSizes.add(nbTrainSamples as usize), nbTestSamples)
    } else {
        totalSamplesSize
    };

    /* Checks */
    let maxDU64: usize = if (d as usize) > core::mem::size_of::<U64>() {
        d as usize
    } else {
        core::mem::size_of::<U64>()
    };
    if totalSamplesSize < maxDU64
        || totalSamplesSize >= FASTCOVER_MAX_SAMPLES_SIZE as usize
    {
        DISPLAYLEVEL!(
            1,
            "Total samples size is too large ({} MB), maximum size is {} MB\n",
            (totalSamplesSize >> 20) as c_uint,
            (FASTCOVER_MAX_SAMPLES_SIZE >> 20)
        );
        return error(code::SRCSIZE_WRONG);
    }

    /* Check if there are at least 5 training samples */
    if nbTrainSamples < 5 {
        DISPLAYLEVEL!(
            1,
            "Total number of training samples is {} and is invalid\n",
            nbTrainSamples
        );
        return error(code::SRCSIZE_WRONG);
    }

    /* Check if there's testing sample */
    if nbTestSamples < 1 {
        DISPLAYLEVEL!(
            1,
            "Total number of testing samples is {} and is invalid.\n",
            nbTestSamples
        );
        return error(code::SRCSIZE_WRONG);
    }

    /* Zero the context */
    memset(ctx as *mut c_void, 0, core::mem::size_of::<FASTCOVER_ctx_t>());
    DISPLAYLEVEL!(
        2,
        "Training on {} samples of total size {}\n",
        nbTrainSamples,
        trainingSamplesSize as c_uint
    );
    DISPLAYLEVEL!(
        2,
        "Testing on {} samples of total size {}\n",
        nbTestSamples,
        testSamplesSize as c_uint
    );

    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as usize;
    (*ctx).nbTrainSamples = nbTrainSamples as usize;
    (*ctx).nbTestSamples = nbTestSamples as usize;
    (*ctx).nbDmers = trainingSamplesSize - maxDU64 + 1;
    (*ctx).d = d;
    (*ctx).f = f;
    (*ctx).accelParams = accelParams;

    /* The offsets of each file */
    (*ctx).offsets =
        calloc((nbSamples as usize + 1), core::mem::size_of::<usize>()) as *mut usize;
    if (*ctx).offsets.is_null() {
        DISPLAYLEVEL!(1, "Failed to allocate scratch buffers \n");
        FASTCOVER_ctx_destroy(ctx);
        return error(code::MEMORY_ALLOCATION);
    }

    /* Fill offsets from the samplesSizes */
    {
        let mut i: U32;
        *(*ctx).offsets.add(0) = 0;
        debug_assert!(nbSamples >= 5);
        i = 1;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) =
                *(*ctx).offsets.add((i - 1) as usize) + *samplesSizes.add((i - 1) as usize);
            i += 1;
        }
    }

    /* Initialize frequency array of size 2^f */
    (*ctx).freqs = calloc((1u64 << f) as usize, core::mem::size_of::<U32>()) as *mut U32;
    if (*ctx).freqs.is_null() {
        DISPLAYLEVEL!(1, "Failed to allocate frequency table \n");
        FASTCOVER_ctx_destroy(ctx);
        return error(code::MEMORY_ALLOCATION);
    }

    DISPLAYLEVEL!(2, "Computing frequencies\n");
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
    let mut epoch: usize;
    DISPLAYLEVEL!(
        2,
        "Breaking content into {} epochs of size {}\n",
        epochs.num as U32,
        epochs.size as U32
    );
    /* Loop through the epochs until there are no more segments or the dictionary
     * is full.
     */
    epoch = 0;
    while tail > 0 {
        let epochBegin: U32 = (epoch * epochs.size as usize) as U32;
        let epochEnd: U32 = epochBegin + epochs.size;
        let mut segmentSize: usize;
        /* Select a segment */
        let segment: COVER_segment_t =
            FASTCOVER_selectSegment(ctx, freqs, epochBegin, epochEnd, parameters, segmentFreqs);

        /* If the segment covers no dmers, then we are out of content.
         * There may be new content in other epochs, for continue for some time.
         */
        if segment.score == 0 {
            zeroScoreRun += 1;
            if zeroScoreRun >= maxZeroScoreRun {
                break;
            }
            epoch = (epoch + 1) % epochs.num as usize;
            continue;
        }
        zeroScoreRun = 0;

        /* Trim the segment if necessary and if it is too small then we are done */
        let cand: usize = (segment.end - segment.begin + parameters.d - 1) as usize;
        segmentSize = if cand < tail { cand } else { tail };
        if segmentSize < parameters.d as usize {
            break;
        }

        /* We fill the dictionary from the back to allow the best segments to be
         * referenced with the smallest offsets.
         */
        tail -= segmentSize;
        memcpy(
            dict.add(tail) as *mut c_void,
            (*ctx).samples.add(segment.begin as usize) as *const c_void,
            segmentSize,
        );
        DISPLAYUPDATE!(
            2,
            "\r{}%       ",
            (((dictBufferCapacity - tail) * 100) / dictBufferCapacity) as c_uint
        );

        epoch = (epoch + 1) % epochs.num as usize;
    }
    DISPLAYLEVEL!(2, "\r{:79}\r", "");
    tail
}

/// Parameters for FASTCOVER_tryParameters().
#[repr(C)]
struct FASTCOVER_tryParameters_data_t {
    ctx: *const FASTCOVER_ctx_t,
    best: *mut COVER_best_t,
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
}

/// Tries a set of parameters and updates the COVER_best_t with the results.
/// This function is thread safe if zstd is compiled with multithreaded support.
/// It takes its parameters as an *OWNING* opaque pointer to support threading.
extern "C" fn FASTCOVER_tryParameters(opaque: *mut c_void) {
    unsafe {
        /* Save parameters as local variables */
        let data: *mut FASTCOVER_tryParameters_data_t =
            opaque as *mut FASTCOVER_tryParameters_data_t;
        let ctx: *const FASTCOVER_ctx_t = (*data).ctx;
        let parameters: ZDICT_cover_params_t = (*data).parameters;
        let dictBufferCapacity: usize = (*data).dictBufferCapacity;
        let totalCompressedSize: usize = error(code::GENERIC);
        /* Initialize array to keep track of frequency of dmer within activeSegment */
        let segmentFreqs: *mut U16 =
            calloc((1u64 << (*ctx).f) as usize, core::mem::size_of::<U16>()) as *mut U16;
        /* Allocate space for hash table, dict, and freqs */
        let dict: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
        let mut selection: COVER_dictSelection_t = COVER_dictSelectionError(error(code::GENERIC));
        let freqs: *mut U32 =
            malloc((1u64 << (*ctx).f) as usize * core::mem::size_of::<U32>()) as *mut U32;
        if segmentFreqs.is_null() || dict.is_null() || freqs.is_null() {
            DISPLAYLEVEL!(1, "Failed to allocate buffers: out of memory\n");
            // goto _cleanup
            free(dict as *mut c_void);
            COVER_best_finish((*data).best, parameters, selection);
            free(data as *mut c_void);
            free(segmentFreqs as *mut c_void);
            COVER_dictSelectionFree(selection);
            free(freqs as *mut c_void);
            return;
        }
        /* Copy the frequencies because we need to modify them */
        memcpy(
            freqs as *mut c_void,
            (*ctx).freqs as *const c_void,
            (1u64 << (*ctx).f) as usize * core::mem::size_of::<U32>(),
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

            let nbFinalizeSamples: c_uint =
                ((*ctx).nbTrainSamples * (*ctx).accelParams.finalize as usize / 100) as c_uint;
            selection = COVER_selectDict(
                dict.add(tail),
                dictBufferCapacity,
                dictBufferCapacity - tail,
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
                DISPLAYLEVEL!(1, "Failed to select dictionary\n");
                // goto _cleanup
                free(dict as *mut c_void);
                COVER_best_finish((*data).best, parameters, selection);
                free(data as *mut c_void);
                free(segmentFreqs as *mut c_void);
                COVER_dictSelectionFree(selection);
                free(freqs as *mut c_void);
                return;
            }
        }
        // _cleanup:
        free(dict as *mut c_void);
        COVER_best_finish((*data).best, parameters, selection);
        free(data as *mut c_void);
        free(segmentFreqs as *mut c_void);
        COVER_dictSelectionFree(selection);
        free(freqs as *mut c_void);
    }
}

fn FASTCOVER_convertToCoverParams(
    fastCoverParams: ZDICT_fastCover_params_t,
    coverParams: *mut ZDICT_cover_params_t,
) {
    unsafe {
        (*coverParams).k = fastCoverParams.k;
        (*coverParams).d = fastCoverParams.d;
        (*coverParams).steps = fastCoverParams.steps;
        (*coverParams).nbThreads = fastCoverParams.nbThreads;
        (*coverParams).splitPoint = fastCoverParams.splitPoint;
        (*coverParams).zParams = fastCoverParams.zParams;
        (*coverParams).shrinkDict = fastCoverParams.shrinkDict;
    }
}

fn FASTCOVER_convertToFastCoverParams(
    coverParams: ZDICT_cover_params_t,
    fastCoverParams: *mut ZDICT_fastCover_params_t,
    f: c_uint,
    accel: c_uint,
) {
    unsafe {
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
}

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
    parameters.f = if parameters.f == 0 { DEFAULT_F } else { parameters.f };
    parameters.accel = if parameters.accel == 0 { DEFAULT_ACCEL } else { parameters.accel };
    /* Convert to cover parameter */
    memset(
        &mut coverParams as *mut ZDICT_cover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(parameters, &mut coverParams);
    /* Checks */
    if FASTCOVER_checkParameters(
        coverParams,
        dictBufferCapacity,
        parameters.f,
        parameters.accel,
    ) == 0
    {
        DISPLAYLEVEL!(1, "FASTCOVER parameters incorrect\n");
        return error(code::PARAMETER_OUTOFBOUND);
    }
    if nbSamples == 0 {
        DISPLAYLEVEL!(1, "FASTCOVER must have at least one input file\n");
        return error(code::SRCSIZE_WRONG);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        DISPLAYLEVEL!(
            1,
            "dictBufferCapacity must be at least {}\n",
            ZDICT_DICTSIZE_MIN as c_uint
        );
        return error(code::DSTSIZE_TOOSMALL);
    }
    /* Assign corresponding FASTCOVER_accel_t to accelParams*/
    accelParams = FASTCOVER_defaultAccelParameters[parameters.accel as usize];
    /* Initialize context */
    {
        let initVal: usize = FASTCOVER_ctx_init(
            &mut ctx,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            coverParams.d,
            parameters.splitPoint,
            parameters.f,
            accelParams,
        );
        if ZSTD_isError(initVal) != 0 {
            DISPLAYLEVEL!(1, "Failed to initialize context\n");
            return initVal;
        }
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, g_displayLevel);
    /* Build the dictionary */
    DISPLAYLEVEL!(2, "Building dictionary\n");
    {
        /* Initialize array to keep track of frequency of dmer within activeSegment */
        let segmentFreqs: *mut U16 =
            calloc((1u64 << parameters.f) as usize, core::mem::size_of::<U16>()) as *mut U16;
        let tail: usize = FASTCOVER_buildDictionary(
            &ctx,
            ctx.freqs,
            dictBuffer,
            dictBufferCapacity,
            coverParams,
            segmentFreqs,
        );
        let nbFinalizeSamples: c_uint =
            (ctx.nbTrainSamples * ctx.accelParams.finalize as usize / 100) as c_uint;
        let dictionarySize: usize = ZDICT_finalizeDictionary(
            dict as *mut c_void,
            dictBufferCapacity,
            dict.add(tail) as *const c_void,
            dictBufferCapacity - tail,
            samplesBuffer,
            samplesSizes,
            nbFinalizeSamples,
            coverParams.zParams,
        );
        if ZSTD_isError(dictionarySize) == 0 {
            DISPLAYLEVEL!(
                2,
                "Constructed dictionary of size {}\n",
                dictionarySize as c_uint
            );
        }
        FASTCOVER_ctx_destroy(&mut ctx);
        free(segmentFreqs as *mut c_void);
        return dictionarySize;
    }
}

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
    let kSteps: c_uint = if (*parameters).steps == 0 { 40 } else { (*parameters).steps };
    let kStepSize: c_uint = {
        let v = (kMaxK - kMinK) / kSteps;
        if v > 1 { v } else { 1 }
    };
    let kIterations: c_uint =
        (1 + (kMaxD - kMinD) / 2) * (1 + (kMaxK - kMinK) / kStepSize);
    let f: c_uint = if (*parameters).f == 0 { DEFAULT_F } else { (*parameters).f };
    let accel: c_uint = if (*parameters).accel == 0 { DEFAULT_ACCEL } else { (*parameters).accel };
    let shrinkDict: c_uint = 0;
    /* Local variables */
    let displayLevel: c_int = (*parameters).zParams.notificationLevel as c_int;
    let mut iteration: c_uint = 1;
    let mut d: c_uint;
    let mut k: c_uint;
    let mut best: COVER_best_t = core::mem::zeroed();
    let mut pool: *mut POOL_ctx = core::ptr::null_mut();
    let mut warned: c_int = 0;
    /* Checks */
    if splitPoint <= 0.0 || splitPoint > 1.0 {
        LOCALDISPLAYLEVEL!(displayLevel, 1, "Incorrect splitPoint\n");
        return error(code::PARAMETER_OUTOFBOUND);
    }
    if accel == 0 || accel > FASTCOVER_MAX_ACCEL {
        LOCALDISPLAYLEVEL!(displayLevel, 1, "Incorrect accel\n");
        return error(code::PARAMETER_OUTOFBOUND);
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        LOCALDISPLAYLEVEL!(displayLevel, 1, "Incorrect k\n");
        return error(code::PARAMETER_OUTOFBOUND);
    }
    if nbSamples == 0 {
        LOCALDISPLAYLEVEL!(displayLevel, 1, "FASTCOVER must have at least one input file\n");
        return error(code::SRCSIZE_WRONG);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        LOCALDISPLAYLEVEL!(
            displayLevel,
            1,
            "dictBufferCapacity must be at least {}\n",
            ZDICT_DICTSIZE_MIN as c_uint
        );
        return error(code::DSTSIZE_TOOSMALL);
    }
    if nbThreads > 1 {
        pool = POOL_create(nbThreads as usize, 1);
        if pool.is_null() {
            return error(code::MEMORY_ALLOCATION);
        }
    }
    /* Initialization */
    COVER_best_init(&mut best);
    memset(
        &mut coverParams as *mut ZDICT_cover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(*parameters, &mut coverParams);
    accelParams = FASTCOVER_defaultAccelParameters[accel as usize];
    /* Turn down global display level to clean up display at level 2 and below */
    g_displayLevel = if displayLevel == 0 { 0 } else { displayLevel - 1 };
    /* Loop through d first because each new value needs a new context */
    LOCALDISPLAYLEVEL!(
        displayLevel,
        2,
        "Trying {} different sets of parameters\n",
        kIterations
    );
    d = kMinD;
    while d <= kMaxD {
        /* Initialize the context for this value of d */
        let mut ctx: FASTCOVER_ctx_t = core::mem::zeroed();
        LOCALDISPLAYLEVEL!(displayLevel, 3, "d={}\n", d);
        {
            let initVal: usize = FASTCOVER_ctx_init(
                &mut ctx,
                samplesBuffer,
                samplesSizes,
                nbSamples,
                d,
                splitPoint,
                f,
                accelParams,
            );
            if ZSTD_isError(initVal) != 0 {
                LOCALDISPLAYLEVEL!(displayLevel, 1, "Failed to initialize context\n");
                COVER_best_destroy(&mut best);
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
            /* Prepare the arguments */
            let data: *mut FASTCOVER_tryParameters_data_t =
                malloc(core::mem::size_of::<FASTCOVER_tryParameters_data_t>())
                    as *mut FASTCOVER_tryParameters_data_t;
            LOCALDISPLAYLEVEL!(displayLevel, 3, "k={}\n", k);
            if data.is_null() {
                LOCALDISPLAYLEVEL!(displayLevel, 1, "Failed to allocate parameters\n");
                COVER_best_destroy(&mut best);
                FASTCOVER_ctx_destroy(&mut ctx);
                POOL_free(pool);
                return error(code::MEMORY_ALLOCATION);
            }
            (*data).ctx = &ctx;
            (*data).best = &mut best;
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
                DISPLAYLEVEL!(1, "FASTCOVER parameters incorrect\n");
                free(data as *mut c_void);
                k += kStepSize;
                continue;
            }
            /* Call the function and pass ownership of data to it */
            COVER_best_start(&mut best);
            if !pool.is_null() {
                POOL_add(pool, FASTCOVER_tryParameters, data as *mut c_void);
            } else {
                FASTCOVER_tryParameters(data as *mut c_void);
            }
            /* Print status */
            LOCALDISPLAYUPDATE!(
                displayLevel,
                2,
                "\r{}%       ",
                ((iteration * 100) / kIterations)
            );
            iteration += 1;

            k += kStepSize;
        }
        COVER_best_wait(&mut best);
        FASTCOVER_ctx_destroy(&mut ctx);

        d += 2;
    }
    LOCALDISPLAYLEVEL!(displayLevel, 2, "\r{:79}\r", "");
    /* Fill the output buffer and parameters with output of the best parameters */
    {
        let dictSize: usize = best.dictSize;
        if ZSTD_isError(best.compressedSize) != 0 {
            let compressedSize: usize = best.compressedSize;
            COVER_best_destroy(&mut best);
            POOL_free(pool);
            return compressedSize;
        }
        FASTCOVER_convertToFastCoverParams(best.parameters, parameters, f, accel);
        memcpy(dictBuffer, best.dict, dictSize);
        COVER_best_destroy(&mut best);
        POOL_free(pool);
        return dictSize;
    }
}
