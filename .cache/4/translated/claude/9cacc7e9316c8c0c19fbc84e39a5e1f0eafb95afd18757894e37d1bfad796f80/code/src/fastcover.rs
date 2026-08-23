//! Transliteration of `dictBuilder/fastcover.c`.
//!
//! Build configuration assumptions (see TRANSLATION_NOTES.md):
//!  * `ZSTD_MULTITHREAD` is **NOT** defined, so `common/threading.h` collapses
//!    `ZSTD_pthread_mutex_t`/`ZSTD_pthread_cond_t` to `int` and every
//!    `ZSTD_pthread_*()` operation to a no-op.  `COVER_best_t` (defined in
//!    `crate::cover`) already reproduces that layout, and `POOL_*` resolves to
//!    the single-threaded stubs in `crate::pool` (`POOL_add()` simply runs the
//!    job inline).
//!  * `DEBUGLEVEL == 0`, so `assert()` compiles to nothing and has been deleted.
//!  * The `DISPLAY` / `DISPLAYLEVEL` / `LOCALDISPLAYLEVEL` / `DISPLAYUPDATE` /
//!    `LOCALDISPLAYUPDATE` macros only ever `fprintf(stderr, ...)` +
//!    `fflush(stderr)`.  stderr is not part of the produced data stream, so they
//!    are reproduced below as empty no-op functions that are still *called* at
//!    exactly the same places; this keeps the control flow and the
//!    `g_displayLevel`/`notificationLevel` bookkeeping faithful.

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

use core::ffi::{c_double, c_int, c_long, c_uint, c_void};

use crate::cover::{COVER_best_t, COVER_dictSelection_t, COVER_epoch_info_t, COVER_segment_t};
use crate::error_private::{
    ERROR, ZSTD_error_GENERIC, ZSTD_error_dstSize_tooSmall, ZSTD_error_memory_allocation,
    ZSTD_error_parameter_outOfBound, ZSTD_error_srcSize_wrong,
};
use crate::mem::{calloc, free, malloc, memcpy, memset, BYTE, U16, U32, U64};
use crate::pool::POOL_ctx;
use crate::zdict_h::{ZDICT_cover_params_t, ZDICT_fastCover_params_t, ZDICT_DICTSIZE_MIN};
use crate::zstd_common::ZSTD_isError;
use crate::zstd_compress_internal::{ZSTD_hash6Ptr, ZSTD_hash8Ptr};
use crate::zstd_internal::{MAX, MIN};

/*-*************************************
*  Constants
***************************************/
/**
 * There are 32bit indexes used to ref samples, so limit samples size to 4GB
 * on 64bit builds.
 * For 32bit builds we choose 1 GB.
 * Most 32bit platforms have 2GB user-mode addressable space and we allocate a
 * large contiguous buffer, so 1GB is already a high limit.
 *
 * `#define FASTCOVER_MAX_SAMPLES_SIZE (sizeof(size_t) == 8 ? ((unsigned)-1) : ((unsigned)1 GB))`
 * with `#define GB *(1U<<30)`.
 */
pub const FASTCOVER_MAX_SAMPLES_SIZE: c_uint = if core::mem::size_of::<usize>() == 8 {
    0u32.wrapping_sub(1)
} else {
    1u32 * (1u32 << 30)
};
pub const FASTCOVER_MAX_F: c_int = 31;
pub const FASTCOVER_MAX_ACCEL: usize = 10;
pub const FASTCOVER_DEFAULT_SPLITPOINT: c_double = 0.75;
pub const DEFAULT_F: c_uint = 20;
pub const DEFAULT_ACCEL: c_uint = 1;

/*-*************************************
*  Console display
***************************************/
/* `static int g_displayLevel = 0;` */
pub static mut g_displayLevel: c_int = 0;

/* All of the display macros below only write to stderr; reproduced as no-ops.
 *
 *   #define DISPLAY(...) { fprintf(stderr, __VA_ARGS__); fflush(stderr); }
 *   #define LOCALDISPLAYLEVEL(displayLevel, l, ...) if (displayLevel >= l) { DISPLAY(__VA_ARGS__); }
 *   #define DISPLAYLEVEL(l, ...) LOCALDISPLAYLEVEL(g_displayLevel, l, __VA_ARGS__)
 *   #define LOCALDISPLAYUPDATE(displayLevel, l, ...)  ... clock() throttled DISPLAY ...
 *   #define DISPLAYUPDATE(l, ...) LOCALDISPLAYUPDATE(g_displayLevel, l, __VA_ARGS__)
 */
#[inline(always)]
pub fn DISPLAY() {}
#[inline(always)]
pub fn LOCALDISPLAYLEVEL(_displayLevel: c_int, _l: c_int) {}
#[inline(always)]
pub fn DISPLAYLEVEL(_l: c_int) {}
#[inline(always)]
pub fn LOCALDISPLAYUPDATE(_displayLevel: c_int, _l: c_int) {}
#[inline(always)]
pub fn DISPLAYUPDATE(_l: c_int) {}

/* `static const clock_t g_refreshRate = CLOCKS_PER_SEC * 15 / 100;`
 * `static clock_t g_time = 0;`
 * Only ever read/written by LOCALDISPLAYUPDATE, which is a no-op here.
 * CLOCKS_PER_SEC == 1000000 on glibc. */
pub static g_refreshRate: c_long = 1000000 * 15 / 100;
pub static mut g_time: c_long = 0;

/*-*************************************
* Hash Functions
***************************************/
/**
 * Hash the d-byte value pointed to by p and mod 2^f into the frequency vector
 */
pub unsafe fn FASTCOVER_hashPtrToIndex(p: *const c_void, f: U32, d: c_uint) -> usize {
    if d == 6 {
        return ZSTD_hash6Ptr(p, f);
    }
    ZSTD_hash8Ptr(p, f)
}

/*-*************************************
* Acceleration
***************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FASTCOVER_accel_t {
    /** Percentage of training samples used for ZDICT_finalizeDictionary */
    pub finalize: c_uint,
    /** Number of dmer skipped between each dmer counted in computeFrequency */
    pub skip: c_uint,
}

pub static FASTCOVER_defaultAccelParameters: [FASTCOVER_accel_t; FASTCOVER_MAX_ACCEL + 1] = [
    FASTCOVER_accel_t {
        finalize: 100,
        skip: 0,
    }, /* accel = 0, should not happen because accel = 0 defaults to accel = 1 */
    FASTCOVER_accel_t {
        finalize: 100,
        skip: 0,
    }, /* accel = 1 */
    FASTCOVER_accel_t {
        finalize: 50,
        skip: 1,
    }, /* accel = 2 */
    FASTCOVER_accel_t {
        finalize: 34,
        skip: 2,
    }, /* accel = 3 */
    FASTCOVER_accel_t {
        finalize: 25,
        skip: 3,
    }, /* accel = 4 */
    FASTCOVER_accel_t {
        finalize: 20,
        skip: 4,
    }, /* accel = 5 */
    FASTCOVER_accel_t {
        finalize: 17,
        skip: 5,
    }, /* accel = 6 */
    FASTCOVER_accel_t {
        finalize: 14,
        skip: 6,
    }, /* accel = 7 */
    FASTCOVER_accel_t {
        finalize: 13,
        skip: 7,
    }, /* accel = 8 */
    FASTCOVER_accel_t {
        finalize: 11,
        skip: 8,
    }, /* accel = 9 */
    FASTCOVER_accel_t {
        finalize: 10,
        skip: 9,
    }, /* accel = 10 */
];

/*-*************************************
* Context
***************************************/
#[repr(C)]
pub struct FASTCOVER_ctx_t {
    pub samples: *const BYTE,
    pub offsets: *mut usize,
    pub samplesSizes: *const usize,
    pub nbSamples: usize,
    pub nbTrainSamples: usize,
    pub nbTestSamples: usize,
    pub nbDmers: usize,
    pub freqs: *mut U32,
    pub d: c_uint,
    pub f: c_uint,
    pub accelParams: FASTCOVER_accel_t,
}

/*-*************************************
*  Helper functions
***************************************/
/**
 * Selects the best segment in an epoch.
 * Segments of are scored according to the function:
 *
 * Let F(d) be the frequency of all dmers with hash value d.
 * Let S_i be hash value of the dmer at position i of segment S which has length k.
 *
 *     Score(S) = F(S_1) + F(S_2) + ... + F(S_{k-d+1})
 *
 * Once the dmer with hash value d is in the dictionary we set F(d) = 0.
 */
pub unsafe fn FASTCOVER_selectSegment(
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
    let mut activeSegment: COVER_segment_t = COVER_segment_t {
        begin: 0,
        end: 0,
        score: 0,
    };

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
            /* Subtract frequency of this index from score if this is the last occurrence of this index in active segment */
            if *segmentFreqs.add(delIndex) == 0 {
                activeSegment.score = activeSegment.score.wrapping_sub(*freqs.add(delIndex));
            }
            /* Increment start of segment */
            activeSegment.begin = activeSegment.begin.wrapping_add(1);
        }

        /* If this segment is the best so far save it */
        if activeSegment.score > bestSegment.score {
            bestSegment.begin = activeSegment.begin;
            bestSegment.end = activeSegment.end;
            bestSegment.score = activeSegment.score;
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
        /*  Zero the frequency of hash value of each dmer covered by the chosen segment. */
        let mut pos: U32;
        pos = bestSegment.begin;
        while pos != bestSegment.end {
            let i: usize =
                FASTCOVER_hashPtrToIndex((*ctx).samples.add(pos as usize) as *const c_void, f, d);
            *freqs.add(i) = 0;
            pos = pos.wrapping_add(1);
        }
    }

    bestSegment
}

pub unsafe fn FASTCOVER_checkParameters(
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
    if f > FASTCOVER_MAX_F as c_uint || f == 0 {
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

/**
 * Clean up a context initialized with `FASTCOVER_ctx_init()`.
 */
pub unsafe fn FASTCOVER_ctx_destroy(ctx: *mut FASTCOVER_ctx_t) {
    if ctx.is_null() {
        return;
    }

    free((*ctx).freqs as *mut c_void);
    (*ctx).freqs = core::ptr::null_mut();

    free((*ctx).offsets as *mut c_void);
    (*ctx).offsets = core::ptr::null_mut();
}

/**
 * Calculate for frequency of hash value of each dmer in ctx->samples
 */
pub unsafe fn FASTCOVER_computeFrequency(freqs: *mut U32, ctx: *const FASTCOVER_ctx_t) {
    let f: c_uint = (*ctx).f;
    let d: c_uint = (*ctx).d;
    let skip: c_uint = (*ctx).accelParams.skip;
    let readLength: c_uint = MAX(d, 8 as c_uint);
    let mut i: usize;
    i = 0;
    while i < (*ctx).nbTrainSamples {
        let mut start: usize = *(*ctx).offsets.add(i); /* start of current dmer */
        let currSampleEnd: usize = *(*ctx).offsets.add(i + 1);
        while start.wrapping_add(readLength as usize) <= currSampleEnd {
            let dmerIndex: usize =
                FASTCOVER_hashPtrToIndex((*ctx).samples.add(start) as *const c_void, f, d);
            *freqs.add(dmerIndex) = (*freqs.add(dmerIndex)).wrapping_add(1);
            start = start.wrapping_add(skip as usize).wrapping_add(1);
        }
        i += 1;
    }
}

/**
 * Prepare a context for dictionary building.
 * The context is only dependent on the parameter `d` and can be used multiple
 * times.
 * Returns 0 on success or error code on error.
 * The context must be destroyed with `FASTCOVER_ctx_destroy()`.
 */
pub unsafe fn FASTCOVER_ctx_init(
    ctx: *mut FASTCOVER_ctx_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    d: c_uint,
    splitPoint: c_double,
    f: c_uint,
    accelParams: FASTCOVER_accel_t,
) -> usize {
    let samples: *const BYTE = samplesBuffer as *const BYTE;
    let totalSamplesSize: usize = crate::cover::COVER_sum(samplesSizes, nbSamples);
    /* Split samples into testing and training sets */
    let nbTrainSamples: c_uint = if splitPoint < 1.0 {
        ((nbSamples as c_double) * splitPoint) as c_uint
    } else {
        nbSamples
    };
    let nbTestSamples: c_uint = if splitPoint < 1.0 {
        nbSamples.wrapping_sub(nbTrainSamples)
    } else {
        nbSamples
    };
    let trainingSamplesSize: usize = if splitPoint < 1.0 {
        crate::cover::COVER_sum(samplesSizes, nbTrainSamples)
    } else {
        totalSamplesSize
    };
    let testSamplesSize: usize = if splitPoint < 1.0 {
        crate::cover::COVER_sum(samplesSizes.add(nbTrainSamples as usize), nbTestSamples)
    } else {
        totalSamplesSize
    };

    /* Checks */
    if totalSamplesSize < MAX(d as usize, core::mem::size_of::<U64>())
        || totalSamplesSize >= FASTCOVER_MAX_SAMPLES_SIZE as usize
    {
        DISPLAYLEVEL(1);
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Check if there are at least 5 training samples */
    if nbTrainSamples < 5 {
        DISPLAYLEVEL(1);
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Check if there's testing sample */
    if nbTestSamples < 1 {
        DISPLAYLEVEL(1);
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Zero the context */
    memset(
        ctx as *mut c_void,
        0,
        core::mem::size_of::<FASTCOVER_ctx_t>(),
    );
    DISPLAYLEVEL(2);
    DISPLAYLEVEL(2);

    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as usize;
    (*ctx).nbTrainSamples = nbTrainSamples as usize;
    (*ctx).nbTestSamples = nbTestSamples as usize;
    (*ctx).nbDmers = trainingSamplesSize
        .wrapping_sub(MAX(d as usize, core::mem::size_of::<U64>()))
        .wrapping_add(1);
    (*ctx).d = d;
    (*ctx).f = f;
    (*ctx).accelParams = accelParams;

    /* The offsets of each file */
    (*ctx).offsets = calloc(
        nbSamples.wrapping_add(1) as usize,
        core::mem::size_of::<usize>(),
    ) as *mut usize;
    if (*ctx).offsets.is_null() {
        DISPLAYLEVEL(1);
        FASTCOVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    /* Fill offsets from the samplesSizes */
    {
        let mut i: U32;
        *(*ctx).offsets.add(0) = 0;
        i = 1;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) = (*(*ctx).offsets.add(i.wrapping_sub(1) as usize))
                .wrapping_add(*samplesSizes.add(i.wrapping_sub(1) as usize));
            i = i.wrapping_add(1);
        }
    }

    /* Initialize frequency array of size 2^f */
    /* `(U64)1 << f`: on x86-64 the shift count is masked to 6 bits, which
     * `wrapping_shl` reproduces exactly (C would be UB for f >= 64). */
    (*ctx).freqs = calloc(
        (1u64.wrapping_shl(f)) as usize,
        core::mem::size_of::<U32>(),
    ) as *mut U32;
    if (*ctx).freqs.is_null() {
        DISPLAYLEVEL(1);
        FASTCOVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    DISPLAYLEVEL(2);
    FASTCOVER_computeFrequency((*ctx).freqs, ctx);

    0
}

/**
 * Given the prepared context build the dictionary.
 */
pub unsafe fn FASTCOVER_buildDictionary(
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
    let epochs: COVER_epoch_info_t = crate::cover::COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).nbDmers as U32,
        parameters.k,
        1,
    );
    let maxZeroScoreRun: usize = 10;
    let mut zeroScoreRun: usize = 0;
    let mut epoch: usize;
    DISPLAYLEVEL(2);
    /* Loop through the epochs until there are no more segments or the dictionary
     * is full.
     */
    epoch = 0;
    'epochLoop: while tail > 0 {
        let epochBegin: U32 = (epoch.wrapping_mul(epochs.size as usize)) as U32;
        let epochEnd: U32 = epochBegin.wrapping_add(epochs.size);
        let segmentSize: usize;
        /* Select a segment */
        let segment: COVER_segment_t =
            FASTCOVER_selectSegment(ctx, freqs, epochBegin, epochEnd, parameters, segmentFreqs);

        /* If the segment covers no dmers, then we are out of content.
         * There may be new content in other epochs, for continue for some time.
         */
        if segment.score == 0 {
            zeroScoreRun = zeroScoreRun.wrapping_add(1);
            if zeroScoreRun >= maxZeroScoreRun {
                break 'epochLoop;
            }
            epoch = (epoch.wrapping_add(1)) % (epochs.num as usize);
            continue 'epochLoop;
        }
        zeroScoreRun = 0;

        /* Trim the segment if necessary and if it is too small then we are done */
        segmentSize = MIN(
            segment
                .end
                .wrapping_sub(segment.begin)
                .wrapping_add(parameters.d)
                .wrapping_sub(1) as usize,
            tail,
        );
        if segmentSize < parameters.d as usize {
            break 'epochLoop;
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
        DISPLAYUPDATE(2);

        epoch = (epoch.wrapping_add(1)) % (epochs.num as usize);
    }
    DISPLAYLEVEL(2);
    tail
}

/**
 * Parameters for FASTCOVER_tryParameters().
 */
#[repr(C)]
pub struct FASTCOVER_tryParameters_data_s {
    pub ctx: *const FASTCOVER_ctx_t,
    pub best: *mut COVER_best_t,
    pub dictBufferCapacity: usize,
    pub parameters: ZDICT_cover_params_t,
}

pub type FASTCOVER_tryParameters_data_t = FASTCOVER_tryParameters_data_s;

/**
 * Tries a set of parameters and updates the COVER_best_t with the results.
 * This function is thread safe if zstd is compiled with multithreaded support.
 * It takes its parameters as an *OWNING* opaque pointer to support threading.
 */
pub unsafe extern "C" fn FASTCOVER_tryParameters(opaque: *mut c_void) {
    /* Save parameters as local variables */
    let data: *mut FASTCOVER_tryParameters_data_t = opaque as *mut FASTCOVER_tryParameters_data_t;
    let ctx: *const FASTCOVER_ctx_t = (*data).ctx;
    let parameters: ZDICT_cover_params_t = (*data).parameters;
    let dictBufferCapacity: usize = (*data).dictBufferCapacity;
    let totalCompressedSize: usize = ERROR(ZSTD_error_GENERIC);
    /* Initialize array to keep track of frequency of dmer within activeSegment */
    let segmentFreqs: *mut U16 = calloc(
        (1u64.wrapping_shl((*ctx).f)) as usize,
        core::mem::size_of::<U16>(),
    ) as *mut U16;
    /* Allocate space for hash table, dict, and freqs */
    let dict: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection: COVER_dictSelection_t =
        crate::cover::COVER_dictSelectionError(ERROR(ZSTD_error_GENERIC));
    let freqs: *mut U32 = malloc(
        (1u64.wrapping_shl((*ctx).f)) as usize * core::mem::size_of::<U32>(),
    ) as *mut U32;

    '_cleanup: {
        if segmentFreqs.is_null() || dict.is_null() || freqs.is_null() {
            DISPLAYLEVEL(1);
            break '_cleanup;
        }
        /* Copy the frequencies because we need to modify them */
        memcpy(
            freqs as *mut c_void,
            (*ctx).freqs as *const c_void,
            (1u64.wrapping_shl((*ctx).f)) as usize * core::mem::size_of::<U32>(),
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
            selection = crate::cover::COVER_selectDict(
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

            if crate::cover::COVER_dictSelectionIsError(selection) != 0 {
                DISPLAYLEVEL(1);
                break '_cleanup;
            }
        }
    }
    /* _cleanup: */
    free(dict as *mut c_void);
    crate::cover::COVER_best_finish((*data).best, parameters, selection);
    free(data as *mut c_void);
    free(segmentFreqs as *mut c_void);
    crate::cover::COVER_dictSelectionFree(selection);
    free(freqs as *mut c_void);
}

pub unsafe fn FASTCOVER_convertToCoverParams(
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

pub unsafe fn FASTCOVER_convertToFastCoverParams(
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
    let mut accelParams: FASTCOVER_accel_t;
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
        core::ptr::addr_of_mut!(coverParams) as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(parameters, core::ptr::addr_of_mut!(coverParams));
    /* Checks */
    if FASTCOVER_checkParameters(
        coverParams,
        dictBufferCapacity,
        parameters.f,
        parameters.accel,
    ) == 0
    {
        DISPLAYLEVEL(1);
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        DISPLAYLEVEL(1);
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        DISPLAYLEVEL(1);
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    /* Assign corresponding FASTCOVER_accel_t to accelParams*/
    accelParams = FASTCOVER_defaultAccelParameters[parameters.accel as usize];
    /* Initialize context */
    {
        let initVal: usize = FASTCOVER_ctx_init(
            core::ptr::addr_of_mut!(ctx),
            samplesBuffer,
            samplesSizes,
            nbSamples,
            coverParams.d,
            parameters.splitPoint,
            parameters.f,
            accelParams,
        );
        if ZSTD_isError(initVal) != 0 {
            DISPLAYLEVEL(1);
            return initVal;
        }
    }
    crate::cover::COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, g_displayLevel);
    /* Build the dictionary */
    DISPLAYLEVEL(2);
    {
        /* Initialize array to keep track of frequency of dmer within activeSegment */
        let segmentFreqs: *mut U16 = calloc(
            (1u64.wrapping_shl(parameters.f)) as usize,
            core::mem::size_of::<U16>(),
        ) as *mut U16;
        let tail: usize = FASTCOVER_buildDictionary(
            core::ptr::addr_of!(ctx),
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
        let dictionarySize: usize = crate::zdict::ZDICT_finalizeDictionary(
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
            DISPLAYLEVEL(2);
        }
        FASTCOVER_ctx_destroy(core::ptr::addr_of_mut!(ctx));
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
    let mut accelParams: FASTCOVER_accel_t;
    /* constants */
    let nbThreads: c_uint = (*parameters).nbThreads;
    let splitPoint: c_double = if (*parameters).splitPoint <= 0.0 {
        FASTCOVER_DEFAULT_SPLITPOINT
    } else {
        (*parameters).splitPoint
    };
    let kMinD: c_uint = if (*parameters).d == 0 {
        6
    } else {
        (*parameters).d
    };
    let kMaxD: c_uint = if (*parameters).d == 0 {
        8
    } else {
        (*parameters).d
    };
    let kMinK: c_uint = if (*parameters).k == 0 {
        50
    } else {
        (*parameters).k
    };
    let kMaxK: c_uint = if (*parameters).k == 0 {
        2000
    } else {
        (*parameters).k
    };
    let kSteps: c_uint = if (*parameters).steps == 0 {
        40
    } else {
        (*parameters).steps
    };
    let kStepSize: c_uint = MAX(kMaxK.wrapping_sub(kMinK) / kSteps, 1 as c_uint);
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
    let mut pool: *mut POOL_ctx = core::ptr::null_mut();
    let mut warned: c_int = 0;
    /* Checks */
    if splitPoint <= 0.0 || splitPoint > 1.0 {
        LOCALDISPLAYLEVEL(displayLevel, 1);
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if accel == 0 || accel > FASTCOVER_MAX_ACCEL as c_uint {
        LOCALDISPLAYLEVEL(displayLevel, 1);
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        LOCALDISPLAYLEVEL(displayLevel, 1);
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        LOCALDISPLAYLEVEL(displayLevel, 1);
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        LOCALDISPLAYLEVEL(displayLevel, 1);
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbThreads > 1 {
        pool = crate::pool::POOL_create(nbThreads as usize, 1);
        if pool.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
    }
    /* Initialization */
    crate::cover::COVER_best_init(core::ptr::addr_of_mut!(best));
    memset(
        core::ptr::addr_of_mut!(coverParams) as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(*parameters, core::ptr::addr_of_mut!(coverParams));
    accelParams = FASTCOVER_defaultAccelParameters[accel as usize];
    /* Turn down global display level to clean up display at level 2 and below */
    g_displayLevel = if displayLevel == 0 {
        0
    } else {
        displayLevel - 1
    };
    /* Loop through d first because each new value needs a new context */
    LOCALDISPLAYLEVEL(displayLevel, 2);
    d = kMinD;
    while d <= kMaxD {
        /* Initialize the context for this value of d */
        let mut ctx: FASTCOVER_ctx_t = core::mem::zeroed();
        LOCALDISPLAYLEVEL(displayLevel, 3);
        {
            let initVal: usize = FASTCOVER_ctx_init(
                core::ptr::addr_of_mut!(ctx),
                samplesBuffer,
                samplesSizes,
                nbSamples,
                d,
                splitPoint,
                f,
                accelParams,
            );
            if ZSTD_isError(initVal) != 0 {
                LOCALDISPLAYLEVEL(displayLevel, 1);
                crate::cover::COVER_best_destroy(core::ptr::addr_of_mut!(best));
                crate::pool::POOL_free(pool);
                return initVal;
            }
        }
        if warned == 0 {
            crate::cover::COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, displayLevel);
            warned = 1;
        }
        /* Loop through k reusing the same context */
        k = kMinK;
        while k <= kMaxK {
            /* Prepare the arguments */
            let data: *mut FASTCOVER_tryParameters_data_t =
                malloc(core::mem::size_of::<FASTCOVER_tryParameters_data_t>())
                    as *mut FASTCOVER_tryParameters_data_t;
            LOCALDISPLAYLEVEL(displayLevel, 3);
            if data.is_null() {
                LOCALDISPLAYLEVEL(displayLevel, 1);
                crate::cover::COVER_best_destroy(core::ptr::addr_of_mut!(best));
                FASTCOVER_ctx_destroy(core::ptr::addr_of_mut!(ctx));
                crate::pool::POOL_free(pool);
                return ERROR(ZSTD_error_memory_allocation);
            }
            (*data).ctx = core::ptr::addr_of!(ctx);
            (*data).best = core::ptr::addr_of_mut!(best);
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
                DISPLAYLEVEL(1);
                free(data as *mut c_void);
                k = k.wrapping_add(kStepSize);
                continue;
            }
            /* Call the function and pass ownership of data to it */
            crate::cover::COVER_best_start(core::ptr::addr_of_mut!(best));
            if !pool.is_null() {
                crate::pool::POOL_add(pool, Some(FASTCOVER_tryParameters), data as *mut c_void);
            } else {
                FASTCOVER_tryParameters(data as *mut c_void);
            }
            /* Print status */
            LOCALDISPLAYUPDATE(displayLevel, 2);
            iteration = iteration.wrapping_add(1);

            k = k.wrapping_add(kStepSize);
        }
        crate::cover::COVER_best_wait(core::ptr::addr_of_mut!(best));
        FASTCOVER_ctx_destroy(core::ptr::addr_of_mut!(ctx));

        d = d.wrapping_add(2);
    }
    LOCALDISPLAYLEVEL(displayLevel, 2);
    /* Fill the output buffer and parameters with output of the best parameters */
    {
        let dictSize: usize = best.dictSize;
        if ZSTD_isError(best.compressedSize) != 0 {
            let compressedSize: usize = best.compressedSize;
            crate::cover::COVER_best_destroy(core::ptr::addr_of_mut!(best));
            crate::pool::POOL_free(pool);
            return compressedSize;
        }
        FASTCOVER_convertToFastCoverParams(best.parameters, parameters, f, accel);
        memcpy(dictBuffer, best.dict as *const c_void, dictSize);
        crate::cover::COVER_best_destroy(core::ptr::addr_of_mut!(best));
        crate::pool::POOL_free(pool);
        return dictSize;
    }
}
