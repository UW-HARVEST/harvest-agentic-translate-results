//! Translation of dictBuilder/cover.c
//!
//! Constructs a dictionary using a heuristic based on the COVER algorithm.
//! Single-threaded build (ZSTD_MULTITHREAD undefined): POOL runs jobs
//! synchronously, and the threading.h mutex/cond wrappers compile to no-ops.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(dead_code, unused_mut, unused_assignments, unused_parens, unused_variables)]

use core::ffi::{c_int, c_uint, c_void};

use crate::common::allocations::{free, malloc, memcpy, memset};
use crate::common::bits::highbit32;
use crate::common::error::{code as ecode, error as zerror};
use crate::common::mem::mem_read_le64;
use crate::common::pool::{POOL_add, POOL_create, POOL_ctx, POOL_free};

use crate::dictBuilder::zdict::{ZDICT_cover_params_t, ZDICT_params_t};

type BYTE = u8;
type U32 = u32;
type U64 = u64;

/*-*************************************
*  Threading placeholders (single-thread build)
***************************************/
/* threading.h: typedef int ZSTD_pthread_mutex_t / ZSTD_pthread_cond_t when
 * ZSTD_MULTITHREAD is undefined; all the mutex/cond ops are no-ops. */
pub type ZSTD_pthread_mutex_t = c_int;
pub type ZSTD_pthread_cond_t = c_int;

/*-*************************************
*  Constants
***************************************/
/* sizeof(size_t) == 8 on this target => (unsigned)-1 */
const COVER_MAX_SAMPLES_SIZE: c_uint = u32::MAX;
const COVER_DEFAULT_SPLITPOINT: f64 = 1.0;

const ZDICT_DICTSIZE_MIN: usize = 256;

/*-*************************************
*  External symbols
***************************************/
extern "C" {
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn qsort_r(
        base: *mut c_void,
        nmemb: usize,
        size: usize,
        compar: extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int,
        arg: *mut c_void,
    );
    fn ZSTD_compressBound(srcSize: usize) -> usize;
    fn ZSTD_createCCtx() -> *mut c_void;
    fn ZSTD_freeCCtx(cctx: *mut c_void) -> usize;
    fn ZSTD_createCDict(dict: *const c_void, dictSize: usize, compressionLevel: c_int) -> *mut c_void;
    fn ZSTD_freeCDict(cdict: *mut c_void) -> usize;
    fn ZSTD_compress_usingCDict(
        cctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        cdict: *const c_void,
    ) -> usize;
    fn ZSTD_isError(code: usize) -> c_uint;
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
    fn ZDICT_isError(code: usize) -> c_uint;
}

#[inline]
fn ERROR(code: c_int) -> usize {
    zerror(code)
}

/*-*************************************
*  Console display (no-op printing; state preserved)
***************************************/
static mut g_displayLevel: c_int = 0;

/*-*************************************
* Hash table
***************************************/

const MAP_EMPTY_VALUE: U32 = u32::MAX; /* (U32)-1 */

#[repr(C)]
#[derive(Clone, Copy)]
struct COVER_map_pair_t {
    key: U32,
    value: U32,
}

#[repr(C)]
struct COVER_map_t {
    data: *mut COVER_map_pair_t,
    sizeLog: U32,
    size: U32,
    sizeMask: U32,
}

/// Clear the map.
unsafe fn COVER_map_clear(map: *mut COVER_map_t) {
    memset(
        (*map).data as *mut c_void,
        MAP_EMPTY_VALUE as c_int,
        (*map).size as usize * core::mem::size_of::<COVER_map_pair_t>(),
    );
}

/// Initializes a map of the given size.
/// Returns 1 on success and 0 on failure.
unsafe fn COVER_map_init(map: *mut COVER_map_t, size: U32) -> c_int {
    (*map).sizeLog = highbit32(size) + 2;
    (*map).size = (1u32) << (*map).sizeLog;
    (*map).sizeMask = (*map).size - 1;
    (*map).data =
        malloc((*map).size as usize * core::mem::size_of::<COVER_map_pair_t>()) as *mut COVER_map_pair_t;
    if (*map).data.is_null() {
        (*map).sizeLog = 0;
        (*map).size = 0;
        return 0;
    }
    COVER_map_clear(map);
    1
}

/// Internal hash function
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

/// Returns the pointer to the value for key.
/// If key is not in the map, it is inserted and the value is set to 0.
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
unsafe fn COVER_map_destroy(map: *mut COVER_map_t) {
    if !(*map).data.is_null() {
        free((*map).data as *mut c_void);
    }
    (*map).data = core::ptr::null_mut();
    (*map).size = 0;
}

/*-*************************************
*  Public COVER types (from cover.h)
***************************************/

/// COVER_best_t is used for two purposes:
/// 1. Synchronizing threads.
/// 2. Saving the best parameters and dictionary.
#[repr(C)]
pub struct COVER_best_s {
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub liveJobs: usize,
    pub dict: *mut c_void,
    pub dictSize: usize,
    pub parameters: ZDICT_cover_params_t,
    pub compressedSize: usize,
}
pub type COVER_best_t = COVER_best_s;

/// A segment is a range in the source as well as the score of the segment.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_segment_t {
    pub begin: U32,
    pub end: U32,
    pub score: U32,
}

/// Number of epochs and size of each epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_epoch_info_t {
    pub num: U32,
    pub size: U32,
}

/// Struct used for the dictionary selection function.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_dictSelection_t {
    pub dictContent: *mut BYTE,
    pub dictSize: usize,
    pub totalCompressedSize: usize,
}

/*-*************************************
* Context
***************************************/

#[repr(C)]
struct COVER_ctx_t {
    samples: *const BYTE,
    offsets: *mut usize,
    samplesSizes: *const usize,
    nbSamples: usize,
    nbTrainSamples: usize,
    nbTestSamples: usize,
    suffix: *mut U32,
    suffixSize: usize,
    freqs: *mut U32,
    dmerAt: *mut U32,
    d: c_uint,
}

/*-*************************************
*  Helper functions
***************************************/

/// Returns the sum of the sample sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_sum(samplesSizes: *const usize, nbSamples: c_uint) -> usize {
    let mut sum: usize = 0;
    let mut i: c_uint = 0;
    while i < nbSamples {
        sum += *samplesSizes.add(i as usize);
        i += 1;
    }
    sum
}

/// Returns -1 if the dmer at lp is less than the dmer at rp.
/// Return 0 if the dmers at lp and rp are equal.
/// Returns 1 if the dmer at lp is greater than the dmer at rp.
unsafe fn COVER_cmp(ctx: *mut COVER_ctx_t, lp: *const c_void, rp: *const c_void) -> c_int {
    let lhs = *(lp as *const U32);
    let rhs = *(rp as *const U32);
    memcmp(
        (*ctx).samples.add(lhs as usize) as *const c_void,
        (*ctx).samples.add(rhs as usize) as *const c_void,
        (*ctx).d as usize,
    )
}

/// Faster version for d <= 8.
unsafe fn COVER_cmp8(ctx: *mut COVER_ctx_t, lp: *const c_void, rp: *const c_void) -> c_int {
    let mask: U64 = if (*ctx).d == 8 {
        u64::MAX
    } else {
        ((1u64) << (8 * (*ctx).d)) - 1
    };
    let lhs =
        mem_read_le64((*ctx).samples.add(*(lp as *const U32) as usize) as *const c_void) & mask;
    let rhs =
        mem_read_le64((*ctx).samples.add(*(rp as *const U32) as usize) as *const c_void) & mask;
    if lhs < rhs {
        return -1;
    }
    (lhs > rhs) as c_int
}

/// Same as COVER_cmp() except ties are broken by pointer value.
/// _GNU_SOURCE signature: (lp, rp, arg).
extern "C" fn COVER_strict_cmp(lp: *const c_void, rp: *const c_void, g_coverCtx: *mut c_void) -> c_int {
    unsafe {
        let mut result = COVER_cmp(g_coverCtx as *mut COVER_ctx_t, lp, rp);
        if result == 0 {
            result = if lp < rp { -1 } else { 1 };
        }
        result
    }
}

/// Faster version for d <= 8.
extern "C" fn COVER_strict_cmp8(lp: *const c_void, rp: *const c_void, g_coverCtx: *mut c_void) -> c_int {
    unsafe {
        let mut result = COVER_cmp8(g_coverCtx as *mut COVER_ctx_t, lp, rp);
        if result == 0 {
            result = if lp < rp { -1 } else { 1 };
        }
        result
    }
}

/// Abstract away divergence of qsort_r() parameters (GNU/Linux variant).
unsafe fn stableSort(ctx: *mut COVER_ctx_t) {
    qsort_r(
        (*ctx).suffix as *mut c_void,
        (*ctx).suffixSize,
        core::mem::size_of::<U32>(),
        if (*ctx).d <= 8 {
            COVER_strict_cmp8
        } else {
            COVER_strict_cmp
        },
        ctx as *mut c_void,
    );
}

/// Returns the first pointer in [first, last) whose element does not compare
/// less than value.  If no such element exists it returns last.
unsafe fn COVER_lower_bound(
    first: *const usize,
    last: *const usize,
    value: usize,
) -> *const usize {
    let mut first = first;
    let mut count = (last as usize - first as usize) / core::mem::size_of::<usize>();
    while count != 0 {
        let step = count / 2;
        let mut ptr = first;
        ptr = ptr.add(step);
        if *ptr < value {
            ptr = ptr.add(1);
            first = ptr;
            count -= step + 1;
        } else {
            count = step;
        }
    }
    first
}

/// Generic groupBy function.
/// Groups an array sorted by cmp into groups with equivalent values.
/// Calls grp for each group.
unsafe fn COVER_groupBy(
    data: *const c_void,
    count: usize,
    size: usize,
    ctx: *mut COVER_ctx_t,
    cmp: unsafe fn(*mut COVER_ctx_t, *const c_void, *const c_void) -> c_int,
    grp: unsafe fn(*mut COVER_ctx_t, *const c_void, *const c_void),
) {
    let mut ptr = data as *const BYTE;
    let mut num: usize = 0;
    while num < count {
        let mut grpEnd = ptr.add(size);
        num += 1;
        while num < count && cmp(ctx, ptr as *const c_void, grpEnd as *const c_void) == 0 {
            grpEnd = grpEnd.add(size);
            num += 1;
        }
        grp(ctx, ptr as *const c_void, grpEnd as *const c_void);
        ptr = grpEnd;
    }
}

/*-*************************************
*  Cover functions
***************************************/

/// Called on each group of positions with the same dmer.
/// Counts the frequency of each dmer and saves it in the suffix array.
/// Fills `ctx->dmerAt`.
unsafe fn COVER_group(ctx: *mut COVER_ctx_t, group: *const c_void, groupEnd: *const c_void) {
    /* The group consists of all the positions with the same first d bytes. */
    let mut grpPtr = group as *const U32;
    let grpEnd = groupEnd as *const U32;
    /* The dmerId is how we will reference this dmer. */
    let dmerId = (grpPtr as usize - (*ctx).suffix as usize) / core::mem::size_of::<U32>();
    let dmerId = dmerId as U32;
    /* Count the number of samples this dmer shows up in */
    let mut freq: U32 = 0;
    /* Details */
    let mut curOffsetPtr = (*ctx).offsets as *const usize;
    let offsetsEnd = (*ctx).offsets.add((*ctx).nbSamples) as *const usize;
    /* Once *grpPtr >= curSampleEnd this occurrence of the dmer is in a
     * different sample than the last.
     */
    let mut curSampleEnd = *(*ctx).offsets;
    while grpPtr != grpEnd {
        /* Save the dmerId for this position so we can get back to it. */
        *(*ctx).dmerAt.add(*grpPtr as usize) = dmerId;
        /* Dictionaries only help for the first reference to the dmer. */
        if (*grpPtr as usize) < curSampleEnd {
            grpPtr = grpPtr.add(1);
            continue;
        }
        freq += 1;
        /* Binary search to find the end of the sample *grpPtr is in. */
        if grpPtr.add(1) != grpEnd {
            let sampleEndPtr = COVER_lower_bound(curOffsetPtr, offsetsEnd, *grpPtr as usize);
            curSampleEnd = *sampleEndPtr;
            curOffsetPtr = sampleEndPtr.add(1);
        }
        grpPtr = grpPtr.add(1);
    }
    /* Store the frequency of the dmer in the first position of the group. */
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
    /* Constants */
    let k = parameters.k;
    let d = parameters.d;
    let dmersInK = k - d + 1;
    /* Try each segment (activeSegment) and save the best (bestSegment) */
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
    /* Reset the activeDmers in the segment */
    COVER_map_clear(activeDmers);
    /* The activeSegment starts at the beginning of the epoch. */
    activeSegment.begin = begin;
    activeSegment.end = begin;
    activeSegment.score = 0;
    /* Slide the activeSegment through the whole epoch. */
    while activeSegment.end < end {
        /* The dmerId for the dmer at the next position */
        let newDmer = *(*ctx).dmerAt.add(activeSegment.end as usize);
        /* The entry in activeDmers for this dmerId */
        let newDmerOcc = COVER_map_at(activeDmers, newDmer);
        /* If the dmer isn't already present in the segment add its score. */
        if *newDmerOcc == 0 {
            activeSegment.score += *freqs.add(newDmer as usize);
        }
        /* Add the dmer to the segment */
        activeSegment.end += 1;
        *newDmerOcc += 1;

        /* If the window is now too large, drop the first position */
        if activeSegment.end - activeSegment.begin == dmersInK + 1 {
            let delDmer = *(*ctx).dmerAt.add(activeSegment.begin as usize);
            let delDmerOcc = COVER_map_at(activeDmers, delDmer);
            activeSegment.begin += 1;
            *delDmerOcc -= 1;
            /* If this is the last occurrence of the dmer, subtract its score */
            if *delDmerOcc == 0 {
                COVER_map_remove(activeDmers, delDmer);
                activeSegment.score -= *freqs.add(delDmer as usize);
            }
        }

        /* If this segment is the best so far save it */
        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }
    {
        /* Trim off the zero frequency head and tail from the segment. */
        let mut newBegin = bestSegment.end;
        let mut newEnd = bestSegment.begin;
        let mut pos = bestSegment.begin;
        while pos != bestSegment.end {
            let freq = *freqs.add(*(*ctx).dmerAt.add(pos as usize) as usize);
            if freq != 0 {
                newBegin = if newBegin < pos { newBegin } else { pos };
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
/// Returns non-zero if the parameters are valid and 0 otherwise.
unsafe fn COVER_checkParameters(parameters: ZDICT_cover_params_t, maxDictSize: usize) -> c_int {
    /* k and d are required parameters */
    if parameters.d == 0 || parameters.k == 0 {
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
    /* 0 < splitPoint <= 1 */
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
/// Returns 0 on success or error code on error.
unsafe fn COVER_ctx_init(
    ctx: *mut COVER_ctx_t,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    d: c_uint,
    splitPoint: f64,
) -> usize {
    let samples = samplesBuffer as *const BYTE;
    let totalSamplesSize = COVER_sum(samplesSizes, nbSamples);
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
    /* Checks: MAX(d, sizeof(U64)) */
    let maxDU64: usize = if (d as usize) > core::mem::size_of::<U64>() {
        d as usize
    } else {
        core::mem::size_of::<U64>()
    };
    if totalSamplesSize < maxDU64 || totalSamplesSize >= COVER_MAX_SAMPLES_SIZE as usize {
        return ERROR(ecode::SRCSIZE_WRONG);
    }
    /* Check if there are at least 5 training samples */
    if nbTrainSamples < 5 {
        return ERROR(ecode::SRCSIZE_WRONG);
    }
    /* Check if there's testing sample */
    if nbTestSamples < 1 {
        return ERROR(ecode::SRCSIZE_WRONG);
    }
    /* Zero the context */
    memset(ctx as *mut c_void, 0, core::mem::size_of::<COVER_ctx_t>());
    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as usize;
    (*ctx).nbTrainSamples = nbTrainSamples as usize;
    (*ctx).nbTestSamples = nbTestSamples as usize;
    /* Partial suffix array */
    (*ctx).suffixSize = trainingSamplesSize - maxDU64 + 1;
    (*ctx).suffix = malloc((*ctx).suffixSize * core::mem::size_of::<U32>()) as *mut U32;
    /* Maps index to the dmerID */
    (*ctx).dmerAt = malloc((*ctx).suffixSize * core::mem::size_of::<U32>()) as *mut U32;
    /* The offsets of each file */
    (*ctx).offsets = malloc((nbSamples as usize + 1) * core::mem::size_of::<usize>()) as *mut usize;
    if (*ctx).suffix.is_null() || (*ctx).dmerAt.is_null() || (*ctx).offsets.is_null() {
        COVER_ctx_destroy(ctx);
        return ERROR(ecode::MEMORY_ALLOCATION);
    }
    (*ctx).freqs = core::ptr::null_mut();
    (*ctx).d = d;

    /* Fill offsets from the samplesSizes */
    {
        *(*ctx).offsets.add(0) = 0;
        let mut i: U32 = 1;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) =
                *(*ctx).offsets.add((i - 1) as usize) + *samplesSizes.add((i - 1) as usize);
            i += 1;
        }
    }
    {
        /* suffix is a partial suffix array. */
        let mut i: U32 = 0;
        while (i as usize) < (*ctx).suffixSize {
            *(*ctx).suffix.add(i as usize) = i;
            i += 1;
        }
        stableSort(ctx);
    }
    COVER_groupBy(
        (*ctx).suffix as *const c_void,
        (*ctx).suffixSize,
        core::mem::size_of::<U32>(),
        ctx,
        if (*ctx).d <= 8 { COVER_cmp8 } else { COVER_cmp },
        COVER_group,
    );
    (*ctx).freqs = (*ctx).suffix;
    (*ctx).suffix = core::ptr::null_mut();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_warnOnSmallCorpus(
    maxDictSize: usize,
    nbDmers: usize,
    displayLevel: c_int,
) {
    let ratio = nbDmers as f64 / maxDictSize as f64;
    if ratio >= 10.0 {
        return;
    }
    /* LOCALDISPLAYLEVEL warning (display disabled) */
    let _ = displayLevel;
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
    epochs.num = if 1 > maxDictSize / k / passes {
        1
    } else {
        maxDictSize / k / passes
    };
    epochs.size = nbDmers / epochs.num;
    if epochs.size >= minEpochSize {
        return epochs;
    }
    epochs.size = if minEpochSize < nbDmers {
        minEpochSize
    } else {
        nbDmers
    };
    epochs.num = nbDmers / epochs.size;
    epochs
}

/// Given the prepared context build the dictionary.
unsafe fn COVER_buildDictionary(
    ctx: *const COVER_ctx_t,
    freqs: *mut U32,
    activeDmers: *mut COVER_map_t,
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
) -> usize {
    let dict = dictBuffer as *mut BYTE;
    let mut tail = dictBufferCapacity;
    /* Divide the data into epochs. We will select one segment from each epoch. */
    let epochs = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).suffixSize as U32,
        parameters.k,
        4,
    );
    /* MAX(10, MIN(100, epochs.num >> 3)) */
    let maxZeroScoreRun: usize = {
        let a = if 100 < (epochs.num >> 3) as usize {
            100usize
        } else {
            (epochs.num >> 3) as usize
        };
        if 10 > a {
            10
        } else {
            a
        }
    };
    let mut zeroScoreRun: usize = 0;
    let mut epoch: usize = 0;
    /* Loop through the epochs until there are no more segments or full. */
    while tail > 0 {
        let epochBegin = (epoch * epochs.size as usize) as U32;
        let epochEnd = epochBegin + epochs.size;
        let mut segmentSize: usize;
        /* Select a segment */
        let segment =
            COVER_selectSegment(ctx, freqs, activeDmers, epochBegin, epochEnd, parameters);
        /* If the segment covers no dmers, then we are out of content. */
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
        segmentSize = {
            let s = (segment.end - segment.begin + parameters.d - 1) as usize;
            if s < tail {
                s
            } else {
                tail
            }
        };
        if segmentSize < parameters.d as usize {
            break;
        }
        /* We fill the dictionary from the back. */
        tail -= segmentSize;
        memcpy(
            dict.add(tail) as *mut c_void,
            (*ctx).samples.add(segment.begin as usize) as *const c_void,
            segmentSize,
        );
        epoch = (epoch + 1) % epochs.num as usize;
    }
    tail
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    mut parameters: ZDICT_cover_params_t,
) -> usize {
    let dict = dictBuffer as *mut BYTE;
    let mut ctx: COVER_ctx_t = core::mem::zeroed();
    let mut activeDmers: COVER_map_t = core::mem::zeroed();
    parameters.splitPoint = 1.0;
    /* Initialize global data */
    g_displayLevel = parameters.zParams.notificationLevel as c_int;
    /* Checks */
    if COVER_checkParameters(parameters, dictBufferCapacity) == 0 {
        return ERROR(ecode::PARAMETER_OUTOFBOUND);
    }
    if nbSamples == 0 {
        return ERROR(ecode::SRCSIZE_WRONG);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ecode::DSTSIZE_TOOSMALL);
    }
    /* Initialize context and activeDmers */
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
        COVER_ctx_destroy(&mut ctx);
        return ERROR(ecode::MEMORY_ALLOCATION);
    }

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
        COVER_ctx_destroy(&mut ctx);
        COVER_map_destroy(&mut activeDmers);
        dictionarySize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_checkTotalCompressedSize(
    parameters: ZDICT_cover_params_t,
    samplesSizes: *const usize,
    samples: *const BYTE,
    offsets: *mut usize,
    nbTrainSamples: usize,
    nbSamples: usize,
    dict: *mut BYTE,
    dictBufferCapacity: usize,
) -> usize {
    let mut totalCompressedSize: usize = ERROR(ecode::GENERIC);
    /* Pointers */
    let cctx: *mut c_void;
    let cdict: *mut c_void;
    let dst: *mut c_void;
    /* Local variables */
    let dstCapacity: usize;
    let mut i: usize;
    /* Allocate dst with enough space to compress the maximum sized sample */
    {
        let mut maxSampleSize: usize = 0;
        i = if parameters.splitPoint < 1.0 {
            nbTrainSamples
        } else {
            0
        };
        while i < nbSamples {
            let s = *samplesSizes.add(i);
            maxSampleSize = if s > maxSampleSize { s } else { maxSampleSize };
            i += 1;
        }
        dstCapacity = ZSTD_compressBound(maxSampleSize);
        dst = malloc(dstCapacity);
    }
    /* Create the cctx and cdict */
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
    /* Compress each sample and sum their sizes (or error) */
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
            samples.add(*offsets.add(i)) as *const c_void,
            *samplesSizes.add(i),
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

/// Initialize the `COVER_best_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_init(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    /* ZSTD_pthread_mutex_init / cond_init are no-ops in single-thread build */
    (*best).liveJobs = 0;
    (*best).dict = core::ptr::null_mut();
    (*best).dictSize = 0;
    (*best).compressedSize = usize::MAX;
    memset(
        &mut (*best).parameters as *mut ZDICT_cover_params_t as *mut c_void,
        0,
        core::mem::size_of::<ZDICT_cover_params_t>(),
    );
}

/// Wait until liveJobs == 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_wait(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    /* single-thread: liveJobs already 0 by the time we wait */
    while (*best).liveJobs != 0 {
        /* cond_wait is a no-op; loop would only terminate when liveJobs==0 */
        break;
    }
}

/// Call COVER_best_wait() and then destroy the COVER_best_t.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_destroy(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    COVER_best_wait(best);
    if !(*best).dict.is_null() {
        free((*best).dict);
    }
    /* mutex_destroy / cond_destroy are no-ops */
}

/// Called when a thread is about to be launched. Increments liveJobs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_start(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    (*best).liveJobs += 1;
}

/// Called when a thread finishes executing.
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
        let liveJobs: usize;
        (*best).liveJobs -= 1;
        liveJobs = (*best).liveJobs;
        /* If the new dictionary is better */
        if compressedSize < (*best).compressedSize {
            /* Allocate space if necessary */
            if (*best).dict.is_null() || (*best).dictSize < dictSize {
                if !(*best).dict.is_null() {
                    free((*best).dict);
                }
                (*best).dict = malloc(dictSize);
                if (*best).dict.is_null() {
                    (*best).compressedSize = ERROR(ecode::GENERIC);
                    (*best).dictSize = 0;
                    return;
                }
            }
            /* Save the dictionary, parameters, and size */
            if !dict.is_null() {
                memcpy((*best).dict, dict as *const c_void, dictSize);
                (*best).dictSize = dictSize;
                (*best).parameters = parameters;
                (*best).compressedSize = compressedSize;
            }
        }
        if liveJobs == 0 {
            /* cond_broadcast no-op */
        }
    }
}

fn setDictSelection(buf: *mut BYTE, s: usize, csz: usize) -> COVER_dictSelection_t {
    COVER_dictSelection_t {
        dictContent: buf,
        dictSize: s,
        totalCompressedSize: csz,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionError(error: usize) -> COVER_dictSelection_t {
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
    dictBufferCapacity: usize,
    mut dictContentSize: usize,
    samplesBuffer: *const BYTE,
    samplesSizes: *const usize,
    nbFinalizeSamples: c_uint,
    nbCheckSamples: usize,
    nbSamples: usize,
    params: ZDICT_cover_params_t,
    offsets: *mut usize,
    mut totalCompressedSize: usize,
) -> COVER_dictSelection_t {
    let mut largestDict: usize = 0;
    let mut largestCompressed: usize = 0;
    let customDictContentEnd = customDictContent.add(dictContentSize);

    let largestDictbuffer = malloc(dictBufferCapacity) as *mut BYTE;
    let candidateDictBuffer = malloc(dictBufferCapacity) as *mut BYTE;
    let regressionTolerance = (params.shrinkDictMaxRegression as f64 / 100.0) + 1.00;

    if largestDictbuffer.is_null() || candidateDictBuffer.is_null() {
        free(largestDictbuffer as *mut c_void);
        free(candidateDictBuffer as *mut c_void);
        return COVER_dictSelectionError(dictContentSize);
    }

    /* Initial dictionary size and compressed size */
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

    /* Largest dict is initially at least ZDICT_DICTSIZE_MIN */
    while dictContentSize < largestDict {
        memcpy(
            candidateDictBuffer as *mut c_void,
            largestDictbuffer as *const c_void,
            largestDict,
        );
        dictContentSize = ZDICT_finalizeDictionary(
            candidateDictBuffer as *mut c_void,
            dictBufferCapacity,
            customDictContentEnd.sub(dictContentSize) as *const c_void,
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
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
}

/// Tries a set of parameters and updates the COVER_best_t with the results.
/// Takes its parameters as an *OWNING* opaque pointer.
extern "C" fn COVER_tryParameters(opaque: *mut c_void) {
    unsafe {
        /* Save parameters as local variables */
        let data = opaque as *mut COVER_tryParameters_data_t;
        let ctx = (*data).ctx;
        let parameters = (*data).parameters;
        let dictBufferCapacity = (*data).dictBufferCapacity;
        let totalCompressedSize: usize = ERROR(ecode::GENERIC);
        /* Allocate space for hash table, dict, and freqs */
        let mut activeDmers: COVER_map_t = core::mem::zeroed();
        let dict = malloc(dictBufferCapacity) as *mut BYTE;
        let mut selection = COVER_dictSelectionError(ERROR(ecode::GENERIC));
        let freqs = malloc((*ctx).suffixSize * core::mem::size_of::<U32>()) as *mut U32;
        'cleanup: {
            if COVER_map_init(&mut activeDmers, parameters.k - parameters.d + 1) == 0 {
                break 'cleanup;
            }
            if dict.is_null() || freqs.is_null() {
                break 'cleanup;
            }
            /* Copy the frequencies because we need to modify them */
            memcpy(
                freqs as *mut c_void,
                (*ctx).freqs as *const c_void,
                (*ctx).suffixSize * core::mem::size_of::<U32>(),
            );
            /* Build the dictionary */
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_optimizeTrainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: *mut ZDICT_cover_params_t,
) -> usize {
    /* constants */
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
    let kStepSize = {
        let v = (kMaxK - kMinK) / kSteps;
        if v > 1 {
            v
        } else {
            1
        }
    };
    let kIterations = (1 + (kMaxD - kMinD) / 2) * (1 + (kMaxK - kMinK) / kStepSize);
    let shrinkDict: c_uint = 0;
    /* Local variables */
    let displayLevel = (*parameters).zParams.notificationLevel as c_int;
    let mut iteration: c_uint = 1;
    let mut d: c_uint;
    let mut k: c_uint;
    let mut best: COVER_best_t = core::mem::zeroed();
    let mut pool: *mut POOL_ctx = core::ptr::null_mut();
    let mut warned: c_int = 0;

    /* Checks */
    if splitPoint <= 0.0 || splitPoint > 1.0 {
        return ERROR(ecode::PARAMETER_OUTOFBOUND);
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        return ERROR(ecode::PARAMETER_OUTOFBOUND);
    }
    if nbSamples == 0 {
        return ERROR(ecode::SRCSIZE_WRONG);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ecode::DSTSIZE_TOOSMALL);
    }
    if nbThreads > 1 {
        pool = POOL_create(nbThreads as usize, 1);
        if pool.is_null() {
            return ERROR(ecode::MEMORY_ALLOCATION);
        }
    }
    /* Initialization */
    COVER_best_init(&mut best);
    /* Turn down global display level */
    g_displayLevel = if displayLevel == 0 { 0 } else { displayLevel - 1 };
    /* Loop through d first because each new value needs a new context */
    d = kMinD;
    while d <= kMaxD {
        /* Initialize the context for this value of d */
        let mut ctx: COVER_ctx_t = core::mem::zeroed();
        {
            let initVal =
                COVER_ctx_init(&mut ctx, samplesBuffer, samplesSizes, nbSamples, d, splitPoint);
            if ZSTD_isError(initVal) != 0 {
                COVER_best_destroy(&mut best);
                POOL_free(pool);
                return initVal;
            }
        }
        if warned == 0 {
            COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.suffixSize, displayLevel);
            warned = 1;
        }
        /* Loop through k reusing the same context */
        k = kMinK;
        while k <= kMaxK {
            /* Prepare the arguments */
            let data = malloc(core::mem::size_of::<COVER_tryParameters_data_t>())
                as *mut COVER_tryParameters_data_t;
            if data.is_null() {
                COVER_best_destroy(&mut best);
                COVER_ctx_destroy(&mut ctx);
                POOL_free(pool);
                return ERROR(ecode::MEMORY_ALLOCATION);
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
            /* Check the parameters */
            if COVER_checkParameters((*data).parameters, dictBufferCapacity) == 0 {
                free(data as *mut c_void);
                k += kStepSize;
                continue;
            }
            /* Call the function and pass ownership of data to it */
            COVER_best_start(&mut best);
            if !pool.is_null() {
                POOL_add(pool, COVER_tryParameters, data as *mut c_void);
            } else {
                COVER_tryParameters(data as *mut c_void);
            }
            iteration += 1;
            k += kStepSize;
        }
        COVER_best_wait(&mut best);
        COVER_ctx_destroy(&mut ctx);
        d += 2;
    }
    /* Fill the output buffer and parameters with output of the best parameters */
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

