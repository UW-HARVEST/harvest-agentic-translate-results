//! Translation of `dictBuilder/cover.c` (+ shared types from `cover.h`).
//!
//! Build configuration: ZSTD_MULTITHREAD is NOT defined, DEBUGLEVEL 0.
//! - `ZSTD_pthread_*` are no-ops; the mutex/cond types are `int`-like placeholders.
//! - `POOL_add` runs jobs synchronously (see common/pool.rs), so the parallel
//!   trials in `ZDICT_optimizeTrainFromBuffer_cover` run sequentially in
//!   submission order -> deterministic and matches the C.
//! - asserts / DEBUGLOG dropped. DISPLAY* macros print to stderr, gated on the
//!   `g_displayLevel` / notificationLevel parameter.

#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::common::error_private::{ERR_isError, ERROR};
use crate::common::error_private::{
    ZSTD_error_GENERIC, ZSTD_error_dstSize_tooSmall, ZSTD_error_memory_allocation,
    ZSTD_error_parameter_outOfBound, ZSTD_error_srcSize_wrong,
};
use crate::common::mem::{size_t, MEM_readLE64, BYTE, U32, U64};
use crate::common::pool::{POOL_add, POOL_create, POOL_ctx, POOL_free};
use crate::common::zstd_internal::{free, malloc, memcpy, memset, MAX, MIN};

use crate::common::bits::ZSTD_highbit32;

/* ===== libc glibc qsort_r (the C compiles the _GNU_SOURCE path) ===== */
unsafe extern "C" {
    fn qsort_r(
        base: *mut c_void,
        nmemb: size_t,
        size: size_t,
        compar: unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int,
        arg: *mut c_void,
    );
    // memcmp for COVER_cmp (d bytes)
    fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    // console display
    static stderr: *mut c_void;
    fn fputs(s: *const c_char, stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/* ===== external public API symbols (owned by concurrent agents) ===== */
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

    fn ZSTD_compressBound(srcSize: size_t) -> size_t;
    fn ZSTD_createCCtx() -> *mut c_void;
    fn ZSTD_createCDict(
        dictBuffer: *const c_void,
        dictSize: size_t,
        compressionLevel: c_int,
    ) -> *mut c_void;
    fn ZSTD_freeCCtx(cctx: *mut c_void) -> size_t;
    fn ZSTD_freeCDict(cdict: *mut c_void) -> size_t;
    fn ZSTD_compress_usingCDict(
        cctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
        cdict: *const c_void,
    ) -> size_t;
}

/* ZSTD_isError / ZDICT_isError are behaviourally ERR_isError. */
#[inline(always)]
unsafe fn ZSTD_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}
#[inline(always)]
unsafe fn ZDICT_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

/* ===================================================================== */
/*  Public header types (zdict.h) — defined here to avoid collision with */
/*  a concurrent agent's zdict.rs, which should `use` these from here.    */
/* ===================================================================== */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_params_t {
    pub compressionLevel: c_int,
    pub notificationLevel: c_uint,
    pub dictID: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZDICT_cover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
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

pub const ZDICT_DICTSIZE_MIN: size_t = 256;

/* ===================================================================== */
/*  cover.h shared types                                                 */
/* ===================================================================== */

/* Non-MT placeholders: typedef int ZSTD_pthread_mutex_t/cond_t */
pub type ZSTD_pthread_mutex_t = c_int;
pub type ZSTD_pthread_cond_t = c_int;

#[repr(C)]
pub struct COVER_best_t {
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub liveJobs: size_t,
    pub dict: *mut c_void,
    pub dictSize: size_t,
    pub parameters: ZDICT_cover_params_t,
    pub compressedSize: size_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_segment_t {
    pub begin: U32,
    pub end: U32,
    pub score: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_epoch_info_t {
    pub num: U32,
    pub size: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_dictSelection_t {
    pub dictContent: *mut BYTE,
    pub dictSize: size_t,
    pub totalCompressedSize: size_t,
}

/* ===================================================================== */
/*  Constants                                                            */
/* ===================================================================== */

/* COVER_MAX_SAMPLES_SIZE (sizeof(size_t)==8 ? (U32)-1 : 1GB); 64-bit build */
const COVER_MAX_SAMPLES_SIZE: c_uint = 0xFFFFFFFF;
const COVER_DEFAULT_SPLITPOINT: f64 = 1.0;

/* ===================================================================== */
/*  Console display                                                      */
/* ===================================================================== */

static mut g_displayLevel: c_int = 0;
/* g_refreshRate = CLOCKS_PER_SEC * 15 / 100; g_time is used only for
 * DISPLAYUPDATE throttling of a progress '\r..%' line. */
static mut g_time: i64 = 0;

/* CLOCKS_PER_SEC is 1000000 on glibc. */
const G_REFRESH_RATE: i64 = 1_000_000i64 * 15 / 100;

unsafe extern "C" {
    fn clock() -> i64;
}

/* DISPLAY(...) : fprintf(stderr, ...); fflush(stderr); — we render the
 * formatted text ourselves and push it to stderr byte-for-byte. */
unsafe fn cover_display(s: &[u8]) {
    // s must be NUL-terminated for fputs.
    fputs(s.as_ptr() as *const c_char, stderr);
    fflush(stderr);
}

/* ===================================================================== */
/*  Hash table (activeDmers)                                             */
/* ===================================================================== */

const MAP_EMPTY_VALUE: U32 = 0xFFFFFFFF; /* (U32)-1 */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_map_pair_t {
    pub key: U32,
    pub value: U32,
}

#[repr(C)]
pub struct COVER_map_t {
    pub data: *mut COVER_map_pair_t,
    pub sizeLog: U32,
    pub size: U32,
    pub sizeMask: U32,
}

/// Clear the map.
pub unsafe fn COVER_map_clear(map: *mut COVER_map_t) {
    memset(
        (*map).data as *mut c_void,
        MAP_EMPTY_VALUE as c_int,
        (*map).size as size_t * core::mem::size_of::<COVER_map_pair_t>(),
    );
}

/// Initializes a map of the given size. Returns 1 on success and 0 on failure.
pub unsafe fn COVER_map_init(map: *mut COVER_map_t, size: U32) -> c_int {
    (*map).sizeLog = ZSTD_highbit32(size) + 2;
    (*map).size = (1u32) << (*map).sizeLog;
    (*map).sizeMask = (*map).size - 1;
    (*map).data = malloc(
        (*map).size as size_t * core::mem::size_of::<COVER_map_pair_t>(),
    ) as *mut COVER_map_pair_t;
    if (*map).data.is_null() {
        (*map).sizeLog = 0;
        (*map).size = 0;
        return 0;
    }
    COVER_map_clear(map);
    1
}

const COVER_prime4bytes: U32 = 2654435761u32;
unsafe fn COVER_map_hash(map: *mut COVER_map_t, key: U32) -> U32 {
    (key.wrapping_mul(COVER_prime4bytes)) >> (32 - (*map).sizeLog)
}

/// Helper function that returns the index that a key should be placed into.
unsafe fn COVER_map_index(map: *mut COVER_map_t, key: U32) -> U32 {
    let hash = COVER_map_hash(map, key);
    let mut i = hash;
    loop {
        let pos = (*map).data.add(i as usize);
        if (*pos).value == MAP_EMPTY_VALUE {
            return i;
        }
        if (*pos).key == key {
            return i;
        }
        i = (i + 1) & (*map).sizeMask;
    }
}

/// Returns the pointer to the value for key. Inserts with value 0 if absent.
unsafe fn COVER_map_at(map: *mut COVER_map_t, key: U32) -> *mut U32 {
    let pos = (*map).data.add(COVER_map_index(map, key) as usize);
    if (*pos).value == MAP_EMPTY_VALUE {
        (*pos).key = key;
        (*pos).value = 0;
    }
    &mut (*pos).value
}

/// Deletes key from the map if present.
unsafe fn COVER_map_remove(map: *mut COVER_map_t, key: U32) {
    let mut i = COVER_map_index(map, key);
    let mut del = (*map).data.add(i as usize);
    let mut shift: U32 = 1;
    if (*del).value == MAP_EMPTY_VALUE {
        return;
    }
    i = (i + 1) & (*map).sizeMask;
    loop {
        let pos = (*map).data.add(i as usize);
        /* If the position is empty we are done */
        if (*pos).value == MAP_EMPTY_VALUE {
            (*del).value = MAP_EMPTY_VALUE;
            return;
        }
        /* If pos can be moved to del do so */
        if ((i.wrapping_sub(COVER_map_hash(map, (*pos).key))) & (*map).sizeMask) >= shift {
            (*del).key = (*pos).key;
            (*del).value = (*pos).value;
            del = pos;
            shift = 1;
        } else {
            shift += 1;
        }
        i = (i + 1) & (*map).sizeMask;
    }
}

/// Destroys a map that is inited with COVER_map_init().
pub unsafe fn COVER_map_destroy(map: *mut COVER_map_t) {
    if !(*map).data.is_null() {
        free((*map).data as *mut c_void);
    }
    (*map).data = core::ptr::null_mut();
    (*map).size = 0;
}

/* ===================================================================== */
/*  Context                                                              */
/* ===================================================================== */

#[repr(C)]
pub struct COVER_ctx_t {
    pub samples: *const BYTE,
    pub offsets: *mut size_t,
    pub samplesSizes: *const size_t,
    pub nbSamples: size_t,
    pub nbTrainSamples: size_t,
    pub nbTestSamples: size_t,
    pub suffix: *mut U32,
    pub suffixSize: size_t,
    pub freqs: *mut U32,
    pub dmerAt: *mut U32,
    pub d: c_uint,
}

/* ===================================================================== */
/*  Helper functions                                                     */
/* ===================================================================== */

/// Returns the sum of the sample sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_sum(samplesSizes: *const size_t, nbSamples: c_uint) -> size_t {
    let mut sum: size_t = 0;
    let mut i: c_uint = 0;
    while i < nbSamples {
        sum = sum.wrapping_add(*samplesSizes.add(i as usize));
        i += 1;
    }
    sum
}

/// Returns memcmp of the dmers at lp/rp over ctx->d bytes.
unsafe fn COVER_cmp(ctx: *mut COVER_ctx_t, lp: *const c_void, rp: *const c_void) -> c_int {
    let lhs = *(lp as *const U32);
    let rhs = *(rp as *const U32);
    memcmp(
        (*ctx).samples.add(lhs as usize) as *const c_void,
        (*ctx).samples.add(rhs as usize) as *const c_void,
        (*ctx).d as size_t,
    )
}

/// Faster version for d <= 8.
unsafe fn COVER_cmp8(ctx: *mut COVER_ctx_t, lp: *const c_void, rp: *const c_void) -> c_int {
    let mask: U64 = if (*ctx).d == 8 {
        0xFFFFFFFFFFFFFFFFu64
    } else {
        (1u64 << (8 * (*ctx).d)) - 1
    };
    let lhs = MEM_readLE64((*ctx).samples.add(*(lp as *const U32) as usize)) & mask;
    let rhs = MEM_readLE64((*ctx).samples.add(*(rp as *const U32) as usize)) & mask;
    if lhs < rhs {
        return -1;
    }
    (lhs > rhs) as c_int
}

/// Same as COVER_cmp() except ties are broken by pointer value.
/// _GNU_SOURCE signature: (lp, rp, ctx).
unsafe extern "C" fn COVER_strict_cmp(
    lp: *const c_void,
    rp: *const c_void,
    g_coverCtx: *mut c_void,
) -> c_int {
    let mut result = COVER_cmp(g_coverCtx as *mut COVER_ctx_t, lp, rp);
    if result == 0 {
        result = if (lp as usize) < (rp as usize) { -1 } else { 1 };
    }
    result
}

/// Faster version for d <= 8.
unsafe extern "C" fn COVER_strict_cmp8(
    lp: *const c_void,
    rp: *const c_void,
    g_coverCtx: *mut c_void,
) -> c_int {
    let mut result = COVER_cmp8(g_coverCtx as *mut COVER_ctx_t, lp, rp);
    if result == 0 {
        result = if (lp as usize) < (rp as usize) { -1 } else { 1 };
    }
    result
}

/// Abstract away divergence of qsort_r() parameters (_GNU_SOURCE path).
unsafe fn stableSort(ctx: *mut COVER_ctx_t) {
    let cmp: unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int =
        if (*ctx).d <= 8 {
            COVER_strict_cmp8
        } else {
            COVER_strict_cmp
        };
    qsort_r(
        (*ctx).suffix as *mut c_void,
        (*ctx).suffixSize,
        core::mem::size_of::<U32>(),
        cmp,
        ctx as *mut c_void,
    );
}

/// Returns the first pointer in [first, last) whose element does not compare
/// less than value.
unsafe fn COVER_lower_bound(
    first: *const size_t,
    last: *const size_t,
    value: size_t,
) -> *const size_t {
    let mut first = first;
    let mut count = (last as usize - first as usize) / core::mem::size_of::<size_t>();
    while count != 0 {
        let step = count / 2;
        let mut ptr = first;
        ptr = ptr.add(step);
        if *ptr < value {
            first = ptr.add(1);
            count -= step + 1;
        } else {
            count = step;
        }
    }
    first
}

/// Generic groupBy function.
unsafe fn COVER_groupBy(
    data: *const c_void,
    count: size_t,
    size: size_t,
    ctx: *mut COVER_ctx_t,
    cmp: unsafe fn(*mut COVER_ctx_t, *const c_void, *const c_void) -> c_int,
    grp: unsafe fn(*mut COVER_ctx_t, *const c_void, *const c_void),
) {
    let mut ptr = data as *const BYTE;
    let mut num: size_t = 0;
    while num < count {
        let mut grpEnd = ptr.add(size);
        num += 1;
        while num < count
            && cmp(ctx, ptr as *const c_void, grpEnd as *const c_void) == 0
        {
            grpEnd = grpEnd.add(size);
            num += 1;
        }
        grp(ctx, ptr as *const c_void, grpEnd as *const c_void);
        ptr = grpEnd;
    }
}

/* ===================================================================== */
/*  Cover functions                                                      */
/* ===================================================================== */

/// Called on each group of positions with the same dmer.
unsafe fn COVER_group(ctx: *mut COVER_ctx_t, group: *const c_void, groupEnd: *const c_void) {
    let grpPtr0 = group as *const U32;
    let grpEnd = groupEnd as *const U32;
    let dmerId = (grpPtr0 as usize - (*ctx).suffix as usize) / core::mem::size_of::<U32>();
    let dmerId = dmerId as U32;
    let mut freq: U32 = 0;
    let mut curOffsetPtr = (*ctx).offsets as *const size_t;
    let offsetsEnd = ((*ctx).offsets as *const size_t).add((*ctx).nbSamples as usize);
    let mut curSampleEnd: size_t = *(*ctx).offsets;
    let mut grpPtr = grpPtr0;
    while grpPtr != grpEnd {
        *(*ctx).dmerAt.add(*grpPtr as usize) = dmerId;
        if (*grpPtr as size_t) < curSampleEnd {
            grpPtr = grpPtr.add(1);
            continue;
        }
        freq += 1;
        if grpPtr.add(1) != grpEnd {
            let sampleEndPtr = COVER_lower_bound(curOffsetPtr, offsetsEnd, *grpPtr as size_t);
            curSampleEnd = *sampleEndPtr;
            curOffsetPtr = sampleEndPtr.add(1);
        }
        grpPtr = grpPtr.add(1);
    }
    *(*ctx).suffix.add(dmerId as usize) = freq;
}

/// Selects the best segment in an epoch.
unsafe fn COVER_selectSegment(
    ctx: *const COVER_ctx_t,
    freqs: *mut U32,
    activeDmers: *mut COVER_map_t,
    begin: U32,
    end: U32,
    parameters: ZDICT_cover_params_t,
) -> COVER_segment_t {
    let k = parameters.k;
    let d = parameters.d;
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
    COVER_map_clear(activeDmers);
    activeSegment.begin = begin;
    activeSegment.end = begin;
    activeSegment.score = 0;
    while activeSegment.end < end {
        let newDmer = *(*ctx).dmerAt.add(activeSegment.end as usize);
        let newDmerOcc = COVER_map_at(activeDmers, newDmer);
        if *newDmerOcc == 0 {
            activeSegment.score = activeSegment
                .score
                .wrapping_add(*freqs.add(newDmer as usize));
        }
        activeSegment.end += 1;
        *newDmerOcc += 1;

        if activeSegment.end - activeSegment.begin == dmersInK + 1 {
            let delDmer = *(*ctx).dmerAt.add(activeSegment.begin as usize);
            let delDmerOcc = COVER_map_at(activeDmers, delDmer);
            activeSegment.begin += 1;
            *delDmerOcc -= 1;
            if *delDmerOcc == 0 {
                COVER_map_remove(activeDmers, delDmer);
                activeSegment.score = activeSegment
                    .score
                    .wrapping_sub(*freqs.add(delDmer as usize));
            }
        }

        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }
    {
        /* Trim off the zero frequency head and tail from the segment. */
        let mut newBegin: U32 = bestSegment.end;
        let mut newEnd: U32 = bestSegment.begin;
        let mut pos = bestSegment.begin;
        while pos != bestSegment.end {
            let freq = *freqs.add(*(*ctx).dmerAt.add(pos as usize) as usize);
            if freq != 0 {
                newBegin = MIN(newBegin, pos);
                newEnd = pos + 1;
            }
            pos += 1;
        }
        bestSegment.begin = newBegin;
        bestSegment.end = newEnd;
    }
    {
        /* Zero out the frequency of each dmer covered by the chosen segment. */
        let mut pos = bestSegment.begin;
        while pos != bestSegment.end {
            *freqs.add(*(*ctx).dmerAt.add(pos as usize) as usize) = 0;
            pos += 1;
        }
    }
    bestSegment
}

/// Check the validity of the parameters.
unsafe fn COVER_checkParameters(parameters: ZDICT_cover_params_t, maxDictSize: size_t) -> c_int {
    if parameters.d == 0 || parameters.k == 0 {
        return 0;
    }
    if parameters.k as size_t > maxDictSize {
        return 0;
    }
    if parameters.d > parameters.k {
        return 0;
    }
    if parameters.splitPoint <= 0.0 || parameters.splitPoint > 1.0 {
        return 0;
    }
    1
}

/// Clean up a context initialized with `COVER_ctx_init()`.
unsafe fn COVER_ctx_destroy(ctx: *mut COVER_ctx_t) {
    if ctx.is_null() {
        return;
    }
    if !(*ctx).suffix.is_null() {
        free((*ctx).suffix as *mut c_void);
        (*ctx).suffix = core::ptr::null_mut();
    }
    if !(*ctx).freqs.is_null() {
        free((*ctx).freqs as *mut c_void);
        (*ctx).freqs = core::ptr::null_mut();
    }
    if !(*ctx).dmerAt.is_null() {
        free((*ctx).dmerAt as *mut c_void);
        (*ctx).dmerAt = core::ptr::null_mut();
    }
    if !(*ctx).offsets.is_null() {
        free((*ctx).offsets as *mut c_void);
        (*ctx).offsets = core::ptr::null_mut();
    }
}

/// Prepare a context for dictionary building.
unsafe fn COVER_ctx_init(
    ctx: *mut COVER_ctx_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    d: c_uint,
    splitPoint: f64,
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
    /* Checks */
    if totalSamplesSize < MAX(d as size_t, core::mem::size_of::<U64>())
        || totalSamplesSize >= COVER_MAX_SAMPLES_SIZE as size_t
    {
        display_fmt(
            1,
            &format!(
                "Total samples size is too large ({} MB), maximum size is {} MB\n",
                (totalSamplesSize >> 20) as c_uint,
                (COVER_MAX_SAMPLES_SIZE >> 20)
            ),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if nbTrainSamples < 5 {
        display_fmt(
            1,
            &format!(
                "Total number of training samples is {} and is invalid.",
                nbTrainSamples
            ),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if nbTestSamples < 1 {
        display_fmt(
            1,
            &format!(
                "Total number of testing samples is {} and is invalid.",
                nbTestSamples
            ),
        );
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    /* Zero the context */
    memset(ctx as *mut c_void, 0, core::mem::size_of::<COVER_ctx_t>());
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
    /* Partial suffix array */
    (*ctx).suffixSize = trainingSamplesSize - MAX(d as size_t, core::mem::size_of::<U64>()) + 1;
    (*ctx).suffix = malloc((*ctx).suffixSize * core::mem::size_of::<U32>()) as *mut U32;
    (*ctx).dmerAt = malloc((*ctx).suffixSize * core::mem::size_of::<U32>()) as *mut U32;
    (*ctx).offsets =
        malloc((nbSamples as size_t + 1) * core::mem::size_of::<size_t>()) as *mut size_t;
    if (*ctx).suffix.is_null() || (*ctx).dmerAt.is_null() || (*ctx).offsets.is_null() {
        display_fmt(1, "Failed to allocate scratch buffers\n");
        COVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*ctx).freqs = core::ptr::null_mut();
    (*ctx).d = d;

    /* Fill offsets from the samplesSizes */
    {
        *(*ctx).offsets.add(0) = 0;
        let mut i: U32 = 1;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) = *(*ctx).offsets.add((i - 1) as usize)
                + *samplesSizes.add((i - 1) as usize);
            i += 1;
        }
    }
    display_fmt(2, "Constructing partial suffix array\n");
    {
        let mut i: U32 = 0;
        while (i as size_t) < (*ctx).suffixSize {
            *(*ctx).suffix.add(i as usize) = i;
            i += 1;
        }
        stableSort(ctx);
    }
    display_fmt(2, "Computing frequencies\n");
    let cmp: unsafe fn(*mut COVER_ctx_t, *const c_void, *const c_void) -> c_int =
        if (*ctx).d <= 8 { COVER_cmp8 } else { COVER_cmp };
    COVER_groupBy(
        (*ctx).suffix as *const c_void,
        (*ctx).suffixSize,
        core::mem::size_of::<U32>(),
        ctx,
        cmp,
        COVER_group,
    );
    (*ctx).freqs = (*ctx).suffix;
    (*ctx).suffix = core::ptr::null_mut();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_warnOnSmallCorpus(
    maxDictSize: size_t,
    nbDmers: size_t,
    displayLevel: c_int,
) {
    let ratio = nbDmers as f64 / maxDictSize as f64;
    if ratio >= 10.0 {
        return;
    }
    local_display_fmt(
        displayLevel,
        1,
        &format!(
            "WARNING: The maximum dictionary size {} is too large compared to the source size {}! size(source)/size(dictionary) = {}, but it should be >= 10! This may lead to a subpar dictionary! We recommend training on sources at least 10x, and preferably 100x the size of the dictionary! \n",
            maxDictSize as U32, nbDmers as U32, ratio
        ),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_computeEpochs(
    maxDictSize: U32,
    nbDmers: U32,
    k: U32,
    passes: U32,
) -> COVER_epoch_info_t {
    let minEpochSize = k * 10;
    let mut epochs = COVER_epoch_info_t { num: 0, size: 0 };
    epochs.num = MAX(1u32, maxDictSize / k / passes);
    epochs.size = nbDmers / epochs.num;
    if epochs.size >= minEpochSize {
        return epochs;
    }
    epochs.size = MIN(minEpochSize, nbDmers);
    epochs.num = nbDmers / epochs.size;
    epochs
}

/// Given the prepared context build the dictionary.
unsafe fn COVER_buildDictionary(
    ctx: *const COVER_ctx_t,
    freqs: *mut U32,
    activeDmers: *mut COVER_map_t,
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    parameters: ZDICT_cover_params_t,
) -> size_t {
    let dict = dictBuffer as *mut BYTE;
    let mut tail = dictBufferCapacity;
    let epochs = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).suffixSize as U32,
        parameters.k,
        4,
    );
    let maxZeroScoreRun = MAX(10usize, MIN(100usize, (epochs.num >> 3) as size_t));
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
        let segment = COVER_selectSegment(
            ctx,
            freqs,
            activeDmers,
            epochBegin,
            epochEnd,
            parameters,
        );
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    mut parameters: ZDICT_cover_params_t,
) -> size_t {
    let dict = dictBuffer as *mut BYTE;
    let mut ctx: COVER_ctx_t = core::mem::zeroed();
    let mut activeDmers: COVER_map_t = core::mem::zeroed();
    parameters.splitPoint = 1.0;
    /* Initialize global data */
    g_displayLevel = parameters.zParams.notificationLevel as c_int;
    /* Checks */
    if COVER_checkParameters(parameters, dictBufferCapacity) == 0 {
        display_fmt(1, "Cover parameters incorrect\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        display_fmt(1, "Cover must have at least one input file\n");
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
    {
        let initVal = COVER_ctx_init(
            &mut ctx,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters.d,
            parameters.splitPoint,
        );
        if ZSTD_isError(initVal) != 0 {
            return initVal;
        }
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.suffixSize, g_displayLevel);
    if COVER_map_init(&mut activeDmers, parameters.k - parameters.d + 1) == 0 {
        display_fmt(1, "Failed to allocate dmer map: out of memory\n");
        COVER_ctx_destroy(&mut ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }

    display_fmt(2, "Building dictionary\n");
    {
        let tail = COVER_buildDictionary(
            &ctx,
            ctx.freqs,
            &mut activeDmers,
            dictBuffer,
            dictBufferCapacity,
            parameters,
        );
        let dictionarySize = ZDICT_finalizeDictionary(
            dict as *mut c_void,
            dictBufferCapacity,
            dict.add(tail) as *const c_void,
            dictBufferCapacity - tail,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters.zParams,
        );
        if ZSTD_isError(dictionarySize) == 0 {
            display_fmt(
                2,
                &format!("Constructed dictionary of size {}\n", dictionarySize as c_uint),
            );
        }
        COVER_ctx_destroy(&mut ctx);
        COVER_map_destroy(&mut activeDmers);
        dictionarySize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_checkTotalCompressedSize(
    parameters: ZDICT_cover_params_t,
    samplesSizes: *const size_t,
    samples: *const BYTE,
    offsets: *mut size_t,
    nbTrainSamples: size_t,
    nbSamples: size_t,
    dict: *mut BYTE,
    dictBufferCapacity: size_t,
) -> size_t {
    let mut totalCompressedSize: size_t = ERROR(ZSTD_error_GENERIC);
    let cctx: *mut c_void;
    let cdict: *mut c_void;
    let dst: *mut c_void;
    let dstCapacity: size_t;
    let mut i: size_t;
    {
        let mut maxSampleSize: size_t = 0;
        i = if parameters.splitPoint < 1.0 {
            nbTrainSamples
        } else {
            0
        };
        while i < nbSamples {
            maxSampleSize = MAX(*samplesSizes.add(i as usize), maxSampleSize);
            i += 1;
        }
        dstCapacity = ZSTD_compressBound(maxSampleSize);
        dst = malloc(dstCapacity);
    }
    cctx = ZSTD_createCCtx();
    cdict = ZSTD_createCDict(
        dict as *const c_void,
        dictBufferCapacity,
        parameters.zParams.compressionLevel,
    );
    if dst.is_null() || cctx.is_null() || cdict.is_null() {
        ZSTD_freeCCtx(cctx);
        ZSTD_freeCDict(cdict);
        if !dst.is_null() {
            free(dst);
        }
        return totalCompressedSize;
    }
    totalCompressedSize = dictBufferCapacity;
    i = if parameters.splitPoint < 1.0 {
        nbTrainSamples
    } else {
        0
    };
    while i < nbSamples {
        let size = ZSTD_compress_usingCDict(
            cctx,
            dst,
            dstCapacity,
            samples.add(*offsets.add(i as usize) as usize) as *const c_void,
            *samplesSizes.add(i as usize),
            cdict,
        );
        if ZSTD_isError(size) != 0 {
            totalCompressedSize = size;
            ZSTD_freeCCtx(cctx);
            ZSTD_freeCDict(cdict);
            if !dst.is_null() {
                free(dst);
            }
            return totalCompressedSize;
        }
        totalCompressedSize += size;
        i += 1;
    }
    ZSTD_freeCCtx(cctx);
    ZSTD_freeCDict(cdict);
    if !dst.is_null() {
        free(dst);
    }
    totalCompressedSize
}

/* ===================================================================== */
/*  COVER_best_t (non-MT: ZSTD_pthread_* are no-ops)                     */
/* ===================================================================== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_init(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    /* ZSTD_pthread_mutex_init / cond_init : no-ops returning 0 */
    (*best).liveJobs = 0;
    (*best).dict = core::ptr::null_mut();
    (*best).dictSize = 0;
    (*best).compressedSize = size_t::MAX; /* (size_t)-1 */
    memset(
        &mut (*best).parameters as *mut ZDICT_cover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_wait(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    /* mutex lock/unlock no-ops; cond_wait no-op. liveJobs is already 0 in the
     * synchronous build (jobs run to completion during submission), so the
     * loop body never executes. */
    while (*best).liveJobs != 0 {
        /* ZSTD_pthread_cond_wait no-op — would spin forever if reached, but
         * liveJobs is decremented synchronously by COVER_best_finish. */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_destroy(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    COVER_best_wait(best);
    if !(*best).dict.is_null() {
        free((*best).dict);
    }
    /* mutex_destroy / cond_destroy : no-ops */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_start(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    (*best).liveJobs += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_finish(
    best: *mut COVER_best_t,
    parameters: ZDICT_cover_params_t,
    selection: COVER_dictSelection_t,
) {
    let dict = selection.dictContent;
    let compressedSize = selection.totalCompressedSize;
    let dictSize = selection.dictSize;
    if best.is_null() {
        return;
    }
    {
        (*best).liveJobs -= 1;
        let liveJobs = (*best).liveJobs;
        if compressedSize < (*best).compressedSize {
            if (*best).dict.is_null() || (*best).dictSize < dictSize {
                if !(*best).dict.is_null() {
                    free((*best).dict);
                }
                (*best).dict = malloc(dictSize);
                if (*best).dict.is_null() {
                    (*best).compressedSize = ERROR(ZSTD_error_GENERIC);
                    (*best).dictSize = 0;
                    return;
                }
            }
            if !dict.is_null() {
                memcpy((*best).dict, dict as *const c_void, dictSize);
                (*best).dictSize = dictSize;
                (*best).parameters = parameters;
                (*best).compressedSize = compressedSize;
            }
        }
        let _ = liveJobs;
        /* if (liveJobs == 0) cond_broadcast — no-op */
    }
}

unsafe fn setDictSelection(buf: *mut BYTE, s: size_t, csz: size_t) -> COVER_dictSelection_t {
    COVER_dictSelection_t {
        dictContent: buf,
        dictSize: s,
        totalCompressedSize: csz,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionError(error: size_t) -> COVER_dictSelection_t {
    setDictSelection(core::ptr::null_mut(), 0, error)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionIsError(selection: COVER_dictSelection_t) -> c_uint {
    (ZSTD_isError(selection.totalCompressedSize) != 0 || selection.dictContent.is_null()) as c_uint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionFree(selection: COVER_dictSelection_t) {
    free(selection.dictContent as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_selectDict(
    customDictContent: *mut BYTE,
    dictBufferCapacity: size_t,
    mut dictContentSize: size_t,
    samplesBuffer: *const BYTE,
    samplesSizes: *const size_t,
    nbFinalizeSamples: c_uint,
    nbCheckSamples: size_t,
    nbSamples: size_t,
    params: ZDICT_cover_params_t,
    offsets: *mut size_t,
    mut totalCompressedSize: size_t,
) -> COVER_dictSelection_t {
    let mut largestDict: size_t = 0;
    let mut largestCompressed: size_t = 0;
    let customDictContentEnd = customDictContent.add(dictContentSize);

    let largestDictbuffer = malloc(dictBufferCapacity) as *mut BYTE;
    let candidateDictBuffer = malloc(dictBufferCapacity) as *mut BYTE;
    let regressionTolerance = (params.shrinkDictMaxRegression as f64 / 100.0) + 1.00;

    if largestDictbuffer.is_null() || candidateDictBuffer.is_null() {
        free(largestDictbuffer as *mut c_void);
        free(candidateDictBuffer as *mut c_void);
        return COVER_dictSelectionError(dictContentSize);
    }

    memcpy(
        largestDictbuffer as *mut c_void,
        customDictContent as *const c_void,
        dictContentSize,
    );
    dictContentSize = ZDICT_finalizeDictionary(
        largestDictbuffer as *mut c_void,
        dictBufferCapacity,
        customDictContent as *const c_void,
        dictContentSize,
        samplesBuffer as *const c_void,
        samplesSizes,
        nbFinalizeSamples,
        params.zParams,
    );

    if ZDICT_isError(dictContentSize) != 0 {
        free(largestDictbuffer as *mut c_void);
        free(candidateDictBuffer as *mut c_void);
        return COVER_dictSelectionError(dictContentSize);
    }

    totalCompressedSize = COVER_checkTotalCompressedSize(
        params,
        samplesSizes,
        samplesBuffer,
        offsets,
        nbCheckSamples,
        nbSamples,
        largestDictbuffer,
        dictContentSize,
    );

    if ZSTD_isError(totalCompressedSize) != 0 {
        free(largestDictbuffer as *mut c_void);
        free(candidateDictBuffer as *mut c_void);
        return COVER_dictSelectionError(totalCompressedSize);
    }

    if params.shrinkDict == 0 {
        free(candidateDictBuffer as *mut c_void);
        return setDictSelection(largestDictbuffer, dictContentSize, totalCompressedSize);
    }

    largestDict = dictContentSize;
    largestCompressed = totalCompressedSize;
    dictContentSize = ZDICT_DICTSIZE_MIN;

    while dictContentSize < largestDict {
        memcpy(
            candidateDictBuffer as *mut c_void,
            largestDictbuffer as *const c_void,
            largestDict,
        );
        dictContentSize = ZDICT_finalizeDictionary(
            candidateDictBuffer as *mut c_void,
            dictBufferCapacity,
            customDictContentEnd.wrapping_sub(dictContentSize) as *const c_void,
            dictContentSize,
            samplesBuffer as *const c_void,
            samplesSizes,
            nbFinalizeSamples,
            params.zParams,
        );

        if ZDICT_isError(dictContentSize) != 0 {
            free(largestDictbuffer as *mut c_void);
            free(candidateDictBuffer as *mut c_void);
            return COVER_dictSelectionError(dictContentSize);
        }

        totalCompressedSize = COVER_checkTotalCompressedSize(
            params,
            samplesSizes,
            samplesBuffer,
            offsets,
            nbCheckSamples,
            nbSamples,
            candidateDictBuffer,
            dictContentSize,
        );

        if ZSTD_isError(totalCompressedSize) != 0 {
            free(largestDictbuffer as *mut c_void);
            free(candidateDictBuffer as *mut c_void);
            return COVER_dictSelectionError(totalCompressedSize);
        }

        if (totalCompressedSize as f64) <= (largestCompressed as f64) * regressionTolerance {
            free(largestDictbuffer as *mut c_void);
            return setDictSelection(candidateDictBuffer, dictContentSize, totalCompressedSize);
        }
        dictContentSize *= 2;
    }
    dictContentSize = largestDict;
    totalCompressedSize = largestCompressed;
    free(candidateDictBuffer as *mut c_void);
    setDictSelection(largestDictbuffer, dictContentSize, totalCompressedSize)
}

/// Parameters for COVER_tryParameters().
#[repr(C)]
struct COVER_tryParameters_data_t {
    ctx: *const COVER_ctx_t,
    best: *mut COVER_best_t,
    dictBufferCapacity: size_t,
    parameters: ZDICT_cover_params_t,
}

/// Tries a set of parameters and updates the COVER_best_t with the results.
unsafe extern "C" fn COVER_tryParameters(opaque: *mut c_void) {
    let data = opaque as *mut COVER_tryParameters_data_t;
    let ctx = (*data).ctx;
    let parameters = (*data).parameters;
    let dictBufferCapacity = (*data).dictBufferCapacity;
    let totalCompressedSize: size_t = ERROR(ZSTD_error_GENERIC);
    let mut activeDmers: COVER_map_t = core::mem::zeroed();
    let dict = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection = COVER_dictSelectionError(ERROR(ZSTD_error_GENERIC));
    let freqs = malloc((*ctx).suffixSize * core::mem::size_of::<U32>()) as *mut U32;
    'cleanup: {
        if COVER_map_init(&mut activeDmers, parameters.k - parameters.d + 1) == 0 {
            display_fmt(1, "Failed to allocate dmer map: out of memory\n");
            break 'cleanup;
        }
        if dict.is_null() || freqs.is_null() {
            display_fmt(1, "Failed to allocate buffers: out of memory\n");
            break 'cleanup;
        }
        memcpy(
            freqs as *mut c_void,
            (*ctx).freqs as *const c_void,
            (*ctx).suffixSize * core::mem::size_of::<U32>(),
        );
        {
            let tail = COVER_buildDictionary(
                ctx,
                freqs,
                &mut activeDmers,
                dict as *mut c_void,
                dictBufferCapacity,
                parameters,
            );
            selection = COVER_selectDict(
                dict.add(tail),
                dictBufferCapacity,
                dictBufferCapacity - tail,
                (*ctx).samples,
                (*ctx).samplesSizes,
                (*ctx).nbTrainSamples as c_uint,
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
    COVER_map_destroy(&mut activeDmers);
    COVER_dictSelectionFree(selection);
    free(freqs as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_optimizeTrainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: size_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const size_t,
    nbSamples: c_uint,
    parameters: *mut ZDICT_cover_params_t,
) -> size_t {
    let nbThreads = (*parameters).nbThreads;
    let splitPoint = if (*parameters).splitPoint <= 0.0 {
        COVER_DEFAULT_SPLITPOINT
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
        local_display_fmt(displayLevel, 1, "Incorrect parameters\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        local_display_fmt(displayLevel, 1, "Incorrect parameters\n");
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        display_fmt(1, "Cover must have at least one input file\n");
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
    if nbThreads > 1 {
        pool = POOL_create(nbThreads as size_t, 1);
        if pool.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
    }
    /* Initialization */
    COVER_best_init(&mut best);
    g_displayLevel = if displayLevel == 0 { 0 } else { displayLevel - 1 };
    local_display_fmt(
        displayLevel,
        2,
        &format!("Trying {} different sets of parameters\n", kIterations),
    );
    d = kMinD;
    while d <= kMaxD {
        let mut ctx: COVER_ctx_t = core::mem::zeroed();
        local_display_fmt(displayLevel, 3, &format!("d={}\n", d));
        {
            let initVal =
                COVER_ctx_init(&mut ctx, samplesBuffer, samplesSizes, nbSamples, d, splitPoint);
            if ZSTD_isError(initVal) != 0 {
                local_display_fmt(displayLevel, 1, "Failed to initialize context\n");
                COVER_best_destroy(&mut best);
                POOL_free(pool);
                return initVal;
            }
        }
        if warned == 0 {
            COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.suffixSize, displayLevel);
            warned = 1;
        }
        k = kMinK;
        while k <= kMaxK {
            let data =
                malloc(core::mem::size_of::<COVER_tryParameters_data_t>())
                    as *mut COVER_tryParameters_data_t;
            local_display_fmt(displayLevel, 3, &format!("k={}\n", k));
            if data.is_null() {
                local_display_fmt(displayLevel, 1, "Failed to allocate parameters\n");
                COVER_best_destroy(&mut best);
                COVER_ctx_destroy(&mut ctx);
                POOL_free(pool);
                return ERROR(ZSTD_error_memory_allocation);
            }
            (*data).ctx = &ctx;
            (*data).best = &mut best;
            (*data).dictBufferCapacity = dictBufferCapacity;
            (*data).parameters = *parameters;
            (*data).parameters.k = k;
            (*data).parameters.d = d;
            (*data).parameters.splitPoint = splitPoint;
            (*data).parameters.steps = kSteps;
            (*data).parameters.shrinkDict = shrinkDict;
            (*data).parameters.zParams.notificationLevel = g_displayLevel as c_uint;
            if COVER_checkParameters((*data).parameters, dictBufferCapacity) == 0 {
                display_fmt(1, "Cover parameters incorrect\n");
                free(data as *mut c_void);
                k += kStepSize;
                continue;
            }
            COVER_best_start(&mut best);
            if !pool.is_null() {
                POOL_add(pool, COVER_tryParameters, data as *mut c_void);
            } else {
                COVER_tryParameters(data as *mut c_void);
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
        COVER_ctx_destroy(&mut ctx);
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
        *parameters = best.parameters;
        memcpy(dictBuffer, best.dict as *const c_void, dictSize);
        COVER_best_destroy(&mut best);
        POOL_free(pool);
        dictSize
    }
}

/* ===================================================================== */
/*  Display helpers (shared with fastcover.rs)                           */
/* ===================================================================== */

/* DISPLAYLEVEL(l, ...) : if (g_displayLevel >= l) DISPLAY(...) */
pub(crate) unsafe fn display_fmt(l: c_int, s: &str) {
    if g_displayLevel >= l {
        emit(s);
    }
}
/* LOCALDISPLAYLEVEL(displayLevel, l, ...) */
pub(crate) unsafe fn local_display_fmt(displayLevel: c_int, l: c_int, s: &str) {
    if displayLevel >= l {
        emit(s);
    }
}
/* DISPLAYUPDATE(l, ...) : throttled progress line */
pub(crate) unsafe fn display_update(l: c_int, s: &str) {
    if g_displayLevel >= l {
        if (clock() - g_time > G_REFRESH_RATE) || (g_displayLevel >= 4) {
            g_time = clock();
            emit(s);
        }
    }
}
pub(crate) unsafe fn local_display_update(displayLevel: c_int, l: c_int, s: &str) {
    if displayLevel >= l {
        if (clock() - g_time > G_REFRESH_RATE) || (displayLevel >= 4) {
            g_time = clock();
            emit(s);
        }
    }
}

/* fprintf(stderr, ...); fflush(stderr); via NUL-terminated buffer */
unsafe fn emit(s: &str) {
    let mut buf: std::vec::Vec<u8> = std::vec::Vec::with_capacity(s.len() + 1);
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    fputs(buf.as_ptr() as *const c_char, stderr);
    fflush(stderr);
}
