//! Translation of `dictBuilder/fastcover.c`.
//!
//! Build config: ZSTD_MULTITHREAD not defined (POOL_add runs synchronously),
//! DEBUGLEVEL 0 (asserts/DEBUGLOG dropped). Shared COVER_* types and helpers
//! come from `crate::dictbuilder::cover`.

#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::common::error_private::{ERR_isError, ERROR};
use crate::common::error_private::{
    ZSTD_error_GENERIC, ZSTD_error_dstSize_tooSmall, ZSTD_error_memory_allocation,
    ZSTD_error_parameter_outOfBound, ZSTD_error_srcSize_wrong,
};
use crate::common::mem::{size_t, BYTE, U16, U32, U64};
use crate::common::pool::{POOL_add, POOL_create, POOL_ctx, POOL_free};
use crate::common::zstd_internal::{calloc, free, malloc, memcpy, memset, MAX, MIN};

use crate::compress::zstd_compress_internal::{ZSTD_hash6Ptr, ZSTD_hash8Ptr};

use crate::dictbuilder::cover::*;

/* console display (fastcover has its own file-static g_displayLevel) */
unsafe extern "C" {
    static stderr: *mut c_void;
    fn fputs(s: *const c_char, stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clock() -> i64;
}

/* external public API (ZDICT_finalizeDictionary owned by concurrent agent) */
unsafe extern "C" {
    fn ZDICT_finalizeDictionary(
        dstDictBuffer: *mut c_void,
        maxDictSize: size_t,
        dictContent: *const c_void,
        dictContentSize: size_t,
        samplesBuffer: *const c_void,
        samplesSizes: *const size_t,
        nbSamples: c_uint,
        parameters: ZDICT_params_t,
    ) -> size_t;
}

#[inline(always)]
unsafe fn ZSTD_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

/* ===================================================================== */
/*  Constants                                                            */
/* ===================================================================== */

const FASTCOVER_MAX_SAMPLES_SIZE: c_uint = 0xFFFFFFFF; /* 64-bit build */
const FASTCOVER_MAX_F: c_uint = 31;
const FASTCOVER_MAX_ACCEL: c_uint = 10;
const FASTCOVER_DEFAULT_SPLITPOINT: f64 = 0.75;
const DEFAULT_F: c_uint = 20;
const DEFAULT_ACCEL: c_uint = 1;

/* ===================================================================== */
/*  Console display (file-local g_displayLevel, mirrors cover.c)         */
/* ===================================================================== */

static mut g_displayLevel: c_int = 0;
static mut g_time: i64 = 0;
const G_REFRESH_RATE: i64 = 1_000_000i64 * 15 / 100;

unsafe fn emit(s: &str) {
    let mut buf: std::vec::Vec<u8> = std::vec::Vec::with_capacity(s.len() + 1);
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    fputs(buf.as_ptr() as *const c_char, stderr);
    fflush(stderr);
}
unsafe fn display_fmt(l: c_int, s: &str) {
    if g_displayLevel >= l {
        emit(s);
    }
}
unsafe fn local_display_fmt(displayLevel: c_int, l: c_int, s: &str) {
    if displayLevel >= l {
        emit(s);
    }
}
unsafe fn display_update(l: c_int, s: &str) {
    if g_displayLevel >= l {
        if (clock() - g_time > G_REFRESH_RATE) || (g_displayLevel >= 4) {
            g_time = clock();
            emit(s);
        }
    }
}
unsafe fn local_display_update(displayLevel: c_int, l: c_int, s: &str) {
    if displayLevel >= l {
        if (clock() - g_time > G_REFRESH_RATE) || (displayLevel >= 4) {
            g_time = clock();
            emit(s);
        }
    }
}

/* ===================================================================== */
/*  Hash Functions                                                       */
/* ===================================================================== */

/// Hash the d-byte value pointed to by p and mod 2^f into the frequency vector.
unsafe fn FASTCOVER_hashPtrToIndex(p: *const c_void, f: U32, d: c_uint) -> size_t {
    if d == 6 {
        return ZSTD_hash6Ptr(p, f);
    }
    ZSTD_hash8Ptr(p, f)
}

/* ===================================================================== */
/*  Acceleration                                                         */
/* ===================================================================== */

#[repr(C)]
#[derive(Clone, Copy)]
struct FASTCOVER_accel_t {
    finalize: c_uint,
    skip: c_uint,
}

static FASTCOVER_defaultAccelParameters: [FASTCOVER_accel_t; (FASTCOVER_MAX_ACCEL + 1) as usize] = [
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

/* ===================================================================== */
/*  Context                                                              */
/* ===================================================================== */

#[repr(C)]
struct FASTCOVER_ctx_t {
    samples: *const BYTE,
    offsets: *mut size_t,
    samplesSizes: *const size_t,
    nbSamples: size_t,
    nbTrainSamples: size_t,
    nbTestSamples: size_t,
    nbDmers: size_t,
    freqs: *mut U32,
    d: c_uint,
    f: c_uint,
    accelParams: FASTCOVER_accel_t,
}

/* ===================================================================== */
/*  Helper functions                                                     */
/* ===================================================================== */

/// Selects the best segment in an epoch.
unsafe fn FASTCOVER_selectSegment(
    ctx: *const FASTCOVER_ctx_t,
    freqs: *mut U32,
    begin: U32,
    end: U32,
    parameters: ZDICT_cover_params_t,
    segmentFreqs: *mut U16,
) -> COVER_segment_t {
    let k = parameters.k;
    let d = parameters.d;
    let f = (*ctx).f;
    let dmersInK = k - d + 1;

    let mut bestSegment = COVER_segment_t {
        begin: 0,
        end: 0,
        score: 0,
    };
    let mut activeSegment = COVER_segment_t {
        begin: 0,
        end: 0,
        score: 0,
    };

    activeSegment.begin = begin;
    activeSegment.end = begin;
    activeSegment.score = 0;

    while activeSegment.end < end {
        let idx =
            FASTCOVER_hashPtrToIndex((*ctx).samples.add(activeSegment.end as usize) as *const c_void, f, d);

        if *segmentFreqs.add(idx) == 0 {
            activeSegment.score = activeSegment.score.wrapping_add(*freqs.add(idx));
        }
        activeSegment.end += 1;
        *segmentFreqs.add(idx) += 1;
        if activeSegment.end - activeSegment.begin == dmersInK + 1 {
            let delIndex = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.add(activeSegment.begin as usize) as *const c_void,
                f,
                d,
            );
            *segmentFreqs.add(delIndex) -= 1;
            if *segmentFreqs.add(delIndex) == 0 {
                activeSegment.score = activeSegment.score.wrapping_sub(*freqs.add(delIndex));
            }
            activeSegment.begin += 1;
        }

        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }

    /* Zero out rest of segmentFreqs array */
    while activeSegment.begin < end {
        let delIndex = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.add(activeSegment.begin as usize) as *const c_void,
            f,
            d,
        );
        *segmentFreqs.add(delIndex) -= 1;
        activeSegment.begin += 1;
    }

    {
        let mut pos = bestSegment.begin;
        while pos != bestSegment.end {
            let i = FASTCOVER_hashPtrToIndex((*ctx).samples.add(pos as usize) as *const c_void, f, d);
            *freqs.add(i) = 0;
            pos += 1;
        }
    }

    bestSegment
}

unsafe fn FASTCOVER_checkParameters(
    parameters: ZDICT_cover_params_t,
    maxDictSize: size_t,
    f: c_uint,
    accel: c_uint,
) -> c_int {
    if parameters.d == 0 || parameters.k == 0 {
        return 0;
    }
    if parameters.d != 6 && parameters.d != 8 {
        return 0;
    }
    if parameters.k as size_t > maxDictSize {
        return 0;
    }
    if parameters.d > parameters.k {
        return 0;
    }
    if f > FASTCOVER_MAX_F || f == 0 {
        return 0;
    }
    if parameters.splitPoint <= 0.0 || parameters.splitPoint > 1.0 {
        return 0;
    }
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

/// Calculate the frequency of hash value of each dmer in ctx->samples.
unsafe fn FASTCOVER_computeFrequency(freqs: *mut U32, ctx: *const FASTCOVER_ctx_t) {
    let f = (*ctx).f;
    let d = (*ctx).d;
    let skip = (*ctx).accelParams.skip;
    let readLength = MAX(d, 8);
    let mut i: size_t = 0;
    while i < (*ctx).nbTrainSamples {
        let mut start = *(*ctx).offsets.add(i as usize);
        let currSampleEnd = *(*ctx).offsets.add((i + 1) as usize);
        while start + readLength as size_t <= currSampleEnd {
            let dmerIndex =
                FASTCOVER_hashPtrToIndex((*ctx).samples.add(start as usize) as *const c_void, f, d);
            *freqs.add(dmerIndex) += 1;
            start = start + skip as size_t + 1;
        }
        i += 1;
    }
}

/// Prepare a context for dictionary building.
unsafe fn FASTCOVER_ctx_init(
    ctx: *mut FASTCOVER_ctx_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    d: c_uint,
    splitPoint: f64,
    f: c_uint,
    accelParams: FASTCOVER_accel_t,
) -> size_t {
    let samples = samplesBuffer as *const BYTE;
    let totalSamplesSize = COVER_sum(samplesSizes, nbSamples);
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
    let trainingSamplesSize = if splitPoint < 1.0 {
        COVER_sum(samplesSizes, nbTrainSamples)
    } else {
        totalSamplesSize
    };
    let testSamplesSize = if splitPoint < 1.0 {
        COVER_sum(samplesSizes.add(nbTrainSamples as usize), nbTestSamples)
    } else {
        totalSamplesSize
    };

    if totalSamplesSize < MAX(d as size_t, core::mem::size_of::<U64>())
        || totalSamplesSize >= FASTCOVER_MAX_SAMPLES_SIZE as size_t
    {
        display_fmt(
            1,
            &format!(
                "Total samples size is too large ({} MB), maximum size is {} MB\n",
                (totalSamplesSize >> 20) as c_uint,
                (FASTCOVER_MAX_SAMPLES_SIZE >> 20)
            ),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if nbTrainSamples < 5 {
        display_fmt(
            1,
            &format!(
                "Total number of training samples is {} and is invalid\n",
                nbTrainSamples
            ),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    if nbTestSamples < 1 {
        display_fmt(
            1,
            &format!(
                "Total number of testing samples is {} and is invalid.\n",
                nbTestSamples
            ),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    memset(ctx as *mut c_void, 0, core::mem::size_of::<FASTCOVER_ctx_t>());
    display_fmt(
        2,
        &format!(
            "Training on {} samples of total size {}\n",
            nbTrainSamples, trainingSamplesSize as c_uint
        ),
    );
    display_fmt(
        2,
        &format!(
            "Testing on {} samples of total size {}\n",
            nbTestSamples, testSamplesSize as c_uint
        ),
    );

    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as size_t;
    (*ctx).nbTrainSamples = nbTrainSamples as size_t;
    (*ctx).nbTestSamples = nbTestSamples as size_t;
    (*ctx).nbDmers = trainingSamplesSize - MAX(d as size_t, core::mem::size_of::<U64>()) + 1;
    (*ctx).d = d;
    (*ctx).f = f;
    (*ctx).accelParams = accelParams;

    (*ctx).offsets =
        calloc(nbSamples as size_t + 1, core::mem::size_of::<size_t>()) as *mut size_t;
    if (*ctx).offsets.is_null() {
        display_fmt(1, "Failed to allocate scratch buffers \n");
        FASTCOVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    {
        *(*ctx).offsets.add(0) = 0;
        let mut i: U32 = 1;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) = *(*ctx).offsets.add((i - 1) as usize)
                + *samplesSizes.add((i - 1) as usize);
            i += 1;
        }
    }

    (*ctx).freqs = calloc((1u64 << f) as size_t, core::mem::size_of::<U32>()) as *mut U32;
    if (*ctx).freqs.is_null() {
        display_fmt(1, "Failed to allocate frequency table \n");
        FASTCOVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    display_fmt(2, "Computing frequencies\n");
    FASTCOVER_computeFrequency((*ctx).freqs, ctx);

    0
}

/// Given the prepared context build the dictionary.
unsafe fn FASTCOVER_buildDictionary(
    ctx: *const FASTCOVER_ctx_t,
    freqs: *mut U32,
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    parameters: ZDICT_cover_params_t,
    segmentFreqs: *mut U16,
) -> size_t {
    let dict = dictBuffer as *mut BYTE;
    let mut tail = dictBufferCapacity;
    let epochs = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).nbDmers as U32,
        parameters.k,
        1,
    );
    let maxZeroScoreRun: size_t = 10;
    let mut zeroScoreRun: size_t = 0;
    let mut epoch: size_t = 0;
    display_fmt(
        2,
        &format!(
            "Breaking content into {} epochs of size {}\n",
            epochs.num, epochs.size
        ),
    );
    while tail > 0 {
        let epochBegin = (epoch * epochs.size as size_t) as U32;
        let epochEnd = epochBegin + epochs.size;
        let segment =
            FASTCOVER_selectSegment(ctx, freqs, epochBegin, epochEnd, parameters, segmentFreqs);

        if segment.score == 0 {
            zeroScoreRun += 1;
            if zeroScoreRun >= maxZeroScoreRun {
                break;
            }
            epoch = (epoch + 1) % epochs.num as size_t;
            continue;
        }
        zeroScoreRun = 0;

        let segmentSize = MIN(
            (segment.end - segment.begin + parameters.d - 1) as size_t,
            tail,
        );
        if segmentSize < parameters.d as size_t {
            break;
        }

        tail -= segmentSize;
        memcpy(
            dict.add(tail) as *mut c_void,
            (*ctx).samples.add(segment.begin as usize) as *const c_void,
            segmentSize,
        );
        display_update(
            2,
            &format!(
                "\r{}%       ",
                (((dictBufferCapacity - tail) * 100) / dictBufferCapacity) as c_uint
            ),
        );
        epoch = (epoch + 1) % epochs.num as size_t;
    }
    display_fmt(2, &format!("\r{:79}\r", ""));
    tail
}

/// Parameters for FASTCOVER_tryParameters().
#[repr(C)]
struct FASTCOVER_tryParameters_data_t {
    ctx: *const FASTCOVER_ctx_t,
    best: *mut COVER_best_t,
    dictBufferCapacity: size_t,
    parameters: ZDICT_cover_params_t,
}

/// Tries a set of parameters and updates the COVER_best_t with the results.
unsafe extern "C" fn FASTCOVER_tryParameters(opaque: *mut c_void) {
    let data = opaque as *mut FASTCOVER_tryParameters_data_t;
    let ctx = (*data).ctx;
    let parameters = (*data).parameters;
    let dictBufferCapacity = (*data).dictBufferCapacity;
    let totalCompressedSize: size_t = ERROR(ZSTD_error_GENERIC);
    let segmentFreqs = calloc((1u64 << (*ctx).f) as size_t, core::mem::size_of::<U16>()) as *mut U16;
    let dict = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection = COVER_dictSelectionError(ERROR(ZSTD_error_GENERIC));
    let freqs = malloc((1u64 << (*ctx).f) as size_t * core::mem::size_of::<U32>()) as *mut U32;
    'cleanup: {
        if segmentFreqs.is_null() || dict.is_null() || freqs.is_null() {
            display_fmt(1, "Failed to allocate buffers: out of memory\n");
            break 'cleanup;
        }
        memcpy(
            freqs as *mut c_void,
            (*ctx).freqs as *const c_void,
            (1u64 << (*ctx).f) as size_t * core::mem::size_of::<U32>(),
        );
        {
            let tail = FASTCOVER_buildDictionary(
                ctx,
                freqs,
                dict as *mut c_void,
                dictBufferCapacity,
                parameters,
                segmentFreqs,
            );

            let nbFinalizeSamples =
                ((*ctx).nbTrainSamples * (*ctx).accelParams.finalize as size_t / 100) as c_uint;
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
                display_fmt(1, "Failed to select dictionary\n");
                break 'cleanup;
            }
        }
    }
    /* _cleanup: */
    free(dict as *mut c_void);
    COVER_best_finish((*data).best, parameters, selection);
    free(data as *mut c_void);
    free(segmentFreqs as *mut c_void);
    COVER_dictSelectionFree(selection);
    free(freqs as *mut c_void);
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    mut parameters: ZDICT_fastCover_params_t,
) -> size_t {
    let dict = dictBuffer as *mut BYTE;
    let mut ctx: FASTCOVER_ctx_t = core::mem::zeroed();
    let mut coverParams: ZDICT_cover_params_t = core::mem::zeroed();
    let accelParams: FASTCOVER_accel_t;
    /* Initialize global data */
    g_displayLevel = parameters.zParams.notificationLevel as c_int;
    /* Assign splitPoint and f if not provided */
    parameters.splitPoint = 1.0;
    parameters.f = if parameters.f == 0 { DEFAULT_F } else { parameters.f };
    parameters.accel = if parameters.accel == 0 {
        DEFAULT_ACCEL
    } else {
        parameters.accel
    };
    /* Convert to cover parameter */
    memset(
        &mut coverParams as *mut ZDICT_cover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
    FASTCOVER_convertToCoverParams(parameters, &mut coverParams);
    /* Checks */
    if FASTCOVER_checkParameters(coverParams, dictBufferCapacity, parameters.f, parameters.accel)
        == 0
    {
        display_fmt(1, "FASTCOVER parameters incorrect\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        display_fmt(1, "FASTCOVER must have at least one input file\n");
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        display_fmt(
            1,
            &format!(
                "dictBufferCapacity must be at least {}\n",
                ZDICT_DICTSIZE_MIN as c_uint
            ),
        );
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    /* Assign corresponding FASTCOVER_accel_t to accelParams */
    accelParams = FASTCOVER_defaultAccelParameters[parameters.accel as usize];
    /* Initialize context */
    {
        let initVal = FASTCOVER_ctx_init(
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
            display_fmt(1, "Failed to initialize context\n");
            return initVal;
        }
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, g_displayLevel);
    /* Build the dictionary */
    display_fmt(2, "Building dictionary\n");
    {
        let segmentFreqs =
            calloc((1u64 << parameters.f) as size_t, core::mem::size_of::<U16>()) as *mut U16;
        let tail = FASTCOVER_buildDictionary(
            &ctx,
            ctx.freqs,
            dictBuffer,
            dictBufferCapacity,
            coverParams,
            segmentFreqs,
        );
        let nbFinalizeSamples =
            (ctx.nbTrainSamples * ctx.accelParams.finalize as size_t / 100) as c_uint;
        let dictionarySize = ZDICT_finalizeDictionary(
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
            display_fmt(
                2,
                &format!("Constructed dictionary of size {}\n", dictionarySize as c_uint),
            );
        }
        FASTCOVER_ctx_destroy(&mut ctx);
        free(segmentFreqs as *mut c_void);
        dictionarySize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_optimizeTrainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    parameters: *mut ZDICT_fastCover_params_t,
) -> size_t {
    let mut coverParams: ZDICT_cover_params_t = core::mem::zeroed();
    let accelParams: FASTCOVER_accel_t;
    let nbThreads = (*parameters).nbThreads;
    let splitPoint = if (*parameters).splitPoint <= 0.0 {
        FASTCOVER_DEFAULT_SPLITPOINT
    } else {
        (*parameters).splitPoint
    };
    let kMinD = if (*parameters).d == 0 { 6 } else { (*parameters).d };
    let kMaxD = if (*parameters).d == 0 { 8 } else { (*parameters).d };
    let kMinK = if (*parameters).k == 0 { 50 } else { (*parameters).k };
    let kMaxK = if (*parameters).k == 0 { 2000 } else { (*parameters).k };
    let kSteps = if (*parameters).steps == 0 {
        40
    } else {
        (*parameters).steps
    };
    let kStepSize = MAX((kMaxK - kMinK) / kSteps, 1);
    let kIterations = (1 + (kMaxD - kMinD) / 2) * (1 + (kMaxK - kMinK) / kStepSize);
    let f = if (*parameters).f == 0 { DEFAULT_F } else { (*parameters).f };
    let accel = if (*parameters).accel == 0 {
        DEFAULT_ACCEL
    } else {
        (*parameters).accel
    };
    let shrinkDict: c_uint = 0;
    let displayLevel = (*parameters).zParams.notificationLevel as c_int;
    let mut iteration: c_uint = 1;
    let mut d: c_uint;
    let mut k: c_uint;
    let mut best: COVER_best_t = core::mem::zeroed();
    let mut pool: *mut POOL_ctx = core::ptr::null_mut();
    let mut warned: c_int = 0;
    /* Checks */
    if splitPoint <= 0.0 || splitPoint > 1.0 {
        local_display_fmt(displayLevel, 1, "Incorrect splitPoint\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if accel == 0 || accel > FASTCOVER_MAX_ACCEL {
        local_display_fmt(displayLevel, 1, "Incorrect accel\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        local_display_fmt(displayLevel, 1, "Incorrect k\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        local_display_fmt(displayLevel, 1, "FASTCOVER must have at least one input file\n");
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        local_display_fmt(
            displayLevel,
            1,
            &format!(
                "dictBufferCapacity must be at least {}\n",
                ZDICT_DICTSIZE_MIN as c_uint
            ),
        );
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbThreads > 1 {
        pool = POOL_create(nbThreads as size_t, 1);
        if pool.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
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
    g_displayLevel = if displayLevel == 0 { 0 } else { displayLevel - 1 };
    local_display_fmt(
        displayLevel,
        2,
        &format!("Trying {} different sets of parameters\n", kIterations),
    );
    d = kMinD;
    while d <= kMaxD {
        let mut ctx: FASTCOVER_ctx_t = core::mem::zeroed();
        local_display_fmt(displayLevel, 3, &format!("d={}\n", d));
        {
            let initVal = FASTCOVER_ctx_init(
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
                local_display_fmt(displayLevel, 1, "Failed to initialize context\n");
                COVER_best_destroy(&mut best);
                POOL_free(pool);
                return initVal;
            }
        }
        if warned == 0 {
            COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, displayLevel);
            warned = 1;
        }
        k = kMinK;
        while k <= kMaxK {
            let data = malloc(core::mem::size_of::<FASTCOVER_tryParameters_data_t>())
                as *mut FASTCOVER_tryParameters_data_t;
            local_display_fmt(displayLevel, 3, &format!("k={}\n", k));
            if data.is_null() {
                local_display_fmt(displayLevel, 1, "Failed to allocate parameters\n");
                COVER_best_destroy(&mut best);
                FASTCOVER_ctx_destroy(&mut ctx);
                POOL_free(pool);
                return ERROR(ZSTD_error_memory_allocation);
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
            if FASTCOVER_checkParameters(
                (*data).parameters,
                dictBufferCapacity,
                (*(*data).ctx).f,
                accel,
            ) == 0
            {
                display_fmt(1, "FASTCOVER parameters incorrect\n");
                free(data as *mut c_void);
                k += kStepSize;
                continue;
            }
            COVER_best_start(&mut best);
            if !pool.is_null() {
                POOL_add(pool, FASTCOVER_tryParameters, data as *mut c_void);
            } else {
                FASTCOVER_tryParameters(data as *mut c_void);
            }
            local_display_update(
                displayLevel,
                2,
                &format!("\r{}%       ", (iteration * 100) / kIterations),
            );
            iteration += 1;
            k += kStepSize;
        }
        COVER_best_wait(&mut best);
        FASTCOVER_ctx_destroy(&mut ctx);
        d += 2;
    }
    local_display_fmt(displayLevel, 2, &format!("\r{:79}\r", ""));
    {
        let dictSize = best.dictSize;
        if ZSTD_isError(best.compressedSize) != 0 {
            let compressedSize = best.compressedSize;
            COVER_best_destroy(&mut best);
            POOL_free(pool);
            return compressedSize;
        }
        FASTCOVER_convertToFastCoverParams(best.parameters, parameters, f, accel);
        memcpy(dictBuffer, best.dict as *const c_void, dictSize);
        COVER_best_destroy(&mut best);
        POOL_free(pool);
        dictSize
    }
}
